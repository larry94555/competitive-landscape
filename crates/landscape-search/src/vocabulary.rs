//! What the market calls the thing a reader described.
//!
//! `COMPETITIVE_DISCOVERY.md` §4: *the user's words are not the market's words*. Somebody types
//! **a free competitive landscape research tool**. Nobody sells that. The market says
//! *competitive intelligence software*, and searching the reader's phrasing finds blog posts
//! where searching the market's finds the market.
//!
//! ```text
//! "a free competitive landscape research tool"
//!    └─> the queries candidates::for_idea already sends
//!         └─> phrases_in       every 2..=4 word run in each result's title
//!              └─> from_titles counted once per independent host, not per page
//!                   └─> Market  "competitive intelligence software"
//! ```
//!
//! # Deterministic, and no model
//!
//! §4 calls this *"almost entirely deterministic"* and `COMPETITIVE_DISCOVERY.md` §9 budgets it
//! at *"3–5 searches; term frequency over titles — deterministic, no model"*. Nothing here
//! calls `landscape_llm`. That is not thrift: a category label decides every query downstream,
//! so a step that produced a different answer on a second run would make the whole report
//! irreproducible for reasons nobody could point at.
//!
//! # A title is not evidence, and this is not a claim
//!
//! [`crate::Hit::title`] says plainly that it is the engine's account of a page and *"not
//! evidence of anything"*, and that rule is intact. **Nothing this module returns can reach a
//! report as a fact about a company.** What it produces is a phrase to search with — the same
//! kind of thing as a template — and §4 requires that phrase be shown to the reader as an
//! interpretation they can overrule, precisely because it was inferred rather than read.
//!
//! The distinction is worth being exact about: a claim is about *one* company and must come
//! from that company's own page. A category label is about *the words a market uses*, and the
//! only way to observe those is across many strangers' pages at once. Counting them is not
//! trusting any one of them.
//!
//! # Review sites are the best source here, and the worst source next door
//!
//! [`crate::candidates`] excludes `g2.com` and `capterra.com` by name: they are places people
//! talk about companies, not companies. **This module deliberately does not exclude them.**
//! §4 step 5 calls their curated category trees *"the strongest signal available"* — they
//! **are** the market's own vocabulary. One list, two opposite uses, and the reason they are
//! opposite is that one question is *who is a company* and the other is *what is this called*.

use std::collections::{HashMap, HashSet};

use crate::candidates::{registrable, Queried, IDEA_TEMPLATES};
use crate::competitors::NOT_CONTENT;
use crate::provider::{Hit, SourceProvider};

/// How many independent hosts must use a phrase before it is the market's word for anything.
///
/// **Two, and it is its own number rather than [`crate::candidates::CORROBORATION`]'s.** That
/// one counts *queries* agreeing about a host; this counts *hosts* agreeing about a phrase.
/// They are the same value today by coincidence of both being the smallest number that is not
/// one, and tying them together would mean a change to how many searches we send silently
/// changing what counts as a category.
pub const INDEPENDENT_HOSTS: usize = 2;

/// The shortest phrase worth calling a category. One word is a topic, not a market.
pub const SHORTEST: usize = 2;

/// The longest. §4's *"battlecard automation platform is too narrow"* is the failure this
/// bounds; past four words a title has stopped naming a category and started describing a
/// product.
pub const LONGEST: usize = 4;

/// How many other phrasings a reader is shown beside the label.
///
/// They are shown and **not searched**. A synonym list that fed the queries would multiply the
/// round trips by its own length for phrasings we have less evidence for than the one we chose.
pub const ALSO: usize = 3;

/// Listicle furniture, trimmed from the ends of a phrase and never from the middle.
///
/// *Best competitive intelligence software* and *competitive intelligence software reviews* are
/// the same category wearing a headline. **Only the ends**, because a word here can be a real
/// part of a name in the middle of one — and nothing in this list is a word a category is
/// plausibly built from, which is the test for adding one.
const FURNITURE: [&str; 16] = [
    "best",
    "top",
    "free",
    "cheap",
    "popular",
    "leading",
    "compare",
    "comparison",
    "review",
    "reviews",
    "pricing",
    "alternatives",
    "alternative",
    "guide",
    "list",
    "vs",
];

