//! One subject in, one report out.
//!
//! Every piece of the pipeline has been runnable on its own for weeks, and `landscape read`
//! has been printing what each of them decided. **That is a run log, not a report.** The
//! difference is not cosmetic:
//!
//! - A run log is ordered by *page*. A report is ordered by *question*, because that is what
//!   a reader came with.
//! - A run log's silence is invisible. A report's silence is a section carrying
//!   [`Coverage::note`] — `PRODUCT_SPEC.md` §4.3's "not found" treatment.
//! - A run log's facts have no citations. **Every [`Claim`] carries a source label and a
//!   verbatim quote**, and [`Report::every_claim_is_traceable`] refuses a report whose
//!   citation does not resolve.
//!
//! # What this does not do yet
//!
//! `PRODUCT_SPEC.md` §4 fixes nine sections, and this builds six — one per question kind that
//! discovery asks. The others (positioning, sentiment themes, market emphasis, the SWOT) are
//! **interpretation over sources this pipeline does not gather yet**, and a section that
//! exists but cannot be filled is worse than one that is honestly absent.
//!
//! [`Coverage::note`]: landscape_core::Coverage::note
//! [`Claim`]: landscape_core::Claim
//! [`Report::every_claim_is_traceable`]: landscape_core::Report::every_claim_is_traceable

use chrono::{DateTime, NaiveDate, Utc};
use landscape_core::{
    Change, Claim, Confidence, Coverage, Disposition, PageChanges, PageFeatures, PageIdentity,
    PagePricing, Report, Section, Source,
};
use landscape_discover::probes::Answers;

mod order;
mod render;
mod sections;
mod stages;
pub mod subject;

pub use order::{
    is_deterministic, may_yield_content, model_calls_for, plan, tally, Plan, PAGES_PER_QUESTION,
};
pub use render::is_publishable;
pub use sections::{title_for, SECTIONS};

/// Whether the caller still wants what this run is producing.
///
/// **A worker can be replaced while it is working.** The staleness sweep hands its row to
/// somebody else, every write it makes from then on is refused, and until this existed it kept
/// reading pages and calling a model for a report that would be thrown away — up to a hundred
/// and eighty seconds of the machine's only scarce resource, spent on nothing.
///
/// Returned from the progress callback because that is where the answer already is: the caller
/// finds out its claim is gone *by writing*, and the progress callback is the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    /// Carry on.
    Yes,
    /// Stop. Nobody is waiting for this any more.
    No,
}

/// Everything one run produced: the report, and the trace that made it.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// What a reader is shown.
    pub report: Report,
    /// What each of the six questions ended up with, including the ones with nothing.
    pub coverage: Vec<Coverage>,
    /// One entry per page the run touched, in the order it touched them.
    ///
    /// Kept because the pipeline's joins are still the thing most worth watching, and a
    /// report deliberately hides them. `landscape read` prints these; the product will not.
    pub pages: Vec<PageResult>,
    /// Whether the run was abandoned because the caller said [`Wanted::No`].
    ///
    /// The report is whatever had been assembled at that point, and **it should not be
    /// published**: it is a partial answer from a worker that has already been replaced.
    pub stopped_early: bool,
}

/// What happened to one page.
#[derive(Debug, Clone)]
pub struct PageResult {
    pub url: String,
    pub question: Answers,
    /// `None` when the page was never opened.
    pub words: Option<usize>,
    pub quality: Option<&'static str>,
    /// How many words of it the model was actually shown.
    pub window_words: Option<usize>,
    /// One line, in the words the CLI has printed since Run 5.
    pub summary: String,
    /// The facts, the dropped answers, the model errors.
    pub details: Vec<String>,
}

pub mod memo;

/// The three things a run is run **with**, as opposed to what it is asked about.
///
/// **Grouped for the reason [`Asked`] is**, and clippy said so again the moment the memo joined
/// them: eight positional arguments is where *"the right value in the wrong position"* starts,
/// and two of these three are now `&`-references to caches that look nothing alike and would
/// swap silently if they had the same type.
///
/// They also share a property [`Asked`] does not: **each is held for the life of the process and
/// shared by every reader.** That is the whole point of the two caches in here — a second one
/// shares nothing, so building one per run is not an inefficiency but a defect, and this shape
/// makes that visible at every call site.
#[derive(Debug, Clone, Copy)]
pub struct With<'a> {
    pub fetcher: &'a landscape_fetch::Fetcher,
    pub llm: &'a landscape_llm::LlamaClient,
    /// What a page has already been read to say. See [`memo`].
    pub memo: &'a memo::Extractions,
}

/// The prompt set this crate speaks. Bumped when any extractor's wording changes.
pub const PROMPT_VERSION: u32 = 1;

/// Everything *else* about how an answer is produced, versioned the same way.
///
/// **A prompt can be word-for-word identical and still mean something different.** The decoding
/// settings in `stages::decode` — temperature, seed, token budget — and the schemas the sampler
/// is constrained by are as much a part of the question as the wording, and review found the
/// hole this closes: `memo::Extractions` remembered an answer under `PROMPT_VERSION` alone, so
/// changing the temperature would have reused answers produced at the old one.
///
/// **Bump this when `decode()` changes, when an extraction schema changes shape, or when the
/// logic that judges an answer changes.** It is deliberately separate from [`PROMPT_VERSION`],
/// which the golden set and the report also carry: those describe what a model was *asked*, and
/// this describes how it was asked and what was done with the reply.
pub const EXTRACTION_VERSION: u32 = 1;

/// Discover, read and extract one company, and assemble what came out.
///
/// `now` and `today` are arguments rather than clock reads. A report states when it was
/// generated and what fell inside a 90-day window, and neither can be tested against a
/// function that asks the operating system.
pub async fn analyze(
    with: &With<'_>,
    budget: &landscape_fetch::Budget,
    origin: &str,
    now: DateTime<Utc>,
    today: NaiveDate,
    search: Option<&dyn landscape_search::SourceProvider>,
) -> Analysis {
    analyze_with(with, budget, origin, now, today, search, &mut |_| {
        Wanted::Yes
    })
    .await
}

/// What one run was asked to do, and with what.
///
/// **Grouped for the reason [`Reading`] is**, and clippy noticed before a reader did: this had
/// grown to eight positional arguments, four of which are `Option`s or timestamps, and *"the
/// right value in the wrong position"* is the failure that shape produces. It is a struct rather
/// than an `#[allow]` because the lint was correct.
pub struct Asked<'a> {
    pub origins: &'a [String],
    /// The clock, passed in. A report states when it was generated and what fell inside a
    /// 90-day window, and neither can be tested against a function that asks the machine.
    pub now: DateTime<Utc>,
    pub today: NaiveDate,
    /// The engine, when one is configured. `None` is the laptop default and changes nothing.
    pub search: Option<&'a dyn landscape_search::SourceProvider>,
    /// Set when a description was searched for rather than the companies being named.
    ///
    /// The report says so first, because it is the one thing a reader cannot check by reading
    /// further down the page: they never typed any of these names.
    pub set: Option<&'a landscape_search::competitors::Set>,
    /// What the market calls this, when the queries turned out to use different words.
    ///
    /// Rides here rather than being stamped on afterwards because the **partial** reports go
    /// to a reader too — `on_progress` fires after every page — and a line that appeared only
    /// on the finished report would tell somebody, ninety seconds in, that the run they had
    /// been watching had been about something else all along.
    pub interpreted: Option<&'a landscape_core::Interpreted>,
}

/// Written by hand because a `dyn SourceProvider` has no `Debug`, and the useful thing about an
/// engine in a log line is **which one** rather than its innards.
impl std::fmt::Debug for Asked<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Asked")
            .field("origins", &self.origins)
            .field("now", &self.now)
            .field("today", &self.today)
            .field(
                "search",
                &self.search.map(landscape_search::SourceProvider::name),
            )
            .field(
                "set",
                &self.set.map(landscape_search::competitors::Set::origins),
            )
            .field("interpreted", &self.interpreted.map(|i| &i.label))
            .finish()
    }
}

/// Several companies, one report.
///
/// **A profile of one company is not a competitive analysis.** Until this existed the pipeline
/// took the first site named in a prompt and dropped the rest, so `basecamp.com vs linear.app`
/// produced a report about Basecamp with nothing on the page saying the other had been ignored.
///
/// # Why this merges rather than generalizing `analyze_with`
///
/// One company is the unit that discovery, fetching and extraction are built around, and that
/// path is the one with every test behind it. This runs it once per subject and joins the
/// results, so nothing about reading a single company changes — and a bug in the joining cannot
/// reach into the reading.
///
/// # What joining means
///
/// Sections merge by question: *What it costs* holds every company's prices, which is what
/// makes the report a comparison rather than several reports stapled together. Source labels
/// are reassigned as they merge, because each run numbers its own sources from `S1` and two
/// runs would otherwise both claim it.
///
/// Progress is reported after every window of every subject, over the *whole* report so far, so
/// a reader watching sees the first company fill in while the second is still being read.
pub async fn analyze_many(
    with: &With<'_>,
    budget: &landscape_fetch::Budget,
    asked: &Asked<'_>,
    on_progress: &mut dyn FnMut(&Report) -> Wanted,
) -> Analysis {
    let &Asked {
        origins,
        now,
        today,
        search,
        set,
        interpreted,
    } = asked;
    let llm = with.llm;
    // The cap is applied here rather than in the parser, so what it costs can be said out
    // loud. Dropping the fourth company in silence would be the defect this feature exists to
    // remove, at a higher count.
    let (analyzing, dropped) = origins.split_at(origins.len().min(subject::MAX_SUBJECTS));
    let mut notes = if dropped.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            // The runs of spaces this line used to carry were a lost `\` continuation, and a
            // reader saw them. Found while fixing the same slip one note below.
            "Comparing the first {} sites named. Not analyzed: {}. Each company is its own \
             discovery, fetches and model calls, so more of them is a longer wait rather than a \
             bigger report - run them separately if you need all of these.",
            analyzing.len(),
            dropped.join(", ")
        )]
    };
    // **A reader who described a market never named any of these companies, and has to be told
    // so.** Every other note here is about what a run did; these are about *who the run is
    // about*, which is the one thing a reader cannot check by reading further down the page.
    // They go first for that reason, and in this order: who, then why, then who is missing.
    if let Some(set) = set {
        for note in subject::found_for_you(set, analyzing.len())
            .into_iter()
            .rev()
        {
            notes.insert(0, note);
        }
    }

    let origins = analyzing;

    let mut finished: Vec<Analysis> = Vec::with_capacity(origins.len());

    for origin in origins {
        let stopped = {
            let so_far = &finished;
            let mut merge_and_report = |partial: &Report| {
                on_progress(&joined(
                    so_far,
                    Some(partial),
                    origins,
                    notes.clone(),
                    llm,
                    now,
                    interpreted,
                ))
            };
            let one = analyze_with(
                with,
                budget,
                origin,
                now,
                today,
                search,
                &mut merge_and_report,
            )
            .await;
            let stopped = one.stopped_early;
            finished.push(one);
            stopped
        };
        // A revoked claim stops the whole report, not just the company being read: the run has
        // been given to somebody else, and the remaining subjects are the largest part of what
        // there is left to save.
        if stopped {
            break;
        }
    }

    let report = joined(&finished, None, origins, notes, llm, now, interpreted);
    Analysis {
        report,
        coverage: coverage_by_question(&finished),
        pages: finished.iter().flat_map(|a| a.pages.clone()).collect(),
        stopped_early: finished.iter().any(|a| a.stopped_early),
    }
}

/// One coverage record per question, however many companies were read.
///
/// **`Analysis::render` zips sections with coverage**, and the merged report has six sections
/// however many subjects it covers. Concatenating each company's six would leave the renderer
/// consuming the first company's and silently ignoring every other company's negative evidence
/// — so a section that found nothing would describe only the first site's attempts, which is
/// this project's honest-negative treatment failing in exactly the place it matters.
///
/// Attribution survives the merge because every [`landscape_core::Attempt`] carries the origin
/// it was tried against. It is **not** carried by the path: `attempts_for` stores `/pricing`,
/// not `https://basecamp.com/pricing`, because the note used to be read under a heading that
/// already named the company. I wrote the opposite here, and review checked the producer.
fn coverage_by_question(finished: &[Analysis]) -> Vec<Coverage> {
    let mut merged: Vec<Coverage> = Vec::new();
    for one in finished.iter().flat_map(|a| a.coverage.iter()) {
        match merged.iter_mut().find(|c| c.question == one.question) {
            Some(into) => {
                into.sources.extend(one.sources.iter().cloned());
                into.attempts.extend(one.attempts.iter().cloned());
                into.pages_read += one.pages_read;
                into.facts += one.facts;
            }
            None => merged.push(one.clone()),
        }
    }
    merged
}

/// One report from several, with source labels reassigned so none collides.
fn joined(
    finished: &[Analysis],
    in_flight: Option<&Report>,
    origins: &[String],
    notes: Vec<String>,
    llm: &landscape_llm::LlamaClient,
    now: DateTime<Utc>,
    // What the market calls this, when it turned out to call it something else. A parameter
    // rather than something stamped on afterwards, because the **partial** reports reach a
    // reader too — a line that appeared only at the end would tell somebody, ninety seconds
    // in, that what they had been watching had been about something else all along.
    interpreted: Option<&landscape_core::Interpreted>,
) -> Report {
    let mut sections: Vec<Section> = Vec::new();
    let mut sources: Vec<Source> = Vec::new();
    // From the companies this run set out to cover — the same reading as `Report::subjects`,
    // and for the same reason: a company that says nothing is still one of the two being
    // compared, and the one that spoke keeps its label.
    let several = origins.len() > 1;

    for report in finished.iter().map(|a| &a.report).chain(in_flight) {
        // Every source this report brought, renumbered from where the last one left off.
        let mut renamed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for source in &report.sources {
            let label = format!("S{}", sources.len() + 1);
            renamed.insert(source.label.clone(), label.clone());
            sources.push(Source {
                label,
                ..source.clone()
            });
        }

        for section in &report.sections {
            // **The lines a reader sees, which are not the merged `Coverage`.** Each company is
            // assembled on its own, so `to_section` rendered `/pricing (404)` while it was still
            // true that one company was all there was. Joining those strings leaves the web
            // report — which renders `section.checked` and never sees a `Coverage` — with two
            // identical paths and no way to tell whose gap is whose. Review found this after
            // the structured coverage had already been fixed, which is the lesson: the surface
            // a person looks at is its own assertion.
            let checked: Vec<String> = section
                .checked
                .iter()
                .map(|line| {
                    if several {
                        landscape_core::attributed(&report.subject, line)
                    } else {
                        line.clone()
                    }
                })
                .collect();

            let claims: Vec<Claim> = section
                .claims
                .iter()
                .map(|claim| Claim {
                    // A label with no new name is one the source list never had, which
                    // `Report::every_claim_is_traceable` refuses. Leaving it unchanged keeps
                    // that refusal working rather than inventing a label that resolves.
                    source_label: renamed
                        .get(&claim.source_label)
                        .cloned()
                        .unwrap_or_else(|| claim.source_label.clone()),
                    // **Stamped here rather than where the claim was built.** Each run's
                    // report knows the company it is about; the extractors do not need to, and
                    // putting it here means the one place that has to get it right is the one
                    // place with tests around it.
                    subject: report.subject.clone(),
                    ..claim.clone()
                })
                .collect();

            match sections.iter_mut().find(|s| s.key == section.key) {
                Some(existing) => {
                    existing.claims.extend(claims);
                    existing.checked.extend(checked);
                    existing.notes.extend(section.notes.iter().cloned());
                    if !existing.claims.is_empty() {
                        existing.status = landscape_core::SectionStatus::Populated;
                    }
                }
                None => {
                    let mut merged = section.clone();
                    merged.claims = claims;
                    merged.checked = checked;
                    sections.push(merged);
                }
            }
        }
    }

    // **Each company's own notes travel with it.** The budget note is written per company -
    // one may have had a second pricing page and the other not - and a merge that kept only
    // the caller's notes would drop the sentence saying what was left unread, which is the
    // half of the shorter wait a reader is owed.
    let mut notes = notes;
    for report in finished.iter().map(|a| &a.report).chain(in_flight) {
        for note in &report.notes {
            if !notes.contains(note) {
                notes.push(note.clone());
            }
        }
    }

    Report {
        subject: origins.join(", "),
        searched_as: origins.join(", "),
        generated_at: now,
        model_id: llm.base().to_owned(),
        prompt_version: PROMPT_VERSION,
        // The companies this report set out to cover, which is not the same list as the
        // companies that produced a claim — and the difference is the case that matters.
        subjects: origins.to_vec(),
        sections,
        sources,
        interpreted: interpreted.cloned(),
        notes,
    }
}

