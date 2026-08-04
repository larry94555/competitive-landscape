//! The paths worth guessing, and what each one answers.
//!
//! `FACT_CHECKING.md` §3.3 puts structured probes ahead of search, and gives the reason:
//!
//! > **Structured probes before search.** They are deterministic, cost nothing, hit primary
//! > sources, and are far more reliable than hoping a search engine surfaces the pricing
//! > page. Search fills gaps; it does not lead.
//!
//! Every path here is on the subject's **own domain**, which makes everything found through
//! them [`Disposition::Primary`] — the only class permitted to set a value in a comparison
//! table.
//!
//! # Each probe names the question it answers
//!
//! Not decoration. The cap in [`crate::rank`] spends its budget on **covering different
//! questions** rather than on the highest-scoring pages, and it can only do that if a probe
//! knows what it is for. Three pricing pages and nothing else is a worse eight than one page
//! each for eight sections.
//!
//! [`Disposition::Primary`]: landscape_core::Disposition

/// What a page is expected to tell us. Maps onto report sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Answers {
    /// What it costs.
    Pricing,
    /// What it does.
    Features,
    /// What changed recently — the freshness signal a competitive report lives on.
    Changes,
    /// Who they are, where, how big.
    Identity,
    /// Reliability and security posture, both of which are competitive facts.
    Trust,
    /// Hiring, which is the cheapest public signal of where a company is investing.
    Direction,
}

impl Answers {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pricing => "pricing",
            Self::Features => "features",
            Self::Changes => "changes",
            Self::Identity => "identity",
            Self::Trust => "trust",
            Self::Direction => "direction",
        }
    }
}

/// One path to try.
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub path: &'static str,
    pub answers: Answers,
    /// Tried in ascending order. Lower runs first.
    ///
    /// Order matters more than it looks: each probe costs a second of the per-host delay,
    /// so a subject with a 90-second budget cannot afford all of these. The cheap, common,
    /// high-yield ones go first so that stopping early stops on the least valuable.
    pub priority: u8,
}

/// The probe list, in the order `FACT_CHECKING.md` §3.3 gives them, prioritised.
///
/// Deliberately short. Every entry is a request against somebody else's server, and a list
/// that tries forty paths on the chance one exists is not the polite crawler this project
/// committed to being.
pub const PROBES: [Probe; 14] = [
    // Pricing first: it is the single most-wanted fact and the most reliably at a
    // guessable path.
    Probe {
        path: "/pricing",
        answers: Answers::Pricing,
        priority: 0,
    },
    Probe {
        path: "/plans",
        answers: Answers::Pricing,
        priority: 1,
    },
    Probe {
        path: "/pricing/",
        answers: Answers::Pricing,
        priority: 2,
    },
    // What it does.
    Probe {
        path: "/features",
        answers: Answers::Features,
        priority: 1,
    },
    Probe {
        path: "/product",
        answers: Answers::Features,
        priority: 2,
    },
    Probe {
        path: "/docs",
        answers: Answers::Features,
        priority: 3,
    },
    // What changed. A dated changelog is the strongest freshness signal available.
    Probe {
        path: "/changelog",
        answers: Answers::Changes,
        priority: 1,
    },
    Probe {
        path: "/releases",
        answers: Answers::Changes,
        priority: 2,
    },
    Probe {
        path: "/blog",
        answers: Answers::Changes,
        priority: 3,
    },
    // Who they are.
    Probe {
        path: "/about",
        answers: Answers::Identity,
        priority: 2,
    },
    // Trust posture — both are competitive facts, and both are usually at these paths.
    Probe {
        path: "/security",
        answers: Answers::Trust,
        priority: 3,
    },
    Probe {
        path: "/status",
        answers: Answers::Trust,
        priority: 4,
    },
    // Where the money is going. Cheapest public signal there is.
    Probe {
        path: "/careers",
        answers: Answers::Direction,
        priority: 4,
    },
    Probe {
        path: "/jobs",
        answers: Answers::Direction,
        priority: 5,
    },
];

/// The probes to run, cheapest and most valuable first.
#[must_use]
pub fn in_order() -> Vec<Probe> {
    let mut probes = PROBES.to_vec();
    probes.sort_by_key(|p| (p.priority, p.path));
    probes
}

