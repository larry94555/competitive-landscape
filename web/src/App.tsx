import { useCallback, useEffect, useRef, useState } from "react";
import {
  analysisInPath,
  ApiError,
  createAnalysis,
  getAnalysis,
  getContext,
  getExamples,
  isTerminal,
  pathFor,
  searchesSent,
  searchFinished,
  watchAnalysis,
  type Analysis,
  type AnalysisStatus,
  type Examples,
  type Progress,
  type Searches,
  type Section,
} from "./api";

/**
 * One box, one question.
 *
 * The layout follows `docs/Demo_Walkthrough.md` §4: what the reader typed, then what was
 * actually searched for, then the results of that search. The order matters — a "searched
 * as" line below the results reads as a summary of them rather than the query they came
 * from, which makes correcting it feel backwards.
 */
export default function App(): React.JSX.Element {
  const [prompt, setPrompt] = useState("");
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [submitting, setSubmitting] = useState(false);
  // Nothing is known yet about a URL that names an analysis. Without this the page renders
  // its empty state for a moment first, which on a shared link reads as "there is nothing
  // here" immediately before the report appears.
  const [opening, setOpening] = useState(() => analysisInPath(window.location.pathname));
  // The ideas offered on the first screen. `null` until they arrive, and `null` for ever if
  // they do not — see `getExamples`: a missing convenience must not look like a broken page.
  const [examples, setExamples] = useState<Examples | null>(null);

  // A URL that names an analysis opens it. This is the whole of the routing: the server
  // returns the page for any path it does not claim, so what the path means is decided here.
  //
  // Runs on mount and on the browser's back and forward buttons, because both are the same
  // event from a reader's side — the address bar says one thing and the page must agree.
  // Which open is the current one. A fetch is started and answered later, and by then the
  // address bar may name something else — so the same rule the worker uses for a revoked claim
  // applies here: **carry a number, and only the newest may write.** Without it, pressing Back
  // while a slow report loaded rendered that report under `/`.
  //
  // **A ref, not a variable inside the effect.** It has to outlive one effect instance: React
  // Strict Mode runs setup, cleanup, setup on mount, and `main.tsx` renders under it — so a
  // counter scoped to the effect gives each of the two setups its own, starting at zero, and
  // the discarded mount's request still looks current when it answers.
  const newest = useRef(0);

  useEffect(() => {
    let canceled = false;
    void getExamples().then((found) => {
      if (!canceled) setExamples(found);
    });
    return () => {
      canceled = true;
    };
  }, []);

  useEffect(() => {
    const open = (): void => {
      const mine = ++newest.current;
      const id = analysisInPath(window.location.pathname);
      setOpening(id);
      if (id === null) {
        // Bumping `newest` above is what cancels whatever was in flight.
        setAnalysis(null);
        return;
      }
      void getAnalysis(id)
        .then((found) => {
          if (mine !== newest.current) return;
          setAnalysis(found);
          setError(null);
        })
        .catch((e: unknown) => {
          if (mine !== newest.current) return;
          // A malformed id and a deleted one are the same situation from here, and the API
          // says so with one 404 rather than two different failures.
          setError(e instanceof ApiError ? e : new ApiError("Something went wrong."));
          setAnalysis(null);
        })
        .finally(() => {
          // Including here: an older `finally` would clear the *newer* open's state and take
          // "Opening this report…" off the screen while it was still true.
          if (mine === newest.current) setOpening(null);
        });
    };
    open();
    window.addEventListener("popstate", open);
    return () => {
      // Bumping it here invalidates anything this instance started — the discarded half of a
      // Strict Mode double-mount, and a real unmount, which should not write either.
      newest.current += 1;
      window.removeEventListener("popstate", open);
    };
  }, []);

  /**
   * Run one analysis, whatever put the words there.
   *
   * **The box and a chip are the same act**, so they are the same function. When they were
   * two, everything below — the URL, the cleared box, the error that survives — was written
   * once and would have had to be remembered twice.
   */
  const start = useCallback(async (text: string) => {
    setError(null);
    setSubmitting(true);
    try {
      const started = await createAnalysis(text);
      setAnalysis(started);
      // The URL names it from the moment it exists, so a reader who reloads — or sends the
      // link to somebody — gets the run rather than an empty box. `pushState` rather than a
      // navigation: the page is already the right page.
      window.history.pushState({}, "", pathFor(started.id));
      // Clear only once it has been accepted. The empty box is what tells an
      // unregistered reader they have spent their one analysis — a box still holding
      // their words invites them to press Analyze again and be refused.
      //
      // On failure the text stays: they have to edit it, and retyping something they
      // just wrote is a worse punishment for a typo than the typo deserved.
      setPrompt("");
    } catch (e) {
      setError(
        e instanceof ApiError ? e : new ApiError("Something went wrong."),
      );
    } finally {
      setSubmitting(false);
    }
  }, []);

  const submit = useCallback(() => start(prompt), [start, prompt]);

  const { status, sections, subjects, progress, generation } = useReport(
    analysis,
    setAnalysis,
  );

  if (opening !== null) {
    // A shared link lands here first. Rendering the empty box for the moment the fetch takes
    // reads as "there is nothing here", which is the opposite of what the link promised.
    return (
      <main>
        <p>Opening this report…</p>
      </main>
    );
  }

  return (
    <main>
      <h1>What is your idea?</h1>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={3}
          aria-label="What is your idea?"
        />
        <button type="submit" disabled={submitting || prompt.trim() === ""}>
          {submitting ? "Starting…" : "Analyze"}
        </button>
      </form>

      {/*
        Ideas to start from, below the box rather than above it. A reader who came with their
        own idea should meet the box first; one who came to look around finds these without
        having to invent a prompt — and the prompts people invent are the ones this pipeline
        cannot resolve, so an empty box is where a demo dies.

        Clicking one **fills the box and does not submit**, and what it fills in is the idea
        and nothing else. These used to read *"project management for a small design agency -
        basecamp.com vs linear.app"*, which made every example a **named set**: nothing was
        discovered, and the first screen of the product demonstrated the one path where its
        central feature does not run.
      */}
      {analysis === null && examples !== null && examples.examples.length > 0 && (
        <section className="examples" aria-labelledby="examples-heading">
          <h2 id="examples-heading">Or start from one of these</h2>
          <ul>
            {examples.examples.map((example) => (
              <li key={example.id}>
                <button
                  type="button"
                  className="example"
                  onClick={() => {
                    setPrompt(example.prompt);
                    setError(null);
                  }}
                >
                  <strong>{example.idea}</strong>
                </button>
              </li>
            ))}
          </ul>
          {/*
            **Before a run is spent, not after.** Every idea here is a description, and
            resolving one into companies needs a search engine. Without it each of these
            refuses — and a reader who clicks first learns about an environment variable at the
            cost of one of their analyses. That happened, which is why this is here.
          */}
          {!examples.discovery && (
            <p className="unavailable" role="status">
              <strong>Searching is not configured on this instance</strong>, so these ideas
              cannot be researched yet — set <code>SEARX_URL</code>. Naming a website still
              works, though finding the companies is the part this is for.
            </p>
          )}
          {/*
            The server's sentence, not one written here. What is curated and what is not is a
            claim about the product, and a claim that lives only in a component is one nobody
            reviews.
          */}
          <p className="curation">{examples.note}</p>
        </section>
      )}

      {error && (
        <p role="alert">
          {error.message}
          {error.remedy && <span> {error.remedy}</span>}
          {/*
            Selectable, monospaced and on its own line, because the only thing a reader
            ever does with this is copy it into a message to us. A reference buried in a
            sentence gets retyped, and a retyped reference has a digit wrong.
          */}
          {error.reference && (
            <>
              {" "}
              <code className="reference">{error.reference}</code>
            </>
          )}
        </p>
      )}

      {analysis && (
        <AnalysisView
          analysis={analysis}
          status={status}
          sections={sections}
          subjects={subjects}
          progress={progress}
          generation={generation}
          onPick={(text) => void start(text)}
          picking={submitting}
        />
      )}
    </main>
  );
}

