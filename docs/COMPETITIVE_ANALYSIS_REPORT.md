# Landscape — The Competitive Analysis Report

> What a state-of-the-art competitive analysis contains, what Landscape can honestly produce
> from public data, and how it should be visualised.
>
> This document specifies **report content**. [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 specifies
> the schema that encodes it; the schema must be expanded to match this document (§9).

---

## 1. What the discipline actually produces

Competitive analysis is a mature practice with a standard set of artifacts. Before deciding
what Landscape generates, it is worth writing down what a good analyst produces and who reads
it, because the product has so far been specified as "seven sections of prose" — which is
closer to a competitor *summary* than a competitive *analysis*.

| Artifact | What it answers | Typical reader |
|---|---|---|
| **Competitive landscape / market map** | Who is in this space, and how do they cluster? | Founder, product lead |
| **Feature comparison matrix** | Who does what? Where are the gaps? | Product, sales |
| **Pricing & packaging comparison** | How is value captured? Where are we cheap or expensive? | Pricing, finance, sales |
| **Positioning map (perceptual map)** | Who occupies which position, and what space is empty? | Marketing, strategy |
| **Strategy canvas / value curve** | Which factors does each player compete on, and how hard? | Strategy |
| **SWOT (per competitor, or focal)** | What is the shape of the threat? | Everyone |
| **Battlecards** | What do I say when I meet them in a deal? | Sales |
| **Timeline / velocity analysis** | Who is shipping, and how fast? | Product |
| **Sentiment & review analysis** | What do customers actually complain about? | Product, marketing |
| **Hiring & investment signals** | Where are they putting money? | Strategy |
| **Porter's Five Forces** | Is this industry structurally attractive? | Strategy, investors |
| **Win/loss analysis** | Why do we actually lose? | Sales, product |
| **Share of voice / traffic** | Who has attention? | Marketing |

Of these, three are the backbone that essentially every real competitive analysis contains —
**the feature matrix, the pricing comparison, and the positioning view** — and all three are
fundamentally *visual or tabular*. A competitive analysis delivered as unbroken prose is not
following best practice. That is the gap this document closes.

---

## 2. What Landscape can honestly produce

The product's constraint is public data with citations, and its central discipline is that
absence is reported rather than filled in. That rules some artifacts in and others out.

### 2.1 Three evidence classes

Every element in a Landscape report falls into exactly one class, and **the class determines
the visual treatment**:

| Class | Definition | Example | Treatment |
|---|---|---|---|
| **Observed** | Stated on a public page; quotable | "Basic is $8/user/month" | Normal. Cited `[S2]`. |
| **Derived** | Arithmetic or counting over observed facts; no judgement | "At 25 seats that is $200/month"; "6 changelog entries in 90 days" | Normal. Cites the inputs, and the operation is stated. |
| **Interpreted** | Requires judgement not present in any source | "They are moving upmarket" | **Visually distinct, labelled *interpretation*,** and must cite the observed facts it rests on. |

This mirrors the existing SWOT treatment ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4.1) and extends
it to every chart. **A positioning map is interpretation with axes; it must look like
interpretation.**

### 2.2 The rule that most competitive tools get wrong

In a feature matrix, **absence of evidence is not evidence of absence.** Most tools render an
empty cell as ✗, which silently manufactures a claim. Landscape uses five cell states:

| State | Meaning | Requires |
|---|---|---|
| **✓ Yes** | The vendor states they have it | A citation |
| **✗ No** | The vendor states they do *not* have it | A citation |
| **◐ Partial** | Present with a stated limitation | A citation + the limitation |
| **? Not found** | We looked and found no public statement | **The list of pages checked** |
| **▲ Per competitor** | Claimed by a rival's comparison page, not the vendor | Marked as an interested-party claim, never presented as neutral |

That last state exists because vendor comparison pages are systematically unreliable and are a
major source of error in automated competitive tooling.

Every populated cell also carries its **source class** — P (primary), A (attributed) or U
(unattributed), per [FACT_CHECKING.md](FACT_CHECKING.md) §3.2.1. Primary-sourced cells render
normally; attributed-source cells carry a visible marker; sources we did not use never populate
a cell at all. The same rule governs charts: an attributed-source series is drawn dashed or
hatched and keyed as such, and the executive one-pager is primary-only.

