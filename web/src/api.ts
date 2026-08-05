/**
 * The API client.
 *
 * These types are hand-written for now and must match `landscape-core`. They are
 * generated from that crate's JSON Schema once the schema settles — a duplicated shape is
 * a known cost, recorded here rather than forgotten.
 */

export type AnalysisStatus = "queued" | "running" | "complete" | "failed";

/**
 * Which situation a failed analysis is in.
 *
 * A closed set, on purpose. The server records an operator's reason too and never sends it:
 * what somebody is told about a failure is a decision made here, in words written for them.
 */
export type Failure = "no_subject" | "internal";

export type SectionStatus = "populated" | "partial" | "not_found_in_public_sources";

export interface Claim {
  readonly text: string;
  readonly source_label: string;
  readonly evidence_quote: string;
  readonly confidence: "high" | "medium" | "low";
  readonly as_of: string;
}

export interface Section {
  readonly key: string;
  readonly title: string;
  readonly status: SectionStatus;
  readonly claims: readonly Claim[];
  /** What was checked when nothing was found. Rendered, never hidden. */
  readonly checked: readonly string[];
  readonly notes: readonly string[];
}

export interface Report {
  readonly subject: string;
  /** What was actually searched for. Shown above the results so a wrong reading is visible. */
  readonly searched_as: string;
  readonly generated_at: string;
  readonly model_id: string;
  readonly prompt_version: number;
  readonly sections: readonly Section[];
  readonly sources: readonly unknown[];
}

export interface Analysis {
  readonly id: string;
  readonly prompt: string;
  readonly status: AnalysisStatus;
  readonly created_at: string;
  readonly report: Report | null;
  readonly failure: Failure | null;
}

/** What the server sends when it refuses a request. */
export interface ApiErrorBody {
  readonly error: string;
  readonly remedy?: string;
  /**
   * Present on internal failures only, and the same value as the `x-request-id` header.
   * Every log line the failed request wrote carries it.
   */
  readonly reference?: string;
}

/** An error carrying a message already written for a person to read. */
export class ApiError extends Error {
  readonly remedy: string | undefined;
  /**
   * The server's reference for this failure.
   *
   * Shown to the reader rather than only logged. Someone who can quote a reference turns
   * an unanswerable report — "it broke this afternoon" — into one line of a log search,
   * and they will only quote it if we put it in front of them.
   */
  readonly reference: string | undefined;

  constructor(message: string, remedy?: string, reference?: string) {
    super(message);
    this.name = "ApiError";
    this.remedy = remedy;
    this.reference = reference;
  }
}

async function readError(response: Response): Promise<never> {
  let body: Partial<ApiErrorBody> = {};
  try {
    body = (await response.json()) as Partial<ApiErrorBody>;
  } catch {
    // A non-JSON error body means something upstream failed, not the API. Fall through
    // to the generic message rather than showing the reader a parse error.
  }
  throw new ApiError(
    body.error ?? "The server could not be reached.",
    body.remedy ?? "Check that the API is running, then try again.",
    body.reference,
  );
}

export async function createAnalysis(prompt: string): Promise<Analysis> {
  const response = await fetch("/api/analyses", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ prompt }),
  });
  if (!response.ok) return readError(response);
  return (await response.json()) as Analysis;
}

export async function getAnalysis(id: string): Promise<Analysis> {
  const response = await fetch(`/api/analyses/${id}`);
  if (!response.ok) return readError(response);
  return (await response.json()) as Analysis;
}

export function isTerminal(status: AnalysisStatus): boolean {
  return status === "complete" || status === "failed";
}

/** What the reader sees while a report is being written. */
export interface Watcher {
  readonly onStatus: (status: AnalysisStatus) => void;
  /** One section, the first time it has anything in it. */
  readonly onSection: (section: Section) => void;
  /**
   * Everything sent so far is withdrawn; the run is starting again.
   *
   * A worker died and its analysis went back to the queue. The sections it produced are not
   * wrong so much as unowned — nobody stands behind them, and the replacement run rebuilds
   * from nothing. The opposite of `onDone`: keep watching, and forget what you have.
   */
  readonly onReset: () => void;
  /** Nothing else is coming. The caller fetches the finished analysis. */
  readonly onDone: () => void;
}

/**
 * Watch a report being written.
 *
 * `GET /api/analyses/{id}/events` sends a section as soon as it has claims, so a reader sees
 * the first one in twenty to forty seconds rather than at the end of ninety —
 * `PRODUCT_SPEC.md` §2.1A. Returns a function that closes the stream.
 *
 * **The stream is an accelerator, not the source of truth.** If it fails — a proxy that
 * buffers, a laptop that slept, a browser without `EventSource` — the caller still has
 * `getAnalysis`, and the reader sees the report a little later rather than not at all. That
 * is why `onDone` fires on error too.
 */
export function watchAnalysis(id: string, watcher: Watcher): () => void {
  if (typeof EventSource === "undefined") {
    watcher.onDone();
    return () => {};
  }

  const source = new EventSource(`/api/analyses/${id}/events`);
  let closed = false;
  const close = (): void => {
    if (closed) return;
    closed = true;
    source.close();
  };

  source.addEventListener("status", (e) => {
    watcher.onStatus((e as MessageEvent<string>).data as AnalysisStatus);
  });
  source.addEventListener("section", (e) => {
    try {
      watcher.onSection(JSON.parse((e as MessageEvent<string>).data) as Section);
    } catch {
      // A section we cannot parse is a bug on our side. Dropping it costs the reader one
      // heading arriving late, which the final fetch puts right.
    }
  });
  source.addEventListener("reset", () => {
    watcher.onReset();
  });
  source.addEventListener("done", () => {
    close();
    watcher.onDone();
  });
  source.onerror = () => {
    // EventSource retries by itself, and retrying against a finished analysis is a
    // connection that opens, says `done`, and closes — which is churn nobody benefits
    // from. Close it and let the caller fetch.
    close();
    watcher.onDone();
  };

  return close;
}
