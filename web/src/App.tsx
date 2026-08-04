import { useCallback, useEffect, useRef, useState } from "react";
import {
  ApiError,
  createAnalysis,
  getAnalysis,
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

  const submit = useCallback(async () => {
    setError(null);
    setSubmitting(true);
    try {
      setAnalysis(await createAnalysis(prompt));
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
 */
function useReport(
  analysis: Analysis | null,
  onFinished: (a: Analysis) => void,
): { status: AnalysisStatus | null; sections: readonly Section[] } {
  const [status, setStatus] = useState<AnalysisStatus | null>(null);
  const [sections, setSections] = useState<readonly Section[]>([]);
  const onFinishedRef = useRef(onFinished);
  onFinishedRef.current = onFinished;

  const id = analysis?.id ?? null;
  const settled = analysis?.report != null || analysis?.failure != null;

  useEffect(() => {
    if (id === null || settled) return;

    setStatus(null);
    setSections([]);
    let cancelled = false;

    const close = watchAnalysis(id, {
      onStatus: (next) => {
        if (!cancelled) setStatus(next);
      },
      onSection: (section) => {
        if (cancelled) return;
        // Arrival order, and the newest version of each. A section is sent again as it
        // grows: pinning the first copy left "What it does: 1 item" on screen for two
        // minutes while eight more were read, which reads as a section that is finished.
        setSections((current) => {
          const at = current.findIndex((s) => s.key === section.key);
          if (at === -1) return [...current, section];
          const next = [...current];
          next[at] = section;
          return next;
        });
      },
      onDone: () => {
        if (cancelled) return;
        void getAnalysis(id)
          .then((finished) => {
            if (!cancelled) onFinishedRef.current(finished);
          })
          .catch(() => {
            // The reader keeps what the stream already gave them. A red banner over a
            // report they can see would be worse than a report that stops updating.
          });
      },
    });

    return () => {
      cancelled = true;
      close();
    };
  }, [id, settled]);

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
  // The finished report once it exists, and what the stream has delivered until then. Both
  // are the same shape, which is why the rendering below does not know which it has.
  const showing = report?.sections ?? sections;
  const live = report == null && analysis.failure == null;

  return (
    <section aria-live="polite">
      <h2>{analysis.prompt}</h2>

      {report && report.searched_as !== "" && (
        <p className="searched-as">
          Searched as <strong>{report.searched_as}</strong>
        </p>
      )}

      <p className="status">{describe(status ?? analysis.status, analysis.failure)}</p>

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