/** How long to wait before opening the stream again after it drops mid-run. */
const RECONNECT_MS = 1000;

/**
 * Watch a report being written, and fetch it once it is.
 *
 * **The stream carries sections; the fetch carries the truth.** A section arrives as soon as
 * it has claims, which is what a reader is waiting for. What the stream deliberately does not
 * send is the sections that found nothing — those carry a "we checked and there was nothing"
 * note that is only true once the run is over, and showing it early would tell somebody we
 * had finished looking when we had not.
 *
 * So the final `getAnalysis` is not a fallback. It is the step that turns what a reader has
 * been watching into the report, coverage notes and all.
 *
 * **Only a terminal status ends this.** A running analysis already carries the report so far,
 * so "we have a report" is not "it is finished" — and treating it as such is how a dropped
 * stream became permanent: one fetch, a still-running answer, and nothing ever reconnected.
 * The stream ends by itself on an error and after ten minutes, both of which happen to a tab
 * left open, so the reconnect is the ordinary path rather than the exceptional one.
 */
function useReport(
  analysis: Analysis | null,
  onFinished: (a: Analysis) => void,
): {
  status: AnalysisStatus | null;
  sections: readonly Section[];
  subjects: readonly string[];
  progress: Progress | null;
  /**
   * Which run the progress belongs to.
   *
   * **State rather than a ref, unlike the copy the callbacks compare against.** A reclaim means
   * the numbers on screen describe a worker that no longer exists, so a change here has to
   * reach the render - which is the one thing the ref was documented as never needing to do.
   */
  generation: number | null;
} {
  const [status, setStatus] = useState<AnalysisStatus | null>(null);
  const [sections, setSections] = useState<readonly Section[]>([]);
  // How far the run has got. `null` before the first tick and after the run ends — the page
  // reads a terminal status as the end, not the absence of a bar.
  const [progress, setProgress] = useState<Progress | null>(null);
  // The companies this run set out to cover, heard from the stream. State rather than a ref:
  // it decides whether every claim on screen carries a label, so hearing it has to render.
  const [subjects, setSubjects] = useState<readonly string[]>([]);
  const [attempt, setAttempt] = useState(0);
  const onFinishedRef = useRef(onFinished);
  onFinishedRef.current = onFinished;
  // Read inside the stream callbacks, which outlive the render that created them.
  const analysisRef = useRef(analysis);
  analysisRef.current = analysis;
  // Which run the sections in state belong to. A ref rather than state because it is compared
  // and written inside callbacks; the copy below is the one a render reads.
  const generationRef = useRef<number | null>(null);
  const [generation, setGeneration] = useState<number | null>(null);

  const id = analysis?.id ?? null;
  const settled = analysis != null && isTerminal(analysis.status);

  // A new analysis starts from nothing. Kept apart from the connection below so that
  // reconnecting does not wipe the sections the reader is already looking at.
  //
  // **During the render, not in an effect, and that is the whole of a reported defect.** An
  // effect runs *after* the browser has painted, so the render in which `analysis` became the
  // new run still read the previous run's sections, subjects and progress — and the reader saw
  // one frame of the last report under the new one's heading before it cleared. Pressing
  // Analyze a second time showed the first report flash past. A reader reported exactly that.
  //
  // Setting state during a render is the documented way to reset it when an input changes:
  // React discards the output and re-renders immediately, before anything reaches the screen.
  // There is no frame to see.
  const [ranFor, setRanFor] = useState(id);
  if (id !== ranFor) {
    setRanFor(id);
    setStatus(null);
    setSections([]);
    setSubjects([]);
    setProgress(null);
    setAttempt(0);
    generationRef.current = null;
    setGeneration(null);
  }

  /**
   * The run these sections belong to, from wherever we heard it.
   *
   * **One decision, two inputs.** The stream says so on every connection and the recovery
   * fetch says so when the stream has dropped, and both have to reach the same conclusion —
   * a drop, a reclaim and a reconnect is precisely the sequence where they arrive in the
   * wrong order and the reader is left holding a dead worker's answers.
   */
  const sawGeneration = useCallback((generation: number) => {
    const held = generationRef.current;
    generationRef.current = generation;
    setGeneration(generation);
    if (held === null || held === generation) return;
    // **The progress belonged to the worker that died.** A reclaim does not have to pass back
    // through `queued` from this component's side: the stream can drop during the sweep and
    // reconnect with the replacement already `running`, so `status` never stops being
    // `running` and nothing else here would clear this. Left alone, the new run's discovery
    // inherits the dead one's percentage as a floor and its elapsed time as a start.
    setProgress(null);
    // The run started over. Two copies of the old run's answers are in play: the sections
    // this hook accumulated, which survive a reconnect on purpose, and the partial report a
    // recovery fetch cached on the analysis.
    setSections([]);
    const current = analysisRef.current;
    if (current?.report != null) {
      onFinishedRef.current({ ...current, report: null });
    }
  }, []);

  useEffect(() => {
    if (id === null || settled) return;

    let canceled = false;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const reconnect = (): void => {
      if (canceled) return;
      retry = setTimeout(() => {
        if (!canceled) setAttempt((n) => n + 1);
      }, RECONNECT_MS);
    };

    const close = watchAnalysis(id, {
      onStatus: (next) => {
        if (!canceled) setStatus(next);
      },
      onSection: (section) => {
        if (canceled) return;
        // Arrival order, and the newest version of each. A section is sent again whenever it
        // changes: pinning the first copy left "What it does: 1 item" on screen for two
        // minutes while eight more were read, which reads as a section that is finished.
        setSections((current) => {
          const at = current.findIndex((s) => s.key === section.key);
          if (at === -1) return [...current, section];
          const next = [...current];
          next[at] = section;
          return next;
        });
      },
      onGeneration: (generation) => {
        if (!canceled) sawGeneration(generation);
      },
      onSubjects: (next) => {
        // Not cleared on a new generation: the replacement run is comparing the same
        // companies, and a screen that stops labeling halfway through a restart is the
        // defect this exists to prevent, wearing a different hat.
        if (!canceled) setSubjects(next);
      },
      onProgress: (next) => {
        if (!canceled) setProgress(next);
      },
      onDone: () => {
        if (canceled) return;
        void getAnalysis(id)
          .then((latest) => {
            if (canceled) return;
            // Before the analysis is replaced below, so the clearing acts on what the reader
            // is holding rather than on what has just arrived.
            sawGeneration(latest.generation);
            // The stream's last word was "running" and the row now says otherwise. Leaving
            // the old value in state would have the page still saying "Reading…" over a
            // finished report.
            setStatus(latest.status);
            onFinishedRef.current(latest);
            // Still running: the stream dropped or timed out rather than finishing, so open
            // it again. Without this the reader keeps a half-written report for ever.
            if (!isTerminal(latest.status)) reconnect();
          })
          .catch(() => {
            // The reader keeps what the stream already gave them, and we try again — a red
            // banner over a report they can see would be worse than a stale one.
            reconnect();
          });
      },
    });

    return () => {
      canceled = true;
      if (retry !== undefined) clearTimeout(retry);
      close();
    };
  }, [id, settled, attempt]);

  return { status, sections, subjects, progress, generation };
}

