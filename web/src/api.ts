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
  // **The one that took a reader to find.** `no_subject` covered this too, so somebody who
  // typed a product idea — the input this product is for — was told we could not work out
  // which company they meant. Nothing was wrong with their words; nothing is configured here.
  | "no_engine"
  | "ambiguous"
  | "too_general"
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
  /**
   * What class of thing the reader gave.
   *
   * **The page cannot say "I found" without it.** A named set is handed straight through with
   * no discovery, so calling those companies *found* takes credit for reading a list — and
   * their order is the reader's instruction rather than our ranking.
   * `PRODUCT_IDEA_RESULTS.md` §2.3.
   */
  readonly asked?: Given | null;
  /** How much of the searching finished. Absent when nothing was searched for. */
  readonly searches?: Searches | null;
  /**
   * Which companies were compared, which were not, and why each.
   *
   * **The last thing this product asserted without showing its evidence.** Every claim inside a
   * report is quoted, dated and cited; the choice of company was computed correctly and shown
   * to nobody. Absent when the reader named their own companies — a page arguing that decision
   * back at them is answering a question they did not ask.
   */
  readonly chosen?: Chosen | null;
}

/** One company, and the sentence saying why it is where it is. */
export interface Reason {
  readonly domain: string;
  readonly name: string;
  readonly why: string;
}

/**
 * Who is in the comparison, who is not, and why each.
 *
 * **Both halves or neither.** Reasons for the companies that got in, beside a silence about
 * everybody else, is the more flattering half of the same evidence.
 */
export interface Chosen {
  /**
   * In the comparison **and with an argument to make**.
   *
   * A company the reader named is not here: *"you named it"* is a fact rather than a case, and
   * a page putting it under somebody's own company argues a decision back at the person who
   * made it.
   */
  readonly argued: readonly Reason[];
  readonly left_out: readonly Reason[];
  /**
   * The day this was decided, `YYYY-MM-DD`.
   *
   * **Decided, not read.** Two of the five reasons a company is left out are *we could not
   * read its page* and *we never asked for it*, so a line claiming these pages were read would
   * sit directly above a sentence saying one of them was not.
   */
  readonly decided_on: string;
}

/** What the reader gave. Mirrors `landscape_core::Given`. */
export type Given =
  | { readonly kind: "described" }
  | { readonly kind: "seeded"; readonly named: string }
  | { readonly kind: "named"; readonly count: number };

/** How many of the searches behind the company set answered, and how many did not. */
export interface Searches {
  readonly answered: number;
  readonly failed: number;
}

/**
 * How many searches were sent, answered or not.
 *
 * **Mirrors `Searches::sent` in the core**, and exists for the same reason `searchFinished`
 * does: the two halves of a coverage are stored, and every question a reader is shown about it
 * is derived, in one place on each side of the wire rather than at each call.
 */
export function searchesSent(searches: Searches): number {
  return searches.answered + searches.failed;
}

/**
 * Whether a count taken from these searches may be stated as a definite number.
 *
 * **Nothing sent is not the same as everything answered.** A run that asked no questions has
 * established no absence, which is why this is not `failed === 0`. Mirrors
 * `Searches::finished` in the core, and the two are asserted against the same cases.
 */
export function searchFinished(searches: Searches | null | undefined): boolean {
  return searches != null && searches.failed === 0 && searches.answered > 0;
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
   * Empty unless `failure` is `"ambiguous"` or `"too_general"` — the two refusals a reader
   * answers in one click. **This is the question**, not decoration on the
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

  /**
   * What goes in the box: the idea, and nothing else.
   *
   * Built by the server rather than assembled here. What an example *is* has changed twice —
   * it used to carry the companies to compare — and both times every caller had to agree
   * about it, including the parser that reads the result back on the other side.
   */
  readonly prompt: string;
}

/** The examples, with the sentence that says what is curated about them. */
export interface Examples {
  /**
   * Whether an idea can be researched at all.
   *
   * **False means every example below will refuse.** Each is a description, and resolving one
   * into companies needs a search engine; without `SEARX_URL` the run stops and the reader has
   * spent an analysis to be told about an environment variable. A reader found that out the
   * hard way, so the first screen says it first.
   */
  readonly discovery: boolean;
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
    // validation and is not: one entry with a null field still reached the render and threw
    // while the first screen was drawing itself. Review found it. A shape checked at the edge
    // and trusted at the leaf is unchecked.
    const usable = (body as Examples).examples.filter(isExample);
    return {
      // **Absent reads as unavailable, not as available.** A server too old to send this is
      // one that predates the field, and guessing "yes" would put the reader back where they
      // started: an idea typed, a run spent, an environment variable explained afterwards.
      discovery: (body as Examples).discovery === true,
      note: (body as Examples).note,
      examples: usable,
    };
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
    typeof example.prompt === "string"
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
   * **Arrives only once the worker has persisted a phase, and not before.** The server used to
   * synthesize one for a running analysis with no report — which then jumped backwards to
   * whatever the run turned out to be doing, because a status of *running* does not say which
   * phase and nothing on that side can make it.
   *
   * So there is a window at the start of every run where this never fires. **What fills it is
   * the status and the elapsed clock**, neither of which claims to know what the run is doing.
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
