# Landscape — Technical Architecture & Stack

> Section **B** of the roadmap. See [ROADMAP.md](ROADMAP.md) for the index and phasing.
>
> **Unfamiliar with a tool named here?** [ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md)
> explains every technology in this document — what it is, what the alternatives were, why
> this one was chosen, and what the choice costs.
>
> Status: **proposed design, not yet implemented.** Every performance number in this
> document is a *design target* that must be confirmed by the Phase 0 benchmark harness
> before it is treated as fact.

---

## 1. System overview

```
                      ┌───────────────────────────────────────────┐
   Browser            │            Single host (start)            │
 ┌──────────┐  HTTPS  │  ┌─────────────────────────────────────┐  │
 │ React SPA│◄───────►│  │ Caddy (TLS, static assets, gzip)    │  │
 │  (TS)    │  SSE    │  └──────────────┬──────────────────────┘  │
 └──────────┘         │                 │ reverse proxy           │
                      │  ┌──────────────▼──────────────────────┐  │
                      │  │ landscape-api  (Rust / axum / tokio)│  │
                      │  │  · HTTP + SSE                       │  │
                      │  │  · auth, billing, rate limits       │  │
                      │  │  · analysis orchestrator            │  │
                      │  │  · KB / support                     │  │
                      │  └──┬────────┬────────┬────────┬───────┘  │
                      │     │        │        │        │          │
                      │  ┌──▼───┐ ┌──▼────┐ ┌─▼─────┐ ┌▼───────┐  │
                      │  │Postgr│ │ Redis │ │Fetcher│ │llama-  │  │
                      │  │ es   │ │(cache,│ │pool   │ │server  │  │
                      │  │(+jobs│ │ rate) │ │(reqwe-│ │(llama. │  │
                      │  │ queue│ └───────┘ │ st)   │ │ cpp)   │  │
                      │  │ +FTS)│           └───┬───┘ └────────┘  │
                      │  └──────┘               │                 │
                      │  ┌──────────────────┐   │   ┌──────────┐  │
                      │  │ landscape-worker │   │   │ SearXNG  │  │
                      │  │ (same binary,    │   │   │ (self-   │  │
                      │  │  --role worker)  │   │   │  hosted) │  │
                      │  └──────────────────┘   │   └──────────┘  │
                      └─────────────────────────┼─────────────────┘
                                                ▼
                                  Public web: sites, pricing pages,
                                  changelogs, review sites, news,
                                  job boards, ad libraries
```

**Design stance:** one Rust binary, one database, one inference server, one host. Every
piece that is not the analysis engine is deliberately boring. Complexity budget is spent
almost entirely on grounding quality and inference throughput.

---

## 2. Frontend — TypeScript + React

### 2.1 Stack

| Concern | Choice | Why |
|---|---|---|
| Build | **Vite** | Fastest DX, trivial config, first-class TS. |
| Language | **TypeScript, `strict: true`** + `noUncheckedIndexedAccess` | Report schema is the product contract; types must be load-bearing. |
| Framework | **React 19**, function components + hooks | Constraint. `use()` + Suspense for streamed data. |
| Routing | **TanStack Router** (or React Router v7) | Type-safe params; the app has ~12 routes, not 100. |
| Server state | **TanStack Query** | Caching, retries, dedup, invalidation for KB/watchlist/billing. |
| Client state | **Zustand** (one small store) + URL state | No Redux. The only real client state is the composer draft and stream buffer. |
| Styling | **Tailwind CSS** + a handful of local primitives (Radix UI for dialog/popover/toast) | Fast, accessible, no design-system tax. |
| Forms | Native + **Zod** | Zod schemas are shared conceptually with the Rust `serde` structs (kept in sync by codegen, §2.5). |
| Charts | None in v1 | Reports are text + tables. Do not add a chart library to look serious. |
| Testing | Vitest + Testing Library + Playwright | Playwright covers the three critical flows end to end. |

### 2.2 Route map

```
/                       Composer (the whole product; also the landing page)
/a/:analysisId          Report view (streamed, then static; shareable, SEO-indexable)
/a/:analysisId/pdf      Triggers PDF download (server-rendered)
/watch                  Watchlist management
/watch/:watchId         Change history for one watched target
/account                Profile, usage meter, plan, billing portal link
/pricing                Plans
/help                   Public KB / Q&A index + search
/help/:slug             One question thread
/help/new               Ask a question
/legal/{terms,privacy,disclaimer,data-sources}
/admin/*                Operator console (role-gated)
```