/// One phrase, and how many independent hosts used it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    /// Lowercased, single-spaced, [`SHORTEST`]`..=`[`LONGEST`] words.
    pub words: String,
    /// Distinct registrable domains whose titles contained it. **Not pages** — see
    /// [`from_titles`].
    pub hosts: usize,
}

/// What a market calls itself, in the words its own pages use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Market {
    /// The phrase every downstream query should be built from.
    pub label: String,
    /// Other phrasings that cleared the floor. Shown to a reader; never searched.
    pub also: Vec<String>,
    /// How many independent hosts used [`Self::label`].
    pub hosts: usize,
}

/// What came of asking what the market calls something.
///
/// **Four outcomes, because a reader acts on each differently** — the same rule
/// `landscape_core::Failure` is built on, and for the same reason: *"we have no way to look"*,
/// *"we did not finish looking"* and *"we looked and the market has no word for this"* are three
/// different facts, and one sentence covering all three is a sentence nobody can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// The market has a name for it.
    Market(Market),
    /// Two or more **unrelated** categories, corroborated by exactly as many sites as each other.
    ///
    /// **The gate the resolver owed.** Two hosts saying *email marketing software* and two
    /// saying *project management software* used to come back as the first of those, chosen by
    /// nothing but which sorted earlier — `landscape_core::subject::AMBIGUITY_MARGIN`'s words
    /// for it are exact: *"picking the top one is a coin flip presented to the reader as a
    /// fact"*. And this one is worse than picking the wrong company, because the label decides
    /// every query downstream: the whole report would be about a market nobody asked for.
    ///
    /// **Unrelated** means neither phrase contains the other. *competitive intelligence* and
    /// *competitive intelligence software* are one market described at two widths, and choosing
    /// the wider is [`most_specific`]'s job rather than a reader's.
    Ambiguous {
        /// The competing categories, each already reduced to its most specific phrasing.
        between: Vec<Market>,
    },
    /// Titles were read and nothing recurred across [`INDEPENDENT_HOSTS`] of them.
    ///
    /// **A finding, not a failure.** Some ideas genuinely have no category yet, and the honest
    /// response is to search the reader's own words — which is what happened before this module
    /// existed, so nothing is lost. The counts are here so the negative is checkable.
    TheirWords {
        /// Titles examined.
        titles: usize,
        /// Distinct hosts they came from.
        hosts: usize,
    },
    /// Queries did not come back, and nothing cleared the floor without them.
    ///
    /// Distinct from [`Self::TheirWords`] for the reason `decide` keeps an outage apart from an
    /// empty market: one is fixed by waiting and the other is not.
    Incomplete {
        /// Queries that did not complete.
        failed: usize,
        /// Queries sent.
        sent: usize,
    },
    /// No engine is configured. The laptop default, and not an error.
    NoEngine,
}

/// Every phrase a title contains that could name a category.
///
/// **Runs of letters, broken by everything else.** A digit, a pipe, a colon or a bullet ends a
/// run, so `Top 10 CRM Software | G2` yields `crm software` and never `top crm` — the number
/// that separates the headline from the category does the separating. A hyphen **between two
/// letters** is a space rather than a break, so `privacy-friendly analytics` stays one run while
/// `Klue - Competitive Intelligence` does not: a spaced dash is a title's separator and a tight
/// one is part of a word.
///
/// Grammar words break a run too — the list is [`NOT_CONTENT`], shared with
/// [`crate::competitors::content_words`] rather than copied, so *competitive intelligence
/// software for product marketing* yields `competitive intelligence software` and
/// `product marketing` instead of the nonsense phrase that spans the `for`.
#[must_use]
pub fn phrases_in(title: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for run in runs_of(title) {
        for window in SHORTEST..=LONGEST {
            for start in 0..run.len().saturating_sub(window - 1) {
                let Some(phrase) = trimmed(&run[start..start + window]) else {
                    continue;
                };
                if !out.contains(&phrase) {
                    out.push(phrase);
                }
            }
        }
    }
    out
}

