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
    // Per host, then per URL: which queries returned each page this publisher has.
    let mut by_host: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();
    for (query, hits) in results.iter().enumerate() {
        for hit in hits {
            let Ok(target) = Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            if !crate::candidates::is_not_a_company(&host) {
                continue;
            }
            // **A publisher's front page is not a guide to anything**, and review found it
            // winning: `guides` kept the shallowest URL per host, so `g2.com/` arriving beside
            // `g2.com/categories/project-management` discarded the page that surveys the market
            // and fetched the one that surveys everything. A root result says the publisher
            // exists, which nobody needed a fetch to learn.
            if crate::candidates::depth_of(&hit.url) == 0 {
                continue;
            }
            let seen = by_host
                .entry(host)
                .or_default()
                .entry(hit.url.clone())
                .or_default();
            if !seen.contains(&query) {
                seen.push(query);
            }
        }
    }

    let mut out: Vec<Guide> = by_host
        .into_iter()
        .filter_map(|(host, pages)| {
            let mut ranked: Vec<(String, usize)> =
                pages.into_iter().map(|(url, qs)| (url, qs.len())).collect();
            // **The page the most queries agreed on**, then the shallowest, then the URL so two
            // runs of one input agree. Depth is the tie-breaker it always was; what changed is
            // that it is no longer the whole rule.
            ranked.sort_by(|a, b| {
                b.1.cmp(&a.1)
                    .then_with(|| {
                        crate::candidates::depth_of(&a.0).cmp(&crate::candidates::depth_of(&b.0))
                    })
                    .then_with(|| a.0.cmp(&b.0))
            });
            let agreed = ranked.iter().map(|(_, n)| *n).max()?;
            let (url, _) = ranked.into_iter().next()?;
            Some(Guide { host, url, agreed })
        })
        .collect();
    // Most queries first, then by host so two runs of one input agree.
    out.sort_by(|a, b| b.agreed.cmp(&a.agreed).then_with(|| a.host.cmp(&b.host)));
    out
}

/// Every company a page **links** to: the registrable domain, and the URL the link pointed at.
///
/// **Links, not prose, and the first version did not enforce that.** Review found it scanning for
/// every `http` in the page, so a URL quoted in a sentence, an image source or a canonical tag
/// counted as a vendor endorsement. A link is something the page *did*; a URL sitting in prose is
/// something we read into it, and this module's whole claim is the first of those.
///
/// So it parses markdown link destinations — `[text](url)` and `<url>` — and **not** images,
/// which are `![alt](url)` and are a picture of a thing rather than a recommendation of it.
///
/// The publisher's own domain is dropped — a review site linking to its own category page has
/// named nobody — and so is every other publisher, because a guide citing a guide is not a
/// company either.
///
/// **The URL is kept, not only the host.** [`crate::products::split`] can turn one domain into
/// several products, and a guide that linked to Microsoft Teams has said nothing about Microsoft
/// Project. See [`Endorsement::at`].
#[must_use]
pub fn linked_from(page: &str, publisher: &str) -> Vec<Endorsement> {
    let mut out: Vec<Endorsement> = Vec::new();
    let bytes = page.as_bytes();
    let mut push = |url: &str| {
        let Ok(target) = Target::parse(url) else {
            return;
        };
        let host = registrable(&target.host);
        if host == publisher || crate::candidates::is_not_a_company(&host) {
            return;
        }
        // Once per destination: a guide linking one page from its table and its footer has
        // linked to it.
        if out.iter().any(|e| e.at == url) {
            return;
        }
        out.push(Endorsement {
            by: publisher.to_owned(),
            host,
            at: url.to_owned(),
        });
    };

    for (open, _) in page.match_indices("](") {
        let Some(text_start) = page[..open].rfind('[') else {
            continue;
        };
        // `![alt](url)` is a picture of a thing, not a link to it.
        if text_start > 0 && bytes[text_start - 1] == b'!' {
            continue;
        }
        let rest = &page[open + 2..];
        let Some(close) = rest.find(')') else {
            continue;
        };
        // A markdown destination may carry a title: `(url "Title")`.
        push(rest[..close].split_whitespace().next().unwrap_or(""));
    }
    for (open, _) in page.match_indices("<http") {
        let rest = &page[open + 1..];
        let Some(close) = rest.find('>') else {
            continue;
        };
        push(&rest[..close]);
    }
    out
}

/// One publisher linking to one page of one company.
///
/// **The URL, not only the host, because a domain is not a product.** Review found a domain's
/// endorsements handed to whichever of its products won the search-based split: two guides
/// linking Microsoft Teams corroborating Microsoft Project. What a guide pointed at is the only
/// thing that says which product it meant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endorsement {
    /// The publisher's registrable host.
    pub by: String,
    /// The company's registrable host.
    pub host: String,
    /// The URL the guide actually linked to.
    pub at: String,
}

/// The independent publishers among a set of endorsements, sorted and without repeats.
///
/// **Publishers, not links.** One guide linking three of a company's pages has said one thing,
/// and counting three would manufacture the corroboration [`crate::candidates::CORROBORATION`]
/// exists to require.
#[must_use]
pub fn publishers(of: &[Endorsement]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for one in of {
        if !out.contains(&one.by) {
            out.push(one.by.clone());
        }
    }
    out.sort();
    out
}

