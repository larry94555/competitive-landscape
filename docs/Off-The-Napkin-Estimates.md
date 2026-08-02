# Landscape — Off-the-Napkin Estimates

> Which quantities we will estimate, how, on what assumptions, and — just as importantly —
> when we will say that an estimate is not possible.
>
> **Relationship to the other documents.** [FACT_CHECKING.md](FACT_CHECKING.md) governs what
> we *report*. This document governs what we *compute*, which is a different kind of claim and
> needs its own rules. [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §2.1
> defines the evidence classes; this adds the fourth.

---

## 1. Why this does not contradict "we do not invent"

Every other document in this plan says the product does not estimate. That was the right rule
against the wrong target, and the distinction matters enough to state precisely.

**What was rejected, and stays rejected: hidden estimation.** A number produced by a model
with no visible derivation, sitting in a report beside verified facts as though it had the same
standing. "This market is worth $2.4 billion" with nothing behind it. The reason that is
dangerous is not that it is a guess — it is that **nobody can tell it is a guess, and nobody
can check it.**

**What this document adds: transparent estimation.** A decomposition where every input is
named and sourced, every assumption is stated, the arithmetic is on the page, the result is a
range rather than a point, and an expert can follow the whole thing and disagree with any
single step.

That is not an assertion about the world. It is **a model with visible inputs**, and it is
*more* checkable than most prose. The product's principle was never "never compute." It was
**never assert something the reader cannot check** — and shown work is the most checkable
thing in the report.

> **The test that separates the two:** can a reader who disagrees with the answer point at the
> exact input they disagree with, change it, and see a different answer? If yes, it is an
> estimate. If no, it is a guess wearing a number's clothes, and it does not ship.

---

## 2. The fourth evidence class

[COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §2.1 defines three classes.
This adds a fourth, with its own treatment:

| Class | Definition | Treatment |
|---|---|---|
| **Observed** | Stated on a public page | Normal, cited |
| **Derived** | Arithmetic over observed facts, no judgement | Normal, operation stated |
| **Interpreted** | Judgement not present in any source | Visually distinct, labelled |
| **Estimated** ← new | Arithmetic over *assumed* inputs as well as observed ones | **Visually distinct, always a range, assumptions on the face of it, full working one click away** |

An estimate is never rendered in the same style as a fact. It carries a range, a confidence,
and a link. It never appears in the executive one-pager, which stays primary-only.

---

## 3. The toolkit

Six techniques. Each is old, well-tested, and chosen because it degrades honestly when inputs
are poor.

### 3.1 Fermi decomposition

Break the unknown into factors you can bound, even loosely. Errors in independent factors
partially cancel, so a product of five rough factors is usually far better than one confident
guess at the answer.

```
market size = (potential buyers) × (fraction who buy) × (annual price) × (purchases per year)
```

**Why it works:** if each factor is wrong by a factor of two in a random direction, the product
is typically wrong by far less than 2⁵. Decomposition converts one unbounded guess into several
bounded ones.

**The rule:** decompose until every factor is either *observed*, *available from a public
statistic*, or *boundable within one order of magnitude*. If any factor is still a free guess
after decomposition, the estimate is refused (§6).

### 3.2 Bounding, and the geometric mean

For every factor, state two numbers:

- a **lower bound you are confident is too low**;
- an **upper bound you are confident is too high**.

The point estimate is the **geometric mean**, `√(lo × hi)`, not the arithmetic mean.

**Why geometric:** these quantities span orders of magnitude, and the error is multiplicative,
not additive. If a factor is somewhere between 1,000 and 100,000, the sensible middle is 10,000
— not 50,500, which is nearly the top of the range on a log scale.

### 3.3 The Rule of Five

With **five randomly chosen samples**, there is a **93.75%** chance the population median lies
between the smallest and largest of them. `1 − 2 × (½)⁵ = 0.9375`.

This is the highest-value technique available to this product, because five samples is a cheap
thing to obtain from public data:

- Five job postings → a bound on salary bands, and therefore on payroll.
- Five customer testimonials → a bound on typical customer size.
- Five published case studies → a bound on deployment scale.

Five samples is not a small sample. It is a usable one, and knowing that is worth more than
most statistics.

### 3.4 Estimation by analogy (nearest neighbour)

Find a comparable business where the quantity **is** public, compute a ratio, and apply it.

The best comparables are **public companies in the same category**, because their filings are
authoritative and free ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.1a). Useful ratios:

- revenue per employee
- revenue per customer
- customers per review left
- employees per published customer

**Comparability must be argued, not assumed.** The report states which comparable was used and
why it is comparable — same category, similar delivery model, similar buyer. A ratio taken from
an enterprise vendor and applied to a self-serve tool is not an estimate; it is a category
error with arithmetic on top.

### 3.5 Two independent paths, always

**The single best quality check in this whole discipline.** Estimate the same quantity two ways
that share no inputs, and compare.

- Agree within a factor of ~3 → report the range spanning both, confidence *moderate*.
- Diverge by more than an order of magnitude → **report both, and say we could not reconcile
  them.** Do not average. Do not pick.

Divergence is information. It usually means one of the assumptions is wrong, and saying so is
more useful than a tidy single number.

### 3.6 Calibration, and honest width

Most people's "90% confident" ranges contain the true value about half the time. The correction
is mechanical: **widen the range until you would genuinely be surprised to be outside it**, then
widen once more.

Two rules that follow:

- **Never a point estimate.** Ever. A single number implies a precision that no napkin
  calculation has.
- **Round to the precision the method supports.** "Roughly 40,000" not "41,732". A number with
  five significant figures is claiming four more than it has earned.

---

## 4. What we will attempt

Each entry states: what it needs, how it is computed, and **when it is refused**.

### 4.1 Market size, currently — *often possible*

**Needs:** a public population statistic for the buyer, plus observed pricing.

**Method:**

```
current market  =  buyers in the population        [public statistic, cited]
                ×  fraction plausibly in the market  [bounded, assumption]
                ×  annual price                      [observed from pricing pages]
```

**Worked example** — software for small farms selling to restaurants:

| Factor | Value | Where from |
|---|---|---|
| US farms with direct-to-market sales | 130,000 | Agricultural census — cited, linked |
| Fraction selling wholesale to restaurants | 5%–25% → **11%** (geometric mean) | Assumption, stated |
| Annual price | $588 ($49/mo, observed) | Observed, three competitors |
| **Estimate** | **$4M – $19M/yr, central ≈ $8M** | |

**Refused when:** no credible public population statistic exists for the buyer. A novel
category with no measurable denominator cannot be sized, and the report says so rather than
inventing a denominator.

### 4.2 Market size, potential — *possible as scenarios, never as prediction*

Identical decomposition, with the adoption fraction varied across **stated scenarios** rather
than predicted:

| Scenario | Adoption of the addressable population | Result |
|---|---|---|
| Conservative | 2% | … |
| Central | 8% | … |
| Optimistic | 25% | … |

**These are arithmetic, not forecasts.** The report says so in those words. Nobody is claiming
the optimistic case will happen; the reader is being shown what it would be worth if it did.

### 4.3 Revenue of a specific business — *rarely possible, and deliberately restricted*

**Public companies: not an estimate.** The figure is in the filings. Report it as an observed
fact with the filing cited.

**Private companies** need at least **two** of:

- a published customer count;
- observed pricing;
- headcount from public job boards or an about page;
- a comparable public company's revenue per employee.

**Two independent paths, both required:**

```
path A:  headcount × revenue per employee (from a named public comparable)
path B:  customers × average price (customers estimated per §4.5)
```

**This is the most restricted output in the product**, and the restriction is deliberate.
Publishing a revenue figure for a named private business is the highest-exposure thing this
system could do (ROADMAP R13):

- **Order of magnitude only.** "$1M–$10M." Never "$4.2M."
- **Never in a shared report, never in a PDF, never on an indexable page.**
- **Shown only on request**, behind an explicit action in the reader's own session.
- **Framed as method, not fact**: *"Two rough calculations put this in the $1M–$10M range."*
- Refused entirely if the two paths diverge by more than an order of magnitude.

**Refused when:** fewer than two inputs are available, or no defensible comparable exists.

### 4.4 Unique visitors — *possible to within an order of magnitude*

**Needs:** a domain popularity rank ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.1a), plus
anchors.

