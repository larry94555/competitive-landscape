# Landscape — Product & Engineering Roadmap

**Public-data competitive intelligence in under 60 seconds** — the product goal, delivered
at Rung 2. The free-tier launch host serves the same report in 90–180s; see §6 for the ladder.
TypeScript + React frontend · Rust backend · local llama.cpp inference on Oracle Always Free.

> Status: **initial roadmap, pending human review.** Nothing in this repository is
> implemented yet. Every latency and throughput figure is a *design target* to be confirmed
> by the Phase 0 benchmark harness, not a measurement.

---

## Document index

| Doc | Covers |
|---|---|
| **ROADMAP.md** (this file) | Executive summary, phased plan (**C**), metrics (**F**), solo-founder execution (**G**), risks (**H**), bootstrapped cost ladder, git/PR workflow |
| [PRODUCT_SPEC.md](PRODUCT_SPEC.md) | Product & UX specification (**A**): user flows, report schema, notification UX, zero-learning-curve mechanisms |
| [UI_FLOWS.md](UI_FLOWS.md) | The seven flows: access tiers, conversational follow-up, notifications, admin, community channels — and four conflicts with earlier decisions |
| [DISTRIBUTION.md](DISTRIBUTION.md) | The owning document for R9: beachhead selection, positioning, channels in yield order, the weekly cadence, and the trap-subject launch benchmark |
| [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) | How a prompt becomes a competitor set: input classes, prompt completeness by convergence, category vocabulary resolution, seed channels, relevance classification, clarifying questions, and how company standing is assessed |
| [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) | What the report contains: the nine sections, the chart catalogue, evidence classes, what is deliberately excluded, and the charting decision |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Technical architecture & stack (**B**): React, Rust, llama.cpp, data, jobs, caching, PDF, email, Stripe, change detection, hosting |
| [ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) | Companion to the above: every technology explained — what it is, the alternatives, the justification, and the cost/benefit trade-off |
| [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) | Support system design (**D**): the open "slash-lite" knowledge base |
| [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) | Quality & trust guardrails (**E**): anti-hallucination stack, evaluation, feedback loops, legal posture |
| [FACT_CHECKING.md](FACT_CHECKING.md) | Information gathering & fact-checking: source discovery, the two-axis trust model, competitive-set derivation, the nine-level verification pipeline, misinformation/disinformation handling, and how a reader independently confirms every claim |
| [Fable_Evaluation.md](Fable_Evaluation.md) · [Evaluation_Action_Plan.md](Evaluation_Action_Plan.md) | Pre-implementation evaluation of the whole plan, and the disposition of every finding with its impact on implementation |
| [CODING_QUALITY.md](CODING_QUALITY.md) | The code quality standard: simplicity budgets, design patterns, testing, linting, Sonar, hooks, ADRs, the tutorial, review process, and the agent contract |

---

## 1. Executive summary

### What we are building

One text box. A user types a product name, a list of competitors, a URL, or a paragraph
describing what they're building. They then read a structured, source-cited competitive
analysis in a fixed nine-section format, streamed section by section as it is written —
in 90–180 seconds on the free-tier launch host, and in 15–25 seconds once revenue pays for
Rung 2 (§6). One click produces a clean one-page PDF. Two more clicks put the competitor's
pricing page and changelog on watch, and they get an email when something meaningful
changes — with an AI summary of what changed and why it might matter.

Everything is grounded in public web sources. When a fact isn't publicly available, the
report says so and lists what was checked. Nothing is estimated, inferred, or invented.

### Why this stack is viable

**Local llama.cpp inference is the right call here, and not merely an acceptable one.**

- **Unit economics invert.** The dominant cost of an LLM product is normally per-token
  inference, which scales linearly with usage and caps gross margin. Here the cost is a fixed
  box — and at launch that box is **free**: Oracle Cloud Always Free (4 ARM cores, 24 GB).
  The 100th analysis of the day costs nothing. That makes a genuinely generous free tier —
  the single strongest acquisition lever for a zero-marketing-budget solo product —
  economically sane instead of suicidal, and it means the product can reach paying customers
  before it costs a euro to run. With no outside capital, that is not a nice property; it is
  the whole plan (§6).
- **The workload suits small models.** This is not open-ended reasoning. It is
  read-this-page → emit-structured-facts-with-quotes, executed over 8–14 sources, then
  assembled — with prices, dates and versions parsed deterministically by code rather than
  generated at all. With retrieval grounding, GBNF-constrained decoding, and mechanical
  verification of every citation, a 4B–8B model at Q4_K_M is not a compromise — the
  verification layer makes it *more* reliable than an unconstrained frontier model, because
  a claim whose quote does not appear in its cited source is deleted before the user sees it.
- **Privacy becomes a feature.** "Your competitive research never leaves our server, and is
  never sent to any AI vendor" is a real, checkable differentiator for exactly the
  strategy-and-product buyers this tool serves. The one exception — a user who opts into
  their own provider key (§Phase 4, BYOK) — is disclosed at the point of choice and marked on
  every affected report, because a privacy claim with a silent exception is worse than no
  claim at all.
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

**The 15–25 second target is not achievable on the free tier, and the plan says so rather
than working around it.** Four Ampere cores cannot run a full-length synthesis that fast, and
on that hardware *prompt processing*, not generation, is the dominant cost. Phase 0 measures
this precisely on the actual instance.

The resolution is architectural, and most of it improves quality at the same time:
**deterministic-first extraction** (prices, dates and versions are parsed by code, not
generated by a model — more accurate *and* it removes most of the prefill), span
pre-selection that cuts each source from ~2,500 to ~400 tokens before the model sees it, tiny
grammar-constrained per-source outputs, section-parallel generation batched across slots, a
≤900-token generation budget, aggressive multi-layer caching (two users analysing the same
competitor share all reading work), and progressive streaming.

That yields an honest **90–180 seconds to a complete report on €0/month**, with first content
in 20–40s. The 15–25s promise arrives at **Rung 2** (~€200/month GPU box), triggered by
revenue — see §6. Until then the landing page promises what the hardware delivers; advertising
Rung 2 speed while serving Rung 0 is the fastest available way to destroy trust.

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

