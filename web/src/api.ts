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
 * **A value earns a place here when a reader would do something different.** It was
 * `no_subject | internal`, and five different endings arrived as `no_subject`: no engine
 * configured, several companies sharing a name, a search that did not finish, and a market we
 * looked at and found empty. All four rendered as *"try naming its website"* — which is wrong
 * for a search that timed out, and throws away a question a reader could answer in one word.
 *
 * The server's `failure_reason` is deliberately not here: it is for operators and never shown
 * verbatim, so the sentence for each of these is written in `statusLine` below.
 */
export type Failure =
  | "no_subject"
  | "ambiguous"
  | "nothing_found"
  | "search_incomplete"
  | "search_refused"
  | "internal";

/**
 * One company a reader can pick when a name matched several.
 *
 * **The chip carries the whole prompt, not the name.** A reader who has to retype their idea
 * with a company bolted onto it has been asked to do the work the question was supposed to
 * save them — and the words that join an idea to a company are a server decision, for the
 * same reason [`Example.prompt`] is: the parser that reads them back lives on that side.
 */
export interface Choice {
  readonly name: string;
  /** Its canonical domain. Shown, because it is what tells two same-named products apart. */
  readonly domain: string;
  /** The one line under the company's own heading. Empty when its page said nothing. */
  readonly what_it_is: string;
  /** What to run instead. Sent verbatim. */
  readonly prompt: string;
}

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

/**
 * What the market calls this, when the queries turned out to use different words.
 *
 * `COMPETITIVE_DISCOVERY.md` §4's *"Interpreted as …"* line. A reader typed one thing and the
 * searches that found these companies said another; that substitution decides everything below
 * it, so it is shown rather than assumed. **Absent when nothing was substituted** — a line
 * saying *"interpreted as ‹what you typed›"* is noise, not disclosure.
 */
export interface Interpreted {
  readonly label: string;
  /** Other phrasings that recurred. Shown, and never searched. */
  readonly also: readonly string[];
  /** Independent sites whose titles used the label. The whole of the evidence for it. */
  readonly hosts: number;
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
  /** What the market calls this. Absent when the reader's own words were searched for. */
  readonly interpreted?: Interpreted | null;
  /** Anything true of the whole report — today, companies named and not analyzed. */
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
   * The companies a reader can pick between, when the run stopped rather than guess.
   *
   * Empty unless `failure` is `"ambiguous"`. **This is the question**, not decoration on the
   * refusal: without it the page can say a name matched several companies and cannot say
   * which, which leaves the reader guessing at exactly what we declined to guess at.
   *
   * Always sent, empty rather than absent, so the page never has to tell "nothing to pick
   * between" apart from "a server that does not know about picking". A row stored before the
   * column existed reads back as an empty list on the server, not as a missing field.
   */
  readonly choices: readonly Choice[];
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
    // **Every field of every entry, not the container.** The first version of this checked
    // that `examples` was an array and stopped there, which is a guard that reads as
    // validation and is not: one entry with a null `companies` still reached
    // `companies.join(" vs ")` and threw while the first screen was drawing itself. Review
    // found it. A shape checked at the edge and trusted at the leaf is unchecked.
    const usable = (body as Examples).examples.filter(isExample);
    return { note: (body as Examples).note, examples: usable };
  } catch {
    return null;
  }
}

/**
 * Whether one entry is safe to render.
 *
 * Malformed entries are **dropped rather than fatal**: a server sending four good ideas and
 * one bad one should cost a reader the bad one, not the screen. Dropping them all would make
 * one broken row look exactly like an outage.
 */
function isExample(value: unknown): value is Example {
  if (typeof value !== "object" || value === null) return false;
  const example = value as Record<string, unknown>;
  return (
    typeof example.id === "string" &&
    typeof example.idea === "string" &&
    typeof example.why === "string" &&
    typeof example.prompt === "string" &&
    Array.isArray(example.companies) &&
    example.companies.every((company) => typeof company === "string")
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

/**
 * The whole report as Markdown, for a reader to paste into their own assistant.
 *
 * **Fetched rather than built here.** The bytes come from `landscape_core::context`, which is
 * also what `curl` gets — so the page, the terminal and whatever a reader pastes are one
 * document. Rendering it in TypeScript would mean a second set of decisions about what a
 * source's standing is called, which is a rule that agrees today.
 */
export async function getContext(id: string): Promise<string> {
  const response = await fetch(`/api/analyses/${id}/context`);
  if (!response.ok) return readError(response);
  return await response.text();
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

/** A count of finished things out of a total that something already knew. */
export interface Counted {
  readonly done: number;
  readonly of: number;
}

/** What a run is doing, and how far through it is. */
export interface Progress {
  readonly phase: "discovering" | "reading" | "searching" | "assembling";
  /** The sentence to show. Written by the server so one place decides the wording. */
  readonly saying: string;
  /**
   * How much of the run is done, or `null` when nothing knows yet.
   *
   * **`null` is not zero.** Before the reading plan exists, no part of this system can say how
   * many pages there will be — so the page shows an indeterminate bar and the phase, rather
   * than a number nobody computed. Treating `null` as `0` here would put the lie back.
   */
  readonly percent: number | null;
  /**
   * The percentage the page may estimate its way up to while discovery runs, and no further.
   *
   * **Computed by the server, because the server owns the arithmetic.** It is exactly where
   * `percent` will land on the first counted tick, so an estimate that stops here and a count
   * that starts here meet rather than collide. `null` once counting has begun.
   */
  readonly estimating_to: number | null;
  readonly companies: Counted;
  /** `null` until a reading plan exists. See `percent`. */
  readonly pages: Counted | null;
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
  /**
   * What the run is doing, and how far through it is.
   *
   * Arrives for a running analysis that has written no report at all, which no other event
   * here does — that stretch is the longest one with nothing on screen, and it is the one a
   * reader is most likely to read as a hang.
   */
  readonly onProgress: (progress: Progress) => void;
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
      // whether every claim on screen is labeled, and the finished report says so anyway.
      if (Array.isArray(parsed) && parsed.every((s) => typeof s === "string")) {
        watcher.onSubjects(parsed);
      }
    } catch {
      // See above: the label is left to the final fetch rather than put on a guess.
    }
  });
  source.addEventListener("progress", (e) => {
    try {
      const parsed = JSON.parse((e as MessageEvent<string>).data) as Progress;
      // A percentage that is not a number is dropped rather than coerced: `Number(null)` is
      // `0`, and a bar that reports zero percent for "we do not know yet" is the one thing
      // this feature is written not to do.
      const number = (value: unknown): number | null =>
        typeof value === "number" && Number.isFinite(value) ? value : null;
      const percent = number(parsed.percent);
      const estimating_to = number(parsed.estimating_to);
      if (typeof parsed.saying === "string") {
        watcher.onProgress({ ...parsed, percent, estimating_to });
      }
    } catch {
      // The bar keeps whatever it last had. A missed tick costs a reader nothing; a guessed
      // one costs them the reason to trust the number.
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
