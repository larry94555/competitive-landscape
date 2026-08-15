//! The whole report as one Markdown file, for the reader's own assistant.
//!
//! `IDEA_ANALYSIS.md` §5 settles a strategic question by building one small thing:
//!
//! > Most readers evaluating an idea already pay for a frontier chatbot, and the correct
//! > response is to **feed it rather than compete with it.** … We are not a worse chatbot. We
//! > are the **evidence file a chatbot cannot assemble**.
//!
//! So this is not an export format. It is the product's argument for itself, written down: a
//! reader pastes it into whatever they already use, and that assistant now has forty pages of
//! public evidence with a URL and a date against every sentence — which is the one part of the
//! job a chatbot genuinely cannot do for them.
//!
//! # Why this is Rust rather than a function in the browser
//!
//! The page already renders this data, so a second renderer in TypeScript would be a second
//! set of decisions about what a disposition is called and when a subject is worth printing.
//! Register entry 51: **a copy of a rule is a rule that agrees today.** `Disposition`'s own
//! `reader_description` writes those words once, here as on the page, and `landscape context`
//! and the API hand out the same bytes.
//!
//! # What it will not do
//!
//! **Say anything the report does not.** Every line below is a field, a label or one of this
//! module's own fixed headings. There is no summarizing, no re-ordering by importance and no
//! judgment about a publisher — the constraints `FACT_CHECKING.md` §3.2.5 puts on the report
//! are not relaxed by changing the file extension.
//!
//! **Drop anything in silence.** [`MAX_BYTES`] is a bound and a bound has to be reachable, so
//! when it is reached the document says which sections are missing and where the whole thing
//! is. A file that quietly stops halfway is worse than a short one, because a reader's
//! assistant will answer from it either way.

use std::fmt::Write as _;

/// One line ending, named so a `format!` can carry it without a `writeln!`.
const NEWLINE: &str = "\n";

use crate::report::{Report, Section, SectionStatus};

/// The most Markdown one report may produce.
///
/// **A paste box is a paste box.** 256 KiB is roughly 60,000 tokens of Markdown, and it is
/// eight times the largest report this pipeline can currently build — six companies, every
/// section full, fourteen sources, which measures **32 KiB**
/// (`the_biggest_report_this_pipeline_builds_is_far_inside_the_bound` checks that rather than
/// trusting this sentence). It exists for the same reason
/// `landscape_fetch::cache::MAX_CACHED_BYTES` does: the input is a stranger's website, and
/// *"reports are small"* is a fact about today.
pub const MAX_BYTES: usize = 256 * 1024;

/// The line `IDEA_ANALYSIS.md` §5 asks to lead with.
///
/// **Trimmed to what this report actually is.** §5's version ends *"argue with the framing
/// questions in sections 6 and 7"*; those sections are Phase 4 and do not exist, and pointing
/// an assistant at them would be the document lying about its own contents on line one. The
/// rest is unchanged, including the part that matters most — *tell me what the evidence does
/// not cover* — which is the request a reader could not easily make of a chatbot alone.
pub const OPENING_LINE: &str = concat!(
    "Here is a public-evidence report on an idea I am considering. Every claim below has a ",
    "source and a date. Tell me what the evidence does not cover, and where you would want ",
    "more before believing it."
);

/// The most of the budget the sources index may take.
///
/// **The index is written into the budget before the sections that cite it**, because a claim
/// whose `[S1]` resolves to nothing is worse than no claim: it looks checkable and is not, which
/// is the whole reason `Report::dangling_source_labels` exists. Review found that sections could
/// eat the budget and leave the index outside it, reproducing that defect by truncation.
///
/// Half rather than all of it, because the opposite failure is just as useless: an evidence
/// index with no findings under it is not a report either. Whatever does not fit in the half is
/// dropped, and every claim citing a dropped label goes with it — counted, and named in the
/// closing note.
const SOURCES_SHARE: usize = 2;

/// The most the closing note can cost, so room for it can be kept before it is written.
///
/// It is one heading, one sentence and three counts — all of this module's own words — plus the
/// permalink, which the caller supplies and is therefore added on top rather than assumed.
const CLOSING_NOTE_MAX: usize = 512;

/// The document under construction, and what would not fit in it.
///
/// **Every push goes through here, which is what makes [`MAX_BYTES`] a bound rather than an
/// intention.** The first version checked the size of section blocks and then appended the
/// sources index, the heading and the closing note unconditionally — and titles, URLs and notes
/// all come from pages we did not write, so a report could pass the check and leave by an
/// arbitrary margin. Review measured 882 KiB against a documented 256 KiB.
struct Paste<'a> {
    out: String,
    /// Kept for the closing note, so that the note itself always has somewhere to go.
    reserve: usize,
    permalink: Option<&'a str>,
    dropped_sections: Vec<&'a str>,
    dropped_lines: usize,
    dropped_sources: usize,
    dropped_claims: usize,
}

