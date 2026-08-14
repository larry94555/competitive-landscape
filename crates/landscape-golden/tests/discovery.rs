//! What the discovery pipeline does today, recorded so a change has something to beat.
//!
//! `cargo test -p landscape-golden --test discovery -- --nocapture` prints both tables.
//!
//! **These tests pass while the answer is bad.** That is deliberate: a red suite is a thing
//! people learn to ignore, and what is wanted here is a *number* that a pull request has to move
//! and cannot move by accident. `the_baseline_has_not_changed_by_accident` is the one that fails
//! when a change alters discovery — and the fix for that failure is to read the printed table,
//! decide the change is an improvement, and edit the recorded values.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use landscape_golden::discovery::{markets, score, Identity, Market, Scored};
use landscape_search::competitors::Aside;

/// What one fixture scores today, exactly.
///
/// **Exact values, not counts, and this got stronger twice under review.** The first version
/// stored three numbers, so swapping *which* expected company came back passed unchanged. The
/// second recorded the found and missed lists but still counted impostors and ignored the
/// exclusions entirely — so swapping *which* impostor got in, or an expected company sliding
/// from `Uncorroborated` to `Unread`, both stayed green. Those are precisely the accidental
/// changes this file claims to catch.
struct Recorded {
    id: &'static str,
    found: Vec<&'static str>,
    missed: Vec<&'static str>,
    admitted: Vec<&'static str>,
    /// Host and **typed** reason, sorted by host. Not the sentence: a wording change must not
    /// fail a test about discovery, and a sentence cannot be compared field by field.
    set_aside: Vec<(&'static str, Aside)>,
}

fn uncorroborated() -> Aside {
    Aside::Uncorroborated {
        agreed: 1,
        asked: 3,
    }
}

fn elsewhere(words: &[&str]) -> Aside {
    Aside::ElsewhereEntirely {
        looked_for: words.iter().map(|w| (*w).to_owned()).collect(),
    }
}

/// The baseline, as of `BENCHMARKS.md` Run 51.
///
/// **A recorded baseline, not a target.** Nothing here is a claim that these numbers are good;
/// most of them are bad on purpose, because the fixtures hold defects the roadmap has not fixed
/// yet.
fn baseline() -> Vec<Recorded> {
    vec![
        Recorded {
            id: "project-management-for-agencies",
            found: vec!["asana.com", "microsoft.com"],
            missed: vec!["workamajig.com"],
            admitted: vec!["projectplusgame.com"],
            set_aside: vec![
                ("notion.so", uncorroborated()),
                ("workamajig.com", uncorroborated()),
            ],
        },
        Recorded {
            id: "one-product-many-urls",
            found: vec!["microsoft.com", "google.com"],
            missed: vec!["airtable.com"],
            admitted: vec![],
            set_aside: vec![("airtable.com", uncorroborated())],
        },
        // **Measured end to end, and it disagrees with the ranking-only number.** The first
        // version of this file scored `from_results` alone and recorded an impostor admitted
        // here; running `assemble` too, the fit test *does* catch `trackingtimemusic.com` — its
        // page shares none of "time", "tracking", "consultants". One word is enough for
        // `projectplusgame.com` and not for this one, which is a finer account of cause 4 than
        // the roadmap had — and it leaves PR 4 needing a fixture that survives one shared word.
        Recorded {
            id: "keyword-impostor",
            found: vec!["harvestapp.com"],
            missed: vec!["toggl.com"],
            admitted: vec![],
            set_aside: vec![
                ("toggl.com", uncorroborated()),
                (
                    "trackingtimemusic.com",
                    elsewhere(&["time", "tracking", "consultants"]),
                ),
            ],
        },
        Recorded {
            id: "specialist-in-one-article",
            found: vec!["freshbooks.com", "intuit.com"],
            missed: vec!["protemos.com"],
            admitted: vec![],
            set_aside: vec![("protemos.com", uncorroborated())],
        },
        Recorded {
            id: "publisher-heavy",
            found: vec!["zendesk.com"],
            missed: vec!["helpscout.com"],
            admitted: vec![],
            // **The other silence.** Not "we looked and it was elsewhere" — we could not look.
            set_aside: vec![("helpscout.com", Aside::Unread)],
        },
    ]
}

async fn scored() -> Vec<(Market, Scored)> {
    let mut out = Vec::new();
    for market in markets() {
        let s = score(&market).await;
        out.push((market, s));
    }
    out
}

/// Host and reason, sorted, so a comparison does not depend on the order a set happens to hold.
fn exclusions(s: &Scored) -> Vec<(String, Aside)> {
    let mut all: Vec<(String, Aside)> = s.set_aside.clone();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

#[tokio::test]
async fn the_baseline_has_not_changed_by_accident() {
    let results = scored().await;

    println!("\nDISCOVERY, AS IT RUNS TODAY\n");
    for (_, s) in &results {
        println!(
            "{:<34} recall {:>3.0}%   returned: {}",
            s.id,
            s.recall() * 100.0,
            s.returned.join(", ")
        );
        for (host, why) in &s.set_aside {
            println!("      set aside  {host}: {}", why.sentence());
        }
    }
    let total: f64 = results.iter().map(|(_, s)| s.recall()).sum();
    #[expect(clippy::cast_precision_loss, reason = "five fixtures")]
    let mean = total / results.len() as f64;
    let admitted: usize = results.iter().map(|(_, s)| s.admitted.len()).sum();
    println!(
        "\nmean recall {:.0}%   impostors admitted {admitted}\n",
        mean * 100.0
    );

    // **The set, not just its rows.** A sixth fixture used to pass unmentioned, because this
    // walked the baseline rather than the markets.
    let mut in_set: Vec<&str> = results.iter().map(|(_, s)| s.id).collect();
    let mut recorded: Vec<&str> = baseline().iter().map(|r| r.id).collect();
    in_set.sort_unstable();
    recorded.sort_unstable();
    assert_eq!(
        in_set, recorded,
        "a fixture was added or removed without recording what it scores"
    );

    for want in baseline() {
        let (_, got) = results
            .iter()
            .find(|(_, s)| s.id == want.id)
            .unwrap_or_else(|| panic!("{} is in the baseline and not in the set", want.id));
        assert_eq!(
            got.found, want.found,
            concat!(
                "\n{}: a different company came back. Read the table above, ",
                "decide whether it is an improvement, and edit baseline()."
            ),
            want.id
        );
        assert_eq!(
            got.missed, want.missed,
            "\n{}: a different company is missing.",
            want.id
        );
        let admitted: Vec<&str> = got.admitted.iter().map(|(host, _)| *host).collect();
        assert_eq!(
            admitted, want.admitted,
            "\n{}: a different impostor got in.",
            want.id
        );
        let want_aside: Vec<(String, Aside)> = want
            .set_aside
            .into_iter()
            .map(|(host, why)| (host.to_owned(), why))
            .collect();
        assert_eq!(
            exclusions(got),
            want_aside,
            "\n{}: somebody was excluded for a different reason.",
            want.id
        );
    }
}

#[tokio::test]
async fn the_reported_failure_is_reproduced() {
    // **The specific answer a reader got**, held as a test rather than as a story in a
    // benchmark. If a change makes this fixture pass, that is the thing to celebrate; if it
    // makes it pass *and* nothing else moves, it is worth suspecting.
    let market = markets()
        .into_iter()
        .find(|m| m.id == "project-management-for-agencies")
        .expect("the reported failure is in the set");
    let got = score(&market).await;

    assert!(
        got.admitted
            .iter()
            .any(|(h, _)| *h == "projectplusgame.com"),
        "the impostor no longer gets in, which is PR 4's job: {:?}",
        got.returned
    );
    assert!(
        got.missed.contains(&"workamajig.com"),
        "the specialist is no longer missed, which is PR 3 and 5's job: {:?}",
        got.returned
    );
    assert!(
        got.returned.iter().any(|h| h == "microsoft.com"),
        "microsoft.com is the domain-collapse defect and should still be here until PR 3: {:?}",
        got.returned
    );
}

#[test]
fn no_page_is_evidence_the_engine_never_returned() {
    // **The alibi check.** A frozen page for a URL no query returned would let an identity rule
    // be proved on evidence production never has. Three register entries in this repository are
    // a fixture that supplied what the real thing does not send.
    for market in markets() {
        let returned: Vec<&str> = market
            .results
            .iter()
            .flatten()
            .map(|h| h.url.as_str())
            .collect();
        for (url, _) in &market.product_pages {
            assert!(
                returned.contains(url),
                "{}: {url} has a frozen page and no query returned it",
                market.id
            );
        }
    }
}

#[test]
fn the_identity_rules_are_compared_rather_than_chosen() {
    // **PR 3's open question, made runnable.** The roadmap lists five candidate rules for what
    // makes two URLs the same product and says the golden set should choose. This prints what
    // each one does to the fixtures' URLs; it asserts only the two things that are decidable
    // without an opinion.
    let urls: Vec<String> = markets()
        .iter()
        .flat_map(|m| m.results.iter().flatten())
        .map(|h| h.url.clone())
        .collect();

    println!("\nHOW EACH IDENTITY RULE GROUPS THE FIXTURE URLS\n");
    for rule in [
        Identity::Domain,
        Identity::FirstSegment,
        Identity::FirstMeaningfulSegment,
        Identity::LastSegment,
    ] {
        let mut keys: Vec<String> = urls.iter().filter_map(|u| rule.key_for(u)).collect();
        keys.sort();
        keys.dedup();
        println!("{rule:?}  ->  {} distinct candidates", keys.len());
        for key in keys.iter().filter(|k| k.contains("microsoft")) {
            println!("      {key}");
        }
    }
    println!();

    let excel = "https://www.microsoft.com/en-us/microsoft-365/excel";
    let excel_de = "https://www.microsoft.com/de-de/microsoft-365/excel";
    let project =
        "https://www.microsoft.com/en-us/microsoft-365/project/project-management-software";
    let teams = "https://www.microsoft.com/en-us/microsoft-365/teams/group-chat-software";

    // Today's rule cannot tell any Microsoft product from any other. This is cause 2.
    assert_eq!(
        Identity::Domain.key_for(excel),
        Identity::Domain.key_for(teams),
        "the domain rule has stopped collapsing products, which would be PR 3 landing"
    );

    // **The rule the roadmap's second draft asserted, failing exactly as review said.** The
    // first segment is a locale, so it merges every product *and* splits one across locales.
    assert_eq!(
        Identity::FirstSegment.key_for(excel),
        Identity::FirstSegment.key_for(teams),
        "first-segment should still merge two different products - it groups on the locale"
    );
    assert_ne!(
        Identity::FirstSegment.key_for(excel),
        Identity::FirstSegment.key_for(excel_de),
        "first-segment should still split one product across locales"
    );

    // **Stripping locales and containers fixes half of it and not the other half.** It joins
    // one product across locales, which the rule above could not do:
    assert_eq!(
        Identity::FirstMeaningfulSegment.key_for(excel),
        Identity::FirstMeaningfulSegment.key_for(excel_de),
        "stripping locale and container should join one product across locales"
    );
    // ...and still cannot tell Excel from Project, because `microsoft-365` is a **suite**, not
    // a container this or any list can enumerate. Both key to `microsoft.com/microsoft-365`.
    //
    // **This is the finding that changes PR 3.** I expected this rule to be the answer and
    // wrote the roadmap leaning on it; it is not, and the harness said so before a line of PR 3
    // was written. Suite names are unbounded - `microsoft-365`, `google-workspace`,
    // `adobe-creative-cloud` - so the "known prefixes" list is not a list that can be finished.
    assert_eq!(
        Identity::FirstMeaningfulSegment.key_for(excel),
        Identity::FirstMeaningfulSegment.key_for(project),
        concat!(
            "if this ever separates them, a suite name was added to CONTAINERS ",
            "and the rule is now only as good as that list"
        )
    );

    // The last-segment rule separates the two products...
    assert_ne!(
        Identity::LastSegment.key_for(excel),
        Identity::LastSegment.key_for(project),
        "last-segment should tell two products apart"
    );
    // ...and splits one product's own pages, which is the other half of the same problem.
    assert_ne!(
        Identity::LastSegment.key_for(excel),
        Identity::LastSegment
            .key_for("https://www.microsoft.com/en-us/microsoft-365/excel/pricing"),
        "last-segment should split a product from its own pricing page"
    );

    // **So no path-shaped rule in the roadmap does both**, on these fixtures: each one either
    // merges two products or splits one. That is a result about the *approach*, not a tuning
    // problem, and it is what PR 3 has to answer before it is written.
}

/// The declared-identity rule, scored on the three things it has to get right.
///
/// **Review's two corrections, both real.** The first version concluded this rule was the answer
/// by eliminating four others — an inference, not a measurement, and the pull request said
/// *"PR 2 selected it"*. The second implemented it as the bare heading and tested it on three
/// strings written in the test file: that keys two vendors who both call their product
/// *Invoicing* to one company, and no fixture could have caught it, because the only pair
/// compared was two Microsoft products on one domain.
///
/// So it is now the domain **and** the declared name, and every case comes from `product_pages`
/// — pages of URLs the fixtures' queries really returned.
#[test]
fn the_declared_identity_is_scored_on_fixture_pages() {
    let all: Vec<(String, String)> = markets()
        .iter()
        .flat_map(|m| m.product_pages.clone())
        .map(|(url, page)| {
            (
                url.to_owned(),
                Identity::declared_for(url, page)
                    .unwrap_or_else(|| panic!("{url} has a frozen page and no declared identity")),
            )
        })
        .collect();

    let key = |url: &str| -> String {
        all.iter()
            .find(|(at, _)| at == url)
            .unwrap_or_else(|| panic!("no fixture page for {url}"))
            .1
            .clone()
    };

    println!(
        "
WHAT EACH FIXTURE PAGE DECLARES
"
    );
    for (url, k) in &all {
        println!("  {k:<44} {url}");
    }
    println!();

    let excel = "https://www.microsoft.com/en-us/microsoft-365/excel";
    let excel_de = "https://www.microsoft.com/de-de/microsoft-365/excel";
    let excel_pricing = "https://www.microsoft.com/en-us/microsoft-365/excel/pricing";
    let project =
        "https://www.microsoft.com/en-us/microsoft-365/project/project-management-software";
    let project_gb =
        "https://www.microsoft.com/en-gb/microsoft-365/project/project-management-software";
    let teams = "https://www.microsoft.com/en-us/microsoft-365/teams/group-chat-software";

    // **One.** Two locales of one product are one product. No first-segment rule does this.
    assert_eq!(
        key(project),
        key(project_gb),
        "one product's localized pages must join"
    );
    assert_eq!(
        key(excel),
        key(excel_de),
        "and again on the market written for it"
    );

    // **Two.** Two products of one suite are two products. No meaningful-segment rule does this,
    // because `microsoft-365` is a suite and suite names cannot be enumerated.
    assert_ne!(
        key(project),
        key(teams),
        "two products in one suite must stay apart"
    );
    // ...and a product's own pricing page is still that product, which `LastSegment` splits.
    assert_eq!(
        key(excel),
        key(excel_pricing),
        "a product and its pricing page must join"
    );

    // **Three, and this is the one the bare heading failed.** Two vendors, one product name.
    let freshbooks = key("https://www.freshbooks.com/invoice");
    let intuit = key("https://quickbooks.intuit.com/invoicing/");
    assert_ne!(
        freshbooks, intuit,
        "two vendors both calling their product Invoicing must not merge: {freshbooks} vs {intuit}"
    );
    assert!(
        freshbooks.starts_with("freshbooks.com") && intuit.starts_with("intuit.com"),
        "the vendor's domain is half the key: {freshbooks}, {intuit}"
    );

    // **And the cost, which is the reason it is not free.** It needs the page, and the page is
    // fetched *after* the merge today. PR 3 has to invert that order, and only the top `NAMED`
    // candidates are fetched at all — so anything below the cut has no declared identity and
    // must fall back to something path-shaped.
    assert_eq!(
        Identity::declared_for("https://www.microsoft.com/en-us/microsoft-365/excel", ""),
        None,
        "a page that could not be read declares nothing, and must not key to the domain alone"
    );
}
