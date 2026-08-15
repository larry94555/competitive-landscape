//! Whether a question has one answer or several, decided from the market's own writing.
//!
//! `IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md` PR 7, and the rest of cause 1 in
//! `PRODUCT_IDEA_RESULTS_LOGIC.md` §4.
//!
//! # The question this answers
//!
//! A reader typed *"project management software"*. That is a real question with several real
//! answers — for agencies, for software teams, for construction — and a single ranked list of
//! five vendors is a confident answer to a question nobody asked. A general web answer handles
//! this by offering the subcategories; the reader who reported the original failure said so.
//!
//! # Where the categories come from
//!
//! **The headings of the pages already fetched.** [`crate::literature`] reads the buyer's guides
//! a search returned, and a guide to a broad market is organized by category: *Best for creative
//! agencies*, *Best for software teams*. A heading is structure **the page wrote**, in the same
//! sense a link is something the page did — no model, no summarizing, nothing asserted that was
//! not read.
//!
//! **And it costs nothing.** These are the same four pages `literature` already fetched for the
//! same run; this module reads them again from memory rather than asking anybody's server twice.
//!
//! # The two decisions, and both thresholds are hypotheses
//!
//! ```text
//! a category is OFFERED when
//!     its heading describes a kind of buyer or use  (`for ...`, `with ...`)
//!     and it is named on >= NAMED_BY_HOSTS independent hosts
//!     and it has >= COMPANIES_PER_CATEGORY companies under it
//!
//! the question is TOO GENERAL when
//!     >= 2 categories clear that bar
//!     and no single category holds more than CONCENTRATION of the candidates
//! ```
//!
//! **Labeled as starting values, exactly as [`landscape_core::subject::AMBIGUITY_MARGIN`] is.**
//! The roadmap says this row is a research problem and should be measured hardest; the numbers
//! below are where that measuring starts, not where it ended.

use std::collections::HashMap;

use crate::literature::{Endorsement, Reading};

/// How many independent publishers must name a category before it is offered to a reader.
///
/// **[`crate::candidates::CORROBORATION`], for the same reason [`crate::literature`] uses it for
/// companies.** One guide's table of contents is one editor's opinion about how a market
/// divides; two unrelated guides agreeing is the market dividing that way.
pub const NAMED_BY_HOSTS: usize = crate::candidates::CORROBORATION;

/// How much of the candidate set one category may hold before the question counts as answered.
///
/// **Sixty percent, and it is a hypothesis.** The claim is narrow: if most of the companies
/// found sit under one heading, a reader asking about that market got the market they asked
/// about, and offering to narrow it is interrogation rather than help — `PRODUCT_SPEC.md` §3's
/// *never ask something inferable from the input*. Where exactly "most" sits is a number nobody
/// has measured, and it is labeled here so the next person moves it deliberately.
pub const CONCENTRATION: f32 = 0.60;

/// How many companies a heading must hold before it is a way the market divides.
///
/// **Two, and this is the rule that does the work — the one before it did not.** The first version
/// asked only that a heading not be *the name of the company under it*, which is a negative
/// signal and leaked twice: `## Jira` linking `atlassian.com` and `## Workfront` linking
/// `adobe.com` are vendor reviews whose product names are nothing like their corporate domains,
/// and both survived as categories. Two guides doing that refused an ordinary
/// project-management report and offered *Jira* and *Workfront* as submarkets.
///
/// **A vendor's review links to that vendor. A category lists several.** That is a fact about the
/// page's structure rather than a guess about its words, it needs no model and no list of
/// product names, and it does not care whether a product is named after its company.
///
/// Counted across the guides that use the heading, because a guide naming two examples under a
/// category and another naming one is still a market two publishers divided the same way.
pub const COMPANIES_PER_CATEGORY: usize = 2;

/// One way a market divides, as the guides describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// The heading, as written. Shown to a reader, so it is the page's words rather than ours.
    pub label: String,
    /// The publishers whose guides use this heading, sorted and without repeats.
    pub named_by: Vec<String>,
    /// The companies linked under it, by registrable host.
    pub companies: Vec<String>,
}

