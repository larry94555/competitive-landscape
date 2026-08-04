# Landscape

Competitive analysis you can check. Type a business idea in plain words; get a report where
every claim carries the page it came from, and every gap says what was looked at.

Public data only. Nothing is estimated that cannot be sourced, and anything we could not
verify is shown as exactly that rather than quietly dropped.

**Status: early.** The skeleton runs end to end — a prompt is queued, claimed by a worker,
and returned as a report — but the report is empty because the gathering pipeline is Phase 1.
It is empty *honestly*: every section renders as "nothing found" with the checks listed,
which is the same treatment a real run gives a real gap.

---

## Run it

> **The standing rule for this repository: the whole application must be runnable and
> testable on a laptop with nothing installed but Rust and Node.** No cloud account, no API
> key, no database required. Anything that breaks that is a bug, not a prerequisite.

### The short way — no database

```bash
cargo run -p landscape -- dev --store memory
```

That is the entire setup. It starts the API and a worker in one process sharing an
in-memory store, on <http://127.0.0.1:8787>.

`-p landscape` is needed because the workspace builds two binaries — the application and
`landscape-bench`. Without it cargo cannot tell which one you meant.

Then, in a second terminal:

```bash
cd web && npm install && npm run dev
```

Open <http://localhost:5173>, type an idea, watch it complete.

`dev` runs both halves in **one process** on purpose. With `--store memory` each process
gets its own map, so a separate `serve` and `worker` would never see each other's queue —
you would get an analysis that stays "queued" forever. One process, one store.

Nothing is saved when you stop it. That is the trade for needing no database.

### The full way — with Postgres

```bash
docker compose up -d db
cp .env.example .env
cargo run -p landscape -- dev
```

Migrations run automatically on boot. To apply them without starting anything:

```bash
cargo run -p landscape -- migrate
```

To run the pieces separately, as production does:

```bash
cargo run -p landscape -- serve      # terminal 1
cargo run -p landscape -- worker     # terminal 2
```

### Postgres without Docker

Docker is one way to get a database, not the only one, and on Windows it is the least
reliable one — Docker Desktop's privileged service silently failing to start leaves named
pipes that exist but answer nothing, which looks like a hang rather than an error.

If you have WSL2, install Postgres in the distro you already have. WSL2 forwards
`localhost:5432` to Windows, so `DATABASE_URL` is unchanged and nothing else moves:

```bash
sudo apt update && sudo apt install -y postgresql
sudo service postgresql start
sudo -u postgres psql -c "CREATE USER landscape WITH PASSWORD 'landscape' SUPERUSER"
sudo -u postgres createdb -O landscape landscape
```

The connection string in `.env.example` then works as written. On Linux and macOS the same
applies with the system package manager, or Postgres.app.

### Commands

| Command | What it does |
|---|---|
| `cargo run -p landscape -- dev` | API and worker in one process. **Use this locally.** |
| `cargo run -p landscape -- serve` | The HTTP API alone |
| `cargo run -p landscape -- worker` | Claims queued analyses and runs them |
| `cargo run -p landscape -- migrate` | Applies migrations and exits |
| `cargo run -p landscape -- fetch <url>` | Fetches one URL through every policy and says what happened. Needs no database |
| `cargo run -p landscape -- gap <file>` | Measures where prices live across a list of pricing pages |
| `cargo run -p landscape -- discover <origin>` | Finds the pages worth reading about one company |
| `cargo run -p landscape -- read <origin>` | The whole path: discover, fetch, convert, extract |

Add `--store memory` to any of them to skip Postgres entirely.

| Variable | Default | Notes |
|---|---|---|
| `DATABASE_URL` | *(none)* | Required unless `--store memory` |
| `BIND_ADDR` | `127.0.0.1:8787` | Change if the port is taken |
| `RUST_LOG` | `landscape=info` | `landscape=debug` for query-level detail |

### Ports

| Port | What |
|---|---|
| **8787** | This API |
| 5173 | Vite dev server |
| 5432 | Postgres |
| 8080+ | `llama-server` — llama.cpp's default, and this project runs several |

**The API deliberately avoids 8080.** It is llama.cpp's default, and the architecture runs
`llama-server` sidecars on the same machine, so anything defaulting there collides with the
one process the application cannot work without.

If a port is taken anyway, the error says how to find the process holding it.

### Check it is up

```bash
curl -s http://127.0.0.1:8787/api/health
```

```json
{ "status": "ok", "queued": 0, "version": "0.1.0" }
```

`queued` comes from storage rather than from a constant, so a healthy response also proves
the process can reach its database. A health check that only proves the process is running
will report healthy while every request fails.

### Start an analysis

