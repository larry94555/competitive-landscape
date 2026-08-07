//! Turning a description into the companies it might be about.
//!
//! `FACT_CHECKING.md` §3.1 puts this before everything: *"Nothing is fetched until the subject
//! is pinned down, because every downstream error inherits from getting this wrong."* Step 4 of
//! that list — the disambiguation gate — has existed in [`landscape_core::subject`] since Phase
//! 1, deliberately built before the thing that feeds it. **This is step 2, and it is what the
//! gate has been waiting for.**
//!
//! ```text
//! "privacy-friendly website analytics"
//!    └─> queries::for_idea        the reader's own words, once, and only here
//!         └─> SourceProvider      the same seam per-question search uses
//!              └─> group by host  one host is one company
//!                   └─> score     arithmetic over URLs, never a model's opinion
//!                        └─> landscape_core::subject::resolve
//! ```
//!
//! # The one place a reader's phrasing may reach an engine
//!
//! `FACT_CHECKING.md` P22 is explicit that retrieval uses **"templated queries from resolved
//! entities, not user phrasing"** — because a query carrying somebody's framing returns pages
//! that agree with it, and a report built on those is a mirror.
//!
//! That rule cannot apply here and the reason is not an exception to it: **there is no resolved
//! entity yet.** Finding one is what this module is for, and the description is the only thing
//! anybody has said. So the words go to an engine exactly once, to produce a *set of companies*
//! — never to produce a fact — and everything downstream is templated from the domain the gate
//! resolves to. The asymmetry is the point: a biased query can put the wrong company on a list
//! a reader then chooses from, and a reader can see that and pick differently. A biased query
//! against a resolved company puts a biased *fact* in a report, and nobody can see it.
//!
//! # Why the score is arithmetic and not a judgement
//!
//! [`landscape_core::subject::AMBIGUITY_MARGIN`] compares two scores and decides whether to ask
//! a reader. A score somebody cannot explain makes that decision unaccountable, so every input
//! to it here is a property of a **URL**:
//!
//! | Signal | Why it is evidence |
//! |---|---|
//! | How many of the queries returned this host | Agreement across differently-worded questions is the closest thing to corroboration available before anything is read |
//! | How shallow its shallowest URL is | A company's front page is `/` or one level down. A page *about* a company is four levels into somebody's blog |
//!
//! **Nothing here reads a title or a snippet.** [`Hit`] carries both so a person running the
//! diagnostic can see what came back, and [`crate::admit::Found`] drops them so engine prose
//! cannot reach a report. The same discipline applies to a score: an engine's summary of a page
//! is the engine's, and letting it move a company up a list a reader chooses from is the same
//! laundering with a shorter supply chain.
//!
//! # What a candidate is not
//!
//! It is **not named yet**. [`Found`] carries a host and a score and nothing a reader could
//! choose between, because the name and the sentence that tells two companies apart have to
//! come from the company's own front page rather than from an engine's title. That fetch is
//! [`describe`], and it is a separate step so the arithmetic above stays pure and testable.

use std::collections::HashMap;

use landscape_core::subject::Candidate;
use landscape_fetch::Target;

use crate::provider::{Hit, SourceProvider};
use crate::queries::Query;

/// The version of the idea query set, stamped on a run.
///
/// Separate from [`crate::QUERY_SET`] because they answer different questions and change for
/// different reasons: that one asks *what does this company publish about pricing*, this one
/// asks *which companies is this description about*. A single version covering both would move
/// for edits that could not affect the other, which is a version nobody can reason from.
pub const IDEA_QUERY_SET: &str = "2026-08-07.1";

/// The most companies worth putting in front of a reader.
///
/// The gate asks a reader to choose between candidates, and a list of twenty is not a question —
/// it is a search results page with our name on it. `PRODUCT_SPEC.md` §3 wants at most three
/// chips; this leaves a little room above that so the gate has something to reject.
pub const MAX_CANDIDATES: usize = 5;

