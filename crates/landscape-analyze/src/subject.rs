//! Turning what somebody typed into the companies a report will read.
//!
//! Two paths in, and they meet at [`decide`]:
//!
//! ```text
//! "basecamp.com vs linear.app"  -> origins_in       -> two sites, in the order written
//! "privacy-friendly analytics"  -> the search path  -> a set, or one of four refusals
//! ```
//!
//! The first needs nothing: [`origins_in`] reads the domains out of the prompt. The second is
//! `FACT_CHECKING.md` §3.1 — candidates, scores, and a gate that refuses to guess — followed by
//! [`landscape_search::competitors`], which turns what survived into the several companies a
//! landscape actually compares. [`decide`] is where every one of those endings is chosen, and
//! it is a pure function for that reason.
//!
//! **The refusals are four different sentences and never one.** No engine configured; the
//! searching did not finish; we looked and found nobody; we found companies and none held up.
//! A reader acts on each of them differently, and a prompt this cannot resolve **fails the
//! analysis with a reason** rather than picking a plausible domain — an analysis of the wrong
//! company is the most expensive wrong answer this product can produce, because everything in
//! it is correctly cited and about somebody else.

/// A hostname's last label, when it looks like a public suffix worth trusting.
///
/// Deliberately short. It is not a public-suffix list and does not try to be: it is the
/// difference between recognising `basecamp.com` in a sentence and treating every word with a
/// dot in it as a website.
const KNOWN_SUFFIXES: [&str; 24] = [
    "com", "org", "net", "io", "co", "dev", "app", "ai", "so", "sh", "xyz", "cloud", "tech",
    "software", "tools", "systems", "uk", "de", "fr", "nl", "se", "eu", "ca", "us",
];

/// The origin to analyse, if the prompt names one.
///
/// Returns a scheme and host — `https://example.com` — because everything downstream fetches
/// it and guessing the scheme for somebody is the kind of silent assumption this codebase
/// keeps finding bugs in. `http://` is preserved where it is written; a bare domain becomes
/// `https://`, which is what a browser does.
#[must_use]
pub fn origin_in(prompt: &str) -> Option<String> {
    origins_in(prompt).into_iter().next()
}

/// How many companies one report will cover.
///
/// **A latency cap, not a design limit.** Each company is its own discovery, fetches and model
/// calls, so a report about four takes four times as long — and a reader waiting ten minutes
/// has stopped waiting. Three is what fits inside the ninety-to-a-hundred-and-eighty seconds
/// `PRODUCT_SPEC.md` §2.1A asks for on the free tier; raising it is a decision about the wait,
/// which is why the number is here and named rather than inline.
pub const MAX_SUBJECTS: usize = 3;

/// Every site named in the prompt, in the order written, without repeats.
///
/// `basecamp.com vs linear.app` used to analyse Basecamp alone: [`origin_in`] took the first
/// domain and the rest were dropped in silence. Naming two companies and being given one is the
/// kind of wrong answer that looks like a right answer, because nothing on the page says the
/// second was ignored.
///
/// **Uncapped on purpose.** [`MAX_SUBJECTS`] is a decision about how long a reader will wait,
/// and applying it here would drop the fourth company as silently as the old code dropped the
/// second. The caller takes as many as it will analyse and *says* what it left out.
#[must_use]
pub fn origins_in(prompt: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in prompt.split_whitespace() {
        let Some(origin) = origin_of_word(word) else {
            continue;
        };
        // `basecamp.com and www.basecamp.com` is one company written twice. The comparison is
        // between *companies*, so a repeat is a duplicate rather than a second subject.
        if !found.iter().any(|seen| same_site(seen, &origin)) {
            found.push(origin);
        }
    }
    found
}

/// Whether two origins are the same site, ignoring a leading `www.`.
fn same_site(a: &str, b: &str) -> bool {
    fn bare(origin: &str) -> &str {
        origin
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
    }
    bare(a) == bare(b)
}

/// One word, if it is a URL or a domain.
fn origin_of_word(word: &str) -> Option<String> {
    let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != ':');
    let (scheme, rest) = match trimmed.split_once("://") {
        Some(("http", rest)) => ("http", rest),
        Some(("https", rest)) => ("https", rest),
        // Any other scheme is not something to fetch, and a bare word has no scheme yet.
        Some(_) => return None,
        None => ("https", trimmed),
    };

    let host = rest.split(['/', '?', '#']).next()?.trim_end_matches('.');
    if host.is_empty() || host.contains('@') || host.contains(' ') {
        return None;
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return None;
    }
    let suffix = labels.last()?.to_lowercase();
    if !KNOWN_SUFFIXES.contains(&suffix.as_str()) {
        return None;
    }
    if !labels
        .iter()
        .all(|l| l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
    {
        return None;
    }
    Some(format!("{scheme}://{}", host.to_lowercase()))
}

/// What the domains in a prompt mean for a run.
///
/// **Three readings of the same box, and the middle one is new.** A prompt naming nothing has to
/// be searched for; a prompt naming several has already said what to compare. The interesting
/// case is **one**: a reader who types `basecamp.com` wants a competitive landscape, and giving
/// them a profile of Basecamp is answering a different question — the same mistake the
/// description path made when it returned one company instead of a set.
///
/// **Naming two is an instruction, not a starting point.** `basecamp.com vs linear.app` is a
/// reader saying which comparison they want, and adding a third company we found would be
/// overruling them. This is why the rule is a function with a name rather than an `if` in the
/// worker: it is a decision about what somebody meant, and it is worth being able to assert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subjects {
    /// Nothing named. Find the companies from the description.
    Describe,
    /// One named. It seeds the report, and its competitors are worth finding.
    Seed(String),
    /// Several named. Exactly these, in the order written.
    Exactly(Vec<String>),
}

/// Read a prompt's domains as one of [`Subjects`].
#[must_use]
pub fn subjects_in(prompt: &str) -> Subjects {
    let mut named = origins_in(prompt);
    match named.len() {
        0 => Subjects::Describe,
        // `swap_remove(0)` on a one-element vector is `remove(0)` without the shift, and says
        // out loud that nothing else is left to preserve the order of.
        1 => Subjects::Seed(named.swap_remove(0)),
        _ => Subjects::Exactly(named),
    }
}

