//! The market's own writing, read as evidence rather than discarded as noise.
//!
//! `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 5, and cause 3 of
//! `PRODUCT_IDEA_RESULTS_LOGIC.md` §4 — **the ranking measures appearance, not fit.**
//!
//! # The reframe
//!
//! Until this, a search engine was the *source of companies*: candidates were the domains that
//! came back, ranked by how many queries returned each one. That measures how widely a company
//! is written about, which is why a household name in every adjacent listicle wins and the
//! specialist named in one careful article scores below the floor and is refused.
//!
//! A search engine is better at a different job: **finding the pages that survey a market.** The
//! companies then come from what those pages say. `workamajig.com` is returned by one query out
//! of three and is listed on every buyer's guide to project management for agencies; the first
//! fact is weak evidence and the second is strong, and until this module only the first was
//! counted.
//!
//! # `NOT_A_COMPANY` inverts
//!
//! [`crate::candidates`] drops review sites, forums and blog hosts as candidates, correctly:
//! `g2.com` is not a competitor. Their **contents** were dropped with them, which is where the
//! comparisons, the categories and the names actually live. The same list is now an index of the
//! pages worth reading. They are still never candidates.
//!
//! # What counts as being named
//!
//! **A link, and only a link.** A page that lists a vendor links to it, and a link is a fact
//! about the page rather than a reading of it — no model, no summarizing, nothing asserted that
//! was not read. `FACT_CHECKING.md`'s rule for claims, applied to the choice of company.
//!
//! **Two independent hosts**, which is §L6's independence rule — and it is
//! [`crate::candidates::CORROBORATION`] itself rather than a second constant beside it. A
//! search returning a company and a guide listing it are two kinds of thing pointing at it, not
//! two different standards, so [`crate::competitors::assemble`] adds them and compares once.
//! One guide listing a company is one publisher's opinion; two unrelated publishers listing it
//! is a market saying who is in it.

use std::collections::HashMap;

use landscape_fetch::Target;

use crate::candidates::registrable;
use crate::Hit;

/// How many publisher pages one analysis will read.
///
/// **Four, and they are somebody else's servers.** A market's results usually contain more
/// review pages than company pages, and reading all of them would multiply the cost of an
/// analysis that already takes minutes. Spent on the publishers the most queries returned, so
/// the ones the engine thought were most central are the ones that get read.
pub const READING_BUDGET: usize = 4;

/// A page that surveys a market, and how many of the queries returned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guide {
    /// Its registrable host — `g2.com`.
    pub host: String,
    /// The URL to read, shallowest first if several came back.
    pub url: String,
    /// How many queries returned this host at all. Counted once per query.
    pub agreed: usize,
}

/// The publisher pages a search returned, best first.
///
/// **The inverse of [`crate::candidates::from_results`]'s filter**, calling the same function so
/// there is one answer to *what is a publisher* rather than two that drift.
#[must_use]
pub fn guides(results: &[Vec<Hit>]) -> Vec<Guide> {
    let mut by_host: HashMap<String, (Vec<usize>, String)> = HashMap::new();
    for (query, hits) in results.iter().enumerate() {
        for hit in hits {
            let Ok(target) = Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            if !crate::candidates::is_not_a_company(&host) {
                continue;
            }
            let entry = by_host
                .entry(host)
                .or_insert_with(|| (Vec::new(), hit.url.clone()));
            if !entry.0.contains(&query) {
                entry.0.push(query);
            }
            if crate::candidates::depth_of(&hit.url) < crate::candidates::depth_of(&entry.1) {
                entry.1 = hit.url.clone();
            }
        }
    }
    let mut out: Vec<Guide> = by_host
        .into_iter()
        .map(|(host, (queries, url))| Guide {
            host,
            url,
            agreed: queries.len(),
        })
        .collect();
    // Most queries first, then by host so two runs of one input agree.
    out.sort_by(|a, b| b.agreed.cmp(&a.agreed).then_with(|| a.host.cmp(&b.host)));
    out
}

/// Every company a page links to, as registrable domains.
///
/// **Links, not prose.** A vendor a guide lists is a vendor it links to, and a link is something
/// the page did rather than something we read into it. Its own domain is dropped — a review site
/// linking to its own category page has named nobody — and so is every other publisher, because
/// a guide citing another guide is not a company either.
#[must_use]
pub fn linked_from(page: &str, publisher: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for start in page.match_indices("http").map(|(i, _)| i) {
        let rest = &page[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '>' | '"' | '\'' | '<'))
            .unwrap_or(rest.len());
        let Ok(target) = Target::parse(&rest[..end]) else {
            continue;
        };
        let host = registrable(&target.host);
        if host == publisher || crate::candidates::is_not_a_company(&host) || out.contains(&host) {
            continue;
        }
        out.push(host);
    }
    out
}

