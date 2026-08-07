//! Sizing the JavaScript-rendering gap, before deciding whether to build for it.
//!
//! [`ARCHITECTURE.md`] §5.5 describes an escalation ladder for pages that build themselves
//! in the browser, and refuses to schedule the expensive rung until it has been measured:
//!
//! > Phase 1 instruments two counters, because building tier 5 before knowing its size would
//! > be speculative work:
//! >
//! > 1. Of pricing pages fetched, what share yield **no price** from static HTML?
//! > 2. Of those, what share are recovered by **tiers 2–4**?
//! >
//! > **If the residual is under ~5%, tier 5 is not built.**
//!
//! This crate is those two counters. It is a **measuring instrument**, not part of the
//! analysis pipeline — the same category as `landscape-golden`, and held to the same rule:
//! an instrument nobody calibrated produces numbers that are worse than none, because they
//! get believed and then acted on.
//!
//! # Why the answer matters more than it sounds
//!
//! Tier 5 is a headless browser. `ARCHITECTURE.md` budgets ~400 MB peak for one at
//! concurrency 1, on a 24 GB box where three resident models already take ~17 GB. **Building
//! it means taking memory from a model.** That is the same trade [ADR 0005] refused for a
//! metrics stack, and it should be refused here too unless the residual justifies it.
//!
//! [`ARCHITECTURE.md`]: ../../../docs/ARCHITECTURE.md
//! [ADR 0005]: ../../../docs/decisions/0005-observability-on-a-24gb-box.md

pub mod assurance;
pub mod capability;
pub mod changes;
pub mod doc;
pub mod embedded;
pub mod identity;
pub mod markdown;
pub mod price;
pub mod quality;
pub mod span;
pub mod text;

/// Where a page's price was found, if anywhere.
///
/// Ordered by the ladder in §5.5. `Tier1` is a page that needs nothing from us; `None` is
/// the residual that would justify a browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Where {
    /// Tier 1 — visible in the static HTML. Most sites.
    Static { evidence: String },
    /// Tier 2 — in the bytes we already fetched, inside a script.
    Embedded {
        shape: embedded::Shape,
        evidence: String,
    },
    /// Neither. **This is the number the decision turns on.**
    ///
    /// Not necessarily a JS-rendered page: a plan that genuinely publishes no price lands
    /// here too, and on a pricing page that is a finding rather than a gap. The two are
    /// separated by hand when the sample is reviewed, which is why the evidence fields
    /// above exist and why [`Report`] keeps the URL.
    NotFound,
}

impl Where {
    #[must_use]
    pub const fn tier(&self) -> u8 {
        match self {
            Self::Static { .. } => 1,
            Self::Embedded { .. } => 2,
            Self::NotFound => 0,
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Static { .. } => "tier 1  static html".to_owned(),
            Self::Embedded { shape, .. } => format!("tier 2  {}", shape.name()),
            Self::NotFound => "none    no price found".to_owned(),
        }
    }
}

/// Look for a price the way the ladder says to: cheapest rung first.
#[must_use]
pub fn locate(html: &str) -> Where {
    let visible = text::visible(html);
    if let Some(found) = price::find(&visible) {
        return Where::Static {
            evidence: found.evidence,
        };
    }
    if let Some(found) = embedded::find(html) {
        return Where::Embedded {
            shape: found.shape,
            evidence: found.evidence,
        };
    }
    Where::NotFound
}

/// One page, measured.
#[derive(Debug, Clone)]
pub struct Reading {
    pub url: String,
    pub found: Where,
}

/// The two counters §5.5 asks for, plus what it takes to trust them.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub readings: Vec<Reading>,
    /// Pages that could not be fetched at all. Kept apart from pages with no price: a
    /// refused or unreachable page says nothing about JavaScript, and folding the two
    /// together would inflate the gap with our own failures.
    pub unreachable: Vec<(String, String)>,
}

impl Report {
    #[must_use]
    pub fn measured(&self) -> usize {
        self.readings.len()
    }

    /// Counter 1 — pages yielding no price from static HTML.
    #[must_use]
    pub fn no_static_price(&self) -> usize {
        self.readings
            .iter()
            .filter(|r| !matches!(r.found, Where::Static { .. }))
            .count()
    }

    /// Counter 2 — of those, how many tier 2 recovers.
    #[must_use]
    pub fn recovered_by_tier_2(&self) -> usize {
        self.readings
            .iter()
            .filter(|r| matches!(r.found, Where::Embedded { .. }))
            .count()
    }

    /// The share left over — the number the tier-5 decision turns on.
    #[must_use]
    pub fn residual(&self) -> f64 {
        if self.readings.is_empty() {
            return 0.0;
        }
        let left = self
            .readings
            .iter()
            .filter(|r| matches!(r.found, Where::NotFound))
            .count();
        #[allow(clippy::cast_precision_loss)]
        {
            left as f64 / self.readings.len() as f64
        }
    }