/// A heading, normalized enough that two guides can be seen to agree.
///
/// **Lowercased, and stripped of the words every guide's headings carry.** *"Best for creative
/// agencies"* and *"For creative agencies"* are one category described twice, and a comparison
/// that missed that would need three guides to agree before two of them did.
///
/// It is deliberately not more than this. Stemming, synonyms and *for agencies ≈ for design
/// teams* are judgments about meaning, and a judgment about meaning made by a program with no
/// model behind it is a guess wearing a rule's clothes.
#[must_use]
pub fn as_category(heading: &str) -> String {
    let lowered = heading.trim().trim_start_matches('#').trim().to_lowercase();
    let mut words: Vec<&str> = lowered.split_whitespace().collect();
    while let Some(first) = words.first() {
        if matches!(*first, "best" | "top" | "the" | "our") {
            words.remove(0);
        } else {
            break;
        }
    }
    words.join(" ")
}

/// The words a heading must start with to be describing a **kind of buyer or use**.
///
/// **A market division is a qualifier, and English marks one with a preposition.** *For creative
/// agencies*, *for software teams*, *with a free tier* — each one narrows the market the guide is
/// about. *Best Overall*, *Best Value*, *Editor's Choice* and *Runner Up* are selection awards:
/// they rank the same market rather than dividing it.
///
/// **This is the third rule this module has had for the same question, and the first positive
/// one.** The first asked *is this heading the name of the company under it*; the second asked
/// *does it hold more than one company*. Both are things a category is **not**, and review found
/// a case each rule let through — `## Jira` linking `atlassian.com`, then `## Best Overall` and
/// `## Best Value` linking two winners apiece. A rule made of exclusions is a rule with a next
/// exception.
const QUALIFIERS: [&str; 3] = ["for", "with", "without"];

/// Whether a heading describes a kind of buyer or use rather than a place in a ranking.
///
/// **Deliberately narrow, and biased toward saying no.** A category this misses costs nothing: the
/// gate does not fire and the report is written exactly as it was before this module existed. A
/// category this invents **stops a report that would have been right**, which is what happened
/// three times in review. *Agency project management* and *Enterprise* are real headings this
/// will not recognize, and that is the trade taken on purpose.
#[must_use]
pub fn describes_a_kind_of_buyer(label: &str) -> bool {
    label
        .split_whitespace()
        .next()
        .is_some_and(|first| QUALIFIERS.contains(&first))
        && label.split_whitespace().count() >= 2
}