Web traffic follows an approximately power-law distribution against rank, so a rank can be
converted to a visit range — but **the constant is not universal and must not be hard-coded.**

**Method:** calibrate locally. Take three or more sites of known traffic *in a similar band*
— public companies that disclose, or sites publishing their own analytics — fit the local
curve, and interpolate. Report the result as a band, never a number.

> Roughly 20,000–90,000 visits a month, from a domain rank of 412,000 [Cloudflare Radar, 1 Aug]
> and three nearby sites that publish their own figures. Rankings are measured; visit counts
> are inferred from them, so treat this as an order of magnitude.

**Refused when:** no rank is available, or no anchors exist within two orders of magnitude of
the target.

**Note the change of position.** Earlier drafts excluded visitor estimates outright, on the
grounds that they are modelled while ranks are measured. That reasoning was right about
*unlabelled* estimates and wrong as a blanket rule. With the method shown and the range wide,
the reader can judge it — which is the standard everything else in this product is held to.

### 4.5 Number of customers — *often possible, with wide bounds*

**Needs:** one of the following, and preferably two.

| Path | Method | Typical width |
|---|---|---|
| Published count | Not an estimate — an observed fact | — |
| Review counts | total reviews ÷ review rate (bounded 0.5%–5%, geometric mean ≈ 1.6%) | ~1 order of magnitude |
| Logos / case studies | A **lower bound only**, never a total | — |
| Headcount | employees × customers-per-employee from a comparable | ~1 order of magnitude |

