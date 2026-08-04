# Fable Evaluation — An Honest Assessment of the Landscape Plan

> An evaluation of the ten-document plan in PR #1 (22 commits, ~7,000 lines), against six
> questions: probability of success, missing topics, per-document completeness, roadmap
> completeness, missing details, and recommendations.
>
> **Method and honesty note.** This evaluation applies the plan's own standard to itself:
> judgments are labelled as judgments, probabilities are calibrated estimates rather than
> measurements, and the evaluator (the same model that wrote most of the documents) has an
> obvious authorship bias toward finding the plan good. The mitigation is the same one the
> plan uses everywhere else: be specific enough to be checkable, and prefer identifying
> defects over defending choices. Where this document criticises the plan, the criticism
> stands even though the author wrote the thing being criticised.
>
> Scope caveat from the question as posed: probabilities assume the plan proceeds as written,
> documentation stays current, execution meets the current state of the market, and
> **competition remains today's competition**. That last assumption is generous — the
> AI-tools market turns over in months — and §2.4 notes what changes without it.

---

## 1. Executive summary

**The plan's engineering is materially stronger than its go-to-market, and the plan knows
it** — R9 (distribution) is already ranked above every technical risk. But knowing it and
acting on it are different: distribution currently occupies one phase out of nine, while the
thing most likely to kill the product gets less sustained attention than PDF generation.

**The single largest structural flaw: the plan builds for ~20 weeks before the first real
user contact.** Phases 0–5 are heads-down construction; validation arrives at Phase 6. A
bootstrapped solo project cannot afford to discover in week 20 that the report format,
pricing, or positioning is wrong. The highest-leverage change available costs almost nothing:
a two-week concierge validation sprint run *in parallel with* Phase 0, producing hand-made
reports for real founders using the documented method — before the method is automated.

**Verdict in one line:** technically credible, epistemically unusually honest, economically
viable at €0 burn — and commercially unproven, on a timeline that is probably 40–60%
optimistic. Proceed, with the modifications in §7.

---

## 2. Question 1 — Probability of success

"Success" must be decomposed to be answerable. Estimates are subjective probabilities,
conditioned on the plan as written; the right-hand column is what would move each number most.