There is no `/dashboard`, no onboarding wizard, no empty-state-with-six-cards. `/` *is*
the app: a single textarea, focused on load.

### 2.3 Component structure

```
src/
  app/
    router.tsx
    providers.tsx            QueryClient, toast, theme, error boundary
  features/
    composer/
      Composer.tsx           the single textarea + submit
      ExampleChips.tsx       3 one-click example inputs
      ClarifyPanel.tsx       0–3 progressive questions, chip-answerable
      useAnalysisStream.ts   EventSource wrapper → typed events
    report/
      ReportView.tsx         orchestrates sections
      sections/
        PositioningSection.tsx
        PricingSection.tsx
        FeaturesSection.tsx
        RecentChangesSection.tsx
        SentimentSection.tsx
        SwotSection.tsx
        SourcesSection.tsx
      SectionSkeleton.tsx    streaming placeholder
      Citation.tsx           [S3] chip → hover card with URL + fetched-at
      NotFoundInPublic.tsx   the standard "no public source found" treatment
      ReportActions.tsx      PDF, share, watch, feedback
      FeedbackWidget.tsx     👍/👎 + optional one-line reason, per section
    watch/
      WatchList.tsx, WatchCreateSheet.tsx, ChangeTimeline.tsx, AlertPreview.tsx
    billing/
      PlanCards.tsx, UsageMeter.tsx, UpgradeDialog.tsx, PortalButton.tsx
    kb/
      KbSearch.tsx, KbThread.tsx, KbComposer.tsx, SlashMenu.tsx, TagFilter.tsx
    auth/
      MagicLinkForm.tsx, SessionGate.tsx
  shared/
    api/                     generated client + typed fetch wrapper
    types/report.ts          GENERATED from Rust — do not hand-edit
    ui/                      Button, Input, Card, Sheet, Tooltip, Toast
    lib/                     formatting, dates, analytics
```

### 2.4 Streaming the report (the core UX mechanism)

Local inference is slower than a hosted frontier API. The frontend hides that with
**progressive section streaming**, not spinners.

- `POST /v1/analyses` returns `{ analysisId }` immediately (202).
- `GET /v1/analyses/:id/events` is a **Server-Sent Events** stream. SSE, not WebSockets:
  unidirectional, survives proxies, trivially resumable with `Last-Event-ID`, one fewer
  protocol in the Rust stack.

Event types (discriminated union, exhaustively matched in TS):

```ts
type AnalysisEvent =
  | { type: 'stage';    stage: 'resolving' | 'fetching' | 'reading' | 'writing'; detail?: string }
  | { type: 'source';   source: SourceRef }                    // sources appear as they land
  | { type: 'section';  key: SectionKey; status: 'started' }
  | { type: 'delta';    key: SectionKey; text: string }        // token deltas
  | { type: 'section';  key: SectionKey; status: 'done'; data: SectionPayload }
  | { type: 'done';     analysis: AnalysisSummary }
  | { type: 'error';    code: string; message: string; retryable: boolean };
```

Perceived-latency ladder — the user always sees motion:

| t (target) | On screen |
|---|---|
| 0.0s | Input collapses into a header chip; stage rail appears. |
| 0.5–3s | Source cards stream in as fetches resolve (favicon + domain + fetched-at). |
| 4–8s | **Positioning** section begins streaming tokens. |
| 8–18s | Pricing → Features → Recent changes → Sentiment stream in order. |
| 15–25s | SWOT + Sources complete; PDF button enables. |

Reconnect: the SPA stores `lastEventId`; the server replays from an in-memory ring buffer
per analysis (and from Postgres once the analysis is complete), so a dropped connection or
a phone locking mid-run never loses a report.

### 2.5 Type safety across the Rust/TS boundary

The report schema is defined **once**, in Rust, and everything else is generated:

1. Rust structs annotated with `serde` + [`schemars`] → JSON Schema.
2. JSON Schema → `src/shared/types/report.ts` via `json-schema-to-typescript` in CI.
3. The *same* JSON Schema → a **GBNF grammar** for llama.cpp constrained decoding (§4.5).
4. CI fails if generated TS drifts from committed TS.

One schema drives the API contract, the UI types, and the model's decoding constraints.
This is the single highest-leverage decision in the codebase.

### 2.6 PDF download handling

PDF is generated **server-side** (§7). The client:

- `GET /v1/analyses/:id/pdf?variant=exec|full` → `Content-Disposition: attachment`.
- The button is a plain `<a download>` — no blob juggling, works on mobile Safari.
- First request renders and caches to object storage; later requests 302 to a signed URL.
- The exec PDF is pre-warmed in the background the moment an analysis completes, so the
  click is instant ~90% of the time.

