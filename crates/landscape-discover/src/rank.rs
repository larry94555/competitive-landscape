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
    /// A search engine returned it. **Ordered lowest deliberately**: every other variant is
    /// something the site itself did — answered a request, listed a path, nominated a page —
    /// and this one is a third party's opinion that a page exists and is relevant. Neither
    /// half of that has been checked when the candidate is made, which is why a page reached
    /// both ways keeps the probe.
    Search,
    /// An applicant-tracking board the subject's own page linked to.
    ///
    /// **Above search and below every on-domain route**, which is exactly where the evidence
    /// puts it: the company said this is where its roles are, so it is not a third party's
    /// opinion — but the bytes come from somebody else's server, so it is not the company's own
    /// page either. See [`crate::boards`], and [`landscape_core::Disposition::Attributed`],
    /// which is the standing it is read at.
    Board,
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
            Self::Search => "search",
            Self::Board => "board",
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
            // The page that names the question most directly, first — **which page it is
            // comes before how we found it**, the same rule locale preference already
            // follows. The coverage note is what showed this mattered: `linear.app/changelog`
            // answers 200 and was never read, because `/docs/releases.md` was named in
            // llms.txt and llms.txt outranked a probe. A documentation page about a feature
            // called Releases beat the changelog on provenance alone.
            crate::probes::specificity(&a.url)
                .cmp(&crate::probes::specificity(&b.url))
                .then_with(|| b.via.cmp(&a.via))
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
    room_for_a_board(&mut out, &by_question);
    out
}

/// Make room for a board the round-robin left out, by taking a slot from whichever question has
/// most.
///
/// **A board cannot win a slot on its own, and that is not a ranking bug.** Round one gives each
/// question its best page, and for hiring that is the company's own careers page — which is how
/// the board was found in the first place. So a board only ever competes for a *second* slot,
/// and by then the questions that sort earlier have taken them.
///
/// Both orderings are wrong for one of two real companies. `linear.app/careers` lists its roles,
/// so its board is worth less than its own page; `vercel.com/careers` is navigation chrome and
/// its roles are entirely on Greenhouse. **Which is which cannot be known before the page is
/// read**, so both are admitted rather than one guessed at, and the slot comes from the question
/// that already has the most — which is [ADR 0010](../../../docs/decisions/0010-spend-the-cap-on-breadth.md)'s
/// argument applied to itself: a second pricing page is the cheapest thing on the list, and a
/// board is the only place some companies' roles exist at all.
fn room_for_a_board(out: &mut Vec<Candidate>, by_question: &BTreeMap<Answers, Vec<Candidate>>) {
    let Some(board) = by_question
        .values()
        .flatten()
        .find(|c| c.via == Via::Board && !out.iter().any(|kept| kept.url == c.url))
    else {
        return;
    };

    // The question with the most slots, and only if it has one to spare. A run that gave every
    // question exactly one page has nothing that is cheaper than a board.
    let mut counts: BTreeMap<Answers, usize> = BTreeMap::new();
    for c in out.iter() {
        *counts.entry(c.answers).or_default() += 1;
    }
    let Some((&crowded, &most)) = counts.iter().max_by_key(|(_, &n)| n) else {
        return;
    };
    if most < 2 {
        return;
    }

    // `rposition` cannot be `None` here — `crowded` came from counting `out` — but saying so
    // with `expect` would be a panic in a library over an invariant a reader has to reconstruct.
    if let Some(last) = out.iter().rposition(|c| c.answers == crowded) {
        out.remove(last);
        out.push(board.clone());
    }
}

