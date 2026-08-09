# Full Feature List

**As of 2026-08-09** · `main` at `b8be90a` · **54 of 130 PRs · 42% of the software deliverable**

Every feature the roadmap describes, sorted by the readiness state that first requires it, with
what it is estimated to cost in pull requests and how much of that is spent.

> ## Every state is reached on a laptop
>
> **S1 to S6 describe what the software can do, and every one of them is finished and verified
> locally.** A state is not "deployed"; it is *demonstrable*, end to end, on a development
> machine with nothing installed but Rust, Node and a `llama-server`.
>
> **Deploying changes nothing but secrets.** The invariant this project holds itself to is that
> a build which works locally works on the box with only configuration differing — `BIND_ADDR`,
> `DATABASE_URL`, `LLAMA_URL`, `WEB_DIR`. If anything ever needs a code change to run there,
> that is a defect in the code, not a step in the plan.
>
> Deployment is therefore **not** in any state's percentage. It is a separate track, listed in
> [Getting it onto a host](#getting-it-onto-a-host), and it gates *who can see* the software
> rather than *what the software can do*.

> **Why pull requests and not weeks.** A PR is a unit of *reviewable change*, and the count is
> something you can move deliberately: the same work in bigger PRs is a smaller number. Time
> estimates would pretend to a precision nothing here has earned.

**Read this beside [PROJECT_STATUS.md](../PROJECT_STATUS.md)**, which says what is true today and
why. This page says how much is left.

---

## How to read the numbers

**Done** counts a PR that is written, green and either merged or in review, **and the Done column
is the source of truth for every percentage on this page** — the totals below are that column
summed, not a count taken from GitHub.

**Those two numbers are not the same, and this line is where the difference is stated.** The
repository has **47 merged pull requests and one open** — this one — while the Done column sums
to **46**. The rows are themes: a pull request that only answered review findings, or hardened a
gate, adds no feature and therefore no row. Counting it would inflate a table that is supposed to
measure how much of the product exists.

**The rows are themes, not individual PRs.** Which one belongs to which row is approximate,
because several did two things at once; the totals are not.

**An estimate that turns out wrong is corrected, and the correction is visible.** D5 was
estimated at two PRs and took one; D4 was estimated at two and took one. Both rows say one — the
point of the number is to be useful next time, not to be defended. Two corrections in the same
direction is itself a reading: the D items were sized before the pipeline underneath them was
this well tested, and a feature built on parts that already work is smaller than one that has to
prove them.

**The estimates are a judgement, and their reliability falls off sharply with distance.** S1 is
estimated against 35 PRs of actual history in this repository — that is the only part calibrated
against evidence. S2 is estimated against a design that exists on paper. S5 and S6 are estimated
against features nobody has designed in detail, and I would not be surprised by ±50% on them.

**A feature is listed under the earliest state that requires it.** Reading a company's pricing
page appears once, under S1, even though every later state depends on it.

**These are software PRs only.** The concierge interviews and the source-terms audit are work
only you can do; they are listed at the bottom and are not counted anywhere.

---

## Summary

| State | What it means | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| [**S1**](#s1--ready-for-a-guided-demo) | Ready for a guided demo | 39 | 39 | **0** | **100%** |
| [**S2**](#s2--ready-for-demonstration) | Any business idea handled correctly | 18 | 14 | **4** | **78%** |
| [**S3**](#s3--ready-for-use) | Friendly users find no issue | 26 | 1 | **25** | **4%** |
| [**S4**](#s4--ready-for-general-use) | Promotable, word-of-mouth quality | 11 | 0 | **11** | **0%** |
| [**S5**](#s5--general-use-free-mode) | Stable, email signup, community | 15 | 0 | **15** | **0%** |
| [**S6**](#s6--general-use-full-mode) | Notifications and paid subscriptions | 21 | 0 | **21** | **0%** |
| | **Total** | **130** | **54** | **76** | **42%** |
| | *Getting it onto a host (not a state)* | *3* | *0* | *3* | *0%* |

**The shape of that table is the answer to "how far are we".** **S1 is complete** — the guided
demo runs on a laptop, end to end, over real companies. Everything after it is a cliff, and the
cliff is one item: nothing turns a description into a set of companies, which is exactly the gap
D4's curated ideas step over rather than close.

---

## S1 — Ready for a guided demo

*Only certain product ideas work reliably.* **Done.** Pick an example idea, watch sections
arrive, read a cited report about real companies, reload and it is still there — on a laptop,
with `cargo run -p landscape -- dev`, a `llama-server` beside it, and a browser.

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
| Planning documents — roadmap, phase checklists, status page, feature list | S1 | 4 | 4 | 0 | 100% |
| **D1** The binary serves the built web app | S1 | 1 | 1 | 0 | 100% |
| **D2** A run has a URL — `/a/{id}`, survives a reload, can be sent to somebody | S1 | 1 | 1 | 0 | 100% |
| **D4** Example ideas that really run — three ideas over six companies checked against the live web | S1 | 1 | 1 | 0 | 100% |
| **D5** More than one company in a report — every site named is a subject, one section holds them all | S1 | 1 | 1 | 0 | 100% |
| Order the reads so content arrives first, and measure the wait end to end | S1 | 1 | 1 | 0 | 100% |
| | | **39** | **39** | **0** | **100%** |

**What the last row actually bought.** The page that needs no model is read first, so first
content on `linear.app` is **23 seconds and does not involve the model at all** — inside §2.1A's
window, and after review, not even dependent on the model being *reachable*. Each question is
worth one page that needs a model, which takes the six demo companies from 128 model calls to 88,
and the pages left unread are named on the report.

**The seconds are still not settled, and that is not a code change.** One call's cost belongs to
the model and the machine. `landscape cost` counts the calls with no model running; the
end-to-end figure is taken from a client's side of a deployment
([ADR 0011](decisions/0011-no-experiments-on-production.md)), which is measurement rather than
software — and it is why this row is done while the number is still open.

**And this is the one number a laptop cannot settle by itself.** The target host is four ARM
cores against a developer machine's many, so the local figure is a lower bound rather than an
estimate. Everything else on this page is finished when it works here; the wait is finished when
it has been measured from a client's side of the deployment
([ADR 0011](decisions/0011-no-experiments-on-production.md)). That is a measurement, not a
code change — the software does not become different.

---

## S2 — Ready for demonstration

*Any business idea handled correctly, limited functionality, friendly users only.* **Past four
fifths.** Fifteen of eighteen PRs are done, and seven of the ten rows are complete — the caches
row closed with the extraction half, so **two readers of one competitor now share both the
fetching and the reading**. Both inputs produce a comparison — a description and a
named company alike — a report that covers **one** company says which of four things happened to
the others, and a run that produces **no** report now says which of five things happened rather
than one. That last one is the same discipline reaching the surface a reader actually looks at:
the analysis had spent four changes keeping those silences apart while the interface collapsed
them back into a single sentence.

| Feature | State | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|---|
| **The search channel** — `landscape-search`, SearXNG or equivalent, off-site sources | S2 | 4 | 2 | **2** | 50% |
| Candidate generation — turn a search result set into scored candidates for the gate | S2 | 2 | 2 | 0 | 100% |
| Competitor set derivation — one idea to several companies, with why each was chosen | S2 | 3 | 2 | **1** | 67% |
| Vocabulary resolution — a reader's words to a category the pipeline can search | S2 | 2 | 2 | 0 | 100% |
| Clarifying questions — ≤3, chip-answerable, skippable, only when discovery diverges | S2 | 2 | 2 | 0 | 100% |
| Trust posture extractor — the fifth question kind | S2 | 1 | 1 | 0 | 100% |
| Investment direction extractor — the sixth question kind | S2 | 1 | 1 | 0 | 100% |
| Honest "no public information" at the level of a whole competitor set | S2 | 1 | 1 | 0 | 100% |
| Fetch cache and per-source extraction cache — two readers of one competitor share work | S2 | 2 | 2 | 0 | 100% |
| Conditional GET and a per-analysis fetch cap | S2 | 1 | 0 | **1** | 0% |
| | | **18** | **15** | **3** | **83%** |

**What the two clarifying-question PRs bought, and what §3 still describes.** The row is
counted complete because what S2 needs is a run that cannot converge saying so in terms a reader
can act on, and being answerable without retyping anything — both are built. The three other
triggers in [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §3 are **not** in this count and are not lost:
*"who should I compare against?"* is the vocabulary-resolution row two lines above, and the buyer
and intent questions need the grammar-constrained router model, which is Phase 2. The one place
this row knowingly departs from the specification is the skip — §3 promises every question is
skippable, and an ambiguous brand name has no honest default to skip to. See
[BENCHMARKS.md](BENCHMARKS.md) Run 34.

**Neither row that has moved is on the critical path, and that is deliberate.** The trust posture
and investment direction extractors need no search channel: discovery already admits `/security`
and `/careers`, the pipeline already reached them, and both sections were permanently empty
because nothing read them. They are the cheapest honest improvements available while
[B2](../PROJECT_STATUS.md#4-blockers) is unbuilt.

**Both caches are built** — [BENCHMARKS.md](BENCHMARKS.md) Runs 37 and 38. The second reader of
a company pays neither a stranger's server nor ours: the fetch half took eight fetches to zero,
and the extraction half takes sixteen model calls to zero for the same company. What is left in
this row is conditional GET, which is about the *first* reader coming back tomorrow rather than
the second arriving today.

**With the sixth extractor, that seam is closed.** All six questions a report has a section for
now have an extractor behind them, so there is no longer a *"one fewer section that can never
fill"* improvement to reach for — **every remaining row of S2 sits behind the search channel or
the caches**. The next piece of work here was the cliff itself, and it started.

**What the first search PR is, and what it is not.** `landscape-search` exists: a versioned,
templated query set built from the questions discovery came back empty on, a `SourceProvider`
seam with a SearXNG adapter behind it, and an admission step that decides what a result may be
used for — the company's own domain is Primary, everything else is Unverified and therefore may
never set a value in a comparison table. It is exercised by `landscape search <origin>` and by 41
tests, seven of them over a real socket.

**It is wired into the orchestrator now, and typing an idea still fails.** Those were two
separate things, and the first of them is done: an analysis asks a search engine for the
questions its own pages left empty, reads what comes back, and marks a page it could not
attribute where the claim is read rather than only in the source list. Turning a *description*
into a company is entity resolution, and that is still ahead. A search channel that no analysis calls
buys a reader nothing yet; what it buys is that the other three PRs are now joins rather than
inventions. The claim being made here is deliberately narrow — see
[BENCHMARKS.md](BENCHMARKS.md) Run 26, including the finding that a gap is currently measured
from what discovery *admitted* rather than from what a section *filled*, which is a sharper
number this slice cannot honestly claim.

**D4 is the reason S1 does not wait for this, and it is now built.** Three curated ideas run a
*real* analysis over *real* competitor domains; only the choice of companies is curated, the
interface says so in a sentence served with the list, and `landscape examples` re-checks all six
against the live web. That is what "only certain product ideas work reliably" means, and it is
the difference between a demo now and a demo after four PRs of search infrastructure.

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
| Quality gates run against a running system, over the network, as a client sees it | S4 | 1 | 0 | **1** | 0% |
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
| `landscape-billing` — Stripe checkout, portal, webhooks, reconciliation (test mode and the Stripe CLI run locally) | S6 | 4 | 0 | **4** | 0% |
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

## Getting it onto a host

**Not a state, and not in any percentage above.** These three PRs decide *who can see* the
software rather than **what it can do**, and the states above are complete without them.

| Feature | Est. PRs | Done | Left | Complete |
|---|---|---|---|---|
| **D3** A deployable artefact — aarch64 build, `web/dist` beside it, service units, [DEPLOY.md](DEPLOY.md) and a RUNBOOK section | 1 | 1 | 0 | 100% |
| **D6** A cap on anonymous runs — two a day per address, counted where a run starts | 1 | 1 | 0 | 100% |
| | **2** | **2** | **0** | **100%** |

**D6 was here rather than in S1 deliberately, and it is done.** Unlimited anonymous inference
is only a problem once strangers can reach the box; on a laptop it is not a missing capability,
it is a setting nobody needs — which is why a request with no proxy in front of it is not
counted at all.

**This track is now complete, and none of it moved a readiness state.** Both items decide who
can see the software rather than what it does. What has *not* happened is a deployment: the
procedure and the cap are written and reviewed, and no box has run either.

**D3 is done in the sense this table counts and not in the sense that matters most.** The
artefact, the units and the procedure exist and are reviewable; **no box has run them.** The
first deployment is where this stops being a plan, and the procedure says so at the top rather
than pretending otherwise.

### The invariant this track depends on

**A build that works locally works on the host with only configuration differing.** Everything
environmental is already an environment variable — `BIND_ADDR`, `DATABASE_URL`, `LLAMA_URL`,
`WEB_DIR` — and nothing in the code branches on where it is running. If deploying ever requires
a code change, that is a defect to fix rather than a task to schedule, and it belongs in
[the mistakes register](../.claude/skills/coding-mistakes/SKILL.md).

---

## What only you can do

Neither of these is a pull request, and neither blocks a state:

| What | Why it matters |
|---|---|
| **The concierge five** | ≥5 hand-made reports delivered to real founders, reactions recorded. The highest-risk open item in the project: everything since Phase 0 rests on an unvalidated assumption about the report, the buyer and the price |
| **Source-terms audit** | Reddit, X, YouTube, Stack Exchange, review-platform `robots.txt`. Two of these can invalidate a planned feature, and the discovery ranking is written against an assumption until they are read |
| **Merchant-of-record decision** | Stripe leaves tax registration with you; Paddle absorbs it at a higher fee. Must be recorded before any billing code is written |

---

## How to check any of this yourself

Every state above is a claim that something works on a laptop, so every one of them is
falsifiable in about a minute:

```bash
cd web && npm run build && cd .. && cargo run -p landscape -- dev --store memory
```

With a `llama-server` on 8080, <http://127.0.0.1:8787> is the whole product as it stands.
`docs/Feature_Walkthrough.md` walks the parts that work today and is explicit about the parts
that do not; every command in it is executed in CI, so it cannot quietly go stale.

**If a row here says done and you cannot see it there, the row is wrong.** That is the point of
defining the states this way.

---

## What would change these numbers most

**One thing, and it is not on the list above.** The estimates for S2 through S6 are built from a
roadmap written before anything was running. The single largest source of error is that
**nobody outside this repository has used the product** — the concierge five exist precisely to
find out which of these features somebody actually wants, and until they happen, every row below
S1 is an estimate of the cost of building something whose value is unmeasured.

That is not an argument for stopping. It is an argument for reaching S1 quickly, showing it to
five people, and re-cutting this table with what they say.
