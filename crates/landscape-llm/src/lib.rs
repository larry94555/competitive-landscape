//! Talking to `llama-server`, with the model's output constrained to a Rust type.
//!
//! This is the spine of the product. Every fact in a report arrives through this path:
//!
//! ```text
//! Rust struct  →  schemars JSON Schema  →  GBNF grammar  →  constrained decode  →  struct
//! ```
//!
//! The middle arrow is done by llama.cpp rather than by us. Its sampler is the thing that
//! actually enforces the grammar, so a second converter written here could differ from the
//! one doing the enforcing — and a grammar that disagrees with the sampler is worse than no
//! grammar, because it fails silently and only sometimes. [`Constraint::Grammar`] is there
//! for the day we need a grammar the schema cannot express.
//!
//! **Why this matters more than it looks.** A model that emits *almost* the right JSON is
//! useless at scale: a 1% parse failure over a report with 40 extracted values means most
//! reports lose something. Constrained decoding turns that from a retry-and-hope problem
//! into a structural guarantee, which is why `docs/ROADMAP.md` makes proving it a Phase 0
//! exit criterion rather than an implementation detail.

use std::time::Duration;

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("could not reach llama-server at {base}: {source}")]
    Unreachable {
        base: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("llama-server returned {status}: {body}")]
    Server { status: u16, body: String },

    /// The model produced something that does not fit the type it was constrained to.
    ///
    /// This should be impossible when the grammar is applied. It is a distinct variant
    /// precisely so that "the constraint is not working" cannot be mistaken for an
    /// ordinary transport failure and retried forever.
    #[error("output did not parse as the requested type: {source}\n  raw: {raw}")]
    Unparseable {
        raw: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("llama-server returned no content")]
    Empty,
}

pub type Result<T> = std::result::Result<T, LlmError>;

/// How the model's output is restricted.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// A JSON Schema. llama.cpp converts it to GBNF and enforces it while sampling.
    JsonSchema(serde_json::Value),
    /// A GBNF grammar, for shapes a JSON Schema cannot express.
    Grammar(String),
}

/// Decoding settings.
///
/// Low temperature by default: this path extracts values that already exist in a source
/// page. Creativity is not a feature here, it is a failure mode.
#[derive(Debug, Clone)]
pub struct Decode {
    pub max_tokens: u32,
    pub temperature: f32,
    /// Fixed seed makes a run reproducible, which is what lets a golden-set failure be
    /// investigated rather than shrugged at.
    pub seed: Option<u32>,
}

impl Default for Decode {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.1,
            seed: None,
        }
    }
}

/// A client for one `llama-server` process.
#[derive(Debug, Clone)]
pub struct LlamaClient {
    base: String,
    http: reqwest::Client,
}

impl LlamaClient {
    /// Point at a running server, e.g. `http://127.0.0.1:8080`.
    ///
    /// The timeout is generous because prefill on four ARM cores is slow — the binding
    /// constraint on the target hardware, per `docs/ARCHITECTURE.md`.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_default();
        Self {
            base: base_url.into().trim_end_matches('/').to_owned(),
            http,
        }
    }

    /// Read the server address from `LLAMA_URL`, defaulting to llama.cpp's own default.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(std::env::var("LLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned()))
    }

    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Whether the server is up and has a model loaded.
    pub async fn is_ready(&self) -> bool {
        match self.http.get(format!("{}/health", self.base)).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    /// Generate a `T`, with the sampler prevented from producing anything that is not one.
    ///
    /// The schema comes from `T` itself, so the constraint and the parse target cannot
    /// drift apart — changing the struct changes the grammar in the same commit.
    pub async fn generate<T>(&self, prompt: &str, decode: &Decode) -> Result<T>
    where
        T: JsonSchema + DeserializeOwned,
    {
        let mut schema = serde_json::to_value(schemars::schema_for!(T))
            .unwrap_or_else(|_| json!({ "type": "object" }));
        tighten_integer_bounds(&mut schema);
        let raw = self
            .complete(prompt, &Constraint::JsonSchema(schema), decode)
            .await?;
        parse(&raw)
    }

    /// Generate text under an explicit constraint, returning it unparsed.
    pub async fn complete(
        &self,
        prompt: &str,
        constraint: &Constraint,
        decode: &Decode,
    ) -> Result<String> {
        let mut body = json!({
            "prompt": prompt,
            "n_predict": decode.max_tokens,
            "temperature": decode.temperature,
            "cache_prompt": true,
        });

        match constraint {
            Constraint::JsonSchema(schema) => body["json_schema"] = schema.clone(),
            Constraint::Grammar(g) => body["grammar"] = json!(g),
        }
        if let Some(seed) = decode.seed {
            body["seed"] = json!(seed);
        }

        let response = self
            .http
            .post(format!("{}/completion", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|source| LlmError::Unreachable {
                base: self.base.clone(),
                source,
            })?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(LlmError::Server {
                status: status.as_u16(),
                // Bounded: a server error body can be enormous, and a log line nobody can
                // read is a log line nobody reads.
                body: text.chars().take(400).collect(),
            });
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|source| LlmError::Unparseable {
                raw: text.chars().take(400).collect(),
                source,
            })?;

        parsed
            .get("content")
            .and_then(|c| c.as_str())
            .map(str::to_owned)
            .ok_or(LlmError::Empty)
    }
}

