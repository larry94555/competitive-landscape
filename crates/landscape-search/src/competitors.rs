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
//! one. **That is the correct behavior for the question it was asked.** Three products sharing
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

use crate::provider::SourceProvider;
use crate::queries::Query;

use crate::candidates::{Described, NoVocabulary, Queried, Seed, Vocabulary, CORROBORATION, NAMED};

/// How many content words a description needs before it is about a *kind* of thing.
///
/// **Two.** One word is a name — `Notion`, `Linear`, `Front` — and three products sharing it is
/// the ambiguity the gate exists for. Two or more is somebody describing what they want, and
/// several companies matching *that* is the answer rather than the problem.
///
/// The number is a starting value and labeled as one, exactly as
/// [`landscape_core::subject::AMBIGUITY_MARGIN`] is. It is the input most likely to be wrong
/// here, and the thing that replaces it is a chip, not a bigger number.
pub const DESCRIBES_A_MARKET: usize = 2;

/// The fewest of the market's words a front page may use and still be in the set.
///
/// **One, and it is the floor rather than the whole test.** A two-word market cannot ask for
/// more than one word without asking for all of it, and demanding all of a short description is
/// how a real competitor gets dropped for terse front matter. See [`enough_words`] for the part
/// that scales.
pub const SHARED_WORDS_FLOOR: usize = 1;

/// How many of the market's words a front page has to use for a company to join the set.
///
/// **Half of them, rounded down, and never fewer than [`SHARED_WORDS_FLOOR`].** The more a
/// reader said about the market, the more of it a page has to actually be about.
///
/// ```text
/// spreadsheet finance                        2 words  ->  1
/// project management design agency           4 words  ->  2
/// time tracking independent consultants      4 words  ->  2
/// ```
///
/// **It was a flat `1`, and a board game passed it.** *"project management for a small design
/// agency"* returned `projectplusgame.com`, whose page says *"a board game about project
/// deadlines"*: one shared word, one word required. The machinery was right and the number was
/// its weakest possible value.
///
/// # Why this shape, and what it cost
///
/// **Four rules were written and scored against the discovery golden set** rather than one being
/// picked — `landscape_golden::discovery::Fit`, and `BENCHMARKS.md` Run 53 has the table. A flat
/// two loses two real competitors whose front pages are terse; half the market's words *rounded
/// up* loses one. Rounded down, no fixture loses anybody and the impostor goes.
///
/// **Five fixtures is a small set, and this number is fitted to them.** It is a starting value
/// and labeled as one, exactly as [`DESCRIBES_A_MARKET`] is. What replaces it is a bigger set,
/// or a signal that is not word counting at all.
///
/// **This is the rule for a market somebody described.** [`Evidence`] is what decides when it
/// applies, and the answer is not *always*.
#[must_use]
pub fn enough_words(market: usize) -> usize {
    (market / 2).max(SHARED_WORDS_FLOOR)
}

/// Where the words a candidate is judged against came from.
///
/// **Two sources, and only one of them is a description.** Review found the scaling rule applied
/// to both: on the named path the words are [`crate::candidates::Seed::words`], which is
/// `content_words` over a sentence lifted from the seed company's own front page. Scaling a bar
/// with the length of *that* means **the wordier a company's marketing copy, the more of it
/// every rival has to repeat verbatim** — a seed page reading *"project management
/// collaboration tasks teams planning timelines communication"* would have asked four exact
/// words of everybody, and excluded Linear for saying *"project management built for speed"*.
///
/// The two are different claims. *A reader said four words about a market, so a page in it uses
/// two of them* is an argument. *A stranger's home page ran to fifteen words, so a rival must
/// repeat seven* is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// The market a reader described, in their words or in the market's own.
    ADescribedMarket,
    /// A seed company's front page, on the path where a reader named one company.
    ASeedsOwnPage,
}

impl Evidence {
    /// How many of `words` a front page has to use.
    #[must_use]
    pub fn enough_words(self, words: usize) -> usize {
        match self {
            Self::ADescribedMarket => enough_words(words),
            // **The floor, and nothing above it.** The claim available here is the narrow one it
            // always was: a company with nothing in common with the seed's own page is not
            // obviously in the same market. There is no reader's description to scale against.
            Self::ASeedsOwnPage => SHARED_WORDS_FLOOR,
        }
    }
}

/// Words that carry grammar rather than meaning.
///
/// **Grammar only, deliberately.** The tempting additions are `software`, `platform`, `tool` and
/// `app` — and dropping those would turn `tax software` into a one-word input, which
/// [`DESCRIBES_A_MARKET`] would then read as a brand name and refuse to build a set for. A weak
/// content word is still a content word.
pub(crate) const NOT_CONTENT: [&str; 26] = [
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

/// What put a company in the set.
///
/// **Two reasons, and they are not the same kind of fact.** A reader who typed a domain has
/// already answered the question; a company we searched for has to justify itself, and does so
/// in countable terms — a reader asking *why is this company in my report* gets two numbers and
/// a list of words back, not a summary somebody would have to trust.
///
/// It was a struct with three fields, which could only say the second thing. Then a named
/// company needed a place in the same list, and *"3 of the 3 searches returned it"* would have
/// been a sentence about a search nobody ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Because {
    /// The reader named it. There is nothing to justify.
    Named,
    /// We searched for it, and this is the evidence.
    Found {
        /// How many of the differently-worded searches returned it.
        agreed: usize,
        /// How many searches were sent. The divisor, not the number that answered.
        asked: usize,
        /// The publishers that list it in this market, by host.
        ///
        /// **Kept apart from [`Self::Found::agreed`] rather than added to it.** *"1 of the 3
        /// searches returned it"* and *"2 buyer's guides list it"* are two different facts, and
        /// a reader asking why a company is in their report is owed both rather than a sum.
        named_by: Vec<String>,
        /// How many publisher pages were read. The divisor for the fact above.
        guides: usize,
        /// The words its front page shares with what was asked about.
        shares: Vec<String>,
    },
}

