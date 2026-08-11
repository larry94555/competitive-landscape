//! Checks on the golden set, not on any model. Needs nothing running.
//!
//! A golden set is a measuring instrument, and an uncalibrated instrument produces numbers
//! that are worse than no numbers because they are believed. The specific ways this one
//! could go wrong:
//!
//! - A reference answer that is not actually on the page. Then a correct model scores as
//!   wrong, we "fix" the model, and the set has made things worse.
//! - The set drifting easy. Traps are the subjects most likely to fail, so they are the
//!   ones under quiet pressure to be deleted — which is precisely when they were working.
//! - A subject nobody can justify, kept because removing it feels like losing coverage.
//!
//! Each of those is a test below.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use landscape_core::extract::BillingPeriod;
use landscape_golden::{load, prompt_for, Subject};

fn subjects() -> Vec<Subject> {
    load().expect("every subject file must parse")
}

#[test]
fn the_set_has_at_least_ten_subjects() {
    // The Phase 0 commitment in ROADMAP.md. Written as a floor rather than an equality so
    // that adding the eleventh subject does not fail the build.
    let n = subjects().len();
    assert!(
        n >= 10,
        "the golden set has {n} subjects, ROADMAP.md commits to 10"
    );
}

#[test]
fn every_subject_has_a_distinct_id() {
    let mut ids: Vec<String> = subjects().into_iter().map(|s| s.id).collect();
    ids.sort();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "two subjects share an id: {ids:?}");
}

#[test]
fn every_subject_says_why_it_exists() {
    // The `why` is what stops a failing subject being deleted instead of investigated.
    // A one-word justification would satisfy a presence check and not that purpose.
    for s in subjects() {
        assert!(
            s.why.split_whitespace().count() >= 12,
            "subject `{}` does not explain what it is for: {:?}",
            s.id,
            s.why
        );
    }
}

#[test]
fn every_expected_answer_is_actually_on_the_page() {
    // The one that matters most. A reference answer not present in the source is asking a
    // model to invent the right thing, and a set that does that punishes exactly the
    // behavior we are trying to reward.
    for s in subjects() {
        if let Some(name) = &s.expect.plan_name {
            assert!(
                s.source.to_lowercase().contains(&name.to_lowercase()),
                "subject `{}` expects plan_name {name:?}, which is not on its page",
                s.id
            );
        }
        if let Some(price) = s.expect.price_usd {
            // Written as a page would write it: no trailing zeros on a whole number.
            #[allow(clippy::cast_possible_truncation)]
            let as_written = if (price.fract()).abs() < f64::EPSILON {
                format!("{}", price as i64)
            } else {
                format!("{price:.2}")
            };
            assert!(
                s.source.contains(&as_written),
                "subject `{}` expects price {as_written}, which is not on its page",
                s.id
            );
        }
    }
}

#[test]
fn every_expected_billing_period_is_stated_on_the_page() {
    // The same rule as `every_expected_answer_is_actually_on_the_page`, applied to the field
    // it is easiest to get wrong. Review caught the Linear Free subject expecting `monthly`
    // from a window that says only `$0` and "Free for everyone" — the cadence belonged to the
    // plan *below* it. An expectation like that marks the honest null answer wrong and
    // rewards copying a neighboring plan's period, which is a real failure mode: the
    // scorecard's own history has models reporting `monthly` for plans with no published
    // price at all.
    // Checking the whole page would not have caught it — the subject carries the neighboring
    // plan on purpose, so `per user/month` *is* somewhere in the source. The cadence has to be
    // in the section belonging to the plan being asked about.
    for s in subjects() {
        let Some(period) = s.expect.billing_period else {
            continue;
        };
        let section = section_for(&s.source, &s.ask).to_lowercase();
        let words: &[&str] = match period {
            BillingPeriod::Monthly => &["month", "monthly", "/mo"],
            BillingPeriod::Yearly => &["year", "yearly", "annual", "annually"],
            BillingPeriod::OneOff => &["one-off", "one time", "once", "single payment"],
        };
        assert!(
            words.iter().any(|w| section.contains(w)),
            "subject `{}` expects billing_period {period:?}, which the part of its page about \
             {:?} never states. Either the honest answer is null, or the subject is quoting a \
             neighboring plan's cadence.",
            s.id,
            s.ask
        );
    }
}

