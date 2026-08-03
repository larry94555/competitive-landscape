//! The Phase 0 exit criterion: grammar-constrained JSON, 0 parse failures over 100 runs.
//!
//! From `docs/ROADMAP.md`: *"Prove GBNF constrained decoding end-to-end: Rust struct →
//! schemars JSON Schema → GBNF → llama-server → parsed back into the struct. This is the
//! spine of the product; validate it before anything else is built on it."*
//!
//! Needs a running `llama-server`. `#[ignore]`d so `cargo test` stays green without one.
//!
//! ```text
//! llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080
//! cargo test -p landscape-llm -- --ignored --nocapture
//! ```
//!
//! `--nocapture` is worth it: the run prints per-call latency, which is the first real
//! timing data this project has and feeds the benchmark work directly.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Instant;

use landscape_llm::{Decode, LlamaClient};
use schemars::JsonSchema;
use serde::Deserialize;

/// Deliberately shaped like real extraction: a string, a number, an integer, an optional,
/// and an enum. The enum is the interesting one — nothing but constrained decoding stops a
/// model inventing a sixth variant.
#[derive(Debug, Deserialize, JsonSchema)]
struct PricingFact {
    plan_name: String,
    price_usd: f64,
    billing_period: BillingPeriod,
    order_limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BillingPeriod {
    Monthly,
    Yearly,
    OneOff,
}

/// One case: the prompt, and what a correct extraction contains.
struct Case {
    prompt: String,
    plan: &'static str,
    price: f64,
}

/// Varied inputs, so the run measures the constraint rather than a warm prompt cache.
fn cases() -> Vec<Case> {
    let plans = [
        ("Starter", 39, "month", "25 orders"),
        ("Grower", 49, "month", "unlimited orders"),
        ("Pro", 89, "month", "no order limit"),
        ("Team", 240, "year", "500 orders"),
        ("Lifetime", 799, "one-off payment", "no limit"),
    ];
    let framings = [
        "Pricing page text",
        "Copied from their site",
        "From the plans table",
        "Marketing page excerpt",
    ];
    let mut out = Vec::new();
    for (i, (name, price, period, limit)) in plans.iter().cycle().take(100).enumerate() {
        let framing = framings[i % framings.len()];
        out.push(Case {
            prompt: format!(
                "{framing}: The {name} plan costs ${price} per {period} and includes {limit}.\n\
                 Extract the pricing as JSON."
            ),
            plan: name,
            price: f64::from(*price),
        });
    }
    out
}

/// A reachable server, or `None` — having said clearly why not.
///
/// `#[ignore]` is not enough on its own. CI runs `cargo test -- --ignored` to pick up the
/// Postgres conformance and README tests, and that sweeps these up too. A missing model
/// server then fails a job that has nothing to do with models — which is what happened, and
/// it is a bad failure because it says "the tests are broken" when nothing is.
///
/// So absence skips by default. Set `LLAMA_REQUIRED=1` to make it a hard failure instead:
/// a job that deliberately provides a model server should not silently pass when the server
/// dies. Skipping quietly in *that* case would be the same mistake in the other direction.
async fn server_or_skip() -> Option<LlamaClient> {
    let client = LlamaClient::from_env();
    if client.is_ready().await {
        return Some(client);
    }
    assert!(
        std::env::var("LLAMA_REQUIRED").is_err(),
        "LLAMA_REQUIRED is set, but no llama-server is reachable at {}",
        client.base()
    );
    eprintln!(
        "SKIPPED: no llama-server at {}. Start one and re-run, or set LLAMA_URL:\n  \
         llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080",
        client.base()
    );
    None
}

#[tokio::test]
#[ignore = "needs a running llama-server; see the module docs"]
async fn one_hundred_constrained_generations_all_parse() {
    let Some(client) = server_or_skip().await else {
        return;
    };

    let decode = Decode {
        max_tokens: 160,
        temperature: 0.1,
        // Varied, so a lucky seed cannot carry the whole run.
        seed: None,
    };

    let mut failures: Vec<String> = Vec::new();
    let mut latencies_ms: Vec<u128> = Vec::new();
    let mut wrong_content = 0_u32;

    let cases = cases();
    let total = cases.len();

    for (n, case) in cases.into_iter().enumerate() {
        let started = Instant::now();
        let result: std::result::Result<PricingFact, _> =
            client.generate(&case.prompt, &decode).await;
        latencies_ms.push(started.elapsed().as_millis());

        match result {
            Ok(fact) => {
                // The schema guarantees the shape. It does not guarantee the content, and
                // conflating the two would let a nonsense extraction count as a pass. These
                // are counted and reported, not asserted: shape is the exit criterion here,
                // and extraction accuracy is what the golden set is for.
                let plan_ok = fact
                    .plan_name
                    .to_lowercase()
                    .contains(&case.plan.to_lowercase());
                let price_ok = (fact.price_usd - case.price).abs() < 0.01;
                // An order limit is present in every case, so a None means it was dropped.
                let limit_ok = fact.order_limit.is_some()
                    || case.prompt.contains("unlimited")
                    || case.prompt.contains("no order limit")
                    || case.prompt.contains("no limit");

                if !plan_ok || !price_ok || !limit_ok {
                    wrong_content += 1;
                }
            }
            Err(e) => failures.push(format!("run {n}: {e}")),
        }

        if (n + 1) % 20 == 0 {
            println!(
                "  {}/{total} done, {} parse failures",
                n + 1,
                failures.len()
            );
        }
    }

    latencies_ms.sort_unstable();
    let median = latencies_ms[latencies_ms.len() / 2];
    let p95 = latencies_ms[latencies_ms.len() * 95 / 100];
    let total_s = latencies_ms.iter().sum::<u128>() as f64 / 1000.0;

    println!("\n  runs                 {total}");
    println!("  parse failures       {}", failures.len());
    println!("  content mismatches   {wrong_content}  (accuracy, not shape - see the golden set)");
    println!("  median latency       {median} ms");
    println!("  p95 latency          {p95} ms");
    println!("  wall clock           {total_s:.1} s");

    assert!(
        failures.is_empty(),
        "constrained decoding produced {} unparseable outputs out of {total}. \
         The exit criterion is zero — a 1% failure rate means most reports lose a value.\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A constrained enum cannot produce a variant that is not in the type.
///
/// Separate from the volume test because it isolates the single property most worth having:
/// without it, every enum in the report schema needs defensive re-mapping downstream.
#[tokio::test]
#[ignore = "needs a running llama-server"]
async fn the_model_cannot_invent_an_enum_variant() {
    let Some(client) = server_or_skip().await else {
        return;
    };

    let decode = Decode {
        max_tokens: 160,
        temperature: 0.9, // High on purpose: give it every chance to wander.
        seed: None,
    };

    // A period that is deliberately none of the three variants.
    for n in 0..10 {
        let fact: PricingFact = client
            .generate(
                "Pricing page: The Weekly plan costs $12 per WEEK with 5 orders. \
                 Extract the pricing as JSON.",
                &decode,
            )
            .await
            .unwrap_or_else(|e| panic!("run {n} failed to parse: {e}"));

        // Whatever it decides "per week" maps to, it must be one of ours — which is
        // guaranteed by the type existing at all. The point is that it parsed.
        assert!(matches!(
            fact.billing_period,
            BillingPeriod::Monthly | BillingPeriod::Yearly | BillingPeriod::OneOff
        ));
    }
}
