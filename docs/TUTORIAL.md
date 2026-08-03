# Landscape — Tutorial

> **An hour, from a fresh clone to changing the code with confidence.**
>
> This is a *learning* path. [`Feature_Walkthrough.md`](Feature_Walkthrough.md) is the other
> kind of document — an exhaustive list of everything that works, for checking that it still
> does. Read that one when you want coverage. Read this one first.

**What you will have at the end:** the application running, one request traced from the box
in the browser to the line it wrote in the log, one deliberate breakage caught by a test,
and a clear idea of which of the six crates you would open to change any given thing.

**What you need:** Rust, Node 22+, and a terminal. No database, no Docker, no API key, no
model. Anything in this tutorial that needs more than that says so first.

> **On Windows**, use **Git Bash**. Every command below works there unchanged. `cmd.exe` and
> PowerShell need different quoting — [`Feature_Walkthrough.md` Part 3A](Feature_Walkthrough.md)
> has verified forms for both.

---

## 1. Run it (5 minutes)

```bash
cargo run -p landscape -- dev --store memory
```

First build takes a few minutes. Then:

```
INFO landscape: worker started
INFO landscape: listening on http://127.0.0.1:8787
```

`dev` runs the API and the worker **in one process**, sharing one in-memory store. That
matters: with `--store memory` each process gets its own map, so running `serve` and `worker`
separately would give you an analysis that stays `queued` for ever. One process, one store.

`-p landscape` is needed because the workspace builds two binaries — the application and
`landscape-bench` — and a bare `cargo run` cannot choose between them.

In a second terminal:

```bash
cd web && npm install && npm run dev
```

Open <http://localhost:5173>, type an idea, press Analyse.

**You will get an empty report.** That is not a bug and it is worth understanding before
anything else — see §3.

---

## 2. Follow one request all the way through

This is the part worth your time. Leave the server running and watch its terminal.

### 2.1 Make a request you can find again

```bash
curl -s -D - -o /dev/null http://127.0.0.1:8787/api/health
```

Among the response headers:

```
x-request-id: 0b9a3289e49a
```

Now look at the server's terminal:

```
INFO request{request_id=0b9a3289e49a}: landscape_api::request_id: handled method=GET path="/api/health" status=200 took_ms=0
```

**Same id in both places.** Every line a request writes carries its id, so one request's
whole story can be pulled out of a log holding thousands.

### 2.2 Choose your own id

Handy when you are about to do something and want to find it afterwards:

```bash
curl -s -o /dev/null -H "x-request-id: tutorial0001" http://127.0.0.1:8787/api/health
```

```
INFO request{request_id=tutorial0001}: … path="/api/health" status=200
```

We accept an id from in front of us so that a proxy — Caddy, in production — can stamp one
and have us log under the same value. **An inbound id is validated before it is trusted**:
hex, dashes, at most 64 characters. Try to smuggle a newline in and you get a fresh id
instead, because a newline in a log field lets a caller append a line of their own invention
to our log.

### 2.3 What the id is really for

When something breaks at our end, the reader is told so, and told what to quote:

```json
{ "error": "Something went wrong at our end.",
  "remedy": "Nothing you did caused this. Try again shortly — and if you tell us, quote 9f2c11ab7d04 and we can find exactly what happened.",
  "reference": "9f2c11ab7d04" }
```

The detail — the actual database error — is logged and **never** returned. Before this
existed both halves were true and nothing joined them: someone writing in to say it had
broken gave us a rough time of day against a log with every other request in it.

`docs/decisions/0005-observability-on-a-24gb-box.md` explains why this, and not a metrics
stack: three resident models leave no room on a 24 GB box, and trading a model for a
dashboard is a bad trade — the models are the product.

### 2.4 Where that request went in the code

Follow it in this order and the architecture explains itself:

| Step | File | What happens |
|---|---|---|
| 1 | `crates/landscape/src/main.rs` | Picks a role, builds a store, binds the port |
| 2 | `crates/landscape-api/src/request_id.rs` | Assigns the id, opens the span, writes the access line |
| 3 | `crates/landscape-api/src/routes.rs` | Matches the route, extracts and validates |
| 4 | `crates/landscape-core/src/analysis.rs` | The domain rules. **No I/O whatsoever** |
| 5 | `crates/landscape-db/src/lib.rs` | The `Store` trait — one interface, two implementations |
| 6 | `crates/landscape-api/src/error.rs` | Failure becomes a response a person can act on |

