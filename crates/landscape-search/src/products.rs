//! Which *product* a search result is about, when a domain sells more than one.
//!
//! `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 3, and cause 2 of
//! `PRODUCT_IDEA_RESULTS_LOGIC.md` §4. A reader asked for *"project management for a small design
//! agency"* and was handed **Microsoft**, because three queries returned three Microsoft pages
//! and [`crate::candidates::from_results`] groups by registrable domain. Two of those pages were
//! Microsoft Project and one was Microsoft Teams; the group agreed with all three queries, and a
//! chat product's appearance corroborated a project-management vendor.
//!
//! # The rule, and why it is not a URL rule
//!
//! The golden set implemented four path-shaped candidates and scored them
//! (`BENCHMARKS.md` Run 51). **Each one either merges two products or splits one:**
//!
//! | Rule | Joins one product's pages? | Separates two products in a suite? |
//! |---|---|---|
//! | Domain (what ran before this) | Yes | **No** |
//! | First path segment | No — splits on locale | No — groups on locale |
//! | First meaningful segment | Yes | No — `microsoft-365` is a suite, not a container |
//! | Last path segment | No — splits `/excel` from `/excel/pricing` | Yes |
//!
//! That is a result about the approach rather than a threshold to tune. What is left is the
//! vendor's domain plus **the name the page declares about itself** — [`declared_for`] — which
//! needs the page, and needs it *before* the merge. This module is that inversion, and
//! [`SPLIT_BUDGET`] is what keeps it from being unbounded.
//!
//! # The domain is half the key
//!
//! An earlier version of this rule, in the golden set, keyed on the declared name alone. Review
//! found what that does: two vendors who both call their product *Invoicing* become one company.
//! **That is a wider failure than the domain-collapse it replaces** — this one crosses a vendor
//! boundary, which grouping by domain never could. The key is `domain#name`, and
//! `landscape-golden`'s `the_declared_identity_is_scored_on_fixture_pages` holds the pair that
//! proves it.
//!
//! # What this does not do
//!
//! **It does not put two products of one domain in one report.** The strongest product becomes
//! the candidate and the rest of the domain's appearances stop corroborating it; a set holding
//! *Microsoft Project* **and** *Microsoft Planner* as two rows needs
//! [`landscape_core::Candidate`] to stop being keyed on a domain, which is a change with a
//! blast radius well outside this one. What a reader gets today is the right product, named,
//! with the agreement that product actually earned.

use std::collections::HashMap;

use landscape_fetch::Target;

use crate::candidates::{registrable, Found};
use crate::Hit;

/// How many **extra** page reads one analysis will spend telling products apart.
///
/// **Extra, not total.** A domain whose results are all non-root pages costs `k` reads here and
/// saves the one [`crate::candidates::describe`] would have spent on its front page, so a single
/// product page is free and only the splitting costs anything.
///
/// Four is the whole budget for a run, spent in rank order. A domain that would cost more than
/// what is left keeps the old behavior rather than being split on half its evidence — **a
/// partial split is worse than none**, because the appearances that were not read stay attached
/// to whichever product happened to be fetched first.
pub const SPLIT_BUDGET: usize = 4;

/// The vendor's domain and the name a page declares about itself: `example.com#the product`.
///
/// The first heading stands in for what a fuller reader would take from a canonical link or an
/// `og:title`. `None` for a page that declares nothing, or a URL that will not parse — **a page
/// that could not be read declares no identity**, and keying it to the domain alone would
/// quietly turn this back into the rule it replaces.
#[must_use]
pub fn declared_for(url: &str, page: &str) -> Option<String> {
    let target = Target::parse(url).ok()?;
    let host = registrable(&target.host);
    let name = page
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|name| name.trim().to_lowercase())
        .filter(|name| !name.is_empty())?;
    Some(format!("{host}#{name}"))
}

/// One URL a search returned, and which of the queries returned it.
///
/// **The query numbers, not a count.** Two URLs of one product returned by the same query agreed
/// with one query between them, and adding their counts would manufacture the corroboration
/// [`crate::candidates::CORROBORATION`] exists to require.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appearance {
    /// As the engine gave it.
    pub url: String,
    /// Indices into the result lists, ascending and without repeats.
    pub queries: Vec<usize>,
}