**Week counts are the optimistic case; plan on 1.5×.** Phases 1–2 carry the most added scope
(nine sections, matrix extraction, SVG charts, source classes, two-pass) and are the likeliest
to overrun. **Honest planning assumption: 42–48 weeks to Phase 6, not 20–24.** The delay costs
almost nothing — €15/month tolerates any schedule — but a founder who planned for month 8 and
arrives at month 12 pre-revenue may read a *working slow* plan as a *failed* one. That
misreading is the actual risk (R13), and re-baselining now is the cheap prevention.

Target: **~$1.5–2k MRR by month 8**, against infrastructure that starts at **€0** and reaches
~€200/month only once revenue justifies it (§6). Total cash required to reach first revenue is
under €100. That is profitable in absolute terms early, and the constraint on growth is
distribution, not compute — which is the correct problem to have.

---

## 2. Phased implementation roadmap (C)

Assumes one technical founder at ~30–40 focused hours/week, augmented by coding agents
(see §4). Every phase has instrumentation and evaluation work; none is optional.

---

### Phase 0 — Foundations & model bake-off (Weeks 1–2)

**Goal:** replace every assumption in this document with a measurement, and stand up the
skeleton everything else attaches to.

**Ship:** nothing user-facing — *except* the two founder-run tracks below, which are the most
important work in this phase.

**Track A — concierge validation (parallel, weeks 1–2).** Produce **5–10 competitive reports
by hand**, following [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) and
[FACT_CHECKING.md](FACT_CHECKING.md) exactly — those documents are already an analyst's SOP.
Deliver them to real founders found in the communities Phase 6 plans to launch into, and
**charge a token amount**. Willingness to pay $10 for a hand-made report is stronger evidence
than a hundred waitlist signups.

Why this is not optional: the plan otherwise builds for ~20 weeks before meeting a user, and a
bootstrapped solo project cannot afford to discover in week 20 that the format, the pricing or
the positioning is wrong. Each hand-made report also becomes a **golden-set reference sheet**,
a **testimonial**, and a **format test** — so the work is never wasted even if the concierge
channel is abandoned.

**Track C — the UI prototype (week 1–2).** Build `prototype/ui-prototype.html`: one throwaway
HTML file, canned data, simulated timings, no backend. It tests the one thing no static mockup
can show — whether a 90–180 second wait reads as *work happening* or as a hang — plus whether
citations invite clicking and whether an honest gap reads as rigour. Show it to concierge
recipients (Track A) in the same conversation as their hand-made report.
**Explicitly disposable: it must not become the production frontend.**

**Track B — pre-launch assets (week 1).** Register and stand up a **waitlist landing page**
(the domain starts ageing and the list starts growing from day one); run the **name and
trademark check** before any brand equity accrues; provision the Oracle A1 and **convert the
account to Pay-As-You-Go** immediately — A1 capacity is genuinely scarce and R11 depends on
holding it.

**Technical tasks**
- Cargo workspace per [ARCHITECTURE.md](ARCHITECTURE.md) §3.2; Vite + React + TS strict
  scaffold; GitHub Actions CI (`fmt`, `clippy -D warnings`, `test`, `audit`, `deny`,
  `tsc`, `eslint`, `vitest`).
- **Quality toolchain from commit one** per [CODING_QUALITY.md](CODING_QUALITY.md): workspace
  clippy lints (`unsafe_code = forbid`, `unwrap_used = deny`), `tsc --strict`, ESLint
  type-checked rules, `lefthook` pre-commit hooks, `gitleaks`, `cargo deny`, SonarCloud
  quality gate on new code, coverage reporting, and the budget-annotation report. **Retrofitting
  a quality bar onto an existing codebase does not work** — the gates must predate the code.
- `docs/decisions/` initialised with the ADR template, and `docs/TUTORIAL.md` skeleton with
  its CI link-checker, so both grow with the code rather than being written at the end.
- **CI speed, in payoff order** — the Rust build dominates, so optimize it first:
  `Swatinem/rust-cache` (or `sccache`), Rust and frontend as parallel jobs,
  `cargo-nextest` in place of `cargo test`, `cargo check` before `cargo build` on PRs.
  Then `bun install` for the frontend (~15–30s), after verifying it behaves on Windows
  and does not break Playwright's postinstall browser download — npm is the fallback.
- Start `RUNBOOK.md` (R7 promises one; it has never been scheduled). It grows with the system:
  the three known llama.cpp failure modes, restore-from-backup, and how to put the product into
  a safe read-only away mode (§2A.4).
- **Provision the Oracle Always Free A1** (4 OCPU / 24 GB / aarch64) *first* — A1 capacity is
  scarce in popular regions and this can take several attempts. **Convert the account to
  Pay-As-You-Go before building on it**, so Always Free resources are not reclaimed; staying
  inside the free limits still bills €0. Then Caddy, Postgres 16, Redis, systemd units,
  swap disabled, `pg_dump`→B2/R2 backups **plus a restore drill**.
- Target **`aarch64-unknown-linux-gnu`** in CI from the first commit. A Rust build that has
  only ever run on x86 finds its first ARM bug in production.
- Build `llama-server` on the host with ARM `dotprod`/`i8mm` enabled; three supervised
  processes per [ARCHITECTURE.md](ARCHITECTURE.md) §4.7, each with `--mlock`, `Restart=always`
  and `MemoryMax=`.
- **Benchmark harness** (`landscape-bench`), run **on the actual A1 instance** — laptop
  numbers are worthless here. For each candidate model × quantization measure prefill tok/s,
  generation tok/s, resident RAM, time-to-first-token, and aggregate throughput at
  `--parallel 1/2/4/8`, on realistic prompts (400-token span window in → 100-token structured
  JSON out; 4k-token bundle in → 700 out).
