# Landscape — Information Gathering & Fact-Checking Strategy

> How public information is found, how it is judged, how claims are verified, and how a
> reader independently confirms every statement in a report.
>
> **Relationship to the other documents.** [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md)
> covers the middle of the chain — *model output → verified claim*. This document covers the
> two ends: **world → trustworthy source → extracted text** (upstream) and **verified claim →
> reader can check it themselves** (downstream). The upstream end is where most real errors
> originate; the downstream end is what makes the product trustworthy rather than merely
> confident.

---

## 1. The problem catalogue

Every problem is numbered so §9 can map each to its mitigation. **P1–P9** were raised in
review; **P10–P23** are ones this design has to solve regardless.

### Raised in review

| # | Problem |
|---|---|
| **P1** | **Gathering method.** How is public information actually retrieved, at what cadence, within what limits? |
| **P2** | **Source identification.** How does the tool decide which pages are worth reading? |
| **P3** | **Market research.** How is the competitive set itself discovered, when the user names only one product — or none? |
| **P4** | **Sufficiency.** How do we know we have gathered *enough* to write a report rather than a guess? |
| **P5** | **Fact-checking.** How is each statement checked before it reaches a reader? |
| **P6** | **Hallucination.** How is invented content detected and removed? |
| **P7** | **Misinformation.** Sources that are wrong without meaning to be — stale pages, sloppy journalism, outdated listicles. |
| **P8** | **Disinformation.** Sources that are wrong *on purpose* — vendor comparison pages, astroturfed reviews, SEO farms. |
| **P9** | **Independent confirmation.** How does a reader check a claim themselves, without trusting us? |

### Not raised, and equally load-bearing

| # | Problem |
|---|---|
| **P10** | **The vendor is the best and worst source about itself.** Their site is authoritative for *what they claim* and unreliable for *what is true*. Conflating those two is the central epistemic error in automated competitive intelligence. |
| **P11** | **Comparison pages are adversarial by construction.** A `/vs-competitor` page is written by an interested party, is usually stale about the rival, and is cherry-picked by design. |
| **P12** | **Review platforms are gamed.** Incentivised reviews, review gating (only happy customers asked), vendor-sponsored placement, and heavy recency skew. Ratings are not samples from a population. |
| **P13** | **Every fact has a shelf life, and they differ wildly.** A price decays in weeks; a founding date never decays. Treating them alike produces confidently stale reports. |
| **P14** | **Source circularity.** Three "independent" sources quoting one press release is **one** source. Corroboration counted by URL is corroboration theatre. |
| **P15** | **AI-generated slop in the corpus.** "Best X alternatives 2026" listicles are increasingly machine-written with fabricated pricing and features. Ingesting them **launders someone else's hallucination into our citation** — the most dangerous single failure mode available to this product. |
| **P16** | **Entity ambiguity.** Wrong entity resolution produces a report that is wrong throughout yet internally consistent and fully cited. Rebrands, acquisitions, subsidiaries and shared names all cause it. |
| **P17** | **Extraction fidelity ≠ truth.** A claim can be perfectly grounded in text we extracted *badly*. If the parser mangled a pricing table, every verification layer passes on a wrong number. **Our verification proves fidelity to the extracted text, not to the page.** |
| **P18** | **Geographic and currency variance.** Pricing pages vary by IP geolocation, currency, and whether tax is included. Our fetcher sits in one region. |
| **P19** | **A/B tests and personalisation.** The page we fetched may not be the page the reader sees. |
| **P20** | **Negative claims are the hardest to audit.** "Not found in public sources" is unfalsifiable to a reader unless we show exactly what was searched. |
| **P21** | **Robots-blocked sources create systematic bias.** A competitor who blocks crawlers appears less documented, not less capable — and a naive report reads that as a weakness. |
| **P22** | **The user's framing can bias the analysis.** "Why is X better than Y" must not steer retrieval or synthesis toward confirming the premise. |
| **P23** | **Publishing wrong claims about real companies carries legal risk.** A fabricated "they raised prices 40%" on an indexable page is a trade-libel exposure, not merely an accuracy bug. |

---

## 2. The governing principle

> **We do not report what is true about a company. We report, with receipts, what public
> sources say — who said it, when, where, and how independently.**

This is not a hedge; it is the only defensible epistemology for an automated tool working
from public data. It resolves P10, P11, P22 and P23 at a stroke, because it converts every
statement from an assertion about the world into an attributed, checkable observation.

**Practical consequence — attribution framing is mandatory.** Never *"Linear is faster than
Jira."* Always *"Linear describes itself as 'purpose-built for speed' [S1]; a G2 review theme
(14 mentions) cites responsiveness as a strength [S9]."* The first is a claim we cannot
support and could be sued over. The second is verifiable, useful, and true regardless of
which product is actually faster.

---

## 3. Information gathering

