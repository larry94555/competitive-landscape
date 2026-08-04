# ADR 0010 — Spend the source cap on breadth, not confidence

**Status:** accepted
**Date:** 2026-08-03

## Context

[FACT_CHECKING.md](../FACT_CHECKING.md) §3.3 ends discovery with **"admission control → rank
→ cap at 8 (Rung 0) / 14 (Rung 2)"** but does not say what to rank *by*. The cap itself is
not negotiable: each admitted page costs a second of per-host politeness and a model pass to
extract from, against the 90–180 second budget in [ARCHITECTURE.md](../ARCHITECTURE.md) §4.4.

The obvious ranking is by confidence — how sure we are that a page is what it claims to be —
and it produces a bad eight.

A typical site answers `/pricing`, `/plans` and `/pricing/`, and lists two more pricing pages
in its sitemap. Ranked by confidence, those take five of the eight slots. The resulting
report can state the price five ways and has nothing to say about what changed last month,
what the product does, or whether the company is hiring.

**That is not a thin report. It is a confidently wrong-shaped one**, and a reader cannot see
from it that the other sections were starved rather than empty.

## Decision

**Candidates are admitted round-robin across the questions they answer.** The best pricing
page, then the best changelog, then the best feature page, and only once every question has
one does any question get a second.

Each probe therefore declares what it is *for* — pricing, features, changes, identity, trust,
direction — and URLs found via sitemap or `llms.txt` are classified from their path. A probe
that did not know its purpose could not participate in this, which is why the purpose is a
field on the probe rather than a comment beside it.

Within a question, the tie-break is **provenance**: a page named in the site's `llms.txt`
beats one listed in its sitemap, which beats one we guessed at and got a 200 from. A site
that publishes an `llms.txt` has told us what it considers worth reading, and that is better
evidence than a path existing.

## Consequences

**An eight covering six questions beats an eight covering two**, and the sections that would
otherwise be starved are the ones a competitive report is actually read for. Measured against
basecamp.com: 6 sources covering 5 questions. Against linear.app: 8 covering 5, including two
pages from its `llms.txt`.

**We will sometimes miss the better of two pricing pages.** If a site's real prices are at
`/plans` and `/pricing` is a marketing stub, breadth-first takes the stub and stops. The
mitigation is the tie-break, not the ranking — and if this turns out to matter, it is a
scoring problem inside one question rather than an argument against spreading across
questions.

**The cap is now coupled to the latency estimate.** `CAP_RUNG_0 = 8` is what the end-to-end
figure in `BENCHMARKS.md` will be derived from once the A1 numbers exist. A test asserts the
value so that changing it fails loudly rather than silently invalidating a published figure.

**Discovery is dominated by its own rate limit.** Fourteen probes plus a sitemap is fifteen
requests at one per second before a single page has been read for *content*. Against 90–180
seconds that is a real share, which is why the probe list is prioritised, why it stays short,
and why `Discovered::stopped_early` exists. If the A1 numbers make the budget tighter than
expected, cutting probes is the first lever — and the priority ordering means the cut lands
on the least valuable ones automatically.

## What this does not settle

**Nothing yet ranks *within* a question by quality.** The tie-break is provenance and then
URL length, which is a proxy for "closer to the root" and not a measure of anything. When the
golden set covers discovery, this is the first thing worth measuring.

**Off-site sources are not part of this.** §3.3's adapters — review platforms, trade press,
SEC filings — are a different admission problem, because they are not Primary and cannot set
table values. They get their own cap and their own decision when they are built.
