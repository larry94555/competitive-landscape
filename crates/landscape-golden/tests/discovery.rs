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

use landscape_golden::discovery::{markets, score, under, Fit, Identity, Market, Scored};
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
    /// **In order**, because the order is what a reader reads first. PR 3 changed nothing about
    /// *which* companies come back for the reported failure and everything about which one is
    /// first, and a set comparison would have called that no change at all.
    returned: Vec<&'static str>,
    /// What each company is **called**, by domain. The number PR 3 moves.
    named: Vec<(&'static str, &'static str)>,
    found: Vec<&'static str>,
    missed: Vec<&'static str>,
    admitted: Vec<&'static str>,
    /// Host and **typed** reason, sorted by host. Not the sentence: a wording change must not
    /// fail a test about discovery, and a sentence cannot be compared field by field.
    set_aside: Vec<(&'static str, Aside)>,
}

fn uncorroborated() -> Aside {
    Aside::Uncorroborated {
        named_by: 0,
        guides: 0,
        agreed: 1,
        asked: 3,
    }
}

fn elsewhere(words: &[&str], used: &[&str]) -> Aside {
    Aside::ElsewhereEntirely {
        looked_for: words.iter().map(|w| (*w).to_owned()).collect(),
        used: used.iter().map(|w| (*w).to_owned()).collect(),
        needed: landscape_search::competitors::enough_words(words.len()),
    }
}

/// The market of `keyword-impostor`, written once because three exclusions quote it.
const TIME: [&str; 4] = ["time", "tracking", "independent", "consultants"];