function AnalysisView({
  analysis,
  status,
  sections,
  subjects,
  progress,
  generation,
  onPick,
  picking,
}: {
  analysis: Analysis;
  status: AnalysisStatus | null;
  sections: readonly Section[];
  subjects: readonly string[];
  /** How far the run has got. `null` before the first tick, and once it is over. */
  progress: Progress | null;
  /** Which run that progress belongs to. A change means a different worker is doing it. */
  generation: number | null;
  /** Run this instead. The chip hands back a whole prompt, not a company name. */
  onPick: (prompt: string) => void;
  /** A run is already starting. Two clicks would spend two analyses on one question. */
  picking: boolean;
}): React.JSX.Element {
  const { report } = analysis;
  // **The stored status wins once it is terminal.** A dropped stream's last word was
  // "running", and a recovery fetch that comes back complete must not leave a finished
  // report sitting under "Reading public web pages…".
  const showing_status = isTerminal(analysis.status)
    ? analysis.status
    : (status ?? analysis.status);
  const live = !isTerminal(showing_status);
  // While it runs, the stream is the live copy; when it is over, the report is. A recovery
  // fetch stores a *partial* report mid-run, and reading from that would both freeze the
  // page at the moment of the fetch and show placeholder sections for questions still being
  // read — the two things the stream exists to avoid.
  const showing = live ? stillArriving(sections, report) : (report?.sections ?? sections);
  // Whether this report covers more than one company, which decides whether each claim has to
  // say whose it is.
  //
  // **From the companies analyzed, not from the ones that produced a claim.** Deriving it from
  // the claims on screen looks reasonable and is wrong in the case that matters: ask about two
  // companies, have one of them say nothing, and the survivor's prices lose their label in a
  // report that is still a comparison.
  //
  // Two sources because there are two moments. The finished report carries `subjects`; while it
  // runs there is no report yet, so the stream says so directly. Leaving the live case to be
  // guessed from the claims is the same defect one surface later — it was, and review found it
  // there too.
  const analyzed = (report?.subjects?.length ?? 0) > 0 ? (report?.subjects ?? []) : subjects;
  // **Read from the failure, not only from the list.** A run that later succeeded can still be
  // holding the question an earlier attempt asked; offering it under a finished report would
  // invite a reader to re-run something they can already read. The server clears it for the
  // same reason — `analysis_from_row` — and neither side relies on the other having done so.
  const choices =
    analysis.status === "failed" && analysis.failure === "ambiguous"
      ? analysis.choices
      : [];
  const several =
    analyzed.length > 1 ||
    // Only for reports stored before `subjects` existed, which deserialize without it.
    new Set(
      showing
        .flatMap((s) => s.claims)
        .map((c) => c.subject)
        .filter((s) => s !== ""),
    ).size > 1;

  return (
    <section aria-live="polite">
      {/*
        **Running or finished, and how far through.** Before this the page said
        `Reading public web pages…` for the whole of a four-to-eight-minute run — one sentence
        as true at eight seconds as at eight minutes, so a reader could not tell a run that had
        nearly finished from one that had barely started, or either from one that had died.

        Three things carry the distinction, deliberately, because one of them is not enough for
        everybody: a **word** (`Working` against `Done`), a **bar** that is either moving or
        full and still, and a **number** when there is one. Color is not among them —
        `CODING_QUALITY.md` §9.5 asks that it never be the sole encoding.

        **Above the four blocks rather than inside them.** Rendered, `Done.` was landing between
        the reader's own words and how those words were read — splitting the two blocks that
        together answer *"did it understand me?"*. A status line is chrome; it goes where chrome
        goes. Found by looking at the page, which is the only way this kind of defect is found.
      */}
      <Waiting
        status={showing_status}
        generation={generation}
        progress={progress}
        failure={analysis.failure}
        offered={choices.length}
      />

      {/*
        **Four blocks, in the order a reader checks them** — `PRODUCT_IDEA_RESULTS.md` §2: what
        you asked, how that was read, what is here, and the lists. Everything the previous page
        put above the results — the interpreted line, `Searched as`, the prompt as a heading —
        said the same three facts in five places and in no particular order.
      */}
      <Asked prompt={analysis.prompt} />

      {/*
        Anything true of the whole report rather than one section — today, which companies were
        named and not analyzed. It sits above the results because it changes what they mean.
      */}
      {report?.notes?.map((note) => (
        <p key={note} className="report-note">
          {note}
        </p>
      ))}

      {/*
        **How the idea was read, and what is here.** Both need a finished report: the
        interpretation because a half-run one has not decided it, and the count because a number
        that keeps climbing is not a count.
      */}
      {report && isTerminal(showing_status) && (
        <>
          <Reading report={report} prompt={analysis.prompt} names={analyzed} />
          <Count report={report} companies={analyzed.length} />

          {/*
            **Without the scheme.** `subjects` holds origins — `https://basecamp.com` —
            because that is what gets fetched, and a column of `https://` is five characters of
            noise on every row that tell a reader nothing. `EditableSet` already made this
            decision for the same strings; this is that decision, not a second one.
          */}
          <Listing
            heading="Companies"
            items={analyzed.map(withoutScheme)}
            note={missed(report.searches)}
          />
          {/*
            **Two headings with nothing under them, on purpose.** Neither search exists —
            `PRODUCT_IDEA_RESULTS.md` §4.1 — and §2.5 asks that a category nobody looked in say
            so rather than show an empty list, which would be this product claiming it looked.
          */}
          <NotSearched heading="Open source projects" />
          <NotSearched heading="Discussions" />

          {/*
            **The set, correctable.** `COMPETITIVE_DISCOVERY.md` §5.5 and §6.3: a competitive set
            presented without a way to correct it is an unfalsifiable editorial choice, and
            direct manipulation beats interrogation — one glance confirms or corrects what was
            decided, with no prompt literacy and no question spent.

            Under the list, because a reader corrects the set having read it rather than before.
          */}
          {analyzed.length > 0 && (
            <EditableSet companies={analyzed} onRun={onPick} running={picking} />
          )}

          {/*
            **The evidence file, on the clipboard.** `IDEA_ANALYSIS.md` §5: most readers
            evaluating an idea already pay for a frontier chatbot, and the honest response is to
            feed it rather than compete with it.
          */}
          <CopyAsContext id={analysis.id} />
        </>
      )}

      {/*
        The question itself, one button per company.
        `PRODUCT_SPEC.md` §3 costs a clarification at one click, and this is where that is
        either true or a sentence in a document. Each chip carries a whole prompt from the
        server, so clicking one is the entire answer — nothing here asks a reader to retype
        their idea with a company bolted onto it.

        **Buttons, not links.** Picking starts a run; it does not navigate to something that
        already exists. A link here would offer a middle-click that leads nowhere.
      */}
      {choices.length > 0 && (
        <ul className="choices">
          {choices.map((choice) => (
            <li key={choice.domain}>
              <button
                type="button"
                className="choice"
                disabled={picking}
                onClick={() => onPick(choice.prompt)}
              >
                <strong>{choice.name}</strong>
                {/*
                  The domain is not decoration: two products sharing a name is exactly the
                  case this screen exists for, so it is the field that tells them apart.
                */}
                {/*
                  Empty when the choice is a *market* rather than a company — there is no
                  website to show, and inventing one would be a claim. What stands in its
                  place is `what_it_is`, which says how many sites agreed on the name.
                */}
                {choice.domain !== "" && (
                  <span className="domain">{choice.domain}</span>
                )}
                {choice.what_it_is !== "" && (
                  <span className="what">{choice.what_it_is}</span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}

      {/*
        **The claims, off the first screen but on the same page.** §3 of
        `PRODUCT_IDEA_RESULTS.md` sends the detail to a separate view and a downloadable report;
        neither is built (open issue 9), and deleting the sections in the meantime would lose
        the only evidence this product has. A disclosure is the smallest honest stand-in: the
        first screen is the four blocks, and nothing was thrown away.

        **Closed from the first paint, including while the run is going.** It was `open={live}`,
        on the reasoning that a `<details>` hiding the only moving thing would make a working
        analysis look stalled — but the moving thing is the bar above it, and what the open
        state actually did was show the *old* page for the whole of a run and then replace it
        with this one at the end. A reader reported seeing exactly that.
      */}
      {showing.length > 0 && (
        <details className="detail">
          <summary>
            The full report — what public sources say about{" "}
            {analyzed.length === 1 ? analyzed[0] : "these companies"}
          </summary>
          {showing.map((section) => (
                <article key={section.key}>
              <h3>{section.title}</h3>
              {section.claims.length === 0 ? (
                <div className="gap">
                  <strong>Nothing found in public sources.</strong>
                  {section.checked.length > 0 && (
                    <ul>
                      {section.checked.map((what) => (
                        <li key={what}>{what}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ) : (
                <ul>
                  {section.claims.map((claim) => (
                    <li key={`${claim.source_label}-${claim.text}`}>
                      {/*
                        Whose is this? A claim says what the page said — "Pro costs $15" — and
                        never names the company, so on a report covering several a reader would be
                        looking at two prices with no way to tell them apart. Shown only when there
                        is more than one, because repeating the same name down a single-company
                        report is noise.
                      */}
                      {several && claim.subject !== "" && (
                        <>
                          {/*
                            The space is written rather than left to CSS: JSX drops the whitespace
                            around a newline, and without it the line reads `basecamp.comPro costs
                            $15`. A margin would space the box and leave the *text* — which is what
                            a reader copies, and what a test reads — still run together.
                          */}
                          <strong className="subject">{withoutScheme(claim.subject)}</strong>{" "}
                        </>
                      )}
                      {claim.text} <cite>[{claim.source_label}]</cite>
                      {claim.evidence_quote !== "" && (
                        <blockquote>{claim.evidence_quote}</blockquote>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </article>
          ))}
        </details>
      )}

      {/*
        While it runs, say what is still coming. An empty space below three sections reads
        as "that is all there is", and the reader closes the tab before the fourth arrives.
      */}
      {live && (
        <p className="pending">
          {showing.length === 0
            ? "Reading the first pages…"
            : "Still reading. More sections will appear here."}
        </p>
      )}
    </section>
  );
}

/**
 * What to show while the run is still going.
 *
 * The stream is the live copy and arrives in the order sections were finished. A partial
 * report from a recovery fetch fills the gaps a dropped connection left — but **only its
 * sections that have claims**: a partial report carries all six from its first write, each
 * already holding the coverage note it will have *if* nothing is found, and rendering
 * "Nothing found in public sources" for a question still being read tells a reader we have
 * finished looking when we have not.
 */
/** An origin without its scheme, which is how a person names a company. */
/**
 * The companies in this report, with a way to change them and run it again.
 *
 * **The prompt is the whole answer**, exactly as a clarifying chip is: the button hands back a
 * list of origins separated by spaces, which is what a reader could have typed, and the run
 * treats it as a named set. Nothing new is invented on the way in.
 *
 * **No origin parsing happens here.** `landscape_analyze::subject::origins_in` owns what counts
 * as a company and how two spellings of one are the same — a second copy in TypeScript would be
 * a rule that agrees today. What this does is refuse to submit an empty set, and refuse to add
 * something with no dot or a space in it, which is a courtesy to stop an obvious typo costing
 * ninety seconds rather than a parser. Whatever survives that, the run decides on.
 *
 * **Sameness here is what the page renders, which is a question about this page.** Every
 * comparison below goes through `asShown`, so two entries this component would draw as one chip
 * are one company as far as it is concerned. That is not an opinion about origins; it is the
 * opinion this page already published when it drew the chip, and holding it in one place is what
 * stops the interface asking for a spelling it then treats as new.
 *
 * **Nothing keeps the edits in step with a changing report, and nothing needs to.** The set is
 * only offered once the run is over, and correcting it starts another one — so between the two
 * reports the guard above unmounts this, and the second set is read fresh. A `useEffect` copying
 * the prop into state, or a `key`, would be a second answer to a question already answered, and a
 * second answer is a thing that can disagree.
 */
/**
 * The whole report as Markdown, on the clipboard in one click.
 *
 * **This is the product's argument for itself, as a button.** `IDEA_ANALYSIS.md` §5: we are not
 * a worse chatbot, we are the evidence file a chatbot cannot assemble — so the last thing a
 * reader does with a report is hand it to the assistant they already use, with a URL and a date
 * against every sentence.
 *
 * **The bytes are the server's.** `getContext` fetches what `curl` would get, which is what
 * `landscape_core::context` wrote. A copy of that renderer here would be a second opinion about
 * what `Attributed` means.
 *
 * **And the clipboard is allowed to say no.** `navigator.clipboard` is absent outside a secure
 * context and can be refused by permission policy, and a button that silently does nothing is
 * worse than one that admits it — so a failure puts the text on the page, selected, with a
 * sentence saying to copy it by hand.
 */
function CopyAsContext({ id }: { id: string }): React.JSX.Element {
  const [state, setState] = useState<"idle" | "working" | "copied">("idle");
  const [fallback, setFallback] = useState("");
  const [failed, setFailed] = useState("");

  // **Two failures, and they are not the same failure.** Fetching can fail, in which case
  // there is nothing to offer; the clipboard can refuse, in which case we still have the
  // document and the reader can have it. One request either way — asking the server twice for
  // bytes already in hand would be this button paying for its own error path.
  const copy = async (): Promise<void> => {
    setState("working");
    setFailed("");
    setFallback("");

    let markdown: string;
    try {
      markdown = await getContext(id);
    } catch (whatever) {
      setState("idle");
      setFailed(
        whatever instanceof ApiError
          ? whatever.message
          : "We could not put the report together. Try again in a moment.",
      );
      return;
    }

    try {
      // Absent outside a secure context, and refusable by permission policy. A button that
      // silently does nothing is worse than one that admits it.
      if (!navigator.clipboard?.writeText) {
        throw new Error("no clipboard here");
      }
      await navigator.clipboard.writeText(markdown);
      setState("copied");
    } catch {
      setState("idle");
      setFallback(markdown);
    }
  };

  return (
    <section className="as-context" aria-label="Copy this report for an assistant">
      <button type="button" onClick={() => void copy()} disabled={state === "working"}>
        {state === "copied" ? "Copied" : "Copy as context"}
      </button>
      <p className="hint">
        The whole report as Markdown, with every source URL and date — paste it into the
        assistant you already use.
      </p>

      {failed !== "" && (
        <p className="refused" role="alert">
          {failed}
        </p>
      )}

      {fallback !== "" && (
        <>
          <p className="refused" role="alert">
            This browser would not let us reach the clipboard. Here it is — select it and copy.
          </p>
          <textarea readOnly aria-label="The report as Markdown" value={fallback} rows={12} />
        </>
      )}
    </section>
  );
}

/**
 * How this page shows a company, and therefore the only sameness it is entitled to judge.
 *
 * **The interface asks for the schemeless form.** The chip reads `basecamp.com`, the box beside
 * it says `example.com`, and a reader who types what is on screen was adding the company already
 * there. Comparing the stored strings said otherwise, so the set grew, the button lit up, and the
 * run put the two back together and returned the report being looked at.
 *
 * This makes no claim `origins_in` does not — it claims something smaller and about this file:
 * **two things drawn identically are one thing.** Anything subtler than what the page renders is
 * still the run's to decide, and the run still decides it.
 */
function asShown(origin: string): string {
  return withoutScheme(origin.trim());
}

function EditableSet({
  companies,
  onRun,
  running,
}: {
  companies: readonly string[];
  /** Run this instead. A whole prompt, as the clarifying chips hand back. */
  onRun: (prompt: string) => void;
  /** A run is already starting; two clicks would spend two analyses on one question. */
  running: boolean;
}): React.JSX.Element {
  const [set, setSet] = useState<readonly string[]>(companies);
  const [typed, setTyped] = useState("");
  const [refused, setRefused] = useState("");

  const add = (): void => {
    const wanted = typed.trim();
    if (wanted === "") {
      return;
    }
    if (!wanted.includes(".") || /\s/.test(wanted)) {
      setRefused(`${wanted} does not look like a domain. Try example.com.`);
      return;
    }
    // **A company added twice is the "already on screen" cost by another route.** The list gets
    // longer, so the button lights up, and the run puts the duplicate back together — ninety
    // seconds to redraw the report being looked at.
    if (shown.has(asShown(wanted))) {
      setRefused(`${asShown(wanted)} is already in this set.`);
      return;
    }
    setSet([...set, wanted]);
    setTyped("");
    setRefused("");
  };

  const shown = new Set(set.map(asShown));

  // **Different, as this page shows it.** Comparing the stored strings in order made two edits
  // that cancel out — remove a company, add it back the way its chip spells it — look like a new
  // question, and the answer was the report already on screen. A competitive set is a set: the
  // members decide whether it is a different question, and the run decides their order.
  const changed =
    set.length !== companies.length ||
    companies.some((company) => !shown.has(asShown(company)));

  return (
    <section className="editable-set" aria-label="The companies in this report">
      <h3>Comparing</h3>
      <ul className="set">
        {set.map((company) => (
          <li key={company}>
            {withoutScheme(company)}
            <button
              type="button"
              className="drop"
              aria-label={`Remove ${withoutScheme(company)}`}
              onClick={() => setSet(set.filter((kept) => kept !== company))}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      <div className="add">
        <label htmlFor="add-company">Add a company</label>
        <input
          id="add-company"
          type="text"
          value={typed}
          placeholder="example.com"
          onChange={(e) => setTyped(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
        />
        <button type="button" onClick={add}>
          Add
        </button>
      </div>

      {refused !== "" && (
        <p className="refused" role="alert">
          {refused}
        </p>
      )}

      {/*
        **Disabled until something is different**, because re-running the set a reader is already
        looking at spends ninety seconds to redraw the page they have. And disabled when the set
        is empty: a comparison of nothing is not a report.
      */}
      <button
        type="button"
        className="rerun"
        disabled={running || set.length === 0 || !changed}
        onClick={() => onRun(set.join(" "))}
      >
        Run this set
      </button>
      {set.length === 0 && (
        <p className="refused">Removing every company leaves nothing to compare.</p>
      )}
    </section>
  );
}

function withoutScheme(origin: string): string {
  return origin.replace(/^https?:\/\//, "");
}

function stillArriving(
  streamed: readonly Section[],
  report: Analysis["report"],
): readonly Section[] {
  const merged = [...streamed];
  for (const section of report?.sections ?? []) {
    if (section.claims.length === 0) continue;
    if (!merged.some((s) => s.key === section.key)) merged.push(section);
  }
  return merged;
}

/** How many rows a list shows before `…more`, and how many it will ever hold. */
const SHOWN = 5;
const CAP = 25;

/**
 * The reader's own words, clamped to two lines.
 *
 * **Two lines is a measurement, not a character count.** A clamp counted in characters is wrong
 * at every window width, and this page is read on a phone - so the clamp is CSS and the `…`
 * appears only when the browser reports that the text is actually cut.
 *
 * **The tooltip and the expand are not alternatives.** A tooltip reaches nobody on a touch
 * screen and nobody using a keyboard; the expand is what serves them. Both show the same text.
 */
function Asked({ prompt }: { prompt: string }): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const [clipped, setClipped] = useState(false);
  const words = useRef<HTMLParagraphElement | null>(null);

  // Measured rather than guessed, and re-measured when the window changes, because the same
  // sentence is two lines wide on a laptop and four on a phone.
  //
  // **`open` is in the dependencies, and that is the fix.** Expanded, the paragraph is exactly
  // as tall as its content, so `scrollHeight === clientHeight` by construction — a resize in
  // that state recorded *"not clipped"* about text that is, and collapsing afterwards left the
  // prompt clamped with the ellipsis gone and no way to reopen it. Review found it. Collapsing
  // now re-runs this and takes a fresh measurement in the only state that can answer.
  //
  // The early return below is not what fixes it — the harness said so, by removing it and
  // watching every test still pass. It is kept because measuring while open is work that can
  // only produce a wrong answer, and it is one line.
  useEffect(() => {
    if (open) return undefined;
    const measure = (): void => {
      const el = words.current;
      if (el) setClipped(el.scrollHeight > el.clientHeight + 1);
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [prompt, open]);

  return (
    <div className="asked">
      <h2 className="asked-label">What you asked</h2>
      <p
        className={open ? "prompt" : "prompt clamped"}
        ref={words}
        title={prompt}
      >
        {prompt}
      </p>
      {(clipped || open) && (
        <button
          type="button"
          className="more-dots"
          onClick={() => setOpen((was) => !was)}
        >
          {open ? "Show less" : "…"}
        </button>
      )}
    </div>
  );
}

/**
 * How the idea was read, and what the reader can do about it.
 *
 * **Three cases, and the first question is not the one about substitution.** `interpreted` is
 * `None` whenever nothing was replaced - which includes a reader who typed a phrase the market
 * already uses. Telling them *"you named these directly"* would be false about the one thing
 * they are being asked to check. So the class of input decides the sentence, and the
 * substitution decides only whether there is anything to justify.
 */
function Reading({
  report,
  prompt,
  names,
}: {
  report: NonNullable<Analysis["report"]>;
  prompt: string;
  /** The set as analyzed, in the order it will be listed. */
  names: readonly string[];
}): React.JSX.Element | null {
  const given = report.asked ?? null;
  const phrase = report.interpreted?.label ?? null;

  if (given?.kind === "named" || given?.kind === "seeded") {
    return (
      <div className="reading">
        <p>
          You named{" "}
          <span className="phrase">
            {given.kind === "seeded" ? given.named : names.join(", ")}
          </span>{" "}
          directly.
        </p>
      </div>
    );
  }

  // **A report stored before `asked` existed knows none of this.** `#[serde(default)]` gives it
  // `null`, and every sentence below is a claim about how the idea was read - so the honest
  // rendering of "we did not record it" is the interpretation if there is one and silence
  // otherwise. Telling somebody who typed two domains that we searched for their words as they
  // wrote them would be the same defect as the one this component was built to avoid, arriving
  // through the back door of an old row.
  if (given === null && phrase === null) return null;

  // A description. Whether the market's words replaced the reader's decides the sentence and
  // whether there is anything to explain.
  return (
    <div className="reading">
      {phrase !== null ? (
        <>
          <p>
            Here&apos;s how I interpreted the business idea:{" "}
            <span className="phrase">{phrase}</span>
          </p>
          <p className="backing">
            {report.interpreted?.hosts ?? 0} independent{" "}
            {(report.interpreted?.hosts ?? 0) === 1 ? "site" : "sites"} use this
            name for it.
            {(report.interpreted?.also.length ?? 0) > 0 && (
              <span className="also">
                {" "}
                Also called {report.interpreted?.also.join(", ")}.
              </span>
            )}
          </p>
        </>
      ) : (
        <p>
          I searched for your words as you wrote them:{" "}
          <span className="phrase">{prompt}</span>
        </p>
      )}
    </div>
  );
}

/**
 * One sentence saying what is here.
 *
 * **"Found" is a claim about provenance and it is false for two of the three input classes.** A
 * named set is handed straight through with no discovery of any kind, so saying *"I found 3
 * companies"* to somebody who typed all three takes credit for reading a list.
 *
 * **And a count over an unfinished search is a definite number about an indefinite thing**, so
 * a partial search says *"at least"*. Both facts come from the report rather than being
 * re-derived here - see `landscape_core::given`.
 */
function Count({
  report,
  companies,
}: {
  report: NonNullable<Analysis["report"]>;
  companies: number;
}): React.JSX.Element {
  const given = report.asked ?? null;
  const sure = given?.kind === "named" || searchFinished(report.searches);
  const about = sure ? "" : "at least ";

  let clause: string;
  if (given?.kind === "named") {
    clause = `You named ${String(given.count)} ${plural(given.count, "company", "companies")}.`;
  } else if (given?.kind === "seeded") {
    const others = Math.max(0, companies - 1);
    clause =
      others === 0
        ? `You named ${given.named}.`
        : `You named ${given.named}, and I found ${about}${String(others)} more like it.`;
  } else if (given === null) {
    // Same old row, and the same rule: state the number, claim nothing about where it came
    // from. "I found" is the one word here that could be false.
    clause = `${String(companies)} ${plural(companies, "company", "companies")}.`;
  } else {
    clause = `I found ${about}${String(companies)} ${plural(companies, "company", "companies")}.`;
  }

  return (
    <p className="count">
      <span>{clause}</span>{" "}
      {/*
        **Not "0 projects, 0 discussions".** A zero is a claim that we looked, and neither
        search exists yet — `PRODUCT_IDEA_RESULTS.md` §2.5 and §4.1. Saying so is the only
        honest shape available until they do.
      */}
      <span className="not-yet">
        I have not looked for open source projects or discussions — neither search is built
        yet.
      </span>
    </p>
  );
}

function plural(n: number, one: string, many: string): string {
  return n === 1 ? one : many;
}

/**
 * What did not come back, when something did not.
 *
 * **The other half of *"at least"*.** `PRODUCT_IDEA_RESULTS.md` §2.5 asks for the hedge *and* a
 * line naming what was missed, because the hedge alone tells a reader the number is soft
 * without telling them whether re-running would plausibly change it. One failed search out of
 * eight and six out of eight are the same word and very different decisions.
 *
 * `null` when the search finished, and when nothing was asked at all — neither is a partial
 * search, and a line about coverage over either would be inventing one.
 */
function missed(searches: Searches | null | undefined): string | undefined {
  if (searches == null || searchFinished(searches)) return undefined;
  // **The noun agrees with the total, not with the failures.** `1 of 3 search` is what
  // pluralising on `failed` produces, and the first version of this did exactly that; the
  // test caught it on the seeded case.
  const sent = searchesSent(searches);
  return `${String(searches.failed)} of ${String(sent)} ${plural(
    sent,
    "search",
    "searches",
  )} did not come back.`;
}

/**
 * One of the three result lists: at most 25, five at a time.
 *
 * **The count and the cap are different numbers and both are shown.** `…more` names what it can
 * actually reveal rather than what is not on screen, and the "showing" line counts what is
 * visible now — a control that promises more than it delivers is checkable by the person
 * reading it, and wrong.
 */
function Listing({
  heading,
  items,
  note,
}: {
  heading: string;
  items: readonly string[];
  note?: string | undefined;
}): React.JSX.Element {
  const [all, setAll] = useState(false);
  const held = items.slice(0, CAP);
  const visible = all ? held : held.slice(0, SHOWN);
  const hidden = held.length - visible.length;

  return (
    // Named, so a reader moving by landmark can go straight to the list they want rather than
    // walking the page. Three unnamed <section>s are three anonymous stops.
    <section className="listing" aria-label={heading}>
      <header>
        <h2>{heading}</h2>
        <span className="of">{items.length} found</span>
      </header>
      {note !== undefined && <p className="note">{note}</p>}
      {items.length > CAP && (
        <p className="note">
          Showing {visible.length} of {items.length} — the rest are in the full report.
        </p>
      )}
      {held.length === 0 ? (
        <p className="note">Searched, and found none.</p>
      ) : (
        <ul>
          {visible.map((item) => (
            <li key={item}>
              <span className="name">{item}</span>
            </li>
          ))}
        </ul>
      )}
      {hidden > 0 && (
        <button type="button" className="reveal" onClick={() => setAll(true)}>
          …more ({hidden})
        </button>
      )}
    </section>
  );
}

/**
 * A category with no pipeline behind it.
 *
 * **The heading stays and the list does not.** A missing heading is indistinguishable from a
 * feature that does not exist, and an empty list would be this product claiming it looked —
 * `PRODUCT_IDEA_RESULTS.md` §2.5.
 */
function NotSearched({ heading }: { heading: string }): React.JSX.Element {
  return (
    <section className="listing" aria-label={heading}>
      <header>
        <h2>{heading}</h2>
        <span className="of not-built">not built</span>
      </header>
      <p className="note">
        This search does not exist yet, so nothing here is a finding about your idea.
      </p>
    </section>
  );
}

/**
 * How long discovery takes, from `BENCHMARKS.md` Run 23: **16-35 seconds a company**.
 *
 * The middle of the measured range, and the only number here that is not counted. **Where the
 * estimate stops is not decided here** - the server sends that, because it is the same number
 * as the first counted tick and a constant in two languages is a constant that will disagree
 * with itself.
 */
const DISCOVERY_MS = 25_000;

/**
 * A percentage for the stretch where nothing has counted anything yet.
 *
 * **This is an estimate, and the interface says so.** `Off-The-Napkin-Estimates.md` §1 draws
 * the line this sits on: what the product refuses is *hidden* estimation - a number nobody can
 * tell is a guess. A progress bar is not an assertion about the world, it is an affordance, and
 * a reader who has waited forty seconds is owed something better than a dash.
 *
 * So: interpolate elapsed time across discovery and stop at `ceiling`, which is where counting
 * begins. **The two meet rather than collide** - the first version derived its own cap from a
 * copy of the server's constant, and the first counted tick was `0%`, so the bar fell from its
 * cap back to nothing in front of whoever was watching.
 *
 * @param started when the run began *running* - not when it was queued, which is time nobody
 *   worked
 * @param now the clock, passed in so a test does not have to wait
 * @param ceiling the percentage counting will start from, per the server
 */
export function estimate(started: Date, now: Date, ceiling: number): number {
  const elapsed = Math.max(0, now.getTime() - started.getTime());
  const through = Math.min(1, elapsed / DISCOVERY_MS);
  return Math.floor(through * Math.max(0, ceiling));
}

/**
 * Whether a run is still going, how far through it is, and what it is doing.
 *
 * **The problem this solves is not decoration.** `Reading public web pages…` was shown for the
 * whole of a run that takes four to eight minutes on the target hardware, and a reader
 * therefore had no way to tell *nearly finished* from *barely started* from *dead*.
 *
 * # Three encodings, because one is never enough
 *
 * | | says |
 * |---|---|
 * | the word | `Working` or `Done.` — readable with images off, and what a screen reader reads first |
 * | the bar | moving while live, full and still when finished |
 * | the number | how much is left, when anything knows |
 *
 * A finished run is **not** a bar that quietly stops moving: it is a full bar, a different
 * word, and no percentage — because a still bar and a stalled bar look identical, and the
 * difference is the entire question a reader is asking.
 *
 * # Why the percentage is sometimes absent
 *
 * `landscape_core::progress` refuses to invent a denominator. Until a reading plan exists —
 * which means until the companies are resolved and their pages discovered — nothing knows how
 * much work there is, so there is no number and the bar is indeterminate. That window is the
 * first stretch of every run, and filling it with a fake `7%` would be lying at exactly the
 * moment a reader is deciding whether to trust the thing.
 */
function Waiting({
  status,
  progress,
  generation,
  failure,
  offered,
}: {
  status: AnalysisStatus;
  progress: Progress | null;
  /** Which run this is about. A change means a different worker picked the analysis up. */
  generation: number | null;
  failure: Analysis["failure"];
  offered: number;
}): React.JSX.Element {
  const live = !isTerminal(status);
  const running = status === "running";

  // **A clock, so the estimate moves.** A number that is an estimate and also frozen reads as a
  // hang, which is the thing this whole component exists to rule out. One second is far finer
  // than an eight-minute wait needs and far coarser than anything that costs.
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    if (!running) return undefined;
    const tick = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(tick);
  }, [running]);

  // **When the work started, not when it was accepted.** `created_at` includes time spent in
  // the queue, and a job that waited thirty seconds for a worker would open at the estimate's
  // ceiling having had nothing done to it. The first moment this page saw `running` is the
  // closest thing it has to when a worker picked the job up.
  const startedRunning = useRef<Date | null>(null);
  if (running && startedRunning.current === null) startedRunning.current = new Date();
  if (!running) startedRunning.current = null;
  const estimated = estimate(
    startedRunning.current ?? now,
    now,
    progress?.estimating_to ?? 0,
  );
  // Only while running. A percentage beside `Done.` is a number about something that is over.
  const counted = running ? (progress?.percent ?? null) : null;
  // **A number always, and the reader can tell which kind it is.** Before anything has been
  // counted the bar shows an estimate from measured phase durations rather than a dash - see
  // `estimate`. `counted` wins the moment it exists, and meets the estimate rather than
  // colliding with it, because the server sends the ceiling the estimate stops at.
  const shown = counted ?? (running ? estimated : null);
  // **And a floor, because "never goes backwards" is a promise to the reader, not to a
  // module.** Both sides are monotonic on their own and the seam between them was not - which
  // is how a bar that could not retreat retreated. This makes the guarantee a property of the
  // thing that makes it.
  const ceiling = useRef(0);
  // **A different worker is a different run, and neither ref may cross that line.** A reclaim
  // does not have to pass back through `queued` from here: the stream can drop during the
  // sweep and reconnect with the replacement already `running`, so nothing else would clear
  // them. Left alone, the dead run's percentage floors the replacement's discovery and its
  // elapsed time dates the replacement's estimate.
  //
  // Reset during render rather than by keying the whole component: a key remounts the DOM as
  // well, which is more than is meant here and turned out to be observable in a test.
  const run = useRef(generation);
  if (run.current !== generation) {
    run.current = generation;
    startedRunning.current = running ? new Date() : null;
    ceiling.current = 0;
  }
  if (shown === null) ceiling.current = 0;
  else ceiling.current = Math.max(ceiling.current, shown);
  const percent = shown === null ? null : ceiling.current;
  const known = percent !== null;
  const guessed = counted === null && percent !== null;

  // **The word and the sentence must not say the same thing twice.** `describe` answers
  // *"what happened"*, which is the whole story for a failure and pure duplication for
  // `complete` — where the word already is the story. A running analysis gets the phase
  // instead, because it is strictly more informative than the one sentence `describe` has.
  // `queued` and `complete` are wholly said by their word; `running` is better served by the
  // phase line below, which knows more than `describe` ever can. That leaves `failed`, where
  // `describe` carries the one sentence telling a reader what to do next - which is the case
  // it was written for.
  const said = status === "failed" ? describe(status, failure, offered) : null;

  return (
    <div className="waiting" data-live={live ? "yes" : "no"}>
      <p className="status">
        {/*
          **A word, before anything visual.** `aria-live="polite"` so a reader using a screen
          reader hears it change rather than having to go looking — and `polite` rather than
          `assertive` because this is progress, not an alarm.
        */}
        <span className="status-word" aria-live="polite">
          {WORD_FOR[status]}
        </span>
        {said !== null && <span className="status-said">{said}</span>}
      </p>

      {/* Queued has not started; finished and failed have nothing left to show. */}
      {running && (
        <>
          <div className="bar-row">
            {/*
              **A real `<progress>`, not a styled div.** It is announced as a progress bar,
              it carries the value without an ARIA attribute having to be kept in step with
              the paint, and with no `value` it is indeterminate — which is precisely the
              state this needs for the window where nothing knows the total.
            */}
            <progress
              className="bar"
              {...(known ? { value: percent, max: 100 } : {})}
              aria-label={
                known
                  ? `${guessed ? "about " : ""}${String(percent)}% of this analysis is done`
                  : "Working out how much there is to do"
              }
            />
            <span className="bar-number">
              {known ? `${guessed ? "~" : ""}${String(percent)}%` : "—"}
            </span>
          </div>

          {progress !== null && (
          <p className="doing">
            {progress.saying}
            {/*
              **Only while there is a next planned page to be on.** An empty plan sends
              `0 of 0` and rendered *"page 1 of 0"*; the plan is kept through the search phase,
              where `done === of` rendered *"page N+1 of N"* about pages that are deliberately
              outside it. Both are the same slip: an ordinal computed without asking whether
              the thing it counts exists.
            */}
            {progress.phase === "reading" &&
            progress.pages &&
            progress.pages.of > 0 &&
            progress.pages.done < progress.pages.of
              ? ` — page ${String(progress.pages.done + 1)} of ${String(progress.pages.of)}`
              : ""}
            {progress.companies.of > 1
              ? `, company ${String(Math.min(progress.companies.done + 1, progress.companies.of))} of ${String(progress.companies.of)}`
              : ""}
          </p>
          )}
        </>
      )}
    </div>
  );
}

/**
 * One word per state, and it is the first thing a reader reads.
 *
 * `Done.` rather than `Complete` and `Stopped` rather than `Failed`: the first of each pair is
 * what somebody would say out loud, and the second sounds like a status code.
 *
 * **`Queued.` and `Done.` keep their full stops**, because they are the words this product has
 * always used for those two states — in `USING_THE_SITE.md`, in the walkthrough, and in the
 * tests that assert a finished run says so. A visual change is not a reason to rename the two
 * states a reader has been taught.
 */
const WORD_FOR: Record<AnalysisStatus, string> = {
  queued: "Queued.",
  running: "Working",
  complete: "Done.",
  failed: "Stopped",
};

function describe(
  status: AnalysisStatus,
  failure: Analysis["failure"],
  /** How many companies are on screen to pick between. */
  offered: number,
): string {
  switch (status) {
    case "queued":
      return "Queued.";
    case "running":
      return "Reading public web pages…";
    case "complete":
      return "Done.";
    case "failed":
      // **One sentence per situation, because each asks for something different.** These were
      // two: `no_subject` and everything else — so a search that timed out told a reader to
      // name a website, which fixes something that was never wrong, and a name several
      // products share threw away the question they could have answered in a word.
      switch (failure) {
        case "no_subject":
          return "We could not work out which company you meant. Try naming its website — for example, basecamp.com.";
        case "no_engine":
          // **Not "try naming its website".** They typed an idea, which is the input this
          // product is for, and naming the companies is the research they came here to have
          // done. Nothing they can type fixes this, so they are not asked to type anything.
          return "No search engine is configured, so nothing was looked for. Your idea is fine — this is ours to fix, by setting SEARX_URL.";
        case "ambiguous":
          // **Two sentences, because the reader's next move is different.** With chips under
          // it, "name the one you mean" asks somebody to type what is already a button — the
          // instruction and the affordance would be telling them to do the work twice.
          return offered > 0
            ? "That name matches more than one company, and we will not guess between them. Pick the one you meant:"
            : "That name matches more than one company, and we will not guess between them. Name the one you mean — a website works.";
        case "nothing_found":
          return "We searched and found no company we could stand behind. Try naming a website, or describing the product in the words a vendor would use.";
        case "search_incomplete":
          // The one a reader fixes by doing nothing, so it is the one that must not send them
          // off to change their prompt.
          return "The search did not finish, so we have not concluded anything. This is usually temporary — try again.";
        case "search_refused":
          // **Identical counts, opposite advice.** The engine answered and said no, and it
          // will say no again — a misconfigured one refuses every query until somebody edits
          // a file. Telling this reader to try again is telling them to wait for something
          // that cannot happen, so they get the one route that skips the engine instead.
          return "Our search engine is refusing us, so we have not concluded anything. That is ours to fix and trying again will not help — naming a website skips the search entirely.";
        default:
          return "This one did not finish. Nothing you did caused it.";
      }
  }
}