/// Give bounded integer formats an explicit `maximum`.
///
/// `schemars` describes a `u32` as `{"type":"integer","format":"uint32","minimum":0}` —
/// the format carries the upper bound, but the schema does not state it. llama.cpp's
/// converter builds its grammar from `minimum`/`maximum` and ignores `format`, so the
/// sampler happily produces integers no `u32` can hold. `serde` then rejects them, and the
/// result surfaces as [`LlmError::Unparseable`] — which reads as "constrained decoding is
/// broken" when in fact the constraint was never told the real limit.
///
/// Found by measurement, not by reading: a 1.7B model returned `"order_limit":
/// 1000000000000000` for an `Option<u32>` in roughly a third of runs. A larger model
/// happened not to, which is exactly how a bug like this reaches production.
///
/// `docs/decisions/0002` names "tight numeric bounds" as the thing that would make
/// delegating the conversion the wrong call. This closes it centrally instead, so no
/// report type has to remember an annotation.
fn tighten_integer_bounds(value: &mut serde_json::Value) {
    // (format, inclusive maximum). i64/u64 exceed what an f64 can represent exactly, so
    // they are left alone: an imprecise bound would be worse than none.
    const BOUNDS: [(&str, u64); 6] = [
        ("uint8", u8::MAX as u64),
        ("uint16", u16::MAX as u64),
        ("uint32", u32::MAX as u64),
        ("int8", i8::MAX as u64),
        ("int16", i16::MAX as u64),
        ("int32", i32::MAX as u64),
    ];

    match value {
        serde_json::Value::Object(map) => {
            // Read the format out before mutating, so the immutable borrow ends first.
            let bound = map
                .get("format")
                .and_then(|f| f.as_str())
                .and_then(|format| {
                    BOUNDS
                        .iter()
                        .find(|(name, _)| *name == format)
                        .map(|(name, max)| (*name, *max))
                });

            if let Some((format, max)) = bound {
                map.entry("maximum").or_insert_with(|| json!(max));
                // Signed formats need the floor too; schemars omits it.
                if format.starts_with("int") {
                    let floor = match format {
                        "int8" => i64::from(i8::MIN),
                        "int16" => i64::from(i16::MIN),
                        _ => i64::from(i32::MIN),
                    };
                    map.entry("minimum").or_insert_with(|| json!(floor));
                }
            }
            for v in map.values_mut() {
                tighten_integer_bounds(v);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                tighten_integer_bounds(v);
            }
        }
        _ => {}
    }
}

