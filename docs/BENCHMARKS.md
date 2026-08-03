# Landscape — Benchmarks

> Real numbers, with the hardware they were measured on. `docs/ROADMAP.md` Phase 0 requires
> this file to exist before a model is chosen, because every latency figure in the
> specification is currently an estimate.
>
> **Nothing here has been measured on the target hardware yet.** The A1 is not provisioned.
> These are laptop numbers, and they are recorded because a baseline that exists beats an
> estimate that does not — not because they answer the question.

---

## Run 1 — constrained decoding, x86 laptop

**Date:** 2026-08-03
**Purpose:** the Phase 0 exit criterion — *"grammar-constrained JSON round-trip working from
Rust with 0 parse failures over 100 runs."*

### Setup

| | |
|---|---|
| Host | Windows 11 laptop, x86-64 — **not the target hardware** |
| Model | Qwen3-4B-Q4_K_M (GGUF) |
| Server | `llama-server`, `--gpu-layers 0` (CPU only), `--ctx-size 12288`, `--threads 8`, `--threads-batch 12`, `--parallel 1`, `--cache-type-k q8_0`, `--cache-type-v q8_0`, `--flash-attn on`, `--cache-prompt` |
| Client | `landscape-llm`, `/completion` with `json_schema`, temperature 0.1 |
| Task | Extract a `PricingFact` — string, f64, 3-variant enum, `Option<u32>` — from a one-sentence pricing line |
| Runs | 100, with varied inputs so the prompt cache cannot carry the result |

### Result

| Measure | Value |
|---|---|
| **Parse failures** | **0 / 100** |
| Content mismatches | 0 / 100 |
| Median latency | **10.9 s** |
| p95 latency | **17.2 s** |
| Total wall clock | 1164 s |

A second test drove the same type at temperature 0.9 — high on purpose — ten times. Every
response parsed, and the enum never left its three variants.

### What this establishes

**The spine works.** Rust struct → `schemars` schema → llama.cpp's GBNF → constrained sample →
parsed back into the struct, with no retry logic and no defensive re-parsing anywhere in the
path. Zero failures is the number that matters: at 1%, a report carrying 40 extracted values
loses something more often than not.

The enum result is worth its own line. Nothing but the grammar stops a model returning
`"weekly"` for a period that is not in the type, and if that were possible every enum in the
report schema would need defensive re-mapping downstream.

### What this does not establish, and the number that should worry us

**11 seconds median for one short extraction, on a laptop that is faster than the target.**

The A1 has 4 ARM cores against this machine's 8 x86 threads, so the target is expected to be
*slower*, not faster. `ARCHITECTURE.md` §4.4 estimates a 90–180 second end-to-end analysis.
At this rate that budget buys **8–16 extractions**, before any fetching, before the
synthesiser, and with nothing left for the router.

Three things could close the gap, and they should be measured rather than assumed:

1. **This is the 4B extractor doing a 1.7B router's job.** The prompts are trivial; the
   right comparison is Qwen3-1.7B on the same task.
2. **Prefill dominates, and these prompts are short.** A realistic 400-token span window may
   not cost proportionally more than a 30-token sentence — or may cost far more. Unmeasured.
3. **`--parallel 1`.** Aggregate throughput at `--parallel 2/4/8` is the number that decides
   how many extractions fit in a report, and it is not this number.

**The honest reading:** the exit criterion about *correctness* is met. The exit criterion
about *latency* — "a measured, realistic end-to-end latency estimate, and a written, honest
decision on what the Rung-0 latency promise will be" — is **not**, and this run is the first
evidence that it may be the harder one. The roadmap's instruction for that case is explicit:
cut scope per analysis, do not quietly ship a promise the hardware cannot keep.

### Reproduce

```bash
llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080 --gpu-layers 0
cargo test -p landscape-llm -- --ignored --nocapture
```

`LLAMA_URL` overrides the server address.

---

## Still to measure — all of it on the A1

Per `ROADMAP.md` Phase 0. None of this is done.

- [ ] Provision the A1 and convert to Pay-As-You-Go
- [ ] Prefill and generation tok/s per candidate model × quantization
- [ ] Q4_K_M **and** Q4_0 — ARM repacking may make the lower-quality format faster; measure
- [ ] Qwen3 1.7B / 4B / 8B / 14B, plus Gemma 3 4B/12B and Llama 3.2 3B. **Licence review
      first** — a model we cannot use commercially is not a candidate
- [ ] Realistic prompt shapes: 400-token span → 100-token JSON, and 4k-token bundle → 700 out
- [ ] Aggregate throughput at `--parallel 1/2/4/8`
- [ ] Resident RAM per model, against the ~17 GB budget for three
- [ ] `q8_0` KV cache quantization validated against the golden set
- [ ] Time-to-first-token
