//! Measures what a `llama-server` can actually do, on the hardware it is running on.
//!
//! `docs/ROADMAP.md` Phase 0: *"Benchmark harness, run on the actual A1 instance — laptop
//! numbers are worthless here."* This is that harness. It runs on a laptop too; the results
//! are simply labelled as what they are.
//!
//! ```text
//! landscape-bench                          the default sweep against LLAMA_URL
//! landscape-bench --runs 20                fewer runs, for a quick look
//! landscape-bench --shape span             only the realistic-extraction shape
//! landscape-bench --label "A1 Q4_K_M"      what to call this run in the output
//! ```
//!
//! **The number to watch is prefill, not generation.** On four ARM cores prefill dominates,
//! and the span-pre-selection design in `ARCHITECTURE.md` §5.4 lives or dies on it. A
//! harness that only reported tokens-per-second of *output* would measure the half that
//! matters less.

use std::time::Instant;

use anyhow::{Context, Result};
use landscape_llm::{Decode, LlamaClient};
use schemars::JsonSchema;
use serde::Deserialize;

/// What a real extraction returns: a few scalars and a constrained enum.
#[derive(Debug, Deserialize, JsonSchema)]
struct PricingFact {
    plan_name: String,
    price_usd: f64,
    billing_period: BillingPeriod,
    order_limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum BillingPeriod {
    Monthly,
    Yearly,
    OneOff,
}

/// The shapes of work the product actually does.
///
/// Named rather than parameterised by token count, because the point is to measure the
/// two cases the architecture is built around, not to draw a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// One sentence in, one small object out. What the router tier mostly sees.
    Sentence,
    /// A ~400-token span window in — the realistic extraction case, and the one that
    /// decides whether span pre-selection is affordable.
    Span,
}

impl Shape {
    const fn name(self) -> &'static str {
        match self {
            Self::Sentence => "sentence",
            Self::Span => "span (~400 tokens in)",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "sentence" => Some(Self::Sentence),
            "span" => Some(Self::Span),
            _ => None,
        }
    }
}

/// Filler that looks like a real page rather than repeated tokens.
///
/// A prompt of `"a a a a"` compresses in the KV cache in ways real prose does not, and
/// would flatter the prefill number — which is the number this exists to measure.
const PAGE: &str = "Our platform connects independent growers with restaurant kitchens in \
their region. Orders are placed the evening before, consolidated by route, and delivered \
the following morning. Kitchens see live availability from every farm they follow, and \
growers see committed demand before they harvest. Invoicing is handled per delivery, with \
statements issued monthly. Support is available by email on weekdays. We work with farms \
of every size, from single-field market gardens to multi-site operations, and with \
kitchens from one-room cafes to hotel groups. Deliveries run six days a week across the \
region, and route planning accounts for both distance and refrigeration windows. ";

/// A prompt, and what a correct extraction of it contains.
struct Case {
    prompt: String,
    plan: &'static str,
    price: f64,
}

fn prompt_for(shape: Shape, n: usize) -> Case {
    let plans = [
        ("Starter", 39, "month", "25 orders"),
        ("Grower", 49, "month", "unlimited orders"),
        ("Pro", 89, "month", "no order limit"),
        ("Team", 240, "year", "500 orders"),
    ];
    let (name, price, period, limit) = plans[n % plans.len()];
    let fact = format!("The {name} plan costs ${price} per {period} and includes {limit}.");

    let prompt = match shape {
        Shape::Sentence => format!("Pricing page: {fact}\nExtract the pricing as JSON."),
        Shape::Span => {
            // Roughly 400 tokens of surrounding page, with the fact buried in it — which
            // is what a real span window looks like after pre-selection.
            let filler = PAGE.repeat(3);
            format!(
                "Pricing page excerpt:\n{filler}\n{fact}\n{filler}\nExtract the pricing as JSON."
            )
        }
    };
    Case {
        prompt,
        plan: name,
        price: f64::from(price),
    }
}

