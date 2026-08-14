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
const BASELINE: [(&str, usize, usize, usize); 5] = [
    // id, expected found, expected missed, impostors admitted
    ("project-management-for-agencies", 2, 1, 1),
    ("one-product-many-urls", 2, 1, 0),
    // **Measured, not predicted.** I guessed (2, 0, 1) here and the harness said (1, 1, 1):
    // `toggl.com` came back from one query, so it is `Uncorroborated` and never reaches the
    // reader. Cause 3, in a fixture written to hold cause 4. The first thing this file did was
    // correct its author.
    ("keyword-impostor", 1, 1, 1),
    ("specialist-in-one-article", 2, 1, 0),
    ("publisher-heavy", 2, 0, 0),
];

fn scored() -> Vec<(Market, Scored)> {
    markets()
        .into_iter()
        .map(|m| {
            let s = score(&m);
            (m, s)
        })
        .collect()
}

#[test]
fn every_fixture_is_worth_having() {
    // A fixture that expects nothing, or that names an impostor it also expects, measures
    // nothing. Cheap, and it is the check the extraction set has too.
    for market in markets() {
        assert_eq!(
            market.results.len(),
            landscape_golden::discovery::QUERIES,
            "{} does not carry one result set per query",
            market.id
        );
        assert!(!market.expected.is_empty(), "{} expects nothing", market.id);
        for (impostor, why) in &market.impostors {
            assert!(
                !market.expected.contains(impostor),
                "{}: {impostor} is both expected and an impostor ({why})",
                market.id
            );
        }
    }
    let mut ids: Vec<&str> = markets().iter().map(|m| m.id).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "two fixtures share an id");
}

#[test]
fn the_baseline_has_not_changed_by_accident() {
    let results = scored();

    println!("\nDISCOVERY, AS IT RUNS TODAY\n");
    println!("{:<34} recall  missed  impostors  returned", "market");
    for (_, s) in &results {
        println!(
            "{:<34} {:>5.0}% {:>7} {:>10}  {}",
            s.id,
            s.recall() * 100.0,
            s.missed.join(", "),
            s.admitted.len(),
            s.returned.join(", ")
        );
    }
    let total: f64 = results.iter().map(|(_, s)| s.recall()).sum();
    #[expect(clippy::cast_precision_loss, reason = "five fixtures")]
    let mean = total / results.len() as f64;
    let admitted: usize = results.iter().map(|(_, s)| s.admitted.len()).sum();
    println!(
        "\nmean recall {:.0}%   impostors admitted {admitted}\n",
        mean * 100.0
    );

    for (id, found, missed, impostors) in BASELINE {
        let (_, got) = results
            .iter()
            .find(|(_, s)| s.id == id)
            .unwrap_or_else(|| panic!("{id} is in the baseline and not in the set"));
        assert_eq!(
            (got.found.len(), got.missed.len(), got.admitted.len()),
            (found, missed, impostors),
            "\n{id} changed. Read the table above, decide whether it is an improvement, and \
             edit BASELINE. A discovery change that nobody had to look at is the thing this \
             file exists to prevent.\n  found:   {:?}\n  missed:  {:?}\n  admitted:{:?}",
            got.found,
            got.missed,
            got.admitted
        );
    }
}

#[test]
fn the_reported_failure_is_reproduced() {
    // **The specific answer a reader got**, held as a test rather than as a story in a
    // benchmark. If a change makes this fixture pass, that is the thing to celebrate; if it
    // makes it pass *and* nothing else moves, it is worth suspecting.
    let market = markets()
        .into_iter()
        .find(|m| m.id == "project-management-for-agencies")
        .expect("the reported failure is in the set");
    let got = score(&market);

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
