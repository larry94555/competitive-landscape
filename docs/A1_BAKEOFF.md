# Running the bake-off on the A1

> **A command sheet, for the person with a terminal on that box.**
>
> The measurements in this file cannot be taken anywhere else. Laptop numbers are not
> transferable: the target is **4 ARM cores**, and prefill — the thing this architecture
> lives or dies on — behaves differently there than on 8 x86 threads. Every figure currently
> in [BENCHMARKS.md](BENCHMARKS.md) is labelled as laptop-only for that reason.
>
> **Nothing here deploys anything.** It builds a model server, runs two harnesses, and prints
> numbers. No systemd units, no Caddy, no database, no ports opened. Deployment is a separate
> job with a separate owner.

**What it closes:** Phase 0 exit criteria **1** (three-model choice) and **5** (`q8_0` KV
quantization decided on evidence), plus the `docs/BENCHMARKS.md` deliverable.

**Roughly 90 minutes**, most of it the models downloading and `llama.cpp` compiling.

---

## 0. Before anything: what is this box?

Two things need confirming, and both change what follows.

```bash
uname -m && nproc && free -g | awk '/^Mem:/ {print $2 " GB total"}'
```

**Expected:** `aarch64`, `4`, and about `24 GB`.

- **Not `aarch64`?** Stop — the architecture assumes ARM, and an x86 instance changes the
  model choice entirely. Tell me what it says.
- **Much less than 24 GB?** Stop. The three-model design needs ~17 GB resident. On a 1 GB
  `E2.1.Micro` nothing here will run.

```bash
swapon --show
```

**Expected: no output.** Swap must be off. A model that swaps does not fail — it goes a
hundred times slower while every health check passes, which presents as "the site is down"
with nothing in the logs. If this prints anything, `sudo swapoff -a` before measuring, or
every number below is fiction.

---

## 1. Build `llama-server` for this CPU

The distributed builds do not enable the ARM instructions that matter. `dotprod` and `i8mm`
are worth roughly a factor on prefill, which is the number this whole exercise is about.

```bash
sudo dnf install -y git cmake gcc-c++ || sudo apt-get update && sudo apt-get install -y git cmake g++
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
cmake -B build -DCMAKE_BUILD_TYPE=Release -DGGML_NATIVE=ON
cmake --build build --config Release -j4
```

`-DGGML_NATIVE=ON` is the load-bearing flag: it compiles for the CPU it is running on rather
than a portable baseline.

**Check it took:**

```bash
./build/bin/llama-server --version 2>&1 | head -3
grep -o "dotprod\|i8mm\|neon" build/CMakeCache.txt | sort -u
```

If neither `dotprod` nor `i8mm` appears, the numbers will be pessimistic and worth
mentioning when you paste them.

---

## 2. The models

**Licence-cleared candidates only** — see [ADR 0007](decisions/0007-model-licences.md).
Qwen3 is Apache-2.0 throughout. Gemma 3 and Llama 3.2 carry obligations, so they are worth
measuring but not worth adopting by accident.

Start with the two that matter most. **Qwen3-1.7B-Q8_0 and Qwen3-4B-Q4_K_M are the current
working pair**, and reproducing Runs 2 and 3 on real hardware is more valuable than a wide
sweep of models we have never used.

```bash
cd ~/llama.cpp
./build/bin/llama-server -hf Qwen/Qwen3-1.7B-GGUF:Q8_0 --host 127.0.0.1 --port 8080 --ctx-size 4096
```

Leave it running; open a second terminal for everything below.

> **`unsloth` quantizations are excluded on purpose.** One of them scored 10% on the golden
> set while being the fastest model in the table — see BENCHMARKS.md Run 3. Official
> repositories only.

---

## 3. The measurements

Both harnesses need the Rust toolchain. If it is not on the box:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
git clone https://github.com/larry94555/competitive-landscape
cd competitive-landscape
```

### 3a. Speed

```bash
LLAMA_URL=http://127.0.0.1:8080 cargo run --release -p landscape-bench -- --runs 20 --label "A1 Qwen3-1.7B Q8_0"
```

**`--release` matters** — a debug build measures the harness rather than the model.

Reports median and p95 for two prompt shapes, and counts three failure kinds separately.
**The `span` shape is the one to care about**: ~400 tokens in, which is the realistic
extraction case the span-pre-selection design depends on.

### 3b. Correctness

```bash
LLAMA_URL=http://127.0.0.1:8080 GOLDEN_MAX_FABRICATIONS=99 \
  cargo nextest run --release -p landscape-golden --run-ignored only --no-capture
```

Ten frozen pricing pages, hand-written answers. **`GOLDEN_MAX_FABRICATIONS=99` stops it
failing the run** so you get the full scorecard rather than an early exit — we want the
numbers, not a pass/fail.

If `cargo nextest` is not installed: `cargo install cargo-nextest --locked`, or use
`cargo test --release -p landscape-golden -- --ignored --nocapture`.

**Paste the whole scorecard.** The per-subject table is the useful part; the summary line
alone hides which subjects failed.

### 3c. Resident memory

While the server is running:

```bash
ps -o rss=,comm= -C llama-server | awk '{printf "%.1f GB  %s\n", $1/1048576, $2}'
```

The three-model design budgets ~17 GB total. This is the number that decides whether it fits.

---

## 4. Then repeat for the 4B

```bash
# stop the 1.7B first, then:
./build/bin/llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080 --ctx-size 4096
```

Re-run 3a, 3b and 3c with the label changed.

> **One server at a time.** Three resident models competing for four cores was a declared
> confound in Run 2 and made every latency pessimistic. Running them one at a time is the
> single biggest improvement over what we have.

---

## 5. `q8_0` KV cache — exit criterion 5

Three resident models share a tight KV budget, so this cannot be deferred the way it could on
a bigger box. The question is whether quantizing the KV cache costs accuracy.

```bash
./build/bin/llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080 \
  --ctx-size 4096 --cache-type-k q8_0 --cache-type-v q8_0
```

Re-run **3b and 3c** only — accuracy and memory. Speed is not the question here.

**What decides it:** if the golden-set score is unchanged and resident memory drops, take it.
If the score moves at all, do not — a KV-quantization decision made on memory alone would
repeat exactly the mistake Run 3 was built to catch.

---

## 6. What to send back

Paste the raw output. No need to summarise — I would rather read what the tool printed than a
description of it.

| | |
|---|---|
| §0 | `uname -m`, `nproc`, memory, `swapon` |
| §1 | Whether `dotprod`/`i8mm` were enabled |
| §3a | The full bench table, per model |
| §3b | The full golden-set scorecard, per model |
| §3c | Resident memory, per model |
| §5 | Scorecard and memory with `q8_0` KV |

I will turn it into `BENCHMARKS.md` Run 4, update the three-model choice in
[ROADMAP.md](ROADMAP.md), and close exit criteria 1 and 5.

**If something fails, paste the failure.** A step that does not work here is a bug in this
document, and this document was written from the specification rather than from having run it
on that hardware — the first honest thing to say about it is that it is unrehearsed.
