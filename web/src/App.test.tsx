import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import App, { estimate } from "./App";
import type { Analysis, Failure } from "./api";

const IDEA = "an app that helps small farms sell to local restaurants";

function queued(): Analysis {
  return {
    id: "abc",
    prompt: IDEA,
    status: "queued",
    created_at: "2026-08-03T00:00:00Z",
    report: null,
    failure: null,
    choices: [],
    generation: 1,
  };
}

/**
 * A stand-in for the browser's `EventSource`.
 *
 * jsdom has none, so without this every test would take the "no EventSource" branch and the
 * streaming path — the thing a reader actually watches — would never be exercised.
 */
class FakeEventSource {
  static last: FakeEventSource | null = null;
  readonly url: string;
  closed = false;
  onerror: (() => void) | null = null;
  private readonly listeners = new Map<string, (e: MessageEvent<string>) => void>();

  constructor(url: string) {
    this.url = url;
    FakeEventSource.last = this;
  }

  addEventListener(type: string, fn: (e: MessageEvent<string>) => void): void {
    this.listeners.set(type, fn);
  }

  close(): void {
    this.closed = true;
  }

  /** Push one event, the way the server would. */
  send(type: string, data: string): void {
    this.listeners.get(type)?.({ data } as MessageEvent<string>);
  }
}

function stubEventSource(): void {
  FakeEventSource.last = null;
  vi.stubGlobal("EventSource", FakeEventSource);
}

function section(key: string, title: string, claim: string, subject = "") {
  return {
    key,
    title,
    status: "populated" as const,
    claims: [
      {
        text: claim,
        subject,
        source_label: "S1",
        evidence_quote: "",
        confidence: "high" as const,
        as_of: "2026-08-04T00:00:00Z",
      },
    ],
    checked: [],
    notes: [],
  };
}

/** A `fetch` that accepts the POST and reports the analysis complete thereafter. */
function stubAccepting(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn((_url: string, init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: init?.method === "POST" ? 201 : 200,
        json: () =>
          Promise.resolve(
            init?.method === "POST"
              ? queued()
              : { ...queued(), status: "complete" },
          ),
      } as Response),
    ),
  );
}

/**
 * A `fetch` whose GET reports the run **back in the queue with no report** — what a reclaim
 * leaves behind after the worker that was running it died.
 */
function stubReclaimed(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn((_url: string, init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: init?.method === "POST" ? 201 : 200,
        json: () =>
          Promise.resolve(
            init?.method === "POST"
              ? queued()
              // A reclaim raises the generation: that is what tells a reconnecting client
              // the sections it is holding belong to a run that no longer exists.
              : { ...queued(), status: "queued", report: null, generation: 2 },
          ),
      } as Response),
    ),
  );
}

/**
 * A `fetch` that answers every GET with an analysis that is **still running** and already
 * carries a partial report — which is what the API really returns mid-run, because
 * `save_progress` writes the report so far.
 */
function stubStillRunning(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn((_url: string, init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: init?.method === "POST" ? 201 : 200,
        json: () =>
          Promise.resolve(
            init?.method === "POST"
              ? queued()
              : {
                  ...queued(),
                  status: "running",
                  report: {
                    subject: "https://e.com",
                    searched_as: "https://e.com",
                    generated_at: "2026-08-04T00:00:00Z",
                    model_id: "test",
                    prompt_version: 1,
                    sections: [
                      section("pricing", "Pricing & packaging", "Pro costs $15"),
                      // Every partial report carries all six sections from its first write,
                      // each already holding the note it will have if nothing is found.
                      {
                        key: "changes",
                        title: "Recent public changes",
                        status: "not_found_in_public_sources" as const,
                        claims: [],
                        checked: ["/changelog (404)"],
                        notes: [],
                      },
                    ],
                    sources: [],
                  },
                },
          ),
      } as Response),
    ),
  );
}

/** A `fetch` that fails the way the API fails when something breaks at our end. */
function stubBreaking(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: false,
        status: 500,
        json: () =>
          Promise.resolve({
            error: "Something went wrong at our end.",
            remedy:
              "Nothing you did caused this. Try again shortly — and if you tell us, quote 9f2c11ab7d04 and we can find exactly what happened.",
            reference: "9f2c11ab7d04",
          }),
      } as Response),
    ),
  );
}

/** A `fetch` that rejects the POST the way the API rejects a short prompt. */
function stubRejecting(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: false,
        status: 400,
        json: () =>
          Promise.resolve({
            error: "a prompt must contain at least 8 characters, got 5",
            remedy: "Edit what you typed and try again.",
          }),
      } as Response),
    ),
  );
}

function box(): HTMLTextAreaElement {
  return screen.getByLabelText("What is your idea?");
}

/**
 * A `fetch` whose GET returns a **finished** analysis with one section in it — what a shared
 * link resolves to once the run behind it is over.
 */
function stubFinished(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn((_url: string, init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: init?.method === "POST" ? 201 : 200,
        json: () =>
          Promise.resolve({
            ...queued(),
            status: "complete",
            report: {
              subject: "https://basecamp.com",
              searched_as: "https://basecamp.com",
              generated_at: "2026-08-05T00:00:00Z",
              model_id: "test",
              prompt_version: 1,
              sections: [
                section("pricing", "Pricing & packaging", "Pro costs $15"),
              ],
              sources: [],
            },
          }),
      } as Response),
    ),
  );
}

/** A `fetch` whose GET refuses, the way the API answers an id that is not there. */
function stubNotFound(): void {
  stubEventSource();
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: false,
        status: 404,
        json: () =>
          Promise.resolve({
            error: "We could not find that analysis.",
            remedy: "Start a new one.",
          }),
      } as Response),
    ),
  );
}

/**
 * A `fetch` whose GET never resolves until the test says so.
 *
 * The only way to see a race between a navigation and a request already in flight.
 */
function stubDeferredGet(): {
  resolve: (body: unknown) => void;
  resolveNth: (n: number, body: unknown) => void;
} {
  stubEventSource();
  const settlers: ((body: unknown) => void)[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (url: string) =>
        // The first screen asks for its examples on mount. Answering that here rather than
        // queueing it keeps `resolveNth` counting the requests these tests are about — a
        // harness that numbers *every* request is one any new fetch silently renumbers.
        String(url).includes("/api/examples")
          ? Promise.resolve({
              ok: true,
              status: 200,
              json: () => Promise.resolve({ note: "", examples: [] }),
            } as Response)
          : new Promise((res) => {
              settlers.push((body: unknown) =>
                res({
                  ok: true,
                  status: 200,
                  json: () => Promise.resolve(body),
                } as Response),
              );
            }),
    ),
  );
  return {
    resolve: (body: unknown) => settlers[0]?.(body),
    resolveNth: (n: number, body: unknown) => settlers[n]?.(body),
  };
}

/** A finished report whose pricing section says one thing, so two can be told apart. */
function finishedSaying(text: string): unknown {
  return {
    ...queued(),
    status: "complete",
    report: {
      subject: "https://basecamp.com",
      searched_as: "https://basecamp.com",
      generated_at: "2026-08-05T00:00:00Z",
      model_id: "test",
      prompt_version: 1,
      sections: [section("pricing", "Pricing & packaging", text)],
      sources: [],
    },
  };
}

/** Put the browser at a URL naming one analysis, as a shared link would. */
function openAt(path: string): void {
  window.history.replaceState({}, "", path);
}

afterEach(() => {
  vi.unstubAllGlobals();
  // One jsdom window serves every test in this file, so a URL pushed by one of them is still
  // there for the next. Without this reset a test that submits leaves `/a/<id>` behind, and
  // every test after it opens that analysis instead of rendering the box.
  window.history.replaceState({}, "", "/");
});