/// Two URLs that differ only by a trailing slash or a `www.` are one page.
///
/// Not a full canonicalisation — query strings and fragments are left alone, because on
/// some sites they genuinely select a different plan. This handles the duplicates the
/// probe list itself creates: `/pricing` and `/pricing/` are both in it.
///
/// **Public because a second channel needs the same answer.** `landscape-search` has to ask
/// *"is this URL one discovery already has?"*, and a second implementation of "same page"
/// would drift from this one the first time either learned something — which is the shape of
/// entry 4 in the mistakes register, one fact derived two ways.
/// **Only the scheme and host are lowercased.** They are case-insensitive by specification;
/// a path and a query are not, and plenty of servers serve `/Docs` and `/docs` as different
/// resources. Lowercasing the whole URL — which this did — merged them, and review caught it
/// when the function became the shared answer for two channels: search would have dropped a
/// page as *"discovery already has it"* when discovery had a different page.
#[must_use]
pub fn normalise(url: &str) -> String {
    let trimmed_input = url.trim();
    // Split at the end of the authority: the first `/`, `?` or `#` after `://`. Everything
    // before it is case-insensitive, everything after it is not.
    let (head, tail) = match trimmed_input.split_once("://") {
        Some((scheme, rest)) => {
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let (authority, remainder) = rest.split_at(end);
            (
                format!("{}://{}", scheme.to_lowercase(), authority.to_lowercase()),
                remainder.to_owned(),
            )
        }
        // Not shaped like a URL. Lowercasing nothing is the safe answer: this is a key for
        // comparing two strings, and two strings that are not URLs are equal when they match.
        None => (trimmed_input.to_owned(), String::new()),
    };

    let without_www = head.replacen("://www.", "://", 1) + &tail;
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
    fn a_board_takes_a_slot_from_whichever_question_has_most() {
        // **Round one gives hiring the company's own careers page**, which is how the board was
        // found, so a board only ever competes for a second slot — and the questions that sort
        // earlier take those. Both orderings are wrong for one of two real companies, so both
        // pages are admitted and the reading decides.
        let mut candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/plans", Answers::Pricing, Via::Probe),
            c("https://e.com/features", Answers::Features, Via::Probe),
            c("https://e.com/careers", Answers::Direction, Via::Probe),
            c("https://jobs.ashbyhq.com/e", Answers::Direction, Via::Board),
        ];
        candidates.reverse();

        let admitted = admit(candidates, 4);
        let urls: Vec<&str> = admitted.iter().map(|c| c.url.as_str()).collect();
        assert!(
            urls.contains(&"https://jobs.ashbyhq.com/e"),
            "the board never got a slot: {urls:?}"
        );
        assert!(
            urls.contains(&"https://e.com/careers"),
            "the page that named the board was dropped for it: {urls:?}"
        );
        assert_eq!(admitted.len(), 4, "the cap moved: {urls:?}");
        assert_eq!(
            urls.iter()
                .filter(|u| u.contains("pricing") || u.contains("plans"))
                .count(),
            1,
            "the slot came from somewhere other than the crowded question: {urls:?}"
        );
    }

    #[test]
    fn a_run_with_one_page_a_question_keeps_them_all() {
        // Nothing here is cheaper than a board, so nothing is given up for one. A cap spent
        // one-per-question is already breadth, which is what ADR 0010 spends it on.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/features", Answers::Features, Via::Probe),
            c("https://e.com/careers", Answers::Direction, Via::Probe),
            c("https://jobs.ashbyhq.com/e", Answers::Direction, Via::Board),
        ];
        let admitted = admit(candidates, 3);
        let urls: Vec<&str> = admitted.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(urls.len(), 3);
        assert!(!urls.contains(&"https://jobs.ashbyhq.com/e"), "{urls:?}");
        assert!(urls.contains(&"https://e.com/pricing"), "{urls:?}");
        assert!(urls.contains(&"https://e.com/features"), "{urls:?}");
    }

    #[test]
    fn a_board_the_round_robin_already_took_is_not_taken_twice() {
        // When the cap is generous enough for hiring to get two slots on its own, the board is
        // already in. Making room again would spend a second slot on the page that is in it,
        // and read the same board twice.
        let candidates = vec![
            c("https://e.com/pricing", Answers::Pricing, Via::Probe),
            c("https://e.com/plans", Answers::Pricing, Via::Probe),
            c("https://e.com/pricing-3", Answers::Pricing, Via::Sitemap),
            c("https://e.com/careers", Answers::Direction, Via::Probe),
            c("https://jobs.ashbyhq.com/e", Answers::Direction, Via::Board),
        ];
        let admitted = admit(candidates, 5);
        let urls: Vec<&str> = admitted.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            urls.iter()
                .filter(|u| **u == "https://jobs.ashbyhq.com/e")
                .count(),
            1,
            "the same board was admitted twice: {urls:?}"
        );
        assert_eq!(admitted.len(), 5, "{urls:?}");
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
    fn the_page_that_names_the_question_beats_the_better_attested_one() {
        // What the coverage note surfaced on linear.app: `/changelog` answered 200 and was
        // never read, because a documentation page about a feature called Releases was named
        // in llms.txt. Provenance is evidence about a page; it is not evidence that the page
        // is the right one.
        let candidates = vec![
            c(
                "https://e.com/docs/releases.md",
                Answers::Changes,
                Via::LlmsTxt,
            ),
            c("https://e.com/changelog", Answers::Changes, Via::Probe),
        ];
        let admitted = admit(candidates, 1);
        assert_eq!(admitted[0].url, "https://e.com/changelog");
    }

    #[test]
    fn the_changelog_outranks_the_blog() {
        // Both answer *what changed*, both came the same way, and the shorter URL used to
        // win. notion.com/blog holds no dated entry; /releases holds dozens.
        let candidates = vec![
            c("https://e.com/blog", Answers::Changes, Via::LlmsTxt),
            c("https://e.com/releases", Answers::Changes, Via::LlmsTxt),
        ];
        let admitted = admit(candidates, 1);
        assert_eq!(admitted[0].url, "https://e.com/releases");
    }

    #[test]
    fn a_case_distinct_path_is_a_different_page() {
        // Review's finding. The whole URL was lowercased, so `/Docs` and `/docs` keyed the
        // same — and once `landscape-search` started asking this function *"does discovery
        // already have this?"*, that merge became a page silently dropped as a duplicate of
        // a page it is not. Paths and queries are case-sensitive on plenty of servers.
        assert_ne!(
            normalise("https://e.com/Docs"),
            normalise("https://e.com/docs")
        );
        assert_ne!(
            normalise("https://e.com/p?Plan=Free"),
            normalise("https://e.com/p?plan=free")
        );
        // Two pages, not one, through the whole admission path.
        let candidates = vec![
            c("https://e.com/Docs", Answers::Features, Via::Probe),
            c("https://e.com/docs", Answers::Features, Via::Probe),
        ];
        assert_eq!(admit(candidates, 8).len(), 2);
    }

    #[test]
    fn the_scheme_and_host_are_still_case_insensitive() {
        // The half that must not regress while fixing the half above: a host is
        // case-insensitive by specification, so these are one page and always were.
        assert_eq!(
            normalise("HTTPS://WWW.Example.com/Pricing"),
            normalise("https://example.com/Pricing")
        );
        // And the path's case survived that, rather than being lowercased on the way.
        assert!(
            normalise("HTTPS://WWW.Example.com/Pricing").ends_with("/Pricing"),
            "{}",
            normalise("HTTPS://WWW.Example.com/Pricing")
        );
    }

    #[test]
    fn a_locale_is_still_stripped_from_a_mixed_case_path() {
        // `locale::is_locale` lowercases its own segment, so this keeps working — asserted
        // rather than assumed, because the normaliser no longer hands it lowercase input.
        assert_eq!(
            normalise("https://e.com/DE/Preise"),
            normalise("https://e.com/Preise")
        );
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