/// What to tell somebody whose prompt named no site.
///
/// A failure reason a reader can act on. It says what is missing rather than what went wrong,
/// because nothing went wrong: this is a capability the pipeline does not have yet.
pub const NO_SUBJECT: &str = "this prompt does not name a website, and finding one from a \
    description needs a search engine, which is not configured here. Try a domain — for \
    example: basecamp.com";

/// What a reader is told when we searched and found nobody.
///
/// **Different from [`NO_SUBJECT`], and the difference is the whole honest-negative
/// discipline.** *"We have no way to look"* and *"we looked and found nothing"* are different
/// facts about the world, and a reader can act on the second by rewording rather than by
/// installing something.
pub const NOTHING_RESOLVED: &str = "we searched for companies matching that description and \
    found none we could identify well enough to report on. Try naming a domain, or describing \
    the product in the words a vendor would use — for example: basecamp.com";

/// What to do with the gate's verdict.
///
/// **The decision, in a function a test can call.** It lived in the worker as a `match` inside a
/// binary, where the only way to exercise it was to run one — so the arm that tells an outage
/// apart from an empty market had no test, and the mutation that deleted it passed. Every branch
/// here is reachable from a unit test, which matters because this is the code that decides
/// whether a report is written about the wrong company or not written at all.
#[derive(Debug, Clone, PartialEq)]
pub enum Decided {
    /// The companies the report will compare, best first.
    Analyse(Box<landscape_search::competitors::Set>),
    /// No set worth writing a report about, and this is what a reader is told about why.
    ///
    /// **The situation travels with the sentence.** They are one decision: the code that picks
    /// the words is the code that knows which of five things happened, and splitting them left
    /// the boundary rendering all five as *"try naming its website"* — wrong for a search that
    /// timed out, and throwing away a question a reader could answer in a word.
    Refuse(Refusal),
}

/// Everything a refusal is: the situation, the sentence, and the question that goes with it.
///
/// **One value rather than three loose ones**, because every caller has to carry all of it to
/// the same place. `refuse(store, analysis, kind, &why, &choices)` was five arguments, three of
/// them describing one refusal, and dropping the third at that call site is a silent defect the
/// type system cannot see. Mutation testing found exactly that: the choices removed on the way
/// from `decide` to the store, and nothing failed. There is nothing to drop now.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    /// For operators. Recorded, and never shown verbatim; see [`landscape_core::Failure`].
    pub why: String,
    /// Which situation this is, in the only terms a reader can act on.
    pub kind: landscape_core::Failure,
    /// The companies a reader can pick between. Empty except on the ambiguous branch.
    ///
    /// **The question travels with the situation for the same reason the situation travels
    /// with the sentence.** Saying *"that name matches more than one company"* without saying
    /// which leaves a reader to guess exactly what the gate refused to guess.
    pub choices: Vec<landscape_core::Choice>,
}

/// Turn the gate's verdict, the set behind it, and the searching that produced both into what
/// happens next.
///
/// **The order of these arms is the design.** A verdict that stops a report stops it first; only
/// then is the set consulted. Reversing any two of them changes which sentence a reader gets for
/// the same run, so each arm has its own test.
///
/// | The gate said | And | What happens |
/// |---|---|---|
/// | Several, and the reader typed a **name** | — | Ask which one. `PRODUCT_SPEC.md` §3 |
/// | Nothing | a search failed | Try again — the searching did not finish |
/// | Nothing | every search ran | We looked and found nobody |
/// | Anything else | the set is empty | Say which of them was found and why none are in it |
/// | Anything else | the set has members | Compare them |
#[must_use]
pub fn decide(
    derived: landscape_search::competitors::Derived,
    queried: &landscape_search::candidates::Queried,
) -> Decided {
    use landscape_core::subject::Resolution;
    let landscape_search::competitors::Derived {
        verdict,
        set,
        about_a_market,
    } = derived;
    match verdict {
        // **Several candidates and one word typed is the ambiguity the gate exists for.** Three
        // products called *Notion* are not a market, and comparing them against each other is a
        // report nobody asked for. A reader answers this in one word; see
        // [`landscape_search::competitors::DESCRIBES_A_MARKET`] for what "one word" means and
        // what it cannot see.
        Resolution::Ambiguous { candidates, .. } if !about_a_market => Decided::Refuse(Refusal {
            why: ambiguous(&candidates),
            kind: landscape_core::Failure::Ambiguous,
            choices: choices_from(&candidates),
        }),
        // **"Nothing found" is a conclusion about a market, and it needs the searching to have
        // happened.** With any query unanswered we have not established that nobody is out
        // there — only that we did not finish looking, which is a different sentence and a
        // retryable one. Review found these collapsed into each other.
        //
        // **This is checked against *no members surviving*, not against the gate's verdict.**
        // It used to guard only the `NothingFound` arm, and review found the way round it: two
        // queries corroborate a candidate, the third fails, the gate resolves it — and then
        // `assemble` sets it aside because its page could not be read. `Resolved` with an empty
        // set falls past a verdict-shaped guard and lands on *"we searched and found nobody"*,
        // which is a conclusion about a market drawn while a query was still unanswered, and
        // the one refusal it makes sense to retry offered as one it does not.
        //
        // A **non-empty** set still beats an outage: an answer we already have is worth more
        // than telling somebody to come back for it.
        // **Which of the two depends on whether the engine spoke.** `fault()` is `Some`
        // whenever anything failed, which the guard above has established; the fallback is
        // unreachable and picks the retryable reading, which is the one that assumes least.
        _ if set.is_empty() && !queried.failed.is_empty() => {
            let fault = queried.fault().unwrap_or(landscape_search::Fault::Silent);
            Decided::Refuse(Refusal {
                why: search_incomplete(queried.failed.len(), queried.sent(), fault),
                kind: if fault == landscape_search::Fault::Refused {
                    landscape_core::Failure::SearchRefused
                } else {
                    landscape_core::Failure::SearchIncomplete
                },
                choices: Vec::new(),
            })
        }
        Resolution::NothingFound { .. } => Decided::Refuse(Refusal {
            why: NOTHING_RESOLVED.to_owned(),
            kind: landscape_core::Failure::NothingFound,
            choices: Vec::new(),
        }),
        // Companies were found and none of them survived. Naming them and saying what happened
        // to each is the difference between a refusal a reader can act on and a shrug.
        _ if set.is_empty() && !set.set_aside.is_empty() => {
            // Companies were found and rejected, which is a statement about a market
            // rather than about the searching — the same situation as finding nobody, arrived
            // at with more to show for it.
            Decided::Refuse(Refusal {
                why: none_of_them(&set.set_aside),
                kind: landscape_core::Failure::NothingFound,
                choices: Vec::new(),
            })
        }
        _ if set.is_empty() => Decided::Refuse(Refusal {
            why: NOTHING_RESOLVED.to_owned(),
            kind: landscape_core::Failure::NothingFound,
            choices: Vec::new(),
        }),
        _ => Decided::Analyse(Box::new(set)),
    }
}