describe("a run has a URL", () => {
  it("puts the analysis in the address bar as soon as it exists", async () => {
    // Until this, every demo of the product died on a refresh: there was nothing in the URL
    // to come back to, so a reader who reloaded — or who was sent a link — got an empty box.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    await waitFor(() =>
      expect(window.location.pathname).toBe("/a/abc"),
    );
  });

  it("opens the report a link points at", async () => {
    stubFinished();
    openAt("/a/abc");
    render(<App />);

    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
    // And it fetched the id from the path rather than starting something new.
    const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
    expect(calls.some((c) => String(c[0]).endsWith("/api/analyses/abc"))).toBe(
      true,
    );
  });

  it("says so while it is opening, rather than showing an empty box first", async () => {
    // A shared link that renders "What is your idea?" for a moment reads as "there is
    // nothing here" — the opposite of what the link promised.
    stubFinished();
    openAt("/a/abc");
    render(<App />);

    expect(screen.getByText(/Opening this report/)).toBeInTheDocument();
    expect(screen.queryByLabelText("What is your idea?")).toBeNull();
    await screen.findByText(/Pro costs \$15/);
  });

  it("tells the reader when the link points at nothing", async () => {
    // A dead link is the most likely thing to be sent to somebody, because it is the one a
    // reader keeps. A blank page would leave them unsure whether it was them or us.
    stubNotFound();
    openAt("/a/gone");
    render(<App />);

    expect(
      await screen.findByText(/could not find that analysis/i),
    ).toBeInTheDocument();
    // And the box is back, so there is something to do about it.
    expect(screen.getByLabelText("What is your idea?")).toBeInTheDocument();
  });


  it("ignores a report that arrives after the reader has navigated away", async () => {
    // Review found this. The fetch for a link is started and then nothing checks, when it
    // comes back, whether the address bar still names it — so pressing Back while a slow
    // report was loading rendered that report under `/`.
    const deferred = stubDeferredGet();
    openAt("/a/slow");
    render(<App />);
    expect(screen.getByText(/Opening this report/)).toBeInTheDocument();

    // Back to the box, while the GET is still in flight.
    act(() => {
      window.history.replaceState({}, "", "/");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(await screen.findByLabelText("What is your idea?")).toBeInTheDocument();

    // And now the old request finishes.
    await act(async () => {
      deferred.resolve(finishedSaying("Pro costs $15"));
    });

    expect(screen.queryByText(/Pro costs \$15/)).toBeNull();
    expect(screen.getByLabelText("What is your idea?")).toBeInTheDocument();
    // And the stale request must not have left the page saying it was opening something.
    expect(screen.queryByText(/Opening this report/)).toBeNull();
  });


  it("does not let a slow report overwrite the one the reader asked for next", async () => {
    // The other half of the same race, and the more damaging one: two links opened in
    // succession, the first answering last. A report under the wrong URL is worse than no
    // report, because nothing about the page says it is the wrong one.
    const deferred = stubDeferredGet();
    openAt("/a/first");
    render(<App />);

    act(() => {
      window.history.replaceState({}, "", "/a/second");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    // The second request answers first.
    await act(async () => {
      deferred.resolveNth(1, finishedSaying("Second costs $19"));
    });
    expect(await screen.findByText(/Second costs \$19/)).toBeInTheDocument();

    // And now the first one, long overtaken, comes back.
    await act(async () => {
      deferred.resolveNth(0, finishedSaying("First costs $15"));
    });

    expect(screen.getByText(/Second costs \$19/)).toBeInTheDocument();
    expect(screen.queryByText(/First costs \$15/)).toBeNull();
  });


  it("keeps saying it is opening when an older request finishes first", async () => {
    // The third face of the race, and the quietest: an overtaken request finishing does not
    // apply its report, but its `finally` still ran — clearing the state that says a *newer*
    // link is loading. The reader gets the empty box while their report is still on its way.
    const deferred = stubDeferredGet();
    openAt("/a/first");
    render(<App />);

    act(() => {
      window.history.replaceState({}, "", "/a/second");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByText(/Opening this report/)).toBeInTheDocument();

    // Only the first, overtaken request answers.
    await act(async () => {
      deferred.resolveNth(0, finishedSaying("First costs $15"));
    });

    expect(screen.getByText(/Opening this report/)).toBeInTheDocument();
    expect(screen.queryByLabelText("What is your idea?")).toBeNull();
  });

  it("discards a request from a remount, which is what Strict Mode does", async () => {
    // Review found this. The guard was a counter declared *inside* the effect, so it lived
    // exactly as long as one effect instance — and React Strict Mode runs setup, cleanup,
    // setup on mount. Two setups, two counters each starting at zero, and the first setup's
    // request still looked current when it answered.
    //
    // `main.tsx` renders under `<StrictMode>`, so this is the production arrangement rather
    // than a testing artifact.
    const deferred = stubDeferredGet();
    openAt("/a/abc");
    render(
      <StrictMode>
        <App />
      </StrictMode>,
    );

    // The live mount's request answers first.
    await act(async () => {
      deferred.resolveNth(1, finishedSaying("Newest costs $19"));
    });
    expect(await screen.findByText(/Newest costs \$19/)).toBeInTheDocument();

    // And the discarded mount's request comes back afterwards.
    await act(async () => {
      deferred.resolveNth(0, finishedSaying("Discarded costs $15"));
    });

    expect(screen.getByText(/Newest costs \$19/)).toBeInTheDocument();
    expect(screen.queryByText(/Discarded costs \$15/)).toBeNull();
  });

  it("follows the back button to the empty box", async () => {
    // `pushState` adds a history entry, so Back is a thing a reader will press. Leaving the
    // report on screen while the address bar says otherwise is the disagreement that makes
    // people distrust a page.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(window.location.pathname).toBe("/a/abc"));

    act(() => {
      window.history.replaceState({}, "", "/");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });

    expect(await screen.findByLabelText("What is your idea?")).toBeInTheDocument();
    expect(screen.queryByText(IDEA)).toBeNull();
  });
});

describe("submitting an idea", () => {
  it("clears the box once the analysis is accepted", async () => {
    // The empty box is what tells an unregistered reader they have spent their one
    // analysis. A box still holding their words invites a second press and a refusal.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    expect(box().value).toBe(IDEA);

    await user.click(screen.getByRole("button", { name: /analyze/i }));

    await waitFor(() => {
      expect(box().value).toBe("");
    });
  });

  it("keeps what was typed when the prompt is rejected", async () => {
    // Clearing here would make a reader retype from scratch to fix a typo, which is a
    // worse punishment than the typo deserves — and they cannot edit what is gone.
    stubRejecting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), "a crm");
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /at least 8 characters/,
    );
    expect(box().value).toBe("a crm");
  });

  it("shows the reference when something breaks at our end", async () => {
    // A reference the reader never sees is a reference they cannot quote, which leaves us
    // with "it broke this afternoon" against a log holding every other request.
    stubBreaking();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("9f2c11ab7d04");
  });

  it("keeps what was typed when the failure was ours", async () => {
    // Doubly true here: they did nothing wrong, so making them retype would be perverse.
    stubBreaking();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    await screen.findByRole("alert");
    expect(box().value).toBe(IDEA);
  });

  it("will not submit an empty box", async () => {
    stubAccepting();
    render(<App />);
    expect(screen.getByRole("button", { name: /analyze/i })).toBeDisabled();
  });

  it("shows the analysis after it is accepted", async () => {
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    // The submitted idea is still visible even though the box is empty — clearing the
    // input must not mean losing sight of what was asked.
    expect(await screen.findByText(IDEA)).toBeInTheDocument();
  });
});


describe("the ideas offered on the first screen", () => {
  const CATALOG = {
    discovery: true,
    note: "These ideas were chosen by hand. The companies are not: clicking one searches for them, and everything the report says is fetched, quoted and cited at that moment - nothing here is stored or written in advance.",
    examples: [
      {
        id: "project-management",
        idea: "project management for a small design agency",
        prompt: "project management for a small design agency",
      },
    ],
  };

  /** A `fetch` that serves the catalog and accepts a POST. */
  function stubWithExamples(catalog: unknown = CATALOG): void {
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              String(url).includes("/api/examples")
                ? catalog
                : init?.method === "POST"
                  ? queued()
                  : { ...queued(), status: "complete" },
            ),
        } as Response),
      ),
    );
  }

  it("fills the box with the idea and nothing else", async () => {
    // **The inversion of what this used to assert.** The box used to receive
    // `"...agency — basecamp.com vs linear.app"`, which is a *named set*: the run discovers
    // nothing, and the first screen demonstrates the one path where the central feature of
    // this product does not run. A reader clicking an idea wants to watch it find them.
    stubWithExamples();
    const user = userEvent.setup();
    render(<App />);

    const chip = await screen.findByRole("button", {
      name: /project management for a small design agency/i,
    });
    await user.click(chip);

    expect(box().value).toBe("project management for a small design agency");
    expect(box().value).not.toContain(".com");
  });

  it("says so before a run is spent when searching is not configured", async () => {
    // **A reader found this the hard way.** Every idea here is a description, and resolving
    // one needs an engine — so with none configured each of these refuses, and the reader
    // learns about an environment variable at the cost of one of their analyses.
    stubWithExamples({ ...CATALOG, discovery: false });
    render(<App />);

    expect(
      await screen.findByText(/Searching is not configured/i),
    ).toBeInTheDocument();
    expect(screen.getByText("SEARX_URL")).toBeInTheDocument();
  });

  it("says nothing about it when searching works", async () => {
    // A warning that is always there is furniture, and furniture is not read.
    stubWithExamples();
    render(<App />);
    await screen.findByRole("button", { name: /project management/i });
    expect(screen.queryByText(/not configured/i)).toBeNull();
  });

  it("treats a server too old to say as one that cannot search", async () => {
    // Guessing "yes" would put the reader back where they started: an idea typed, a run
    // spent, an environment variable explained afterwards.
    stubWithExamples({ note: CATALOG.note, examples: CATALOG.examples });
    render(<App />);
    expect(
      await screen.findByText(/Searching is not configured/i),
    ).toBeInTheDocument();
  });

  it("does not start the run when a chip is clicked", async () => {
    // Four minutes of somebody else's electricity, started by a click meant to read a label.
    // The reader looks at the sentence and presses Analyze themselves.
    stubWithExamples();
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /project management/i }),
    );

    const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
    expect(calls.some((c) => (c[1] as RequestInit | undefined)?.method === "POST")).toBe(
      false,
    );
  });

  it("says what is curated and what is not", async () => {
    // The sentence comes from the server with the list. Rendering the chips without it would
    // let a reader assume the reports were written in advance too.
    stubWithExamples();
    render(<App />);
    expect(await screen.findByText(/chosen by hand/i)).toBeInTheDocument();
    expect(screen.getByText(/fetched, quoted and cited/i)).toBeInTheDocument();
    // And it must no longer say the companies were chosen for them, because they are not.
    expect(screen.getByText(/companies are not/i)).toBeInTheDocument();
  });

  it("still lets somebody type when the examples cannot be loaded", async () => {
    // They are a way in, not the product. An error banner over an empty box would make a
    // missing convenience look like a broken application.
    // A body with a perfectly good note and a list that is not a list. Two guards read this
    // shape, and a fixture missing *both* fields would let either one carry the test alone.
    stubWithExamples({ discovery: true, note: "chosen by hand", examples: "not a list" });
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    expect(await screen.findByText(IDEA)).toBeInTheDocument();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows no ideas at all when the sentence that qualifies them is missing", async () => {
    // The rule the API enforces on its side: the note travels with the list. Chips rendered
    // without it would let a reader assume the reports were written in advance too — so the
    // honest degradation is no chips, not chips with the qualification quietly dropped.
    stubWithExamples({ discovery: true, examples: CATALOG.examples });
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    expect(
      screen.queryByRole("button", { name: /project management/i }),
    ).toBeNull();
    expect(screen.queryByText(/Or start from one of these/i)).toBeNull();
  });

  it("drops one malformed idea and still shows the others", async () => {
    // Review found this. The first guard checked that `examples` was an array and stopped
    // there, which reads as validation and is not: an entry with a null field reached the
    // render and threw while the first screen was drawing itself — the one path that is
    // supposed to degrade in silence. A bad row costs the reader that row.
    stubWithExamples({
      ...CATALOG,
      examples: [
        { id: "broken", idea: "a broken one", prompt: null },
        CATALOG.examples[0],
      ],
    });
    render(<App />);

    // The good one is on screen, so the malformed entry did not take the list with it.
    expect(
      await screen.findByRole("button", {
        name: /project management for a small design agency/i,
      }),
    ).toBeInTheDocument();
    expect(screen.queryByText("a broken one")).toBeNull();
    // And the page is drawn rather than blank, which is what the exception used to cost.
    expect(screen.getByText(/chosen by hand/i)).toBeInTheDocument();
  });

  it("takes the examples off the screen once there is a report to read", async () => {
    // They are scaffolding for the empty state. Leaving them under a report invites a click
    // that throws away what the reader is looking at.
    stubWithExamples();
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /project management/i }),
    );
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    // The analysis is on screen — that is what `IDEA` is, the prompt the stub reports back.
    expect(await screen.findByText(IDEA)).toBeInTheDocument();
    expect(screen.queryByText(/chosen by hand/i)).toBeNull();
    expect(screen.queryByRole("button", { name: /project management/i })).toBeNull();
  });
});

