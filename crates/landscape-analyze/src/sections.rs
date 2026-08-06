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

/// Whether anything can read a page admitted for this question yet.
///
/// **One of the six has no extractor.** It still gets a section, and it carries the coverage
/// note saying the pages were found and not opened — our gap, stated as ours.
#[must_use]
pub(crate) const fn has_extractor(question: Answers) -> bool {
    matches!(
        question,
        Answers::Pricing
            | Answers::Features
            | Answers::Changes
            | Answers::Identity
            | Answers::Trust
    )
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
    fn a_section_without_an_extractor_still_exists() {
        // It carries the coverage note instead of claims. Dropping the section would hide
        // the gap; keeping it states it.
        //
        // **Direction is the last one.** Trust used to be here too, and gained an extractor;
        // this test moved with it rather than being deleted, because the property it holds is
        // about the section surviving the absence, not about which question is absent.
        assert!(!has_extractor(Answers::Direction));
        assert_eq!(title_for(Answers::Direction), "Where they are investing");
    }

    #[test]
    fn a_question_with_an_extractor_is_read_rather_than_reported_as_a_gap() {
        // The other half, and the one this change moves: a page admitted for trust used to be
        // fetched and skipped with "no extractor yet", which spent the fetch and produced a
        // permanently empty section.
        assert!(has_extractor(Answers::Trust));
    }
}