/// What the market's literature turned out to say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reading {
    /// Per company host, every link a guide made to it.
    pub named: HashMap<String, Vec<Endorsement>>,
    /// **Pages that came back**, not pages we chose to ask for.
    ///
    /// Review found this counted before the fetching: four guides selected and four fetches
    /// failing left every candidate rescored as though four guides had looked and found nothing,
    /// and an exclusion saying *"none of the 4 buyer's guides we read"* about pages nobody read.
    /// That is the same divisor rule [`crate::candidates::Queried::sent`] states for searches,
    /// pointed the other way: a guide that did not arrive checked nothing.
    pub read: usize,
}

/// Read the market's literature.
///
/// Bounded by [`READING_BUDGET`]. A page that cannot be read names nobody **and counts as
/// nothing** — the same silence [`crate::products::split`] treats as *no evidence* rather than
/// as evidence of absence.
pub async fn named_in<F, Fut>(results: &[Vec<Hit>], fetch: &F) -> Reading
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut out = Reading::default();
    for guide in guides(results).into_iter().take(READING_BUDGET) {
        let Some(page) = fetch(guide.url.clone()).await else {
            continue;
        };
        out.read += 1;
        for endorsement in linked_from(&page, &guide.host) {
            out.named
                .entry(endorsement.host.clone())
                .or_default()
                .push(endorsement);
        }
    }
    out
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
    reading: &Reading,
    sources: crate::candidates::Sources,
) -> Vec<crate::candidates::Found> {
    let named = &reading.named;
    let mut out: Vec<crate::candidates::Found> = found
        .into_iter()
        .map(|mut one| {
            if let Some(by) = named.get(&one.host) {
                one.named_by.clone_from(by);
                one.confidence = crate::candidates::score(
                    one.agreed + publishers(by).len(),
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
            confidence: crate::candidates::score(publishers(by).len(), sources.of(), 0),
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

    fn hosts(of: &[Endorsement]) -> Vec<&str> {
        of.iter().map(|e| e.host.as_str()).collect()
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
        let by: Vec<&str> = found.iter().map(|g| g.host.as_str()).collect();
        assert_eq!(
            by,
            vec!["g2.com", "capterra.com"],
            "publishers only, most-returned first"
        );
        assert_eq!(found[0].agreed, 2);
        assert_eq!(found[1].agreed, 1);
    }

    #[test]
    fn a_publishers_front_page_is_not_a_guide_to_anything() {
        // **Review found the homepage winning.** Keeping the shallowest URL per host meant
        // `g2.com/` arriving beside the page that surveys this market discarded the guide and
        // fetched the front door. A root result says the publisher exists.
        let found = guides(&[
            vec![
                hit("https://www.g2.com/"),
                hit("https://www.g2.com/categories/project-management"),
            ],
            vec![hit("https://www.g2.com/")],
        ]);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].url, "https://www.g2.com/categories/project-management",
            "the page about this market, not the one about every market"
        );

        // And a publisher that came back only at its root is not a guide at all.
        assert!(guides(&[vec![hit("https://www.g2.com/")]]).is_empty());
    }

    #[test]
    fn the_page_the_most_queries_agreed_on_is_the_one_read() {
        let found = guides(&[
            vec![hit("https://www.g2.com/categories/deep/one")],
            vec![hit("https://www.g2.com/categories/deep/one")],
            vec![hit("https://www.g2.com/shallow")],
        ]);
        assert_eq!(
            found[0].url, "https://www.g2.com/categories/deep/one",
            "two queries beats one, and depth is only the tie-breaker"
        );
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
            hosts(&named),
            vec!["asana.com", "workamajig.com"],
            "its own pages and another publisher are not companies it named"
        );
        assert_eq!(named[0].at, "https://asana.com/", "and the URL is kept");
        assert_eq!(named[0].by, "g2.com");
    }

    #[test]
    fn a_url_that_is_not_a_link_names_nobody() {
        // **The boundary this module claims, enforced.** The first version scanned for every
        // `http` in the page, so a URL quoted in a sentence, an image source or a canonical tag
        // was a vendor endorsement. Review found it; a link is something the page *did*.
        let prose = "Their site is https://asana.com/ if you want to look.";
        assert!(
            linked_from(prose, "g2.com").is_empty(),
            "prose is not a link"
        );

        let image = "![Asana's logo](https://asana.com/logo.png)";
        assert!(
            linked_from(image, "g2.com").is_empty(),
            "a picture of a thing is not a recommendation of it"
        );

        let meta = "<link rel=\"canonical\" href=\"https://asana.com/\" />";
        assert!(
            linked_from(meta, "g2.com").is_empty(),
            "metadata is not a listing"
        );

        // What does count: a markdown link, and an autolink.
        assert_eq!(
            hosts(&linked_from("[Asana](https://asana.com/)", "g2.com")),
            vec!["asana.com"]
        );
        assert_eq!(
            hosts(&linked_from("<https://asana.com/>", "g2.com")),
            vec!["asana.com"]
        );
        // A destination may carry a title, and it is still one link.
        assert_eq!(
            hosts(&linked_from(
                "[Asana](https://asana.com/ \"Asana\")",
                "g2.com"
            )),
            vec!["asana.com"]
        );
    }

    #[test]
    fn one_guide_naming_a_company_three_times_has_named_it_once() {
        let page = "[X](https://x-corp.example/) [X](https://x-corp.example/pricing) \
                    [X](https://x-corp.example/)";
        let named = linked_from(page, "g2.com");
        assert_eq!(
            named.len(),
            2,
            "two distinct pages, and the repeat is not a third"
        );
        assert_eq!(
            publishers(&named),
            vec!["g2.com".to_owned()],
            "one publisher"
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
        let reading = named_in(&results, &|url: String| {
            std::future::ready(Some(if url.contains("capterra") {
                "[Workamajig](https://www.workamajig.com/)".to_owned()
            } else {
                "[Workamajig](https://www.workamajig.com/) [Asana](https://asana.com/)".to_owned()
            }))
        })
        .await;

        assert_eq!(reading.read, 2);
        assert_eq!(
            publishers(&reading.named["workamajig.com"]),
            vec!["capterra.com".to_owned(), "g2.com".to_owned()],
            "two publishers, whatever order they were read in"
        );
        assert_eq!(
            publishers(&reading.named["asana.com"]),
            vec!["g2.com".to_owned()],
            "and g2 having two pages in the results is still one publisher"
        );
    }

    #[tokio::test]
    async fn a_guide_nobody_could_read_names_nobody_and_counts_as_nothing() {
        // **Review's first finding.** The divisor used to be the guides *selected*, counted
        // before any fetch. Four selected and four failing left every candidate rescored as
        // though four guides had looked and found nothing, and an exclusion saying "none of the
        // 4 buyer's guides we read" about pages nobody read.
        let results = vec![vec![
            hit("https://www.g2.com/categories/pm"),
            hit("https://www.capterra.com/pm/"),
        ]];
        let reading = named_in(&results, &|_url: String| std::future::ready(None)).await;
        assert!(reading.named.is_empty());
        assert_eq!(
            reading.read, 0,
            "nothing was read, so nothing checked anything"
        );

        // And the score is the one it would have had with no literature step at all.
        let one = crate::candidates::Found {
            host: "obscure.example".to_owned(),
            confidence: 0.0,
            agreed: 2,
            shallowest: "https://obscure.example/".to_owned(),
            declared: None,
            named_by: Vec::new(),
        };
        let sources = crate::candidates::Sources {
            asked: 3,
            guides: reading.read,
        };
        let after = admit(vec![one.clone()], &reading, sources);
        assert!(
            (after[0].confidence - crate::candidates::score(2, 3, 0)).abs() < f32::EPSILON,
            "{}",
            after[0].confidence
        );

        // And the sentence says nobody was asked, rather than that nobody listed it.
        let said = crate::competitors::Aside::Uncorroborated {
            agreed: 1,
            asked: 3,
            named_by: 0,
            guides: reading.read,
        }
        .sentence();
        assert!(said.contains("no buyer's guide"), "{said}");
        assert!(!said.contains("we read list"), "{said}");
    }

    #[test]
    fn a_company_only_the_literature_names_still_becomes_a_candidate() {
        // **The whole reframe in one assertion.** The queries never returned this domain; two
        // buyer's guides link to it. Before this module the answer was *which domains came
        // back*, so it could not have been in the set at all.
        let reading = Reading {
            named: HashMap::from([(
                "workamajig.com".to_owned(),
                vec![
                    Endorsement {
                        by: "capterra.com".to_owned(),
                        host: "workamajig.com".to_owned(),
                        at: "https://www.workamajig.com/".to_owned(),
                    },
                    Endorsement {
                        by: "g2.com".to_owned(),
                        host: "workamajig.com".to_owned(),
                        at: "https://www.workamajig.com/".to_owned(),
                    },
                ],
            )]),
            read: 2,
        };
        let out = admit(
            Vec::new(),
            &reading,
            crate::candidates::Sources {
                asked: 3,
                guides: 2,
            },
        );

        assert_eq!(out.len(), 1, "{out:#?}");
        assert_eq!(out[0].host, "workamajig.com");
        assert_eq!(
            out[0].agreed, 0,
            "no search returned it, and that is not hidden"
        );
        assert_eq!(publishers(&out[0].named_by).len(), 2);
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
            &Reading::default(),
            crate::candidates::Sources {
                asked: 3,
                guides: 0,
            },
        );
        let with_guides = admit(
            vec![one],
            &Reading {
                named: HashMap::new(),
                read: 3,
            },
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
    async fn the_reading_is_bounded() {
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
        let reading = named_in(&many, &move |_url: String| {
            *seen.lock().expect("not poisoned") += 1;
            std::future::ready(Some("[A](https://a.example/)".to_owned()))
        })
        .await;
        assert_eq!(*asked.lock().expect("not poisoned"), READING_BUDGET);
        assert_eq!(reading.read, READING_BUDGET);
    }
}