describe("a report about several companies", () => {
  it("says whose each answer is, in the words the extractor really produces", async () => {
    // Review found this, and found the reason the first version of the test missed it: my
    // fixtures invented claim text like "A costs $10", which names the company. The real
    // extractors produce "Pro costs $15" — what the page says, and nothing more — so a merged
    // pricing section showed two prices with no way to tell whose either was.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    const pricing = {
      ...section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
      claims: [
        section("pricing", "p", "Pro costs $15", "https://basecamp.com").claims[0],
        section("pricing", "p", "Business costs $16", "https://linear.app").claims[0],
      ],
    };
    act(() => stream.send("section", JSON.stringify(pricing)));

    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
    // Both companies named, without their schemes, which is how a person writes them.
    expect(screen.getByText("basecamp.com")).toBeInTheDocument();
    expect(screen.getByText("linear.app")).toBeInTheDocument();
  });

  it("keeps the label when one of two companies says nothing", async () => {
    // Review found this. Deriving "is this a comparison" from the claims on screen looks
    // reasonable and is wrong exactly here: ask about two companies, have one of them yield
    // no pricing, and the survivor's price loses its label in a report that is still a
    // comparison — so a reader sees "Pro costs $15" and cannot tell whose it is.
    const finished = {
      ...queued(),
      status: "complete" as const,
      report: {
        subject: "https://basecamp.com, https://linear.app",
        searched_as: "https://basecamp.com, https://linear.app",
        generated_at: "2026-08-05T00:00:00Z",
        model_id: "test",
        prompt_version: 1,
        subjects: ["https://basecamp.com", "https://linear.app"],
        sections: [
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ],
        sources: [],
      },
    };
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : finished),
        } as Response),
      ),
    );

    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));

    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
    // **The label on the claim**, not merely the name somewhere on the page: the editable set
    // lists the same company above, and a query that finds either would pass with the label
    // gone — which is the defect this test exists for.
    const claim = screen.getByText(/Pro costs \$15/).closest("li");
    expect(claim).not.toBeNull();
    expect(within(claim!).getByText("basecamp.com")).toBeInTheDocument();
  });

  it("labels the first claim while the second company is still being read", async () => {
    // Review found this one *after* the finished report was fixed. Mid-run there is no report
    // yet — `subjects` lives on it, and it is not fetched until `done` — so the label was being
    // guessed from the claims on screen again, one surface later. Basecamp answers first,
    // Linear is still being read, and the price on screen has to say whose it is *now*: the
    // reader is looking at it for the ninety seconds before the report exists.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    act(() =>
      stream.send(
        "subjects",
        JSON.stringify(["https://basecamp.com", "https://linear.app"]),
      ),
    );
    act(() =>
      stream.send(
        "section",
        JSON.stringify(
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ),
      ),
    );

    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
    expect(screen.getByText("basecamp.com")).toBeInTheDocument();
  });

  it("puts a space between the company and what it said", async () => {
    // Review found `basecamp.comPro costs $15` on screen: JSX drops the whitespace around a
    // newline, and every assertion so far had matched the two halves separately. This one
    // reads the line the way a person does — and the way a copy-paste does.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    act(() =>
      stream.send(
        "subjects",
        JSON.stringify(["https://basecamp.com", "https://linear.app"]),
      ),
    );
    act(() =>
      stream.send(
        "section",
        JSON.stringify(
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ),
      ),
    );

    const line = await screen.findByRole("listitem");
    expect(line.textContent).toContain("basecamp.com Pro costs $15");
  });

  it("does not repeat one company down a report about one company", async () => {
    // The other half. A name against every line of a single-company report is noise, and
    // noise is what teaches a reader to stop reading the labels that matter.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() =>
      FakeEventSource.last!.send(
        "section",
        JSON.stringify(
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ),
      ),
    );

    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
    expect(screen.queryByText("basecamp.com")).toBeNull();
  });
});

describe("watching a report being written", () => {
  it("shows a section as soon as it arrives, before the run is over", async () => {
    // The whole point of the stream. PRODUCT_SPEC §2.1A: first content in twenty to forty
    // seconds, not everything at ninety.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    act(() => stream.send("status", "running"));
    act(() => stream.send(
      "section",
      JSON.stringify(section("pricing", "Pricing & packaging", "Pro costs $15")),
    ));

    expect(await screen.findByText("Pricing & packaging")).toBeInTheDocument();
    expect(screen.getByText(/Pro costs \$15/)).toBeInTheDocument();
    // And it says more is coming, so three sections do not read as the whole report.
    expect(screen.getByText(/Still reading/)).toBeInTheDocument();
  });

  it("says what it is doing before anything has arrived", async () => {
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));

    expect(await screen.findByText(/Reading the first pages/)).toBeInTheDocument();
  });

  it("replaces a section as it grows rather than repeating it", async () => {
    // Watching a real run showed why this matters: a section sent once said "1 item" and
    // sat there for two minutes while eight more were read, which reads as finished.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const first = section("pricing", "Pricing & packaging", "Pro costs $15");
    act(() => FakeEventSource.last!.send("section", JSON.stringify(first)));
    await screen.findByText(/Pro costs \$15/);

    const grown = {
      ...first,
      claims: [
        ...first.claims,
        { ...first.claims[0], text: "Business costs $19" },
      ],
    };
    act(() => FakeEventSource.last!.send("section", JSON.stringify(grown)));

    expect(await screen.findByText(/Business costs \$19/)).toBeInTheDocument();
    expect(screen.getAllByText("Pricing & packaging")).toHaveLength(1);
    expect(screen.getByText(/Pro costs \$15/)).toBeInTheDocument();
  });

  it("takes back the dead worker's answers when the run is reclaimed", async () => {
    // Review found this one. Clearing the report in the store fixes a fresh GET and a
    // reconnection; it does nothing for the connection that is already open, which has
    // already *sent* those sections. The reader's screen is the thing that has to change.
    //
    // The interval is not small: the row has to be claimed by a polling worker, then
    // discovery and a fetch and a model call before the replacement has anything to say. And
    // if the second run never reaches that question at all — a page that 404s this time — the
    // retracted claim sits there until the run ends.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    act(() => stream.send("generation", "1"));
    act(() => stream.send("status", "running"));
    act(() =>
      stream.send(
        "section",
        JSON.stringify(section("pricing", "Pricing & packaging", "Pro costs $15")),
      ),
    );
    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();

    // The worker died and the sweep put the run back in the queue, which raises the
    // generation — the signal that these sections belong to a run that no longer exists.
    act(() => stream.send("generation", "2"));
    act(() => stream.send("status", "queued"));

    await waitFor(() =>
      expect(screen.queryByText(/Pro costs \$15/)).not.toBeInTheDocument(),
    );
    // And it still reads as a run in progress, not a finished report with nothing in it.
    expect(screen.queryByText("Done.")).not.toBeInTheDocument();
  });

  it("does not bring back a retracted answer the second run never reaches", async () => {
    // The sharp version of the case above, and the reason a `reset` has to exist rather than
    // relying on the replacement overwriting things. If the second run finds pricing again,
    // that key is overwritten and the stale claim would have gone anyway. If it does not —
    // the page 404s this time, or discovery picks different pages — nothing overwrites it,
    // and without a retraction the dead worker's answer stays on screen until the run ends.
    //
    // A `reset` and not a `done`: a reader told a run finished does not reconnect, and a
    // reclaimed run is the opposite of finished.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const stream = FakeEventSource.last!;
    act(() => stream.send("generation", "1"));
    act(() =>
      stream.send(
        "section",
        JSON.stringify(section("pricing", "Pricing & packaging", "Pro costs $15")),
      ),
    );
    await screen.findByText(/Pro costs \$15/);

    act(() => stream.send("generation", "2"));
    // The replacement run answers a different question entirely.
    act(() =>
      stream.send(
        "section",
        JSON.stringify(section("changes", "What changed recently", "Shipped annotations")),
      ),
    );

    expect(await screen.findByText(/Shipped annotations/)).toBeInTheDocument();
    expect(screen.queryByText(/Pro costs \$15/)).not.toBeInTheDocument();
    expect(screen.queryByText("Pricing & packaging")).not.toBeInTheDocument();
    // The stream is still the one we opened: a reset is not a reason to reconnect.
    expect(stream.closed).toBe(false);
  });

  it("closes the stream when the run is done", async () => {
    // A stream left open against a finished analysis is a connection nobody is reading.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("done", ""));
    expect(FakeEventSource.last!.closed).toBe(true);
    // And the finished report is fetched, which is where the coverage notes live.
    await waitFor(() => expect(screen.getByText("Done.")).toBeInTheDocument());
  });

  it("keeps what arrived when the stream drops", async () => {
    // A proxy that buffers, a laptop that slept. The reader keeps their sections and the
    // fetch puts the rest right — a red banner over a readable report would be worse.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send(
      "section",
      JSON.stringify(section("changes", "Recent public changes", "Shipped annotations")),
    ));
    await screen.findByText("Recent public changes");

    act(() => FakeEventSource.last!.onerror?.());
    expect(screen.getByText("Recent public changes")).toBeInTheDocument();
  });
});