---

## 3. Backend — Rust

### 3.1 Stack

| Concern | Crate | Notes |
|---|---|---|
| Runtime | `tokio` (multi-thread) | Inference is out-of-process, so the runtime stays free for I/O. |
| HTTP | **`axum`** 0.8 + `tower` / `tower-http` | Tower middleware gives tracing, timeout, compression, CORS, rate-limit layering for free. Actix is fine; axum wins on ecosystem cohesion with tokio/tracing. |
| SSE | `axum::response::Sse` over a `tokio::sync::broadcast` per analysis | Ring-buffered for replay. |
| DB | **`sqlx`** (Postgres, compile-time-checked queries) | No ORM. Migrations via `sqlx migrate`. |
| Cache / rate limit | `redis` (`fred` or `redis-rs`) | Also holds fetch cache metadata and the anonymous quota counters. |
| Jobs | **Postgres-backed queue**, `FOR UPDATE SKIP LOCKED`, hand-rolled (~200 LOC) | One less moving part than Redis-queue crates; transactional with business writes; survives restarts. Revisit `apalis` only if job types exceed ~15. |
| HTTP client | `reqwest` (rustls) + `governor` for per-domain politeness | |
| HTML → text | `scraper` + `readability`-style main-content extraction, then `htmd` to Markdown | Markdown is what the model reads. |
| Robots | `texting_robots` | Enforced, cached per host for 24h. |
| Diffing | `similar` | Line/word diffs for change detection. |
| Auth | Magic link + `argon2` for optional passwords; sessions as signed cookies (`tower-cookies`), JWT only for the future API | |
| Payments | `async-stripe` | |
| Email | Provider HTTP API (Postmark or Resend) via `reqwest` | No SMTP. |
| PDF | **`typst`** as a library | See §7. |
| Observability | `tracing` + `tracing-subscriber` + OpenTelemetry → Grafana/Tempo; `metrics` + Prometheus exporter | |
| Config | `figment` (env + TOML) | |
| Errors | `thiserror` internally, one `AppError` → typed JSON problem responses | |
| Tests | `cargo test` + `wiremock` for fetch fixtures + `sqlx::test` | |

**One binary, three roles**: `landscape --role api|worker|all`. Early on, `all`. Splitting
later is a flag change, not a refactor.

### 3.2 Module layout

```
crates/
  landscape-core/      domain types, report schema (schemars), errors
  landscape-db/        sqlx queries, migrations, job queue
  landscape-fetch/     robots, politeness, cache, extraction, normalization
  landscape-search/    SearXNG + Brave adapters, source discovery & ranking
  landscape-llm/       llama-server client, grammars, prompts, slot pool, budgets
  landscape-analyze/   the orchestrator: plan → fetch → read → synthesize → verify
  landscape-watch/     change detection, importance scoring, alert composition
  landscape-pdf/       typst templates + render
  landscape-billing/   Stripe, plans, entitlements
  landscape-kb/        support KB, search, moderation
  landscape-api/       axum routers, auth, rate limits, SSE
  landscape-worker/    job handlers
  landscape/           bin: wires roles together
```

### 3.3 Data model (Postgres 16)

```sql
users(id, email, email_verified_at, created_at, plan, stripe_customer_id, role)
sessions(id, user_id, expires_at, ip_hash, ua_hash)
plans(key, analyses_per_month, watches, watch_interval_minutes, price_cents)
usage_counters(subject_id, subject_kind /*user|ip*/, period, analyses_used, updated_at)

analyses(id, user_id NULL, anon_key_hash NULL, input_text, resolved_subject jsonb,
         status, model_id, prompt_version, started_at, finished_at,
         latency_ms, tokens_in, tokens_out, cost_compute_ms, share_slug, visibility)
analysis_sections(analysis_id, key, payload jsonb, tokens_out, verify_status)
sources(id, url, canonical_url, host, first_seen_at)
analysis_sources(analysis_id, source_id, label /*S1..Sn*/, fetched_at,
                 content_hash, http_status, extraction_quality, trust_tier)
claims(id, analysis_id, section_key, text, evidence_quote, source_label,
       verify_status /*verified|weak|dropped*/, verifier_notes)

fetch_cache(url_hash, url, fetched_at, expires_at, http_status, etag, last_modified,
            content_hash, extracted_md_path, bytes)

watches(id, user_id, target_url, label, kind /*pricing|changelog|homepage|news*/,
        interval_minutes, enabled, created_at, last_checked_at, last_change_at)
watch_snapshots(id, watch_id, taken_at, content_hash, extracted_md_path, simhash)
watch_changes(id, watch_id, from_snapshot, to_snapshot, diff_path,
              importance_score, importance_label, summary_md, notified_at, feedback)

jobs(id, kind, payload jsonb, run_at, attempts, max_attempts, locked_by, locked_at,
     status, last_error, priority)

kb_threads(id, slug, title, body_md, author_user_id NULL, author_display,
           status /*open|answered|official*/, tags text[], views, created_at,
           search_tsv tsvector)
kb_replies(id, thread_id, body_md, author_user_id, is_official, votes, created_at)
kb_flags(id, thread_id NULL, reply_id NULL, reason, created_at)

feedback(id, analysis_id NULL, section_key NULL, rating, note, user_id NULL, created_at)
events(id, name, user_id NULL, anon_key_hash NULL, props jsonb, created_at)
```

