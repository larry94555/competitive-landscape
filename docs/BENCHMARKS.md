# Landscape — Benchmarks

> Real numbers, with the hardware they came from. `docs/ROADMAP.md` Phase 0 requires this
> file before a model is chosen, because every latency figure in the specification is
> otherwise an estimate.
>
> **Still nothing measured on the target hardware.** The A1 is not provisioned. These are
> laptop numbers and are labelled as such.

Two harnesses, measuring different things. Timings come from the first; whether the answers
are *true* comes from the second, and nothing in the first can tell you.

```bash
cargo run -p landscape-bench -- --runs 20 --label "what this is"
```

```bash
cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

`LLAMA_URL` selects the server for both.

A third measures neither speed nor truth but **shape** — where on a page a price actually
lives, which decides whether a headless browser is ever built:

```bash
cargo run -p landscape -- gap docs/js-gap-sample.txt
```

---

## Run 4 — the JavaScript-rendering gap, measured

**Date:** 2026-08-03 · **Sample:** 28 real pricing pages, `docs/js-gap-sample.txt` ·
**Question:** [ARCHITECTURE.md](ARCHITECTURE.md) §5.5's two counters.

```bash
cargo run -p landscape -- gap docs/js-gap-sample.txt
```

| Where the price was | Pages | Share |
|---|---|---|
| **Tier 1** — visible in static HTML | 24 | **85.7%** |
| **Tier 2** — recovered from embedded JSON (JSON-LD) | 1 | 3.6% |
| Residual — neither | 3 | 10.7% |
| *Unreachable, excluded* | *1* | — |

### The residual is two different things, and separating them is the finding

| Page | What it is |
|---|---|
| `hetzner.com/cloud` | **A genuine JS-rendered page** — no `€` amount and no price-shaped JSON key anywhere in the bytes |
| `databricks.com/company/contact` | Publishes no price. Control group |
| `palantir.com/platforms/foundry/` | Publishes no price. Control group |

**So the JavaScript-rendering gap is 1 page in 28 — 3.6% — and the residual is 10.7%.**
Those fall on opposite sides of §5.5's ~5% threshold, and only the first is the number the
rule is about. No headless browser renders a price that was never written.

**Tier 5 is not built.** [ADR 0009](decisions/0009-no-headless-browser.md).

### Two things this run says about the plan

**"The big one" is smaller than expected.** §5.5 predicted embedded state would close most of
the gap. It recovered exactly one page — because the gap it was aimed at barely exists.
**85.7% of pricing pages simply print their prices in HTML.** The prediction is not wrong so
much as pointed at a problem that turned out to be small.

**The control group caught a bug in the instrument on its first run.**
`databricks.com/company/contact` was reported as *priced*, because the detector matched
*"Learn professional Data and AI tools for free"*. Without two deliberately price-free URLs in
the sample, the tier-1 count would have been quietly one too high and nobody would have
looked.

### How far to trust it

**Twenty-eight pages chosen by one person tells 40% from 4% and nothing finer** — which is
the decision in front of us, and it is not close. It is **not a market statistic.**

The sample skews toward companies that publish prices, because those are the pages this
product reads. Tiers 3–4 were not measured, so 3.6% is an **upper bound**. And one page's
classification moves the figure by 3.6 points, which is why §5.5 requires a Phase 2
re-measurement rather than treating this as settled.

---

## Run 3 — the golden set, and what shape tests could never see

**Date:** 2026-08-03 · **Host:** same laptop, three `llama-server` processes resident ·
**Task:** ten frozen pricing pages, one plan each, scored against hand-written references.

```bash
LLAMA_URL=http://127.0.0.1:8080 \
  cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

Prompt **v2**. Seed 7, temperature 0. Verdicts are reproducible — two consecutive 4B runs
produced byte-identical verdict tables, differing only in latency.

| Model | Fields correct | Perfect subjects | **Invented prices** | Any fabrication | Median |
|---|---|---|---|---|---|
| Qwen3-4B Q4_K_M | **90%** | 8 / 10 | **0** | 1 | 11.7 s |
| Qwen3-1.7B Q8_0 | 87% | 7 / 10 | **1** | 2 | 7.5 s |
| ~~Qwen3-1.7B Q4_K_M (`unsloth`)~~ | **10%** | 0 / 10 | **3** | 8 | 5.7 s |

### The result this was built for

**Run 2 gave the defective quantisation a clean bill of health on every measure it had:**
0/20 unparseable, and the *fastest* median in the table. Only a hand-written note recorded
that its output was garbage.