### 3.1 Entity resolution comes first (P16)

Nothing is fetched until the subject is pinned down, because every downstream error inherits
from getting this wrong.

1. **Normalise the input** — URL, brand name, or free description.
2. **Candidate generation** — search, plus the domain if a URL was given.
3. **Canonical domain selection** — the domain that the candidate's own pages, its social
   profiles, and its documentation agree on. Disagreement is a signal, not noise.
4. **Disambiguation gate** — if two candidates score within a margin, **ask the user**
   (PRODUCT_SPEC §3). One chip click prevents an entire wrong report.
5. **Record identity evidence**: canonical domain, legal name if published, and any rebrand or
   acquisition notice found. Rebrands are a common silent failure — *"Company A (formerly B,
   renamed 2025-03)"* appears in the header.

**Wikipedia and Wikidata are used for disambiguation only** — to tell three products apart —
and are **never** cited as a fact source in the report body.

### 3.2 Source classes and the two-axis trust model (P10)

The critical insight, and the thing most automated CI tools get wrong: **a source's
trustworthiness depends on what you are asking it.** So every source carries two ratings.

| | **Authority for "what they claim"** | **Authority for "what is true"** |
|---|---|---|
| Vendor's own site | **Highest** — definitionally correct | **Low** — marketing copy |
| Vendor's pricing page | **Highest** | **High** — a published price is a commitment |
| Vendor's changelog / release notes | **Highest** | **High** — dated, specific, checkable |
| Vendor's `/vs-competitor` page | High *about themselves* | **Lowest about the rival** (P11) |
| Official docs / API reference | High | **High** — hardest to fake, rarely marketing |
| Public status page | High | **High** |
| Public job postings | High | **Medium-high** — reveals investment areas |
| SEC/EDGAR & regulatory filings | Highest | **Highest** — legally binding |
| Established trade press | Medium | Medium — check for press-release provenance |
| Review platforms | Medium | **Medium-low** (P12) |
| Forums (Reddit, HN) | Low | **Low individually, useful in aggregate** |
| Listicles / "alternatives" content | Low | **Lowest** (P15) |

**Rules that fall out of this table:**

- **Any claim about company B sourced from company A's page** is retagged and rendered as
  *"per Linear's own comparison page"* — never as neutral fact.
- **Docs and status pages are underrated and preferred** where they cover the question. They
  are written for users, not buyers, and are correspondingly honest.
- **Primary sources set the authoritative values** — the pricing table, the feature matrix,
  the charts. Secondary sources may add, corroborate, and fill gaps, under the placement
  rules in §3.2.4.

### 3.2.1 The four source classes

Every source is placed in one of four classes by **objective, checkable signals** — never by a
model's opinion of a publisher, and never as a judgement about the publisher at all.

**The framing rule that governs this entire section:** a class records *what we were able to
confirm*, not what a site is or lacks. The subject of every sentence is us. We do not say a
page is unreliable, incomplete, or wrong. We say what we confirmed, what we could not confirm,
and where we found information we could not reconcile. See §3.2.5 for the exact language.

| Class | What it records | Use |
|---|---|---|
| **P — Primary** | The subject's own properties, or a regulator's | Sets authoritative values |
| **A — Attributed** | We confirmed authorship, dating, publisher identity and sourcing | Included by default, marked, with the confirmed signals shown |
| **U — Unattributed** | We could not confirm enough of those signals to attribute it | Included only on the permissive setting, marked |
| **N — Not used** | We could not verify it against our criteria, or it carries information we could not reconcile with a primary source | Not in the report body; **its existence is always disclosed** |

**Primary (P):** the subject's own domain and documented properties — site, docs, changelog,
status page, official blog, public ATS board, GitHub org, verified app-store listing — plus
regulatory filings (SEC/EDGAR and equivalents).

**Attributed (A)** — we confirmed **at least four** of the following, and the report shows
*which*. This is a list of things we found, not a list of things a page owes anyone:

- An identified author
- A publication date, and revision dates where shown
- Published ownership, masthead, or editorial policy
- A published corrections policy
- Sources cited and linked
- An identifiable publisher with contact details
- A multi-year publishing history on the topic
- Independently written rather than syndicated (§6)

**Unattributed (U):** we confirmed some of the above but fewer than four, and found nothing we
could not reconcile. A dated, source-citing post on a personal blog sits here. It is **usable
and labelled** — the class says we could not attribute it, which is a statement about our
confirmation, not about its accuracy.

**Not used (N)** — one of exactly two situations, and the report says which:

1. **"We were unable to verify this source against our criteria."** Applies when we could not
   confirm authorship, dating, publisher identity or sourcing; when a page appears to be a
   syndicated copy (we cite the origin instead); when a ranking order could not be tied to a
   stated methodology; when a domain is one where we have not previously been able to confirm
   attribution; or when our extraction of the page did not produce usable text.
