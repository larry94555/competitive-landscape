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
  /**
   * Which company this is about, as the origin it was read from.
   *
   * A claim's text says what the page says — *"Pro costs $15"* — and never names the company.
   * Once one section holds several companies, that is not enough to tell whose price is whose.
   */
  readonly subject: string;
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
  /** Every company the run set out to cover — not only the ones that produced a claim. */
  readonly subjects?: readonly string[];
  readonly sections: readonly Section[];
  readonly sources: readonly unknown[];
  /** Anything true of the whole report — today, companies named and not analysed. */
  readonly notes?: readonly string[];
}

export interface Analysis {
  readonly id: string;
  readonly prompt: string;
  readonly status: AnalysisStatus;
  readonly created_at: string;
  readonly report: Report | null;
  readonly failure: Failure | null;
  /**
   * How many times this run has been started.
   *
   * A worker can die and the queue hand its row to another, which starts over from nothing.
   * When this changes, the sections already on screen belong to a run that no longer exists.
   */
  readonly generation: number;
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

/** One idea the first screen offers, and the text clicking it puts in the box. */
export interface Example {
  readonly id: string;
  readonly idea: string;
  readonly companies: readonly string[];
  readonly why: string;
  /**
   * What goes in the box.
   *
   * Built by the server, and it already contains the companies. The browser must not
   * assemble this: which words join an idea to its domains is one decision, and the parser
   * that reads them back out lives on the other side of it.
   */
  readonly prompt: string;
}

/** The examples, with the sentence that says what is curated about them. */
export interface Examples {
  readonly note: string;
  readonly examples: readonly Example[];
}

/**
 * The ideas to offer on the first screen.
 *
 * **Failure is silent on purpose.** These are a way in, not the product: a reader who can
 * type still has everything, and an error banner over an empty box would make a missing
 * convenience look like a broken application.
 */
export async function getExamples(): Promise<Examples | null> {
  try {
    const response = await fetch("/api/examples");
    if (!response.ok) return null;
    const body: unknown = await response.json();
    // Checked rather than asserted. Everything else here is reached through a `!response.ok`
    // guard that makes the shape the server's promise; this one is rendered on the *first*
    // screen, so a body that is not what we expect must degrade to "no chips" and not to an
    // exception thrown while the page is drawing itself.
    if (
      typeof body !== "object" ||
      body === null ||
      !Array.isArray((body as Examples).examples) ||
      typeof (body as Examples).note !== "string"
    ) {
      return null;
    }
    return body as Examples;
  } catch {
    return null;
  }
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

/**
 * The analysis a URL is pointing at, if it is pointing at one.
 *
 * **A run with no URL cannot be shared, reopened or refreshed**, which is the single thing
 * that made every demo of this die on a reload. `/a/{id}` is the whole of the routing; the
 * server returns the page for any path it does not claim, so the client is what decides.
 *
 * Deliberately loose about what an id looks like. A malformed one produces a 404 from the API
 * and the same "we could not find it" the reader would get for a deleted one, which is the
 * honest answer to both and one code path instead of two.
 */
export function analysisInPath(pathname: string): string | null {
  const found = /^\/a\/([^/]+)\/?$/.exec(pathname);
  return found?.[1] ?? null;
}

/** Where one analysis lives. */
export function pathFor(id: string): string {
  return `/a/${id}`;
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
   * Which run the sections that follow belong to.
   *
   * A worker died and its analysis went back to the queue, so a different one is starting it
   * over from nothing. The sections it produced are not wrong so much as unowned. The caller
   * compares against the number it is holding, because only the caller knows what that is —
   * a reconnected stream has no memory of the one it replaced.
   */
  readonly onGeneration: (generation: number) => void;
  /**
   * Which companies this run set out to cover.
   *
   * Not which ones produced a claim — the difference is a company that says nothing, and the
   * survivor's prices losing their label in a report that is still a comparison. The stream
   * carries it because the report that holds `subjects` is not fetched until the run is over,
   * and the first claim arrives long before that.
   */
  readonly onSubjects: (subjects: readonly string[]) => void;
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
  source.addEventListener("generation", (e) => {
    const n = Number((e as MessageEvent<string>).data);
    // A generation we cannot read is worse than one we ignore: clearing on `NaN` would wipe
    // the reader's screen every poll.
    if (Number.isInteger(n)) watcher.onGeneration(n);
  });
  source.addEventListener("subjects", (e) => {
    try {
      const parsed: unknown = JSON.parse((e as MessageEvent<string>).data);
      // Anything else is dropped rather than guessed at. A malformed list would decide
      // whether every claim on screen is labelled, and the finished report says so anyway.
      if (Array.isArray(parsed) && parsed.every((s) => typeof s === "string")) {
        watcher.onSubjects(parsed);
      }
    } catch {
      // See above: the label is left to the final fetch rather than put on a guess.
    }
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