The review-count path costs nothing extra: the review data is already gathered for the
sentiment section.

**Refused when:** no review volume, no published logos, and no headcount.

### 4.6 Growth — *not on a first report; possible after monitoring*

Growth needs two observations separated by time, and a first analysis has one.

But the watch infrastructure creates the second observation. After a few months of monitoring,
several quantities become genuinely measurable rather than estimated:

- change in open roles (a real hiring-trend signal);
- rate of review accumulation (a proxy for customer growth);
- pricing changes;
- release cadence over time.

**This is worth saying to the reader**, because it is a real reason to keep watching: *"Growth
cannot be estimated from a single look. Watch these pages and it becomes measurable in about
three months."* An honest limitation that points at the product's retention hinge.

### 4.7 Sanity checks that cost nothing

Every estimate runs three cheap tests before it is shown:

1. **Bounds check** — is the result physically possible? A market larger than the buyer
   population times the highest observed price is arithmetic that went wrong somewhere.
2. **Consistency check** — do the estimates agree with each other? Estimated customers ×
   observed price should land near estimated revenue. If not, say so.
3. **Reversal check** — work backwards. If the market estimate is right, does the implied
   number of customers per competitor look plausible against their headcount?

A failed check does not silently adjust the number. It is **reported alongside it**.

---

## 5. What we will never estimate

| Never | Why |
|---|---|
| Profitability, margins, burn, runway | No public input exists at any decomposition. There is nothing to estimate *from*. |
| Churn, retention, CAC, LTV | Same — internal by nature |
| Valuation | Depends on private terms; published multiples do not transfer |
| Market share | Requires both a numerator and a denominator we do not have |
| Anything about a named individual | Privacy posture ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §6) |
| **Future values of anything** | This product does not forecast. Scenarios (§4.2) are arithmetic on stated assumptions, which is a different object. |

The rule underneath all of these: **if decomposition does not terminate in inputs that are
observed, publicly published, or defensibly bounded, there is no estimate — there is a guess.**

---

## 6. When we refuse, and how we say so

The refusal is as valuable as the estimate, and it is written to be useful rather than
apologetic. Consistent with the "what we checked" treatment used everywhere else:

> **We cannot estimate the size of this market.**
> Sizing needs a count of potential buyers from a public statistic, and we could not find one
> for this category. We looked at: national business registers, two industry association
> membership pages, and the census categories closest to this description.
>
> **What would make it possible:** any published count of businesses of this kind. If you know
> of one, the calculation is four lines and we will show it.

Three properties that make this good rather than a shrug:

1. It names **what was missing**, not just that something was.
2. It names **what was tried**, so the reader can judge whether we looked properly.
3. It tells the reader **what one thing would unlock it** — which is often something they
   already know.

---

## 7. How estimates are presented

Three layers, and every layer is reachable from the one above.