The golden set scores it at **10%**, against 87% for the official quantisation of the same
model at the same size and the same speed. That gap is now a number a test can fail on
rather than a comment somebody has to read.

The value is not that we caught this one — we already had. It is that the next one gets
caught by machinery instead of by luck.

### The 4B is the only model here that can be trusted with a price

The single assertion in the golden-set test is that no price is returned for a plan whose
page publishes none. **Qwen3-4B passes it. Qwen3-1.7B Q8_0 does not**, and the way it fails
is worth reading:

| | expected | returned |
|---|---|---|
| `contact-sales`, plan `Enterprise` | no price | **$49 / monthly** |

$49 is the price of *Grower*, the plan directly above it on the page, and the model quoted
that line verbatim. It did not hallucinate — it answered a neighbouring question and
supported the answer honestly. Every fact in the output is true of *something* on the page.

That failure mode matters more than a hallucination would. A fabricated number often looks
wrong; a correctly-quoted number attached to the wrong plan looks exactly like a correct
answer, including to the quote-fidelity check, which passes it.

**So the tiering thesis from Run 2 survives, but with a boundary Run 2 could not see.** The
1.7B is 1.6× faster and nearly as accurate in aggregate — and aggregate accuracy is the
wrong measure for the tier that assigns a dollar figure to a named company. A 1.7B router
picking which spans to read is still well supported. A 1.7B extractor is not.

### What v2 of the prompt changed, including what it cost

Both good models returned `billing_period: "monthly"` for plans they had *correctly*
reported as having no published price — an invalid state, and one the type cannot forbid
(see [ADR 0004](decisions/0004-require-every-property.md) for why the tagged-union fix
measured worse). v2 states the invariant in prose instead.

| | v1 | v2 |
|---|---|---|
| Qwen3-4B — perfect subjects | 7 / 10 | **8 / 10** |
| Qwen3-4B — fabrications | 3 | **1** |
| Qwen3-1.7B Q8_0 — fields correct | 73% | **87%** |

**It was not free.** The 4B now returns nothing at all for `plain-table` — the easiest
subject in the set, a price in a plain table — where v1 answered it correctly. Reproducible
across both runs. Three added lines of caution bought two fewer fabrications and one new
silence on the simplest possible case, which is the trade constrained extraction seems to
offer everywhere: abstention and coverage move together.

Recorded rather than tuned away. Chasing it with a fourth prompt revision would be fitting
the prompt to ten pages we wrote ourselves.

### What the set does not tell us

It has ten subjects, all pricing, all in English, all written by the same person on the same
afternoon — so it shares that person's blind spots, and a model could score 100% on it while
failing on the first real page it meets. It is a floor, not a certificate. `ROADMAP.md`
takes it to 25 subjects in Phase 1 and 50 in Phase 2, and the first user-reported error
belongs in it the day it arrives.

---

## Run 2 — the tiering thesis, tested

**Date:** 2026-08-03 · **Host:** Windows 11 laptop, x86-64, 8 threads, CPU only — **not the
target hardware** · **Task:** extract a `PricingFact` (string, f64, 3-variant enum,
`Option<u32>`) under constrained decoding.

> [!WARNING]
> **Confound, stated up front.** These runs happened with **three `llama-server` processes
> resident** on one laptop, competing for the same cores. Absolute latencies are therefore
> pessimistic. The *comparison* between models is fair — all three ran under the same
> contention — but do not read any single figure as what one model alone would do. Run 1
> measured the same 4B task at **10.9s** median with only one server up, against **12.6s**
> here.

| Model | Shape | Median | p95 | Unparseable | Wrong contents |
|---|---|---|---|---|---|
| **Qwen3-1.7B Q8_0** | span (~400 tok) | **3.8 s** | 4.4 s | 0/20 | 0/20 |
| Qwen3-1.7B Q8_0 | sentence | 4.6 s | 5.7 s | 0/20 | 0/20 |
| Qwen3-4B Q4_K_M | span (~400 tok) | 12.6 s | 18.5 s | 0/20 | 0/20 |
| Qwen3-4B Q4_K_M | sentence | 16.6 s | 25.6 s | 0/20 | 0/20 |
| ~~Qwen3-1.7B Q4_K_M (`unsloth`)~~ | either | 12–16 s | — | 1/20 | **20/20** |

### What this settles