| Milestone | Estimate | Dominant driver |
|---|---|---|
| **Technical delivery** — the product works as specified (Rung 0, verified reports, watches, billing) | **75–85%** | Solo-founder timeline risk, not feasibility. Every component is proven technology; the verification pipeline is unusual but deterministic. |
| **First paying customers** (any revenue) | **45–60%** | Whether 90–180s reports convert *anyone*; whether launch reaches the right audience at all |
| **$1.5–2k MRR by month 8** (the roadmap's stated target) | **15–25%** | Distribution. The target requires ~40–80 paying customers from a standing start with zero marketing budget on an optimistic timeline. |
| **Sustainable business** ($2k+ MRR held for 6+ months, founder still solvent and sane) | **20–30%** | Retention (watch adoption) plus the founder outlasting the slow-growth trough |

For calibration: these numbers are *well above* base rates for solo bootstrapped SaaS
(commonly cited failure rates exceed 90%), and deliberately so, for three reasons:

1. **The cost structure removes the most common killer.** Most bootstrapped SaaS dies of
   burn before product-market fit. This plan's burn is ~€15/month. It cannot die of costs;
   it can only die of neglect or irrelevance. A product that can wait indefinitely for its
   market has structurally better odds than one on a runway.
2. **The differentiation is real and defensible.** Verified citations, honest gaps, trap-subject
   resistance, and monitoring are genuine advantages that agent-loop competitors *cannot*
   copy without abandoning their architecture (a report that changes on every run cannot be
   regression-tested). This is a moat made of discipline rather than technology, which is the
   kind a solo founder can actually hold.
3. **The plan is unusually self-aware.** It names its own most-likely cause of death (R9),
   its own quality ceiling (extraction, not the model), and its own conversion trap (R12).
   Plans that know where they are weak get fixed; plans that don't, don't.

And they are *capped* by three things:

1. **The invisible-advantage problem (R12).** Accuracy is invisible at first glance; latency
   is not. The product's best qualities require either a second visit or a burned first
   impression to appreciate. Free-tier latency (90–180s) will be compared side-by-side
   against tools answering in seconds, and most first-time visitors will not know that the
   fast tool's report contains errors.
2. **Distribution effort is under-allocated** relative to its stated risk ranking (§5.1).
3. **The timeline is optimistic** (§5.2), and a solo founder who expects month 8 and reaches
   month 14 is at high risk of abandoning a plan that was actually working slowly.

### 2.4 The caveat on "competes only with existing competition"

The estimates above hold competition constant, as the question specified. In reality the
plausible 12-month threats are: frontier chat products adding live browsing with citations
by default (erodes the one-shot report; does *not* erode monitoring or verification), and
free-tier expansions from funded CI vendors (erodes pricing headroom). The plan's durable
positions against both are the watch loop, the verification pipeline, and the privacy claim.
The one-shot report alone would not survive as a business; the plan already treats monitoring
as the retention hinge, which is correct.

### 2.5 Is anything fatal? No — and here is the reasoning, not just the reassurance

Every risk named in this evaluation and in the plan's own register comes with a mitigation
already specified or specified here. But the stronger claim deserves its own argument: **no
identified problem is fatal, because every one of them is a positioning problem in disguise,
and positioning is a choice.** Walking the worst three:

**The invisible-advantage problem (accuracy is unseen, latency is seen).** Fatal only if the
product must win *first-glance* comparisons in front of a *speed-sensitive* audience. Neither
is fixed. The fix is niche selection: sell to the people for whom a wrong number has a
*named personal cost* — an associate whose diligence memo gets checked by a partner, an
agency whose client deliverable carries their letterhead, a founder pasting numbers into an
investor deck, a journalist who prints a correction. For that buyer, "every claim carries a
verifiable receipt, and we delete what we cannot prove" is not an invisible property — it is
the purchase reason, and 90 seconds is nothing against the hour of checking it replaces.
The mass-market casual comparer was never the beachhead; the plan should say so explicitly
in the forthcoming DISTRIBUTION.md.

**Distribution failure (R9).** Fatal only for products that get one launch. This product's
economics (€15/month) permit unlimited attempts at unlimited niches: if "competitive analysis
for founders" doesn't catch, the same engine is "vendor due-diligence for procurement,"
"pre-pitch research for agencies," "market scans for accelerator cohorts," or a white-label
report engine — each a repositioning, not a rebuild, because the pipeline is subject-agnostic.
The mitigation is to *plan* for multiple positioning attempts (the §5.3 pivot gates) instead
of treating the first launch as the verdict.

**Free-tier latency gating revenue that would fix latency (R12).** Already has three
independent escape valves in the plan: BYOK (the user fixes it themselves today), the
concierge/manual channel (revenue with zero latency constraint), and the one written
exception to the 20%-of-MRR rule. A problem with three exits is not a trap.

The honest formulation is therefore not "nothing can kill this" — neglect can, and a founder
who reads a slow month as a dead product can. It is: **every external obstacle identified
here converts into a smaller, better-fitting niche rather than a wall, and the plan's cost
structure grants unlimited attempts to find it.** What capitalism does not forgive is
spending money you do not have while searching — which is the one mistake this plan is
structurally incapable of making.

---

## 3. Question 2 — Missing topics and documents

### 3.1 Genuinely missing (should exist before or during early implementation)

| Missing | Why it matters | Effort |
|---|---|---|
| **GTM / distribution plan** (`DISTRIBUTION.md`) | The #1 ranked risk has no owning document. Needed: positioning statement for Landscape itself, the launch narrative, channel-by-channel plan with weekly cadence, build-in-public strategy, and the comparison-page SEO plan as a schedule rather than a mention. | 1–2 days |
| **Validation gate** (section in ROADMAP, or `VALIDATION.md`) | The plan has no demand checkpoint before Phase 6. Define now: what evidence of demand must exist by end of Phase 2, and what happens if it doesn't (reposition, not push on). | Half a day |
| **Pre-launch asset plan** | Waitlist landing page from week 1; the domain earns age and the list earns launch day. Currently nothing exists until Phase 6. | Half a day to plan, one day to ship |
| **Name and trademark check** | "Landscape" is generic, collides with several existing products, and is nearly un-SEOable. A rename after launch is far more expensive than one now. Not a document — a task, this week. | Hours |
| **Ops runbook** (`RUNBOOK.md`) | R7 promises "a documented 10-minute runbook for the three known failure modes" — it is referenced but nowhere scheduled as a deliverable. | Grows with Phase 0–1 |
| **App-wide accessibility standard** | Charts have an a11y spec (§7.3 of the report doc); the application does not. Radix provides primitives, but keyboard flows, focus order, and reduced-motion for streaming output need a stated bar. | Half a day |

### 3.2 Referenced but not yet created (expected — scheduled deliverables)

`docs/decisions/` ADR seeds · `TUTORIAL.md` · `BENCHMARKS.md` · `/methodology` content ·
seed KB articles · legal page contents. All are correctly scheduled in phases; listed here
only so the reviewer knows they are absences by design, not oversight.

### 3.3 Decisions worth recording that currently live nowhere

- **English-only v1** — implied everywhere, stated nowhere. One line in an ADR.
- **The comparative benchmark** (Landscape vs. free alternatives on the same 20 subjects) —
  discussed in review, agreed to be a Phase 2 exit criterion, **never actually added to the
  documents**. This is a real gap between the conversation and the repository.
- **Review-mining for unmet needs** ("I wish it did X" patterns from 2–3★ reviews) —
  identified in review as the one technique worth adopting from competitors; not yet in
  COMPETITIVE_ANALYSIS_REPORT §5.
- **Crunchbase: deferred, with reasoning** (cost, licensing, coverage bias, secondary-source
  status) — discussed and agreed in review, not recorded. Six months from now this gets
  re-litigated, which is exactly what the ADR process exists to prevent.

---

## 4. Question 3 — Per-document completeness

| Document | Complete on its topic? | Gaps found |
|---|---|---|
| **ROADMAP.md** | Mostly | Exec summary still says **"seven-section format"** (now nine). No validation gate (§3.1). Timeline optimistic (§5.2). No bus-factor note — a solo founder's two-week illness stalls support SLAs and corrections commitments that are publicly promised. |
| **PRODUCT_SPEC.md** | Mostly | §4.2's rendered example is still the **old 7-section report** — the single most-read part of the spec shows a format the product no longer has. No mobile-experience spec (reports will be read on phones; the composer flow assumes desktop). No returning-user empty states. |
| **ARCHITECTURE.md** | Yes, strong | The perceived-latency ladder in PRODUCT_SPEC §2.1 and the section-ordering in §2.4 predate deterministic-first pricing; mostly reconciled, but the SSE event list has no `pass2`/version events yet. Minor: no rate-limit numbers table for the API itself. |
| **ARCHITECTURE_EXPLANATION.md** | Yes | §1.2 still describes "a seven-section structure." No entry yet for the two-pass/render-tier decision (the newest §5.5 material). |
| **COMPETITIVE_ANALYSIS_REPORT.md** | Yes, strong | No full worked example of the nine-section report — the strongest possible spec artifact and currently absent. Review-mining (§3.3 above) belongs in §5. |
| **COMPETITIVE_DISCOVERY.md** | Yes, strong | **The review-platform access risk is unexamined** — see §6.1, the most important single finding in this evaluation. No handling for the "category genuinely does not exist yet" case beyond the 0–2-candidates question. |
| **FACT_CHECKING.md** | Yes, strong | Intro says "numbered so **§11** can map each" — the mapping table is **§9**. Subsection order goes 3.2.1 → 3.2.2 → 3.2.3 → 3.2.4 → 3.2.5 with 3.2.5 (language rules) sitting after the settings it governs; harmless but worth a renumber. |
| **QUALITY_GUARDRAILS.md** | Yes | Golden set (§3.1) still specifies only *named-product* subjects. Discovery-shaped prompts (class C/D/E inputs) are now half the product's difficulty and are unevaluated. The comparative benchmark (§3.3 above) belongs in §3.2. |
| **SUPPORT_SYSTEM.md** | Yes | Complete for its scope. |
| **CODING_QUALITY.md** | Yes | Complete; budgets self-identified as needing Phase 1 tuning. |

**Pattern worth naming:** every gap in the left-behind category (7-section references, the
old example, §11-vs-§9) is *drift from incremental revision* — the documents were internally
consistent at each commit and the sweeps missed stragglers. This is precisely the failure
mode CODING_QUALITY predicts for code and cures with CI checks; the documents have no
equivalent. A trivial CI grep for banned stale terms ("seven-section") would have caught all
of these. Worth adding when the repo gains CI.

---

## 5. Question 4 — Is the roadmap complete?

Structurally yes: phases have goals, ship-lists, technical tasks, instrumentation, exit
criteria, and cost posture — that discipline is genuinely rare. Three completeness problems
remain:

### 5.1 Distribution is one phase; it needs to be a workstream

R9 says distribution is the most likely cause of death. The roadmap's answer is Phase 6, one
launch window in week 20–24. Everything known about zero-budget product launches says the
audience must exist *before* launch day. Missing from every phase before 6: a standing weekly
distribution task (build-in-public updates, waitlist growth, seeding comparison pages as
static content early, engaging the communities that will later be launch channels). This is
~2–4 hours a week from Phase 1 onward and probably changes the Phase 6 outcome more than any
technical work in the plan.

### 5.2 The timeline is optimistic, and the plan should say by how much

30 weeks at 30–40 focused hours/week, solo, through: an inference bake-off, a verification
pipeline, a chart renderer, billing, a KB, watches with noise suppression, BYOK with
credential custody, and a two-pass render tier. Phases 1–2 alone (the engine plus
verification plus now the feature matrix and charts) are the likeliest to overrun, and the
scope added during this review (nine sections, matrix extraction, SVG charts, two-pass,
source classes) landed almost entirely in Phases 1–5 **without the week counts moving**.
A 1.5× multiplier is the honest planning assumption: **call it 42–48 weeks to Phase 6**, or
cut scope. The danger is not the delay itself — €15/month tolerates any delay — it is a
founder who planned for month 8, arrives at month 12 pre-revenue, and reads a working slow
plan as a failed one. Re-baselining now is cheap; despairing later is not.

### 5.3 No kill/pivot criteria

Exit criteria say when a phase is *done*; nothing says when the plan is *wrong*. Two gates
worth writing down now, while no ego is invested: (a) end of Phase 2 — if N concierge/beta
users have seen reports and none would pay or return, stop building and fix the product
concept, not the code; (b) 3 months post-launch — if organic signups are near zero despite
the SEO surface, the problem is positioning, and more features will not fix it.

---

## 6. Question 5 — Additional missing details

### 6.1 The most important finding: the review-platform dependency is at risk from the plan's own ethics

COMPETITIVE_DISCOVERY names review-site category pages as the **highest-yield discovery
channel**, and the sentiment section leans on review platforms. But major review platforms
aggressively restrict automated access — restrictive robots.txt, bot-detection, and terms
that prohibit scraping. **The plan honours robots.txt as a hard ethical commitment
(FACT_CHECKING, QUALITY_GUARDRAILS).** These two positions may directly collide: the
discovery channel ranked #1 and one of nine report sections may be substantially unavailable
*by the plan's own rules*.

The documents never confront this. Needed, in Phase 1: an explicit access audit of the named
platforms' robots policies; a stated fallback ranking for discovery (vendor `/alternatives`
pages, marketplaces, community threads — already present, but not framed as the primary
path); and honest treatment in the sentiment section ("review platforms could not be accessed
under our fetching rules" is exactly the kind of disclosed gap the product already knows how
to render). This is survivable — the multi-channel design was built for exactly this — but
discovering it in week 8 instead of deciding it in week 1 would cost real time and morale.

