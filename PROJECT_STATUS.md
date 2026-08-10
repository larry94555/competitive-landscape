# Project Status

**As of 2026-08-09** · `main` at `b8be90a`, plus the branch this page is on.

This page answers one question: **what can somebody actually do with this today, and what
stands between here and each of the six states that matter.** It is deliberately separate from
[docs/ROADMAP.md](docs/ROADMAP.md), which is the plan. A plan says what is intended; this says
what is true.

> **How to keep it true:** update this file in the same PR that changes what it describes. A
> status page maintained on its own schedule becomes fiction within a fortnight, and a
> confident fiction is worse than no page. The [update rule](#8-how-this-page-is-maintained)
> at the bottom says exactly what to touch.

---

## 1. The headline: can we show this to anyone?

The six states, in the order they must be reached. **S1 is met.** The rest are not.

| # | State | Met? | Percentage Left | The single thing standing in the way |
|---|---|---|---|---|
| **S1** | **Ready for a guided demo** — only certain product ideas work reliably | **Yes.** Pick an example idea, watch sections arrive, read a cited report about real companies, reload and it is still there. On a laptop. *This row once said the opposite twice* — see [§1.5](#15-the-correction-that-produced-phase-d). | [**0%**](docs/Full_Feature_List.md#s1--ready-for-a-guided-demo) | Nothing. Serve the app, a run has a URL, several companies in one report, example ideas that really run, and reads ordered so first content costs no model call — 23s on `linear.app`, measured. |
| **S2** | **Ready for demonstration** — any business idea handled correctly, limited functionality, friendly users only | **No.** | [**11%**](docs/Full_Feature_List.md#s2--ready-for-demonstration) | **Both inputs produce a comparison; a report that cannot says why, and so does a run that produces none.** *This row said "a business idea does not run at all" for six phases.* A description is searched for, grouped into companies, scored on cross-query agreement, named from each company's own front page, and the **set** comes out rather than the pick; a **named** company brings its rivals the same way. When a report covers **one** company it now says which of four things happened to the others — no engine, its own page gave nothing to judge one against, the searching did not finish, or nobody held up — because a reader acts on each of those differently — and a run that produces **no** report now says which of five situations it is in rather than answering all five with *"try naming its website"* — and when that situation is *a name several products share*, **the companies it would not choose between arrive as buttons**, each carrying a whole prompt so answering costs one click rather than a retyped idea. And a reader's words are now turned into the market's **before anything is searched for, and the companies are found with them** — *"a free competitive landscape research tool"* is searched as *competitive intelligence software*, counted once per independent site, and the substitution is shown on the report rather than assumed. And a page fetched once is neither fetched nor read again — the worker held **three** fetchers per run and now holds one for the process, and beside it one memory of what each page has already been read to say. Two readers of the same competitor now share **both** halves: the fetches a stranger's server would have answered, and the sixteen model calls `landscape cost` says reading `linear.app` costs. An extraction is keyed on the page's own text, so there is no expiry window and none is needed — an edited page is a different question rather than a stale answer — and a run that failed or that nobody waited for is never remembered at all. And a page past its hour is now **asked about** rather than downloaded again — `If-None-Match` goes out, a `304` comes back with no body, and a page the origin confirms is dated *now* rather than *when we first read it*. One reader's question also has a bound on what it costs strangers at last: **64 requests per analysis**, counting `robots.txt` and every redirect, shared across all three passes — and running out is reported as our doing rather than as the site refusing. And a careers page that links away is no longer a company with nothing to say about hiring: three of four real careers pages keep their vacancies on an applicant-tracking board, and the board a company's own page **links to** — never one guessed from its name — is now read, at `Attributed` rather than `Primary`, because the bytes are a stranger's even when the authorship is not. **89% done.** What is left is the engine-backed half of the search channel and the last of competitor-set derivation, and nothing else. *(This page said 89% before this change too: the feature table's totals row had drifted from its own columns and the honest figure was 84%. A fourteenth gate adds it up now.)* See [F1](#f1--searching-for-competitive-information-on-a-product-idea). |
| **S3** | **Ready for use** — friendly users should find no issue | **No.** | [**96%**](docs/Full_Feature_List.md#s3--ready-for-use) | 6 of 9 report sections, no verification layer, no comparison matrix, no accounts. |
| **S4** | **Ready for general use** — promotable, word-of-mouth quality | **No.** | [**100%**](docs/Full_Feature_List.md#s4--ready-for-general-use) | Everything in S3, plus no quality gates have ever been run against a deployed system. |
| **S5** | **General use, free mode** — stable, email signup, community channels | **No.** | [**100%**](docs/Full_Feature_List.md#s5--general-use-free-mode) | No authentication code exists anywhere in the repository. No knowledge base. |
| **S6** | **General use, full mode** — notifications and paid subscriptions | **No.** | [**100%**](docs/Full_Feature_List.md#s6--general-use-full-mode) | No billing, no watches, no email. A blocking commercial decision (merchant of record) is unmade. |

**Every state is reached on a laptop.** S1 to S6 describe what the software can *do*, and each is
finished when it is demonstrable end to end on a development machine — Rust, Node and a
`llama-server`, nothing else. **Deploying changes nothing but secrets**: `BIND_ADDR`,
`DATABASE_URL`, `LLAMA_URL`, `WEB_DIR`. If running on the box ever needs a code change, that is a
defect rather than a step in the plan.

**Percentage Left is software only**, counted in pull requests and linked to the feature it comes
from in [Full_Feature_List.md](docs/Full_Feature_List.md) — **54 of 130 PRs done, 42% of the whole
deliverable.** Getting it onto a host is a
[separate three-PR track](docs/Full_Feature_List.md#getting-it-onto-a-host) that gates *who can see*
the software rather than what it can do, and the concierge interviews and source-terms audit are
yours and are not counted.

### 1.5 The correction that produced Phase D

**This page told a comfortable lie, twice, and both are worth reading.**

**The first was a missing capability described as a missing deployment.** The S1 row said the
guided demo was blocked on deployment alone. In fact `router()` served `/api/*` and nothing else
and the React app existed only behind Vite's dev server, so there was no website to deploy —
and no test anywhere asserted that a browser pointed at the binary received HTML. Nothing was
lying on purpose: the claim was assembled from true parts and the join between them was never
checked. **A status page is where that error is most expensive**, because it is the page
somebody plans from.

**The second was the framing itself.** Saying a state was blocked on deployment made deployment
sound like a milestone the software was waiting for, when the intent is the opposite: *every
state is finished and demonstrated on a laptop, and deploying changes only secrets.* If running
on the box ever needed a code change, that would be a defect rather than a phase. Restated
throughout this page and in [Full_Feature_List.md](docs/Full_Feature_List.md), and the deployment
track is now counted separately from every state.

The binary serves the app now, with a test that a deep link returns the page and another that
the API still wins over the fallback. The remaining items are
[Phase D](docs/ROADMAP.md#2d-phase-d--a-demo-you-can-send-someone).

The other half of the correction is about sequencing rather than fact. Runs 17–20 hardened the
states a run only reaches when something goes wrong. Each was chosen as the adjacent step from
the one before, and **none was chosen by asking what a demo needs** — which is how four
consecutive pieces of correct work left the readiness table untouched.

### The three facts behind that table

**1. An analysis searches the public web for what its own reading left empty.** Discovery still
runs first and unchanged — `/pricing`, `/changelog`, `sitemap.xml`, `llms.txt`, capped at eight
pages. **What is new is the join**: after those pages are read, the questions that produced *no
claim* become templated queries, a SearXNG behind a provider seam is asked, and up to three
admitted pages are read on top of the plan. Nothing changes without `SEARX_URL`, which is the
laptop default. What is still missing for the "Searching the Public Web" milestones is the front
half: nothing turns a *description* into a company, so search fills gaps about a company you
named rather than finding you one.

**2. It analyses a company you name, or the one a description resolves to — and it still finds
no *set*.** A description now becomes candidate companies, the gate picks one when the evidence
is clear, and the run proceeds against it; the report says on its first line that the company was
chosen rather than named. What is still missing is **competitor-set derivation**: one idea
producing several companies to compare. So a description gets you one company's report, not a
comparison, and `landscape candidates` remains the way to see the whole list. Every site in the
prompt is now a subject — `basecamp.com vs linear.app` reports on both, capped at three for the
wait, with one section holding every company's prices
([`landscape-analyze/src/subject.rs`](crates/landscape-analyze/src/subject.rs)). What is still
missing is *discovery*: nothing turns an idea into a set of competitors, and there is no feature
matrix. So it compares a set you supply rather than one it finds.

**3. The percentage rungs cannot be reported, because nothing measures them.** The ladder below
asks for "10% / 25% / 50% / 80% of relevant information returned for some sample ideas". That
requires a set of sample **ideas** with known correct competitor sets and a recall metric.
`landscape-golden` measures something different and narrower: whether the extractors and the
model read a *frozen page* correctly. **Building the instrument is a prerequisite for reporting
the ladder at all**, and it is not on any phase's task list. See [B1](#4-blockers).

---

## 2. Feature milestones

### The ladder

Every feature below is tracked against the same eight rungs.

| Rung | Means |
|---|---|
| **R1** | Happy path — some information comes back |
| **R2** | 10% of the relevant information, on sample ideas |
| **R3** | 25% |
| **R4** | 50% |
| **R5** | 80% |
| **R6** | Relevant information is returned — the feature does its job |
| **R7** | **Ready for demo** — deployed, works for inputs we choose, demonstrable |
| **R8** | **Ready for use** — deployed, works for any input, and an honest "nothing found" when there is nothing |

For flow features — signup, subscription, community, notifications — "% of relevant information"
has no meaning, so R2–R5 read as **% of the flow's steps working end to end**. That
substitution is deliberate and is noted on each one.

### Where every feature stands

| Feature | Rung | One-line status |
|---|---|---|
| [F1 Searching for competitive information on a product idea](#f1--searching-for-competitive-information-on-a-product-idea) | **R1 partial** | Works for a prompt naming a domain, and the first screen now offers three ideas that do — real analyses over six curated companies. An idea nobody curated still returns nothing: the search channel exists as a crate and no analysis calls it. |
| [F2 Editing the product idea to get better results](#f2--editing-the-product-idea) | **R0** | A run has a URL now, so there is something to return to — and still no control that edits it. |
| [F3 Asking follow-up questions](#f3--asking-follow-up-questions) | **R0** | Not started. Report is terminal — read it or run another. |
| [F4 Sign up / registration](#f4--sign-up--registration) | **R0** | No authentication code exists in the repository. |
| [F5 Subscription setup and cancellation](#f5--subscription-setup-and-cancellation) | **R0** | Not started, and blocked on an unmade commercial decision. |
| [F6 Slack-like discussion and troubleshooting](#f6--slack-like-discussion-and-troubleshooting) | **R0** | Not started. |
| [F7 Notifications for changes in public information](#f7--notifications-for-changes-in-public-information) | **R0** | Not started. The *detection* half has a deterministic parser already. |
| [F8 The report itself](#f8--the-report-itself) | **R3–R4** | 6 of 9 sections, **all 6 extractors**, several companies in one report, no matrix, no charts. |
| [F9 Claims you can check](#f9--claims-you-can-check) | **R4** | Structurally enforced at the type level; the verification pass is not built. |
| [F10 PDF export](#f10--pdf-export) | **R0** | Not started. |
| [F11 Share / permalink](#f11--share--permalink) | **R1** | **A run has a URL.** `/a/{id}` opens it, the address bar carries it from the moment it exists, and Back returns to the box. Sharing beyond a raw link — a copy button, an OG card, an expiry story — is not built. |
| [F12 What people are saying](#f12--what-people-are-saying) | **R0** | Not started. |
| [F13 Copy as context](#f13--copy-as-context) | **R0** | Not started. Cheapest customer-visible item on the list. |
| [F14 The wait](#f14--the-wait) | **R3** | First content in 23s on a laptop, with no model involved. The whole-report figure still needs the target hardware. |

---

### F1 — Searching for competitive information on a product idea

**Rung: R1, for both input shapes.** A description now produces a comparison rather than a
refusal; what is unmeasured is whether it is the *right* comparison.

| Rung | State | Why |
|---|---|---|
| R1 happy path, minimal information | **Met for a domain and for a description, and both now produce a comparison.** | *"privacy-friendly website analytics"* and `basecamp.com` both end in several companies compared side by side, each carrying the reason it is there. The description is searched for; the named company has its rivals searched for, off queries templated from its own name. What is **not** met is R2 and up — nothing measures whether either set is the right set. |
| R2 10% of relevant information | **Unknown — unmeasurable.** | No idea-level golden set exists. See [B1](#4-blockers). |
| R3 25% · R4 50% · R5 80% | **Unknown — unmeasurable.** | Same. |
| R6 relevant information returned | **No.** | Several companies, all 6 question kinds, off-domain pages when a company's own leave a question empty, and a set derived from **either** input. What is missing is any measurement that the set is right ([B1](#4-blockers)) — which is now the only thing missing here that is about this feature rather than about the product around it. |
| R7 ready for demo | **No.** | Not deployed. |
| R8 ready for use | **No.** | Not deployed, and unmeasured. A genuinely ambiguous one-word name is told apart from every other refusal and **answerable in one click**, and so is an ambiguous *market* — the two questions share one chip. What is still missing here is measurement ([B1](#4-blockers)), not a way to answer. |

**And a reader's words are turned into the market's before anything is searched for.**
*"a free competitive landscape research tool"* is searched as *competitive intelligence
software*, counted once per independent site over the titles of the searches a run already
sends — and the companies are found **with those words**, which is a different set from the one
the reader's phrasing returns. The substitution is on the report as *"Interpreted as …"*, so a
wrong reading is visible before anything under it is believed, and a second round of searches
goes out only when the words actually changed. [BENCHMARKS.md](docs/BENCHMARKS.md) Runs 35 and
36.

**What exists.** `crates/landscape-discover` (on-domain probes, sitemap, `llms.txt`, ranked and
capped at 8), `crates/landscape-fetch` (SSRF guard at 100% coverage, robots.txt, per-host
politeness), `crates/landscape-extract` (Markdown conversion, span pre-selection, **all six**
extractors), `crates/landscape-search` (templated versioned queries, a `SourceProvider` seam, a
SearXNG adapter, host-based admission, candidate generation, competitor-set derivation, and
vocabulary resolution),
`crates/landscape-llm` (grammar-constrained decoding), `crates/landscape-analyze` (the
orchestrator), SSE streaming, and a React page that renders sections as they land.

**What the example ideas are and are not.** Three ideas on the first screen, each naming two
real companies, each producing a real fetched and cited report. Only the *choice of companies*
is curated: the sentence a chip puts in the box contains the domains, so a reader can see and
edit them, and `landscape examples` re-checks all six against the live web and fails if one has
lost its pricing page. **This does not move the rungs below**, because it does not turn a
description into companies — it hands over three descriptions whose companies are already known.

**What is missing, in the order it blocks things:**

1. **No measurement that the set is the right set.** Three searches agreeing and a shared word
   on a front page is the whole of the evidence a company is in somebody's market. It is
   arithmetic a reader can check, and it is not a *recall* number — nothing says how many real
   competitors were missed. That is [B1](#4-blockers), and it is now the top item here because
   the software that needed building got built. [BENCHMARKS.md](docs/BENCHMARKS.md) Run 30.
2. **Two clarifying questions exist; two do not.** An ambiguous brand name and an ambiguous
   *market* are both asked about with chips. The buyer and intent questions in
   `PRODUCT_SPEC.md` §3 need the grammar-constrained router model, which is Phase 2. Neither
   built question has a **skip**, deliberately — §3 promises every question is skippable, and
   skipping either means guessing: which of two same-named companies, or which of two markets
   every query below would be built from.
3. **Gaps are measured from admission, not from coverage.** `landscape search` asks about a
   question when discovery admitted *no page* for it. A page that was found and yielded nothing —
   Help Scout's `/blog` for *changes* — therefore triggers no search, even though the section
   comes back empty. `Coverage` already tells those silences apart; wiring the trigger to it
   needs the orchestrator.
4. **Half the caching.** The fetch cache is built and the worker holds one `Fetcher` for the
   process, so two readers of the same competitor share the pages rather than both paying a
   stranger's server for them. The **per-source extraction cache** is not: one page read twice
   still costs two sets of model calls across processes and across `PROMPT_VERSION` bumps.
   Nothing is shared between processes either — that needs a store, and the laptop rule is that
   nothing requires a database.

**All six extractors are built.** Trust posture reads a closed vocabulary of compliance
standards; investment direction reads a careers page and asks no model at all.

**Blockers:** [B1](#4-blockers) (no measurement — now the only one holding this feature back
on its own merits), [B5](#4-blockers) (not deployed).
**Risks:** [K1](#5-risks), [K2](#5-risks), [K5](#5-risks).

---

### F2 — Editing the product idea

**Rung: R0.** There is something to return to, and nothing that edits it.

**The dependency this waited on is met**: D2 gave a finished analysis a URL, so a reload no
longer loses it. What is still unbuilt is the control — the interpretation header from
`COMPETITIVE_DISCOVERY.md` §4–§6, *"we read your idea as this; correct it"*. The `searched_as`
line in the UI is its read-only ancestor: it tells a reader what was searched for and gives
them no way to change it.

**Depends on:** ~~[F11](#f11--share--permalink) first~~ — done — then the interpretation
header, then re-run-with-edits.
**Blockers:** [B3](#4-blockers).

---

### F3 — Asking follow-up questions

**Rung: R0.**

Two different features are specified. **Clarifying questions** (≤3, chip-answerable, fired only
when discovery fails to converge) have started: an ambiguous brand name is asked about with one
chip per candidate, and answering starts a new analysis. The other three triggers need a router
model or vocabulary resolution. **Conversational follow-up** on a finished report (`UI_FLOWS.md`)
is not started — a report today is terminal, and a chip answers the question *instead of* the
run rather than continuing it.

**Depends on:** F1 reaching R6, since a follow-up over a thin report inherits its thinness.

---

### F4 — Sign up / registration

**Rung: R0.** Ladder reads as % of flow steps.

There is no authentication code in the repository — no session, no cookie, no magic link, no
`users` table. The three migrations are `analyses`, `failure_kind` and `generation`. Everything
runs anonymously — but no longer unlimited: **the specified 2/day anonymous cap is built**, per
address, counted where a run starts, and a failed analysis costs nothing. It needs no account,
which is the point of it; what is still missing is everything a *signed-in* reader would get
instead.

Specified for Phase 3: magic-link auth, ~90-day sessions, free tier of 10 analyses/month,
history, saved reports, usage meter, and GDPR export/delete.

**Blockers:** [B6](#4-blockers) — transactional email is unprovisioned, and magic-link
deliverability is on the critical path for this flow.

---

### F5 — Subscription setup and cancellation

**Rung: R0.** Ladder reads as % of flow steps.

Nothing exists. Beyond the code, one decision blocks writing any of it: **merchant of record,
or not.** Stripe leaves VAT/GST/sales-tax registration and remittance as our legal obligation;
Paddle or Lemon Squeezy absorb it at a higher fee. The roadmap recommends Stripe and requires
the choice be recorded before billing code is written, because switching after customers exist
is genuinely painful.

**Blockers:** [B7](#4-blockers) — the merchant-of-record decision, unmade.
**Risks:** [K7](#5-risks).

---

### F6 — Slack-like discussion and troubleshooting

**Rung: R0.** Ladder reads as % of flow steps.

The design is the "slash-lite" open knowledge base in `docs/SUPPORT_SYSTEM.md`: public
searchable `/help` with slash commands, tags, threads, replies, voting and flagging, seeded
with 25–30 official articles, indexable, with private support as the minority channel. None of
it is built; no seed articles are written.

Worth noting because it is easy to under-rate: this is the only feature on the list that is
simultaneously **support**, **SEO surface** and **community**, and it is specified to exist
*before* it is needed rather than after the first support spike.

---

### F7 — Notifications for changes in public information

**Rung: R0**, with one genuine head start.

`crates/landscape-extract/src/changes.rs` already extracts dated public changes from a
changelog **using no model at all** — dates are on the deterministic side of the line. That is
the detection primitive the watch loop needs.

Everything else is absent: the scheduler, conditional GET and content hashing, near-duplicate
suppression, importance scoring, digests, alert email, one-click feedback, unsubscribe and
bounce handling. And there is no email provider.

**Risks:** [K6](#5-risks) — this is the retention thesis. If watches are not the hook, the
subscription's primary justification is wrong, and that is not knowable until it ships.

---

### F8 — The report itself

**Rung: R3–R4.** The shape is right; the content is a third of what is specified.

| Specified | Built |
|---|---|
| Nine sections | **Six.** Pricing, what it does, recent changes, company facts, trust, direction. |
| Six question kinds with extractors | **All six.** Two of them — recent changes and where they are investing — run no model at all. |
| Feature comparison matrix, five-state cells | **None.** Needs a competitor set, which does not exist. |
| SVG charts (feature matrix, cost-at-scale) | **None.** |
| Coverage notes — "nothing found, here is what was checked" | **Built, and it is the strongest thing here.** Distinguishes four different silences. |

The three absent sections — positioning, sentiment themes, SWOT — are interpretation over
sources the pipeline does not gather. They are honestly absent rather than empty, which is the
right call: an empty section that can never fill teaches a reader to skim past empty sections.

---

### F9 — Claims you can check

**Rung: R4.** The strongest area of the codebase, and still half-built.

**Built:** a `Claim` cannot be constructed without a source label and a verbatim quote, so an
unsourced sentence is not representable. `Report::every_claim_is_traceable` refuses a report
whose citation does not resolve. Constrained decoding gives a 100/100 schema-valid round trip.
Ten real pages are frozen with what the parsers must make of them.

**Not built:** `landscape-verify` — exact→normalized→fuzzy quote matching against the fetched
source, drop accounting, type-specific validators (price/period consistency, date sanity,
version verbatim checks), and the single regeneration retry. Until it exists, "the quote is in
the source" is guaranteed by *construction* in our own code but never *re-checked against the
page*, and the citation-coverage and drop-rate gates (≥97% / ≤3%) cannot be measured.

**Risk [K3](#5-risks):** a defective quantisation once passed every check in place at the time —
fast, always parseable, schema-valid, and wrong. Shape guarantees are not accuracy guarantees.

---

### F10 — PDF export

**Rung: R0.** Not started. Typst templates specified for Phase 2. Note the metric attached to
it: PDF download rate ≥25% of completed reports — it is a headline value moment, not a nicety.

### F11 — Share / permalink

**Rung: R1.** A run has a URL. `/a/{id}` opens it, `pushState` puts it in the address bar the
moment the analysis exists, and the browser's Back button returns to the empty box. A link that
points at nothing says so and leaves the box there, because a dead link is the one a reader is
most likely to have kept.

That unblocks [F2](#f2--editing-the-product-idea) and [F3](#f3--asking-follow-up-questions),
both of which need something to return to, and it is what stops a demo dying on a reload.

**Not built:** a copy-link control, link previews, and any answer to how long a URL stays good.
Anonymous analyses live as long as the row does, which is a retention decision nobody has made.

### F12 — What people are saying

**Rung: R0.** Hacker News and GitHub only, both unambiguously open. Reddit, X and LinkedIn are
excluded — X on cost, Reddit on terms — and the plan's answer is to disclose them by name with
prefilled searches the reader opens themselves. The Reddit exclusion currently **rests on
vendor blogs who sell Reddit data access**, which our own trust model would classify as
interested parties. Unconfirmed ([B4](#4-blockers)).

### F13 — Copy as context

**Rung: R0.** The whole report as clean Markdown with every URL and date, sized to paste into
the reader's own AI assistant. Near-free to build, and it is the feature that settles the
positioning: not a worse chatbot, but the evidence file a chatbot cannot assemble.

### F14 — The wait

**Rung: R3.** Streaming is real: sections appear as they are finished, and watching it in a
browser found two defects 425 passing tests had not.

**And first content now meets §2.1A.** The page that needs no model is read first, so on
`linear.app` the first thing on screen is seven dated changes at **23 seconds, with no model
involved at all** — measured, not estimated, and not delayed even by a model that has stopped
answering. Each question is worth one page that needs a model, taking the six demo companies from
128 model calls to 88, and the skipped pages are named on the report
([BENCHMARKS.md](docs/BENCHMARKS.md) Run 23).

**What is still open is the seconds, and it is not a code change.** One call's cost belongs to
the model and the machine; `landscape cost` counts the calls with no model running, and the
end-to-end figure is taken from a client's side of a deployment
([ADR 0011](docs/decisions/0011-no-experiments-on-production.md)). Of the 23 seconds above,
about 20 are discovery — **that is the next lever**, and it is nobody's blocker today.

What is not true yet: **first content in 40 seconds.** Locally it is nearer two minutes,
because nothing chooses the order pages are read in — the changelog needs no model and could go
first. And no end-to-end figure exists for the target hardware at all, because the target
hardware has nothing on it. The target is p50 ≤180s with first content ≤40s on the free tier.

---

## 3. Phase milestones

Percentages are **software only** — work an agent does in this repository — and exclude founder
work such as the concierge interviews, the terms audit and the deployment. They are a judgement
made by counting each phase's "Ship" and "Technical tasks" entries against what is in the tree,
with the evidence beside them. Where a phase's remaining work is mostly *not* software, that is
said rather than hidden in a number.

| Phase | Begun | 25% | 50% | 80% | Complete | Software |
|---|---|---|---|---|---|---|
| **0** Foundations & model bake-off | ☑ | ☑ | ☑ | ☑ | ☐ | **~90%** |
| **1** Vertical slice: anonymous analysis | ☑ | ☑ | ☑ | ☑ | ☐ | **~65%** |
| **2** Grounding verification, PDF & quality | ☑ | ☐ | ☐ | ☐ | ☐ | **~10%** |
| **3** Accounts, limits, knowledge base | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |
| **4** Monetization | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |
| **5** Watchlists & notifications | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |
| **6** Cold start & growth | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |
| **7** Retention, scale & model upgrade | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |
| **8** Sustainability | ☐ | ☐ | ☐ | ☐ | ☐ | **0%** |

### Phase 0 — at 80%, not complete

**Software remaining:** one exit criterion, and it is not really software — *a measured,
realistic end-to-end latency estimate.* Partial: one extraction takes ~3.8s on the router tier
under contention, so 120s buys roughly 32 of them before any fetching. There is no end-to-end
figure because there is no deployment to measure from a client's side, which is the only place
[ADR 0011](docs/decisions/0011-no-experiments-on-production.md) permits measuring it.

**Non-software remaining, and it is the higher-risk half:**

- **The concierge five (G1 gate).** ≥5 hand-made reports delivered to real founders, with
  reactions recorded. **Not started.** This is the highest-risk open item in the whole roadmap:
  everything from Phase 1 onward is built on an unvalidated assumption about the report format,
  the buyer and the price.
- **The source-terms audit (Track D).** **Not started.** Decides whether review platforms are a
  data source or a wall, and the discovery ranking is written against an assumption until it is
  answered.
- **The Oracle box.** Provisioned, but nothing here can reach it — no address, no SSH user, no
  key. The shape (4 OCPU / 24 GB aarch64) and the Pay-As-You-Go conversion are both unconfirmed
  from this workspace.

**Closed on evidence, and worth keeping visible:** the model choice (Qwen3-4B Q4_K_M extracts;
the 1.7B invented a price on `contact-sales` and is a router, not an extractor), `q8_0` KV
quantisation, grammar-constrained round trip at 100/100, the observability decision, and every
merge gate green on an empty repository.

### Phase 1 — past 50%, short of 80%

**Built:** the whole spine. Prompt → queue → worker → discovery → fetch → Markdown → span
pre-selection → extraction → assembled cited report → SSE → a React page that fills in as it
runs. Plus the failure states a run only reaches when something goes wrong: a worker that dies,
a reclaim, out-of-order progress writes, and a generation number that stops a replaced worker
finishing over the run that replaced it.

A replaced worker now also **stops**: the progress callback answers whether the run is still
wanted, and a worker the sweep has replaced abandons the pages it has not read rather than
spending prefill on a report nothing will accept. Twelve model calls become one on the page that
made a reader wait four minutes ([BENCHMARKS.md](docs/BENCHMARKS.md) Run 20).

**The report side is complete.** All six questions a report has a section for now have an
extractor behind them, and the three frozen careers pages brought the deterministic golden set
to fourteen. What is left in this phase is almost entirely about *getting to* a company rather
than reading one.

**Not built — the 35%:**

| Missing | Consequence |
|---|---|
| `landscape-search` wired into an analysis | **The crate is built** — templated versioned queries for the questions probes left unanswered, a `SourceProvider` seam, a SearXNG adapter, host-based admission, 41 tests (34 unit, 7 over a socket). **Nothing calls it**, so an idea still cannot become a company. **The phase's defining gap**, now a join rather than an invention. |
| Candidate generation for entity resolution | The disambiguation gate has nothing to disambiguate. |
| Competitor *discovery* | Several companies can be analysed, but only ones the reader names. |
| ~~One of six extractors (direction)~~ | **Built.** All six questions extract, and two of them need no model. The `no extractor yet` branch is deleted rather than left unreachable, so a seventh question is a build error. |
| ~~Fetch cache~~ + per-source extraction cache | **The fetch half is built** — one `Fetcher` per process, bounded in bytes, and a page served again reports when it was actually read. The extraction half is not. |
| ~~Anonymous rate limit (2/day)~~ | **Built.** Two a day per address, hashed, reset daily. |
| Stage rail, source cards, citation hover cards | The UI is functional and unfinished. **Example chips are built** — three ideas over six companies, with what is curated said beside them — **and so are clarification chips**: a name matching several companies comes back as one button per candidate. |
| Conditional GET; per-analysis fetch cap | Every run re-fetches everything. |
| SSE replay ring buffer (`Last-Event-ID`) | Reconnect re-reads from the row rather than resuming. Works; not what was specified. |
| Golden set to 25 subjects | 15 of 25, five of them fetched. |
| Review-platform access audit | Same item as Phase 0 Track D. |

**Exit criteria, none yet met:** 20 consecutive varied analyses without a crash (never run);
p50 ≤240s and first content ≤45s on the target host (never measured there); 100% schema
validity (held on the golden set, not on live runs); *"the founder would show it to a stranger
without apologizing"* — the honest answer today is no, because the stranger would type an idea.

### Phase 2 — begun, ~10%

Two items are genuinely in: **embedded-state extraction** (`landscape-extract::embedded` —
tier 2 of the rendering ladder), and the **JS-rendering gap measurement** that produced
[ADR 0009](docs/decisions/0009-no-headless-browser.md): 28 real pricing pages, 85.7% print the
price in static HTML, gap 3.6%, so the browser tier is not built and 400 MB stays with the
models. That decision is marked provisional — Phase 2's exit re-measures it.

Absent: the comparison matrix, charts, `landscape-verify`, discovery from a category or an
idea, clarifying questions, discussion signals, PDF, evidence badges, feedback capture, the
demo pipeline, the eval suite with CI gates, and the golden set at 50 subjects.

### Phases 3–8 — not begun

No code. Each is gated on the phase before it, and Phases 4–8 are additionally gated on the
G2 validation gate (≥10 beta users, ≥3 who say they would pay, ≥1 who does) which cannot be
attempted until [S2](#1-the-headline-can-we-show-this-to-anyone) is reached.

---

## 4. Blockers

Ordered by what they hold up.

| # | Blocker | Holds up | Owner | Note |
|---|---|---|---|---|
| **B1** | **No measurement of "relevant information returned."** The golden set scores extractors against frozen pages; nothing scores a *report about an idea* against a known-correct answer. | Every R2–R5 rung of [F1](#f1--searching-for-competitive-information-on-a-product-idea). Reporting the ladder at all. | Founder defines correct; agent builds | Needs sample ideas with hand-written competitor sets and a recall metric. Not on any phase's task list — **this is a gap in the plan, not just in the code.** |
| ~~**B2**~~ | ~~**The search channel reaches nothing.**~~ **Closed.** Queries, the provider seam, a SearXNG adapter, admission, candidate generation, and set derivation — and an analysis that starts from a description and ends in a comparison. | ~~F1 R6/R7/R8, F2, F3, F8's matrix, S2~~ | Agent | Four PRs, from *"does not exist"* to a description producing a set. **What it does not close is [B1](#4-blockers)**: nothing measures whether the set is right, and a channel nobody can score is a channel nobody can improve. **SearXNG itself has still never been run against this** — the compose profile and `settings.yml` are checked in and unwalked. |
| ~~**B3**~~ | ~~**No permalink.**~~ **Closed.** `/a/{id}` opens a run, and the address bar carries it from the moment it exists. | ~~F2, F3, F11~~ | Agent | Small. Disproportionate. Done in Phase D. |
| **B4** | **Source terms unaudited.** Reddit, X, YouTube, Stack Exchange, GitHub search limits, review-platform robots.txt. | F12, the discovery channel ranking, part of Phase 0's exit. | Founder — must be read in a browser from the primary source | Two of these can invalidate a planned feature. Half a day. |
| **B5** | **Nothing is deployed.** No address, no SSH user, no key reachable from here; instance shape and Pay-As-You-Go status unconfirmed. **The procedure now exists** — [DEPLOY.md](docs/DEPLOY.md), with units and a build script — and has never been run. | Phase 0's latency exit criterion, and every number that can only be taken from a client's side. **No longer any readiness state**: those are defined by what the software does, verified locally. | Founder | Claude stops at the PR. The first person through DEPLOY.md is the one who finds out where it is wrong. |
| **B6** | **No transactional email provider.** | F4 (magic links), F7 (alerts and digests). | Founder provisions; agent integrates | Deliverability (SPF/DKIM/DMARC) is on the critical path for signup. |
| **B7** | **Merchant-of-record decision unmade.** | F5, and all of Phase 4. | Founder | Must be recorded *before* billing code is written. |
| **B8** | **The G1 concierge gate is unattempted.** ≥5 hand-made reports to real people. | Confidence in everything built since. Phase 0's completion. | Founder | Needs no code and no server. |

---

## 5. Risks

| # | Risk | Likelihood · Impact | Where it shows | Mitigation, and whether it is in place |
|---|---|---|---|---|
| **K1** | **We are 20 weeks from a user and have not met one.** The report format, the buyer and the price are all assumptions. | High · Severe | Everything built since Phase 0 | The concierge track exists precisely for this and is [B8](#4-blockers), unstarted. **Not mitigated.** |
| **K2** | **The headline input runs, and nothing says the answer is right.** "Type a business idea" is the product's promise; typing one now produces a comparison of several companies. Whether they are *the* companies is unmeasured. | Certain today · Severe | [F1](#f1--searching-for-competitive-information-on-a-product-idea) | [B2](#4-blockers) is closed. The risk **moved** rather than cleared: it was *"the promise does not run"* and is now *"the promise runs and its answer is unscored"*, which is harder to see and is [B1](#4-blockers). What is in place is that every company in the set carries the countable reason it is there, and every one left out carries the reason it is not — a reader can audit the answer even though we cannot yet score it. |
| **K3** | **Shape is not truth.** Constrained decoding guarantees a valid type and nothing about the values. A defective quantisation once passed every check we had. | Medium · Severe | [F9](#f9--claims-you-can-check) | Golden set with abstain-required subjects: in place. `landscape-verify` re-checking quotes against sources: **not built.** |
| **K4** | **Free-tier latency.** Four ARM cores cannot serve the 15–25s promise; prefill dominates. Users abandon mid-stream. | Certain at Rung 0 · High | [F14](#f14--the-wait) | Deterministic-first extraction and span pre-selection are built and working. Caching, section-parallel generation and read-ordering are not. Never measured on the target host ([B5](#4-blockers)). |
| **K5** | **Distribution, not features, is the likely cause of death.** The plan's only answer is one launch window in Phase 6, and the weekly distribution workstream was meant to start at Phase 1. | High · Severe | Not visible in the repository at all | The workstream is written as a standing weekly commitment. **No evidence it has started.** |
| **K6** | **The retention thesis may be wrong.** Subscriptions are justified by watches; nobody has been asked whether they want alerts. | Medium · High | [F7](#f7--notifications-for-changes-in-public-information) | Gate G3 (≥35% of users create a watch) exists and is unreachable for now. |
| **K7** | **Tax and merchant-of-record.** Choosing Stripe leaves VAT/GST/sales-tax obligations with us; switching after customers exist is painful. | Medium · Medium | [F5](#f5--subscription-setup-and-cancellation) | Decision required before code. [B7](#4-blockers). |
| **K8** | **A working-slow plan read as a failed plan.** Honest re-baseline is 42–48 weeks to Phase 6, not 20–24. Infrastructure at €0–15/month tolerates any schedule; a founder's morale may not. | Medium · High | This page | Re-baselining early is the cheap prevention. **This page is part of that mitigation** — it is here to make slow-but-real progress legible. |
| **K9** | **This page going stale.** A status page nobody updates becomes a confident fiction. | Medium · Medium | This page | The [update rule](#8-how-this-page-is-maintained) below. |

---

## 6. What "done" looks like for the next three states

The shortest honest path, in order. Nothing here is a schedule.

**To S1 — a guided demo. Reached.** The permalink is built, the binary serves the page, a
report covers every company named, three ideas over six real companies are on the first screen
and re-checkable against the live web, and the reads are ordered so the first thing on screen
costs no model call. Deploying it ([B5](#4-blockers)) changes who can see it, not what it does.

**To S2 — any business idea handled correctly. Past half.** The search channel is finished
([B2](#4-blockers)), both inputs produce a set, a report that covers one company says which of
four things happened to the others, and a run that produces no report says which of five
situations it is in — and the ambiguous one comes back as buttons, so answering it costs a
click rather than a retyped idea, a reader's words are resolved to the market's and searched
for in them, and a page fetched once is neither fetched nor read again — nor, past its hour,
downloaded again to learn it has not changed, and a careers page that links away is followed to
the board it names. What is left is the engine-backed half of the search channel and the last of
competitor-set derivation. **All six extractors are
done**, so no section is permanently empty any more.

**To S3 — friendly users find no issue.** `landscape-verify` and the quality gates; caching so a
second user on the same subject is fast; the report at nine sections with the matrix; PDF; and
the eval suite running in CI. Plus the first honest answer to *"would you show this to a
stranger without apologizing?"*

---

## 7. What is genuinely strong

A status page that only lists gaps misrepresents the project as much as one that only lists
wins. Four things here are ahead of where a project this early usually is, and each is a thing
that is expensive to retrofit and cheap to keep:

- **The quality gates predate the code.** `fmt`, `clippy -D warnings`, `deny`, `gitleaks`,
  `tsc --strict`, aarch64 cross-compile — all green on an empty repository, so the first real PR
  met the bar rather than establishing a lower one. `unwrap`/`expect`/`panic` are denied, not
  warned.
- **The honest-negative treatment is real and tested.** A section that found nothing says what
  was checked, and distinguishes four different silences: our gap, the company's gap, a page
  found and never opened, and a page read that stated nothing. Its first run found a changelog
  answering 200 that nothing had ever opened.
- **The failure states are driven by tests, not reasoned about.** A worker that dies mid-run, a
  reclaim, out-of-order progress writes, a reader holding a dead worker's answers. Two defects
  invisible to 442 passing tests were found by driving the real stream while mutating the store
  underneath it.
- **The documentation is executed.** Every fenced `bash` block in the README is run against a
  booted binary in CI, because two bugs once reached a reader through correct code and stale
  prose.

---

## 8. How this page is maintained

**Update it in the PR that changes what it describes.** Specifically:

| If the PR… | Touch |
|---|---|
| Moves a feature's capability | Its row in [§2](#where-every-feature-stands) and its detail block |
| Completes or adds a phase item | The row in [§3](#3-phase-milestones) and that phase's remaining-work list |
| Closes or opens a blocker | [§4](#4-blockers), and any feature that referenced it |
| Changes what a risk depends on | [§5](#5-risks) |
| Makes a readiness state reachable | [§1](#1-the-headline-can-we-show-this-to-anyone) — and say so in the PR title |

**Two rules for the numbers.** A percentage moves only when something in the tree moves — not
when a plan changes. And a rung is claimed only when it can be demonstrated: **R2 through R5
cannot honestly be claimed by anyone until [B1](#4-blockers) is closed**, because until then
there is nothing that could tell us we are wrong.
