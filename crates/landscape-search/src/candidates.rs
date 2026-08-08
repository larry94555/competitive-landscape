//! Turning a description into the companies it might be about.
//!
//! `FACT_CHECKING.md` §3.1 puts this before everything: *"Nothing is fetched until the subject
//! is pinned down, because every downstream error inherits from getting this wrong."* Step 4 of
//! that list — the disambiguation gate — has existed in [`landscape_core::subject`] since Phase
//! 1, deliberately built before the thing that feeds it. **This is step 2, and it is what the
//! gate has been waiting for.**
//!
//! ```text
//! "privacy-friendly website analytics"
//!    └─> queries::for_idea        the reader's own words, once, and only here
//!         └─> SourceProvider      the same seam per-question search uses
//!              └─> group by host  one host is one company
//!                   └─> score     arithmetic over URLs, never a model's opinion
//!                        └─> describe  each company's own front page names it
//!                             ├─> landscape_core::subject::resolve   may a report be written
//!                             └─> crate::competitors::assemble       what goes in it
//! ```
//!
//! **The last two are different questions and both are asked.** The gate decides whether we
//! know enough to write anything; the set decides which companies a landscape compares. See
//! [`crate::competitors`] for why answering the first one alone produced *"which of these three
//! did you mean?"* for every market description anybody typed.
//!
//! # The one place a reader's phrasing may reach an engine
//!
//! `FACT_CHECKING.md` P22 is explicit that retrieval uses **"templated queries from resolved
//! entities, not user phrasing"** — because a query carrying somebody's framing returns pages
//! that agree with it, and a report built on those is a mirror.
//!
//! That rule cannot apply here and the reason is not an exception to it: **there is no resolved
//! entity yet.** Finding one is what this module is for, and the description is the only thing
//! anybody has said. So the words go to an engine exactly once, to produce a *set of companies*
//! — never to produce a fact — and everything downstream is templated from the domain the gate
//! resolves to. The asymmetry is the point: a biased query can put the wrong company on a list
//! a reader then chooses from, and a reader can see that and pick differently. A biased query
//! against a resolved company puts a biased *fact* in a report, and nobody can see it.
//!
//! # Why the score is arithmetic and not a judgement
//!
//! [`landscape_core::subject::AMBIGUITY_MARGIN`] compares two scores and decides whether to ask
//! a reader. A score somebody cannot explain makes that decision unaccountable, so every input
//! to it here is a property of a **URL**:
//!
//! | Signal | Why it is evidence |
//! |---|---|
//! | How many of the queries returned this host | Agreement across differently-worded questions is the closest thing to corroboration available before anything is read |
//! | How shallow its shallowest URL is | A company's front page is `/` or one level down. A page *about* a company is four levels into somebody's blog |
//!
//! **Nothing here reads a title or a snippet.** [`Hit`] carries both so a person running the
//! diagnostic can see what came back, and [`crate::admit::Found`] drops them so engine prose
//! cannot reach a report. The same discipline applies to a score: an engine's summary of a page
//! is the engine's, and letting it move a company up a list a reader chooses from is the same
//! laundering with a shorter supply chain.
//!
//! # What a candidate is not
//!
//! It is **not named yet**. [`Found`] carries a host and a score and nothing a reader could
//! choose between, because the name and the sentence that tells two companies apart have to
//! come from the company's own front page rather than from an engine's title. That fetch is
//! [`describe`], and it is a separate step so the arithmetic above stays pure and testable.

use std::collections::HashMap;

use landscape_core::subject::Candidate;
use landscape_fetch::Target;

use crate::provider::{Hit, SourceProvider};
use crate::queries::Query;

/// The version of the idea query set, stamped on a run.
///
/// Separate from [`crate::QUERY_SET`] because they answer different questions and change for
/// different reasons: that one asks *what does this company publish about pricing*, this one
/// asks *which companies is this description about*. A single version covering both would move
/// for edits that could not affect the other, which is a version nobody can reason from.
pub const IDEA_QUERY_SET: &str = "2026-08-07.1";

/// How many differently-worded queries have to agree before a candidate can be chosen for a
/// reader rather than offered to one.
///
/// **Two, and the number is the whole argument of this module.** A host one query returned has
/// agreed with nothing; the score is built on agreement, so a candidate with none of it has no
/// score worth comparing. Review found what that cost: three queries sent, two failing, and the
/// single hit from the third scored `0.8 × 1/3 + 0.2 = 0.47` — above
/// [`landscape_core::subject::MINIMUM_CONFIDENCE`], so the gate resolved it and an analysis
/// would have run against a company that appeared in one search.
pub const CORROBORATION: usize = 2;

/// How many candidates get their front page fetched — **and therefore how many a reader can be
/// asked to choose between.**
///
/// Every fetch is a request against somebody's server before a reader has asked for anything, so
/// the number is small and stated. [`describe`] works in score order, so the strongest
/// candidates are the ones that get named.
///
/// **Two jobs, and they are the same set rather than a coincidence.** A candidate whose front
/// page we never read has no name of its own — only a domain — and *"which of these six did you
/// mean: c5.example (c5.example)?"* is not a question anybody can answer. So the list the
/// disambiguation gate sees is exactly the list that got fetched, and `PRODUCT_SPEC.md` §3's
/// *at most three chips* stays bounded by something other than how many results a provider felt
/// like returning.
///
/// **It is a budget and not a truncation.** It used to be `MAX_CANDIDATES` and [`from_results`]
/// applied it to the whole list, so a sixth corroborated company vanished before anything could
/// say it existed — the silent drop [`crate::competitors`] was written to remove. Removing it
/// there then handed all six to the gate, which was the same mistake pointing the other way.
/// The list is complete for [`crate::competitors::assemble`], bounded here for the gate, and
/// what the bound costs is a [`crate::competitors::Aside::BeyondTheFetchBudget`] a reader can
/// read.
pub const NAMED: usize = 5;

/// Hosts that are pages *about* a market rather than companies in it.
///
/// **A closed list, and the same shape as the compliance standards the trust extractor reads:**
/// naming them is a decision about what this product knows, so it is here and reviewable rather
/// than a regular expression somewhere. `FACT_CHECKING.md` §3.2 puts listicles and forums at the
/// bottom of both axes, and P15 names *"alternatives"* content as the worst; the point here is
/// narrower than trust, though. A G2 category page is often an *excellent* way to find out which
/// companies exist. It is simply not one of them, and a report about `g2.com` is not the report
/// anybody asked for.
///
/// Matched on the registrable suffix, so `blog.medium.com` is excluded with `medium.com`.
const NOT_A_COMPANY: [&str; 24] = [
    "g2.com",
    "capterra.com",
    "getapp.com",
    "softwareadvice.com",
    "trustradius.com",
    "producthunt.com",
    "alternativeto.net",
    "slant.co",
    "sourceforge.net",
    "reddit.com",
    "news.ycombinator.com",
    "quora.com",
    "stackoverflow.com",
    "medium.com",
    "substack.com",
    "wordpress.com",
    "blogspot.com",
    "youtube.com",
    "vimeo.com",
    "linkedin.com",
    "facebook.com",
    "twitter.com",
    "x.com",
    "wikipedia.org",
];

/// Which queries reached an engine and came back, and which did not.
///
/// **Two rules pull in opposite directions here and both are right.** The score divides by the
/// queries *sent*, because an engine that answered once out of three has not produced unanimity.
/// The audit trail lists the queries that *completed*, because `FACT_CHECKING.md` §5.4 says a
/// negative nobody can check is not a finding — and a query that never ran is not a check.
///
/// Review found what happened when only the first rule existed: three failed searches produced
/// *"we searched and found none"*, with all three queries listed as evidence of the looking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Queried {
    /// Asked and answered. This is the checkable half of a negative.
    pub completed: Vec<String>,
    /// Asked and did not come back. Not evidence of anything except an engine.
    pub failed: Vec<String>,
}

impl Queried {
    /// How many were sent — the divisor the score uses.
    #[must_use]
    pub fn sent(&self) -> usize {
        self.completed.len() + self.failed.len()
    }

    /// Whether anything at all came back.
    #[must_use]
    pub fn nothing_completed(&self) -> bool {
        self.completed.is_empty() && !self.failed.is_empty()
    }
}

/// One company a description might be about, before anybody has read its pages.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The registrable host, lowercased and without `www.`
    pub host: String,
    /// `0.0..=1.0`, from [`score`].
    pub confidence: f32,
    /// How many of the queries returned this host at all.
    pub agreed: usize,
    /// The shallowest URL seen on this host.
    ///
    /// **The depth signal, and not the fetch target.** Review found [`describe`] fetching this:
    /// the shallowest *search result* can still be `/pricing`, and a page's first heading then
    /// becomes the company's name — a reader offered *"Pricing"* as one of three companies to
    /// choose between. The home page is built from [`Self::host`] instead.
    pub shallowest: String,
}