impl Because {
    /// The sentence a reader is shown.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            Self::Named => "you named it".to_owned(),
            Self::Found {
                agreed,
                asked,
                named_by,
                guides,
                shares,
            } => {
                let searches = format!("{agreed} of the {asked} searches returned it");
                let listed = if named_by.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", {} of the {guides} buyer's guides we read list it ({})",
                        named_by.len(),
                        named_by.join(", ")
                    )
                };
                format!(
                    "{searches}{listed}, and its own front page uses {}",
                    quoted(shares)
                )
            }
        }
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
    /// Fewer than [`CORROBORATION`] independent things pointed at it.
    ///
    /// **Two kinds of pointing, and the sentence says both.** A company one search returned and
    /// no guide lists, and a company one search returned when no guide was read, are different
    /// findings: the first was looked for in the literature and not found, the second was not
    /// looked for. That is [`landscape_core::coverage`]'s rule again.
    Uncorroborated {
        agreed: usize,
        asked: usize,
        /// How many publishers named it, which is fewer than [`CORROBORATION`] here.
        named_by: usize,
        /// How many publisher pages were read at all.
        guides: usize,
    },
    /// Corroborated, and still below [`landscape_core::subject::MINIMUM_CONFIDENCE`].
    Unconvincing,
    /// Its front page was read and uses none of the words the comparison is built on.
    ///
    /// A statement about the page, and only reachable when the page was read.
    ///
    /// **It carries the words, because it used to guess at them.** The sentence said *"none of
    /// the words you typed"* — true when a description was searched for, false on the seeded
    /// path, where the words come from the seed company's own front page and the reader typed
    /// only a domain. A reader was given a false account of the evidence used to exclude
    /// somebody. Naming them is true on both paths, and lets a reader judge the exclusion
    /// instead of taking it.
    ElsewhereEntirely {
        looked_for: Vec<String>,
        /// The ones it did use, which may be some of them rather than none.
        ///
        /// **Added when the bar moved off one.** While one word was the whole test, *excluded*
        /// and *uses none of them* were the same fact and the sentence could say either. They
        /// are now different: a page can use one of four and still be set aside, and a reader
        /// told it used *none* would be told something untrue about a page they can open.
        used: Vec<String>,
        /// How many it needed, from [`enough_words`].
        needed: usize,
    },
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
            Self::Uncorroborated {
                agreed,
                asked,
                named_by,
                guides,
            } => {
                let searches = format!("only {agreed} of the {asked} searches returned it");
                let literature = match (guides, named_by) {
                    (0, _) => ", and we read no buyer's guide that might have".to_owned(),
                    (g, 0) => format!(", and none of the {g} buyer's guides we read list it"),
                    (g, n) => format!(", and only {n} of the {g} buyer's guides we read list it"),
                };
                format!("{searches}{literature}, so nothing corroborates it")
            }
            Self::Unconvincing => "it scored below the level worth putting in a report".to_owned(),
            Self::ElsewhereEntirely {
                ref looked_for,
                ref used,
                ..
            } if used.is_empty() => format!(
                "its own front page uses none of the words this comparison is built on: {}",
                quoted(looked_for)
            ),
            Self::ElsewhereEntirely {
                ref looked_for,
                ref used,
                needed,
            } => {
                let (used, all, of) = (quoted(used), looked_for.len(), quoted(looked_for));
                let uses = format!("its own front page uses {used} of the {all} words");
                format!("{uses} this comparison is built on ({of}), and a company in this market uses at least {needed}")
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
    /// Why nobody else is here, when the report covers one company.
    ///
    /// `None` when there is a comparison to read. `Some` is the honest empty at the level of the
    /// **whole set** — the thing [`Aside`] does for one company, done for the absence of all of
    /// them.
    pub alone: Option<NoRivals>,
}

/// Why a report covers one company and no others.
///
/// **Four, and a reader acts on each of them differently.** *"Nothing is configured"*, *"we
/// could not read your company's page"*, *"the searching did not finish"* and *"we looked and
/// nobody held up"* are four different facts; only the last is about the market, and only the
/// third is fixed by waiting.
///
/// Collapsing them was refused three times while the rest of this row was built — see
/// `BENCHMARKS.md` Runs 29 to 31 — on the grounds that one sentence covering all of them is
/// worse than no sentence, because a reader acts on it and gets the same silence back. This is
/// that refusal paid off rather than deferred again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoRivals {
    /// No search engine is configured, so nothing off this company's own site can be reached.
    NoEngine,
    /// The company's own page gave nothing to judge a competitor against; see [`NoVocabulary`].
    ///
    /// **Nothing was asked at all in this case**, which is why it is a separate fact from
    /// [`Self::NobodyHeldUp`]: no engine was consulted, so nothing is being claimed about who
    /// is out there.
    NothingToCompare(NoVocabulary),
    /// Some of the searches did not come back. About us, and **not always retryable**.
    ///
    /// **The retryable part was an assumption, and it was wrong for the most likely case.**
    /// This said *"that is usually temporary - try again"* whatever had happened, including
    /// when the engine answered every query with a refusal — which is what an unconfigured
    /// SearXNG does, permanently, and is the reason `deploy/searxng/settings.yml` is checked
    /// in at all. A reader was being told to wait for something that would never change.
    SearchIncomplete {
        failed: usize,
        sent: usize,
        sought: Sought,
        /// What a reader would do about it. See [`crate::provider::Fault`].
        fault: crate::provider::Fault,
    },
    /// Every search ran, and nothing else that came back held up.
    ///
    /// The only one of the four that is a statement about the world rather than about us.
    /// Whatever was found and rejected is in [`Set::set_aside`], named.
    NobodyHeldUp { sought: Sought },
}

/// What the searches behind a set were looking for.
///
/// **The same silence needs different words on the two paths.** A description path searched for
/// companies matching what somebody typed; a seeded path searched for the rivals of a company
/// they named. Review found the seeded wording on a description's report:
///
/// ```text
/// You described a market ... This report is about Plausible (plausible.io).
/// We searched for companies it competes with and none of what came back held up.
/// Why each one is here: Plausible - 3 of the 3 searches returned it, ...
/// ```
///
/// Two adjacent notes, one saying nothing held up and the next naming the company that did —
/// and neither search was for Plausible's competitors. It is carried on the variant rather than
/// passed to `sentence()` so a caller cannot supply the wrong one: the two paths each state it
/// once, where they know it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sought {
    /// Rivals of the company the reader named.
    RivalsOfTheCompany,
    /// Companies matching the description the reader typed.
    CompaniesMatchingTheDescription,
}

impl Sought {
    /// What a sentence calls the thing that was searched for.
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::RivalsOfTheCompany => "companies it competes with",
            Self::CompaniesMatchingTheDescription => "companies matching that description",
        }
    }
}

impl NoRivals {
    /// Why a run with **no engine configured** found nobody — which is not always the engine.
    ///
    /// **Review found the report recommending a remedy that cannot work.** With the seed's own
    /// page unreadable, `seed.words()` already fails; configuring `SEARX_URL` would change
    /// nothing, because [`of_company`] would then send zero queries and say so. The report told
    /// a reader to install something while `landscape candidates` told them it would not help —
    /// two surfaces, two answers, and the wrong one on the surface that matters.
    ///
    /// No vocabulary takes precedence, because it is the fact that makes the engine irrelevant.
    #[must_use]
    pub fn when_no_engine_is_configured(seed: &Seed) -> Self {
        match seed.words() {
            Err(why) => Self::NothingToCompare(why),
            Ok(_) => Self::NoEngine,
        }
    }