/// The same, reporting the report so far after every page.
///
/// **A report is assembled a section at a time and a reader waits ninety seconds for it.**
/// `PRODUCT_SPEC.md` §2.1A wants the pricing section on screen when pricing is done rather
/// than everything at the end, and this callback is where that becomes possible: the worker
/// writes each partial report to the store, and the API streams the difference.
///
/// The partial report is **complete in shape from the first call** — all six sections exist,
/// each carrying its coverage note until it has claims. A reader never sees a section appear
/// out of nowhere; they see one fill in.
pub async fn analyze_with(
    with: &With<'_>,
    budget: &landscape_fetch::Budget,
    origin: &str,
    now: DateTime<Utc>,
    today: NaiveDate,
    search: Option<&dyn landscape_search::SourceProvider>,
    on_progress: &mut dyn FnMut(&Report) -> Wanted,
) -> Analysis {
    let &With { fetcher, llm, memo } = with;

    // **Which model these answers will belong to, asked once and asked in parallel.** The memo
    // outlives `llama-server`, so a restart with a different model at the same address would
    // otherwise have this run reuse the previous model's answers and label them with the
    // current one's address — review's finding, and the failure this pipeline exists to
    // prevent, arriving through the cache.
    //
    // **Here rather than on the hit path**, which is the other half of review's finding: a page
    // already read must cost nothing at all, and asking the model who it is before answering
    // from memory would be a request. One metadata call per analysis, to a local process we
    // run, with its own two-second timeout — `LlamaClient::identity` explains why it does not
    // borrow the generous one prefill needs.
    //
    // An identity we could not read changes nothing. A question we could not ask is not
    // evidence that the answer changed, and forgetting on an outage would throw the memory away
    // in exactly the hour it is worth most.
    let found = landscape_discover::discover(fetcher, budget, origin).await;
    if let Some(identity) = llm.identity().await {
        if memo.serving(&identity) {
            tracing::info!(
                identity,
                "the model changed; what earlier readers were told is forgotten"
            );
        }
    }

    // **When each page is read, and how many are read at all** — `order`. The page that needs
    // no model goes first so the first thing on screen costs a fetch rather than a chain of
    // model calls, and each question is worth one page that does need one. The pages that are
    // left are named on the report rather than dropped in silence.
    let plan = order::plan(&found.sources);
    let mut notes: Vec<String> = plan.note().into_iter().collect();

    let mut so_far = SoFar::default();
    let mut stopped_early = false;

    {
        let reading = Reading {
            found: &found,
            origin,
            llm,
            budget,
            memo,
            now,
            notes: &notes,
        };
        for source in &plan.read {
            if read_one(
                fetcher,
                &reading,
                today,
                &ToRead::probed(source),
                &mut so_far,
                on_progress,
            )
            .await
                == Wanted::No
            {
                stopped_early = true;
                break;
            }
        }
    }

    // **Search runs last, and only for what is still empty.** `FACT_CHECKING.md` §3.3 puts it
    // after probes and says why: probes are deterministic, free and hit primary sources, so
    // search fills gaps rather than leading. Asking here rather than before the first page is
    // what makes that structural — the questions handed to it are computed from claims that
    // exist, so a company whose own pages answered everything costs no search at all.
    let gaps = so_far.unanswered();
    if worth_searching(stopped_early, &gaps) {
        let (admitted, failures) = search_for_gaps(search, origin, &gaps, &so_far.urls()).await;
        so_far.searched.extend(searched_pages(&admitted));
        {
            let reading = Reading {
                found: &found,
                origin,
                llm,
                budget,
                memo,
                now,
                notes: &notes,
            };
            for hit in &admitted {
                if read_one(
                    fetcher,
                    &reading,
                    today,
                    &ToRead::found(hit),
                    &mut so_far,
                    on_progress,
                )
                .await
                    == Wanted::No
                {
                    stopped_early = true;
                    break;
                }
            }
        }
        // **After the reading, from what the reading did.** The first version wrote this note
        // straight after admission and said every admitted page *"was read"* — so a page that
        // failed to fetch, or was too thin to open, or was never reached because the reader
        // walked away, was reported as read anyway. Review found it, and the fix is not more
        // careful wording: it is computing the sentence from `so_far` rather than from a list
        // of intentions.
        notes.extend(note_for(origin, &gaps, &admitted, &so_far, failures));
    }

    let reading = Reading {
        found: &found,
        origin,
        llm,
        budget,
        memo,
        now,
        notes: &notes,
    };
    let (report, coverage) = assemble(
        &reading,
        &so_far.claims,
        &so_far.sources,
        &so_far.opened,
        &so_far.searched_evidence(),
    );
    Analysis {
        report,
        coverage,
        pages: so_far.pages,
        stopped_early,
    }
}

/// A page to read, and what a claim from it may be used for.
///
/// **The two travel together because separating them is how one gets the other's answer.**
/// Entry 7 of the register is a value parted from its evidence; a URL and its disposition are
/// exactly that pair, and passing them as two arguments to [`read_one`] let a mutation label
/// every page search found as the company's own without a single test noticing.
struct ToRead {
    candidate: landscape_discover::rank::Candidate,
    disposition: Disposition,
}

impl ToRead {
    /// A page discovery reached, at the standing the route it was reached by earns.
    ///
    /// **Every route but one is the subject's own domain**, and a page the company serves is
    /// `Primary` — the only class permitted to set a value in a comparison table. The exception
    /// is an applicant-tracking board: the bytes come from a stranger's host, so it is not the
    /// company's own page, but the company's own careers page is what named it, so authorship is
    /// not in doubt either. That is `Attributed`, exactly — and reading it any higher would
    /// widen what `Primary` means in order to fit a feature into it.
    fn probed(candidate: &landscape_discover::rank::Candidate) -> Self {
        Self {
            candidate: candidate.clone(),
            disposition: match candidate.via {
                landscape_discover::rank::Via::Board => Disposition::Attributed,
                _ => Disposition::Primary,
            },
        }
    }

    /// A page a search engine returned, at whatever standing admission gave it.
    fn found(hit: &landscape_search::Found) -> Self {
        Self {
            candidate: hit.to_candidate(),
            disposition: hit.disposition,
        }
    }
}

/// Whether a run should spend anything more on searching.
///
/// **Both refusals in one place, with one reason each.** A run whose reader has walked away is
/// not owed more network — the progress callback already said [`Wanted::No`], and search is the
/// most expensive thing left. A run with no gaps has nothing to ask about, which is
/// `FACT_CHECKING.md` §3.3's ordering made structural: the gaps are computed from claims that
/// exist, so a company whose own pages answered everything costs no search at all.
const fn worth_searching(stopped_early: bool, gaps: &[Answers]) -> bool {
    !stopped_early && !gaps.is_empty()
}

/// The most pages one run will read on top of the ones discovery planned.
///
/// Three. Every one is a fetch and, for four of the six questions, a model call per window —
/// spent *after* a reader has already waited for the whole plan. The number is small because
/// the wait is the thing search is most able to damage, and it is a named constant because
/// raising it is a decision about that wait rather than a tuning detail.
pub const PAGES_FROM_SEARCH: usize = 3;

/// Ask a search engine for the questions nothing filled, and say what came of it.
///
/// Returns the pages worth reading and, when there is something a reader needs to know, one
/// note for the report. **The note is the point as much as the pages are**: a reader cannot
/// otherwise tell a company with nothing published from a run that never asked, and those are
/// different facts about the world.
async fn search_for_gaps(
    search: Option<&dyn landscape_search::SourceProvider>,
    origin: &str,
    gaps: &[Answers],
    already: &[String],
) -> (Vec<landscape_search::Found>, Failures) {
    let Some(engine) = search else {
        // Not an error and not silence. A laptop with no `SEARX_URL` is the common case, and
        // the honest thing to say is that these gaps were not searched for rather than
        // letting them read as gaps somebody looked into.
        return (Vec::new(), Failures::NoEngine);
    };

    let Ok(target) = landscape_fetch::Target::parse(origin) else {
        return (Vec::new(), Failures::NoEngine);
    };
    // **The host, and it is a placeholder.** Turning a description into a company's name is
    // entity resolution, which is its own piece of work; `landscape search` uses the same
    // default and labels it the same way. A wrong name here costs a search that returns
    // nothing useful, which the admission step then admits nothing from.
    let queries = landscape_search::queries::for_questions(&target.host, gaps);

    let mut results = Vec::with_capacity(queries.len());
    let mut faults: Vec<landscape_search::Fault> = Vec::new();
    for query in queries {
        match engine.search(&query).await {
            Ok(hits) => results.push((query, hits)),
            Err(e) => {
                // A search that fails is not an analysis that fails. The section keeps the
                // coverage note it already had, and the count is reported rather than logged.
                tracing::warn!(query = %query.text, error = %e, "a search did not complete");
                faults.push(e.fault());
            }
        }
    }

    let admitted = landscape_search::admit::admit(
        &target.host,
        already,
        &results,
        engine.name(),
        PAGES_FROM_SEARCH,
    );
    (
        admitted,
        Failures::Asked {
            failures: faults.len(),
            fault: landscape_search::Fault::worst_of(faults),
        },
    )
}

/// A page search found, and what became of it.
///
/// **Both halves, because the checked evidence needs both.** A URL alone cannot say whether we
/// opened it, and `Coverage` renders *"found and not read"* and *"read, none stated anything"*
/// as different findings.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Searched {
    question: Answers,
    url: String,
    read: bool,
}

/// Every admitted page paired with the question it was admitted for.
///
/// **Named rather than inlined, because the wiring is the part with no test otherwise.** The
/// mutation that dropped this step left every coverage row built from discovery alone — the
/// defect review found — and nothing failed, because the test for it handed `assemble` a list
/// built by the test rather than by the run.
fn searched_pages(admitted: &[landscape_search::Found]) -> Vec<(Answers, String)> {
    admitted
        .iter()
        .map(|hit| (hit.answers, hit.url.clone()))
        .collect()
}

/// Whether an engine was asked at all, and how many of the asks did not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failures {
    /// Nothing was asked, because nothing could be. Different from *asked and got nothing*.
    NoEngine,
    /// Asked, and this many did not come back — with what a reader would do about it.
    ///
    /// **The count alone said the same thing about three different situations.** An engine
    /// that answers and refuses produces the same number as one that timed out, and the note
    /// a reader was left with pointed at neither.
    Asked {
        failures: usize,
        fault: Option<landscape_search::Fault>,
    },
}

/// What the report says about a search, in the words `FACT_CHECKING.md` §3.2.5 allows.
///
/// The subject of every sentence is us: what we looked for, what we could read, and how far we
/// got with it. Nothing here characterizes a publisher.
///
/// **Every count comes from `so_far`**, which is the record of what happened, rather than from
/// the list of pages somebody meant to read.
///
/// **And it names the company.** `analyze_many` merges each subject's notes into one report and
/// drops duplicates, so two companies with the same gaps would have collapsed into a single
/// sentence saying *"this company"* — ambiguous where it matters most, and silently dropping one
/// of the two.
fn note_for(
    origin: &str,
    gaps: &[Answers],
    admitted: &[landscape_search::Found],
    so_far: &SoFar,
    failures: Failures,
) -> Option<String> {
    let who = origin
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if failures == Failures::NoEngine {
        return Some(format!(
            "{who}: {} question(s) were not answered by this company's own pages and no search \
             engine is configured, so nothing further was looked for: {}.",
            gaps.len(),
            named(gaps)
        ));
    }

    let mut said = format!(
        "{who}: {} question(s) were not answered by this company's own pages, so a search engine \
         was asked for them: {}.",
        gaps.len(),
        named(gaps)
    );

    let read: Vec<&landscape_search::Found> = admitted
        .iter()
        .filter(|hit| so_far.was_read(&hit.url))
        .collect();
    let primary = read
        .iter()
        .filter(|f| f.disposition == Disposition::Primary)
        .count();
    let unverified = read.len() - primary;

    if read.is_empty() {
        said.push_str(" It returned nothing we could read.");
    } else {
        said.push_str(&format!(" {} page(s) were read:", read.len()));
        if primary > 0 {
            said.push_str(&format!(" {primary} on this company's own site"));
        }
        if unverified > 0 {
            said.push_str(&format!(
                "{} {unverified} elsewhere, which we were unable to attribute and which are \
                 labeled unverified",
                if primary > 0 { "," } else { "" }
            ));
        }
        said.push('.');
    }

    // Admitted and not read: a fetch that failed, a page too thin to open, or a run the reader
    // walked away from before we reached it. Counted rather than folded into the number above,
    // because "we read it and it said nothing" and "we never opened it" are different findings.
    let unread = admitted.len() - read.len();
    if unread > 0 {
        said.push_str(&format!(
            " {unread} further page(s) were found and not read."
        ));
    }
    if let Failures::Asked {
        failures: n,
        fault: Some(fault),
    } = failures
    {
        // `fault` is `Some` exactly when something failed, so the count is never zero here —
        // the two travel together rather than being checked apart.
        said.push_str(&format!(
            " {n} search(es) did not complete, so those questions were not searched for. {}",
            fault.advice()
        ));
    }
    Some(said)
}

