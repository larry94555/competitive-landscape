# Landscape — Product & Engineering Roadmap

**Public-data competitive intelligence in under 60 seconds.**
TypeScript + React frontend · Rust backend · local llama.cpp inference.

> Status: **initial roadmap, pending human review.** Nothing in this repository is
> implemented yet. Every latency and throughput figure is a *design target* to be confirmed
> by the Phase 0 benchmark harness, not a measurement.

---

## Document index

| Doc | Covers |
|---|---|
| **ROADMAP.md** (this file) | Executive summary, phased plan (**C**), metrics (**F**), solo-founder execution (**G**), risks (**H**), git/PR workflow |
| [PRODUCT_SPEC.md](PRODUCT_SPEC.md) | Product & UX specification (**A**): user flows, report schema, notification UX, zero-learning-curve mechanisms |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Technical architecture & stack (**B**): React, Rust, llama.cpp, data, jobs, caching, PDF, email, Stripe, change detection, hosting |
| [ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) | Companion to the above: every technology explained — what it is, the alternatives, the justification, and the cost/benefit trade-off |
| [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) | Support system design (**D**): the open "slash-lite" knowledge base |
| [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) | Quality & trust guardrails (**E**): anti-hallucination stack, evaluation, feedback loops, legal posture |

---

## 1. Executive summary

### What we are building

One text box. A user types a product name, a list of competitors, a URL, or a paragraph
describing what they're building. Within 15–25 seconds they are reading a structured,
source-cited competitive analysis in a fixed seven-section format, streamed section by
section. One click produces a clean one-page PDF. Two more clicks put the competitor's
pricing page and changelog on watch, and they get an email when something meaningful
changes — with an AI summary of what changed and why it might matter.

Everything is grounded in public web sources. When a fact isn't publicly available, the
report says so and lists what was checked. Nothing is estimated, inferred, or invented.

### Why this stack is viable

**Local llama.cpp inference is the right call here, and not merely an acceptable one.**

- **Unit economics invert.** The dominant cost of an LLM product is normally per-token
  inference, which scales linearly with usage and caps gross margin. Here the cost is a
  fixed €60–250/month box. The 500th analysis of the day is free. That makes a genuinely
  generous free tier — the single strongest acquisition lever for a zero-marketing-budget
  solo product — economically sane instead of suicidal.
- **The workload suits small models.** This is not open-ended reasoning. It is
  read-this-page → emit-structured-facts-with-quotes, executed over 8–14 sources, then
  assembled. With retrieval grounding, GBNF-constrained decoding, and mechanical
  verification of every citation, an 8B–14B model at Q4_K_M is not a compromise — the
  verification layer makes it *more* reliable than an unconstrained frontier model, because
  a claim whose quote does not appear in its cited source is deleted before the user sees it.
- **Privacy becomes a feature.** "Your competitive research never leaves our server, and is
  never sent to any AI vendor" is a real, checkable differentiator for exactly the
  strategy-and-product buyers this tool serves.
- **No vendor risk.** No rate limits, no price changes, no deprecation notices, no terms
  changes on the critical path.

**Rust is the right backend** because the workload is I/O-heavy fan-out (10+ concurrent
polite fetches per analysis, hundreds of scheduled change checks) sharing a box with an
inference engine that wants every spare core and gigabyte. Predictable memory, no GC
pauses, and low idle footprint mean the app tier stays out of the model's way. `axum` +
`tokio` + `sqlx` is a mature, boring, well-documented core; a single static binary deploys
by `scp`. And the report schema defined once in Rust generates the TypeScript types, the
API contract, *and* the decoding grammar — one source of truth for the product's central
data structure.

**TypeScript + React is the right frontend** because the entire perceived-quality problem
is a streaming problem. Local generation is slower than a hosted API, so the UI must show
sources landing and prose arriving continuously rather than a spinner. React with SSE and
a strictly-typed discriminated-union event stream handles that cleanly, and strict-mode TS
over generated report types means a schema change breaks the build rather than the product.

### The honest tension, stated up front

The 15–25 second target is **not achievable on a CPU-only box for a full-length synthesis**.
Phase 0 exists to measure this precisely. The plan resolves it with architecture, not hope:
map-reduce over sources with tiny per-source outputs, section-parallel generation across
llama.cpp slots, aggressive multi-layer caching (two users analyzing the same competitor
share all reading work), extractive-first synthesis with a ~600-token generation budget,
and progressive streaming so the first content appears in 4–8 seconds. That ships on a
€60/month CPU box. The €180/month GPU box moves p50 comfortably inside target and is
triggered by a defined metric, not by ambition.

### Shape of the plan