/// Split a guide into its sections: each heading, and the links that follow it.
///
/// **The links under a heading, not the links on the page.** A guide's first heading is its
/// title and everything on the page follows it; what makes *Best for creative agencies* a
/// category with companies in it is the three links between that heading and the next one.
///
/// The page's own title — the first `#` — is not a category. It is the market, which is the
/// thing being divided.
#[must_use]
pub fn sections(page: &str, publisher: &str) -> Vec<(String, Vec<Endorsement>)> {
    let mut out: Vec<(String, Vec<Endorsement>)> = Vec::new();
    let mut heading: Option<String> = None;
    let mut body = String::new();

    let mut flush = |heading: &mut Option<String>, body: &mut String| {
        if let Some(label) = heading.take() {
            out.push((label, crate::literature::linked_from(body, publisher)));
        }
        body.clear();
    };

    for line in page.lines() {
        let hashes = line.len() - line.trim_start_matches('#').len();
        if hashes >= 2 && line.starts_with('#') {
            flush(&mut heading, &mut body);
            heading = Some(line.trim_start_matches('#').trim().to_owned());
        } else if line.starts_with('#') {
            // The page's own title. Everything before the first real heading belongs to nothing.
            flush(&mut heading, &mut body);
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut heading, &mut body);
    out
}

/// The categories the guides agree on, best supported first.
///
/// A category needs [`NAMED_BY_HOSTS`] independent publishers, [`COMPANIES_PER_CATEGORY`]
/// companies under it, and not to be one vendor's own review. A heading two guides share with
/// nothing beneath it is a section of prose; a heading with one company under it is that
/// company.
#[must_use]
pub fn categories(read: &HashMap<String, String>) -> Vec<Category> {
    // label -> (publishers, companies)
    let mut by_label: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();
    // Sorted, so the categories a reader is offered do not depend on how a map iterated.
    let mut pages: Vec<(&String, &String)> = read.iter().collect();
    pages.sort();

    for (publisher, page) in pages {
        for (heading, linked) in sections(page, publisher) {
            let label = as_category(&heading);
            // **A positive signal, and it is the point.** Everything before this asked what a
            // category is not; this asks what one is. See `describes_a_kind_of_buyer`.
            if !describes_a_kind_of_buyer(&label) {
                continue;
            }
            let entry = by_label.entry(label).or_default();
            if !entry.0.contains(publisher) {
                entry.0.push(publisher.clone());
            }
            for one in linked {
                if !entry.1.contains(&one.host) {
                    entry.1.push(one.host);
                }
            }
        }
    }

    let mut out: Vec<Category> = by_label
        .into_iter()
        .filter(|(_, (by, companies))| {
            by.len() >= NAMED_BY_HOSTS && companies.len() >= COMPANIES_PER_CATEGORY
        })
        .map(|(label, (named_by, companies))| Category {
            label,
            named_by,
            companies,
        })
        .collect();
    out.sort_by(|a, b| {
        b.named_by
            .len()
            .cmp(&a.named_by.len())
            .then_with(|| b.companies.len().cmp(&a.companies.len()))
            .then_with(|| a.label.cmp(&b.label))
    });
    out
}

/// The categories in a run's literature, read from the pages it already fetched.
#[must_use]
pub fn of(reading: &Reading) -> Vec<Category> {
    categories(&reading.pages)
}

/// Whether the question covers several markets rather than one.
///
/// **Both sides of the fraction are the companies in the answer**, and review found them being
/// two different populations. `Category::companies` is everything the guides linked, including
/// companies that were never candidates, were set aside, or ranked past the fetch budget; the
/// denominator was the admitted set. Four linked in each of two categories against five admitted
/// gave `4 / 5` twice, so a question that was genuinely two markets read as concentrated in
/// both. A share of one universe over the size of another is not a share.
///
/// `admitted` is the **distinct domains** in the set, which is also what stops one vendor's two
/// products counting as two companies.
///
/// **Two conditions, and the second is the one doing the work.** Any broad market has
/// subheadings; what makes a question *too general* is that the companies in the answer are
/// spread across them. A reader who asked about project management for agencies and got mostly
/// agency tools was answered.
#[must_use]
pub fn too_general(categories: &[Category], admitted: &[String]) -> bool {
    if admitted.is_empty() {
        return false;
    }
    // Only the categories that hold somebody who is actually in the answer. A heading dividing
    // companies nobody will be shown divides nothing a reader can see.
    let held: Vec<usize> = categories
        .iter()
        .map(|c| {
            c.companies
                .iter()
                .filter(|host| admitted.contains(host))
                .count()
        })
        .filter(|n| *n > 0)
        .collect();
    if held.len() < 2 {
        return false;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are single digits; NAMED caps the candidates at five"
    )]
    let share = |n: usize| n as f32 / admitted.len() as f32;
    !held.iter().any(|n| share(*n) > CONCENTRATION)
}

/// The distinct domains in a set, which is the universe [`too_general`] divides.
///
/// **Distinct, because a vendor with two products is one company on a page.** `Set::members` is
/// keyed on a domain today and `products::split` can put two products of one vendor in front of
/// a reader tomorrow; counting rows would make that vendor two.
#[must_use]
pub fn admitted(set: &crate::competitors::Set) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for member in &set.members {
        if !out.contains(&member.candidate.canonical_domain) {
            out.push(member.candidate.canonical_domain.clone());
        }
    }
    out
}