Postgres full-text search (`tsvector` + `pg_trgm`) powers KB search. No Elasticsearch,
no vector DB in v1 — the corpus is hundreds of threads, not millions.

### 3.4 Background jobs

| Job | Trigger | Notes |
|---|---|---|
| `analysis.run` | API enqueue | The orchestrator. Priority by plan tier. |
| `pdf.render` | analysis complete | Pre-warms exec PDF. |
| `watch.check` | scheduler tick | One job per due watch; jittered. |
| `watch.notify` | importance ≥ threshold | Batches per user per digest window. |
| `cache.evict` | hourly | LRU over `fetch_cache` byte budget. |
| `eval.regression` | nightly / on prompt change | Runs the golden set (§ Quality doc). |
| `stripe.reconcile` | hourly | Repairs missed webhooks. |
| `kb.reindex` | on write | tsvector refresh. |

Scheduler: a single `tokio` interval task holding a Postgres advisory lock, enqueuing due
work. No external cron.

**Concurrency control is the whole game.** A global `tokio::sync::Semaphore` sized to the
number of llama-server slots gates every LLM call. Analyses queue rather than thrash. Queue
position is streamed to the client ("2 ahead of you, ~14s") — honest, and better than a
spinner.

---

## 4. Local LLM — llama.cpp strategy

### 4.1 Integration approach: sidecar `llama-server`, not in-process bindings

Run upstream **`llama-server`** as a separate supervised process; the Rust backend talks to
it over localhost HTTP.

Why sidecar over `llama-cpp-2` / `llama_cpp_rs` in-process bindings:

- **Continuous batching + slots** are implemented and maintained upstream. Reimplementing
  scheduling over raw bindings is where local-LLM projects lose months.
- **Prefix/KV cache reuse** across requests comes free with `--slot-save-path` and
  slot-aware routing.
- Crashes, OOM, and CUDA faults are isolated from the API process. A segfault in ggml
  restarts a sidecar; it does not drop user sessions.
- Model upgrades are a process restart, not a redeploy.
- llama.cpp moves fast; tracking a binary release is cheaper than tracking an FFI crate.

Cost: one localhost hop (~0.2ms) and JSON serialization. Irrelevant next to generation time.

The Rust side is a thin, well-tested client:

```rust
pub struct LlmClient { base: Url, http: reqwest::Client, slots: Arc<Semaphore>, }

pub struct GenRequest {
    pub prompt: String,
    pub grammar: Option<Grammar>,     // GBNF, from JSON Schema
    pub max_tokens: u32,
    pub temperature: f32,             // 0.1–0.3 for extraction, 0.4 for prose
    pub stop: Vec<String>,
    pub cache_prompt: bool,           // true — shared system prefix
    pub deadline: Instant,            // hard budget; cancel on expiry
}

impl LlmClient {
    pub async fn generate(&self, r: GenRequest) -> Result<GenOutput>;
    pub fn stream(&self, r: GenRequest) -> impl Stream<Item = Result<Token>>;
    pub async fn health(&self) -> Health;   // slots idle/busy, ctx, model id
}
```

Every call carries a **deadline** and a **token budget**. Nothing runs unbounded.

Escape hatch: if in-process ever becomes worth it (e.g. embedding a tiny reranker),
`landscape-llm` already isolates it behind this trait.

### 4.2 Model selection

Two model roles, both llama.cpp/GGUF:

