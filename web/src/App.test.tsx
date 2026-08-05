import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

function section(key: string, title: string, claim: string) {
  return {
    key,
    title,
    status: "populated" as const,
    claims: [
      {
        text: claim,
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

afterEach(() => {
  vi.unstubAllGlobals();
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
