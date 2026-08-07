# Landscape — Technical Architecture & Stack

> Section **B** of the roadmap. See [ROADMAP.md](ROADMAP.md) for the index and phasing.
>
> **Unfamiliar with a tool named here?** [ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md)
> explains every technology in this document — what it is, what the alternatives were, why
> this one was chosen, and what the choice costs.
>
> **Writing code against this design?** [CODING_QUALITY.md](CODING_QUALITY.md) is the standard
> every change is measured against — simplicity budgets, justified patterns, testing, review.
>
> **What the product actually outputs** is specified in
> [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) — report sections, the chart
> catalogue, and what is deliberately excluded. **Where the information comes from and how it
> is checked** is in [FACT_CHECKING.md](FACT_CHECKING.md).
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
| Package manager | **Bun** (`bun install` only) | Fastest installs, drop-in, build-time only. Adopted for CI speed since the cost is near zero — *not* as a runtime, bundler, or test runner (see §2.7). Verify on Windows + Playwright postinstall in Phase 0; fall back to npm if either misbehaves. |
| Language | **TypeScript, `strict: true`** + `noUncheckedIndexedAccess` | Report schema is the product contract; types must be load-bearing. |
| Framework | **React 19**, function components + hooks | Constraint. `use()` + Suspense for streamed data. |
| Routing | **TanStack Router** (or React Router v7) | Type-safe params; the app has ~12 routes, not 100. |
| Server state | **TanStack Query** | Caching, retries, dedup, invalidation for KB/watchlist/billing. |
| Client state | **Zustand** (one small store) + URL state | No Redux. The only real client state is the composer draft and stream buffer. |
| Styling | **Tailwind CSS** + a handful of local primitives (Radix UI for dialog/popover/toast) | Fast, accessible, no design-system tax. |
| Forms | Native + **Zod** | Zod schemas are shared conceptually with the Rust `serde` structs (kept in sync by codegen, §2.5). |
| Charts | **No JS charting library.** Charts are static SVG generated server-side in Rust (`landscape-charts`) | One renderer serves the React app, the Typst PDF, SSR pages and email. A JS library serves only one of those, breaks replay determinism, and ships a plotting engine to draw fixed SVG. See [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §7. |
| Testing | Vitest + Testing Library + Playwright | Playwright covers the three critical flows end to end. |

### 2.2 Route map

```
/                       Composer (the whole product; also the landing page)
/a/:analysisId          Report view (streamed, then static; shareable, SEO-indexable)
/a/:analysisId/pdf      Triggers PDF download (server-rendered)
/compare/:a-vs-:b       Pre-generated comparison page (SSR, indexable; Phase 6)
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
  | { type: 'done';     analysis: AnalysisSummary; completeness: 'complete' | 'awaiting_pass2' }
  | { type: 'version'; version: number; changed: SectionKey[] }   // pass 2 landed
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

### 2.7 Where Bun stops

`bun install` is adopted; **Bun as a runtime, bundler, or test runner is not.** The two have
unrelated cost profiles and should not be decided together.

Install is pure build-time: identical output, no runtime surface, no effect on latency, user
experience, or infrastructure cost. It is a free CI win, taken on that basis alone.

Replacing Vite would cost the mature React plugin and HMR ecosystem; replacing Vitest would
cost the shared-transform-pipeline guarantee that makes tests and app build identically; and
Bun-as-runtime has nothing to run, because there is no JavaScript process in production —
static files are served by Caddy and SSR lives in Rust (§2.2, and
[ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) §1.11). That last point decides
it: Bun's defining feature has no surface in this architecture.

**Where the CI time actually is:** the Rust build, by a wide margin. Ordered by payoff —
`Swatinem/rust-cache` or `sccache` (minutes), running the Rust and frontend jobs in
parallel (minutes), `cargo-nextest` in place of `cargo test` (tens of seconds), and
`cargo check` ahead of `cargo build` on PRs. Swapping npm for Bun is worth ~15–30s against
those. Take it, but do not mistake it for the optimization.

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
| Observability | `tracing` + `tracing-subscriber` + OpenTelemetry → Grafana/Tempo; `metrics` + Prometheus exporter | Instrument with `tracing` from day one; the *backend* is swappable. Start with local logs + a hosted error tracker, and defer self-hosting the metrics stack until there is traffic to look at — it competes for RAM with the model. Decide in Phase 0. |
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
  landscape-search/    SearXNG adapter behind a SourceProvider seam, versioned query
                       set, host-based admission (Brave is the documented fallback,
                       deliberately not built until the primary has run)
  landscape-signals/   discussion venues: HN + GitHub adapters, venue-fit gate,
                       absence panel, cross-post independence grouping
  landscape-llm/       llama-server client, grammars, prompts, slot pool, budgets
  landscape-analyze/   the orchestrator: plan → fetch → read → synthesize → verify
  landscape-watch/     change detection, importance scoring, alert composition
  landscape-charts/    static SVG emitters (8 fixed chart types) + resvg for email PNG
  landscape-pdf/       typst templates + render
  landscape-billing/   Stripe, plans, entitlements
  landscape-kb/        support KB, search, moderation
  landscape-api/       axum routers, auth, rate limits, SSE
  landscape-worker/    job handlers
  landscape/           bin: wires roles together
```

**`landscape-signals` is deliberately separate from `landscape-fetch`.** Discussion venues
are read through typed APIs rather than crawled HTML, they carry per-venue terms that decide
whether we may read at all ([DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4), and their
records are **immutable** — a thread read once never needs re-reading. That last property
gives this crate a fundamentally different caching strategy from the rest of the fetch path,
which is reason enough for its own boundary.

It is also the crate where a terms decision becomes code: each venue adapter declares whether
it may be `fetched`, `linked_only`, or is `unavailable`, and the absence panel refuses to
publish a negative for any venue not in the first state.

### 3.3 Data model (Postgres 16)

```sql
users(id, email, email_verified_at, created_at, plan, stripe_customer_id, role,
      default_inference_provider /*local|byok*/)
user_api_keys(...)  -- see §4.8; encrypted, excluded from ordinary backups
sessions(id, user_id, expires_at, ip_hash, ua_hash)
plans(key, analyses_per_month, watches, watch_interval_minutes, price_cents)
usage_counters(subject_id, subject_kind /*user|ip*/, period, analyses_used, updated_at)

analyses(id, user_id NULL, anon_key_hash NULL, input_text, resolved_subject jsonb,
         status, version int, superseded_by NULL, completeness /*complete|awaiting_pass2*/,
         model_id, prompt_version, strictness_setting /*primary|primary_attributed|all*/,
         inference_provider /*local|openai_compatible|anthropic*/,
         byok_key_id NULL, fell_back_to_local bool, structured_output_mode,
         started_at, finished_at,
         latency_ms, tokens_in, tokens_out, cost_compute_ms, share_slug, visibility)
analysis_sections(analysis_id, key, payload jsonb, tokens_out, verify_status)
sources(id, url, canonical_url, host, first_seen_at, publisher_group)
analysis_sources(analysis_id, source_id, label /*S1..Sn*/, fetched_at,
                 content_hash, http_status, extraction_quality,
                 source_class /*P|A|U|N*/, attribution_signals_confirmed jsonb,
                 independence_group, claim_authority, truth_authority)
sources_not_used(analysis_id, url, host,
                 not_used_reason /*unverified_by_our_criteria|could_not_reconcile*/,
                 signals_confirmed jsonb, what_it_stated,
                 primary_value, primary_fetched_at, quarantine_path)
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
| `pdf.render` | analysis complete **and not awaiting pass 2** | Pre-warms exec PDF. Held while a render is pending so nobody forwards a stale one (PRODUCT_SPEC §2.1A). |
| `source.render` | pass 1 found a page needing a browser | Tier 5 (§5.5). Concurrency 1, off-peak, memory-capped, pauses under inference load. |
| `analysis.pass2` | all render jobs for an analysis resolved | Re-runs affected sections, emits v2, notifies. |
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
`landscape-llm` already isolates it behind this trait. The same isolation is what makes
user-supplied hosted providers a matter of adding an adapter rather than a refactor — see
§4.8.

### 4.2 Model selection

**The launch host is Oracle Cloud Always Free: one Ampere A1 instance, 4 OCPU
(Neoverse N1, aarch64), 24GB RAM, network-attached block storage, no GPU.** Every choice in
this section is anchored to that budget. See §4.4 for the rung ladder beyond it.

**Memory budget — the number that actually constrains model choice is not 24GB.**
The same box runs everything else:

| Component | Budget |
|---|---|
| OS + kernel page cache headroom | ~2.0 GB |
| Postgres (`shared_buffers` 1GB, tuned small) | ~1.5 GB |
| Redis | ~0.3 GB |
| SearXNG (Python) | ~0.7 GB |
| `landscape` binary (api + worker) | ~0.4 GB |
| Caddy | ~0.1 GB |
| Safety margin (never OOM the box) | ~2.0 GB |
| **Available to `llama-server` (weights + KV + compute buffers)** | **≈ 17 GB** |

Three model roles, all llama.cpp/GGUF, **all resident simultaneously** — a 24GB box can hold
several small models at once, and that is strictly better than swapping one large model in and
out, which would thrash network block storage.

| Role | Job | Rung 0 (Oracle Free) | Requirement |
|---|---|---|---|
| **Router** | Subject resolution, clarify-or-not classification, KB thread matching, alert importance scoring | **Qwen3-1.7B** Q4_K_M (~1.1 GB) | Fast, reliable under grammar. Outputs are enums and short strings. |
| **Extractor** | Per-source structured reading — the high-volume role | **Qwen3-4B** Q4_K_M (~2.5 GB) | Strong instruction-following on *short* contexts; faithful span selection. |
| **Synthesizer** | Report prose, sentiment themes, SWOT — quality-critical, one pass | **Qwen3-8B** Q4_K_M (~5.0 GB) | Grounded summarization, low invention. Q3-free. |

Resident total ≈ **8.6 GB of weights**, leaving ~8 GB for KV caches and compute buffers
across all three. Comfortable, with room to promote the synthesizer to **Qwen3-14B**
(~9 GB, total ~12.6 GB) **if and only if Phase 0 shows the latency is affordable** — on
4 ARM cores it probably is not. That is a measurement, not a preference.

**Why not a 30B MoE on this box.** Qwen3-30B-A3B is the quality-per-token bargain and the
roadmap previously called it the sleeper pick — but at Q4_K_M it is ~18.6 GB of weights
*alone*, which does not fit in a ~17 GB budget once KV cache and compute buffers are
counted. IQ4_XS (~16.5 GB) technically fits and leaves nothing for anything else. It is
Rung 1's first upgrade, not Rung 0's baseline.

**Why not stream experts from disk.** The technique of keeping the shared core resident and
paging per-token MoE experts from storage is real and works well on Apple Silicon with local
NVMe. It does not transfer here: Oracle Free storage is network-attached block, where
per-token random reads are orders of magnitude worse than local NVMe, and the box has no GPU
to pair it with. It also solves the wrong problem — see §4.4, where the binding constraint is
compute and memory bandwidth, not capacity.

Selection principles, unchanged:

- **Licensing gate first.** Apache-2.0 / permissive preferred (Qwen3, Mistral Small).
  Read the license before the benchmark — Llama and Gemma carry use restrictions that
  matter for a commercial SaaS.
- **Long-context faithfulness** matters more than raw MMLU. The job is "read this page, do not
  invent." Evaluate on *our* golden set (see Quality doc), never on leaderboards.
- **Model choice is a config value**, not a code change. `MODEL_SYNTH=...gguf`. Swapping
  models must be a one-line change plus an eval run.
- **Re-bake-off quarterly.** Small open models improve faster than any other input to this
  product. A model chosen in Phase 0 is very likely wrong by Phase 5.

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
quantization damage shows up precisely in long-context faithfulness. On Rung 0 the case for
`q8_0` KV is stronger than usual because three models share ~8 GB of cache budget; validate
it in Phase 0 rather than deferring.

**aarch64-specific: benchmark `Q4_0` against `Q4_K_M`.** llama.cpp repacks `Q4_0` weights at
load time to use ARM `dotprod` / `i8mm` instructions, which on Neoverse N1 can make it
materially faster than K-quants despite `Q4_0` being the lower-quality format. This is a
genuine quality-versus-latency trade that only exists on ARM, and on a box this slow it may
be worth taking for the Router and Extractor roles while keeping `Q4_K_M` for the
Synthesizer. Phase 0 measures both on the golden set; ship whichever passes the quality
gates fastest.

### 4.4 Hardware rungs & latency budget

The product **launches on Rung 0 at €0/month** and climbs only when revenue pays for the
next step. Full revenue triggers are in [ROADMAP.md](ROADMAP.md) §6.

#### The binding constraint is not memory

On Oracle Free the intuition that "a bigger model needs more RAM" is a distraction. 24 GB
holds far more model than 4 Neoverse N1 cores can *run*. Two costs dominate, and both scale
with model size:

- **Generation** is memory-bandwidth-bound: tokens/second ≈ (bandwidth) ÷ (bytes read per
  token). A 4-core slice of a shared Ampere Altra has modest effective bandwidth.
- **Prompt processing (prefill)** is compute-bound, and on 4 cores it is the larger problem.
  **Prefill, not generation, is what makes a naive design unusable here** — 16,000 tokens of
  source context at ARM CPU prefill rates is minutes, not seconds.

Every design decision below follows from that second sentence.

#### Rung 0 — Oracle Cloud Always Free (launch, €0/mo)

One Ampere A1: 4 OCPU (Neoverse N1, aarch64), 24 GB RAM, 200 GB network block storage, no GPU.
Models per §4.2. **Order-of-magnitude estimates only — Phase 0 replaces every number here
with a measurement, and the whole latency plan is contingent on them:**

| Model (Q4_K_M) | Prefill | Generation |
|---|---|---|
| Qwen3-1.7B | ~80–200 tok/s | ~12–25 tok/s |
| Qwen3-4B | ~40–100 tok/s | ~6–12 tok/s |
| Qwen3-8B | ~20–60 tok/s | ~3–7 tok/s |

Continuous batching helps more than it looks: on a memory-bound CPU, running 4 sequences
concurrently reads the weights **once** and applies them to all four, so aggregate throughput
is far better than 4× a single stream would suggest. This is why `--parallel` matters more on
CPU than on GPU.

**Honest latency target for Rung 0: first content in 20–40s, complete report in 90–180s.**
That is not the 15–25s product goal, and the documents should not pretend otherwise. The
15–25s target is a **Rung 2** figure. What Rung 0 must deliver instead is *visible, honest
progress* and *content worth waiting for* — see [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1.

#### The seven Rung-0 compensations

All are permanent architecture improvements that also raise quality. None is a hack.

1. **Deterministic-first extraction (§5.4).** Prices, dates, tier names, changelog entries and
   version numbers are parsed by *code*, not by a model. This is the single biggest lever: it
   removes most prefill, and parsed values are more accurate than generated ones.
2. **Span pre-selection before the model sees anything.** Heuristics (heading structure,
   table detection, keyword windows) reduce each source from ~2,500 tokens to a ~400-token
   candidate window. Eight sources then cost ~3,200 prefill tokens instead of ~20,000.
3. **Tiny structured outputs.** Per-source extraction emits ~80–120 tokens of grammar-constrained
   JSON, never prose.
4. **Section-parallel generation.** Independent sections are separate small calls across
   slots, batched, not one monolith.
5. **Total generation budget ≤ 900 tokens** per analysis on Rung 0 (versus 2,500 on GPU).
6. **Cache everything (§6).** The per-source extraction cache means the second analysis of a
   popular competitor costs almost nothing. On free-tier hardware, cache hit rate *is* the
   capacity plan.
7. **Honest queue display.** "3 ahead of you, about 4 minutes" beats a spinner, and beats a
   lie.

**Capacity estimate:** ~60–120 analyses/day before queueing becomes user-visible, heavily
dependent on cache hit rate. Enough for validation and early users; not enough for a
successful launch, which is why Rung 1 is triggered by demand, not by taste.

#### Rung 1 — split tiers (~€50–70/mo)

Oracle Free keeps the web tier (Rust API, Postgres, Redis, SearXNG, Caddy) — which it handles
comfortably and which stays free forever. Add **one Hetzner AX52-class dedicated box**
(Ryzen 7 7700, 8c/16t, 64 GB DDR5, local NVMe) running only `llama-server`.

This is the most capital-efficient step available: it puts the entire first spend on the
actual bottleneck and nothing else.

- Unlocks **Qwen3-30B-A3B** (MoE, ~18.6 GB at Q4_K_M, ~3B active/token) — a large quality
  jump at CPU-friendly speed, because MoE reads only the active experts per token.
- Estimated: 1,200-token synthesis in **60–90s**; p50 report ~45–70s.
- Backend change required: **none**. `LLAMA_BASE_URL` points at the new host over a private
  network or WireGuard tunnel.

#### Rung 2 — single GPU box (~€180–250/mo)

Hetzner GEX44 (RTX 4000 SFF Ada, 20 GB VRAM) or an RTX 4090-class dedicated.

- Qwen3-14B or Qwen3-30B-A3B Q4_K_M fully offloaded: generation ~45–75 tok/s, prefill
  ~2,000+ tok/s.
- **This is the rung at which the 15–25s product target becomes real.**
- Flash attention (`-fa`), `--cont-batching`, `--parallel 4–8`, and optional **speculative
  decoding** (`-md` with a 0.5–1.5B draft) for another 1.3–2×.
- The Rung-0 compensations all still apply and now buy headroom instead of survival.

#### Rung 3 — large-model GPU (~€600–1,200/mo)

48–80 GB VRAM (RTX 6000 Ada, L40S, A100 80GB). Unlocks 70B-class dense models or large MoEs
such as gpt-oss-120b (~5B active) at Q4. Meaningful quality gain on synthesis and SWOT.

#### Rung 4 — frontier open weights (€3,000+/mo)

Worth stating plainly because it is a common planning error: **Kimi K2-class models are not a
"when revenue comes in" upgrade for a bootstrapped product.** K2 is ~1T total parameters
(~32B active); at 4-bit that is roughly 550–600 GB of weights, requiring a multi-GPU server
(8×H100-class) — on the order of €3,000–6,000/month, or €40–70k/year. At the roadmap's
"infrastructure ≤ 20% of MRR" rule that implies ~$200k+ ARR. It is a real destination, but it
sits well beyond the bootstrapped horizon, and the ladder should not be planned around it.
The realistic quality ceiling for the next two years of this product is Rung 3.

#### Rung 5 — scale-out

Multiple inference nodes behind the same Postgres job queue; the Rust API is already stateless
with respect to inference. Register nodes in an `inference_nodes` table and route by
least-loaded slots.

**Promotion triggers are revenue-gated, not latency-gated** — see [ROADMAP.md](ROADMAP.md) §6.
Latency alone cannot justify a spend that has no income behind it.

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

### 4.7 Process topology, queueing & batching

**`llama-server` serves exactly one model per process.** Three roles therefore means three
supervised processes on Rung 0:

| Process | Model | Port | `--threads` | `--parallel` |
|---|---|---|---|---|
| `llama-router` | Qwen3-1.7B | 8081 | 2 | 2 |
| `llama-extract` | Qwen3-4B | 8082 | 4 | 4 |
| `llama-synth` | Qwen3-8B | 8083 | 4 | 2 |

Thread counts deliberately over-subscribe 4 cores, because the pipeline is **phase-ordered** —
the router is idle while synthesis runs, and vice versa. Overlap is bounded in Rust instead:

```
call → global Semaphore(2–3 in-flight, all servers) → per-server slot
          ↑ paid tier holds a reserved permit
```

- A single **global** semaphore across all three servers, not one per server. On 4 cores you
  cannot usefully run two models' forward passes at once; the semaphore is what prevents
  concurrent analyses from thrashing. Sized 2–3 on Rung 0, tuned in Phase 0.
- Within a server, `--parallel` gives **continuous batching**, which on memory-bound CPU
  inference is the single largest throughput win: weights are read once and applied to every
  sequence in the batch. Extraction calls are issued as a `FuturesUnordered` and batch
  naturally.
- Reserve one permit for interactive traffic so watch-checking never starves a live user
  analysis. Watch jobs run at low priority and off-peak.
- Deadlines (Rung 0): router 5s, extraction 20s, section synthesis 60s. On expiry the call is
  cancelled and that section degrades to "could not be completed within the time budget"
  rather than blocking the report. Deadlines tighten by rung.

**Two operational rules that are non-negotiable on Oracle Free:**

- **`--mlock` on every model, and swap disabled.** Storage is network-attached. If model
  weights are ever paged out, performance does not degrade — it collapses. Locking ~9 GB of
  weights into RAM is the entire reason the memory budget in §4.2 is calculated so carefully.
- **`MemoryMax=` on each systemd unit**, sized so that a runaway inference process is killed
  and restarted rather than triggering the kernel OOM killer against Postgres.

### 4.8 Bring-your-own-key (BYOK) — optional user-supplied inference

**The local model is the default and always will be.** The product must be fully functional,
with no degraded features, for a user who never supplies a key. BYOK is an *opt-in override*
for users who would rather pay their own provider than wait on free-tier hardware.

This does not weaken the local-inference constraint — it strengthens the product's position
against it. The core analysis path ships local, works local, and is evaluated local. BYOK is
a preference a user may express about their own account.

#### Why it belongs in this architecture

- **It is the escape valve for R12** (ROADMAP §5), the bootstrapping stall. A user who finds
  90–180s intolerable can paste a key and get 15–25s *today*, at their cost rather than ours.
  That converts the free tier's worst property into a segmentable one.
- **It costs us nothing to serve.** Inference moves off our box entirely, so BYOK analyses do
  not consume the scarcest resource in the system.
- **It answers the "why not just use ChatGPT" objection** (R6) from the other side: the
  product's value is the retrieval, the verification, the schema and the monitoring — not the
  model. Letting users bring a frontier model and *still* getting a better result than a chat
  session is the strongest possible demonstration of that.

#### Provider abstraction

`landscape-llm` already isolates inference behind `LlmClient`. BYOK adds implementations:

| Adapter | Covers | Structured output mechanism |
|---|---|---|
| `local` (default) | `llama-server` | **GBNF grammar** from JSON Schema |
| `openai_compatible` | OpenAI, Azure OpenAI, OpenRouter, Together, Fireworks, Groq, DeepInfra, and any self-hosted vLLM/llama.cpp endpoint | **Strict JSON Schema** (structured outputs) |
| `anthropic` | Claude models | **Tool `input_schema`** — a forced single-tool call |

**The same `schemars`-generated JSON Schema drives all three** (§2.5). One schema → GBNF for
llama.cpp, strict-schema for OpenAI-compatible, tool input schema for Anthropic. This is the
payoff of having defined the report shape once in Rust: adding a provider does not mean
re-expressing the contract.

Adapters declare their capability, and the orchestrator degrades knowingly:

```rust
pub struct ProviderCapabilities {
    pub structured_output: StructuredOutput,  // Grammar | StrictSchema | JsonMode | None
    pub streaming: bool,
    pub max_context: u32,
}
```

`JsonMode` and `None` providers fall back to parse-and-retry (max 2 attempts) and the report
records that a weaker constraint was used. A provider that cannot produce valid structured
output twice in a row is rejected with a clear message rather than silently degrading the
report.

#### Key storage & security

Third-party credentials are the most dangerous data this product will ever hold. They are
treated accordingly.

- **Session-only by default.** A key supplied for interactive analyses is held in memory for
  the session and never written to disk. Persisting is a separate, explicit opt-in — and is
  *required* only if the user wants BYOK applied to background jobs (watch checks, digests),
  which is stated plainly at the point of choice.
- **Encrypted at rest** with **XChaCha20-Poly1305**, using a master key from the systemd
  `EnvironmentFile` (mode `0600`) — never in the database, never in the repo. Rows carry a
  `key_version` so rotation and re-encryption are possible.
- **Never returned to the client.** After storage the API exposes only
  `{ provider, model, last_four, added_at, last_used_at, status }`. There is no "reveal key"
  endpoint, because there is no legitimate use for one.
- **Never logged.** A `Secret<String>` newtype with redacting `Debug`/`Display`, plus a CI
  grep that fails on any format string interpolating it. Provider error bodies are
  sanitized before they reach logs or the user.
- **Deleted on request and on account deletion**, immediately and verifiably.
- **`user_api_keys` is excluded from database backups**, or backed up under a separate key.
  A restored backup should not resurrect credentials a user deleted.
- **Custom base URLs go through the SSRF guard** (§11.4). A user-supplied
  `openai_compatible` endpoint is exactly the "fetch a URL a stranger typed" problem again,
  now with our credentials-handling code attached. Same resolve-then-validate rules, no
  exceptions, plus an allowlist of known provider hosts with custom hosts as an explicit
  opt-in.

```sql
user_api_keys(id, user_id, provider, model, base_url NULL,
              ciphertext bytea, nonce bytea, key_version int,
              last_four text, status /*active|invalid|rate_limited|exhausted*/,
              scope /*interactive|interactive_and_background*/,
              created_at, last_used_at, last_error_at, last_error_code)
```

#### Failure handling — fall back, never fail silently

Keys expire, run out of credit, and get rate-limited, usually at the worst moment.

- On provider failure the analysis **falls back to the local model automatically** and the
  report carries a visible notice explaining what happened. A watch alert at 3am must not
  simply not arrive.
- The key's `status` is updated and the user is emailed **once** (not per failure) that their
  key stopped working and what the product did instead.
- Fallback is never silent and never disguised: provenance is recorded per analysis.

#### Provenance & accounting

Every analysis records `inference_provider`, `model_id`, and whether fallback occurred. This
appears on the report, in the PDF footer, and in the admin console — and, critically,
**quality metrics are always sliced by provider** (Quality doc §3.2), because BYOK reports
generated by a frontier model would otherwise flatter the local model's measured quality.

Token counts are surfaced to the user per analysis so a BYOK bill is never a surprise.
We do not, and cannot, see their spend — only tokens submitted and returned.

#### What BYOK does and does not change

- **It does not bypass any quality control.** Layers 1–5 of the anti-hallucination stack are
  model-agnostic Rust. A claim from Claude or GPT whose evidence quote is absent from its
  cited source is deleted exactly as one from Qwen3-8B would be. This is precisely what makes
  BYOK safe to offer: we do not trust the user's model any more than our own.
- **It does not remove rate limits.** Fetching, extraction, PDF rendering and storage still
  cost us bandwidth and CPU. BYOK **raises the analysis quota substantially** because
  inference stops being ours, but it does not make an account unlimited.
- **It does not unlock paid features.** Watch counts, alert cadence, webhook delivery, API
  access and team features remain tier-gated, because none of them is an inference cost.

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

Sources are **ranked and capped** (target 8–14 per analysis) by trust tier, and tier is
**two-dimensional**: a vendor's own page is the highest authority for *what they claim* and a
weak authority for *what is true*. Conflating those is the central epistemic error in
automated competitive intelligence. The full model, the admission rules (including the
primary-source-only requirement for pricing, features and changelog facts), and independence
grouping are specified in [FACT_CHECKING.md](FACT_CHECKING.md) §3.2 and §4.

### 5.2 Fetching politely

- Honor `robots.txt` (cached 24h). Skip disallowed paths; record the skip and *say so* in
  the report rather than silently omitting.
- Identify honestly: `User-Agent: LandscapeBot/1.0 (+https://<domain>/bot)`, with a public
  bot page explaining behavior and an opt-out contact.
- Per-host rate limit (≥1s between requests, `governor`), global concurrency cap,
  conditional requests (`If-None-Match` / `If-Modified-Since`), 8s timeout, 2MB body cap.
  **No JS rendering on the request path, ever** — see §5.5 for the escalation ladder and the
  deferred render tier.
- No paywall circumvention, no login-gated content, no scraping of anything behind ToS
  that forbids it. When a source is inaccessible, the report says "not accessible to
  automated retrieval," which is *more* trustworthy than a fabricated summary.

### 5.3 Extraction

`HTML → boilerplate removal → main content → Markdown`, preserving headings, tables
(critical for pricing), lists, and `<time>` values. Store the Markdown, not the HTML —
it is what the model reads, what the diff runs on, and what the verifier matches against.

An `extraction_quality` score (text/markup ratio, heading presence, length) gates whether a
source is trusted enough to cite; low-quality extractions are dropped with a logged reason.

### 5.4 Deterministic-first extraction

**Do not ask a language model to do what a parser can do better.** This is presented as a
latency compensation for Rung 0, but it is primarily a *quality* decision and it stays in
place at every rung.

| Fact type | Extracted by | Why |
|---|---|---|
| Prices, currencies, billing periods, seat/usage basis | **Code** — HTML table parsing + currency/period regex over the pricing page | A parsed `$8/user/mo` is exact. A generated one can be off by a digit, and pricing is the most-read and most-quoted section in the product. |
| Tier names and per-tier limits | **Code** — table row/column structure | Structure is already in the markup; re-deriving it through a model discards information. |
| Changelog entries, release dates, version numbers | **Code** — heading + `<time>` + date regex | Dates are the most common LLM fabrication in "recent changes" and are trivially verifiable. |
| Feature lists on structured pages | **Code first**, model for normalization only | Bullet lists parse cleanly; the model only harmonizes wording across competitors. |
| Compliance standards a page names | **Code** — a closed list of standard names, longest spelling first | A company does not invent a standard, so the name can come from the page by construction. The model is asked one thing about each: held, or being worked towards. |
| Open roles on a careers page | **Code** — the page's own *Open roles* heading, then title-shaped lines | A job title is a line somebody wrote down on purpose. The titles are reported as written and **not sorted into functions**: three real pages file roles under labels a keyword gets wrong. **A page that does not announce its list is not read** — the shape rules clean up inside a list, they do not find one, and a testimonial byline satisfies every one of them. |
| Positioning, category language, differentiators | **Model** | Genuinely requires language understanding. |
| Review/sentiment themes | **Model** | Genuinely requires language understanding. |
| SWOT interpretation | **Model** | The one place inference is permitted. |

Consequences:

- **Prefill collapses.** The largest source documents — pricing pages and changelogs — stop
  entering the model's context at all. On Rung 0, where prefill is the dominant cost, this is
  worth more than any other optimization in this document.
- **Faithfulness improves.** Deterministically parsed values carry an exact source offset, so
  the verifier (Quality doc §2, Layer 3) matches them trivially and Layer 4's price/date
  validators become near-tautological rather than best-effort.
- **Failures become honest.** When the parser cannot find a pricing table, the answer is
  "no public pricing found, here is what we checked" — which is the correct output, and
  strictly better than a model guessing from prose.

**Span pre-selection** applies the same idea to what the model *does* read: heading structure,
table proximity, and keyword windows reduce each source from ~2,500 tokens to a ~400-token
candidate window before the extractor sees it. Eight sources then cost roughly 3,200 prefill
tokens instead of ~20,000. The selection heuristic is versioned alongside prompts and is
itself part of the golden-set evaluation, because a bad window is indistinguishable from a bad
model at the output.

### 5.5 JavaScript-rendered pages — an escalation ladder, not a browser

Some pages build their content in the browser, so a plain fetch returns a shell. **The
response is a ladder, and a headless browser is the last rung — most of the gap closes
without one.**

| Tier | Method | Cost | Notes |
|---|---|---|---|
| **1** | Static HTML parse | current | Most sites |
| **2** | **Embedded state** — `__NEXT_DATA__`, `__NUXT__`, inline JSON, JSON-LD `Product`/`Offer` | ~free | **The big one.** A Next.js pricing page ships its pricing as JSON *in the initial HTML*. The page looks JS-rendered; the data is already in the bytes we fetched. |
| **3** | **Discovered JSON API** — the endpoint the page itself calls | ~free | Cheaper *and better* than rendering: structured data instead of scraped text |
| **4** | **Archive snapshot** — an existing Internet Archive capture | free | Occasionally sufficient |
| **5** | **Headless render** | expensive | The true residual only |

**Tiers 2–4 ship regardless of any decision about browsers.** They are a superior data path
where they work, and they cost nothing.

#### Sizing before building

Phase 1 instruments two counters, because building tier 5 before knowing its size would be
speculative work:

1. Of pricing pages fetched, what share yield **no price** from static HTML?
2. Of those, what share are recovered by **tiers 2–4**?

Phase 2's exit re-measures. **If the residual is under ~5%, tier 5 is not built** and the
honest-gap treatment stands.

#### Tier 5, if the residual justifies it

Never synchronous, never in the request path. Rendering is a **job**:

```
static fetch → no data → enqueue render job → pass 1 report ships with an honest marker
                              ↓  (off-peak, concurrency 1, memory-capped)
                        render → extract → cache content-addressed, permanently
                              ↓
                        pass 2 → report updates to v2, user notified
```

Two-pass UX is specified in [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1A.

**Resource control — the constraint is a hard ceiling, not a spend threshold.** Oracle Always
Free is a fixed allocation (4 OCPU / 24 GB / 200 GB / 10 TB egress); exceeding CPU or RAM
produces contention or an OOM kill, never a bill. Charges arise only from *provisioning*
beyond the free limits, which heavy use of existing resources never does. Ingress is
unmetered, so **bandwidth is not the constraint — memory and CPU are.**

- Chromium on arm64: ~150 MB base RSS, ~150–250 MB per page → **~400 MB peak at concurrency 1**.
- **`MemoryMax=512M` on the render systemd unit.** An oversized page kills that render process
  only; the job retries later and Postgres and `llama-server` never notice. Same isolation
  pattern as §4.7.
- **Circuit breaker:** rendering pauses whenever inference queue depth exceeds threshold.
  Live users always win.
- Rendered snapshots join the fetch-cache byte budget and its LRU eviction (§6).
- Benchmark **Lightpanda** (a lightweight headless browser built for this use case) against
  Chromium before assuming Chromium is the only option.

**Deployment:** on-box is viable *because* rendering is deferred — the contention argument for
a separate host disappears once nothing waits on it. If Phase 2 measurement shows heavy
demand, a **~€4/mo Hetzner CX22** running only the render worker removes it from the inference
host entirely, and slots into the Rung 1 spend.

**Rejected:** splitting the Oracle allocation (the 4 OCPU / 24 GB is one shared pool — a render
node steals ~25% from the bottleneck), and GitHub Actions as a render worker (free minutes,
but product data pipelines fall outside their terms of use).

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

On Rung 0 this is not an optimization at all — **cache hit rate is the capacity plan.** A 50%
full-analysis hit rate literally doubles the number of users the free tier can serve, and it
is the only lever that costs nothing.

Resource control — budgets are per rung, because the whole point of climbing is to spend them:

| Budget | Rung 0 (Free) | Rung 1 (CPU box) | Rung 2 (GPU) |
|---|---|---|---|
| Sources per analysis | ≤ 8 | ≤ 12 | ≤ 14 |
| Prefill tokens (after span pre-selection, §5.4) | ≤ 4,000 | ≤ 12,000 | ≤ 24,000 |
| Generated tokens | ≤ 900 | ≤ 1,600 | ≤ 2,500 |
| Wall clock | ≤ 240s | ≤ 120s | ≤ 90s |

- Global semaphore on inference (§4.7); analyses queue, never thrash.
- Exceeding a budget degrades the report gracefully and marks it partial — it never silently
  truncates.
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
  table, **the cost-at-scale chart**, top 5 feature-matrix rows, top 3 changes, SWOT grid,
  source count, disclaimer footer) and **`full.typ`** (everything, with numbered sources and
  evidence quotes).
- Charts arrive as SVG from `landscape-charts` and embed natively in Typst — the same SVG the
  web report renders, so the two cannot drift ([COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §7.2).

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

**Sales tax / VAT is an open decision, not an implementation detail.** Stripe is a payment
processor, not a merchant of record: registration, calculation, and remittance of VAT, GST,
and US sales tax remain our legal obligation. **Stripe Tax** (a paid add-on) calculates
correctly but does not file. The alternative — **Paddle** or **Lemon Squeezy** — becomes the
legal seller and absorbs compliance entirely, at a higher fee (~5% + fixed vs ~2.9% + 30¢)
and a less pleasant API. The recommendation stands on developer experience and build speed,
but **the choice must be made deliberately in Phase 4, before customers exist** — migrating
billing providers afterwards is genuinely painful. See
[ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) §7.

**Supply risk:** Stripe maintains no official Rust SDK, so `async-stripe` is
community-maintained and can lag the API. The blast radius is deliberately small — because
entitlements come from local tables and Stripe is never called on the request path, the SDK
surface is a handful of Checkout, Portal, and webhook calls, replaceable with raw `reqwest`
in a day. Keep it that way: no Stripe types in domain code.

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

### 11.1 Start (Phase 0–5): Oracle Cloud Always Free, one machine, €0/mo

- **Oracle Cloud Always Free — Ampere A1: 4 OCPU, 24 GB RAM, aarch64, 200 GB block storage.**
  This is the launch host and it costs nothing, forever. The bootstrapping premise is that
  the product must reach paying customers before it costs money; see
  [ROADMAP.md](ROADMAP.md) §6.
- Everything on it: Caddy, `landscape` (api+worker), Postgres, Redis, the three
  `llama-server` processes (§4.7), SearXNG. Managed by **systemd units** with a
  `docker compose` alternative for Postgres/Redis/SearXNG.
- **Build for aarch64.** Rust cross-compiles cleanly to `aarch64-unknown-linux-gnu`, but CI
  must actually target it — GitHub Actions provides ARM runners, and a build that only ever
  ran on x86 will find its first ARM bug in production. llama.cpp is built on the host with
  ARM `dotprod`/`i8mm` support enabled.
- **Three Oracle-specific operational rules**, each of which has taken down someone's free
  tier before:
  1. **Convert the account to Pay-As-You-Go.** Always Free resources on a *trial* account
     can be reclaimed; on a PAYG account they are retained, and staying inside the free
     limits still bills €0. Do this before building anything on the instance.
  2. **A1 capacity is genuinely scarce** in popular regions and instance creation frequently
     returns "out of capacity." Provision early, in whichever region has capacity, and never
     terminate the instance to "recreate it later."
  3. **Take the 200 GB as block storage and keep the fetch cache bounded** (§6). Storage is
     network-attached; treat it as slow, not as an extension of RAM. Swap stays disabled.
- **Known limitation, stated plainly:** this host cannot meet the 15–25s product latency
  target (§4.4). It is chosen because €0 with honest 90–180s reports beats €70/mo with fast
  reports and no revenue. Rung 1 fixes latency the moment there is income to pay for it.
- **Why systemd for `llama-server` specifically**: `Restart=always`, `MemoryMax=`,
  `CPUAffinity=`, and OOM isolation are exactly the controls needed, without container
  GPU-passthrough friction later.
- Deploy = build a static `x86_64-unknown-linux-gnu` binary in GitHub Actions → `scp` →
  `systemctl restart`. A 20-line script. No Kubernetes, no Nomad, no ECS.
- Frontend built in CI, served as static files by Caddy (and optionally fronted by
  Cloudflare free tier for DDoS protection and global asset caching). **If Cloudflare
  proxies the API host, verify SSE is not buffered** — a CDN that buffers `text/event-stream`
  turns progressive streaming into a single delivery at the end, silently destroying the
  product's core UX mechanism. Test this deliberately; it fails quietly.
- Backups: `pg_dump` nightly + WAL archiving to Backblaze B2 or Cloudflare R2 (~$1/mo;
  R2 has no egress fees and is the better pick if Cloudflare is already in the stack).
  Restore drill in Phase 3, repeated quarterly. **An untested backup is not a backup.**
- Deploy restarts drop in-flight SSE connections. This is acceptable only because the client
  reconnects and replays via `Last-Event-ID` (§2.4) — that mechanism is load-bearing for
  deploys, not just for phone locks.

**Explicitly: can one free machine serve early traffic?** Yes, for validation and early
users, with the numbers stated plainly: roughly **60–120 analyses/day** before queueing
becomes user-visible, heavily dependent on cache hit rate. That is enough to find out whether
anyone wants this. It is *not* enough for a successful launch, which is why Phase 6 gates the
public launch on Rung 1 being affordable.

The binding constraint is inference, never Rust: axum on 4 ARM cores still handles hundreds of
req/s of non-LLM traffic, and Postgres at this data volume is idle. The correct scaling reflex
is therefore *always* "improve cache hit rate, then buy inference capacity," never "add app
servers."

### 11.2 Scale path

The web tier never moves. Every step below buys **inference capacity only**, which is the
whole point of the split — it keeps the free tier working for everything it is good at.

1. **Rung 1:** add a dedicated CPU box running only `llama-server`; point `LLAMA_BASE_URL` at
   it over WireGuard. Web tier stays on Oracle Free. Backend code unchanged.
2. **Rung 2:** replace that box with a GPU box. Again just a URL.
3. **Rung 3:** larger-VRAM GPU for 70B-class or large-MoE models.
4. Split `--role worker` onto its own host if job throughput ever demands it; both talk to
   the same Postgres.
5. Multiple inference nodes behind a least-loaded router (`inference_nodes` table).
6. Managed Postgres only when backup/HA operational load exceeds the founder's tolerance —
   and note this is the first step that *removes* a free-tier benefit, so it should be late.

### 11.3 Environments

`local` (docker compose + a 4B model), `staging` (small VPS, real Stripe test mode,
same model as prod at lower `--parallel`), `production`. Migrations run on deploy, forward-only,
always backward-compatible for one release so rollback is safe.

### 11.4 Security baseline

**SSRF protection is the highest-severity control in this system and is called out first
for that reason.** The core feature is "fetch a URL a stranger typed," which makes the
backend a general-purpose request proxy unless it is explicitly prevented from being one.
Required, on every outbound fetch including redirect targets:

- Resolve the hostname, then validate the **resolved IP** against a denylist — RFC1918
  private ranges, loopback, link-local (`169.254.0.0/16`, including the cloud metadata
  endpoint `169.254.169.254`), IPv6 equivalents (`::1`, `fc00::/7`, `fe80::/10`), and
  IPv4-mapped IPv6 forms. **Validating the hostname string alone is defeated by DNS
  rebinding**; resolve-then-connect-to-that-IP is the only correct pattern.
- Re-validate after **every** redirect — a permitted host may 302 to `127.0.0.1`.
- Restrict schemes to `http`/`https` (no `file:`, `gopher:`, `ftp:`).
- Cap response size (2MB) and time (8s), and never echo raw fetch errors to the user in a
  form that discloses internal network topology.
- Treat this as security-critical code: 100% human review, and unit tests covering
  rebinding, redirect chains, and IPv6 forms.

**Second-highest severity: user-supplied provider credentials** (§4.8). Session-only by
default; XChaCha20-Poly1305 at rest with the master key in a `0600` `EnvironmentFile`; a
`Secret<String>` newtype with redacting `Debug`; a CI check that fails on any format string
interpolating it; no reveal endpoint; excluded from ordinary backups; and user-supplied base
URLs routed through the same SSRF guard above — a custom endpoint is the fetch-a-stranger's-URL
problem again, this time with our credential handling attached.

The rest of the baseline: Argon2id for any passwords; magic links single-use, 15-min TTL,
constant-time compare; `SameSite=Lax` `HttpOnly` `Secure` session cookies; CSRF tokens on
state-changing forms; strict CSP; sqlx parameterized queries throughout; secrets in systemd
`EnvironmentFile` with `0600`, never in the repo; `cargo audit` + `cargo deny` in CI;
KB posts sanitized (Markdown → safe subset, no raw HTML) and rate-limited.