```bash
curl -sX POST http://127.0.0.1:8787/api/analyses \
  -H 'content-type: application/json' \
  -d '{"prompt":"an app that helps small farms sell to local restaurants"}'
```

Take the `id` from the response and read it back a second later — it will be `complete`.

**On Windows, that command needs a different shell or a different form.** The `\`
continuations and single quotes above are POSIX syntax; `cmd.exe` and PowerShell both reject
them, and PowerShell additionally aliases `curl` to `Invoke-WebRequest`.
[`Feature_Walkthrough.md` Part 3A](docs/Feature_Walkthrough.md) has forms verified on both.

**The `content-type` header is required.** Leaving it off is the most common way to hit this
endpoint by hand, so the rejection names the missing header rather than making you guess:

```json
{ "error": "This endpoint takes JSON, and the request did not say it was sending any.",
  "remedy": "Add -H 'content-type: application/json' to the request." }
```

---

## Test it

```bash
cargo test          # every Rust test, no services needed
cd web && npm test  # frontend
```

`cargo test` needs **nothing running**. Every test — including the whole HTTP request path —
runs against an in-memory store. That is the point of the `Store` trait: a request path
nobody can exercise on a laptop is a request path nobody exercises.

The cost of two implementations is that they can drift, so there is one contract
(`landscape-db/src/conformance.rs`) run against both:

```bash
docker compose up -d db
DATABASE_URL=postgres://landscape:landscape@127.0.0.1:5432/landscape \
  cargo test -- --ignored
```

Those tests are `#[ignore]`d so the default run stays fast and dependency-free; CI runs them
against a real Postgres on every push. Without that, the fast tests would slowly become
fiction.

### Scoring a model

With a `llama-server` running, score it against the golden set:

```bash
cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

`--nocapture` is the point — the scorecard is the output. The run **fails on one thing**: a
price returned for a plan whose page publishes none. Everything else is printed for reading.

Today Qwen3-4B passes and Qwen3-1.7B does not, and the way the 1.7B fails is instructive:
it returns the price of the plan *above* the one asked about, quoting that line correctly.
Nothing was hallucinated. See [BENCHMARKS.md](docs/BENCHMARKS.md) Run 3.

The set also checks itself, and that half needs no model at all:

```bash
cargo test -p landscape-golden
```

### The documentation is tested

Two bugs once reached a reader through correct code and a stale README: a port that had
moved, and a documented `curl` missing its `content-type` header. No unit test could catch
either, because both were in prose.

So the README is executed. `crates/landscape/tests/docs.rs` parses every fenced `bash`
block, boots the binary on an OS-assigned port, and runs each documented command against
it — asserting none returns a 4xx. Three faster checks need nothing running: the port in
the prose matches `DEFAULT_ADDR`, every `cargo run -- X` is a real subcommand, and no
documented POST omits its content-type.

```bash
cargo test -p landscape --test docs
```

Pull request descriptions get the same treatment, but linted rather than executed — a PR
body is untrusted input:

```bash
python3 scripts/lint_instructions.py README.md
```

Every internal link and heading anchor across all the documentation is resolved too:

```bash
python3 scripts/check_links.py
```

The anchors are the half worth having. A link to a deleted file breaks loudly the first time
someone clicks it; a link to `#4.7` in a document that has renumbered its sections drops the
reader at the top of a long page with no sign anything is wrong, and they conclude the
cross-reference was vague rather than broken. It found one on its first run.

For fast local feedback, enable the pre-push hook once per clone:

```bash
git config core.hooksPath .githooks
```

