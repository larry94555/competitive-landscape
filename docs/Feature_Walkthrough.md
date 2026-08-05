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
| Trace one request through the log by its reference | **Yes** |
| Scoring a model on whether its answers are *true* | **Yes**, with `llama-server` |
| Deciding *who* a report is about, before fetching | Logic only — no UI yet, see below |
| **Fetching a public page politely, and refusing to be aimed inwards** | **Yes** — Part 8B |
| **Measuring where a price lives on real pages** | **Yes** — Part 8C |
| **Finding which pages to read about a company** | **Yes** — Part 8D |
| **The whole path: discover, fetch, convert, extract plans and capabilities** | **Yes** — Part 8E |
| Reading real web pages *as part of a report* | No — the fetcher exists, nothing calls it yet |
| Real competitors, prices, features | No — the report comes back empty on purpose |
| Accounts, quotas, payment | No |
| Alerts, watches, email | No |
| PDF, sharing, follow-up questions | No |
| The community area, admin console | No |

**The empty report is the feature, not a gap.** Sections render as "nothing found" with a list
of what was checked, which is exactly how a real run reports a real gap. The honest case is
built first, so it never has to be retrofitted over a happy path.

**One row above says "logic only", and it is worth being clear about.** The
*disambiguation gate* — the code that decides whether we know which company a report is
about, and refuses to continue when two candidates are too close — exists and is fully
tested. **You cannot exercise it from the UI**, because nothing yet produces candidates for
it to judge; that needs the fetching the gate exists to authorise. It is built first on
purpose, and the only thing you can run today is its tests:

```bash
cargo nextest run -p landscape-core subject
```

**12 tests**, covering the case that matters: two candidates within the margin produce a
question rather than a report.

Everything in the demo films is a **prototype** with invented data. It is not this code.

---

## Which shell

**This matters on Windows, and the commands below are written for one shell.**

Every `curl` in this document uses POSIX syntax — single quotes around the JSON, `\` to
continue a line. That works in **Git Bash, WSL, macOS and Linux**. It does **not** work in
`cmd.exe` or PowerShell, and the failure is confusing rather than obvious: `cmd` treats `\`
as the end of the command and runs the next line as a separate one, so you get a rejection
from the API followed by *"'-H' is not recognized"*.

If you are on Windows, either **use Git Bash** — simplest, and everything below works
unchanged — or use the per-shell forms in [Part 3A](#part-3a--the-same-request-in-cmdexe-or-powershell).

> **PowerShell has an extra trap.** In Windows PowerShell 5.1, `curl` is an *alias for
> `Invoke-WebRequest`*, not curl at all. You need `curl.exe`, and even then PowerShell eats
> the quotes around a JSON body. Part 3A gives two forms that work.

---

## Setup

You need **Rust** and **Node**. Nothing else for Part 1–6.

```bash
git switch main
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

**The box empties**, but your idea stays on screen as the heading.

**Why it matters.** This is the whole loop: your text became a job, a worker picked it up, and
a report came back — with the page polling until it settled. The report is empty because
nothing fetches yet, but the *shape* is the real one.

The box clearing is deliberate. Without an account you get one analysis a day, and an empty
box is what says so — a box still holding your words invites you to press Analyse again and
be refused.

### Now break it

**Type:** `a crm` and press **Analyse**.

**You should see:**

> a prompt must contain at least 8 characters, got 5 Edit what you typed and try again.

**And what you typed is still there.** Rejection does *not* clear the box: you have to edit
it, and retyping something you just wrote is a worse punishment than a typo deserves.

**Why it matters.** The message says the limit *and* what to do. It comes from the server, not
from the frontend inventing its own wording — so there is one rule, in one place, and the UI
cannot drift from it.

---

## Part 2A — The run has a URL

**Do this.** With an analysis on screen, look at the address bar. It says `/a/<id>` — it changed
the moment the run was accepted.

**Now press reload.**

**You should see** the same analysis come back: the heading you typed, and either the report or
the sections still arriving. **Copy the URL into another tab** and it opens there too.

**Why it matters.** Until this existed a refresh lost the run entirely, and there was nothing to
send anybody. It is also what the two features above it wait on — editing an idea and asking a
follow-up both need something to return to.

**Now press Back.** The box comes back empty, because the address bar and the page have to agree;
a report still on screen under a URL that no longer names it is the disagreement that makes
people stop trusting a page.

**Try a link that points at nothing:** <http://localhost:8787/a/not-a-real-id>.

> We could not find that analysis. Start a new one.

