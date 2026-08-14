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
//! # A front page is about the vendor, and does not stop a split
//!
//! The first version refused to split any domain a search had returned at its root. Review found
//! that this **kept the reported failure whole**: `[microsoft.com/, /project, /teams]` stayed at
//! three of three, with Teams still corroborating a project-management vendor. A front page says
//! what the *company* is — it is no evidence that every other page on the domain is the same
//! product. It is now one appearance among the others, keyed to the vendor, and its read is the
//! front-page read [`crate::candidates::describe`] would have made anyway.
//!
//! # What this does not do
//!
//! **It does not put two products of one domain in one report.** The strongest product becomes
//! the candidate and the rest of the domain's appearances stop corroborating it; a set holding
//! *Microsoft Project* **and** *Microsoft Planner* as two rows needs
//! [`landscape_core::Candidate`] to stop being keyed on a domain, which is a change with a
//! blast radius well outside this one. What a reader gets today is the right product, named,
//! with the agreement that product actually earned.
//!
//! **And it does not guess between two products the evidence ranks equally.** See [`strongest`]:
//! a tie keeps the vendor rather than the product whose heading sorts first.

use std::collections::HashMap;

use landscape_fetch::Target;

use crate::candidates::{registrable, Found};
use crate::Hit;

/// How many **extra** page reads one analysis will spend telling products apart.
///
/// **Extra, not total, and the saving has to be earned.** A domain whose results are all
/// non-root pages costs `k` reads here, and saves the one [`crate::candidates::describe`] would
/// have spent on its front page **only if one of those reads produced a name** — a candidate
/// nobody could read still needs its front page fetched.
///
/// Review found the first version charging `k - 1` before knowing that. A single unreadable
/// product page then cost one read and refunded nothing, and because the arithmetic ran before
/// the fetch, **any number of such candidates could each add a request while the budget sat at
/// four**. Reads are reserved in full now and the front page is refunded once it is genuinely
/// not going to be fetched.
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
/// **The rule for the root.** When a search returns `asana.com`, the company and the product are
/// one thing and one name is right; only a non-root URL can produce *Microsoft Project, by
/// Microsoft*. So a root appearance is grouped as the **vendor** rather than as a product — and
/// review found the first version going further than that and refusing to split the domain at
/// all, which left the reported failure exactly where it was.
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
/// A domain qualifies when it is within the [`crate::candidates::NAMED`] candidates whose pages
/// are read at all, and when reading all of its URLs fits in what is left of [`SPLIT_BUDGET`].
/// Everything else is returned untouched, so a run with no budget left or a page nobody could
/// read behaves exactly as it did before this module existed.
///
/// **Every read is charged, and the front page is refunded only when it is really saved.** See
/// [`SPLIT_BUDGET`] for what charging `k - 1` up front cost.
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

    for (rank, one) in found.into_iter().enumerate() {
        // **Nobody below the fetch budget is read here either.** `describe` stops at `NAMED`,
        // and a stage that reads pages it does not is a stage that spends strangers' bandwidth
        // on candidates a reader will never be shown.
        if rank >= crate::candidates::NAMED {
            out.push(one);
            continue;
        }
        let Some(urls) = by_host.get(&one.host) else {
            out.push(one);
            continue;
        };
        // `k` reads for `k` URLs, charged before any of them happen. A domain that does not
        // fit is left whole: half a split attributes the unread appearances to the wrong
        // product.
        if urls.is_empty() || urls.len() > budget {
            out.push(one);
            continue;
        }
        budget -= urls.len();

        let mut read: Vec<(&Appearance, Option<String>)> = Vec::with_capacity(urls.len());
        for at in urls {
            // **A root is read as the front page, at the URL `describe` would have used.** The
            // engine may return `https://www.example.com/` and the front page is built from the
            // registrable host; asking for the returned form would be a second request for the
            // same page, and the refund below assumes there is only one.
            let url = if is_root(&at.url) {
                crate::candidates::home_page(&one.host)
            } else {
                at.url.clone()
            };
            read.push((at, fetch(url).await));
        }
        let candidate = strongest(&one, asked, &read);
        // **The refund, and only now can it be known.** A candidate that came out of there with
        // a name will not have its front page fetched by `describe`, so one of these reads
        // replaced a read rather than adding one. A candidate that did not still costs the
        // front page, and the budget has to say so.
        if candidate.declared.is_some() {
            budget += 1;
        }
        out.push(candidate);
    }

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.host.cmp(&b.host))
    });
    out
}