| Role | Requirement | Recommended | Alternates |
|---|---|---|---|
| **Extractor** (per-source structured reading, high volume, must be fast) | Strong instruction-following on short contexts, reliable JSON under grammar | **Qwen3-4B** or **Qwen3-8B**, Q4_K_M | Llama 3.1 8B, Gemma 3 4B/12B |
| **Synthesizer** (final report prose, one call per analysis, quality-critical) | Long context (32k+), good summarization, low hallucination when grounded | **Qwen3-14B** Q4_K_M *(GPU)* / **Qwen3-8B** Q4_K_M *(CPU-only)* | Mistral Small 3.x 24B, Gemma 3 27B, Qwen3-30B-A3B (MoE — very strong tok/s if VRAM allows) |

Selection principles, not brand loyalty:

- **Licensing gate first.** Apache-2.0 / permissive preferred (Qwen3, Mistral Small).
  Read the license before the benchmark — Llama and Gemma carry use restrictions that
  matter for a commercial SaaS.
- **Long-context quality** matters more than raw MMLU. The job is "read 8 pages, do not
  invent." Evaluate on *our* golden set (see Quality doc), never on leaderboards.
- **MoE is the sleeper pick.** Qwen3-30B-A3B activates ~3B params/token: near-14B quality
  at near-4B speed, if you have the VRAM/RAM for the full weights.
- **Model choice is a config value**, not a code change. `MODEL_SYNTH=...gguf`. Swapping
  models must be a one-line change plus an eval run.

### 4.3 Quantization strategy

| Level | Use |
|---|---|
| **Q4_K_M** | **Default for both roles.** Best quality/size knee; ~4.8 bits/weight effective. |
| Q5_K_M | Synthesizer, if VRAM allows and eval shows a real gain. Measure, don't assume. |
| IQ4_XS | CPU-only fallback when RAM-bound; slightly better ppl than Q4_0 at similar size. |
| Q8_0 / F16 | **Reference only** — used in eval to measure the quality cost of quantization. |
| Q3_* and below | Rejected. Structured extraction degrades sharply; false confidence is the exact failure mode this product cannot afford. |

Keep **KV cache at F16** initially. `q8_0` KV quantization roughly halves KV memory and is
usually acceptable, but it must be validated against the golden set before shipping — KV
quantization damage shows up precisely in long-context faithfulness.

### 4.4 Hardware assumptions & latency budget

Three deployment rungs. The product ships on Rung 1 and is *designed* for Rung 2.

**Rung 0 — dev laptop.** Apple Silicon (M-series, ≥24GB) via Metal, or any dev box.
Fine for building; not a latency reference.

**Rung 1 — CPU-only VPS/dedicated (launch, €50–70/mo).**
Hetzner AX52-class: Ryzen 7 7700 (8c/16t), 64GB DDR5, NVMe.
Expected order of magnitude (validate in Phase 0):
- Qwen3-8B Q4_K_M: prompt processing ~120–250 tok/s, generation ~9–14 tok/s.
- A 1,200-token synthesis = **85–130s**. Too slow for the 15–25s SLO on its own.

Therefore Rung 1 ships with these compensations, all of which are permanent
architecture improvements, not hacks:
1. **Map-reduce with tiny outputs.** Per-source extraction emits ~120–200 tokens of
   structured JSON, not prose. These run concurrently across slots.
2. **Extractive-first synthesis.** The final call assembles mostly-already-written
   fragments; target ~500–700 generated tokens, not 1,500.
3. **Section-parallel generation.** Independent sections (Pricing, Features, Sentiment)
   are separate small grammar-constrained calls across slots, not one monolith.
4. **Streaming UX** so time-to-first-content is 4–8s.
5. Extractor role uses **Qwen3-4B** on Rung 1.
6. Honest queue-position display under load.

**Rung 2 — single GPU box (target, €180–250/mo).**
Hetzner GEX44 (RTX 4000 SFF Ada, 20GB) or an RTX 4090/5090 dedicated.
- Qwen3-14B Q4_K_M fully offloaded: generation ~45–75 tok/s, prompt ~2,000+ tok/s.
- 1,200-token synthesis ≈ **16–27s** single-stream; with section parallelism across
  4 slots, **p50 well inside the 15–25s target**.
- Flash attention (`-fa`), `--cont-batching`, `--parallel 4`, and optional
  **speculative decoding** (`-md` with a 0.5–1.5B draft model) for another 1.3–2×.

**Rung 3 — scale-out.** Multiple GPU workers behind the same Postgres job queue; the
Rust API is already stateless with respect to inference. Add a second box, register it in
a `inference_nodes` table, and route by least-loaded slots.

