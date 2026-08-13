/**
 * The analyses this browser has started, so a reader can get back to them.
 *
 * **A run costs minutes and there are two a day.** Until this, the only record of one was the
 * URL in the address bar: close the tab and the work was gone, and the message telling somebody
 * they had used both of today's analyses did not say where either of them went. A reader
 * reported exactly that.
 *
 * # Why the browser remembers rather than the server
 *
 * There are no accounts. The server counts runs per address to enforce the cap, and an address
 * is not a person — reading a list back from it would mean handing one reader another reader's
 * questions whenever two share an office, a household or a NAT. **The only thing that knows
 * these are yours is the browser you started them in**, so that is what keeps the list.
 *
 * It travels no further: nothing here is sent anywhere, and `PRIVACY.md`'s account of what is
 * stored does not change because of it.
 *
 * # What is stored, and what is deliberately not
 *
 * The id, the words that were typed, and when. **Not the status and not the report** — those
 * live on the analysis, they change after this is written, and a second copy would be a stale
 * one. Following the link asks the server what actually happened.
 */

/** One analysis this browser started. */
export interface Remembered {
  readonly id: string;
  /** What was typed. Shown, so a list of ids is a list a person can read. */
  readonly prompt: string;
  /** When it was started, ISO-8601, so the list can be ordered without trusting its order. */
  readonly at: string;
}

const KEY = "landscape.analyses";

/**
 * How many are kept.
 *
 * Two a day, so twenty is a fortnight of somebody using every one of them. Past that the list
 * stops being a way back to your work and becomes an archive nobody reads — and unbounded
 * growth in a store with a hard size limit fails by throwing away *everything*, which is worse
 * than forgetting the oldest.
 */
export const KEPT = 20;

/**
 * Read the list, newest first.
 *
 * **Never throws.** `localStorage` is absent in some privacy modes and unwritable in others,
 * and a way back to your work must not be able to take the page down with it — the same rule
 * `getExamples` follows for the same reason.
 */
export function recall(): readonly Remembered[] {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    // Every field of every entry. A shape checked at the edge and trusted at the leaf is
    // unchecked, and this one is parsed from something a person can edit by hand.
    return parsed
      .filter(isRemembered)
      .sort((a, b) => b.at.localeCompare(a.at))
      .slice(0, KEPT);
  } catch {
    return [];
  }
}

/**
 * Put one at the top, and drop any earlier record of the same analysis.
 *
 * **Idempotent on the id**, because a reader who reloads a run they started should not find it
 * listed twice — and because re-running the same words is a *different* analysis with its own
 * id, which is the case that must still produce two rows.
 */
export function remember(entry: Remembered): void {
  try {
    const kept = [entry, ...recall().filter((held) => held.id !== entry.id)].slice(
      0,
      KEPT,
    );
    window.localStorage.setItem(KEY, JSON.stringify(kept));
  } catch {
    // A reader whose browser will not store this still gets the analysis they asked for.
  }
}

/** Forget all of them. The reader's list is the reader's to clear. */
export function forget(): void {
  try {
    window.localStorage.removeItem(KEY);
  } catch {
    // Nothing to do: if it cannot be written it was almost certainly never read either.
  }
}

function isRemembered(value: unknown): value is Remembered {
  if (typeof value !== "object" || value === null) return false;
  const held = value as Record<string, unknown>;
  return (
    typeof held.id === "string" &&
    held.id !== "" &&
    typeof held.prompt === "string" &&
    typeof held.at === "string"
  );
}
