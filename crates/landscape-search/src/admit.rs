//! What a search result is allowed to become.
//!
//! A hit arrives as three strings from a machine we did not write, about a page nobody has
//! read. Three things happen to it here, and each exists because of a different way that
//! goes wrong.
//!
//! 1. **It is checked for being a URL at all** ([`landscape_fetch::Target::parse`]). An
//!    engine — misconfigured, compromised, or merely indexing something odd — can return
//!    `file:///etc/passwd` or a `javascript:` URL. The SSRF guard in [`landscape_fetch`]
//!    would refuse to fetch it; this refuses to *hold* it, so it never reaches a list a
//!    later change might loop over.
//! 2. **Its standing is decided from its host** ([`disposition_for`]), never from its
//!    position in the results. See below.
//! 3. **Its title and snippet are dropped.** [`Found`] has no field for either. They were
//!    the engine's prose about a page, and the only text that may reach a report is text
//!    quoted from a page we fetched ourselves.
//!
//! # Ranking and standing are different questions
//!
//! It is tempting to treat the first result as the best source. It is not a source at all
//! until it is fetched, and its rank was decided by an engine optimising for a search user,
//! not for a competitive analyst. So [`disposition_for`] looks only at the host: the
//! subject's own domain is [`Disposition::Primary`] — the company saying it, exactly as a
//! probe would have found — and everything else is [`Disposition::Unverified`], which
//! `FACT_CHECKING.md` §3.2.1 defines as *included by default and labelled*.
//!
//! **This is what search is for, and what it is not for.** Only a primary source may set a
//! value in a comparison table ([`Disposition::may_set_a_table_value`]). Search buys pages
//! to report beside the table; it cannot buy a cell inside it. A pricing figure found on an
//! aggregator stays a pricing figure found on an aggregator.
//!
//! # Deduplication is discovery's, not a copy of it
//!
//! [`landscape_discover::rank`] already answers *"are these two URLs one page"* — trailing
//! slash, `www.`, locale — and already spends a cap round-robin across questions so one
//! question cannot eat the budget. Both are reused rather than reimplemented, and hits are
//! carried as [`Candidate`]s with [`Via::Search`] so a page reached by both channels keeps
//! the probe.

use std::collections::HashMap;

use landscape_core::Disposition;
use landscape_discover::probes::Answers;
use landscape_discover::rank::{self, Candidate, Via};
use landscape_fetch::Target;

use crate::provider::Hit;
use crate::queries::Query;

/// A page a search returned, admitted.
///
/// **There is no `snippet` and no `title`.** That is the design, not an omission: the
/// engine's prose about a page must not be able to travel into a report by being present in
/// the type that gets there. Run 8 found a model reporting a feature it had read in a
/// prompt's worked example; the answer then was to delete the example rather than to
/// remember not to trust it, and the answer here is the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub url: String,
    /// The question this was searched for.
    pub answers: Answers,
    /// What it may be used for, decided by [`disposition_for`].
    pub disposition: Disposition,
    /// The exact query text that produced it, so a retrieval regression is attributable to
    /// a template rather than to a guess. `FACT_CHECKING.md` §3.3.
    pub query: String,
    /// Which provider answered — [`crate::SourceProvider::name`].
    pub engine: String,
}

impl Found {
    /// The same page, as something [`landscape_discover::rank`] can rank beside a probe.
    #[must_use]
    pub fn to_candidate(&self) -> Candidate {
        Candidate {
            url: self.url.clone(),
            answers: self.answers,
            via: Via::Search,
        }
    }
}

/// What a page found at `url` may be used for, given the subject's own host.
///
/// The subject's own domain — and its subdomains, because `docs.linear.app` is Linear
/// speaking — is [`Disposition::Primary`]. Everything else is [`Disposition::Unverified`].
///
/// A URL that is not a URL is [`Disposition::NotRead`], which is the honest label: we have
/// something that names a page and no way to reach it.
#[must_use]
pub fn disposition_for(subject_host: &str, url: &str) -> Disposition {
    let Ok(target) = Target::parse(url) else {
        return Disposition::NotRead;
    };
    if is_the_subject(subject_host, &target.host) {
        Disposition::Primary
    } else {
        Disposition::Unverified
    }
}