| Phase | Weeks | Outcome |
|---|---|---|
| 0 | 1–2 | Foundations, model bake-off, latency truth |
| 1 | 3–6 | Anonymous end-to-end analysis, streamed |
| 2 | 7–9 | Grounding verification, PDF, eval suite |
| 3 | 10–12 | Accounts, limits, public knowledge base |
| 4 | 13–15 | Stripe, tiers, upgrade flow |
| 5 | 16–19 | Watchlists, change detection, alerts |
| 6 | 20–24 | Cold start: launch, SEO surface, first revenue |
| 7 | 25–30 | Retention, scale, GPU, model upgrade loop |
| 8 | 31+ | Sustainability and margin |

Target: **~$1.5–2k MRR by month 8**, against ~$250/month of infrastructure. That is
profitable in absolute terms early, and the constraint on growth is distribution, not
compute — which is the correct problem to have.

---

## 2. Phased implementation roadmap (C)

Assumes one technical founder at ~30–40 focused hours/week, augmented by coding agents
(see §4). Every phase has instrumentation and evaluation work; none is optional.

---

### Phase 0 — Foundations & model bake-off (Weeks 1–2)

**Goal:** replace every assumption in this document with a measurement, and stand up the
skeleton everything else attaches to.

**Ship:** nothing user-facing.

**Technical tasks**
- Cargo workspace per [ARCHITECTURE.md](ARCHITECTURE.md) §3.2; Vite + React + TS strict
  scaffold; GitHub Actions CI (`fmt`, `clippy -D warnings`, `test`, `audit`, `deny`,
  `tsc`, `eslint`, `vitest`).
- Provision the launch box (Hetzner AX52-class dedicated). Caddy, Postgres 16, Redis,
  systemd units, `pg_dump`→B2 backups **plus a restore drill**.
- Build `llama-server` from source; supervise via systemd with `Restart=always` and
  `MemoryMax=`.
- **Benchmark harness** (`landscape-bench`): for each candidate model × quantization,
  measure prompt-processing tok/s, generation tok/s, RAM, time-to-first-token, and
  throughput at `--parallel 1/2/4/8` on realistic prompts (6k-token source bundle in,
  200-token structured JSON out; and 20k in, 700 out).
- Bake-off across **Qwen3-4B / Qwen3-8B / Qwen3-14B / Llama 3.1 8B / Gemma 3 12B /
  Mistral Small 3.x**, at Q4_K_M and Q5_K_M, plus a Q8_0 reference. **License review
  precedes benchmarking** — a model we cannot use commercially is not a candidate.
- Prove GBNF constrained decoding end-to-end: Rust struct → `schemars` JSON Schema →
  GBNF → llama-server → parsed back into the struct. This is the spine of the product;
  validate it before anything else is built on it.
- Hand-build **10 golden-set subjects** with frozen HTML fixtures and human-written
  reference sheets.

**UX polish:** design tokens, typography scale, the report layout in Figma or static HTML.
Decide the visual language for citations, confidence, and "not found" *now* — they are
structural, and retrofitting them is expensive.

**Support:** register the domain; put up a one-page holding site; open the GitHub repo.

**Instrumentation & eval:** the benchmark harness *is* the deliverable. Publish an internal
`docs/BENCHMARKS.md` with real numbers and the chosen model + quantization, with reasoning.

**Exit criteria**
- A documented model choice with measured tok/s at the target `--parallel`.
- A measured, realistic end-to-end latency estimate for a full analysis, and a written
  decision on whether Rung 1 (CPU) ships or Phase 1 starts on GPU.
- Grammar-constrained JSON round-trip working from Rust with 0 parse failures over 100 runs.

**Cost posture:** ~€70/mo infra, ~€15 domain. No revenue.

---

### Phase 1 — Vertical slice: anonymous analysis (Weeks 3–6)

**Goal:** a stranger types into the box and reads a real, streamed, cited report. This is
the single most important phase; everything after it is commerce and polish.

**Ship**
- `/` composer with one autofocused textarea and three example chips.
- Subject resolution from free-form input.
- Source discovery (SearXNG self-hosted + targeted `/pricing`, `/changelog`, `/blog` probes).
- Polite fetch + extraction pipeline.
- Map-reduce analysis: per-source structured extraction → section synthesis.
- All seven report sections, streamed over SSE.
- Anonymous rate limit (2/day), share URL.

**Technical tasks**
- `landscape-fetch`: robots caching, per-host `governor` limits, conditional GET, timeouts,
  size caps, **SSRF protection on user-supplied URLs** (block private ranges, resolve-then-verify).
- `landscape-search`: `SourceProvider` trait; SearXNG adapter; trust tiering; source ranking
  and capping at 8–14.
- Extraction → Markdown, preserving tables and headings; `extraction_quality` scoring.
- `landscape-llm`: llama-server client with deadlines, token budgets, a global semaphore,
  compiled grammar cache, prefix-stable system prompts.
- `landscape-analyze`: the orchestrator, with per-analysis hard budgets and graceful section
  degradation on timeout.
