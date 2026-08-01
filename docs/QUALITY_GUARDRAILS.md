# Landscape — Quality & Trust Guardrails

> Section **E** of the roadmap. See [ROADMAP.md](ROADMAP.md) for the index and phasing.
>
> This document covers the quality of the **product's output** — specifically the middle of
> the chain, *model output → verified claim*. The two ends are in
> [FACT_CHECKING.md](FACT_CHECKING.md): how sources are found and judged before the model
> sees them, and how a reader independently confirms a claim afterwards.
>
> For the quality of the **code that produces it**, see [CODING_QUALITY.md](CODING_QUALITY.md).

---

## 1. The core bet

A locally-run 4B–8B model, given *only* retrieved public text, forced through a strict
grammar, and mechanically verified against its own sources, produces **more trustworthy
competitive analysis than a frontier model prompted freely** — because the failure mode
that destroys this product is not "prose is slightly worse," it is "invented a price."

Everything below exists to make invention structurally difficult rather than
merely discouraged.

**One limitation, stated at the top rather than buried.** Layers 3–5 prove a claim is
faithful to *the text we extracted*. They cannot prove the extraction was correct, and they
cannot prove the source was telling the truth. Those are the jobs of
[FACT_CHECKING.md](FACT_CHECKING.md) levels L0–L2 (source admission, provenance, extraction
fidelity) and of attribution framing — the rule that the product reports *what sources say*,
attributed and dated, rather than what is true.

§7 addresses the other obvious question — how a model that small, running on four ARM cores,
is good enough — and the short answer is that on this workload grounding, not parameter count,
is what binds quality.

---

## 2. The five-layer anti-hallucination stack

```
Layer 1  Retrieval gate        — no source text, no section. Full stop.
Layer 2  Constrained decoding  — GBNF grammar; citation fields are non-optional.
Layer 3  Evidence verification — every claim's quote must exist in its cited source.
Layer 4  Type-specific checks  — prices, dates, versions, numbers re-validated.
Layer 5  Presentation honesty  — confidence, tier, timestamps, and gaps shown to the user.
```

### Layer 1 — Retrieval gate

- The synthesizer's prompt contains **only** fetched source text. No parametric knowledge is
  solicited, and the system prompt states that unsupported statements are errors.
- If a section has zero qualifying sources, the section is **not generated at all**. It
  renders as `not_found_in_public_sources` with the list of what was checked. The model is
  never given the chance to fill a gap.
- Source text is labelled inline (`[S3] linear.app/pricing — fetched 2026-07-31T14:21Z`),
  which is what makes labels attributable at verification time.

### Layer 2 — Constrained decoding

Every structured call runs under a GBNF grammar derived from the same JSON Schema that
generates the API and UI types (see [ARCHITECTURE.md](ARCHITECTURE.md) §2.5, §4.5).

- `Claim` cannot be emitted without `source_label` **and** `evidence_quote`. Citation is a
  grammar requirement, not a prompt request.
- Enums make refusal cheap and reliable: `"enterprise_pricing": "unknown"` is a legal token
  path, so the model has a well-formed way to *not know*. Models hallucinate most when the
  grammar gives them no way to decline.
- `source_label` is constrained to `^S[0-9]{1,2}$`, so a fabricated citation format is
  impossible — only a *wrong* label is possible, and Layer 3 catches that.

### Layer 3 — Evidence verification (the load-bearing layer)

Every emitted `Claim` is checked in Rust — deterministic code, no model involved:

1. **Label validity** — does `S7` exist in this analysis's source set? If not → **dropped**.
2. **Quote presence** — does `evidence_quote` appear in that source's extracted Markdown?
   Exact match first; then normalized (whitespace/case/smart-quotes/entities); then
   token-level fuzzy match (trigram Jaccard ≥ 0.85 over a sliding window) to tolerate
   benign truncation.
   - exact/normalized → `verified`
   - fuzzy → `weak`
   - no match → **dropped**
3. **Entailment sanity** — the claim's key content tokens (numbers, currency symbols,
   product names, dates, negations) must be present in a ±400-character window around the
   matched quote. Catches the "correct quote, wrong conclusion" failure.
4. **Drop accounting** — dropped claims are stored with the reason. Never silently deleted:
   they are the primary quality telemetry.

Enforcement on the report:

