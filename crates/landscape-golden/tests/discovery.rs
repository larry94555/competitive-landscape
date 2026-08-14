//! What the discovery pipeline does today, recorded so a change has something to beat.
//!
//! `cargo test -p landscape-golden --test discovery -- --nocapture` prints both tables.
//!
//! **These tests pass while the answer is bad.** That is deliberate: a red suite is a thing
//! people learn to ignore, and what is wanted here is a *number* that a pull request has to move
//! and cannot move by accident. `the_baseline_has_not_changed_by_accident` is the one that fails
//! when a change alters discovery — and the fix for that failure is to read the printed table,
//! decide the change is an improvement, and edit the number.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use landscape_golden::discovery::{markets, score, Identity, Market, Scored};

/// What the pipeline scores today, as of the run recorded in `BENCHMARKS.md` Run 51.
///
/// **A recorded baseline, not a target.** Nothing here is a claim that these numbers are good;
/// three of them are bad on purpose, because the fixtures hold defects the roadmap has not fixed
/// yet.
/// What the pipeline returns today, exactly, as of `BENCHMARKS.md` Run 51.
///
/// **Exact lists, not counts.** The first version stored `(found, missed, admitted)` as three
/// numbers, and review found the hole: swapping *which* expected company came back leaves every
/// count identical and passes. So does adding a sixth fixture, because the test iterated the
/// baseline rather than the set. Both are precisely the accidental change this file claims to
/// catch.
const BASELINE: [(&str, &[&str], &[&str], usize); 5] = [
    // id, found, missed, impostors admitted
    (
        "project-management-for-agencies",
        &["asana.com", "microsoft.com"],
        &["workamajig.com"],
        1,
    ),
    (
        "one-product-many-urls",
        &["microsoft.com", "google.com"],
        &["airtable.com"],
        0,
    ),
    // **Measured end to end, and it disagrees with the ranking-only number twice.** The first
    // version of this file scored `from_results` alone and recorded 1 impostor here; running
    // `assemble` too, the fit test *does* catch `trackingtimemusic.com` - its page shares none
    // of "time", "tracking", "consultants". One word is enough for `projectplusgame.com` and
    // not for this one, which is a finer account of cause 4 than the roadmap had.
    ("keyword-impostor", &["harvestapp.com"], &["toggl.com"], 0),
    (
        "specialist-in-one-article",
        &["freshbooks.com", "intuit.com"],
        &["protemos.com"],
        0,
    ),
    ("publisher-heavy", &["zendesk.com"], &["helpscout.com"], 0),
];

async fn scored() -> Vec<(Market, Scored)> {
    let mut out = Vec::new();
    for market in markets() {
        let s = score(&market).await;
        out.push((market, s));
    }
    out
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
            println!("      set aside  {host}: {why}");
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
    let mut recorded: Vec<&str> = BASELINE.iter().map(|(id, ..)| *id).collect();
    in_set.sort_unstable();
    recorded.sort_unstable();
    assert_eq!(
        in_set, recorded,
        "a fixture was added or removed without recording what it scores"
    );

    for (id, found, missed, impostors) in BASELINE {
        let (_, got) = results
            .iter()
            .find(|(_, s)| s.id == id)
            .unwrap_or_else(|| panic!("{id} is in the baseline and not in the set"));
        assert_eq!(
            got.found, found,
            concat!(
                "\n{}: a different company came back. Read the table above, ",
                "decide whether it is an improvement, and edit BASELINE."
            ),
            id
        );
        assert_eq!(
            got.missed, missed,
            "\n{id}: a different company is missing."
        );
        assert_eq!(got.admitted.len(), impostors, "\n{id}: {:?}", got.admitted);
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

#[test]
fn the_fifth_rule_is_scored_rather_than_concluded_by_elimination() {
    // **Review's correction.** The first version disproved four rules and then asserted the
    // fifth is the answer — which is an inference, not a measurement, and the PR body said
    // "PR 2 selected it". This scores it.
    //
    // `Identity::Declared` keys on the name a page declares about itself, which is what the
    // roadmap's fifth candidate means. The fixtures give it the pages.
    let excel = "# Microsoft Excel

The spreadsheet.";
    let excel_de = "# Microsoft Excel

Die Tabellenkalkulation.";
    let project = "# Microsoft Project

Project management software.";

    assert_eq!(
        Identity::declared_from(excel),
        Identity::declared_from(excel_de),
        "a product's own name should join its localized pages"
    );
    assert_ne!(
        Identity::declared_from(excel),
        Identity::declared_from(project),
        "a product's own name should separate two products in one suite"
    );

    // **And the cost, which is the reason it is not free.** It needs the page, and the page is
    // fetched *after* the merge today. PR 3 has to invert that order, and only the top `NAMED`
    // candidates are fetched at all — so anything below the cut has no declared identity and
    // must fall back to something path-shaped.
    assert_eq!(
        Identity::declared_from(""),
        None,
        "a page that could not be read declares nothing, and must not key to an empty product"
    );
}