**Why it matters.** A dead link is the one a reader is most likely to still have. A blank page
would leave them unsure whether it was them or us, and the box is right there underneath so
there is something to do about it.

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

## Part 3A — The same request in cmd.exe or PowerShell

Skip this if you are in Git Bash, WSL, macOS or Linux — Part 3 already worked.

**Every command here was run on Windows before being written down.**

### cmd.exe

Double quotes outside, escaped double quotes inside. No single quotes, and `^` — not `\` —
continues a line:

```bat
curl -s -X POST localhost:8787/api/analyses -H "content-type: application/json" -d "{\"prompt\":\"an app that helps small farms sell to local restaurants\"}"
```

Doubling the inner quotes works too, if you find it easier to read:

```bat
curl -s -X POST localhost:8787/api/analyses -H "content-type: application/json" -d "{""prompt"":""an app that helps small farms sell to local restaurants""}"
```

### PowerShell

`curl.exe`, not `curl` — and `--%` stops PowerShell parsing the rest, which is what keeps the
quotes intact:

```powershell
curl.exe --% -s -X POST localhost:8787/api/analyses -H "content-type: application/json" -d "{\"prompt\":\"an app that helps small farms sell to local restaurants\"}"
```

Or skip curl entirely and use PowerShell's own client, which needs no quoting gymnastics:

```powershell
$body = @{ prompt = 'an app that helps small farms sell to local restaurants' } | ConvertTo-Json
Invoke-RestMethod -Uri http://localhost:8787/api/analyses -Method Post -ContentType 'application/json' -Body $body
```

**GET requests are fine everywhere** — no body, no quoting:

```bat
curl -s localhost:8787/api/health
```

---

## Part 3B — Find one request in the log

Every response carries a reference. This is how a fault someone reports becomes a fault you
can look at.

**Do this:**

```bash
curl -s -D - -o /dev/null http://127.0.0.1:8787/api/health
```

**You should see**, among the headers:

```
x-request-id: 0b9a3289e49a
```

And in the terminal running the server, the same twelve characters:

```
INFO request{request_id=0b9a3289e49a}: landscape_api::request_id: handled method=GET path="/api/health" status=200 took_ms=0
```

**Now pick your own**, which is handy when you are about to do something and want to find it
afterwards:

```bash
curl -s -o /dev/null -H "x-request-id: walkthrough01" http://127.0.0.1:8787/api/health
```

The log line now reads `request{request_id=walkthrough01}`. We accept an id from in front of
us so a proxy can stamp one and have us log under the same value.

**Now try to forge one:**

```bash
curl -s -D - -o /dev/null -H "x-request-id: aaa bbb" http://127.0.0.1:8787/api/health
```

**You should get a fresh generated id, not `aaa bbb`.** An inbound id is checked before it is
trusted — hex, dashes, at most 64 characters. A space is harmless; a **newline** is not,
because it would let a caller append a line of their own writing to our log, including a
convincing forged error. The rule is a narrow allow-list rather than a list of characters to
strip, since that second form is the one that is always incomplete.

**Why it matters.** When something breaks at our end you are told so, and told what to quote:

```json
{ "error": "Something went wrong at our end.",
  "remedy": "Nothing you did caused this. Try again shortly — and if you tell us, quote 9f2c11ab7d04 and we can find exactly what happened.",
  "reference": "9f2c11ab7d04" }
