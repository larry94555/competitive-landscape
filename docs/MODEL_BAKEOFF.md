# The model bake-off

> **Run locally, on a laptop, by whoever is working on the code.** Not on the production host
> — see [ADR 0011](decisions/0011-no-experiments-on-production.md). A benchmark leaves
> assumptions behind in whatever it runs on, and the one thing the deployment host has to stay
> is boring.

**What it closed:** Phase 0 exit criteria **1** (three-model choice) and **5** (`q8_0` KV
quantization decided on evidence). Results are in [BENCHMARKS.md](BENCHMARKS.md) **Run 14**.

**What it costs:** about forty minutes, most of it the two golden-set passes.

---

## What a laptop can and cannot answer

This matters more than any number below, so it goes first.

| Question | A laptop answers it |
|---|---|
| Which model invents fewer facts? | **Yes.** Accuracy is a property of the weights and the prompt, not of the CPU. |
| Is one model faster than another? | **Yes, as a ratio.** The 4B is ~2.8× slower than the 1.7B here, and it will be ~2.8× slower anywhere. |
| Does `q8_0` KV cost accuracy? | **Yes.** Same weights, same prompts, one flag changed. |
| **How many seconds will a report take in production?** | **No.** Not on this hardware, not from this side of the network. |

The last row is the reason the old version of this document existed, and it is the wrong way
to get that number. **Latency a user experiences is measured from where the user is**, against
a deployed system, through the product's own interface — not by benchmarking a box from the
inside. A stopwatch on the server measures the server; the thing worth knowing is how long
somebody waits.

---

## The candidates

**License-cleared only** — see [ADR 0007](decisions/0007-model-licenses.md). Qwen3 is
Apache-2.0 throughout.

| Port | Model | Why it is in the run |
|---|---|---|
| 8080 | `Qwen/Qwen3-4B-GGUF:Q4_K_M` | The working extraction model since Run 4 |
| 8082 | `Qwen/Qwen3-1.7B-GGUF:Q8_0` | The cheap tier — is it good enough to promote? |
| 8081 | `unsloth/Qwen3-1.7B-GGUF:Q4_K_M` | **The control.** Scored 10% in Run 3, and if it does not score 10% again the instrument has drifted |

> **`unsloth` quantizations are not candidates.** One of them was the fastest model in Run 3's
> table and scored 10%. It stays in the run as a control group, and only as that.

Start them with `llama-server`, one flag apiece:

```bash
llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080 --ctx-size 12288
```

---

## 1. Speed

```bash
cargo build --release -p landscape-bench
LLAMA_URL=http://127.0.0.1:8082 ./target/release/landscape-bench --runs 15 --label "laptop Qwen3-1.7B Q8_0"
```

**`--release` matters** — a debug build measures the harness rather than the model.

Two prompt shapes, and **the `span` shape is the one to care about**: ~400 tokens in, which is
what span pre-selection actually sends.

---

## 2. Correctness

```bash
LLAMA_URL=http://127.0.0.1:8082 GOLDEN_MAX_FABRICATIONS=99 \
  cargo nextest run --release -p landscape-golden --run-ignored only --no-capture
```

Ten frozen pricing pages, hand-written answers. `GOLDEN_MAX_FABRICATIONS=99` stops the run
failing early so the whole scorecard prints — the per-subject table is the useful part.

**Run the control too.** If `unsloth/Qwen3-1.7B-Q4_K_M` scores anything other than about 10%,
stop and find out what changed in the harness before believing any other row.

---

## 3. Memory

On Windows, mmap'd weights make working set misleading; private bytes is the number that moves
when the KV cache changes:

```bash
powershell -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='llama-server.exe'\" | Select-Object ProcessId,@{n='Private_GB';e={[math]::Round(\$_.PrivatePageCount/1GB,2)}}"
```

The transferable numbers are the **model file size** (which must be resident anywhere) and the
**difference** private bytes shows between two KV settings.

---

## 4. The `q8_0` KV question

Three resident models share a tight KV budget, so this is decided rather than deferred.

```bash
llama-server -m <the same gguf> --host 127.0.0.1 --port 8084 --ctx-size 12288 -ngl 0 \
  --cache-type-k q8_0 --cache-type-v q8_0
```

Run **§2 and §3 only** — accuracy and memory. Speed is not the question.

**`-ngl 0` on both sides of the comparison.** CPU-only removes the GPU from the experiment
entirely, which is both the fairer A/B and the closer analogue of a host with no GPU.

**What decides it:** if the golden-set score is unchanged and memory drops, take it. If the
score moves at all, do not — a KV decision made on memory alone would repeat exactly the
mistake Run 3 was built to catch.

---

## 5. What to do with the output

Write it into [BENCHMARKS.md](BENCHMARKS.md) as a numbered run, with the hardware named in the
first line, and record the decision it closes in [ROADMAP.md](ROADMAP.md). **Paste the raw
scorecard**, not a summary of it: the per-subject table is what makes a later disagreement
resolvable.

If a step fails, that is a bug in this document. Unlike the version this replaced, everything
above has been run end to end on the machine it was written on.
