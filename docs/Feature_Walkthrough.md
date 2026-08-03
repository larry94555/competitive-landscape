# Landscape — Feature Walkthrough

> A test run you can do yourself, on this branch, from a user's point of view.
>
> Every command and every expected output below was **executed on this branch** before being
> written down, and pasted from the result. If something behaves differently for you, that is
> a bug worth reporting — not a typo in the instructions.

---

## Read this first: what actually exists

The specification in `docs/` describes a finished product. **The code does not implement most
of it yet**, and a walkthrough that implied otherwise would waste your afternoon.

| Feature | Testable today? |
|---|---|
| Type an idea, get an analysis back | **Yes** |
| Watch it move queued → running → complete | **Yes** |
| A report that says what it could not find, and what it checked | **Yes** |
| Every rejection tells you what to do about it | **Yes** |
| The queue: one worker claims each job exactly once | **Yes** |
| Persistence across a restart | **Yes**, with Postgres |
| Constrained model output, guaranteed to fit a Rust type | **Yes**, with `llama-server` |
| Reading real web pages | No — not built |
| Real competitors, prices, features | No — the report comes back empty on purpose |
| Accounts, quotas, payment | No |
| Alerts, watches, email | No |
| PDF, sharing, follow-up questions | No |
| The community area, admin console | No |

**The empty report is the feature, not a gap.** Sections render as "nothing found" with a list
of what was checked, which is exactly how a real run reports a real gap. The honest case is
built first, so it never has to be retrofitted over a happy path.

Everything in the demo films is a **prototype** with invented data. It is not this code.

---

## Setup

You need **Rust** and **Node**. Nothing else for Part 1–6.

```bash
git switch docs/initial-roadmap
cargo build
cd web && npm install && cd ..
```

`rust-toolchain.toml` pins the compiler, so rustup installs the right one on first build.

If a step goes wrong, `docs/RUNBOOK.md` §5 lists the common local problems.

---

## Part 1 — Start it

**Do this.** One terminal, and no database:

```bash
cargo run -p landscape -- dev --store memory
```

**You should see:**

```
WARN landscape: using the in-memory store - nothing is saved when this stops
INFO landscape: listening on http://127.0.0.1:8787
INFO landscape: worker started
```

**Why it matters.** That is the whole setup. No account, no key, no database, no Docker. The
project's standing rule is that the full application runs on a laptop with nothing installed;
this is that rule being true rather than claimed.

**Note the port.** 8787, not 8080 — 8080 is llama.cpp's default and this application runs
`llama-server` beside itself.

### Check it is alive

```bash
curl -s localhost:8787/api/health
```

```json
{"status":"ok","queued":0,"version":"0.1.0"}
```

**Why it matters.** `queued` is read from storage, not a constant. A healthy answer proves the
process is up *and* can reach its store. A health check that only proves the process is
running reports healthy while every request fails.

---

## Part 2 — The thing a user actually does

**Do this.** Second terminal:

```bash
cd web && npm run dev
```

Open <http://localhost:5173>.

**You should see** one heading — *What is your idea?* — one box, one button. No examples, no
options, no navigation. **Analyse** is greyed out until you type something.

**Type:** `an app that helps small farms sell to local restaurants` and press **Analyse**.

**You should see**, in order:

1. Your words repeated back as a heading
2. `Queued.` then `Reading public web pages…` then `Done.`
3. Two sections, each saying **Nothing found in public sources**, each listing what was
   checked

**Why it matters.** This is the whole loop: your text became a job, a worker picked it up, and
a report came back — with the page polling until it settled. The report is empty because
nothing fetches yet, but the *shape* is the real one.

### Now break it

**Type:** `a crm` and press **Analyse**.

**You should see:**

> a prompt must contain at least 8 characters, got 5 Edit what you typed and try again.

**Why it matters.** The message says the limit *and* what to do. It comes from the server, not
from the frontend inventing its own wording — so there is one rule, in one place, and the UI
cannot drift from it.

---

## Part 3 — The same thing over the API

**Do this.**

```bash
curl -s -X POST localhost:8787/api/analyses \
  -H 'content-type: application/json' \
  -d '{"prompt":"an app that helps small farms sell to local restaurants"}'
```

**You should see** something like:

```json
{
  "id": "e8af4190-cf74-46ea-bc74-3884d7e24f5a",
  "prompt": "an app that helps small farms sell to local restaurants",
  "status": "queued",
  "created_at": "2026-08-03T07:56:24.732745400Z",
  "report": null
}
```

Take that `id` and read it back:

```bash
curl -s localhost:8787/api/analyses/PASTE_THE_ID_HERE
```

