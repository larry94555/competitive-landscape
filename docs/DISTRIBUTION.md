# Landscape — Distribution

> The owning document for R9, the risk the plan ranks above every technical one.
>
> **Status: scaffold.** The mechanics below are settled. The three sections marked
> **[FOUNDER]** — beachhead, positioning statement, and launch narrative — are judgment calls
> that must be authored by the founder, not delegated. They are the "humans decide what
> correct means" half of [CODING_QUALITY.md](CODING_QUALITY.md) §11.2, and a plausible-sounding
> placeholder would be worse than an honest blank.

---

## 1. Why this document exists

R9 says distribution is the most likely cause of death — more likely than any technical
failure. Until now the plan's entire answer was one launch window in Phase 6. Everything known
about zero-budget launches says the audience must exist *before* launch day, which makes a
single-phase answer structurally insufficient.

The correction is in [ROADMAP.md](ROADMAP.md) §2A.1: **2–4 hours per week from Phase 1
onward**, every week, alongside the build. This document is what that time is spent on.

---

## 2. The buyer  **[FOUNDER]**

The single most consequential decision in this document, and the one most likely to be got
wrong by defaulting to "founders" because founders are who the author knows.

**The structural argument for who it should be.** The product's advantages — verified
citations, honest gaps, deleted-if-unprovable claims — are *invisible at first glance* and
only valuable to someone for whom **a wrong number has a named personal cost**. That is the
selection criterion. It points away from casual comparers and toward roles where being wrong
in writing is professionally expensive:

| Candidate beachhead | Why the citation trail matters to them | Why it might not work |
|---|---|---|
| Diligence / corp-dev associates | A partner checks their memo | May already have paid tools |
| Agency & consultancy strategists | The deliverable carries their letterhead | Price-sensitive; may want white-label |
| Founders preparing investor materials | Numbers get challenged in the room | Infrequent need; poor retention |
| Product managers doing quarterly reviews | Recurring, and monitoring fits naturally | Often have an internal CI function |
| Procurement / vendor evaluation | Formal, documented, auditable by nature | Longer sales cycle |

**[FOUNDER] — pick one to start.** Not two. The concierge sprint (ROADMAP Phase 0, Track A)
exists to make this choice from evidence rather than intuition: deliver hand-made reports to
people in two or three of these groups and see who asks for another one.

---

## 3. Positioning statement  **[FOUNDER]**

To be written after the concierge sprint, in this shape:

> For **[beachhead]** who need **[job]**, Landscape is **[category]** that **[differentiator]**.
> Unlike **[the alternative they use today]**, Landscape **[the thing only it does]**.

Two constraints on whatever fills those blanks:

- **The alternative is usually not another tool.** It is an afternoon of browser tabs, or a
  frontier chatbot answering confidently without sources. Position against the real substitute.
- **Do not lead with speed.** On the free tier the product is slower than the alternatives it
  competes with, and a promise the hardware cannot keep is the fastest available way to lose
  the trust the whole product is built on ([ROADMAP.md](ROADMAP.md) §1). Lead with *checkable*.

---

## 4. Channels, in expected-yield order

| # | Channel | Effort | Compounds? | Starts |
|---|---|---|---|---|
| 1 | **Programmatic comparison pages** (`/compare/:a-vs-:b`) | Build once, refresh weekly | **Yes — strongest** | Phase 2 (as static pages, before the app is public) |
| 2 | **Public KB / help corpus** (`QAPage` structured data) | 15 min/day already budgeted | **Yes** | Phase 3 |
| 3 | **Build-in-public** (weekly progress, findings, screenshots) | 1 h/week | Partially | Phase 1 |
| 4 | **Community participation** (IndieHackers, relevant subreddits, Slack/Discord) | 1 h/week | No — but it is what makes launch day land | Phase 1 |
| 5 | **Show HN / Product Hunt launch** | One-off, ~1 week prep | No | Phase 6 |
| 6 | **Shared reports as artifacts** | Free — a product feature | **Yes** | Phase 1 |
| 7 | **The trap-subject benchmark** (§6) | ~2 days | No, but high-signal | Phase 6 |

**Channels 1, 2 and 6 are the only ones that appreciate.** They should be prioritized
accordingly, and none of them requires the product to be publicly launched — comparison pages
and KB articles can be published as static content months before the app is.

---

## 5. The weekly cadence

Copied into ROADMAP §2A.1 so it cannot be quietly dropped:

| Activity | Cadence | Time |
|---|---|---|
| Build-in-public update | Weekly | 45 min |
| Genuine participation in one target community | Weekly | 45 min |
| Publish one KB article or comparison page | Weekly, from Phase 2 | 30 min |
| Waitlist check + one outreach conversation | Weekly | 30 min |
| Review acquisition metrics vs ROADMAP §3.1 | Monthly | 30 min |

**Rule:** if a week's build work and the distribution hour conflict, **the distribution hour
wins.** It is protected precisely because it is the one that always feels deferrable, and
because R9 says it is the one that matters most.

---

## 6. The trap-subject benchmark as a launch asset

The single highest-signal, lowest-cost marketing artifact available.

Run the golden set's **trap subjects** — a product that does not exist, and a name shared by
three real products — through Landscape and the well-known free alternatives. Publish the
results with screenshots, methodology, and the dates.

Why it works: it is cheap, reproducible by anyone, demonstrates the differentiator in about
ten seconds, and **the architecture is the only one in the comparison that structurally cannot
fail it** — a retrieval-gated pipeline has nothing to invent from. Competitors cannot
neutralize it without rebuilding.

Two conditions, both non-negotiable and both derived from the product's own rules
([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5): **publish the dimensions where we lose too**
(time to result, and possibly coverage), and **describe what each tool did, never what each
company is**. A benchmark that only reports favorable dimensions is exactly the marketing
behavior this product exists to be an alternative to.

---

## 7. Launch narrative  **[FOUNDER]**

To be drafted in Phase 5, rehearsed before Phase 6. Must answer, in the first two sentences:
what it does, who for, and why it is different from asking a chatbot. The pre-written answers
to the predictable launch questions (accuracy, sources, privacy, "why not just use ChatGPT",
pricing, why it is slow) are already scheduled in ROADMAP Phase 6.

---

## 8. Measurement

Tracked from Phase 1, reviewed monthly against ROADMAP §3.1:

- Waitlist size and weekly growth
- Signups by channel; **cost or effort per signup by channel**
- Comparison-page impressions → clicks → signups
- KB sessions → signups
- Share-link opens per report
- **Gate G4** (ROADMAP §2A.2): 3 months post-launch, is any channel repeatable?

If no channel is repeatable at G4, the problem is positioning, not features — and the pivot
ladder in ROADMAP §2A.3 is the response, not another sprint of building.