/// A candidate after its own front page has been read.
///
/// **The page is read once and answers two questions**: what this company calls itself, and
/// which of the reader's words it uses. [`crate::competitors`] needs the second, and fetching
/// the same page again to get it would be a second request to somebody's server for text we
/// already have.
#[derive(Debug, Clone, PartialEq)]
pub struct Described {
    /// Name, domain and the one line that tells two companies apart.
    pub candidate: Candidate,
    /// How many of the differently-worded searches returned this host. Carried through from
    /// [`Found`] because a reader asking *why is this company here* is owed the number.
    pub agreed: usize,
    /// What its front page turned out to say, or why nobody knows.
    pub shares: Vocabulary,
}

impl Described {
    /// Whether we asked for its front page.
    ///
    /// **The line between a candidate a reader can be offered and a bare domain.** One has a
    /// name it gave itself and a sentence saying what it is; the other has a host. Review found
    /// the second kind reaching a disambiguation question as *"c5.example (c5.example)"*.
    ///
    /// It is a property of the candidate rather than an index compared against [`NAMED`],
    /// because the caller doing that arithmetic itself is how the two lists drift apart.
    #[must_use]
    pub fn was_requested(&self) -> bool {
        !matches!(self.shares, Vocabulary::NotRequested)
    }
}

/// What a company's front page turned out to say about the description — or why nobody knows.
///
/// **Three states, and an `Option` could only hold two.** It was `Option<Vec<String>>`, where
/// `None` meant *the page could not be read*; then review found a fourth company that was never
/// fetched at all, and there was nowhere to put it that was not a lie about one of the other
/// two. This is [`landscape_core::coverage`]'s rule — distinguishable silences — for one
/// company rather than one section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vocabulary {
    /// The front page was read. These are the description's words it uses, possibly none.
    ///
    /// An empty list here is a **finding**: we looked, and the page is about something else.
    Read(Vec<String>),
    /// The front page was requested and did not come back. About us, not about the company.
    Unreadable,
    /// The front page was never requested — this company ranked below [`NAMED`].
    ///
    /// Not a failure of anything. It is a budget being spent, and it is reported as one.
    NotRequested,
}

/// The queries a description produces.
///
/// Three, and each asks the same thing a different way, because the whole score below rests on
/// *agreement between differently-worded questions*. One query cannot agree with anything.
///
/// A blank description yields none: interpolating one sends bare boilerplate to an engine, which
/// returns the internet.
#[must_use]
pub fn for_idea(description: &str) -> Vec<Query> {
    /// Interpolated with the reader's words, and nothing else in this codebase does that.
    const TEMPLATES: [&str; 3] = [
        r#"best {} software"#,
        r#"{} tools comparison"#,
        r#"{} vendors"#,
    ];

    let cleaned = crate::queries::safe_words(description);
    if cleaned.is_empty() {
        return Vec::new();
    }
    TEMPLATES
        .into_iter()
        .map(|template| Query {
            text: template.replacen("{}", &cleaned, 1),
            // Every question, because a candidate is not about one section. The field is on
            // `Query` for the per-question path and carries no meaning here.
            answers: landscape_discover::probes::Answers::Identity,
            template,
        })
        .collect()
}

/// Ask, group and score. One round trip per query, and none if the description is blank.
///
/// # Errors
/// Never. A query that fails is counted and the rest carry on — a search that did not complete
/// is a thinner candidate list, and returning nothing because one engine call timed out would
/// turn a degraded answer into no answer.
pub async fn suggest(engine: &dyn SourceProvider, description: &str) -> (Vec<Found>, Queried) {
    let queries = for_idea(description);
    let mut results: Vec<Vec<Hit>> = Vec::with_capacity(queries.len());
    let mut queried = Queried::default();
    for query in &queries {
        match engine.search(query).await {
            Ok(hits) => {
                results.push(hits);
                queried.completed.push(query.text.clone());
            }
            Err(e) => {
                tracing::warn!(query = %query.text, error = %e, "a candidate search did not complete");
                queried.failed.push(query.text.clone());
            }
        }
    }
    // **Sent, not answered.** A host found by the one query that came back has agreed with
    // nothing, and dividing by the number that answered would call an outage unanimity.
    let found = from_results(&results, queried.sent());
    (found, queried)
}

/// The pure half: hits in, scored companies out.
///
/// `asked` is how many queries were *sent*, not how many answered — a host that appeared in the
/// only query that came back has agreed with nothing, and scoring it as unanimous would turn an
/// engine outage into a confident wrong answer.
#[must_use]
pub fn from_results(results: &[Vec<Hit>], asked: usize) -> Vec<Found> {
    let mut by_host: HashMap<String, (usize, String)> = HashMap::new();
    for hits in results {
        // Per query, not per hit: a listicle host returning five pages for one query has said
        // one thing, and counting five would let volume stand in for agreement.
        let mut seen_this_query: Vec<String> = Vec::new();
        for hit in hits {
            let Ok(target) = Target::parse(&hit.url) else {
                continue;
            };
            let host = registrable(&target.host);
            if is_not_a_company(&host) {
                continue;
            }
            let entry = by_host.entry(host.clone()).or_insert((0, hit.url.clone()));
            // **Agreement is counted once per query; the shallowest URL is the shallowest
            // anywhere.** These were one `continue` and the test for the second one failed:
            // a host's front page arriving after a deep page in the same result set was
            // skipped entirely, so `describe` would have fetched the blog post.
            if !seen_this_query.contains(&host) {
                seen_this_query.push(host);
                entry.0 += 1;
            }
            if depth(&hit.url) < depth(&entry.1) {
                entry.1 = hit.url.clone();
            }
        }
    }

    let mut found: Vec<Found> = by_host
        .into_iter()
        .map(|(host, (agreed, shallowest))| Found {
            confidence: score(agreed, asked, depth(&shallowest)),
            host,
            agreed,
            shallowest,
        })
        .collect();
    // Highest first, then by host so a tie is stable rather than however the map iterated —
    // a list that reorders between two runs of the same input is one nobody can reproduce.
    found.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.host.cmp(&b.host))
    });
    // **Complete, and ranked.** The cap that used to be here is now [`NAMED`], and it applies
    // to fetching rather than to the list: a company dropped before [`crate::competitors`] can
    // see it is a company nothing can report as excluded.
    found
}

/// How much we believe a host is one of the companies a description is about.
///
/// **Agreement is most of it.** A host every query returned is the strongest thing available
/// before a page is read; a host one query returned is a guess. The shallowest URL adds a
/// little, because a front page is what a company puts at `/` and an article about a company is
/// several levels into somebody else's site.
///
/// The weights are a starting point and are labelled as one, exactly as
/// [`landscape_core::subject::AMBIGUITY_MARGIN`] is. What matters more than their values is that
/// both inputs are countable from a URL, so a reader asking *why is this first* gets an answer
/// rather than a shrug.
#[must_use]
pub fn score(agreed: usize, asked: usize, depth: usize) -> f32 {
    if asked == 0 {
        return 0.0;
    }
    if agreed < CORROBORATION {
        // **Below the floor the gate refuses at, derived from it rather than guessed.** A number
        // chosen to be "clearly low" would drift the day somebody moved `MINIMUM_CONFIDENCE`,
        // and the drift would be silent: an uncorroborated candidate would start resolving
        // again with nothing in either file to say so.
        return landscape_core::subject::MINIMUM_CONFIDENCE / 2.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are single digits; the cap is three queries and five candidates"
    )]
    let agreement = agreed.min(asked) as f32 / asked as f32;
    // `/` and `/product` are a company's own front matter; `/blog/2024/05/a-review-of-x` is a
    // page about one. Two levels is the whole of the bonus, and it can never outweigh agreement.
    let shallow = match depth {
        0 | 1 => 0.2,
        2 => 0.1,
        _ => 0.0,
    };
    (agreement * 0.8 + shallow).clamp(0.0, 1.0)
}

/// Give each candidate the name and the line a reader picks between.
///
/// **From the company's own front page, never from the engine's title.** A disambiguation chip
/// is the one place a reader is asked to choose, and choosing between three summaries written by
/// a search engine is choosing between an engine's opinions. `fetch` returns the Markdown of a
/// URL, so this is the same conversion every extractor reads.
///
/// **And the front page is the front page**, built from the host, rather than the shallowest URL
/// a search happened to return. Review found the difference: the shallowest result for a company
/// is often `/pricing`, whose first heading is *"Pricing"* — offered to a reader as the name of
/// one of the companies they are choosing between.
///
/// A host whose front page cannot be read keeps its host as its name and says so, rather than
/// being dropped: *"we could not read this one"* is a thing a reader can act on, and a candidate
/// silently missing from a list is not.
pub async fn describe<F, Fut>(found: &[Found], words: &[String], fetch: F) -> Vec<Described>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut out = Vec::with_capacity(found.len());
    for (rank, one) in found.iter().enumerate() {
        // **Every candidate comes back; only the first [`NAMED`] cost somebody a request.** The
        // rest carry [`Vocabulary::NotRequested`] rather than being dropped, because a company
        // that never reaches [`crate::competitors::assemble`] is one nothing can report as
        // excluded — which is what review found happening to the sixth.
        if rank >= NAMED {
            out.push(Described {
                candidate: Candidate {
                    name: one.host.clone(),
                    canonical_domain: one.host.clone(),
                    what_it_is: "we did not read its front page".to_owned(),
                    confidence: one.confidence,
                },
                agreed: one.agreed,
                shares: Vocabulary::NotRequested,
            });
            continue;
        }
        let page = fetch(home_page(&one.host)).await;
        // **One read, two questions.** The name and the vocabulary both come from the front
        // page, and fetching it twice would be a second request to somebody's server for text
        // we already have.
        let shares = page.as_deref().map_or(Vocabulary::Unreadable, |markdown| {
            Vocabulary::Read(crate::competitors::shared(words, markdown))
        });
        out.push(Described {
            candidate: from_its_own_page(&one.host, page.as_deref(), one.confidence),
            agreed: one.agreed,
            shares,
        });
    }
    out
}