2. **"This source carries information we could not reconcile with a primary source."** Applies
   when a value on the page differs from the subject's own current published value. **We do
   not claim the source is wrong.** Both values are shown with their dates and links, and the
   reader decides:

   > This page states $6.00/user/month (page undated). Shortcut's own pricing page states
   > $8.50/user/month as of 2026-07-31. We were unable to reconcile these, so we did not use
   > this figure. [Both sources]

   A primary source must be **more recently fetched** than the secondary one for this to apply
   — otherwise a legitimately recent price change would look like an error on the other page.
   Regional and currency variance (P18) is checked before this rule fires.

### 3.2.2 What we did not use is disclosed, never silently dropped

Every report states what it found and did not use. Collapsed by default:

> **3 sources found and not used.** [Show what we found]

Expanded:

> `example.com/best-project-tools-2026` — mentions Shortcut pricing
> **Not used:** we were unable to verify this source against our criteria — we could not
> confirm an author, a publication date, or cited sources. It also states $6.00/user/month,
> which we were unable to reconcile with Shortcut's own current pricing page ($8.50, fetched
> 2026-07-31).
> [Show what we found] · [What are our criteria?](/methodology)

**Three rules keep this from becoming a liability of its own:**

1. **We describe our own confirmation, never the publisher.** The report never states that a
   site is unreliable, low-quality, or wrong. Those are reputational claims about a third
   party and are not ours to make. Every sentence is about what *we* could and could not
   confirm (P23).
2. **Both figures, both dates, both links.** Where values differ we show them side by side
   rather than adjudicating. A reader who follows both links may reasonably conclude we were
   the ones who got it wrong — that possibility has to remain visible.
3. **"Show what we found" is quarantined.** It opens in a clearly marked panel: never in the
   report body, never in the PDF, never in a chart or matrix cell, never in a shared or public
   report, and never indexable. It is visible to the person who ran the analysis, because they
   asked — not published to the world on our authority.

### 3.2.3 The strictness setting — the reader chooses

One setting, three values, per account with a per-analysis override:

| Setting | Includes | For |
|---|---|---|
| **Primary sources only** | P | Due diligence; anything going into a decision or a document |
| **Primary + attributed** ← *default* | P, A | Everyday use |
| **Include unattributed sources** | P, A, U | Thin-footprint subjects where little is published |

Class **N is not used at any setting.** That is what "automatically ignore" means; the setting
governs U, not the two not-used situations.

The setting is **recorded on the analysis and printed on the report and the PDF** — *"Sources:
primary + attributed"* — so a shared report is reproducible and a reader knows which lens
produced it. Evidence strength (§3.5) recomputes per setting, so a permissive report does not
inherit a strictness it did not earn.

Frictionless posture, consistent with BYOK: **nobody is asked about this to get a report.**
The default is sensible, the control lives in `/account`, and it surfaces contextually only
when a section is empty under the current setting — *"2 unattributed sources mention pricing.
Include them?"*

### 3.2.4 Where each class may appear

| Surface | P | A | U | N |
|---|:-:|:-:|:-:|:-:|
| Pricing table values, feature matrix cells | ✓ | marked "secondary" | ✗ | ✗ |
| Charts (cost-at-scale, velocity, ratings) | ✓ | dashed/hatched, keyed | ✗ | ✗ |
| Report body claims | ✓ | ✓ marked | ✓ marked "attribution not confirmed" | ✗ |
| One-page executive PDF | ✓ | ✗ | ✗ | ✗ |
| Corroboration counting (§6) | ✓ | ✓ | ✗ | ✗ |
| Shared / public report pages | ✓ | ✓ | ✓ marked | ✗ |

The executive one-pager is primary-only by design. It is the artifact that gets forwarded,
screenshotted, and pasted into decks, and it should carry only what the vendor itself
publishes.

### 3.2.5 How we describe sources — the language rules

These are **review-blocking** ([CODING_QUALITY.md](CODING_QUALITY.md) §10.3) and apply to every
surface: report body, exclusion notices, hover cards, PDF endnotes, emails, and the KB.

**The rule in one line: the subject of the sentence is us, not them.** We report the limits of
our own verification. We never characterise a publisher.

| Never write | Write instead |
|---|---|
| "This site is not reputable" | "We were unable to verify this source against our criteria" |
| "The page lacks an author and a date" | "We could not confirm an author or a publication date" |
| "Low-quality source" / "content farm" | "We could not confirm attribution for this source" |
| "This source is wrong / outdated / false" | "We were unable to reconcile this with the vendor's own current page" |
| "Fails our criteria" | "We were unable to confirm the signals we look for" |
| "Excluded" / "rejected" / "blocked" | "Not used in this report" |
| "Untrustworthy claim" | "A claim we could not corroborate independently" |
| "X is more expensive than Y" | "X's published price is $10; Y's is $8, as of 2026-07-31" |

