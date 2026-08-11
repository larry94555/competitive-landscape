# ADR 0003 — Add the bounds `schemars` omits before constraining the model

**Status:** accepted
**Date:** 2026-08-03

## Context

[ADR 0002](0002-constrained-decoding-via-llama-cpp.md) delegates JSON Schema → GBNF to
llama.cpp, and named the condition that would make that the wrong call:

> **This becomes the wrong choice if** llama.cpp's converter turns out to mishandle a schema
> feature the report types need — nested `$ref`, or **tight numeric bounds** […] The signal
> would be `LlmError::Unparseable` appearing at all.

It appeared. During the Run 2 benchmark, Qwen3-1.7B returned:

```json
{"plan_name": "Grower", "price_usd": 49, "billing_period": "monthly",
 "order_limit": 1000000000000000}
```

`order_limit` is `Option<u32>`. `u32` stops at 4,294,967,295. `serde` rejected it, in **6 of
20 runs**.

The cause is a gap between two tools, neither of which is wrong on its own:

- `schemars` describes a `u32` as `{"type":"integer","format":"uint32","minimum":0}`. The
  bound lives in `format`, which is a *hint*, not a constraint.
- llama.cpp builds its grammar from `minimum`/`maximum` and ignores `format` — reasonably,
  since `format` is advisory in JSON Schema.

So the sampler was told "any integer ≥ 0" and complied. **The constraint was never broken;
it was never given the real limit.** That distinction matters, because the symptom — an
`Unparseable` error — looks exactly like the constraint failing.

The 4B model never hit this. It simply never chose an absurd number. That is how the bug
would have reached production: invisible on the model used in development, intermittent on
another.

## Decision

`landscape-llm` walks the generated schema and adds the `maximum` (and, for signed types,
the `minimum`) that each bounded integer format implies, before sending it.

Existing bounds are never overwritten: a type that declares its own range knows better than
a format-derived guess.

`u64` and `i64` are **deliberately left alone**. Their extremes cannot be represented exactly
as an `f64`, and JSON numbers are `f64` — an imprecise bound would reject values that
legitimately fit, which is worse than no bound at all.

## Alternatives considered

**Annotate every field.** `#[schemars(range(max = ...))]` on each integer in the report
schema. Rejected: it is the same fact written twice, in a form that can be forgotten on the
next field added. The report schema is large and still growing.

**Widen the Rust types to `u64`.** Rejected: it does not fix the problem, it moves it. A
model that emits 10^15 for an order count is wrong whether or not the type can hold it, and
widening throws away a genuine domain constraint to work around a plumbing gap.

**Validate after parsing instead.** Rejected: it converts a structural guarantee back into a
runtime check, which is the thing constrained decoding exists to avoid. It also wastes the
generation — the model has already spent tokens on a value we will discard.

**Write our own schema → GBNF converter.** Still rejected, for ADR 0002's reason: a second
converter could disagree with the sampler that enforces the grammar. This fix works *with*
llama.cpp's converter by giving it better input, rather than replacing it.

## Consequences

- Every report type gains correct bounds without remembering anything.
- One more place where our schema differs from raw `schemars` output. It is a single
  function with tests naming each case, including that `u64` is untouched.
- ADR 0002 stands. The gap was in the input to the converter, not the converter.

**Verified 2026-08-03:** same model, same prompts, `landscape-bench --shape span --runs 20`
against Qwen3-1.7B Q8_0 — **6/20 unparseable before, 0/20 after**. Numbers in
[BENCHMARKS.md](../BENCHMARKS.md).

**This becomes the wrong choice if** llama.cpp starts honoring `format` itself, at which
point this becomes redundant rather than harmful — or if a report type needs a bound the
format cannot express, which is what `#[schemars(range(...))]` remains available for.