    /// The sentence a reader is shown after the company this report is about.
    #[must_use]
    pub fn sentence(&self) -> String {
        match *self {
            Self::NoEngine => "We did not look for companies it competes with: no search engine \
                 is configured here, so nothing off this company's own site can be reached."
                .to_owned(),
            Self::NothingToCompare(why) => format!(
                "We did not look for companies it competes with: {}, so there was nothing to \
                 judge one against. This says nothing about who else is out there.",
                why.sentence()
            ),
            // **The count is the same fact on all three; the last sentence is not.** What
            // changed here is only what a reader should do next, which is the part they can
            // act on — and the words never name a status code, a variable or a file, because
            // the person reading a report cannot fix any of them.
            Self::SearchIncomplete {
                failed,
                sent,
                sought,
                fault,
            } => format!(
                "We could not complete {failed} of the {sent} searches for {}, so this report \
                 is about one company. {}",
                sought.phrase(),
                fault.advice()
            ),
            // **Not "none of what came back held up" on the description path**, where the one
            // company in the report is exactly what came back and did hold up.
            Self::NobodyHeldUp {
                sought: Sought::RivalsOfTheCompany,
            } => "We searched for companies it competes with and none of what came back held up."
                .to_owned(),
            Self::NobodyHeldUp {
                sought: Sought::CompaniesMatchingTheDescription,
            } => "Only one company matched that description well enough to report on.".to_owned(),
        }
    }
}

/// Why a set has nobody in it beside the first company — when that is true at all.
///
/// **One function, because both paths need the same answer and would otherwise each guess.** A
/// description that resolved to a single company and a named company nobody could be found for
/// are the same shape from a reader's side: one company in a tool that promises a comparison.
///
/// `nothing_asked` is for the cases decided before any query goes out — no engine, or a seed
/// page with no vocabulary — because [`Queried`] cannot tell *"we sent none"* from *"there were
/// none to send"*.
#[must_use]
pub fn alone_because(
    members: usize,
    queried: &Queried,
    sought: Sought,
    nothing_asked: Option<NoRivals>,
) -> Option<NoRivals> {
    // More than one company is a comparison, whatever else went wrong on the way to it.
    if members > 1 {
        return None;
    }
    if let Some(why) = nothing_asked {
        return Some(why);
    }
    // `fault()` is `None` only when nothing failed, which the line above has ruled out — so
    // the fallback is unreachable rather than a default being chosen. `Silent` is the one that
    // says *try again*, which is the answer that assumes least about an engine nobody heard
    // from at all.
    if let Some(fault) = queried.fault() {
        return Some(NoRivals::SearchIncomplete {
            failed: queried.failed.len(),
            sent: queried.sent(),
            sought,
            fault,
        });
    }
    Some(NoRivals::NobodyHeldUp { sought })
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
    /// How the market's own guides divide it, when two of them agree.
    ///
    /// **Carried rather than recomputed**, for the reason `about_a_market` is: the pages were
    /// read here, and a caller asking for them again would be a second fetch of somebody's
    /// server for text this run is still holding. Empty when nothing was read, when the guides
    /// have no headings two of them share, or when the reader named their companies.
    pub categories: Vec<crate::breadth::Category>,
}

/// The version of the rival query set, stamped on a run.
///
/// Its own version, beside [`crate::candidates::IDEA_QUERY_SET`], for the reason that one is
/// separate from [`crate::QUERY_SET`]: these ask *who else is in this company's market*, and a
/// single number covering all three would move for edits that could not affect the others.
pub const RIVAL_QUERY_SET: &str = "2026-08-08.1";

/// The queries a named company produces.
///
/// **This is the case `FACT_CHECKING.md` P22 was written for, rather than the exception to it.**
/// P22 requires *"templated queries from resolved entities, not user phrasing"*, and
/// [`crate::candidates::for_idea`] documents at length why it cannot comply — there is no
/// resolved entity when all anybody has typed is a description. Here there is one: the reader
/// named a domain, and the company's own name comes off its own front page. Nothing a reader
/// phrased reaches the engine.
///
/// Three, differently worded, because the score below is agreement between them and one query
/// agrees with nothing. They deliberately ask the question three ways a *buyer* would, since
/// that is the vocabulary the pages worth finding are written in.
#[must_use]
pub fn for_company(name: &str) -> Vec<Query> {
    /// Interpolated with a resolved company's own name for itself.
    const TEMPLATES: [&str; 3] = [
        r#"{} alternatives"#,
        r#"{} competitors"#,
        r#"companies like {}"#,
    ];

    let cleaned = crate::queries::safe_words(name);
    if cleaned.is_empty() {
        return Vec::new();
    }
    TEMPLATES
        .into_iter()
        .map(|template| Query {
            text: template.replacen("{}", &cleaned, 1),
            answers: landscape_discover::probes::Answers::Identity,
            template,
        })
        .collect()
}

