# Evaluation Action Plan

> A disposition record for every finding in [Fable_Evaluation.md](Fable_Evaluation.md):
> what was changed, what was not, and what each means for implementing the roadmap.
>
> **Method.** Every finding in the evaluation was enumerated, dispositioned, and — where a
> change was made — verified against the committed documents rather than against intent.
> Findings are given IDs (F1…F40) so the justifications in §3 map one-to-one onto the table
> in §2. Document changes landed in commit `dfc5808`.
>
> **The bar for "no change needed"** is deliberately high: it requires showing that the
> *existing* plan, if followed, already handles the concern — not merely that the concern is
> minor. Where that cannot be shown, a change was made.

---

## 1. Summary

| Disposition | Count |
|---|---|
| Changed — documents updated | 34 |
| No change — existing plan sufficient | 3 |
| No change — deferred to a later phase with a stated trigger | 2 |
| No change — analysis, not an actionable finding | 1 |

**Net effect on the roadmap:** Phase 0 gains two parallel founder-run tracks and grows from
2 weeks to ~3; a standing weekly distribution commitment begins at Phase 1; Phase 2 gains one
exit criterion; the overall timeline is re-baselined from 30 to 42–48 weeks. **No phase's
technical scope was reduced, and no new engineering work was added** — every change is
sequencing, validation, or documentation. That distinction matters: the evaluation found
process gaps, not design defects.

---

## 2. Disposition table

### 2.1 Structural findings

| ID | Issue identified by Fable | Change made? | Impact on roadmap implementation |
|---|---|:--:|---|
| **F1** | 20 weeks of building before first user contact | **Yes** | **Largest single change.** Phase 0 gains Track A: 5–10 hand-made concierge reports, delivered and charged for, in parallel with benchmarking. Phase 0 grows ~1 week. Gate G1 blocks Phase 1 if nobody wants a second report. |
| **F2** | Distribution under-allocated relative to its own risk ranking | **Yes** | New standing commitment: 2–4 h/week from Phase 1 onward (ROADMAP §2A.1), plus `DISTRIBUTION.md`. Consumes ~5–8% of weekly capacity for the whole project. |
| **F3** | Timeline 40–60% optimistic | **Yes** | Re-baselined to **42–48 weeks to Phase 6**, with reasoning stated. No scope change; expectation change only. |
| **F4** | No kill/pivot criteria — nothing says when the plan is *wrong* | **Yes** | Four validation gates (G1–G4) at Phase 0, 2, 5 and 3-months-post-launch, plus a four-rung pivot ladder (ROADMAP §2A.2–2A.3). Each gate can halt or redirect a phase. |
| **F5** | No `DISTRIBUTION.md` | **Yes** | Created. Three sections marked **[FOUNDER]** — beachhead, positioning, launch narrative — deliberately left unwritten pending concierge evidence. |
| **F6** | Bus factor: public SLAs assume a continuously healthy founder | **Yes** | ROADMAP §2A.4: SLAs restated as targets-with-exception, status page, away-mode switch, `RUNBOOK.md`. Adds ~2 days across Phases 0–3. |

### 2.2 The technical finding that mattered most

| ID | Issue | Change made? | Impact |
|---|---|:--:|---|
| **F7** | **Review-platform access may collide with the plan's own robots.txt commitment** — the #1 discovery channel and part of the sentiment section may be unavailable by our own rules | **Yes** | `COMPETITIVE_DISCOVERY.md` §5.1.1: a **week-1 access audit** with three pre-decided outcomes and a fallback channel ranking. Moves a possible week-8 crisis into a week-1 decision. Phase 0 task; no engineering cost. |

### 2.3 Missing documents and unrecorded decisions