/// How many of the shown candidates get their front page fetched.
///
/// Every one is a request against somebody's server before a reader has asked for anything, so
/// the number is small and stated. [`describe`] fetches in score order, so the ones a reader is
/// most likely to pick are the ones that get named.
pub const NAMED: usize = MAX_CANDIDATES;

/// Hosts that are pages *about* a market rather than companies in it.
///
/// **A closed list, and the same shape as the compliance standards the trust extractor reads:**
/// naming them is a decision about what this product knows, so it is here and reviewable rather
/// than a regular expression somewhere. `FACT_CHECKING.md` §3.2 puts listicles and forums at the
/// bottom of both axes, and P15 names *"alternatives"* content as the worst; the point here is
/// narrower than trust, though. A G2 category page is often an *excellent* way to find out which
/// companies exist. It is simply not one of them, and a report about `g2.com` is not the report
/// anybody asked for.
///
/// Matched on the registrable suffix, so `blog.medium.com` is excluded with `medium.com`.
const NOT_A_COMPANY: [&str; 24] = [
    "g2.com",
    "capterra.com",
    "getapp.com",
    "softwareadvice.com",
    "trustradius.com",
    "producthunt.com",
    "alternativeto.net",
    "slant.co",
    "sourceforge.net",
    "reddit.com",
    "news.ycombinator.com",
    "quora.com",
    "stackoverflow.com",
    "medium.com",
    "substack.com",
    "wordpress.com",
    "blogspot.com",
    "youtube.com",
    "vimeo.com",
    "linkedin.com",
    "facebook.com",
    "twitter.com",
    "x.com",
    "wikipedia.org",
];

/// One company a description might be about, before anybody has read its pages.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The registrable host, lowercased and without `www.`
    pub host: String,
    /// `0.0..=1.0`, from [`score`].
    pub confidence: f32,
    /// How many of the queries returned this host at all.
    pub agreed: usize,
    /// The shallowest URL seen on this host, which is the one worth fetching.
    pub shallowest: String,
}

/// The queries a description produces.
///
/// Three, and each asks the same thing a different way, because the whole score below rests on
/// *agreement between differently-worded questions*. One query cannot agree with anything.
///
/// A blank description yields none: interpolating one sends bare boilerplate to an engine, which
/// returns the internet.
#[must_use]
pub fn for_idea(description: &str) -> Vec<Query> {
    /// Interpolated with the reader's words, and nothing else in this codebase does that.
    const TEMPLATES: [&str; 3] = [
        r#"best {} software"#,
        r#"{} tools comparison"#,
        r#"{} vendors"#,
    ];

    let cleaned = crate::queries::safe_words(description);
    if cleaned.is_empty() {
        return Vec::new();
    }
    TEMPLATES
        .into_iter()
        .map(|template| Query {
            text: template.replacen("{}", &cleaned, 1),
            // Every question, because a candidate is not about one section. The field is on
            // `Query` for the per-question path and carries no meaning here.
            answers: landscape_discover::probes::Answers::Identity,
            template,
        })
        .collect()
}

/// Ask, group and score. One round trip per query, and none if the description is blank.
///
/// # Errors
/// Never. A query that fails is counted and the rest carry on — a search that did not complete
/// is a thinner candidate list, and returning nothing because one engine call timed out would
/// turn a degraded answer into no answer.
pub async fn suggest(engine: &dyn SourceProvider, description: &str) -> (Vec<Found>, usize) {
    let queries = for_idea(description);
    let asked = queries.len();
    let mut results: Vec<Vec<Hit>> = Vec::with_capacity(asked);
    let mut failures = 0usize;
    for query in &queries {
        match engine.search(query).await {
            Ok(hits) => results.push(hits),
            Err(e) => {
                tracing::warn!(query = %query.text, error = %e, "a candidate search did not complete");
                failures += 1;
            }
        }
    }
    (from_results(&results, asked), failures)
}