The wording of every source annotation follows the language rules in
[FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5: annotations describe what **we** were able to
confirm, never what a publisher is or lacks.

---

## 3. The report specification

Nine sections. The current seven, plus the two that a real competitive analysis cannot omit.

### Section 0 — Header
Subject, comparison set, generated-at (UTC), evidence strength (strong/moderate/thin), source
count, model and prompt version, inference provider, disclaimer — plus, when the set was
derived rather than given, the **interpretation line**: the resolved category, the constraints
applied, the counts of direct/adjacent/substitute competitors, and an editable link to the
full candidate list ([COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §5.5).

### Section 1 — Positioning
- **Content:** each competitor's self-described category, target segment, stated
  differentiators, and the exact category language they use (quoted).
- **Visual:** *Positioning map* (interpreted) — see §4.7.
- **Sourcing:** homepage, about, and any "why us" page. Category language is quoted verbatim.

### Section 2 — Pricing & packaging  ← *the most-read section*
- **Content:** tier table (name, price, period, seat/usage basis, notable limits, as-of date);
  free tier and trial presence; whether enterprise pricing is public; observed pricing changes.
- **Visuals:** *Pricing tier table* (observed) and *Cost-at-scale curve* (derived) — §4.2.
- **Sourcing:** parsed deterministically from pricing pages
  ([ARCHITECTURE.md](ARCHITECTURE.md) §5.4). Never generated.

### Section 3 — Feature comparison matrix  ← **new, and the backbone**
- **Content:** capabilities (rows) × competitors (columns), using the five cell states above.
  Capabilities are derived from the union of what the competitors themselves name, not from a
  fixed taxonomy — a fixed taxonomy imports our assumptions into their positioning.
- **Visual:** *Feature matrix* (observed) — §4.1.
- **Also:** *stated* gaps only. A capability nobody mentions is `? Not found`, never a gap.

### Section 4 — Recent public changes
- **Content:** dated events (release, pricing, positioning, funding, personnel, policy) within
  the lookback window, with a coverage note naming competitors that publish no changelog.
- **Visuals:** *Shipping velocity* (derived) and *Event timeline* (observed) — §4.3, §4.4.

### Section 5 — Review & sentiment themes
- **Content:** recurring themes with valence and frequency, representative quotes, platforms
  covered, and an explicit volume caveat.
- **Visuals:** *Theme sentiment bars* (derived) and *Rating comparison* (observed) — §4.5, §4.6.
- **Never:** a synthesised numeric score. Ratings are reported with their source or not at all.
- **Unmet-need mining — the highest-value part of this section.** Mine **2–3 star reviews
  specifically** for recurring *"I wish it did X"* / *"the only thing missing is…"* patterns.
  Unlike 1-star reviews (often support incidents) and 5-star reviews (often marketing), the
  middle band is where users describe what a product they otherwise like fails to do.
  Each theme is reported as an observed, quoted, counted pattern — *"Feature gap: 7 reviewers
  across 2 platforms mention wanting bulk export [S9][S11]"* — never as an inferred market
  opportunity.

  **Why this matters strategically:** it is the evidenced version of what "underserved market"
  tools assert without evidence. A counted, quoted, cited unmet need is a defensible finding;
  "this market is underserved" is an unfalsifiable claim that costs a founder a year if wrong.
  This is the one competitor technique worth adopting, and adopting it *honestly* is the
  differentiator.

### Section 6 — Market emphasis (strategy canvas)  ← **new**
- **Content:** how prominently each competitor markets each competing factor — measured from
  the prominence of that factor in their own public copy (headline, nav, section headings,
  homepage word share).
- **Visual:** *Value curve* — §4.8.
- **The important framing:** this plots **marketing emphasis, not capability.** Emphasis is
  observable from public copy; capability is not. Every comparable tool that plots "how good
  each product is at X" is inventing a rating. Plotting emphasis is both more honest and, for
  positioning work, more useful.

### Section 7 — SWOT-style summary
Unchanged. Strengths and Weaknesses cite observed facts; Opportunities and Threats are
explicitly labelled interpretation.

### Section 7A — Operating signals *(where evidence exists)*
- **Content:** per competitor, dated public activity — release cadence, last public update,
  open roles by function, documentation freshness, status-page history, published pricing
  changes. Longevity from first web-archive capture alongside any self-stated founding year.
- **Financial standing:** publicly announced funding, acquisitions and, for public companies,
  figures from regulatory filings. **Private-company revenue is never estimated** — the report
  states that it is not disclosed.
- **No composite health score.** Combining non-comparable signals into one number would be
  interpretation dressed as measurement. See
  [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §7.

### Section 8 — Sources
Every source with label, URL, title, host, trust tier, fetched-at, extraction quality, and
status — plus an **exclusions list** with reasons (robots-disallowed, unreachable, paywalled).

### Optional sections (paid tiers, when evidence exists)

- **Hiring signals** — open roles by function over time, from public job boards. Aggregate
  only; never individual people ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §6).
- **Funding & milestones** — publicly announced rounds and dates.
- **Battlecard** — a one-page sales-facing view: their pitch, their weaknesses as stated by
  reviewers, their pricing, and the three questions that expose the gaps. Derived entirely
  from sections 1–7.

---

## 4. The chart catalogue

Each entry states what it shows, what data it needs, its evidence class, and how it renders.
**Every chart carries the same citation obligations as prose.**

### 4.1 Feature matrix — *observed*
Grid of capability × competitor with the five cell states. Cells are clickable to their
citation; `? Not found` expands to what was checked.
**Not a chart** — a styled table, in HTML and in Typst. Highest-value visual in the report.

### 4.2 Cost-at-scale curve — *derived*
Monthly cost (Y) against seat count (X: 1, 5, 10, 25, 50, 100), one line per competitor,
step-shaped where tiers change. Only plotted where per-seat pricing is public; competitors
with "contact sales" appear as a labelled gap, not an estimate.
**Why it matters:** headline per-seat prices routinely invert at scale because of tier
minimums and included-seat bundles. This is pure arithmetic over published numbers — the
highest-value, lowest-risk chart in the product.

### 4.3 Shipping velocity — *derived*
Grouped bar chart: changelog/release entries per month over 12 months, per competitor.
Competitors without a public changelog are shown as an explicit "no public changelog" row —
never as zero, which would read as "they ship nothing."

### 4.4 Event timeline — *observed*
Horizontal timeline of dated public events, colour-coded by kind, one lane per competitor.

### 4.5 Theme sentiment — *derived*
Diverging stacked bars: one row per theme, negative left, positive right, width proportional
to mention count. Sample size printed on every row, because a theme with four mentions and one
with four hundred must not look alike.

### 4.6 Rating comparison — *observed*
Bar chart of published ratings per platform, with **review count printed beside every bar**.
A 4.8 from 12 reviews and a 4.4 from 3,000 are not comparable, and a bare bar chart implies
they are.

### 4.7 Positioning map — *interpreted*
2D scatter. **Rule: at least one axis must be observed** (published entry price, published
seat minimum, free-tier presence). The other may be a stated-segment ordinal. Rendered in the
interpretation style with the axis definitions printed on the chart. If no axis can be
grounded in observed data, **the chart is omitted** and the section says why.

### 4.8 Value curve (strategy canvas) — *derived, presented as interpretation*
Competing factors along X (drawn from the union of the competitors' own marketing language),
emphasis 0–100 on Y, one line per competitor. Emphasis is computed from prominence in their
public copy, and the method is stated on the chart.

### 4.9 Hiring signal — *observed* (optional section)
Stacked bars of open roles by function over time. Aggregate counts only.

### Charts deliberately **not** used

| Chart | Why not |
|---|---|
| **Radar / spider** | The most-requested and least defensible CI chart. Enclosed area scales with the *square* of the values, the shape changes entirely with arbitrary axis ordering, and more than three overlaid series is unreadable. The value curve (§4.8) shows the same data honestly. |
| **Market-share pie** | We have no reliable public market share. A pie implies a measured denominator we do not have. |
| **Traffic / visitor estimates** | Third-party estimates are paid, modelled, and frequently wrong. Plotting them lends borrowed precision. |
| **Word clouds** | Near-zero information density; size encodes frequency badly and encodes nothing else. |
| **Trend lines, forecasts, regressions** | Extrapolation is invention. The product does not predict. |
| **Dual-axis charts** | Two Y-scales let any two series be made to look correlated. |
| **Truncated-axis bar charts** | Bars must start at zero. Non-negotiable. |

---

## 5. What is included, and what is not

### Included

**Observed:** published pricing, tiers, limits, free tiers and trials · self-described
positioning and category language · stated features and stated limitations · changelog and
release entries with dates · public announcements (funding, partnerships, policy) · published
review ratings and counts · review text themes and quotes · public job postings in aggregate ·
documentation and status pages · robots/accessibility status of every source.

**Derived:** cost at seat counts · shipping cadence · theme frequency · marketing emphasis ·
tier-over-tier price deltas · time since last public change.

**Interpreted (labelled):** SWOT opportunities and threats · positioning-map placement ·
strategic implications of observed changes.

### Not included, and why

| Excluded | Reason |
|---|---|
| Market share, TAM/SAM/SOM | Not reliably public; analyst estimates are modelled, not observed |
| Revenue, ARR, burn, margins (private companies) | Not public |
| Traffic, visitor counts, conversion rates | Third-party estimates only |
| Customer counts and named logos beyond what is published | Frequently stale or promotional |
| Win/loss data | Internal by definition |
| NPS or satisfaction scores not published by the vendor | Would have to be invented |
| Individual employees, org charts, LinkedIn profiles | Public-company-behaviour analysis, not people-tracking ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §6) |
| Anything behind a login, paywall, or robots-disallowed path | Public data only |
| Roadmaps and unannounced plans | Speculation |
| Forecasts, predictions, "likely to" | The product does not predict |
| Security, compliance, or legal assessments | Requires expertise and liability we do not have |
| Head-to-head "winner" verdicts | The user's context decides; asserting a winner without knowing their constraints is a confident guess |

**Stated once, plainly:** this is good-enough public intelligence, not verified enterprise
competitive intelligence. It replaces two hours of tab-opening, not an analyst.

---

## 6. Report tiers

| Tier | Format | Contents |
|---|---|---|
| **Executive** | 1-page PDF | Header, positioning summary, pricing table, **cost-at-scale chart**, top 5 matrix rows, 3 recent changes, SWOT grid, disclaimer |
| **Standard** | Web | All 9 sections, 5–7 visuals, inline citations |
| **Full** | PDF | Standard plus every matrix row, evidence quotes, full source list and exclusions |
| **Battlecard** | 1-page PDF (paid) | Their pitch, cited weaknesses, pricing, three exposing questions |

The executive PDF carries **one chart** — cost-at-scale — because it is the one a reader acts
on. More charts on a one-pager is decoration.

---

## 7. Reassessing the charting decision

The original decision ([ARCHITECTURE.md](ARCHITECTURE.md) §2.1) was *"Charts: none in v1.
Reports are text + tables. Do not add a chart library to look serious."*

**The reasoning was sound; the conclusion was wrong.** It correctly rejected decorative charts,
then over-generalised to reject all visualisation — in a product whose entire category
communicates through matrices and comparisons. A competitive analysis tool without a feature
matrix and a pricing comparison is not competitive analysis. The decision is reversed.

### 7.1 But a JavaScript charting library is still the wrong answer

Rejecting **Recharts, Chart.js, D3, visx, Nivo** — for four reasons that have nothing to do
with taste:

1. **It solves only one of four surfaces.** Charts must appear in the React app, the
   **Typst PDF**, the server-rendered shared/comparison pages, and (later) alert emails. A JS
   library serves one. We would end up with two chart implementations that drift — precisely
   the duplication [CODING_QUALITY.md](CODING_QUALITY.md) §1 calls a defect.
2. **It breaks the one-source-of-truth principle.** The report schema is defined once in Rust
   and generates everything else ([ARCHITECTURE.md](ARCHITECTURE.md) §2.5). Charts should be
   the same: data in the schema, one renderer.
3. **It breaks replay determinism.** `replay` (CODING_QUALITY §9.2) requires a report to
   re-render byte-identically. Client-side charts with animation and layout-dependent sizing
   do not.
4. **Bundle weight for no benefit.** These charts are static. Nothing hovers, zooms, or
   streams. Shipping a plotting engine to draw fixed SVG is pure cost.

### 7.2 The recommendation: server-side SVG, generated in Rust

A `landscape-charts` crate emits **static, themed SVG** from the report data. One renderer,
four surfaces:

```
report data (schema) ──> landscape-charts ──> SVG string
                                               ├─> React: dangerouslySetInnerHTML / <img>
                                               ├─> Typst PDF: native SVG embed
                                               ├─> SSR pages: inline
                                               └─> Email: PNG via resvg (clients dislike SVG)
```

**Why SVG:** vector-crisp in print and on retina, styleable by CSS to match the report,
text stays selectable and searchable, small, and trivially deterministic.

**Hand-rolled emitters, not a plotting crate.** The catalogue is **eight fixed chart types**
with fixed layouts, not a general plotting problem. `plotters` (the mature Rust option, with
an SVG backend) is built for arbitrary charts with generic axes, scales and legends — most of
which we would fight to get the typography and theming to match Typst. Per
[CODING_QUALITY.md](CODING_QUALITY.md) §3.4, *no dependency > 50 lines of our code > a small
focused crate*: roughly 400–600 lines of SVG emitters buys complete control and zero
adaptation cost.

**The revisit trigger** (rule of three, CODING_QUALITY §3.3): if a **ninth** chart type is
needed, or if axis/scale logic starts being duplicated across emitters, adopt `plotters` then.
That is a real trigger, not a hedge.

**`resvg`** (Rust, MIT) rasterises SVG → PNG for email. One small crate, one job.

### 7.3 Accessibility and text parity — required, not optional

Every chart ships with:

- `role="img"`, a `<title>` and a `<desc>` stating the finding, not the chart type
  ("Linear costs less than Jira above 25 seats", not "line chart of pricing").
- **A data table containing the same numbers**, visually hidden on the web and printed in the
  full PDF. The chart is a faster path to the data, never the only path.
- Colour that is not the sole encoding — line style and direct labels carry it too, so the
  charts survive greyscale printing and colour-vision deficiency.
- Citations on every plotted series, exactly as for prose.

### 7.4 Cost on free-tier hardware — charts are *cheaper* than prose

The point that settles it. Charts render from **already-extracted structured data**, so they
consume **zero model tokens**. A feature matrix conveys in one grid what would otherwise be
several hundred generated tokens of comparison prose — and on Rung 0, where generation is the
scarce resource ([ARCHITECTURE.md](ARCHITECTURE.md) §4.4), **shifting comparison from prose to
tables and charts reduces the generation budget while improving the output.**

SVG emission is microseconds of CPU and deterministic. This is the same lever as
deterministic-first extraction (§5.4): move work out of the model, and get something more
accurate *and* faster.

---

## 8. Honesty rules for charts

Charts persuade faster than prose, which makes them a faster route to misleading someone.
These are review-blocking:

1. **Bars start at zero.** Always.
2. **Source class is visible on every plotted series** — primary solid, attributed dashed,
   unattributed never plotted ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.4).
3. **Sample size is printed** wherever a chart aggregates counts or ratings.
4. **Missing data is drawn as missing** — a labelled gap, never zero, never interpolated.
5. **Interpreted charts look interpreted** — same visual treatment as the SWOT section, with
   the method printed on the chart.
6. **Every series carries its citation.**
7. **No chart without a data table.**
8. **A chart with fewer than three data points is a sentence.** Render the sentence.
9. **If the data cannot support the chart, omit the chart** and say what was missing — the
   same "what we checked" discipline the rest of the product uses.

---

## 9. Consequences for the rest of the design

This document expands the product beyond what the current schema encodes. Each of these is an
ADR-worthy change ([CODING_QUALITY.md](CODING_QUALITY.md) §8.1):

- **Report schema** ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4) gains `FeatureMatrix`,
  `MarketEmphasis`, and a `Chart` payload per section. Since one schema drives the API, the UI
  types and the decoding grammar, this is the highest-impact change in this document.
- **New crate** `landscape-charts`, plus `resvg` for email rasterisation.
- **Extraction** must parse capability lists and per-tier limits structurally, not just prose —
  the feature matrix is only as good as the parser (ARCHITECTURE §5.4).
- **Typst templates** gain chart embedding; the executive one-pager is redesigned around the
  cost-at-scale chart.
- **Golden set** gains matrix and chart-data assertions — a chart plotting the wrong number is
  a fact error, and must be caught by the same gates as a hallucinated price.
- **Phasing:** feature matrix and pricing charts in **Phase 2** (they are the deliverable);
  velocity, sentiment and timeline in **Phase 5** (they need watch history); positioning map,
  value curve and battlecard in **Phase 7**.