/// What a reader is told when every company we found was set aside.
///
/// **Each one is named with its own reason**, because *"we found nothing"* would be false — we
/// found companies and rejected them, and a reader who can see which ones can tell a bad search
/// from a market that really is empty. This is the same rule `FACT_CHECKING.md` §5.4 applies to
/// an empty section: a negative nobody can check is not a finding.
#[must_use]
pub fn none_of_them(
    set_aside: &[(
        landscape_core::subject::Candidate,
        landscape_search::competitors::Aside,
    )],
) -> String {
    let named: Vec<String> = set_aside
        .iter()
        .map(|(c, why)| format!("{} ({}) - {}", c.name, c.canonical_domain, why.sentence()))
        .collect();
    format!(
        "we found companies for that description and none of them held up: {}. Try naming a \
         domain, or describing the product in the words a vendor would use.",
        named.join("; ")
    )
}

/// The notes a report opens with when nobody named the companies in it.
///
/// **Three notes rather than one paragraph, and each one is a separate fact:** who this report
/// is about, why each of them is here, and which companies were found and left out. A reader
/// scanning the top of a report should be able to stop after the first and still know what they
/// are looking at.
///
/// `analysing` is how many of the set the run will actually read — [`MAX_SUBJECTS`] is a
/// decision about how long somebody waits, and naming five companies in a note while comparing
/// three would be the silent-drop defect with better prose in front of it.
#[must_use]
pub fn found_for_you(set: &landscape_search::competitors::Set, analysing: usize) -> Vec<String> {
    let compared: Vec<&landscape_search::competitors::Member> =
        set.members.iter().take(analysing).collect();
    if compared.is_empty() {
        return Vec::new();
    }

    let named: Vec<String> = compared
        .iter()
        .map(|m| format!("{} ({})", m.candidate.name, m.candidate.canonical_domain))
        .collect();
    // **Who this report is about, and it is a different sentence depending on how they got
    // here.** A reader who named a domain knows what they typed; telling them they described
    // a market is telling them something false about their own input. What they do not know
    // is that we went looking for the others.
    let seeded = matches!(
        compared.first().map(|m| &m.because),
        Some(landscape_search::competitors::Because::Named)
    );
    // **A report about one company must not claim we compared it with anybody.** When the
    // set is alone, the sentence about searching would be false in three of the four cases
    // and misleading in the fourth, so it is not said at all - `set.alone` says what
    // happened instead, in the note below.
    let seed_domain = compared
        .first()
        .map_or_else(String::new, |m| m.candidate.canonical_domain.clone());
    let mut notes = vec![match (seeded, set.alone.is_some()) {
        (true, true) => format!("You named {seed_domain}, so this report is about it."),
        (true, false) => format!(
            "You named {seed_domain}, so we searched for the companies it competes with. \
             This report compares {}. If those are not the companies you meant, name the \
             domains and we will read those instead.",
            joined_with_and(&named)
        ),
        (false, true) => format!(
            "You described a market rather than naming companies, so we searched for them. \
             This report is about {}.",
            joined_with_and(&named)
        ),
        (false, false) => format!(
            "You described a market rather than naming companies, so we searched for them. \
             This report compares {}. If those are not the companies you meant, name the \
             domains and we will read those instead.",
            joined_with_and(&named)
        ),
    }];

    // **The honest empty, at the level of a whole set.** Four different reasons a report
    // covers one company, and a reader acts on each differently: one is fixed by
    // configuring something, one by waiting, one by naming a different company, and one is
    // the only statement about the market. Collapsing them was refused three times while
    // the rest of this row was built; this is that refusal paid off.
    if let Some(why) = set.alone.as_ref() {
        notes.push(why.sentence());
    }

    notes.push(format!(
        "Why each one is here: {}.",
        compared
            .iter()
            .map(|m| format!("{} - {}", m.candidate.name, m.because.sentence()))
            .collect::<Vec<_>>()
            .join("; ")
    ));

    // **A company found and left out in silence is the defect this row exists to remove**, at
    // one company's remove from the one it removed first. Naming them costs a sentence.
    //
    // **Named, or counted, and the line between them is corroboration.** A company two searches
    // agreed on could have been in the report, so it is named with its reason. A host one search
    // returned could not have been, and there can be twenty of them — naming those turns a note
    // into a search results page, which is a different way of not being read. They are counted
    // instead, and the count is still not a silence: `landscape candidates` prints every one.
    let (could_have, single): (Vec<_>, Vec<_>) = set.set_aside.iter().partition(|(_, why)| {
        !matches!(
            why,
            landscape_search::competitors::Aside::Uncorroborated { .. }
        )
    });

    let mut said = Vec::new();
    if !could_have.is_empty() {
        said.push(format!(
            "Also found and not compared: {}",
            could_have
                .iter()
                .map(|(c, why)| format!("{} ({}) - {}", c.name, c.canonical_domain, why.sentence()))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    if !single.is_empty() {
        said.push(format!(
            "{} further {} returned by only one search, which is not enough to corroborate \
             anything",
            single.len(),
            if single.len() == 1 { "site" } else { "sites" }
        ));
    }
    if !said.is_empty() {
        notes.push(format!("{}.", said.join(". ")));
    }
    notes
}

/// `a`, `a and b`, `a, b and c` — the way somebody would say a list out loud.
fn joined_with_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// What an **operator** is told when the searching itself did not finish.
///
/// **Not the same as finding nobody, and review found the two collapsed.** With every query
/// failing, the previous version said *"we searched and found none"* — a conclusion about a
/// market, drawn from a conclusion about nothing. This one is about us.
///
/// **And it is not always retryable, which this used to promise it was.** It ended *"this is
/// usually temporary - try again"* whatever had happened, including when every query came back
/// refused. The count is the same fact either way; the advice is not, and
/// [`landscape_search::Fault`] is what tells them apart.
///
/// This string is `failure_reason` — the operator's column, never rendered verbatim
/// (`migrations/0001_init.sql`). What a reader sees comes from the [`landscape_core::Failure`]
/// beside it, which is why the two are chosen together at one call site.
#[must_use]
pub fn search_incomplete(failed: usize, sent: usize, fault: landscape_search::Fault) -> String {
    format!(
        "we could not complete {failed} of the {sent} searches needed to work out which company \
         you mean, so we have not concluded anything about who is out there. {} Naming a domain \
         skips the search entirely.",
        fault.advice()
    )
}

/// The companies a reader picks between, and the words that pick each one.
///
/// **A domain, not a name.** The prompt a chip sends is the company's canonical domain, because
/// that is the one input `subjects_in` reads as *"this company, definitely"* — a name would go
/// back through the search that produced the ambiguity and could return the same question.
///
/// **As an origin, not a bare host, and that is a bug fix rather than a preference.** A chip
/// sending `box.com` produced a `400`: seven characters, and [`landscape_core::MIN_PROMPT`] is
/// eight. The chip looked like one click and was a dead end for every company with a short
/// domain. `https://` is eight characters by itself, so an origin is long enough whatever the
/// domain is — the prompt is now valid *by construction* rather than for most inputs.
///
/// It is also what [`landscape_search::competitors::Set::origins`] already produces from the
/// same field, so a chip sends the pipeline the shape it uses internally rather than a second
/// spelling of it.
///
/// One click is the whole answer: nothing here asks a reader to retype their idea with a company
/// bolted onto it, which is the difference between answering a question and doing the work.
#[must_use]
pub fn choices_from(
    candidates: &[landscape_core::subject::Candidate],
) -> Vec<landscape_core::Choice> {
    candidates
        .iter()
        .map(|c| landscape_core::Choice {
            name: c.name.clone(),
            domain: c.canonical_domain.clone(),
            what_it_is: c.what_it_is.clone(),
            prompt: format!("https://{}", c.canonical_domain),
        })
        .collect()
}

/// What a reader is told when we found several and will not choose for them.
///
/// `PRODUCT_SPEC.md` §3: *one chip click prevents an entire wrong report*. There are no chips
/// yet, so the companies are named in the sentence and the reader picks one by typing it —
/// which is the same choice, made more slowly, and far better than a confident wrong report.
#[must_use]
pub fn ambiguous(candidates: &[landscape_core::subject::Candidate]) -> String {
    let named: Vec<String> = candidates
        .iter()
        .map(|c| format!("{} ({})", c.name, c.canonical_domain))
        .collect();
    format!(
        "that description matches more than one company and we will not guess between them: \
         {}. Name the one you mean - a domain works.",
        named.join(", ")
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod reading_a_box {
    //! What the domains in a prompt mean, and why one is not the same as two.

    use super::*;

    #[test]
    fn nothing_named_is_a_description_to_search_for() {
        assert_eq!(
            subjects_in("a shared inbox for a small support team"),
            Subjects::Describe
        );
    }

    #[test]
    fn one_named_company_seeds_a_landscape() {
        // **The change this row is for.** A reader who types one domain into a competitive
        // landscape tool is asking who else is out there, not for a profile.
        assert_eq!(
            subjects_in("basecamp.com"),
            Subjects::Seed("https://basecamp.com".to_owned())
        );
        assert_eq!(
            subjects_in("what is going on at https://linear.app these days"),
            Subjects::Seed("https://linear.app".to_owned())
        );
    }

    #[test]
    fn two_named_companies_are_an_instruction_rather_than_a_starting_point() {
        // Adding a third company we found would be overruling somebody who has already said
        // what they want compared.
        assert_eq!(
            subjects_in("basecamp.com vs linear.app"),
            Subjects::Exactly(vec![
                "https://basecamp.com".to_owned(),
                "https://linear.app".to_owned()
            ])
        );
    }

    #[test]
    fn one_company_written_twice_is_still_one_company() {
        // `origins_in` already drops the repeat; this pins that the *reading* follows it, so
        // `basecamp.com and www.basecamp.com` seeds a landscape rather than asking for a
        // comparison of a company with itself.
        assert_eq!(
            subjects_in("basecamp.com and www.basecamp.com"),
            Subjects::Seed("https://basecamp.com".to_owned())
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod deciding {
    //! Every arm of the decision, and why no two of them are the same sentence.

    use super::*;
    use landscape_core::subject::{Candidate, Resolution};
    use landscape_search::candidates::Queried;
    use landscape_search::competitors::{Aside, Because, Derived, Member, Set};

    fn candidate(name: &str, domain: &str) -> Candidate {
        Candidate {
            name: name.to_owned(),
            canonical_domain: domain.to_owned(),
            what_it_is: "a company".to_owned(),
            confidence: 0.9,
        }
    }

    fn member(name: &str, domain: &str) -> Member {
        Member {
            candidate: candidate(name, domain),
            because: Because::Found {
                agreed: 3,
                asked: 3,
                shares: vec!["analytics".to_owned()],
            },
        }
    }

    fn all_answered() -> Queried {
        Queried {
            completed: vec!["q1".to_owned(), "q2".to_owned(), "q3".to_owned()],
            failed: Vec::new(),
        }
    }

    fn derived(verdict: Resolution, set: Set, about_a_market: bool) -> Derived {
        Derived {
            verdict,
            set,
            about_a_market,
        }
    }

    fn tie() -> Resolution {
        Resolution::Ambiguous {
            question: "which one?".to_owned(),
            candidates: vec![
                candidate("Alpha", "alpha.example"),
                candidate("Beta", "beta.example"),
            ],
        }
    }

    fn two() -> Set {
        Set {
            members: vec![
                member("Alpha", "alpha.example"),
                member("Beta", "beta.example"),
            ],
            set_aside: Vec::new(),
            alone: None,
        }
    }

    #[test]
    fn every_refusal_carries_the_situation_a_reader_would_act_on() {
        // **These were one situation and five sentences.** `Failure` had two values, so every
        // ending here arrived as `NoSubject` and the interface rendered *"try naming its
        // website"* - which fixes nothing when a search timed out, and throws away the question
        // a reader could have answered in a word.
        use landscape_core::Failure;

        let outage = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec![
                landscape_search::candidates::Failed::new(
                    "q2",
                    landscape_search::Condition::NoAnswer,
                ),
                landscape_search::candidates::Failed::new(
                    "q3",
                    landscape_search::Condition::NoAnswer,
                ),
            ],
        };
        let cases: Vec<(&str, Decided, Failure)> = vec![
            (
                "a name several products share",
                decide(derived(tie(), two(), false), &all_answered()),
                Failure::Ambiguous,
            ),
            (
                "a search that did not finish",
                decide(
                    derived(
                        Resolution::NothingFound {
                            checked: Vec::new(),
                        },
                        Set::default(),
                        true,
                    ),
                    &outage,
                ),
                Failure::SearchIncomplete,
            ),
            (
                "a market we looked at and found empty",
                decide(
                    derived(
                        Resolution::NothingFound {
                            checked: vec!["q1".to_owned()],
                        },
                        Set::default(),
                        true,
                    ),
                    &all_answered(),
                ),
                Failure::NothingFound,
            ),
            (
                "companies found and all set aside",
                decide(
                    derived(
                        Resolution::Resolved {
                            entity: candidate("Alpha", "alpha.example"),
                        },
                        Set {
                            members: Vec::new(),
                            set_aside: vec![(candidate("Beta", "beta.example"), Aside::Unread)],
                            alone: None,
                        },
                        true,
                    ),
                    &all_answered(),
                ),
                Failure::NothingFound,
            ),
        ];

        for (what, decided, expected) in cases {
            let Decided::Refuse(Refusal { kind, why, .. }) = decided else {
                panic!("{what} was not a refusal")
            };
            assert_eq!(kind, expected, "{what}: {why}");
        }
    }

    #[test]
    fn the_companies_we_would_not_choose_between_travel_with_the_question() {
        // **A refusal that names the situation and not the candidates is half a question.**
        // Telling somebody a name matched several companies, without saying which, leaves them
        // guessing at exactly what the gate declined to guess at.
        let Decided::Refuse(Refusal { kind, choices, .. }) =
            decide(derived(tie(), two(), false), &all_answered())
        else {
            panic!("a tie about one company is a refusal")
        };
        assert_eq!(kind, landscape_core::Failure::Ambiguous);
        assert_eq!(
            choices
                .iter()
                .map(|c| c.domain.as_str())
                .collect::<Vec<_>>(),
            ["alpha.example", "beta.example"],
            "the candidates the gate was tied between are the ones offered"
        );
    }

    #[test]
    fn only_the_question_a_reader_can_answer_comes_with_choices() {
        // Every other refusal is about something a chip cannot fix: nobody to pick between, a
        // search still running, a prompt with no company in it. Attaching candidates to those
        // would offer a reader a button that changes nothing.
        let outage = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec![
                landscape_search::candidates::Failed::new(
                    "q2",
                    landscape_search::Condition::NoAnswer,
                ),
                landscape_search::candidates::Failed::new(
                    "q3",
                    landscape_search::Condition::NoAnswer,
                ),
            ],
        };
        let empty_market = decide(
            derived(
                Resolution::NothingFound {
                    checked: vec!["q1".to_owned()],
                },
                Set::default(),
                true,
            ),
            &all_answered(),
        );
        let interrupted = decide(
            derived(
                Resolution::NothingFound {
                    checked: Vec::new(),
                },
                Set::default(),
                true,
            ),
            &outage,
        );
        for (what, decided) in [
            ("a market we looked at and found empty", empty_market),
            ("a search that did not finish", interrupted),
        ] {
            let Decided::Refuse(Refusal { kind, choices, .. }) = decided else {
                panic!("{what} was not a refusal")
            };
            assert!(
                choices.is_empty(),
                "{what} ({kind:?}) offered a choice that would not help"
            );
        }
    }

    #[test]
    fn one_click_is_the_whole_answer() {
        // `PRODUCT_SPEC.md` §3 prices a clarification at one click, which is only true if the
        // chip carries a prompt rather than a name to be typed back into a sentence. **A
        // domain, not a name**: a name goes back through the search that produced the tie and
        // can return the same question.
        let offered = choices_from(&[
            candidate("Notion", "notion.so"),
            candidate("Notion Energy", "notionenergy.com"),
        ]);
        assert_eq!(
            offered
                .iter()
                .map(|c| c.prompt.as_str())
                .collect::<Vec<_>>(),
            ["https://notion.so", "https://notionenergy.com"],
            "what a chip sends must be the domain that resolves without another search"
        );
        assert_eq!(offered[0].name, "Notion");
        assert_eq!(
            offered[0].what_it_is, "a company",
            "the line that tells two candidates apart is the one they were described by"
        );
        assert_eq!(
            offered[0].domain, "notion.so",
            "what a reader reads is the bare domain; the scheme is for the parser"
        );
    }

    #[test]
    fn a_chip_a_short_domain_cannot_send_is_not_one_click() {
        // **Review found this, and the shape is worth more than the case.** `Choice::prompt` was
        // the bare `canonical_domain`, and `NewAnalysis::parse` rejects anything under
        // `MIN_PROMPT`. `box.com` is seven characters: the chip rendered, the click posted, and
        // the reader got *"a prompt must contain at least 8 characters"* for a company we had
        // resolved ourselves and put in front of them.
        //
        // **Two properties, not one example.** Length alone would pass if the prompt were
        // padded with anything; what a chip owes is a prompt the API accepts *and* one that
        // resolves back to the company whose button it is.
        for domain in ["box.com", "wix.com", "notion.so", "notionenergy.com"] {
            let offered = choices_from(&[candidate("Whoever", domain)]);
            let prompt = &offered[0].prompt;

            landscape_core::NewAnalysis::parse(prompt).unwrap_or_else(|e| {
                panic!("a chip for {domain} sends a prompt the API rejects: {e}")
            });

            assert_eq!(
                subjects_in(prompt),
                Subjects::Seed(format!("https://{domain}")),
                "a chip for {domain} must resolve to that company and nothing else"
            );
        }
    }

    #[test]
    fn the_only_retryable_refusal_is_the_one_about_us() {
        // A reader fixes exactly one of these by doing nothing, so exactly one may say so.
        // **`SearchRefused` is the newest and it is on the other side**: the engine answered,
        // so waiting is waiting for a decision that has already been made.
        use landscape_core::Failure;
        for (kind, retryable) in [
            (Failure::NoSubject, false),
            (Failure::Ambiguous, false),
            (Failure::NothingFound, false),
            (Failure::SearchIncomplete, true),
            (Failure::SearchRefused, false),
        ] {
            assert_eq!(
                kind == Failure::SearchIncomplete,
                retryable,
                "{kind:?} changed sides"
            );
        }
        assert!(search_incomplete(2, 3, landscape_search::Fault::Silent).contains("try again"));
        assert!(
            !search_incomplete(2, 3, landscape_search::Fault::Refused).contains("try again"),
            "a refusal must not be offered as something waiting fixes"
        );
        assert!(!NOTHING_RESOLVED.contains("try again"));
        assert!(!NO_SUBJECT.contains("try again"));
    }

    #[test]
    fn an_engine_that_refused_is_a_different_refusal_from_one_that_never_answered() {
        // The counts are identical and the two are not the same event. This is the whole-run
        // outcome rather than a note on a report, so it is the one a reader meets first.
        use landscape_search::candidates::Failed;
        use landscape_search::Condition;
        for (condition, expected) in [
            (
                Condition::Answered(403),
                landscape_core::Failure::SearchRefused,
            ),
            (
                Condition::Answered(401),
                landscape_core::Failure::SearchRefused,
            ),
            (
                Condition::Unreadable,
                landscape_core::Failure::SearchRefused,
            ),
            (
                Condition::NoAnswer,
                landscape_core::Failure::SearchIncomplete,
            ),
            // The two that answered and still mean *later*.
            (
                Condition::Answered(408),
                landscape_core::Failure::SearchIncomplete,
            ),
            (
                Condition::Answered(429),
                landscape_core::Failure::SearchIncomplete,
            ),
            (
                Condition::Answered(503),
                landscape_core::Failure::SearchIncomplete,
            ),
        ] {
            let queried = Queried {
                completed: Vec::new(),
                failed: vec![Failed::new("q1", condition), Failed::new("q2", condition)],
            };
            let decided = decide(
                derived(
                    Resolution::NothingFound {
                        checked: Vec::new(),
                    },
                    Set::default(),
                    true,
                ),
                &queried,
            );
            let Decided::Refuse(refusal) = decided else {
                panic!("no set and failed queries is a refusal");
            };
            assert_eq!(refusal.kind, expected, "{condition:?}");
        }
    }

    #[test]
    fn a_market_becomes_the_set_rather_than_a_question() {
        // **The row this change is for.** Three companies every search returned score alike, so
        // the gate calls them tied - and for a description of a market, tied is the answer.
        let decided = decide(derived(tie(), two(), true), &all_answered());
        let Decided::Analyse(set) = decided else {
            panic!("a market was refused instead of compared")
        };
        assert_eq!(
            set.origins(),
            vec!["https://alpha.example", "https://beta.example"]
        );
    }

    #[test]
    fn a_name_several_products_share_is_still_a_question() {
        // The protection `FACT_CHECKING.md` 3.1 step 4 asks for, unchanged: one word typed and
        // several products matching it is ambiguity, not a market, and one chip click prevents
        // an entire wrong report.
        let decided = decide(derived(tie(), two(), false), &all_answered());
        let Decided::Refuse(Refusal { why, .. }) = decided else {
            panic!("a shared name was reported on instead of asked about")
        };
        assert!(why.contains("Alpha (alpha.example)"), "{why}");
        assert!(why.contains("Beta (beta.example)"), "{why}");
    }

    #[test]
    fn one_company_clearly_ahead_still_brings_the_rest_of_its_market() {
        // `Resolved` used to mean a report about exactly one company. It now means the gate had
        // no objection - what goes in the report is the set.
        let decided = decide(
            derived(
                Resolution::Resolved {
                    entity: candidate("Alpha", "alpha.example"),
                },
                two(),
                true,
            ),
            &all_answered(),
        );
        let Decided::Analyse(set) = decided else {
            panic!("a resolved subject was refused")
        };
        assert_eq!(set.members.len(), 2);
    }

    #[test]
    fn a_set_is_still_compared_when_a_search_failed() {
        // A real answer we already have beats telling somebody to try again for it.
        let decided = decide(
            derived(tie(), two(), true),
            &Queried {
                completed: vec!["q1".to_owned()],
                failed: vec![
                    landscape_search::candidates::Failed::new(
                        "q2",
                        landscape_search::Condition::NoAnswer,
                    ),
                    landscape_search::candidates::Failed::new(
                        "q3",
                        landscape_search::Condition::NoAnswer,
                    ),
                ],
            },
        );
        assert!(
            matches!(decided, Decided::Analyse(_)),
            "an answer was thrown away over an outage: {decided:?}"
        );
    }

    #[test]
    fn a_market_we_searched_and_found_empty_says_so() {
        let decided = decide(
            derived(
                Resolution::NothingFound {
                    checked: vec!["q1".to_owned()],
                },
                Set::default(),
                true,
            ),
            &all_answered(),
        );
        let Decided::Refuse(Refusal { why, .. }) = decided else {
            panic!("nothing found was not a refusal")
        };
        assert_eq!(why, NOTHING_RESOLVED);
        assert!(!why.contains("try again"), "{why}");
    }

    #[test]
    fn a_candidate_rejected_after_an_outage_is_still_an_outage() {
        // **Review found the way round the guard above.** Two queries corroborate a candidate,
        // the third fails, the gate resolves it - and `assemble` then sets it aside because its
        // page could not be read. `Resolved` with an empty set fell past a verdict-shaped check
        // and landed on *"we searched and found nobody"*: a conclusion about a market drawn
        // while a query was still unanswered, and the one refusal worth retrying offered as one
        // that is not.
        //
        // Both shapes of empty set, because the arm below it is the one that used to catch them.
        for set_aside in [
            vec![(candidate("Alpha", "alpha.example"), Aside::Unread)],
            Vec::new(),
        ] {
            let queried = Queried {
                completed: vec!["q1".to_owned(), "q2".to_owned()],
                failed: vec![landscape_search::candidates::Failed::new(
                    "q3",
                    landscape_search::Condition::NoAnswer,
                )],
            };
            let decided = decide(
                derived(
                    Resolution::Resolved {
                        entity: candidate("Alpha", "alpha.example"),
                    },
                    Set {
                        members: Vec::new(),
                        set_aside: set_aside.clone(),
                        alone: None,
                    },
                    true,
                ),
                &queried,
            );
            let Decided::Refuse(Refusal { kind, why, .. }) = decided else {
                panic!("an empty set was analysed")
            };
            assert_eq!(
                kind,
                landscape_core::Failure::SearchIncomplete,
                "a market was declared empty while a query was unanswered: {why}"
            );
            assert!(why.contains("1 of the 3"), "{why}");
            assert!(why.contains("try again"), "{why}");
        }
    }

    #[test]
    fn an_answer_we_already_have_still_beats_an_outage() {
        // The other side of the same rule, so it stays a rule: a set with somebody in it is a
        // real answer, and telling a reader to come back for it would throw that away.
        let decided = decide(
            derived(
                Resolution::Resolved {
                    entity: candidate("Alpha", "alpha.example"),
                },
                two(),
                true,
            ),
            &Queried {
                completed: vec!["q1".to_owned()],
                failed: vec![
                    landscape_search::candidates::Failed::new(
                        "q2",
                        landscape_search::Condition::NoAnswer,
                    ),
                    landscape_search::candidates::Failed::new(
                        "q3",
                        landscape_search::Condition::NoAnswer,
                    ),
                ],
            },
        );
        assert!(
            matches!(decided, Decided::Analyse(_)),
            "an answer was thrown away over an outage: {decided:?}"
        );
    }

    #[test]
    fn a_search_that_did_not_finish_is_never_reported_as_an_empty_market() {
        // **Review found these collapsed.** With every query failing, the reader was told *"we
        // searched and found none"* - a conclusion about a market drawn from a conclusion about
        // nothing. It is retryable and it is about us.
        use landscape_search::candidates::Failed;
        use landscape_search::Condition;
        for failed in [
            vec![
                Failed::new("q1", Condition::NoAnswer),
                Failed::new("q2", Condition::NoAnswer),
                Failed::new("q3", Condition::NoAnswer),
            ],
            vec![Failed::new("q3", Condition::NoAnswer)],
        ] {
            let queried = Queried {
                completed: vec!["q1".to_owned(); 3 - failed.len()],
                failed: failed.clone(),
            };
            let decided = decide(
                derived(
                    Resolution::NothingFound {
                        checked: queried.completed.clone(),
                    },
                    Set::default(),
                    true,
                ),
                &queried,
            );
            let Decided::Refuse(Refusal { why, .. }) = decided else {
                panic!("nothing found was not a refusal")
            };
            assert_ne!(why, NOTHING_RESOLVED, "{failed:?}");
            assert!(why.contains("try again"), "{why}");
            assert!(
                why.contains(&format!("{} of the 3", failed.len())),
                "the count a reader needs is missing: {why}"
            );
        }
    }

    #[test]
    fn companies_found_and_all_set_aside_are_named_with_their_reasons() {
        // *"We found nothing"* would be false, and a reader who can see which companies were
        // rejected can tell a bad search from a market that really is empty.
        let decided = decide(
            derived(
                Resolution::Resolved {
                    entity: candidate("Alpha", "alpha.example"),
                },
                Set {
                    members: Vec::new(),
                    set_aside: vec![
                        (
                            candidate("Alpha", "alpha.example"),
                            Aside::ElsewhereEntirely {
                                looked_for: vec!["analytics".to_owned()],
                            },
                        ),
                        (candidate("Beta", "beta.example"), Aside::Unread),
                    ],
                    alone: None,
                },
                true,
            ),
            &all_answered(),
        );
        let Decided::Refuse(Refusal { why, .. }) = decided else {
            panic!("an empty set was reported on")
        };
        assert_ne!(why, NOTHING_RESOLVED);
        assert!(why.contains("Alpha (alpha.example)"), "{why}");
        assert!(
            why.contains("none of the words this comparison is built on"),
            "{why}"
        );
        assert!(why.contains("Beta (beta.example)"), "{why}");
        assert!(why.contains("could not read its front page"), "{why}");
    }

    #[test]
    fn an_empty_set_with_nothing_to_report_falls_back_to_the_plain_negative() {
        let decided = decide(
            derived(
                Resolution::Resolved {
                    entity: candidate("Alpha", "alpha.example"),
                },
                Set::default(),
                true,
            ),
            &all_answered(),
        );
        assert_eq!(
            decided,
            Decided::Refuse(Refusal {
                why: NOTHING_RESOLVED.to_owned(),
                kind: landscape_core::Failure::NothingFound,
                choices: Vec::new(),
            })
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod the_examples {
    //! The curated ideas, run through the parser that will really read them.
    //!
    //! **This is the test the catalogue exists for.** A list of domains in a file is a promise;
    //! putting each prompt through `origins_in` - the function the worker calls, not a
    //! reimplementation of it - is the part that can fail. A wording change that breaks the
    //! parser's reading of a domain would otherwise be found by whoever clicked the chip.

    use super::{origins_in, MAX_SUBJECTS};

    #[test]
    fn every_example_prompt_resolves_to_exactly_the_companies_it_names() {
        for example in landscape_core::examples() {
            let expected: Vec<String> = example
                .companies
                .iter()
                .map(|c| format!("https://{c}"))
                .collect();
            assert_eq!(
                origins_in(&example.prompt()),
                expected,
                "{} does not parse to its own companies. The prompt is {:?}",
                example.id,
                example.prompt()
            );
        }
    }

    #[test]
    fn no_example_is_capped_when_it_runs() {
        // The cap drops companies and says so in a note above the sections. That note is right
        // for a prompt somebody typed and wrong on a curated example, where it would mean the
        // demo shipped an idea it cannot run.
        for example in landscape_core::examples() {
            assert!(
                example.companies.len() <= MAX_SUBJECTS,
                "{} names {} companies and the cap is {MAX_SUBJECTS}",
                example.id,
                example.companies.len()
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    #[test]
    fn every_site_named_is_a_subject() {
        // The defect this exists for. `origin_in` took the first and dropped the rest, so a
        // prompt naming two companies produced a report about one — with nothing on the page
        // saying the other had been ignored.
        assert_eq!(
            origins_in("compare basecamp.com vs linear.app for a small team"),
            vec!["https://basecamp.com", "https://linear.app"]
        );
    }

    #[test]
    fn the_same_site_twice_is_one_subject() {
        // A comparison is between companies, so `www.` and a repeat are not a second one.
        assert_eq!(
            origins_in("basecamp.com and www.basecamp.com and basecamp.com"),
            vec!["https://basecamp.com"]
        );
    }

    #[test]
    fn the_order_written_is_the_order_reported() {
        // A reader who writes B before A and reads A before B has to work out whether we
        // reordered them or misread them.
        assert_eq!(
            origins_in("linear.app vs basecamp.com"),
            vec!["https://linear.app", "https://basecamp.com"]
        );
    }

    #[test]
    fn every_site_is_returned_and_the_cap_is_somebody_elses_job() {
        // Capping here would drop the fourth company as silently as the old code dropped the
        // second — the same defect at a higher count. The parser returns what the prompt says
        // and the caller decides what it can afford, so it can say what it left out.
        let many = "a.com b.com c.com d.com e.com";
        assert_eq!(origins_in(many).len(), 5);
        assert!(origins_in(many).len() > MAX_SUBJECTS);
    }

    #[test]
    fn a_prompt_naming_nothing_has_no_subjects() {
        assert!(origins_in("an app that helps small farms sell to restaurants").is_empty());
    }

    #[test]
    fn the_first_site_named_is_still_the_single_subject() {
        // `origin_in` is now the first of `origins_in`, and callers that want one still get
        // the same answer they always did.
        assert_eq!(
            origin_in("compare basecamp.com vs linear.app"),
            Some("https://basecamp.com".to_owned())
        );
    }

    use super::*;

    #[test]
    fn a_url_is_taken_as_written() {
        assert_eq!(
            origin_in("https://basecamp.com/pricing").as_deref(),
            Some("https://basecamp.com")
        );
        assert_eq!(
            origin_in("http://example.org").as_deref(),
            Some("http://example.org")
        );
    }

    #[test]
    fn a_bare_domain_becomes_https() {
        // What a browser does with the same input.
        assert_eq!(
            origin_in("basecamp.com").as_deref(),
            Some("https://basecamp.com")
        );
    }

    #[test]
    fn a_domain_inside_a_sentence_is_found() {
        assert_eq!(
            origin_in("please compare basecamp.com against the others").as_deref(),
            Some("https://basecamp.com")
        );
        assert_eq!(
            origin_in("what does linear.app cost?").as_deref(),
            Some("https://linear.app")
        );
    }

    #[test]
    fn a_description_resolves_to_nothing() {
        // The case that matters. Guessing a domain here would produce a report that is
        // correctly cited and about the wrong company, which is the most expensive wrong
        // answer available.
        assert_eq!(
            origin_in("an app that helps small farms sell to restaurants"),
            None
        );
        assert_eq!(origin_in("a tool that chases unpaid invoices"), None);
    }

    #[test]
    fn an_ordinary_sentence_with_a_full_stop_is_not_a_domain() {
        assert_eq!(origin_in("we ship weekly.and we are proud of it"), None);
        assert_eq!(origin_in("version 1.2 of the thing"), None);
    }

    #[test]
    fn an_email_address_is_not_a_site_to_read() {
        assert_eq!(origin_in("write to hello@basecamp.com"), None);
    }

    #[test]
    fn a_scheme_we_do_not_fetch_is_refused() {
        assert_eq!(origin_in("ftp://files.example.com"), None);
        assert_eq!(origin_in("mailto:hello@example.com"), None);
    }

    #[test]
    fn the_first_site_named_is_the_subject() {
        // One subject per analysis. A prompt naming two is a comparison, which is a
        // different product surface and not something to guess at here.
        assert_eq!(
            origin_in("basecamp.com vs linear.app").as_deref(),
            Some("https://basecamp.com")
        );
    }

    #[test]
    fn nothing_in_an_empty_prompt() {
        assert_eq!(origin_in(""), None);
        assert_eq!(origin_in("   "), None);
    }
}
