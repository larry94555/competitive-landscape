import { render, screen, waitFor } from "@testing-library/react";
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
  };
}

/** A `fetch` that accepts the POST and reports the analysis complete thereafter. */
function stubAccepting(): void {
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

/** A `fetch` that fails the way the API fails when something breaks at our end. */
function stubBreaking(): void {
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