| Condition | Action |
|---|---|
| Any claim dropped | Removed from output; counted. |
| >30% of a section's claims dropped | Section regenerated **once** with the failed claims quoted back as negative examples. |
| Still >30% after retry | Section marked `partial` with an explicit note. |
| >50% of all claims dropped | Report marked **thin evidence**, prominently. |
| Verification error rate >5% over 24h | Ops alert — the model, prompt, or extractor has regressed. |

Cost: a few milliseconds of string matching against a hallucinated price shipped to a user
who quotes it in a board deck. This layer is the product's actual moat.

### Layer 4 — Type-specific validators

Free-form text passes Layer 3 easily; *numbers* are where damage happens.

- **Prices**: regex-extract currency, amount, period, and basis from the evidence quote and
  require an exact match to the structured `PricingTier`. `$8/user/mo` in the tier and
  `$8 per user, billed annually` in the quote → flagged for the `billing_period` mismatch,
  and the report shows the annual-billing caveat rather than dropping the tier.
- **Dates**: must be ≤ `fetched_at` and within the stated lookback window. Future-dated
  "recent changes" are dropped outright — a common and very visible failure.
- **Versions / counts / percentages**: must appear verbatim in the quote.
- **Superlatives**: "the only," "the fastest," "the leading" are permitted **only** when the
  source is the company's own site *and* the claim is framed as a company statement
  ("Linear describes itself as…"). Otherwise the modifier is stripped.
- **Competitor attribution**: in comparison reports, a claim about product B sourced from
  product A's comparison page is retagged `tier3` and labelled *"per competitor's own
  comparison page"* — vendor comparison pages are systematically unreliable and must never
  be presented as neutral.

### Layer 5 — Presentation honesty

Trust is set by what the interface shows, not by what the copy promises:

- `[S4]` chips on every claim, with hover cards: URL, page title, trust tier, fetched-at.
- Per-claim confidence (high/medium/low), derived from verification status × source tier ×
  corroboration count — **not** from anything the model self-reports. Model-reported
  confidence is discarded; self-assessed confidence is uncalibrated in small models.
- Report-level `evidence_strength: strong | moderate | thin`, always visible, in the PDF too.
- The SWOT section — the one place inference is permitted — is visually distinct and labelled
  *interpretation*, with each item citing the observed facts it rests on.
- Timestamps everywhere, in UTC, on screen and in the PDF.
- The permanent disclaimer line (calm, not a modal):
  > *Generated from public web sources on 2026-07-31. Good-enough public intelligence, not
  > verified enterprise competitive intelligence. Check anything you'll act on.*
- **"What we checked"** on every gap. Showing the negative space is the strongest available
  signal that the system is not guessing.

---

## 3. Evaluation

### 3.1 The golden set

**50 subjects** (growing to 150), fixed and version-controlled, spanning: well-documented
SaaS, thin-footprint startups, ambiguous brand names, non-English sites, JS-heavy sites,
robots-restricted sites, recently-repriced products, and two deliberate traps (a product
that does not exist; a name shared by three real products).

**Discovery-shaped prompts are half the set.** Named products test the analysis pipeline;
they do not test discovery, which is now equally hard and equally capable of being wrong
([COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md)). The set therefore includes **class C/D/E
inputs** — bare categories, product ideas in the user's own words, and job-to-be-done
descriptions — each with a human-curated *expected competitor set* plus the classification
(direct / adjacent / substitute) each candidate should receive. Concierge reports from Phase 0
are the natural first source of these. Without them, a discovery regression is invisible until
a user reports a report about the wrong market.

For each, a human-curated **reference sheet** of verifiable public facts as of a fixed
date, plus fixture snapshots of every source page. **Fixtures are essential**: without
frozen HTML, every eval run measures the web changing rather than the model changing.

### 3.2 Automated metrics (nightly + on every prompt/model change)

| Metric | Definition | Gate |
|---|---|---|
| **Citation coverage** | % of claims with a verified quote | ≥ 97% |
| **Hallucination rate** | dropped-claim rate at Layer 3 | ≤ 3% |
| **Fact recall** | % of reference-sheet facts present in the report | ≥ 70% |
| **Fact precision** | % of report claims consistent with the reference sheet | ≥ 95% |
| **Refusal correctness** | % of genuinely-absent facts reported as `not_found` rather than invented | ≥ 98% |
| **Trap resistance** | 0 fabricated content on the two trap subjects | **hard gate** |
| **Schema validity** | % of runs parsing under the grammar | 100% |
| **Latency** | p50 / p95 wall clock | Rung 0: p50 ≤ 180s · Rung 2: p50 ≤ 25s |
| **Quality per second** | rubric score ÷ p50 latency | tracked, not gated — the number that decides prompt changes on slow hardware |
| **Determinism** | claim-set Jaccard across 3 runs at temp 0.2 | ≥ 0.85 |
| **Discovery precision** | % of returned competitors in the curated expected set | ≥ 80% |
| **Discovery recall** | % of expected competitors returned | ≥ 60% |
| **Classification accuracy** | direct / adjacent / substitute assigned correctly | ≥ 75% |