/// Parse the model's output into `T`.
///
/// Trims first: the grammar constrains the JSON, not what follows it, and llama.cpp
/// happily emits trailing whitespace and newlines after the closing brace. Feeding that
/// straight to serde produces a "trailing characters" error that looks like the constraint
/// failed when it did not.
fn parse<T: DeserializeOwned>(raw: &str) -> Result<T> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LlmError::Empty);
    }
    serde_json::from_str(trimmed).map_err(|source| LlmError::Unparseable {
        raw: trimmed.chars().take(400).collect(),
        source,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct Tiny {
        name: String,
        count: u32,
    }

    #[test]
    fn trailing_whitespace_does_not_look_like_a_constraint_failure() {
        // Exactly what the server returns: valid JSON, then padding.
        let raw = "{\"name\": \"starter\", \"count\": 25}\n\n      ";
        let parsed: Tiny = parse(raw).expect("should parse after trimming");
        assert_eq!(parsed.name, "starter");
        assert_eq!(parsed.count, 25);
    }

    #[test]
    fn empty_output_is_reported_as_empty_not_as_a_parse_error() {
        // These need telling apart: empty means the server produced nothing, a parse error
        // means the constraint is not working. Conflating them hides the second.
        assert!(matches!(parse::<Tiny>("   \n "), Err(LlmError::Empty)));
    }

    #[test]
    fn a_genuine_shape_mismatch_keeps_the_raw_output() {
        let err = parse::<Tiny>("{\"name\": \"starter\"}").expect_err("missing field");
        match err {
            LlmError::Unparseable { raw, .. } => {
                assert!(
                    raw.contains("starter"),
                    "the raw output must be preserved for triage"
                );
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn the_schema_derives_from_the_type() {
        let schema = serde_json::to_value(schemars::schema_for!(Tiny)).expect("serialise");
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("schema has properties");
        assert!(props.contains_key("name") && props.contains_key("count"));
    }

    /// Exists only so `schema_for!` has something to describe. The fields are never read —
    /// their *types* are the whole point, and each one covers a different case:
    /// a plain bounded integer, one nested inside `Option`, a signed type needing both
    /// ends, and a width too large to bound safely.
    #[derive(Debug, Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Bounded {
        small: u32,
        maybe: Option<u32>,
        signed: i16,
        wide: u64,
    }

    #[test]
    fn a_u32_field_gains_the_maximum_its_type_implies() {
        // The bug this exists for: without a maximum, the grammar allows integers no u32
        // can hold, and the overflow surfaces as if the constraint had failed.
        let mut schema = serde_json::to_value(schemars::schema_for!(Bounded)).expect("schema");
        tighten_integer_bounds(&mut schema);

        let small = &schema["properties"]["small"];
        assert_eq!(small["maximum"], serde_json::json!(u32::MAX as u64));
        assert_eq!(small["minimum"], serde_json::json!(0.0));
    }

    #[test]
    fn an_optional_u32_is_bounded_too() {
        // Option<T> nests the type inside anyOf/allOf, so a walk that only looked at
        // top-level properties would miss exactly the field that caused the failure.
        let mut schema = serde_json::to_value(schemars::schema_for!(Bounded)).expect("schema");
        tighten_integer_bounds(&mut schema);
        let text = schema.to_string();
        let bounds = text.matches("\"maximum\"").count();
        assert!(
            bounds >= 3,
            "expected small, maybe and signed to be bounded; schema was {text}"
        );
    }

    #[test]
    fn a_signed_field_gains_both_ends() {
        let mut schema = serde_json::to_value(schemars::schema_for!(Bounded)).expect("schema");
        tighten_integer_bounds(&mut schema);
        let signed = &schema["properties"]["signed"];
        assert_eq!(signed["maximum"], serde_json::json!(i16::MAX as u64));
        assert_eq!(signed["minimum"], serde_json::json!(i64::from(i16::MIN)));
    }

    #[test]
    fn a_u64_is_left_alone() {
        // u64::MAX cannot be represented exactly as an f64, and JSON numbers are f64. An
        // imprecise bound would be worse than none: it would reject values that fit.
        let mut schema = serde_json::to_value(schemars::schema_for!(Bounded)).expect("schema");
        tighten_integer_bounds(&mut schema);
        assert!(
            schema["properties"]["wide"].get("maximum").is_none(),
            "u64 must not be given a lossy bound"
        );
    }

    #[test]
    fn an_existing_bound_is_never_overwritten() {
        // A type that declares its own range knows better than a format guess.
        let mut schema = serde_json::json!({
            "type": "integer", "format": "uint32", "maximum": 100
        });
        tighten_integer_bounds(&mut schema);
        assert_eq!(schema["maximum"], serde_json::json!(100));
    }

    #[test]
    fn base_url_trailing_slash_does_not_produce_a_double_slash() {
        assert_eq!(
            LlamaClient::new("http://127.0.0.1:8080/").base(),
            "http://127.0.0.1:8080"
        );
    }
}
