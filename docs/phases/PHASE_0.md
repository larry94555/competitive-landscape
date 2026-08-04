# Phase 0 — your actions

> **Goal of the phase:** replace every assumption in the roadmap with a measurement, and
> stand up the skeleton everything else attaches to.
>
> **Engineering is complete.** Every remaining item is on this page.

**Where it stands: 3 of 8 exit criteria met.** The other five are below. Two of them close
with a single 90-minute sitting.

| Exit criterion | Closed by |
|---|---|
| ✅ Grammar-constrained JSON, 0 parse failures / 100 | Done |
| ✅ A written observability decision | Done — [ADR 0005](../decisions/0005-observability-on-a-24gb-box.md) |
| ✅ Every merge gate green from commit one | Done |
| ❌ Three-model choice, measured on the A1 | **Step 1** |
| ❌ `q8_0` KV quantization decided on evidence | **Step 1** |
| ❌ Concierge: ≥5 reports to real people | **Step 2** |
| ❌ Review-platform access decision recorded | **Step 3** |
| ❌ Realistic end-to-end latency estimate | **Mine**, after Step 1 |

---

## Step 1 — Run the model bake-off on the A1

**The highest-value thing available.** Closes two exit criteria and unblocks six technical
tasks queued behind it.

| | |
|---|---|
| **Do** | Run the commands in the bake-off sheet on the Oracle A1, and paste the output back to me |
| **Where** | [`docs/A1_BAKEOFF.md`](../A1_BAKEOFF.md) — open it on the box, or read it here and paste commands over |
| **Time** | ~90 minutes, most of it models downloading and `llama.cpp` compiling. One sitting, not spread out |
| **Done when** | You have pasted me: the §0 machine check, whether `dotprod`/`i8mm` compiled in, and for each of two models — the bench table, the golden-set scorecard, and resident memory. Plus the same scorecard again with `q8_0` KV |
| **I confirm by** | **Automatically.** I turn your output into `BENCHMARKS.md` **Run 4**, write the three-model choice into `ROADMAP.md` with the numbers behind it, and tick exit criteria 1 and 5. If the numbers contradict the tiering thesis, I say so rather than fitting them to it |

**Start with §0 of that sheet before anything else.** It runs `uname -m`, `nproc`, `free` and
`swapon`. That answers three things I have been hedging about for several messages — whether
the shape is really `A1.Flex` at 4 OCPU / 24 GB aarch64, and whether swap is off. If any of
those comes back wrong, **stop and tell me**: an x86 instance or a 1 GB box changes the model
choice completely and the remaining 85 minutes would be wasted.

> **Deployment is not part of this.** The sheet installs no services, opens no ports, writes
> no systemd units and touches no database. It compiles `llama.cpp`, runs two harnesses, and
> prints numbers.

**If a step fails, paste the failure.** That sheet was written from the specification and
verified on a laptop — it has **never been run on ARM**, and it says so in its own closing
section. A step that does not work there is a bug in my document.

---

## Step 2 — The concierge five (the G1 gate)

**The highest-risk open item in the roadmap**, and it needs no infrastructure, no code and no
A1.

| | |
|---|---|
| **Do** | Produce **5 competitive reports by hand**, deliver them to real founders, and **charge a token amount** |
| **Where** | Follow [`COMPETITIVE_ANALYSIS_REPORT.md`](../COMPETITIVE_ANALYSIS_REPORT.md) and [`FACT_CHECKING.md`](../FACT_CHECKING.md) — they are already written as an analyst's SOP |
| **Time** | Days to weeks, and the calendar time is the point. This is not a task to batch |
| **Done when** | Five reports delivered, and for each one you have recorded: what they read first, what they ignored, what they asked for that was not there, and **whether they paid** |
| **I confirm by** | **You tell me, and I record it.** I write the five into `ROADMAP.md` under the G1 gate with your notes, and turn each report's subject into a golden-set reference sheet — so the work counts twice |

**Why the token charge matters more than it looks.** Willingness to pay $10 for a hand-made
report is stronger evidence than a hundred waitlist signups. Free reports measure politeness.

**Why now rather than after Phase 1.** The plan otherwise builds for ~20 weeks before meeting
a user. If the format, the pricing or the positioning is wrong, this is the cheapest week to
find out and week 20 is the most expensive.

**What to send me afterwards:** the five subjects, and for each one two or three sentences on
the reaction. I do not need the reports themselves unless you want them in the repository.

---

## Step 3 — The source-terms audit (Track D)

Half a day, and **two of these can invalidate a planned feature**. Cheap now, expensive in
Phase 3.

| | |
|---|---|
| **Do** | Read the terms of each source below **in a browser, from the primary source**, and tell me what you find |
| **Where** | The six links in the table below |
| **Time** | Half a day. Reddit is the one to do first |
| **Done when** | You have an answer for each of the six |
| **I confirm by** | **Automatically, once you tell me.** I write each into `docs/decisions/` as an ADR, and update `DISCUSSION_SIGNALS.md` and `COMPETITIVE_DISCOVERY.md` where an answer changes the design. The review-platform answer closes exit criterion 7 |