- Bake-off across **Qwen3-1.7B / 4B / 8B / 14B**, plus **Gemma 3 4B/12B** and
  **Llama 3.2 3B** as alternates, at **Q4_K_M *and* Q4_0** (ARM repacking may make the
  lower-quality format the faster one — measure, don't assume), with a Q8_0 reference.
  **License review precedes benchmarking** — a model we cannot use commercially is not a
  candidate.
- **Prefill is the thing to measure most carefully.** On 4 ARM cores it dominates, and the
  span-pre-selection design (§5.4) lives or dies on these numbers.
- Validate `q8_0` KV cache quantization against the golden set — three resident models share
  a tight KV budget, so this decision cannot be deferred as it could on a 64 GB box.
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
- A documented three-model choice (Router / Extractor / Synthesizer) with measured prefill
  and generation tok/s on the A1, at the chosen `--parallel`, fitting the ~17 GB budget.
- **A measured, realistic end-to-end latency estimate**, and a written, honest decision on
  what the Rung-0 latency promise will be. If the measurement is materially worse than the
  90–180s estimate in ARCHITECTURE §4.4, the response is to cut scope per analysis (fewer
  sources, tighter span windows, smaller synthesizer) — **not** to quietly ship a promise the
  hardware cannot keep.
- Grammar-constrained JSON round-trip working from Rust with 0 parse failures over 100 runs.
- A written decision on the observability backend: `tracing` instrumentation is fixed, but
  self-hosted Prometheus/Grafana/Tempo competes for RAM with the model on a 24 GB box.
  On this hardware the answer is almost certainly "hosted error tracking only, for now."
- `q8_0` KV quantization decided on evidence, and the `--mlock` + no-swap configuration
  verified under load.
- **Concierge track: ≥5 reports delivered to real people**, with their reactions recorded —
  what they read first, what they ignored, what they asked for, whether they would pay.
- **Review-platform access decision recorded**, with the discovery channel ranking updated to
  match reality.
- Every merge gate in [CODING_QUALITY.md](CODING_QUALITY.md) §10.4 green on an empty
  repository, so the first real PR meets the bar rather than establishing a lower one.

**Cost posture:** **€0/mo infra**, ~€15 domain. No revenue. See §6.4.

---

### Phase 1 — Vertical slice: anonymous analysis (Weeks 3–6)

**Goal:** a stranger types into the box and reads a real, streamed, cited report. This is
the single most important phase; everything after it is commerce and polish.

**Ship**
- `/` composer with one autofocused textarea and three example chips.
- Subject resolution from free-form input; input classification and the named-set /
  named-single discovery paths ([COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §10).
- **Entity resolution before any fetch**, with the disambiguation gate
  ([FACT_CHECKING.md](FACT_CHECKING.md) §3.1) — wrong entity resolution produces a report that
  is wrong throughout yet internally consistent and fully cited.
- Source discovery: **structured probes first** (`/pricing`, `/changelog`, `sitemap.xml`,
  `llms.txt`, docs, status, public ATS boards), templated search second (SearXNG self-hosted),
  adapters third. Probes are deterministic, free, and hit primary sources; search fills gaps
  rather than leading (FACT_CHECKING §3.3).
- Per-section coverage thresholds and computed evidence strength (FACT_CHECKING §3.5), so a
  thin report is shipped marked rather than silently guessed.
- Polite fetch + extraction pipeline.
- Map-reduce analysis: per-source structured extraction → section synthesis.
- All seven report sections, streamed over SSE.
- Anonymous rate limit (2/day), share URL.

**Technical tasks**
- `landscape-fetch`: robots caching, per-host `governor` limits, conditional GET, timeouts,
  size caps, **SSRF protection on user-supplied URLs** (block private ranges, resolve-then-verify).
- **Review-platform access audit** (§ week 1, before any architecture assumes the channel):
  [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) ranks review-site category pages as the
  **highest-yield discovery channel**, and the sentiment section leans on the same platforms —
  but the plan honours robots.txt as a hard ethical commitment. **These two positions may
  collide.** Audit the named platforms' robots policies and terms, record the result, and if
  access is disallowed, promote the documented fallbacks (vendor `/alternatives` pages,
  marketplaces, community threads) to the primary path *by decision rather than by discovery
  in week 8*.
- **Instrument the JS-rendering gap** ([ARCHITECTURE.md](ARCHITECTURE.md) §5.5): two counters —
  what share of pricing pages yield no price from static HTML, and what share of those tiers
  2–4 will recover. One day of work, and it decides whether a browser tier is ever built.
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
- p50 end-to-end ≤ 240s, time-to-first-content ≤ 45s on Rung 0 (tightened in Phase 2).
- 100% schema validity.
- The founder would show it to a stranger without apologizing.

**Cost posture:** €0/mo. No revenue.

---

### Phase 2 — Grounding verification, PDF & quality (Weeks 7–9)

**Goal:** make the output trustworthy and exportable. This is where the product earns the
right to charge money.

**Ship**
- **The feature comparison matrix and pricing comparison** — the backbone of a real
  competitive analysis ([COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md)
  §3), including the five-state cell model where *not found* is distinct from *no*.
- `landscape-charts`: static SVG emitters for the **feature matrix** and **cost-at-scale
  curve**, rendered identically in the web report and the Typst PDF. Charts consume zero
  model tokens, so they improve the report while *reducing* the generation budget.
- Layers 3–5 of the anti-hallucination stack ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §2).
- **Discovery from a category or a product idea** — vocabulary resolution (the user's words to
  the market's words), seed harvesting across independent channels, relevance classification
  into direct / adjacent / substitute, and the editable interpretation header
  ([COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §4–§6). This is what lets someone
  describe an idea rather than name competitors.
- Clarifying questions (≤3, chip-answerable, skippable), fired **only when discovery fails to
  converge** — not when the prompt looks incomplete.
- **Tiers 2–4 of the rendering ladder** ([ARCHITECTURE.md](ARCHITECTURE.md) §5.5): embedded-state
  extraction (`__NEXT_DATA__`, `__NUXT__`, JSON-LD), discovered JSON endpoints, and archive
  fallback. ~2–3 days, €0, and a *better* data path than rendering where it applies — structured
  data instead of scraped text.
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

**Demo pipeline** ([CODING_QUALITY.md](CODING_QUALITY.md) §9.5): Playwright `@demo` specs,
the HTML code-walkthrough deck, ffmpeg caption burn-in, the `demo-assets` orphan branch with
its pruning job, and the CI gate requiring a demo for user-visible changes. Built here rather
than later because Phase 2 is the first phase with a UI worth demonstrating, and because a
silent, subtitled 90-second video is the only review artifact that shows *progressive
streaming* — which no screenshot can. Free on a public repository.

**Support:** write the first 10 seed KB articles as static Markdown (published in Phase 3).

**Instrumentation & eval**
- Golden set to 50 subjects, including both traps, with **matrix and chart-data assertions** —
  a chart plotting the wrong number is a fact error and must fail the same gates as a
  hallucinated price.
- Full automated eval suite with **CI gates** ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §3.2).
- Daily human review of 5 sampled reports begins here and never stops.
- Dashboards: citation coverage, drop rate, latency percentiles, cache hit rates.

**Exit criteria**
- Citation coverage ≥ 97%; drop rate ≤ 3%; trap subjects produce zero fabricated content.
- **JS-rendering gap re-measured after tiers 2–4.** If the residual is under ~5%, the browser
  tier is **not built** and the honest-gap treatment stands. If it is material, tier 5 and the
  two-pass flow (PRODUCT_SPEC §2.1A) are scheduled — with a written decision either way.
- p50 ≤ 180s, time-to-first-content ≤ 40s on Rung 0. (Rung 2 equivalents: 25s / 8s.)
- PDF click-to-download ≤ 1s in the pre-warmed case.
- 10 external testers rate report usefulness ≥ 4/5.

**Cost posture:** €0/mo (object storage ~€1). No revenue.

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

**Cost posture:** ~€15/mo (email). No revenue.

---

### Phase 4 — Monetization (Weeks 13–15)

**Goal:** be able to take money without breaking the frictionless promise.

**Ship**
- **Bring-your-own-key (BYOK)** — optional user-supplied OpenAI-compatible or Anthropic
  credentials ([ARCHITECTURE.md](ARCHITECTURE.md) §4.8,
  [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §5A). Shipped here rather than later because it is the
  **primary mitigation for R12** — a user who finds free-tier latency intolerable can fix it
  themselves, today, at their own cost, instead of churning.
- **Starter $19/mo**: 100 analyses/mo, 25 watches (daily), priority queue, full PDF, refresh-bypass.
- **Pro $49/mo**: 500 analyses/mo, 100 watches (6-hourly), instant major alerts,
  Slack/webhook (delivered Phase 7), API access (Phase 7), team seats (later).
- `/pricing`, Stripe Checkout, Billing Portal, upgrade dialogs at the moment of the limit,
  resume-the-blocked-action after upgrade.
- Annual billing at 2 months free.

**Decision required before writing billing code: merchant of record, or not?**
Stripe is a payment processor — VAT, GST, and US sales-tax registration, calculation, and
remittance remain *our* legal obligation. Paddle or Lemon Squeezy become the legal seller and
absorb that entirely, at a higher fee and a worse API. The roadmap recommends Stripe, but
this must be an explicit, recorded choice made **now**, because switching providers after
customers exist is genuinely painful. If Stripe: budget for Stripe Tax and identify the
registration thresholds that will eventually be crossed. See
[ARCHITECTURE.md](ARCHITECTURE.md) §9 and
[ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) §7.

**Technical tasks**
- `landscape-billing`: `async-stripe`, Checkout sessions, portal sessions, webhook endpoint
  with signature verification and `event.id` idempotency, hourly reconciliation job.
- Keep Stripe types out of domain code — `async-stripe` is community-maintained with no
  official Stripe backing, and the whole SDK surface should stay replaceable in a day.
- `landscape-llm`: `openai_compatible` and `anthropic` adapters behind the existing
  `LlmClient` trait, each declaring its `ProviderCapabilities`. The same schemars JSON Schema
  drives GBNF, OpenAI strict schema, and Anthropic tool `input_schema` — no second definition
  of the report contract.
- BYOK key handling: session-only by default; XChaCha20-Poly1305 at rest when persisted;
  `Secret<String>` newtype with redacting `Debug`; **a CI check that fails the build on any
  format string interpolating a secret**; no reveal endpoint; excluded from ordinary backups;
  custom base URLs through the SSRF guard.
- Automatic fallback to the local model on provider failure, with a visible report notice and
  a single (not per-failure) notification email.
- Provenance: `inference_provider`, `byok_key_id`, `fell_back_to_local` on every analysis,
  surfaced on the report, in the PDF footer, and sliced in every quality metric.
- Quota model: BYOK raises the analysis limit substantially; watches, cadence, webhooks and
  API stay tier-gated, because none of those is an inference cost.
- Entitlement resolution from local tables only — never call Stripe on the request path.
- Priority queue: reserved inference slots for paid tiers; visible "Priority" state.
- Dunning: `invoice.payment_failed` → email → 7-day grace → downgrade (never delete data).
- Tests: Stripe CLI fixtures in CI; seeded fake-clock tests for renewal, upgrade proration,
  cancellation, and dunning.

**UX polish:** the upgrade dialog appears only at a real limit; cancellation is one visible
click. Both rules are anti-support-load measures as much as ethics.

**Support:** billing KB articles (`/quota`, `/pricing`, cancellation, refunds). A written,
published, no-argument refund policy — refund disputes cost more than refunds. Plus BYOK
articles: which providers work, what it costs, **what leaves our servers**, how to delete a
key, and why a report says it fell back to the built-in model.

**Instrumentation & eval**
- Revenue events, conversion funnel by limit-hit type, plan distribution, churn.
- Track **which limit** triggers upgrades — it tells you whether the tiers are priced on the
  right axis.

**Exit criteria**
- End-to-end purchase, upgrade, downgrade, and cancel verified in test mode and once live.
- Webhook replay and reconciliation verified by deliberately dropping webhooks in staging.
- Zero cases of a paying user being blocked by anonymous traffic.
- The merchant-of-record decision recorded in `docs/DECISIONS.md`, with the tax obligations
  it implies written down rather than assumed.
- BYOK verified end to end on both adapters, including: structured output parity with the
  local grammar, an invalid key falling back cleanly mid-analysis, and a persisted key
  surviving a watch-alert run.
- **A deliberate secret-leak audit**: grep the codebase and a full request/response log
  capture for any occurrence of a test key. Zero hits required before this ships.

**Cost posture:** ~€15/mo + Stripe fees. First revenue possible.

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
- **Two-pass reports and the deferred render tier**, *if* Phase 2's measurement justified it
  ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1A, [ARCHITECTURE.md](ARCHITECTURE.md) §5.5): the
  `source.render` and `analysis.pass2` jobs, report versioning with v1 retained at its own URL,
  held PDF pre-warm, and the completion notice. Built here because it reuses this phase's
  scheduling, off-peak batching and notification machinery rather than duplicating it.
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
- If two-pass shipped: pass 1 latency **unchanged** by rendering (the render queue never
  touches the request path), pass 2 median under 15 minutes, and v1 reachable after v2 lands.

**Cost posture:** ~€15/mo. Watches raise inference load, which on Rung 0 is the scarcest
resource in the system; watch caps per tier and off-peak scheduling exist to bound it.

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

**Cost posture:** €15/mo, rising to ~€80/mo if the Rung 1 trigger fires ($350 MRR), plus
~€100 one-off launch costs. Target ~$350–500 MRR exiting.

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
- p50 ≤ 25s, p95 ≤ 45s at 10× Phase 6 traffic — **achieved by reaching Rung 2**, not by
  optimising Rung 0 further.
- Quality gates all passing on the larger model.
- D30 retention ≥ 25% for registered users.
- ~$1k+ MRR.

**Cost posture:** ~€200/mo (Rung 2 GPU box) + ~€15 email. Must be ≤ 20% of collected MRR
per §6.1; if it is not, the trigger to migrate had not actually fired.

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

## 2A. Validation gates, pivot criteria, and the distribution workstream

Three things the phased plan does not otherwise contain. All are cheap; all address risks the
plan already ranks as its most serious.

### 2A.1 The distribution workstream — weekly, from Phase 1

R9 names distribution as the most likely cause of death, and the plan's only answer is one
launch window in Phase 6. Everything known about zero-budget launches says the audience must
exist **before** launch day.

**From Phase 1 onward, 2–4 hours per week, every week:**

| Activity | Cadence |
|---|---|
| Build-in-public update (progress, a finding, a screenshot) | Weekly |
| Participate genuinely in one community that will later be a launch channel | Weekly |
| Publish one `/help` article or comparison page as static content | Weekly from Phase 2 |
| Waitlist growth check + one outreach conversation | Weekly |
| Review acquisition metrics against §3.1 | Monthly |

This is probably worth more to the Phase 6 outcome than any single technical task in
Phases 1–5. It is written here as a standing commitment rather than a phase so that it cannot
be quietly deferred; see [DISTRIBUTION.md](DISTRIBUTION.md) for the channel plan.

### 2A.2 Validation gates — evidence required before proceeding

Exit criteria say when a phase is *done*. These say when the plan is *wrong*. Written now,
while no ego is invested in the answer.

| Gate | When | Evidence required | If not met |
|---|---|---|---|
| **G1 — Anyone wants this** | End of Phase 0 | ≥5 concierge reports delivered; ≥2 recipients ask for another, or pay | Do not start Phase 1 as specified. Re-scope the report, or re-target the buyer, using what the 5 conversations actually said. |
| **G2 — The output is worth money** | End of Phase 2 | ≥10 beta users have run real analyses; ≥3 say they would pay; ≥1 actually does (manual invoice is fine) | Fix the product concept before building accounts, billing and watches on top of it. |
| **G3 — Monitoring is the hook** | End of Phase 5 | ≥35% of registered users create a watch; alert 👍 rate ≥70% | The retention thesis is wrong. Re-plan around one-shot value, or find the segment where monitoring matters. |
| **G4 — Distribution works at all** | 3 months post-launch | Organic signups trending up; ≥1 channel with repeatable cost/effort per signup | The problem is positioning, not features. Re-position (§2A.3) rather than building more. |

### 2A.3 Pivot ladder — what "re-position" concretely means

The engine is subject-agnostic: discovery, fetching, verification, reporting and monitoring do
not care what kind of organisation is being analysed. So a failed positioning is a **relabelling
exercise, not a rebuild** — and at €15/month there is no limit on attempts.

Ordered by distance from the current plan:

1. **Same product, sharper buyer** — target roles where a wrong number has a *named personal
   cost*: diligence associates, agency strategists, procurement, journalists. This is the first
   move, not the last, and it is mostly a copy change.
2. **Same engine, different subject** — vendor due-diligence for procurement; supplier
   monitoring; grant/funder landscape scans for non-profits.
3. **Same engine, different delivery** — a monitoring-first product where the report is the
   onboarding artifact rather than the point.
4. **Same engine, someone else's brand** — white-label report generation for agencies and
   consultancies who already have the client relationships.

Each step reuses the pipeline entirely. **Record which positioning is being tested and for how
long**, so that a pivot is a decision with a date rather than a drift.

### 2A.4 Bus factor and the promises that assume a healthy founder

The plan makes public commitments that assume continuous availability: a 2-business-day
correction SLA, a support SLA, and watch alerts users rely on. A two-week illness breaks all
three, publicly.

Cheap mitigations, none requiring a second person:
- **Publish SLAs as targets with a stated exception** ("usually within 2 business days") rather
  than guarantees.
- **A status page** (a static page updated by hand is sufficient) so an outage or absence is
  visible rather than mysterious.
- **A documented `RUNBOOK.md`**, grown from Phase 0 onward, covering the three known failure
  modes (R7), restore-from-backup (R11), and how to put the product into a safe read-only mode.
- **An away-mode switch** that pauses watch checks and shows an honest banner, so absence
  degrades gracefully instead of silently.

---

## 3. Metrics & success criteria (F)

Instrumented from Phase 1. Reviewed weekly; a dashboard the founder actually opens.

### 3.1 Product & funnel

| Metric | Definition | Target |
|---|---|---|
| **Time to first value** | Land → first section streaming | Rung 0: p50 ≤ 40s · Rung 2: p50 ≤ 10s |
| **First-run completion** | % of first-time visitors who submit and read a complete report | ≥ 60% |
| **PDF rate** | % of completed reports downloaded | ≥ 25% |
| **Activation** | New user runs ≥2 analyses **or** creates a watch in 7 days | ≥ 40% |
| **Watch adoption** | % of registered users with ≥1 watch | ≥ 35% |
| **D7 / D30 retention** | Registered users returning | ≥ 40% / ≥ 25% |
| **Free → paid** | Conversion within 30 days of signup | ≥ 3% |
| **Churn** | Monthly paid logo churn | ≤ 6% |

### 3.2 Compute & latency

Latency targets are **per rung** — a single number would be either a lie on Rung 0 or a
sandbagged goal on Rung 2.

| Metric | Rung 0 (free) | Rung 2 (GPU) |
|---|---|---|
| End-to-end analysis latency | p50 ≤ 180s, p95 ≤ 240s | p50 ≤ 25s, p95 ≤ 45s |
| Time to first streamed content | p50 ≤ 40s | p50 ≤ 8s |
| Prefill tokens per analysis | ≤ 4,000 | ≤ 24,000 |
| Tokens generated per analysis | ≤ 900 | ≤ 2,500 |

| Metric | Target |
|---|---|
| **Compute-ms per analysis** (the real unit cost) | trending down |
| Cache hit rate — fetch / extraction / section | ≥ 60% / ≥ 45% / ≥ 25% |
| Inference queue depth | p95 ≤ 2 |
| Slot utilization | 40–75% (below is waste; above is queueing) |
| Watch-check inference share | ≤ 25% of daily capacity |
| Analyses needing pass 2 | tracked — the number that decides whether the render tier earns its keep |
| Pass-2 completion time | p50 ≤ 15 min on Rung 0 |
| Pass-2 value rate | % of pass-2 runs that actually add a fact; a low rate means stop rendering |
| Infra cost / paying customer | ≤ $4/mo |
| **Infrastructure as % of collected MRR** (§6.1) | **≤ 20%** |
| BYOK adoption | % of registered users with a key configured — tracked, not targeted |
| BYOK share of analyses | inference we do not pay for; rises = free-tier pressure relieved |
| BYOK fallback rate | % of BYOK analyses that fell back to local; > 10% means bad key UX |
| Analyses served per day on current rung | tracked against rung capacity |

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

All eight are measured **per inference provider**. A BYOK analysis on a frontier model must
clear the same bar, and its results must never be pooled with the local model's — pooling
would hide a local regression behind someone else's GPU.

### 3.4 Quality — the measurable definition of "frictionless"

1. **No-instruction success**: ≥4 of 5 unmoderated testers reach a downloaded PDF **without
   abandoning**, having read nothing. On Rung 0 that means surviving a 90–180s wait, which
   makes this the hardest UX measure in the product.
2. **Zero required fields** beyond the one textarea.
3. **≤1 clarifying question** in ≥80% of analyses; 100% skippable to a complete report.
4. **Zero documentation reads** required for the core flow — measured as `/help` visits
   *before* a first completed analysis (target < 5%).
5. **Mid-stream abandonment** — the free tier's defining risk. Target ≤ 20% on Rung 0,
   ≤ 5% on Rung 2. If this is high, the fix is a better waiting experience or a faster rung,
   not more copy.
6. **Support threads per 100 analyses** trending down month over month.
7. **Signup-to-first-analysis time**: p50 ≤ 60s (submission, not completion).

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
| Provider adapters (`openai_compatible`, `anthropic`) against a specified capability trait | **Anything touching BYOK credential custody** — storage, redaction, logging, backup exclusion (R13) |
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
- **One PR per concern, always reviewed**, against the checklist in
  [CODING_QUALITY.md](CODING_QUALITY.md) §10.3. Agent-generated code is read, not merged on
  faith; the hot zones — auth, billing, SSRF, BYOK credentials, the verification layer, and
  migrations — are hand-reviewed line by line, 100% of the time, and never merged the same
  hour they were written.
- **Timebox the infrastructure.** Kubernetes, microservices, and a custom design system are
  all traps. The deploy script is 20 lines and stays that way until it demonstrably hurts.
- **Batch the founder-only work.** Quality review, support, and prompt iteration go in one
  daily 45-minute block. Context-switching is the scarcest resource in a one-person company.
- **Write the decision down.** `docs/BENCHMARKS.md` and numbered ADRs in `docs/decisions/`
  ([CODING_QUALITY.md](CODING_QUALITY.md) §8.1). In six months you will not remember why
  Q5_K_M lost, and re-deciding costs more than recording.
- **Ship on a fixed cadence** (weekly). A solo founder's real failure mode is an eight-week
  refactor nobody asked for.

### 4.3 What to buy rather than build

Stripe Billing Portal · Postmark/Resend · Hetzner (not a cloud provider's GPU pricing) ·
Cloudflare free tier · Backblaze B2 · Plausible or self-hosted analytics.
**Build**: the analysis engine, the verification layer, the KB. Those are the product.

---

## 5. Risks & mitigations (H)

### R1 — Free-tier inference latency
*Risk:* four Ampere cores cannot deliver the 15–25s product promise. Reports take 90–180s,
and users abandon mid-stream. On this hardware **prefill, not generation, is the dominant
cost**, so the naive design (feed 20k tokens of source text to a model) is not merely slow —
it is unusable.
**Likelihood: certain at Rung 0. Impact: high.**
*Mitigations:* deterministic-first extraction so prices, dates and versions never enter the
model's context at all ([ARCHITECTURE.md](ARCHITECTURE.md) §5.4); span pre-selection cutting
each source from ~2,500 to ~400 tokens; tiny grammar-constrained per-source outputs;
section-parallel generation batched across slots (continuous batching is worth more on
memory-bound CPU than on GPU); a ≤900-token generation budget; aggressive caching, where hit
rate *is* the capacity plan; progressive streaming with first content in 20–40s; honest
queue-position display; and a revenue-triggered ladder to Rung 1 and Rung 2 (§6). Phase 0
measures all of this on the actual A1 instance before any product code depends on it.
*Critically:* the landing page promises Rung-0 reality, not Rung-2 aspiration.
*Early warning:* p95 latency, queue depth, abandonment rate mid-stream, prefill share of
total inference time.

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
*Note on where quality actually fails:* the expected dominant cause is **extraction, not the
model** — a missed pricing table, a client-rendered page that reads as empty, nav junk
swamping the content. The daily review sample must record *which* layer failed, because the
instinct to reach for a bigger model when the real bug is in the HTML parser would waste both
money and weeks.
*Early warning:* drop rate, 👎 rate by section, inaccuracy reports per 100 analyses, and
`extraction_quality` scores clustered by domain.

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

### R10 — The fetch primitive is an attack surface (SSRF and abuse)
*Risk:* the core feature is "the server fetches a URL a stranger typed." Without explicit
prevention, that is a general-purpose request proxy: an attacker points it at
`169.254.169.254`, at `localhost:5432`, or at a service on the host's private network and
reads the response back out of a rendered report. Secondarily, the same primitive can be
aimed at a third party as an amplification or scanning tool, which gets the host's IP blocked.
**Likelihood: certain to be attempted. Impact: severe — credential disclosure or an abuse
complaint that takes the box offline.**
*Mitigations:* [ARCHITECTURE.md](ARCHITECTURE.md) §11.4 — resolve-then-validate against the
*resolved IP*, not the hostname (hostname checks are defeated by DNS rebinding);
re-validation after every redirect; scheme restriction; size and time caps; no raw fetch
errors echoed to users. Plus per-host and global rate limits, an honest user-agent with a
public bot page and opt-out, and anonymous quotas that bound how much fetching a stranger can
provoke. This code is on the 100%-human-review list alongside auth, billing, and the
verification layer, with unit tests covering rebinding, redirect chains, and IPv6-mapped
forms.
*Early warning:* fetch attempts to non-public address space (should be zero, and alarmed);
abuse complaints to the host; anomalous per-IP analysis volume.

### R11 — Single-vendor dependency on Oracle Always Free
*Risk:* the entire product runs on a free tier controlled by one vendor. Oracle can reclaim
idle Always Free instances on trial accounts, A1 capacity is genuinely scarce in popular
regions, and free-tier terms can change with little notice. Losing the instance means losing
the product.
**Likelihood: medium. Impact: severe at Rung 0, declining thereafter.**
*Mitigations:* convert the account to Pay-As-You-Go before building on it, which retains
Always Free resources while still billing €0; provision early and never terminate the
instance to recreate it; nightly `pg_dump` plus WAL archiving to Backblaze B2 or Cloudflare
R2 off Oracle entirely, with a quarterly restore drill; keep the deployment
provider-agnostic — systemd units, a static aarch64 binary and standard Postgres, with no
Oracle-specific services (notably *not* Oracle Autonomous Database) anywhere in the stack.
Recovery from total loss should be a documented afternoon: provision anywhere with ARM or
x86, restore, repoint DNS. **Rehearse it once in Phase 3.**
*Early warning:* Oracle service notices, instance health, any capacity or quota mail.

### R12 — Bootstrapping stall: quality gated behind revenue that quality gates
*Risk:* the free tier produces 90–180s reports from an 8B model. That may be good enough to
convert users — or it may be exactly what stops them converting, in which case revenue never
reaches the $350 trigger, and the product never gets the hardware that would make it good.
This is the specific failure mode of a bootstrapped, compute-bound product.
**Likelihood: medium. Impact: severe — it is the difference between a slow start and no start.**
*Mitigations:* **BYOK (Phase 4) is the direct answer** — a user who finds the wait intolerable
can supply their own provider key and get 15–25s immediately, at their cost rather than ours,
instead of churning. That converts the free tier's worst property into a segmentable one and
costs us nothing to serve. Beyond that: make Rung 0 quality as high as the hardware allows
rather than accepting it —
deterministic extraction, span pre-selection, and verification are quality mechanisms first
and latency mechanisms second ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §7). Set
expectations honestly so slowness reads as thoroughness, not brokenness. Track conversion
against *report quality ratings*, not latency, so the actual blocker is identifiable. If
Phase 6 shows users converting but latency is the top complaint, Rung 1 is worth funding from
personal savings as the one deliberate exception to §6.1 — a decision to make consciously and
in writing, not by drift.
*Early warning:* free→paid conversion versus 👍 rate; churn reasons citing speed; abandonment
rate mid-stream.


### R13 — Custody of user-supplied provider credentials (BYOK)
*Risk:* BYOK means holding third-party API keys. A leak — through a log line, an error
response, a database backup, or a restored snapshot resurrecting deleted keys — bills a user's
provider account and ends the product's credibility in a single incident. Adding it also
creates a second SSRF surface, because a custom `base_url` is a user-supplied URL our server
calls, now with credential-handling code attached.
**Likelihood: low with discipline, near-certain without it. Impact: severe.**
*Mitigations:* [ARCHITECTURE.md](ARCHITECTURE.md) §4.8 — session-only by default, so most
users' keys are never written to disk at all; XChaCha20-Poly1305 at rest with the master key
in a `0600` `EnvironmentFile` and a `key_version` for rotation; a `Secret<String>` newtype
with redacting `Debug`, plus **a CI check that fails the build on any format string
interpolating it**; provider error bodies sanitized before logging; no reveal endpoint;
`user_api_keys` excluded from ordinary backups; deletion honoured immediately; custom base
URLs through the same resolve-then-validate SSRF guard as R10, with known provider hosts
allowlisted and custom hosts an explicit opt-in. On the 100%-human-review list.
*Early warning:* any secret-shaped string appearing in logs (alarmed, not sampled); key
`status` transitions; support threads mentioning unexpected provider charges.

### R14 — BYOK erodes the privacy differentiator or the subscription
*Risk:* two commercial failure modes. **Trust:** "your research never leaves our server" is a
headline claim, and BYOK breaks it — if that exception is discovered rather than disclosed,
the claim reads as a lie. **Revenue:** if BYOK users get everything for free, the subscription
loses its reason to exist.
**Likelihood: medium. Impact: medium.**
*Mitigations:* disclosure is mandatory, unmissable and at the point of choice, with an
explicit acknowledgement checkbox ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §5A.2), a provider chip
on every affected report and PDF, and coverage in `/legal/privacy` plus a dedicated KB thread.
On revenue: BYOK raises the **analysis** quota because inference stops being ours, but watch
counts, alert cadence, webhooks, API access and team features stay tier-gated — none of them
is an inference cost. Track BYOK adoption against conversion; if BYOK users convert
materially worse, the tier boundaries are on the wrong axis and should move, rather than BYOK
being restricted.
*Early warning:* free→paid conversion split by BYOK usage; privacy questions in the KB;
churn reasons citing data handling.


### R15 — Founder burnout / scope creep
*Risk:* an eight-week refactor, a design system, a Kubernetes migration.
**Likelihood: medium. Impact: high.**
*Mitigations:* fixed weekly ship cadence; phase exit criteria written before the phase
starts; a hard rule that new data sources must prove themselves on the golden set;
buy-don't-build defaults; agents on everything downstream of a specification.

---

## 6. Bootstrapped hosting & cost ladder

**Premise: no angel or VC funding.** Every euro of infrastructure is paid for by revenue
already collected. There is no runway to burn, so the product must be free to operate until
it is not free to operate — which is why the launch host is Oracle Cloud Always Free and why
each upgrade has a revenue trigger rather than a date.

If outside capital ever arrives, this section is the first thing to rewrite. Until then it is
a hard constraint, not a preference.

### 6.1 The two rules

1. **Infrastructure ≤ 20% of MRR.** Below that, upgrading is premature; above it, the
   business is subsidising its own hosting out of a founder's pocket.
2. **Hold three months of the next rung's cost in cash before climbing.** A rung you cannot
   sustain through one bad month is a rung you cannot afford. Downgrading after a migration
   is far more disruptive than waiting.

Both rules are checked monthly against actual collected revenue — not MRR run-rate, not
annualised, not "committed."

### 6.2 The ladder

| Rung | Host | Models | Est. latency (p50) | Cost/mo | Trigger (MRR) |
|---|---|---|---|---:|---:|
| **0** | Oracle Always Free A1 (4 OCPU, 24 GB) | Qwen3-1.7B / 4B / 8B | 90–180s | **€0** | — (launch here) |
| **1** | Oracle Free web tier **+** Hetzner AX52 dedicated for inference | Qwen3-30B-A3B (MoE) | 45–70s | ~€65 | **$350** |
| **2** | Oracle Free web tier **+** Hetzner GEX44 (RTX 4000 Ada 20 GB) | Qwen3-14B / 30B-A3B, GPU-resident | **15–25s** | ~€200 | **$1,100** |
| **3** | 48–80 GB VRAM GPU (RTX 6000 Ada / L40S / A100) | 70B-class dense or gpt-oss-120b MoE | 15–25s, better output | ~€700 | **$3,800** |
| **4** | Multi-GPU server (8×H100-class) | Kimi K2-class (~1T params, ~32B active) | — | €3,000–6,000 | **$18,000** (~$215k ARR) |

Read the last row as a reality check, not a plan. **A Kimi-class model is not a
"when revenue comes in" upgrade for a bootstrapped solo product** — at 4-bit it is roughly
550–600 GB of weights and needs a multi-GPU server. The realistic quality ceiling for the
first two years is **Rung 3**, and the honest framing is that Rungs 0→2 are the ones that
determine whether this business exists at all.

### 6.3 What each rung buys, in order of what users notice

- **0 → 1** is a *quality* jump more than a speed jump: Qwen3-30B-A3B is a materially better
  grounded summariser than an 8B, and MoE keeps it CPU-affordable. Reports get better before
  they get fast.
- **1 → 2** is the *speed* jump that finally makes the 15–25s product promise true, and it is
  the rung at which the marketing copy and the product stop disagreeing.
- **2 → 3** is diminishing returns on this workload. Most remaining quality is in retrieval,
  extraction, and prompting — not parameters. **Spend on Rung 3 only after the golden set
  shows the model, not the pipeline, is the ceiling.**

### 6.4 Cost posture by phase

| Phase | Infra/mo | Other | Revenue target |
|---|---:|---|---:|
| 0–2 (build) | €0 | domain ~€15/yr | €0 |
| 3–4 (accounts, billing) | €0 | email ~€15/mo once sending | €0 |
| 5 (watches) | €0 | email ~€15/mo | first customers |
| 6 (launch) | €0 → €65 | ~€100 one-off launch costs | **$350–500** |

**One variable cost to watch:** SearXNG depends on upstream engines that rate-limit it, and
Brave Search API is the documented fallback — which is **paid per query**. Budget **~$5–15/month
from Phase 2** as a contingency, and track queries-per-analysis as a cost metric. It is small,
but it is the only line in this plan that scales with usage, which makes it the one worth
watching.
| 7 (retention/scale) | €65 → €200 | | **$1,100–2,000** |
| 8 (sustain) | €200 | | **$2,000+**, infra ≤ 20% |

Total cash required to reach first revenue: **under €100.** That is the entire point of the
free-tier launch, and it is what makes a no-outside-capital plan credible rather than
aspirational.

### 6.5 Bootstrapping discipline

- **BYOK relieves rung pressure without spending.** Users who bring their own key take their
  inference off our box entirely, so heavy users can be served well *before* the next rung is
  affordable. Watch BYOK share of analyses alongside queue depth — rising BYOK adoption
  legitimately defers a rung, and that is a feature of the ladder, not an accident.
- **Never pre-buy capacity.** Rung 1 is triggered by paying customers, not by a benchmark the
  founder finds disappointing.
- **The free tier is the runway.** Because it does not expire, there is no clock forcing a
  premature launch or a bad pricing decision.
- **Downgrade must stay possible.** Nothing above Rung 0 may become architecturally load-bearing:
  the web tier never moves off Oracle Free, and every inference rung is one environment
  variable. If revenue falls, step back down without a migration.
- **Prefer variable-to-fixed conversions late.** Managed Postgres, hosted observability, and
  paid search APIs all convert founder time into monthly cost. Each is defensible eventually;
  none is defensible before revenue.
- **Price for the rung you are on.** Rung 0 latency is a product fact and must be reflected in
  what is promised on the landing page (see [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1) — the
  fastest way to destroy trust is to advertise Rung 2 speed while serving Rung 0.

---

## 7. Git / pull-request workflow (mandatory)

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
