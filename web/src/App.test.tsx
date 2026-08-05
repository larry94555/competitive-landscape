import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import type { Analysis } from "./api";

const IDEA = "an app that helps small farms sell to local restaurants";

function queued(): Analysis {
  return {
    id: "abc",
    prompt: IDEA,
    status: "queued",
    created_at: "2026-08-03T00:00:00Z",
    report: null,
    failure: null,
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
      () =>
        new Promise((res) => {
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));

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
    // than a testing artefact.
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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

    await user.click(screen.getByRole("button", { name: /analyse/i }));

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
    await user.click(screen.getByRole("button", { name: /analyse/i }));

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
    await user.click(screen.getByRole("button", { name: /analyse/i }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("9f2c11ab7d04");
  });

  it("keeps what was typed when the failure was ours", async () => {
    // Doubly true here: they did nothing wrong, so making them retype would be perverse.
    stubBreaking();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyse/i }));

    await screen.findByRole("alert");
    expect(box().value).toBe(IDEA);
  });

  it("will not submit an empty box", async () => {
    stubAccepting();
    render(<App />);
    expect(screen.getByRole("button", { name: /analyse/i })).toBeDisabled();
  });

  it("shows the analysis after it is accepted", async () => {
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);

    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyse/i }));

    // The submitted idea is still visible even though the box is empty — clearing the
    // input must not mean losing sight of what was asked.
    expect(await screen.findByText(IDEA)).toBeInTheDocument();
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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

  it("does not repeat one company down a report about one company", async () => {
    // The other half. A name against every line of a single-company report is noise, and
    // noise is what teaches a reader to stop reading the labels that matter.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));

    expect(await screen.findByText(/Reading the first pages/)).toBeInTheDocument();
  });

  it("replaces a section as it grows rather than repeating it", async () => {
    // Watching a real run showed why this matters: a section sent once said "1 item" and
    // sat there for two minutes while eight more were read, which reads as finished.
    stubAccepting();
    const user = userEvent.setup();
    render(<App />);
    await user.type(box(), IDEA);
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("done", ""));

    expect(
      await screen.findByText(/could not work out which company/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/basecamp\.com/)).toBeInTheDocument();
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
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
    await user.click(screen.getByRole("button", { name: /analyse/i }));
    await waitFor(() => expect(FakeEventSource.last).not.toBeNull());

    act(() => FakeEventSource.last!.send("status", "running"));
    expect(await screen.findByText(/Reading public web pages/)).toBeInTheDocument();

    act(() => FakeEventSource.last!.onerror?.());

    expect(await screen.findByText("Done.")).toBeInTheDocument();
    expect(screen.queryByText(/Still reading|Reading the first pages/)).toBeNull();
  });
});
