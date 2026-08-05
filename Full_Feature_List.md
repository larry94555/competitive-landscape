# Full Feature List

**As of 2026-08-05** · `main` at `c219ad1` · **36 of 136 PRs · 26% of the software deliverable**

Every feature the roadmap describes, sorted by the readiness state that first requires it, with
what it is estimated to cost in pull requests and how much of that is spent.

> **Why pull requests and not weeks.** A PR is a unit of *reviewable change*, and the count is
> something you can move deliberately: the same work in bigger PRs is a smaller number. Time
> estimates would pretend to a precision nothing here has earned.

**Read this beside [PROJECT_STATUS.md](PROJECT_STATUS.md)**, which says what is true today and
why. This page says how much is left.

---

## How to read the numbers

**Done** counts a PR that is written, green and either merged or in review. There are 35 merged
and one open ([#36](https://github.com/larry94555/competitive-landscape/pull/36)) — **36, and the
Done column sums to exactly that.**

**The rows are themes, not individual PRs.** Which of the 36 belongs to which row is approximate,
because several did two things at once; the totals are not.

**The estimates are a judgement, and their reliability falls off sharply with distance.** S1 is
estimated against 35 PRs of actual history in this repository — that is the only part calibrated
against evidence. S2 is estimated against a design that exists on paper. S5 and S6 are estimated
against features nobody has designed in detail, and I would not be surprised by ±50% on them.

**A feature is listed under the earliest state that requires it.** Reading a company's pricing
page appears once, under S1, even though every later state depends on it.

**These are software PRs only.** Deployment, the concierge interviews and the source-terms audit
are not on this page; they are in [what is not software](#what-is-not-software-and-cannot-be-a-pr)
at the bottom, and **S1 cannot be reached without the first of them.**

---

## Summary

| State | What it means | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| [**S1**](#s1--ready-for-a-guided-demo) | Ready for a guided demo | 44 | 35 | **9** | **80%** |
| [**S2**](#s2--ready-for-demonstration) | Any business idea handled correctly | 19 | 0 | **19** | **0%** |
| [**S3**](#s3--ready-for-use) | Friendly users find no issue | 26 | 1 | **25** | **4%** |
| [**S4**](#s4--ready-for-general-use) | Promotable, word-of-mouth quality | 11 | 0 | **11** | **0%** |
| [**S5**](#s5--general-use-free-mode) | Stable, email signup, community | 15 | 0 | **15** | **0%** |
| [**S6**](#s6--general-use-full-mode) | Notifications and paid subscriptions | 21 | 0 | **21** | **0%** |
| | **Total** | **136** | **36** | **100** | **26%** |

**The shape of that table is the answer to "how far are we".** S1 is nearly done and the nine
remaining PRs are small and known. Everything after it is a cliff, and the cliff is one item:
nothing turns a description into a set of companies.

---

## S1 — Ready for a guided demo

*Only certain product ideas work reliably.* Nine PRs left, all of them scoped.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| Foundations — workspace, quality gates from commit one, CI, aarch64 cross-compile | S1 | 4 | 4 | 0 | 100% |
| Model selection — bake-off, benchmark harness, licence review, `q8_0` validation | S1 | 3 | 3 | 0 | 100% |
| Constrained decoding — Rust type → JSON Schema → GBNF → parsed back | S1 | 2 | 2 | 0 | 100% |
| Polite fetching — SSRF guard, robots.txt, per-host rate limiting | S1 | 1 | 1 | 0 | 100% |
| Source discovery on one domain — probes, sitemap, `llms.txt`, locale, top eight | S1 | 2 | 2 | 0 | 100% |
| Extraction — Markdown conversion, span pre-selection, four of six question kinds | S1 | 6 | 6 | 0 | 100% |
| Report assembly — six sections, citations, coverage notes for four kinds of silence | S1 | 2 | 2 | 0 | 100% |
| Entity resolution gate — refuse to guess which company was meant | S1 | 1 | 1 | 0 | 100% |
| Queue, worker, SSE streaming, and the states an interrupted run reaches | S1 | 5 | 5 | 0 | 100% |
| Regression instruments — golden set, ten frozen pages, the mistakes register | S1 | 3 | 3 | 0 | 100% |
| Observability — request ids through span, header and body; runbook | S1 | 1 | 1 | 0 | 100% |
| Planning documents — roadmap, phase checklists, status page | S1 | 3 | 3 | 0 | 100% |
| **D1** The binary serves the built web app | S1 | 1 | 1 | 0 | 100% |
| **D2** A run has a URL — `/a/{id}`, survives a reload, can be sent to somebody | S1 | 1 | 1 | 0 | 100% |
| **D3** A deployable artefact — aarch64 build, service unit, RUNBOOK deploy section | S1 | 2 | 0 | **2** | 0% |
| **D4** Example ideas that really run — chips mapping to curated competitor sets | S1 | 2 | 0 | **2** | 0% |
| **D5** More than one company in a report — grouped by company, not one profile | S1 | 2 | 0 | **2** | 0% |
| **D6** A cap on anonymous runs — per-IP daily count, before the URL is public | S1 | 1 | 0 | **1** | 0% |
| Measure the wait from a client, and order the reads so content arrives first | S1 | 2 | 0 | **2** | 0% |
| | | **44** | **35** | **9** | **80%** |

**The risk inside those nine.** A single company takes about two minutes on a laptop and the
target host is four ARM cores. D5 multiplies that by the number of competitors, so a demo could
take ten minutes — which is not a demo. The last row exists to find that out before it is
promised, and the answer if it is too slow is fewer pages and a better read order, not a faster
model.

---

## S2 — Ready for demonstration

*Any business idea handled correctly, limited functionality, friendly users only.* **This is the
cliff.** Nothing here is started, and the first row is the largest single piece of unbuilt
software in the project.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| **The search channel** — `landscape-search`, SearXNG or equivalent, off-site sources | S2 | 4 | 0 | **4** | 0% |
| Candidate generation — turn a search result set into scored candidates for the gate | S2 | 2 | 0 | **2** | 0% |
| Competitor set derivation — one idea to several companies, with why each was chosen | S2 | 3 | 0 | **3** | 0% |
| Vocabulary resolution — a reader's words to a category the pipeline can search | S2 | 2 | 0 | **2** | 0% |
| Clarifying questions — ≤3, chip-answerable, skippable, only when discovery diverges | S2 | 2 | 0 | **2** | 0% |
| Trust posture extractor — the fifth question kind | S2 | 1 | 0 | **1** | 0% |
| Investment direction extractor — the sixth question kind | S2 | 1 | 0 | **1** | 0% |
| Honest "no public information" at the level of a whole competitor set | S2 | 1 | 0 | **1** | 0% |
| Fetch cache and per-source extraction cache — two readers of one competitor share work | S2 | 2 | 0 | **2** | 0% |
| Conditional GET and a per-analysis fetch cap | S2 | 1 | 0 | **1** | 0% |
| | | **19** | **0** | **19** | **0%** |

**D4 is the reason S1 does not wait for this.** A curated set of example ideas runs a *real*
analysis over *real* competitor domains; only the choice of companies is curated, and the
interface says so. That is what "only certain product ideas work reliably" means, and it is the
difference between a demo now and a demo after four PRs of search infrastructure.

---

## S3 — Ready for use

*Friendly users should find no issue.* One PR of this exists — the embedded-state extraction tier
and the measurement that decided against a headless browser.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| `landscape-verify` — quote matching against the source, validators, one retry | S3 | 3 | 0 | **3** | 0% |
| Feature comparison matrix and pricing comparison — the backbone of a real report | S3 | 3 | 0 | **3** | 0% |
| `landscape-charts` — SVG feature matrix and cost-at-scale curves | S3 | 2 | 0 | **2** | 0% |
| The three remaining report sections — positioning, sentiment themes, SWOT | S3 | 3 | 0 | **3** | 0% |
| PDF export — Typst executive and full templates, pre-warmed | S3 | 2 | 0 | **2** | 0% |
| Evidence strength badges, per-claim confidence, "what we checked" blocks | S3 | 1 | 0 | **1** | 0% |
| Feedback capture — per-section 👍/👎 and "report an inaccuracy" | S3 | 1 | 0 | **1** | 0% |
| Golden set to 50 subjects, with matrix and chart-data assertions | S3 | 2 | 0 | **2** | 0% |
| Automated eval suite with CI gates — citation coverage ≥97%, drop rate ≤3% | S3 | 2 | 0 | **2** | 0% |
| Rendering ladder tiers 2–4 — embedded state, then the rest | S3 | 2 | 1 | **1** | 50% |
| Latency work — section-parallel generation, prompt-prefix stabilisation | S3 | 2 | 0 | **2** | 0% |
| "What people are saying" — Hacker News and GitHub only, with named exclusions | S3 | 2 | 0 | **2** | 0% |
| **Copy as context** — the whole report as clean Markdown, sized to paste elsewhere | S3 | 1 | 0 | **1** | 0% |
| | | **26** | **1** | **25** | **4%** |

**Copy as context is the cheapest row on this page and worth pulling forward.** It is the feature
that settles what the product *is*: not a worse chatbot, but the evidence file a chatbot cannot
assemble.

---

## S4 — Ready for general use

*Promotable, word-of-mouth quality.* Mostly about proving quality against a system that is
actually running, which is why none of it can start before S1 is deployed.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| Quality gates run against a deployed system, from a client's side | S4 | 1 | 0 | **1** | 0% |
| Human review process and admin review queue | S4 | 2 | 0 | **2** | 0% |
| Dashboards — citation coverage, drop rate, latency percentiles, cache hit rates | S4 | 2 | 0 | **2** | 0% |
| Load test and abuse controls under a launch-day spike | S4 | 2 | 0 | **2** | 0% |
| Shareable public report pages — SSR, indexable, `noindex` unless shared | S4 | 2 | 0 | **2** | 0% |
| Landing-page proof — a live example report above the fold, not a screenshot | S4 | 2 | 0 | **2** | 0% |
| | | **11** | **0** | **11** | **0%** |

---

## S5 — General use, free mode

*Stable, email signup, community channels.* No authentication code exists anywhere in the
repository today.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| Magic-link auth — single-use, 15-minute TTL, ~90-day signed sessions | S5 | 2 | 0 | **2** | 0% |
| Free tier — 10 analyses/month, history, saved reports, usage meter | S5 | 2 | 0 | **2** | 0% |
| Transactional email with SPF/DKIM/DMARC — deliverability gates signup | S5 | 1 | 0 | **1** | 0% |
| GDPR basics — export and delete my data | S5 | 1 | 0 | **1** | 0% |
| `landscape-kb` — public `/help`, threads, tags, votes, flags, Postgres FTS | S5 | 4 | 0 | **4** | 0% |
| 25–30 seeded official articles | S5 | 1 | 0 | **1** | 0% |
| `/legal/*` and the public `/bot` page | S5 | 1 | 0 | **1** | 0% |
| Admin console v1 — usage, quality queue, support queue | S5 | 2 | 0 | **2** | 0% |
| Funnel and knowledge-base metrics | S5 | 1 | 0 | **1** | 0% |
| | | **15** | **0** | **15** | **0%** |

---

## S6 — General use, full mode

*Notifications and paid subscriptions.* Blocked on a commercial decision as well as on code: the
merchant-of-record choice has to be recorded before any billing is written.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| `landscape-billing` — Stripe checkout, portal, webhooks, reconciliation | S6 | 4 | 0 | **4** | 0% |
| Pricing page, upgrade dialogs at the moment of the limit, annual billing | S6 | 2 | 0 | **2** | 0% |
| Entitlements and quotas — plans, usage counters, priority queue | S6 | 2 | 0 | **2** | 0% |
| Dunning — failed payment to grace period to downgrade, never deleting data | S6 | 1 | 0 | **1** | 0% |
| `landscape-watch` — scheduler, conditional GET, content hashing, suppression | S6 | 3 | 0 | **3** | 0% |
| Importance scoring and digest batching | S6 | 2 | 0 | **2** | 0% |
| Alert email — one-click feedback, side-by-side diff, re-run | S6 | 2 | 0 | **2** | 0% |
| Noise regression suite — 20 recorded page pairs, 10 material and 10 cosmetic | S6 | 1 | 0 | **1** | 0% |
| Bring-your-own-key — OpenAI-compatible and Anthropic adapters, fallback, provenance | S6 | 2 | 0 | **2** | 0% |
| Slack and webhook delivery, and the public API | S6 | 2 | 0 | **2** | 0% |
| | | **21** | **0** | **21** | **0%** |

---

## What is not software, and cannot be a PR

**S1 is not reachable on the nine PRs above alone.** These are yours, and no amount of code
substitutes for them:

| | What | Blocks |
|---|---|---|
| **Deploy to the Oracle A1** | No host address, SSH user or key is reachable from this repository | **S1 and everything after it** |
| **The concierge five** | ≥5 hand-made reports delivered to real founders, reactions recorded | Confidence in everything built since Phase 0 |
| **Source-terms audit** | Reddit, X, YouTube, Stack Exchange, review-platform robots.txt | S3's "what people are saying", the discovery ranking |
| **Merchant-of-record decision** | Stripe leaves tax registration with you; Paddle absorbs it at a higher fee | All of S6 |

The first one is the only thing standing between the current nine PRs and a URL that exists.

---

## What would change these numbers most

**One thing, and it is not on the list above.** The estimates for S2 through S6 are built from a
roadmap written before anything was running. The single largest source of error is that
**nobody outside this repository has used the product** — the concierge five exist precisely to
find out which of these features somebody actually wants, and until they happen, every row below
S1 is an estimate of the cost of building something whose value is unmeasured.

That is not an argument for stopping. It is an argument for reaching S1 quickly, showing it to
five people, and re-cutting this table with what they say.
