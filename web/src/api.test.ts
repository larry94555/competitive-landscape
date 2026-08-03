import { describe, expect, it, vi, afterEach } from "vitest";

import { ApiError, createAnalysis, getAnalysis, isTerminal } from "./api";

/** A `fetch` that returns one canned response. */
function stubFetch(status: number, body: unknown): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: status >= 200 && status < 300,
        status,
        json: () => Promise.resolve(body),
      } as Response),
    ),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("isTerminal", () => {
  it("treats only finished states as terminal", () => {
    expect(isTerminal("queued")).toBe(false);
    expect(isTerminal("running")).toBe(false);
    expect(isTerminal("complete")).toBe(true);
    expect(isTerminal("failed")).toBe(true);
  });
});

describe("createAnalysis", () => {
  it("returns the analysis on success", async () => {
    stubFetch(201, { id: "abc", prompt: "an idea", status: "queued", report: null });
    const analysis = await createAnalysis("an idea worth checking");
    expect(analysis.status).toBe("queued");
  });

  it("surfaces the server's message and remedy, not a generic one", async () => {
    // The API writes rejections for people to read. Replacing them with our own wording
    // would throw away the specific thing that went wrong.
    stubFetch(400, {
      error: "a prompt must contain at least 8 characters, got 5",
      remedy: "Edit what you typed and try again.",
    });

    await expect(createAnalysis("a crm")).rejects.toThrow(/at least 8 characters/);
  });

  it("still fails usefully when the error body is not JSON", async () => {
    // A proxy or a dead server returns HTML. The reader should get something actionable
    // rather than a JSON parse error from our own code.
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        Promise.resolve({
          ok: false,
          status: 502,
          json: () => Promise.reject(new Error("not json")),
        } as unknown as Response),
      ),
    );

    const failure = await createAnalysis("an idea worth checking").catch(
      (e: unknown) => e,
    );
    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).remedy).toBeTruthy();
  });
});

describe("getAnalysis", () => {
  it("requests the id it was given", async () => {
    stubFetch(200, { id: "xyz", prompt: "p", status: "complete", report: null });
    await getAnalysis("xyz");
    expect(fetch).toHaveBeenCalledWith("/api/analyses/xyz");
  });
});