/// The part of a page that belongs to one plan: its heading, down to the next heading of the
/// same or a shallower level.
///
/// A plan's subtitle is a *deeper* heading — `## Pro Unlimited` above `### Top-of-the-line,
/// all-inclusive pricing` — so stopping at any heading at all would cut most plans off before
/// their price. Pages with no headings (the prose subjects) have one section, which is the
/// whole page.
fn section_for<'a>(source: &'a str, plan: &str) -> &'a str {
    let lines: Vec<&str> = source.lines().collect();
    let level = |l: &str| l.len() - l.trim_start_matches('#').len();
    let text = |l: &str| l.trim_start_matches('#').trim().to_lowercase();
    let wanted = plan.to_lowercase();

    // Exact heading first: `Pro` must not match `Pro Unlimited` when both are on the page.
    let start = lines
        .iter()
        .position(|l| level(l) > 0 && text(l) == wanted)
        .or_else(|| {
            lines
                .iter()
                .position(|l| level(l) > 0 && text(l).contains(&wanted))
        });
    let Some(start) = start else {
        return source;
    };

    let depth = level(lines[start]);
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| level(l) > 0 && level(l) <= depth)
        .map_or(lines.len(), |(i, _)| i);

    let from = source
        .lines()
        .take(start)
        .map(|l| l.len() + 1)
        .sum::<usize>();
    let to = source
        .lines()
        .take(end)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        .min(source.len());
    &source[from..to]
}

#[test]
fn the_plan_being_asked_about_is_named_on_the_page() {
    // Otherwise the subject tests whether a model can find something that is not there,
    // which is a different and much less useful question.
    for s in subjects() {
        assert!(
            s.source.to_lowercase().contains(&s.ask.to_lowercase()),
            "subject `{}` asks about {:?}, which its page never names",
            s.id,
            s.ask
        );
    }
}

#[test]
fn at_least_three_subjects_publish_no_price() {
    // The set's whole reason for existing is fabrication, and fabrication can only be
    // measured on subjects where the honest answer is "nothing". If this floor is ever
    // hit by deletion rather than met by addition, the set has stopped measuring the
    // thing it was built for.
    let abstentions = subjects()
        .iter()
        .filter(|s| s.expect.price_usd.is_none())
        .count();
    assert!(
        abstentions >= 3,
        "only {abstentions} subjects require abstaining on price; the set cannot \
         measure fabrication without them"
    );
}

#[test]
fn one_subject_puts_a_real_price_next_to_a_plan_that_has_none() {
    // Abstaining on a page with no numbers is easy. Abstaining on a page that shows a
    // correctly formatted price for a different product is the real test, and it is worth
    // asserting that such a subject is still in the set.
    let s = subjects()
        .into_iter()
        .find(|s| s.id == "price-belongs-to-something-else")
        .expect("the hardest abstention subject has been removed from the set");
    assert!(s.expect.price_usd.is_none());
    assert!(s.source.contains('$'), "its page no longer shows a price");
}

#[test]
fn one_subject_publishes_a_price_of_zero() {
    // Some(0.0) and None are different findings. Without a free tier in the set, a model
    // that collapsed them would score perfectly.
    let has_zero = subjects()
        .iter()
        .any(|s| s.expect.price_usd.is_some_and(|p| p.abs() < f64::EPSILON));
    assert!(has_zero, "no subject publishes a free tier");
}

#[test]
fn every_page_is_long_enough_to_be_a_page() {
    // A two-line fixture measures nothing about reading a real page, where the answer
    // competes with everything around it.
    for s in subjects() {
        let words = s.source.split_whitespace().count();
        assert!(
            words >= 50,
            "subject `{}` has a {words}-word page, too short to be realistic",
            s.id
        );
    }
}

#[test]
fn the_prompt_carries_the_page_and_the_plan() {
    let s = &subjects()[0];
    let prompt = prompt_for(s);
    assert!(prompt.contains(&s.source));
    assert!(prompt.contains(&s.ask));
    // The abstention instruction is the load-bearing sentence in the prompt. If it is
    // ever edited away, the fabrication numbers move for a reason nobody records.
    assert!(prompt.contains("leave that field null"));
}