Four supporting rules:

- **State what was confirmed, not what was absent.** An attributed source lists the signals we
  found. Prefer that framing even when the list is short.
- **Both sides of a discrepancy, always.** Never assert which figure is right. Show both, with
  dates and links, and say we could not reconcile them.
- **No adjectives about publishers.** No "reputable-looking", "questionable", "sketchy",
  "dubious". The class name and the confirmed-signal list carry the meaning.
- **Leave room for us to be wrong.** Wording must never imply our verification is
  authoritative. "We were unable to" is accurate and honest; "this is unverifiable" is neither.

**Field and class names follow the same rule**, because internal vocabulary leaks into
interfaces: `attribution_signals_confirmed`, `not_used_reason`, `sources_not_used` — not
`criteria_failed`, `excluded`, or `blocklist`. The domain memo of hosts where attribution
could not previously be confirmed is a **cache of our own past determinations**, not a
judgement list, and is named and treated as such.

### 3.3 The gathering pipeline (P1, P2)

```
resolve entity
   └─> canonical domain
        ├─ Structured probes (deterministic, cheap, highest yield)
        │    /pricing /plans /changelog /releases /blog /about /careers /security
        │    /docs  /status  sitemap.xml  robots.txt  llms.txt  RSS/Atom feeds
        │    public ATS boards (Greenhouse/Lever), public GitHub org
        ├─ Search-based discovery (SearXNG self-hosted; Brave API fallback)
        │    templated queries per section — see below
        └─ Off-site adapters (SourceProvider trait)
             review platforms · trade press · SEC/EDGAR for public companies
             public ad libraries · app stores
   └─> admission control → rank → cap at 8 (Rung 0) / 14 (Rung 2)
```

**Structured probes before search.** They are deterministic, cost nothing, hit primary
sources, and are far more reliable than hoping a search engine surfaces the pricing page.
Search fills gaps; it does not lead.

**Templated queries**, one small set per section, so retrieval is reproducible and auditable
rather than model-improvised: `"<name>" pricing`, `"<name>" changelog OR "release notes"`,
`"<name>" review site:g2.com`, `"<name>" raises OR funding`, `"<name>" vs`, and so on. The
query set is versioned like a prompt and recorded on the analysis, so a retrieval regression
is attributable.

**Fetching discipline** (unchanged from [ARCHITECTURE.md](ARCHITECTURE.md) §5.2): robots.txt
honoured and cached, honest user-agent with a public bot page, ≥1s per host, conditional
requests, 8s timeout, 2MB cap, SSRF guard on every URL including redirects, no paywall or
login circumvention.

### 3.4 Discovering the competitive set — the market-research step (P3)

> Summarised here; the full design — input classes, category vocabulary resolution, seed
> channels, relevance classification, and when clarifying questions fire — is in
> [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md).

When the user names one product, or describes an idea and names nobody, the competitive set
must be *derived*. It is derived by **co-occurrence across independent source classes**, never
by asking a model to recall competitors from memory — which is exactly where an 8B model
invents plausible companies that do not exist.

Five independent signals, each producing candidates:

1. The subject's own **"alternatives" / "vs" / comparison** pages — vendors name their real
   rivals.
2. **Review-platform category pages** and "compared to" modules.
3. **Search co-occurrence**: `"<name>" alternatives`, `"<name>" vs`, `"<name>" competitors`.
4. **Community threads** (Reddit, HN) asking for alternatives — noisy but unusually honest.
5. **Ad-library co-occurrence** — who bids on the same category language.

A candidate is admitted when it appears in **≥2 independent signals** *and* resolves to a live
canonical domain with a reachable homepage. Candidates are ranked by signal count, then by
category-language overlap with the subject. The top 3–5 are used; **the full candidate list
with its scores is shown in the report**, so the reader can see who was considered and
rejected. A competitive set presented without its derivation is an unfalsifiable editorial
choice.

**Existence gate:** every competitor must have a resolving domain that we successfully
fetched. This single rule eliminates the fabricated-competitor failure mode entirely.

### 3.5 Sufficiency — knowing when we have enough (P4)

Coverage is computed per section, deterministically, before anything is written:

| Section | Minimum to populate | Otherwise |
|---|---|---|
| Pricing | Vendor pricing page fetched **or** pricing stated on a fetched vendor page | `not_found`, with pages checked listed |
| Features | ≥1 vendor page naming capabilities | `not_found` + checked list |
| Recent changes | Changelog/blog/press fetched, ≥1 dated item in window | `no_public_changelog_found` (**never** "no changes") |
| Sentiment | ≥2 platforms **or** ≥8 distinct review/comment items | `thin`, with volume caveat |
| Positioning | Homepage + one of about/docs | `not_found` |