/// The categories as something a reader can click.
///
/// **The same shape the ambiguous-company question already uses**, and for the reason
/// [`crate::vocabulary::choices_from`] gives: the interface has one way of asking *which did you
/// mean* rather than three. `domain` is empty because a category has no website, and what stands
/// in its place is the evidence there is for it — how many guides named it, and how many
/// companies they put under it.
///
/// **Each chip carries a whole prompt**, so a click is a new run with its own URL rather than a
/// word to be typed back into a sentence. The reader's own words come first and the category
/// follows, unless the category already contains them.
#[must_use]
pub fn choices_from(between: &[Category], asked: &str) -> Vec<landscape_core::Choice> {
    between
        .iter()
        .map(|c| landscape_core::Choice {
            name: c.label.clone(),
            domain: String::new(),
            what_it_is: format!(
                "{} guides list {} {} under this",
                c.named_by.len(),
                c.companies.len(),
                if c.companies.len() == 1 {
                    "company"
                } else {
                    "companies"
                }
            ),
            prompt: if c.label.contains(&asked.to_lowercase()) {
                c.label.clone()
            } else {
                format!("{asked} {}", c.label)
            },
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const G2: &str = "# Best Project Management Software\n\n\
        Our picks, by the kind of team you run.\n\n\
        ## Best for creative agencies\n\n\
        [Workamajig](https://www.workamajig.com/) and [Asana](https://asana.com/).\n\n\
        ## Best for software teams\n\n\
        [Linear](https://linear.app/) and [Jira](https://www.atlassian.com/software/jira).\n";

    const CAPTERRA: &str = "# Project Management Software\n\n\
        ## For creative agencies\n\n\
        [Workamajig](https://www.workamajig.com/).\n\n\
        ## For software teams\n\n\
        [Linear](https://linear.app/).\n\n\
        ## Methodology\n\n\
        How we rank.\n";

    fn read(of: &[(&str, &str)]) -> HashMap<String, String> {
        of.iter()
            .map(|(host, page)| ((*host).to_owned(), (*page).to_owned()))
            .collect()
    }

    #[test]
    fn a_heading_is_the_pages_own_word_for_a_category() {
        // Two guides describing one category two ways. Stripping the words every guide's
        // headings carry is the whole of the normalizing: anything more is a judgment about
        // meaning, made by a program with nothing behind it.
        assert_eq!(
            as_category("## Best for creative agencies"),
            "for creative agencies"
        );
        assert_eq!(
            as_category("For creative agencies"),
            "for creative agencies"
        );
        assert_eq!(
            as_category("# Best Project Management Software"),
            "project management software"
        );
    }

    #[test]
    fn the_companies_under_a_heading_are_the_ones_between_it_and_the_next() {
        let found = sections(G2, "g2.com");
        let labels: Vec<&str> = found.iter().map(|(h, _)| h.as_str()).collect();
        assert_eq!(
            labels,
            vec!["Best for creative agencies", "Best for software teams"],
            "the page's own title is the market, not a category"
        );
        let agencies: Vec<&str> = found[0].1.iter().map(|e| e.host.as_str()).collect();
        assert_eq!(agencies, vec!["workamajig.com", "asana.com"]);
        let software: Vec<&str> = found[1].1.iter().map(|e| e.host.as_str()).collect();
        assert_eq!(software, vec!["linear.app", "atlassian.com"]);
    }

    #[test]
    fn a_category_needs_two_publishers_and_somebody_under_it() {
        let found = categories(&read(&[("g2.com", G2), ("capterra.com", CAPTERRA)]));
        let labels: Vec<&str> = found.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["for creative agencies", "for software teams"],
            "and *methodology* is one publisher's section, not a market: {found:#?}"
        );
        assert_eq!(found[0].named_by, vec!["capterra.com", "g2.com"]);
        assert_eq!(found[0].companies, vec!["workamajig.com", "asana.com"]);
    }

    #[test]
    fn one_guide_dividing_a_market_is_one_editors_opinion() {
        let found = categories(&read(&[("g2.com", G2)]));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn two_guides_reviewing_the_same_vendors_have_not_divided_a_market() {
        // **Review found this stopping reports that would have been right.** Buyer's guides are
        // full of `## Asana` and `### Monday.com`, one section per vendor. Two guides doing that
        // is exact agreement on a "category" with a company under it - so an ordinary single
        // market was refused as several, and the **vendors themselves** were offered to a reader
        // as submarkets to pick between.
        //
        // The first fix asked whether a heading was the *name* of the company under it, which is
        // a negative signal and leaked twice; what settles it is that a vendor's review links to
        // that vendor and a category lists several.
        let by_vendor = concat!(
            "# Best project management software\n\n",
            "## Asana\n\n[Asana](https://asana.com/) is broad.\n\n",
            "## Monday.com\n\n[Monday](https://monday.com/) is colorful.\n\n",
            "### Workamajig\n\n[Workamajig](https://www.workamajig.com/) for agencies.\n",
        );
        let found = categories(&read(&[("g2.com", by_vendor), ("capterra.com", by_vendor)]));
        assert!(
            found.is_empty(),
            "a vendor's own review is not a way the market divides: {found:#?}"
        );
    }

    #[test]
    fn a_product_whose_name_is_nothing_like_its_domain_is_still_one_vendors_review() {
        // **Review's second round on the same blocker, and the reason the rule is positive
        // now.** `## Jira` links `atlassian.com`; `## Workfront` links `adobe.com`. Neither
        // heading resembles its corporate domain, so asking *is this heading the name of the
        // company under it* said no to both - and two guides doing that refused an ordinary
        // project-management report and offered **Jira** and **Workfront** as submarkets.
        //
        // What separates them is not their names: a vendor's review links to that vendor, and a
        // category lists several.
        let by_product = concat!(
            "# Best project management software\n\n",
            "## Jira\n\n[Jira](https://www.atlassian.com/software/jira)\n\n",
            "## Workfront\n\n[Workfront](https://business.adobe.com/products/workfront)\n",
        );
        let found = categories(&read(&[
            ("g2.com", by_product),
            ("capterra.com", by_product),
        ]));
        assert!(
            found.is_empty(),
            "one company under a heading is that company: {found:#?}"
        );
    }

    #[test]
    fn a_qualifier_with_one_company_under_it_is_still_thin() {
        // **The company bar is reachable and was untested**, which the catalog said before a
        // reviewer had to: with the heading rule in place, nothing exercised
        // `COMPANIES_PER_CATEGORY` any more. It is not dead - a heading two guides use, phrased
        // as a kind of buyer, with one company under it, clears every other bar. One company is
        // thin evidence for a division, and the safe direction here is not to fire.
        let one_each = concat!(
            "# Best project management software\n\n",
            "## Best for creative agencies\n\n[Workamajig](https://www.workamajig.com/)\n\n",
            "## Best for software teams\n\n[Linear](https://linear.app/)\n",
        );
        let found = categories(&read(&[("g2.com", one_each), ("capterra.com", one_each)]));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn an_award_is_a_place_in_a_ranking_rather_than_a_market() {
        // **Review's third round on the same question, and the reason the rule is positive at
        // last.** Two guides both running `## Best Overall` and `## Best Value` with two winners
        // apiece cleared every bar this module had: two publishers, two companies, no heading
        // that is the name of a company under it. So an ordinary project-management report was
        // refused and *overall* and *value* were offered to a reader as markets.
        //
        // They are selection awards. They rank the same market rather than dividing it.
        let a = concat!(
            "# Best project management software\n\n",
            "## Best Overall\n\n[Asana](https://asana.com/)\n\n",
            "## Best Value\n\n[ClickUp](https://clickup.com/)\n",
        );
        let b = concat!(
            "# Project management software\n\n",
            "## Best Overall\n\n[Monday](https://monday.com/)\n\n",
            "## Best Value\n\n[Wrike](https://www.wrike.com/)\n",
        );
        let found = categories(&read(&[("g2.com", a), ("capterra.com", b)]));
        assert!(
            found.is_empty(),
            "an award ranks a market rather than dividing it: {found:#?}"
        );
    }

    #[test]
    fn what_a_category_is_rather_than_what_it_is_not() {
        // The rule stated directly, so the next person changing it can see its shape and its
        // cost: it says yes to a qualifier and no to everything else, including real headings.
        assert!(describes_a_kind_of_buyer("for creative agencies"));
        assert!(describes_a_kind_of_buyer("with a free tier"));
        assert!(!describes_a_kind_of_buyer("overall"));
        assert!(!describes_a_kind_of_buyer("value"));
        assert!(!describes_a_kind_of_buyer("jira"));
        assert!(
            !describes_a_kind_of_buyer("for"),
            "a preposition alone qualifies nothing"
        );
        // **The cost, asserted rather than admitted.** These are real category headings this
        // will not recognize, and missing one only means the gate does not fire.
        assert!(!describes_a_kind_of_buyer("enterprise"));
        assert!(!describes_a_kind_of_buyer("agency project management"));
    }

    #[test]
    fn a_category_named_after_nobody_it_links_to_is_still_a_category() {
        // The other half, so the rule above is a rule rather than a way of never finding
        // anything: these headings name a kind of buyer, and the companies under them are not
        // what they are called.
        let found = categories(&read(&[("g2.com", G2), ("capterra.com", CAPTERRA)]));
        assert_eq!(found.len(), 2, "{found:#?}");
    }

    #[test]
    fn a_heading_with_nobody_under_it_is_a_section_of_prose() {
        let empty = "# A market\n\n## For agencies\n\nWe could not find any.\n";
        let found = categories(&read(&[("g2.com", empty), ("capterra.com", empty)]));
        assert!(found.is_empty(), "{found:#?}");
    }

    fn owned(of: &[&str]) -> Vec<String> {
        of.iter().map(|s| (*s).to_owned()).collect()
    }

    fn category(label: &str, companies: &[&str]) -> Category {
        Category {
            label: label.to_owned(),
            named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
            companies: owned(companies),
        }
    }

    #[test]
    fn a_question_whose_answers_sit_under_one_heading_was_answered() {
        // **The condition doing the work.** Any broad market has subheadings; what makes a
        // question too general is the companies **in the answer** being spread across them.
        let concentrated = vec![
            category(
                "for creative agencies",
                &["a.example", "b.example", "c.example"],
            ),
            category("for software teams", &["d.example"]),
        ];
        assert!(
            !too_general(
                &concentrated,
                &owned(&["a.example", "b.example", "c.example", "d.example"])
            ),
            "three of four under one heading is an answer, not a question"
        );

        let spread = vec![
            category("for creative agencies", &["a.example", "b.example"]),
            category("for software teams", &["c.example", "d.example"]),
        ];
        assert!(
            too_general(
                &spread,
                &owned(&["a.example", "b.example", "c.example", "d.example"])
            ),
            "half and half is two markets"
        );
    }

    #[test]
    fn the_share_is_of_the_companies_in_the_answer_and_not_of_everything_linked() {
        // **Review found two populations either side of the fraction.** A category's companies
        // are everything the guides linked, including candidates set aside or never fetched;
        // the denominator was the admitted set. Four linked against five admitted gave `4 / 5`
        // twice, and a question that really was two markets read as concentrated in both.
        let each_links_four = vec![
            category(
                "for creative agencies",
                &["a.example", "b.example", "gone.example", "unread.example"],
            ),
            category(
                "for software teams",
                &["c.example", "d.example", "e.example", "budget.example"],
            ),
        ];
        // Two of the first category's four made it in, and three of the second's.
        let in_the_answer = owned(&[
            "a.example",
            "b.example",
            "c.example",
            "d.example",
            "e.example",
        ]);
        assert!(
            too_general(&each_links_four, &in_the_answer),
            "2/5 and 3/5 is spread; only counting what the guides linked made it 4/5 twice"
        );
    }

    #[test]
    fn a_heading_holding_nobody_in_the_answer_divides_nothing_a_reader_can_see() {
        let one_real = vec![
            category("for creative agencies", &["a.example", "b.example"]),
            category("for construction", &["gone.example", "unread.example"]),
        ];
        assert!(
            !too_general(&one_real, &owned(&["a.example", "b.example"])),
            "the second category holds nobody who will be shown"
        );
    }

    #[test]
    fn one_vendor_with_two_products_is_one_company_on_both_sides() {
        // `admitted` is distinct domains, so a vendor whose two products both made it in cannot
        // make its category look twice as full as it is.
        let set = crate::competitors::Set {
            members: vec![
                member("microsoft.com"),
                member("microsoft.com"),
                member("asana.com"),
            ],
            set_aside: Vec::new(),
            alone: None,
        };
        assert_eq!(admitted(&set), owned(&["microsoft.com", "asana.com"]));
    }

    #[test]
    fn one_category_is_never_a_question() {
        let only = vec![category("for creative agencies", &["a.example"])];
        let four = owned(&["a.example", "b.example", "c.example", "d.example"]);
        assert!(
            !too_general(&only, &four),
            "there is nothing to choose between"
        );
        assert!(!too_general(&[], &four));
        // And a run that found nobody has no denominator, so there is no share to compare.
        assert!(!too_general(&only, &[]));
    }

    fn member(domain: &str) -> crate::competitors::Member {
        crate::competitors::Member {
            candidate: landscape_core::subject::Candidate {
                name: domain.to_owned(),
                canonical_domain: domain.to_owned(),
                what_it_is: "a company".to_owned(),
                confidence: 0.9,
            },
            because: crate::competitors::Because::Named,
        }
    }
}