```

The actual cause — a database error, a connection string — is logged and **never** returned.
Before this, both halves of that were true and nothing joined them.

**In the browser**, the reference appears in the error message as a selectable monospace
chip, because the only thing anyone ever does with it is copy it into a message.

**Only a 5xx carries one.** Every rejection in Part 5 is a 4xx and has no `reference` — a
rejected prompt is fully explained by its own message, and a reference number there would
tell someone who mistyped that they have found a fault worth reporting.

`docs/decisions/0005-observability-on-a-24gb-box.md` records why correlated logs, and not a
metrics stack: three resident models leave no spare RAM on a 24 GB box.

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

**None of them carries a `reference` either**, and that is deliberate — every one is a 4xx,
meaning you can fix it yourself from what the message already says. The reference in
[Part 3B](#part-3b--find-one-request-in-the-log) is for the other case: something broke at
our end and there is nothing useful you can do except tell us which failure it was.

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

## Part 8A — Is the model actually *right*?

Everything in Part 8 measures shape and speed. Neither can tell you whether an answer is
true. This part can.

**Do this** — no model needed:

```bash
cargo test -p landscape-golden
```

**You should see 32 passing.** They are two things.

Eleven check the *golden set itself*: that every reference answer really appears on its page, that
at least three subjects publish no price, that every subject explains in prose why it exists. A
measuring instrument nobody calibrates produces numbers that are worse than none, because they
get believed.

The rest check the **parsers**, against ten real pages frozen in
`crates/landscape-golden/pages/`:

```bash
cargo test -p landscape-golden --test the_pages
```

**0.11 seconds, and no model is involved.** Every extraction step before the model is
deterministic — which windows a pricing page yields, which capabilities a features page names,
how many dated entries a changelog carries — so the answer can be written down and checked on
every pull request. `BENCHMARKS.md` Runs 5 to 16 found sixteen defects by reading real output
by hand; this is the half of that method a build can do. It was calibrated by putting six of
those defects back into the code: it caught five, and the sixth is caught by a unit test
elsewhere. Building it found a seventeenth — a changelog of two dozen releases that read as
having no dates at all (Run 17).

When one fails it names the page, quotes why that page is in the set, and prints expected
against got for **every** page that moved, because a change to a shared rule usually moves
several and which ones is the diagnosis.

**Now score a running model:**

```bash
cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

**You should see** a scorecard — ten frozen pricing pages, four columns each:

```
subject                            plan price period quote
plain-table                        ok   ok    ok     ok      7210 ms
contact-sales                      ok   FAB   FAB    ok      7618 ms
...
fields correct 87%   perfect 7/10   median 7513 ms
invented prices 1   any fabrication 2
```

**`FAB` is the column to read.** It means the page says nothing and the model filled the
field in anyway. Three of the ten pages publish no price at all, and one of those three
shows a real, correctly formatted price — for a *different product* on the same page.

**The run fails on exactly one thing:** a price returned for a plan whose page publishes
none. Not on wrong answers, not on misses. A test that fails for a reason you consider
negotiable is a test you learn to ignore, and this one has to survive being run against
whatever model someone happens to have loaded.

**So it is expected to be red against Qwen3-1.7B and green against Qwen3-4B.** That is the
finding, not a broken test. When the 1.7B fails `contact-sales` it returns `$49` — the price
of the plan directly above on the page — and quotes that line *verbatim*. It did not invent
anything. It answered a neighbouring question well.

That is the failure worth understanding before trusting any of this: a fabricated number
often looks wrong, while a correctly-quoted number attached to the wrong plan looks exactly
like a right answer.

**Every failure prints what came back**, next to what was expected and why the subject is in
the set — so a red run explains itself rather than sending you off to re-run it by hand.

**To explore a model that fabricates** without editing the test:

```bash
GOLDEN_MAX_FABRICATIONS=99 cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

---

## Part 8B — Fetch a page, and try to make it misbehave

**This is the newest thing you can test, and the most security-relevant code in the project.**
It needs nothing running — no database, no model, no server.

### It fetches

```bash
cargo run -p landscape -- fetch https://example.com/
```

**You should see:**

```
url     https://example.com/
status  200
bytes   559
fetched 2026-08-03T23:25:53.289297700+00:00
```

### Now try to aim it at us

The whole product is "fetch a URL a stranger named", which makes this the one attack we are
guaranteed to face. On most cloud providers the first address below hands out credentials to
anything that asks.

```bash
cargo run -p landscape -- fetch http://169.254.169.254/latest/meta-data/
```

```
refused That address is link-local and not reachable from the internet.
```

```bash
cargo run -p landscape -- fetch http://127.0.0.1:8787/api/health
```

```
refused That address points back at this machine.
```

**Even with the server running**, that second one is refused — the guard never gets as far
as asking whether something is listening.

```bash
cargo run -p landscape -- fetch file:///etc/passwd
```

```
refused only http and https are fetched, not file
```

**Note what the messages do and do not say.** They explain enough for an honest user to
understand, and never repeat the address back — someone probing us should learn nothing
about what resolved to what internally.

### And check that we are a good citizen

Google's `robots.txt` disallows `/search`. We fetch the rules first, then honour them:

```bash
cargo run -p landscape -- fetch https://www.google.com/search?q=test
```

```
refused www.google.com asks crawlers not to fetch /search?q=test
```

The same host, on a path it permits:

```bash
cargo run -p landscape -- fetch https://www.google.com/robots.txt
```

```
status  200
```

**Why it matters.** `FACT_CHECKING.md` treats honouring `robots.txt` as an ethical
commitment rather than a risk position, which decides the ambiguous cases: a **404 means
allowed** (no rules exist), but a **500 or a 429 means disallowed** — we cannot read what the
site wants, and the polite reading of "I am struggling" is not "carry on". There is no
`--ignore-robots` flag and there will not be one; a flag that exists gets used at 2am by
someone in a hurry.

**One honest caveat.** Each `landscape fetch` is a separate process, so the one-second
per-host delay does not apply *between* invocations — the pacer lives in the `Fetcher`, and
you get a new one each time. Inside a single run it does apply. That is
[ADR 0008](decisions/0008-fetching-from-strangers.md)'s recorded limitation: per-process
pacing is correct for one worker and wrong the moment there are two.

**The guard is the one file in this project at 100% coverage with no exemption path**
(`CODING_QUALITY.md` §6.2). You can check that claim rather than take it:

```bash
cargo llvm-cov nextest -p landscape-fetch --summary-only
```

---

## Part 8C — Measure where prices actually live

**This one produced a decision.** [ADR 0009](decisions/0009-no-headless-browser.md) says no
headless browser gets built, and this is the measurement it rests on. Needs nothing running.

```bash
cargo run -p landscape -- gap docs/js-gap-sample.txt
```

**It fetches 28 real pricing pages**, so it takes a minute or two and is polite about it —
one request per host per second, `robots.txt` honoured, everything through the same guard as
Part 8B.

**You should see** a table, then:

```
measured 28   no static price 4   of those, tier 2 recovered 1
residual 10.7% of pages showed no price by either route