**You should see** `"status": "complete"` and a `report` object.

**Why it matters.** `report` is `null` while queued and an object once complete. It is never a
half-filled shape — a partial report is not representable.

---

## Part 4 — Read the report properly

Look at the `report` from Part 3:

```json
"sections": [
  {
    "key": "pricing",
    "title": "Prices",
    "status": "not_found_in_public_sources",
    "claims": [],
    "checked": [
      "nothing was fetched: the gathering pipeline is not built yet"
    ],
    "notes": []
  },
  ...
]
```

**What to notice, and why each is deliberate:**

| Field | Why |
|---|---|
| `status: not_found_in_public_sources` | A gap is a **finding**, not an error. It renders as a calm block, never as a failure |
| `checked` | An unfalsifiable "we found nothing" becomes one you can repeat. This is the trust mechanism, not a consolation message |
| `claims: []` | Nothing is asserted, because nothing was read. There is no placeholder text pretending to be a result |
| `sources: []` | Same. No sources were read, so none are listed |
| `searched_as: ""` | Empty because nothing resolved a category yet. When it does, this appears in the UI **above** the results, so a wrong reading is visible before you believe the report |

**Why it matters.** A `Claim` cannot be constructed in this codebase without a source label
and a verbatim quote from that source. An unsourced sentence is not merely discouraged — it is
**unrepresentable**, so it cannot reach you by accident.

---

## Part 5 — Every way to be told no

Each of these is worth running once: the point is that none returns a bare error.

**A prompt that is too short:**

```bash
curl -s -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{"prompt":"a crm"}'
```
```json
{"error":"a prompt must contain at least 8 characters, got 5","remedy":"Edit what you typed and try again."}
```

**Only whitespace** — note it says `got 0`, not `got 8`:

```bash
curl -s -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{"prompt":"        "}'
```
```json
{"error":"a prompt must contain at least 8 characters, got 0","remedy":"Edit what you typed and try again."}
```

**Forgetting the content type** — the most common way to hit this by hand:

```bash
# expect-failure
curl -s -X POST localhost:8787/api/analyses -d '{"prompt":"an app for farm to restaurant orders"}'
```
```json
{"error":"This endpoint takes JSON, and the request did not say it was sending any.","remedy":"Add -H 'content-type: application/json' to the request."}
```

**Malformed JSON** — told apart from the above:

```bash
# expect-failure
curl -s -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{oops'
```
```json
{"error":"That is not valid JSON: ...","remedy":"Check the quoting - a shell often eats the quotes around a JSON body."}
```

**Right JSON, wrong field:**

```bash
curl -s -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{"idea":"an app for farm to restaurant orders"}'
```
```json
{"error":"The JSON was readable but not the shape this endpoint wants: ... missing field `prompt` ...","remedy":"This endpoint expects an object with a \"prompt\" string."}
```

**An id that does not exist, and one that is not an id** — both `404`, deliberately identical:

```bash
curl -s localhost:8787/api/analyses/00000000-0000-0000-0000-000000000000
curl -s localhost:8787/api/analyses/not-a-uuid
```
```json
{"error":"No analysis with that reference.","remedy":"It may have been removed. Start a new one."}
```

**Why it matters.** Every rejection carries a remedy, and the three JSON failures are told
apart — a missing header and a malformed body look identical from your side otherwise. The two
`404`s are identical **on purpose**: from your side a mistyped reference and a deleted one are
the same situation, and distinguishing them only tells a prober what our ids look like.

---

## Part 6 — The queue, and the trap

**Do this.** Stop the `dev` process. Start only the API:

```bash
cargo run -p landscape -- serve --store memory
```

Submit an analysis, then read it back after a few seconds.

**You should see** it stay `queued` forever, and:

```json
{"status":"ok","queued":1,"version":"0.1.0"}
```

**Why it matters.** `--store memory` gives each **process** its own store. `serve` alone has
no worker, so nothing claims the job. This is the single most likely local confusion, which is
why `dev` exists: it runs both halves in one process sharing one store.

`queued: 1` in the health response is how you tell "nothing is running" from "everything is
broken" without reading a log.

Now stop it and go back to `cargo run -p landscape -- dev --store memory`. Same submission, completes.

---

## Part 7 — With a real database

Everything above forgets itself when you stop the process. This part is about persistence.

You need Postgres. Either:

```bash
docker compose up -d db
```

Or, if Docker is unreliable on your machine — it commonly is on Windows — install it in WSL2
instead. `README.md` has the four commands, and `RUNBOOK.md` §5 covers the Docker Desktop
failure that presents as a hang rather than an error.