describe("the four blocks", () => {
  /** Run to a finished report, with whatever the case under test needs on it. */
  async function reportWith(extra: Record<string, unknown>): Promise<void> {
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : {
                    ...queued(),
                    status: "complete",
                    report: {
                      subject: IDEA,
                      searched_as: "competitive intelligence software",
                      generated_at: "2026-08-09T00:00:00Z",
                      model_id: "test",
                      prompt_version: 1,
                      sections: [],
                      sources: [],
                      interpreted: null,
                      asked: { kind: "described" },
                      searches: { answered: 8, failed: 0 },
                      ...extra,
                    },
                  },
            ),
        } as Response),
      ),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));
    await screen.findByText("Done.");
  }

  it("says nothing conclusive over a report the run is still writing", async () => {
    // **The case the guard exists for, and the one nothing reached.** A reader who reloads
    // mid-run is served the *partial* report the recovery fetch stored — so `report` is truthy
    // while the run is still going, and every test that checked these blocks were absent had
    // been checking `report` was null rather than that the run had finished. Both mutations
    // aimed at `isTerminal` survived, which is how this was found.
    //
    // Nothing here is settled yet: the count would climb, the set is still being decided, and
    // half a report handed to an assistant is answered from as confidently as all of it.
    stubEventSource();
    openAt("/a/abc");
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        Promise.resolve({
          ok: true,
          status: 200,
          json: () =>
            Promise.resolve({
              ...queued(),
              status: "running",
              report: {
                subject: IDEA,
                searched_as: "",
                generated_at: "2026-08-09T00:00:00Z",
                model_id: "test",
                prompt_version: 1,
                sections: [],
                sources: [],
                subjects: ["basecamp.com"],
                interpreted: null,
                asked: { kind: "described" },
                searches: { answered: 2, failed: 0 },
              },
            }),
        } as Response),
      ),
    );
    render(<App />);
    await screen.findByTitle(IDEA);

    expect(screen.queryByRole("region", { name: "Companies" })).toBeNull();
    expect(screen.queryByRole("button", { name: /copy as context/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /run this set/i })).toBeNull();
  });

  describe("1. what you asked", () => {
    it("carries the whole prompt even while it shows two lines of it", async () => {
      // The clamp is CSS, so the text is all there for anybody who copies it, and the tooltip
      // shows the rest. **A clamp that truncated the string** would put the reader's own words
      // out of reach of selection, search, and a screen reader all at once.
      await reportWith({});
      const shown = await screen.findByTitle(IDEA);
      expect(shown.textContent).toBe(IDEA);
      expect(shown.className).toContain("clamped");
    });

    it("can be reopened after a resize taken while it was open", async () => {
      // **jsdom has no layout**, so overflow is stubbed: a clamped paragraph reports more
      // content than box, an unclamped one reports exactly its content. That is the real
      // relationship, and it is the whole of what the bug turned on.
      const overflowing = (el: HTMLElement): boolean =>
        el.className.includes("clamped");
      for (const [prop, value] of [
        ["scrollHeight", 100],
        ["clientHeight", 40],
      ] as const) {
        Object.defineProperty(HTMLElement.prototype, prop, {
          configurable: true,
          get(this: HTMLElement) {
            return prop === "scrollHeight" || overflowing(this) ? value : 100;
          },
        });
      }
      try {
        await reportWith({});
        const user = userEvent.setup();
        const dots = await screen.findByRole("button", { name: "…" });

        await user.click(dots);
        // Open, the paragraph is exactly as tall as its text — so a resize measured here
        // used to record "not clipped" about a prompt that is.
        act(() => window.dispatchEvent(new Event("resize")));
        await user.click(screen.getByRole("button", { name: /show less/i }));

        expect(screen.getByTitle(IDEA).className).toContain("clamped");
        expect(
          screen.getByRole("button", { name: "…" }),
        ).toBeInTheDocument();
      } finally {
        for (const prop of ["scrollHeight", "clientHeight"]) {
          delete (HTMLElement.prototype as unknown as Record<string, unknown>)[
            prop
          ];
        }
      }
    });
  });

  describe("2. how that was interpreted", () => {
    it("says what it searched for when that is not what the reader typed", async () => {
      // COMPETITIVE_DISCOVERY.md section 4. The substitution decides every query underneath it,
      // so a wrong reading has to be visible *before* anything below it is believed.
      await reportWith({
        interpreted: {
          label: "competitive intelligence software",
          also: ["competitive intelligence", "intelligence software"],
          hosts: 3,
        },
      });
      const line = await screen.findByText(/how I interpreted/i);
      expect(line.textContent).toBe(
        "Here's how I interpreted the business idea: competitive intelligence software",
      );
      expect(screen.getByText(/3 independent sites/)).toHaveTextContent(
        "3 independent sites use this name for it." +
          " Also called competitive intelligence, intelligence software.",
      );
    });

    it("repeats the reader's own words back when nothing was substituted", async () => {
      // **The row an earlier draft left out.** `interpreted` is null whenever nothing was
      // replaced, which includes somebody who typed a phrase the market already uses. Telling
      // them "you named these directly" would be false about the one thing they are checking.
      await reportWith({ interpreted: null, asked: { kind: "described" } });
      const line = await screen.findByText(/as you wrote them/i);
      expect(line.textContent).toBe(
        "I searched for your words as you wrote them: " + IDEA,
      );
      expect(screen.queryByText(/how I interpreted/i)).toBeNull();
    });

    it("does not claim to have interpreted a set the reader named", async () => {
      await reportWith({
        asked: { kind: "named", count: 2 },
        subjects: ["basecamp.com", "linear.app"],
      });
      expect(
        (await screen.findByText(/directly/)).textContent,
      ).toBe("You named basecamp.com, linear.app directly.");
    });

    it("counts one site in the singular", async () => {
      // A number a reader is meant to weigh has to read like one.
      await reportWith({ interpreted: { label: "crm software", also: [], hosts: 1 } });
      expect(await screen.findByText(/1 independent site\b/)).toBeInTheDocument();
    });

    it("says nothing about how a report stored before the contract was read", async () => {
      // `asked` is `#[serde(default)]`, so every row written before this change deserializes
      // with `null`. Each of the three sentences this block can produce is a claim about how
      // the idea was read, and we did not record it — so it makes none of them.
      await reportWith({ asked: null, interpreted: null });
      expect(screen.queryByText(/as you wrote them/i)).toBeNull();
      expect(screen.queryByText(/directly/)).toBeNull();
      expect(screen.queryByText(/how I interpreted/i)).toBeNull();
    });
  });

  describe("3. the count", () => {
    it("counts an old report without claiming to have found it", async () => {
      await reportWith({ asked: null, subjects: ["basecamp.com", "linear.app"] });
      expect(
        (await screen.findByText(/^2 companies/, { selector: ".count span" })).textContent,
      ).toBe("2 companies.");
      expect(screen.queryByText(/I found/)).toBeNull();
    });

    it("says it found what it discovered", async () => {
      await reportWith({ subjects: ["basecamp.com", "linear.app"] });
      expect((await screen.findByText(/^I found/, { selector: ".count span" })).textContent).toBe(
        "I found 2 companies.",
      );
    });

    it("never claims to have found companies the reader typed", async () => {
      // `Subjects::Exactly` hands the domains straight through — no discovery of any kind — so
      // "I found 2 companies" would be this product taking credit for reading a list.
      await reportWith({
        asked: { kind: "named", count: 2 },
        subjects: ["basecamp.com", "linear.app"],
      });
      expect((await screen.findByText(/^You named 2/, { selector: ".count span" })).textContent).toBe(
        "You named 2 companies.",
      );
      expect(screen.queryByText(/I found/)).toBeNull();
    });

    it("separates the seed from what was found around it", async () => {
      await reportWith({
        asked: { kind: "seeded", named: "basecamp.com" },
        subjects: ["basecamp.com", "linear.app", "height.app"],
      });
      expect((await screen.findByText(/^You named basecamp/, { selector: ".count span" })).textContent).toBe(
        "You named basecamp.com, and I found 2 more like it.",
      );
    });

    it("does not hedge a seed whose rival search finished", async () => {
      // **The fixture used to answer this for the seeded case**, so the shared
      // `searches: { answered: 8, failed: 0 }` hid a worker that left `searches` null on that
      // path entirely and rendered every complete search as "at least". Set explicitly here,
      // both ways, so the test says what it is testing.
      await reportWith({
        asked: { kind: "seeded", named: "basecamp.com" },
        subjects: ["basecamp.com", "linear.app", "height.app"],
        searches: { answered: 3, failed: 0 },
      });
      expect(
        (await screen.findByText(/^You named basecamp/, { selector: ".count span" }))
          .textContent,
      ).toBe("You named basecamp.com, and I found 2 more like it.");
    });

    it("hedges a seed whose rival search did not finish", async () => {
      await reportWith({
        asked: { kind: "seeded", named: "basecamp.com" },
        subjects: ["basecamp.com", "linear.app", "height.app"],
        searches: { answered: 2, failed: 1 },
      });
      expect(
        (await screen.findByText(/^You named basecamp/, { selector: ".count span" }))
          .textContent,
      ).toBe("You named basecamp.com, and I found at least 2 more like it.");
    });

    it("hedges a seed the worker reported no coverage for", async () => {
      // `searches: null` means nothing was asked — no engine configured, or the seed's own
      // page gave nothing to search with. A bare count over that is a definite number about a
      // search that never happened; `report.notes` carries which of those it was.
      await reportWith({
        asked: { kind: "seeded", named: "basecamp.com" },
        subjects: ["basecamp.com", "linear.app"],
        searches: null,
      });
      expect(
        (await screen.findByText(/^You named basecamp/, { selector: ".count span" }))
          .textContent,
      ).toBe("You named basecamp.com, and I found at least 1 more like it.");
    });

    it("hedges a count taken over a search that did not finish", async () => {
      // A bare "12" over a partial search is a definite number about an indefinite thing: the
      // thirteenth may not exist, or may be behind the query that timed out, and a reader
      // cannot tell which. "At least" costs one word and is true.
      await reportWith({
        subjects: ["basecamp.com", "linear.app"],
        searches: { answered: 6, failed: 2 },
      });
      expect((await screen.findByText(/^I found/, { selector: ".count span" })).textContent).toBe(
        "I found at least 2 companies.",
      );
    });

    it("says the other two searches do not exist rather than reporting zero", async () => {
      // A zero is a claim that we looked. PRODUCT_IDEA_RESULTS.md 2.5 and 4.1.
      await reportWith({ subjects: ["basecamp.com"] });
      expect(await screen.findByText(/have not looked/, { selector: ".not-yet" })).toBeInTheDocument();
      expect(screen.queryByText(/0 open source projects/)).toBeNull();
      expect(screen.queryByText(/0 discussions/)).toBeNull();
    });
  });

  describe("what did not come back", () => {
    /** The one block whose search actually ran. */
    async function companies(): Promise<HTMLElement> {
      return screen.findByRole("region", { name: "Companies" });
    }

    it("names what was missed under a description whose search did not finish", async () => {
      // **The other half of "at least."** The hedge says the number is soft; this says whether
      // re-running would plausibly change it. One failed search out of eight and six out of
      // eight are the same word and very different decisions — `PRODUCT_IDEA_RESULTS.md`
      // §2.5 asks for both, and the first version of this page shipped only the word.
      await reportWith({
        subjects: ["basecamp.com", "linear.app"],
        searches: { answered: 6, failed: 2 },
      });
      expect(
        within(await companies()).getByText(
          "2 of 8 searches did not come back.",
        ),
      ).toBeInTheDocument();
    });

    it("names what was missed under a seed whose rival search did not finish", async () => {
      // The path the contract reached second, and the one review found empty twice.
      await reportWith({
        asked: { kind: "seeded", named: "basecamp.com" },
        subjects: ["basecamp.com", "linear.app"],
        searches: { answered: 2, failed: 1 },
      });
      expect(
        within(await companies()).getByText("1 of 3 searches did not come back."),
      ).toBeInTheDocument();
    });

    it("says nothing when every search came back", async () => {
      // A line about coverage over a complete search is noise, and noise is what stops the
      // real one being read.
      await reportWith({
        subjects: ["basecamp.com", "linear.app"],
        searches: { answered: 8, failed: 0 },
      });
      expect(
        within(await companies()).queryByText(/did not come back/),
      ).toBeNull();
    });

    it("says nothing when no search was sent at all", async () => {
      // `null` is what the worker sends when nothing was asked — no engine, or the seed's own
      // page gave nothing to search with. A coverage line over that would invent a search.
      await reportWith({ subjects: ["basecamp.com"], searches: null });
      expect(
        within(await companies()).queryByText(/did not come back/),
      ).toBeNull();
    });
  });

  describe("4. the lists", () => {
    const MANY = Array.from({ length: 31 }, (_, i) => "co" + String(i) + ".example");

    it("shows five, and offers exactly what it can reveal", async () => {
      // The count and the cap are different numbers. The control names what it can actually
      // show — 20, not the 26 that are off screen — because a promise a reader can check is a
      // promise a reader will check.
      await reportWith({ subjects: MANY });
      const list = await screen.findByRole("region", { name: "Companies" });
      expect(within(list).getAllByRole("listitem")).toHaveLength(5);
      expect(within(list).getByText("31 found")).toBeInTheDocument();
      expect(
        within(list).getByRole("button", { name: "…more (20)" }),
      ).toBeInTheDocument();
    });

    it("counts what is on screen now, not what will be", async () => {
      // Before the click it reads "Showing 5 of 31". A line that says 25 while five rows are
      // visible is false in the state a reader spends most of their time in.
      await reportWith({ subjects: MANY });
      const list = await screen.findByRole("region", { name: "Companies" });
      expect(within(list).getByText(/Showing 5 of 31/)).toBeInTheDocument();
      await userEvent.setup().click(within(list).getByRole("button", { name: /more/ }));
      expect(within(list).getAllByRole("listitem")).toHaveLength(25);
      expect(within(list).getByText(/Showing 25 of 31/)).toBeInTheDocument();
    });

    it("shows a company as a reader would type it", async () => {
      await reportWith({ subjects: ["https://basecamp.com", "https://linear.app"] });
      const list = await screen.findByRole("region", { name: "Companies" });
      expect(
        within(list)
          .getAllByRole("listitem")
          .map((li) => li.textContent),
      ).toEqual(["basecamp.com", "linear.app"]);
    });

    it("keeps the order a reader wrote", async () => {
      // `Subjects::Exactly` is "exactly these, in the order written", and somebody comparing
      // `basecamp.com vs linear.app` has put the one they care about first. Re-scoring would
      // overrule them in the one place they were most explicit.
      await reportWith({
        asked: { kind: "named", count: 3 },
        subjects: ["zulip.com", "basecamp.com", "linear.app"],
      });
      const list = await screen.findByRole("region", { name: "Companies" });
      expect(
        within(list)
          .getAllByRole("listitem")
          .map((li) => li.textContent),
      ).toEqual(["zulip.com", "basecamp.com", "linear.app"]);
    });

    it("keeps the headings of the searches that were never built", async () => {
      // A missing heading is indistinguishable from a feature that does not exist, and an
      // empty list would be this product claiming it looked.
      await reportWith({ subjects: ["basecamp.com"] });
      for (const heading of ["Open source projects", "Discussions"]) {
        const block = await screen.findByRole("region", { name: heading });
        // **The visible heading, not the landmark's name.** The first version of this asserted
        // `findByRole("region", { name: heading })` and stopped, which passes with the <h2>
        // deleted - the accessible name comes from `aria-label`, so the one thing the test was
        // about was the one thing it could not see. The mutation harness found it.
        expect(
          within(block).getByRole("heading", { name: heading }),
        ).toBeInTheDocument();
        expect(within(block).getByText("not built")).toBeInTheDocument();
        // And the sentence, because an empty <ul> also has no rows in it.
        expect(
          within(block).getByText(/This search does not exist yet/),
        ).toBeInTheDocument();
        expect(within(block).queryAllByRole("listitem")).toHaveLength(0);
      }
    });
  });
});

