//! Choosing which eight pages to read.
//!
//! `FACT_CHECKING.md` §3.3 ends the discovery pipeline with **"admission control → rank →
//! cap at 8 (Rung 0) / 14 (Rung 2)"**, and the cap is not a performance detail. Each page
//! costs a second of per-host politeness and a model pass to extract from, against a
//! 90–180 second budget for the whole report.
//!
//! # The rule: spend the budget on different questions
//!
//! The obvious ranking is by confidence, and it produces a terrible eight. A site with
//! `/pricing`, `/plans` and `/pricing/` scores three near-identical pricing pages at the
//! top, and a report built from them can tell you the price three ways and nothing about
//! what changed last month.
//!
//! So candidates are taken **round-robin across the questions they answer** — the best
//! pricing page, then the best changelog, then the best feature page — before any question
//! gets a second entry. An eight covering six questions beats an eight covering two, and
//! the sections that would otherwise be empty are the ones a competitive report is
//! actually read for.
//!
//! This is the same shape as the "not found" treatment elsewhere: it is better to say
//! something about six things than everything about one.

use std::collections::BTreeMap;

use crate::probes::Answers;

/// Rung 0 — the free tier, and what the timings in `BENCHMARKS.md` are budgeted against.
pub const CAP_RUNG_0: usize = 8;

/// Rung 2 — a paid GPU box. Here so the number has a name rather than appearing later as
/// a magic 14.
pub const CAP_RUNG_2: usize = 14;

/// A page found by any route, waiting to be admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub url: String,
    pub answers: Answers,
    /// How the page was found. Higher is better evidence that it is the real thing.
    pub via: Via,
}

/// How a page came to our attention.
///
/// Ordered by how much it says about the page. A site listing a URL in `llms.txt` has
/// explicitly nominated it for automated readers; a probe that happened to return 200 has
/// told us only that the path exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// A path we guessed that answered.
    Probe,
    /// Listed in the site's own `sitemap.xml`.
    Sitemap,
    /// Named in the site's `llms.txt` — the site pointing at it deliberately.
    LlmsTxt,
}

impl Via {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Sitemap => "sitemap",
            Self::LlmsTxt => "llms.txt",
        }
    }
}

/// Admit, rank, and cap.
///
/// Duplicates are removed first — the same page reached by probe and by sitemap is one
/// page, and keeping both would spend two of eight slots on it.
#[must_use]
pub fn admit(candidates: Vec<Candidate>, cap: usize) -> Vec<Candidate> {
    let mut best_by_url: BTreeMap<String, Candidate> = BTreeMap::new();
    for c in candidates {
        let key = normalise(&c.url);
        best_by_url
            .entry(key)
            .and_modify(|existing| {
                // Which page first, then how well it is attested. A locale is a variant of a
                // page rather than a different page — Run 7 spent two slots on
                // `todoist.com/cs/pricing` and `/da/pricing` and never read the English one —
                // so the variant is chosen before provenance is considered. Between two
                // variants of equal standing, being named in llms.txt says more about a page
                // than a probe having found it.
                let better = (crate::locale::preference(&c.url), std::cmp::Reverse(c.via))
                    < (
                        crate::locale::preference(&existing.url),
                        std::cmp::Reverse(existing.via),
                    );
                if better {
                    *existing = c.clone();
                }
            })
            .or_insert(c);
    }

    // Group by question, each group ordered by provenance.
    let mut by_question: BTreeMap<Answers, Vec<Candidate>> = BTreeMap::new();
    for c in best_by_url.into_values() {
        by_question.entry(c.answers).or_default().push(c);
    }
    for group in by_question.values_mut() {
        group.sort_by(|a, b| {
            b.via
                .cmp(&a.via)
                .then_with(|| a.url.len().cmp(&b.url.len()))
        });
    }

    // Round-robin: one per question, then a second per question, and so on.
    let mut out = Vec::with_capacity(cap);
    let mut round = 0usize;
    while out.len() < cap {
        let mut took_any = false;
        for group in by_question.values() {
            if let Some(c) = group.get(round) {
                out.push(c.clone());
                took_any = true;
                if out.len() == cap {
                    break;
                }
            }
        }
        if !took_any {
            break;
        }
        round += 1;
    }
    out
}