CI fails on any gate regression. A prompt change that improves prose while dropping recall
3 points does not ship.

### 3.2A The comparative benchmark — measured against competitors, not only ground truth

The metrics above measure us against **ground truth**. They do not measure us against the free
alternatives a user will actually compare us to, which means every claim about being *better*
is currently belief rather than evidence.

**A Phase 2 exit criterion.** Run the same **20 subjects** through Landscape and 3–4 free
alternatives, and score:

| Dimension | How |
|---|---|
| **Fact precision** | Claims correct, against hand-verified reference sheets |
| **Citation validity** | Of claims that cite a source, how many are actually supported by it |
| **Hallucination rate** | Claims contradicted by the primary source |
| **Coverage** | Facts found, of those publicly available |
| **Honest-gap rate** | Does it say "not found" when the fact genuinely is not public? |
| **Time to result** | Wall clock, stated honestly |

**The trap subjects are the most revealing test and cost nothing.** A tool that generates
confident detail for a product that does not exist has disclosed its architecture. Expect to
win decisively on precision, citation validity and honest gaps; expect to lose on time and
possibly on coverage. **Both halves get published** — a benchmark that only reports favourable
dimensions is the marketing behaviour this product exists to be an alternative to.

Re-run at each rung change and before any public accuracy claim.

### 3.3 LLM-as-judge — used narrowly, run locally

A larger local model (e.g. Qwen3-30B-A3B or a Q8 build of the synthesizer) scores
**readability, structure adherence, and usefulness** on a 1–5 rubric. Deliberately *not*
used for factual grading — Layer 3 and the reference sheets do that deterministically, and
a judge sharing the generator's blind spots would launder them into a passing score.

### 3.4 Human review — 15 minutes a day

The admin console surfaces a **daily sample of 5 reports**, weighted toward: 👎 feedback,
high drop rates, thin-evidence flags, and new source domains. The founder grades each
1–5 against a written rubric and tags failure modes. This queue is where prompt work
originates — never intuition.

### 3.5 Model & prompt version management

- Every analysis records `model_id`, `prompt_version` **and `inference_provider`**. Quality
  metrics are always sliced by all three. **This slicing is not optional once BYOK exists**
  ([ARCHITECTURE.md](ARCHITECTURE.md) §4.8): reports generated by a user's frontier model
  would otherwise flatter the local model's measured quality and hide a regression.
- **Shadow evaluation**: a candidate model runs the golden set offline; if it passes gates
  it is promoted to 10% of live traffic with metrics compared for a week before full cutover.
- **Rollback is a config change**, and old GGUFs stay on disk for exactly this reason.
- Prompt changes are PRs containing the eval diff. No prompt merges without one.

---

## 4. Improving structured output from a small local model

Ordered by leverage, cheapest first:

1. **Grammar first, prompt second.** Anything expressible as a grammar constraint should be
   a grammar constraint. Instructions are advisory; grammars are not.
