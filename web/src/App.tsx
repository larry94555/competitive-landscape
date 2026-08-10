import { useCallback, useEffect, useRef, useState } from "react";
import {
  analysisInPath,
  ApiError,
  createAnalysis,
  getAnalysis,
  getExamples,
  isTerminal,
  pathFor,
  watchAnalysis,
  type Analysis,
  type AnalysisStatus,
  type Examples,
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
    let cancelled = false;
    void getExamples().then((found) => {
      if (!cancelled) setExamples(found);
    });
    return () => {
      cancelled = true;
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
      // their words invites them to press Analyse again and be refused.
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

  const { status, sections, subjects } = useReport(analysis, setAnalysis);

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
          {submitting ? "Starting…" : "Analyse"}
        </button>
      </form>

      {/*
        Ideas to start from, below the box rather than above it. A reader who came with their
        own idea should meet the box first; one who came to look around finds these without
        having to invent a prompt — and the prompts people invent are the ones this pipeline
        cannot resolve, so an empty box is where a demo dies.

        Clicking one **fills the box and does not submit.** The reader sees the sentence,
        including the companies, and can change it before anything is fetched. That is the
        whole of what "the curation is visible" means here.
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
                  <span className="companies">{example.companies.join(" vs ")}</span>
                  <span className="why">{example.why}</span>
                </button>
              </li>
            ))}
          </ul>
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
} {
  const [status, setStatus] = useState<AnalysisStatus | null>(null);
  const [sections, setSections] = useState<readonly Section[]>([]);
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
  // and written inside callbacks, and because changing it is never itself a reason to render.
  const generationRef = useRef<number | null>(null);

  const id = analysis?.id ?? null;
  const settled = analysis != null && isTerminal(analysis.status);

  // A new analysis starts from nothing. Kept apart from the connection below so that
  // reconnecting does not wipe the sections the reader is already looking at.
  useEffect(() => {
    setStatus(null);
    setSections([]);
    setSubjects([]);
    setAttempt(0);
    generationRef.current = null;
  }, [id]);

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
    if (held === null || held === generation) return;
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

    let cancelled = false;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const reconnect = (): void => {
      if (cancelled) return;
      retry = setTimeout(() => {
        if (!cancelled) setAttempt((n) => n + 1);
      }, RECONNECT_MS);
    };

    const close = watchAnalysis(id, {
      onStatus: (next) => {
        if (!cancelled) setStatus(next);
      },
      onSection: (section) => {
        if (cancelled) return;
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
        if (!cancelled) sawGeneration(generation);
      },
      onSubjects: (next) => {
        // Not cleared on a new generation: the replacement run is comparing the same
        // companies, and a screen that stops labelling halfway through a restart is the
        // defect this exists to prevent, wearing a different hat.
        if (!cancelled) setSubjects(next);
      },
      onDone: () => {
        if (cancelled) return;
        void getAnalysis(id)
          .then((latest) => {
            if (cancelled) return;
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
      cancelled = true;
      if (retry !== undefined) clearTimeout(retry);
      close();
    };
  }, [id, settled, attempt]);

  return { status, sections, subjects };
}

function AnalysisView({
  analysis,
  status,
  sections,
  subjects,
  onPick,
  picking,
}: {
  analysis: Analysis;
  status: AnalysisStatus | null;
  sections: readonly Section[];
  subjects: readonly string[];
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
  // **From the companies analysed, not from the ones that produced a claim.** Deriving it from
  // the claims on screen looks reasonable and is wrong in the case that matters: ask about two
  // companies, have one of them say nothing, and the survivor's prices lose their label in a
  // report that is still a comparison.
  //
  // Two sources because there are two moments. The finished report carries `subjects`; while it
  // runs there is no report yet, so the stream says so directly. Leaving the live case to be
  // guessed from the claims is the same defect one surface later — it was, and review found it
  // there too.
  const analysed = (report?.subjects?.length ?? 0) > 0 ? (report?.subjects ?? []) : subjects;
  // **Read from the failure, not only from the list.** A run that later succeeded can still be
  // holding the question an earlier attempt asked; offering it under a finished report would
  // invite a reader to re-run something they can already read. The server clears it for the
  // same reason — `analysis_from_row` — and neither side relies on the other having done so.
  const choices =
    analysis.status === "failed" && analysis.failure === "ambiguous"
      ? analysis.choices
      : [];
  const several =
    analysed.length > 1 ||
    // Only for reports stored before `subjects` existed, which deserialise without it.
    new Set(
      showing
        .flatMap((s) => s.claims)
        .map((c) => c.subject)
        .filter((s) => s !== ""),
    ).size > 1;

  return (
    <section aria-live="polite">
      <h2>{analysis.prompt}</h2>

      {/*
        Anything true of the whole report rather than one section — today, which companies were
        named and not analysed. It sits above the sections because it changes what the sections
        below it mean.
      */}
      {report?.notes?.map((note) => (
        <p key={note} className="report-note">
          {note}
        </p>
      ))}

      {/*
        **What the market calls this, above the results and below what the reader typed.**
        `COMPETITIVE_DISCOVERY.md` §4: the substitution decides every query underneath it, so a
        wrong reading has to be visible before anything below it is believed. It is absent when
        nothing was substituted — repeating somebody's own words back at them discloses nothing.
      */}
      {report?.interpreted && (
        <p className="interpreted">
          Interpreted as <strong>{report.interpreted.label}</strong>
          {report.interpreted.also.length > 0 && (
            <span className="also"> (also: {report.interpreted.also.join(", ")})</span>
          )}
          <span className="backing">
            {" "}
            — {report.interpreted.hosts} independent{" "}
            {report.interpreted.hosts === 1 ? "site" : "sites"} use this name
          </span>
        </p>
      )}

      {report && report.searched_as !== "" && (
        <p className="searched-as">
          Searched as <strong>{report.searched_as}</strong>
        </p>
      )}

      {/*
        **The set, correctable.** `COMPETITIVE_DISCOVERY.md` §5.5 and §6.3: a competitive set
        presented without a way to correct it is an unfalsifiable editorial choice, and direct
        manipulation beats interrogation — one glance confirms or corrects what was decided,
        with no prompt literacy and no question spent.

        Under the notes that say *why* each company is here, because a reader corrects the set
        having read the reasons rather than before.
      */}
      {isTerminal(showing_status) && analysed.length > 0 && (
        <EditableSet companies={analysed} onRun={onPick} running={picking} />
      )}

      <p className="status">
        {describe(showing_status, analysis.failure, choices.length)}
      </p>

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
          // The only one a reader fixes by doing nothing, so it is the only one that must not
          // send them off to change their prompt.
          return "The search did not finish, so we have not concluded anything. This is usually temporary — try again.";
        default:
          return "This one did not finish. Nothing you did caused it.";
      }
  }
}
