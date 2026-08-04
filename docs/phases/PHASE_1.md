# Phase 1 — your actions

> **Goal of the phase:** a stranger types into the box and reads a real, streamed, cited
> report. *"This is the single most important phase; everything after it is commerce and
> polish."*

**Most of this phase is mine.** Two things are not, and the second one is the gate that
closes the phase.

**Where it stands:** the Postgres queue, the entity-resolution gate, and `landscape-fetch`
are built. Source discovery, extraction, the orchestrator, SSE streaming, the seven report
sections and the caches are not.

---

## Step 1 — Test each piece as it lands, and tell me when it is wrong

| | |
|---|---|
| **Do** | Work through the walkthrough on the current branch whenever I open a PR, and report anything that behaves differently from what it says |
| **Where** | [`docs/Feature_Walkthrough.md`](../Feature_Walkthrough.md) — every part has a "Do this / You should see" pair |
| **Time** | 15–30 minutes per PR |
| **Done when** | You have run the parts that changed, and either the checklist rows tick or you have told me which did not |
| **I confirm by** | **Automatically.** Every command in that document is executed by CI before it is written down. If one fails for you, that is a real bug — the document and the code are checked against each other on every push |

**This is not a formality.** Every user-facing bug this project has shipped was found by you
running something and it not working: the port, the missing `content-type` header, the
ambiguous `cargo run`, the Windows shell quoting. **None of them was caught by a test**,
because each was in prose or in wiring rather than in a function.

**The most useful report format** is the command you ran, what the document said would
happen, and what actually happened. I do not need a diagnosis.

---

## Step 2 — Decide whether it is good enough to show a stranger

**This is Phase 1's exit criterion and it is a judgement, not a measurement.** The roadmap
states it exactly as: *"The founder would show it to a stranger without apologizing."*

| | |
|---|---|
| **Do** | Run 20 analyses of subjects you actually care about, and answer one question honestly: would you show this to someone whose opinion you value, without a preamble explaining what to ignore? |
| **Where** | The running application, once the pipeline is complete |
| **Time** | An afternoon |
| **Done when** | You have answered yes, or told me specifically what the apology would have been about |
| **I confirm by** | **You tell me.** The other three exit criteria I verify automatically — 20 consecutive runs without a crash, p50 ≤ 240s and time-to-first-content ≤ 45s from the instrumentation, 100% schema validity from the golden set. **This fourth one is deliberately yours**, because a number cannot hold it |

**"What would the apology be about" is the useful output**, more than yes or no. *"The prices
are right but it missed two obvious competitors"* is a discovery task. *"It reads like a
robot wrote it"* is a synthesis task. Those are different work and I would build the wrong
one from a no.

---

## Step 3 *(conditional)* — Only if the review platforms said no

Skip this if [Phase 0 Step 2](PHASE_0.md#step-2--the-source-terms-audit-track-d) came back
permissive.

| | |
|---|---|
| **Do** | Choose which fallback discovery channel becomes the primary one |
| **Where** | The options are in [`COMPETITIVE_DISCOVERY.md`](../COMPETITIVE_DISCOVERY.md): vendor `/alternatives` pages, marketplaces, community threads |
| **Time** | A conversation |
| **Done when** | You have picked one and I have written the ADR |
| **I confirm by** | **Automatically once decided.** The ADR exists and the discovery code follows it |

**Why it is a decision rather than a default.** The roadmap is explicit that this should be
promoted *"by decision rather than by discovery in week 8"* — i.e. it should not be something
we back into when the primary channel turns out to be closed.

---

## What I am doing meanwhile

For context, so you can see what a PR is likely to contain next. None of this needs you:

| Piece | State |
|---|---|
| Postgres schema + job queue | ✅ Done |
| Entity resolution + disambiguation gate | ✅ Gate done; candidate generation pending |
| `landscape-fetch` — SSRF guard, robots, rate limits, size caps | ✅ Done |
| **JS-rendering gap instrumentation** | **Next.** Two counters, one day. Decides whether a browser tier is ever built |
| Source discovery — structured probes, then search | After that |
| Extraction → Markdown, quality scoring | |
| `landscape-analyze` orchestrator, per-analysis budgets | |
| SSE streaming with replay buffer | |
| The seven report sections | |
| Fetch + per-source extraction caches | |
| Golden set 10 → 25 subjects | |

**The one I would flag:** the JS-rendering gap measurement. If most pricing pages yield no
price from static HTML, a browser tier appears that is not currently in the plan — and that
is a schedule change worth knowing about in week 3 rather than week 8. It is one day of work
and I would rather spend it before writing discovery than after.

---

## Phase 1 closes when

- 20 consecutive analyses of varied subjects complete without a crash — *I verify*
- p50 end-to-end ≤ 240s, time-to-first-content ≤ 45s — *I verify*
- 100% schema validity — *I verify*
- **You would show it to a stranger without apologizing** — *you decide*

Three of four are mine. The fourth is the one that matters.