**Move to Rung 2 when** p95 analysis latency exceeds 45s for two consecutive days, or
queue depth regularly exceeds 3. Not before — €120/mo matters at zero revenue.

### 4.5 Constrained decoding (non-negotiable)

llama.cpp supports **GBNF grammars** and JSON-Schema-derived grammars natively.
Every structured call passes a grammar. Consequences:

- Parse failures approach zero; no "retry until it's valid JSON" loop burning local compute.
- Enums are enforceable, which is how `"not_found_in_public_sources"` becomes a *reliable*
  output rather than a hopeful instruction.
- Citation fields become mandatory: a claim object cannot be emitted without a
  `source_label` and an `evidence_quote`.

Example (abridged) claim schema:

```json
{
  "type": "object",
  "required": ["text", "source_label", "evidence_quote", "confidence"],
  "properties": {
    "text":           { "type": "string", "maxLength": 400 },
    "source_label":   { "type": "string", "pattern": "^S[0-9]{1,2}$" },
    "evidence_quote": { "type": "string", "minLength": 12, "maxLength": 300 },
    "confidence":     { "enum": ["high", "medium", "low"] }
  },
  "additionalProperties": false
}
```

Grammars are compiled once at startup and cached; grammar compilation is not free and must
not sit in the request path.

### 4.6 Prompting & context discipline

- **System prefix is byte-identical across all calls of a role** so llama.cpp prefix-cache
  hits. Variable content goes strictly at the end. This is worth seconds per request.
- Context assembly is budgeted: hard cap per source (e.g. 3,000 tokens), hard cap total
  (e.g. 24,000), with deterministic truncation that keeps pricing tables and headings.
- **No world knowledge permitted.** Prompts state that the model may only use text present
  in the provided sources, and the verifier (Quality doc §3) enforces it mechanically.
- Few-shot examples are versioned artifacts (`prompts/v3/synthesize.md`) with the version
  recorded on every `analyses` row, so quality regressions are attributable.

### 4.7 Queueing & batching

```
request → global Semaphore(N_SLOTS) → llama-server slot
              ↑ fair-ish: paid tier gets a reserved slot subset
```

- `--parallel 4` on Rung 1, `--parallel 4..8` on Rung 2.
- Extraction calls are naturally batchable and dominate slot demand; they are issued as a
  `FuturesUnordered` bounded by the semaphore.
- Reserve **one slot** exclusively for interactive traffic so watch-checking never starves
  a live user analysis. Watch jobs run at low priority and off-peak.
- Deadlines: extraction 6s, section synthesis 20s, importance scoring 4s. On expiry the
  call is cancelled and the section degrades gracefully to "could not be summarized in
  time" rather than blocking the report.

---

## 5. Source discovery & fetching

### 5.1 Discovery

1. **Resolve the subject** from free-form input: URL(s), brand name, or description.
   A small grammar-constrained call normalizes to `{ companies: [{name, homepage?}], intent }`.
2. **Search** via **self-hosted SearXNG** (zero marginal cost, no vendor lock) with
   **Brave Search API** as a paid fallback for reliability. Search APIs are not LLM APIs —
   using them does not violate the local-inference constraint.
3. **Targeted probes** on the resolved homepage: `/pricing`, `/plans`, `/changelog`,
   `/releases`, `/blog`, `/about`, `/careers`, `sitemap.xml`, plus `llms.txt` if present.
4. **Off-site public sources**: G2/Capterra/Trustpilot/Reddit/HN threads (search-surfaced),
   news, job boards, public ad libraries. Each is an adapter behind a `SourceProvider` trait
   so providers can be added or disabled without touching the orchestrator.

Sources are **ranked and capped** (target 8–14 per analysis) by a trust tier:
`tier1` = the company's own site; `tier2` = major review/news/job platforms;
`tier3` = forums and aggregators. Tier is displayed and drives claim confidence.

### 5.2 Fetching politely

- Honor `robots.txt` (cached 24h). Skip disallowed paths; record the skip and *say so* in
  the report rather than silently omitting.
- Identify honestly: `User-Agent: LandscapeBot/1.0 (+https://<domain>/bot)`, with a public
  bot page explaining behavior and an opt-out contact.
- Per-host rate limit (≥1s between requests, `governor`), global concurrency cap,
  conditional requests (`If-None-Match` / `If-Modified-Since`), 8s timeout, 2MB body cap,
  no JS rendering in v1 (headless Chrome is a Phase 6+ decision with real cost).