    /// A table meant to be pasted into `docs/BENCHMARKS.md`.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(out, "\n{:<52} where the price was", "page");
        let rule = "-".repeat(78);
        let _ = writeln!(out, "{rule}");
        for r in &self.readings {
            let _ = writeln!(out, "{:<52} {}", trim(&r.url, 50), r.found.label());
        }
        for (url, why) in &self.unreachable {
            let _ = writeln!(out, "{:<52} unreachable: {}", trim(url, 50), trim(why, 22));
        }
        let _ = writeln!(out, "{rule}");
        let _ = writeln!(
            out,
            "measured {}   no static price {}   of those, tier 2 recovered {}",
            self.measured(),
            self.no_static_price(),
            self.recovered_by_tier_2(),
        );
        let _ = writeln!(
            out,
            "residual {:.1}% of pages showed no price by either route",
            self.residual() * 100.0,
        );
        // Deliberately does NOT apply the 5% rule. The residual mixes two different things
        // - a page whose price needs JavaScript, and a page that publishes no price at all
        // - and ARCHITECTURE 5.5's threshold is about the first. Applying the rule to the
        // combined figure would schedule a headless browser to solve "contact sales", which
        // no browser can solve.
        //
        // The first real run had a residual of 10.7% and a JS gap of 3.6%: opposite sides
        // of the threshold. So the tool reports, and a human classifies.
        let _ = writeln!(
            out,
            "\nThat is not the tier-5 number yet. Classify each residual page by hand:"
        );
        let _ = writeln!(
            out,
            "    price needs JavaScript         -> counts toward the tier-5 decision"
        );
        let _ = writeln!(
            out,
            "    page publishes no price at all -> a finding, not a gap. Excluded"
        );
        let _ = writeln!(
            out,
            "Then apply ARCHITECTURE 5.5: under ~5% JS-gap, tier 5 is not built."
        );
        if !self.unreachable.is_empty() {
            let _ = writeln!(
                out,
                "{} page(s) unreachable, excluded from every figure above",
                self.unreachable.len()
            );
        }
        out
    }
}

fn trim(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_pricing_page_is_tier_1() {
        let html = "<body><h1>Pricing</h1><p>Grower — $49 per month</p></body>";
        assert_eq!(locate(html).tier(), 1);
    }

    #[test]
    fn a_page_whose_price_is_only_in_a_script_is_tier_2() {
        // The claim ARCHITECTURE §5.5 rests on. The visible text says nothing; the price is
        // in the bytes we already have.
        let html = r#"<body><div id="__next">Loading…</div>
          <script id="__NEXT_DATA__" type="application/json">
          {"props":{"plans":[{"price":"49.00"}]}}</script></body>"#;
        let found = locate(html);
        assert_eq!(found.tier(), 2, "{found:?}");
    }

    #[test]
    fn static_text_wins_even_when_a_script_also_has_a_price() {
        // Cheapest rung first. A page a reader can already read must never be counted as
        // needing recovery, or counter 1 is inflated.
        let html = r#"<body><p>$49 per month</p>
          <script type="application/ld+json">{"offers":{"price":49}}</script></body>"#;
        assert_eq!(locate(html).tier(), 1);
    }

    #[test]
    fn a_contact_sales_page_is_the_residual() {
        let html = "<body><h1>Enterprise</h1><p>Contact sales for pricing.</p></body>";
        assert_eq!(locate(html), Where::NotFound);
    }

    #[test]
    fn the_counters_are_the_ones_the_architecture_asks_for() {
        let report = Report {
            readings: vec![
                Reading {
                    url: "a".into(),
                    found: Where::Static {
                        evidence: "$49".into(),
                    },
                },
                Reading {
                    url: "b".into(),
                    found: Where::Static {
                        evidence: "$99".into(),
                    },
                },
                Reading {
                    url: "c".into(),
                    found: Where::Embedded {
                        shape: embedded::Shape::JsonLd,
                        evidence: "49".into(),
                    },
                },
                Reading {
                    url: "d".into(),
                    found: Where::NotFound,
                },
            ],
            unreachable: vec![],
        };
        assert_eq!(report.measured(), 4);
        // Counter 1: no static price — the tier-2 page and the residual one.
        assert_eq!(report.no_static_price(), 2);
        // Counter 2: of those, tier 2 recovered one.
        assert_eq!(report.recovered_by_tier_2(), 1);
        // Residual: one page in four.
        assert!((report.residual() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn unreachable_pages_are_excluded_rather_than_counted_as_gaps() {
        // Folding our own failures into the gap would inflate the number that decides
        // whether a browser gets built — and would do it in the direction of more work.
        let report = Report {
            readings: vec![Reading {
                url: "a".into(),
                found: Where::Static {
                    evidence: "$49".into(),
                },
            }],
            unreachable: vec![("b".into(), "timed out".into())],
        };
        assert_eq!(report.measured(), 1);
        assert!(report.residual() < f64::EPSILON);
        assert!(report.render().contains("excluded"));
    }

    #[test]
    fn an_empty_report_does_not_divide_by_zero() {
        let empty = Report::default();
        assert!(empty.residual() < f64::EPSILON);
        assert!(empty.render().contains("measured 0"));
    }

    #[test]
    fn the_report_refuses_to_apply_the_threshold_itself() {
        // The residual mixes "needs JavaScript" with "publishes no price", and the 5% rule
        // is only about the first. The first real run had a residual of 10.7% and a JS gap
        // of 3.6% - opposite sides of the threshold - so a tool applying the rule on its own
        // would have scheduled a headless browser to solve "contact sales".
        let over = Report {
            readings: vec![Reading {
                url: "a".into(),
                found: Where::NotFound,
            }],
            unreachable: vec![],
        };
        let rendered = over.render();
        assert!(rendered.contains("Classify each residual page by hand"));
        assert!(rendered.contains("a finding, not a gap"));
    }
}
