//! The report schema.
//!
//! Defined once here. `schemars` generates the JSON Schema that the frontend types are
//! built from and that the model's decoding grammar is derived from, so the model cannot
//! emit a shape other than this one. See `docs/PRODUCT_SPEC.md` §4.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::source::Source;

/// How much a single statement is worth relying on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// One assertion, with the evidence that supports it.
///
/// There is no way to construct a `Claim` without a source label and a verbatim quote.
/// That is the point: an unsourced sentence cannot be represented, so it cannot reach a
/// reader by accident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Claim {
    /// One assertion, at most 400 characters.
    pub text: String,
    /// Which company this is about, as the origin it was read from.
    ///
    /// **A claim's text does not name its subject, and should not.** An extractor reading
    /// Basecamp's pricing page produces *"Pro costs $15"*, because that is what the page says;
    /// putting the company into the sentence would be the renderer's job leaking into the data.
    /// But once one section holds several companies, a reader looking at two prices has no way
    /// to tell which is whose — so the subject travels *with* the claim rather than being
    /// recoverable from it.
    ///
    /// Always set by the pipeline. **Whether to show it is the renderer's decision**: on a
    /// report about one company every claim carries the same subject and printing it against
    /// each one is noise, so it appears only when a report covers more than one.
    #[serde(default)]
    pub subject: String,
    /// Which source this came from — matches `Source::label`.
    pub source_label: String,
    /// The span from the source that supports it, copied verbatim.
    pub evidence_quote: String,
    pub confidence: Confidence,
    /// From the source where it states one, otherwise when we read the page.
    pub as_of: chrono::DateTime<chrono::Utc>,
}

/// Whether a section has anything in it, and if not, why not.
///
/// `NotFoundInPublicSources` is a finding, not an error. It renders as a calm block
/// listing what was checked — see PRODUCT_SPEC.md §4.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SectionStatus {
    Populated,
    Partial,
    NotFoundInPublicSources,
}

/// One section of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Section {
    pub key: String,
    pub title: String,
    pub status: SectionStatus,
    pub claims: Vec<Claim>,
    /// What was checked when nothing was found. Turns an unfalsifiable negative into one
    /// a reader can repeat — FACT_CHECKING.md §5.4.
    pub checked: Vec<String>,
    pub notes: Vec<String>,
}

impl Section {
    /// A section with nothing in it, recording what was looked at.
    #[must_use]
    pub fn not_found(
        key: impl Into<String>,
        title: impl Into<String>,
        checked: Vec<String>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            status: SectionStatus::NotFoundInPublicSources,
            claims: Vec::new(),
            checked,
            notes: Vec::new(),
        }
    }
}

/// A finished report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    /// What the user typed, unchanged.
    pub subject: String,
    /// What we searched for, derived from the subject. Shown to the reader directly under
    /// their own words and above the results, so an incorrect reading is visible and
    /// editable before it is believed.
    pub searched_as: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    pub prompt_version: u32,
    pub sections: Vec<Section>,
    pub sources: Vec<Source>,
    /// Anything true of the whole report rather than of one section.
    ///
    /// Today that is one thing: **which companies were named and not analysed.** Dropping a
    /// subject in silence is the defect multi-company support exists to remove, and doing it at
    /// a higher count would be the same defect wearing a bigger number.
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Report {
    /// Every source label referenced by a claim that has no matching entry in `sources`.
    ///
    /// A citation that does not resolve is worse than no citation: it looks checkable and
    /// is not. Callers reject a report rather than publishing one with dangling labels.
    #[must_use]
    pub fn dangling_source_labels(&self) -> Vec<&str> {
        let known: std::collections::HashSet<&str> =
            self.sources.iter().map(|s| s.label.as_str()).collect();
        let mut missing: Vec<&str> = self
            .sections
            .iter()
            .flat_map(|s| s.claims.iter())
            .map(|c| c.source_label.as_str())
            .filter(|label| !known.contains(label))
            .collect();
        missing.sort_unstable();
        missing.dedup();
        missing
    }

    /// Whether every claim resolves to a source we listed.
    #[must_use]
    pub fn every_claim_is_traceable(&self) -> bool {
        self.dangling_source_labels().is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;
    use crate::source::Disposition;

    fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_default()
    }

    fn source(label: &str) -> Source {
        Source {
            label: label.to_owned(),
            url: format!("https://example.test/{label}"),
            title: "Pricing".to_owned(),
            disposition: Disposition::Primary,
            fetched_at: at("2026-08-01T09:12:00Z"),
            independence_group: "G1".to_owned(),
        }
    }

    fn claim(source_label: &str) -> Claim {
        Claim {
            text: "Starter is $39 a month.".to_owned(),
            subject: String::new(),
            source_label: source_label.to_owned(),
            evidence_quote: "Starter  $39 per month, up to 25 orders".to_owned(),
            confidence: Confidence::High,
            as_of: at("2026-08-01T09:12:00Z"),
        }
    }

    fn report_with(claims: Vec<Claim>, sources: Vec<Source>) -> Report {
        Report {
            subject: "an app that helps small farms sell to restaurants".to_owned(),
            searched_as: "ordering software for small farms".to_owned(),
            generated_at: at("2026-08-01T09:14:00Z"),
            model_id: "qwen3-8b".to_owned(),
            prompt_version: 4,
            sections: vec![Section {
                key: "pricing".to_owned(),
                title: "Prices".to_owned(),
                status: SectionStatus::Populated,
                claims,
                checked: Vec::new(),
                notes: Vec::new(),
            }],
            sources,
            notes: Vec::new(),
        }
    }

    #[test]
    fn a_report_whose_claims_all_resolve_is_traceable() {
        let r = report_with(vec![claim("S1")], vec![source("S1")]);
        assert!(r.every_claim_is_traceable());
        assert!(r.dangling_source_labels().is_empty());
    }

    #[test]
    fn a_claim_citing_a_missing_source_is_caught() {
        let r = report_with(vec![claim("S1"), claim("S9")], vec![source("S1")]);
        assert!(!r.every_claim_is_traceable());
        assert_eq!(r.dangling_source_labels(), vec!["S9"]);
    }

    #[test]
    fn dangling_labels_are_deduplicated() {
        let r = report_with(vec![claim("S9"), claim("S9")], Vec::new());
        assert_eq!(r.dangling_source_labels(), vec!["S9"]);
    }

    #[test]
    fn a_not_found_section_records_what_was_checked() {
        let s = Section::not_found(
            "pricing",
            "Prices",
            vec!["/pricing (403)".to_owned(), "homepage".to_owned()],
        );
        assert_eq!(s.status, SectionStatus::NotFoundInPublicSources);
        assert!(s.claims.is_empty());
        assert_eq!(s.checked.len(), 2, "a negative must show its working");
    }

    #[test]
    fn the_schema_generates() {
        // The frontend types and the decoding grammar are both built from this. If it
        // stops generating, both drift silently, so it is worth a test of its own.
        let schema = schemars::schema_for!(Report);
        let json = serde_json::to_value(&schema).expect("schema serialises");
        assert!(json.get("properties").is_some(), "schema has properties");
    }
}