impl<'a> Paste<'a> {
    fn new(permalink: Option<&'a str>) -> Self {
        Self {
            out: String::with_capacity(4096),
            reserve: CLOSING_NOTE_MAX + permalink.map_or(0, str::len),
            permalink,
            dropped_sections: Vec::new(),
            dropped_lines: 0,
            dropped_sources: 0,
            dropped_claims: 0,
        }
    }

    /// Append a block if it fits beside what is still owed, and say whether it did.
    fn take(&mut self, block: &str) -> bool {
        if self.out.len() + block.len() + self.reserve > MAX_BYTES {
            return false;
        }
        self.out.push_str(block);
        true
    }

    /// A line of the heading. Counted when it does not fit, never shortened.
    fn line(&mut self, block: &str) {
        if !self.take(block) {
            self.dropped_lines += 1;
        }
    }

    /// **What is not in this file**, and where the whole of it is.
    ///
    /// Written last, from counts rather than from more of the report, because this is the one
    /// paragraph that has to be guaranteed room. Section titles are named when they fit —
    /// they are the useful version — and the fallback is counts alone, which cannot grow.
    fn finish(mut self) -> String {
        if self.dropped_sections.is_empty()
            && self.dropped_lines == 0
            && self.dropped_sources == 0
            && self.dropped_claims == 0
        {
            return self.out;
        }
        self.reserve = 0;
        let named = self.note(true);
        if !self.take(&named) {
            let counted = self.note(false);
            let _ = self.take(&counted);
        }
        self.out
    }

    fn note(&self, name_them: bool) -> String {
        let mut note = String::new();
        let _ = writeln!(note, "## What is not in this file");
        let _ = writeln!(note);
        let _ = write!(
            note,
            "This report was too long to paste in one piece, so {} section(s)",
            self.dropped_sections.len()
        );
        if name_them && !self.dropped_sections.is_empty() {
            let _ = write!(note, " ({})", self.dropped_sections.join(", "));
        }
        let _ = writeln!(
            note,
            ", {} claim(s), {} source(s) and {} other line(s) are missing here.",
            self.dropped_claims, self.dropped_sources, self.dropped_lines
        );
        if let Some(path) = self.permalink {
            let _ = writeln!(note, "They are on the report itself, at `{path}`.");
        }
        let _ = writeln!(note);
        note
    }
}

/// The report as Markdown, and the permalink to say where the rest of it is.
///
/// `permalink` is a path rather than a URL because this crate does not know the host it is
/// being served from, and a guessed one on a document meant to travel is worse than none.
///
/// The result is **never longer than [`MAX_BYTES`]**, whatever the report contains, and never
/// shorter without saying so.
#[must_use]
pub fn of(report: &Report, permalink: Option<&str>) -> String {
    let mut paste = Paste::new(permalink);

    // **The evidence index is chosen first, and the sections are then written into what is
    // left.** It appears at the bottom of the document and is decided at the top of this
    // function, because the alternative is a file whose claims cite labels it does not
    // contain — checkable-looking and uncheckable, which is the one thing this document exists
    // not to be.
    let (sources, kept) = sources_that_fit(&mut paste, report);
    paste.reserve += sources.len();

    heading(&mut paste, report, permalink);

    // **Sections in the report's own order**, which is the order the reader saw them in. Any
    // other order here would be this module having an opinion about importance.
    for section in &report.sections {
        let mut block = String::new();
        let anything = write_section(
            &mut block,
            report,
            section,
            &kept,
            &mut paste.dropped_claims,
        );
        // A heading with every claim removed under it is not a section, it is a puzzle.
        if !anything || !paste.take(&block) {
            paste.dropped_sections.push(section.title.as_str());
        }
    }

    // Written last and refused by nothing: the room was taken out of the budget above.
    paste.reserve -= sources.len();
    let _ = paste.take(&sources);
    paste.finish()
}