/// Which question a discovered URL looks like it answers.
///
/// Used for URLs that came from a sitemap or `llms.txt` rather than from a probe, where
/// nothing tells us what the page is for except its path.
#[must_use]
pub fn guess(path: &str) -> Option<Answers> {
    // Defensive: callers pass a path, but a full URL or a query string reaching here would
    // silently classify nothing rather than fail, which is the worst kind of wrong.
    let p = path.split(['?', '#']).next().unwrap_or("").to_lowercase();
    // The check is on path segments, not substrings — but a segment may be hyphenated,
    // and that is where the judgement is. `release-notes` is a page name;
    // `11.5-personify-your-product` is an article slug that happens to end in a word we
    // recognise, and treating it as a product page put a book chapter in a real run.
    //
    // So a hyphenated segment is split only when it is **short**. Page names are two or
    // three words; article slugs are five or more.
    const MAX_WORDS_IN_A_PAGE_NAME: usize = 3;
    let mut segments: Vec<&str> = Vec::new();
    for segment in p.split('/').filter(|s| !s.is_empty()) {
        let words: Vec<&str> = segment
            .split(['-', '_', '.'])
            .filter(|w| !w.is_empty())
            .collect();
        if words.len() <= MAX_WORDS_IN_A_PAGE_NAME {
            segments.extend(words);
        } else {
            segments.push(segment);
        }
    }
    let has = |w: &str| segments.contains(&w);

    // A page you *do* something on is not a page that states a fact. `todoist.com/cs/pricing/
    // setup` and `/cs/pricing/upgrade` are both in its sitemap, both classify as pricing on
    // the word `pricing`, and both are steps in buying rather than publications of a price —
    // between them they took two of the five slots that run admitted.
    if segments.iter().any(|s| is_transactional(s)) {
        return None;
    }

    if has("pricing") || has("plans") || has("price") {
        return Some(Answers::Pricing);
    }
    if has("changelog") || has("releases") || has("release") || has("whatsnew") {
        return Some(Answers::Changes);
    }
    if has("security") || has("trust") || has("status") || has("compliance") {
        return Some(Answers::Trust);
    }
    if has("careers") || has("jobs") || has("hiring") {
        return Some(Answers::Direction);
    }
    if has("about") || has("company") || has("team") {
        return Some(Answers::Identity);
    }
    if has("features") || has("product") {
        return Some(Answers::Features);
    }
    // Documentation answers *how do I use this*, and only its front page answers *what does
    // this product do*. A page underneath it does not.
    //
    // `BENCHMARKS.md` Run 8: `linear.app/docs/mcp.md` is a setup guide, and read as a features
    // page it reported `Setup`, `Claude` and `Cursor` as capabilities of Linear. Worse, it
    // took the slot: both of Linear's feature sources were documentation, and its real
    // `/features` page — eight clean capabilities — was never read. The `llms.txt` those pages
    // came from outranks a probe, so the wrong classification wins the slot every time.
    if has("docs") || has("documentation") {
        return depth(&p).le(&1).then_some(Answers::Features);
    }
    None
}

/// Whether a segment names something a visitor does rather than something a company states.
///
/// Kept narrow on purpose: each of these is a step in a transaction, and none of them is ever
/// the page where a fact is published. `/pricing/upgrade` is where you buy; `/pricing` is
/// where the price is written down.
fn is_transactional(segment: &str) -> bool {
    const ACTIONS: [&str; 12] = [
        "setup",
        "upgrade",
        "checkout",
        "cart",
        "signup",
        "register",
        "login",
        "signin",
        "subscribe",
        "billing",
        "trial",
        "demo",
    ];
    ACTIONS.contains(&segment)
}