/// The word runs of a title, already split at numbers and at grammar words.
///
/// **A number on its own is a separator; a number inside a word is part of it.** Review found
/// the difference: breaking on every digit turned `B2B Marketing Automation Software` into
/// `b marketing automation software` and `3D Modeling Software` into `d modeling software` —
/// corrupted phrases that two hosts using either title would have promoted to a market label.
/// The `10` in `Top 10 CRM Software` still separates the headline from the category, because it
/// is a whole word made of digits and `b2b` is not.
fn runs_of(title: &str) -> Vec<Vec<String>> {
    let mut runs: Vec<Vec<String>> = Vec::new();
    let mut run: Vec<String> = Vec::new();
    let mut word = String::new();
    let chars: Vec<char> = title.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else if c.is_whitespace() || tight_hyphen(&chars, i) {
            // A space separates two words of one phrase; a **tight** hyphen is the same thing
            // spelled differently. `Klue - Competitive Intelligence` reaches neither arm,
            // because that hyphen has a space on each side and is the title's own punctuation.
            close_word(&mut word, &mut run, &mut runs);
        } else {
            close_word(&mut word, &mut run, &mut runs);
            close_run(&mut run, &mut runs);
        }
    }
    close_word(&mut word, &mut run, &mut runs);
    close_run(&mut run, &mut runs);
    runs
}

/// Whether this `-` joins two characters rather than separating two words.
fn tight_hyphen(chars: &[char], i: usize) -> bool {
    chars[i] == '-'
        && i > 0
        && chars[i - 1].is_alphanumeric()
        && chars.get(i + 1).is_some_and(char::is_ascii_alphanumeric)
}

/// End the current word. A bare number and a grammar word both end the *run* instead.
fn close_word(word: &mut String, run: &mut Vec<String>, runs: &mut Vec<Vec<String>>) {
    if word.is_empty() {
        return;
    }
    let finished = std::mem::take(word);
    if finished.chars().all(|c| c.is_numeric()) || NOT_CONTENT.contains(&finished.as_str()) {
        close_run(run, runs);
    } else {
        run.push(finished);
    }
}

/// Keep a run if it is long enough to hold a phrase, and start a new one either way.
fn close_run(run: &mut Vec<String>, runs: &mut Vec<Vec<String>>) {
    if run.len() >= SHORTEST {
        runs.push(std::mem::take(run));
    } else {
        run.clear();
    }
}

/// One window of a run as a phrase, or nothing if what is left is not one.
///
/// Furniture comes off both ends, then the result has to still be long enough — and it has to
/// say something we did not say ourselves. `IDEA_TEMPLATES` is read rather than retyped, so a
/// template gaining a word does not quietly let that word become an answer.
fn trimmed(window: &[String]) -> Option<String> {
    let mut words: &[String] = window;
    while words
        .first()
        .is_some_and(|w| FURNITURE.contains(&w.as_str()))
    {
        words = &words[1..];
    }
    while words
        .last()
        .is_some_and(|w| FURNITURE.contains(&w.as_str()))
    {
        words = &words[..words.len() - 1];
    }
    if words.len() < SHORTEST {
        return None;
    }
    // **Our own boilerplate is not the market's word for anything.** We search `best {} software`
    // and `{} vendors`; a title reading *"Software Vendors"* is our query looking back at us.
    if words.iter().all(|w| ours().contains(w.as_str())) {
        return None;
    }
    Some(words.join(" "))
}

/// The words this codebase puts into a query itself, from the templates that put them there.
fn ours() -> HashSet<&'static str> {
    IDEA_TEMPLATES
        .iter()
        .flat_map(|t| t.split(|c: char| !c.is_alphabetic()))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Count the phrases in a result set and pick the market's word for it.
