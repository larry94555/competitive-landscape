//! Which sections a report has, and what each is called.
//!
//! `PRODUCT_SPEC.md` §4 fixes nine. This is the six that map onto a question discovery asks,
//! and **the gap is deliberate**: positioning, sentiment themes, market emphasis and the SWOT
//! are interpretation over sources this pipeline does not gather. A section that exists and
//! cannot be filled teaches a reader to skim past empty sections, which is the one habit this
//! product cannot afford.

use landscape_discover::probes::Answers;

/// Every section, in the order a report presents them.
///
/// Pricing first because it is the most-read section in the product, then what it does, then
/// what changed — the three a competitor report is opened for.
pub const SECTIONS: [(Answers, &str); 6] = [
    (Answers::Pricing, "Pricing & packaging"),
    (Answers::Features, "What it does"),
    (Answers::Changes, "Recent public changes"),
    (Answers::Identity, "Company facts"),
    (Answers::Trust, "Trust & security posture"),
    (Answers::Direction, "Where they are investing"),
];

/// The title a question's section carries.
#[must_use]
pub fn title_for(question: Answers) -> &'static str {
    SECTIONS
        .iter()
        .find(|(q, _)| *q == question)
        .map_or("Other", |(_, title)| *title)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_question_discovery_asks_has_a_section() {
        // Otherwise a page could be admitted, read, and have nowhere to be reported — the
        // failure mode where work happens and disappears.
        for question in [
            Answers::Pricing,
            Answers::Features,
            Answers::Changes,
            Answers::Identity,
            Answers::Trust,
            Answers::Direction,
        ] {
            assert!(
                SECTIONS.iter().any(|(q, _)| *q == question),
                "{question:?} has no section"
            );
        }
    }

    #[test]
    fn the_report_leads_with_pricing() {
        assert_eq!(SECTIONS[0].0, Answers::Pricing);
    }

    #[test]
    fn the_last_question_without_an_extractor_has_one() {
        // **This test used to assert the opposite**, and moved twice: trust was the question
        // with no extractor, then direction was, and now neither is. What it held each time was
        // that a section survives the absence of its extractor — a page found and not opened
        // still gets a titled section carrying the coverage note, because dropping the section
        // would hide the gap and keeping it states it.
        //
        // With every question covered, `has_extractor` was deleted rather than left returning
        // `true` for all six: a predicate no caller can make false is a check that cannot fail,
        // and the branch behind it was unreachable code that still looked like a safety net.
        // What replaced it is the compiler — `stages::extract` matches every variant with no
        // wildcard, so a seventh question is a build error rather than a string in a run log
        // nobody reads.
        assert_eq!(title_for(Answers::Direction), "Where they are investing");
        assert_eq!(SECTIONS.len(), 6);
    }
}