That is not the tier-5 number yet. Classify each residual page by hand:
    price needs JavaScript         -> counts toward the tier-5 decision
    page publishes no price at all -> a finding, not a gap. Excluded
```

**The exact numbers will drift** — sites change, and one page in the sample redirects to a
host whose `robots.txt` refuses us. What should hold is the shape: **the overwhelming
majority are `tier 1  static html`.**

**Why it refuses to draw its own conclusion.** The residual mixes two different things: a
page whose price needs JavaScript, and a page that publishes no price at all.
`ARCHITECTURE.md` §5.5's 5% threshold is about the first, and no browser renders a price that
was never written. On the first run those were 3.6% and 10.7% — **opposite sides of the
threshold**. A tool that applied the rule itself would have scheduled a headless browser to
solve "contact sales".

**The last two URLs in the sample are a control group.** They publish no price on purpose. On
the very first run one of them reported a price — the detector matched *"Learn professional
Data and AI tools for free"* — which is how a measuring instrument tells you it is broken.
Open [`docs/js-gap-sample.txt`](js-gap-sample.txt) to see the whole list and why each is
there.

**To point it at your own list**, make a file with one URL per line and `#` for comments:

```bash
cargo run -p landscape -- gap my-urls.txt
```

---

## Part 8D — Find the pages worth reading about a company

The step that comes after knowing *who* a report is about. Needs nothing running.

```bash
cargo run -p landscape -- discover https://basecamp.com
```

**It takes about twenty seconds** — sixteen requests at one per host per second, which is the
politeness rule doing its job rather than the tool being slow.

**You should see** something close to:

```
source                                      answers      found via
------------------------------------------------------------------------
https://basecamp.com/pricing                pricing      sitemap
https://basecamp.com/features               features     sitemap
https://basecamp.com/about                  identity     sitemap
https://basecamp.com/status                 trust        probe
https://basecamp.com/jobs                   direction    probe
https://basecamp.com/security               trust        probe
------------------------------------------------------------------------
6 source(s) admitted from 16 path(s) checked
```

**Read the `answers` column, not the row count.** The cap is 8, and it is spent **round-robin
across the questions a page answers** rather than on the most confident matches. That matters
because ranking by confidence would fill the eight with `/pricing`, `/plans`, `/pricing/` and
two more pricing pages from the sitemap — a report that states the price five ways and has
nothing to say about anything else. [ADR 0010](decisions/0010-spend-the-cap-on-breadth.md).

**Try one that publishes an `llms.txt`:**

```bash
cargo run -p landscape -- discover https://linear.app
```

Some sources will say `llms.txt` in the `found via` column. That is the site naming pages it
would like an automated reader to read — better evidence than a path existing, so it wins the
tie-break within a question.

**Why every source is on the company's own domain.** Those are **Primary** sources, and
`FACT_CHECKING.md` §3.2.1 permits only Primary sources to set a value in a comparison table.
Search returns other people's pages, which can be reported but never tabulated — which is why
probes come first rather than merely being cheaper.