**Evidence strength** is then a computed function — not a vibe — of qualifying source count,
tier distribution, **independence-group count** (§6), and recency (§7). It appears on the
report, in the PDF, and in the schema:

`strong` ≥6 independent groups incl. ≥2 primary · `moderate` 3–5 · `thin` ≤2.

**A thin report is still delivered**, clearly marked, with the gaps named. Refusing to answer
is worse than answering with visible limits.

---

## 4. The verification pipeline

Nine levels. Levels 3–5 are the existing anti-hallucination stack
([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §2); this document adds the levels above and
below it, which is where P7, P8, P14, P15 and P17 live.

```
L0  Source admission      — is this source allowed to be cited at all?
L1  Retrieval provenance  — hash, timestamp, status, snapshot stored
L2  Extraction fidelity   — did we parse the page correctly?          ← the gap most tools miss
L3  Claim grounding       — quote exists verbatim in extracted text
L4  Type validation       — prices, dates, versions re-checked
L5  Presentation honesty  — confidence, tier, timestamps, gaps shown
L6  Independence          — corroboration counted by group, not URL
L7  Interested-party weighting — vendors on rivals, platforms we cannot treat as samples
L8  Temporal validity     — is this fact still in date for its type?
L9  Reader auditability   — can someone check it without trusting us?
```

### L0 — Source admission (P15, P8)

A page must earn the right to be cited. Rejected before extraction:

- **Classified N — not used** (§3.2.1): we could not confirm attribution signals, the page
  appears to be a syndicated copy, a ranking order could not be tied to a stated methodology,
  the host is one where we have not previously confirmed attribution, our extraction did not
  produce usable text, or the page carries a value we could not reconcile with a primary
  source.
- **Attribution signals we could not confirm**: no identified author *and* no publication date
  *and* no cited sources, with listicle structure and unattributed superlatives. Scored, not
  binary — a low confirmation score places the source in U or N and keeps it away from
  authoritative values.
- **Hosts where we have not previously been able to confirm attribution** — a cache of our own
  past determinations, grown from the daily review queue, not a judgement about those hosts.
- **Extraction quality below threshold** — nav-heavy, near-empty, or JS-shell pages.
- **Syndication detection**: byline-free copy of a press release is collapsed into the
  originating release (§6).

**The strongest rule, stated plainly: primary sources set the authoritative values.** A price
in the pricing table, a cell in the feature matrix, a point on a chart, and anything on the
executive one-pager come from the vendor's own pages. Secondary sources add, corroborate and
fill gaps — always classified, always marked, always outside the authoritative surfaces
(§3.2.4). A third-party price is presented as *what that source reports*, never as the price.

This is deliberately not the stricter "primary or nothing" rule. Silence is not neutral: a
reader who sees an empty pricing row and can find the number in ten seconds concludes the tool
is incompetent rather than careful. Reporting *"a dated, sourced article states $8.50; we could
not confirm this on the vendor's own site"* is more useful **and** more honest than saying
nothing — provided the number never enters a table, a chart, or a forwardable PDF.

### L1 — Retrieval provenance

Every fetch records URL, canonical URL, HTTP status, fetch timestamp (UTC), content hash,
ETag/Last-Modified, our request region, and the **stored extracted snapshot**. The snapshot is
what makes L9 possible: a reader can see exactly what we read, not merely where we read it.

### L2 — Extraction fidelity (P17)

**The honest limitation, stated first:** L3 proves a claim matches the *text we extracted*. If
extraction was wrong, the claim is wrong and every downstream layer passes it. This is the
single largest residual risk in the system, and QUALITY_GUARDRAILS §7 already predicts
extraction will be the dominant failure cause.

Countermeasures:

- **Deterministic parsing for the high-stakes types** — prices, dates, versions come from
  table/heading structure, not prose ([ARCHITECTURE.md](ARCHITECTURE.md) §5.4).
- **Round-trip check on numbers**: every extracted number must appear verbatim in the raw HTML
  text. A price that exists in our parse but not in the source bytes is a parser bug and is
  dropped with an alert.
- **Structural sanity**: a pricing tier without a currency symbol, a plan with no name, a
  changelog entry without a date — all rejected.
- **`extraction_quality` gates citation eligibility**, and clusters of low scores by domain go
  to the review queue as a parser bug, not a model bug.
- **Golden extraction fixtures** in CI (CODING_QUALITY §6.4) freeze HTML → expected parse.

### L3–L5 — Grounding, type validation, presentation

Unchanged; see [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §2. In summary: every claim
carries a verbatim `evidence_quote` and `source_label`; Rust checks the quote actually occurs
in the extracted text (exact → normalised → fuzzy); unmatched claims are **deleted, not
flagged**; prices, dates and versions are independently re-validated; and confidence,
tier, timestamps and gaps are shown to the reader.

**Why deterministic Rust and not an LLM fact-checker:** a model asked to check its own output
shares its own blind spots and will ratify them. Self-verification is a comfort, not a
control. The verifier is string matching and arithmetic precisely because those cannot be
persuaded.

### L6 — Independence (P14)

Corroboration is counted in **independence groups**, never in URLs. Two sources join a group
when any of:

- Near-duplicate content (SimHash below threshold on normalised text).
- A shared verbatim span above ~25 words — the classic press-release-copy signature.
- Explicit attribution to the same origin ("according to the company's announcement").
- Publication timestamps clustered within 48h of an identifiable origin release.
- Same publisher network (maintained ownership map for the majors).

A claim supported by five articles that are one press release is reported as **one** source,
and the report says so: *"5 sources, 1 independent group (company announcement)."* This is
the difference between corroboration and echo.

### L7 — Interested-party weighting (P8, P11, P12)

| Adversarial pattern | Handling |
|---|---|
| Vendor comparison page about a rival | Rendered as *"per <vendor>'s own comparison page"*; **never** counts toward corroboration; never sources a pricing or feature cell |
| Marketing superlatives ("the only", "the fastest") | Permitted **only** as attributed self-description; the modifier is stripped otherwise |
| Review platforms | Rating always shown **with review count and platform**; recency distribution noted; incentivised-review disclosure surfaced where the platform publishes one; no synthesised composite score, ever |
| Rating patterns we cannot treat as a representative sample | Distribution shape and submission timing that we cannot treat as representative lower that platform's weight for the subject, and the volume caveat says so — we do not assert that reviews were manipulated |
| Press releases | Labelled `company_announcement`, not `news`; one independence group |
| Pages where attribution could not be confirmed | Classified U or N per §3.2.1; never set authoritative values |

### L8 — Temporal validity (P13)

Every fact type carries a **shelf life**, and a fact past it is re-fetched or labelled stale:

| Fact type | Shelf life |
|---|---|
| Pricing, plan limits | 7 days |
| Feature claims | 30 days |
| Changelog / releases | 7 days |
| Review themes | 90 days |
| Funding, acquisitions | 180 days |
| Founding date, HQ | never expires |

Every claim renders its own `as_of` date. **Dates in "recent changes" must fall inside the
stated lookback window and never exceed `fetched_at`** — future-dated changes are dropped
outright, a common and highly visible failure. A cached report older than its most perishable
fact shows *"Pricing last checked 9 days ago — refresh"*.

### L9 — Reader auditability

The whole of §5.

---

## 5. Making every claim checkable by the reader (P9, P20)

This is what separates a trustworthy tool from a confident one. **The reader must never have
to take our word for anything.**

### 5.1 The footnote, in full

Every claim carries an `[S3]` marker. Hovering or tapping opens a card; the Sources section
and the PDF endnotes carry the same fields:

```
[S3]  Linear — Pricing
      https://linear.app/pricing
      Publisher: linear.app          Trust tier: 1 (primary — vendor's own page)
      Fetched: 2026-07-31 14:21 UTC  Content hash: 8f3c…a91d
      Independence group: G1 (sole member)
      Quoted: "Basic  $8 per user / month, billed annually"

      → Open the live page at this quote        (deep link, §5.2)
      → View what we saw          (stored snapshot, quote highlighted)
      → Independent archive       (web.archive.org capture)
      ⚠ This page has changed since we fetched it   (shown only when true, §5.3)
```

### 5.2 Deep links that land on the sentence

Live-page links use **URL text fragments** (`#:~:text=Basic%20%248%20per%20user`), so clicking
a footnote scrolls to and highlights the quoted sentence on the vendor's own page. Supported in
Chromium browsers and Safari; **Firefox support is partial**, so the link still opens the page
but may not scroll to the quote. The stored snapshot (§5.3) is the fallback and is always
offered alongside — verification never depends on a browser feature. **Verification should cost one click,
not a page-scan** — friction here is why people stop checking.

### 5.3 "What we saw" snapshots, and change detection at read time

Citing a URL is not enough: pages change, and a reader who finds different content concludes
we were wrong even when we were right at fetch time.

- **Archive submission is a deferred job, never inline.** The Internet Archive's save endpoint
  is rate-limited and intermittently slow; blocking a report on it would trade our own
  reliability for someone else's. Submissions queue alongside render jobs (ARCHITECTURE §5.5)
  and the citation shows an archive link once one exists.
- The **extracted snapshot** is stored with its content hash and served at
  `/a/:id/source/:label`, with the quoted span highlighted.
- On report view, a cheap conditional re-fetch of tier-1 sources compares hashes. A changed
  page shows **"This page has changed since we fetched it — see the difference"**, linking a
  diff. This turns staleness from a hidden defect into a visible, useful signal.
- **Independent archiving:** tier-1 sources for pricing and changelog claims are submitted to
  the Internet Archive's save endpoint. This matters more than it looks — it creates a record
  **we do not control**, so verification does not require trusting our snapshot.

### 5.4 Auditable negatives (P20)

"Not found in public sources" is unfalsifiable unless we show our work. Every negative renders
what was actually attempted:

> **No public pricing found for Shortcut.**
> Checked: `/pricing` (403), `/plans` (404), homepage, blog (90 days), docs, G2 listing.
> Searched: `"Shortcut" pricing`, `"Shortcut" plans cost`.
> Last attempt: 2026-07-31 14:23 UTC. **We do not estimate prices.**

The reader can repeat those exact steps in two minutes. Showing the negative space is the
strongest available evidence that the system is not guessing — and it converts our biggest
weakness into a demonstration of rigour.

### 5.5 The methodology page

Every report links `/methodology`: how sources are found, the two-axis trust model, the
independence rule, shelf lives, and what we never do. Linked from the report, the PDF, and the
KB. **A reader who wants to audit the method, not just the facts, must be able to.**

### 5.6 In the PDF

Hover cards do not print. The full PDF carries numbered endnotes with title, publisher, full
URL, fetch timestamp, short content hash, trust tier, independence group, and the quoted span
— everything needed to verify from paper.

---

## 6. Bias controls

- **P21 — blocked sources bias the report.** A competitor who blocks crawlers must not read as
  less capable. Blocked and unreachable sources are listed **in the body of the affected
  section**, not buried in Sources, and evidence strength is computed per competitor so
  asymmetric coverage is visible: *"Coverage is uneven: 7 sources for Linear, 2 for Shortcut
  (robots-disallowed)."*
- **P22 — the user's framing.** Retrieval uses templated queries built from the *resolved
  entities*, never from the user's phrasing, so "why is X better than Y" retrieves the same
  sources as "X vs Y". The synthesis prompt receives the entity list and the intent category,
  not the raw question. Evaluative framing in the input is acknowledged in the UI —
  *"Analysing X and Y on equal terms"* — rather than silently obeyed.
- **Presentation symmetry.** Every competitor gets the same sections in the same order with
  the same treatment. No "winner" is declared (§9).

---

## 7. What we do not claim — the honest limits

Stated here, on `/methodology`, and in the KB. A tool that overstates its own reliability is
committing the error it exists to prevent.

1. **We verify fidelity to sources, not truth about the world.** If a vendor publishes a false
   claim on their own site, we will faithfully report that they claim it. Attribution framing
   (§2) is what keeps that honest rather than misleading.
2. **Extraction can be wrong** (P17). The number in the report is the number we parsed. The
   snapshot link exists so you can check the parse.
3. **Absence is bounded by what we checked** (§5.4) — never by what exists.
4. **Prices vary by region, currency and tax treatment** (P18). We record our request region
   and say so; a price you see may differ.
5. **Pages are A/B tested and personalised** (P19). We captured one variant at one moment.
6. **Review data is not a representative sample** (P12).
7. **We do not predict, forecast, or estimate** anything not published.
8. **We do not assess security, compliance, or legal posture.**
9. **We are not a substitute for primary due diligence** on a decision that matters.

---

## 8. Corrections, and the legal posture (P23)

Publishing wrong claims about real companies is a liability, not just a defect.

- **"Report an inaccuracy"** on every claim → creates a KB-linked quality thread carrying the
  analysis id, claim, source, and verification status.
- **Correction SLA:** a confirmed factual error is corrected or the claim withdrawn within
  **2 business days**. Shared and comparison pages are regenerated; the previous version stays
  reachable with a visible correction notice, because silently editing a public page is worse
  than the original error.
- **Subject-of-report channel:** a company may contest a claim about itself at
  `/methodology#corrections`. Contested claims are re-verified against the cited source; if
  the source no longer supports them, they are removed. If the source *does* support them, the
  claim stays with attribution — and their statement is added.
- **Exclusion requests** from site owners are honoured within 5 business days and recorded in
  a public exclusion list.
- **Every confirmed error becomes a golden-set regression case**, so the corpus improves from
  each mistake rather than merely absorbing it.
- **No verdicts.** The product never states which product is better. The reader's constraints
  decide, and asserting a winner without knowing them is both unhelpful and the highest-risk
  sentence we could publish.

---

## 9. How each problem is addressed

| # | Problem | Mechanism |
|---|---|---|
| **P1** | Gathering method | §3.3 pipeline: structured probes first, templated search second, adapters third; polite fetching under robots, rate limits, SSRF guard |
| **P2** | Source identification | §3.2 two-axis trust model + §3.3 probe list + L0 admission control |
| **P3** | Market research | §3.4 competitive set from **≥2 independent co-occurrence signals** + existence gate; candidate list published with scores |
| **P4** | Sufficiency | §3.5 per-section coverage thresholds; evidence strength computed from independent groups, tiers and recency; thin reports shipped and marked |
| **P5** | Fact-checking | §4 nine-level pipeline; deterministic Rust, not model self-review |
| **P6** | Hallucination | L3 quote-must-exist + L4 type validation; unmatched claims **deleted**; grammar makes citations structurally mandatory |
| **P7** | Misinformation | §3.2.1 four-class model; primary sources set authoritative values; a value we cannot reconcile with a primary source is not used, with both figures shown; L8 shelf lives; L2 extraction round-trip |
| **P8** | Disinformation | §2 attribution framing; L7 interested-party weighting; comparison pages never set fact cells; rating patterns we cannot treat as representative lower that platform's weight, stated as our limitation |
| **P9** | Independent confirmation | §5 — full footnotes, text-fragment deep links, stored snapshots, content hashes, Internet Archive captures, PDF endnotes; plus §3.2.2 disclosure of what was found and not used, in our own terms |
| **P10** | Vendor bias about itself | §3.2 two-axis trust: authoritative for *claims*, weak for *truth*; attribution framing throughout |
| **P11** | Comparison pages | Labelled as the rival's own claim; not counted toward corroboration; never set a fact cell |
| **P12** | Gamed reviews | Ratings always with counts and platform; no composite score; distribution anomalies noted; themes preferred over stars |
| **P13** | Staleness | L8 per-type shelf lives; `as_of` on every claim; read-time re-check; "last checked N days ago" |
| **P14** | Circularity | L6 independence groups via SimHash, shared-span detection, timing clusters, publisher map; report states group count |
| **P15** | AI slop | Class **N** — attribution signals we could not confirm, hosts where attribution was not confirmed previously, and no access to any authoritative surface; existence disclosed in our own terms (§3.2.5), content quarantined |
| **P16** | Entity ambiguity | §3.1 resolution before fetching; disambiguation gate asks the user; rebrand/acquisition noted in header; Wikipedia for disambiguation only |
| **P17** | Extraction ≠ truth | L2 deterministic parsing, number round-trip against raw bytes, structural sanity, quality gating, golden fixtures — **and the limitation stated openly in §7** |
| **P18** | Region/currency | Request region recorded and displayed; currency and tax treatment captured verbatim; stated as a limit |
| **P19** | A/B and personalisation | Snapshot + hash + timestamp so the reader sees which variant we got; stated as a limit |
| **P20** | Auditable negatives | §5.4 — every negative lists pages checked, queries run, and time attempted |
| **P21** | Blocked-source bias | Blocked sources named in-section; per-competitor evidence strength; explicit uneven-coverage note |
| **P22** | User framing bias | Templated queries from resolved entities, not user phrasing; intent category only reaches synthesis; symmetric presentation |
| **P23** | Legal exposure | Attribution framing; no verdicts; correction SLA and public notices; subject-of-report channel; exclusion list |

---

## 10. Cost on free-tier hardware

Verification is **almost entirely deterministic**, which is what makes it affordable on four
ARM cores ([ARCHITECTURE.md](ARCHITECTURE.md) §4.4):

| Mechanism | Cost |
|---|---|
| Quote matching, type validation, round-trip checks | String ops and arithmetic — microseconds |
| SimHash independence grouping | Microseconds |
| Shelf-life and coverage computation | Trivial |
| Structured probes | HTTP only; no model involvement |
| Attribution-signal scoring | Heuristics over structure and metadata; **no model call** |
| Entity disambiguation | One 1.7B router call (~50 tokens) |
| Competitive-set ranking | Deterministic co-occurrence counting; **no model call** |

**Rigour here is nearly free, and it partly pays for itself**: preferring primary sources means
fetching fewer, better pages, which reduces prefill — the scarce resource. Source
classification is heuristics over page structure and metadata, with **no model call**. The
product's central quality mechanism is also one of its cheapest.

---

## 11. Phasing

- **Phase 1** — entity resolution with the disambiguation gate; structured probes; templated
  search; L1 provenance; per-section coverage thresholds.
- **Phase 2** — the four-class source model (§3.2.1) with the not-used disclosure, the language
  rules (§3.2.5) and the quarantine view; L2 extraction round-trip; full footnote UI with snapshots, hashes and
  text-fragment deep links; auditable negatives; `/methodology`.
- **Phase 3** — the strictness setting (§3.2.3) in `/account`, recorded on each analysis and
  printed on the report; needs accounts, so it lands with them.
- **Phase 3** — L6 independence grouping; competitive-set derivation with published candidate
  scores; corrections channel and SLA.
- **Phase 5** — L8 read-time staleness re-checks (reuses the watch infrastructure);
  Internet Archive submission.
- **Phase 7** — rating-pattern representativeness checks; publisher-ownership map;
  per-competitor coverage-asymmetry reporting.
- **Ongoing** — the attribution cache and the golden-set regression cases grow from the daily
  review queue and from every confirmed correction.