### Everything CI runs

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo test --all-features --doc
cargo deny check advisories licenses bans sources
cd web && npm run typecheck && npm run build
```

`cargo test` still works and is what the rest of this README documents — **`nextest` is a
faster runner for the same tests**, not a different suite. Install it with
`cargo install cargo-nextest --locked` if you want CI's exact behaviour locally.

**The `--doc` line is not optional.** `nextest` does not run doctests, so replacing
`cargo test` with it silently stops checking them. That is the classic way a test suite
shrinks without anyone deciding to shrink it.

**Run `cargo deny` with exactly those four checks.** Dropping `bans` is the tempting one —
it is about duplicate and wildcard dependencies rather than security — and it is the check a
new crate fails. Adding one without `publish = false` trips `allow-wildcard-paths`, which
only applies to private crates, and a local run that omitted `bans` reported everything
green.

`unwrap`, `expect`, `panic` and `todo` are **denied**, not warned — a panic in a handler
takes down work a user is waiting on. Test modules opt out explicitly, because panicking is
how a test reports failure.

**Coverage is reported, never gated.** `cargo llvm-cov` runs on every push and prints a
summary — currently **77.8%** of regions. There is no threshold, on purpose: a percentage
target on a codebase this young buys tests written to move a number rather than to catch a
fault. What the number is for is noticing a **drop** — a change that adds a module and no
tests for it.

```bash
cargo llvm-cov nextest --all-features --summary-only
```

**It understates, and knowing why matters more than the figure.** It measures the fast suite
only, so `landscape-db/src/pg.rs` reads **0%** — every line of it is covered by the same
conformance contract as the in-memory store, in a separate CI job against a real database.
Reading that 0% as "untested" would be exactly backwards.

**Retries are off** (`.config/nextest.toml`). A flaky test that is quietly retried is a bug
we have agreed not to find. Nothing in this suite touches the network or the clock, so a
failure is real.

---

## Layout

```
crates/
  landscape-analyze/ one subject in, one report out: the sections, the claims, the cites
  landscape-core/   domain types, the report schema, and what was checked when nothing
                    was found. No I/O — pure, fully unit-tested
  landscape-db/     the Store trait, an in-memory implementation, and Postgres
  landscape-api/    axum routers, request validation, error mapping, and the event stream
                    that lets a reader watch a report fill in
  landscape-llm/    constrained generation: a Rust type in, that type out
  landscape-bench/  measures how fast a llama-server is
  landscape-discover/ which pages to read about a company, in which language,
                     and which eight
  landscape-extract/ a page into Markdown, the ~400 tokens per plan, capability or
                     fact worth reading, and the dates a changelog states
  landscape-fetch/  the outside world: SSRF guard, robots.txt, per-host politeness
  landscape-golden/ measures whether it is right. Ten frozen pages, known answers
  landscape/        the binary: dev | serve | worker | migrate
migrations/         SQL, applied on boot
web/                Vite + React + TypeScript (strict)
docs/               the specification this is built from
prototype/          throwaway UI prototype and the demo films. Not the production frontend
```

A few decisions worth knowing before reading the code:

**The queue is a column, not a service.** An analysis in state `queued` *is* the queue entry,
claimed with `FOR UPDATE SKIP LOCKED`. One fewer thing to run and pay for, and "the job
exists" and "the row exists" become a single transaction rather than two facts in two
systems that eventually disagree.

**The report schema is defined once**, in `landscape-core`. `schemars` generates the JSON
Schema from it; the model's decoding grammar is derived from the same place. Two
hand-maintained copies of a schema diverge, one generated copy cannot.

**A `Claim` cannot be built without a source label and a verbatim quote.** An unsourced
sentence is not representable, so it cannot reach a reader by accident.

**Errors never leak internals.** A database failure becomes "Something went wrong at our
end" plus a remedy; the detail is logged. There is a test asserting a connection string
cannot appear in a response body.

**Constrained decoding guarantees shape, never truth.** A model is forced to return the
right *type*; nothing about that makes the values in it correct. A defective quantisation
once passed every check we had — fast, always parseable, schema-valid throughout, and
wrong. `landscape-golden` is the other half: ten frozen pages with known answers, three of
which publish no price at all, because the failure that would sink this product is a
confident dollar figure attached to a company that never named one.

---

## Toolchain

`rust-toolchain.toml` pins the compiler, so rustup installs the right one on first build and
CI cannot drift from your laptop. Node 22+ for the frontend.

Raise the pinned version deliberately — a compiler bump deserves its own commit and its own
CI run.

---

## Where the design lives

The code implements a specification written first. Start with
[`docs/ROADMAP.md`](docs/ROADMAP.md), which indexes the rest.

| Document | Covers |
|---|---|
| [phases/](docs/phases/) | **What only the founder can do, per phase** — with what "done" looks like and how it is verified |
| [TUTORIAL.md](docs/TUTORIAL.md) | **An hour from a fresh clone to changing the code.** Start here if you are new |
| [MODEL_BAKEOFF.md](docs/MODEL_BAKEOFF.md) | How the model choice was measured — locally, because production is not a laboratory |
| [PRODUCT_SPEC.md](docs/PRODUCT_SPEC.md) | User flows and the report schema |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | The stack, and why each piece is there |
| [FACT_CHECKING.md](docs/FACT_CHECKING.md) | Source dispositions and auditable negatives |
| [IDEA_ANALYSIS.md](docs/IDEA_ANALYSIS.md) | The eight levels, and what we refuse to answer |
| [CODING_QUALITY.md](docs/CODING_QUALITY.md) | The standard this code is held to |
| [Feature_Walkthrough.md](docs/Feature_Walkthrough.md) | **Test every feature that exists today, by hand.** Start here if you want to see what works |
| [RUNBOOK.md](docs/RUNBOOK.md) | What to do when something breaks |
| [Demo_Walkthrough.md](docs/Demo_Walkthrough.md) | The demo films, and the build that compiles them |

---

## Licence

MIT.
