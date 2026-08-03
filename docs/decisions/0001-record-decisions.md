# ADR 0001 — Record architecture decisions

**Status:** accepted
**Date:** 2026-08-03

## Context

This project was specified before it was built, and the specification carries a lot of
decisions with reasoning attached. That reasoning erodes: a year from now the code will say
*what* it does, and the only record of *why* will be a document nobody re-reads because it
describes the product rather than the choices.

Two specific failure modes this project is exposed to:

- **Decisions that rest on facts that expire.** Several already do — Reddit's API terms, X's
  per-read pricing, YouTube's daily search quota. If the fact changes and nobody recorded
  that a design depended on it, the design silently becomes wrong.
- **Decisions taken under a constraint that later lifts.** Almost everything here is shaped
  by 24 GB of shared ARM. When that stops being true, whoever is here needs to know which
  choices were consequences of it and which were preferences.

## Decision

Every decision that is expensive to reverse, or that rests on an external fact, gets a short
file in `docs/decisions/`, numbered sequentially, using the template below.

An ADR is **not** a design document. It is a record of one choice, the alternatives that were
genuinely considered, and what would have to change for the choice to be wrong.

Superseded ADRs are **not deleted**. They are marked `superseded by NNNN`, because the value
of the record is largely in the decisions that turned out badly.

## Consequences

- A pull request that changes an expensive-to-reverse decision without an ADR is incomplete.
- The `docs/` specification stays a description of the product; `docs/decisions/` becomes the
  history of how it got that way.
- Small cost per decision, paid at the moment the reasoning is freshest and cheapest.

## Template

```markdown
# ADR NNNN — <the decision, as a statement>

**Status:** proposed | accepted | superseded by NNNN
**Date:** YYYY-MM-DD

## Context
What forced a choice. Include the facts it rests on, with sources and dates where they
are external — those are the ones that expire.

## Decision
What was chosen, stated plainly.

## Alternatives considered
What else was real, and why it lost. "None" is almost always a sign the problem was not
examined.

## Consequences
What this makes easy, what it makes hard, and **what would have to change for this to
become the wrong choice.**
```