| ID | Issue | Change made? | Impact |
|---|---|:--:|---|
| **F8** | Validation gate missing | **Yes** | See F4. |
| **F9** | Pre-launch assets (waitlist) not planned | **Yes** | Phase 0 Track B. ~1 day. Domain begins ageing and the list begins growing in week 1. |
| **F10** | Name / trademark check never scheduled | **Yes** | Phase 0 Track B, week 1. Hours of work; renaming after launch costs weeks. |
| **F11** | `RUNBOOK.md` promised by R7, never scheduled | **Yes** | Started in Phase 0, grows with the system. |
| **F12** | No app-wide accessibility standard | **Yes** | `CODING_QUALITY.md` §7.2: `jsx-a11y` + `axe-core` in Playwright flows, with `prefers-reduced-motion` called out for the 90–180s streaming report. Adds CI checks, not phases. |
| **F13** | English-only v1 implied but never stated | **Yes** | Recorded as a deferred decision with reasoning. |
| **F14** | Comparative benchmark agreed in review, never landed in the repo | **Yes** | `QUALITY_GUARDRAILS.md` §3.2A + **Phase 2 exit criterion**. ~2 days of work in Phase 2. |
| **F15** | Review-mining for unmet needs agreed, never landed | **Yes** | `COMPETITIVE_ANALYSIS_REPORT.md` §5. Small addition to an existing section; no new pipeline. |
| **F16** | Crunchbase deferral discussed, never recorded | **Yes** | Recorded with all four reasons (cost, licensing, coverage bias, secondary-source status) and the free-source alternatives. |

### 2.4 Per-document completeness

| ID | Issue | Change made? | Impact |
|---|---|:--:|---|
| **F17** | ROADMAP exec summary says "seven-section" | **Yes** | Corrected to nine. |
| **F18** | `PRODUCT_SPEC.md` §4.2 example is the old 7-section report | **Yes** | **Replaced with the full nine-section worked example** — source classes, five-state matrix cells, a not-used source, an auditable negative, chart placeholders. Highest-value single documentation fix. |
| **F19** | No mobile experience spec | **Yes** | `PRODUCT_SPEC.md` §2A. Notably: matrix scrolls in its own container, citations open a sheet, `Last-Event-ID` replay tested on a real device. |
| **F20** | No returning-user / empty states | **Yes** | `PRODUCT_SPEC.md` §2A table, five states including pass-2-pending. |
| **F21** | SSE event list has no pass-2 / version events | **Yes** | Added `completeness` on `done` and a `version` event. |
| **F22** | No API rate-limit numbers table | **No — deferred** | The public API is a **Phase 7** deliverable; numbers set now would be guesses. Trigger: write them when the API spec is written. |
| **F23** | `ARCHITECTURE_EXPLANATION.md` §1.2 says "seven-section" | **Yes** | Corrected. |
| **F24** | No explanation entry for the two-pass / render decision | **Yes** | §3.6e added, including why report versioning was not optional. |
| **F25** | No full worked example of the nine-section report | **Yes** | See F18. |
| **F26** | Category-does-not-exist-yet case unhandled | **Yes** | `COMPETITIVE_DISCOVERY.md` §6.1: the report **changes shape** — substitutes and adjacent categories, plus the searches run — rather than failing. |
| **F27** | `FACT_CHECKING.md` intro says §11, mapping is §9 | **Yes** | Corrected. |
| **F28** | §3.2.5 sits after the settings it governs | **No — cosmetic** | Subsections already read in order (3.2.1→3.2.5); renumbering would break inbound cross-references in four documents for no reader benefit. |
| **F29** | Golden set contains only named-product subjects | **Yes** | Discovery-shaped prompts (classes C/D/E) added with curated expected sets, plus discovery precision ≥80%, recall ≥60%, classification ≥75% gates. |
| **F30** | Documentation drift has no CI check | **Yes** | `CODING_QUALITY.md` §8.3: banned-stale-term grep fails the build. |

### 2.5 Detail-level findings