**The `Store` trait is the decision that shapes everything else.** It is why `cargo test`
needs nothing installed, and why you got this far without a database.

---

## 3. Why the report is empty

Type an idea and you get a report where every section says **"Nothing found in public
sources"**, with a list of what was checked.

That is the current, honest state: **the gathering pipeline is Phase 1 and does not exist
yet.** Nothing fetches a web page.

It renders that way on purpose rather than showing a spinner or an error, because "we looked
here, here and here and found nothing" is exactly how a *finished* run reports a real gap.
The honest case is built first so it never has to be retrofitted over a happy path — by which
time the happy path is what everyone has designed around.

---

## 4. Break something on purpose

The fastest way to trust a test suite is to watch it fail.

```bash
cargo test
```

**97 passing, nothing running.** Now open
`crates/landscape-core/src/analysis.rs` and find the minimum prompt length. Change `8` to
`2`, then:

```bash
cargo test -p landscape-core
```

A test fails, and it tells you what the rule was for rather than only that a number changed.
Put it back.

Now try one that is less obvious. In `crates/landscape-api/src/error.rs`, make the internal
error return its `detail` to the caller instead of a generic message:

```bash
cargo test -p landscape-api
```

`an_internal_failure_never_leaks_detail` fails. It exists because a database error reaching a
browser as a connection string is a security problem, not just an untidy message. Put it back.

---

## 5. The two harnesses

Both need a `llama-server`, and they behave differently without one — which is correct,
and worth knowing before you run them:

```bash
cargo run -p landscape-bench -- --runs 20 --label "my laptop"
```

Measures **how fast**. Prefill dominates on the target hardware, so it reports the realistic
~400-token span shape separately from a one-sentence prompt.

**Without a model this exits 1 with a message telling you how to start one.** It is a command
you ran on purpose; silently doing nothing would be the unhelpful answer.

```bash
cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

Measures **whether it is right** — ten frozen pricing pages with answers written by hand.
Three of the ten publish no price at all.

**Without a model this passes, printing `SKIPPED` and how to start one.** The opposite choice
from the benchmark, for the opposite reason: this runs under `cargo test`, and a test suite
that fails on a laptop with no model is a test suite people stop running.

**You need both, and the second is the one that is usually missing.** A defective
quantisation of Qwen3-1.7B once passed every check we had: fastest in the table, never failed
to parse, schema-valid throughout, and completely wrong. Constrained decoding guarantees the
*shape* of an answer and says nothing about its truth. The golden set scores that model at
10% against 87% for the correct build of the same model.

The set also checks itself, and that half needs no model:

```bash
cargo test -p landscape-golden
```

---

## 6. The rules this code is held to

Worth knowing before you write any of it:

- **`unwrap`, `expect`, `panic` and `todo` are denied, not warned.** A panic in a handler
  takes down work a user is waiting on. Tests opt out explicitly, because panicking is how a
  test reports failure.
- **A `Claim` cannot be constructed without a source label and a verbatim quote.** An
  unsourced sentence is not representable, so it cannot reach a reader by accident.
- **The documentation is executed.** `crates/landscape/tests/docs.rs` runs every `bash` block
  in `README.md` against the real binary. Two bugs once reached a reader through correct code
  and stale prose.
- **Links are checked.** `python3 scripts/check_links.py` resolves every internal link and
  every heading anchor across all the documentation.

```bash
python3 scripts/check_links.py
```

[`CODING_QUALITY.md`](CODING_QUALITY.md) is the full standard.

---

## 7. Where to go next

| If you want to | Read |
|---|---|
| Check that everything still works, feature by feature | [Feature_Walkthrough.md](Feature_Walkthrough.md) |
| Know what is being built and in what order | [ROADMAP.md](ROADMAP.md) |
| Understand the stack and why each piece is there | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Know what the product promises a reader | [PRODUCT_SPEC.md](PRODUCT_SPEC.md) |
| See how a fact earns its place in a report | [FACT_CHECKING.md](FACT_CHECKING.md) |
| Fix it at 23:00 | [RUNBOOK.md](RUNBOOK.md) |
| Know why a decision was made the way it was | [decisions/](decisions/) |

**The single most useful thing you can do next** is
[`Feature_Walkthrough.md`](Feature_Walkthrough.md) end to end. It is written from a user's
seat, every command in it was executed before it was written down, and if anything differs
from what it says, that is a bug worth reporting.