/// The sources index, trimmed to its share of the budget, and the labels that survived.
///
/// Greedy and in the report's own order, one line at a time, so a single page with a
/// preposterous title costs its own line rather than the whole index.
fn sources_that_fit<'a>(
    paste: &mut Paste<'_>,
    report: &'a Report,
) -> (String, std::collections::HashSet<&'a str>) {
    let mut out = String::new();
    let mut kept = std::collections::HashSet::new();
    let _ = writeln!(out, "## Sources");
    let _ = writeln!(out);
    if report.sources.is_empty() {
        let _ = writeln!(out, "No page was read.");
        let _ = writeln!(out);
        return (out, kept);
    }

    let share = (MAX_BYTES.saturating_sub(paste.reserve)) / SOURCES_SHARE;
    // **Every URL and every date**, which is §5's whole requirement: an assistant that cannot
    // check a claim is being asked to trust us, and that is the thing this file exists to
    // avoid asking.
    for source in &report.sources {
        let line = format!(
            "- **[{}]** {} — <{}> (read {}; {}){NEWLINE}",
            source.label,
            source.title,
            source.url,
            source.fetched_at.format("%Y-%m-%d"),
            source.disposition.reader_description()
        );
        if out.len() + line.len() > share {
            paste.dropped_sources += 1;
            continue;
        }
        out.push_str(&line);
        kept.insert(source.label.as_str());
    }
    let _ = writeln!(out);
    (out, kept)
}

fn heading(paste: &mut Paste<'_>, report: &Report, permalink: Option<&str>) {
    // **Line by line, because a heading is external data too.** The subject is what a reader
    // typed and the notes are built from pages we did not write; a single absurd one is
    // dropped and counted rather than carrying the document past its bound.
    paste.line(&format!(
        "{OPENING_LINE}{NEWLINE}{NEWLINE}---{NEWLINE}{NEWLINE}"
    ));
    paste.line(&format!("# {}{NEWLINE}{NEWLINE}", report.subject));

    // **Provenance before findings.** A reader's assistant should be able to tell how old this
    // is and what produced it before it reads a single claim.
    paste.line(&format!(
        "- Generated: {}{NEWLINE}",
        report.generated_at.format("%Y-%m-%d %H:%M UTC")
    ));
    // **`Report::model_id` is deliberately not here, and running this is how that was
    // found.** The field holds whatever identifies the model *to us*, which today is
    // `llm.base()` — the address of the inference server. On a laptop that is
    // `http://127.0.0.1:8080` and harmless; on the deployed box it is `LLAMA_URL`, an
    // internal host. This is the one document written to be pasted into somebody else's
    // service, so it carries what a reader can use and nothing about where we run.
    paste.line(&format!(
        "- Produced by: Landscape, prompt version {}{NEWLINE}",
        report.prompt_version
    ));
    if let Some(path) = permalink {
        paste.line(&format!("- This report: `{path}`{NEWLINE}"));
    }
    if report.searched_as != report.subject && !report.searched_as.is_empty() {
        paste.line(&format!("- Searched as: {}{NEWLINE}", report.searched_as));
    }
    if let Some(interpreted) = &report.interpreted {
        // The substitution that decided every query. Shown here for the same reason it is
        // shown on the page: if the reading is wrong, everything under it is about another
        // market, and only the reader can tell.
        paste.line(&format!(
            "- Interpreted as: **{}** ({} independent sites used that phrase{}){NEWLINE}",
            interpreted.label,
            interpreted.hosts,
            if interpreted.also.is_empty() {
                String::new()
            } else {
                format!("; also seen: {}", interpreted.also.join(", "))
            }
        ));
    }
    if !report.subjects.is_empty() {
        paste.line(&format!(
            "- Companies compared: {}{NEWLINE}",
            report.subjects.join(", ")
        ));
    }
    paste.line(NEWLINE);

    if !report.notes.is_empty() {
        paste.line(&format!("## About this comparison{NEWLINE}{NEWLINE}"));
        for note in &report.notes {
            paste.line(&format!("- {note}{NEWLINE}"));
        }
        paste.line(NEWLINE);
    }
}

