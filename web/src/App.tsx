import { useCallback, useEffect, useRef, useState } from "react";
import {
  ApiError,
  createAnalysis,
  getAnalysis,
  isTerminal,
  type Analysis,
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

  const id = analysis?.id ?? null;
  const status = analysis?.status ?? null;
  const settled = status !== null && isTerminal(status);
  usePoll(id, settled, setAnalysis);

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

      {analysis && <AnalysisView analysis={analysis} />}
    </main>
  );
}

/**
 * Poll until the analysis reaches an end state.
 *
 * Polling rather than SSE while the run takes seconds. It becomes a stream when runs take
 * minutes and a reader is watching sources arrive — that is a Phase 1 change, and the
 * shape of this hook does not need to change for it.
 */
function usePoll(
  id: string | null,
  settled: boolean,
  onUpdate: (a: Analysis) => void,
): void {
  const onUpdateRef = useRef(onUpdate);
  onUpdateRef.current = onUpdate;

  useEffect(() => {
    if (id === null || settled) return;

    let cancelled = false;
    const timer = setInterval(() => {
      void getAnalysis(id)
        .then((next) => {
          // A response that arrives after unmount must not set state.
          if (!cancelled) onUpdateRef.current(next);
        })
        .catch(() => {
          // A single failed poll is not worth showing: the next one usually succeeds,
          // and a flickering error is worse than a slightly stale view.
        });
    }, 1000);

    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [id, settled]);
}

function AnalysisView({ analysis }: { analysis: Analysis }): React.JSX.Element {
  const { report } = analysis;

  return (
    <section aria-live="polite">
      <h2>{analysis.prompt}</h2>

      {report && report.searched_as !== "" && (
        <p className="searched-as">
          Searched as <strong>{report.searched_as}</strong>
        </p>
      )}

      <p className="status">{describe(analysis.status)}</p>

      {report?.sections.map((section) => (
        <article key={section.key}>
          <h3>{section.title}</h3>
          {section.status === "not_found_in_public_sources" ? (
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
                </li>
              ))}
            </ul>
          )}
        </article>
      ))}
    </section>
  );
}

function describe(status: Analysis["status"]): string {
  switch (status) {
    case "queued":
      return "Queued.";
    case "running":
      return "Reading public web pages…";
    case "complete":
      return "Done.";
    case "failed":
      return "This one did not finish. Nothing you did caused it.";
  }
}
