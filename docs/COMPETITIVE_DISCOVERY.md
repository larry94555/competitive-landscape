# Landscape — Competitive Discovery

> How a free-text prompt becomes a verified list of companies to investigate, how their
> standing is assessed, and when the product asks a question instead of guessing.
>
> **Relationship to the other documents.** [FACT_CHECKING.md](FACT_CHECKING.md) covers how
> sources are judged *once we know what to look for*. This document covers what comes first:
> turning "research the competitive landscape for a free competitive landscape research tool"
> into a set of real companies. [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §3 specifies the
> clarifying-question UX; §6 here specifies when it fires.

---

## 1. Five input classes, one pipeline

Prompts arrive in five shapes, and the amount of discovery work differs enormously.

| Class | Example | Discovery needed |
|---|---|---|
| **A — Named set** | "Notion vs Coda vs Obsidian" | None. The set is given; resolve entities and go. |
| **B — Named single** | "Linear" · `https://linear.app` | Find the competitors of a known anchor. Moderate. |
| **C — Category** | "project management software" | No anchor. Resolve the category, then harvest. |
| **D — Product idea** | *"a free competitive landscape research tool"* | **Hardest.** No anchor, and the user's words are not the market's words. |
| **E — Job to be done** | "I'm building something for freelancers to chase invoices" | Hardest plus a category-inference step. |

Everything below is written for **D and E**, because A–C are strict subsets of the same
pipeline with earlier stages skipped.

---

## 2. Prompt completeness — measured by convergence, not by checklist

The obvious design is to parse the prompt for required fields and ask about whatever is
missing. **That design is wrong**, and it is worth saying why: it asks questions the prompt's
*wording* implies are needed, rather than questions the *world* actually requires.

- *"a tool for tracking competitor pricing"* names no segment, no geography and no delivery
  model — yet it maps to exactly one recognisable market. A checklist would ask three
  pointless questions.
- *"a CRM"* looks complete and is not. A solo-freelancer CRM and Salesforce are not in the
  same market, and comparing them produces a useless report.

So completeness is defined **operationally**:

> **A prompt is complete when a cheap discovery probe converges on one category with a stable
> vocabulary and a candidate set that is neither empty nor absurdly broad.**

The probe runs first, costs about five searches and no model tokens, and its output decides
whether a question is needed. This inverts the usual order — **discover, then ask only if
discovery was ambiguous** — and it is why most analyses ask nothing at all.

### 2.1 What completeness actually requires

| Dimension | Required? | How it is usually obtained |
|---|---|---|
| **Category** (in market vocabulary) | **Always** | Resolved in §4; asked only if it fails to converge |
| **Buyer segment** | Only when it splits the market | Inferred from the candidates' own language; asked when the set spans disjoint segments |
| **Delivery model** (product / service / API / open source) | Only when it splits the market | Inferred; asked when candidates are a mix |
| **Geography** | Rarely | Defaults to global-English; asked only when candidate sets differ sharply by region |
| **Intent** (positioning / pricing / features / gaps) | Never blocking | Defaults to "everything"; changes emphasis, not the competitor set |

**Constraints are recorded, not resolved.** "Free", "open source", "for teams", "self-hosted"
are filters on the candidate set, not part of the category. They are captured and applied at
ranking, and surfaced to the user as removable chips.

### 2.2 The convergence score

Computed deterministically from the probe:

| Signal | Converged | Ambiguous |
|---|---|---|
| Category term frequency across independent result titles/H1s | One term dominant | 2+ terms with comparable weight and low overlap |
| Review-site category mapping | Lands in one curated category | Spans unrelated categories |
| Candidate overlap between channels | Top candidates recur across ≥3 channels | Each channel returns a disjoint set |
| Candidate count after relevance filtering | 4–25 | 0–2, or >60 |
| Segment language among candidates | Consistent | Split (e.g. half say "for enterprises", half "for solo founders") |

**All converged → no question.** Otherwise §6 fires, at most once per dimension, at most three
in total.

---

## 3. The pipeline

```
0  Parse & classify      input class, named entities, category phrases, constraints
1  Vocabulary resolution user's words  ->  market's words                  (§4)
2  Seed harvesting       candidate names from independent channels          (§5)
3  Entity resolution     name -> canonical domain, deduped, live-checked
4  Relevance             direct | adjacent | substitute | not relevant      (§5.3)
5  Rank & select         top N, constraints applied                         (§5.4)
6  Disclose              full candidate list with scores and reasons        (§5.5)
   └─> hand off to the analysis pipeline (ARCHITECTURE §5, FACT_CHECKING §3)
```

Stages 0–2 constitute the **probe**. Convergence (§2.2) is scored after stage 4 on a cheap
pass, and the pipeline either continues or pauses to ask (§6).

---

## 4. Vocabulary resolution — the user's words are not the market's words

This is the step that makes class D work, and it is almost entirely deterministic.

A user writes *"a free competitive landscape research tool."* Nobody in the market uses that
phrase. The market says *competitive intelligence software*, *competitor analysis tools*,
*market intelligence platform*. Searching the user's phrasing finds blog posts; searching the
market's phrasing finds the market.

**Method:**

1. Issue 3–5 templated searches on the user's phrasing and obvious rewrites.
2. Collect **titles, `<h1>`s, meta descriptions, and review-site category names** from the
   results — not body text.
3. Extract candidate category terms by frequency **across independent hosts** (a term
   appearing on twelve pages from one publisher network counts once — see
   [FACT_CHECKING.md](FACT_CHECKING.md) §6).
4. Prefer the **most specific term that still recurs widely**. "Software" is too broad;
   "competitive intelligence software" is right; "battlecard automation platform" is too narrow.
5. **Review-site taxonomies are the strongest signal available.** G2, Capterra and similar
   maintain curated category trees that *are* the market's own vocabulary, with membership
   lists attached. A user phrase that maps cleanly onto one curated category is a converged
   category, and the category page doubles as a seed channel.

The resolved vocabulary — a primary label plus synonyms — is **recorded on the analysis and
shown in the report header**, because it determines everything downstream:

> Interpreted as: **Competitive intelligence software** (also: competitor analysis, market
> intelligence). Constraint applied: **free tier available**. [Change]

That line is doing real work. If the interpretation is wrong, the user sees it immediately and
fixes it in one click — which is faster and less annoying than any question we could have
asked upfront.

---

## 5. Finding the companies

### 5.1 Seed channels

Candidates are harvested from channels that are **independent of one another**, so that
recurrence across channels is meaningful rather than an echo.

| # | Channel | Yields |
|---|---|---|
| 1 | **Review-site category pages** (G2, Capterra, GetApp, Software Advice) | Curated membership lists — highest-yield channel for classes C/D **where access is permitted (§5.1.1)** |
| 2 | **"Best X" / "top X tools" searches** | Broad name lists |
| 3 | **"Alternatives to Y"** once one seed exists | Snowball; the highest-precision channel |
| 4 | **Vendor comparison pages** (`/vs`, `/alternatives`) | Vendors name their real rivals |
| 5 | **Community threads** (Reddit, HN, Stack Overflow) | Noisy, honest, surfaces substitutes others miss |
| 6 | **Marketplaces & app directories** (Chrome Web Store, Slack, Shopify, app stores) | Category-scoped, for categories that have one |
| 7 | **GitHub topics / awesome-lists** | Open-source and developer-tool categories |
| 8 | **Public ad libraries** | Who pays to bid on the category language |

**Snowballing** is bounded: two hops from the first seeds, breadth-first, stopping when a
round adds no new names that survive relevance filtering.

#### 5.1.1 The access question this channel depends on — settle it in week 1

Major review platforms commonly restrict automated access through `robots.txt`, bot detection
and terms of use. **This plan honours `robots.txt` as a hard commitment**
([FACT_CHECKING.md](FACT_CHECKING.md) §5.2), so the highest-yield discovery channel and part
of the sentiment section may be unavailable *by our own rules*. The two positions can collide,
and the plan must not discover that in week 8.

**Phase 0 runs an access audit** and records the outcome. Three cases:

| Outcome | Response |
|---|---|
| Access permitted | Channel 1 stays primary. No change. |
| Disallowed for our paths | **Channels 3, 4 and 6 become primary** — vendor `/alternatives` pages, snowballed alternatives searches, and marketplace directories. Discovery still works; recurrence across channels is what matters, not any one channel. |
| Partially permitted | Use what is allowed, disclose the rest as an access limitation. |

**The design already survives this**, which is why it is a scheduling risk rather than an
architectural one: candidates require **≥2 independent channels** and an existence gate, and
eight channels are specified precisely so no single one is load-bearing. What changes is the
*ranking*, and ranking is configuration.

**In the report**, an inaccessible platform is stated plainly rather than silently omitted:
*"Review-platform coverage: G2 not accessible under our fetching rules; themes below are drawn
from community threads and vendor-published testimonials only."* This is the same disclosed-gap
treatment used everywhere else, and it doubles as a fairness control — P21 warns that
differential access must not read as differential quality.

### 5.2 The distinction that makes this compatible with the fact-checking rules

A listicle is a source we may not be able to attribute
([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.1) — so how can we harvest names from it?

> **Discovery uses low-attribution sources for *leads*. Verification uses primary sources for
> *facts*.**

A name is a pointer, not a claim. When a page we could not attribute mentions "Kompyte," we do
not report anything that page said. We resolve the name to a canonical domain, **fetch that
company's own site**, and every subsequent fact comes from there. The lead is discarded once
it has served its only purpose.

Two consequences, both important:

- **A company only enters the report if its own site resolves and fetches.** This existence
  gate eliminates fabricated competitors entirely — the failure mode where a model invents a
  plausible company name that has never existed.
- **Lead sources are not cited in the report body.** They appear in the discovery disclosure
  (§5.5) as "where we looked", not as evidence for anything.

### 5.3 Relevance classification

Every surviving candidate is classified. This is a competitive-analysis distinction, not a
technical one, and getting it right is most of the value:

| Class | Definition | In the report |
|---|---|---|
| **Direct** | Same category, same buyer segment, same delivery model | Full comparison — matrix, pricing, everything |
| **Adjacent** | Same category, different segment or delivery model | Named with the difference stated; not compared head-to-head |
| **Substitute** | Different category, solves the same job | **Named explicitly** — see below |
| **Not relevant** | Fails category-language overlap | Listed in the disclosure with the reason |

Classification is measurable, not judgemental: **category-language overlap** between the
candidate's own homepage/meta language and the resolved vocabulary; **segment language**
("for enterprises" / "for freelancers"); **delivery model** from their own pricing and
signup pages.

**Substitutes deserve their own line.** The most common true answer to "who are my
competitors" is *a spreadsheet*, *a manual process*, *an existing general-purpose tool*, or
*doing nothing*. A competitive analysis that omits the substitute is describing a market the
buyer does not live in. Substitutes are surfaced from community threads ("we just use a
Google Sheet") and from vendors' own "why not spreadsheets" pages, and are named without a
feature comparison, because comparing a product to a spreadsheet on features is not useful.

### 5.4 Ranking and selection

Score = channel recurrence (independent channels only) × category-language overlap ×
segment match × constraint satisfaction, with a small recency-of-activity term (§7).

Selected: **3–5 direct** competitors for the comparison, plus **up to 3 adjacent** and
**up to 2 substitutes** named without full comparison. On Rung 0 the source budget
([ARCHITECTURE.md](ARCHITECTURE.md) §6) caps the comparison at 3 direct competitors; the rest
are named.

### 5.5 Disclosure — always show the work

The report always contains a discovery section, collapsed by default:

> **How this set was chosen.** Interpreted as *competitive intelligence software*; constraint
> *free tier*. 23 candidates found across 6 channels; 7 kept.
> [Show all candidates and why]

Expanded: every candidate with its score, the channels it appeared in, its classification, and
the reason for its position — including the ones not carried forward. **A competitive set
presented without its derivation is an unfalsifiable editorial choice**, and the reader has no
way to tell a thoughtful selection from an arbitrary one.

The set is **directly editable**: remove a competitor, add one by name or URL, and re-run.
See §6.3 — this is usually better than asking a question.

---

## 6. Clarifying questions

Governed by [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §3: at most three, one at a time,
chip-answerable, always skippable to a complete report.

### 6.1 When

Only when §2.2 convergence fails, and only for the dimension that failed. Ranked by
**information gain** — ask the question that most splits the candidate set, not the first one
that is technically unanswered.

| Failure | Question | Chips |
|---|---|---|
| Category spans 2+ unrelated markets | "Which of these are you working on?" | The resolved category labels |
| Segment language splits the set | "Who's the buyer?" | SMB / Mid-market / Enterprise / Consumer / Developers |
| Delivery models mixed | "Is this a product people use themselves, or a service you deliver?" | Product / Service / API / Not sure |
| Constraint is ambiguous | *"Free" as in a free tier, or open source?* | Free tier / Open source / Either |
| 0–2 candidates survive | "I found very little in this space. Is it very new, or is there another name for it?" | Free text + "Analyse anyway" |

**When the category genuinely does not exist yet** — a real case for novel product ideas, and
distinct from a failed search. If the user confirms the space is new, the report **changes
shape rather than failing**: it reports the **substitutes and adjacent categories** people
currently use for the same job, states plainly that no established category was found, and
lists the searches run so the negative is auditable (§ FACT_CHECKING §5.4). *"We could not find
an established category for this. Here is what people appear to use instead"* is a genuinely
useful answer to a founder with a novel idea — arguably more useful than a competitor list —
and it is honest, which an invented set of competitors would not be.
| >60 candidates survive | "That's a broad space. Narrow it?" | Segment chips + "Show me the whole landscape" |

### 6.2 When *not* to ask

- Anything inferable from a named product, a URL, or the candidates' own language.
- Intent. It changes emphasis, not the set, and defaults to "everything".
- Geography, unless candidate sets differ sharply by region.
- Anything where a sensible default plus a visible, editable interpretation line is enough —
  which is most things.

### 6.3 The better alternative: show the work, let them correct it

Direct manipulation beats interrogation. Where convergence is *marginal* rather than failed,
the product does not ask — it proceeds, and shows:

> Interpreted as **competitive intelligence software** · free tier · comparing **Crayon,
> Klue, Kompyte** · also found: Visualping *(adjacent)*, spreadsheets *(substitute)*
> [Edit this set]

One glance confirms or corrects it. This is faster than a question, requires no prompt
literacy, and is honest about what the system decided — consistent with the zero-learning-curve
principle. **Questions are reserved for cases where proceeding would waste 90–180 seconds of
free-tier compute on the wrong market.**

---

## 7. How each company is doing

This is where a competitive analysis is most tempted to invent, so the boundary is drawn
sharply: **we can report how actively a company is operating in public. We cannot report
whether it is financially healthy.**

### 7.1 Longevity — how long it has been around

| Signal | Source | Quality |
|---|---|---|
| **First web archive capture** | Internet Archive CDX API (free) | **Best available.** Independent, checkable, hard to game. Gives a firm "publicly visible since". |
| Self-stated founding year | About page | Primary, but self-reported |
| Domain registration date | WHOIS / RDAP | Often privacy-proxied; a floor, not a fact |
| First changelog or blog entry | Vendor's own site | Primary, precise |
| GitHub organisation creation | GitHub API | Precise, for dev tools |
| App-store listing date | Store listing | Precise where applicable |

Reported as *"publicly visible since 2019 (first archive capture 2019-04); the company states
it was founded in 2018"* — two dated, sourced facts rather than one asserted age.

### 7.2 Operating activity — the honest substitute for "how are they doing"

These are all public, dated, and checkable, and for most buying decisions they are **more
decision-relevant than revenue**:

| Signal | What it indicates |
|---|---|
| **Changelog / release cadence** over 12 months | Whether they are still building |
| **Last public update** (blog, changelog, release) | A changelog silent for 18 months is a strong, honest signal |
| **Open roles, count and function mix** (public ATS boards) | Where investment is going; whether they are growing |
| **Documentation freshness** | Sustained investment in the product |
| **Status-page incident history** | Operational maturity |
| **Public pricing changes** | Positioning shifts |
| **Support/community responsiveness** (public forums, GitHub issues) | Whether anyone is home |

Presented as a compact **operating signals** panel per competitor, every item dated and
sourced. Absence is reported as absence — *"no public changelog found"* — never as inactivity
([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5).

### 7.3 Financial standing — what is and is not knowable

| Public and reportable | Not reportable |
|---|---|
| **Public companies:** revenue, growth, segments, headcount from **SEC EDGAR** filings (free API, legally binding) | **Private-company revenue, ARR, burn, margins, runway** — not public |
| Announced funding rounds, amounts, dates, named investors (from company announcements and filings) | Valuations not disclosed in a filing |
| Announced acquisitions, mergers, shutdowns | Employee counts scraped from professional networks (privacy posture — QUALITY_GUARDRAILS §6) |
| Announced layoffs, and public regulatory notices where filed | Traffic, visitor and conversion estimates |
| Published customer counts, where the company publishes them | Profitability, churn, CAC, or any modelled metric |

**Anything a private company has not published, we do not estimate.** The report says so
plainly rather than leaving a suggestive gap:

> **Financial standing:** not publicly disclosed. Kompyte is privately held and publishes no
> financial statements. Publicly announced: Series A, $5M, 2021-06 [S12].
> We do not estimate revenue.

**For public companies this section is genuinely strong** — EDGAR is free, authoritative, and
structured — and the report should say when a competitor is publicly traded, because it means
far more is verifiable about them than about their private rivals. That asymmetry is itself
worth stating, so a reader does not mistake better documentation for better performance
(the same trap as P21).

### 7.4 The composite that is deliberately absent

No "health score", no traffic-light rating, no momentum index. A single number combining
non-comparable signals would be interpretation dressed as measurement, and it is exactly the
kind of confident-looking output this product exists to avoid. The panel shows the dated
signals; the reader draws the conclusion.

---

## 8. Worked example

**Prompt:** *"Research the competitive landscape for a free competitive landscape research
tool."*

**Stage 0 — parse.** Class D. No named entity. Category phrase: "competitive landscape
research tool". Constraint: "free" (ambiguous — recorded, not resolved).

**Stage 1 — vocabulary.** Five searches on the phrase and rewrites. Recurring terms across
independent hosts: *competitive intelligence software* (dominant), *competitor analysis
tools*, *market intelligence*. Maps cleanly onto one curated review-site category.
**Converged.**

**Stage 2 — seeds.** Category page membership · "best competitive intelligence tools" ·
community threads · vendors' own `/alternatives` pages. 23 distinct names.

**Stage 3 — entity resolution.** Each name → canonical domain → homepage fetched. Four fail
to resolve or are the same company under two names; 19 remain.

**Stage 4 — relevance.** Direct: enterprise CI platforms whose own language matches the
category. Adjacent: web-change-monitoring tools (same job, narrower scope); SEO/traffic
intelligence suites (different primary category, overlapping buyer). Substitutes: manual
research, general-purpose AI chat, spreadsheets.

**Convergence check.** Category ✓. Segment **✗** — the direct set splits between
enterprise-priced platforms and self-serve tools, and the "free" constraint interacts with
that split.

**Question fired (one):** *"Free as in a free tier, or open source?"* → chips.
Answer narrows both the constraint and, indirectly, the segment.

**Stage 5–6.** Top 3 direct + 2 adjacent + 2 substitutes. Discovery disclosure lists all 23
candidates with scores, channels, classification and reasons.

**Header shown to the user:**

> Interpreted as **competitive intelligence software** · free tier · comparing 3 · also found
> 2 adjacent, 2 substitutes · 23 candidates considered [Show] · [Edit this set]

**Note the meta-point:** this is the product analysing its own market, and it lands on the
honest answer — the direct competitors are enterprise platforms at enterprise prices, the
nearest free options are adjacent single-purpose tools, and the real substitute is a person
with browser tabs. That is a more useful result than a list of five products, and it comes
from classifying rather than merely listing.

---

## 9. Cost on free-tier hardware

Discovery must be cheap, because it happens before the analysis and the user is already
waiting ([ARCHITECTURE.md](ARCHITECTURE.md) §4.4).

| Stage | Cost |
|---|---|
| Parse & classify input | One 1.7B router call, ~60 tokens |
| Vocabulary resolution | 3–5 searches; term frequency over titles — **deterministic, no model** |
| Seed harvesting | 4–6 searches + 2–3 category page fetches |
| Entity resolution | HTTP only; one router call **only** when candidates are ambiguous |
| Relevance classification | Term overlap — **deterministic, no model** |
| Ranking, convergence scoring | Arithmetic |

**Roughly 10–15 HTTP requests and one or two small model calls.** Discovery is cached by
resolved category, so the second analysis in a popular category skips stages 1–2 entirely —
and popular categories are exactly where repeat traffic lands.

---

## 10. Phasing

- **Phase 1** — input classification; entity resolution with the disambiguation gate;
  named-set and named-single paths (classes A/B).
- **Phase 2** — vocabulary resolution; seed harvesting from review categories, search and
  vendor comparison pages; relevance classification; the interpretation header and the
  editable set (§6.3). This is what makes classes C/D/E work.
- **Phase 3** — convergence scoring and the clarifying-question ladder; full discovery
  disclosure with candidate scores.
- **Phase 5** — operating-activity panel (reuses watch and changelog infrastructure);
  archive-based longevity.
- **Phase 7** — EDGAR adapter for public companies; substitute detection from community
  threads; ad-library channel.