/// One section, and whether anything of it survived.
///
/// A claim whose source did not fit is not printed: the label would resolve to nothing, and a
/// citation that looks checkable and is not is worse than a claim that was never made.
fn write_section(
    out: &mut String,
    report: &Report,
    section: &Section,
    kept: &std::collections::HashSet<&str>,
    dropped_claims: &mut usize,
) -> bool {
    let _ = writeln!(out, "## {}", section.title);
    let _ = writeln!(out);

    match section.status {
        SectionStatus::NotFoundInPublicSources => {
            // **A negative a reader can repeat.** `FACT_CHECKING.md` §5.4: a finding that
            // nothing was found is only worth anything beside the list of what was looked at.
            let _ = writeln!(out, "Nothing was found in public sources for this.");
            if !section.checked.is_empty() {
                let _ = writeln!(out);
                let _ = writeln!(out, "Checked:");
                for url in &section.checked {
                    let _ = writeln!(out, "- {url}");
                }
            }
        }
        SectionStatus::Populated | SectionStatus::Partial => {
            if section.status == SectionStatus::Partial {
                let _ = writeln!(out, "*Partial — some of this section could not be filled.*");
                let _ = writeln!(out);
            }
            for claim in &section.claims {
                if !kept.contains(claim.source_label.as_str()) {
                    *dropped_claims += 1;
                    continue;
                }
                // The subject travels with the claim and is printed on the same terms the page
                // prints it: only when the report covers more than one company, because on a
                // single-company report every line would carry the same name.
                let who = if report.subjects.len() > 1 && !claim.subject.is_empty() {
                    format!("**{}** — ", without_scheme(&claim.subject))
                } else {
                    String::new()
                };
                let _ = writeln!(
                    out,
                    "- {who}{} [{}] ({} confidence, as of {})",
                    claim.text,
                    claim.source_label,
                    confidence_word(claim.confidence),
                    claim.as_of.format("%Y-%m-%d")
                );
                let _ = writeln!(out, "  > {}", one_line(&claim.evidence_quote));
            }
        }
    }

    if !section.notes.is_empty() {
        let _ = writeln!(out);
        for note in &section.notes {
            let _ = writeln!(out, "*{note}*");
        }
    }
    let _ = writeln!(out);

    // A section that found nothing is still a finding; one whose every claim lost its source
    // is not.
    section.status == SectionStatus::NotFoundInPublicSources
        || section.claims.is_empty()
        || section
            .claims
            .iter()
            .any(|c| kept.contains(c.source_label.as_str()))
}

