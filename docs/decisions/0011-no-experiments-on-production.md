# 0011 — No experiments on production

**Status:** accepted · **Date:** 2026-08-04

## Context

Phase 0 carried an action item asking for a model bake-off **on the Oracle A1** — build
`llama.cpp` there, download two models, run two harnesses, paste the numbers back. The
justification was that the target hardware is 4 ARM cores and a laptop is not, so laptop
numbers do not transfer.

The justification is half right, and the half that is wrong is the important half.

## Decision

**No measurement, benchmark or experiment runs on the production host.** Bake-offs, harnesses
and model comparisons run locally, on a development machine, by whoever is working on the
code.

**Any testing that does involve production is from a client's perspective only** — through the
product's own interface, over the network, as a user experiences it.

## Why

**An experiment leaves things behind.** A compiler toolchain, a downloaded model, a half-built
`llama.cpp`, a swap file someone turned on to get past an OOM. Each one is an assumption the
deployment then quietly depends on, and the value of the production host is that it is
*boring*: what runs there should be what was deployed there, and nothing else. Keeping it that
way is also what makes the platform swappable, which is worth more than any single number.

**Measuring from inside the box measures the box.** The question a latency budget answers is
*how long does somebody wait*, and that includes the network, the browser, and every hop in
between. A stopwatch on the server cannot see any of them. Testing from the client's side
keeps the work pointed at the experience rather than at the hardware.

**And most of what the bake-off asks is not hardware-dependent at all.** Which model invents
fewer prices is a property of the weights and the prompt. Whether `q8_0` KV costs accuracy is
one flag, changed twice, on the same machine. Both were answered locally in an afternoon —
[BENCHMARKS.md](../BENCHMARKS.md) Run 14 — after months of being blocked on somebody having a
terminal open on a server.

## Consequences

**Speed numbers are ratios, not budgets.** The 4B is ~2.8× slower than the 1.7B on the span
shape, and it will be ~2.8× slower on any CPU. What a laptop cannot say is how many seconds a
report takes on 4 ARM cores, and this ADR accepts that gap rather than closing it with an
experiment on production.

**The end-to-end latency criterion changes shape.** It is no longer *"measure the box"*; it is
*"measure the wait"*, from a client, against whatever is deployed. That measurement belongs
after a deployment exists, and it belongs to the same discipline as the rest of the product:
what a user experiences is the only thing worth optimizing.

**`docs/A1_BAKEOFF.md` is gone**, replaced by [MODEL_BAKEOFF.md](../MODEL_BAKEOFF.md), which
has been run end to end rather than written from a specification and hedged about.

## What this is not

It is not a claim that ARM numbers do not matter. When the host exists and something is
deployed to it, **the honest way to learn its latency is to use the product and time it** —
which is a client-side measurement, needs no toolchain on the server, and answers the question
the roadmap actually cares about.