describe("when it does not finish", () => {
  it("tells a reader who named no company what to do instead", async () => {
    // The failure they can fix. "Nothing you did caused it" would be both wrong and a
    // dead end.
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : { ...queued(), status: "failed", failure: "no_subject" },
            ),
        } as Response),
      ),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("done", ""));

    expect(
      await screen.findByText(/could not work out which company/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/basecamp\.com/)).toBeInTheDocument();
  });

  /** Run to a failure of one kind, and hand back what the reader is shown. */
  async function shownFor(
    failure: Failure,
    choices: Analysis["choices"] = [],
  ): Promise<void> {
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : { ...queued(), status: "failed", failure, choices },
            ),
        } as Response),
      ),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));
  }

  // **These four were one sentence.** `Failure` had two values, so every ending the analysis
  // produced arrived as `no_subject` and read *"try naming its website"* — which fixes nothing
  // when a search timed out, and throws away the question a reader could answer in a word.
  //
  // Each case asserts what it must *not* say as well as what it must: the wrong instruction is
  // the part that sends somebody the wrong way, and it is the half a wording test forgets.

  it("offers the choice back when a name matches more than one company", async () => {
    // With nothing to pick between, the sentence has to carry the whole instruction.
    await shownFor("ambiguous");
    expect(
      await screen.findByText(/matches more than one company/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/name the one you mean/i)).toBeInTheDocument();
    expect(screen.queryByText(/try again/i)).toBeNull();
  });

  // `prompt` is an origin rather than the bare domain, and the server decides that — see
  // `choices_from`. A bare `box.com` is seven characters and the API rejects anything under
  // eight, so a chip for a short domain rendered and then answered a click with a 400.
  const NOTION = {
    name: "Notion",
    domain: "notion.so",
    what_it_is: "one workspace for notes, docs and projects",
    prompt: "https://notion.so",
  };
  const NOTION_ENERGY = {
    name: "Notion Energy",
    domain: "notionenergy.com",
    what_it_is: "battery storage for commercial sites",
    prompt: "https://notionenergy.com",
  };

  it("puts the companies on screen rather than asking a reader to name one", async () => {
    // The question we refused to answer, handed back in the form that answers it. A page that
    // says "we found several" without saying *which* leaves the reader guessing at precisely
    // what the gate declined to guess at.
    await shownFor("ambiguous", [NOTION, NOTION_ENERGY]);

    expect(
      await screen.findByRole("button", { name: /notion energy/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("notion.so")).toBeInTheDocument();
    expect(screen.getByText(/battery storage/i)).toBeInTheDocument();
    // And the instruction changes with the affordance. Telling somebody to type a website
    // while the websites are buttons under the sentence asks them to do the work twice.
    expect(screen.getByText(/pick the one you meant/i)).toBeInTheDocument();
    expect(screen.queryByText(/a website works/i)).toBeNull();
  });

  it("spends one click on the answer, not a retyped idea", async () => {
    // `PRODUCT_SPEC.md` §3 prices a clarification at one click. That is either true here or
    // it is a sentence in a document: what the chip sends has to be a whole prompt, sent
    // verbatim, with nothing assembled in the browser.
    stubEventSource();
    const sent: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        if (init?.method === "POST") {
          sent.push((JSON.parse(String(init.body)) as { prompt: string }).prompt);
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : {
                    ...queued(),
                    status: "failed",
                    failure: "ambiguous",
                    choices: [NOTION, NOTION_ENERGY],
                  },
            ),
        } as Response);
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));

    await user.click(
      await screen.findByRole("button", { name: /notion energy/i }),
    );

    await waitFor(() => expect(sent).toHaveLength(2));
    expect(sent[1]).toBe("https://notionenergy.com");
    // The run it started is the one the URL now names, so a reader who reloads keeps it.
    expect(window.location.pathname).toBe("/a/abc");
  });

  it("does not let a second click spend a second analysis", async () => {
    // One question, one answer. The run starts on the first click and the chips stay on
    // screen until it replaces them, which is exactly long enough for an impatient second
    // click to start a whole second analysis on the same question.
    stubEventSource();
    let posts = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        if (init?.method === "POST") {
          posts += 1;
          // The second POST never settles, so the button's state is the only thing
          // standing between one question and two runs.
          if (posts > 1) return new Promise<Response>(() => {});
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : {
                    ...queued(),
                    status: "failed",
                    failure: "ambiguous",
                    choices: [NOTION, NOTION_ENERGY],
                  },
            ),
        } as Response);
      }),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));

    const chip = await screen.findByRole("button", { name: /notion energy/i });
    await user.click(chip);
    await waitFor(() => expect(chip).toBeDisabled());
    await user.click(chip);
    expect(posts).toBe(2);
  });

  it("shows a market to pick between without inventing a website for it", async () => {
    // The ambiguous-*market* question reuses the chip the ambiguous-*company* question built,
    // and a market has no domain. An empty one must not render as a blank line where a
    // reader has learned to expect the thing that tells two choices apart.
    await shownFor("ambiguous", [
      {
        name: "inventory management software",
        domain: "",
        what_it_is: "2 independent sites use this name",
        prompt: "inventory management software",
      },
      {
        name: "project management software",
        domain: "",
        what_it_is: "2 independent sites use this name",
        prompt: "project management software",
      },
    ]);
    expect(
      await screen.findByRole("button", { name: /inventory management software/i }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/2 independent sites use this name/)).toHaveLength(2);
    expect(document.querySelectorAll(".choice .domain")).toHaveLength(0);
  });

  it("does not offer a question the report on screen has already answered", async () => {
    // A row can carry the choices an earlier attempt left behind. Offering them under a
    // finished report invites somebody to re-run something they can already read.
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : {
                    ...queued(),
                    status: "complete",
                    choices: [NOTION, NOTION_ENERGY],
                  },
            ),
        } as Response),
      ),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));

    expect(await screen.findByText("Done.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /notion energy/i })).toBeNull();
  });

  it("does not blame a reader's idea for an environment variable", async () => {
    // **The bug a reader found, and the sharpest one in this file.** With no engine the run
    // refused as `no_subject`, which renders as *"we could not work out which company you
    // meant, try naming its website"* — told to somebody who typed a perfectly good product
    // idea, which is the input this product exists for.
    //
    // Worse than wrong: the one thing it tells them to type instead, a domain, is the research
    // they came here to have done. The server's own sentence had named the cause since it was
    // written; `Failure` is what the surface reads, and it had one kind for two situations.
    await shownFor("no_engine");
    expect(
      await screen.findByText(/No search engine is configured/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/Your idea is fine/i)).toBeInTheDocument();
    expect(screen.queryByText(/naming its website/i)).toBeNull();
    expect(screen.queryByText(/could not work out which company/i)).toBeNull();
  });

  it("says the market was empty rather than blaming the prompt", async () => {
    await shownFor("nothing_found");
    expect(
      await screen.findByText(/found no company we could stand behind/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/try again/i)).toBeNull();
  });

  it("tells a reader to wait, and only when waiting is what would help", async () => {
    // The one situation a reader fixes by doing nothing. It must not send them off to change
    // a prompt that was never the problem — and no other case may say "try again", or the
    // phrase stops meaning anything on the one where it is true.
    await shownFor("search_incomplete");
    expect(await screen.findByText(/search did not finish/i)).toBeInTheDocument();
    expect(screen.getByText(/try again/i)).toBeInTheDocument();
    expect(screen.queryByText(/naming its website/i)).toBeNull();
    expect(screen.queryByText(/nothing you did caused it/i)).toBeNull();
  });

  it("does not offer waiting when the engine has already answered", async () => {
    // **The same counts, the opposite advice.** An engine that refuses will refuse again, so
    // "try again" is an instruction to wait for something that cannot happen. This is the
    // documented first-run state of the checked-in search profile, not an exotic case.
    await shownFor("search_refused");
    expect(await screen.findByText(/refusing us/i)).toBeInTheDocument();
    expect(screen.queryByText(/try again/i)).toBeNull();
    // And it still offers the one route that does not need the engine at all.
    expect(screen.getByText(/naming a website skips the search/i)).toBeInTheDocument();
  });

  it("takes the blame when the failure was ours", async () => {
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () =>
            Promise.resolve(
              init?.method === "POST"
                ? queued()
                : { ...queued(), status: "failed", failure: "internal" },
            ),
        } as Response),
      ),
    );
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("done", ""));

    expect(
      await screen.findByText(/Nothing you did caused it/),
    ).toBeInTheDocument();
  });
});

