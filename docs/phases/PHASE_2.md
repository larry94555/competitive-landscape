# Phase 2 — your actions

> **Goal of the phase:** make the output trustworthy and exportable. *"This is where the
> product earns the right to charge money."*

**One action is yours**, and it needs ten other people, so it is worth reading before the
phase starts rather than when it ends.

---

## Step 1 — Find ten external testers and get a rating

| | |
|---|---|
| **Do** | Get **10 people who are not you** to use it and rate the report's usefulness out of 5 |
| **Where** | The running product. The concierge recipients from [Phase 0](PHASE_0.md#step-2--the-concierge-five-the-g1-gate) are the obvious first five |
| **Time** | A week or two of calendar, an hour or two of yours |
| **Done when** | Ten ratings recorded, **averaging ≥ 4/5** |
| **I confirm by** | **You tell me the ratings and I record them** in `ROADMAP.md`. I have no way to see them, and I will write down the average you give me rather than the one that would be convenient |

**Start recruiting during Phase 1, not Phase 2.** Ten people who will actually look at
something and answer honestly is harder to arrange than it sounds, and this is the criterion
most likely to hold the phase open while everything else is green.

**The concierge five are the natural starting point** — they have already seen a hand-made
report, so they can compare, which is more useful than a cold rating.

---

## Step 2 *(conditional)* — A decision I will bring you

Not an action to schedule. It is here so the decision is not a surprise.

| | |
|---|---|
| **What** | Whether to build a **browser-rendering tier** |
| **When** | After I re-measure the JS-rendering gap with tiers 2–4 in place |
| **The rule** | Under ~5% residual → the tier is **not built** and the honest-gap treatment stands. Materially more → tier 5 and the two-pass flow get scheduled |
| **Your part** | Agreeing to the schedule change if it is material. A browser tier is a real cost in RAM on a 24 GB box shared with three models |
| **I confirm by** | **Automatically.** I write the decision either way — the roadmap requires a written decision even when the answer is "no tier", so the reasoning survives |

---

## Phase 2 closes when

| Criterion | Who |
|---|---|
| Citation coverage ≥ 97%, drop rate ≤ 3%, trap subjects produce zero fabricated content | *I verify* — the golden set already measures fabrication |
| JS-rendering gap re-measured, with a written decision either way | *I verify* |
| p50 ≤ 180s, time-to-first-content ≤ 40s | *I verify* |
| PDF click-to-download ≤ 1s pre-warmed | *I verify* |
| **10 external testers rate usefulness ≥ 4/5** | **You** |

---

# Phases 3–8 — what will be asked of you

Summarised rather than detailed, because a checklist written 20 weeks early is a checklist
written wrong. Each gets its own page when its phase starts.

| Phase | What is yours | Roughly |
|---|---|---|
| **3** — Accounts, limits, knowledge base | Write the support content before it is needed. Decide the anonymous/registered quota split | Weeks 10–12 |
| **4** — Monetization | **The pricing decision**, and a payment provider account. Neither is mine to make | Weeks 13–15 |
| **5** — Watchlists & notifications | A sending domain, and its DNS records. Decide what is worth interrupting someone for | Weeks 16–19 |
| **6** — Cold start & growth | **All of it.** Community presence, launch posts, the first hundred users. An agent cannot build an audience | Weeks 20–24 |
| **7** — Retention, scale, model upgrade | Approve the model swap after shadow evaluation. Decide when to spend on Rung 2 hardware | Weeks 25–30 |
| **8** — Sustainability | Decide what the project is for once it works | Week 31+ |

**Two of these deserve early thought even though they are far off:**

**Phase 4's pricing decision** is informed by Phase 0's concierge track. What people paid for
a hand-made report — and whether they hesitated — is the only real data the plan will ever
have on this, and it is being collected now.

**Phase 6 is the one an agent cannot help with.** Every phase before it produces something I
can build and verify. That one is you talking to people, and it is scheduled 20 weeks out on
the assumption that the earlier phases will have produced something worth talking about.