- SSE endpoint with a per-analysis broadcast channel and a replay ring buffer
  (`Last-Event-ID` resume).
- Postgres schema + job queue (`FOR UPDATE SKIP LOCKED`).
- React: `useAnalysisStream`, stage rail, source cards, seven section components, `Citation`
  hover cards, `NotFoundInPublic`.
- Fetch cache + **per-source extraction cache** (the highest-leverage cache in the system —
  build it in Phase 1, not later).

**UX polish:** the perceived-latency ladder from [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1.
Sources appearing live within 3 seconds is the moment the product stops feeling like a
loading screen; treat it as a P0 requirement.

**Support:** none yet.

**Instrumentation & eval**
- `tracing` spans on every stage; per-analysis record of latency, tokens in/out, sources
  fetched, cache hits, compute-ms.
- Structured event log (`events` table) from day one — retrofitting analytics is painful.
- Golden set to 25 subjects; nightly eval job reporting schema validity, latency, and
  (manually, this phase) spot-checked accuracy.

**Exit criteria**
- 20 consecutive analyses of varied subjects complete without a crash.
- p50 end-to-end ≤ 40s, time-to-first-content ≤ 10s (tightened in Phase 2).
- 100% schema validity.
- The founder would show it to a stranger without apologizing.

**Cost posture:** ~€70/mo. No revenue.

---

### Phase 2 — Grounding verification, PDF & quality (Weeks 7–9)

**Goal:** make the output trustworthy and exportable. This is where the product earns the
right to charge money.

**Ship**
- Layers 3–5 of the anti-hallucination stack ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §2).
- Clarifying questions (≤3, chip-answerable, skippable).
- PDF export: one-page executive summary + full version.
- Evidence strength badges, per-claim confidence, "what we checked" gap blocks.
- Per-section 👍/👎 and "report an inaccuracy."

**Technical tasks**
- `landscape-verify`: label validation, exact→normalized→fuzzy quote matching, entailment
  sanity window, drop accounting, single regeneration retry with negative examples.
- Type-specific validators: price/period/basis consistency, date sanity vs `fetched_at`,
  version and number verbatim checks, superlative stripping, competitor-comparison-page retagging.
- `landscape-pdf`: Typst templates `exec.typ` and `full.typ`; background pre-warm on
  completion; object-storage caching.
- Clarifier: fast grammar-constrained classifier + chip generation.
- Latency work: section-parallel generation across slots; prompt-prefix stabilization;
  context budgeting; section cache.
- Feedback capture and admin review queue v0.

**UX polish:** citation hover cards, the "not found" block, the interpretation label on
SWOT, and the disclaimer line — all shipped together so the trust story reads as one design
rather than a pile of warnings.

**Support:** write the first 10 seed KB articles as static Markdown (published in Phase 3).

**Instrumentation & eval**
- Golden set to 50 subjects, including both traps.
- Full automated eval suite with **CI gates** ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §3.2).
- Daily human review of 5 sampled reports begins here and never stops.
- Dashboards: citation coverage, drop rate, latency percentiles, cache hit rates.

**Exit criteria**
- Citation coverage ≥ 97%; drop rate ≤ 3%; trap subjects produce zero fabricated content.
- p50 ≤ 30s, time-to-first-content ≤ 8s.
- PDF click-to-download ≤ 1s in the pre-warmed case.
- 10 external testers rate report usefulness ≥ 4/5.

**Cost posture:** ~€80/mo (object storage). No revenue.

---

### Phase 3 — Accounts, limits & the public knowledge base (Weeks 10–12)

**Goal:** identity, quotas, and a support system that exists *before* it is needed.

**Ship**
- Magic-link auth, ~90-day sessions.
- Free tier: 10 analyses/month, analysis history, saved reports.
- Usage meter; limit dialogs that state exactly what was blocked.
- `/help`: public searchable KB with slash commands, tags, threads, replies, voting,
  flagging; seeded with 25–30 official articles.
- `/legal/*` pages and the public `/bot` page.
- Admin console v1: usage, quality sample queue, support queue.

**Technical tasks**
- Auth: single-use magic links (15-min TTL, constant-time compare), signed `HttpOnly`
  `SameSite=Lax` cookies, CSRF on state-changing forms.
- Entitlements: `plans` table, `usage_counters`, Redis-backed anonymous quotas, admission
  controller that protects signed-in users from anonymous load.
- `landscape-kb`: threads/replies/tags/votes/flags, Postgres FTS + `pg_trgm`, SSR for
  `/help/*` with `QAPage` JSON-LD, sitemap generation, safe-subset Markdown rendering,
  spam controls.
- React: slash menu, KB search with live search-before-ask, `SessionGate`, usage meter.
- Transactional email (Postmark/Resend) with SPF/DKIM/DMARC — magic-link deliverability is
  now on the critical path.