/// The companies a named one competes with.
///
/// **The seed is dropped from its own results**, and it is the highest-scoring host in every one
/// of them: a search for *"basecamp alternatives"* returns Basecamp. Leaving it in would put the
/// company in the set twice, once as *"you named it"* and once as *"3 of the 3 searches returned
/// it"* — two reasons for one company, one of which is circular.
///
/// The words a rival's front page has to share come from the **seed's own front page**, not from
/// anything a reader typed — so the bar is [`SHARED_WORDS_FLOOR`] and does not scale. See
/// [`Evidence`] for why: the length of a stranger's marketing sentence is not a measure of how
/// specific a reader was. It does the same narrow job it always did: a company whose page has
/// nothing in common with the seed's is not obviously in the same market.
///
/// # Errors
/// Never. A query that fails is counted and the rest carry on; see [`crate::candidates::suggest`].
pub async fn of_company<F, Fut>(
    engine: &dyn SourceProvider,
    seed: &Seed,
    fetch: F,
) -> (Set, Queried)
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    // **No vocabulary, no rivals, and no queries either** — decided here, before anything is
    // asked of anybody, because that is the only place it can be true. A rival is admitted by
    // sharing a word with the seed's own description of itself, and there are two ways to have
    // none: a front page nobody could read, and one that was read and said nothing quotable.
    //
    // Review found both. The first version mined the unreadable-page fallback for words —
    // `unable`, `read`, `front`, `page` — so a rival saying *"read more on this page"* joined
    // the report citing our error message as its evidence. The second gated on `seed.read`
    // alone, so a page reading only `# Basecamp` sent all three queries and then set every
    // corroborated rival aside as *elsewhere entirely*: a negative claim about those companies
    // built out of missing evidence about this one.
    //
    // Stopping rather than searching-and-excluding is deliberate: three queries and five
    // front-page fetches, on other people's servers, to conclude that nothing can be judged is
    // work nobody should pay for. **What a reader is not yet told is that this happened** — that
    // is *"no public information at the level of a whole competitor set"*, its own roadmap row,
    // and the reason it is not invented here is that one sentence covering *no engine*, *the
    // search did not finish* and *we could not read your company's page* is the collapse this
    // project has un-made three times.
    let words = match seed.words() {
        Ok(words) => words,
        Err(why) => {
            return (
                Set {
                    members: vec![Member {
                        candidate: seed.candidate.clone(),
                        because: Because::Named,
                    }],
                    set_aside: Vec::new(),
                    alone: Some(NoRivals::NothingToCompare(why)),
                },
                Queried::default(),
            )
        }
    };

    let queries = for_company(&seed.candidate.name);
    let mut results: Vec<Vec<crate::provider::Hit>> = Vec::with_capacity(queries.len());
    let mut queried = Queried::default();
    for query in &queries {
        match engine.search(query).await {
            Ok(hits) => {
                results.push(hits);
                queried.completed.push(query.text.clone());
            }
            Err(e) => {
                tracing::warn!(query = %query.text, error = %e, "a rival search did not complete");
                queried.failed.push(crate::candidates::Failed::new(
                    query.text.clone(),
                    crate::provider::Condition::of(&e),
                ));
            }
        }
    }

    let found: Vec<crate::candidates::Found> =
        crate::candidates::from_results(&results, queried.sent())
            .into_iter()
            .filter(|f| !same_company(&f.host, &seed.candidate.canonical_domain))
            .collect();

    let described = crate::candidates::describe(&found, &words, fetch).await;
    // **No literature on this path.** A reader who named a company asked about *that company's*
    // rivals, and the guides a market's own description surfaces are not what was searched for
    // here - `for_company` asks about the seed by name. `guides: 0` says so, and the exclusion
    // sentence says *we read no buyer's guide that might have* rather than implying one was
    // consulted.
    let mut set = assemble(
        described,
        crate::candidates::Sources {
            asked: queried.sent(),
            guides: 0,
        },
        &words,
        Evidence::ASeedsOwnPage,
    );

    // **The seed goes first and is not scored.** It is in the report because somebody typed it,
    // which is a different fact from every other row and outranks all of them.
    set.members.insert(
        0,
        Member {
            candidate: seed.candidate.clone(),
            because: Because::Named,
        },
    );
    set.alone = alone_because(
        set.members.len(),
        &queried,
        Sought::RivalsOfTheCompany,
        None,
    );
    (set, queried)
}