/// The company a reader named, described by its own front page.
///
/// **The seed of a competitor set, and the one company in it that needed no searching.** It gets
/// the same treatment every candidate gets — its name and its one-line self-description come
/// from its own page rather than from anywhere else — because a report that names two companies
/// differently depending on how they got there is a report that looks assembled.
///
/// `confidence` is 1.0 and it is not a measurement: the reader typed the domain, so there is
/// nothing to be confident *about*. Nothing scores this candidate;
/// [`crate::competitors::Because::Named`] is what a reader is shown instead.
pub async fn named_seed<F, Fut>(host: &str, fetch: F) -> Seed
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let page = fetch(home_page(host)).await;
    Seed {
        candidate: from_its_own_page(host, page.as_deref(), 1.0),
        read: page.is_some(),
    }
}

/// A named company, and whether anybody actually read its page.
///
/// **`read` is a separate field because the alternative was a lie in prose.** The first version
/// returned a bare [`Candidate`] and callers asked `what_it_is` for the company's vocabulary —
/// which, when the front page could not be fetched, holds *"we were unable to read its front
/// page"*. Review found what that meant for [`crate::competitors::of_company`]: the words
/// `unable`, `read`, `front` and `page` became the market vocabulary a rival had to match, so a
/// page saying *"read more on this page"* was admitted to the report with its reason citing
/// words that came from **our own error message**.
///
/// A fact about whether a fetch succeeded belongs in a `bool`, not inside a sentence written for
/// a reader. Anything derived from the sentence inherits the sentence's other job.
#[derive(Debug, Clone, PartialEq)]
pub struct Seed {
    /// Its name, domain and one-line self-description.
    pub candidate: Candidate,
    /// Whether its front page was read. `false` means [`Candidate::what_it_is`] is our sentence
    /// about the failure rather than the company's about itself.
    ///
    /// **Not the question a caller should ask.** See [`Self::vocabulary`]: a page that was read
    /// and said nothing quotable leaves exactly as little to compare a rival against as a page
    /// that could not be read at all, and review found the difference being treated as one.
    pub read: bool,
}

/// Why a named company's own page gives nothing to compare a rival against.
///
/// **Two ways to have nothing, and a reader is owed which.** One is about the company's server
/// and one is about what the company chose to put on its page; only the first is worth trying
/// again. Same distinction [`crate::competitors::Aside`] draws between `Unread` and
/// `ElsewhereEntirely`, one level up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoVocabulary {
    /// The front page could not be read at all.
    Unreadable,
    /// It was read, and had no line of prose long enough to quote.
    NothingQuotable,
}

impl NoVocabulary {
    /// The clause a reader is shown after *"nothing was searched for, because…"*.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Unreadable => "we could not read its front page",
            Self::NothingQuotable => "its front page has no sentence describing what it does",
        }
    }
}

impl Seed {
    /// The words a rival's front page has to have one of — or why there are none.
    ///
    /// **One predicate, and everything that decides on it calls this.** The first version gated
    /// on [`Self::read`] alone, and review found the hole: a reachable page reading only
    /// `# Basecamp` is `read == true` with an **empty** `what_it_is`, because [`naming`] wants a
    /// line of at least four words before it will quote one. Three queries went out, front pages
    /// were fetched, and every corroborated rival was set aside as *elsewhere entirely* — a
    /// negative claim about those companies, made out of missing evidence about the seed.
    ///
    /// **Then the same question got two answers.** `of_company` was fixed and
    /// `landscape candidates <domain>` was not, so the diagnostic printed three queries for a
    /// page the worker would send none for — a diagnostic that agrees with the worker by
    /// coincidence, which is the failure this codebase keeps deleting. There is one function
    /// now, and it returns a *reason* rather than a `bool` so the two callers cannot even
    /// describe the outcome differently.
    ///
    /// # Errors
    /// [`NoVocabulary`] when the page could not be read, or was read and said nothing quotable.
    pub fn words(&self) -> Result<Vec<String>, NoVocabulary> {
        if !self.read {
            // `what_it_is` is our sentence about the failure here, and mining it produced
            // `unable`, `read`, `front`, `page` as the market's vocabulary.
            return Err(NoVocabulary::Unreadable);
        }
        let words = crate::competitors::content_words(&self.candidate.what_it_is);
        if words.is_empty() {
            return Err(NoVocabulary::NothingQuotable);
        }
        Ok(words)
    }
}

/// A candidate built from a host and whatever its front page turned out to be.
///
/// **One copy of the fallback**, because [`describe`] and [`named_seed`] both need it and two
/// copies would be two wordings of *"we could not read its front page"* that drift apart the
/// first time one is edited.
fn from_its_own_page(host: &str, page: Option<&str>, confidence: f32) -> Candidate {
    let (name, what_it_is) = page.map_or_else(
        || {
            (
                host.to_owned(),
                "we were unable to read its front page".to_owned(),
            )
        },
        |markdown| naming(host, markdown),
    );
    Candidate {
        name,
        canonical_domain: host.to_owned(),
        what_it_is,
        confidence,
    }
}

/// A description in, the gate's verdict **and** the competitor set out.
///
/// `FACT_CHECKING.md` §3.1 steps 2 to 4, plus the step this product is actually for: turning one
/// description into the several companies a landscape compares. See
/// [`crate::competitors`] for why those are two questions rather than one, and why the gate is
/// still asked first.
///
/// **One path, three callers.** `landscape candidates` prints what this returns, the worker acts
/// on it, and the tests assert on it. The sequence was written out by hand in the first two of
/// those and the third is the one that decides whether a report gets written, so a diagnostic
/// that agreed with the worker only by coincidence was a matter of time.
///
/// `fetch` names each candidate from its own front page; see [`describe`]. The [`Queried`]
/// returned beside the verdict says which searches completed, because a thin list and a quiet
/// market are different findings and only that tells them apart.
pub async fn for_description<F, Fut>(
    engine: &dyn SourceProvider,
    description: &str,
    fetch: F,
) -> (crate::competitors::Derived, Queried)
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let (found, queried) = suggest(engine, description).await;
    let words = crate::competitors::content_words(description);
    let named = describe(&found, &words, fetch).await;
    // **Two consumers, two lists, and the difference is deliberate.** The set gets everything
    // that was found, because a company it cannot see is a company nothing can report as
    // excluded. The gate gets only what was *fetched*, because it asks a reader to choose and a
    // candidate with no name of its own is not a choice. Review found each of these in turn,
    // from opposite directions.
    let mut set = crate::competitors::assemble(named.clone(), queried.sent(), &words);
    // **A description that produced one company is the same shape from a reader's side as a
    // named company nobody could be found for**: one company, in a tool that promises a
    // comparison. Same function, so the two paths cannot come to different conclusions about
    // the same evidence.
    set.alone = crate::competitors::alone_because(
        set.members.len(),
        &queried,
        crate::competitors::Sought::CompaniesMatchingTheDescription,
        None,
    );
    let choices: Vec<Candidate> = named
        .into_iter()
        .filter(Described::was_requested)
        .map(|d| d.candidate)
        .collect();
    // **Only what came back.** This list is what a reader is shown as *"we checked these"*, and
    // a query that never reached an engine checked nothing. Review found the previous version
    // listing all three after all three had failed.
    let checked = queried.completed.clone();
    let verdict = landscape_core::subject::resolve(description, choices, checked);
    (
        crate::competitors::Derived {
            verdict,
            set,
            about_a_market: crate::competitors::about_a_market(&words),
        },
        queried,
    )
}

/// The company's front page.
///
/// `https`, always: a candidate arrives from a search engine, and asking for a company's home
/// page over plaintext because a result happened to be `http` is a downgrade nobody asked for.
/// The fetcher's guard is still what decides whether the URL may be reached at all.
fn home_page(host: &str) -> String {
    format!("https://{host}/")
}

/// A page's own name for itself, and the first line that says what it is.
///
/// The heading is the name; the first sentence of prose under it is the distinguisher. Both come
/// from the page, so a reader choosing between two candidates is reading what each company says
/// about itself rather than what an engine said about them.
fn naming(host: &str, markdown: &str) -> (String, String) {
    let mut lines = markdown.lines().map(str::trim).filter(|l| !l.is_empty());
    let name = lines
        .by_ref()
        .find_map(|line| {
            let text = line.trim_start_matches('#').trim();
            (line.starts_with('#') && !text.is_empty()).then(|| text.to_owned())
        })
        .unwrap_or_else(|| host.to_owned());

    /// Long enough to distinguish, short enough for a chip.
    const MOST: usize = 140;
    let what = lines
        .find(|line| !line.starts_with('#') && line.split_whitespace().count() >= 4)
        .map_or_else(String::new, |line| {
            let mut trimmed: String = line.chars().take(MOST).collect();
            if line.chars().count() > MOST {
                trimmed.push('…');
            }
            trimmed
        });
    (name, what)
}