/// Two URLs that differ only by a trailing slash or a `www.` are one page.
///
/// Not a full canonicalisation — query strings and fragments are left alone, because on
/// some sites they genuinely select a different plan. This handles the duplicates the
/// probe list itself creates: `/pricing` and `/pricing/` are both in it.
fn normalise(url: &str) -> String {
    let lowered = url.trim().to_lowercase();
    let without_www = lowered.replacen("://www.", "://", 1);
    // `/cs/pricing`, `/da/pricing` and `/pricing` are one page in three languages. Keying
    // them apart is what let one page take two of eight slots.
    let without_locale = crate::locale::stripped(&without_www);
    let trimmed = without_locale.trim_end_matches('/').to_owned();
    if trimmed.ends_with("://") {
        without_locale
    } else {
        trimmed
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn c(url: &str, answers: Answers, via: Via) -> Candidate {
        Candidate {
            url: url.to_owned(),
            answers,
            via,
        }
    }

    #[test]
    fn the_cap_is_spent_covering_different_questions() {
        // The point of the module. Ranking by confidence alone would take four pricing
        // pages and tell a reader the price four ways and nothing about anything else.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/plans", Answers::Pricing, Via::Probe),
            c("https://e.com/pricing-2", Answers::Pricing, Via::Sitemap),
            c("https://e.com/pricing-3", Answers::Pricing, Via::Sitemap),
            c("https://e.com/changelog", Answers::Changes, Via::Probe),
            c("https://e.com/security", Answers::Trust, Via::Probe),
        ];
        let admitted = admit(candidates, 3);
        let questions: Vec<Answers> = admitted.iter().map(|c| c.answers).collect();

        assert_eq!(admitted.len(), 3);
        assert!(questions.contains(&Answers::Changes), "{questions:?}");
        assert!(questions.contains(&Answers::Trust), "{questions:?}");
        assert_eq!(
            questions.iter().filter(|q| **q == Answers::Pricing).count(),
            1,
            "one question took more than its first slot: {questions:?}"
        );
    }

    #[test]
    fn a_second_page_per_question_is_taken_only_once_every_question_has_one() {
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/plans", Answers::Pricing, Via::Probe),
            c("https://e.com/changelog", Answers::Changes, Via::Probe),
        ];
        let admitted = admit(candidates, 3);
        assert_eq!(
            admitted.len(),
            3,
            "a spare slot should be filled, not wasted"
        );
    }

    #[test]
    fn the_same_page_found_twice_takes_one_slot() {
        // /pricing and /pricing/ are both in the probe list, so this is not hypothetical.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/pricing/", Answers::Pricing, Via::Probe),
            c("https://www.e.com/pricing", Answers::Pricing, Via::Sitemap),
        ];
        assert_eq!(admit(candidates, 8).len(), 1);
    }

    #[test]
    fn the_better_provenance_survives_deduplication() {
        // A page the site named in llms.txt is the same page, better attested.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/pricing", Answers::Pricing, Via::LlmsTxt),
        ];
        let admitted = admit(candidates, 8);
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].via, Via::LlmsTxt);
    }

    #[test]
    fn one_page_in_three_languages_takes_one_slot() {
        // Run 7 read todoist's pricing page in Czech and in Danish, and never in English.
        let candidates = vec![
            c(
                "https://todoist.com/cs/pricing",
                Answers::Pricing,
                Via::Sitemap,
            ),
            c(
                "https://todoist.com/da/pricing",
                Answers::Pricing,
                Via::Sitemap,
            ),
            c("https://todoist.com/pricing", Answers::Pricing, Via::Probe),
        ];
        let admitted = admit(candidates, 8);
        assert_eq!(admitted.len(), 1, "{admitted:#?}");
        // And the one kept is the page in no language at all, even though the Czech one was
        // better attested. Which page it is comes first; how we found it comes second.
        assert_eq!(admitted[0].url, "https://todoist.com/pricing");
    }

    #[test]
    fn a_site_that_only_publishes_in_one_language_is_still_read() {
        // The filter that would have been wrong. Some sites have no unlocalised path, and
        // `/de/preise` is then the only pricing page there is.
        let candidates = vec![c("https://e.de/de/preise", Answers::Pricing, Via::Sitemap)];
        assert_eq!(admit(candidates, 8).len(), 1);
    }

    #[test]
    fn english_wins_when_there_is_no_unlocalised_page() {
        let candidates = vec![
            c("https://e.com/da/pricing", Answers::Pricing, Via::LlmsTxt),
            c("https://e.com/en-us/pricing", Answers::Pricing, Via::Probe),
        ];
        let admitted = admit(candidates, 8);
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].url, "https://e.com/en-us/pricing");
    }

    #[test]
    fn a_query_string_is_not_treated_as_a_duplicate() {
        // On some sites ?plan=team genuinely selects a different page.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c(
                "https://e.com/pricing?plan=team",
                Answers::Pricing,
                Via::Sitemap,
            ),
        ];
        assert_eq!(admit(candidates, 8).len(), 2);
    }

    #[test]
    fn fewer_candidates_than_the_cap_are_all_admitted() {
        let candidates = vec![c("https://e.com/pricing", Answers::Pricing, Via::Probe)];
        assert_eq!(admit(candidates, CAP_RUNG_0).len(), 1);
    }

    #[test]
    fn nothing_in_yields_nothing_out_rather_than_looping() {
        assert!(admit(vec![], CAP_RUNG_0).is_empty());
        // A zero cap is not something the code should ever ask for, but the loop must not
        // depend on that being true.
        assert!(admit(vec![c("https://e.com/x", Answers::Pricing, Via::Probe)], 0).is_empty());
    }

    #[test]
    fn the_rung_0_cap_matches_what_the_latency_budget_assumes() {
        // BENCHMARKS.md budgets extractions against this number. If it changes, the
        // end-to-end latency estimate changes with it and should be re-derived.
        assert_eq!(CAP_RUNG_0, 8);
        // Compared through a binding so the assertion is about the constants rather than
        // being folded away as a literal comparison.
        let (rung_0, rung_2) = (CAP_RUNG_0, CAP_RUNG_2);
        assert!(rung_2 > rung_0, "rung 2 must read more, not less");
    }
}