| ID | Issue | Change made? | Impact |
|---|---|:--:|---|
| **F31** | Brave Search fallback is paid but absent from the cost ladder | **Yes** | ~$5–15/mo contingency from Phase 2, flagged as **the only line that scales with usage**. |
| **F32** | Internet Archive rate limits need deferred-job treatment | **Yes** | Archive submission queues alongside render jobs; never inline. |
| **F33** | Text fragments unsupported in Firefox | **Yes** | Caveat stated; snapshot is the always-offered fallback. |
| **F34** | `analyses` table has drifted across four commits | **No — implementation task** | Correctly a **schema-coherence pass immediately before writing the first migration** (Phase 1). Doing it in prose now would be re-done at implementation. |
| **F35** | Does pass 2 consume anonymous quota? | **Yes** | Stated: quota counts analyses, not passes. |
| **F36** | Which model does pass 2 use under BYOK? | **Yes** | Stated: same provider as pass 1, with fallback disclosed. |
| **F37** | Concierge validation sprint | **Yes** | See F1. |
| **F38** | Watch creation should be in the first-run flow | **Yes** | `PRODUCT_SPEC.md` §2.1: presented at the completion moment beside the PDF button. No new engineering — a placement decision. |
| **F39** | Publish the trap-subject benchmark at launch | **Yes** | `DISTRIBUTION.md` §6, with two non-negotiable conditions: publish losing dimensions too, describe what tools *did*, never what companies *are*. |
| **F40** | Competition assumed constant (§2.4) / fatality analysis (§2.5) | **No — analysis** | Not actionable findings. §2.5's conclusion is already operationalised by the pivot ladder (F4). |

---

## 3. Justifications

### 3.1 Why F1 (concierge validation) required action

The plan's cost structure means it cannot die of burn — but that same property removes the
forcing function that normally makes founders talk to users. A funded startup runs out of
money and is compelled to sell; this one can build indefinitely in comfortable isolation.
**The absence of financial pressure is a real risk, not only a benefit**, and nothing in the
original plan corrected for it.

The specific danger is compounding: the report format, the nine sections, the schema, the
charts and the verification thresholds are all *assumptions about what a user wants*, and
every subsequent phase builds on them. Discovering in week 20 that users only wanted the
pricing table would invalidate a great deal of correct work.

**During implementation:** deliver the reports personally and watch what happens — what the
recipient reads first, what they skip, what they ask for that is not there, and whether they
forward it. Record it. Each report doubles as a golden-set reference sheet, so the work is
retained even if the concierge channel itself is abandoned. G1 is a real gate: if nobody asks
for a second report, Phase 1 does not start as specified.

### 3.2 Why F2 (distribution workstream) required action

The plan itself ranked distribution above every technical risk and then allocated it one phase
out of nine. That is an internal contradiction, and internal contradictions in a plan are
resolved by whichever side has a schedule — which was the technical side.

A weekly commitment fixes it structurally rather than by exhortation. The rule that "if build
work and the distribution hour conflict, distribution wins" exists because that hour is
always the one that *feels* deferrable.

**During implementation:** protect the hour. The compounding channels — comparison pages, the
KB, shared reports — are the ones worth prioritising, and none requires the app to be
publicly launched, so they can begin accumulating months before Phase 6.

### 3.3 Why F3 (timeline) required action, and why it is not a scope problem

The re-baseline changes no scope. It exists because **the failure mode is psychological, not
operational**: at €15/month the plan tolerates any delay, but a founder who planned for month
8 and finds themselves at month 12 pre-revenue may abandon something that is working slowly.
R13 already names founder abandonment as a risk; an optimistic timeline is one of its causes,
and re-baselining is the cheapest available mitigation.

**During implementation:** treat 42–48 weeks as the plan and anything faster as good news.

### 3.4 Why F4 (gates and pivot ladder) required action

Exit criteria answer "is this phase finished?" Nothing answered "is this plan wrong?" Those
are different questions, and the second is far harder to ask honestly *after* investing
months. Writing the gates now, before any ego is attached to the answers, is the entire point.