///
/// **Once per host, never per page.** `FACT_CHECKING.md` §6 is explicit that a term on twelve
/// pages from one publisher counts once, and it is the whole reason this number means anything:
/// a single content farm with a template can put the same phrase in forty titles, and forty is
/// not agreement.
///
/// The arms below are ordered as [`landscape_analyze::subject::decide`]'s are, and for the same
/// reason: **an outage that produced no answer is not a market with no word.** A phrase that
/// cleared the floor still beats an outage, because an answer already in hand is worth more than
/// telling somebody to come back for it.
///
/// [`landscape_analyze::subject::decide`]: https://docs.rs/landscape-analyze
#[must_use]
pub fn from_titles(results: &[Vec<Hit>], queried: &Queried) -> Resolved {
    let mut by_phrase: HashMap<String, HashSet<String>> = HashMap::new();
    let mut titles = 0usize;
    let mut hosts: HashSet<String> = HashSet::new();
    for hits in results {
        for hit in hits {
            let Ok(target) = landscape_fetch::Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            titles += 1;
            hosts.insert(host.clone());
            for phrase in phrases_in(&hit.title) {
                by_phrase.entry(phrase).or_default().insert(host.clone());
            }
        }
    }

    let mut counted: Vec<Phrase> = by_phrase
        .into_iter()
        .map(|(words, seen)| Phrase {
            words,
            hosts: seen.len(),
        })
        .filter(|p| p.hosts >= INDEPENDENT_HOSTS)
        .collect();
    // Most agreed first; then the shorter phrase, so the anchor below is the broadest thing that
    // recurs rather than whichever extension of it the map happened to yield first; then by text,
    // so two runs of one input cannot disagree.
    counted.sort_by(|a, b| {
        b.hosts
            .cmp(&a.hosts)
            .then_with(|| a.words.split(' ').count().cmp(&b.words.split(' ').count()))
            .then_with(|| a.words.cmp(&b.words))
    });

    let found = clusters(&counted);
    match found.split_first() {
        None if !queried.failed.is_empty() => Resolved::Incomplete {
            failed: queried.failed.len(),
            sent: queried.sent(),
        },
        None => Resolved::TheirWords {
            titles,
            hosts: hosts.len(),
        },
        Some((best, rest)) => {
            // **Equal corroboration, and nothing else to choose on.** A fractional margin is the
            // wrong instrument here: these are counts of two to five, where 15% is either zero
            // or everything. Two independent sites each is a tie, and a tie is a question.
            let tied: Vec<Market> = rest
                .iter()
                .filter(|m| m.hosts == best.hosts)
                .cloned()
                .collect();
            if tied.is_empty() {
                Resolved::Market(best.clone())
            } else {
                Resolved::Ambiguous {
                    between: std::iter::once(best.clone()).chain(tied).collect(),
                }
            }
        }
    }
}

/// The categories the titles describe, one entry per market rather than one per phrase.
///
/// **Grouped by containment, which is the relation [`most_specific`] already uses.** Every phrase
/// that contains another, or is contained by it, belongs to the same market: *email marketing*,
/// *marketing software* and *email marketing software* are one thing said three ways. *Project
/// management software* is not, and that is what makes a tie between them a question rather than
/// a sort order.
///
/// Transitive on purpose. *marketing software* links to *email marketing software* and nothing
/// else; without transitivity it would become a third "market" that is a fragment of the first,
/// and three tied clusters where there are two real ones.
fn clusters(counted: &[Phrase]) -> Vec<Market> {
    let mut group: Vec<usize> = (0..counted.len()).collect();
    fn root(group: &mut [usize], mut i: usize) -> usize {
        while group[i] != i {
            group[i] = group[group[i]];
            i = group[i];
        }
        i
    }
    for a in 0..counted.len() {
        for b in (a + 1)..counted.len() {
            if !related(&counted[a], &counted[b]) {
                continue;
            }
            let (ra, rb) = (root(&mut group, a), root(&mut group, b));
            if ra != rb {
                group[rb] = ra;
            }
        }
    }

    // `counted` is already sorted best-first, so walking it in order puts the strongest market
    // first and keeps every tie broken the same way on a second run.
    let mut out: Vec<Market> = Vec::new();
    let mut seen: Vec<usize> = Vec::new();
    for i in 0..counted.len() {
        let r = root(&mut group, i);
        if seen.contains(&r) {
            continue;
        }
        seen.push(r);
        let members: Vec<Phrase> = (0..counted.len())
            .filter(|&j| root(&mut group, j) == r)
            .map(|j| counted[j].clone())
            .collect();
        out.push(most_specific(&counted[i], &members));
    }
    out
}