/// Whether a host found by a search is the company the search was about.
///
/// Compared on the registrable domain and ignoring `www.`, because
/// [`crate::candidates::from_results`] has already reduced a host to that — so this is a string
/// comparison and not a second, quieter copy of the suffix rules.
#[must_use]
pub fn same_company(host: &str, seed_domain: &str) -> bool {
    fn bare(d: &str) -> &str {
        d.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .trim_end_matches('/')
    }
    bare(host).eq_ignore_ascii_case(bare(seed_domain))
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
pub fn assemble(
    described: Vec<Described>,
    sources: crate::candidates::Sources,
    looked_for: &[String],
    from: Evidence,
) -> Set {
    let crate::candidates::Sources { asked, guides } = sources;
    let mut set = Set::default();
    for one in described {
        let Described {
            candidate,
            agreed,
            named_by,
            shares,
        } = one;
        // Ordered from the cheapest fact to the most specific, so a company that fails two of
        // these is reported by the first one — the reader is told the strongest reason it is
        // absent rather than the last one that happened to be checked.
        // **Two kinds of pointing, one threshold.** A search returning a company and a
        // publisher listing it are both one independent thing saying it belongs here, which is
        // why [`crate::literature`] has no second constant beside `CORROBORATION`.
        //
        // **Publishers, not links**: one guide linking three of a company's pages has said one
        // thing, and counting three would manufacture the corroboration this asks for.
        let named_by = crate::literature::publishers(&named_by);
        let aside = if agreed + named_by.len() < CORROBORATION {
            Some(Aside::Uncorroborated {
                agreed,
                asked,
                named_by: named_by.len(),
                guides,
            })
        } else if candidate.score() < MINIMUM_CONFIDENCE {
            Some(Aside::Unconvincing)
        } else {
            match shares {
                Vocabulary::NotRequested => Some(Aside::BeyondTheFetchBudget { budget: NAMED }),
                Vocabulary::Unreadable => Some(Aside::Unread),
                Vocabulary::Read(ref s) if s.len() < from.enough_words(looked_for.len()) => {
                    Some(Aside::ElsewhereEntirely {
                        looked_for: looked_for.to_vec(),
                        used: s.clone(),
                        needed: from.enough_words(looked_for.len()),
                    })
                }
                Vocabulary::Read(_) => None,
            }
        };
        match (aside, shares) {
            (Some(why), _) => set.set_aside.push((candidate, why)),
            (None, Vocabulary::Read(shares)) => set.members.push(Member {
                candidate,
                because: Because::Found {
                    agreed,
                    asked,
                    named_by,
                    guides,
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
            named_by: Vec::new(),
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

    /// The words a comparison is built on, for the tests that do not care which they are.
    fn market() -> Vec<String> {
        vec!["analytics".to_owned()]
    }

    fn read(words: &[&str]) -> Vocabulary {
        Vocabulary::Read(words.iter().map(|w| (*w).to_owned()).collect())
    }

    struct Canned {
        per_query: Vec<Result<Vec<crate::provider::Hit>, ()>>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl SourceProvider for Canned {
        fn name(&self) -> &str {
            "canned"
        }
        async fn search(
            &self,
            query: &Query,
        ) -> Result<Vec<crate::provider::Hit>, crate::provider::SearchError> {
            let mut asked = self.asked.lock().unwrap();
            let answer = self.per_query.get(asked.len()).cloned();
            asked.push(query.text.clone());
            match answer {
                Some(Ok(hits)) => Ok(hits),
                _ => Err(crate::provider::SearchError::Unreachable(
                    "no route to host".to_owned(),
                )),
            }
        }
    }

    fn hit(url: &str) -> crate::provider::Hit {
        crate::provider::Hit {
            url: url.to_owned(),
            title: String::new(),
            snippet: String::new(),
        }
    }

    /// Searches only, which is what every test here that predates the literature assumes.
    fn sources(asked: usize) -> crate::candidates::Sources {
        crate::candidates::Sources { asked, guides: 0 }
    }

    fn seed() -> Seed {
        Seed {
            candidate: Candidate {
                name: "Basecamp".to_owned(),
                canonical_domain: "basecamp.com".to_owned(),
                what_it_is: "Project management and team communication".to_owned(),
                confidence: 1.0,
            },
            read: true,
        }
    }

    /// The same company, with its front page unreadable - so `what_it_is` is our sentence.
    fn seed_nobody_could_read() -> Seed {
        Seed {
            candidate: Candidate {
                name: "basecamp.com".to_owned(),
                canonical_domain: "basecamp.com".to_owned(),
                what_it_is: "we were unable to read its front page".to_owned(),
                confidence: 1.0,
            },
            read: false,
        }
    }

    #[test]
    fn the_queries_a_company_produces_use_its_own_name_and_nothing_a_reader_typed() {
        let asked: Vec<String> = for_company("Basecamp")
            .into_iter()
            .map(|q| q.text)
            .collect();
        assert_eq!(
            asked,
            vec![
                "Basecamp alternatives",
                "Basecamp competitors",
                "companies like Basecamp"
            ]
        );
    }

    #[test]
    fn three_queries_because_one_agrees_with_nothing() {
        // The same argument `CORROBORATION` rests on, at the other end of the pipe.
        assert_eq!(for_company("Basecamp").len(), 3);
        assert!(
            for_company("   ").is_empty(),
            "blank input asked an engine anyway"
        );
    }

    #[test]
    fn a_company_is_recognized_however_its_domain_is_written() {
        assert!(same_company("basecamp.com", "basecamp.com"));
        assert!(same_company("www.basecamp.com", "basecamp.com"));
        assert!(same_company("BASECAMP.COM", "https://basecamp.com/"));
        assert!(!same_company("basecamp.co", "basecamp.com"));
        assert!(!same_company("notbasecamp.com", "basecamp.com"));
    }

    #[tokio::test]
    async fn a_named_company_is_first_and_is_not_scored() {
        let all = vec![
            hit("https://basecamp.com/"),
            hit("https://linear.app/"),
            hit("https://asana.com/"),
        ];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed(), |url| async move {
            Some(format!(
                "# A company at {url}\n\nProject management for a team."
            ))
        })
        .await;

        assert_eq!(queried.completed.len(), 3);
        assert_eq!(set.members[0].candidate.canonical_domain, "basecamp.com");
        assert_eq!(set.members[0].because, Because::Named);
        assert_eq!(set.members[0].because.sentence(), "you named it");

        let rest: Vec<&str> = set.members[1..]
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(rest, vec!["asana.com", "linear.app"], "{:#?}", set.members);
    }

    #[tokio::test]
    async fn a_company_is_never_its_own_competitor() {
        // **Every one of these searches returns the seed**, and it outscores everything else -
        // `basecamp alternatives` is a page about Basecamp. Leaving it in would put one company
        // in the report twice, once because a reader named it and once because a search for it
        // found it, which is a reason that argues with itself.
        let all = vec![
            hit("https://www.basecamp.com/pricing"),
            hit("https://basecamp.com/"),
        ];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, _) = of_company(&engine, &seed(), |url| async move {
            Some(format!(
                "# Basecamp at {url}\n\nProject management for a team."
            ))
        })
        .await;

        assert_eq!(set.members.len(), 1, "{:#?}", set.members);
        assert_eq!(set.members[0].because, Because::Named);
        assert!(
            set.set_aside
                .iter()
                .all(|(c, _)| c.canonical_domain != "basecamp.com"),
            "the seed was reported as an excluded competitor: {:#?}",
            set.set_aside
        );
    }

    #[tokio::test]
    async fn a_rival_shares_the_seeds_words_rather_than_a_readers() {
        // The vocabulary floor, pointed at the only words available here: the seed's own
        // one-line description of itself. A company with nothing in common with it is set
        // aside and named, exactly as on the description path.
        let all = vec![hit("https://linear.app/"), hit("https://bakery.example/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, _) = of_company(&engine, &seed(), |url| async move {
            Some(if url.contains("bakery") {
                "# A Bakery\n\nSourdough, baked fresh each morning.".to_owned()
            } else {
                "# Linear\n\nProject management built for speed.".to_owned()
            })
        })
        .await;

        let compared: Vec<&str> = set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(
            compared,
            vec!["basecamp.com", "linear.app"],
            "{:#?}",
            set.members
        );
        assert_eq!(set.set_aside.len(), 1);
        assert_eq!(set.set_aside[0].0.canonical_domain, "bakery.example");
        assert!(matches!(
            set.set_aside[0].1,
            Aside::ElsewhereEntirely { .. }
        ));
    }

    #[tokio::test]
    async fn a_rival_is_scored_against_the_queries_that_were_sent() {
        // The same rule the description path follows, on the other input. Two searches agreeing
        // out of three sent is agreement between two of three; dividing by the two that
        // answered would let an engine outage read as unanimity.
        let both = vec![hit("https://linear.app/")];
        let engine = Canned {
            per_query: vec![Ok(both.clone()), Ok(both), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed(), |_url| async {
            Some("# Linear\n\nProject management built for speed.".to_owned())
        })
        .await;

        assert_eq!(queried.failed.len(), 1);
        let rival = set
            .members
            .iter()
            .find(|m| m.candidate.canonical_domain == "linear.app")
            .expect("the rival two searches agreed on");
        let Because::Found { agreed, asked, .. } = &rival.because else {
            panic!("{:?}", rival.because)
        };
        assert_eq!(*agreed, 2);
        assert_eq!(*asked, 3, "an outage was counted as unanimity");
        assert!(
            rival.because.sentence().contains("2 of the 3"),
            "{}",
            rival.because.sentence()
        );
    }

    #[tokio::test]
    async fn the_coverage_a_reader_is_shown_is_what_was_actually_asked() {
        // **The number under *"at least"*.** A seeded report says *"You named basecamp.com, and
        // I found 4 more like it"* only when the searching finished; otherwise it says *"at
        // least 4"*, and `Searches` is the whole of what decides which. Both halves come from
        // the queries this function sent, so this is where the conversion is checked — the
        // worker used to do the arithmetic itself, in two places, and the second one was
        // written without it.
        let both = vec![hit("https://linear.app"), hit("https://height.app")];

        let finished = Canned {
            per_query: vec![Ok(both.clone()), Ok(both.clone()), Ok(both.clone())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (_, queried) = of_company(&finished, &seed(), |_url| async {
            Some(
                "# Linear

Project management built for speed."
                    .to_owned(),
            )
        })
        .await;
        let covered = queried.coverage().expect("queries were sent");
        assert_eq!(covered.answered, 3);
        assert_eq!(covered.failed, 0);
        assert!(
            covered.finished(),
            "a complete search was reported as incomplete, so the page hedges for no reason"
        );

        let partial = Canned {
            per_query: vec![Ok(both.clone()), Ok(both), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (_, queried) = of_company(&partial, &seed(), |_url| async {
            Some(
                "# Linear

Project management built for speed."
                    .to_owned(),
            )
        })
        .await;
        let covered = queried.coverage().expect("queries were sent");
        assert_eq!(covered.answered, 2);
        assert_eq!(covered.failed, 1);
        assert_eq!(
            covered.sent(),
            3,
            "a query that failed was not counted as sent"
        );
        assert!(
            !covered.finished(),
            "one query did not come back, and the count over it is not a definite number"
        );

        // **And a configured engine that was never asked anything reports nothing.** With the
        // seed's own page unreadable there is no vocabulary, so `of_company` returns before it
        // sends a query — and a coverage of nought out of nought would read as a search that
        // came back empty. Review found the same run serializing two different ways depending
        // only on whether an unused engine happened to be set.
        let unused = Canned {
            per_query: Vec::new(),
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) =
            of_company(&unused, &seed_nobody_could_read(), |_url| async { None }).await;
        assert_eq!(
            queried.sent(),
            0,
            "a query was sent with nothing to search for"
        );
        assert_eq!(
            queried.coverage(),
            None,
            "an engine nobody asked anything was reported as a search that found nothing"
        );
        assert!(
            matches!(set.alone, Some(NoRivals::NothingToCompare(_))),
            "{:?}",
            set.alone
        );
    }

    #[tokio::test]
    async fn our_own_error_message_is_never_the_market_vocabulary() {
        // **Review found a report citing our error message as a company's evidence.** With the
        // seed's front page unread, `what_it_is` holds *"we were unable to read its front
        // page"*; `content_words` of that yields `unable`, `read`, `front`, `page`, and a rival
        // whose page says *"read more on this page"* then clears `SHARED_WORDS` and joins the
        // comparison with `Because::Found` naming words that were ours, not theirs.
        //
        // Corroborated by all three searches, so nothing else would have kept it out.
        let all = vec![hit("https://toolbox.example/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed_nobody_could_read(), |_url| async {
            Some("# Toolbox\n\nRead more on this page.".to_owned())
        })
        .await;

        assert_eq!(
            set.members.len(),
            1,
            "a rival was admitted: {:#?}",
            set.members
        );
        assert_eq!(set.members[0].candidate.canonical_domain, "basecamp.com");
        assert_eq!(set.members[0].because, Because::Named);
        assert!(
            set.set_aside.is_empty(),
            "companies were fetched and judged against nothing: {:#?}",
            set.set_aside
        );
        // **And nothing was asked.** Searching, then fetching five front pages on other
        // people's servers, to conclude that none of them can be judged is work nobody should
        // pay for.
        assert_eq!(
            queried.sent(),
            0,
            "queries went out with nothing to judge against"
        );
        assert!(queried.completed.is_empty() && queried.failed.is_empty());
    }

    #[tokio::test]
    async fn a_seed_whose_page_says_nothing_quotable_asks_nothing_either() {
        // **Review found `read` was the wrong question.** A reachable page reading only
        // `# Basecamp` is `read == true` with an empty `what_it_is`, because `naming` wants a
        // line of four words before it will quote one. Three queries went out, a front page was
        // fetched, and a corroborated rival was set aside as *elsewhere entirely* - a negative
        // claim about that company, built out of missing evidence about this one.
        let seed = crate::candidates::named_seed("basecamp.com", |_url| async {
            Some("# Basecamp".to_owned())
        })
        .await;
        assert!(seed.read, "the page was reachable");
        assert_eq!(
            seed.words(),
            Err(crate::candidates::NoVocabulary::NothingQuotable),
            "a page with no prose gave us words"
        );

        let all = vec![hit("https://linear.app/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed, |_url| async {
            Some("# Linear\n\nProject management built for speed.".to_owned())
        })
        .await;

        assert_eq!(
            queried.sent(),
            0,
            "queries went out with nothing to judge against"
        );
        assert_eq!(set.members.len(), 1, "{:#?}", set.members);
        assert_eq!(set.members[0].because, Because::Named);
        assert!(
            set.set_aside.is_empty(),
            "a company was excluded on evidence we never had: {:#?}",
            set.set_aside
        );
    }

    #[test]
    fn an_exclusion_names_the_words_it_was_judged_against() {
        // **Review found the sentence saying *"none of the words you typed"*.** True when a
        // description was searched for; false on the seeded path, where the words come from the
        // seed company's own page and the reader typed only a domain - a false account of the
        // evidence used to exclude somebody. Naming them is true on both paths.
        let aside = Aside::ElsewhereEntirely {
            looked_for: vec!["project".to_owned(), "management".to_owned()],
            used: Vec::new(),
            needed: 1,
        };
        let said = aside.sentence();
        assert!(!said.contains("you typed"), "{said}");
        assert!(said.contains("\"project\", \"management\""), "{said}");

        // **And a page that used some of them is not told it used none.** While one word was
        // the whole test the two were the same fact; they stopped being the same the moment the
        // bar could be cleared by a page that shares something.
        let some = Aside::ElsewhereEntirely {
            looked_for: vec![
                "project".to_owned(),
                "management".to_owned(),
                "design".to_owned(),
                "agency".to_owned(),
            ],
            used: vec!["project".to_owned()],
            needed: 2,
        };
        let said = some.sentence();
        assert!(!said.contains("none"), "{said}");
        assert!(said.contains("\"project\""), "{said}");
        assert!(said.contains(" 2"), "{said}");
    }

    #[tokio::test]
    async fn a_seed_we_could_read_still_admits_rivals_on_its_own_words() {
        // The other half, so the rule above is a rule and not a refusal to do the work: the
        // same rival, the same page, and a seed whose description is really its own.
        let all = vec![hit("https://linear.app/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed(), |_url| async {
            Some("# Linear\n\nProject management built for speed.".to_owned())
        })
        .await;

        assert_eq!(queried.sent(), 3);
        let compared: Vec<&str> = set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(
            compared,
            vec!["basecamp.com", "linear.app"],
            "{:#?}",
            set.members
        );
    }

    #[tokio::test]
    async fn a_wordy_seed_page_does_not_raise_the_bar_for_its_rivals() {
        // **Review found the fit rule scaling off the wrong thing.** `enough_words` scales with
        // how much a *reader* said about a market. On this path nobody described a market: the
        // words are `content_words` over a sentence lifted from the seed's own front page, and
        // scaling with that means **the wordier a company's marketing copy, the more of it every
        // rival has to repeat verbatim**.
        //
        // This seed's sentence carries eight content words, which the described-market rule
        // would turn into a demand for four. Linear says *"project management built for speed"*
        // and shares two. It was in the set before this pull request and has to stay in it.
        let wordy = Seed {
            candidate: Candidate {
                name: "Basecamp".to_owned(),
                canonical_domain: "basecamp.com".to_owned(),
                what_it_is: "Project management collaboration tasks teams planning timelines \
                             communication"
                    .to_owned(),
                confidence: 1.0,
            },
            read: true,
        };
        assert_eq!(
            wordy.words().expect("a readable seed").len(),
            8,
            "the fixture only means something while the sentence is long"
        );
        assert_eq!(
            Evidence::ADescribedMarket.enough_words(8),
            4,
            "and while the other rule would ask for four of them"
        );

        let all = vec![hit("https://linear.app/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, _) = of_company(&engine, &wordy, |_url| async {
            Some("# Linear\n\nProject management built for speed.".to_owned())
        })
        .await;

        let compared: Vec<&str> = set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(
            compared,
            vec!["basecamp.com", "linear.app"],
            "a rival sharing the two words that matter is still a rival: {:#?}",
            set.set_aside
        );
    }

    #[test]
    fn the_two_kinds_of_evidence_are_not_the_same_claim() {
        // A reader who said four words about a market is asking for two of them back. A
        // stranger's home page running to fourteen words is not asking for anything.
        assert_eq!(Evidence::ADescribedMarket.enough_words(4), 2);
        assert_eq!(Evidence::ADescribedMarket.enough_words(14), 7);
        assert_eq!(Evidence::ASeedsOwnPage.enough_words(4), SHARED_WORDS_FLOOR);
        assert_eq!(Evidence::ASeedsOwnPage.enough_words(14), SHARED_WORDS_FLOOR);
    }

    #[test]
    fn a_company_the_literature_names_is_admitted_and_the_set_says_by_whom() {
        // **The rule and the sentence, asserted through `assemble` rather than beside it.** One
        // search is below `CORROBORATION`; two independent guides carry it over, and what a
        // reader is shown has to be those two facts rather than their sum.
        let mut one = described("workamajig.com", 1, 0.7, read(&["analytics"]));
        one.named_by = ["capterra.com", "g2.com"]
            .iter()
            .map(|by| crate::literature::Endorsement {
                by: (*by).to_owned(),
                host: "workamajig.com".to_owned(),
                at: "https://www.workamajig.com/".to_owned(),
            })
            .collect();
        let set = assemble(
            vec![one],
            crate::candidates::Sources {
                asked: 3,
                guides: 3,
            },
            &market(),
            Evidence::ADescribedMarket,
        );

        assert_eq!(set.members.len(), 1, "{set:#?}");
        let Because::Found {
            agreed,
            ref named_by,
            guides,
            ..
        } = set.members[0].because
        else {
            panic!("{:?}", set.members[0].because)
        };
        assert_eq!(agreed, 1, "the searches, not the sum");
        assert_eq!(named_by.len(), 2);
        assert_eq!(guides, 3);
    }

    #[test]
    fn a_reason_keeps_the_searches_and_the_guides_apart() {
        // **Two facts, not a sum.** *One search returned it* and *two buyer's guides list it*
        // are different evidence, and adding them would tell a reader three searches agreed
        // about a company one of them found. That is the collapse `landscape_core::coverage`
        // exists to stop, applied to why a company is in a report.
        let because = Because::Found {
            agreed: 1,
            asked: 3,
            named_by: vec!["capterra.com".to_owned(), "g2.com".to_owned()],
            guides: 3,
            shares: vec!["project".to_owned()],
        };
        let said = because.sentence();
        assert!(said.contains("1 of the 3 searches"), "{said}");
        assert!(said.contains("2 of the 3 buyer's guides"), "{said}");
        assert!(said.contains("capterra.com, g2.com"), "{said}");
        assert!(!said.contains("3 of the 3 searches"), "{said}");
    }

    #[test]
    fn an_exclusion_says_whether_the_literature_was_read_at_all() {
        // Three findings, not two: nobody listed it, nobody was asked, and one listed it. A
        // reader acts on each differently, which is the same rule `Vocabulary` follows.
        let none_read = Aside::Uncorroborated {
            agreed: 1,
            asked: 3,
            named_by: 0,
            guides: 0,
        };
        assert!(
            none_read.sentence().contains("no buyer's guide"),
            "{}",
            none_read.sentence()
        );
        let none_listed = Aside::Uncorroborated {
            agreed: 1,
            asked: 3,
            named_by: 0,
            guides: 4,
        };
        assert!(
            none_listed.sentence().contains("none of the 4"),
            "{}",
            none_listed.sentence()
        );
    }

    #[test]
    fn a_comparison_is_never_explained_as_an_absence() {
        // Two companies is a comparison, whatever else went wrong on the way to it. The note
        // this drives sits beside "this report compares X and Y", and a reason for having
        // nobody would contradict it.
        assert_eq!(
            alone_because(
                2,
                &Queried {
                    completed: vec!["q1".to_owned()],
                    failed: vec![
                        crate::candidates::Failed::new("q2", crate::provider::Condition::NoAnswer),
                        crate::candidates::Failed::new("q3", crate::provider::Condition::NoAnswer)
                    ],
                },
                Sought::RivalsOfTheCompany,
                None
            ),
            None
        );
    }

    #[test]
    fn four_reasons_for_one_company_and_no_two_are_the_same() {
        let all_answered = Queried {
            completed: vec!["q1".to_owned(), "q2".to_owned(), "q3".to_owned()],
            failed: Vec::new(),
        };
        let outage = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec![
                crate::candidates::Failed::new("q2", crate::provider::Condition::NoAnswer),
                crate::candidates::Failed::new("q3", crate::provider::Condition::NoAnswer),
            ],
        };

        // **Nothing was asked**, and the caller says which of the two ways that happened.
        assert_eq!(
            alone_because(
                1,
                &Queried::default(),
                Sought::RivalsOfTheCompany,
                Some(NoRivals::NoEngine)
            ),
            Some(NoRivals::NoEngine)
        );
        assert_eq!(
            alone_because(
                1,
                &Queried::default(),
                Sought::RivalsOfTheCompany,
                Some(NoRivals::NothingToCompare(
                    crate::candidates::NoVocabulary::NothingQuotable
                ))
            ),
            Some(NoRivals::NothingToCompare(
                crate::candidates::NoVocabulary::NothingQuotable
            ))
        );
        // Asked, and some did not come back. Retryable, and about us.
        assert_eq!(
            alone_because(1, &outage, Sought::RivalsOfTheCompany, None),
            Some(NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: Sought::RivalsOfTheCompany,
                fault: crate::provider::Fault::Silent,
            })
        );
        // Asked, all answered, nothing held up. The only one about the market.
        assert_eq!(
            alone_because(1, &all_answered, Sought::RivalsOfTheCompany, None),
            Some(NoRivals::NobodyHeldUp {
                sought: Sought::RivalsOfTheCompany
            })
        );

        // And every one of them reads differently, because a reader acts on each differently.
        let said: Vec<String> = [
            NoRivals::NoEngine,
            NoRivals::NothingToCompare(crate::candidates::NoVocabulary::Unreadable),
            NoRivals::NothingToCompare(crate::candidates::NoVocabulary::NothingQuotable),
            NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: Sought::RivalsOfTheCompany,
                fault: crate::provider::Fault::Silent,
            },
            NoRivals::NobodyHeldUp {
                sought: Sought::RivalsOfTheCompany,
            },
            NoRivals::NobodyHeldUp {
                sought: Sought::CompaniesMatchingTheDescription,
            },
        ]
        .iter()
        .map(NoRivals::sentence)
        .collect();
        for (i, one) in said.iter().enumerate() {
            for other in said.iter().skip(i + 1) {
                assert_ne!(one, other, "two silences collapsed into one sentence");
            }
        }
        assert!(said[3].contains("2 of the 3"), "{}", said[3]);
        assert!(said[3].contains("try again"), "{}", said[3]);
        assert!(
            !said[4].contains("try again"),
            "a statement about the market was made retryable: {}",
            said[4]
        );
    }

    #[tokio::test]
    async fn a_company_nobody_could_be_found_for_says_which_kind_of_nobody() {
        // End to end, both shapes that end in one company, and the sentences differ.
        let alone = |per_query: Vec<Result<Vec<crate::provider::Hit>, ()>>| async move {
            let engine = Canned {
                per_query,
                asked: std::sync::Mutex::new(Vec::new()),
            };
            of_company(&engine, &seed(), |_url| async {
                Some("# A bakery\n\nSourdough, baked fresh each morning.".to_owned())
            })
            .await
            .0
            .alone
        };

        // Every search ran; the one company that came back shares nothing with the seed.
        let found = vec![hit("https://bakery.example/")];
        assert_eq!(
            alone(vec![Ok(found.clone()), Ok(found.clone()), Ok(found)]).await,
            Some(NoRivals::NobodyHeldUp {
                sought: Sought::RivalsOfTheCompany
            })
        );
        // Two searches never came back. That is about us, not about the market.
        assert_eq!(
            alone(vec![Ok(Vec::new()), Err(()), Err(())]).await,
            Some(NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: Sought::RivalsOfTheCompany,
                fault: crate::provider::Fault::Silent,
            })
        );
    }

    #[test]
    fn an_engine_that_refuses_is_not_an_engine_that_is_slow() {
        use crate::candidates::Failed;
        use crate::provider::Condition;

        // Three searches, all refused - which is what an unconfigured SearXNG does to every
        // query, for ever. The sentence used to end "that is usually temporary - try again".
        let refused = Queried {
            completed: Vec::new(),
            failed: vec![
                Failed::new("q1", Condition::Answered(403)),
                Failed::new("q2", Condition::Answered(403)),
                Failed::new("q3", Condition::Answered(403)),
            ],
        };
        let why = alone_because(1, &refused, Sought::RivalsOfTheCompany, None)
            .expect("one company and three failures is a report about one company");
        let said = why.sentence();
        assert!(
            said.contains("asking again will not change it"),
            "a refusal must not be reported as weather: {said}"
        );
        assert!(!said.contains("usually temporary"), "{said}");

        // The same shape, with the engine silent, keeps the sentence it always had.
        let silent = Queried {
            completed: Vec::new(),
            failed: vec![Failed::new("q1", Condition::NoAnswer)],
        };
        let waited = alone_because(1, &silent, Sought::RivalsOfTheCompany, None)
            .expect("one company and a failure is a report about one company");
        assert!(waited.sentence().contains("usually temporary - try again"));
    }

    #[test]
    fn the_count_of_failures_is_the_same_fact_however_they_failed() {
        use crate::candidates::Failed;
        use crate::provider::{Condition, Fault};

        // Only the advice changes. A reader is still told how much of the looking happened,
        // because that is what makes the thinness of the report checkable.
        let queried = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec![
                Failed::new("q2", Condition::Answered(403)),
                Failed::new("q3", Condition::NoAnswer),
            ],
        };
        let why = alone_because(1, &queried, Sought::RivalsOfTheCompany, None).expect("alone");
        assert!(
            why.sentence()
                .contains("could not complete 2 of the 3 searches"),
            "{}",
            why.sentence()
        );
        assert_eq!(
            why,
            NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: Sought::RivalsOfTheCompany,
                // The refusal, not the timeout: one query has already proved that waiting
                // will not help.
                fault: Fault::Refused,
            }
        );
    }

    #[tokio::test]
    async fn a_seed_whose_page_gave_nothing_says_so_rather_than_naming_a_market() {
        let (set, _) = of_company(
            &Canned {
                per_query: Vec::new(),
                asked: std::sync::Mutex::new(Vec::new()),
            },
            &seed_nobody_could_read(),
            |_url| async { None },
        )
        .await;
        assert_eq!(
            set.alone,
            Some(NoRivals::NothingToCompare(
                crate::candidates::NoVocabulary::Unreadable
            ))
        );
        let said = set.alone.expect("a reason").sentence();
        assert!(
            said.contains("says nothing about who else is out there"),
            "{said}"
        );
    }

    #[tokio::test]
    async fn a_real_comparison_carries_no_reason_for_being_alone() {
        let all = vec![hit("https://linear.app/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, _) = of_company(&engine, &seed(), |_url| async {
            Some("# Linear\n\nProject management built for speed.".to_owned())
        })
        .await;
        assert_eq!(set.members.len(), 2);
        assert_eq!(set.alone, None, "a comparison was explained as an absence");
    }

    #[tokio::test]
    async fn a_named_company_survives_every_search_failing() {
        // The reader named it. An engine outage cannot take that away, and a report about the
        // one company they asked about is a real answer rather than a refusal.
        let engine = Canned {
            per_query: vec![Err(()), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (set, queried) = of_company(&engine, &seed(), |_url| async { None }).await;
        assert_eq!(queried.failed.len(), 3);
        assert_eq!(set.members.len(), 1);
        assert_eq!(set.members[0].because, Because::Named);
        assert!(set.set_aside.is_empty());
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
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
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
                        asked: 3,
                        named_by: 0,
                        guides: 0
                    }
                ),
                (
                    "notionpress.example",
                    &Aside::ElsewhereEntirely {
                        looked_for: vec!["analytics".to_owned()],
                        used: Vec::new(),
                        needed: 1
                    }
                ),
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
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
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
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
        );
        assert_eq!(set.set_aside.len(), 1);
        assert_eq!(set.set_aside[0].1, Aside::Unread);
        assert!(!matches!(
            set.set_aside[0].1,
            Aside::ElsewhereEntirely { .. }
        ));
        assert!(set.set_aside[0].1.sentence().contains("name the domain"));
    }

    #[test]
    fn the_strongest_reason_is_the_one_a_reader_is_given() {
        // One search found it *and* its page is about something else. "Nothing corroborates it"
        // is the fact that decided; reporting the vocabulary would suggest the search agreed.
        let set = assemble(
            vec![described("thin.example", 1, 0.175, read(&[]))],
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
        );
        assert_eq!(
            set.set_aside[0].1,
            Aside::Uncorroborated {
                agreed: 1,
                asked: 3,
                named_by: 0,
                guides: 0
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
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
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
        let because = Because::Found {
            named_by: Vec::new(),
            guides: 0,
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
            sources(3),
            &market(),
            Evidence::ADescribedMarket,
        );
        assert!(set.members[0].because.sentence().contains("2 of the 3"));
    }
}