/// The one product on this domain that the queries agreed about most, when the evidence says
/// there is one.
///
/// **Three groupings, and only the middle one is a product.**
///
/// | An appearance at | Keys to | Because |
/// |---|---|---|
/// | The domain's **root** | The vendor | A front page says what the *company* is, not which product |
/// | A page that **declares a name** | `domain#name` | That is the identity rule |
/// | A page that declares **nothing** | The vendor | Nothing about it says it is a different product |
///
/// So a host whose pages could not be read comes out of here exactly as it went in. That is the
/// fallback the whole module rests on: **an unreadable page must never split a candidate.**
///
/// # When it declines to split
///
/// **Fewer than two products is nothing to separate.** A vendor whose results are its front page
/// and one feature page """ + D + """ `freshbooks.com/` and `freshbooks.com/invoice` """ + D + """ is one company that
/// sells one thing, and cutting its agreement in half because a feature page has its own heading
/// would drop it from the answer. It keeps the agreement it had, and takes its name from its
/// front page when a query returned one.
///
/// **Two products with equal support is a question, not a choice.** Review found the first
/// version breaking that tie on the identity key, which is alphabetical order of an `h1` """ + D + """ so
/// whether a vendor survived [`crate::competitors::assemble`] depended on which of two equally
/// supported products sorted first. It now keeps **no** product: the candidate is the vendor, at
/// the support the tie actually shows, with no name of its own so
/// [`crate::candidates::describe`] reads the front page. A tie between two one-query products is
/// a vendor with one query behind it, which is what [`crate::candidates::CORROBORATION`] is for.
fn strongest(one: &Found, asked: usize, read: &[(&Appearance, Option<String>)]) -> Found {
    /// One identity's share of a domain's appearances.
    ///
    /// **`page` is the page at `shallowest`, and the two move together.** Review found them
    /// moving apart: the page was kept from the first readable appearance and the URL later
    /// slid to a shallower one, so a group whose pricing page arrived first was linked by its
    /// product URL and *named* by its pricing page. That is the *"Pricing"*-as-a-company-name
    /// defect this codebase already fixed once, recreated one level down.
    struct Grouped {
        queries: Vec<usize>,
        shallowest: String,
        page: Option<String>,
    }

    let mut by_identity: HashMap<String, Grouped> = HashMap::new();
    for (at, page) in read {
        let home = is_root(&at.url);
        let declared = if home {
            None
        } else {
            page.as_deref()
                .and_then(|markdown| declared_for(&at.url, markdown))
        };
        // **A page that declares nothing is not evidence, even though it arrived.** It keys to
        // the vendor, and it must not become the name either: a readable page with no heading
        // would otherwise name the candidate after its own host and skip the front page that
        // would have named it properly. A **front** page is evidence about the vendor, which is
        // the group it lands in.
        let evidence = if home {
            page.clone()
        } else {
            declared.as_ref().and(page.clone())
        };
        let key = declared.unwrap_or_else(|| one.host.clone());
        let entry = by_identity.entry(key).or_insert_with(|| Grouped {
            queries: Vec::new(),
            shallowest: at.url.clone(),
            page: evidence.clone(),
        });
        for query in &at.queries {
            if !entry.queries.contains(query) {
                entry.queries.push(*query);
            }
        }
        if crate::candidates::depth(&at.url) < crate::candidates::depth(&entry.shallowest) {
            entry.shallowest = at.url.clone();
            entry.page = evidence;
        }
    }

    let vendor = by_identity.remove(&one.host);
    // Most queries wins; then the shallowest URL. The key is last and only for determinism """ + D + """
    // a tie that reaches it is resolved by declining to split, below.
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

    let named = |page: Option<String>| {
        page.map(|markdown| {
            let (name, what_it_is) = crate::candidates::naming(&one.host, &markdown);
            Declared {
                name,
                what_it_is,
                page: markdown,
            }
        })
    };

    // **Nothing to separate.** The vendor keeps the agreement it already had, named by its own
    // front page when a query returned one and by the single product page otherwise.
    if products.len() < 2 {
        let evidence = vendor
            .and_then(|v| v.page)
            .or_else(|| products.first().and_then(|(_, g)| g.page.clone()));
        return Found {
            declared: named(evidence),
            ..one.clone()
        };
    }

    let tied = products[0].1.queries.len() == products[1].1.queries.len()
        && crate::candidates::depth(&products[0].1.shallowest)
            == crate::candidates::depth(&products[1].1.shallowest);
    if tied {
        // The vendor, at whichever support is larger: its own front page's, or the one a tied
        // product earned. Never the sum, which is the corroboration this module removes.
        let agreed = vendor
            .as_ref()
            .map_or(0, |v| v.queries.len())
            .max(products[0].1.queries.len());
        return Found {
            confidence: crate::candidates::score(
                agreed,
                asked,
                crate::candidates::depth(&one.shallowest),
            ),
            agreed,
            declared: named(vendor.and_then(|v| v.page)),
            ..one.clone()
        };
    }

    let (_, best) = products.remove(0);
    let agreed = best.queries.len();
    Found {
        confidence: crate::candidates::score(
            agreed,
            asked,
            crate::candidates::depth(&best.shallowest),
        ),
        host: one.host.clone(),
        agreed,
        shallowest: best.shallowest,
        declared: named(best.page),
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
        // A use-case page that declares nothing and the company's own front page are one
        // company. The candidate is unchanged in every way a reader would notice, and the read
        // it cost is the front-page read `describe` no longer has to make.
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
        assert_eq!(after[0].agreed, before[0].agreed, "still one company");
        assert_eq!(after[0].shallowest, before[0].shallowest);
        assert_eq!(after[0].declared.as_ref().expect("named").name, "Asana");
    }

    #[tokio::test]
    async fn a_root_result_names_the_vendor_and_does_not_stop_the_split() {
        // **The reported failure with its front page in the results, which review found still
        // broken.** The first version refused to split any domain a search had returned at its
        // root, so `[microsoft.com/, /project, /project, /teams]` kept Microsoft at three of
        // three and let the Teams appearance corroborate a project-management vendor - cause 2
        // exactly. A front page says what the *company* is; it is not evidence that every other
        // page on the domain is the same product.
        let home = "https://www.microsoft.com/";
        let results = vec![
            vec![hit(PROJECT)],
            vec![hit(home)],
            vec![hit(PROJECT), hit(TEAMS)],
        ];
        let before = from_results(&results, 3);
        assert_eq!(
            before[0].agreed, 3,
            "before: the domain agreed with all three"
        );

        let after = split(
            before,
            3,
            &results,
            &pages(&[
                (
                    "https://microsoft.com/",
                    "# Microsoft\n\nCloud, computers and more.",
                ),
                (
                    PROJECT,
                    "# Microsoft Project\n\nProject management software.",
                ),
                (TEAMS, "# Microsoft Teams\n\nGroup chat."),
            ]),
        )
        .await;
        assert_eq!(after[0].agreed, 2, "Project earned two of the three");
        assert_eq!(
            after[0].declared.as_ref().expect("named").name,
            "Microsoft Project"
        );
    }

    #[tokio::test]
    async fn a_front_page_and_one_feature_page_are_still_one_company() {
        // **The other half of the same rule.** FreshBooks' front page and its invoicing page are
        // one company selling one thing; halving its agreement because a feature page carries
        // its own heading would drop it out of the answer entirely. Fewer than two products is
        // nothing to separate.
        let home = "https://www.freshbooks.com/";
        let invoice = "https://www.freshbooks.com/invoice";
        let results = vec![vec![hit(home)], vec![hit(invoice)]];
        let before = from_results(&results, 2);
        let after = split(
            before.clone(),
            2,
            &results,
            &pages(&[
                (
                    "https://freshbooks.com/",
                    "# FreshBooks\n\nInvoicing and accounting.",
                ),
                (invoice, "# Invoicing\n\nSend an invoice, get paid."),
            ]),
        )
        .await;
        assert_eq!(after[0].agreed, before[0].agreed, "still two of two");
        assert_eq!(
            after[0].declared.as_ref().expect("named").name,
            "FreshBooks",
            "and named by its own front page, not by a feature"
        );
    }

    #[tokio::test]
    async fn two_products_with_equal_support_are_a_question_rather_than_a_choice() {
        // **Review's second finding.** The tie used to be broken on the identity key, which is
        // alphabetical order of an `h1` - so whether a vendor survived the fit test downstream
        // depended on which of two equally supported products sorted first. Both orders and both
        // namings are run here and must agree, and neither product may be presented as the
        // strongest.
        let mut answers = Vec::new();
        for names in [
            ["# Alpha Board", "# Zeta Board"],
            ["# Zeta Board", "# Alpha Board"],
        ] {
            let one = "https://vendor.example/products/one";
            let two = "https://vendor.example/products/two";
            let results = vec![vec![hit(one)], vec![hit(two)]];
            let after = split(
                from_results(&results, 2),
                2,
                &results,
                &pages(&[(one, names[0]), (two, names[1])]),
            )
            .await;
            let candidate = after.into_iter().next().expect("one candidate");
            assert!(
                candidate.declared.is_none(),
                "neither product is the vendor's answer: {:?}",
                candidate.declared
            );
            assert_eq!(
                candidate.agreed, 1,
                "two one-query products is a vendor with one query behind it"
            );
            answers.push(candidate);
        }
        assert_eq!(
            answers[0], answers[1],
            "and the result cannot depend on which name sorts first"
        );
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

    /// Every URL a `fetch` was asked for, shared with the test that made it.
    type Asked = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

    /// A `fetch` that answers from a table and counts every request it was given.
    fn counting(
        known: &[(String, &'static str)],
    ) -> (impl Fn(String) -> std::future::Ready<Option<String>>, Asked) {
        let known: Vec<(String, String)> = known
            .iter()
            .map(|(u, p)| (u.clone(), (*p).to_owned()))
            .collect();
        let asked: Asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = std::sync::Arc::clone(&asked);
        let fetch = move |url: String| {
            seen.lock().expect("not poisoned").push(url.clone());
            std::future::ready(
                known
                    .iter()
                    .find(|(at, _)| *at == url)
                    .map(|(_, page)| page.clone()),
            )
        };
        (fetch, asked)
    }

    #[tokio::test]
    async fn a_page_that_could_not_be_read_still_costs_its_front_page() {
        // **The budget hole review found.** `split` used to charge `k - 1` before fetching, on
        // the assumption that a product page always replaces the front page `describe` would
        // have read. It only replaces it if it produced a name. Six one-URL domains whose pages
        // all fail therefore cost one read each and refunded nothing, while the budget sat at
        // four - so the cap bounded nothing.
        let hosts: Vec<String> = (0..SPLIT_BUDGET + 2)
            .map(|i| format!("https://d{i}.example/product/thing"))
            .collect();
        let results: Vec<Vec<Hit>> = vec![hosts.iter().map(|u| hit(u)).collect()];
        let (fetch, asked) = counting(&[]);
        let after = split(from_results(&results, 1), 1, &results, &fetch).await;

        let reads = asked.lock().expect("not poisoned").len();
        assert!(
            reads <= SPLIT_BUDGET,
            "{reads} reads for a budget of {SPLIT_BUDGET}"
        );
        // And every one of them is unchanged, because nothing was learned about any of them.
        assert!(after.iter().all(|f| f.declared.is_none()));
    }

    #[tokio::test]
    async fn a_read_that_named_somebody_is_refunded_and_one_that_did_not_is_not() {
        // The other half of the same arithmetic: a domain whose page *does* declare a name
        // costs nothing extra, because `describe` will not read its front page.
        let named: Vec<String> = (0..SPLIT_BUDGET + 2)
            .map(|i| format!("https://n{i}.example/product/thing"))
            .collect();
        let results: Vec<Vec<Hit>> = vec![named.iter().map(|u| hit(u)).collect()];
        let known: Vec<(String, &str)> = named
            .iter()
            .cloned()
            .map(|url| (url, "# A Product\n\nWhat it does."))
            .collect();
        let (fetch, asked) = counting(&known);
        let after = split(from_results(&results, 1), 1, &results, &fetch).await;

        assert_eq!(
            asked.lock().expect("not poisoned").len(),
            crate::candidates::NAMED,
            "every one is free - each read replaces a front-page read - and reading still stops \
             where `describe` does"
        );
        assert_eq!(
            after.iter().filter(|f| f.declared.is_some()).count(),
            crate::candidates::NAMED
        );
    }

    #[tokio::test]
    async fn the_page_that_names_a_product_is_the_page_at_the_url_it_links_to() {
        // **Review's second finding, and it is the "Pricing" defect one level down.** The page
        // was kept from the first readable appearance while the URL slid to a shallower one, so
        // a group whose pricing page arrived first linked to the product and was named by the
        // pricing page. Both orders are run here, and they must agree.
        let product = "# Microsoft Excel\n\nThe spreadsheet finance teams use.";
        let pricing = "# Microsoft Excel\n\nPlans and pricing.";
        let mut both = Vec::new();
        for order in [[EXCEL_PRICING, EXCEL], [EXCEL, EXCEL_PRICING]] {
            let results = vec![vec![hit(order[0])], vec![hit(order[1])]];
            let split = split(
                from_results(&results, 2),
                2,
                &results,
                &pages(&[(EXCEL, product), (EXCEL_PRICING, pricing)]),
            )
            .await;
            let one = split.into_iter().next().expect("one candidate");
            let declared = one.declared.expect("a name from a page");
            both.push((one.shallowest, declared.what_it_is, declared.page));
        }
        assert_eq!(
            both[0], both[1],
            "the result must not depend on result order"
        );
        assert_eq!(both[0].0, EXCEL, "it links to the product page");
        assert!(
            both[0].1.contains("finance teams"),
            "and it is described by the same page it links to, not by the pricing page: {}",
            both[0].1
        );
    }

    #[tokio::test]
    async fn the_budget_is_spent_in_rank_order() {
        // Two splittable domains and room for one. The stronger candidate gets the reads, and
        // the weaker one keeps the behavior it had before this module existed.
        // `strong` takes the whole budget - `SPLIT_BUDGET` URLs, one refunded because it ends
        // up named - and `weak` needs two where one is left.
        let strong: Vec<String> = (0..SPLIT_BUDGET)
            .map(|i| format!("https://strong.example/a/p{i}"))
            .collect();
        let weak = ["https://weak.example/b/one", "https://weak.example/b/two"];
        let mut results: Vec<Vec<Hit>> = Vec::new();
        for query in 0..SPLIT_BUDGET {
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