#[derive(Debug)]
struct Stats {
    shape: Shape,
    runs: usize,
    /// The constraint failed: output that does not fit the type. This is the number ADR
    /// 0002 says to watch, and it must not be diluted by counting transport problems
    /// alongside it — an earlier version of this harness did, and reported a healthy
    /// server's timeouts as evidence that constrained decoding was broken.
    unparseable: usize,
    /// The server returned nothing, or could not be reached. An operational problem.
    transport: usize,
    /// Parsed, but did not contain what the prompt said. Separate from `failures`:
    /// a shape guarantee is not an accuracy guarantee, and reporting them as one
    /// number would let a nonsense extraction count as a success.
    wrong: usize,
    latencies_ms: Vec<u128>,
}

impl Stats {
    fn percentile(&self, p: usize) -> u128 {
        if self.latencies_ms.is_empty() {
            return 0;
        }
        let i = (self.latencies_ms.len() * p / 100).min(self.latencies_ms.len() - 1);
        self.latencies_ms[i]
    }

    fn report(&self, label: &str) {
        let total_s = self.latencies_ms.iter().sum::<u128>() as f64 / 1000.0;
        println!("\n  {label} — {}", self.shape.name());
        println!("  {:-<58}", "");
        println!("  runs                {}", self.runs);
        println!(
            "  unparseable         {}   (the constraint failing)",
            self.unparseable
        );
        println!(
            "  transport errors    {}   (the server, not the constraint)",
            self.transport
        );
        println!(
            "  wrong contents      {}   (accuracy, not shape)",
            self.wrong
        );
        println!("  median              {} ms", self.percentile(50));
        println!("  p95                 {} ms", self.percentile(95));
        println!("  slowest             {} ms", self.percentile(100));
        println!("  wall clock          {total_s:.1} s");

        // The figure the roadmap actually needs: how many of these fit in one analysis.
        let median_s = self.percentile(50) as f64 / 1000.0;
        if median_s > 0.0 {
            println!(
                "  fits in 120s        {:.0} extractions   (before any fetching)",
                120.0 / median_s
            );
        }
    }
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let runs: usize = arg(&args, "--runs")
        .as_deref()
        .map_or(Ok(30), str::parse)
        .context("--runs takes a number")?;
    let label = arg(&args, "--label").unwrap_or_else(|| "unlabelled run".to_owned());

    let shapes: Vec<Shape> = match arg(&args, "--shape") {
        None => vec![Shape::Sentence, Shape::Span],
        Some(s) => vec![Shape::parse(&s).context("--shape is 'sentence' or 'span'")?],
    };

    let client = LlamaClient::from_env();
    if !client.is_ready().await {
        anyhow::bail!(
            "no llama-server at {}.\n\
             Start one, or point LLAMA_URL somewhere else:\n  \
             llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080",
            client.base()
        );
    }

    println!("landscape-bench — {label}");
    println!("server: {}", client.base());
    println!(
        "\nNote: these numbers describe THIS machine. Compare them with each other -\n\
         the ratios travel, the seconds do not. How long a user waits is measured from\n\
         a client against a deployment, never from inside one."
    );

    let decode = Decode {
        max_tokens: 160,
        temperature: 0.1,
        seed: None,
    };

