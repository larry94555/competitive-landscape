import { afterEach, describe, expect, it, vi } from "vitest";

import { forget, KEPT, recall, remember, type Remembered } from "./history";

const KEY = "landscape.analyses";

afterEach(() => {
  // **Unstub before clearing.** A test that replaces `localStorage` with something that
  // refuses leaves the replacement in place until this runs — so clearing first throws, this
  // hook dies, and every test after it inherits the broken store. Which is what happened.
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

/** One entry, `n` minutes ago, so order is a fact rather than an argument. */
function ran(id: string, minutesAgo: number): Remembered {
  return {
    id,
    prompt: `idea ${id}`,
    at: new Date(Date.UTC(2026, 7, 12, 12, 0) - minutesAgo * 60_000).toISOString(),
  };
}

describe("what this browser has run", () => {
  it("keeps the newest first, whatever order they were written in", () => {
    remember(ran("older", 60));
    remember(ran("newest", 1));
    remember(ran("middle", 30));

    expect(recall().map((e) => e.id)).toEqual(["newest", "middle", "older"]);
  });

  it("lists one analysis once, however many times it is remembered", () => {
    // A reader who reloads a run they started should not find it twice. Re-running the same
    // words is a *different* analysis with its own id, and that case must still make two rows.
    remember(ran("abc", 5));
    remember(ran("abc", 1));

    expect(recall().map((e) => e.id)).toEqual(["abc"]);
    expect(recall()).toHaveLength(1);
  });

  it("stops at the cap rather than growing until the store refuses everything", () => {
    // **Unbounded growth in a store with a hard size limit fails by throwing away all of it**,
    // which is worse than forgetting the oldest. Two analyses a day, so the cap is a fortnight
    // of somebody using every one.
    for (let i = 0; i < KEPT + 5; i += 1) remember(ran(`run-${String(i)}`, KEPT + 5 - i));

    const held = recall();
    expect(held).toHaveLength(KEPT);
    expect(held[0]?.id).toBe(`run-${String(KEPT + 4)}`);
    // And the store itself is bounded, not just the read - otherwise it grows for ever and
    // this test would pass over a `localStorage` that eventually throws.
    const raw: unknown = JSON.parse(window.localStorage.getItem(KEY) ?? "[]");
    expect(Array.isArray(raw) && raw.length).toBe(KEPT);
  });

  it("drops entries that are not the shape it wrote", () => {
    // Parsed from something a person can edit by hand. A shape checked at the edge and
    // trusted at the leaf is unchecked.
    window.localStorage.setItem(
      KEY,
      JSON.stringify([
        { id: "good", prompt: "an idea", at: "2026-08-12T12:00:00.000Z" },
        { id: "", prompt: "empty id", at: "2026-08-12T12:00:00.000Z" },
        { id: "no-prompt", at: "2026-08-12T12:00:00.000Z" },
        "not an object",
        null,
      ]),
    );

    expect(recall().map((e) => e.id)).toEqual(["good"]);
  });

  it("survives a store that is not there at all", () => {
    // Absent in some privacy modes and unwritable in others. A way back to your work must not
    // be able to take the page down with it.
    const denied = (): never => {
      throw new Error("denied");
    };
    vi.stubGlobal("localStorage", {
      getItem: denied,
      setItem: denied,
      removeItem: denied,
      // `clear` too, because the teardown uses it and a stub missing one method is a stub
      // that breaks the next test rather than this one.
      clear: () => undefined,
    });

    expect(recall()).toEqual([]);
    expect(() => remember(ran("abc", 1))).not.toThrow();
    expect(() => forget()).not.toThrow();
  });

  it("reads nothing out of something that is not a list", () => {
    window.localStorage.setItem(KEY, JSON.stringify({ id: "not a list" }));
    expect(recall()).toEqual([]);
    window.localStorage.setItem(KEY, "{ this is not json");
    expect(recall()).toEqual([]);
  });

  it("forgets all of them when asked", () => {
    remember(ran("abc", 1));
    forget();
    expect(recall()).toEqual([]);
    expect(window.localStorage.getItem(KEY)).toBeNull();
  });
});