- GDPR-adjacent basics: export and delete my data.

**UX polish:** account creation must feel like a consequence, not a gate — it is only ever
requested when the user asks for something that requires it.

**Support:** the system goes live with a seeded corpus and a published SLA.

**Instrumentation & eval**
- Funnel events: land → submit → first content → complete → PDF → return.
- KB metrics: search sessions, zero-result searches, threads created, deflection ratio.
- Eval suite runs nightly; human review continues.

**Exit criteria**
- Magic-link delivery ≥ 99% within 60s; login-related support threads ≈ 0.
- KB search returns a relevant result for 80% of the seeded questions.
- Zero-result search queries logged and reviewed weekly (they are the KB backlog).

**Cost posture:** ~€95/mo (email). No revenue.

---

### Phase 4 — Monetization (Weeks 13–15)

**Goal:** be able to take money without breaking the frictionless promise.

**Ship**
- **Starter $19/mo**: 100 analyses/mo, 25 watches (daily), priority queue, full PDF, refresh-bypass.
- **Pro $49/mo**: 500 analyses/mo, 100 watches (6-hourly), instant major alerts,
  Slack/webhook (delivered Phase 7), API access (Phase 7), team seats (later).
- `/pricing`, Stripe Checkout, Billing Portal, upgrade dialogs at the moment of the limit,
  resume-the-blocked-action after upgrade.
- Annual billing at 2 months free.

**Technical tasks**
- `landscape-billing`: `async-stripe`, Checkout sessions, portal sessions, webhook endpoint
  with signature verification and `event.id` idempotency, hourly reconciliation job.
- Entitlement resolution from local tables only — never call Stripe on the request path.
- Priority queue: reserved inference slots for paid tiers; visible "Priority" state.
- Dunning: `invoice.payment_failed` → email → 7-day grace → downgrade (never delete data).
- Tests: Stripe CLI fixtures in CI; seeded fake-clock tests for renewal, upgrade proration,
  cancellation, and dunning.

**UX polish:** the upgrade dialog appears only at a real limit; cancellation is one visible
click. Both rules are anti-support-load measures as much as ethics.

**Support:** billing KB articles (`/quota`, `/pricing`, cancellation, refunds). A written,
published, no-argument refund policy — refund disputes cost more than refunds.

**Instrumentation & eval**
- Revenue events, conversion funnel by limit-hit type, plan distribution, churn.
- Track **which limit** triggers upgrades — it tells you whether the tiers are priced on the
  right axis.

**Exit criteria**
- End-to-end purchase, upgrade, downgrade, and cancel verified in test mode and once live.
- Webhook replay and reconciliation verified by deliberately dropping webhooks in staging.
- Zero cases of a paying user being blocked by anonymous traffic.

**Cost posture:** ~€95/mo + Stripe fees. First revenue possible.

---

### Phase 5 — Watchlists & notifications (Weeks 16–19)

**Goal:** convert a one-shot tool into a recurring habit. **This is the retention phase and
the primary justification for a subscription.**

**Ship**
- Watch creation from a finished report (two clicks, pre-selected sources) and from `/watch`.
- Scheduled change detection with noise suppression.
- LLM importance scoring and change summaries.
- Weekly digest (free) / daily digest + instant major alerts (paid).
- Alert emails with 👍/👎, "see the diff," and "re-run full analysis."
- `/watch` management and per-watch change timeline.

**Technical tasks**
- `landscape-watch`: scheduler with jitter, conditional GET, content-hash short-circuit,
  normalization before hashing, SimHash near-duplicate suppression, `similar` word diffs.
- Grammar-constrained importance scoring: `{importance 0–100, label, category, summary_md,
  why_it_matters}`.
- Digest batching per user per window; signed one-click feedback links; per-watch and global
  unsubscribe with `List-Unsubscribe-Post`; bounce/complaint webhooks auto-disabling delivery.
- Rendered side-by-side diff view.
- Learned thresholds (two 👎 → raise threshold, tell the user).
- **Low-priority scheduling** for watch jobs so they never starve interactive analyses;
  off-peak batching.

**UX polish:** subject lines that state the change, not the notification. Silence as the
default state.

**Support:** notification KB articles, especially "why did I get an alert about something
cosmetic?" — the highest-volume predictable question in this feature.

**Instrumentation & eval**
- Alerts/watch/week, 👍 rate, watch pause/delete rate, re-run click-through.
- A **noise regression suite**: 20 recorded page-change pairs (10 material, 10 cosmetic)
  replayed in CI. Suppression logic must not regress.
- Inference cost of watch checking tracked separately from analysis cost.

**Exit criteria**
- ≥70% 👍 on alerts; ≤2 alerts/watch/week median.
- Zero alerts fired by pure timestamp/counter/testimonial churn in the regression suite.
- Watch checking consumes <25% of daily inference capacity.