    for shape in shapes {
        let mut stats = Stats {
            shape,
            runs,
            unparseable: 0,
            transport: 0,
            wrong: 0,
            latencies_ms: Vec::with_capacity(runs),
        };

        for n in 0..runs {
            let case = prompt_for(shape, n);
            let started = Instant::now();
            let result: std::result::Result<PricingFact, _> =
                client.generate(&case.prompt, &decode).await;
            stats.latencies_ms.push(started.elapsed().as_millis());

            match result {
                Err(landscape_llm::LlmError::Unparseable { raw, source }) => {
                    stats.unparseable += 1;
                    eprintln!(
                        "  unparseable on run {n}: {source}
    raw: {raw:.120}"
                    );
                }
                Err(e) => {
                    stats.transport += 1;
                    eprintln!("  transport error on run {n}: {e:.160}");
                }
                Ok(fact) => {
                    let plan_ok = fact
                        .plan_name
                        .to_lowercase()
                        .contains(&case.plan.to_lowercase());
                    let price_ok = (fact.price_usd - case.price).abs() < 0.01;
                    let period_ok = matches!(
                        fact.billing_period,
                        BillingPeriod::Monthly | BillingPeriod::Yearly | BillingPeriod::OneOff
                    );
                    // A limit is stated in every case, so None means it was dropped -
                    // except where the page says there is no limit.
                    let limit_ok = fact.order_limit.is_some()
                        || case.prompt.contains("unlimited")
                        || case.prompt.contains("no order limit");
                    if !plan_ok || !price_ok || !period_ok || !limit_ok {
                        stats.wrong += 1;
                    }
                }
            }
            if (n + 1) % 10 == 0 {
                println!("  {}/{runs} {}", n + 1, shape.name());
            }
        }

        stats.latencies_ms.sort_unstable();
        stats.report(&label);
    }

    println!("\nRecord these in docs/BENCHMARKS.md, with the hardware they came from.");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;

    #[test]
    fn the_span_shape_is_substantially_larger_than_the_sentence_shape() {
        // If these were similar sizes the benchmark would be measuring one thing twice,
        // and the prefill question - the one that matters - would go unanswered.
        let sentence = prompt_for(Shape::Sentence, 0).prompt;
        let span = prompt_for(Shape::Span, 0).prompt;
        assert!(
            span.len() > sentence.len() * 10,
            "span {} chars vs sentence {} - too close to distinguish prefill cost",
            span.len(),
            sentence.len()
        );
    }

    #[test]
    fn the_span_filler_is_prose_not_repetition() {
        // Repeated single tokens compress in ways real pages do not, which would flatter
        // the prefill number this harness exists to measure.
        let span = prompt_for(Shape::Span, 0).prompt;
        let words: std::collections::HashSet<&str> = span.split_whitespace().collect();
        assert!(
            words.len() > 60,
            "filler has only {} distinct words; it should read like a page",
            words.len()
        );
    }

    #[test]
    fn every_prompt_still_contains_the_fact_to_extract() {
        // A benchmark whose prompts do not contain the answer measures refusal latency.
        for n in 0..8 {
            for shape in [Shape::Sentence, Shape::Span] {
                let p = prompt_for(shape, n).prompt;
                assert!(p.contains('$'), "{shape:?} run {n} has no price in it");
                assert!(
                    p.contains("as JSON"),
                    "{shape:?} run {n} has no instruction"
                );
            }
        }
    }

    #[test]
    fn prompts_vary_across_runs_so_the_cache_cannot_carry_the_result() {
        let a = prompt_for(Shape::Sentence, 0).prompt;
        let b = prompt_for(Shape::Sentence, 1).prompt;
        assert_ne!(a, b, "identical prompts would measure the prompt cache");
    }

    #[test]
    fn percentiles_do_not_run_off_the_end() {
        let s = Stats {
            shape: Shape::Sentence,
            runs: 3,
            unparseable: 0,
            transport: 0,
            wrong: 0,
            latencies_ms: vec![10, 20, 30],
        };
        assert_eq!(s.percentile(50), 20);
        assert_eq!(
            s.percentile(100),
            30,
            "p100 must index the last element, not past it"
        );
    }

    #[test]
    fn an_empty_run_reports_zero_rather_than_panicking() {
        let s = Stats {
            shape: Shape::Span,
            runs: 0,
            unparseable: 0,
            transport: 0,
            wrong: 0,
            latencies_ms: Vec::new(),
        };
        assert_eq!(s.percentile(50), 0);
    }

    #[test]
    fn shape_names_round_trip() {
        assert_eq!(Shape::parse("sentence"), Some(Shape::Sentence));
        assert_eq!(Shape::parse("span"), Some(Shape::Span));
        assert_eq!(Shape::parse("nonsense"), None);
    }
}