The pivot ladder matters because of a property the plan already has but never exploited:
**the engine is subject-agnostic.** Discovery, fetching, verification, reporting and
monitoring do not care whether the subject is a SaaS competitor, a procurement vendor, or a
grant funder. So a failed positioning is a relabelling exercise, not a rebuild — which is
precisely why no finding in the evaluation is fatal, and why the answer to "what if the
niche is wrong" is "try the next one" rather than "start over."

**During implementation:** record which positioning is being tested and for how long. A pivot
should be a decision with a date, not a drift.

### 3.5 Why F7 (review-platform access) was the most important technical finding

It is the only finding where **two of the plan's own commitments may be mutually
unsatisfiable**: robots.txt compliance is a hard ethical rule, and review-site category pages
are the highest-yield discovery channel. Both were written confidently, in different
documents, by an author who never put them side by side.

Action was needed not because the problem is unsolvable — the multi-channel design already
survives it, since candidates require ≥2 independent channels and eight are specified — but
because **the cost of discovering it late is entirely avoidable.** An audit costs an hour in
week 1. Finding out in week 8 costs a re-plan plus the morale of a surprise.

**During implementation:** run the audit before any code assumes the channel exists. If
access is disallowed, promote the fallbacks by decision and update the ranking in the
document. Report-side disclosure is already specified and doubles as a fairness control
(P21 warns that differential *access* must not read as differential *quality*).

### 3.6 Why F14, F15, F16 required action — the "agreed but never landed" class

These three were discussed in review, agreed, and never written down. That is the most
insidious documentation failure available: everyone believes the decision is recorded, so
nobody re-examines it, and six months later it is re-litigated from scratch. `CODING_QUALITY.md`
§8.1 exists specifically to prevent this, and these were three live counter-examples.

The comparative benchmark (F14) additionally converts the plan's central claim — that this
approach is *more accurate* than free alternatives — from belief into a measurement. Until it
runs, that claim is unevidenced, including in this document.

**During implementation:** the benchmark is a Phase 2 exit criterion. Publish the dimensions
where the product loses, not only where it wins.

### 3.7 Why F18/F25 (worked example) was the highest-value documentation fix

The rendered example is the artifact an implementer — human or agent — will look at first and
copy most literally. Leaving a seven-section example in place while specifying nine sections
does not merely mislead; it actively instructs the wrong thing. The new example carries the
source classes, the five matrix states, a not-used source, an auditable negative, and the
interpretation labelling, so the honesty mechanisms are demonstrated rather than described.

### 3.8 Why F19/F20 (mobile, returning states) required action

Reports get forwarded, and forwarded links open on phones. The plan's most distinctive
artifact — the feature matrix — is also the one most likely to break a narrow viewport, and
the 90–180 second wait is most fragile precisely where a screen locks. `Last-Event-ID` replay
was already designed for this; what was missing was the instruction to *test it on a real
device*.

### 3.9 Why F12 (accessibility) required action

Charts had an accessibility specification; the application did not. The specific hazard is
`prefers-reduced-motion`: this product animates continuously for 90–180 seconds, which is far
beyond typical UI motion and genuinely affects motion-sensitive users. Radix supplies correct
primitives, but nothing enforces keyboard flow, focus order or motion preference without a
stated bar and a CI check.

### 3.10 Why F30 (documentation drift check) required action

Every documentation inconsistency the evaluation found — three stale "seven-section"
references, an outdated example, a wrong section number — was a straggler left by an otherwise
complete sweep, and **all of them were greppable**. `CODING_QUALITY.md` already argues that
enforced beats agreed, and already applies that argument to code. The documents are now the
product's source of truth and have identical drift dynamics; extending the same mechanism is
consistent rather than novel.

### 3.11 Why F22 (API rate limits) needs no action now

