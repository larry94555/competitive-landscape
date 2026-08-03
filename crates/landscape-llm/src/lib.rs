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
        let schema = serde_json::to_value(schemars::schema_for!(T))
            .unwrap_or_else(|_| json!({ "type": "object" }));
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

    #[test]
    fn base_url_trailing_slash_does_not_produce_a_double_slash() {
        assert_eq!(
            LlamaClient::new("http://127.0.0.1:8080/").base(),
            "http://127.0.0.1:8080"
        );
    }
}