/// Whether one phrase is the other said at a different width.
fn related(a: &Phrase, b: &Phrase) -> bool {
    contains(&a.words, &b.words) || contains(&b.words, &a.words)
}

/// Whether `outer` holds `inner` as a whole run of words.
fn contains(outer: &str, inner: &str) -> bool {
    outer == inner
        || outer.starts_with(&format!("{inner} "))
        || outer.ends_with(&format!(" {inner}"))
        || outer.contains(&format!(" {inner} "))
}

/// §4 step 4: **the most specific term that still recurs widely.**
///
/// The anchor is the phrase the most hosts agreed on — the broadest thing the market really
/// says. The label is the **longest** phrase that contains it and still clears the floor, which
/// is what makes *competitive intelligence software* win over *competitive intelligence* without
/// letting *battlecard automation platform* win over either: a narrower phrase only counts if it
/// is an extension of what everybody already agreed on.
///
/// **A ratio would have been the obvious alternative and is worse.** "Recurs widely" as a
/// fraction of the top count needs a number nobody can defend, and it would move every time the
/// query count moved. Containment needs none.
fn most_specific(anchor: &Phrase, cluster: &[Phrase]) -> Market {
    let label = cluster
        .iter()
        .filter(|p| contains(&p.words, &anchor.words))
        .max_by(|a, b| {
            a.words
                .split(' ')
                .count()
                .cmp(&b.words.split(' ').count())
                .then_with(|| a.hosts.cmp(&b.hosts))
                .then_with(|| b.words.cmp(&a.words))
        })
        .unwrap_or(anchor);
    Market {
        label: label.words.clone(),
        also: cluster
            .iter()
            .filter(|p| p.words != label.words)
            .take(ALSO)
            .map(|p| p.words.clone())
            .collect(),
        hosts: label.hosts,
    }
}

