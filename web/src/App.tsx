import { useCallback, useEffect, useRef, useState } from "react";
import {
  analysisInPath,
  ApiError,
  createAnalysis,
  getAnalysis,
  isTerminal,
  pathFor,
  watchAnalysis,
  type Analysis,
  type AnalysisStatus,
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

  const submit = useCallback(async () => {
    setError(null);
    setSubmitting(true);
    try {
      const started = await createAnalysis(prompt);
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
  }, [prompt]);

  const { status, sections } = useReport(analysis, setAnalysis);

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
        <AnalysisView analysis={analysis} status={status} sections={sections} />
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
): { status: AnalysisStatus | null; sections: readonly Section[] } {
  const [status, setStatus] = useState<AnalysisStatus | null>(null);
  const [sections, setSections] = useState<readonly Section[]>([]);
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

  return { status, sections };
}

function AnalysisView({
  analysis,
  status,
  sections,
}: {
  analysis: Analysis;
  status: AnalysisStatus | null;
  sections: readonly Section[];
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
  // say whose it is. Derived from what is on screen rather than from the prompt, so a report
  // that named three companies and found facts for one does not label that one needlessly.
  const several =
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

      {report && report.searched_as !== "" && (
        <p className="searched-as">
          Searched as <strong>{report.searched_as}</strong>
        </p>
      )}

      <p className="status">{describe(showing_status, analysis.failure)}</p>

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
                    <strong className="subject">{withoutScheme(claim.subject)}</strong>
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
): string {
  switch (status) {
    case "queued":
      return "Queued.";
    case "running":
      return "Reading public web pages…";
    case "complete":
      return "Done.";
    case "failed":
      // Two different failures, and telling somebody "nothing you did caused it" when they
      // typed a description we could not resolve sends them away with no way forward.
      return failure === "no_subject"
        ? "We could not work out which company you meant. Try naming its website — for example, basecamp.com."
        : "This one did not finish. Nothing you did caused it.";
  }
}