| Source | The question | Why it matters |
|---|---|---|
| **[Reddit Data API Terms](https://www.redditinc.com/policies/data-api-terms)** | Does commercial use require prior approval and a monthly minimum? | **Do this one first.** Our entire "Reddit is unusable" assumption rests on *vendor blogs who sell Reddit data access* — an interested party under our own trust model. Reddit's own pages refuse automated fetching, so a human has to look |
| **[Review platforms' robots.txt](https://www.g2.com/robots.txt)** and terms | Are we permitted to read category pages? | **Closes exit criterion 7.** `COMPETITIVE_DISCOVERY.md` ranks these as the *highest-yield* discovery channel, and we honour robots.txt as a hard commitment. Those two positions may collide, and it is better to know by decision than by discovery in week 8 |
| **[X / Twitter API pricing](https://developer.x.com/en/products/x-api)** | Confirm the per-post read pricing | We believe it is disqualifying rather than merely expensive — 200 posts ≈ $1, which is the whole monthly subscription for one report section |
| **[YouTube Data API quota](https://developers.google.com/youtube/v3/getting-started#quota)** | Confirm the **data-storage** limits in the terms | The 100 `search.list`/day figure is confirmed. The storage rules are stricter than our normal caching and we have not read them |
| **[Stack Exchange terms](https://stackoverflow.com/legal/terms-of-service/public)** | What does CC BY-SA oblige the surrounding report to do? | Quoting may impose obligations on the whole document, not just the quote |
| **[GitHub search rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)** | The **search** endpoint limit specifically | The 5,000/hour figure is the general REST limit. Search is documented separately and is lower |

---

## Step 4 — Register the domain and put up a waitlist page

| | |
|---|---|
| **Do** | Register the domain, point it somewhere, and put up a one-page holding site with an email capture |
| **Where** | Any registrar. The page can be a single static file |
| **Time** | An hour, plus whatever the name decision takes |
| **Done when** | The domain resolves and the page loads |
| **I confirm by** | **Automatically.** Tell me the domain and I fetch it — `landscape fetch https://yourdomain/` runs it through the real fetcher, and I check it resolves, returns 200, and has a working capture form. Then I record it in `ROADMAP.md` |

**Why it is on the list now rather than at launch:** the domain starts ageing from the day it
is registered, and the list starts growing. Both are cheap to start and impossible to
backdate.

---

## Step 5 — Name and trademark check

| | |
|---|---|
| **Do** | Search the trademark registers for the name before any brand equity accrues |
| **Where** | [USPTO TESS](https://www.uspto.gov/trademarks/search) and [EUIPO eSearch](https://euipo.europa.eu/eSearch/) |
| **Time** | An hour |
| **Done when** | You know whether the name is clear in the classes that matter, and have written down what you found |
| **I confirm by** | **You tell me, and I record it** in `ROADMAP.md`. I cannot give legal advice and will not pretend the search result is one |

**Do this before Step 4** if the two are close together — registering a domain for a name you
then have to change is the wrong order.

---

## Step 6 — Confirm the A1 is Pay-As-You-Go

| | |
|---|---|
| **Do** | Check whether the Oracle account is a trial or has been converted to Pay-As-You-Go |
| **Where** | [Oracle Cloud console](https://cloud.oracle.com/) → Billing & Cost Management → Upgrade and Payment |
| **Time** | Two minutes |
| **Done when** | You know which it is |
| **I confirm by** | **You tell me, and I record it** in `ROADMAP.md`. There is no way for me to see this |

**Why it matters.** On a trial account, Always Free resources are **reclaimed when the trial
ends**. Roadmap risk R11 depends on holding that instance, and A1 capacity is scarce enough
that losing it is not a quick thing to undo. Staying inside the free limits on a PAYG account
still bills €0.

*(§0 of the bake-off sheet answers the **shape** question. This one is separate and the
console is the only place to see it.)*

---

## What I do once these land

Nothing on this list is blocked on me, but three things are waiting behind it:

| After | I will |
|---|---|
| Step 1 | Write `BENCHMARKS.md` Run 4; decide the three-model split with numbers; **write the end-to-end latency estimate and the honest Rung-0 promise** — that is the eighth exit criterion, and it is mine, not yours |
| Step 2 | Fold each concierge subject into the golden set as a reference sheet |
| Step 3 | Write one ADR per source; promote fallback discovery channels to the primary path if the review platforms say no |

**Phase 0 closes when all six steps above are done and I have written the latency decision.**
At that point I will update `ROADMAP.md`, tick all eight exit criteria, and say so plainly —
including if any measurement contradicts something the roadmap currently assumes.
