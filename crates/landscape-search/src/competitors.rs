//! One description, several companies — and the reason each one is in the report.
//!
//! [`crate::candidates`] answers *"which company is this description about?"* and hands the
//! answer to the gate in [`landscape_core::subject`]. That is the right question when somebody
//! types a **name**. It is the wrong question when somebody types a **market**, and a market is
//! what this product is for.
//!
//! # What the gate says about a market, and why it is not wrong
//!
//! Three companies that all three differently-worded searches returned score identically —
//! `0.8 × 3/3 + 0.2 = 1.0` each. [`landscape_core::subject::resolve`] compares them against
//! [`landscape_core::subject::AMBIGUITY_MARGIN`], finds them tied, and asks the reader to pick
//! one. **That is the correct behaviour for the question it was asked.** Three products sharing
//! a name is exactly the situation `PRODUCT_SPEC.md` §3 wants a chip for, and one chip click
//! really does prevent an entire wrong report.
//!
//! It is also, for a market description, the most common answer — and answering *"privacy-
//! friendly website analytics"* with *"which one did you mean?"* is answering a question about a
//! landscape with a question about a company.
//!
//! # Telling the two apart with arithmetic
//!
//! | The reader typed | Content words | What the tie means |
//! |---|---|---|
//! | `Notion` | 1 | Several products share a name. Ask. |
//! | `privacy-friendly website analytics` | 4 | Several companies share a market. Compare them. |
//!
//! [`DESCRIBES_A_MARKET`] is that rule and it is deliberately crude: one word is a name, two or
//! more is a description of a kind of thing. It is checkable, it is explainable to a reader, and
//! it does not need a model. **What it cannot do** is tell *"Notion project management"* — a
//! market — from a two-word brand, and there is no attempt here to pretend otherwise; the
//! clarifying-question row of S2 is where that gets a chip rather than a heuristic.
//!
//! # Why each company is in the set
//!
//! Every member carries a [`Because`] built from countable things: how many of the searches
//! returned it, and which of the reader's own words its front page uses. Every company that was
//! found and *not* included carries an [`Aside`] saying which of four different things happened
//! to it — the same discipline [`landscape_core::coverage`] applies to an empty section, applied
//! to an absent company. A competitor missing from a comparison in silence is the defect this
//! module exists to remove.

use landscape_core::subject::{Candidate, Resolution, MINIMUM_CONFIDENCE};

use crate::candidates::{Described, Vocabulary, CORROBORATION, NAMED};

/// How many content words a description needs before it is about a *kind* of thing.
///
/// **Two.** One word is a name — `Notion`, `Linear`, `Front` — and three products sharing it is
/// the ambiguity the gate exists for. Two or more is somebody describing what they want, and
/// several companies matching *that* is the answer rather than the problem.
///
/// The number is a starting value and labelled as one, exactly as
/// [`landscape_core::subject::AMBIGUITY_MARGIN`] is. It is the input most likely to be wrong
/// here, and the thing that replaces it is a chip, not a bigger number.
pub const DESCRIBES_A_MARKET: usize = 2;

/// How many of the reader's own words a front page has to use for a company to join the set.
///
/// **One, and this is a floor rather than a filter.** The claim being made is narrow: a company
/// whose front page uses *none* of the words somebody typed is not obviously in the market they
/// described. It is not a claim that a page using one of them is a good competitor — `software`
/// and `platform` appear on nearly every front page there is, and this rule cannot see that.
///
/// It exists for one failure it can see: a search for a market returning a company from a
/// different one, which then gets compared against companies it has nothing to do with.
pub const SHARED_WORDS: usize = 1;

/// Words that carry grammar rather than meaning.
///
/// **Grammar only, deliberately.** The tempting additions are `software`, `platform`, `tool` and
/// `app` — and dropping those would turn `tax software` into a one-word input, which
/// [`DESCRIBES_A_MARKET`] would then read as a brand name and refuse to build a set for. A weak
/// content word is still a content word.
const NOT_CONTENT: [&str; 26] = [
    "the", "and", "for", "with", "that", "this", "from", "are", "was", "its", "our", "your",
    "their", "his", "her", "you", "who", "what", "how", "any", "all", "can", "has", "have", "not",
    "but",
];

/// A company the report will compare, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// Its name, domain and one-line self-description, from its own front page.
    pub candidate: Candidate,
    /// What put it in the set.
    pub because: Because,
}