/// The registrable domain: lowercased, and one host per company.
///
/// **Two rounds of review, and the second one is the reason this is a dependency.** The first
/// version lowercased and stripped `www.`, so `example.com`, `app.example.com` and
/// `docs.example.com` were three candidates — which divides the cross-query agreement the whole
/// score rests on. The fix for that was a hand-written list of thirty multi-label suffixes, and
/// it was worse in the direction that matters:
///
/// ```text
/// alpha.github.io  ->  github.io
/// beta.github.io   ->  github.io      one "company", agreed = 2
/// ```
///
/// Two unrelated tenants, seen by two different queries, **manufacturing the corroboration**
/// that [`CORROBORATION`] was added to require one round earlier. A missing suffix does not
/// merely merge two companies: it forges the evidence that lets the merged thing auto-resolve.
///
/// So the boundary comes from the Public Suffix List, **including its private section**, which
/// is what knows that `github.io` is a suffix and `github.com` is a company. A thirty-entry
/// sample cannot be completed by adding entries, because the failure is *not knowing what is
/// missing* — and that is precisely what a maintained list is for.
///
/// **An IP literal is not a domain, and the list will happily pretend otherwise.**
/// `psl::domain_str("127.0.0.1")` is `Some("0.1")` — and so is `psl::domain_str("10.0.0.1")`, so
/// two unrelated addresses forge the same agreement the private suffixes did. Review found it;
/// the previous version of this comment claimed an IP address was "returned unchanged", which
/// was true of IPv6 and false of the case that matters. Addresses are recognised before the list
/// is consulted now, and [`is_not_a_company`] keeps them off a reader's list entirely.
///
/// A host the list cannot place at all — `localhost`, a bare label — is returned lowercased and
/// unchanged rather than guessed at.
fn registrable(host: &str) -> String {
    let lowered = host.trim().trim_end_matches('.').to_lowercase();
    if lowered.parse::<std::net::IpAddr>().is_ok() {
        return lowered;
    }
    let Some(domain) = psl::domain_str(&lowered) else {
        return lowered;
    };
    domain.to_owned()
}

/// Whether this host is a place people talk about companies rather than a company.
///
/// Suffix-matched on a label boundary, so `blog.medium.com` is excluded and `mediumroast.com` is
/// not — the same whole-token rule the trust scanner needed, for the same reason.
fn is_not_a_company(host: &str) -> bool {
    // **A raw address is not a company.** Nobody publishes a company at `93.184.216.34`, and
    // offering one as a choice a reader makes between five names is offering nonsense. Keeping
    // them out here rather than in `registrable` also means [`home_page`] never has to decide
    // whether an IPv6 literal needs brackets: the case cannot arrive.
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    NOT_A_COMPANY
        .iter()
        .any(|known| host == *known || host.ends_with(&format!(".{known}")))
}