describe("when the stream drops before the run is over", () => {
  it("opens it again rather than leaving a half-written report", async () => {
    // The failure this guards. A running analysis already carries the report so far, so
    // "we have a report" is not "it is finished": treating it as settled meant one fetch,
    // a still-running answer, and nothing ever reconnecting.
    stubStillRunning();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const dropped = FakeEventSource.last!;
    act(() => dropped.onerror?.());

    // A second connection, to the same analysis, without the reader doing anything.
    await waitFor(() => expect(FakeEventSource.last).not.toBe(dropped), {
      timeout: 4000,
    });
    expect(FakeEventSource.last!.url).toContain("/events");
    expect(dropped.closed).toBe(true);
  });

  it("still says it is reading, even holding a partial report", async () => {
    // The same bug wearing its other face: a partial report made the view look finished.
    stubStillRunning();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("done", ""));

    expect(
      await screen.findByText(/Still reading|Reading the first pages/),
    ).toBeInTheDocument();
  });

  it("stops once the analysis is complete", async () => {
    // And the other side of it: a finished run must not keep reconnecting for ever.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const finished = FakeEventSource.last!;
    act(() => finished.send("done", ""));
    await waitFor(() => expect(screen.getByText("Done.")).toBeInTheDocument());

    await new Promise((resolve) => setTimeout(resolve, 1500));
    expect(FakeEventSource.last).toBe(finished);
  });
});

describe("after a reconnect", () => {
  it("does not carry a dead worker's answers across the reconnect", async () => {
    // Review found this one. The retraction worked on a live connection and not across a
    // broken one: the reader's sections survive a reconnect *on purpose*, while the server's
    // record of what it has sent starts empty on every new stream.
    //
    // The sequence: a section arrives, the connection drops, the recovery fetch finds the run
    // back in the queue with no report, and the fresh stream opens on `queued`. At that
    // moment two stale copies of the dead worker's answer are in play — the sections this
    // hook accumulated and the report a recovery fetch cached — and nothing on the new
    // connection had been told to take either of them back.
    stubReclaimed();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const dropped = FakeEventSource.last!;
    act(() => dropped.send("generation", "1"));
    act(() => dropped.send("status", "running"));
    act(() =>
      dropped.send(
        "section",
        JSON.stringify(section("pricing", "Pricing & packaging", "Pro costs $15")),
      ),
    );
    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();

    // The connection drops. The run is reclaimed before the client gets back.
    act(() => dropped.onerror?.());
    await waitFor(() => expect(FakeEventSource.last).not.toBe(dropped), {
      timeout: 4000,
    });

    await waitFor(() =>
      expect(screen.queryByText(/Pro costs \$15/)).not.toBeInTheDocument(),
    );
    // And it is still a run in progress, not a finished report with nothing in it.
    expect(screen.queryByText("Done.")).not.toBeInTheDocument();
  });


  it("keeps rendering what the new stream sends", async () => {
    // The recovery fetch stores a still-running analysis whose partial report is a snapshot.
    // Reading from that snapshot froze the page at the moment of the fetch and threw away
    // every section the reconnected stream delivered.
    stubStillRunning();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    const dropped = FakeEventSource.last!;
    act(() => dropped.onerror?.());
    await waitFor(() => expect(FakeEventSource.last).not.toBe(dropped), {
      timeout: 4000,
    });

    act(() =>
      FakeEventSource.last!.send(
        "section",
        JSON.stringify(section("identity", "Company facts", "says it was founded in 2018")),
      ),
    );

    expect(await screen.findByText(/founded in 2018/)).toBeInTheDocument();
  });

  it("does not say a question found nothing while it is still being read", async () => {
    // A partial report's placeholder sections carry the coverage note they *would* have if
    // nothing were found. Showing one mid-run tells a reader we finished looking.
    stubStillRunning();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.onerror?.());
    await screen.findByText(/Pro costs \$15/);

    expect(screen.queryByText(/Nothing found in public sources/)).toBeNull();
    expect(screen.queryByText("/changelog (404)")).toBeNull();
  });

  it("says Done when the recovery fetch finds the run finished", async () => {
    // The stream's last word was "running" and then it dropped. A finished report under a
    // "Reading public web pages…" line is worse than either half of that.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("status", "running"));
    // The running indicator, not the old sentence: a run that has reported no phase yet says
    // it is working and refuses to say what it is working *on*, because it does not know.
    expect(await screen.findByText("Working")).toBeInTheDocument();

    act(() => FakeEventSource.last!.onerror?.());

    expect(await screen.findByText("Done.")).toBeInTheDocument();
    expect(screen.queryByText("Working")).toBeNull();
  });
});

describe("the estimate, where nothing has been counted yet", () => {
  // **Unit tests, because this is the one number in the interface that is not counted.**
  // `Off-The-Napkin-Estimates.md` §1 permits it and says what the condition is: a reader must
  // be able to tell it is an estimate. These assert the other half - that it cannot run ahead
  // of the count it is standing in for.
  const started = new Date("2026-08-11T12:00:00Z");
  const after = (seconds: number): Date =>
    new Date(started.getTime() + seconds * 1000);

  it("starts at nothing and climbs", () => {
    expect(estimate(started, started, 17)).toBe(0);
    expect(estimate(started, after(12), 17)).toBeGreaterThan(0);
    expect(estimate(started, after(24), 17)).toBeGreaterThan(
      estimate(started, after(12), 17),
    );
  });

  it("never passes the ceiling the server sent", () => {
    // The ceiling is where counting begins, computed by `landscape-core::progress`. A minute
    // in, ten minutes in, an hour in - the estimate stops there, so the counted percentage
    // never has to come down to meet it.
    for (const seconds of [30, 60, 600, 3600]) {
      expect(estimate(started, after(seconds), 17)).toBeLessThanOrEqual(17);
      expect(estimate(started, after(seconds), 5)).toBeLessThanOrEqual(5);
    }
  });

  it("a clock that runs backwards does not produce a negative bar", () => {
    expect(estimate(started, new Date(started.getTime() - 60_000), 17)).toBe(0);
  });

  it("a ceiling of nothing is nothing, not a negative", () => {
    // What arrives before the first progress event: no ceiling known yet.
    expect(estimate(started, after(600), 0)).toBe(0);
  });
});