/// Whether `host` is the subject or one of its subdomains.
///
/// **The dot is load-bearing.** `host.ends_with(subject)` alone makes
/// `linear.app.attacker.test` the subject's own domain, and a page on it would then be
/// permitted to set a table value — the strongest thing this codebase grants any source,
/// handed to whoever registers the right name.
///
/// **An empty subject needs no guard of its own, and does not have one.** The first draft
/// had an `is_empty` early return; mutating it away changed no test and no behaviour,
/// because `host == ""` is false for every host [`Target::parse`] admits and no host
/// survives [`strip_www`] ending in a dot. Per `docs/mutations/README.md`, a guard nothing
/// needs is deleted rather than given a test that keeps it for ever — the mutation was
/// inverted instead, and now puts the *wrong* rule in.
fn is_the_subject(subject_host: &str, host: &str) -> bool {
    let subject = strip_www(subject_host);
    let host = strip_www(host);
    host == subject || host.ends_with(&format!(".{subject}"))
}

fn strip_www(host: &str) -> String {
    let lowered = host.trim().trim_end_matches('.').to_lowercase();
    lowered.strip_prefix("www.").unwrap_or(&lowered).to_owned()
}

/// Turn what the engines said into the pages worth reading.
///
/// `already` is every URL discovery has already found for this subject. A hit matching one
/// of them is dropped rather than admitted: the probe reached the page, and re-admitting it
/// through search would spend one of `cap` slots on a page already in the list.
///
/// `cap` is spent round-robin across questions by [`landscape_discover::rank::admit`], so
/// four hits for *pricing* cannot starve *changes*.
#[must_use]
pub fn admit(
    subject_host: &str,
    already: &[String],
    results: &[(Query, Vec<Hit>)],
    engine: &str,
    cap: usize,
) -> Vec<Found> {
    let known: Vec<String> = already.iter().map(|u| rank::normalise(u)).collect();

    let mut by_url: HashMap<String, Found> = HashMap::new();
    let mut candidates = Vec::new();
    for (query, hits) in results {
        for hit in hits {
            // Not a URL we fetch — refused before anything holds it.
            if Target::parse(&hit.url).is_err() {
                tracing::debug!(url = %hit.url, "a search result was not a URL we fetch");
                continue;
            }
            if known.contains(&rank::normalise(&hit.url)) {
                continue;
            }
            let found = Found {
                url: hit.url.clone(),
                answers: query.answers,
                disposition: disposition_for(subject_host, &hit.url),
                query: query.text.clone(),
                engine: engine.to_owned(),
            };
            // **First occurrence wins, and it wins in both maps or in neither.**
            //
            // This was `by_url.insert(...)`, which pushed a candidate only for the first
            // occurrence and then let every later one *overwrite the entry it was paired
            // with*. The same URL returned for pricing and then for trust left a candidate
            // ranked as pricing and a `Found` claiming trust, so a page came out of here
            // labelled with a question it was not found for, and quoting the wrong query.
            //
            // Review found it. The test that should have — the one named for exactly this —
            // used two different URLs, so nothing in it could ever collide. Entry 7 of the
            // register is the class: a value separated from its evidence.
            if let std::collections::hash_map::Entry::Vacant(slot) = by_url.entry(hit.url.clone()) {
                slot.insert(found.clone());
                candidates.push(found.to_candidate());
            }
        }
    }

    rank::admit(candidates, cap)
        .into_iter()
        .filter_map(|c| by_url.get(&c.url).cloned())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::queries;

    fn hit(url: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: "a title the engine wrote".to_owned(),
            snippet: "a snippet the engine assembled".to_owned(),
        }
    }

    fn results(answers: Answers, urls: &[&str]) -> Vec<(Query, Vec<Hit>)> {
        let query = queries::for_questions("Linear", &[answers])
            .into_iter()
            .next()
            .unwrap();
        vec![(query, urls.iter().map(|u| hit(u)).collect())]
    }

    #[test]
    fn a_page_on_the_subjects_own_domain_is_primary() {
        // The asymmetry that makes searching worth doing: the same page, found by search
        // rather than by a guessed path, is still the company speaking.
        assert_eq!(
            disposition_for("linear.app", "https://linear.app/changelog"),
            Disposition::Primary
        );
        assert_eq!(
            disposition_for("linear.app", "https://docs.linear.app/releases"),
            Disposition::Primary
        );
        // And www is the same site, in both directions.
        assert_eq!(
            disposition_for("www.linear.app", "https://linear.app/pricing"),
            Disposition::Primary
        );
    }

    #[test]
    fn somebody_elses_page_about_the_subject_is_unverified() {
        let d = disposition_for("linear.app", "https://www.g2.com/products/linear/reviews");
        assert_eq!(d, Disposition::Unverified);
        assert!(
            !d.may_set_a_table_value(),
            "an aggregator would be setting figures in a comparison table"
        );
    }

    #[test]
    fn a_host_that_merely_ends_with_the_subject_is_not_the_subject() {
        // The defect the dot prevents. Without it, whoever registers
        // `linear.app.attacker.test` is granted the strongest standing this codebase has.
        for host in [
            "https://linear.app.attacker.test/pricing",
            "https://notlinear.app/pricing",
            "https://xlinear.app/pricing",
        ] {
            assert_eq!(
                disposition_for("linear.app", host),
                Disposition::Unverified,
                "{host} was treated as the subject's own domain"
            );
        }
    }

    #[test]
    fn with_no_subject_nothing_is_the_subjects_own_domain() {
        // The other direction of the same failure: an empty subject making the whole web
        // primary is worse than an empty subject making none of it primary.
        assert_eq!(
            disposition_for("", "https://linear.app/pricing"),
            Disposition::Unverified
        );
    }

    #[test]
    fn a_result_that_is_not_a_url_we_fetch_is_never_admitted() {
        // An engine can return these, whether it was compromised or merely indexing
        // something odd. The SSRF guard would refuse the fetch; this refuses to carry it as
        // far as something that fetches.
        let found = admit(
            "linear.app",
            &[],
            &results(
                Answers::Changes,
                &[
                    "file:///etc/passwd",
                    "javascript:alert(1)",
                    "not a url at all",
                    "https://linear.app/changelog",
                ],
            ),
            "searxng",
            8,
        );
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].url, "https://linear.app/changelog");
    }

    #[test]
    fn a_page_discovery_already_found_does_not_take_a_second_slot() {
        // Search fills gaps. Re-admitting the probe's own page spends one of eight on a page
        // that is already in the list.
        let already = vec!["https://linear.app/changelog".to_owned()];
        let found = admit(
            "linear.app",
            &already,
            &results(
                Answers::Changes,
                &[
                    // The same page, spelled the three ways discovery already reconciles.
                    "https://linear.app/changelog/",
                    "https://www.linear.app/changelog",
                    "https://linear.app/releases",
                ],
            ),
            "searxng",
            8,
        );
        let urls: Vec<&str> = found.iter().map(|f| f.url.as_str()).collect();
        assert_eq!(urls, vec!["https://linear.app/releases"], "{found:#?}");
    }

    #[test]
    fn the_engines_snippet_does_not_survive_admission() {
        // Structural rather than a rule to remember: `Found` has nowhere to put it. The
        // assertion is on the rendered debug output, which is what a later change adding a
        // field would alter.
        let found = admit(
            "linear.app",
            &[],
            &results(Answers::Changes, &["https://linear.app/changelog"]),
            "searxng",
            8,
        );
        let rendered = format!("{found:?}");
        assert!(
            !rendered.contains("snippet the engine assembled"),
            "the engine's prose reached the admitted page: {rendered}"
        );
        assert!(
            !rendered.contains("title the engine wrote"),
            "the engine's prose reached the admitted page: {rendered}"
        );
    }

    #[test]
    fn every_admitted_page_says_which_query_found_it() {
        // `FACT_CHECKING.md` §3.3 wants retrieval attributable. A page with no query beside
        // it cannot be traced to the template that produced it.
        let found = admit(
            "linear.app",
            &[],
            &results(Answers::Changes, &["https://linear.app/changelog"]),
            "searxng",
            8,
        );
        assert_eq!(found[0].query, "\"Linear\" changelog OR \"release notes\"");
        assert_eq!(found[0].engine, "searxng");
        assert_eq!(found[0].answers, Answers::Changes);
    }

    #[test]
    fn one_question_cannot_spend_the_whole_cap() {
        // The reason discovery's ranking is reused rather than a `take(cap)` written here.
        let mut all = results(
            Answers::Pricing,
            &[
                "https://e.test/a",
                "https://e.test/b",
                "https://e.test/c",
                "https://e.test/d",
            ],
        );
        all.extend(results(Answers::Trust, &["https://e.test/security"]));
        let found = admit("linear.app", &[], &all, "searxng", 3);
        assert_eq!(found.len(), 3);
        assert!(
            found.iter().any(|f| f.answers == Answers::Trust),
            "the question with one hit was starved: {found:#?}"
        );
    }

    #[test]
    fn one_url_returned_for_two_questions_keeps_the_question_it_was_ranked_as() {
        // Review's finding, and the case the test below could not reach because it used two
        // different URLs. A search engine returns the same page for more than one query
        // routinely — a company's `/security` answers both "what is their posture" and "who
        // are they" — so this is the ordinary case rather than a contrived one.
        //
        // The defect it replaces: the candidate was built from the first occurrence and the
        // `Found` from the last, so the page came out ranked as pricing and labelled trust,
        // quoting a query that did not find it.
        let url = "https://linear.app/security";
        let mut all = results(Answers::Pricing, &[url]);
        all.extend(results(Answers::Trust, &[url]));

        let found = admit("linear.app", &[], &all, "searxng", 8);
        assert_eq!(found.len(), 1, "one page, one slot: {found:#?}");

        let only = &found[0];
        let expected = queries::for_questions("Linear", &[only.answers])[0]
            .text
            .clone();
        assert_eq!(
            only.query, expected,
            "the page is labelled {:?} and quotes a query for something else",
            only.answers
        );
        // And it is the first occurrence that survived, which is what was ranked.
        assert_eq!(only.answers, Answers::Pricing, "{only:#?}");
    }

    #[test]
    fn the_query_that_found_a_page_is_not_paired_with_a_different_page() {
        // Entry 7 of the register: a value separated from its evidence. Two questions, two
        // pages, and each must keep its own query through the ranking step — which reorders.
        let mut all = results(Answers::Pricing, &["https://e.test/pricing"]);
        all.extend(results(Answers::Trust, &["https://e.test/security"]));
        let found = admit("linear.app", &[], &all, "searxng", 8);
        for f in &found {
            let expected = queries::for_questions("Linear", &[f.answers])[0]
                .text
                .clone();
            assert_eq!(f.query, expected, "{f:#?} carries another question's query");
        }
    }

    #[test]
    fn nothing_found_is_an_empty_list_rather_than_a_failure() {
        assert!(admit("linear.app", &[], &[], "searxng", 8).is_empty());
        assert!(admit(
            "linear.app",
            &[],
            &results(Answers::Changes, &[]),
            "searxng",
            8
        )
        .is_empty());
    }
}
