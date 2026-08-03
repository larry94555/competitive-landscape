# ADR 0002 — Constrain model output with llama.cpp's schema converter, not our own GBNF

**Status:** accepted
**Date:** 2026-08-03

## Context

Every fact in a report arrives as structured output from a local model. The chain
`docs/ROADMAP.md` specifies is:

```
Rust struct → schemars JSON Schema → GBNF grammar → llama-server → parsed back
```

The middle step can be done in two places: we convert the schema to GBNF ourselves and send
the grammar, or we send the schema and let llama.cpp convert it.

This matters more than it looks. The failure mode of unconstrained generation is not "the
model refuses" — it is a small percentage of *almost* valid JSON. A 1% parse failure across
a report carrying ~40 extracted values loses something from most reports, and the loss is
silent.

## Decision

Send the JSON Schema. llama.cpp converts it to GBNF internally, and its sampler enforces the
result.

`Constraint::Grammar` exists in `landscape-llm` for shapes a JSON Schema cannot express, but
it is not the default path.

## Alternatives considered

**Write a JSON Schema → GBNF converter in Rust.** Rejected. llama.cpp's sampler is the thing
that actually enforces the grammar; a second converter here could disagree with the one doing
the enforcing. A grammar that differs from the sampler's understanding fails *silently and
intermittently*, which is the worst available failure mode — worse than no constraint, which
at least fails loudly. It is also a non-trivial piece of code to own and keep in step with
upstream for no behavioural gain.

**Retry-and-validate without constraints.** Rejected. It converts a structural guarantee into
a probabilistic one, and burns prefill on retries — which is the binding resource on the
target hardware, per `ARCHITECTURE.md`.

**A hosted model with native structured output.** Rejected: the product's core promise is that
analysis runs on a local open-weights model. This remains available to users via BYOK.

## Consequences

- One less component to own; the constraint and the enforcement cannot disagree.
- A dependency on llama.cpp's schema coverage. Anything it does not support has to be
  expressed as a hand-written GBNF grammar through `Constraint::Grammar`, or the type
  simplified.
- The schema is derived from the Rust type via `schemars`, so changing a struct changes the
  grammar in the same commit. There is no second copy to forget.

**This becomes the wrong choice if** llama.cpp's converter turns out to mishandle a schema
feature the report types need — nested `$ref`, or tight numeric bounds — or if its output
diverges from the sampler in some version. The signal would be `LlmError::Unparseable`
appearing at all: it is defined as the "constraint is not working" variant precisely so this
is visible rather than lost among transport errors.

**Verified 2026-08-03:** `crates/landscape-llm/tests/constrained_decoding.rs` — 100
generations against Qwen3-4B-Q4_K_M, **0 parse failures, 0 content mismatches**, plus 10 runs
at temperature 0.9 in which the enum never left its three variants. That is the Phase 0 exit
criterion for constrained decoding. Numbers and caveats in [BENCHMARKS.md](../BENCHMARKS.md);
note that the same run produced an 11-second median latency on hardware faster than the
target, which is a separate and unresolved problem.