/// Read the market's literature and return, per company, which publishers named it.
///
/// Bounded by [`READING_BUDGET`]. A page that cannot be read names nobody, which is the same
/// silence [`crate::products::split`] treats as *no evidence* rather than as evidence of
/// absence.
pub async fn named_in<F, Fut>(results: &[Vec<Hit>], fetch: &F) -> HashMap<String, Vec<String>>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut named: HashMap<String, Vec<String>> = HashMap::new();
    for guide in guides(results).into_iter().take(READING_BUDGET) {
        let Some(page) = fetch(guide.url.clone()).await else {
            continue;
        };
        for host in linked_from(&page, &guide.host) {
            // **Once per publisher, and `guides` is what makes that true.** A guide that
            // mentions a vendor in its table, its summary and its footer has said one thing;
            // [`linked_from`] deduplicates within the page, and a host has exactly one entry
            // here however many of its pages came back.
            named.entry(host).or_default().push(guide.host.clone());
        }
    }
    for by in named.values_mut() {
        by.sort();
    }
    named
}

/// Fold what the literature said into the candidates the queries produced.
///
/// **Two routes into one list, and both are counted.** A company the queries returned keeps
/// everything it had and gains the publishers that named it; a company **only** the literature
/// names joins with `agreed = 0` and the publishers as its whole case. Its evidence URL is its
/// own front page, because no query returned a deeper one and the front page is what
/// [`crate::candidates::describe`] is about to read anyway.
///
/// **A company named by one publisher is still added**, at one source, and
/// [`crate::competitors::assemble`] is what refuses it. That is deliberate: a candidate dropped
/// before the set can see it is a candidate nothing can report as excluded, which is the defect
/// [`crate::competitors::Aside`] was written to remove.
#[must_use]
pub fn admit(
    found: Vec<crate::candidates::Found>,
    named: &HashMap<String, Vec<String>>,
    sources: crate::candidates::Sources,
) -> Vec<crate::candidates::Found> {
    let mut out: Vec<crate::candidates::Found> = found
        .into_iter()
        .map(|mut one| {
            if let Some(by) = named.get(&one.host) {
                one.named_by.clone_from(by);
                one.confidence = crate::candidates::score(
                    one.agreed + by.len(),
                    sources.of(),
                    crate::candidates::depth_of(&one.shallowest),
                );
            } else {
                // **Re-scored even with no publishers, because the divisor moved.** Reading four
                // guides and finding a company on none of them is a fact about that company, and
                // a score that ignored the guides it was absent from would be arithmetic about a
                // smaller world than the one we looked at.
                one.confidence = crate::candidates::score(
                    one.agreed,
                    sources.of(),
                    crate::candidates::depth_of(&one.shallowest),
                );
            }
            one
        })
        .collect();

    for (host, by) in named {
        if out.iter().any(|f| f.host == *host) {
            continue;
        }
        out.push(crate::candidates::Found {
            confidence: crate::candidates::score(by.len(), sources.of(), 0),
            host: host.clone(),
            agreed: 0,
            shallowest: format!("https://{host}/"),
            declared: None,
            named_by: by.clone(),
        });
    }

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.host.cmp(&b.host))
    });
    out
}