/// What a product's own page said about itself, kept so nobody fetches it twice.
///
/// **Both halves come from the page this candidate was found at.** Before this, a candidate's
/// name and its one line came from the *domain's front page*, which for `microsoft.com` is a
/// page about a company rather than about the product three queries actually returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// What the page calls the product, in the page's own capitalization.
    pub name: String,
    /// The first line of prose under that heading.
    pub what_it_is: String,
    /// The Markdown, carried so [`crate::candidates::describe`] can ask it which of the market's
    /// words it uses without spending a second request on somebody's server.
    pub page: String,
}

/// Every distinct URL each host was returned at, and by which queries.
///
/// Hosts are the registrable domain, and known publishers are dropped, exactly as
/// [`crate::candidates::from_results`] does — **by calling the same two functions**, because a
/// second copy of *what counts as a company* is a rule that goes stale in one place only.
#[must_use]
pub fn appearances(results: &[Vec<Hit>]) -> HashMap<String, Vec<Appearance>> {
    let mut by_host: HashMap<String, Vec<Appearance>> = HashMap::new();
    for (query, hits) in results.iter().enumerate() {
        for hit in hits {
            let Ok(target) = Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            if crate::candidates::is_not_a_company(&host) {
                continue;
            }
            let seen = by_host.entry(host).or_default();
            if let Some(at) = seen.iter_mut().find(|a| a.url == hit.url) {
                if !at.queries.contains(&query) {
                    at.queries.push(query);
                }
            } else {
                seen.push(Appearance {
                    url: hit.url.clone(),
                    queries: vec![query],
                });
            }
        }
    }
    by_host
}

/// Whether a URL is the domain's own front page.
///
/// **The rule for the root, and it is safe whichever identity rule wins.** When a search returns
/// `asana.com`, the company and the product are one thing and one name is right; only a non-root
/// URL can produce *Microsoft Project, by Microsoft*. A host returned at its root anywhere keeps
/// the front-page path this module is otherwise replacing.
///
/// Built on the ranking's own [`depth`], not on a second reading of a URL: how deep a page is
/// already decides part of its score, and two answers to that would be two answers to *is this a
/// front page*.
///
/// [`depth`]: crate::candidates::depth
fn is_root(url: &str) -> bool {
    crate::candidates::depth(url) == 0
}

/// Read the pages behind the results and let each domain's strongest **product** be the
/// candidate.
///
/// Runs between [`crate::candidates::from_results`] and [`crate::candidates::describe`], which is
/// the inversion the identity rule forces: the merge now happens after a read rather than before
/// one.
///
/// A domain qualifies when **no query returned its root** — see [`is_root`] — and when reading
/// all of its URLs fits in what is left of [`SPLIT_BUDGET`]. Everything else is returned
/// untouched, so a run with no budget left, an unreadable page or a front-page result behaves
/// exactly as it did before this module existed.
///
/// The list comes back re-sorted, because agreement can only fall here: a domain that was
/// returned by three queries and turns out to be three products has a strongest product that
/// agreed with one of them.
pub async fn split<F, Fut>(
    found: Vec<Found>,
    asked: usize,
    results: &[Vec<Hit>],
    fetch: &F,
) -> Vec<Found>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let by_host = appearances(results);
    let mut budget = SPLIT_BUDGET;
    let mut out: Vec<Found> = Vec::with_capacity(found.len());

    for one in found {
        let Some(urls) = by_host.get(&one.host) else {
            out.push(one);
            continue;
        };
        // A front page among the results is the evidence already, and reading it is what
        // `describe` is about to do anyway.
        if urls.is_empty() || urls.iter().any(|a| is_root(&a.url)) {
            out.push(one);
            continue;
        }
        // `k` reads for `k` URLs, minus the front page this saves. A domain that does not fit
        // is left whole: half a split attributes the unread appearances to the wrong product.
        let extra = urls.len().saturating_sub(1);
        if extra > budget {
            out.push(one);
            continue;
        }
        budget -= extra;

        let mut read: Vec<(&Appearance, Option<String>)> = Vec::with_capacity(urls.len());
        for at in urls {
            read.push((at, fetch(at.url.clone()).await));
        }
        out.push(strongest(&one, asked, &read));
    }

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.host.cmp(&b.host))
    });
    out
}