### Layer 1 — the headline, on the report

> **Roughly $4M–$19M a year.** *Off-the-napkin estimate, not a measurement.*
> Central figure around $8M. Based on 130,000 US farms selling direct, of which we assume
> 5%–25% sell wholesale to restaurants, at the $588/year price the three competitors publish.
> **[See the working]**

Assumptions are on the face of it. A reader who thinks 25% is far too high knows immediately
that they disagree, and with what.

### Layer 2 — the working, one click away

A page an expert can check line by line:

- every factor, its bounds, and where each came from;
- every source cited and linked, exactly as in the main report;
- the arithmetic, written out;
- **a sensitivity table** — what happens to the answer if each factor moves;
- the second independent path and how far it agreed;
- the sanity checks and their outcomes.

### Layer 3 — the one line that is most useful

Every estimate ends with the input that dominates its uncertainty:

> **What would sharpen this most:** the fraction of small farms selling wholesale. It spans a
> factor of five and drives almost all the width above. Every other input is within a factor
> of two.

This is the most actionable sentence in the whole estimate, and often more valuable than the
number. It tells the reader the **one fact worth going to find** — and a reader who works in
the industry may already know it.

---

## 8. Quality gates

Applied to every estimate before it is shown. These belong in the golden set
([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §3.2) alongside the citation gates.

| Gate | Requirement |
|---|---|
| **Never a point** | Every estimate is a range. No exceptions. |
| **Every input accounted** | Observed and cited, from a public statistic and cited, or an assumption and labelled. No silent inputs. |
| **Assumptions visible** | On the report itself, not only in the working |
| **Two paths where possible** | And divergence reported rather than averaged away |
| **Sanity checks run** | And failures reported alongside the number |
| **Rounded honestly** | Significant figures the method supports, no more |
| **Never in the executive PDF** | That artifact is primary-only |
| **Recomputed, not cached** | An estimate whose inputs have changed is recomputed, and carries its own `as of` date |
| **Refusal is a valid output** | And a well-formed one (§6) — not an error |

**Evaluation.** The golden set gains subjects where the true value is *known* — public
companies whose revenue, headcount and traffic are published. Estimates are run against them
blind, and scored on whether the **true value falls inside the stated range**. That is the only
honest test of an estimator: not whether the central figure is close, but whether the range is
calibrated. A method whose 90% ranges contain the truth 50% of the time is not producing
estimates; it is producing false confidence, and the fix is wider bounds.

---

## 9. Consequences elsewhere

| Document | Change |
|---|---|
| [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) | Fourth evidence class; a new optional section for estimates; the exclusion list changes from "no estimates" to "no *hidden* estimates" |
| [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) | Calibration gates in the golden set; known-value subjects added |
| [FACT_CHECKING.md](FACT_CHECKING.md) | Public statistics become a source class of their own — census and register data are primary for population counts |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Estimates are **computed in Rust, not generated by the model** — arithmetic must be arithmetic. The model's only role is choosing the decomposition and writing the prose around it. |
| [ROADMAP.md](ROADMAP.md) | R13 tightened: revenue estimates for named private businesses are the most exposed output and are restricted per §4.3 |

**The architectural point deserves emphasis.** The model may propose *which* decomposition
fits and may write the sentences; **it must never do the multiplication.** Arithmetic performed
by a language model is arithmetic that can be silently wrong, and the entire value of this
document rests on the working being correct. Estimates are computed in code, from a structured
set of factors, and the model sees the result rather than producing it.

---

## 10. Phasing

- **Phase 2** — the estimate framework in Rust: factor structures, bounds, geometric means,
  sensitivity, sanity checks, the working page, and the refusal template. Plus §4.5 customers
  from review counts, which needs no new data.
- **Phase 3** — §4.1/§4.2 market sizing, once public-statistics adapters exist.
- **Phase 5** — §4.6 growth, once monitoring history has accumulated.
- **Phase 7** — §4.4 visitors, which needs the anchor set; and §4.3 revenue, which should be
  last because it is the most exposed.

**Build the refusal path first.** It is the output that will fire most often at the start, and
a well-formed "we cannot estimate this, here is what was missing, here is what would unlock it"
is more valuable — and considerably safer — than a poorly-bounded number.