/// The pure half: hits in, scored companies out.
///
/// `asked` is how many queries were *sent*, not how many answered — a host that appeared in the
/// only query that came back has agreed with nothing, and scoring it as unanimous would turn an
/// engine outage into a confident wrong answer.
#[must_use]
pub fn from_results(results: &[Vec<Hit>], asked: usize) -> Vec<Found> {
    let mut by_host: HashMap<String, (usize, String)> = HashMap::new();
    for hits in results {
        // Per query, not per hit: a listicle host returning five pages for one query has said
        // one thing, and counting five would let volume stand in for agreement.
        let mut seen_this_query: Vec<String> = Vec::new();
        for hit in hits {
            let Ok(target) = Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            if is_not_a_company(&host) {
                continue;
            }
            let entry = by_host.entry(host.clone()).or_insert((0, hit.url.clone()));
            // **Agreement is counted once per query; the shallowest URL is the shallowest
            // anywhere.** These were one `continue` and the test for the second one failed:
            // a host's front page arriving after a deep page in the same result set was
            // skipped entirely, so `describe` would have fetched the blog post.
            if !seen_this_query.contains(&host) {
                seen_this_query.push(host);
                entry.0 += 1;
            }
            if depth(&hit.url) < depth(&entry.1) {
                entry.1 = hit.url.clone();
            }
        }
    }

    let mut found: Vec<Found> = by_host
        .into_iter()
        .map(|(host, (agreed, shallowest))| Found {
            confidence: score(agreed, asked, depth(&shallowest)),
            host,
            agreed,
            shallowest,
        })
        .collect();
    // Highest first, then by host so a tie is stable rather than however the map iterated —
    // a list that reorders between two runs of the same input is one nobody can reproduce.
    found.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.host.cmp(&b.host))
    });
    found.truncate(MAX_CANDIDATES);
    found
}

/// How much we believe a host is one of the companies a description is about.
///
/// **Agreement is most of it.** A host every query returned is the strongest thing available
/// before a page is read; a host one query returned is a guess. The shallowest URL adds a
/// little, because a front page is what a company puts at `/` and an article about a company is
/// several levels into somebody else's site.
///
/// The weights are a starting point and are labelled as one, exactly as
/// [`landscape_core::subject::AMBIGUITY_MARGIN`] is. What matters more than their values is that
/// both inputs are countable from a URL, so a reader asking *why is this first* gets an answer
/// rather than a shrug.
#[must_use]
pub fn score(agreed: usize, asked: usize, depth: usize) -> f32 {
    if asked == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are single digits; the cap is three queries and five candidates"
    )]
    let agreement = agreed.min(asked) as f32 / asked as f32;
    // `/` and `/product` are a company's own front matter; `/blog/2024/05/a-review-of-x` is a
    // page about one. Two levels is the whole of the bonus, and it can never outweigh agreement.
    let shallow = match depth {
        0 | 1 => 0.2,
        2 => 0.1,
        _ => 0.0,
    };
    (agreement * 0.8 + shallow).clamp(0.0, 1.0)
}

/// Give each candidate the name and the line a reader picks between.
///
/// **From the company's own front page, never from the engine's title.** A disambiguation chip
/// is the one place a reader is asked to choose, and choosing between three summaries written by
/// a search engine is choosing between an engine's opinions. `fetch` returns the Markdown of a
/// URL, so this is the same conversion every extractor reads.
///
/// A host whose front page cannot be read keeps its host as its name and says so, rather than
/// being dropped: *"we could not read this one"* is a thing a reader can act on, and a candidate
/// silently missing from a list is not.
pub async fn describe<F, Fut>(found: &[Found], fetch: F) -> Vec<Candidate>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut out = Vec::with_capacity(found.len().min(NAMED));
    for one in found.iter().take(NAMED) {
        let page = fetch(one.shallowest.clone()).await;
        let (name, what_it_is) = page.map_or_else(
            || {
                (
                    one.host.clone(),
                    "we were unable to read its front page".to_owned(),
                )
            },
            |markdown| naming(&one.host, &markdown),
        );
        out.push(Candidate {
            name,
            canonical_domain: one.host.clone(),
            what_it_is,
            confidence: one.confidence,
        });
    }
    out
}