/// How many path segments a URL has. `https://a.com/` is 0.
fn depth(url: &str) -> usize {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .split_once('/')
        .map_or("", |(_, path)| path)
        .split(['?', '#'])
        .next()
        .unwrap_or("");
    path.split('/').filter(|s| !s.is_empty()).count()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::provider::SearchError;
    use landscape_core::subject::Resolution;

    fn hit(url: &str) -> Hit {
        Hit {
            url: url.to_owned(),
            title: "a title the engine wrote".to_owned(),
            snippet: "a snippet the engine wrote".to_owned(),
        }
    }

    #[test]
    fn a_description_becomes_three_differently_worded_queries() {
        let queries = for_idea("privacy-friendly website analytics");
        assert_eq!(queries.len(), 3);
        for q in &queries {
            assert!(
                q.text.contains("privacy-friendly website analytics"),
                "{}",
                q.text
            );
        }
        // Differently worded, because the score rests on agreement between them and three
        // copies of one question agree with themselves.
        let texts: Vec<&str> = queries.iter().map(|q| q.text.as_str()).collect();
        assert_eq!(
            texts.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "{texts:?}"
        );
    }

    #[test]
    fn a_blank_description_asks_nothing() {
        // Interpolating an empty description sends `best  software` to an engine, which
        // returns the internet.
        assert!(for_idea("").is_empty());
        assert!(for_idea("   ").is_empty());
    }

    #[test]
    fn a_host_every_query_returned_outranks_one_that_appeared_once() {
        let results = vec![
            vec![hit("https://a.com/"), hit("https://b.com/")],
            vec![hit("https://a.com/pricing")],
            vec![hit("https://a.com/")],
        ];
        let found = from_results(&results, 3);
        assert_eq!(found[0].host, "a.com");
        assert_eq!(found[0].agreed, 3);
        assert!(found[0].confidence > found[1].confidence, "{found:#?}");
    }

    #[test]
    fn one_query_returning_a_host_five_times_has_said_one_thing() {
        // Volume is not agreement. A listicle host with five pages in one result set would
        // otherwise outscore a company two separate queries both found.
        let results = vec![vec![
            hit("https://a.com/one"),
            hit("https://a.com/two"),
            hit("https://a.com/three"),
            hit("https://a.com/four"),
            hit("https://a.com/five"),
        ]];
        let found = from_results(&results, 3);
        assert_eq!(found[0].agreed, 1, "{found:#?}");
    }

    #[test]
    fn a_review_site_is_not_a_company() {
        // `FACT_CHECKING.md` §3.2 puts these at the bottom of both axes. The point here is
        // narrower: a report about g2.com is not the report anybody asked for.
        let results = vec![vec![
            hit("https://www.g2.com/categories/web-analytics"),
            hit("https://old.reddit.com/r/analytics/comments/abc"),
            hit("https://usefathom.com/"),
        ]];
        let found = from_results(&results, 1);
        let hosts: Vec<&str> = found.iter().map(|f| f.host.as_str()).collect();
        assert_eq!(hosts, vec!["usefathom.com"]);
    }

    #[test]
    fn a_company_whose_name_contains_a_review_site_is_still_a_company() {
        // Suffix-matched on a label boundary. `mediumroast.com` is not `medium.com`, and the
        // substring version of this rule would delete a real company from the list.
        assert!(is_not_a_company("medium.com"));
        assert!(is_not_a_company("blog.medium.com"));
        assert!(!is_not_a_company("mediumroast.com"));
        assert!(!is_not_a_company("notmedium.com"));
    }

    #[test]
    fn www_and_the_bare_host_are_one_company() {
        let results = vec![
            vec![hit("https://www.a.com/")],
            vec![hit("https://a.com/pricing")],
        ];
        let found = from_results(&results, 2);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].agreed, 2);
    }

    #[test]
    fn the_shallowest_url_is_the_one_kept() {
        // It is the one `describe` fetches, and a company's front page is what says what the
        // company is. A blog post four levels down says what one writer thinks.
        let results = vec![vec![
            hit("https://a.com/blog/2026/05/a-review"),
            hit("https://a.com/"),
        ]];
        let found = from_results(&results, 1);
        assert_eq!(found[0].shallowest, "https://a.com/");
    }

    #[test]
    fn a_query_that_never_answered_does_not_make_a_host_unanimous() {
        // Two of three queries failed. The host appeared in the one that answered, and
        // scoring it against one would call an engine outage a confident answer.
        let results = vec![vec![hit("https://a.com/")]];
        let found = from_results(&results, 3);
        assert!(
            found[0].confidence < 0.5,
            "an outage produced confidence: {found:#?}"
        );
    }

    #[test]
    fn agreement_outweighs_a_shallow_url() {
        // A front page found once must not outrank a company every query agreed on, however
        // deep the latter's shallowest page happened to be.
        let unanimous_but_deep = score(3, 3, 4);
        let shallow_but_alone = score(1, 3, 0);
        assert!(
            unanimous_but_deep > shallow_but_alone,
            "{unanimous_but_deep} vs {shallow_but_alone}"
        );
    }

    #[test]
    fn a_score_is_never_outside_the_range_the_gate_compares() {
        for agreed in 0..6 {
            for asked in 0..4 {
                for depth in 0..6 {
                    let s = score(agreed, asked, depth);
                    assert!((0.0..=1.0).contains(&s), "{agreed}/{asked}/{depth} = {s}");
                }
            }
        }
    }

    #[test]
    fn nothing_is_scored_when_nothing_was_asked() {
        assert!((score(3, 0, 0) - 0.0).abs() < f32::EPSILON);
        assert!(from_results(&[], 0).is_empty());
    }

    #[test]
    fn every_company_found_survives_the_scoring() {
        // **Review found the opposite behaviour and the reason it was wrong.** This used to
        // assert the list was truncated to five, which is a fine rule for a list a reader picks
        // one company from and a silent drop for a set: the sixth was neither compared nor
        // reported as excluded. The budget still exists - it is `NAMED`, and it applies to
        // fetching front pages, which is a cost somebody else pays.
        let many: Vec<Hit> = (0..12)
            .map(|i| hit(&format!("https://company{i}.com/")))
            .collect();
        let found = from_results(&[many], 1);
        assert_eq!(
            found.len(),
            12,
            "a company was dropped before anything could report it"
        );
    }

    #[test]
    fn candidates_that_tie_come_back_in_the_same_order_every_time() {
        // A `HashMap` iterates in whatever order it likes. A list that reorders under a reader
        // is one nobody can reproduce, and the gate's margin compares the first two — so which
        // two those are cannot be luck.
        //
        // **Eight tying hosts, not two.** The first version of this test used two, and the
        // mutation that deletes the tie-break survived: with two entries the map's order agreed
        // with the sorted one often enough to pass. Eight makes agreement by chance a one-in-
        // forty-thousand event rather than a coin flip.
        let hosts = ["h", "c", "a", "g", "b", "e", "d", "f"];
        let results = vec![hosts
            .iter()
            .map(|h| hit(&format!("https://{h}.com/")))
            .collect::<Vec<_>>()];
        let found = from_results(&results, 1);

        let got: Vec<&str> = found.iter().map(|f| f.host.as_str()).collect();
        // Every one ties on score, so the whole order is the tie-break - all eight of it now
        // that nothing is truncated, which is a stricter reading of the same rule.
        assert_eq!(
            got,
            vec!["a.com", "b.com", "c.com", "d.com", "e.com", "f.com", "g.com", "h.com"],
            "{found:#?}"
        );
        assert_eq!(found, from_results(&results, 1), "two runs, two answers");
    }

    #[tokio::test]
    async fn a_candidate_is_named_by_its_own_page() {
        let found = vec![Found {
            host: "usefathom.com".to_owned(),
            confidence: 0.9,
            agreed: 3,
            shallowest: "https://usefathom.com/".to_owned(),
        }];
        let described = describe(&found, &[], |_url| async {
            Some(
                "# Fathom Analytics\nSimple, privacy-first website analytics with no cookies."
                    .to_owned(),
            )
        })
        .await;
        assert_eq!(described[0].candidate.name, "Fathom Analytics");
        assert_eq!(
            described[0].candidate.what_it_is,
            "Simple, privacy-first website analytics with no cookies."
        );
        assert_eq!(described[0].candidate.canonical_domain, "usefathom.com");
    }

    #[tokio::test]
    async fn a_named_company_takes_its_name_from_its_own_front_page() {
        // The seed of a competitor set gets the same treatment as every candidate beside it. A
        // report that names one company from its own page and another from its domain is a
        // report that looks assembled, and the difference would sit in the first note.
        let seed = named_seed("basecamp.com", |url| async move {
            assert_eq!(
                url, "https://basecamp.com/",
                "the front page was not the page read"
            );
            Some("# Basecamp\nProject management and team communication.".to_owned())
        })
        .await;
        assert_eq!(seed.candidate.name, "Basecamp");
        assert!(seed.read, "a page that was read is reported as unread");
        assert_eq!(seed.candidate.canonical_domain, "basecamp.com");
        assert_eq!(
            seed.candidate.what_it_is,
            "Project management and team communication."
        );
    }

    #[tokio::test]
    async fn a_named_company_we_could_not_read_still_seeds_its_own_report() {
        // The reader named it. A front page that will not load is a reason to say less about
        // it, never a reason to drop the one company somebody actually asked about.
        let seed = named_seed("basecamp.com", |_url| async { None }).await;
        assert_eq!(seed.candidate.name, "basecamp.com");
        assert!(seed.candidate.what_it_is.contains("unable to read"));
        // **The fact lives in a `bool`, not in the sentence.** Review found the sentence
        // being mined for market vocabulary.
        assert!(!seed.read);
    }

    #[tokio::test]
    async fn a_candidate_we_could_not_read_says_so_rather_than_vanishing() {
        // Dropping it would shorten a list a reader is choosing from, with nothing saying one
        // was removed — the silent-truncation failure this project keeps deleting.
        let found = vec![Found {
            host: "unreachable.example".to_owned(),
            confidence: 0.5,
            agreed: 1,
            shallowest: "https://unreachable.example/".to_owned(),
        }];
        let described = describe(&found, &["analytics".to_owned()], |_url| async { None }).await;
        assert_eq!(described.len(), 1);
        assert_eq!(described[0].candidate.name, "unreachable.example");
        assert!(described[0].candidate.what_it_is.contains("unable to read"));
        // **A page nobody read shares nothing and knows nothing, and those are different.**
        // `Read(vec![])` here would say the company is in some other market; `Unreadable` says
        // we tried and could not - the distinction `competitors::Aside` is built on.
        assert_eq!(described[0].shares, Vocabulary::Unreadable);
    }

    #[tokio::test]
    async fn no_more_front_pages_are_fetched_than_the_number_that_is_stated() {
        // `from_results` already caps its list, so this is a guard on *this function's* own
        // contract rather than a second copy of that one: `describe` is public, and a caller
        // handing it twenty hosts would put twenty requests on twenty servers before a reader
        // had asked for anything.
        let many: Vec<Found> = (0..8)
            .map(|i| Found {
                host: format!("c{i}.example"),
                confidence: 0.5,
                agreed: 1,
                shallowest: format!("https://c{i}.example/"),
            })
            .collect();
        let fetched = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let counter = std::sync::Arc::clone(&fetched);
        let described = describe(&many, &[], move |_url| {
            let counter = std::sync::Arc::clone(&counter);
            async move {
                *counter.lock().unwrap() += 1;
                Some(
                    "# A company
It does a thing for other companies."
                        .to_owned(),
                )
            }
        })
        .await;
        // **Every candidate comes back; only `NAMED` of them cost a request.** Returning five
        // of eight is what review found: the other three were gone before anything could report
        // them as excluded.
        assert_eq!(
            described.len(),
            8,
            "a candidate was dropped rather than deferred"
        );
        assert_eq!(
            *fetched.lock().unwrap(),
            NAMED,
            "the fetch budget was not kept"
        );
        for one in described.iter().take(NAMED) {
            assert!(
                matches!(one.shares, Vocabulary::Read(_)),
                "{:?}",
                one.shares
            );
        }
        for one in described.iter().skip(NAMED) {
            // Not `Unreadable`: nobody asked, so nothing failed.
            assert_eq!(one.shares, Vocabulary::NotRequested);
        }
    }

    #[test]
    fn every_subdomain_of_one_company_is_one_company() {
        // **Review found the first version splitting one company into three.** Agreement is
        // what the whole score rests on, and `example.com`, `app.example.com` and
        // `docs.example.com` as three candidates divides it three ways — then spends three of
        // the five slots a reader chooses from on one vendor.
        let results = vec![
            vec![hit("https://example.com/")],
            vec![hit("https://app.example.com/dashboard")],
            vec![hit("https://docs.example.com/start")],
        ];
        let found = from_results(&results, 3);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].host, "example.com");
        assert_eq!(found[0].agreed, 3, "the agreement was split: {found:#?}");
    }

    #[test]
    fn two_companies_are_still_two_companies() {
        // The other direction, and the worse one: merging two vendors into one candidate would
        // put a company on a reader's list that no query ever returned.
        let results = vec![vec![hit("https://a.com/"), hit("https://b.com/")]];
        let found = from_results(&results, 1);
        assert_eq!(found.len(), 2, "{found:#?}");
    }

    #[test]
    fn a_public_suffix_is_not_a_company() {
        // `bbc.co.uk` is a company; `co.uk` is not, and grouping on the last two labels would
        // file every British company under one candidate.
        assert_eq!(registrable("www.bbc.co.uk"), "bbc.co.uk");
        assert_eq!(registrable("news.bbc.co.uk"), "bbc.co.uk");
        assert_eq!(registrable("shop.example.com.au"), "example.com.au");
        assert_ne!(registrable("bbc.co.uk"), registrable("itv.co.uk"));

        // And an ordinary two-label suffix is not treated as one.
        assert_eq!(registrable("deep.sub.example.com"), "example.com");
        assert_eq!(registrable("EXAMPLE.COM."), "example.com");
        assert_eq!(registrable("example.com"), "example.com");
    }

    #[test]
    fn two_tenants_on_a_private_suffix_are_two_companies() {
        // **The reason the hand-written list became a dependency.** `github.io` is a suffix and
        // `github.com` is a company, and only the Public Suffix List's private section knows
        // the difference. A thirty-entry sample said both tenants were one company called
        // `github.io`.
        assert_eq!(registrable("alpha.github.io"), "alpha.github.io");
        assert_ne!(
            registrable("alpha.github.io"),
            registrable("beta.github.io")
        );
        assert_eq!(registrable("github.com"), "github.com");
        assert_eq!(registrable("www.github.com"), "github.com");

        // The same shape on a suffix the sample also missed.
        assert_ne!(registrable("alpha.com.cn"), registrable("beta.com.cn"));
    }

    #[test]
    fn two_tenants_on_a_private_suffix_do_not_manufacture_corroboration() {
        // Worse than merging two companies: two unrelated tenants seen by two different queries
        // forged the agreement that `CORROBORATION` exists to require, so a "company" nobody
        // searched for could auto-resolve. Review found it one round after the corroboration
        // rule went in.
        let results = vec![
            vec![hit("https://alpha.github.io/")],
            vec![hit("https://beta.github.io/")],
        ];
        let found = from_results(&results, 2);
        assert_eq!(found.len(), 2, "{found:#?}");
        assert!(
            found.iter().all(|f| f.agreed == 1),
            "corroboration was manufactured: {found:#?}"
        );
        assert!(
            found
                .iter()
                .all(|f| f.confidence < landscape_core::subject::MINIMUM_CONFIDENCE),
            "and it reached the gate: {found:#?}"
        );
    }

    #[test]
    fn a_host_the_list_cannot_place_is_returned_rather_than_guessed_at() {
        assert_eq!(registrable("localhost"), "localhost");
        assert_eq!(registrable(""), "");
    }

    #[test]
    fn an_address_is_not_carved_into_a_domain() {
        // **The list will happily pretend an address is one.** `psl::domain_str("127.0.0.1")`
        // is `Some("0.1")`, and so is `psl::domain_str("10.0.0.1")`. The previous version of
        // this test covered `localhost` and the empty string — the branch where `domain_str`
        // returns `None` and the claim happened to hold — and missed the one that does not.
        assert_eq!(registrable("127.0.0.1"), "127.0.0.1");
        assert_eq!(registrable("10.0.0.1"), "10.0.0.1");
        assert_eq!(registrable("93.184.216.34"), "93.184.216.34");
        assert_ne!(registrable("127.0.0.1"), registrable("10.0.0.1"));
        assert_eq!(registrable("::1"), "::1");
        assert_eq!(registrable("2001:db8::1"), "2001:db8::1");
    }

    #[test]
    fn two_addresses_sharing_their_last_octets_do_not_manufacture_corroboration() {
        // The end of it, asserted where it matters. Both used to become `0.1` with
        // `agreed == 2` — a "company" nobody searched for, corroborated by nothing.
        let results = vec![
            vec![hit("https://127.0.0.1/")],
            vec![hit("https://10.0.0.1/")],
        ];
        let found = from_results(&results, 2);
        assert!(
            found.is_empty(),
            "an address reached a reader's list: {found:#?}"
        );
    }

    #[test]
    fn an_address_is_never_offered_as_a_company() {
        // And it never reaches `home_page`, which is why that function has no IPv6 bracket
        // case to get wrong.
        assert!(is_not_a_company("127.0.0.1"));
        assert!(is_not_a_company("93.184.216.34"));
        assert!(is_not_a_company("::1"));
        assert!(!is_not_a_company("usefathom.com"));
    }

    #[tokio::test]
    async fn the_page_that_names_a_company_is_its_front_page() {
        // **Review found `describe` fetching the shallowest search result.** For a real company
        // that is often `/pricing`, whose first heading is `Pricing` — so a reader would have
        // been asked to choose between three companies, one of them called "Pricing".
        let found = vec![Found {
            host: "usefathom.com".to_owned(),
            confidence: 0.9,
            agreed: 3,
            shallowest: "https://usefathom.com/pricing".to_owned(),
        }];
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen = std::sync::Arc::clone(&asked);
        let described = describe(&found, &[], move |url| {
            let seen = std::sync::Arc::clone(&seen);
            async move {
                seen.lock().unwrap().push(url.clone());
                Some(if url.ends_with("/pricing") {
                    "# Pricing
Plans start at $14 a month."
                        .to_owned()
                } else {
                    "# Fathom Analytics
Simple, privacy-first website analytics."
                        .to_owned()
                })
            }
        })
        .await;

        assert_eq!(
            asked.lock().unwrap().as_slice(),
            ["https://usefathom.com/".to_owned()],
            "the pricing page was fetched to name the company"
        );
        assert_eq!(described[0].candidate.name, "Fathom Analytics");
    }

    #[tokio::test]
    async fn one_search_out_of_three_never_resolves_by_itself() {
        // **End to end, because the number was not the point — the gate's answer was.** The old
        // test asserted `confidence < 0.5` and passed while 0.47 sailed over
        // `MINIMUM_CONFIDENCE`: two queries fail, the third returns one company, and an
        // analysis runs against a company that appeared in a single search.
        let engine = Canned {
            per_query: vec![Ok(vec![hit("https://lonely.example/")]), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, queried) = suggest(&engine, "a market nobody agrees about").await;
        assert_eq!(queried.failed.len(), 2);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].agreed, 1);

        let named = describe(&found, &[], |_url| async {
            Some(
                "# Lonely
The only thing one search returned."
                    .to_owned(),
            )
        })
        .await;
        let verdict = landscape_core::subject::resolve(
            "a market nobody agrees about",
            named.into_iter().map(|d| d.candidate).collect(),
            vec!["three queries, two of which did not complete".to_owned()],
        );
        assert!(
            matches!(
                verdict,
                landscape_core::subject::Resolution::NothingFound { .. }
            ),
            "one search resolved a subject: {verdict:?}"
        );
    }

    #[tokio::test]
    async fn two_searches_agreeing_do_resolve() {
        // The other half, so the rule above is a rule and not a refusal to answer.
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://agreed.example/")]),
                Ok(vec![hit("https://agreed.example/pricing")]),
                Err(()),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, _) = suggest(&engine, "something two engines agree about").await;
        assert_eq!(found[0].agreed, 2);

        let named = describe(&found, &[], |_url| async {
            Some(
                "# Agreed
A company two searches both returned."
                    .to_owned(),
            )
        })
        .await;
        let verdict = landscape_core::subject::resolve(
            "something",
            named.into_iter().map(|d| d.candidate).collect(),
            Vec::new(),
        );
        match verdict {
            landscape_core::subject::Resolution::Resolved { entity } => {
                assert_eq!(entity.canonical_domain, "agreed.example");
            }
            other => panic!("corroborated candidate did not resolve: {other:?}"),
        }
    }

    #[test]
    fn more_agreement_is_more_confidence_all_the_way_up() {
        // **Not just corroborated-or-not.** Adding the corroboration floor left every test
        // above it comparing a corroborated candidate with a floored one, so nothing
        // distinguished two-of-three from three-of-three and the divisor could have been
        // anything. The mutation harness said so.
        let two_of_three = score(2, 3, 0);
        let three_of_three = score(3, 3, 0);
        assert!(
            three_of_three > two_of_three,
            "unanimity is worth no more than a majority: {three_of_three} vs {two_of_three}"
        );
        // And the gap is the agreement, not the URL: both are front pages here.
        assert!(
            (three_of_three - two_of_three) > 0.2,
            "{three_of_three} vs {two_of_three}"
        );
    }

    #[test]
    fn an_uncorroborated_candidate_scores_below_the_floor_the_gate_refuses_at() {
        // Derived from the gate's own constant rather than a number chosen to look low, so the
        // two cannot drift apart in silence.
        let alone = score(1, 3, 0);
        assert!(
            alone < landscape_core::subject::MINIMUM_CONFIDENCE,
            "{alone} would reach the gate"
        );
        assert!(score(2, 3, 0) > landscape_core::subject::MINIMUM_CONFIDENCE);
    }

    #[tokio::test]
    async fn a_description_that_pins_one_company_resolves_it() {
        // The whole sequence in one call, because three places used to run it by hand and the
        // one that decides whether a report gets written is not the one anybody watches.
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://agreed.example/")]),
                Ok(vec![hit("https://agreed.example/pricing")]),
                Ok(vec![hit("https://agreed.example/about")]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, queried) =
            for_description(&engine, "a market with one answer", |_url| async {
                Some(
                    "# Agreed
A company every search returned."
                        .to_owned(),
                )
            })
            .await;
        assert!(queried.failed.is_empty());
        match derived.verdict {
            Resolution::Resolved { entity } => {
                assert_eq!(entity.canonical_domain, "agreed.example");
                assert_eq!(entity.name, "Agreed");
            }
            other => panic!("a unanimous description did not resolve: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_description_matching_two_companies_asks_rather_than_guessing() {
        // `PRODUCT_SPEC.md` §3: one chip click prevents an entire wrong report. The gate has to
        // *reach* this verdict for that to be true, and the candidates have to arrive named,
        // because "choose between two domains" is a worse question than "choose between two
        // companies".
        let engine = Canned {
            per_query: vec![
                Ok(vec![
                    hit("https://alpha.example/"),
                    hit("https://beta.example/"),
                ]),
                Ok(vec![
                    hit("https://alpha.example/x"),
                    hit("https://beta.example/y"),
                ]),
                Ok(vec![
                    hit("https://alpha.example/z"),
                    hit("https://beta.example/w"),
                ]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(&engine, "a crowded market", |url| async move {
            Some(if url.contains("alpha") {
                "# Alpha
The first of two."
                    .to_owned()
            } else {
                "# Beta
The second of two."
                    .to_owned()
            })
        })
        .await;
        match derived.verdict {
            Resolution::Ambiguous { candidates, .. } => {
                assert_eq!(candidates.len(), 2, "{candidates:#?}");
                let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
                assert!(
                    names.contains(&"Alpha") && names.contains(&"Beta"),
                    "{names:?}"
                );
            }
            other => panic!("two equal companies did not ask: {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_engine_that_answered_nothing_checked_nothing() {
        // **Review found a total outage reported as an empty market**, with all three queries
        // listed as evidence of the looking. `FACT_CHECKING.md` §5.4: a negative nobody can
        // check is not a finding, and a query that never reached an engine checked nothing.
        let engine = Canned {
            per_query: vec![Err(()), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, queried) =
            for_description(&engine, "privacy-friendly analytics", |_url| async { None }).await;

        assert_eq!(queried.failed.len(), 3);
        assert!(queried.completed.is_empty());
        assert!(queried.nothing_completed());
        assert_eq!(queried.sent(), 3, "the divisor is still what was sent");

        match derived.verdict {
            Resolution::NothingFound { checked } => assert!(
                checked.is_empty(),
                "queries that never completed are listed as checked: {checked:?}"
            ),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn a_query_that_never_ran_does_not_agree_with_the_ones_that_did() {
        // **The divisor is what was sent.** Two queries came back naming the same host and the
        // third never reached the engine; that is agreement between two of three, not unanimity.
        // Dividing by the queries that *answered* would let an outage manufacture certainty.
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://twoagree.example/")]),
                Ok(vec![hit("https://twoagree.example/")]),
                Err(()),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, queried) = suggest(&engine, "a market with an outage").await;

        assert_eq!(queried.completed.len(), 2);
        assert_eq!(queried.failed.len(), 1);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].agreed, 2);
        assert!(
            (found[0].confidence - score(2, 3, 1)).abs() < f32::EPSILON,
            "scored against the queries that answered, not those sent: {}",
            found[0].confidence
        );
        assert!(
            found[0].confidence < score(2, 2, 1),
            "an outage manufactured unanimity: {}",
            found[0].confidence
        );
    }

    #[tokio::test]
    async fn a_partial_outage_reports_only_the_queries_that_came_back() {
        // The harder half: something answered, so this is not a total outage — but the audit
        // trail must still name only what ran, and the score must still divide by what was sent.
        let engine = Canned {
            per_query: vec![Ok(vec![hit("https://alone.example/")]), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, queried) =
            for_description(&engine, "a thin market", |_url| async { None }).await;

        assert_eq!(queried.completed.len(), 1);
        assert_eq!(queried.failed.len(), 2);
        assert!(!queried.nothing_completed(), "something did come back");

        // One query found one host, so it is uncorroborated and the gate refuses it - and the
        // one checked query is what a reader is shown, not three.
        match derived.verdict {
            Resolution::NothingFound { checked } => {
                assert_eq!(checked, queried.completed, "{checked:?}");
                assert_eq!(checked.len(), 1);
            }
            other => panic!("an uncorroborated candidate resolved: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_market_description_produces_every_company_rather_than_one() {
        // **The whole row, end to end.** Three companies every search returned tie at 1.0, the
        // gate calls that ambiguous, and for a description of a market the tie *is* the answer.
        let engine = Canned {
            per_query: vec![
                Ok(vec![
                    hit("https://usefathom.com/"),
                    hit("https://plausible.io/"),
                ]),
                Ok(vec![
                    hit("https://plausible.io/pricing"),
                    hit("https://usefathom.com/"),
                ]),
                Ok(vec![
                    hit("https://usefathom.com/"),
                    hit("https://plausible.io/"),
                ]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |url| async move {
                Some(if url.contains("fathom") {
                    "# Fathom\nSimple privacy analytics.".to_owned()
                } else {
                    "# Plausible\nPrivacy-first website analytics.".to_owned()
                })
            },
        )
        .await;

        assert!(derived.about_a_market, "four words read as a name");
        assert!(
            matches!(derived.verdict, Resolution::Ambiguous { .. }),
            "the gate no longer sees a tie: {:?}",
            derived.verdict
        );
        let names: Vec<&str> = derived
            .set
            .members
            .iter()
            .map(|m| m.candidate.name.as_str())
            .collect();
        assert_eq!(names.len(), 2, "{:#?}", derived.set);
        assert!(
            names.contains(&"Fathom") && names.contains(&"Plausible"),
            "{names:?}"
        );
        assert!(
            derived.set.set_aside.is_empty(),
            "{:#?}",
            derived.set.set_aside
        );
        for m in &derived.set.members {
            let crate::competitors::Because::Found {
                agreed,
                asked,
                shares,
            } = &m.because
            else {
                panic!(
                    "a company we searched for was reported as named: {:?}",
                    m.because
                )
            };
            assert_eq!(*agreed, 3);
            assert_eq!(*asked, 3);
            assert!(shares.contains(&"analytics".to_owned()));
        }
    }

    #[tokio::test]
    async fn a_sixth_corroborated_company_is_never_missing_from_both_lists() {
        // **The guarantee, end to end.** Six companies, every search returning all of them,
        // and a fetch budget of five. Review found the sixth vanishing inside `from_results`:
        // not a member, not an exclusion, and named nowhere a reader could see it.
        let all: Vec<Hit> = (0..6)
            .map(|i| hit(&format!("https://c{i}.example/")))
            .collect();
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |_url| async { Some("# A company\nPrivacy-first website analytics.".to_owned()) },
        )
        .await;

        let mut accounted: Vec<&str> = derived
            .set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .chain(
                derived
                    .set
                    .set_aside
                    .iter()
                    .map(|(c, _)| c.canonical_domain.as_str()),
            )
            .collect();
        accounted.sort_unstable();
        assert_eq!(
            accounted,
            vec![
                "c0.example",
                "c1.example",
                "c2.example",
                "c3.example",
                "c4.example",
                "c5.example"
            ],
            "a company was found and appears in neither list: {:#?}",
            derived.set
        );

        let (sixth, why) = derived
            .set
            .set_aside
            .iter()
            .find(|(c, _)| c.canonical_domain == "c5.example")
            .expect("the sixth is somewhere");
        assert_eq!(
            *why,
            crate::competitors::Aside::BeyondTheFetchBudget { budget: NAMED }
        );
        assert_eq!(sixth.canonical_domain, "c5.example");
        assert_eq!(derived.set.members.len(), NAMED);
    }

    #[tokio::test]
    async fn a_reader_is_never_asked_to_choose_between_bare_domains() {
        // **Review found this arriving from the opposite direction to the last one.** Removing
        // the truncation so the *set* could see every company also handed every company to the
        // *gate* - including the ones nobody fetched, which have no name of their own:
        //
        // ```text
        // the reader is offered 6 choices:
        //    Company at https://c0.example/ (c0.example)
        //    ...
        //    c5.example (c5.example)
        // ```
        //
        // A question bounded by how many results a provider felt like returning, whose last
        // option is a domain repeated twice.
        let all: Vec<Hit> = (0..6)
            .map(|i| hit(&format!("https://c{i}.example/")))
            .collect();
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(&engine, "Notion", |url| async move {
            Some(format!(
                "# Company at {url}\n\nOne workspace for your notes."
            ))
        })
        .await;

        assert!(!derived.about_a_market, "one word read as a market");
        let Resolution::Ambiguous { candidates, .. } = derived.verdict else {
            panic!("six tied hosts did not ask")
        };
        assert_eq!(
            candidates.len(),
            NAMED,
            "the question is bounded by the provider rather than by us: {candidates:#?}"
        );
        for c in &candidates {
            assert_ne!(
                c.name, c.canonical_domain,
                "a candidate with no name of its own was offered as a choice: {c:?}"
            );
        }

        // **And the set still accounts for all six.** The two lists differ on purpose, so a fix
        // to either one that quietly re-joins them fails here.
        let mut accounted: Vec<&str> = derived
            .set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .chain(
                derived
                    .set
                    .set_aside
                    .iter()
                    .map(|(c, _)| c.canonical_domain.as_str()),
            )
            .collect();
        accounted.sort_unstable();
        assert_eq!(accounted.len(), 6, "{:#?}", derived.set);
        assert!(accounted.contains(&"c5.example"), "{accounted:?}");
    }

    #[tokio::test]
    async fn a_reason_counts_the_searches_that_were_sent_not_the_ones_that_answered() {
        // The same rule the score follows, carried into the sentence a reader reads. Two
        // searches agreeing out of three sent is agreement between two of three; an engine
        // outage must not turn it into "2 of the 2 searches returned it".
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://plausible.io/")]),
                Ok(vec![hit("https://plausible.io/pricing")]),
                Err(()),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, queried) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |_url| async { Some("# Plausible\nPrivacy-first website analytics.".to_owned()) },
        )
        .await;

        assert_eq!(queried.failed.len(), 1);
        assert_eq!(derived.set.members.len(), 1, "{:#?}", derived.set);
        let because = &derived.set.members[0].because;
        let crate::competitors::Because::Found { agreed, asked, .. } = because else {
            panic!("{because:?}")
        };
        assert_eq!(*agreed, 2);
        assert_eq!(*asked, 3, "an outage was counted as unanimity");
        assert!(
            because.sentence().contains("2 of the 3"),
            "{}",
            because.sentence()
        );
    }

    #[tokio::test]
    async fn a_description_that_produced_one_company_says_why_nobody_else_is_there() {
        // The same honest empty the seeded path gets, on the path that came first. One company
        // in a tool that promises a comparison is the same shape to a reader however it
        // happened, so it is the same function deciding it.
        let all = vec![hit("https://plausible.io/")];
        let engine = Canned {
            per_query: vec![Ok(all.clone()), Ok(all.clone()), Ok(all)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |_url| async { Some("# Plausible\nPrivacy-first website analytics.".to_owned()) },
        )
        .await;
        assert_eq!(derived.set.members.len(), 1, "{:#?}", derived.set);
        assert_eq!(
            derived.set.alone,
            Some(crate::competitors::NoRivals::NobodyHeldUp {
                sought: crate::competitors::Sought::CompaniesMatchingTheDescription
            }),
            "one company arrived with no reason for being alone"
        );

        // Two companies is a comparison, and carries no explanation for an absence.
        let both = vec![hit("https://plausible.io/"), hit("https://usefathom.com/")];
        let engine = Canned {
            per_query: vec![Ok(both.clone()), Ok(both.clone()), Ok(both)],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |_url| async { Some("# A company\nPrivacy-first website analytics.".to_owned()) },
        )
        .await;
        assert_eq!(derived.set.members.len(), 2);
        assert_eq!(derived.set.alone, None);
    }

    #[tokio::test]
    async fn one_word_typed_is_still_a_question_rather_than_a_set() {
        // The gate's own protection, unchanged: several products sharing a *name* is what
        // `FACT_CHECKING.md` 3.1 step 4 asks a reader about, and a set would answer it for them.
        let engine = Canned {
            per_query: vec![
                Ok(vec![
                    hit("https://notion.example/"),
                    hit("https://notionpress.example/"),
                ]),
                Ok(vec![
                    hit("https://notionpress.example/"),
                    hit("https://notion.example/"),
                ]),
                Ok(vec![
                    hit("https://notion.example/"),
                    hit("https://notionpress.example/"),
                ]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(&engine, "Notion", |url| async move {
            Some(if url.contains("press") {
                "# Notion Press\nSelf-publish your book.".to_owned()
            } else {
                "# Notion\nOne workspace for your notes.".to_owned()
            })
        })
        .await;

        assert!(!derived.about_a_market, "one word read as a market");
        assert!(matches!(derived.verdict, Resolution::Ambiguous { .. }));
    }

    #[tokio::test]
    async fn a_company_from_another_market_is_set_aside_and_named() {
        // A search for one market returning a company from another. It is excluded from the
        // comparison and **named**, because a competitor dropped in silence is the defect the
        // set exists to remove.
        let engine = Canned {
            per_query: vec![
                Ok(vec![
                    hit("https://plausible.io/"),
                    hit("https://notionpress.example/"),
                ]),
                Ok(vec![
                    hit("https://plausible.io/"),
                    hit("https://notionpress.example/"),
                ]),
                Ok(vec![
                    hit("https://plausible.io/"),
                    hit("https://notionpress.example/"),
                ]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) = for_description(
            &engine,
            "privacy-friendly website analytics",
            |url| async move {
                Some(if url.contains("press") {
                    "# Notion Press\nSelf-publish your book.".to_owned()
                } else {
                    "# Plausible\nPrivacy-first website analytics.".to_owned()
                })
            },
        )
        .await;

        let compared: Vec<&str> = derived
            .set
            .members
            .iter()
            .map(|m| m.candidate.canonical_domain.as_str())
            .collect();
        assert_eq!(compared, vec!["plausible.io"], "{:#?}", derived.set);
        assert_eq!(derived.set.set_aside.len(), 1);
        assert_eq!(derived.set.set_aside[0].0.name, "Notion Press");
        let crate::competitors::Aside::ElsewhereEntirely { ref looked_for } =
            derived.set.set_aside[0].1
        else {
            panic!("{:?}", derived.set.set_aside[0].1)
        };
        // **The words are named, so the exclusion can be judged rather than taken.**
        assert!(
            looked_for.contains(&"analytics".to_owned()),
            "{looked_for:?}"
        );
    }

    #[tokio::test]
    async fn a_description_matching_nobody_says_so_rather_than_picking() {
        let engine = Canned {
            per_query: vec![Ok(Vec::new()), Ok(Vec::new()), Ok(Vec::new())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (derived, _) =
            for_description(&engine, "something nobody sells", |_url| async { None }).await;
        match derived.verdict {
            Resolution::NothingFound { checked } => {
                // The queries are the auditable half of a negative — §5.4's rule that a
                // negative nobody can check is not a finding.
                assert_eq!(checked.len(), 3, "{checked:?}");
                assert!(checked.iter().all(|q| q.contains("something nobody sells")));
            }
            other => panic!("an empty market did not say so: {other:?}"),
        }
    }

    #[test]
    fn a_front_page_with_no_prose_still_names_the_company() {
        let (name, what) = naming("a.com", "# Acme\n## Pricing\n");
        assert_eq!(name, "Acme");
        assert!(what.is_empty(), "{what:?}");
    }

    #[test]
    fn a_front_page_with_no_heading_falls_back_to_the_host() {
        let (name, _) = naming("a.com", "Just some words with no heading at all here.");
        assert_eq!(name, "a.com");
    }

    /// A provider that answers from a list, so the round trips can be exercised with no
    /// network. The real engine has never been run against this — Docker was unavailable where
    /// this was built — which is the same limit `landscape search` carries and is stated in
    /// [BENCHMARKS.md](../../../docs/BENCHMARKS.md) Run 28 rather than left to be discovered.
    struct Canned {
        per_query: Vec<Result<Vec<Hit>, ()>>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl SourceProvider for Canned {
        fn name(&self) -> &str {
            "canned"
        }
        async fn search(&self, query: &Query) -> Result<Vec<Hit>, SearchError> {
            let mut asked = self.asked.lock().unwrap();
            let answer = self.per_query.get(asked.len()).cloned();
            asked.push(query.text.clone());
            match answer {
                Some(Ok(hits)) => Ok(hits),
                _ => Err(SearchError::Unreachable("no route to host".to_owned())),
            }
        }
    }

    #[tokio::test]
    async fn every_query_is_asked_and_the_hosts_are_grouped_across_them() {
        let engine = Canned {
            per_query: vec![
                Ok(vec![hit("https://usefathom.com/"), hit("https://g2.com/x")]),
                Ok(vec![hit("https://usefathom.com/pricing")]),
                Ok(vec![hit("https://plausible.io/")]),
            ],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, queried) = suggest(&engine, "privacy-friendly website analytics").await;
        assert!(queried.failed.is_empty());
        assert_eq!(engine.asked.lock().unwrap().len(), 3, "one round trip each");
        assert_eq!(found[0].host, "usefathom.com");
        assert_eq!(found[0].agreed, 2, "{found:#?}");
        assert!(
            found.iter().all(|f| f.host != "g2.com"),
            "a review site reached the list: {found:#?}"
        );
    }

    #[tokio::test]
    async fn a_query_that_did_not_complete_is_counted_and_the_rest_carry_on() {
        // A search failure is not an analysis failure, and it is also not nothing: a thinner
        // list is a different thing from a shorter market, and only the count can say which.
        let engine = Canned {
            per_query: vec![Ok(vec![hit("https://a.com/")]), Err(()), Err(())],
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, queried) = suggest(&engine, "anything at all").await;
        assert_eq!(queried.failed.len(), 2);
        assert_eq!(found.len(), 1);
        // Scored against three asked, not one answered. An outage is not unanimity.
        assert!(found[0].confidence < 0.5, "{found:#?}");
    }

    #[tokio::test]
    async fn a_blank_description_asks_no_engine_anything() {
        let engine = Canned {
            per_query: Vec::new(),
            asked: std::sync::Mutex::new(Vec::new()),
        };
        let (found, queried) = suggest(&engine, "   ").await;
        assert!(found.is_empty());
        assert!(queried.failed.is_empty());
        assert!(engine.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn a_description_carrying_searxng_control_tokens_is_disarmed() {
        // The freest text this system accepts, arriving from a stranger's text box. SearXNG
        // splits `q` on whitespace and reads `!!` as *redirect to the first result* before
        // anything is searched for, so the grammar in `queries` is what stands between a
        // description and somebody else's page.
        let queries = for_idea("analytics !! :fr <99 !google");
        assert_eq!(queries.len(), 3);
        for q in &queries {
            assert!(!q.text.contains("!!"), "{}", q.text);
            assert!(!q.text.contains(":fr"), "{}", q.text);
            assert!(!q.text.contains('<'), "{}", q.text);
            assert!(q.text.contains("analytics"), "{}", q.text);
        }
    }

    #[test]
    fn depth_counts_path_segments_and_ignores_the_query() {
        assert_eq!(depth("https://a.com"), 0);
        assert_eq!(depth("https://a.com/"), 0);
        assert_eq!(depth("https://a.com/pricing"), 1);
        assert_eq!(depth("https://a.com/blog/2026/05/post"), 4);
        assert_eq!(depth("https://a.com/pricing?utm=x"), 1);
    }
}