**Cost posture:** ~€110/mo. Watches raise inference load; watch caps per tier exist to
bound it.

---

### Phase 6 — Cold start & growth (Weeks 20–24)

**Goal:** first hundred real users, first paying customers, first durable acquisition channel.

**Ship**
- Public launch: Product Hunt, Hacker News (Show HN), relevant subreddits and Slack/Discord
  communities, IndieHackers.
- Shareable public report pages (SSR, indexable, `noindex` unless the user opts to share).
- **Programmatic comparison pages**: pre-generated `/compare/:a-vs-:b` for a curated list of
  ~200 popular SaaS pairs — real reports, refreshed weekly, each an entry point that
  demonstrates the product before signup.
- Landing-page proof: a live example report above the fold, not a screenshot.
- Onboarding email sequence (3 messages, useful not promotional).

**Technical tasks**
- SSR/prerender path for shared reports and comparison pages (a small `axum` route rendering
  the same schema server-side; the SPA hydrates).
- Sitemap, structured data, OG images generated from the report (Typst → PNG).
- Cache warming for the comparison corpus during off-peak hours.
- Abuse controls under launch-day load: admission controller, per-IP limits, a "high demand"
  message that degrades gracefully rather than erroring.
- **Load-test before launch day.** A Show HN spike will hit the inference bottleneck, not
  the web tier; know the queue behavior in advance and set the messaging.

**UX polish:** the first-run experience gets a final usability pass with 5 fresh testers.
Launch day is the one day where a confusing first screen costs disproportionately.

**Support:** expect the largest support spike of the project. Pre-write answers for the
predictable launch questions (accuracy, data sources, privacy, "why not just use ChatGPT",
pricing). Watch the tag distribution daily.

**Instrumentation & eval**
- Acquisition by channel, activation rate, time-to-first-value, D1/D7/D30 retention,
  free→paid conversion, comparison-page → signup conversion.
- Continue nightly eval and daily human review — quality regressions during a traffic spike
  are the worst possible time to discover them.

**Exit criteria**
- ≥500 analyses run by non-founder users.
- ≥100 registered accounts; ≥10 paying customers.
- ≥1 acquisition channel with a repeatable, measurable cost/effort per signup.
- Comparison pages beginning to appear in search results.

**Cost posture:** ~€120/mo + ~€100 one-off launch costs. Target ~$300–500 MRR exiting.

---

### Phase 7 — Retention, scale & model upgrade (Weeks 25–30)

**Goal:** make the product sticky and the infrastructure boring at 10× the traffic.

**Ship**
- Slack and generic webhook alert delivery (Pro).
- Public API (Pro): submit analysis, fetch report JSON, manage watches.
- Saved competitor sets / "my landscape" — a persistent set the user re-runs.
- Weekly "your landscape this week" digest across all watches.
- Admin console v2: model performance, per-tier usage, cost/analysis, quality trends,
  cohort retention.

**Technical tasks**
- **GPU migration** when the trigger fires (p95 > 45s for two consecutive days, or queue
  depth regularly > 3): move `llama-server` to a GPU host; upgrade the synthesizer to
  Qwen3-14B or Qwen3-30B-A3B; enable flash attention and speculative decoding.
- Shadow evaluation of the new model on the golden set, then 10% traffic, then cutover, with
  documented rollback.
- Split `--role worker` to its own process/host; add an `inference_nodes` table and
  least-loaded routing.
- API keys, per-key rate limiting, OpenAPI spec generated from the same schema.
- Cost accounting per analysis in compute-ms, sliced by tier — the input to pricing decisions.

**UX polish:** reduce report time-to-scan. By now there is enough usage data to know which
sections users actually read; reorder or collapse accordingly. This is the first UX change
that should be data-led rather than judgment-led.

**Support:** first staleness sweep of `official` KB threads. Publish a changelog and link
resolved support threads to shipped fixes.

**Instrumentation & eval**
- Golden set to 150 subjects, including every confirmed user-reported error.
- Model A/B results documented in `docs/BENCHMARKS.md`.
- Retention cohorts by acquisition channel and by whether the user created a watch — the
  expected finding is that watch creation is the retention hinge; if so, move watch creation
  earlier in the funnel.

**Exit criteria**
- p50 ≤ 20s, p95 ≤ 35s at 10× Phase 6 traffic.
- Quality gates all passing on the larger model.
- D30 retention ≥ 25% for registered users.
- ~$1k+ MRR.

**Cost posture:** ~€250/mo (GPU box). Should be comfortably covered by revenue; if it is
not, the trigger to migrate had not actually fired.

---

### Phase 8 — Sustainability (Week 31+, ongoing)

**Goal:** a durable, self-sustaining, one-person business.

**Focus areas**
- **Margin discipline**: cost/analysis in compute-ms tracked weekly. Cache hit rate is the
  primary lever; every 10 points of hit rate is a deferred hardware purchase.