- No paywall circumvention, no login-gated content, no scraping of anything behind ToS
  that forbids it. When a source is inaccessible, the report says "not accessible to
  automated retrieval," which is *more* trustworthy than a fabricated summary.

### 5.3 Extraction

`HTML → boilerplate removal → main content → Markdown`, preserving headings, tables
(critical for pricing), lists, and `<time>` values. Store the Markdown, not the HTML —
it is what the model reads, what the diff runs on, and what the verifier matches against.

An `extraction_quality` score (text/markup ratio, heading presence, length) gates whether a
source is trusted enough to cite; low-quality extractions are dropped with a logged reason.

---

## 6. Caching & resource control

Local inference means **cache aggressiveness is a product feature, not an optimization.**

| Layer | Key | TTL | Notes |
|---|---|---|---|
| HTTP fetch cache | `sha256(canonical_url)` | 6h pricing/changelog, 24h general, 7d static | Disk + Postgres metadata; ETag revalidation makes refresh nearly free. |
| Extraction cache | `content_hash` | ∞ (content-addressed) | Same bytes never re-extracted. |
| **Per-source extraction cache** | `hash(content_hash + prompt_version + model_id)` | ∞ | **Highest-value cache.** Two users analyzing the same competitor reuse all reading work. |
| Section cache | `hash(sorted source hashes + section + prompt_version + model_id)` | 24h | |
| Full analysis cache | normalized subject + date bucket | 12h (anon), 6h (paid, or "refresh" button) | A popular competitor's report is generated once per window. |
| llama.cpp prefix cache | shared system prompt | process lifetime | Free wins from stable prompts. |
| CDN/HTTP | public shared reports | 5m + SWR | Shared report pages are static HTML. |

Resource control:
- Global semaphore on inference (§4.7); analyses queue, never thrash.
- Per-analysis hard budgets: ≤14 sources, ≤24k context tokens, ≤2,500 generated tokens,
  ≤90s wall clock. Exceeding a budget degrades the report gracefully and marks it partial.
- Anonymous quota keyed on hashed IP + coarse fingerprint, enforced in Redis.
- A **global admission controller**: if queue depth > threshold, anonymous requests get a
  "high demand — sign in for priority" message instead of joining the queue. Paying users
  are never blocked by free traffic.
- Disk budget for the fetch cache with hourly LRU eviction.

---

## 7. PDF generation

**Recommendation: `typst` as a Rust library.**

- Pure Rust, no headless Chrome, no `wkhtmltopdf`, no LaTeX install.
- Renders in tens of milliseconds; deterministic; excellent typography out of the box.
- Templates are plain `.typ` files with data injected as JSON — designers/founders can
  iterate on layout without touching Rust.
- Two templates: **`exec.typ`** (one page: subject, as-of timestamp, positioning, pricing
  table, top 5 features, top 3 changes, SWOT grid, source count, disclaimer footer) and
  **`full.typ`** (everything, with numbered sources and evidence quotes).

Rejected alternatives: headless Chrome (heavyweight on a box already running an LLM),
`printpdf` (too low-level), client-side jsPDF (quality and fidelity too poor for a
deliverable users forward to their boss).

Every PDF carries a footer: model id, prompt version, generation timestamp (UTC), source
count, and the standard public-data disclaimer.

---

## 8. Email delivery

- **Postmark** (best transactional deliverability) or **Resend** (nicer DX). Either via
  HTTPS API from Rust.
- Domains: `notifications@` for alerts, `hello@` for support replies, separate message
  streams so a noisy alert never poisons magic-link deliverability.
- SPF, DKIM, DMARC configured at day one. Magic links are the login path; deliverability
  *is* authentication reliability.
- MJML-derived HTML templates + a plaintext part, rendered with `minijinja`.
- One-click unsubscribe (`List-Unsubscribe-Post`), per-watch and global.
- Bounce/complaint webhooks disable delivery automatically.

---

## 9. Payments (Stripe)

- **Stripe Checkout** for purchase, **Stripe Billing Portal** for upgrades, downgrades,
  cancellation, invoices, and card updates. Writing custom billing UI is wasted effort.
- `async-stripe` for API calls; webhook endpoint with **signature verification** and
  **idempotent** handling keyed on `event.id` stored in Postgres.
- Events handled: `checkout.session.completed`, `customer.subscription.{created,updated,deleted}`,
  `invoice.payment_failed`, `invoice.paid`.
- **Entitlements are derived from the local `users.plan` + `plans` table**, refreshed from
  Stripe. Never call Stripe on the request path.