/// How many publisher pages were actually read, which is the divisor a reader is owed.
///
/// **Read, not returned.** A guide nobody could fetch checked nothing, for the same reason
/// [`crate::candidates::Queried::sent`] is the divisor for searches and the completed list is
/// the audit trail.
#[must_use]
pub fn read(results: &[Vec<Hit>]) -> usize {
    guides(results).len().min(READING_BUDGET)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn hit(url: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: String::new(),
            snippet: String::new(),
        }
    }

    #[test]
    fn the_pages_a_search_returned_that_are_not_companies_are_the_ones_worth_reading() {
        let found = guides(&[
            vec![
                hit("https://www.g2.com/categories/project-management"),
                hit("https://asana.com/"),
            ],
            vec![
                hit("https://www.g2.com/categories/project-management"),
                hit("https://www.capterra.com/project-management-software/"),
            ],
        ]);
        let hosts: Vec<&str> = found.iter().map(|g| g.host.as_str()).collect();
        assert_eq!(
            hosts,
            vec!["g2.com", "capterra.com"],
            "publishers only, most-returned first"
        );
        assert_eq!(found[0].agreed, 2);
        assert_eq!(found[1].agreed, 1);
    }

    #[test]
    fn a_guide_names_the_companies_it_links_to_and_nobody_else() {
        let page = "# Best project management software\n\n\
             1. [Asana](https://asana.com/) is broad.\n\
             2. [Workamajig](https://www.workamajig.com/) is for agencies.\n\n\
             See also [our methodology](https://www.g2.com/about) and \
             [the subreddit](https://www.reddit.com/r/projectmanagement).";
        let named = linked_from(page, "g2.com");
        assert_eq!(
            named,
            vec!["asana.com".to_owned(), "workamajig.com".to_owned()],
            "its own pages and another publisher are not companies it named"
        );
    }

    #[test]
    fn one_guide_naming_a_company_three_times_has_named_it_once() {
        let page = "[X](https://x-corp.example/) [X](https://x-corp.example/pricing) \
                    [X](https://x-corp.example/about)";
        assert_eq!(
            linked_from(page, "g2.com"),
            vec!["x-corp.example".to_owned()]
        );
    }

    #[tokio::test]
    async fn two_independent_guides_are_two_and_one_guide_twice_is_one() {
        let results = vec![
            vec![
                hit("https://www.g2.com/categories/pm"),
                hit("https://www.capterra.com/pm/"),
            ],
            vec![
                hit("https://www.g2.com/categories/pm"),
                hit("https://www.g2.com/categories/pm-for-agencies"),
            ],
        ];
        let named = named_in(&results, &|url: String| {
            std::future::ready(Some(if url.contains("capterra") {
                "[Workamajig](https://www.workamajig.com/)".to_owned()
            } else {
                "[Workamajig](https://www.workamajig.com/) [Asana](https://asana.com/)".to_owned()
            }))
        })
        .await;

        assert_eq!(
            named.get("workamajig.com"),
            Some(&vec!["capterra.com".to_owned(), "g2.com".to_owned()]),
            "two publishers, whatever order they were read in"
        );
        assert_eq!(
            named.get("asana.com"),
            Some(&vec!["g2.com".to_owned()]),
            "and g2 naming it on two of its own pages is still one publisher"
        );
    }

    #[test]
    fn a_company_only_the_literature_names_still_becomes_a_candidate() {
        // **The whole reframe in one assertion.** The queries never returned this domain; two
        // buyer's guides list it. Before this module the answer was *which domains came back*,
        // so it could not have been in the set at all.
        let mut named = HashMap::new();
        named.insert(
            "workamajig.com".to_owned(),
            vec!["capterra.com".to_owned(), "g2.com".to_owned()],
        );
        let sources = crate::candidates::Sources {
            asked: 3,
            guides: 2,
        };
        let out = admit(Vec::new(), &named, sources);

        assert_eq!(out.len(), 1, "{out:#?}");
        assert_eq!(out[0].host, "workamajig.com");
        assert_eq!(
            out[0].agreed, 0,
            "no search returned it, and that is not hidden"
        );
        assert_eq!(out[0].named_by.len(), 2);
        assert_eq!(
            out[0].shallowest, "https://workamajig.com/",
            "its own front page is the evidence, because no query offered a deeper one"
        );
        assert!(
            out[0].confidence >= landscape_core::subject::MINIMUM_CONFIDENCE,
            "two independent guides clear the floor: {}",
            out[0].confidence
        );
    }

    #[test]
    fn a_company_no_guide_named_is_scored_against_the_guides_it_was_absent_from() {
        // **The divisor moved, so every score moves.** Reading four guides and finding a company
        // on none of them is a fact about that company; scoring it as if the guides had not been
        // read would be arithmetic about a smaller world than the one we looked at.
        let one = crate::candidates::Found {
            host: "obscure.example".to_owned(),
            confidence: 0.0,
            agreed: 2,
            shallowest: "https://obscure.example/".to_owned(),
            declared: None,
            named_by: Vec::new(),
        };
        let searches_only = admit(
            vec![one.clone()],
            &HashMap::new(),
            crate::candidates::Sources {
                asked: 3,
                guides: 0,
            },
        );
        let with_guides = admit(
            vec![one],
            &HashMap::new(),
            crate::candidates::Sources {
                asked: 3,
                guides: 3,
            },
        );
        assert!(
            with_guides[0].confidence < searches_only[0].confidence,
            "{} should be below {}",
            with_guides[0].confidence,
            searches_only[0].confidence
        );
    }

    #[tokio::test]
    async fn a_guide_nobody_could_read_names_nobody() {
        let results = vec![vec![hit("https://www.g2.com/categories/pm")]];
        let named = named_in(&results, &|_url: String| std::future::ready(None)).await;
        assert!(named.is_empty());
    }

    #[tokio::test]
    async fn the_reading_is_bounded() {
        let results: Vec<Vec<Hit>> = vec![(0..READING_BUDGET + 3)
            .map(|i| hit(&format!("https://www.g2.com/categories/c{i}")))
            .collect()];
        // All on one host, so there is one guide however many pages came back.
        assert_eq!(guides(&results).len(), 1);

        let many: Vec<Vec<Hit>> = vec![[
            "https://www.g2.com/a",
            "https://www.capterra.com/a",
            "https://www.getapp.com/a",
            "https://www.trustradius.com/a",
            "https://www.slant.co/a",
            "https://www.quora.com/a",
        ]
        .iter()
        .map(|u| hit(u))
        .collect()];
        let asked = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let seen = std::sync::Arc::clone(&asked);
        let _ = named_in(&many, &move |_url: String| {
            *seen.lock().expect("not poisoned") += 1;
            std::future::ready(None)
        })
        .await;
        assert_eq!(*asked.lock().expect("not poisoned"), READING_BUDGET);
        assert_eq!(read(&many), READING_BUDGET);
    }
}