**A thin result is not a failure.** The line saying *16 path(s) checked* is the point: a
negative nobody can check is not a finding, the same rule the report's "nothing found"
sections follow.

**Exact results will drift.** Sites reorganise. What should hold is that the sources are
absolute URLs on that domain, none is listed twice, and the `answers` column shows more than
one kind of thing.

---

## Part 8E — Run the whole path against a real company

**The command that runs every piece in order**, and the one worth watching most closely.

```bash
cargo run -p landscape -- read https://basecamp.com
```

Without a `llama-server` it stops after conversion and tells you so. With one:

```
page                                         words  qual  span   extracted
----------------------------------------------------------------------------------------------
basecamp.com/pricing                         1729   good  210    2 plans found in 2 windows
                                               Pro Unlimited at $299/mo
                                               Pro at $15/mo
basecamp.com/features                        1436   good  867    10 capabilities stated (of 18 the page names)
                                               Message Boards
                                               Hill Charts
                                               Card Tables
                                               Campfire chats
                                               ...
                                               2 name(s) dropped - not words from the section
basecamp.com/about                           -      -     -      no extractor yet for identity pages
```

**Both plans, and both prices are right.** That page publishes `$299/month` Pro Unlimited and
`$15/user` Pro, and until [BENCHMARKS.md](BENCHMARKS.md) Run 7 the second one was invisible —
one window per page meant one plan per page, and a report showing a rival's cheapest plan and
silently dropping the rest does not read as incomplete, it reads as wrong.

**The `span` column is 210 words against a page of 1729.** The model is sent one small window
per plan — [ARCHITECTURE.md](ARCHITECTURE.md) §5.4's span pre-selection. Before those windows
existed this page returned *"no price published"* (Run 5).

**The features page answers the second question**, and it is answered differently. §5.4:
*"Feature lists on structured pages — code first, model for normalization only."* The parser
finds the sections that name something; the model only shortens `## Message Boards for
announcements and discussions` to `Message Boards`.

**Two lines under that answer are the interesting ones.** *"of 18 the page names"* is a cap
being stated rather than applied quietly — twelve read out of eighteen is a short list, and
twelve with no number beside it is a wrong one. *"2 name(s) dropped"* is a check: a capability
name is a paraphrase by design, so the one thing that can be demanded of it is that its words
came from the section. A 4B model that cannot name a section returns `string`, the field's own
type, and that is what this drops.

**Every stage prints what it decided**, and that is the point — a pipeline reporting only its
last step gives a wrong answer six possible causes. The `span` column is how you tell a bad
window from a bad model, which §5.4 warns are otherwise indistinguishable.

**Rows saying "not a pricing page — no extractor yet" are honest, not broken.** Discovery
labels what each page answers, and only pricing has an extractor so far. Running the pricing
extractor over a documentation page produced *"MCP server at $0"* — a plan that does not
exist — which is why it no longer does.

**Try a second company**, where the first plan is free:

```bash
cargo run -p landscape -- read https://linear.app
```

`linear.app/pricing` should report three plans — **Free at $0, Basic at $10, Business at
$16** — which is what the page publishes. You may also see `/plans` skipped with 2 words: the
quality gate refusing to spend a model pass on a page that converted to nothing.

**And try one where the answer is that there is no answer:**

```bash
cargo run -p landscape -- read https://todoist.com
```

Every pricing page reports **"no pricing content on the page"**, and that is correct: Todoist
renders its prices in JavaScript, so what we fetch contains no dollar amount at all. Before
Run 7 this same page reported *"Beginner at $5"* — the 5 was how many personal projects the
plan allows, read out of a feature-comparison table. A page that publishes nothing we can see
is a **finding**, and §5.5's JavaScript-gap counter is what it feeds.

**A changelog needs no model at all**, so this part works with `llama-server` stopped:

```bash
cargo run -p landscape -- read https://www.notion.com
```

```
www.notion.com/releases                      3136   good  8      8 change(s) in 90 days, 0 older
                                               2026-07-31  AI Meeting Notes can now trigger Custom Agents
                                               2026-07-30  High contrast mode
                                               ...
```

[ARCHITECTURE.md](ARCHITECTURE.md) §5.4: *"Dates are the most common LLM fabrication in 'recent
changes' and are trivially verifiable."* So they are parsed, and every date printed is on the
page at a line the code can point to.