2. **Decompose ruthlessly.** One small task per call ("extract pricing tiers from this one
   page") beats one large task ("write the report"). Small models degrade steeply with task
   breadth, and per-call outputs are independently cacheable and verifiable.
3. **Few-shot with real, versioned examples** — 2–3 per call type, drawn from human-approved
   past outputs, including one showing correct *refusal*. Kept in `prompts/vN/` and pinned
   in the shared system prefix so they hit the llama.cpp prefix cache.
4. **Stable prefix, variable suffix.** Ordering prompts for cache hits is a quality *and*
   latency lever.
5. **Sampling discipline.** Extraction: `temperature 0.1`, `top_p 0.9`, `repeat_penalty 1.05`.
   Prose: `temperature 0.4`. Never sample creatively over facts.
6. **Negative-example repair loop.** When Layer 3 drops claims, the single retry includes
   the dropped claims verbatim with "these were not supported by the cited source."
   Concrete negative examples outperform stern instructions in small models.
7. **Retrieval quality beats model size.** Most quality failures trace to bad extraction
   (nav junk, missing pricing table, truncated changelog), not to the model. Fix the
   extractor before reaching for more parameters.
8. **Quantization is a measured tradeoff.** Q4_K_M vs Q5_K_M vs Q8_0 are compared on the
   golden set, and the smallest that passes all gates ships.
9. **Speculative decoding** for latency (a 0.5–1.5B draft model). Output-identical under
   greedy-ish sampling, so quality gates are unaffected.
10. **Fine-tuning is deliberately deferred.** A LoRA on human-approved outputs is a Phase 7+
    option, only after prompting, grammar, and retrieval are exhausted — it adds a training
    pipeline, a serving artifact, and a whole class of silent regressions.

---

## 5. Feedback loops

### 5.1 Report quality

- Per-section 👍/👎 with an optional one-line reason.
- **"Report an inaccuracy"** on any claim → creates a KB-linked quality thread carrying the
  analysis id, claim, source, and verification status. Public by default (identifiers
  stripped), so corrections become searchable content.
- Signals routed to: the human review queue, the golden set (a confirmed error becomes a
  regression case — this is how the eval suite grows for free), and prompt work.
- Weekly review of 👎 clustered by section and by source domain. Domain clustering usually
  reveals an *extraction* bug, not a model bug.

### 5.2 Notification usefulness

- One-click 👍/👎 in every alert email (signed link, no login) — the highest-response-rate
  feedback surface in the product.
- 👎 raises that watch's threshold and is logged with the diff for offline review.
- Tracked: alerts per watch per week, 👍 rate, watch pause/delete rate, and re-run-analysis
  click rate (the strongest positive signal — the alert made someone act).
- **Health target: ≥70% 👍, ≤2 alerts/watch/week.** Below that, tighten thresholds globally
  before adding any feature.

### 5.3 Frictionless-ness

- Funnel: land → submit → first content → complete → PDF → return. Every drop-off is a
  friction bug with an owner.
- 5-person unmoderated usability run each phase, single task: *"Find out how this product
  compares to its competitors."* No instructions given. Watching two people fail is worth
  more than a month of analytics.
- Support-tag distribution is treated as a friction map: a spike in `/quota` posts means
  the limit dialog is unclear, not that users are confused.

---


## 6. Legal, ethical, and data-handling posture

- **Public data only.** No login-gated content, no paywall circumvention, no ToS-violating
  scraping. Sources that block us are reported as blocked.
- **robots.txt honored** on every fetch; honest user-agent with a public `/bot` page
  documenting behavior, cadence, and an opt-out address; per-host rate limiting.
- **Right to be excluded**: a public form for site owners to request exclusion, honored
  within 5 business days and recorded in a public exclusion list.
- **Quoting**: evidence quotes are short excerpts under fair-use/quotation norms, always
  attributed with a link. Long-form reproduction is a hard limit in the grammar
  (`evidence_quote` maxLength 300) — enforced structurally, not by policy.
- **User data**: inputs and reports belong to the user; **never used to train anything**;
  deletable from `/account`; retained 12 months by default. Because inference is local by
  default, no analysis content ever leaves the server — a genuine differentiator, stated
  plainly on the landing page and in the KB.
- **The BYOK exception, stated as prominently as the claim it qualifies.** If a user opts
  into their own provider key, their query and the fetched page text *do* leave our servers,
  to that provider, under that provider's terms. This is disclosed at the point of choice
  with an explicit acknowledgement (PRODUCT_SPEC §5A.2), shown on every affected report, and
  covered in `/legal/privacy` and a dedicated KB thread. A privacy promise with a silent
  exception is worse than no promise.
- **User-supplied provider credentials** are session-only by default, encrypted at rest when
  persisted, never returned to the client, never logged, excluded from ordinary backups, and
  deleted immediately on request. See ARCHITECTURE §4.8 and §11.4.
- **Disclaimers**: one calm line under every report and in every PDF footer; a fuller
  `/legal/disclaimer` page; explicit statements that the product does not verify claims,
  does not provide investment/legal advice, and may be wrong or out of date.
- **No competitor targeting features.** No "monitor this person," no scraping of individual
  employees' profiles beyond publicly posted job listings in aggregate. The product analyzes
  companies' public positioning, not people.

---

## 7. Maximising quality on free-tier hardware

The launch host is Oracle Always Free — 4 Ampere cores, 24 GB, no GPU
([ARCHITECTURE.md](ARCHITECTURE.md) §4.4). The synthesizer is an 8B model, not a 70B. This
section is the answer to the obvious question: *how is that good enough?*

The premise is that **on this workload, parameter count is not the binding constraint on
quality — grounding is.** The job is "read this page, report what it says, cite it, and
decline when it doesn't say." A 70B model does that better than an 8B, but not
categorically better, and every mechanism below closes more of the gap than the next model
size up would.

### 7.1 Move work out of the model entirely

The largest single quality gain available on constrained hardware is **not asking the model
to do things code does better** (ARCHITECTURE.md §5.4). Prices, billing periods, tier names,
release dates and version numbers are parsed deterministically from HTML tables and headings.

This is not a compromise forced by the hardware. A parsed `$8/user/mo` carries an exact
source offset and cannot be off by a digit; a generated one can. Pricing is the most-read and
most-quoted section in the product, and it is now the section least exposed to model error.
The same fact removes most of the prefill cost — the quality win and the latency win are the
same change.

### 7.2 Narrow the task until the model is good at it

Small models degrade steeply with task breadth, and gracefully with task depth. So:

- **One narrow job per call.** "Given this 400-token window from a pricing page, list the
  stated plan limits" is a task an 8B does reliably. "Write a competitive analysis" is not.
- **Three model sizes, matched to difficulty** (§4.2): a 1.7B router for enums and
  classification, a 4B extractor for span selection, an 8B synthesizer for the only work
  requiring real language understanding. Spending 8B-class compute on "is this input
  ambiguous?" is waste that shows up as latency, which shows up as abandonment.
- **Span pre-selection before the model reads.** A 400-token window containing the answer
  beats a 2,500-token page containing the answer *and* a navigation menu — for a small model
  the difference is large, because distractor text is where small models lose the thread.

### 7.3 Constrain harder, because the model is weaker

Every mechanism in §2 becomes *more* load-bearing at 8B, not less:

- **GBNF grammars are non-optional.** An unconstrained 8B produces malformed JSON often
  enough to matter; under a grammar it cannot. Retry loops that would cost fractions of a
  cent on a hosted API cost *seconds of user-visible latency* here, so preventing invalid
  output is a latency mechanism too.
- **Mechanical verification (Layer 3) does not care how big the model is.** A claim whose
  evidence quote is absent from its cited source is deleted whether an 8B or a 405B wrote it.
  This is why the verification layer, not the model, is the product's quality floor.
- **Explicit refusal paths at every level of the grammar.** Small models invent most when the
  output format gives them no legal way to say nothing.

### 7.4 Spend the prompt budget where it compounds

Prefill is the scarce resource, so few-shot examples are expensive — *except* in the system
prefix, which llama.cpp caches across calls. Therefore:

- Few-shot examples live in the **byte-identical system prefix** and are paid for once per
  process, not once per request. Two or three examples per call type, including one
  demonstrating correct refusal.
- Variable content goes strictly at the end. On this hardware, prompt ordering is worth
  seconds per request.
- Examples are drawn from **human-approved past outputs** (§3.4) and versioned in
  `prompts/vN/`, so the corpus improves as the daily review queue is worked.

### 7.5 Evaluate against the hardware you have

- The golden set runs **on the A1 instance**, not on a laptop. A prompt change that improves
  quality but adds 40 seconds is a regression here and must be visible as one.
- Track **quality per second**, not just quality: the eval report carries both the rubric
  score and the measured latency for every configuration.
- **Re-run the model bake-off quarterly.** Small open models are improving faster than any
  other input to this product, and a 4B released next quarter may beat today's 8B at half the
  cost. This is the single highest-leverage recurring task in the roadmap.
- When the ladder moves to Rung 1 or 2 (ROADMAP §6), **re-run the full golden set before
  cutting over**. A larger model is not automatically better on a grounded, constrained task,
  and shipping one on faith is how a quality regression reaches users.

### 7.6 What a bigger model would actually buy

Recorded honestly, so the Rung 3 spend is made on evidence rather than hope. Expect gains in:
prose quality of the Positioning and SWOT sections, subtler sentiment-theme clustering, and
robustness on unusual page structures. Expect **little or no gain** in: citation coverage,
hallucination rate, refusal correctness, or pricing accuracy — all of which are governed by
retrieval, deterministic parsing and mechanical verification.

The practical consequence: **do not buy Rung 3 until the golden set shows the model, rather
than the pipeline, is the ceiling.** In the daily review sample, record which layer failed —
retrieval, extraction, or generation. The expectation is that extraction dominates for a long
time, and reaching for a bigger model when the bug is in the HTML parser would waste both
money and weeks.