### 6.2 Smaller items, in decreasing order of consequence

- **Search dependency realism.** SearXNG instances are routinely rate-limited by upstream
  engines; the plan knows this and holds Brave as fallback, but the *budget* for Brave
  (paid, per-query) appears nowhere in the cost ladder. A number belongs in §6.4 of ROADMAP.
- **Internet Archive rate limits.** The plan submits tier-1 pages to the Archive and uses
  CDX for longevity; both endpoints are rate-limited and occasionally slow or down. Needs
  the same deferred-job treatment as rendering, not inline calls.
- **Text-fragment deep links** don't work in Firefox (partial support). Fine — the snapshot
  link is the fallback — but the docs assert the feature without the caveat.
- **The `analyses` table schema drift**: fields added across four commits
  (`inference_provider`, `strictness_setting`, `version`, `completeness`…) have never been
  re-checked as one coherent whole. One pass before migration-writing.
- **Anonymous quota (2/day) vs. two-pass**: does a pass-2 update consume quota? (It should
  not; unstated.)
- **BYOK + two-pass interaction**: pass 2 re-runs sections with whose model? (Presumably the
  same provenance as pass 1; unstated.)

### 6.3 Known inconsistencies to fix (one small commit)

1. ROADMAP exec summary: "seven-section" → nine.
2. ARCHITECTURE_EXPLANATION §1.2: same.
3. PRODUCT_SPEC §4.2: regenerate the rendered example as a nine-section report (worked
   example is the highest-value fix in this list).
