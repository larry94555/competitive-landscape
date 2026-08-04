# ADR 0009 — No headless browser, on a 3.6% measured gap

**Status:** accepted, **provisional** — Phase 2 re-measures and may supersede this
**Date:** 2026-08-03

## Context

[ARCHITECTURE.md](../ARCHITECTURE.md) §5.5 sets out an escalation ladder for pages that build
themselves in the browser, and refuses to schedule the expensive rung on a guess:

> Phase 1 instruments two counters, because building tier 5 before knowing its size would be
> speculative work […] **If the residual is under ~5%, tier 5 is not built.**

Tier 5 is a headless browser. `ARCHITECTURE.md` budgets **~400 MB peak at concurrency 1** on
a 24 GB machine where three resident models already take ~17 GB. **Building it means taking
memory from a model** — the same trade [ADR 0005](0005-observability-on-a-24gb-box.md)
refused for a metrics stack, and it should be refused here for the same reason unless the
numbers demand it.

`crates/landscape-extract` is those two counters. It was run against 28 real pricing pages
(`docs/js-gap-sample.txt`) on 2026-08-03.

## What was measured

| | Pages | Share |
|---|---|---|
| **Tier 1** — price visible in static HTML | 24 | 85.7% |
| **Tier 2** — price recovered from embedded JSON | 1 | 3.6% |
| **Residual** — no price by either route | 3 | 10.7% |
| *Unreachable, excluded* | *1* | — |

**The 10.7% is not the number the rule applies to**, and separating it is the whole finding.
The residual contains two different things:

| Page | What it actually is |
|---|---|
| `hetzner.com/cloud` | **A genuine JS-rendered page.** No price anywhere in the bytes: no `€` amount, no price-shaped JSON key, nothing. Its prices are fetched at runtime |
| `databricks.com/company/contact` | Publishes no price. Correctly reported |
| `palantir.com/platforms/foundry/` | Publishes no price. Correctly reported |

The last two were in the sample **on purpose, as a control group**. A measurement that cannot
tell *"hidden behind JavaScript"* from *"not published"* is measuring something other than
what §5.5 asks about — and no headless browser renders a price that was never written.

**So the JavaScript-rendering gap is 1 page in 28: 3.6%.**

## Decision

**Tier 5 is not built.** 3.6% is under the ~5% threshold, and the honest-gap treatment —
already built, already how every report section renders a missing fact — stands in for it.

Tiers 2–4 ship regardless, as §5.5 always said they would. Tier 2 is implemented and
recovered 1 of the 4 non-static pages.

## Consequences

**The 400 MB stays with the models.** That is the entire point: at 3.6%, a browser would buy
one page in twenty-eight at the cost of a meaningful share of the memory the product runs on.

**Tier 2 is real but smaller than "the big one" implies.** §5.5 predicted embedded state
would be where most of the gap closes. On this sample it recovered exactly one page, because
**the gap it was meant to close barely exists** — 85.7% of pricing pages simply print their
prices in HTML. The prediction is not wrong so much as aimed at a problem that turned out to
be small.

**The residual is a reporting problem, not a rendering one.** Two of three residual pages
publish no price at all, which is precisely the case
[FACT_CHECKING.md](../FACT_CHECKING.md) §5.4's auditable-negative treatment exists for. The
work that closes those is not a browser; it is saying "this company does not publish a
price" and listing what was checked.

## How much to trust this

**Twenty-eight pages, chosen by one person, is enough to tell 40% from 4% and nothing
finer.** That is the decision in front of us and it is not close, which is why the ADR is
accepted rather than deferred. It is not a market statistic and `BENCHMARKS.md` says so where
the number appears.

Specific limits, so nobody has to infer them:

- **The sample skews to companies that publish prices**, because those are the pages this
  product reads. A sample of enterprise software would find a much larger "no price"
  fraction and the same tiny JS gap.
- **Tiers 3 and 4 were not measured.** A discovered JSON API or an archive snapshot might
  well recover Hetzner, which would take the gap to 0%. The 3.6% is therefore an **upper
  bound**, and the decision is safe in the direction that matters.
- **One page's classification is doing a lot of work.** Move Hetzner and the figure moves 3.6
  points. That fragility is why §5.5 requires a re-measurement in Phase 2 rather than
  treating this as settled.

**This ADR is superseded if Phase 2's re-measurement, over more pages and with tiers 3–4 in
place, puts the gap materially above 5%.** The roadmap already schedules that check, and it
requires a written decision either way — including a written "still no", so the reasoning
survives rather than the conclusion alone.

## A note on the instrument

The control group earned its place on the first run. `databricks.com/company/contact` — a
page with no prices on it — was reported as **priced**, because the price detector matched
*"Learn professional Data and AI tools for free"* on a rule that accepted any sentence ending
in the word "free".

Without those two deliberately price-free URLs, the tier-1 count would have been quietly one
too high and nobody would have looked. **A control-group page reporting a price is how a
measuring instrument tells you it is broken**, and it is the same discipline
`landscape-golden` applies to itself.