describe("how far through it is", () => {
  /** Start a run and get it to `running`, which is where the indicator lives. */
  async function running(): Promise<void> {
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("status", "running"));
  }

  const bar = (): HTMLElement | null =>
    document.querySelector("progress.bar");

  it("estimates while nothing has been counted, and says it is estimating", async () => {
    // **A number always, and the reader can tell which kind.** Before this the bar showed `—`
    // for the opening stretch of an eight-minute wait, on the argument that no denominator
    // existed. That was the report's "never invent a fact" rule applied to an affordance;
    // `Off-The-Napkin-Estimates.md` §1 draws the line in the right place, and what it forbids
    // is estimation a reader cannot *tell* is an estimate. The tilde is that telling.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "discovering",
          saying: "Finding the pages worth reading",
          percent: null,
          estimating_to: 17,
          companies: { done: 0, of: 0 },
          pages: null,
        }),
      ),
    );

    const shown = await screen.findByText(/^~\d+%$/);
    expect(shown).toBeInTheDocument();
    expect(screen.queryByText("—")).toBeNull();
    expect(
      await screen.findByText(/Finding the pages worth reading/),
    ).toBeInTheDocument();
  });

  it("a counted percentage replaces the estimate, and drops the tilde", async () => {
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "discovering",
          saying: "Finding the pages worth reading",
          percent: null,
          estimating_to: 17,
          companies: { done: 0, of: 1 },
          pages: null,
        }),
      ),
    );
    await screen.findByText(/^~\d+%$/);

    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 40,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 2, of: 5 },
        }),
      ),
    );

    expect(await screen.findByText("40%")).toBeInTheDocument();
    expect(screen.queryByText(/^~/)).toBeNull();
  });

  it("shows the real fraction once a plan exists", async () => {
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 40,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 2, of: 5 },
        }),
      ),
    );

    expect(await screen.findByText("40%")).toBeInTheDocument();
    expect(bar()).toHaveAttribute("value", "40");
    // The page being read now is the one after the pages already read.
    expect(await screen.findByText(/page 3 of 5/)).toBeInTheDocument();
  });

  it("names the company when there is more than one", async () => {
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 50,
          estimating_to: null,
          companies: { done: 1, of: 3 },
          pages: { done: 1, of: 4 },
        }),
      ),
    );
    expect(await screen.findByText(/company 2 of 3/)).toBeInTheDocument();
  });

  it("does not name a company when the report is about one", async () => {
    // Noise. Every report covers at least one company, and saying "company 1 of 1" tells a
    // reader something they cannot act on.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 20,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 1, of: 5 },
        }),
      ),
    );
    await screen.findByText("20%");
    expect(screen.queryByText(/company 1 of 1/)).toBeNull();
  });

  it("a percentage that is not a number is dropped rather than read as zero", async () => {
    // `Number(null)` is `0`, and a bar reporting zero percent for "we do not know" is exactly
    // the lie this refuses. A malformed tick leaves the previous state alone.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: "lots",
          estimating_to: 17,
          companies: { done: 0, of: 1 },
          pages: null,
        }),
      ),
    );
    // **The guard is still the point**, and it is now about which number is shown rather than
    // whether one is: a malformed tick must not be coerced to a *counted* `0%`. It falls back
    // to the estimate, which carries a tilde and is therefore not the same claim.
    expect(await screen.findByText(/^~\d+%$/)).toBeInTheDocument();
    expect(screen.queryByText("0%")).toBeNull();
  });

  it("does not fall back to zero when the first counted tick arrives", async () => {
    // **The seam, driven end to end.** The estimate climbs across discovery; the first counted
    // tick after `order::plan` is `Reading, pages 0/N`, whose percentage used to be `0`. The
    // bar therefore fell from its cap straight back to nothing - the one thing this feature
    // promises it will not do, at the join neither side owned. The earlier test jumped from
    // the estimate to 40% and never exercised the handoff at all.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "discovering",
          saying: "Finding the pages worth reading",
          percent: null,
          estimating_to: 17,
          companies: { done: 0, of: 1 },
          pages: null,
        }),
      ),
    );
    const before = Number(
      (await screen.findByText(/^~?\d+%$/)).textContent!.replace(/[~%]/g, ""),
    );

    // The real first tick: a plan exists, and nothing has been read from it yet.
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 17,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 0, of: 9 },
        }),
      ),
    );

    const after = Number(
      (await screen.findByText(/^~?\d+%$/)).textContent!.replace(/[~%]/g, ""),
    );
    expect(after).toBeGreaterThanOrEqual(before);
    expect(screen.queryByText("0%")).toBeNull();
  });

  it("does not offer a page ordinal when the plan is empty", async () => {
    // A company discovery found nothing for is a real plan of zero pages, and it rendered
    // "page 1 of 0" - an ordinal for a page that does not exist.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 17,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 0, of: 0 },
        }),
      ),
    );
    await screen.findByText(/Reading public web pages/);
    expect(screen.queryByText(/page 1 of 0/)).toBeNull();
    // No ordinal of any shape. `/page/` alone would match "Reading public web pages".
    expect(screen.queryByText(/page \d+ of \d+/)).toBeNull();
  });

  it("does not count a page past the plan once searching starts", async () => {
    // The finished plan is kept through the search phase, where `done === of` rendered
    // "page N+1 of N" about pages that are deliberately outside the plan.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "searching",
          saying: "Searching for what their own pages did not say",
          percent: 90,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 5, of: 5 },
        }),
      ),
    );
    expect(
      await screen.findByText(/Searching for what their own pages did not say/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/page 6 of 5/)).toBeNull();
    expect(screen.queryByText(/page \d+ of \d+/)).toBeNull();
  });

  it("a new worker does not inherit the dead one's percentage", async () => {
    // **A reclaim need not pass back through `queued` from here.** The stream can drop during
    // the sweep and reconnect with the replacement already `running`, so `status` never stops
    // being `running` and nothing else in this component would clear the progress. The floor
    // that makes the bar monotonic lives in a ref, and a ref survives a re-render - so the
    // dead worker's 90% became the replacement's starting floor, and its elapsed time dated
    // the replacement's estimate.
    await running();
    act(() =>
      FakeEventSource.last!.send("generation", "1"),
    );
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 90,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 9, of: 10 },
        }),
      ),
    );
    expect(await screen.findByText("90%")).toBeInTheDocument();

    // The sweep hands the run to another worker. Status stays `running` throughout.
    act(() => FakeEventSource.last!.send("generation", "2"));
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "discovering",
          saying: "Finding the pages worth reading",
          percent: null,
          estimating_to: 17,
          companies: { done: 0, of: 1 },
          pages: null,
        }),
      ),
    );

    // The replacement is discovering, so it is estimating - and from nothing, not from 90.
    const shown = await screen.findByText(/^~\d+%$/);
    expect(Number(shown.textContent!.replace(/[~%]/g, ""))).toBeLessThan(90);
    expect(screen.queryByText("90%")).toBeNull();
  });

  it("a finished run is a different word and no bar, not a bar that stopped", async () => {
    // **A still bar and a hung bar look identical.** The question a reader is asking is which
    // of the two they are looking at, so the finished state answers it in words and by the
    // bar being gone rather than by the absence of movement.
    await running();
    act(() =>
      FakeEventSource.last!.send(
        "progress",
        JSON.stringify({
          phase: "reading",
          saying: "Reading public web pages",
          percent: 60,
          estimating_to: null,
          companies: { done: 0, of: 1 },
          pages: { done: 3, of: 5 },
        }),
      ),
    );
    expect(await screen.findByText("Working")).toBeInTheDocument();
    expect(bar()).not.toBeNull();

    act(() => FakeEventSource.last!.send("status", "complete"));
    act(() => FakeEventSource.last!.send("done", ""));

    expect(await screen.findByText("Done.")).toBeInTheDocument();
    expect(screen.queryByText("Working")).toBeNull();
    expect(screen.queryByText("60%")).toBeNull();
    await waitFor(() => expect(bar()).toBeNull());
  });
});

