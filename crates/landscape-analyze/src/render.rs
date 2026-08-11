//! A report, as text a person reads.
//!
//! `PRODUCT_SPEC.md` §4.3 fixes the shape of an empty section, and it is the part worth getting
//! right: *"a calm block listing what was checked"*. Not an apology, not a warning triangle —
//! the same weight as a section with facts in it, because **the absence is a finding**.
//!
//! Rendering lives beside the analysis rather than in the CLI so that the report a reader is
//! shown and the report the API will serialize are assembled from one set of words.

use std::fmt::Write as _;

use landscape_core::{Confidence, Disposition, Report, SectionStatus};

use crate::Analysis;

impl Analysis {
    /// The report, as Markdown.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let report = &self.report;
        // Whether this report covers more than one company, which decides whether each claim
        // has to say whose it is.
        //
        // **From the companies analyzed, not from the companies that produced a claim.** Those
        // are different questions and they come apart in the case that matters: two companies
        // asked about, one of them silent, and the survivor's prices rendered without a label
        // in a report that is still a comparison. Review found it.
        let several = report.subjects.len() > 1;

        let _ = writeln!(out, "# {}", report.subject);
        for note in &report.notes {
            let _ = writeln!(
                out,
                "
> {note}"
            );
        }
        let _ = writeln!(
            out,
            "\nRead {} · {} source(s) cited · prompts v{}",
            report.generated_at.format("%Y-%m-%d %H:%M UTC"),
            report.sources.len(),
            report.prompt_version
        );

        for (section, coverage) in report.sections.iter().zip(self.coverage.iter()) {
            let _ = writeln!(out, "\n## {}", section.title);
            if section.status == SectionStatus::NotFoundInPublicSources {
                // §4.3's calm block. The note says which of the four silences this is.
                let _ = writeln!(out, "\n{}", coverage.note());
                continue;
            }
            for claim in &section.claims {
                // Whose price is this? On a report about one company the subject is the same
                // on every line and printing it is noise; on a comparison it is the difference
                // between a table and a list of numbers.
                let whose = if several && !claim.subject.is_empty() {
                    format!("**{}** - ", short(&claim.subject))
                } else {
                    String::new()
                };
                // **Marked in the body, not only in the source list.** FACT_CHECKING §3.2.4
                // puts an unverified source in the report body *"marked unverified, with
                // why"*, and a code beside a URL two screens down is neither: a reader
                // skimming claims would take a stranger's page for the company speaking. The
                // "why" is on the source line, which is where the link is.
                let _ = writeln!(
                    out,
                    "\n- {whose}{}{} [{}·{}]",
                    marking(report, &claim.source_label),
                    claim.text,
                    claim.source_label,
                    confidence_code(claim.confidence)
                );
                if !claim.evidence_quote.trim().is_empty() {
                    let _ = writeln!(out, "  > {}", claim.evidence_quote.trim());
                }
            }
        }

        let _ = writeln!(out, "\n## Sources");
        if report.sources.is_empty() {
            let _ = writeln!(out, "\nNone. Nothing above is cited, and nothing above is.");
        }
        for source in &report.sources {
            let _ = writeln!(
                out,
                "\n[{}] {} — {} ({} — {})",
                source.label,
                source.title,
                source.url,
                source.disposition.code(),
                source.disposition.reader_description()
            );
        }

        // A citation that does not resolve is worse than no citation: it looks checkable and
        // is not. The report says so about itself rather than leaving a reader to find out.
        let dangling = report.dangling_source_labels();
        if !dangling.is_empty() {
            let _ = writeln!(
                out,
                "\n**This report is not publishable**: {} unresolved citation(s): {}",
                dangling.len(),
                dangling.join(", ")
            );
        }
        out
    }
}

/// What goes in front of a claim whose source is not the company speaking.
///
/// Empty for a primary source, which is nearly every claim — a marker on every line is a marker
/// nobody reads. A claim whose label resolves to nothing gets no marking here and is caught by
/// [`is_publishable`] instead, which refuses the whole report rather than annotating one line
/// of it.
fn marking(report: &Report, label: &str) -> String {
    report
        .sources
        .iter()
        .find(|s| s.label == label)
        .filter(|s| s.disposition != Disposition::Primary)
        .map_or_else(String::new, |s| format!("({}) ", s.disposition.code()))
}

/// An origin without its scheme, which is how a person names a company.
fn short(origin: &str) -> &str {
    origin
        .trim_start_matches("https://")
        .trim_start_matches("http://")
}

/// `H`, `M`, `L` — the same one-letter code the source dispositions use.
const fn confidence_code(confidence: Confidence) -> char {
    match confidence {
        Confidence::High => 'H',
        Confidence::Medium => 'M',
        Confidence::Low => 'L',
    }
}