4. FACT_CHECKING intro: "§11" → "§9".
5. QUALITY_GUARDRAILS §3.1: add discovery-shaped prompts to the golden set.
6. Add the comparative benchmark to QUALITY_GUARDRAILS §3.2 and Phase 2 exit criteria (agreed
   in review; never landed).
7. Record the Crunchbase deferral and English-only-v1 as ADR-style notes.

---

## 7. Question 6 — Insights to improve the odds

Ranked by expected impact per unit of effort.

1. **Sell the report before building the machine.** Two weeks, parallel with Phase 0: produce
   5–10 reports *by hand*, following the documented method exactly (the documents are
   effectively an analyst's SOP already), for real founders found in the communities Phase 6
   plans to launch in. Charge a token amount — willingness to pay $10 for a hand-made report
   is more evidence than a hundred waitlist emails. Every hand-made report also becomes a
   golden-set reference sheet, a testimonial, and a format test. This converts the plan's
   largest unknown (does anyone want this?) from a week-20 discovery into a week-2 one.
2. **Make distribution a standing weekly task from Phase 1** (§5.1). 2–4 hours/week.
3. **Resolve the review-platform question in week 1** (§6.1), before any architecture
   assumes the channel exists.
4. **Publish the trap-subject benchmark at launch.** Run the nonexistent-product test against
   the well-known free alternatives and publish the results with screenshots. It is cheap,
   newsworthy, demonstrates the differentiator in ten seconds, and — because the product's
   architecture is the only one in the comparison that *structurally cannot* fail the test —
   it is a marketing asset competitors cannot neutralise without rebuilding.
5. **Elevate watch creation into the first-run flow.** The plan already believes watches are
   the retention hinge (Phase 7 expects to confirm it). Don't wait: make "watch these pages"
   part of the first report's completion moment, not a separate later discovery. One-shot
   analysis is the demo; monitoring is the product.
6. **Re-baseline the timeline publicly in the roadmap** (§5.2) so that month 12 pre-revenue
   reads as on-plan rather than as failure.
7. **Add the stale-term CI check for documentation** (§4, "pattern worth naming") when CI
   exists — the docs are now the product's source of truth and have the same drift dynamics
   as code.

---

## 8. Recommended next steps, in order

1. **Human review and merge of PR #1.** Nothing else proceeds until the baseline is agreed.
2. **Follow-up documentation PR** (small, one day): the seven fixes in §6.3, the validation
   gate (§3.1), the timeline re-baseline (§5.2), and the kill/pivot criteria (§5.3).
3. **This week, in parallel, three founder tasks** (no code): provision the Oracle A1 and
   convert to PAYG *now* (capacity scarcity is real and R11 depends on it); run the name /
   trademark check; put up the waitlist landing page.
4. **Concierge validation sprint** (weeks 1–2, parallel with Phase 0 benchmarking): 5–10
   hand-made reports for real founders, per §7.1.
5. **Phase 0 as written** — plus the week-1 review-platform access audit (§6.1).
   *(Editor's note, 2026-08-04: this recommendation said "on the actual A1". That is no longer
   how Phase 0 measures anything —
   [ADR 0011](decisions/0011-no-experiments-on-production.md). The bake-off it refers to ran
   locally and closed both criteria; the text is left as it was written.)*
6. **Write `DISTRIBUTION.md`** before Phase 1 begins, and begin the weekly distribution
   cadence with Phase 1.
7. **Then Phase 1**, with the golden set gaining discovery-shaped prompts from the concierge
   reports.

---

## 9. Bottom line

The plan is in the top tier of what a solo founder could reasonably produce before writing
code: technically sound, economically robust at €0 burn, epistemically honest to a degree
that is itself the product's moat, and unusually willing to name its own weaknesses. Its
engineering risk is low; its market risk is high and — more importantly — **currently
scheduled to be discovered late**. The single change that most improves the odds is moving
first user contact from week 20 to week 2. Everything else in this evaluation is refinement;
that one is direction.

*Probability estimates herein are the evaluator's calibrated judgment, not measurements, and
should be re-derived by the human reviewer against their own base rates.*