/// A list of question names a reader can read.
fn named(questions: &[Answers]) -> String {
    questions
        .iter()
        .map(|q| q.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What one company's read has accumulated.
///
/// **Grouped so that a second reading pass appends to the same thing the first did.** The
/// alternative — a search pass with its own copy of the labeling, the citing and the progress
/// reporting — is the second source of truth this project keeps deleting, and it would drift on
/// the first change to either.
#[derive(Default)]
struct SoFar {
    pages: Vec<PageResult>,
    sources: Vec<Source>,
    claims: Vec<(Answers, Claim)>,
    opened: Vec<(Answers, usize)>,
    /// **Asked for at most once, and not until a page needs it.** Health is one request against
    /// the same client, and that client waits 180 seconds - the whole report budget - because
    /// prefill on four ARM cores is slow. An endpoint that accepts the connection and never
    /// answers would therefore have spent the entire wait *before the changelog was fetched*,
    /// which is this feature's central guarantee failing in exactly the case it is for: the
    /// model being unhealthy. Review found it. `None` means nobody has needed to know yet.
    model_ready: Option<bool>,
    /// Every page search admitted, and the question it was admitted for.
    ///
    /// **Coverage is built from discovery, and search is the first thing that ever added a
    /// page discovery had not.** Without this a question search read a page for reported
    /// *"nothing was checked - our gap, not theirs"* — the report telling a reader we did not
    /// look, immediately after looking. Review found it.
    ///
    /// Admitted rather than read, deliberately, because that is what `Coverage::sources` means
    /// everywhere else: `pages_read` is the other number, and the gap between them is what
    /// *"found and not read"* is built from.
    searched: Vec<(Answers, String)>,
}

impl SoFar {
    /// The questions no claim has filled.
    ///
    /// **Not the questions discovery found no page for**, which is the weaker measure and the
    /// one `landscape search` prints: a page can be admitted, fetched, read, and state nothing.
    /// [BENCHMARKS.md](../../../docs/BENCHMARKS.md) Run 26 named the difference and said the
    /// sharper number was not available until an analysis did the asking. It is here.
    fn unanswered(&self) -> Vec<Answers> {
        sections::SECTIONS
            .iter()
            .map(|(question, _)| *question)
            .filter(|question| !self.claims.iter().any(|(q, _)| q == question))
            .collect()
    }

    /// Whether a page was actually opened and handed to an extractor.
    ///
    /// `window_words` is set on exactly that path and left `None` by all three refusals — a
    /// fetch that failed, a page below the quality floor, and a model that was not ready. A
    /// page the run never reached is not in `pages` at all.
    fn was_read(&self, url: &str) -> bool {
        self.pages
            .iter()
            .any(|p| p.url == url && p.window_words.is_some())
    }

    /// What search found, and what became of each one.
    ///
    /// **Derived here rather than recorded as it happens.** The outcome of a page changes when
    /// it is read, and a field updated in two places is the second source of truth this crate
    /// keeps deleting. `pages` is the record; this reads it.
    fn searched_evidence(&self) -> Vec<Searched> {
        self.searched
            .iter()
            .map(|(question, url)| Searched {
                question: *question,
                url: url.clone(),
                read: self.was_read(url),
            })
            .collect()
    }

    /// Every URL this run has already touched, so search does not re-admit one.
    fn urls(&self) -> Vec<String> {
        self.pages.iter().map(|p| p.url.clone()).collect()
    }
}

/// Read one page for the question it was admitted for, and fold what it produced in.
///
/// **One path, two callers.** A probe's page and a search hit's page differ in exactly two
/// things — where the URL came from, and what a claim from it may be used for — and both are
/// arguments. Everything else, including the labeling, the citing and the per-window progress,
/// happens once.
/// Whether this page can be read yet, given what the run already knows about the model.
///
/// `None` means *nobody has asked* — the caller asks, remembers the answer, and every later page
/// gets it for free, so a run makes at most one health request and only once something needs it.
///
/// **Pulled out of the loop so the interesting case can be asserted.** Inside `read_one` the
/// only way to reach this branch is a real fetch, and the fetcher refuses `127.0.0.1`, so no
/// test in this crate gets here — the mutation that deletes the deterministic arm survived for
/// exactly that reason. The case that matters is the second one below: **the model is known to
/// be down and a changelog is still read**, which is the whole of why `ARCHITECTURE.md` §5.4
/// separates parsing from generation.
#[must_use]
pub(crate) const fn can_read(question: Answers, model_ready: Option<bool>) -> Option<bool> {
    if order::is_deterministic(question) {
        return Some(true);
    }
    model_ready
}

async fn read_one(
    fetcher: &landscape_fetch::Fetcher,
    reading: &Reading<'_>,
    today: NaiveDate,
    page_to_read: &ToRead,
    so_far: &mut SoFar,
    on_progress: &mut dyn FnMut(&Report) -> Wanted,
) -> Wanted {
    let ToRead {
        candidate,
        disposition,
    } = page_to_read;
    let (question, disposition) = (candidate.answers, *disposition);
    let url = candidate.url.clone();
    let (llm, now, origin) = (reading.llm, reading.now, reading.origin);

    // **Every question has an extractor now**, so the branch that used to skip a page here is
    // gone rather than left in as a wildcard nothing could reach. `stages::extract` matches all
    // six with no `_` arm, which makes a seventh question a build error.
    let page = match fetcher.get(&url, reading.budget).await {
        Ok(page) => page,
        Err(why) => {
            // **Our own bound and a stranger's failure are different findings.** A reader who is
            // told *"could not fetch"* about a page this run simply declined to ask for would go
            // and check the page by hand, find it fine, and stop believing the rest of the
            // report. `landscape_fetch::budget` is where that number comes from.
            let summary = why_not_read(&why);
            so_far.pages.push(PageResult {
                url,
                question,
                words: None,
                quality: None,
                window_words: None,
                summary,
                details: Vec::new(),
            });
            return Wanted::Yes;
        }
    };
    let markdown = landscape_extract::markdown::from_body(&page.body);
    let assessment = landscape_extract::quality::assess(&markdown);

    if !assessment.quality.worth_extracting() {
        so_far.pages.push(PageResult {
            url,
            question,
            words: Some(assessment.words),
            quality: Some(assessment.quality.name()),
            window_words: None,
            summary: "skipped - nothing to read".to_owned(),
            details: Vec::new(),
        });
        return Wanted::Yes;
    }

    let label = format!("S{}", so_far.sources.len() + 1);
    let cite = |title: String| Source {
        label: label.clone(),
        url: url.clone(),
        title,
        disposition,
        fetched_at: now,
        // **The subject's group is the subject; a stranger's group is its own host.** Two
        // sources in one group are not independent of each other, so filing a third-party page
        // under the company's group would say the company and its reviewer are one voice.
        independence_group: independence_group(origin, &url, disposition),
    };

    // **Everything that decides what the extractor will say**, built before it is asked. The
    // markdown is in here rather than a digest of it — see `memo` for why a collision is the one
    // failure this pipeline exists to prevent.
    let already_read = memo::Read::of(question, &url, &markdown, today);

    // **The second reader does not pay the model.** No progress is emitted on a hit, and there
    // is nothing to emit it for: the per-window callback exists so a four-minute page does not
    // look like a hung screen, and a page that costs nothing does not hang. Everything the
    // reader sees is assembled from the returned outcome at the end of this function, exactly
    // as it is on a miss.
    let Some(outcome) = memo::remembering(reading.memo, already_read, || async {
        // **The model is asked about only on a miss, and health is part of asking.** Review
        // found this: the readiness gate used to sit above the lookup, so a page whose answer
        // was already held came back as `(no model)` during an outage — the cache failing in
        // precisely the hour it exists for. A hit now makes no request of any kind.
        //
        // A changelog is parsed, not generated, so it is read whether or not a model is
        // running — ARCHITECTURE §5.4. Everything else needs one, and this is the first place
        // that is true, so this is where the question is finally asked.
        let ready = match can_read(question, so_far.model_ready) {
            Some(known) => known,
            None => {
                let asked = llm.is_ready().await;
                so_far.model_ready = Some(asked);
                asked
            }
        };
        if !ready {
            return None;
        }

        let (claims, sources) = (&so_far.claims, &so_far.sources);
        // **This page, counted as opened, for the partial reports only.** The real push happens
        // once below, on both paths, after there is an outcome — because *"read, and it said
        // nothing"* and *"never opened"* are different findings and a page the model could not
        // be asked about is the second one.
        let opened = &{
            let mut so_far_and_this = so_far.opened.clone();
            so_far_and_this.push((question, 1));
            so_far_and_this
        };
        let searched = so_far.searched_evidence();
        let label = label.clone();
        let cite = &cite;
        let mut emit = |partial: &[Finding]| {
            let mut with_partial: Vec<(Answers, Claim)> = claims.clone();
            with_partial.extend(partial.iter().map(|f| {
                (
                    question,
                    Claim {
                        text: f.text.clone(),
                        subject: String::new(),
                        source_label: label.clone(),
                        evidence_quote: f.quote.clone(),
                        confidence: f.confidence,
                        as_of: f.as_of.unwrap_or(now),
                    },
                )
            }));
            // The source is cited from the first claim that needs it, so nothing a reader sees
            // mid-run points at a label the source list does not have.
            let mut with_source = sources.to_vec();
            if !partial.is_empty() && !with_source.iter().any(|s| s.label == label) {
                with_source.push(cite(page_title(&markdown, &url)));
            }
            let (report, _) = assemble(reading, &with_partial, &with_source, opened, &searched);
            on_progress(&report)
        };
        Some(stages::extract(llm, question, &url, &markdown, today, &mut emit).await)
    })
    .await
    else {
        so_far.pages.push(PageResult {
            url,
            question,
            words: Some(assessment.words),
            quality: Some(assessment.quality.name()),
            window_words: None,
            summary: "(no model)".to_owned(),
            details: Vec::new(),
        });
        return Wanted::Yes;
    };
    so_far.opened.push((question, 1));

    for text in outcome.claims {
        so_far.claims.push((
            question,
            Claim {
                text: text.text,
                subject: String::new(),
                source_label: label.clone(),
                evidence_quote: text.quote,
                confidence: text.confidence,
                as_of: text.as_of.unwrap_or(now),
            },
        ));
    }
    so_far.pages.push(PageResult {
        url: url.clone(),
        question,
        words: Some(assessment.words),
        quality: Some(assessment.quality.name()),
        window_words: Some(outcome.window_words),
        summary: outcome.summary,
        details: outcome.details,
    });
    // Only pages that produced something are cited. A source list is what a reader clicks
    // through to; an entry that supports no claim is furniture.
    if so_far.claims.iter().any(|(_, c)| c.source_label == label) {
        so_far.sources.push(cite(page_title(&markdown, &url)));
    }

    // One page done. Whoever is waiting can have what exists so far.
    let (report, _) = assemble(
        reading,
        &so_far.claims,
        &so_far.sources,
        &so_far.opened,
        &so_far.searched_evidence(),
    );
    on_progress(&report)
}

/// What to tell a reader about a page that was not read.
///
/// **Our own bound and a stranger's failure are different findings.** A reader told *"could not
/// fetch"* about a page this run simply declined to ask for would check it by hand, find it
/// fine, and stop believing the rest of the report. `landscape_fetch::budget` is where the
/// number in that sentence comes from, and it is in the sentence because a bound nobody can see
/// reads as a bug.
///
/// A function rather than a `match` inside `read_one`, which needs a fetched page and therefore
/// cannot be driven by a test at all — the same reason `memo::remembering` exists.
fn why_not_read(why: &landscape_fetch::FetchError) -> String {
    match why {
        landscape_fetch::FetchError::BudgetSpent { limit } => {
            format!("not read - this run had already made its {limit} requests")
        }
        _ => "could not fetch".to_owned(),
    }
}

/// Which sources are not independent of each other.
///
/// The subject's own pages are one voice, however many of them are read. A page on somebody
/// else's host is its own voice — filing it under the subject's group would let a future
/// corroboration count treat a reviewer and the company as the same source, in whichever
/// direction happened to be convenient.
fn independence_group(origin: &str, url: &str, disposition: Disposition) -> String {
    if disposition == Disposition::Primary {
        return origin.to_owned();
    }
    landscape_fetch::Target::parse(url).map_or_else(|_| origin.to_owned(), |t| t.host)
}

/// Build the report from what is known so far.
///
/// Called after every page as well as at the end, which is the point: a partial report and a
/// finished one are the same shape, and nothing downstream needs to know which it has.
/// What does not change while one company is being read.
///
/// Grouped because [`assemble`] is called four times a run - after every window, after every
/// page, and at the end - and threading seven arguments through each of those is how a caller
/// ends up passing the right value in the wrong position.
struct Reading<'a> {
    found: &'a landscape_discover::Discovered,
    origin: &'a str,
    llm: &'a landscape_llm::LlamaClient,
    /// What is left of this analysis's allowance of requests to strangers. **Per analysis, not
    /// per process**, unlike everything in [`With`]: it exists to bound one reader's question,
    /// and sharing it would let a quiet afternoon pay for a busy one.
    budget: &'a landscape_fetch::Budget,
    /// What a page has already been read to say. Beside the model rather than beside the
    /// clock, because the two together are what an extraction costs.
    memo: &'a memo::Extractions,
    now: DateTime<Utc>,
    /// Anything true of the whole report - today, the pages the budget left unread.
    notes: &'a [String],
}

fn assemble(
    reading: &Reading<'_>,
    claims: &[(Answers, Claim)],
    sources: &[Source],
    opened: &[(Answers, usize)],
    searched: &[Searched],
) -> (Report, Vec<Coverage>) {
    let Reading {
        found,
        origin,
        llm,
        now,
        notes,
        ..
    } = *reading;
    let coverage: Vec<Coverage> = sections::SECTIONS
        .iter()
        .map(|(question, _)| {
            let read = opened.iter().filter(|(q, _)| q == question).count();
            let facts = claims.iter().filter(|(q, _)| q == question).count();
            let mut coverage = found.coverage(*question, read, facts);
            // Discovery's pages, then search's. Both were admitted for this question, and a
            // note that counted only the first would describe a run that did not happen.
            let found_by_search = searched.iter().filter(|s| s.question == *question);
            for page in found_by_search {
                coverage.sources.push(page.url.clone());
                // **And into the checked evidence, which is the half a reader sees.** Review
                // found the first fix stopping at `sources`: the note read *"read 1 page(s),
                // none stated anything. Checked: nothing"*, so the one page that was opened
                // appeared nowhere a reader could go and look. `sources` is a count; `attempts`
                // is the list.
                coverage.attempts.push(landscape_core::Attempt {
                    // **The whole URL, not a path.** A probe's path is retypeable because the
                    // heading says whose site it is; a page on somebody else's host is not the
                    // subject's, and the host is the part that matters about it.
                    path: page.url.clone(),
                    outcome: if page.read { "read" } else { "not read" }.to_owned(),
                    subject: origin.to_owned(),
                });
            }
            coverage
        })
        .collect();

    let sections: Vec<Section> = coverage
        .iter()
        .zip(sections::SECTIONS.iter())
        .map(|(coverage, (question, title))| {
            let mut section = coverage.to_section(*title);
            section.claims = claims
                .iter()
                .filter(|(q, _)| q == question)
                .map(|(_, c)| c.clone())
                .collect();
            section
        })
        .collect();

    let report = Report {
        subject: origin.to_owned(),
        searched_as: origin.to_owned(),
        generated_at: now,
        // The server does not name its model over HTTP, so what is recorded is where the
        // answers came from. A report that cannot say which model wrote it is a report
        // nobody can reproduce, and this is the honest half of that.
        model_id: llm.base().to_owned(),
        prompt_version: PROMPT_VERSION,
        subjects: Vec::new(),
        sections,
        sources: sources.to_vec(),
        interpreted: None,
        notes: notes.to_vec(),
    };
    (report, coverage)
}

/// A page's own title, or its path if it has none.
fn page_title(markdown: &str, url: &str) -> String {
    markdown
        .lines()
        .find(|l| l.trim_start().starts_with("# "))
        .map(|l| l.trim().trim_start_matches('#').trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| url.to_owned())
}

/// One fact on its way to becoming a [`Claim`].
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub text: String,
    pub quote: String,
    pub confidence: Confidence,
    /// When the *source* says it happened, for a dated change. `None` means "when we read it".
    pub as_of: Option<DateTime<Utc>>,
}