**Read the count beside it, not just the list.** `plausible.io/changelog` reports *"4 change(s)
in 90 days, 36 older"* and *"read 40 of 70 dated entries"*. Four entries with no numbers around
them would read as a quiet quarter at a company that ships weekly.

**And `linear.app/docs/releases.md` says "no dated entries on the page"** — it is documentation
about a feature called Releases, not a changelog. [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 is
strict about this distinction: **not "no changes."**

**The fourth question is who they are**, and it is the one where a model is most likely to
answer from memory:

```bash
cargo run -p landscape -- read https://plausible.io
```

```
plausible.io/about                           914    good  195    3 of 3 facts stated
                                               founded 2018
                                               based in EU
                                               10 people
                                               1 answer(s) dropped - not written in the window
```

**Then try one that states nothing**, which is most of them:

```bash
cargo run -p landscape -- read https://basecamp.com
```

`basecamp.com/about` reports *"no stated facts about the company"*, and that is right: the page
says *"We're here for them, 23 years and running"* and never names a year. **2003 is arithmetic,
not reading**, so nothing computes it.

**The dropped-answer line is the interesting one.** A model has read about these companies, and
an about page of pure story is exactly the prompt that invites it to answer from memory. Every
value has to be written in the window it came from — checked field by field, so a correct year
is not thrown away by an invented headquarters beside it.

**And then it ends with a report**, which is the point of all of it:

```
# https://plausible.io
Read 2026-08-04 19:58 UTC · 4 source(s) cited · prompts v1

## Pricing & packaging
no page found. Checked: /pricing (404), /plans (404), /pricing/ (404)

## Company facts
- says it was founded in 2018 [S3·H]
  > Uku Taht started Plausible in December 2018, building it alone…
```

**Everything above the report is a run log** — ordered by page, showing every join, and the
diagnostic that has found a dozen runs' worth of bugs. **The report is ordered by question**,
cites every claim with a source label and a verbatim quote, and puts a coverage note where it
has nothing. The product will have the second one.

`[S3·H]` is the source and the confidence. A quoted price is `H`; a capability name is `M` and
can never be more, because shortening a heading is a paraphrase by design.

**And in a browser, which is where it is judged:**

```bash
npm run dev --prefix web
```

Type `compare plausible.io for me` and watch. The first section appears while it is still
running, and **grows as more facts arrive** — then the finished report replaces it with the
coverage notes for the questions that found nothing.

Both of those behaviours came from watching this exact screen: the first section used to take
four minutes, and then it used to freeze at one item.
[BENCHMARKS.md](BENCHMARKS.md) Run 16 records what that looked like and why.

**Type a description instead** — `an app that helps small farms sell to restaurants` — and it
tells you it could not work out which company you meant, and to name a website. Not *"nothing
you did caused it"*, which was wrong and a dead end.

**And the same thing happens through the API**, which is what the walkthrough uses when there
is no browser to hand:

```bash
cargo run -p landscape -- dev --store memory
```

Then, in another terminal — no database, no setup:

```bash
curl -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{"prompt":"compare plausible.io for me"}'
```

Take the `id` it returns and watch the report being written:

```bash
curl -N localhost:8787/api/analyses/PASTE_THE_ID_HERE/events
```

```
event: status    running
event: section   features    9 claims
event: section   changes    11 claims
event: section   identity    3 claims
event: status    complete
```

**Sections arrive as they are finished, not at the end.** That is
[PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1A — first content in twenty to forty seconds rather
than ninety seconds of spinner.

**Try a prompt that names no site**, and read the failure:

```bash
curl -X POST localhost:8787/api/analyses -H 'content-type: application/json' -d '{"prompt":"an app that helps small farms sell to restaurants"}'
```

It fails, and says why: finding a company from a description needs a search channel that is
not built. **A guessed domain would produce a report that is correctly cited and about the
wrong company**, which is the most expensive wrong answer available here.

**Every run now ends with what it did not find**, which is the half of a report that is
usually missing:

```
question     coverage
----------------------------------------------------------------------------------------------
pricing      2 fact(s) from 1 source(s)
features     10 fact(s) from 1 source(s)
changes      no page found. Checked: /changelog (404), /releases (404), /blog (404)
identity     1 page(s) found and not read - our gap, not theirs
trust        2 page(s) found and not read - our gap, not theirs
```

**Basecamp publishes no changelog**, and that line is what makes the claim checkable — three
paths, three answers, all of which you can try yourself.
[FACT_CHECKING.md](FACT_CHECKING.md) §5.4: *a negative nobody can check is not a finding.*

**Read the difference between the last two kinds of line.** *"No page found"* is a fact about
the company. *"Found and not read"* is a fact about us — identity, trust and direction have no
extractor yet, so their pages are admitted and never opened. Reporting those as *"we read it
and it said nothing"* would be this feature committing the error it exists to prevent, and the
first version of it did exactly that for one run.

**Three things are still wrong, and all are visible if you look:**

```bash
cargo run -p landscape -- read https://www.notion.com
```

Notion reports four plans where three are real — `### Custom Agents` is an add-on, and an
add-on is a heading with a price under it, which is exactly the shape a plan has. You may also
see a billing period that disagrees with the page: `$16 per user/month` and `Billed yearly`
are two facts and `BillingPeriod` has one field for them.

Both are in [ROADMAP.md](ROADMAP.md) rather than tuned away.

**What discovery chose is worth looking at on its own**, and it needs no model:

```bash
cargo run -p landscape -- discover https://linear.app
```

`linear.app/features` should be in that list. Until Run 9 it never was — `/docs/mcp.md`, a
setup guide, held the features slot and reported `Setup` and `Claude` as capabilities of
Linear. Only the front page of a documentation site answers *what does this product do*.

```bash
cargo run -p landscape -- discover https://todoist.com
```

**Three sources from eighteen paths checked, and that is the improvement.** Before Run 9 this
was eight, of which two were Todoist's pricing page in Czech and Danish and two more were the
`setup` and `upgrade` steps of buying it. The English pricing page was never read at all. A
source that cannot answer the question it was admitted for costs a fetch, a model pass, and a
line in the report that nobody can stand behind.

**If a page reports nothing, look at what it converted to** — no model needed:

```bash
cargo run -p landscape-extract --example md -- saved-page.html
```

That is how `linear.app/docs/mcp.md` was found coming through as a single 2,167-word line.

**Try the conversion on its own**, which does work and is worth seeing:

```bash
cargo run -p landscape -- fetch https://basecamp.com/pricing
```

That prints the page's size; the Markdown behind it keeps the headings and tables that
`text::visible` throws away — which is the whole reason the module exists.

---

## Part 9 — What the tests prove

```bash
cargo test
```

**You should see 427 passing, with nothing running.**

CI runs the same tests through `cargo nextest run`, which is faster and gives each test its
own process — so a test that panics is reported as a failure instead of taking the run down
with it. If you want CI's exact behaviour locally:

```bash
cargo install cargo-nextest --locked
cargo nextest run --all-features
```

**`485 tests run: 485 passed, 6 skipped`** — the six are the `#[ignore]`d ones that need a
database or a model. `cargo test --all-features --doc` runs alongside it, because nextest
does not run doctests.

Worth knowing what a few of them are actually for:

| Test | What it protects |
|---|---|
| `memory_store_satisfies_the_contract` | One contract body, run against both the in-memory and Postgres stores, so the fast tests cannot slowly become fiction |
| `an_internal_failure_never_leaks_detail` | Asserts a database password cannot appear in a response body |
| `no_reader_description_judges_the_publisher` | Enforces the language rule from `FACT_CHECKING.md` — we say what we confirmed, never that a site is bad |
| `length_is_measured_in_characters_not_bytes` | Eight accented characters are 16 bytes; measuring bytes would reject them and accept the ASCII equivalent |
| `every_documented_curl_actually_works` | Runs every command in `README.md` against the real binary |
| `every_expected_answer_is_actually_on_the_page` | Checks the golden set's own answers, so a correct model cannot be scored wrong and then "fixed" |
| `every_property_becomes_required` | A model that stops writing after two keys parses as a model that found nothing. This is what stops that |
| `filling_in_a_price_the_page_does_not_state_is_a_fabrication` | The scoring rule the whole golden set turns on, tested without a model |
| `every_frozen_page_still_reads_the_way_it_did` | Ten real pages and what the parsers must make of them. Sixteen defects were found by reading output by hand; this is the half of that a build can do |
| `the_subjects_carved_from_real_pages_are_still_verbatim` | A subject a model keeps failing invites a quiet edit to its page text, and a subject edited until it passes measures nothing |
| `a_slow_write_never_overwrites_a_newer_report` | Progress writes used to race. The store could end up holding the older report, so a correction a reader had already seen was undone in front of them |
| `a_reader_watching_a_reclaimed_run_is_never_told_it_finished` | A reclaimed run goes `running` → `queued` → `running`. A stream that ended on "not running any more" would tell a reader it was finished, and a reader told that does not reconnect |
| `after_a_retraction_the_same_answer_is_sent_again_rather_than_suppressed` | The stream skips a payload it has already sent. Once a reader's screen is cleared, "already sent" is no longer true — and a replacement reaching the same answer would be suppressed and never appear |
| `every_claim_still_points_at_its_own_source` | Each company's run numbers its sources from `S1`, so merging without renaming makes a reader following a citation for one company's price arrive at another's page |
| `every_response_carries_a_request_id_including_the_page` | ADR 0005's invariant on the surface a visitor actually touches. `Router::layer` wraps the routes present when it is called, so a fallback added afterwards is the one response with no id and no access line |
| `a_deep_link_reaches_the_app_rather_than_a_404` | The binary serves only `/api/*` until this exists, so a deployment is an API nobody can see. The page owns its own routing, so a permalink has to reach the client rather than 404 |
| `a_run_told_to_stop_does_not_call_the_model_again` | A worker the queue has replaced used to read every remaining window for a report nothing would accept. Twelve model calls become one, counted against a stub server so it runs without a GPU |
| `a_window_the_model_could_not_answer_is_still_a_chance_to_stop` | The cancellation check used to sit inside the branch where a window succeeded, so a run whose model calls were failing read every window anyway — and a failing run is the slow one the sweep takes away |
| `a_run_nobody_stops_reads_every_window` | The control: a stop that fires always would look like a saving in every number, and would turn every report into its first window |
| `memory_store_satisfies_the_contract` (the revoked-claim half) | Two live workers on one analysis both see `running`. The generation is the only thing that can tell them apart, so a worker whose claim was swept away cannot write over the run that replaced it |

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
| 2 — web UI, submit, completes, box clears | | |
| 2 — short prompt rejected in the UI, box NOT cleared | | |
| 3 — API create and read back | | |
| 3A — the same request in your shell, if on Windows | | |
| 3B — request id in the header matches the log line | | |
| 3B — a forged id (`aaa bbb`) is replaced, not echoed | | |
| 4 — report shows `checked`, no invented claims | | |
| 5 — all seven rejections carry a remedy | | |
| 6 — `serve` alone leaves it queued | | |
| 7 — survives a restart with Postgres | | |
| 8 — 100 generations, 0 parse failures | | |
| 8 — `landscape-bench` reports three error kinds separately | | |
| 8A — golden set validates itself, 32 passing, no model | | |
| 8A — `--test the_pages` passes: ten frozen pages, no model, under a second | | |
| 8A — scorecard prints; `FAB` on the abstention subjects reads as expected | | |
| 8B — example.com fetches; metadata endpoint and loopback refused | | |
| 8B — Google `/search` refused by robots, `/robots.txt` allowed | | |
| 8C — most pages report `tier 1  static html` | | |
| 8C — the report refuses to apply the 5% rule itself | | |
| 8D — discovery returns absolute URLs, no duplicates | | |
| 8D — the `answers` column shows several different kinds | | |
| 8E — every stage prints a verdict, not just the last | | |
| 8E — basecamp `/pricing` reports **both** of its plans, priced | | |
| 8E — linear `/pricing` reports three, starting at $0 | | |
| 8E — todoist says "no pricing content" instead of guessing | | |
| 8E — basecamp `/features` names ten real capabilities | | |
| 8E — the cap and the dropped names are printed, not hidden | | |
| 8E — pages of the other four kinds say "no extractor yet" | | |
| 8E — `discover linear.app` lists `/features`, not a setup guide | | |
| 8E — `discover todoist.com` lists three English pages, no duplicates | | |
| 8E — notion `/releases` lists dated changes, with no model running | | |
| 8E — linear `/docs/releases.md` says "no dated entries", not "no changes" | | |
| 8E — plausible `/about` states three facts, all correct | | |
| 8E — basecamp `/about` states none, and says so | | |
| 8E — every run ends with a coverage line per question | | |
| 8E — and then a report, with every claim cited and quoted | | |
| 8E — the same report through the API, streamed section by section | | |
| 8E — a prompt naming no site fails with a reason, not an empty report | | |
| 8E — in a browser: a section appears mid-run and grows as facts arrive | | |
| 8E — a description gets told to name a website, not that nothing was its fault | | |
| 8E — basecamp's `changes` line names the three paths tried | | |
| 9 — `cargo test` green with nothing running | | |

**If something differs from what is written here, that is a bug.** Every command above was run
on this branch before it was written down. The most useful report is the command, what you
expected from this document, and what you actually got.