/// How many segments deep a path is. `/docs` is 1, `/docs/mcp.md` is 2.
///
/// A leading locale does not count — `/es/docs` is the front page of the documentation in
/// Spanish, not a page inside it.
fn depth(path: &str) -> usize {
    let without_locale = crate::locale::leading(path).map_or(path, |locale| {
        path.trim_start_matches('/')
            .get(locale.len()..)
            .unwrap_or_default()
    });
    without_locale.split('/').filter(|s| !s.is_empty()).count()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn a_page_you_do_something_on_answers_nothing() {
        // Both of these are in todoist's sitemap, both classify on the word `pricing`, and
        // between them they took two of the five slots that run admitted.
        assert_eq!(guess("/cs/pricing/setup"), None);
        assert_eq!(guess("/cs/pricing/upgrade"), None);
        assert_eq!(guess("/checkout"), None);
        // And the page the price is actually on is untouched.
        assert_eq!(guess("/pricing"), Some(Answers::Pricing));
        assert_eq!(guess("/pricing/enterprise"), Some(Answers::Pricing));
    }

    #[test]
    fn the_front_page_of_the_docs_answers_what_the_product_does() {
        assert_eq!(guess("/docs"), Some(Answers::Features));
        assert_eq!(guess("/documentation/"), Some(Answers::Features));
    }

    #[test]
    fn a_page_inside_the_docs_does_not() {
        // Run 8: read as a features page, linear.app/docs/mcp.md reported Setup, Claude and
        // Cursor as capabilities of Linear — and took the slot its real /features page
        // should have had, because llms.txt outranks a probe.
        assert_eq!(guess("/docs/mcp.md"), None);
        assert_eq!(guess("/docs/api/webhooks"), None);
    }

    #[test]
    fn a_locale_does_not_make_the_docs_front_page_look_deep() {
        assert_eq!(guess("/es/docs"), Some(Answers::Features));
        assert_eq!(guess("/es/docs/mcp.md"), None);
    }

    #[test]
    fn a_docs_page_about_something_else_is_classified_by_that() {
        // The order of the checks matters here, and it is worth asserting rather than
        // assuming: these are answers to other questions that happen to live under /docs.
        assert_eq!(guess("/docs/security.md"), Some(Answers::Trust));
        assert_eq!(guess("/docs/releases.md"), Some(Answers::Changes));
    }

    #[test]
    fn every_question_has_at_least_one_probe() {
        // A question nothing probes for is a report section that will always be empty, and
        // it would be empty for a reason nobody can see from the report.
        let covered: HashSet<Answers> = PROBES.iter().map(|p| p.answers).collect();
        for q in [
            Answers::Pricing,
            Answers::Features,
            Answers::Changes,
            Answers::Identity,
            Answers::Trust,
            Answers::Direction,
        ] {
            assert!(covered.contains(&q), "nothing probes for {}", q.name());
        }
    }

    #[test]
    fn pricing_is_probed_first() {
        // It is the most-wanted fact and the most reliably at a guessable path, and each
        // probe costs a second of per-host delay — so if the budget runs out, it must not
        // run out before pricing.
        let first = in_order().first().copied().expect("probes exist");
        assert_eq!(first.answers, Answers::Pricing);
    }

    #[test]
    fn the_probe_list_stays_short() {
        // Every entry is a request against somebody else's server. A list that tries forty
        // paths on the chance one exists is not the crawler this project committed to being.
        assert!(PROBES.len() <= 16, "{} probes is too many", PROBES.len());
    }

    #[test]
    fn no_probe_path_is_listed_twice() {
        let mut paths: Vec<&str> = PROBES.iter().map(|p| p.path).collect();
        paths.sort_unstable();
        let before = paths.len();
        paths.dedup();
        assert_eq!(
            before,
            paths.len(),
            "a duplicate probe would double a request"
        );
    }

    #[test]
    fn every_probe_path_is_absolute_and_on_our_own_domain() {
        // These are joined onto the canonical domain. A path that did not start with `/`
        // would resolve relative to something, and a full URL would leave the domain -
        // which would quietly make a Primary source something else.
        for p in PROBES {
            assert!(p.path.starts_with('/'), "{} is not absolute", p.path);
            assert!(!p.path.contains("://"), "{} leaves the domain", p.path);
        }
    }

    #[test]
    fn a_url_can_be_classified_from_its_path() {
        assert_eq!(guess("/pricing"), Some(Answers::Pricing));
        assert_eq!(guess("/en/plans/"), Some(Answers::Pricing));
        assert_eq!(guess("/changelog/2026-01"), Some(Answers::Changes));
        assert_eq!(guess("/security"), Some(Answers::Trust));
        assert_eq!(guess("/careers/engineering"), Some(Answers::Direction));
    }

    #[test]
    fn a_long_article_slug_does_not_classify_as_a_page() {
        // Found in a real run against basecamp.com: a chapter of *Getting Real* was
        // admitted as a feature page because its slug ends in "product".
        assert_eq!(guess("/gettingreal/11.5-personify-your-product"), None);
        assert_eq!(guess("/blog/what-we-learned-about-pricing-in-2019"), None);
        // Short hyphenated page names still work.
        assert_eq!(guess("/release-notes"), Some(Answers::Changes));
        assert_eq!(guess("/en-gb/plans"), Some(Answers::Pricing));
    }

    #[test]
    fn a_word_inside_another_word_does_not_classify_a_url() {
        // "/blog/our-pricing-philosophy" is an essay, not a pricing page, and treating it
        // as one would put an opinion piece where a price list belongs.
        assert_ne!(
            guess("/blog/how-we-think-about-pricingstrategy"),
            Some(Answers::Pricing)
        );
        assert_eq!(guess("/nothing/here"), None);
    }

    #[test]
    fn a_query_string_does_not_stop_a_path_classifying() {
        // Found by a test on the sitemap parser: `/pricing?ref=nav` classified as nothing,
        // because `pricing?ref=nav` is not the segment `pricing`.
        assert_eq!(guess("/pricing?ref=nav"), Some(Answers::Pricing));
        assert_eq!(guess("/plans#team"), Some(Answers::Pricing));
    }

    #[test]
    fn a_hyphenated_path_still_classifies() {
        // Real sites write it both ways, and missing half of them would halve the yield.
        assert_eq!(guess("/release-notes"), Some(Answers::Changes));
        assert_eq!(
            guess("/what_s_new"),
            None,
            "not a segment we claim to recognise"
        );
    }
}