The public API is a Phase 7 deliverable. Rate limits written today would be invented numbers
with no traffic data, no cost model at Rung 2, and no knowledge of what API consumers do —
and once written they would acquire false authority. **The existing plan is sufficient
because Phase 7 already specifies "API keys, per-key rate limiting, OpenAPI spec generated
from the same schema"** — the mechanism is planned; only the constants are absent, and
constants are exactly what should not be guessed. *Trigger: set them when the API spec is
written, informed by observed per-tier usage.*

### 3.12 Why F28 (subsection renumbering) needs no action

The subsections already read in sequence. Fable's objection was that the language rules appear
after the setting they govern — a reading-order nicety, not a comprehension problem, since the
rules are cross-referenced from every place they apply. Renumbering would invalidate inbound
references in four documents. **The existing structure is sufficient because no reader is
misled**; the cost of the fix exceeds its benefit, which is itself the standard
`CODING_QUALITY.md` §12 sets for whether a rule is worth keeping.

### 3.13 Why F34 (schema coherence) needs no action *now* but does need action later

The `analyses` table accumulated fields across four commits — `inference_provider`,
`strictness_setting`, `version`, `completeness`, `byok_key_id`, `fell_back_to_local` — and has
never been re-read as one coherent whole. That is a genuine defect.

But the right moment to fix it is **immediately before writing the first migration**, not in
prose now: any reconciliation done today would be redone at implementation, when field types,
nullability, indexes and constraints are actually decided. **The existing plan is sufficient
because `CODING_QUALITY.md` §3.1 already classifies a schema change as an architecture change
requiring design review and an ADR** — so the coherence pass is already mandated by process;
it simply needs doing at the right time.

*Action during implementation:* the first `landscape-db` PR reviews the full `analyses`
definition as a unit and produces an ADR, rather than transcribing accumulated prose.

### 3.14 Why F40 (analysis sections) needs no action

§2.4 (competition held constant) and §2.5 (nothing is fatal) are reasoning, not findings.
§2.5's operational conclusion — that every identified obstacle converts into a niche question
rather than a wall — is now embodied in the pivot ladder (F4), which is the actionable form of
that argument. Nothing further is required.

### 3.15 On the [FOUNDER] sections left blank in DISTRIBUTION.md

`DISTRIBUTION.md` was created, but its three most consequential sections — beachhead,
positioning statement, launch narrative — are deliberately unwritten.

This is not an omission. **Filling them now would be the exact failure the plan warns about
elsewhere**: a confident, plausible-sounding answer produced without evidence. The concierge
sprint (F1) exists to generate that evidence, and the structural argument for *which kind* of
buyer to test is already written — roles where a wrong number carries a named personal cost,
because that is the only audience for whom this product's invisible advantages are visible.

**During implementation:** write the positioning statement at the end of Phase 0, from what
the concierge recipients actually said, and record the date so that G4 can later judge whether
it worked.

---

## 4. What changes about implementing the roadmap

Consolidated, so the impact is legible without re-reading the table.

**Sequencing changes**
- Phase 0 grows from 2 to ~3 weeks and gains two founder-run tracks (concierge validation,
  pre-launch assets) plus the review-platform audit.
- A weekly distribution commitment begins at Phase 1 and never stops.
- Phase 2 gains the comparative benchmark as an exit criterion.
- Four validation gates can halt or redirect the plan at Phases 0, 2, 5 and post-launch.

**Expectation changes**
- Timeline re-baselined to 42–48 weeks to Phase 6.
- ~$5–15/month contingency for paid search fallback from Phase 2 — the only cost line that
  scales with usage.

**No change**
- No phase's technical scope was reduced or expanded. Every finding was a process, validation,
  or documentation gap. **The architecture survived the evaluation intact**, which is the most
  reassuring single result in this exercise — the design decisions made across twenty-three
  commits held up under adversarial review by the same model that made them.

**Three things to do before Phase 1 begins**
1. Provision the Oracle A1 and convert to Pay-As-You-Go — capacity is scarce and R11 depends
   on holding it.
2. Run the name and trademark check.
3. Deliver at least five concierge reports and record what happened.