- **Depth over breadth**: the temptation is to add ad-library scraping, LinkedIn headcount
  trends, funding databases, and a dozen more sources. Add sources only when the eval suite
  shows they raise fact recall on the golden set. Every source is permanent operational
  surface.
- **Distribution compounding**: the comparison-page corpus and the KB are the two assets
  that appreciate. Both should grow weekly with near-zero marginal effort.
- **Team plans** (shared watchlists, seats) once ≥5 customers ask unprompted.
- **Fine-tuning** (LoRA on human-approved outputs) only if prompting, grammar, and retrieval
  are demonstrably exhausted.
- **Quarterly**: backup restore drill, dependency audit, model re-bake-off (the open-weights
  landscape moves fast enough that a 6-month-old choice is often wrong), KB staleness sweep,
  and a pricing review.

**Exit criteria (steady state)**
- MRR > 4× infrastructure cost.
- Founder time: ≤15 min/day support, ≤1 day/week operations.
- Quality gates green for 90 consecutive days.

---

## 3. Metrics & success criteria (F)

Instrumented from Phase 1. Reviewed weekly; a dashboard the founder actually opens.

### 3.1 Product & funnel

| Metric | Definition | Target |
|---|---|---|
| **Time to first value** | Land → first section streaming | p50 ≤ 10s |
| **First-run completion** | % of first-time visitors who submit and read a complete report | ≥ 60% |
| **PDF rate** | % of completed reports downloaded | ≥ 25% |
| **Activation** | New user runs ≥2 analyses **or** creates a watch in 7 days | ≥ 40% |
| **Watch adoption** | % of registered users with ≥1 watch | ≥ 35% |
| **D7 / D30 retention** | Registered users returning | ≥ 40% / ≥ 25% |
| **Free → paid** | Conversion within 30 days of signup | ≥ 3% |
| **Churn** | Monthly paid logo churn | ≤ 6% |

### 3.2 Compute & latency

| Metric | Target |
|---|---|
| End-to-end analysis latency | p50 ≤ 25s, p95 ≤ 45s |
| Time to first streamed content | p50 ≤ 8s |
| Tokens generated per analysis | ≤ 2,500 |
| **Compute-ms per analysis** (the real unit cost) | trending down |
| Cache hit rate — fetch / extraction / section | ≥ 60% / ≥ 45% / ≥ 25% |
| Inference queue depth | p95 ≤ 2 |
| Slot utilization | 40–75% (below is waste; above is queueing) |
| Watch-check inference share | ≤ 25% of daily capacity |
| Infra cost / paying customer | ≤ $4/mo |

### 3.3 Quality — the measurable definition of "high quality"

A report is high quality when **all** of:

1. **Citation coverage ≥ 97%** — claims with a verified evidence quote.
2. **Hallucination rate ≤ 3%** — Layer 3 drop rate.
3. **Fact precision ≥ 95%**, **fact recall ≥ 70%** against golden-set reference sheets.
4. **Refusal correctness ≥ 98%** — genuinely-absent facts reported as `not_found`.
5. **Schema validity 100%**.
6. **Human rubric ≥ 4/5** on the daily 5-report sample.
7. **User 👍 rate ≥ 80%** on section feedback.
8. **Zero fabrication on trap subjects** — a hard gate, never traded away.

### 3.4 Quality — the measurable definition of "frictionless"

1. **No-instruction success**: ≥4 of 5 unmoderated testers reach a downloaded PDF within
   90 seconds, having read nothing.
2. **Zero required fields** beyond the one textarea.
3. **≤1 clarifying question** in ≥80% of analyses; 100% skippable to a complete report.
4. **Zero documentation reads** required for the core flow — measured as `/help` visits
   *before* a first completed analysis (target < 5%).
5. **Support threads per 100 analyses** trending down month over month.
6. **Signup-to-first-analysis time**: p50 ≤ 60s.

### 3.5 Support

| Metric | Target |
|---|---|
| KB deflection (help sessions ending without a new post) | ≥ 60% by month 6 |
| Zero-result KB searches | ≤ 15% of searches |
| Private share of support volume | ≤ 20% by month 6 |
| Founder support time | ≤ 15 min/day |
| Median first response — public / private | same day / ≤ 2 business days |

### 3.6 Notification health

Alerts/watch/week ≤ 2 · 👍 rate ≥ 70% · watch delete rate ≤ 10%/mo ·
re-run-analysis click-through ≥ 15% · false-positive rate (cosmetic alerts) ≤ 5%.

---

## 4. Solopreneur / agent-assisted execution (G)

### 4.1 The division of labor