/// A page's own name for itself, and the first line that says what it is.
///
/// The heading is the name; the first sentence of prose under it is the distinguisher. Both come
/// from the page, so a reader choosing between two candidates is reading what each company says
/// about itself rather than what an engine said about them.
fn naming(host: &str, markdown: &str) -> (String, String) {
    let mut lines = markdown.lines().map(str::trim).filter(|l| !l.is_empty());
    let name = lines
        .by_ref()
        .find_map(|line| {
            let text = line.trim_start_matches('#').trim();
            (line.starts_with('#') && !text.is_empty()).then(|| text.to_owned())
        })
        .unwrap_or_else(|| host.to_owned());

    /// Long enough to distinguish, short enough for a chip.
    const MOST: usize = 140;
    let what = lines
        .find(|line| !line.starts_with('#') && line.split_whitespace().count() >= 4)
        .map_or_else(String::new, |line| {
            let mut trimmed: String = line.chars().take(MOST).collect();
            if line.chars().count() > MOST {
                trimmed.push('…');
            }
            trimmed
        });
    (name, what)
}

/// The registrable host: lowercased, no `www.`
fn registrable(host: &str) -> String {
    let lowered = host.trim().trim_end_matches('.').to_lowercase();
    lowered.strip_prefix("www.").unwrap_or(&lowered).to_owned()
}

/// Whether this host is a place people talk about companies rather than a company.
///
/// Suffix-matched on a label boundary, so `blog.medium.com` is excluded and `mediumroast.com` is
/// not — the same whole-token rule the trust scanner needed, for the same reason.
fn is_not_a_company(host: &str) -> bool {
    NOT_A_COMPANY
        .iter()
        .any(|known| host == *known || host.ends_with(&format!(".{known}")))
}