**Then:**

```bash
cp .env.example .env
cargo run -p landscape -- dev
```

Migrations apply on boot.

**Do this.** Submit an analysis, wait for `complete`, note the id. **Stop the process. Start
it again.** Read the same id back.

**You should see** the same report. With `--store memory` it would be a `404`.

**Why it matters.** The queue is a **column in the database**, not a separate service — an
analysis in state `queued` *is* the queue entry. That makes "the job exists" and "the row
exists" a single transaction rather than two facts in two systems that eventually disagree.

---

## Part 8 — The model path

This is the piece the whole product rests on, and you can exercise it locally.

**You need `llama-server` running.** If you already have one on 8080, it will be found.

```bash
llama-server -hf Qwen/Qwen3-4B-GGUF:Q4_K_M --host 127.0.0.1 --port 8080
```

**Do this:**

```bash
cargo test -p landscape-llm -- --ignored --nocapture
```

**You should see**, after several minutes:

```
  runs                 100
  parse failures       0
  content mismatches   0
  median latency       10855 ms
  p95 latency          17201 ms
```

**Why it matters.** A Rust struct becomes a JSON Schema becomes a grammar the model is
*sampled against* — so it cannot produce anything that is not that type. The second test drives
a three-variant enum at high temperature and it never wanders outside the three.

**Read the latency line too.** Eleven seconds for one short extraction, on a laptop faster than
the server this is meant to run on. `docs/BENCHMARKS.md` explains why that is a problem and
what has not yet been measured. It is the open question in this project.

**No `llama-server`?** The test skips and tells you how to start one. It does not fail — that
is why CI, which runs no model, stays green.

### Measure it yourself

```bash
cargo run -p landscape-bench -- --runs 20 --label "my laptop"
```

Two prompt shapes: a single sentence, and a realistic ~400-token span window. It reports
median and p95 latency, and — separately — how many outputs **failed the constraint**, how
many hit a **transport error**, and how many parsed but had the **wrong contents**.

**Those three are counted apart on purpose.** An earlier version lumped them together and
reported a healthy server's timeouts as evidence that constrained decoding was broken. And
the last one exists because a shape guarantee is not an accuracy guarantee: a defective
quantisation of Qwen3-1.7B returned perfectly-formed JSON containing
`"plan_name": "/:D!01:56:G>!#9*2-@1F-08@E5A0'"`. See `docs/BENCHMARKS.md`.

---

## Part 9 — What the tests prove

```bash
cargo test
```

**You should see 38 passing, with nothing running.**

Worth knowing what a few of them are actually for:

| Test | What it protects |
|---|---|
| `memory_store_satisfies_the_contract` | One contract body, run against both the in-memory and Postgres stores, so the fast tests cannot slowly become fiction |
| `an_internal_failure_never_leaks_detail` | Asserts a database password cannot appear in a response body |
| `no_reader_description_judges_the_publisher` | Enforces the language rule from `FACT_CHECKING.md` — we say what we confirmed, never that a site is bad |
| `length_is_measured_in_characters_not_bytes` | Eight accented characters are 16 bytes; measuring bytes would reject them and accept the ASCII equivalent |
| `every_documented_curl_actually_works` | Runs every command in `README.md` against the real binary |

The last one exists because two bugs once reached a reader through correct code and a stale
README. Run it with:

```bash
cargo test -p landscape --test docs -- --ignored
```

Against a database:

```bash
DATABASE_URL=postgres://landscape:landscape@127.0.0.1:5432/landscape cargo test -- --ignored
```

---

## Part 10 — The prototype films

Not this code — a throwaway prototype with invented companies on the reserved `.example`
domain. Worth watching to see what the finished product is meant to feel like, and worth
**not** confusing with what you just tested.

`prototype/video/links.json` has the published URLs. Or run one locally:

```bash
python prototype/build.py --preview
```

---

## Record what you found

| Part | Works? | Notes |
|---|---|---|
| 1 — starts, health answers | | |
| 2 — web UI, submit, completes | | |
| 2 — short prompt rejected in the UI | | |
| 3 — API create and read back | | |
| 4 — report shows `checked`, no invented claims | | |
| 5 — all seven rejections carry a remedy | | |
| 6 — `serve` alone leaves it queued | | |
| 7 — survives a restart with Postgres | | |
| 8 — 100 generations, 0 parse failures | | |
| 8 — `landscape-bench` reports three error kinds separately | | |
| 9 — `cargo test` green with nothing running | | |

**If something differs from what is written here, that is a bug.** Every command above was run
on this branch before it was written down. The most useful report is the command, what you
expected from this document, and what you actually got.