/// The baseline, as of `BENCHMARKS.md` Run 51.
///
/// **A recorded baseline, not a target.** Nothing here is a claim that these numbers are good;
/// most of them are bad on purpose, because the fixtures hold defects the roadmap has not fixed
/// yet.
fn baseline() -> Vec<Recorded> {
    vec![
        // **The reported failure, answered.** A reader typed this and got *Microsoft* and a
        // board game. PR 3 made the candidate a product; PR 4 turned the board game away; PR 5
        // read the market's own buyer's guides, and `workamajig.com` - the specialist the prompt
        // actually asks for, returned by one query and refused by `CORROBORATION` - is in the
        // answer because two independent guides list it.
        Recorded {
            id: "project-management-for-agencies",
            returned: vec!["asana.com", "workamajig.com", "microsoft.com", "notion.so"],
            named: vec![
                ("asana.com", "Asana"),
                ("workamajig.com", "Workamajig"),
                ("microsoft.com", "Microsoft Project"),
                ("notion.so", "Notion Projects"),
            ],
            found: vec!["asana.com", "microsoft.com", "workamajig.com"],
            missed: vec![],
            admitted: vec![],
            set_aside: vec![(
                "projectplusgame.com",
                elsewhere(&["project", "management", "design", "agency"], &["project"]),
            )],
        },
        // Four URL shapes of one product stay one candidate, at the same agreement: this is
        // the fixture that would catch a rule that split what it was meant to join.
        Recorded {
            id: "one-product-many-urls",
            returned: vec!["microsoft.com", "google.com"],
            named: vec![
                ("microsoft.com", "Microsoft Excel"),
                ("google.com", "Google Sheets"),
            ],
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
            returned: vec!["harvestapp.com"],
            named: vec![("harvestapp.com", "Harvest")],
            found: vec!["harvestapp.com"],
            missed: vec!["toggl.com"],
            admitted: vec![],
            set_aside: vec![
                ("timezonecheck.com", elsewhere(&TIME, &["time"])),
                ("toggl.com", uncorroborated()),
                ("trackingtimemusic.com", elsewhere(&TIME, &[])),
            ],
        },
        Recorded {
            id: "specialist-in-one-article",
            returned: vec!["freshbooks.com", "intuit.com"],
            named: vec![
                ("freshbooks.com", "FreshBooks"),
                ("intuit.com", "QuickBooks"),
            ],
            found: vec!["freshbooks.com", "intuit.com"],
            missed: vec!["protemos.com"],
            admitted: vec![],
            set_aside: vec![("protemos.com", uncorroborated())],
        },
        Recorded {
            id: "publisher-heavy",
            returned: vec!["zendesk.com"],
            named: vec![("zendesk.com", "Zendesk")],
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
        for (host, name) in &s.named {
            println!("      named      {host}: {name}");
        }
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
            got.returned, want.returned,
            "
{}: a different set came back, or in a different order.",
            want.id
        );
        let named: Vec<(&str, &str)> = got
            .named
            .iter()
            .map(|(host, name)| (host.as_str(), name.as_str()))
            .collect();
        assert_eq!(
            named, want.named,
            "
{}: a company is called something else. This is the naming PR 3 changed.",
            want.id
        );
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
    // benchmark. It said *Microsoft* and *projectplusgame.com*, and each half is now a
    // different assertion: the half a change fixed, and the half still waiting for one.
    let market = markets()
        .into_iter()
        .find(|m| m.id == "project-management-for-agencies")
        .expect("the reported failure is in the set");
    let got = score(&market).await;

    // **PR 4 fixed this half.** The board game shares `project` with the prompt and nothing
    // else, and one shared word used to be the whole test.
    assert!(
        !got.returned.iter().any(|h| h == "projectplusgame.com"),
        "the board game is back in the answer: {:?}",
        got.returned
    );
    assert!(
        got.set_aside
            .iter()
            .any(|(host, why)| host == "projectplusgame.com"
                && matches!(why, Aside::ElsewhereEntirely { .. })),
        "and it should be excluded for what its page says, not for something else: {:?}",
        got.set_aside
    );

    // **PR 3 fixed this half.** The reader was shown *Microsoft*; they are now shown the
    // product that earned the agreement.
    assert_eq!(
        got.named
            .iter()
            .find(|(host, _)| host == "microsoft.com")
            .map(|(_, name)| name.as_str()),
        Some("Microsoft Project"),
        "{:?}",
        got.named
    );

    // **PR 5 fixed the last of it.** The specialist the prompt actually asks for came back
    // from one query, which is below `CORROBORATION` — and every buyer's guide to this market
    // lists it. The market's own word is what corroborates it now.
    assert!(
        got.returned.iter().any(|h| h == "workamajig.com"),
        "the specialist is missing again: {:?}",
        got.returned
    );
    let workamajig = got
        .named
        .iter()
        .position(|(host, _)| host == "workamajig.com")
        .expect("in the answer");
    assert!(
        workamajig < got.returned.len() - 1,
        "and it is not last, because two guides is stronger than one search: {:?}",
        got.returned
    );

    // **Nothing about the reader's complaint is left.** Recorded as one assertion, so that a
    // change which brings any of it back fails here rather than only in a table.
    assert!(
        got.missed.is_empty(),
        "the reported failure is fully answered and something has come undone: {:?}",
        got.missed
    );
}

/// The four fit rules, scored against the set rather than one of them being picked.
///
/// **The roadmap says to settle this against the golden set rather than by choosing a number**,
/// and this is that, in the same shape PR 3's identity comparison took. It prints the table and
/// asserts the two things that are decidable without an opinion: what the old rule cost, and
/// that exactly one of the four turns away every impostor while keeping every real company.
#[tokio::test]
async fn the_fit_rules_are_compared_rather_than_chosen() {
    let results = scored().await;

    println!("\nWHAT EACH FIT RULE WOULD DO\n");
    let mut totals: Vec<(Fit, usize, usize)> = Vec::new();
    for rule in Fit::all() {
        let (mut admitted, mut lost) = (Vec::new(), Vec::new());
        for (market, s) in &results {
            let (a, l) = under(rule, market, s);
            admitted.extend(a);
            lost.extend(l);
        }
        println!(
            "{rule:?}  ->  impostors admitted {}, real companies lost {}",
            admitted.len(),
            lost.len()
        );
        for host in admitted.iter().chain(lost.iter()) {
            println!("      {host}");
        }
        totals.push((rule, admitted.len(), lost.len()));
    }
    println!();

    let count = |rule: Fit| {
        totals
            .iter()
            .find(|(r, _, _)| *r == rule)
            .map(|(_, a, l)| (*a, *l))
            .expect("every rule is scored")
    };

    // **What one word cost, which is the whole reason for this change.** A board game and a
    // world clock, both admitted to comparisons they have nothing to do with.
    assert_eq!(
        count(Fit::AnyWord),
        (2, 0),
        "the old rule should still admit both impostors"
    );

    // **And what the obvious replacement costs.** A flat two asks a two-word market for all of
    // it, and terse front matter is not a reason to drop a real competitor.
    let (admitted, lost) = count(Fit::TwoWords);
    assert_eq!(admitted, 0, "a flat two does turn the impostors away");
    assert!(
        lost > 0,
        "a flat two should cost real companies - if it stopped, the fixtures changed"
    );

    // Rounding up is the same trade in miniature: it asks a three-word market for two.
    let (admitted, lost_up) = count(Fit::HalfRoundedUp);
    assert_eq!(admitted, 0);
    assert!(lost_up > 0, "rounding up should still cost somebody");

    // **The one that is chosen, and it is chosen by being the only one that costs nothing.**
    assert_eq!(
        count(Fit::HalfRoundedDown),
        (0, 0),
        "the rule in production should turn away every impostor and lose nobody"
    );

    // **Five fixtures is a small set and this is a fitted number.** Said here as well as in the
    // documentation, because a reader of this test is exactly the person who should know it.
    assert_eq!(results.len(), 5, "the set this was fitted to");
}

/// The bar scales with how much a reader said, and never below one.
#[test]
fn a_short_description_is_never_asked_for_all_of_itself() {
    use landscape_search::competitors::enough_words;
    // A market nobody could describe in two words still has to admit somebody.
    assert_eq!(enough_words(0), 1);
    assert_eq!(enough_words(1), 1);
    assert_eq!(
        enough_words(2),
        1,
        "two words asked for both is asked for all of it"
    );
    assert_eq!(enough_words(3), 1);
    assert_eq!(enough_words(4), 2);
    assert_eq!(enough_words(7), 3);
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
        for (url, _) in market.product_pages.iter().chain(market.guide_pages.iter()) {
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