/// What put a company in the set, in countable terms.
///
/// **Countable on purpose.** A reader asking *why is this company in my report* gets two numbers
/// and a list of their own words back, not a summary somebody would have to trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Because {
    /// How many of the differently-worded searches returned it.
    pub agreed: usize,
    /// How many searches were sent. The divisor, not the number that answered.
    pub asked: usize,
    /// The description's own words that its front page uses.
    pub shares: Vec<String>,
}

impl Because {
    /// The sentence a reader is shown.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "{} of the {} searches returned it, and its own front page uses {}",
            self.agreed,
            self.asked,
            quoted(&self.shares)
        )
    }
}

/// A company that was found and is not in the report, and which of five things happened.
///
/// **Five, and they are not interchangeable.** *"One search found it"*, *"we do not believe the
/// score"*, *"its page says nothing about what you asked for"*, *"we could not read its page"*
/// and *"we never asked for its page"* are five different facts, and the last two are about us.
/// Collapsing them into *"not included"* is the same defect [`landscape_core::coverage`] was
/// written to stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aside {
    /// Fewer than [`CORROBORATION`] of the searches returned it.
    Uncorroborated { agreed: usize, asked: usize },
    /// Corroborated, and still below [`landscape_core::subject::MINIMUM_CONFIDENCE`].
    Unconvincing,
    /// Its front page was read and uses none of the description's words.
    ///
    /// A statement about the page, and only reachable when the page was read.
    ElsewhereEntirely,
    /// Its front page was requested and could not be read, so nothing could be checked.
    ///
    /// **Not the same as [`Self::ElsewhereEntirely`].** One says the company is in another
    /// market; this one says we do not know, and a reader who names the domain themselves gets
    /// the report anyway.
    Unread,
    /// It ranked below the front pages we were willing to fetch.
    ///
    /// **Review found this one missing entirely.** The candidate list used to be truncated to
    /// five before anything here could see it, so a sixth corroborated company was neither
    /// compared nor reported as excluded — a competitor dropped in silence, which is the defect
    /// this module exists to remove. The budget is real and stays; what changed is that
    /// spending it is now something a reader is told about.
    BeyondTheFetchBudget { budget: usize },
}

impl Aside {
    /// The sentence a reader is shown.
    #[must_use]
    pub fn sentence(&self) -> String {
        match *self {
            Self::Uncorroborated { agreed, asked } => format!(
                "only {agreed} of the {asked} searches returned it, so nothing corroborates it"
            ),
            Self::Unconvincing => "it scored below the level worth putting in a report".to_owned(),
            Self::ElsewhereEntirely => {
                "its own front page uses none of the words you typed".to_owned()
            }
            Self::Unread => {
                "we could not read its front page, so we could not check what it does - name the \
                 domain yourself and we will read it"
                    .to_owned()
            }
            Self::BeyondTheFetchBudget { budget } => format!(
                "it ranked below the {budget} companies whose front pages we read, so we never \
                 asked for its page - name the domain yourself and we will read it"
            ),
        }
    }
}

/// The companies a description produced, both the ones in the report and the ones that are not.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Set {
    /// In the report, best first.
    pub members: Vec<Member>,
    /// Found and not in the report, each with the reason.
    pub set_aside: Vec<(Candidate, Aside)>,
}

impl Set {
    /// The origins an analysis will read, in the order the report will compare them.
    #[must_use]
    pub fn origins(&self) -> Vec<String> {
        self.members
            .iter()
            .map(|m| format!("https://{}", m.candidate.canonical_domain))
            .collect()
    }