/// Whether an outcome is the whole answer to the question that was asked.
///
/// **This exists so a failure is never remembered.** A model error, or a run the reader stopped
/// waiting for, produces an outcome that is real, reportable and *incomplete* — and
/// [`memo::Extractions`] must not keep one, because a cached failure is replayed to every later
/// reader and nothing ever goes back to ask again. `landscape_fetch` was corrected for exactly
/// that one row earlier, over HTTP statuses; this is the same rule where the cost is a model.
///
/// A deterministic extractor is always `Complete`: there is nothing in a changelog parse that
/// can half-happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Settled {
    /// Every window was read and every answer came back.
    Complete,
    /// Something failed, or nobody was still waiting. Reportable, not reusable.
    Partial,
}

/// What one page's extractor produced.
#[derive(Debug, Clone)]
pub(crate) struct Outcome {
    pub claims: Vec<Finding>,
    pub summary: String,
    pub details: Vec<String>,
    pub window_words: usize,
    /// Whether this is worth remembering. See [`Settled`].
    pub settled: Settled,
}

/// Turn the four assembled page types into claims. Kept here so the wording of a claim is in
/// one place rather than three.
pub(crate) fn claims_from_pricing(page: &PagePricing) -> Vec<Finding> {
    page.plans
        .iter()
        .map(|plan| {
            let name = plan.plan_name.as_deref().unwrap_or("an unnamed plan");
            let text = match plan.price_usd {
                Some(price) => format!("{name} costs ${price}"),
                None => format!("{name} is listed with no published price"),
            };
            Finding {
                text,
                quote: plan.evidence_quote.clone().unwrap_or_default(),
                // A price is published to be read, and the quote is verbatim or the run said
                // so. This is the strongest kind of fact this pipeline produces.
                confidence: Confidence::High,
                as_of: None,
            }
        })
        .collect()
}

pub(crate) fn claims_from_features(page: &PageFeatures) -> Vec<Finding> {
    page.features
        .iter()
        .map(|feature| Finding {
            text: format!(
                "states it offers {}",
                feature
                    .capability
                    .as_deref()
                    .unwrap_or("an unnamed capability")
            ),
            quote: feature.evidence_quote.clone().unwrap_or_default(),
            // A capability name is a paraphrase by design — the model's job here is to
            // shorten a heading — so it can never carry the confidence a quoted price does.
            confidence: Confidence::Medium,
            as_of: None,
        })
        .collect()
}

/// What a trust page states, in words that do not overclaim.
///
/// **The wording is the feature.** *"states SOC 2 Type II"* is a fact about the page; *"is SOC 2
/// certified"* is a fact about the world, and this pipeline reads pages. A standard the page
/// names without claiming is reported as exactly that — dropping it would turn "mentioned, not
/// claimed" into silence, and silence reads as "not mentioned".
pub(crate) fn claims_from_trust(page: &landscape_core::PageTrust) -> Vec<Finding> {
    page.assurances
        .iter()
        .map(|assurance| {
            let name = assurance
                .standard
                .as_deref()
                .unwrap_or("an unnamed standard");
            Finding {
                text: match assurance.status {
                    Some(status) => format!("{} {name}", status.wording()),
                    None => format!("names {name} without saying whether they hold it"),
                },
                quote: assurance.evidence_quote.clone().unwrap_or_default(),
                // The name came from the page by construction - the scanner found it there -
                // so the only judgment is the status, and that is a reading of a sentence.
                confidence: Confidence::Medium,
                as_of: None,
            }
        })
        .collect()
}

/// What a careers page advertises, in words that stay about the page.
///
/// **The wording is the feature, again.** *"lists an open role"* is a fact about a page; *"is
/// growing its engineering team"* is an interpretation, and this pipeline reads pages. The
/// investment signal is the list itself — four titles ending in *Engineer* and one in *Counsel*
/// — so the reader does the comparing and every line of it can be checked against the source.
pub(crate) fn claims_from_hiring(page: &landscape_core::PageHiring) -> Vec<Finding> {
    page.roles
        .iter()
        .map(|role| Finding {
            text: format!("lists an open role: {}", role.title),
            quote: role.evidence_quote.clone().unwrap_or_default(),
            // No model touched this. The title is the line, the line is on the page, and the
            // only way it could be wrong is if the parser read a heading as a vacancy - which
            // is what the frozen careers pages are for.
            confidence: Confidence::High,
            as_of: None,
        })
        .collect()
}

pub(crate) fn claims_from_changes(page: &PageChanges, today: NaiveDate) -> Vec<Finding> {
    page.recent(today)
        .into_iter()
        .map(|change: &Change| Finding {
            text: change
                .summary
                .clone()
                .unwrap_or_else(|| "an undescribed change".to_owned()),
            quote: change.evidence_quote.clone().unwrap_or_default(),
            confidence: Confidence::High,
            // The date the page states, not the date we read it. A changelog entry that is
            // three weeks old should say so.
            as_of: change
                .happened_on
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc()),
        })
        .collect()
}