/// Whether a report is fit to show. Kept here so the CLI and the API agree.
#[must_use]
pub fn is_publishable(report: &Report) -> bool {
    report.every_claim_is_traceable()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use landscape_core::{Claim, Coverage, Disposition, Section, Source};

    fn at() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-04T09:00:00Z")
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_default()
    }

    fn analysis(sections: Vec<Section>, coverage: Vec<Coverage>, sources: Vec<Source>) -> Analysis {
        Analysis {
            report: Report {
                subject: "https://e.com".to_owned(),
                searched_as: "https://e.com".to_owned(),
                generated_at: at(),
                model_id: "http://127.0.0.1:8080".to_owned(),
                prompt_version: 1,
                subjects: Vec::new(),
                sections,
                sources,
                interpreted: None,
                notes: Vec::new(),
            },
            coverage,
            pages: Vec::new(),
            stopped_early: false,
        }
    }

    fn populated() -> (Section, Coverage, Source) {
        let claim = Claim {
            subject: String::new(),
            text: "Pro costs $15".to_owned(),
            source_label: "S1".to_owned(),
            evidence_quote: "$15/user, billed monthly".to_owned(),
            confidence: Confidence::High,
            as_of: at(),
        };
        let section = Section {
            key: "pricing".to_owned(),
            title: "Pricing & packaging".to_owned(),
            status: SectionStatus::Populated,
            claims: vec![claim],
            checked: Vec::new(),
            notes: Vec::new(),
        };
        let coverage = Coverage {
            question: "pricing".to_owned(),
            sources: vec!["https://e.com/pricing".to_owned()],
            pages_read: 1,
            facts: 1,
            attempts: Vec::new(),
        };
        let source = Source {
            label: "S1".to_owned(),
            url: "https://e.com/pricing".to_owned(),
            title: "Pricing".to_owned(),
            disposition: Disposition::Primary,
            fetched_at: at(),
            independence_group: "https://e.com".to_owned(),
        };
        (section, coverage, source)
    }

    #[test]
    fn a_claim_is_rendered_with_its_source_and_its_quote() {
        let (section, coverage, source) = populated();
        let text = analysis(vec![section], vec![coverage], vec![source]).render();
        assert!(text.contains("Pro costs $15 [S1·H]"), "{text}");
        assert!(text.contains("> $15/user, billed monthly"), "{text}");
        assert!(
            text.contains("[S1] Pricing — https://e.com/pricing (P — the company's own page)"),
            "{text}"
        );
        // A primary claim carries no marking. A marker on every line is a marker nobody reads.
        assert!(!text.contains("- (P)"), "{text}");
    }

    #[test]
    fn a_claim_from_a_page_we_could_not_attribute_says_so_where_it_is_read() {
        // FACT_CHECKING §3.2.4 puts an unverified source in the report body *marked unverified,
        // with why*. A code beside a URL two screens below the claim is neither, and search is
        // what makes this reachable: until an analysis called it, every source was the
        // company's own page and nothing could be anything but `P`.
        let (mut section, coverage, mut source) = populated();
        source.disposition = Disposition::Unverified;
        source.url = "https://someblog.example/linear-review".to_owned();
        section.claims[0].text = "states it offers unlimited seats".to_owned();

        let text = analysis(vec![section], vec![coverage], vec![source]).render();
        assert!(
            text.contains("- (U) states it offers unlimited seats [S1·H]"),
            "an unattributed page read as the company speaking:\n{text}"
        );
        assert!(
            text.contains("(U — a page we could read but could not fully attribute)"),
            "the why is missing from the source line:\n{text}"
        );
    }

    #[test]
    fn an_empty_section_carries_what_was_checked() {
        // PRODUCT_SPEC §4.3: a calm block, the same weight as a section with facts in it.
        let coverage = Coverage {
            question: "changes".to_owned(),
            sources: Vec::new(),
            pages_read: 0,
            facts: 0,
            attempts: vec![landscape_core::Attempt {
                subject: String::new(),
                path: "/changelog".to_owned(),
                outcome: "404".to_owned(),
            }],
        };
        let section = coverage.to_section("Recent public changes");
        let text = analysis(vec![section], vec![coverage], Vec::new()).render();
        assert!(text.contains("## Recent public changes"), "{text}");
        assert!(text.contains("/changelog (404)"), "{text}");
    }

    #[test]
    fn a_report_with_an_unresolved_citation_says_it_is_not_publishable() {
        // The one thing worse than no citation is one that looks checkable and is not.
        let (section, coverage, _) = populated();
        let analysis = analysis(vec![section], vec![coverage], Vec::new());
        assert!(!is_publishable(&analysis.report));
        assert!(
            analysis.render().contains("not publishable"),
            "{}",
            analysis.render()
        );
    }

    #[test]
    fn a_report_with_no_sources_says_so_rather_than_showing_an_empty_heading() {
        let text = analysis(Vec::new(), Vec::new(), Vec::new()).render();
        assert!(text.contains("## Sources"), "{text}");
        assert!(text.contains("None."), "{text}");
    }
}