/// How many path segments a URL has. `https://a.com/` is 0.
fn depth(url: &str) -> usize {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .split_once('/')
        .map_or("", |(_, path)| path)
        .split(['?', '#'])
        .next()
        .unwrap_or("");
    path.split('/').filter(|s| !s.is_empty()).count()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::provider::SearchError;

    fn hit(url: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: "a title the engine wrote".to_owned(),
            snippet: "a snippet the engine wrote".to_owned(),
        }
    }

    #[test]
    fn a_description_becomes_three_differently_worded_queries() {
        let queries = for_idea("privacy-friendly website analytics");
        assert_eq!(queries.len(), 3);
        for q in &queries {
            assert!(
                q.text.contains("privacy-friendly website analytics"),
                "{}",
                q.text
            );
        }
        // Differently worded, because the score rests on agreement between them and three
        // copies of one question agree with themselves.
        let texts: Vec<&str> = queries.iter().map(|q| q.text.as_str()).collect();
        assert_eq!(
            texts.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "{texts:?}"
        );
    }

    #[test]
    fn a_blank_description_asks_nothing() {
        // Interpolating an empty description sends `best  software` to an engine, which
        // returns the internet.
        assert!(for_idea("").is_empty());
        assert!(for_idea("   ").is_empty());
    }

    #[test]
    fn a_host_every_query_returned_outranks_one_that_appeared_once() {
        let results = vec![
            vec![hit("https://a.com/"), hit("https://b.com/")],
            vec![hit("https://a.com/pricing")],
            vec![hit("https://a.com/")],
        ];
        let found = from_results(&results, 3);
        assert_eq!(found[0].host, "a.com");
        assert_eq!(found[0].agreed, 3);
        assert!(found[0].confidence > found[1].confidence, "{found:#?}");
    }

    #[test]
    fn one_query_returning_a_host_five_times_has_said_one_thing() {
        // Volume is not agreement. A listicle host with five pages in one result set would
        // otherwise outscore a company two separate queries both found.
        let results = vec![vec![
            hit("https://a.com/one"),
            hit("https://a.com/two"),
            hit("https://a.com/three"),
            hit("https://a.com/four"),
            hit("https://a.com/five"),
        ]];
        let found = from_results(&results, 3);
        assert_eq!(found[0].agreed, 1, "{found:#?}");
    }

    #[test]
    fn a_review_site_is_not_a_company() {
        // `FACT_CHECKING.md` §3.2 puts these at the bottom of both axes. The point here is
        // narrower: a report about g2.com is not the report anybody asked for.
        let results = vec![vec![
            hit("https://www.g2.com/categories/web-analytics"),
            hit("https://old.reddit.com/r/analytics/comments/abc"),
            hit("https://usefathom.com/"),
        ]];
        let found = from_results(&results, 1);
        let hosts: Vec<&str> = found.iter().map(|f| f.host.as_str()).collect();
        assert_eq!(hosts, vec!["usefathom.com"]);
    }

    #[test]
    fn a_company_whose_name_contains_a_review_site_is_still_a_company() {
        // Suffix-matched on a label boundary. `mediumroast.com` is not `medium.com`, and the
        // substring version of this rule would delete a real company from the list.
        assert!(is_not_a_company("medium.com"));
        assert!(is_not_a_company("blog.medium.com"));
        assert!(!is_not_a_company("mediumroast.com"));
        assert!(!is_not_a_company("notmedium.com"));
    }

    #[test]
    fn www_and_the_bare_host_are_one_company() {
        let results = vec![
            vec![hit("https://www.a.com/")],
            vec![hit("https://a.com/pricing")],
        ];
        let found = from_results(&results, 2);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].agreed, 2);
    }

    #[test]
    fn the_shallowest_url_is_the_one_kept() {
        // It is the one `describe` fetches, and a company's front page is what says what the
        // company is. A blog post four levels down says what one writer thinks.
        let results = vec![vec![
            hit("https://a.com/blog/2026/05/a-review"),
            hit("https://a.com/"),
        ]];
        let found = from_results(&results, 1);
        assert_eq!(found[0].shallowest, "https://a.com/");
    }

    #[test]
    fn a_query_that_never_answered_does_not_make_a_host_unanimous() {
        // Two of three queries failed. The host appeared in the one that answered, and
        // scoring it against one would call an engine outage a confident answer.
        let results = vec![vec![hit("https://a.com/")]];
        let found = from_results(&results, 3);
        assert!(
            found[0].confidence < 0.5,
            "an outage produced confidence: {found:#?}"
        );
    }

    #[test]
    fn agreement_outweighs_a_shallow_url() {
        // A front page found once must not outrank a company every query agreed on, however
        // deep the latter's shallowest page happened to be.
        let unanimous_but_deep = score(3, 3, 4);
        let shallow_but_alone = score(1, 3, 0);
        assert!(
            unanimous_but_deep > shallow_but_alone,
            "{unanimous_but_deep} vs {shallow_but_alone}"
        );
    }

    #[test]
    fn a_score_is_never_outside_the_range_the_gate_compares() {
        for agreed in 0..6 {
            for asked in 0..4 {
                for depth in 0..6 {
                    let s = score(agreed, asked, depth);
                    assert!((0.0..=1.0).contains(&s), "{agreed}/{asked}/{depth} = {s}");
                }
            }
        }
    }

    #[test]
    fn nothing_is_scored_when_nothing_was_asked() {
        assert!((score(3, 0, 0) - 0.0).abs() < f32::EPSILON);
        assert!(from_results(&[], 0).is_empty());
    }

    #[test]
    fn the_list_a_reader_sees_is_capped() {
        let many: Vec<Hit> = (0..12)
            .map(|i| hit(&format!("https://company{i}.com/")))
            .collect();
        let found = from_results(&[many], 1);
        assert_eq!(found.len(), MAX_CANDIDATES);
    }

    #[test]
    fn candidates_that_tie_come_back_in_the_same_order_every_time() {
        // A `HashMap` iterates in whatever order it likes. A list that reorders under a reader
        // is one nobody can reproduce, and the gate's margin compares the first two — so which
        // two those are cannot be luck.
        //
        // **Eight tying hosts, not two.** The first version of this test used two, and the
        // mutation that deletes the tie-break survived: with two entries the map's order agreed
        // with the sorted one often enough to pass. Eight makes agreement by chance a one-in-
        // forty-thousand event rather than a coin flip.
        let hosts = ["h", "c", "a", "g", "b", "e", "d", "f"];
        let results = vec![hosts
            .iter()
            .map(|h| hit(&format!("https://{h}.com/")))
            .collect::<Vec<_>>()];
        let found = from_results(&results, 1);

        let got: Vec<&str> = found.iter().map(|f| f.host.as_str()).collect();
        // Every one ties on score, so the whole order is the tie-break, and the cap then keeps
        // the first five of it.
        assert_eq!(
            got,
            vec!["a.com", "b.com", "c.com", "d.com", "e.com"],
            "{found:#?}"
        );
        assert_eq!(found, from_results(&results, 1), "two runs, two answers");
    }

    #[tokio::test]
    async fn a_candidate_is_named_by_its_own_page() {
        let found = vec![Found {
            host: "usefathom.com".to_owned(),
            confidence: 0.9,
            agreed: 3,
            shallowest: "https://usefathom.com/".to_owned(),
        }];
        let described = describe(&found, |_url| async {
            Some(
                "# Fathom Analytics\nSimple, privacy-first website analytics with no cookies."
                    .to_owned(),
            )
        })
        .await;
        assert_eq!(described[0].name, "Fathom Analytics");
        assert_eq!(
            described[0].what_it_is,
            "Simple, privacy-first website analytics with no cookies."
        );
        assert_eq!(described[0].canonical_domain, "usefathom.com");
    }

    #[tokio::test]
    async fn a_candidate_we_could_not_read_says_so_rather_than_vanishing() {
        // Dropping it would shorten a list a reader is choosing from, with nothing saying one
        // was removed — the silent-truncation failure this project keeps deleting.
        let found = vec![Found {
            host: "unreachable.example".to_owned(),
            confidence: 0.5,
            agreed: 1,
            shallowest: "https://unreachable.example/".to_owned(),
        }];
        let described = describe(&found, |_url| async { None }).await;
        assert_eq!(described.len(), 1);
        assert_eq!(described[0].name, "unreachable.example");
        assert!(described[0].what_it_is.contains("unable to read"));
    }

    #[tokio::test]
    async fn no_more_front_pages_are_fetched_than_the_number_that_is_stated() {
        // `from_results` already caps its list, so this is a guard on *this function's* own
        // contract rather than a second copy of that one: `describe` is public, and a caller
        // handing it twenty hosts would put twenty requests on twenty servers before a reader
        // had asked for anything.
        let many: Vec<Found> = (0..8)
            .map(|i| Found {
                host: format!("c{i}.example"),
                confidence: 0.5,
                agreed: 1,
                shallowest: format!("https://c{i}.example/"),
            })
            .collect();
        let fetched = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let counter = std::sync::Arc::clone(&fetched);
        let described = describe(&many, move |_url| {
            let counter = std::sync::Arc::clone(&counter);
            async move {
                *counter.lock().unwrap() += 1;
                Some(
                    "# A company
It does a thing for other companies."
                        .to_owned(),
                )
            }
        })
        .await;
        assert_eq!(described.len(), NAMED);
        assert_eq!(*fetched.lock().unwrap(), NAMED);
    }

    #[test]
    fn a_front_page_with_no_prose_still_names_the_company() {
        let (name, what) = naming("a.com", "# Acme\n## Pricing\n");
        assert_eq!(name, "Acme");
        assert!(what.is_empty(), "{what:?}");
    }

    #[test]
    fn a_front_page_with_no_heading_falls_back_to_the_host() {
        let (name, _) = naming("a.com", "Just some words with no heading at all here.");
        assert_eq!(name, "a.com");
    }

    /// A provider that answers from a list, so the round trips can be exercised with no
    /// network. The real engine has never been run against this — Docker was unavailable where
    /// this was built — which is the same limit `landscape search` carries and is stated in
    /// [BENCHMARKS.md](../../../docs/BENCHMARKS.md) Run 28 rather than left to be discovered.
    struct Canned {
        per_query: Vec<Result<Vec<Hit>, ()>>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl SourceProvider for Canned {
        fn name(&self) -> &str {
            "canned"
        }
        async fn search(&self, query: &Query) -> Result<Vec<Hit>, SearchError> {
            let mut asked = self.asked.lock().unwrap();
            let answer = self.per_query.get(asked.len()).cloned();
            asked.push(query.text.clone());
            match answer {
                Some(Ok(hits)) => Ok(hits),
                _ => Err(SearchError::Unreachable("no route to host".to_owned())),
            }
        }
    }

    #[tokio::test]
    async fn every_query_is_asked_and_the_hosts_are_grouped_across_them() {
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://usefathom.com/"), hit("https://g2.com/x")]),
                Ok(vec![hit("https://usefathom.com/pricing")]),
                Ok(vec![hit("https://plausible.io/")]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, failures) = suggest(&engine, "privacy-friendly website analytics").await;
        assert_eq!(failures, 0);
        assert_eq!(engine.asked.lock().unwrap().len(), 3, "one round trip each");
        assert_eq!(found[0].host, "usefathom.com");
        assert_eq!(found[0].agreed, 2, "{found:#?}");
        assert!(
            found.iter().all(|f| f.host != "g2.com"),
            "a review site reached the list: {found:#?}"
        );
    }

    #[tokio::test]
    async fn a_query_that_did_not_complete_is_counted_and_the_rest_carry_on() {
        // A search failure is not an analysis failure, and it is also not nothing: a thinner
        // list is a different thing from a shorter market, and only the count can say which.
        let engine = Canned {
            per_query: vec![Ok(vec![hit("https://a.com/")]), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, failures) = suggest(&engine, "anything at all").await;
        assert_eq!(failures, 2);
        assert_eq!(found.len(), 1);
        // Scored against three asked, not one answered. An outage is not unanimity.
        assert!(found[0].confidence < 0.5, "{found:#?}");
    }

    #[tokio::test]
    async fn a_blank_description_asks_no_engine_anything() {
        let engine = Canned {
            per_query: Vec::new(),
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, failures) = suggest(&engine, "   ").await;
        assert!(found.is_empty());
        assert_eq!(failures, 0);
        assert!(engine.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn a_description_carrying_searxng_control_tokens_is_disarmed() {
        // The freest text this system accepts, arriving from a stranger's text box. SearXNG
        // splits `q` on whitespace and reads `!!` as *redirect to the first result* before
        // anything is searched for, so the grammar in `queries` is what stands between a
        // description and somebody else's page.
        let queries = for_idea("analytics !! :fr <99 !google");
        assert_eq!(queries.len(), 3);
        for q in &queries {
            assert!(!q.text.contains("!!"), "{}", q.text);
            assert!(!q.text.contains(":fr"), "{}", q.text);
            assert!(!q.text.contains('<'), "{}", q.text);
            assert!(q.text.contains("analytics"), "{}", q.text);
        }
    }

    #[test]
    fn depth_counts_path_segments_and_ignores_the_query() {
        assert_eq!(depth("https://a.com"), 0);
        assert_eq!(depth("https://a.com/"), 0);
        assert_eq!(depth("https://a.com/pricing"), 1);
        assert_eq!(depth("https://a.com/blog/2026/05/post"), 4);
        assert_eq!(depth("https://a.com/pricing?utm=x"), 1);
    }
}