- Hourly reconciliation job repairs any missed webhook — webhooks are a delivery
  optimization, not a source of truth.
- Test mode + Stripe CLI fixtures in CI; a seeded fake clock test for renewal and dunning.

---

## 10. Change detection

```
watch.check → conditional GET (ETag/Last-Modified)
   ├─ 304 or identical content_hash → record check, exit (zero LLM cost)
   └─ changed → extract Markdown → normalize → SimHash
        ├─ SimHash distance < ε (noise: timestamps, CSRF tokens, rotating testimonials,
        │   view counters, cache-busting query strings) → discard
        └─ material diff → `similar` word-diff → LLM importance call (grammar-constrained)
             → { importance: 0–100, label: major|minor|cosmetic,
                 category: pricing|feature|positioning|policy|other,
                 summary_md, why_it_matters }
             → if importance ≥ user threshold → queue alert (digest-batched)
```

Noise suppression is the difference between a product and a nuisance:

- **Normalization before hashing**: strip timestamps, nonces, session ids, tracking params,
  rotating social proof, "N customers" counters, and dynamic year strings.
- **Per-element selectors** for known page types (pricing tables) so a blog-sidebar change
  doesn't fire a pricing alert.
- **Three-strike learning**: if a user marks two alerts from a watch "not useful," the
  importance threshold for that watch auto-raises and they're told why.
- **Digest by default** (daily), instant only for `major` pricing/positioning changes on
  paid tiers.
- LLM importance scoring only runs on material diffs — most checks cost zero inference.

---

## 11. Hosting & deployment

### 11.1 Start (Phase 0–4): one machine

- **Hetzner dedicated AX52-class** (~€55–70/mo) — dedicated CPU is essential; shared-vCPU
  VPS instances make local inference latency wildly unpredictable. This is the one place to
  not chase the free tier.
- Everything on it: Caddy, `landscape` (api+worker), Postgres, Redis, `llama-server`,
  SearXNG. Managed by **systemd units** with a `docker compose` alternative for Postgres/Redis/SearXNG.
- **Why systemd for `llama-server` specifically**: `Restart=always`, `MemoryMax=`,
  `CPUAffinity=`, and OOM isolation are exactly the controls needed, without container
  GPU-passthrough friction later.
- Deploy = build a static `x86_64-unknown-linux-gnu` binary in GitHub Actions → `scp` →
  `systemctl restart`. A 20-line script. No Kubernetes, no Nomad, no ECS.
- Frontend built in CI, served as static files by Caddy (and optionally fronted by
  Cloudflare free tier for DDoS protection and global asset caching).
- Backups: `pg_dump` nightly + WAL archiving to Backblaze B2 (~$1/mo). Restore drill in
  Phase 3, repeated quarterly. **An untested backup is not a backup.**

**Explicitly: can one machine serve early traffic?** Yes, with the numbers stated plainly.
At Rung 1 with 4 slots and ~60s of inference per analysis, sustained throughput is roughly
**200–350 analyses/day** before queueing becomes user-visible — comfortably above what a
launch generates. The binding constraint is inference, never Rust: axum on this hardware
handles thousands of req/s of non-LLM traffic, and Postgres is nowhere near loaded. The
correct scaling reflex is therefore *always* "improve cache hit rate or add a GPU," never
"add app servers."

### 11.2 Scale path

1. Add GPU box; move `llama-server` to it (Rung 2). Backend unchanged — just a URL.
2. Split `--role worker` onto its own host; both talk to the same Postgres.
3. Multiple inference nodes behind a least-loaded router.
4. Managed Postgres only when backup/HA operational load exceeds the founder's tolerance.

### 11.3 Environments

`local` (docker compose + a 4B model), `staging` (small VPS, real Stripe test mode,
same model as prod at lower `--parallel`), `production`. Migrations run on deploy, forward-only,
always backward-compatible for one release so rollback is safe.

### 11.4 Security baseline

Argon2id for any passwords; magic links single-use, 15-min TTL, constant-time compare;
SameSite=Lax `HttpOnly` `Secure` session cookies; CSRF tokens on state-changing forms;
strict CSP; **SSRF protection on user-supplied URLs** (block RFC1918/link-local/metadata
endpoints, resolve-then-verify to defeat DNS rebinding) — this matters a lot when the core
feature is "fetch a URL the user typed"; sqlx parameterized queries throughout; secrets in
systemd `EnvironmentFile` with `0600`, never in the repo; `cargo audit` + `cargo deny` in CI;
KB posts sanitized (Markdown → safe subset, no raw HTML) and rate-limited.
