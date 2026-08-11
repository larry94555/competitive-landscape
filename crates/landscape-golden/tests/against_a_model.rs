//! Score a running model against the golden set.
//!
//! `#[ignore]`d, so `cargo test` stays green on a laptop with no model.
//!
//! ```text
//! cargo test -p landscape-golden -- --ignored --nocapture
//! LLAMA_URL=http://127.0.0.1:8082 cargo test -p landscape-golden -- --ignored --nocapture
//! ```
//!
//! `--nocapture` is the point: the scorecard is the output, and a pass/fail bit throws
//! away everything worth reading.
//!
//! # What this asserts, and what it only reports
//!
//! It **fails on one thing**: a price returned for a plan whose page publishes none.
//! Everything else — wrong answers, misses, fabricated billing periods, quotes that are not
//! on the page — is printed and not asserted.
//!
//! That asymmetry is deliberate, and it is narrower than it first looks for a reason.
//! Accuracy thresholds belong in a nightly job comparing runs over time; a hard floor in a
//! test that any contributor runs against any model they happen to have loaded would fail
//! constantly and be switched off within a week. A test that fails for a reason the reader
//! considers negotiable teaches them to ignore it.
//!
//! An invented price is not negotiable. It is a false, checkable, confident claim about a
//! named company, published under our name, which the reader has no way to catch — and it
//! is the one failure that makes the product worse than not existing. That earns a red test
//! on anybody's hardware.
//!
//! `GOLDEN_MAX_FABRICATIONS` raises the bar to explore a model known to fabricate, without
//! editing the test.
//!
//! Measured 2026-08-03 (see `docs/BENCHMARKS.md`): Qwen3-4B passes at zero. Qwen3-1.7B
//! Q8_0 returns one — it answers about a neighboring plan on the same page — so this test
//! is **expected to be red against the 1.7B**. That is the finding, not a broken test.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Instant;

use landscape_core::PricingExtraction;
use landscape_golden::{load, prompt_for, score, Scorecard, SubjectResult};
use landscape_llm::{Decode, LlamaClient};

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
         llama-server -hf Qwen/Qwen3-1.7B-GGUF:Q8_0 --host 127.0.0.1 --port 8080",
        client.base()
    );
    None
}

#[tokio::test]
#[ignore = "needs a running llama-server; see the module docs"]
async fn the_model_does_not_invent_prices() {
    let Some(client) = server_or_skip().await else {
        return;
    };

    let subjects = load().expect("the golden set must load");
    let decode = Decode {
        max_tokens: 300,
        temperature: 0.0,
        // Fixed, so a failure can be re-run and looked at rather than shrugged at. The
        // constrained-decoding test deliberately varies its seed to prove the constraint
        // holds regardless; here the answer is what is being measured, so the run is
        // pinned instead.
        seed: Some(7),
    };

    let mut card = Scorecard {
        label: client.base().to_owned(),
        results: Vec::new(),
        errors: Vec::new(),
    };

    for subject in &subjects {
        let started = Instant::now();
        let got: Result<PricingExtraction, _> =
            client.generate(&prompt_for(subject), &decode).await;
        let ms = started.elapsed().as_millis();

        match got {
            Ok(extraction) => card.results.push(score(subject, &extraction, ms)),
            // An error is not a wrong answer, and merging the two would let a server that
            // is simply down look like a model that abstains perfectly.
            Err(e) => card.errors.push((subject.id.clone(), e.to_string())),
        }
    }

    println!("{}", card.render());
    print_fabrications(&subjects, &card);

    assert!(
        card.errors.is_empty(),
        "{} subject(s) did not produce an extraction at all",
        card.errors.len()
    );

    let allowed: usize = std::env::var("GOLDEN_MAX_FABRICATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Only invented *prices* fail the run. Every other verdict is printed above and is
    // worth reading, but a dollar figure attached to a company that never published one
    // is the failure this product cannot survive, and it is the one worth a red test on
    // anybody's hardware.
    assert!(
        card.invented_prices() <= allowed,
        "{} subject(s) got a price the page does not publish. A report that does this is \
         worse than no report: the reader cannot tell which figures to check.",
        card.invented_prices()
    );
}

/// Print what came back and why the subject is in the set, so a red run is actionable
/// from its own output rather than from a hand re-run that never happens.
fn print_fabrications(subjects: &[landscape_golden::Subject], card: &Scorecard) {
    let bad: Vec<&SubjectResult> = card.results.iter().filter(|r| r.fabricated()).collect();
    if bad.is_empty() {
        return;
    }
    println!("fabrications in detail:");
    for r in bad {
        let Some(s) = subjects.iter().find(|s| s.id == r.id) else {
            continue;
        };
        println!("\n  {}", s.id);
        println!(
            "    expected  price {:?}, period {:?}",
            s.expect.price_usd, s.expect.billing_period
        );
        println!(
            "    returned  price {:?}, period {:?}",
            r.got.price_usd, r.got.billing_period
        );
        if let Some(q) = &r.got.evidence_quote {
            println!("    quote     {:?}", trim_to(q, 100));
        }
        println!("    why       {}", s.why);
    }
}

fn trim_to(s: &str, n: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= n {
        return flat;
    }
    flat.chars().take(n).collect::<String>() + "…"
}