describe("the set a report was built from", () => {
  /** A finished report about two companies, which is what §5.5 says must be correctable. */
  function about(...companies: string[]) {
    return {
      ...queued(),
      status: "complete" as const,
      report: {
        subject: companies.join(", "),
        searched_as: companies.join(", "),
        generated_at: "2026-08-05T00:00:00Z",
        model_id: "test",
        prompt_version: 1,
        subjects: companies,
        sections: [
          section("pricing", "Pricing & packaging", "Pro costs $15", companies[0]),
        ],
        sources: [],
      },
    };
  }

  /** Every POST body the page sent, in order. Re-running is a new analysis, not a mutation. */
  function watchPosts(finished: ReturnType<typeof about>): string[] {
    const sent: string[] = [];
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        if (init?.method === "POST") {
          sent.push(JSON.parse(String(init.body)).prompt as string);
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : finished),
        } as Response);
      }),
    );
    return sent;
  }

  async function arrive(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));
    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
  }

  it("shows the companies it compared, so the choice can be checked", async () => {
    // **A competitive set presented without its derivation is an unfalsifiable editorial
    // choice** — COMPETITIVE_DISCOVERY §5.5. The notes above say *why* each one is here; this
    // is the part a reader can act on.
    watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    const set = screen.getByRole("region", { name: /companies in this report/i });
    expect(within(set).getByText("basecamp.com")).toBeInTheDocument();
    expect(within(set).getByText("linear.app")).toBeInTheDocument();
  });

  it("runs the corrected set rather than asking the reader to retype the idea", async () => {
    // §6.3: direct manipulation beats interrogation. The button hands back a whole prompt, the
    // same way a clarifying chip does, so correcting the set costs a click.
    const sent = watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    await user.click(screen.getByRole("button", { name: /remove linear\.app/i }));
    await user.type(screen.getByLabelText(/add a company/i), "notion.so");
    await user.click(screen.getByRole("button", { name: /^add$/i }));
    await user.click(screen.getByRole("button", { name: /run this set/i }));

    expect(sent).toEqual([IDEA, "https://basecamp.com notion.so"]);
  });

  it("will not re-run the set already on screen", async () => {
    // Ninety seconds to redraw a page a reader is already looking at is a cost with nothing
    // on the other side of it.
    watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: /remove linear\.app/i }));
    expect(screen.getByRole("button", { name: /run this set/i })).toBeEnabled();
  });

  it("refuses an empty set and says what it would mean", async () => {
    watchPosts(about("https://basecamp.com"));
    const user = userEvent.setup();
    await arrive(user);

    await user.click(screen.getByRole("button", { name: /remove basecamp\.com/i }));
    expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled();
    expect(screen.getByText(/nothing to compare/i)).toBeInTheDocument();
  });

  it("says why a typo was not added rather than dropping it in silence", async () => {
    // A courtesy rather than a parser — `origins_in` decides what a company is, and this only
    // stops an obvious typo costing ninety seconds. What it must not do is swallow the input.
    watchPosts(about("https://basecamp.com"));
    const user = userEvent.setup();
    await arrive(user);

    await user.type(screen.getByLabelText(/add a company/i), "basecamp");
    await user.click(screen.getByRole("button", { name: /^add$/i }));

    expect(screen.getByRole("alert")).toHaveTextContent(/does not look like a domain/i);
    const set = screen.getByRole("region", { name: /companies in this report/i });
    expect(within(set).queryByText("basecamp")).not.toBeInTheDocument();
  });

  it("does not spend two analyses on two clicks", async () => {
    // The same guard the clarifying chips carry, for the same reason: a run takes ninety
    // seconds and a second click starts a second one before the first has said anything.
    const finished = about("https://basecamp.com", "https://linear.app");
    const sent: string[] = [];
    stubEventSource();
    let release: (() => void) | null = null;
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        if (init?.method === "POST") {
          sent.push(JSON.parse(String(init.body)).prompt as string);
          if (sent.length > 1) {
            // The re-run: hold it open so the page is caught mid-start.
            return new Promise<Response>((resolve) => {
              release = () =>
                resolve({
                  ok: true,
                  status: 201,
                  json: () => Promise.resolve(queued()),
                } as Response);
            });
          }
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : finished),
        } as Response);
      }),
    );

    const user = userEvent.setup();
    await arrive(user);
    await user.click(screen.getByRole("button", { name: /remove linear\.app/i }));
    await user.click(screen.getByRole("button", { name: /run this set/i }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled(),
    );
    await user.click(screen.getByRole("button", { name: /run this set/i }));
    expect(sent).toHaveLength(2);
    act(() => release?.());
  });

  it("shows the set the report on screen was built from, not the one before it", async () => {
    // The report's set is the truth and it changes underneath: a corrected run finishing
    // replaces the companies, and edits to the previous report's set are not edits to this one.
    // Which is the second thing the "still being read" guard buys — a correction restarts the
    // run, so the set is put away and read again rather than carried across.
    const first = about("https://basecamp.com", "https://linear.app");
    const second = about("https://basecamp.com", "https://notion.so");
    let serving = first;
    stubEventSource();
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) =>
        Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : serving),
        } as Response),
      ),
    );

    const user = userEvent.setup();
    await arrive(user);
    await user.click(screen.getByRole("button", { name: /remove linear\.app/i }));

    // The corrected run comes back covering somebody else entirely, which is the case that
    // matters: the reader's edit belonged to the report before this one.
    serving = second;
    await user.click(screen.getByRole("button", { name: /run this set/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));

    const set = await screen.findByRole("region", {
      name: /companies in this report/i,
    });
    await waitFor(() =>
      expect(within(set).getByText("notion.so")).toBeInTheDocument(),
    );
    expect(within(set).getByText("basecamp.com")).toBeInTheDocument();
  });

  it("refuses a company that is already in the set rather than sending it twice", async () => {
    // A second `https://basecamp.com` is a longer list, so the button lights up, and the run
    // deduplicates it back to the set on screen — ninety seconds to redraw the same report,
    // which is exactly what the "already on screen" guard exists to prevent.
    const sent = watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    await user.type(screen.getByLabelText(/add a company/i), "https://basecamp.com");
    await user.click(screen.getByRole("button", { name: /^add$/i }));

    expect(screen.getByRole("alert")).toHaveTextContent(/already in this set/i);
    const set = screen.getByRole("region", { name: /companies in this report/i });
    expect(within(set).getAllByText("basecamp.com")).toHaveLength(1);
    expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled();
    expect(sent).toEqual([IDEA]);
  });

  it("refuses the company in the spelling the page itself shows", async () => {
    // The chip says `basecamp.com` and the box says `example.com`, so the schemeless form is
    // the one the interface asks for — and it is the one that slipped past a byte comparison.
    const sent = watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    await user.type(screen.getByLabelText(/add a company/i), "basecamp.com");
    await user.click(screen.getByRole("button", { name: /^add$/i }));

    expect(screen.getByRole("alert")).toHaveTextContent(/already in this set/i);
    const set = screen.getByRole("region", { name: /companies in this report/i });
    expect(within(set).getAllByText("basecamp.com")).toHaveLength(1);
    expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled();
    expect(sent).toEqual([IDEA]);
  });

  it("will not re-run a set that only looks different", async () => {
    // Removing a company and putting it back in the spelling on the chip is the same set. The
    // change check compared strings, so it said yes and spent ninety seconds on the same report.
    const sent = watchPosts(about("https://basecamp.com", "https://linear.app"));
    const user = userEvent.setup();
    await arrive(user);

    await user.click(screen.getByRole("button", { name: /remove linear\.app/i }));
    await user.type(screen.getByLabelText(/add a company/i), "linear.app");
    await user.click(screen.getByRole("button", { name: /^add$/i }));

    expect(screen.getByRole("button", { name: /run this set/i })).toBeDisabled();
    expect(sent).toEqual([IDEA]);
  });

  it("is not offered while the report is still being read", async () => {
    // Mid-run the set is what the run is working through, and correcting it would be
    // correcting something that has not happened yet.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    expect(
      screen.queryByRole("button", { name: /run this set/i }),
    ).not.toBeInTheDocument();
  });
});

describe("the report as something to paste", () => {
  /**
   * A finished report, and whatever the context endpoint should answer with.
   *
   * **The clipboard is `userEvent`'s.** `userEvent.setup()` installs its own, so stubbing
   * `navigator` here would be quietly replaced a line later and the test would assert against
   * a stub nothing ever wrote to. Reading it back with `readText` exercises the real call.
   */
  function ready(markdown: string): { asked: string[] } {
    const asked: string[] = [];
    stubEventSource();
    const finished = {
      ...queued(),
      status: "complete" as const,
      report: {
        subject: "basecamp.com",
        searched_as: "basecamp.com",
        generated_at: "2026-08-05T00:00:00Z",
        model_id: "test",
        prompt_version: 1,
        subjects: ["https://basecamp.com"],
        sections: [
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ],
        sources: [],
      },
    };
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string, init?: RequestInit) => {
        if (String(url).endsWith("/context")) {
          asked.push(String(url));
          return Promise.resolve({
            ok: true,
            status: 200,
            text: () => Promise.resolve(markdown),
          } as Response);
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : finished),
        } as Response);
      }),
    );
    return { asked };
  }

  /** Take the clipboard away, the way a browser outside a secure context does. */
  function withNoClipboard(): void {
    Object.defineProperty(navigator, "clipboard", {
      value: undefined,
      configurable: true,
    });
  }

  async function arrive(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());
    act(() => FakeEventSource.last!.send("done", ""));
    expect(await screen.findByText(/Pro costs \$15/)).toBeInTheDocument();
  }

  it("puts the whole report on the clipboard, as the server wrote it", async () => {
    // **IDEA_ANALYSIS section 5**: we are not a worse chatbot, we are the evidence file a
    // chatbot cannot assemble. The bytes are the server's - a copy of that renderer here
    // would be a second opinion about what a source's standing is called.
    const md = "Here is a public-evidence report...\n\n# basecamp.com\n";
    ready(md);
    const user = userEvent.setup();
    await arrive(user);

    await user.click(screen.getByRole("button", { name: /copy as context/i }));
    expect(await screen.findByRole("button", { name: /^copied$/i })).toBeInTheDocument();
    expect(await navigator.clipboard.readText()).toBe(md);
  });

  it("is not offered until there is a whole report to paste", async () => {
    // Half a report handed to an assistant is answered from as confidently as all of it.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyze/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    expect(
      screen.queryByRole("button", { name: /copy as context/i }),
    ).not.toBeInTheDocument();
  });

  it("asks the server once, not once per failure", async () => {
    // The document is already in hand when the clipboard refuses; asking again would be
    // this button paying for its own error path.
    const { asked } = ready("# basecamp.com\n");
    const user = userEvent.setup();
    await arrive(user);
    withNoClipboard();

    await user.click(screen.getByRole("button", { name: /copy as context/i }));
    await screen.findByRole("alert");
    expect(asked).toHaveLength(1);
  });

  it("hands over the text when the browser will not allow a clipboard", async () => {
    // A button that silently does nothing is worse than one that admits it - and the reader
    // still wanted the document, so they still get it.
    const md = "# basecamp.com\n\n- Pro costs $15 [S1]\n";
    ready(md);
    const user = userEvent.setup();
    await arrive(user);
    withNoClipboard();

    await user.click(screen.getByRole("button", { name: /copy as context/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/select it and copy/i);
    expect(screen.getByLabelText(/the report as markdown/i)).toHaveValue(md);
  });

  it("says so when the report could not be assembled at all", async () => {
    // Nothing to offer, so nothing is offered - and the reader is told rather than left
    // looking at a button that did nothing.
    stubEventSource();
    const finished = {
      ...queued(),
      status: "complete" as const,
      report: {
        subject: "basecamp.com",
        searched_as: "basecamp.com",
        generated_at: "2026-08-05T00:00:00Z",
        model_id: "test",
        prompt_version: 1,
        subjects: ["https://basecamp.com"],
        sections: [
          section("pricing", "Pricing & packaging", "Pro costs $15", "https://basecamp.com"),
        ],
        sources: [],
      },
    };
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string, init?: RequestInit) => {
        if (String(url).endsWith("/context")) {
          return Promise.resolve({
            ok: false,
            status: 500,
            json: () => Promise.resolve({ error: "Something went wrong." }),
          } as Response);
        }
        return Promise.resolve({
          ok: true,
          status: init?.method === "POST" ? 201 : 200,
          json: () => Promise.resolve(init?.method === "POST" ? queued() : finished),
        } as Response);
      }),
    );
    const user = userEvent.setup();
    await arrive(user);

    await user.click(screen.getByRole("button", { name: /copy as context/i }));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.queryByLabelText(/the report as markdown/i)).not.toBeInTheDocument();
  });
});