    /// Whether anything survived to be compared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

/// A description in, the gate's verdict *and* the set out.
///
/// **Both, because they answer different questions and only one of them can stop a report.**
/// The verdict decides whether a report may be written at all — that is `FACT_CHECKING.md` §3.1
/// step 4 and this module does not weaken it. The set decides what goes *in* the report once it
/// may be. Returning one without the other is how a caller ends up re-deriving the missing half.
#[derive(Debug, Clone, PartialEq)]
pub struct Derived {
    /// The gate's answer, unchanged.
    pub verdict: Resolution,
    /// The companies the report will compare, when the verdict allows one.
    pub set: Set,
    /// Whether the reader described a kind of thing rather than naming one.
    ///
    /// Computed here because this is where the description's words are already split; a caller
    /// recomputing it from the raw prompt is the second source of truth this codebase keeps
    /// deleting.
    pub about_a_market: bool,
}

/// Split a description into the words that carry its meaning.
///
/// Lowercased, non-letters treated as separators, short words and grammar dropped, order kept
/// and repeats removed. `privacy-friendly website analytics` becomes
/// `["privacy", "friendly", "website", "analytics"]`.
#[must_use]
pub fn content_words(description: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in description
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
    {
        // Three, so `AI` and `HR` are dropped along with `to` and `of`. That loses two real
        // categories and is the honest cost of not keeping a second list; a reader who types
        // `HR software` still has `software`.
        if word.len() < 3 || NOT_CONTENT.contains(&word.as_str()) || out.contains(&word) {
            continue;
        }
        out.push(word);
    }
    out
}

/// Which of the description's words a page uses.
///
/// **Whole words, on the page's own token boundaries.** Review found what substring matching
/// did: `content_words("art marketplace")` yields `art`, a front page reading *"Tools for
/// startups"* contains those three letters inside `startups`, and the company was admitted to
/// the report with the sentence *"its own front page uses \"art\""* — evidence for a claim,
/// manufactured out of a coincidence of spelling.
///
/// The substring version was justified as finding `analytics` inside `web analytics` and
/// `Analytics.`, and **it never needed to be**: splitting on non-alphanumerics finds both, plus
/// `analytics/` and `(analytics)`. What is genuinely lost is plurals — a page saying `tool` does
/// not match `tools` — and being strict is the safe direction for evidence a reader is shown.
#[must_use]
pub fn shared(words: &[String], page: &str) -> Vec<String> {
    let on_the_page: Vec<String> = page
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    words
        .iter()
        .filter(|w| on_the_page.iter().any(|t| t == *w))
        .cloned()
        .collect()
}

/// Whether a reader described a kind of thing rather than naming one.
///
/// Takes the words rather than the description so the rule has **one** home: a caller that has
/// already split a description would otherwise compare a length against
/// [`DESCRIBES_A_MARKET`] itself, and the two would drift the day the splitting changed.
#[must_use]
pub fn about_a_market(words: &[String]) -> bool {
    words.len() >= DESCRIBES_A_MARKET
}

/// Named candidates in, the set and its exclusions out.
///
/// Pure: no clock, no network, no model. Every branch is reachable from a unit test, which
/// matters because this is the code that decides which companies a reader is shown.
///
/// `asked` is how many searches were **sent**, for the same reason [`crate::candidates::score`]
/// divides by it: a company returned by the one search that came back has agreed with nothing.
#[must_use]
pub fn assemble(described: Vec<Described>, asked: usize) -> Set {
    let mut set = Set::default();
    for one in described {
        let Described {
            candidate,
            agreed,
            shares,
        } = one;
        // Ordered from the cheapest fact to the most specific, so a company that fails two of
        // these is reported by the first one — the reader is told the strongest reason it is
        // absent rather than the last one that happened to be checked.
        let aside = if agreed < CORROBORATION {
            Some(Aside::Uncorroborated { agreed, asked })
        } else if candidate.score() < MINIMUM_CONFIDENCE {
            Some(Aside::Unconvincing)
        } else {
            match shares {
                Vocabulary::NotRequested => Some(Aside::BeyondTheFetchBudget { budget: NAMED }),
                Vocabulary::Unreadable => Some(Aside::Unread),
                Vocabulary::Read(ref s) if s.len() < SHARED_WORDS => Some(Aside::ElsewhereEntirely),
                Vocabulary::Read(_) => None,
            }
        };
        match (aside, shares) {
            (Some(why), _) => set.set_aside.push((candidate, why)),
            (None, Vocabulary::Read(shares)) => set.members.push(Member {
                candidate,
                because: Because {
                    agreed,
                    asked,
                    shares,
                },
            }),
            // Unreachable: every state but `Read` produces an `Aside` above. Written as a
            // set-aside rather than an `unreachable!`, because a panic here takes a worker down
            // over a company nobody had to include.
            (None, _) => set.set_aside.push((candidate, Aside::Unread)),
        }
    }
    // Best first, then by domain so a tie is stable between runs rather than however the
    // candidate list happened to arrive.
    set.members.sort_by(|a, b| {
        b.candidate
            .score()
            .total_cmp(&a.candidate.score())
            .then_with(|| {
                a.candidate
                    .canonical_domain
                    .cmp(&b.candidate.canonical_domain)
            })
    });
    set
}

/// Wrap each word in quotes for a sentence a reader reads, or say there were none.
fn quoted(words: &[String]) -> String {
    if words.is_empty() {
        return "none of your words".to_owned();
    }
    words
        .iter()
        .map(|w| format!("\"{w}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn described(domain: &str, agreed: usize, confidence: f32, shares: Vocabulary) -> Described {
        Described {
            candidate: Candidate {
                name: domain.to_owned(),
                canonical_domain: domain.to_owned(),
                what_it_is: "a company".to_owned(),
                confidence,
            },
            agreed,
            shares,
        }
    }

    fn read(words: &[&str]) -> Vocabulary {
        Vocabulary::Read(words.iter().map(|w| (*w).to_owned()).collect())
    }

    #[test]
    fn a_description_splits_into_the_words_that_carry_it() {
        assert_eq!(
            content_words("privacy-friendly website analytics"),
            vec!["privacy", "friendly", "website", "analytics"]
        );
    }

    #[test]
    fn grammar_and_repeats_are_dropped_and_the_order_is_kept() {
        assert_eq!(
            content_words("The best CRM for the small business CRM"),
            vec!["best", "crm", "small", "business"]
        );
    }

    #[test]
    fn a_weak_word_is_still_a_word() {
        // **The rule that stops the stop list growing.** Dropping `software` would leave one
        // content word, which `about_a_market` reads as a brand name - so a real market
        // description would be answered with "which one did you mean?".
        assert_eq!(content_words("tax software"), vec!["tax", "software"]);
        assert!(about_a_market(&content_words("tax software")));
    }

    #[test]
    fn one_word_is_a_name_and_two_is_a_market() {
        assert!(!about_a_market(&content_words("Notion")));
        assert!(!about_a_market(&content_words("  Linear  ")));
        assert!(about_a_market(&content_words("Notion project management")));
        assert!(about_a_market(&content_words("privacy-friendly analytics")));
    }

    #[test]
    fn a_page_shares_the_words_it_actually_uses() {
        let words = content_words("privacy-friendly website analytics");
        assert_eq!(
            shared(
                &words,
                "# Plausible\n\nSimple, privacy-first web analytics."
            ),
            vec!["privacy", "analytics"]
        );
    }

    #[test]
    fn a_word_inside_another_word_is_not_a_word_the_page_uses() {
        // **Review found the evidence being manufactured.** `shared` matched substrings, so a
        // front page reading *"Tools for startups"* was reported as using the word `art` -
        // enough on its own to admit the company, with `Because::sentence` telling the reader
        // a reason that was never true of that page.
        let words = content_words("art marketplace");
        assert!(
            shared(&words, "# Toolbox\n\nTools for startups.").is_empty(),
            "a coincidence of spelling was reported as evidence"
        );
        assert_eq!(
            shared(&words, "An art marketplace."),
            vec!["art", "marketplace"]
        );
    }

    #[test]
    fn punctuation_and_case_do_not_hide_a_word_the_page_really_uses() {
        // The other half, and the reason substring matching looked justified: these were the
        // cases it was written for, and splitting on non-alphanumerics finds every one.
        let words = content_words("analytics");
        for page in [
            "Web Analytics.",
            "# ANALYTICS",
            "(analytics)",
            "privacy/analytics/web",
            "**analytics**",
        ] {
            assert_eq!(shared(&words, page), vec!["analytics"], "{page}");
        }
        // And the honest cost of being strict: a plural is a different token.
        assert!(shared(&["tool".to_owned()], "Tools for teams.").is_empty());
    }

    #[test]
    fn a_page_that_shares_nothing_shares_nothing() {
        let words = content_words("privacy-friendly website analytics");
        assert!(shared(&words, "# Notion Press\n\nSelf-publish your book.").is_empty());
    }

    #[test]
    fn the_set_is_the_companies_whose_pages_match_and_the_rest_say_why() {
        let set = assemble(
            vec![
                described("plausible.io", 3, 1.0, read(&["privacy", "analytics"])),
                described("onesearch.example", 1, 0.175, read(&["analytics"])),
                described("notionpress.example", 3, 1.0, read(&[])),
                described("unreachable.example", 3, 1.0, Vocabulary::Unreadable),
                described("weak.example", 2, 0.30, read(&["analytics"])),
            ],
            3,
        );

        assert_eq!(set.members.len(), 1, "{set:#?}");
        assert_eq!(set.members[0].candidate.canonical_domain, "plausible.io");
        assert_eq!(set.origins(), vec!["https://plausible.io"]);

        let reasons: Vec<(&str, &Aside)> = set
            .set_aside
            .iter()
            .map(|(c, why)| (c.canonical_domain.as_str(), why))
            .collect();
        assert_eq!(
            reasons,
            vec![
                (
                    "onesearch.example",
                    &Aside::Uncorroborated {
                        agreed: 1,
                        asked: 3
                    }
                ),
                ("notionpress.example", &Aside::ElsewhereEntirely),
                ("unreachable.example", &Aside::Unread),
                ("weak.example", &Aside::Unconvincing),
            ]
        );
    }

    #[test]
    fn a_company_below_the_fetch_budget_is_reported_rather_than_dropped() {
        // **Review found it dropped.** The candidate list was truncated to five before this
        // function saw it, so a sixth corroborated company was neither compared nor named as
        // excluded - which is the whole guarantee this module makes.
        let set = assemble(
            vec![described("sixth.example", 3, 1.0, Vocabulary::NotRequested)],
            3,
        );
        assert!(set.members.is_empty());
        assert_eq!(
            set.set_aside[0].1,
            Aside::BeyondTheFetchBudget { budget: NAMED }
        );
        // Not `Unread`: nobody asked for the page, so nothing about it failed.
        assert_ne!(set.set_aside[0].1, Aside::Unread);
        assert!(set.set_aside[0].1.sentence().contains("never"));
    }

    #[test]
    fn a_page_we_could_not_read_is_not_reported_as_a_page_about_something_else() {
        // Two silences that look identical from the outside: nothing shared because the page
        // is about another market, and nothing shared because there was no page. Only the
        // first is a statement about the company.
        let set = assemble(
            vec![described(
                "unreachable.example",
                3,
                1.0,
                Vocabulary::Unreadable,
            )],
            3,
        );
        assert_eq!(set.set_aside.len(), 1);
        assert_eq!(set.set_aside[0].1, Aside::Unread);
        assert_ne!(set.set_aside[0].1, Aside::ElsewhereEntirely);
        assert!(set.set_aside[0].1.sentence().contains("name the domain"));
    }

    #[test]
    fn the_strongest_reason_is_the_one_a_reader_is_given() {
        // One search found it *and* its page is about something else. "Nothing corroborates it"
        // is the fact that decided; reporting the vocabulary would suggest the search agreed.
        let set = assemble(vec![described("thin.example", 1, 0.175, read(&[]))], 3);
        assert_eq!(
            set.set_aside[0].1,
            Aside::Uncorroborated {
                agreed: 1,
                asked: 3
            }
        );
    }

    #[test]
    fn the_set_is_ordered_best_first_and_ties_are_stable() {
        let set = assemble(
            vec![
                described("zeta.example", 3, 0.8, read(&["analytics"])),
                described("alpha.example", 3, 0.8, read(&["analytics"])),
                described("best.example", 3, 1.0, read(&["analytics"])),
            ],
            3,
        );
        let order: Vec<&str> = set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(order, vec!["best.example", "alpha.example", "zeta.example"]);
    }

    #[test]
    fn a_reason_names_the_numbers_and_the_readers_own_words() {
        let because = Because {
            agreed: 3,
            asked: 3,
            shares: vec!["privacy".to_owned(), "analytics".to_owned()],
        };
        assert_eq!(
            because.sentence(),
            "3 of the 3 searches returned it, and its own front page uses \"privacy\", \"analytics\""
        );
    }

    #[test]
    fn the_divisor_in_a_reason_is_what_was_sent() {
        // The same rule the score follows. A company found by the one search that came back
        // must not read as "1 of the 1 searches returned it".
        let set = assemble(
            vec![described("one.example", 2, 0.53, read(&["analytics"]))],
            3,
        );
        assert!(set.members[0].because.sentence().contains("2 of the 3"));
    }
}