| Heavily automate with coding agents | Requires human judgment |
|---|---|
| CRUD endpoints, sqlx queries, migrations | **Model & quantization selection** (read benchmarks, decide the tradeoff) |
| React components from a specified design | **Prompt engineering** (agents write plausible prompts; only evals tell truth) |
| Test scaffolding, fixtures, wiremock stubs | **Quality gate thresholds** — what counts as good enough |
| Stripe webhook plumbing and idempotency | **The report schema** — the product's core contract |
| Typst PDF templates | **Trust presentation** — how citations, confidence, and gaps look |
| Email templates, MJML → HTML | **The first-run experience** — every pixel above the fold |
| CI/CD, systemd units, deploy scripts | **Pricing and tier boundaries** |
| KB CRUD, FTS queries, moderation plumbing | **Reading the daily 5-report quality sample** |
| Boilerplate refactors, dependency upgrades | **Whether to add a data source** (permanent ops surface) |
| Docs, changelogs, seed KB article drafts | **Launch messaging and positioning** |

**The rule:** agents write code; the founder decides what "correct" means. Anything that
defines correctness — schemas, evals, thresholds, rubrics — is authored by hand and reviewed
line by line. Everything downstream of a good specification can be delegated aggressively.

### 4.2 Working practices that make this sustainable

- **Evals are the contract with the agents.** An agent that can run `cargo test` and the
  golden-set eval can iterate on the analysis pipeline safely. Without the eval suite,
  agent-written prompt changes are unfalsifiable — which is why Phase 2 builds it before
  Phase 3 accelerates.
- **One PR per concern, always reviewed.** Agent-generated code is read, not merged on faith,
  particularly anything touching auth, billing, SSRF, or the verification layer. Those four
  areas are hand-reviewed 100% of the time.
- **Timebox the infrastructure.** Kubernetes, microservices, and a custom design system are
  all traps. The deploy script is 20 lines and stays that way until it demonstrably hurts.
- **Batch the founder-only work.** Quality review, support, and prompt iteration go in one
  daily 45-minute block. Context-switching is the scarcest resource in a one-person company.
- **Write the decision down.** `docs/BENCHMARKS.md`, `docs/DECISIONS.md` (ADR-lite). In six
  months you will not remember why Q5_K_M lost, and re-deciding costs more than recording.
- **Ship on a fixed cadence** (weekly). A solo founder's real failure mode is an eight-week
  refactor nobody asked for.

### 4.3 What to buy rather than build