/// Ask, read the titles, and say what the market calls it.
///
/// **The queries are [`crate::candidates::for_idea`]'s, unchanged.** §4 asks for 3–5 searches on
/// the reader's phrasing, and those are exactly the searches an analysis already sends to find
/// companies — so once this is wired into the worker it costs **zero** extra round trips, and
/// the diagnostic below sees precisely what the run would see rather than an approximation of it.
///
/// **The missing engine is decided here rather than by each caller**, because there will be two
/// of them — the diagnostic below and the worker — and two places deciding what *"no engine"*
/// means is how the same run came to be described differently on two surfaces before
/// ([`crate::competitors::alone_because`] exists for that reason). A caller holding
/// `Option<&dyn SourceProvider>` hands it over as it is.
///
/// # Errors
/// Never. A query that fails is counted, and [`from_titles`] tells an outage apart from a market
/// with no word for itself.
pub async fn resolve(
    engine: Option<&dyn SourceProvider>,
    description: &str,
) -> (Resolved, Queried) {
    let Some(engine) = engine else {
        return (Resolved::NoEngine, Queried::default());
    };
    let queries = crate::candidates::for_idea(description);
    let mut results: Vec<Vec<Hit>> = Vec::with_capacity(queries.len());
    let mut queried = Queried::default();
    for query in &queries {
        match engine.search(query).await {
            Ok(hits) => {
                results.push(hits);
                queried.completed.push(query.text.clone());
            }
            Err(e) => {
                tracing::warn!(query = %query.text, error = %e, "a vocabulary query did not complete");
                queried.failed.push(query.text.clone());
            }
        }
    }
    (from_titles(&results, &queried), queried)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn hit(url: &str, title: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: title.to_owned(),
            snippet: String::new(),
        }
    }

    fn asked(n: usize) -> Queried {
        Queried {
            completed: (0..n).map(|i| format!("q{i}")).collect(),
            failed: Vec::new(),
        }
    }

    /// A plausible first page for *"a free competitive landscape research tool"*.
    ///
    /// Five hosts, and the market's phrase is on three of them in two lengths — which is the
    /// case §4 is written about and the only one worth a fixture.
    fn a_market() -> Vec<Vec<Hit>> {
        vec![vec![
            hit(
                "https://www.g2.com/categories/competitive-intelligence",
                "Best Competitive Intelligence Software 2025 | G2",
            ),
            hit(
                "https://www.capterra.com/competitive-intelligence-software/",
                "Competitive Intelligence Software - Capterra",
            ),
            hit(
                "https://klue.com/",
                "Klue | Competitive Intelligence Platform",
            ),
            hit(
                "https://www.crayon.co/product",
                "Competitive Intelligence Software for Product Marketing",
            ),
            hit(
                "https://someblog.example/posts/9",
                "Top 10 Competitor Analysis Tools",
            ),
        ]]
    }

    #[test]
    fn a_tight_hyphen_is_a_word_and_a_spaced_one_is_a_separator() {
        // Both are `-`. `privacy-friendly analytics` is one phrase somebody would search for;
        // `Klue - Competitive Intelligence` is a company and a category that have nothing to do
        // with each other, joined by a title's punctuation.
        assert!(phrases_in("privacy-friendly analytics").contains(&"privacy friendly".to_owned()));
        let split = phrases_in("Klue - Competitive Intelligence");
        assert!(split.contains(&"competitive intelligence".to_owned()));
        assert!(
            !split.iter().any(|p| p.starts_with("klue ")),
            "a company name glued onto a category: {split:?}"
        );
    }

    #[test]
    fn a_grammar_word_breaks_a_phrase_rather_than_sitting_inside_one() {
        // `... software for product marketing` holds two phrases and no phrase that spans the
        // `for`. The list is `content_words`'s, so the two cannot drift apart.
        let found = phrases_in("Competitive Intelligence Software for Product Marketing");
        assert!(found.contains(&"competitive intelligence software".to_owned()));
        assert!(found.contains(&"product marketing".to_owned()));
        // **The property, not a guess at what the symptom would look like.** This asserted that
        // no phrase contained `software product`, and a mutation deleting the break produced
        // `software for product` — which is the defect, spelled the other way, and the test
        // could not fail. No phrase may contain a grammar word *at all*; that is the rule, and
        // it does not depend on which words happen to be on either side of one.
        assert!(
            !found
                .iter()
                .any(|p| p.split(' ').any(|w| NOT_CONTENT.contains(&w))),
            "a phrase spanned a grammar word: {found:?}"
        );
    }

    #[test]
    fn a_headline_is_trimmed_to_the_category_inside_it() {
        let found = phrases_in("Best Competitive Intelligence Software Reviews");
        assert!(found.contains(&"competitive intelligence software".to_owned()));
        assert!(
            !found
                .iter()
                .any(|p| p.starts_with("best ") || p.ends_with(" reviews")),
            "furniture survived the trim: {found:?}"
        );
    }

    #[test]
    fn our_own_query_words_cannot_become_the_answer() {
        // We send `best {} software` and `{} vendors`. A title of *"Software Vendors"* is this
        // codebase's own phrasing coming back, and counting it would let a template vote.
        assert!(
            phrases_in("Software Vendors").is_empty(),
            "our own boilerplate was offered as a category"
        );
        assert!(
            phrases_in("Competitive Intelligence Software")
                .contains(&"competitive intelligence software".to_owned()),
            "a real category containing one of our words was dropped with the boilerplate"
        );
    }

    #[test]
    fn the_market_word_is_the_most_specific_one_that_still_recurs() {
        // §4 step 4. `competitive intelligence` is on four hosts and `competitive intelligence
        // software` on three; the longer one wins **because it extends the broader one**, which
        // is what keeps `battlecard automation platform` from winning on two.
        let Resolved::Market(market) = from_titles(&a_market(), &asked(3)) else {
            panic!("a market with a name resolved to something else")
        };
        assert_eq!(market.label, "competitive intelligence software");
        assert_eq!(market.hosts, 3);
        assert!(
            market.also.contains(&"competitive intelligence".to_owned()),
            "the broader phrasing is not offered beside it: {:?}",
            market.also
        );
    }

    #[test]
    fn one_host_saying_a_thing_forty_times_has_said_it_once() {
        // `FACT_CHECKING.md` §6. A content farm with a title template is the whole reason this
        // is counted per host, and a page count would make it the loudest voice in the market.
        let farm: Vec<Vec<Hit>> = vec![(0..40)
            .map(|i| {
                hit(
                    &format!("https://farm.example/posts/{i}"),
                    "Battlecard Automation Platform",
                )
            })
            .collect()];
        assert!(
            matches!(from_titles(&farm, &asked(3)), Resolved::TheirWords { .. }),
            "forty pages from one host became the market's word"
        );
    }

    #[test]
    fn a_market_with_no_word_for_itself_is_a_finding_and_says_what_was_checked() {
        let scattered = vec![vec![
            hit("https://a.example/", "Alpha Widgets"),
            hit("https://b.example/", "Beta Gadgets"),
        ]];
        let Resolved::TheirWords { titles, hosts } = from_titles(&scattered, &asked(3)) else {
            panic!("nothing recurring resolved to a market")
        };
        assert_eq!((titles, hosts), (2, 2), "the negative is not checkable");
    }

    #[test]
    fn an_outage_is_not_a_market_with_no_word_for_itself() {
        // The same rule `decide` keeps: a reader fixes one of these by waiting and cannot fix
        // the other at all, so they must not arrive as one sentence.
        let queried = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec!["q2".to_owned(), "q3".to_owned()],
        };
        assert_eq!(
            from_titles(&[], &queried),
            Resolved::Incomplete { failed: 2, sent: 3 }
        );
    }

    #[test]
    fn an_answer_we_already_have_still_beats_an_outage() {
        // The other half of the arm above, so the precedence is a rule rather than an accident
        // of which `match` arm was written first.
        let queried = Queried {
            completed: vec!["q1".to_owned()],
            failed: vec!["q2".to_owned()],
        };
        assert!(
            matches!(from_titles(&a_market(), &queried), Resolved::Market(_)),
            "an answer in hand was thrown away because a query timed out"
        );
    }

    #[test]
    fn review_sites_count_here_and_are_excluded_next_door() {
        // The inversion this module rests on: `candidates` refuses g2.com because it is not a
        // company, and §4 step 5 calls its category tree the strongest signal there is. If this
        // ever starts sharing that exclusion list, the label loses its two best sources.
        let Resolved::Market(market) = from_titles(&a_market(), &asked(3)) else {
            panic!("no market")
        };
        assert_eq!(
            market.hosts, 3,
            "g2 and capterra stopped counting toward the market's own words"
        );
    }

    #[tokio::test]
    async fn no_engine_is_answered_without_asking_anybody() {
        // One place decides what "no engine" is, so the diagnostic and the worker cannot come
        // to different words for the same laptop. And nothing is sent: a `Queried` claiming
        // three completed searches would make a negative look checked when nothing was checked.
        let (resolved, queried) = resolve(None, "privacy friendly website analytics").await;
        assert_eq!(resolved, Resolved::NoEngine);
        assert_eq!(
            queried.sent(),
            0,
            "queries were counted with no engine to send them to"
        );
    }

    #[test]
    fn a_digit_inside_a_category_is_part_of_it() {
        // **Review found this.** Breaking on every digit turned `B2B` into `b` and `3D` into
        // `d`, and two hosts using either title would have promoted the corrupted phrase to a
        // market label — every query in the report built from a word that does not exist.
        let b2b = phrases_in("B2B Marketing Automation Software");
        assert!(
            b2b.contains(&"b2b marketing automation software".to_owned()),
            "a category with a digit in it came out mangled: {b2b:?}"
        );
        assert!(
            !b2b.iter().any(|p| p.starts_with("b ")),
            "a digit was cut out of a word: {b2b:?}"
        );

        let threed = phrases_in("3D Modeling Software");
        assert!(
            threed.contains(&"3d modeling software".to_owned()),
            "a category starting with a digit came out mangled: {threed:?}"
        );
        assert!(!threed.iter().any(|p| p.starts_with("d ")), "{threed:?}");
    }

    #[test]
    fn a_number_on_its_own_still_separates_a_headline_from_a_category() {
        // The other half of the rule above, and the half that came first. `10` is a whole word
        // made of digits; `b2b` is not. Fixing one must not cost the other.
        let found = phrases_in("Top 10 CRM Software");
        assert!(found.contains(&"crm software".to_owned()), "{found:?}");
        assert!(
            !found.iter().any(|p| p.contains("top")),
            "a headline word crossed a bare number: {found:?}"
        );
        // **The rule, not one spelling of the failure.** `!p.contains("top")` passed with the
        // break deleted, because the furniture trim removes `top` anyway - and left `10 crm
        // software` behind, which is the actual defect. No phrase may contain a bare number.
        assert!(
            !found
                .iter()
                .any(|p| p.split(' ').any(|w| w.chars().all(char::is_numeric))),
            "a bare number ended up inside a phrase: {found:?}"
        );
    }

    #[test]
    fn two_unrelated_markets_with_the_same_backing_are_not_chosen_between() {
        // **Review found this too, and it is the worse of the pair.** Two hosts say one thing
        // and two say another; the resolver returned the first alphabetically, and the label
        // decides *every query downstream* — so the whole report would have been about a market
        // nobody asked for, picked by a sort order.
        let split = vec![vec![
            hit("https://a.example/", "Email Marketing Software"),
            hit("https://b.example/", "Email Marketing Software"),
            hit("https://c.example/", "Project Management Software"),
            hit("https://d.example/", "Project Management Software"),
        ]];
        let Resolved::Ambiguous { between } = from_titles(&split, &asked(3)) else {
            panic!(
                "two equally backed markets were chosen between: {:?}",
                from_titles(&split, &asked(3))
            )
        };
        let labels: Vec<&str> = between.iter().map(|m| m.label.as_str()).collect();
        assert!(
            labels.contains(&"email marketing software")
                && labels.contains(&"project management software"),
            "both markets have to be offered, not just the one that sorted first: {labels:?}"
        );
        assert!(
            between.iter().all(|m| m.hosts == 2),
            "the tie is the reason this is a question: {between:?}"
        );
    }

    #[test]
    fn one_market_described_at_two_widths_is_not_a_tie() {
        // The other side, so the gate above cannot fire on every input. `competitive
        // intelligence` and `competitive intelligence software` are one market said two ways,
        // and choosing the wider is `most_specific`'s job rather than a reader's.
        assert!(
            matches!(from_titles(&a_market(), &asked(3)), Resolved::Market(_)),
            "one market at two widths was reported as a choice a reader has to make"
        );
    }

    #[test]
    fn a_fragment_of_one_market_is_not_a_second_market() {
        // `marketing software` is inside `email marketing software` and nothing else, so it
        // belongs to that cluster. Without transitivity it would be a third "market" made of a
        // piece of the first, and a tie between three things where there are two.
        let split = vec![vec![
            hit("https://a.example/", "Email Marketing Software"),
            hit("https://b.example/", "Email Marketing Software"),
            hit("https://c.example/", "Project Management Software"),
            hit("https://d.example/", "Project Management Software"),
        ]];
        let Resolved::Ambiguous { between } = from_titles(&split, &asked(3)) else {
            panic!("not ambiguous")
        };
        assert_eq!(
            between.len(),
            2,
            "a fragment became a market of its own: {between:?}"
        );
    }

    #[test]
    fn two_runs_of_one_input_agree() {
        // A label that reorders between runs is one nobody can reproduce, and every query
        // downstream is built from it.
        let first = from_titles(&a_market(), &asked(3));
        for _ in 0..5 {
            assert_eq!(from_titles(&a_market(), &asked(3)), first);
        }
    }
}