pub(crate) fn claims_from_identity(page: &PageIdentity) -> Vec<Finding> {
    let mut out = Vec::new();
    // Each fact carries the quote it was read with. Pairing them by position put a founding
    // sentence under "based in the EU" — evidence for a claim it does not support.
    if let Some(founded) = &page.founded_year {
        out.push(Finding {
            text: format!("says it was founded in {}", founded.value),
            quote: founded.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    if let Some(place) = &page.headquarters {
        out.push(Finding {
            text: format!("says it is based in {}", place.value),
            quote: place.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    if let Some(people) = &page.employees {
        out.push(Finding {
            text: format!("says {} people work there", people.value),
            quote: people.quote_or_empty(),
            confidence: Confidence::High,
            as_of: None,
        });
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use landscape_core::{FeatureExtraction, PricingExtraction};

    #[test]
    fn a_priced_plan_becomes_a_claim_that_reads_as_a_sentence() {
        let page = PagePricing::assembled([PricingExtraction {
            plan_name: Some("Pro".to_owned()),
            price_usd: Some(15.0),
            billing_period: None,
            evidence_quote: Some("$15/user, billed monthly".to_owned()),
        }]);
        let claims = claims_from_pricing(&page);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].text, "Pro costs $15");
        assert_eq!(claims[0].quote, "$15/user, billed monthly");
    }

    #[test]
    fn a_plan_with_no_price_is_a_claim_about_the_absence() {
        // "Contact sales" is a fact about the company, and reporting it as nothing at all
        // would lose it.
        let page = PagePricing::assembled([PricingExtraction {
            plan_name: Some("Enterprise".to_owned()),
            ..PricingExtraction::empty()
        }]);
        assert!(claims_from_pricing(&page)[0]
            .text
            .contains("no published price"));
    }

    #[test]
    fn a_capability_is_never_as_confident_as_a_price() {
        // The name is a paraphrase by design — that is what the model is for here — so it
        // cannot carry the confidence a quoted number does.
        let page = PageFeatures::assembled(
            [FeatureExtraction {
                capability: Some("Message Boards".to_owned()),
                qualifier: None,
                evidence_quote: Some("## Message Boards".to_owned()),
            }],
            1,
        );
        assert_eq!(
            claims_from_features(&page)[0].confidence,
            Confidence::Medium
        );
    }

    #[test]
    fn a_change_is_dated_by_the_page_rather_than_by_the_run() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = PageChanges {
            changes: vec![Change {
                happened_on: NaiveDate::from_ymd_opt(2026, 7, 14),
                summary: Some("Shipped annotations".to_owned()),
                evidence_quote: Some("- Jul 14, 2026".to_owned()),
            }],
            considered: 1,
        };
        let claims = claims_from_changes(&page, today);
        assert_eq!(claims.len(), 1);
        assert_eq!(
            claims[0].as_of.map(|d| d.date_naive()),
            NaiveDate::from_ymd_opt(2026, 7, 14)
        );
    }

    #[test]
    fn a_change_outside_the_window_is_not_a_claim() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
        let page = PageChanges {
            changes: vec![Change {
                happened_on: NaiveDate::from_ymd_opt(2025, 1, 1),
                summary: Some("Ancient history".to_owned()),
                evidence_quote: None,
            }],
            considered: 1,
        };
        assert!(claims_from_changes(&page, today).is_empty());
    }

    #[test]
    fn each_identity_fact_is_its_own_claim() {
        use landscape_core::Stated;
        let page = PageIdentity {
            founded_year: Some(Stated {
                value: 2018,
                quote: Some("started Plausible in December 2018".to_owned()),
            }),
            headquarters: Some(Stated {
                value: "the EU".to_owned(),
                quote: Some("a company based in the EU".to_owned()),
            }),
            employees: Some(Stated {
                value: 10,
                quote: Some("a team of 10".to_owned()),
            }),
        };
        let claims = claims_from_identity(&page);
        assert_eq!(claims.len(), 3);
        assert!(claims[0].text.contains("2018"));
        assert!(claims[1].text.contains("the EU"));
        assert!(claims[2].text.contains("10 people"));
        // And each carries its own words, not the ones before it.
        assert_eq!(claims[1].quote, "a company based in the EU");
    }

    #[test]
    fn a_page_title_falls_back_to_the_url() {
        assert_eq!(page_title("# Pricing\n$15", "https://e.com/x"), "Pricing");
        assert_eq!(
            page_title("no heading", "https://e.com/x"),
            "https://e.com/x"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod trust_wording {
    //! What the report says about a compliance claim, which is the part that can overclaim.

    use landscape_core::{Assurance, PageTrust, TrustExtraction};

    fn one(standard: &str, status: Option<Assurance>) -> PageTrust {
        PageTrust::assembled(
            [TrustExtraction {
                standard: Some(standard.to_owned()),
                status,
                evidence_quote: Some("as the page says".to_owned()),
            }],
            1,
        )
    }

    #[test]
    fn the_report_says_what_the_page_states_not_what_is_true() {
        // *"states SOC 2 Type II"* is a fact about the page. *"is SOC 2 certified"* is a fact
        // about the world, and this pipeline reads pages.
        let claims = super::claims_from_trust(&one("SOC 2 Type II", Some(Assurance::Holds)));
        assert_eq!(claims[0].text, "states SOC 2 Type II");
        assert!(
            !claims[0].text.contains("certified"),
            "the report is asserting the certification rather than the claim: {}",
            claims[0].text
        );
    }

    #[test]
    fn a_roadmap_item_never_reads_as_a_certification() {
        // The defect this extractor exists to prevent. Both spellings contain "SOC 2".
        let pursuing = super::claims_from_trust(&one("SOC 2", Some(Assurance::Pursuing)));
        let holds = super::claims_from_trust(&one("SOC 2", Some(Assurance::Holds)));
        assert!(
            pursuing[0].text.contains("working toward"),
            "{}",
            pursuing[0].text
        );
        assert_ne!(pursuing[0].text, holds[0].text);
    }

    #[test]
    fn a_standard_named_without_a_claim_says_so() {
        let claims = super::claims_from_trust(&one("HIPAA", None));
        assert!(
            claims[0].text.contains("without saying"),
            "a mention became a claim: {}",
            claims[0].text
        );
    }

    #[test]
    fn every_trust_claim_carries_the_words_it_came_from() {
        // The same rule as every other claim: an unsourced sentence is not representable.
        let claims = super::claims_from_trust(&one("ISO 27001", Some(Assurance::Holds)));
        assert!(!claims[0].quote.is_empty());
    }

    fn hiring(title: &str) -> landscape_core::PageHiring {
        landscape_core::PageHiring {
            roles: vec![landscape_core::Role {
                title: title.to_owned(),
                evidence_quote: Some(title.to_owned()),
            }],
            considered: 1,
            announced: true,
        }
    }

    #[test]
    fn a_vacancy_is_reported_as_a_listing_and_not_as_a_plan() {
        // *"lists an open role"* is a fact about a page. *"is growing its engineering team"* is
        // an interpretation of one, and it is the sentence this section would be easiest to
        // write and hardest to defend.
        let claims = super::claims_from_hiring(&hiring("Senior / Staff Product Engineer"));
        assert_eq!(
            claims[0].text,
            "lists an open role: Senior / Staff Product Engineer"
        );
        assert!(
            !claims[0].text.contains("growing") && !claims[0].text.contains("investing"),
            "the claim reads as a conclusion: {}",
            claims[0].text
        );
    }

    #[test]
    fn a_vacancy_is_as_confident_as_a_price_because_no_model_saw_it() {
        // Pricing is High because the number is quoted; a capability is Medium because the
        // model paraphrased a heading. Nothing paraphrased this — the title *is* the line.
        let claims = super::claims_from_hiring(&hiring("Senior Counsel"));
        assert_eq!(claims[0].confidence, landscape_core::Confidence::High);
        assert_eq!(claims[0].quote, "Senior Counsel");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod joining {
    //! Merging several companies into one report.
    //!
    //! `analyze_many` itself needs a fetcher and a model, so what is asserted here is the join
    //! — which is where a bug would silently mislabel somebody's evidence.

    use super::*;
    use landscape_core::{Confidence, SectionStatus};

    fn at() -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-08-05T09:00:00Z")
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_default()
    }

    /// A one-company report: one source labeled `S1`, one claim citing it.
    fn one_company(origin: &str, says: &str) -> Analysis {
        let report = Report {
            subject: origin.to_owned(),
            searched_as: origin.to_owned(),
            generated_at: at(),
            model_id: "test".to_owned(),
            prompt_version: PROMPT_VERSION,
            subjects: Vec::new(),
            sections: vec![Section {
                key: "pricing".to_owned(),
                title: "What it costs".to_owned(),
                status: SectionStatus::Populated,
                claims: vec![Claim {
                    text: says.to_owned(),
                    subject: origin.to_owned(),
                    source_label: "S1".to_owned(),
                    evidence_quote: says.to_owned(),
                    confidence: Confidence::High,
                    as_of: at(),
                }],
                checked: vec![format!("{origin}/pricing")],
                notes: Vec::new(),
            }],
            notes: Vec::new(),
            sources: vec![Source {
                label: "S1".to_owned(),
                url: format!("{origin}/pricing"),
                title: "Pricing".to_owned(),
                disposition: Disposition::Primary,
                fetched_at: at(),
                independence_group: origin.to_owned(),
            }],
            interpreted: None,
        };
        Analysis {
            report,
            coverage: Vec::new(),
            pages: Vec::new(),
            stopped_early: false,
        }
    }

    fn llm() -> landscape_llm::LlamaClient {
        landscape_llm::LlamaClient::new("http://127.0.0.1:8080")
    }

    /// A run with nothing warm in it: no pages held, nothing read before.
    ///
    /// Leaks both caches on purpose. A test process is short, and the alternative is threading
    /// two owned values through every one of these tests to say the same thing.
    pub(crate) fn offline() -> With<'static> {
        With {
            fetcher: Box::leak(Box::new(landscape_fetch::Fetcher::new())),
            llm: Box::leak(Box::new(llm())),
            memo: Box::leak(Box::new(memo::Extractions::new())),
        }
    }

    #[test]
    fn a_board_is_read_at_the_standing_a_stranger_s_server_earns() {
        // **`Primary` means the company's own domain, and this feature does not widen it.**
        // A board's bytes come from somebody else's host, so it cannot set a value in a
        // comparison table — but the company's own careers page is what named it, so authorship
        // is not in doubt either, and that is exactly what `Attributed` is for.
        use landscape_discover::rank::{Candidate, Via};
        let board = Candidate {
            url: "https://jobs.ashbyhq.com/Linear".to_owned(),
            answers: Answers::Direction,
            via: Via::Board,
        };
        assert_eq!(ToRead::probed(&board).disposition, Disposition::Attributed);

        for via in [Via::LlmsTxt, Via::Sitemap, Via::Probe] {
            let own = Candidate {
                url: "https://linear.app/careers".to_owned(),
                answers: Answers::Direction,
                via,
            };
            assert_eq!(
                ToRead::probed(&own).disposition,
                Disposition::Primary,
                "{via:?}: a page the company serves is the company saying it"
            );
        }
    }

    #[test]
    fn a_page_this_run_declined_to_ask_for_does_not_read_as_a_broken_site() {
        // **The distinction a reader acts on.** *"Could not fetch"* sends somebody to check a
        // page that is fine; *"this run had already made its 64 requests"* tells them the run
        // stopped, which is our doing and is fixable by asking about fewer companies.
        let spent = why_not_read(&landscape_fetch::FetchError::BudgetSpent { limit: 64 });
        assert!(spent.contains("64"), "the bound is invisible: {spent}");
        assert!(!spent.contains("could not fetch"), "{spent}");

        assert_eq!(
            why_not_read(&landscape_fetch::FetchError::Transport("refused".into())),
            "could not fetch",
            "a site that really did fail must still say so"
        );
        assert_eq!(
            why_not_read(&landscape_fetch::FetchError::RobotsDisallowed {
                host: "e.com".to_owned(),
                path: "/pricing".to_owned(),
            }),
            "could not fetch"
        );
    }

    #[test]
    fn a_page_that_needs_no_model_is_read_while_the_model_is_down() {
        // **The property the whole read order exists for.** A changelog is parsed rather than
        // generated, so a model outage costs a reader the sections that need one and nothing
        // else. Both deterministic questions, both states of the model, and the important pair
        // is the one where `Some(false)` still says yes.
        for question in [Answers::Changes, Answers::Direction] {
            assert_eq!(can_read(question, None), Some(true), "{question:?}");
            assert_eq!(
                can_read(question, Some(false)),
                Some(true),
                "{question:?} waited on a model it does not use"
            );
        }
    }

    #[test]
    fn a_page_that_needs_a_model_asks_once_and_then_remembers() {
        // `None` is the run saying nobody has asked yet, which is what makes the health check
        // lazy - and what stops it happening at all when only changelogs are read.
        assert_eq!(can_read(Answers::Pricing, None), None);
        assert_eq!(can_read(Answers::Pricing, Some(true)), Some(true));
        assert_eq!(can_read(Answers::Pricing, Some(false)), Some(false));
    }

    #[test]
    fn one_section_holds_every_company() {
        // The thing that makes it a comparison rather than two reports stapled together: a
        // reader looking at "What it costs" sees both prices in one place.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );

        let pricing = merged
            .sections
            .iter()
            .find(|s| s.key == "pricing")
            .expect("one pricing section, not two");
        assert_eq!(merged.sections.len(), 1, "sections merged by question");
        assert_eq!(pricing.claims.len(), 2);
        assert!(pricing.claims.iter().any(|c| c.text == "Pro costs $15"));
        assert!(pricing
            .claims
            .iter()
            .any(|c| c.text == "Business costs $16"));
    }

    #[test]
    fn every_claim_still_points_at_its_own_source() {
        // **The defect this join could most easily introduce.** Each run numbers its sources
        // from `S1`, so merging without renaming gives two companies the same label — and a
        // reader following a citation for A's price arrives at B's pricing page. Evidence
        // attached to the wrong claim is the worst output this product can produce.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );

        assert!(
            merged.dangling_source_labels().is_empty(),
            "a citation that does not resolve: {:?}",
            merged.dangling_source_labels()
        );
        let labels: Vec<&str> = merged.sources.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["S1", "S2"],
            "labels are unique across companies"
        );

        // **Real claim text, deliberately.** The first version of this test invented
        // `A costs $10` and `B costs $20`, which name their company — and review pointed out
        // that this is exactly what hid the defect, because the real extractors produce
        // `Pro costs $15`: what the page says, and nothing about who said it.
        for claim in &merged.sections[0].claims {
            assert!(
                !claim.text.contains("a.com") && !claim.text.contains("b.com"),
                "the fixture names the company in the claim text, which no extractor does"
            );
            let source = merged
                .sources
                .iter()
                .find(|s| s.label == claim.source_label)
                .expect("every claim resolves");
            assert!(
                source.url.starts_with(&claim.subject),
                "{} says it is about {} and cites {}, which is another company's page",
                claim.text,
                claim.subject,
                source.url
            );
        }
    }

    #[test]
    fn a_company_still_being_read_is_part_of_the_report() {
        // What a reader watching sees: the first company's answers, plus whatever the second
        // has produced so far. Without the in-flight half, the page would show nothing new
        // until a whole company finished.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let partial = one_company("https://b.com", "Business costs $16").report;
        let merged = joined(
            &[one_company("https://a.com", "Pro costs $15")],
            Some(&partial),
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );

        assert_eq!(merged.sections[0].claims.len(), 2);
        assert!(merged.dangling_source_labels().is_empty());
    }

    #[test]
    fn every_claim_says_which_company_it_is_about() {
        // Without this a merged pricing section is two numbers and no names: the claim text is
        // whatever the page said, and the web UI renders the text and a bare `[S1]`.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(
            &[
                one_company("https://a.com", "Pro costs $15"),
                one_company("https://b.com", "Business costs $16"),
            ],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );

        let subjects: Vec<&str> = merged.sections[0]
            .claims
            .iter()
            .map(|c| c.subject.as_str())
            .collect();
        assert_eq!(subjects, vec!["https://a.com", "https://b.com"]);
    }

    #[test]
    fn coverage_is_one_record_per_question_however_many_companies() {
        // `Analysis::render` zips sections with coverage, and the merged report has one section
        // per question however many subjects it covers. Concatenating each company's six would
        // leave the renderer reading the first company's and silently dropping the rest — so a
        // section that found nothing would describe only the first site's attempts, which is
        // the honest-negative treatment failing exactly where it matters most.
        let a = Analysis {
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: vec!["https://a.com/pricing".to_owned()],
                pages_read: 1,
                facts: 0,
                attempts: vec![landscape_core::Attempt {
                    subject: "https://a.com".to_owned(),
                    path: "/pricing".to_owned(),
                    outcome: "404".to_owned(),
                }],
            }],
            ..one_company("https://a.com", "Pro costs $15")
        };
        let b = Analysis {
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: vec!["https://b.com/plans".to_owned()],
                pages_read: 1,
                facts: 0,
                attempts: vec![landscape_core::Attempt {
                    subject: "https://b.com".to_owned(),
                    path: "/plans".to_owned(),
                    outcome: "404".to_owned(),
                }],
            }],
            ..one_company("https://b.com", "Business costs $16")
        };

        let merged = coverage_by_question(&[a, b]);
        assert_eq!(merged.len(), 1, "one record per question, not per company");
        assert_eq!(merged[0].pages_read, 2);
        // And both companies' negative evidence survives, because a path names its company.
        // **Paths, as discovery really stores them** — review caught the first version of this
        // hand-building full URLs, which is the property under test supplied by the fixture.
        let attributed: Vec<String> = merged[0]
            .attempts
            .iter()
            .map(|a| format!("{}{}", a.subject, a.path))
            .collect();
        assert_eq!(
            attributed,
            vec!["https://a.com/pricing", "https://b.com/plans"],
            "one company's attempts were dropped or lost their company"
        );
    }

    /// Origins in `.invalid`, which by RFC 2606 can never resolve.
    ///
    /// Enough to drive `analyze_many` end to end without a model or a reachable page: every
    /// fetch fails fast, every subject produces an empty report, and what is being asserted is
    /// the *joining* — which is the part that has no other way of being reached.
    /// A run over companies the prompt named, with no engine.
    ///
    /// No engine because these assert the join between companies, and a search pass would put a
    /// network round trip inside a test about merging two reports.
    fn named<'a>(origins: &'a [String]) -> Asked<'a> {
        Asked {
            origins,
            now: at(),
            today: at().date_naive(),
            search: None,
            set: None,
            interpreted: None,
        }
    }

    fn unreachable(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("https://s{i}.invalid")).collect()
    }

    #[tokio::test]
    async fn what_was_searched_for_reaches_the_finished_report() {
        let origins = unreachable(2);
        let market = landscape_core::Interpreted {
            label: "competitive intelligence software".to_owned(),
            also: vec!["competitive intelligence".to_owned()],
            hosts: 3,
        };
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                interpreted: Some(&market),
                ..named(&origins)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        assert_eq!(
            outcome.report.interpreted.as_ref(),
            Some(&market),
            "the finished report does not say what it searched for"
        );
    }

    #[test]
    fn a_partial_report_says_what_it_is_being_searched_with_too() {
        // **The reason it rides on `Asked` rather than being stamped on at the end.** A reader
        // watches the partial reports for ninety seconds; a line that appeared only on the
        // finished one would tell them, at the end, that what they had been watching had been
        // about something else all along.
        //
        // Asserted on `joined` with an in-flight report, which is exactly the call
        // `on_progress` makes — a test driving `analyze_many` cannot reach it without a network.
        let origins = vec!["https://a.com".to_owned()];
        let market = landscape_core::Interpreted {
            label: "competitive intelligence software".to_owned(),
            also: Vec::new(),
            hosts: 3,
        };
        let in_flight = one_company("https://a.com", "Pro costs $15").report;
        let partial = joined(
            &[],
            Some(&in_flight),
            &origins,
            Vec::new(),
            &llm(),
            at(),
            Some(&market),
        );
        assert_eq!(partial.interpreted.as_ref(), Some(&market));
    }

    #[tokio::test]
    async fn a_report_about_the_reader_s_own_words_claims_no_interpretation() {
        // Absence is the disclosure working. Repeating somebody's words back at them as an
        // "interpretation" is noise, and noise is what stops the real line being read.
        let origins = unreachable(1);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &named(&origins),
            &mut |_| Wanted::Yes,
        )
        .await;
        assert!(
            outcome.report.interpreted.is_none(),
            "nothing was substituted, so there is nothing to disclose"
        );
    }

    #[tokio::test]
    async fn analyze_many_says_which_companies_it_left_out() {
        // Review's point: capping at three and dropping the fourth in silence is the same
        // defect as taking the first and dropping the second, one count higher. Asserted on
        // the real function rather than on a note handed to the joiner by a test.
        let origins = unreachable(subject::MAX_SUBJECTS + 1);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &named(&origins),
            &mut |_| Wanted::Yes,
        )
        .await;

        let last = origins.last().expect("a dropped origin");
        assert!(
            outcome.report.notes.iter().any(|n| n.contains(last)),
            "{last} was dropped without the report saying so: {:?}",
            outcome.report.notes
        );
    }

    /// A model endpoint that accepts the connection and then says nothing, for ever.
    ///
    /// **The failure mode a timeout is written for and a test almost never has.** An
    /// unreachable address answers immediately - the connection is refused - so it exercises
    /// none of the waiting. This accepts, holds the socket, and never writes a byte.
    async fn a_model_that_never_answers(
    ) -> (landscape_llm::LlamaClient, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a port for the stalling model");
        let port = listener.local_addr().expect("the bound address").port();
        let holding = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                // Kept alive deliberately: dropping it would close the connection and turn a
                // stall into a fast error, which is the case this is not about.
                held.push(socket);
            }
        });
        (
            landscape_llm::LlamaClient::new(format!("http://127.0.0.1:{port}")),
            holding,
        )
    }

    /// A `llama-server` that will say which model it has loaded and nothing else.
    async fn a_model_calling_itself(named: &'static str) -> landscape_llm::LlamaClient {
        let app = axum::Router::new().route(
            "/props",
            axum::routing::get(move || async move {
                axum::Json(serde_json::json!({ "model_path": named }))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let base = format!("http://{}", listener.local_addr().expect("an address"));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        landscape_llm::LlamaClient::new(base)
    }

    #[tokio::test]
    async fn a_model_swapped_under_a_running_worker_does_not_answer_as_the_old_one() {
        // **Review's finding, through the real run.** The memo outlives `llama-server`, so that
        // server can restart with a different model at the same address — and a report would
        // then carry the previous model's words labeled with this one's. The origins are
        // `.invalid`, so nothing is fetched and no page is read: what is under test is that an
        // analysis finds out who is answering before it serves anybody from memory.
        let memo = memo::Extractions::new();
        memo.serving("models/the-one-that-was-loaded-yesterday.gguf");
        memo.remember(
            memo::Read::of(
                Answers::Pricing,
                "https://a.example/pricing",
                "# Pricing",
                chrono::Utc::now().date_naive(),
            ),
            &Outcome {
                claims: Vec::new(),
                summary: "1 plan found in 1 window".to_owned(),
                details: Vec::new(),
                window_words: 40,
                settled: Settled::Complete,
            },
        );
        assert_ne!(
            memo.held(),
            (0, 0),
            "the memory should start with something"
        );

        let llm = a_model_calling_itself("models/the-one-loaded-now.gguf").await;
        analyze_with(
            &With {
                fetcher: &landscape_fetch::Fetcher::new(),
                llm: &llm,
                memo: &memo,
            },
            &landscape_fetch::Budget::for_one_analysis(),
            "https://subject.invalid",
            chrono::Utc::now(),
            chrono::Utc::now().date_naive(),
            None,
            &mut |_| Wanted::Yes,
        )
        .await;

        assert_eq!(
            memo.held(),
            (0, 0),
            "a different model's answers survived the swap"
        );
    }

    #[tokio::test]
    async fn a_model_that_never_answers_does_not_delay_a_run_that_needs_no_model() {
        // Review found this, and it is the whole feature failing in the case it exists for.
        // `is_ready()` used to be awaited before the plan was built and before anything was
        // fetched — on the same client, which waits 180 seconds because prefill on four ARM
        // cores is slow. So a model that accepted connections and stalled would spend the
        // entire report budget *before the changelog was opened*, and the page that needs no
        // model would arrive after the wait it was supposed to remove.
        //
        // The origins are `.invalid` (RFC 2606), so nothing is fetched and no page ever needs
        // the model. Health must therefore never be asked, and this must return at once.
        let (stalling, holding) = a_model_that_never_answers().await;
        let started = std::time::Instant::now();
        let outcome = analyze_many(
            &With {
                fetcher: &landscape_fetch::Fetcher::new(),
                llm: &stalling,
                memo: &memo::Extractions::new(),
            },
            &landscape_fetch::Budget::for_one_analysis(),
            &named(&unreachable(1)),
            &mut |_| Wanted::Yes,
        )
        .await;
        let took = started.elapsed();
        holding.abort();

        assert!(
            took < std::time::Duration::from_secs(30),
            "a stalled model held up a run that needed no model: {took:?}"
        );
        assert!(
            !outcome.report.sections.is_empty(),
            "the run produced no report at all"
        );
    }

    #[tokio::test]
    async fn analyze_many_reports_one_coverage_record_per_question() {
        // `Analysis::render` zips sections with coverage. Two companies' worth concatenated
        // would leave the second company's negative evidence unreachable — and this asserts it
        // on the function the worker actually calls, not on the merge helper alone.
        let origins = unreachable(2);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &named(&origins),
            &mut |_| Wanted::Yes,
        )
        .await;

        assert_eq!(
            outcome.coverage.len(),
            outcome.report.sections.len(),
            "the renderer zips these two, so a mismatch drops somebody's evidence"
        );
        let mut questions: Vec<&str> = outcome
            .coverage
            .iter()
            .map(|c| c.question.as_str())
            .collect();
        questions.sort_unstable();
        let before = questions.len();
        questions.dedup();
        assert_eq!(
            before,
            questions.len(),
            "a question appears twice: {questions:?}"
        );

        // **Real attempts, from real discovery, for two origins.** Everything above could be
        // satisfied by hand-built coverage; this cannot. `attempts_for` stores a *path*, so the
        // only thing separating two companies' `/pricing` is the subject beside it.
        let subjects: std::collections::BTreeSet<&str> = outcome
            .coverage
            .iter()
            .flat_map(|c| c.attempts.iter())
            .map(|a| a.subject.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            subjects.len(),
            origins.len(),
            "discovery's attempts do not say which company they belong to: {subjects:?}"
        );

        // **The lines the web report renders, not the coverage behind them.** Review found
        // this after the coverage above was already right: `joined()` concatenates strings each
        // company rendered while it was still the only one, so `report.sections[*].checked` -
        // which the interface reads and a `Coverage` never reaches - still said `/pricing (404)`
        // twice. Asserting the model and not the surface is how a fixed defect stays visible.
        let checked: Vec<&str> = outcome
            .report
            .sections
            .iter()
            .flat_map(|s| s.checked.iter())
            .map(String::as_str)
            .collect();
        assert!(
            !checked.is_empty(),
            "two unreachable origins produced no checked lines, so this asserts nothing"
        );
        for origin in &origins {
            let host = origin.trim_start_matches("https://");
            assert!(
                checked.iter().any(|line| line.starts_with(host)),
                "no checked line in the report belongs to {host}: {checked:?}"
            );
        }
        let unattributed: Vec<&&str> = checked.iter().filter(|l| l.starts_with('/')).collect();
        assert!(
            unattributed.is_empty(),
            "checked lines a reader cannot attribute: {unattributed:?}"
        );
    }

    #[test]
    fn each_company_keeps_its_own_note_about_what_it_did_not_read() {
        // The budget is decided per company: one may have had a second pricing page and the
        // other not. A merge that kept only the caller's notes would drop the sentence saying
        // what the shorter wait cost — which is the half of the trade a reader is owed, and it
        // would go missing exactly on the comparisons that are slowest.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let mut first = one_company("https://a.com", "Pro costs $15");
        first.report.notes = vec!["a.com: 2 further page(s) were found and not read".to_owned()];
        let mut second = one_company("https://b.com", "Business costs $16");
        second.report.notes = vec!["b.com: 1 further page(s) were found and not read".to_owned()];

        let merged = joined(
            &[first, second],
            None,
            &origins,
            vec!["Comparing the first 2 sites named.".to_owned()],
            &llm(),
            at(),
            None,
        );

        assert!(
            merged.notes.iter().any(|n| n.contains("a.com: 2 further")),
            "the first company's note was dropped: {:?}",
            merged.notes
        );
        assert!(
            merged.notes.iter().any(|n| n.contains("b.com: 1 further")),
            "the second company's note was dropped: {:?}",
            merged.notes
        );
        assert!(
            merged
                .notes
                .iter()
                .any(|n| n.contains("Comparing the first")),
            "the caller's own note was lost: {:?}",
            merged.notes
        );
    }

    #[test]
    fn one_note_written_by_both_companies_is_said_once() {
        // Two companies with the same shape of skipped page produce the same sentence. Saying
        // it twice above the sections reads as two different findings.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let note = "1 further page(s) were found and not read".to_owned();
        let mut first = one_company("https://a.com", "Pro costs $15");
        first.report.notes = vec![note.clone()];
        let mut second = one_company("https://b.com", "Business costs $16");
        second.report.notes = vec![note.clone()];

        let merged = joined(
            &[first, second],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );
        assert_eq!(
            merged.notes.iter().filter(|n| **n == note).count(),
            1,
            "{:?}",
            merged.notes
        );
    }

    #[test]
    fn a_silent_company_does_not_take_the_label_off_the_one_that_spoke() {
        // Review found this. Deriving "is this a comparison" from the claims looks reasonable
        // and is wrong exactly here: ask about two companies, have one say nothing, and the
        // survivor's prices lose their label in a report that is still a comparison.
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let silent = Analysis {
            report: Report {
                sections: vec![Section::not_found("pricing", "What it costs", Vec::new())],
                sources: Vec::new(),
                ..one_company("https://b.com", "unused").report
            },
            ..one_company("https://b.com", "unused")
        };
        let merged = joined(
            &[one_company("https://a.com", "Pro costs $15"), silent],
            None,
            &origins,
            Vec::new(),
            &llm(),
            at(),
            None,
        );

        assert_eq!(
            merged.subjects, origins,
            "the report has to carry what it set out to cover, not what answered"
        );
        let rendered = Analysis {
            report: merged,
            coverage: vec![Coverage {
                question: "pricing".to_owned(),
                sources: Vec::new(),
                pages_read: 0,
                facts: 1,
                attempts: Vec::new(),
            }],
            pages: Vec::new(),
            stopped_early: false,
        }
        .render();
        assert!(
            rendered.contains("**a.com**"),
            "the one company that answered lost its label because the other said nothing:
{rendered}"
        );
    }

    #[test]
    fn merged_attempts_keep_their_company_in_the_shape_discovery_stores_them() {
        // Review's second point, and the sharper one: I claimed every attempt was a URL, and
        // `attempts_for` stores a *path* — `/pricing` — because the heading above it used to
        // name the company. Two companies then contribute indistinguishable lines, and my
        // first test hand-built full URLs, which is the fixture supplying the property under
        // test all over again.
        let attempts = |origin: &str| {
            vec![landscape_core::Attempt {
                subject: origin.to_owned(),
                path: "/pricing".to_owned(),
                outcome: "404".to_owned(),
            }]
        };
        let coverage = |origin: &str| Coverage {
            question: "pricing".to_owned(),
            sources: Vec::new(),
            pages_read: 0,
            facts: 0,
            attempts: attempts(origin),
        };
        let merged = coverage_by_question(&[
            Analysis {
                coverage: vec![coverage("https://a.com")],
                ..one_company("https://a.com", "unused")
            },
            Analysis {
                coverage: vec![coverage("https://b.com")],
                ..one_company("https://b.com", "unused")
            },
        ]);

        // Both are `/pricing`. Without the company they are one line written twice.
        let paths: Vec<&str> = merged[0].attempts.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, vec!["/pricing", "/pricing"], "the real shape");

        let section = merged[0].to_section("What it costs");
        assert!(
            section.checked.iter().any(|c| c.contains("a.com"))
                && section.checked.iter().any(|c| c.contains("b.com")),
            "a reader cannot tell which company found nothing: {:?}",
            section.checked
        );
    }

    fn found(name: &str, domain: &str, agreed: usize) -> landscape_search::competitors::Member {
        landscape_search::competitors::Member {
            candidate: landscape_core::subject::Candidate {
                name: name.to_owned(),
                canonical_domain: domain.to_owned(),
                what_it_is: "a company".to_owned(),
                confidence: 0.9,
            },
            because: landscape_search::competitors::Because::Found {
                agreed,
                asked: 3,
                shares: vec!["analytics".to_owned()],
            },
        }
    }

    #[tokio::test]
    async fn a_report_about_companies_nobody_named_says_so_first() {
        // **The one thing a reader cannot check by reading further down the page.** Every other
        // note is about what a run did; this is about *who it is about*, and a reader who typed
        // a description never said any of these names.
        let set = landscape_search::competitors::Set {
            members: vec![
                found("Fathom Analytics", "usefathom.com", 3),
                found("Plausible", "plausible.io", 3),
            ],
            set_aside: vec![(
                landscape_core::subject::Candidate {
                    name: "Notion Press".to_owned(),
                    canonical_domain: "notionpress.example".to_owned(),
                    what_it_is: "a publisher".to_owned(),
                    confidence: 0.9,
                },
                landscape_search::competitors::Aside::ElsewhereEntirely {
                    looked_for: vec!["analytics".to_owned()],
                },
            )],
            alone: None,
        };
        // **Four companies, so another note exists to be first *of*.** With one company the
        // list holds only this note, and `insert(0)` and `insert(len())` are the same place -
        // the mutation that moved it to the end survived, and the assertion below could not
        // have failed. The fourth origin is dropped by the subject cap, which writes the note
        // this one has to sit above.
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        assert!(
            outcome.report.notes.len() >= 2,
            "the fixture needs a second note for `first` to mean anything: {:#?}",
            outcome.report.notes
        );

        let first = outcome.report.notes.first().expect("a note");
        assert!(first.contains("You described a market"), "{first}");
        assert!(
            first.contains("Fathom Analytics (usefathom.com) and Plausible (plausible.io)"),
            "{first}"
        );
        assert!(
            first.contains("name the domains"),
            "a reader is not told how to correct us: {first}"
        );
        // **First, not merely present.** A sentence about who the report is about, printed
        // under the note about which pages the budget skipped, is a sentence nobody reads.
        assert_eq!(
            outcome
                .report
                .notes
                .iter()
                .position(|n| n.contains("You described a market")),
            Some(0),
            "{:#?}",
            outcome.report.notes
        );

        let why = &outcome.report.notes[1];
        assert!(why.starts_with("Why each one is here"), "{why}");
        assert!(
            why.contains("Fathom Analytics - 3 of the 3 searches"),
            "{why}"
        );

        let left_out = &outcome.report.notes[2];
        assert!(
            left_out.contains("Notion Press (notionpress.example)"),
            "{left_out}"
        );
        // **Named, not alluded to.** The sentence used to say *"none of the words you typed"*,
        // which is false on the seeded path where the words come from the seed's own page.
        assert!(
            left_out.contains("none of the words this comparison is built on"),
            "{left_out}"
        );
        assert!(
            left_out.contains("\"analytics\""),
            "the words a reader would need to judge the exclusion are missing: {left_out}"
        );
    }

    #[tokio::test]
    async fn a_company_that_could_have_been_compared_is_named_and_the_rest_are_counted() {
        // **Two different silences in one list.** A company two searches agreed on could have
        // been in the report, so it is named with its reason. A host one search returned could
        // not have been, and there can be twenty of them - naming those turns a note into a
        // search results page, which is its own way of not being read.
        let aside = |domain: &str, why: landscape_search::competitors::Aside| {
            (
                landscape_core::subject::Candidate {
                    name: domain.to_owned(),
                    canonical_domain: domain.to_owned(),
                    what_it_is: "a company".to_owned(),
                    confidence: 0.9,
                },
                why,
            )
        };
        let set = landscape_search::competitors::Set {
            members: vec![found("Fathom", "usefathom.com", 3)],
            set_aside: vec![
                aside(
                    "sixth.example",
                    landscape_search::competitors::Aside::BeyondTheFetchBudget { budget: 5 },
                ),
                aside(
                    "one.example",
                    landscape_search::competitors::Aside::Uncorroborated {
                        agreed: 1,
                        asked: 3,
                    },
                ),
                aside(
                    "two.example",
                    landscape_search::competitors::Aside::Uncorroborated {
                        agreed: 1,
                        asked: 3,
                    },
                ),
            ],
            alone: None,
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;

        let left_out = outcome
            .report
            .notes
            .iter()
            .find(|n| n.starts_with("Also found"))
            .expect("the exclusions note");
        assert!(left_out.contains("sixth.example"), "{left_out}");
        assert!(left_out.contains("never asked for its page"), "{left_out}");
        // Counted, not named - and counted, not silent.
        assert!(left_out.contains("2 further sites"), "{left_out}");
        assert!(
            !left_out.contains("one.example") && !left_out.contains("two.example"),
            "twenty one-hit hosts would be a search results page: {left_out}"
        );
    }

    #[tokio::test]
    async fn a_report_seeded_by_a_named_company_does_not_claim_they_described_a_market() {
        // A reader who typed a domain knows what they typed. Telling them they described a
        // market is telling them something false about their own input - what they do not know
        // is that we went looking for the others.
        let set = landscape_search::competitors::Set {
            members: vec![
                landscape_search::competitors::Member {
                    candidate: landscape_core::subject::Candidate {
                        name: "Basecamp".to_owned(),
                        canonical_domain: "basecamp.com".to_owned(),
                        what_it_is: "Project management".to_owned(),
                        confidence: 1.0,
                    },
                    because: landscape_search::competitors::Because::Named,
                },
                found("Linear", "linear.app", 3),
            ],
            set_aside: Vec::new(),
            alone: None,
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;

        let first = outcome.report.notes.first().expect("a note");
        assert!(first.starts_with("You named basecamp.com"), "{first}");
        assert!(
            !first.contains("You described"),
            "a reader was told they described what they named: {first}"
        );
        assert!(first.contains("Linear (linear.app)"), "{first}");

        let why = &outcome.report.notes[1];
        assert!(why.contains("Basecamp - you named it"), "{why}");
        assert!(why.contains("Linear - 3 of the 3 searches"), "{why}");
    }

    #[tokio::test]
    async fn a_seeded_report_never_blames_the_reader_for_words_it_chose_itself() {
        // The exclusions note on the path where nobody typed any words: they came off the seed
        // company's own front page, and saying otherwise is a false account of the evidence.
        let set = landscape_search::competitors::Set {
            members: vec![landscape_search::competitors::Member {
                candidate: landscape_core::subject::Candidate {
                    name: "Basecamp".to_owned(),
                    canonical_domain: "basecamp.com".to_owned(),
                    what_it_is: "Project management".to_owned(),
                    confidence: 1.0,
                },
                because: landscape_search::competitors::Because::Named,
            }],
            set_aside: vec![(
                landscape_core::subject::Candidate {
                    name: "A Bakery".to_owned(),
                    canonical_domain: "bakery.example".to_owned(),
                    what_it_is: "Sourdough".to_owned(),
                    confidence: 0.9,
                },
                landscape_search::competitors::Aside::ElsewhereEntirely {
                    looked_for: vec!["project".to_owned(), "management".to_owned()],
                },
            )],
            alone: None,
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;

        let left_out = outcome
            .report
            .notes
            .iter()
            .find(|n| n.starts_with("Also found"))
            .expect("the exclusions note");
        assert!(
            !left_out.contains("you typed"),
            "the reader was blamed for words we chose ourselves: {left_out}"
        );
        assert!(
            left_out.contains("\"project\", \"management\""),
            "{left_out}"
        );
    }

    #[tokio::test]
    async fn a_report_about_one_company_never_claims_to_have_compared_it() {
        // **The row this change is for.** A one-company report used to say *"we searched for
        // the companies it competes with. This report compares Basecamp."* - which is a
        // comparison claimed and not made, and in three of the four cases a search claimed and
        // not run.
        for (why, must_say, must_not_say) in [
            (
                landscape_search::competitors::NoRivals::NoEngine,
                "no search engine is configured",
                "try again",
            ),
            (
                landscape_search::competitors::NoRivals::SearchIncomplete {
                    failed: 3,
                    sent: 3,
                    sought: landscape_search::competitors::Sought::RivalsOfTheCompany,
                    fault: landscape_search::Fault::Silent,
                },
                "3 of the 3 searches",
                "none of what came back held up",
            ),
            (
                landscape_search::competitors::NoRivals::NobodyHeldUp {
                    sought: landscape_search::competitors::Sought::RivalsOfTheCompany,
                },
                "none of what came back held up",
                "try again",
            ),
        ] {
            let set = landscape_search::competitors::Set {
                members: vec![landscape_search::competitors::Member {
                    candidate: landscape_core::subject::Candidate {
                        name: "Basecamp".to_owned(),
                        canonical_domain: "basecamp.com".to_owned(),
                        what_it_is: "Project management".to_owned(),
                        confidence: 1.0,
                    },
                    because: landscape_search::competitors::Because::Named,
                }],
                set_aside: Vec::new(),
                alone: Some(why.clone()),
            };
            let many = unreachable(4);
            let outcome = analyze_many(
                &offline(),
                &landscape_fetch::Budget::for_one_analysis(),
                &Asked {
                    set: Some(&set),
                    ..named(&many)
                },
                &mut |_| Wanted::Yes,
            )
            .await;
            let notes = outcome.report.notes.join("\n");

            assert!(
                notes.contains("You named basecamp.com, so this report is about it."),
                "{notes}"
            );
            assert!(
                !notes.contains("This report compares"),
                "a comparison was claimed and not made: {notes}"
            );
            assert!(notes.contains(must_say), "{why:?} did not say it: {notes}");
            assert!(
                !notes.contains(must_not_say),
                "{why:?} said something belonging to another silence: {notes}"
            );
        }
    }

    #[tokio::test]
    async fn a_description_report_never_says_its_own_company_failed_to_hold_up() {
        // **Review found two adjacent notes contradicting each other.** The reason a set is
        // alone was written for the seeded path, so a description that matched one company
        // said *"none of what came back held up"* immediately before *"Plausible - 3 of the 3
        // searches returned it"*. Plausible is what came back, and it held up.
        let set = landscape_search::competitors::Set {
            members: vec![found("Plausible", "plausible.io", 3)],
            set_aside: Vec::new(),
            alone: Some(landscape_search::competitors::NoRivals::NobodyHeldUp {
                sought: landscape_search::competitors::Sought::CompaniesMatchingTheDescription,
            }),
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        let notes = outcome.report.notes.join(
            "
",
        );

        assert!(
            notes.contains("Plausible - 3 of the 3 searches returned it"),
            "{notes}"
        );
        assert!(
            !notes.contains("none of what came back held up"),
            "the report says its own company failed to hold up: {notes}"
        );
        assert!(
            !notes.contains("companies it competes with"),
            "a description search was described as a search for one company's rivals: {notes}"
        );
        assert!(
            notes.contains("Only one company matched that description"),
            "{notes}"
        );
    }

    #[tokio::test]
    async fn an_incomplete_description_search_is_described_as_one() {
        let set = landscape_search::competitors::Set {
            members: vec![found("Plausible", "plausible.io", 2)],
            set_aside: Vec::new(),
            alone: Some(landscape_search::competitors::NoRivals::SearchIncomplete {
                failed: 2,
                sent: 3,
                sought: landscape_search::competitors::Sought::CompaniesMatchingTheDescription,
                fault: landscape_search::Fault::Silent,
            }),
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        let notes = outcome.report.notes.join(
            "
",
        );
        assert!(
            notes.contains("searches for companies matching that description"),
            "{notes}"
        );
        assert!(!notes.contains("companies it competes with"), "{notes}");
        assert!(notes.contains("try again"), "{notes}");
    }

    #[tokio::test]
    async fn a_real_comparison_still_says_it_compared_them() {
        // The other half, so the rule above is a rule rather than a refusal to claim anything.
        let set = landscape_search::competitors::Set {
            members: vec![
                found("Fathom", "usefathom.com", 3),
                found("Plausible", "plausible.io", 3),
            ],
            set_aside: Vec::new(),
            alone: None,
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        let first = outcome.report.notes.first().expect("a note");
        assert!(first.contains("This report compares"), "{first}");
        assert!(
            !outcome.report.notes.join("\n").contains("did not look"),
            "{:#?}",
            outcome.report.notes
        );
    }

    #[tokio::test]
    async fn no_company_is_named_in_a_note_that_the_report_does_not_compare() {
        // **The cap and the note have to agree.** `MAX_SUBJECTS` drops the fourth company, and
        // a note naming five while three are read is the silent-drop defect with better prose
        // in front of it.
        let set = landscape_search::competitors::Set {
            members: vec![
                found("One", "one.example", 3),
                found("Two", "two.example", 3),
                found("Three", "three.example", 3),
                found("Four", "four.example", 3),
            ],
            set_aside: Vec::new(),
            alone: None,
        };
        let many = unreachable(4);
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &Asked {
                set: Some(&set),
                ..named(&many)
            },
            &mut |_| Wanted::Yes,
        )
        .await;
        let first = outcome.report.notes.first().expect("a note");
        assert!(first.contains("Three (three.example)"), "{first}");
        assert!(
            !first.contains("Four"),
            "a company the report never read was named as compared: {first}"
        );
    }

    #[tokio::test]
    async fn a_report_about_a_company_somebody_named_says_nothing_extra() {
        // The other half: a prompt that named a domain must not grow a sentence explaining a
        // choice nobody made.
        let outcome = analyze_many(
            &offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            &named(&unreachable(1)),
            &mut |_| Wanted::Yes,
        )
        .await;
        assert!(
            !outcome
                .report
                .notes
                .iter()
                .any(|n| n.contains("You described a market")),
            "{:#?}",
            outcome.report.notes
        );
    }

    #[test]
    fn the_report_says_which_companies_it_is_about() {
        let origins = vec!["https://a.com".to_owned(), "https://b.com".to_owned()];
        let merged = joined(&[], None, &origins, Vec::new(), &llm(), at(), None);
        assert_eq!(merged.subject, "https://a.com, https://b.com");
        assert_eq!(merged.searched_as, "https://a.com, https://b.com");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod filling_gaps {
    //! Search, and the three things it must never do: lead, repeat a page, or let a stranger's
    //! page read as the company speaking.

    use super::*;
    use landscape_search::provider::{Hit, SearchError, SourceProvider};
    use landscape_search::Query;

    /// A provider that answers from a list, so the join can be exercised with no network.
    struct Canned {
        hits: Vec<Hit>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl Canned {
        fn holding(urls: &[&str]) -> Self {
            Self {
                hits: urls
                    .iter()
                    .map(|u| Hit {
                        url: (*u).to_owned(),
                        title: "a title the engine wrote".to_owned(),
                        snippet: "a snippet the engine wrote".to_owned(),
                    })
                    .collect(),
                asked: std::sync::Mutex::new(Vec::new()),
            }
        }
        fn queries(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl SourceProvider for Canned {
        fn name(&self) -> &str {
            "canned"
        }
        async fn search(&self, query: &Query) -> Result<Vec<Hit>, SearchError> {
            self.asked.lock().unwrap().push(query.text.clone());
            Ok(self.hits.clone())
        }
    }

    /// A run that opened exactly these URLs, so a note can be asserted against what happened
    /// rather than against what was admitted.
    fn page(url: &str, window_words: Option<usize>, summary: &str) -> PageResult {
        PageResult {
            url: url.to_owned(),
            question: Answers::Pricing,
            words: window_words.map(|_| 500),
            quality: window_words.map(|_| "good"),
            // The one field set on the extract path and left `None` by every refusal.
            window_words,
            summary: summary.to_owned(),
            details: Vec::new(),
        }
    }

    /// A run that opened exactly these URLs, so a note can be asserted against what happened.
    fn having_read(urls: &[&str]) -> SoFar {
        SoFar {
            pages: urls.iter().map(|u| page(u, Some(120), "read")).collect(),
            ..SoFar::default()
        }
    }

    /// A provider that answers every query with a refusal.
    ///
    /// **The other common real one, and the more likely of the two on a first run.** A SearXNG
    /// that has not opted into JSON answers 403 to everything, for ever —
    /// `deploy/searxng/settings.yml` is checked in for that reason alone.
    struct Refusing;

    #[async_trait::async_trait]
    impl landscape_search::SourceProvider for Refusing {
        fn name(&self) -> &str {
            "refusing"
        }
        async fn search(&self, _query: &Query) -> Result<Vec<Hit>, SearchError> {
            Err(SearchError::Status { status: 403 })
        }
    }

    /// A provider that always fails, which is the common real one: an engine that is down.
    struct Down;
    #[async_trait::async_trait]
    impl SourceProvider for Down {
        fn name(&self) -> &str {
            "down"
        }
        async fn search(&self, _query: &Query) -> Result<Vec<Hit>, SearchError> {
            Err(SearchError::Unreachable("no route to host".to_owned()))
        }
    }

    #[test]
    fn a_company_whose_own_pages_answered_everything_is_not_searched_for() {
        // `FACT_CHECKING.md` §3.3's order, made structural rather than commented: the gaps are
        // computed from claims that exist, so there is nothing to ask about.
        //
        // **Asserted on the decision rather than on `search_for_gaps`.** The early return that
        // used to live inside it made two places responsible for one rule, and the one a caller
        // could get wrong was the other one.
        assert!(!worth_searching(false, &[]));
        assert!(worth_searching(false, &[Answers::Trust]));
    }

    #[test]
    fn a_run_the_reader_walked_away_from_searches_for_nothing() {
        // The progress callback already said `Wanted::No`. Search is the most expensive thing
        // left in the run, and spending it on a report nobody will read is the same waste the
        // generation number was added to stop.
        assert!(!worth_searching(true, &[Answers::Trust]));
        assert!(!worth_searching(true, &[]));
    }

    #[tokio::test]
    async fn the_cap_bounds_what_one_search_adds_to_the_wait() {
        // Every extra page is a fetch and, for four of the six questions, a model call per
        // window - spent after a reader has already waited for the whole plan.
        let many: Vec<String> = (0..8)
            .map(|i| format!("https://elsewhere{i}.example/page"))
            .collect();
        let urls: Vec<&str> = many.iter().map(String::as_str).collect();
        let engine = Canned::holding(&urls);
        let (admitted, _) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Trust], &[]).await;
        assert_eq!(admitted.len(), PAGES_FROM_SEARCH, "{admitted:#?}");
    }

    #[tokio::test]
    async fn a_gap_nobody_could_search_is_said_out_loud() {
        // The laptop default. A reader cannot otherwise tell a company with nothing published
        // from a run that never asked, and those are different facts about the world.
        let (admitted, failures) =
            search_for_gaps(None, "https://e.com", &[Answers::Trust], &[]).await;
        assert!(admitted.is_empty());
        assert_eq!(failures, Failures::NoEngine);
        let note = note_for(
            "https://e.com",
            &[Answers::Trust],
            &admitted,
            &SoFar::default(),
            failures,
        )
        .expect("a gap nobody searched has to be reported");
        assert!(note.contains("no search engine is configured"), "{note}");
        assert!(note.contains("trust"), "{note}");
        assert!(note.starts_with("e.com:"), "the note names nobody: {note}");
    }

    #[tokio::test]
    async fn a_page_already_read_is_not_admitted_twice() {
        // `already` is every URL this run has touched. Re-admitting one spends a slot of the
        // cap on a page the report already cites.
        let engine = Canned::holding(&["https://e.com/security", "https://e.com/trust"]);
        let (admitted, _) = search_for_gaps(
            Some(&engine),
            "https://e.com",
            &[Answers::Trust],
            &["https://e.com/security".to_owned()],
        )
        .await;
        let urls: Vec<&str> = admitted.iter().map(|f| f.url.as_str()).collect();
        assert_eq!(urls, vec!["https://e.com/trust"], "{urls:?}");
    }

    #[tokio::test]
    async fn the_company_speaks_for_itself_and_a_stranger_does_not() {
        let engine = Canned::holding(&[
            "https://e.com/pricing-details",
            "https://someblog.example/e-review",
        ]);
        let (admitted, failures) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Pricing], &[]).await;

        let own = admitted
            .iter()
            .find(|f| f.url.contains("e.com"))
            .expect("the company's own page");
        assert_eq!(own.disposition, Disposition::Primary);
        let other = admitted
            .iter()
            .find(|f| f.url.contains("someblog"))
            .expect("the other page");
        assert_eq!(other.disposition, Disposition::Unverified);

        let read = having_read(&[
            "https://e.com/pricing-details",
            "https://someblog.example/e-review",
        ]);
        let note = note_for(
            "https://e.com",
            &[Answers::Pricing],
            &admitted,
            &read,
            failures,
        )
        .expect("a search that read pages has to say so");
        assert!(note.starts_with("e.com:"), "{note}");
        assert!(note.contains("2 page(s) were read"), "{note}");
        assert!(note.contains("1 on this company's own site"), "{note}");
        assert!(note.contains("labeled unverified"), "{note}");

        // One query, and it names the company. A query that does not is a query about the
        // whole web, and one query per gap is what the wait can afford.
        let asked = engine.queries();
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert!(asked[0].contains("e.com"), "{asked:?}");
    }

    #[tokio::test]
    async fn an_engine_that_is_down_is_reported_rather_than_swallowed() {
        // A search failure is not an analysis failure — and it is also not nothing. Without
        // this line the section keeps a coverage note that reads as "we looked and there was
        // nothing", which is the one thing we did not establish.
        let (admitted, failures) =
            search_for_gaps(Some(&Down), "https://e.com", &[Answers::Changes], &[]).await;
        assert!(admitted.is_empty());
        assert_eq!(
            failures,
            Failures::Asked {
                failures: 1,
                fault: Some(landscape_search::Fault::Silent),
            }
        );
        let note = note_for(
            "https://e.com",
            &[Answers::Changes],
            &admitted,
            &SoFar::default(),
            failures,
        )
        .expect("a failed search has to be reported");
        assert!(note.contains("did not complete"), "{note}");
    }

    #[tokio::test]
    async fn an_engine_that_refuses_does_not_tell_a_reader_to_try_again() {
        // Same count, same coverage note, **different last sentence** - because the run that
        // produced this will produce it again, and "try again" is an instruction to wait for
        // something that cannot happen.
        let (admitted, failures) =
            search_for_gaps(Some(&Refusing), "https://e.com", &[Answers::Changes], &[]).await;
        assert!(admitted.is_empty());
        assert_eq!(
            failures,
            Failures::Asked {
                failures: 1,
                fault: Some(landscape_search::Fault::Refused),
            }
        );
        let note = note_for(
            "https://e.com",
            &[Answers::Changes],
            &admitted,
            &SoFar::default(),
            failures,
        )
        .expect("a failed search has to be reported");
        assert!(note.contains("did not complete"), "{note}");
        assert!(
            !note.contains("usually temporary"),
            "a refusal reported as weather: {note}"
        );
        assert!(note.contains("will not change"), "{note}");
        // And nothing a reader cannot act on reaches them.
        for internal in ["403", "SEARX_URL", "SearXNG"] {
            assert!(
                !note.contains(internal),
                "{internal} in a report note: {note}"
            );
        }
    }

    #[test]
    fn a_question_whose_page_was_read_and_said_nothing_is_still_unanswered() {
        // **The sharper measure, and the reason this belongs in an analysis rather than in the
        // CLI.** `landscape search` computes gaps from what discovery *admitted*; a page can be
        // admitted, fetched, read, and state nothing. BENCHMARKS Run 26 named the difference.
        let mut so_far = SoFar {
            pages: vec![PageResult {
                url: "https://e.com/security".to_owned(),
                question: Answers::Trust,
                words: Some(400),
                quality: Some("good"),
                window_words: Some(0),
                summary: "no standards named on the page".to_owned(),
                details: Vec::new(),
            }],
            ..SoFar::default()
        };
        assert!(
            so_far.unanswered().contains(&Answers::Trust),
            "a page that stated nothing left the question answered"
        );
        assert_eq!(so_far.urls(), vec!["https://e.com/security".to_owned()]);

        so_far.claims.push((
            Answers::Trust,
            Claim {
                subject: String::new(),
                text: "states SOC 2 Type II".to_owned(),
                source_label: "S1".to_owned(),
                evidence_quote: "SOC 2 Type II".to_owned(),
                confidence: Confidence::Medium,
                as_of: chrono::Utc::now(),
            },
        ));
        assert!(!so_far.unanswered().contains(&Answers::Trust));
    }

    #[tokio::test]
    async fn a_page_carries_what_it_may_be_used_for_wherever_it_came_from() {
        // **The pairing, asserted as a pairing.** This was two arguments to `read_one` and a
        // mutation that labeled every search result `Primary` survived the whole suite: a
        // stranger's page would have been rendered as the company speaking, with no marking,
        // at whatever confidence the extractor gave it.
        let engine = Canned::holding(&["https://e.com/deep/pricing", "https://blog.example/e"]);
        let (admitted, _) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Pricing], &[]).await;

        for hit in &admitted {
            let to_read = ToRead::found(hit);
            assert_eq!(to_read.candidate.url, hit.url, "a page lost its own URL");
            assert_eq!(
                to_read.disposition, hit.disposition,
                "{} was read as something admission did not say it was",
                hit.url
            );
        }
        assert!(
            admitted
                .iter()
                .any(|f| f.disposition == Disposition::Unverified),
            "the fixture has to contain a page that is not the company's own"
        );

        // And a probe's page is the company speaking, which is the other half of the pair.
        let probed = ToRead::probed(&landscape_discover::rank::Candidate {
            url: "https://e.com/pricing".to_owned(),
            answers: Answers::Pricing,
            via: landscape_discover::rank::Via::Probe,
        });
        assert_eq!(probed.disposition, Disposition::Primary);
    }

    #[tokio::test]
    async fn a_page_that_failed_to_fetch_is_not_reported_as_read() {
        // Review's first finding. The note used to be written straight after admission, so a
        // page that never came back — a dead host, a page too thin to open, a run the reader
        // walked away from — was counted as read anyway.
        let engine = Canned::holding(&["https://e.com/one", "https://elsewhere.example/two"]);
        let (admitted, failures) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Pricing], &[]).await;
        assert_eq!(admitted.len(), 2);

        // **The second is in `pages` with no window**, which is what a failed fetch actually
        // leaves behind — an earlier version of this test left it out of `pages` entirely, so
        // deleting the `window_words` check broke nothing and the mutation survived. Entry 32:
        // a test named for one guard, held up by another.
        let mut read = having_read(&["https://e.com/one"]);
        read.pages.push(page(
            "https://elsewhere.example/two",
            None,
            "could not fetch",
        ));
        let note = note_for(
            "https://e.com",
            &[Answers::Pricing],
            &admitted,
            &read,
            failures,
        )
        .expect("a note");
        assert!(note.contains("1 page(s) were read"), "{note}");
        assert!(
            note.contains("1 further page(s) were found and not read"),
            "a page nobody opened is missing from the note: {note}"
        );
    }

    #[tokio::test]
    async fn a_run_that_opened_none_of_them_says_exactly_that() {
        let engine = Canned::holding(&["https://e.com/one"]);
        let (admitted, failures) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Pricing], &[]).await;
        let note = note_for(
            "https://e.com",
            &[Answers::Pricing],
            &admitted,
            &SoFar::default(),
            failures,
        )
        .expect("a note");
        assert!(note.contains("nothing we could read"), "{note}");
        assert!(
            note.contains("1 further page(s) were found and not read"),
            "{note}"
        );
    }

    #[test]
    fn a_question_search_read_a_page_for_is_not_reported_as_unchecked() {
        // Review's second finding, and the sharpest of the three: `Coverage` is built from
        // discovery, and search was the first thing ever to add a page discovery had not. A
        // question whose only page came from search reported *"nothing was checked - our gap,
        // not theirs"* — the report saying we did not look, immediately after looking.
        let found = landscape_discover::Discovered {
            sources: Vec::new(),
            checked: Vec::new(),
            stopped_early: false,
        };
        let notes: Vec<String> = Vec::new();
        let reading = Reading {
            found: &found,
            origin: "https://e.com",
            llm: &landscape_llm::LlamaClient::new("http://127.0.0.1:1".to_owned()),
            budget: &landscape_fetch::Budget::for_one_analysis(),
            memo: &memo::Extractions::new(),
            now: chrono::Utc::now(),
            notes: &notes,
        };
        let searched = vec![Searched {
            question: Answers::Trust,
            url: "https://elsewhere.example/e".to_owned(),
            read: true,
        }];

        let (report, coverage) = assemble(&reading, &[], &[], &[(Answers::Trust, 1)], &searched);
        let trust = coverage
            .iter()
            .find(|c| c.question == "trust")
            .expect("a coverage row per question");

        // **Asserted on what a reader is shown**, not on the field behind it. The first fix
        // extended `sources` and stopped there, and `sources` is a count: the note still read
        // *"Checked: nothing"* about a page that had just been opened, and review found it
        // because I had asserted on the struct.
        let note = trust.note();
        assert!(note.contains("read 1 page"), "{note}");
        assert!(
            note.contains("https://elsewhere.example/e"),
            "a reader cannot see which page was read: {note}"
        );
        assert!(note.contains("(read)"), "{note}");

        let section = trust.to_section("Trust & security posture");
        assert!(
            section
                .checked
                .iter()
                .any(|c| c.contains("elsewhere.example")),
            "{:?}",
            section.checked
        );

        // And through the whole renderer, which is what a person actually reads.
        let rendered = Analysis {
            report,
            coverage: coverage.clone(),
            pages: Vec::new(),
            stopped_early: false,
        }
        .render();
        assert!(
            rendered.contains("https://elsewhere.example/e"),
            "the page is in no surface a reader sees:
{rendered}"
        );
    }

    #[test]
    fn a_searched_page_nobody_opened_says_so_in_the_evidence() {
        // The other outcome, and they must not read alike: "we found it and did not open it"
        // is a different finding from "we read it and it said nothing".
        let found = landscape_discover::Discovered {
            sources: Vec::new(),
            checked: Vec::new(),
            stopped_early: false,
        };
        let notes: Vec<String> = Vec::new();
        let reading = Reading {
            found: &found,
            origin: "https://e.com",
            llm: &landscape_llm::LlamaClient::new("http://127.0.0.1:1".to_owned()),
            budget: &landscape_fetch::Budget::for_one_analysis(),
            memo: &memo::Extractions::new(),
            now: chrono::Utc::now(),
            notes: &notes,
        };
        let searched = vec![Searched {
            question: Answers::Trust,
            url: "https://elsewhere.example/e".to_owned(),
            read: false,
        }];
        let (_, coverage) = assemble(&reading, &[], &[], &[], &searched);
        let note = coverage
            .iter()
            .find(|c| c.question == "trust")
            .expect("a row")
            .note();
        assert!(note.contains("found and not read"), "{note}");
        assert!(note.contains("https://elsewhere.example/e"), "{note}");
        // **The outcome on the line, not only the count above it.** The mutation harness made
        // this one: `found and not read` comes from `pages_read`, and asserting only that left
        // every attempt free to say `(read)` regardless. The two outcomes are what the evidence
        // is for.
        assert!(
            note.contains("(not read)"),
            "a page nobody opened is listed as read: {note}"
        );
    }

    #[tokio::test]
    async fn every_page_search_admitted_reaches_the_coverage_it_belongs_to() {
        // The wiring, asserted. Dropping this step rebuilt every coverage row from discovery
        // alone — the defect review found — and the test for that defect passed anyway,
        // because it handed `assemble` a list the test had built rather than the run.
        let engine = Canned::holding(&["https://e.com/a", "https://elsewhere.example/b"]);
        let (admitted, _) =
            search_for_gaps(Some(&engine), "https://e.com", &[Answers::Trust], &[]).await;
        assert_eq!(admitted.len(), 2);

        let pairs = searched_pages(&admitted);
        assert_eq!(pairs.len(), admitted.len());
        for hit in &admitted {
            assert!(
                pairs.contains(&(hit.answers, hit.url.clone())),
                "{} never reached the coverage for {:?}",
                hit.url,
                hit.answers
            );
        }
    }

    #[tokio::test]
    async fn an_admitted_page_reaches_its_coverage_through_the_real_run() {
        // **The call site, not the helper.** `searched_pages` had a test and the statement that
        // calls it did not, so deleting the call left every coverage row built from discovery
        // alone with a green suite. Nothing here resolves — `.invalid` is reserved and never
        // does — so discovery finds nothing, every question is a gap, and the one page search
        // admits fails to fetch. That is enough: coverage has to know the page existed.
        let engine = Canned::holding(&["https://found.invalid/trust"]);
        let analysis = analyze_with(
            &crate::joining::offline(),
            &landscape_fetch::Budget::for_one_analysis(),
            "https://subject.invalid",
            chrono::Utc::now(),
            chrono::Utc::now().date_naive(),
            Some(&engine),
            &mut |_| Wanted::Yes,
        )
        .await;

        assert!(
            analysis
                .coverage
                .iter()
                .any(|c| c.sources.iter().any(|u| u == "https://found.invalid/trust")),
            "a page search admitted is missing from every coverage row: {:#?}",
            analysis.coverage
        );
        // And it is reported as found and not read, because the fetch failed.
        assert!(
            analysis
                .report
                .notes
                .iter()
                .any(|n| n.contains("found and not read")),
            "{:#?}",
            analysis.report.notes
        );
    }

    #[test]
    fn each_company_s_search_note_says_which_company() {
        // Review's third finding. `analyze_many` merges notes and drops duplicates, so two
        // companies with the same gaps collapsed into one sentence saying "this company".
        let gaps = [Answers::Trust];
        let first = note_for(
            "https://a.com",
            &gaps,
            &[],
            &SoFar::default(),
            Failures::NoEngine,
        )
        .expect("a note");
        let second = note_for(
            "https://b.com",
            &gaps,
            &[],
            &SoFar::default(),
            Failures::NoEngine,
        )
        .expect("a note");
        assert!(first.starts_with("a.com:"), "{first}");
        assert!(second.starts_with("b.com:"), "{second}");
        assert_ne!(
            first, second,
            "two companies with the same gaps produce one note, and the merge drops one"
        );
    }

    #[test]
    fn a_third_party_page_is_its_own_voice() {
        // Two sources in one group are not independent of each other. Filing a reviewer under
        // the company's group would tell a future corroboration count that they are one voice.
        assert_eq!(
            independence_group(
                "https://e.com",
                "https://e.com/pricing",
                Disposition::Primary
            ),
            "https://e.com"
        );
        assert_eq!(
            independence_group(
                "https://e.com",
                "https://someblog.example/e-review",
                Disposition::Unverified
            ),
            "someblog.example"
        );
    }
}