Stripe Billing Portal · Postmark/Resend · Hetzner (not a cloud provider's GPU pricing) ·
Cloudflare free tier · Backblaze B2 · Plausible or self-hosted analytics.
**Build**: the analysis engine, the verification layer, the KB. Those are the product.

---

## 5. Risks & mitigations (H)

### R1 — Local inference latency and hardware limits
*Risk:* CPU-only inference cannot meet 15–25s for full synthesis; users abandon.
**Likelihood: high. Impact: high.**
*Mitigations:* Phase 0 measures this before a line of product code depends on it;
map-reduce with tiny per-source outputs; section-parallel generation across slots;
extractive-first synthesis at ~600 generated tokens; progressive streaming so
time-to-first-content is 4–8s; honest queue-position display; a defined metric trigger for
the GPU box; speculative decoding and prefix caching as further levers.
*Early warning:* p95 latency, queue depth, abandonment rate mid-stream.

### R2 — Free-tier resource exhaustion
*Risk:* fixed capacity means free users can starve paying ones — the failure mode unique to
self-hosted inference.
**Likelihood: medium. Impact: high.**
*Mitigations:* global inference semaphore with **reserved slots for paid tiers**; admission
controller that sheds anonymous load with a clear message rather than queueing it; tight
anonymous limits (2/day) on hashed IP + fingerprint; aggressive full-analysis caching so
popular subjects cost nothing after the first run; watch checks at low priority and off-peak;
per-analysis hard budgets.
*Early warning:* paid-user queue wait time — should be ~0 always; if it isn't, shed free load.

### R3 — Content quality from a small local model
*Risk:* an 8B model fabricates a price and a user takes it to a board meeting.
**Likelihood: medium. Impact: severe — this is an existential trust risk.**
*Mitigations:* the entire five-layer stack in [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) —
retrieval gating, grammar-enforced citations, mechanical quote verification (unsupported
claims are *deleted*, not flagged), type-specific validators for prices and dates, and
honest presentation of confidence and gaps. Plus CI quality gates, trap subjects, daily human
review, and user error reporting that feeds the golden set.
*Early warning:* drop rate, 👎 rate by section, inaccuracy reports per 100 analyses.

### R4 — Notification noise
*Risk:* alerts on cosmetic changes; users mute, then churn. Noise is how monitoring products
die quietly.
**Likelihood: high. Impact: medium.**
*Mitigations:* normalize-before-hash; SimHash near-duplicate suppression; per-element
selectors for known page types; LLM importance scoring with a threshold; digest-by-default;
learned per-watch thresholds from one-click feedback; a CI noise regression suite of recorded
material/cosmetic change pairs.
*Early warning:* alerts/watch/week, 👎 rate, unsubscribe rate.

### R5 — Support load overwhelming one person
*Risk:* support consumes the time that should build the product.
**Likelihood: medium. Impact: medium.**
*Mitigations:* the whole design in [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) — public-first
answers, search-before-ask, promote-to-official, generalize-and-publish from private threads,
contextual deep links, and product fixes for the two predictable ticket generators (alert
noise, surprise limits). Seeded corpus before launch. A 15-min/day budget with monthly tag
review to find the *product* cause of recurring questions.
*Early warning:* threads per 100 analyses; private share of volume; tag concentration.

### R6 — Competition from free cloud LLMs ("why not just ask ChatGPT?")
*Risk:* the obvious objection, asked by every visitor and every commenter on launch day.
**Likelihood: certain. Impact: medium.**
*Mitigations:* answer it in the product, not in marketing copy. A frontier chatbot will
confidently state a stale or invented price with no citation; Landscape fetches the pricing
page *now*, quotes it, timestamps it, deletes anything it cannot verify, and tells you what
it could not find. It also **watches for changes**, which no chat session does. Reinforced by:
a fixed scannable schema, an instant presentable PDF, and a genuine privacy claim (nothing
leaves the server). The moat is the verification pipeline and the monitoring loop — not the
model. Have this answer pre-written for launch day.
*Early warning:* the objection appearing in support threads and launch comments; conversion
from comparison pages.

### R7 — Operational complexity of running llama.cpp in production
*Risk:* OOM kills, CUDA faults, a bad upstream release, model files filling the disk,
2 a.m. restarts.
**Likelihood: medium. Impact: medium.**
*Mitigations:* sidecar process isolation — an inference crash never takes down the API;
systemd `Restart=always` with `MemoryMax=`; health checks with automatic circuit-breaking to
a "temporarily unavailable, your analysis is queued" state; pinned llama.cpp versions
upgraded deliberately with a re-run of the eval suite; old GGUFs retained for one-config
rollback; disk quotas and alerting; a documented 10-minute runbook for the three known
failure modes.
*Early warning:* llama-server restart count, health-check failures, slot errors, disk usage.

### R8 — Legal / ethical exposure from public-data fetching
*Risk:* a site owner objects; a ToS complaint; a copyright concern over quoted material.
**Likelihood: low–medium. Impact: medium.**
*Mitigations:* robots.txt honored; honest user-agent with a public bot page; per-host rate
limiting; no paywall or login circumvention; short attributed quotes with a grammar-enforced
300-character cap; a public exclusion request form honored within 5 business days; clear
disclaimers; no individual-person targeting.
*Early warning:* exclusion requests; abuse complaints to the host.

### R9 — Distribution failure (the most likely way this dies)
*Risk:* the product is good and nobody sees it. For a solo technical founder this is more
probable than any technical risk in this list.
**Likelihood: high. Impact: severe.**
*Mitigations:* build the compounding assets early — programmatic comparison pages and an
indexable KB both start in Phase 3–6 and appreciate weekly; shareable public reports make
every user a distribution channel; launch to multiple communities rather than one; a
generous free tier is affordable *precisely because* inference is local — lean on it hard.
*Early warning:* organic signups per week; comparison-page impressions; share-link opens.

### R10 — Founder burnout / scope creep
*Risk:* an eight-week refactor, a design system, a Kubernetes migration.
**Likelihood: medium. Impact: high.**
*Mitigations:* fixed weekly ship cadence; phase exit criteria written before the phase
starts; a hard rule that new data sources must prove themselves on the golden set;
buy-don't-build defaults; agents on everything downstream of a specification.

---

## 6. Git / pull-request workflow (mandatory)

This roadmap and every subsequent change to it follow a strict PR workflow.

1. **Every roadmap change is a pull request.** No direct commits to the target branch.
2. **PRs are never merged by the agent.** The human reviewer is solely responsible for
   review, approval, and merge.
3. **One PR, one focused change.** The initial PR introduces the complete roadmap document
   set. Later PRs make incremental, readable edits — Markdown so diffs review cleanly.
4. **Commits** are small, with concise messages describing the change in that commit.
5. **PR descriptions** are GitHub-flavored Markdown covering: what changed, why, and what
   the reviewer should scrutinize.
6. **After opening a PR, work stops** until the human has reviewed and merged.
7. Roadmap documents live in `docs/`, with [ROADMAP.md](ROADMAP.md) as the index.

Once implementation begins, code PRs follow the same rules with additional gates: CI green
(`fmt`, `clippy -D warnings`, tests, `cargo audit`, `cargo deny`, `tsc`, `eslint`, `vitest`),
golden-set eval green for anything touching the analysis pipeline, and 100% human review of
changes to auth, billing, SSRF handling, or the verification layer.