/// A quote on one line, because a `>` block that contains newlines stops being a quote.
///
/// Whitespace only — no words are removed, no ellipsis is added, and nothing is shortened.
/// **A verbatim span that has been edited is not a verbatim span**, which is the whole reason
/// `Claim` carries one.
fn one_line(quote: &str) -> String {
    quote.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn without_scheme(origin: &str) -> &str {
    origin
        .trim_start_matches("https://")
        .trim_start_matches("http://")
}

const fn confidence_word(confidence: crate::report::Confidence) -> &'static str {
    match confidence {
        crate::report::Confidence::High => "high",
        crate::report::Confidence::Medium => "medium",
        crate::report::Confidence::Low => "low",
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Panicking IS how a test reports failure. The lints stay denied everywhere else.
mod tests {
    use super::*;
    use crate::report::{Claim, Confidence, Interpreted, Section, SectionStatus};
    use crate::source::{Disposition, Source};

    fn when() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-05T09:30:00Z")
            .expect("a fixed instant")
            .with_timezone(&chrono::Utc)
    }

    fn claim(text: &str, subject: &str, label: &str, quote: &str) -> Claim {
        Claim {
            text: text.to_owned(),
            subject: subject.to_owned(),
            source_label: label.to_owned(),
            evidence_quote: quote.to_owned(),
            confidence: Confidence::High,
            as_of: when(),
        }
    }

    fn source(label: &str, url: &str, disposition: Disposition) -> Source {
        Source {
            label: label.to_owned(),
            url: url.to_owned(),
            title: format!("{label} page"),
            disposition,
            fetched_at: when(),
            independence_group: url.to_owned(),
        }
    }

    fn two_companies() -> Report {
        Report {
            chosen: None,
            // A finished report, so it carries a finished run.
            progress: Some(crate::Progress::finished(2)),
            asked: None,
            searches: None,
            subject: "basecamp.com linear.app".to_owned(),
            searched_as: "basecamp.com, linear.app".to_owned(),
            generated_at: when(),
            model_id: "test-model".to_owned(),
            prompt_version: 3,
            subjects: vec![
                "https://basecamp.com".to_owned(),
                "https://linear.app".to_owned(),
            ],
            sections: vec![
                Section {
                    key: "pricing".to_owned(),
                    title: "Pricing & packaging".to_owned(),
                    status: SectionStatus::Populated,
                    claims: vec![
                        claim(
                            "Pro costs $15 per user per month",
                            "https://basecamp.com",
                            "S1",
                            "Pro   is\n$15 per user\nper month",
                        ),
                        claim(
                            "Business costs $8 per user per month",
                            "https://linear.app",
                            "S2",
                            "Business $8 per user per month",
                        ),
                    ],
                    checked: Vec::new(),
                    notes: vec!["linear.app: 1 question was not answered.".to_owned()],
                },
                Section::not_found(
                    "trust",
                    "Trust posture",
                    vec!["https://basecamp.com/security".to_owned()],
                ),
            ],
            sources: vec![
                source("S1", "https://basecamp.com/pricing", Disposition::Primary),
                source("S2", "https://linear.app/pricing", Disposition::Primary),
            ],
            interpreted: Some(Interpreted {
                label: "project management software".to_owned(),
                also: vec!["team collaboration tools".to_owned()],
                hosts: 4,
            }),
            notes: vec!["You named basecamp.com and linear.app.".to_owned()],
        }
    }

    #[test]
    fn every_claim_arrives_with_a_url_and_a_date() {
        // §5's requirement in one assertion: an assistant that cannot check a claim is being
        // asked to trust us, and that is exactly what this file exists not to ask.
        let report = two_companies();
        let md = of(&report, Some("/a/abc"));
        for claim in report.sections.iter().flat_map(|s| s.claims.iter()) {
            // **On the claim's own line.** Asserting the label appears *somewhere* passes
            // with every claim unlabeled, because the sources index below lists them all —
            // the harness found that, and it is the same defect as looking for a company
            // name on a page that repeats it.
            let line = md
                .lines()
                .find(|l| l.contains(&claim.text))
                .unwrap_or_else(|| panic!("the claim itself is missing: {}", claim.text));
            assert!(
                line.contains(&format!("[{}]", claim.source_label)),
                "a claim with no label to resolve: {line}"
            );
        }
        for source in &report.sources {
            assert!(md.contains(&source.url), "the URL for {}", source.label);
        }
        assert!(md.contains("2026-08-05"), "a date on the evidence");
    }

    #[test]
    fn nothing_about_where_this_runs_travels_with_the_document() {
        // **Found by running it**, not by review: the provenance line printed `model_id`,
        // which is the inference server's base URL. A file whose entire purpose is to be
        // pasted into a third party must not carry our own addresses.
        let mut report = two_companies();
        report.model_id = "http://10.4.1.9:8080".to_owned();
        let md = of(&report, Some("/a/abc"));
        for internal in ["10.4.1.9", "8080", "http://10"] {
            assert!(
                !md.contains(internal),
                "{internal} reached the document: {md}"
            );
        }
        // What a reader *can* use is still there.
        assert!(md.contains("prompt version 3"), "{md}");
    }

    #[test]
    fn it_opens_with_the_line_that_makes_it_useful() {
        // The reader is handing this to something that will answer from it either way. The
        // opening line is what turns a paste into a question worth asking.
        let md = of(&two_companies(), None);
        assert!(md.starts_with(OPENING_LINE), "{}", &md[..80.min(md.len())]);
        assert!(
            md.contains("what the evidence does not cover"),
            "the request only this file makes possible"
        );
        // And it must not point at sections this report does not have.
        assert!(!md.contains("sections 6 and 7"), "{md}");
    }

    #[test]
    fn a_quote_is_never_edited_beyond_its_own_whitespace() {
        // **A verbatim span that has been shortened is not a verbatim span.** Line breaks
        // inside a `>` block would end the quote, so they collapse; nothing else changes.
        let md = of(&two_companies(), None);
        assert!(
            md.contains("> Pro is $15 per user per month"),
            "the quote, on one line, with every word: {md}"
        );
        assert!(!md.contains("..."), "nothing was elided");
    }

    #[test]
    fn a_company_is_named_only_when_the_report_compares_several() {
        // The same rule the page follows. On a report about one company every line would
        // carry the same name, which is noise rather than attribution.
        let both = of(&two_companies(), None);
        assert!(both.contains("**basecamp.com** — Pro costs $15"), "{both}");

        let mut alone = two_companies();
        alone.subjects = vec!["https://basecamp.com".to_owned()];
        alone.sections[0].claims.truncate(1);
        let one = of(&alone, None);
        assert!(one.contains("- Pro costs $15"), "{one}");
        assert!(!one.contains("**basecamp.com** —"), "{one}");
    }

    #[test]
    fn a_section_that_found_nothing_says_what_was_checked() {
        // A negative nobody can repeat is not a finding — FACT_CHECKING §5.4.
        let md = of(&two_companies(), None);
        assert!(md.contains("Nothing was found in public sources"), "{md}");
        assert!(md.contains("https://basecamp.com/security"), "{md}");
    }

    #[test]
    fn a_source_is_described_in_the_words_the_report_already_uses() {
        // `Disposition::reader_description` is written once and read here, so this file and
        // the page cannot come to different views of what `Attributed` means.
        let mut report = two_companies();
        report.sources[1].disposition = Disposition::Attributed;
        let md = of(&report, None);
        assert!(
            md.contains(Disposition::Primary.reader_description()),
            "{md}"
        );
        assert!(
            md.contains(Disposition::Attributed.reader_description()),
            "{md}"
        );
    }

    #[test]
    fn the_substitution_that_decided_every_query_is_disclosed() {
        let md = of(&two_companies(), None);
        assert!(md.contains("project management software"), "{md}");
        assert!(md.contains("4 independent sites"), "{md}");
        assert!(md.contains("team collaboration tools"), "{md}");
    }

    #[test]
    fn a_report_too_long_to_paste_says_which_parts_are_missing() {
        // **The bound has to be reachable and it has to be honest.** A file that stops
        // halfway is worse than a short one: the assistant answers from it either way.
        let mut report = two_companies();
        let long = "x".repeat(4096);
        for n in 0..200 {
            report.sections.push(Section {
                key: format!("k{n}"),
                title: format!("Section {n}"),
                status: SectionStatus::Populated,
                claims: vec![claim(&long, "https://basecamp.com", "S1", &long)],
                checked: Vec::new(),
                notes: Vec::new(),
            });
        }
        let md = of(&report, Some("/a/abc"));
        assert!(
            md.len() <= MAX_BYTES,
            "{} bytes past a hard bound",
            md.len()
        );
        assert!(md.contains("What is not in this file"), "{}", &md[..400]);
        assert!(md.contains("/a/abc"), "where the rest of it is");
        // The sources index survives the cut, because it is what makes the rest checkable.
        assert!(md.contains("https://basecamp.com/pricing"), "sources kept");
    }

    #[test]
    fn the_biggest_report_this_pipeline_builds_is_far_inside_the_bound() {
        // **The number in `MAX_BYTES`'s docstring, checked rather than remembered.** Six
        // companies is `landscape_search::competitors::MOST` in practice, six sections is
        // every question kind there is, and three claims each is a generous page. If this
        // ever stops being comfortable the bound is the thing to revisit, and this fails
        // first rather than a reader finding a truncated file.
        let mut report = two_companies();
        report.subjects = (0..6).map(|n| format!("https://c{n}.example")).collect();
        report.sections = (0..6)
            .map(|n| Section {
                key: format!("k{n}"),
                title: format!("Section {n}"),
                status: SectionStatus::Populated,
                claims: (0..6)
                    .flat_map(|c| {
                        (0..3).map(move |i| {
                            claim(
                                &format!("A claim of the length these run to, {i} for {c}"),
                                &format!("https://c{c}.example"),
                                "S1",
                                "A verbatim span, about the length a pricing row is.",
                            )
                        })
                    })
                    .collect(),
                checked: Vec::new(),
                notes: Vec::new(),
            })
            .collect();
        report.sources = (0..14)
            .map(|n| {
                source(
                    &format!("S{n}"),
                    &format!("https://c{n}.example/pricing"),
                    Disposition::Primary,
                )
            })
            .collect();

        let md = of(&report, Some("/a/abc"));
        assert!(
            md.len() < MAX_BYTES / 4,
            "a full report is {} bytes; MAX_BYTES's claim that reports are far inside it no longer holds",
            md.len()
        );
        assert!(!md.contains("What is not in this file"), "nothing was cut");
    }

    #[test]
    fn nothing_outside_the_sections_can_carry_the_file_past_its_bound() {
        // **Review measured 882 KiB against a documented 256 KiB.** The first version checked
        // section blocks and then appended the sources index, the heading and the closing note
        // unconditionally - and titles, URLs, subjects and notes all come from pages we did not
        // write. Each of those is its own oversized case here.
        let huge = "x".repeat(300_000);

        let mut sources = two_companies();
        sources.sources = (0..40)
            .map(|n| {
                let mut src = source("S1", "https://a.example", Disposition::Primary);
                src.title = "t".repeat(20_000);
                src.url = format!("https://{}.example", "u".repeat(n * 100 + 1));
                src
            })
            .collect();

        let mut notes = two_companies();
        notes.notes = vec![huge.clone(), huge.clone()];

        let mut subject = two_companies();
        subject.subject = huge.clone();
        subject.searched_as = huge.clone();

        let mut interpreted = two_companies();
        interpreted.interpreted = Some(Interpreted {
            label: huge.clone(),
            also: vec![huge.clone()],
            hosts: 3,
        });

        let mut named = two_companies();
        named.subjects = vec![huge.clone(), huge];

        for (what, report) in [
            ("sources", sources),
            ("report notes", notes),
            ("the subject", subject),
            ("the interpreted line", interpreted),
            ("the company list", named),
        ] {
            let md = of(&report, Some("/a/abc"));
            assert!(
                md.len() <= MAX_BYTES,
                "{what}: {} bytes past a hard bound",
                md.len()
            );
            assert!(
                md.contains("What is not in this file"),
                "{what}: something was dropped and the file does not say so"
            );
        }
    }

    #[test]
    fn a_source_too_long_to_print_costs_its_own_line_and_no_more() {
        // One page with a preposterous title must not take the index with it: the rest of the
        // evidence is what keeps the rest of the claims checkable.
        let mut report = two_companies();
        let mut absurd = source("S9", "https://absurd.example", Disposition::Primary);
        absurd.title = "t".repeat(MAX_BYTES);
        report.sources.push(absurd);

        let md = of(&report, Some("/a/abc"));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
        assert!(
            md.contains("https://basecamp.com/pricing"),
            "the others stay"
        );
        assert!(md.contains("https://linear.app/pricing"), "the others stay");
        assert!(
            md.contains("1 source(s)"),
            "and the missing one is counted: {md}"
        );
    }

    #[test]
    fn a_document_packed_to_the_last_byte_still_has_room_to_say_what_is_missing() {
        // **This is what the reserve is for, and coarse blocks hide it.** With sections of a
        // few kilobytes the greedy fill happens to stop with kilobytes to spare, so the note
        // fits whether or not room was kept for it. Small sections pack tight: the fill stops
        // when the next one will not fit, which without a reserve is a handful of bytes short
        // of the bound - and the note is longer than that.
        let mut report = two_companies();
        report.sections = (0..20_000)
            .map(|n| Section {
                key: format!("k{n}"),
                title: format!("S{n}"),
                status: SectionStatus::Populated,
                claims: vec![claim("a claim", "https://basecamp.com", "S1", "a quote")],
                checked: Vec::new(),
                notes: Vec::new(),
            })
            .collect();

        let md = of(&report, Some("/a/abc"));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
        assert!(
            md.contains("What is not in this file"),
            "the file was cut and does not say so; the last {} bytes are: {}",
            200.min(md.len()),
            &md[md.len().saturating_sub(200)..]
        );
        assert!(md.contains("/a/abc"), "and where the rest of it is");
    }

    /// Every `[S1]` a claim cites, against every `**[S1]**` the index resolves.
    ///
    /// The rendered document rather than the report, because truncation is what breaks this
    /// and truncation only exists here.
    fn dangling_labels(md: &str) -> Vec<String> {
        let resolved: std::collections::HashSet<String> = md
            .lines()
            .filter_map(|l| l.strip_prefix("- **["))
            .filter_map(|l| l.split_once("]**"))
            .map(|(label, _)| label.to_owned())
            .collect();
        md.lines()
            .filter(|l| l.starts_with("- ") && !l.starts_with("- **["))
            .filter_map(|l| l.rsplit_once(" ["))
            .filter_map(|(_, rest)| rest.split_once(']'))
            .map(|(label, _)| label.to_owned())
            .filter(|label| !resolved.contains(label))
            .collect()
    }

    #[test]
    fn a_claim_that_survives_truncation_keeps_the_source_that_resolves_it() {
        // **A citation that does not resolve is worse than no citation**: it looks checkable
        // and is not, which is why `Report::dangling_source_labels` exists. Review found that
        // sections could eat the budget and leave the index outside it, reproducing exactly
        // that defect by truncation — claims citing `[S1]` in a file with no `S1` in it.
        let mut report = two_companies();
        report.sections = (0..20_000)
            .map(|n| Section {
                key: format!("k{n}"),
                title: format!("S{n}"),
                status: SectionStatus::Populated,
                claims: vec![claim("a claim", "https://basecamp.com", "S1", "a quote")],
                checked: Vec::new(),
                notes: Vec::new(),
            })
            .collect();

        let md = of(&report, Some("/a/abc"));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
        assert!(
            md.contains("**[S1]**"),
            "the index survived: {}",
            &md[..200]
        );
        assert_eq!(
            dangling_labels(&md),
            Vec::<String>::new(),
            "a claim cites a label this file does not resolve"
        );
    }

    #[test]
    fn a_claim_whose_source_did_not_fit_is_not_printed_at_all() {
        // The other side of the same rule. When the index itself has to be trimmed, the claims
        // that cited what was trimmed go with it — counted, and named in the closing note,
        // because a reader who cannot see them can still ask for them.
        let mut report = two_companies();
        // One source large enough to take the whole share on its own, so the second cannot fit.
        // Wide margins on purpose: the first source is comfortably inside the share and the
        // second is comfortably outside what is left of it, so this does not turn on the exact
        // length of a line of prose.
        report.sources[0].title = "t".repeat(MAX_BYTES / SOURCES_SHARE / 2);
        report.sources[1].title = "t".repeat(MAX_BYTES / SOURCES_SHARE);

        let md = of(&report, Some("/a/abc"));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
        assert_eq!(
            dangling_labels(&md),
            Vec::<String>::new(),
            "a claim cites a label this file does not resolve"
        );
        assert!(
            md.contains("claim(s)"),
            "the dropped claims are counted: {md}"
        );
    }

    #[test]
    fn a_section_that_loses_every_claim_is_not_left_as_an_empty_heading() {
        // A heading with everything removed under it is not a section, it is a puzzle: a
        // reader cannot tell it from one where nothing was ever found, and that is a finding.
        let mut report = two_companies();
        report.sections.push(Section {
            key: "changes".to_owned(),
            title: "Recent public changes".to_owned(),
            status: SectionStatus::Populated,
            claims: vec![claim(
                "Shipped a thing",
                "https://linear.app",
                "S2",
                "shipped",
            )],
            checked: Vec::new(),
            notes: Vec::new(),
        });
        // S1 takes the whole share, so S2 and everything citing it go with it.
        // Wide margins on purpose: the first source is comfortably inside the share and the
        // second is comfortably outside what is left of it, so this does not turn on the exact
        // length of a line of prose.
        report.sources[0].title = "t".repeat(MAX_BYTES / SOURCES_SHARE / 2);
        report.sources[1].title = "t".repeat(MAX_BYTES / SOURCES_SHARE);

        let md = of(&report, Some("/a/abc"));
        assert!(md.contains("**[S1]**"), "S1 survived");
        assert!(!md.contains("**[S2]**"), "S2 did not");
        assert!(
            !md.contains("## Recent public changes"),
            "a heading with nothing under it: {md}"
        );
        // The section that kept a claim is still there, and still whole.
        assert!(md.contains("## Pricing & packaging"), "{md}");
        assert!(md.contains("Pro costs $15 per user per month"), "{md}");
        assert_eq!(dangling_labels(&md), Vec::<String>::new());
    }

    #[test]
    fn the_index_may_not_take_the_whole_file_either() {
        // **An evidence index with no findings under it is not a report.** The share is what
        // stops the fix for one failure from becoming the other one.
        let mut report = two_companies();
        report.sources = (0..400)
            .map(|n| {
                let mut src = source("S1", "https://a.example", Disposition::Primary);
                src.title = "t".repeat(4096);
                src.url = format!("https://s{n}.example");
                src
            })
            .collect();

        let md = of(&report, Some("/a/abc"));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
        assert!(
            md.matches("- **[").count() > 0,
            "some of the index survives"
        );
        assert!(md.contains("source(s)"), "and the rest of it is counted");
        // The point of the share, stated as what it buys: the findings are still under it.
        assert!(
            md.contains("Pro costs $15 per user per month"),
            "the index crowded out the report it is evidence for"
        );
        assert_eq!(dangling_labels(&md), Vec::<String>::new());
    }

    #[test]
    fn the_closing_note_always_has_somewhere_to_go() {
        // The note is the one paragraph that must fit, because it is what stops a truncated
        // file from reading as a whole one. `CLOSING_NOTE_MAX` is reserved before anything
        // else is written, and the caller's permalink is added on top rather than assumed.
        let mut report = two_companies();
        report.sections = (0..400)
            .map(|n| Section {
                key: format!("k{n}"),
                title: format!("Section {n}"),
                status: SectionStatus::Populated,
                claims: vec![claim(
                    &"c".repeat(4096),
                    "https://basecamp.com",
                    "S1",
                    &"q".repeat(4096),
                )],
                checked: Vec::new(),
                notes: Vec::new(),
            })
            .collect();

        for permalink in [None, Some("/a/abc"), Some("/a/" as &str)] {
            let md = of(&report, permalink);
            assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
            assert!(md.contains("What is not in this file"), "the note is there");
            assert!(md.contains("section(s)"), "and it counts them");
        }

        // Even a permalink nobody would write cannot push the file past the bound.
        let silly = "/a/".to_owned() + &"z".repeat(100_000);
        let md = of(&report, Some(&silly));
        assert!(md.len() <= MAX_BYTES, "{} bytes", md.len());
    }

    #[test]
    fn a_report_that_fits_says_nothing_about_being_cut() {
        let md = of(&two_companies(), Some("/a/abc"));
        assert!(!md.contains("What is not in this file"), "{md}");
    }

    #[test]
    fn a_report_with_no_sources_says_so_rather_than_printing_a_heading_over_nothing() {
        let mut report = two_companies();
        report.sources.clear();
        report.sections = vec![Section::not_found("trust", "Trust posture", Vec::new())];
        let md = of(&report, None);
        assert!(md.contains("No page was read."), "{md}");
    }
}