**The tiering thesis holds.** A 1.7B router is **~3.3× faster** than the 4B on the realistic
span shape, at no measured cost in accuracy on this task. `ARCHITECTURE.md` §4.7's three-tier
design is worth building.

**In budget terms:** 120 seconds buys roughly **32 extractions** on the 1.7B against **10** on
the 4B — before any fetching, and under three-way contention. Run 1's pessimism about the
90–180s promise was premature: it measured a 4B doing a 1.7B's job.

**Longer prompts did not cost proportionally more.** The span shape is roughly 13× the
sentence shape in characters, and was *faster* on both models. Prefill on x86 with 8 threads
is not the binding constraint the architecture expects it to be on 4 ARM cores — **which is
exactly why this must be re-run on the A1 before anything is concluded.**

### Two findings that cost more than the timings

**A defective quantisation produces schema-valid garbage.** The `unsloth` Q4_K_M build of
Qwen3-1.7B returned things like:

```json
{"plan_name": "/:D!01:56:G>!#9*2-@1F-08@E5A0'(5,#0#D>9G", "price_usd": 9, ...}
```

Perfectly shaped. Entirely wrong. **Constrained decoding guarantees shape, never content** —
so a broken model or quantisation sails straight through the one mechanism that looks like it
should catch it. The official Q8_0 of the *same model* was flawless.

The practical consequence: **a model swap needs an accuracy check, not just a latency
check.** `landscape-bench` now reports `wrong contents` separately for that reason, and the
golden set exists to make the check meaningful.

**`schemars` does not bound its integers, and llama.cpp does not infer the bound.** A `u32`
becomes `{"type":"integer","format":"uint32","minimum":0}` — with no `maximum`. The grammar
therefore permits integers no `u32` can hold, and the 1.7B produced
`"order_limit": 1000000000000000` in **6 of 20** runs. `serde` rejected them, surfacing as
`LlmError::Unparseable` — which reads as *"constrained decoding is broken"* when the
constraint was simply never told the real limit.

`landscape-llm` now adds the missing bounds before sending the schema. Same model, same
prompts, after the fix: **0 of 20**. See [ADR 0003](decisions/0003-bound-integer-schemas.md).

The larger model happened not to hit it. That is precisely how a bug like this reaches
production.

---

## Run 1 — constrained decoding, the exit criterion

**Date:** 2026-08-03 · Qwen3-4B-Q4_K_M, single server, same laptop.

| Measure | Value |
|---|---|
| **Parse failures** | **0 / 100** |
| Content mismatches | 0 / 100 |
| Median latency | 10.9 s |
| p95 | 17.2 s |

Plus ten runs at temperature 0.9 in which a three-variant enum never wandered outside its
three variants.

**This is the Phase 0 exit criterion for constrained decoding, and it is met.** Rust struct →
`schemars` schema → llama.cpp's GBNF → constrained sample → parsed back, with no retry logic
and no defensive re-parsing anywhere in the path.

```bash
cargo test -p landscape-llm -- --ignored --nocapture
```

---

## Still to measure — all of it on the A1

None of this is done. It is the remainder of Phase 0's model work.

- [ ] Provision the A1 and convert to Pay-As-You-Go
- [ ] Re-run everything above on 4 ARM cores. **Prefill is expected to behave differently**,
      and the tiering conclusion depends on it
- [ ] Prefill and generation tok/s separately, rather than end-to-end latency
- [ ] `Q4_K_M` **and** `Q4_0` — ARM repacking may make the lower-quality format faster
- [ ] Qwen3 1.7B / 4B / 8B / 14B, Gemma 3 4B/12B, Llama 3.2 3B. **Licence review first**
- [ ] Aggregate throughput at `--parallel 1/2/4/8`
- [ ] Resident RAM for three models against the ~17 GB budget
- [ ] `q8_0` KV cache quantization validated against the golden set
- [ ] Time-to-first-token
- [ ] One server at a time, so the numbers are not contended

Run 3 adds two of its own:

- [ ] **Field order as a lever.** llama.cpp walks properties in the order the schema
      serialises them, and `serde_json`'s sorted maps currently pick that order for us.
      `preserve_order` would hand it back. Quote-first (read, then answer) against
      quote-last (answer, then justify) is a measurable question, and a hand probe showed
      quote-first spends the whole token budget quoting — so it needs a `maxLength` too
- [ ] **Whether the 1.7B's plan confusion survives better prompting**, or is a size limit.
      This decides whether the Extractor tier can ever be a 1.7B, which is the difference
      between ~7 s and ~12 s per span on the numbers above