/// The one product on this domain that the queries agreed about most.
///
/// Appearances whose page declares nothing keep the **domain** as their key, so a host whose
/// pages could not be read comes out of here identical to what went in. That is the fallback the
/// whole module rests on: an unreadable page must never be able to split a candidate.
fn strongest(one: &Found, asked: usize, read: &[(&Appearance, Option<String>)]) -> Found {
    /// One product's share of a domain's appearances.
    struct Grouped {
        queries: Vec<usize>,
        shallowest: String,
        page: Option<String>,
    }

    let mut by_identity: HashMap<String, Grouped> = HashMap::new();
    for (at, page) in read {
        let declared = page
            .as_deref()
            .and_then(|markdown| declared_for(&at.url, markdown));
        let key = declared.clone().unwrap_or_else(|| one.host.clone());
        let entry = by_identity.entry(key).or_insert_with(|| Grouped {
            queries: Vec::new(),
            shallowest: at.url.clone(),
            page: None,
        });
        for query in &at.queries {
            if !entry.queries.contains(query) {
                entry.queries.push(*query);
            }
        }
        if crate::candidates::depth(&at.url) < crate::candidates::depth(&entry.shallowest) {
            entry.shallowest = at.url.clone();
        }
        if declared.is_some() && entry.page.is_none() {
            entry.page.clone_from(page);
        }
    }

    // Most queries wins; then the shallowest URL; then the key, so two products that tie are
    // ordered the same way on every run rather than however the map iterated.
    let mut products: Vec<(String, Grouped)> = by_identity.into_iter().collect();
    products.sort_by(|a, b| {
        b.1.queries
            .len()
            .cmp(&a.1.queries.len())
            .then_with(|| {
                crate::candidates::depth(&a.1.shallowest)
                    .cmp(&crate::candidates::depth(&b.1.shallowest))
            })
            .then_with(|| a.0.cmp(&b.0))
    });
    let Some((_, best)) = products.into_iter().next() else {
        return one.clone();
    };

    let agreed = best.queries.len();
    let declared = best.page.as_deref().map(|markdown| {
        let (name, what_it_is) = crate::candidates::naming(&one.host, markdown);
        Declared {
            name,
            what_it_is,
            page: markdown.to_owned(),
        }
    });
    Found {
        confidence: crate::candidates::score(
            agreed,
            asked,
            crate::candidates::depth(&best.shallowest),
        ),
        host: one.host.clone(),
        agreed,
        shallowest: best.shallowest,
        declared,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::candidates::from_results;

    fn hit(url: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: String::new(),
            snippet: String::new(),
        }
    }

    /// Pages by exact URL; anything else could not be read.
    fn pages(
        known: &[(&'static str, &'static str)],
    ) -> impl Fn(String) -> std::future::Ready<Option<String>> {
        let known: Vec<(String, String)> = known
            .iter()
            .map(|(u, p)| ((*u).to_owned(), (*p).to_owned()))
            .collect();
        move |url: String| {
            std::future::ready(
                known
                    .iter()
                    .find(|(at, _)| *at == url)
                    .map(|(_, page)| page.clone()),
            )
        }
    }

    /// The same, for URLs built at run time.
    fn owned_pages(
        known: &[(String, &'static str)],
    ) -> impl Fn(String) -> std::future::Ready<Option<String>> {
        let known: Vec<(String, String)> = known
            .iter()
            .map(|(u, p)| (u.clone(), (*p).to_owned()))
            .collect();
        move |url: String| {
            std::future::ready(
                known
                    .iter()
                    .find(|(at, _)| *at == url)
                    .map(|(_, page)| page.clone()),
            )
        }
    }

    const EXCEL: &str = "https://www.microsoft.com/en-us/microsoft-365/excel";
    const EXCEL_DE: &str = "https://www.microsoft.com/de-de/microsoft-365/excel";
    const EXCEL_PRICING: &str = "https://www.microsoft.com/en-us/microsoft-365/excel/pricing";
    const PROJECT: &str = "https://www.microsoft.com/en-us/microsoft-365/project/pm-software";
    const TEAMS: &str = "https://www.microsoft.com/en-us/microsoft-365/teams/group-chat";

    #[test]
    fn the_key_says_whose_product_it_is() {
        // Two vendors, one product name. The bare heading merges them; this must not.
        let a = declared_for("https://www.freshbooks.com/invoice", "# Invoicing").unwrap();
        let b = declared_for("https://quickbooks.intuit.com/invoicing/", "# Invoicing").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("freshbooks.com"), "{a}");
        assert!(b.starts_with("intuit.com"), "{b}");
    }

    #[test]
    fn a_page_that_declares_nothing_declares_nothing() {
        // Not the domain: a page we could not read must not key to the rule this replaces.
        assert_eq!(declared_for(EXCEL, ""), None);
        assert_eq!(declared_for(EXCEL, "no heading here"), None);
        assert_eq!(declared_for("javascript:alert(1)", "# Excel"), None);
    }

    #[test]
    fn a_url_returned_twice_by_one_query_agreed_once() {
        // The same URL in one result list is one appearance; the same URL in two is two.
        let by_host = appearances(&[
            vec![hit(EXCEL), hit(EXCEL), hit(PROJECT)],
            vec![hit(EXCEL)],
            vec![hit("https://www.g2.com/categories/spreadsheets")],
        ]);
        let microsoft = by_host.get("microsoft.com").expect("a candidate");
        let excel = microsoft.iter().find(|a| a.url == EXCEL).expect("excel");
        assert_eq!(excel.queries, vec![0, 1]);
        let project = microsoft
            .iter()
            .find(|a| a.url == PROJECT)
            .expect("project");
        assert_eq!(project.queries, vec![0]);
        // The publisher rule is the pipeline's, called rather than copied.
        assert!(!by_host.contains_key("g2.com"));
    }

    #[tokio::test]
    async fn a_suite_is_split_and_the_product_keeps_only_what_it_earned() {
        // The reported failure in miniature: two Project pages and one Teams page, and a group
        // that agreed with three queries because they share a domain.
        let results = vec![vec![hit(PROJECT)], vec![hit(TEAMS)], vec![hit(PROJECT)]];
        let found = from_results(&results, 3);
        assert_eq!(
            found[0].agreed, 3,
            "before: the domain agreed with all three"
        );

        let split = split(
            found,
            3,
            &results,
            &pages(&[
                (
                    PROJECT,
                    "# Microsoft Project\n\nProject management software.",
                ),
                (TEAMS, "# Microsoft Teams\n\nGroup chat."),
            ]),
        )
        .await;
        assert_eq!(split.len(), 1, "one domain is still one candidate");
        assert_eq!(split[0].agreed, 2, "Project agreed with two, not three");
        let declared = split[0].declared.as_ref().expect("a name from the page");
        assert_eq!(declared.name, "Microsoft Project");
        assert!(split[0].shallowest.contains("/project/"));
    }

    #[tokio::test]
    async fn one_product_at_four_urls_stays_one_candidate() {
        // The trap the first draft of this change would have fallen into: raw URLs as
        // candidates turn one corroborated product into three refused ones.
        let results = vec![
            vec![hit(EXCEL)],
            vec![hit(EXCEL_PRICING)],
            vec![hit(EXCEL_DE)],
        ];
        let split = split(
            from_results(&results, 3),
            3,
            &results,
            &pages(&[
                (EXCEL, "# Microsoft Excel\n\nThe spreadsheet."),
                (EXCEL_PRICING, "# Microsoft Excel\n\nPlans and pricing."),
                (EXCEL_DE, "# Microsoft Excel\n\nDie Tabellenkalkulation."),
            ]),
        )
        .await;
        assert_eq!(split.len(), 1);
        assert_eq!(
            split[0].agreed, 3,
            "three locales of one product agreed three times"
        );
        assert_eq!(
            split[0].shallowest, EXCEL,
            "the shallowest page of that product"
        );
    }

    #[tokio::test]
    async fn two_pages_of_one_product_in_one_result_list_agreed_once() {
        // **The corroboration trap, on the other side of the merge.** Grouping by domain counts
        // agreement once per query; grouping by product has to do the same, or a query that
        // returned a product page and its pricing page has corroborated itself.
        let results = vec![vec![hit(EXCEL), hit(EXCEL_PRICING)], vec![hit(TEAMS)]];
        let split = split(
            from_results(&results, 2),
            2,
            &results,
            &pages(&[
                (
                    EXCEL,
                    "# Microsoft Excel

The spreadsheet.",
                ),
                (
                    EXCEL_PRICING,
                    "# Microsoft Excel

Plans and pricing.",
                ),
                (
                    TEAMS,
                    "# Microsoft Teams

Group chat.",
                ),
            ]),
        )
        .await;
        assert_eq!(
            split[0].agreed, 1,
            "one query returned both Excel pages, so Excel agreed with one query"
        );
    }

    #[tokio::test]
    async fn a_front_page_in_the_results_is_the_evidence_already() {
        // Nothing is fetched here, and the candidate is unchanged: `describe` is about to read
        // this company's front page because that is what a search returned.
        let results = vec![
            vec![hit("https://asana.com/uses/design-teams")],
            vec![hit("https://asana.com/")],
        ];
        let before = from_results(&results, 2);
        let after = split(
            before.clone(),
            2,
            &results,
            &pages(&[("https://asana.com/", "# Asana\n\nWork management.")]),
        )
        .await;
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn a_page_nobody_could_read_cannot_split_anybody() {
        // The fallback the whole module rests on. Two products, neither page readable: the
        // candidate must come out exactly as the domain rule left it.
        let results = vec![vec![hit(PROJECT)], vec![hit(TEAMS)], vec![hit(PROJECT)]];
        let before = from_results(&results, 3);
        let after = split(before.clone(), 3, &results, &pages(&[])).await;
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn a_domain_that_would_cost_more_than_the_budget_is_left_whole() {
        // Half a split attributes the unread appearances to whichever product was fetched
        // first, which is worse than not splitting at all.
        let many: Vec<Hit> = (0..=SPLIT_BUDGET + 1)
            .map(|i| hit(&format!("https://big.example/product/p{i}")))
            .collect();
        let results = vec![many];
        let before = from_results(&results, 1);
        let after = split(before.clone(), 1, &results, &pages(&[])).await;
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn the_budget_is_spent_in_rank_order() {
        // Two splittable domains and room for one. The stronger candidate gets the reads, and
        // the weaker one keeps the behavior it had before this module existed.
        // `strong` costs the whole budget - one URL per query - and `weak` costs one more
        // than is left.
        let strong: Vec<String> = (0..=SPLIT_BUDGET)
            .map(|i| format!("https://strong.example/a/p{i}"))
            .collect();
        let weak = ["https://weak.example/b/one", "https://weak.example/b/two"];
        let mut results: Vec<Vec<Hit>> = Vec::new();
        for query in 0..=SPLIT_BUDGET {
            results.push(vec![hit(&strong[query]), hit(weak[query % 2])]);
        }
        // Both domains agreed with every query, so rank is decided by host and `strong` wins.
        let known: Vec<(String, &str)> = strong
            .iter()
            .cloned()
            .chain(weak.iter().map(|w| (*w).to_owned()))
            .map(|url| {
                (
                    url,
                    "# One

A product.",
                )
            })
            .collect();
        let sent = results.len();
        let after = split(
            from_results(&results, sent),
            sent,
            &results,
            &owned_pages(&known),
        )
        .await;
        let by = |host: &str| {
            after
                .iter()
                .find(|f| f.host == host)
                .cloned()
                .expect("both hosts are candidates")
        };
        assert!(
            by("strong.example").declared.is_some(),
            "the stronger one was read"
        );
        assert!(
            by("weak.example").declared.is_none(),
            "and the budget was gone by the time we reached the other"
        );
    }
}
