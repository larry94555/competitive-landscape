//! What the frozen pages must produce, checked on every pull request.
//!
//! **These need no model, no network and no `llama-server`.** That is the whole point: the
//! subjects in `subjects/` measure a model and are `#[ignore]`d because CI has no GPU, so
//! until now nothing in this crate ran on a pull request at all.
//!
//! `BENCHMARKS.md` Runs 5 to 16 found sixteen defects by pointing the pipeline at real
//! companies and reading the output. The deterministic half of that — which windows a page
//! yields, which capabilities it names, which dates it carries — is written down here, so the
//! seventeenth is caught by a build rather than by somebody noticing.

#![allow(clippy::expect_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.

use landscape_golden::pages;

#[test]
fn every_frozen_page_still_reads_the_way_it_did() {
    let expectations = pages::load().expect("the page set loads");
    let mut wrong: Vec<String> = Vec::new();

    for expectation in &expectations {
        let differences = expectation
            .differences()
            .unwrap_or_else(|e| panic!("{}: {e}", expectation.page));
        if differences.is_empty() {
            continue;
        }
        // Everything about this page, then the next page. A change to a shared rule usually
        // moves several, and *which* ones is the diagnosis — stopping at the first would hide
        // the difference between "one page shifted" and "the scorer changed".
        wrong.push(format!(
            "\n{}  ({:?})\n  why it is here: {}\n{}",
            expectation.page,
            expectation.answers,
            expectation.why,
            differences
                .iter()
                .map(|d| format!("  {d}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    assert!(
        wrong.is_empty(),
        "{} of {} frozen pages no longer read the way they did:\n{}\n\n\
         If this is an improvement, update the expectation in the same commit and say so in \
         its `why`. If it is not, the page has not changed — these are frozen — so something \
         in extraction has.",
        wrong.len(),
        expectations.len(),
        wrong.join("\n")
    );
}

#[test]
fn the_set_covers_every_question_that_has_an_extractor() {
    // **All six extract now**, so this list is every question there is. A question with an
    // extractor and no frozen page is a rule nothing holds still — which is how span selection
    // went five runs before anyone noticed it had been picking the FAQ.
    //
    // Each new one has paid for its page on the first run. Trust: linear.app says `SOC 2` in a
    // banner and `SOC 2 Type II` in the section below, and the first version reported both.
    // Direction: front.com's footer link *"Become a Partner"* arrived as an open vacancy.
    let expectations = pages::load().expect("the page set loads");
    for question in [
        pages::Answers::Pricing,
        pages::Answers::Features,
        pages::Answers::Changes,
        pages::Answers::Identity,
        pages::Answers::Trust,
        pages::Answers::Direction,
    ] {
        assert!(
            expectations.iter().any(|e| e.answers == question),
            "no frozen page covers {question:?}"
        );
    }
}

#[test]
fn the_set_holds_a_page_that_answers_nothing() {
    // The control group, and the thing this set exists to protect. Two pages here are
    // negatives — todoist publishes no price we can read, and Linear's release documentation
    // dates nothing — and a set of only positives cannot tell a working extractor from one
    // that fills every field it is given.
    let expectations = pages::load().expect("the page set loads");
    let negatives = expectations
        .iter()
        .filter(|e| {
            e.plan_windows.as_ref().is_some_and(Vec::is_empty)
                || e.changes.as_ref().is_some_and(|c| c.considered == 0)
                || e.facts.as_ref().is_some_and(Vec::is_empty)
                || e.assurances.as_ref().is_some_and(|a| a.named.is_empty())
                || e.openings.as_ref().is_some_and(|o| o.roles.is_empty())
        })
        .count();
    assert!(
        negatives >= 2,
        "a golden set of only positives cannot catch an extractor that always finds something"
    );
}

#[test]
fn every_expectation_freezes_a_body_and_not_just_a_heading() {
    // Review found this hole in one try: with only headings frozen, changing Linear Business
    // from `$16 per user/month` to `$999` in the frozen page left every test green. The window
    // *is* the product — §5.4's argument is that a bad window and a bad model cannot be told
    // apart at the output — so an expectation listing which headings won and nothing else
    // asserts the cheaper half of the thing that matters.
    //
    // An empty body is legitimate for a heading with nothing under it, so this asks that the
    // page as a whole froze some text, not that every window did.
    for e in pages::load().expect("the page set loads") {
        let mut lines = 0usize;
        if let Some(windows) = &e.plan_windows {
            lines += windows.iter().map(|w| w.text.len()).sum::<usize>();
        }
        if let Some(caps) = &e.capabilities {
            lines += caps.named.iter().map(|w| w.text.len()).sum::<usize>();
        }
        if let Some(changes) = &e.changes {
            // A quote is the evidence a reader is shown. A title can stay right while the
            // line behind it drifts, so both are frozen and both are counted.
            lines += changes
                .entries
                .iter()
                .filter(|d| !d.quote.trim().is_empty())
                .count();
        }
        if let Some(facts) = &e.facts {
            lines += facts.iter().map(|f| f.text.len()).sum::<usize>();
        }
        if let Some(assurances) = &e.assurances {
            // The window, not the name. A standard read from the wrong paragraph is a
            // confident answer about a different sentence, and the name alone would not show
            // it — the same hole review found in the heading-only expectations.
            lines += assurances.named.iter().map(|n| n.text.len()).sum::<usize>();
        }
        if let Some(openings) = &e.openings {
            // The line, not the title. On all three frozen careers pages the two are the same
            // string, because a role is written on a line of its own — so this counts the
            // quote deliberately, to fail the day a page lists roles as `- Title` and the
            // marker starts leaking into the evidence a reader is shown.
            lines += openings
                .roles
                .iter()
                .filter(|r| !r.quote.trim().is_empty())
                .count();
        }

        // The two pages that legitimately yield nothing are the controls, and they are
        // asserted by `the_set_holds_a_page_that_answers_nothing` instead.
        let yields_nothing = e.plan_windows.as_ref().is_some_and(Vec::is_empty)
            || e.changes.as_ref().is_some_and(|c| c.considered == 0)
            || e.facts.as_ref().is_some_and(Vec::is_empty)
            || e.assurances.as_ref().is_some_and(|a| a.named.is_empty())
            || e.openings.as_ref().is_some_and(|o| o.roles.is_empty());
        assert!(
            yields_nothing || lines > 0,
            "{} freezes headings but no content, so a change inside a window would pass",
            e.page
        );
    }
}

#[test]
fn the_subjects_carved_from_real_pages_are_still_verbatim() {
    // Five of the fifteen model subjects are windows cut from the pages in `pages/` rather
    // than prose somebody wrote. That is their entire value, and it is silently reversible: a
    // subject a model keeps failing invites a small edit to the page text, and a subject
    // edited until it passes measures nothing. So the text has to still be *in* the page.
    //
    // Whitespace is normalised because the window is joined from block elements and the JSON
    // carries it as `\n`; the words and the numbers are what must match.
    let pages: Vec<String> = pages::load()
        .expect("the page set loads")
        .iter()
        .map(|e| squash(&e.markdown().expect("a frozen page reads")))
        .collect();

    let carved = [
        "flat-price-for-the-whole-company",
        "per-user-price-among-service-numbers",
        "free-tier-with-real-quotas",
        "price-buried-under-its-own-feature-list",
        "a-pricing-page-that-states-no-price",
    ];
    let subjects = landscape_golden::load().expect("the subject set loads");

    for id in carved {
        let subject = subjects.iter().find(|s| s.id == id).unwrap_or_else(|| {
            panic!("subject `{id}` is gone; it was carved from a page in pages/")
        });
        // The Linear subject spans two windows with a blank line between them, so check each
        // paragraph rather than the whole block.
        for chunk in subject.source.split("\n\n") {
            let needle = squash(chunk);
            assert!(
                pages.iter().any(|p| p.contains(&needle)),
                "subject `{id}` is no longer verbatim from any frozen page. Its text was cut \
                 from one, and editing it turns a real page into a synthetic one that looks \
                 real. Missing:\n{needle}"
            );
        }
    }
}

fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn a_known_wrong_expectation_says_what_would_be_right() {
    // An expectation that freezes today's behaviour rather than the right answer is a
    // specification unless it argues otherwise. `known_wrong` is where that argument lives,
    // and an empty one would turn a defect into a decision.
    for expectation in pages::load().expect("the page set loads") {
        if let Some(note) = &expectation.known_wrong {
            assert!(
                note.split_whitespace().count() >= 20,
                "{}: `known_wrong` has to say what the right answer is and why we kept this \
                 one — got {note:?}",
                expectation.page
            );
        }
    }
}
