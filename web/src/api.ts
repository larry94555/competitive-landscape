/**
 * The API client.
 *
 * These types are hand-written for now and must match `landscape-core`. They are
 * generated from that crate's JSON Schema once the schema settles — a duplicated shape is
 * a known cost, recorded here rather than forgotten.
 */

export type AnalysisStatus = "queued" | "running" | "complete" | "failed";

export type SectionStatus = "populated" | "partial" | "not_found_in_public_sources";

export interface Claim {
  readonly text: string;
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
  readonly sections: readonly Section[];
  readonly sources: readonly unknown[];
}

export interface Analysis {
  readonly id: string;
  readonly prompt: string;
  readonly status: AnalysisStatus;
  readonly created_at: string;
  readonly report: Report | null;
}

/** What the server sends when it refuses a request. */
export interface ApiErrorBody {
  readonly error: string;
  readonly remedy?: string;
}

/** An error carrying a message already written for a person to read. */
export class ApiError extends Error {
  readonly remedy: string | undefined;

  constructor(message: string, remedy?: string) {
    super(message);
    this.name = "ApiError";
    this.remedy = remedy;
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

export function isTerminal(status: AnalysisStatus): boolean {
  return status === "complete" || status === "failed";
}
