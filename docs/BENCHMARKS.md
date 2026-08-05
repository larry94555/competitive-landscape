# Landscape — Benchmarks

> Real numbers, with the hardware they came from. `docs/ROADMAP.md` Phase 0 requires this
> file before a model is chosen, because every latency figure in the specification is
> otherwise an estimate.
>
> **Nothing here is measured on the deployment host, and nothing will be** —
> [ADR 0011](decisions/0011-no-experiments-on-production.md). A benchmark leaves a toolchain,
> a downloaded model and a set of assumptions behind in the thing customers depend on. What a
> laptop cannot answer — how long somebody waits — is answered from the client's side of a
> deployment, not from inside the box. These are
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

## Run 20 — stopping work nobody wants

**Date:** 2026-08-04 · **Where:** this laptop · **Model:** a stub, on purpose — see below.

Run 19 gave a claim a number, so a worker the staleness sweep has replaced is refused at every
write. It said plainly what it did not do:

> A replaced worker keeps working. It finds out at its next write — within a page for progress,
> at the very end for `complete` — and until then it is spending prefill on a report that will
> be discarded.

**Prefill is the only scarce resource this machine has.** A run is 90–180 seconds of it, and a
replaced worker was spending all of the remainder on a report nothing would accept.

### The stop

The progress callback returns an answer now:

```rust
pub enum Wanted { Yes, No }
```

`Wanted::No` breaks the loop — the page loop in `analyse_with`, and the window loop in each of
the three model-backed stages. **The callback is the right place for it because the answer is
already there**: a worker learns its claim is gone *by writing*, and the progress callback is
the write.

The answer `record` gives is as of the last write that **landed**, so it lags by a window. That
is soon enough: what it saves is everything the run had not started yet.

### Counting what it saves

A model call is the expensive thing, so a model call is the unit. The stub `llama-server`
answers instantly and counts requests — and for this question a stub is a *better* instrument
than a real model, not a worse one: the question is not what the model says, it is **how many
times it is asked**. It also runs on every pull request, which `against_a_model` cannot.

Against the real Basecamp features page — twelve capability windows, the page that made a reader
wait four minutes in Run 16:

| | model calls |
|---|---|
| told to stop after the first answer | **1** |
| nobody stopping it | **12** |

And the Notion pricing page, five plan windows: **1** against **5**.

### Does it hold up?

Five ways of getting it wrong, put back one at a time:

| Reintroduced defect | Caught? |
|---|---|
| the features loop ignores the answer | **yes** |
| the pricing loop ignores the answer | **yes** |
| `record` never reports the claim gone | **yes** |
| the writer never sets the flag | **yes** |
| *every* run stops after one window | **yes** — the control |

That last row is the one that matters most. A stop that fires always looks like a saving in
every number above, and would quietly turn every report into its first window.

| | tests |
|---|---|
| Run 19 | 456 |
| now | **460** |

### Why the stub, and not the pipeline

`analyse_with` needs a `Fetcher`, whose SSRF guard refuses loopback **on purpose** — so a local
page server is unreachable by design, and the whole-pipeline version of this test cannot be
written without a seam that does not exist. `stages::extract` takes the Markdown directly, which
is where every model call is made, so that is where the counting happens.

### What is still not right

**The page-loop break has no test.** Four of the five mutations above are caught; the fifth
thing — `analyse_with` breaking *between pages* rather than between windows — is one line, and
nothing exercises it, for the reason in the paragraph above. It is subsumed in practice (a
revoked claim stops the window loop first) but that is an argument, not a test.

**Nothing measures how often this fires.** The saving is real and unquantified: twenty minutes
against a 90–180 s analysis makes a revoked claim rare, and there is no counter that would tell
us if that changed.

**The identity stage's loop is not asserted**, only pricing and features. It has the same break
and at most three windows, so it is the cheapest of the three to get wrong and the least
expensive to have wrong.

---

## Run 19 — telling two workers apart

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — this is the queue, the store
and the stream.

Run 18 left two things open and said so. Both were the same hole seen from different ends, and
this closes it: **there was nothing that identified a claim.**

The sweep returns a row that has been `running` for twenty minutes to the queue, because a row
that has been running that long is probably a dead worker. It cannot tell a *dead* worker from a
*slow* one — nothing can — so when the slow case happens, two live workers are running the same
analysis and `status` says `running` for both. Whichever finished last won, and a reader got
whichever report that was, with nothing anywhere recording that two had been produced.

The reader's side is the same gap: Runs 17 and 18 chased a dead worker's sections off the screen
twice, and each fix was a server-side judgement about **the connection** — *the report went
away*, *this connection has already sent something*. A reader's sections survive a reconnect on
purpose, and a fresh connection remembers nothing, so both were correct until the reader
reconnected and wrong immediately after.

### A claim is a number

Every row carries a `generation`: how many times the run has been started. `claim_next` raises
it, `reclaim_stale` raises it, and a worker quotes the number it was given on every write.

```text
save_progress(id, generation, report) -> Applied::Yes | Applied::ClaimRevoked
```

`ClaimRevoked` is an outcome and not an error, because the sweep doing its job is not a failure.
The worker is told once, at the end, rather than per write.

[ADR 0012](decisions/0012-a-claim-is-a-number.md) has the reasoning. The short version is that
**a state describes the row and a number identifies the attempt**: "running" is true no matter
which of two workers is running it, so no test on `status` can ever separate them.

### And the number goes to the reader

The stream announces it, and the **client** compares against the one it is holding — which is
the part the previous two attempts could not do, because only the client knows what the client
is showing.

It also fixes something subtler that fell out while testing. `running` → `queued` → `running` is
an *edge*, and the stream polls twice a second: in one of these tests the sweep and the
replacement claim both landed between two polls, and **the status never said `queued` at all.**

| | survives a reconnect | survives a poll gap |
|---|---|---|
| watching for `running` → `queued` | no | **no** |
| the report going to `NULL` | no | yes |
| a generation the client compares | **yes** | **yes** |

A design that watched for the transition would have missed exactly the restarts that happened
quickly.

### Does any of it hold up?

Eight ways of getting it wrong, put back into the code one at a time:

| Reintroduced defect | Caught? |
|---|---|
| claiming does not raise the generation | **yes** |
| the sweep does not raise the generation | **yes**, 2 tests |
| `save_progress` ignores the generation | **yes** |
| `complete` ignores the generation | **yes** |
| the stream never says which run it is watching | **yes**, 3 tests |
| a new generation does not clear the sent-payload memory | **yes** |
| the client ignores a change of generation | **yes**, 3 frontend tests |
| the recovery fetch does not check the generation | **yes**, 1 frontend test |

| | Rust tests | frontend tests |
|---|---|---|
| Run 18 | 454 | 27 |
| now | **456** | **27** |

The frontend count is unchanged because the three tests that covered this were rewritten rather
than added to: they used to send a `reset` event and now send a generation, which is the same
scenario asserted against a better signal.

### What is still not right

**A replaced worker keeps working.** It finds out at its next write — within a page for progress,
at the very end for `complete` — and until then it is spending prefill on a report that will be
discarded. Stopping it needs cancellation threaded into `analyse_with`, which changes the
pipeline's shape rather than the queue's and is worth doing on its own.

**Postgres was not exercised here.** Docker would not start on this machine, so the `generation`
column, the four `WHERE generation = $n` predicates and the two-query failure path are verified
by CI's `rust (against postgres)` job rather than by me. The in-memory store and the Postgres
store share one conformance body, which is what makes that a reasonable division of labour, but
it is not the same as having run it.

**The replacement wins because it holds the current number, not because its report is better.**
That is the right default — the sweep exists precisely because the older claim looked dead — but
it is a policy, and nothing measures how often it fires.

---

## Run 18 — the states a run only reaches when something goes wrong

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — every defect here is in the
worker, the store and the stream.

Run 17 froze what a *page* yields. This is the other gap it left, and the one the
coding-mistakes register argues is larger: **nine of that register's eleven entries are states
that only exist when something goes wrong mid-operation**, and three of them were introduced by
the fix for the previous one. A suite built from complete runs cannot see any of them, because a
complete run never enters them.

So: drive the orchestrator through a worker that dies, a run that is reclaimed, and a store that
is slower than the pipeline feeding it. **Two defects, both invisible to 442 passing tests.**

### A slow write overwrote a newer report

The worker spawned a task per progress snapshot, with this reasoning beside it:

> Ordering is not a concern because each write is the whole report so far, not a delta.

**That is true of the payloads and false of the writes.** Two spawned tasks are two concurrent
round-trips over two pooled connections, and nothing makes the first land first. Given a store
whose first write is slow:

```text
writes landed ["second", "first"]
  left: "first"
 right: "second"
```

The store keeps the **older** report. So a section that had four claims goes back to three, and
`PagePricing::assembled` replacing *"Free is listed with no published price"* with *"Free costs
$0"* is undone in front of a reader who already saw the correction. It repairs itself at
`complete` — which means the window where it is visible is exactly the ninety seconds somebody
is watching.

This is Run 16's `claims.len()` defect wearing a different hat: a correction that arrives and
then un-arrives.

Progress now goes through one writer task fed by a `watch` channel. Writes are strictly ordered,
and a run producing snapshots faster than the store can take them **coalesces** rather than
queueing — every snapshot is a whole report, so the newest is strictly better than a backlog of
stale ones, and a queue that grows under load is how a slow database becomes a slow reader.
`finish()` waits for the write in flight before `complete`, so the final report cannot be
overtaken by a partial one.

### A reclaimed run kept the dead worker's half-report

`reclaim_stale` returns a stranded row to the queue, which is right. It also left the partial
report attached, which is not: a **queued** analysis served sections from a worker that no
longer exists, and the run that replaced it started from an empty report — so a reader watching
saw answers they had already been shown blank themselves out. That reads as a retraction rather
than a restart.

Both stores now clear the report with the requeue, and the assertion lives in the shared
conformance contract so Postgres and the in-memory store cannot disagree about it.

### What review found: clearing the row is not retracting the answer

The fix above was half of one. `reclaim_stale` clearing the partial report fixes a fresh `GET`
and a reconnection — and does nothing for **the connection that is already open**, which has
already sent those sections. `sent_sections` still held them, the transition sent only
`status: queued`, and the client's `onStatus` left its section state untouched. Reproduced
through `App`: after a `$15` section and then `queued`, the dead worker's claim stayed on screen.

The interval is not small. The row has to be picked up by a worker polling once a second, then
discovery and a fetch and a model call before the replacement has anything to say — call it
thirty to ninety seconds. And **if the second run never reaches that question**, because the page
404s this time or discovery picks different pages, nothing overwrites that key and the retracted
claim stays until the run ends.

The test could not see any of it, because it asserted the *eventual* value was the second run's.
The defect is entirely in the interval.

So the stream now sends a **`reset`** when the report goes away under a live connection, and
clears `sent_sections` with it. The client throws away both copies it holds: the sections the
stream sent, and the partial report a recovery fetch cached on the analysis.

`reset` rather than `done` matters. A reader told a run finished does not reconnect, and a
reclaimed run is the opposite of finished — *keep watching, and forget what you have*.

**And review found the fix for that was itself half a fix.** It sent the retraction only when
*this connection* had already sent something — and the reader's sections survive a reconnect on
purpose, while the server's record of what it has sent starts empty on every new stream. Drop,
reclaim, reconnect, and the guard suppressed the retraction on the one connection that needed it.

The condition is the **row's** state now, not the connection's: *no report on the row means
nothing backs what the reader holds*, whoever is connected. The same rule is applied at the other
boundary, in the recovery fetch, which is the first thing to find out in that sequence. An
ordinary new analysis is also report-less, so a run now opens by retracting nothing — correct,
and free.

Once it fires per *episode* rather than per poll, something has to rearm it. Nothing tested that,
and a flag that is set and never cleared works for the first reclaim and is silent for the
second — so a run that loses two workers leaves the second one's answers on screen with no sign
anything is wrong. There is a test for two reclaims on one connection because breaking it on
purpose was the only thing that noticed.

Clearing `sent_sections` is not tidiness either: without it, a replacement run that reaches the
**same** answer would be suppressed as a duplicate, and a reader whose screen had just been
cleared would sit in front of an empty section for the whole second run. That is the failure mode
of every de-duplicating cache — correct until the thing it deduplicates against is cleared
somewhere else.

### Are the new tests worth anything?

Same question as Run 17, same method — break it on purpose and see:

| Reintroduced defect | Caught? |
|---|---|
| the stream treats any non-`running` status as the end | **yes** |
| a failed run closes the connection without a `done` | **yes**, 2 tests |
| empty sections are streamed before their question is answered | **yes** |
| a reclaimed run keeps the dead worker's partial report | **yes** |
| the row is cleared but the wire sends no retraction | **yes**, 2 tests |
| the retraction is sent but the already-sent memory is kept | **yes** |
| the memory is cleared but the reader is never told | **yes**, 2 tests |
| a `done` is sent instead of a retraction | **yes**, 2 tests |
| the client ignores the retraction | **yes**, 2 frontend tests |
| the retraction is guarded on the connection rather than the row | **yes**, 2 tests |
| it fires on every poll instead of once per episode | **yes** |
| the flag never rearms, so a second reclaim is silent | **yes** |
| the recovery fetch keeps stale sections across a reconnect | **yes**, 1 frontend test |

The first row is the one worth pausing on. A reclaimed run goes `running` → **`queued`** →
`running`, which is a *backwards* status transition no healthy analysis ever makes. A stream
that ended on "not running any more" would tell a reader the report was finished when a second
worker was about to start it — and a reader who has been told it finished does not reconnect.

Until now the stream's tests all checked *helpers*: does this section serialise, is a correction
a different payload. The loop itself — where all four of Run 16's defects lived — was driven by
nothing. It now runs against the real router with the store being mutated underneath it.

| | Rust tests | frontend tests |
|---|---|---|
| before | 442 | 24 |
| after | **454** | **27** |

### What is still not right

**There is no claim token, so a slow worker can still finish over its replacement.** If a run
takes longer than the twenty-minute staleness threshold, the sweep hands it to a second worker
while the first is alive; the first then calls `complete` and its report wins. `save_progress`
is guarded — both stores refuse a write to a row that is not `running` — but `complete` cannot
tell the two workers apart, because nothing identifies a claim. Fixing it properly is a
generation column threaded through `claim_next`, `save_progress` and `complete`, which is a
schema change and its own piece of work rather than a line in this one. Twenty minutes against a
90–180 s analysis makes it unlikely, not impossible.

**Nothing drives a worker that dies *while* the pipeline is mid-page.** These tests drive the
store and the stream through that sequence; the worker's own loop is still only exercised by a
run that completes.

**The 500 ms poll is in the tests' wall-clock.** Four tests take 2.5 s of the suite's 6 s. That
is worth it here and would not be at ten times the count.

---

## Run 17 — the sixteen defects, written down, and a seventeenth found while writing them

**Date:** 2026-08-05 · **Subjects:** ten frozen pages from six companies · **Where:** this
laptop · **Model:** none, and that is the point.

Runs 5 to 16 found sixteen defects. Every one was found the same way — point the pipeline at a
real company, read the output, notice something wrong — and **not one of them is asserted
anywhere.** The pipeline could regress to its Run 5 behaviour today and 437 tests would pass.

The half of that method which needs no GPU is now a test. Ten real pages are frozen as
Markdown in `crates/landscape-golden/pages/` (222 KB with their expectations), each beside a
JSON file saying what the deterministic passes must make of it: **every plan window a pricing
page yields, heading and body**; every capability window a features page names; the date, title
and quote of every entry a changelog carries; the window each fact on an about page was read
from.

```bash
cargo test -p landscape-golden --test the_pages
```

**0.11 s, no model, no network.** It runs on every pull request; `against_a_model` still cannot.

### Does it actually catch anything?

A golden set that has never failed is a set nobody has calibrated. So five of the sixteen
defects were put back into the code, one at a time, and the set was asked:

| The defect, as it originally was | Caught? |
|---|---|
| Run 7 — the scoring floor that let Basecamp's FAQ outrank both its plans | **yes**, 2 pages |
| Run 7 — a window with no price in it could still win | **yes**, 1 page |
| Run 8 — a page footer read as fourteen capabilities | **yes**, 1 page |
| Run 12 — the identity window's floor | **yes**, 1 page |
| Run 10 — a date mid-sentence filed as a shipped change | no |
| *(added after review)* the window body silently shortened, `WINDOW_CHARS` 1600 → 900 | **yes**, 1 page |

**Five of six.** The one that gets away is caught by a unit test in `landscape-extract` that
already existed, so the suite as a whole stops all six — but no frozen page carries a line that
begins with a date and runs into prose, and that is a real gap rather than a rounding error. It
is recorded here rather than papered over.

The last row is there because of what review found, below.

The failures name the page, quote its reason for being in the set, and print expected against
got for every page that moved, not just the first:

```text
2 of 10 frozen pages no longer read the way they did:

basecamp-pricing.md  (Pricing)
  why it is here: Two plans on one page, which is the finding Run 7 exists for…
  plan windows
  expected ["## Pro Unlimited", "## Pro"]
  got      ["## Pro Unlimited", "## Pro", "## I have pricing questions&hellip;"]
```

### What review found: freezing the heading is not freezing the window

The first version of this froze **only which headings won**. Review took one try to show what
that leaves out: change Linear Business from `$16 per user/month` to `$999` in the frozen page,
and all five tests stay green. Heading selection was unchanged, so nothing noticed.

That is the wrong half. §5.4's argument is that a bad window and a bad model **cannot be told
apart at the output**, which is the entire reason span selection is a stage — and a set that
freezes the label on a window but not its contents asserts the cheap part. Every expectation now
carries the body, stored one line per entry so a change reads as a changed line in review:

```text
linear-pricing.md  (Pricing)
  the window under `### Business`
  line 1
    expected "$16 per user/month $16 per user/month"
    got      "$999 per user/month $999 per user/month"
```

The `WINDOW_CHARS` row in the table above is the mutation that measures this, and it is one the
first version could not have caught: shrinking a window does not change which heading wins.

Review found a second thing in the subjects — the Linear Free subject expected
`billing_period: monthly` from a window that states only `$0` and *"Free for everyone"*. The
cadence belonged to the plan **below** it. That expectation would have marked the honest null
answer wrong and rewarded a model for copying a neighbour's period, which is not a hypothetical
failure: Run 3's scorecard has a model reporting `monthly` for plans with no published price at
all. The subject expects `null` now, and a new check asserts every expected period is stated in
**the part of the page about the plan being asked**, not merely somewhere on it.

### The seventeenth defect, found by building the set

Cloudflare's changelog was fetched as a tenth page. It reported **0 entries out of 0
considered** — a page of two dozen releases read as a page with no dates on it.

Every date on it is a `<time datetime="2026-08-04">Aug 4, 2026</time>`, which converts to:

```text
2026-08-04 Aug 4, 2026
```

Run 10 added a rule that a date must be followed by a separator or nothing, because *"Starting
August 11 2026, Workers will run…"* is a promise, not a release. Here the thing following the
date is **the same date, written for a person** — and the rule read it as prose and threw the
line away. The most explicitly machine-readable date shape on the web was the one we could not
read.

| | entries read |
|---|---|
| before | **0** of 0 |
| after | **24** of 24 |

Two renderings of one date are now stepped over as one date. The guard is untouched for every
other case, including the two-different-dates line — `2026-08-04 Aug 10, 2026 scheduled
changes` is a publication date and a scheduled date, and which the entry belongs to is exactly
what this parser refuses to guess.

### The model-facing set, from real pages at last

`docs/ROADMAP.md` has said since Run 5 that the new subjects **should be fetched pages, not
written ones** — because the synthetic set was predicting a performance the pipeline does not
have. Five of the fifteen subjects now are: each `source` is the verbatim window
`span::every_plan` cuts from a frozen page, so the model is scored on the text it will actually
be handed.

| Subject | From | The thing it makes hard |
|---|---|---|
| `flat-price-for-the-whole-company` | Basecamp Pro Unlimited | $299/month flat, beside a storage figure written as `10x Pro` |
| `per-user-price-among-service-numbers` | Basecamp Pro | $15/user competing with 500 GB, `24/7/365` and two trial lengths |
| `free-tier-with-real-quotas` | Linear Free | `$0` competing with 2 teams and 250 issues |
| `price-buried-under-its-own-feature-list` | Notion Plus | the answer is line two; eighty words of features follow |
| `a-pricing-page-that-states-no-price` | Todoist | says `per user/month` with **no figure** — the page that produced `Beginner at $5`, where 5 is a project limit |

A test asserts they are still verbatim in the frozen pages, because a subject a model keeps
failing invites a quiet edit to the page text, and a subject edited until it passes measures
nothing.

### What this run does not claim

It measures **us**, not a model. No accuracy number moved, because none was taken — the model
half of the set still needs a `llama-server` and is still `#[ignore]`d. Ten pages from six
companies is not a representative sample of the web, and four of the ten answer the pricing
question. The claim is narrower and worth having anyway: the specific things sixteen runs of
hand-reading found now fail a build instead of being remembered.

### What is still not right

**The date-in-prose rule has no frozen page**, per the table above.

**Nothing freezes a page against the model half.** The five real subjects are scored only when
somebody runs the bake-off by hand.

**Four of ten pages are pricing.** Changes has three, features two, identity one — and identity
is the extractor with the fewest rules and the most guesswork.

---

## Run 16 — watched in a browser, which found two things the tests could not

**Date:** 2026-08-04 · **Subject:** plausible.io · **Where:** a browser, on this laptop ·
**Store:** in-memory.

Run 15 measured the stream with `curl`. This is the same run watched the way a reader watches
it — [ADR 0011](decisions/0011-no-experiments-on-production.md)'s rule that testing happens
from the client's side, applied to our own interface for the first time.

Both defects below were invisible to 425 passing tests and to the `curl` transcript.

### The first section took four minutes

`PRODUCT_SPEC.md` §2.1A asks for content in twenty to forty seconds. The screen said *"Reading
the first pages…"* for **four minutes**.

Nothing was broken. Progress was written **per page**, and `plausible.io`'s first page is
twelve capability windows read one at a time at ~15 s each. A page is simply too large a unit
to deliver anything.

Progress is per *window* now — one fact is enough to put a section on screen:

| | first section on screen |
|---|---|
| per page | **~4 min** |
| per window | **~2 min** |

Still short of §2.1A, and the remainder is now visible rather than hidden: ~20 s of discovery,
a fetch, and one model call. What would close it is reading the cheap questions first — the
changelog needs no model at all.

### Then the section froze

With the first fix in, the screen showed *"What it does — 1 item"* and **stayed that way for
two minutes** while eight more capabilities were read.

A section was sent once, when it first had claims. **A section that arrives and then stops
changing reads as a section that is finished**, so a reader would have taken nine capabilities
for one. It is now sent again whenever it grows, and the client replaces rather than skips.

### What the finished report looks like to a reader

```text
compare plausible.io for me
Searched as https://plausible.io                      Done.

Pricing & packaging     Nothing found in public sources.
                        /pricing (404) · /plans (404) · /pricing/ (404)
What it does            9 capabilities, each quoted
Recent public changes   11 dated entries
Company facts           founded 2018 · based in EU · 10 people
Trust & security        Nothing found. /security (200) · /status (200)
```

Every claim carries its source label and the page's own words underneath.

### And failures got an audience

A prompt that names no company used to reach the reader as *"This one did not finish. Nothing
you did caused it."* — which was **both wrong and a dead end**, because they had done
something and could fix it.

`core::Failure` is a closed set of situations, not the operator's text:
`migrations/0001_init.sql` is explicit that `failure_reason` is never shown verbatim, and what
somebody is told is a presentation decision. The interface now writes:

> We could not work out which company you meant. Try naming its website — for example,
> basecamp.com.

### Two more the review found

Neither would have shown up in a happy-path run, and both are the same class of thing — a
reader left looking at something that is no longer true.

**A partial report is not a finished one.** The client treated *"we have a report"* as
settled, but `save_progress` gives a running analysis a report from its first fact. So when a
stream dropped or hit its ten-minute cutoff, the one recovery fetch came back *running*, the
client called it settled, and nothing ever reconnected — a half-written report, for ever.
Settlement is a **terminal status** now, and a non-terminal `done` reopens the stream after a
second.

**A section can change without growing.** The stream tracked `claims.len()`, and
`PagePricing::assembled` replaces a plan when a later window supplies the price the first one
lacked: *"Free is listed with no published price"* becomes *"Free costs $0"* at the same
length. The correction was suppressed and the retracted claim stayed on screen until the run
ended. It compares the payload now.

**And two more in the reconnect that fixed the first.** The recovery fetch stores a *partial*
report, and the view preferred `report.sections` the moment one existed — so the page froze at
the instant of the fetch, ignoring everything the reconnected stream sent, and rendered the
partial report's placeholder sections as *"Nothing found in public sources"* for questions
still being read. Mid-run the view now merges the stream with only those fetched sections that
have claims. Separately, the stream's last word before a drop is *"running"*, and it outranked
a recovery fetch that came back **complete** — a finished report under a "Reading…" line. A
terminal stored status now wins.

**Four defects, all in the same twenty lines, none visible to a passing test suite.** The
pattern is worth naming: every one of them is a state that only exists when something goes
*wrong mid-run*, and nothing about the happy path exercises it.

### What is still not right

**Two minutes is not forty seconds.** The order pages are read in decides what a reader sees
first, and nothing chooses it deliberately.

**A reload loses the analysis.** There is no URL for one, so a refresh mid-run leaves a reader
with nothing to return to.

**Nobody has watched this on a slow connection or a phone**, which is where a stream that
depends on a proxy not buffering is most likely to behave differently.

---

## Run 15 — the queue, end to end, with a reader watching

**Date:** 2026-08-04 · **Subject:** plausible.io · **Store:** in-memory, no database ·
**Model:** Qwen3-4B Q4_K_M.

Every previous run drove the pipeline from `landscape read`, a command. This one goes through
the product's own path: **POST an analysis, and watch the stream.**

```bash
cargo run --release -p landscape -- dev --store memory
curl -X POST localhost:8787/api/analyses -H 'content-type: application/json' \
     -d '{"prompt":"compare plausible.io for me"}'
curl -N localhost:8787/api/analyses/<id>/events
```

```text
event: status    queued
event: status    running
event: section   features    9 claims
event: section   changes    11 claims
event: section   identity    3 claims
event: status    complete
event: done
```

**Three sections arrived while it was still running.** `PRODUCT_SPEC.md` §2.1A asks for first
content in twenty to forty seconds rather than everything at ninety, and this is the shape of
it: a reader watches a report fill in.

The finished report:

| section | |
|---|---|
| pricing | *no page found. Checked: /pricing (404), /plans (404), /pricing/ (404)* |
| **features** | **9 claims** |
| **changes** | **11 claims** |
| **identity** | **3 claims** |
| trust · direction | *pages found and not read — our gap, not theirs* |

Four sources cited, every claim traceable.

### What the worker does with a prompt it cannot resolve

`NewAnalysis` takes free text, and the pipeline reads a **site**. `FACT_CHECKING.md` §3.1's
entity resolution needs candidates, candidates come from search, and §3.3 puts search last on
purpose. So the worker reads a URL or a bare domain out of the prompt, and when there is none:

```text
status: failed
reason: this prompt does not name a website, and finding one from a description needs
        the search channel that is not built yet (FACT_CHECKING.md §3.3).
        Try a domain — for example: basecamp.com
```

**A failure, not an empty report.** Guessing a domain from *"an app that helps small farms
sell to restaurants"* would produce something correctly cited and about the wrong company,
which is the most expensive wrong answer this product can make.

### Streaming without a broker

The worker and the API are two processes sharing one row. A message broker between them would
deliver events a few hundred milliseconds sooner and would be **a second piece of
infrastructure to run, supervise and lose events through** — on a box with no spare memory
([ADR 0005](decisions/0005-observability-on-a-24gb-box.md)).

So the endpoint reads the row every 500 ms and sends the difference. The queue already lives in
the database for exactly this reason. **A reader cannot tell**, and there is one fewer thing to
fail.

Two rules fell out of writing it:

- **A section is sent once**, when it first has claims. A section that is still empty carries a
  *"we found nothing"* note that is only true at the end, and sending it early would tell a
  reader we had finished looking.
- **A late write cannot resurrect a finished row.** Both stores refuse progress unless the
  analysis is still running — a worker that lost a race must not overwrite the report somebody
  is reading.

### What is still not right

**The frontend does not consume the stream.** The endpoint exists and `web/` still polls.

**`searched_as` is the origin, not a resolution.** Until search exists, what we searched for
and what the user typed are the same string in every report.

**Nothing measured how long the reader waited.** That is the client-side measurement
[ADR 0011](decisions/0011-no-experiments-on-production.md) describes, and it belongs against a
deployment rather than a laptop.

---

## Run 14 — the bake-off, run here

**Date:** 2026-08-04 · **Machine:** this laptop — 13th Gen Intel i7-1360P, 16 threads, 16 GB.
**Not the deployment host, on purpose** —
[ADR 0011](decisions/0011-no-experiments-on-production.md).

Two Phase 0 exit criteria had been waiting on somebody opening a terminal on a server. Both of
them turn out to be questions about weights and flags rather than about hardware, and both
were answered in an afternoon.

### The three-model choice — exit criterion 1

| Model | fields | perfect | **invented prices** | span median |
|---|---|---|---|---|
| **Qwen3-4B Q4_K_M** | **90%** | **8/10** | **0** | 16.4 s |
| Qwen3-1.7B Q8_0 | 87% | 7/10 | **1** | **5.8 s** |
| `unsloth` 1.7B Q4_K_M *(control)* | 10% | 0/10 | 3 | 5.5 s |

**Three points between them, and the three points are the whole decision.** The 1.7B is 2.8×
faster and it invented a price — on `contact-sales`, which the golden set describes as *"the
single most common real case on a competitor's pricing page, and the one where an invented
number does the most damage"*:

```text
expected  price None
returned  price Some(100.0), period Some(Yearly)
```

There is no page on the open web where that number appears. **The 4B has never invented one**,
across Runs 4, 12 and this one.

So the extraction tier stays the 4B, and the 1.7B is not promoted to it. Its speed is real and
it can have work where a wrong number is cheap — routing, classification — but not the tier
that fills a table a reader will check.

### The control did its job

`unsloth/Qwen3-1.7B-Q4_K_M` scored **10%, 0/10, three invented prices** — the same numbers as
Run 3, on a harness that has changed a great deal since. An instrument that still separates a
defective quantisation from a good one is an instrument the other two rows can be believed
from.

### `q8_0` KV cache — exit criterion 5

Same weights, same prompts, `-ngl 0` on both sides so the GPU is out of the experiment.

| | fields | perfect | invented | private memory |
|---|---|---|---|---|
| KV `f16` | 90% | 8/10 | 0 | 2.52 GB |
| **KV `q8_0`** | **90%** | **8/10** | **0** | **1.73 GB** |

**Identical scorecards, subject by subject — and 0.79 GB back.** The rule this was measured
against was written before the numbers existed: *take it if the score is unchanged and memory
drops; do not if the score moves at all.* The score did not move. **Adopt `q8_0` KV.**

Three resident models make that ~2.4 GB across the fleet, which is the difference between the
three-model design fitting a 24 GB box and being an argument.

### What this run cannot tell anyone

**How long a report takes in production.** Not on 16 x86 threads, and not from inside a
server either. The ratios transfer — the 4B is 2.8× the 1.7B on the span shape and will be on
any CPU — and the seconds do not.

That number is a **client-side measurement**: use the deployed product, time the wait, and
include the network and the browser, because the user does. Phase 0's latency criterion has
been restated to say so.

### Reproducing it

[MODEL_BAKEOFF.md](MODEL_BAKEOFF.md), which unlike the document it replaced has been run end
to end on the machine it was written on.

---

## Run 13 — a report, and what assembling one revealed

**Date:** 2026-08-04 · **Subject:** plausible · **Model:** Qwen3-4B Q4_K_M, unchanged since
Run 5.

Twelve runs of a pipeline printing what each stage decided. This is the first one that
produces **a report**:

```bash
cargo run -p landscape -- read https://plausible.io
```

```text
# https://plausible.io
Read 2026-08-04 19:58 UTC · 4 source(s) cited · prompts v1

## Pricing & packaging
no page found. Checked: /pricing (404), /plans (404), /pricing/ (404)

## Recent public changes
- Add notes to your traffic chart with annotations [S2·H]
  > - Jul 14, 2026

## Company facts
- says it was founded in 2018 [S3·H]
  > Uku Taht started Plausible in December 2018, building it alone…

## Trust & security posture
2 page(s) found and not read - our gap, not theirs
```

| | a run log | a report |
|---|---|---|
| Ordered by | page | **question** — which is what a reader arrived with |
| Its silence | invisible | a section carrying its coverage note |
| Its facts | uncited | **a source label and a verbatim quote, each** |

### The report found two things the run log had hidden

**A quote under the wrong fact.** `PageIdentity` held one list of quotes and paired them to
facts by position, and the assembled report rendered *"says it is based in the EU"* above a
sentence about web analytics drifting from its purpose. The run log had printed the facts and
the quote count separately and looked perfectly healthy.

**Evidence for a claim it does not support is the one thing this codebase must never render.**
Each fact now carries the quote it arrived with — `core::Stated<T>` — and the type makes the
old arrangement unrepresentable.

**And then: a quote that does not contain its fact.** With the pairing fixed, the headquarters
claim still cited a sentence that does not mention the EU — the model had picked a
neighbouring line out of the window it was given. The quote is now kept only if it contains
the value. The fact survives, because it was checked against the whole window; **the quote is
the smaller loss**.

That is the pattern this run is about. Assembling facts into claims is itself a check, and it
caught two things twelve runs of printing had not.

### What the sections are

Six, one per question discovery asks. `PRODUCT_SPEC.md` §4 fixes nine, and the other three —
positioning, sentiment themes, the SWOT — are **interpretation over sources this pipeline does
not gather**. A section that exists and cannot be filled teaches a reader to skim past empty
sections, which is the one habit this product cannot afford.

### Confidence is the fact's, not the model's

| | |
|---|---|
| a price, quoted verbatim | **High** |
| a dated change, parsed | **High**, and `as_of` the date the page states |
| a capability name | **Medium**, and it can never be more |

A capability name is a paraphrase by design — shortening a heading is what the model is *for*
here — so it cannot carry the confidence a quoted number does.

### The report checks itself

`Report::every_claim_is_traceable` refuses a citation that does not resolve, and the renderer
prints **"this report is not publishable"** rather than showing one. A citation that looks
checkable and is not is worse than no citation at all.

### What is still not right

**Two questions have no extractor**, and the report says so six times: *"2 page(s) found and
not read — our gap, not theirs."*

**`searched_as` is the origin the user typed.** The entity gate exists and this does not call
it, so the report's own account of what it looked for is a copy of its input.

**Nothing is stored.** The report is assembled, rendered and dropped. The queue, the worker and
the API have been waiting for something to carry since Phase 1 began.

---

## Run 12 — facts a page states by accident, and three checks that were not checks

**Date:** 2026-08-04 · **Subjects:** plausible, linear, basecamp, notion · **Model:** Qwen3-4B
Q4_K_M, unchanged since Run 5.

The fourth question kind: *who they are, where, how big*. It is the first one whose answers a
page is not built to publish — a pricing page exists to state a price, and **an about page
exists to tell a story**.

```bash
cargo run -p landscape -- read https://plausible.io
```

| Subject | Facts stated | What came out |
|---|---|---|
| `plausible.io/about` | **3 of 3** | founded 2018 · based in the EU · 10 people |
| `linear.app/about` | **1 of 3** | founded 2019 |
| `basecamp.com/about` | **0 of 3** | *"no stated facts about the company"* |
| `notion.com/about` | **0 of 3** | *"read 1 page, none stated anything"* |

All four are correct. Basecamp's about page says *"We're here for them, **23 years and
running**"* and never names a year — and 2003 is **arithmetic, not reading**. Nothing here
computes it.

### Three checks that were not checks

**A substring test on a number.** The model answered **"0 people"** for a page reading *"Today
Plausible is a team of 10"*, and the grounding check passed it: `"10".contains("0")`. Numbers
are matched as whole tokens now, and the test that would have caught it is in
`landscape-core`.

**All-or-nothing grounding.** The check cleared an *extraction* rather than a *field*, so one
carrying a correct founding year and an invented headquarters was discarded whole — throwing
away the answer that window had been asked for. `plausible.io/about` reported two facts of the
three it states because of it. Fields are cleared individually now, and the count is printed.

**A phrase that assumed word order.** `started in` does not match *"Uku Taht **started**
Plausible **in** December 2018"* — a name sits between the verb and the preposition. The
window went elsewhere, the model answered 2018 from somewhere it had not been shown, and the
grounding check dropped a year the page states plainly. Both halves were wrong and they
cancelled out to look like one right answer.

### And a source that was not ours

`notion.com`'s `llms.txt` lists **`linkedin.com/company/notionhq`**, and it took the identity
slot for that company. It could not be fetched, so the run printed *"could not fetch"* and the
question went unanswered.

Had it been fetched, it would have been worse. `FACT_CHECKING.md` §3.3 defines a primary source
as a page on the subject's own domain, and **only a primary source may set a value in a
comparison table**. Admitting somebody else's page here would launder it into the class that
outranks everything. Everything discovery admits is now on the subject's own site — subdomains
included, lookalike domains not.

With that fixed, notion reads its own `/about`, which states none of the three.

### What the fourth question is really like

| | pricing | identity |
|---|---|---|
| The page's purpose | state the price | tell a story |
| Where the fact is | a plan card | a subordinate clause |
| Checkable | exactly | **only that the words are on the page** |

The grounding check does more work here than anywhere else, and for a specific reason: **a
model has read about these companies**. Ask it where Notion is based over a page of prose and
the answer is available to it whether or not the page says so. Every value reported now has to
be written in the window it came from.

### What is still not right

**One window per fact means one chance per fact.** `linear.app/about` states more than a
founding year; the headquarters and headcount lines either did not score or did not survive.

**`headquarters` is a free-text field.** *"the EU"*, *"Chicago, Illinois"* and *"remote"* are
all valid answers and none of them compares to another company's.

**Nothing measures any of this.** Four about pages read by hand, no golden-set subject, and
the facts are the hardest of the four kinds to state a right answer for.

---

## Run 11 — the note that reads back what was checked, and what it found first

**Date:** 2026-08-04 · **Subjects:** basecamp, linear, notion · **Model:** Qwen3-4B Q4_K_M for
pricing and features; none for changes.

Ten runs have ended with a version of the same gap: `read` printed what it found and was silent
about what it looked for. [FACT_CHECKING.md](FACT_CHECKING.md) §5.4 has the rule — *a negative
nobody can check is not a finding* — and [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 the output:

> Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
> /releases (404), blog (90d). **Not "no changes."**

```bash
cargo run -p landscape -- read https://basecamp.com
```

```text
question     coverage
----------------------------------------------------------------------------------------------
pricing      2 fact(s) from 1 source(s)
features     10 fact(s) from 1 source(s)
changes      no page found. Checked: /changelog (404), /releases (404), /blog (404)
identity     1 page(s) found and not read - our gap, not theirs
trust        2 page(s) found and not read - our gap, not theirs
direction    1 page(s) found and not read - our gap, not theirs
```

**Basecamp publishes no changelog.** Run 10 knew that and said nothing; this states it with the
three paths that were tried and what each returned.

### Four silences, and no two of them mean the same thing

| The note | What it means |
|---|---|
| `nothing was checked` | our gap, and the only honest thing is to say so |
| `no page found. Checked: …` | **the company publishes none of this** |
| `found and not read` | our gap again, and a different one |
| `read N page(s), none stated anything` | the page exists and does not say |

The third one exists because the fourth was lying. Four of the six questions have **no
extractor yet**, so their pages are admitted and never opened — and the first version of this
note reported *"read 1 page, it stated nothing"* about `basecamp.com/about`, a page nothing had
opened. **That is this feature committing the exact error it was built to prevent**, and it
survived one run before the numbers were separated.

### What the note found on its first honest run

```text
linear   changes   read 1 page(s), none stated anything.
                   Checked: /changelog (200), /releases (404), /blog (200)
```

`linear.app/changelog` **answers 200 and had never been read.** The page that held the changes
slot was `/docs/releases.md` — documentation about a feature called Releases, the same page Run
10 recorded as correctly reporting no dated entries. It won the slot because it was named in
`llms.txt`, and `llms.txt` outranked a probe.

So the ordering changed to match the rule locale preference already follows in Run 9: **which
page it is comes before how we found it.** Provenance is evidence *about* a page; it is not
evidence that the page is the right one.

| `linear.app` | Before | After |
|---|---|---|
| changes | `/docs/releases.md` → no dated entries | **`/changelog` → 7 changes in the window** |
| trust | `/docs/security.md` | `/security` |

Seven dated changes — *Coding sessions on mobile*, *Introducing Loops*, *Initiative
properties* — from a page that had been reachable and unread since Run 5.

### The counts, after

| | pricing | features | changes |
|---|---|---|---|
| basecamp | 2 | 10 | **none published** |
| linear | 3 | 20 | **7** |
| notion | 4 | 19 | 8 |

### What is still not right

**Three questions have no extractor, and the note now says so six times a run.** That is the
correct output and an uncomfortable one: identity, trust and direction are found on every
subject and opened on none.

**`/blog (200)` under *changes* does not say the blog was read and rejected**, only that the
path answered. The attempts list carries what discovery saw, not what extraction did with it.

**The note is per question, not per section.** `PRODUCT_SPEC.md` §4 wants it under the section
in the report; this is the CLI, and `Coverage::to_section` is the piece that will carry it
there — written and tested, not yet used by anything that renders a report.

---

## Run 10 — the question a model never sees

**Date:** 2026-08-04 · **Subjects:** notion, plausible, linear, basecamp · **Model:** none, and
that is the point.

The third question kind is *what shipped recently*, and
[ARCHITECTURE.md](ARCHITECTURE.md) §5.4 answers it without asking anything:

> | Changelog entries, release dates, version numbers | **Code** — heading + `<time>` + date
> regex | **Dates are the most common LLM fabrication in "recent changes"** and are trivially
> verifiable. |

```bash
cargo run -p landscape -- read https://www.notion.com
```

```text
www.notion.com/releases   3136 words   8 change(s) in 90 days, 0 older
  2026-07-31  AI Meeting Notes can now trigger Custom Agents
  2026-07-30  High contrast mode
  2026-07-24  Workers, now in your Notion credits dashboard
  2026-07-16  New calendar tools for your agent
  ... and 4 more inside the window
```

| Page | In the 90-day window | Older | On the page |
|---|---|---|---|
| `notion.com/releases` | **8** | 0 | 8 |
| `plausible.io/changelog` | **4** | 36 | 70 |
| `plausible.io/blog` | **7** | 33 | 104 |
| `linear.app/docs/releases.md` | — | — | **"no dated entries on the page"** |
| `basecamp.com` | — | — | **no changelog discovered at all** |

Every date above was checked against the page. The last two rows are the interesting ones.

### "No dated entries" is not "no changes"

`linear.app/docs/releases.md` is **documentation about a feature called Releases**. It has no
dates on it, and the honest output is to say so — which is what
[PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 insists on for exactly this case:

> Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
> /releases (404), blog (90d). **Not "no changes."**

The same discipline shows up in the numbers beside each page. Plausible's changelog has 70
dated entries and four of them are inside the window; a report that printed four and stopped
would read as a quiet quarter for a company that ships weekly.

### The line has to *be* a date

Notion's changelog contains this sentence:

> *"Starting August 11 2026, Workers will run on Notion credits."*

A parser that takes any date it finds files a **future price change as a shipped feature**.
That is the single worst thing this could do, and the rule that prevents it is small: a date is
an entry only when it starts the line — alone, or followed by a separator and a title. The
six formats real changelogs use are all read; a date inside a sentence is not one of them.

### Two more discovery findings, both from running this

**A blog post is not a section answer.** `plausible.io/blog/product-hunt-launch` classified as
*features* on the word `product`, and `/blog/changelog-podcast` as *changes* on the word
`changelog` — a podcast episode. Both were admitted, both were read, and both cost a slot. The
index of a publication still counts; a post inside it does not.

**The page that names the question outranks the broad one.** `notion.com` publishes `/releases`
and `/blog`. Both answer *what changed*, both came from `llms.txt`, and the tiebreak was URL
length — so `/blog` went into the report with **no dated entry on it** and the changelog went
unread. Length is now the last word rather than the second.

### What is still not right

**The window is counted, not the coverage.** *"4 in 90 days, 36 older"* says what the page
holds. It does not say whether the page is the company's only changelog, which is the other
half of §4's coverage note.

**Basecamp has no discovered changelog**, and the run says nothing about that at all — the
absence of a source is currently invisible, where §4 wants it stated with the paths that were
checked.

**A job posting is still an article.** `plausible.io/jobs/customer-success` is admitted as
*direction*, which is the same shape as the blog-post problem in a place the rule does not
reach.

---

## Run 9 — the wrong pages, and what reading the right ones changed

**Date:** 2026-08-04 · **Subjects:** linear, todoist, notion, basecamp · **Model:** Qwen3-4B
Q4_K_M, unchanged since Run 5.

Runs 7 and 8 each ended with a finding nothing downstream could fix. An extractor pointed at
the wrong page has two options, a wrong answer and silence, and it had been producing both.

```bash
cargo run -p landscape -- discover https://linear.app
```

| Subject | Before | After |
|---|---|---|
| **linear** | two setup documents as the feature sources | **`/features` and `/docs`** |
| **todoist** | 8 sources: Czech, Danish, and two steps in buying | **3, all English, `/pricing` read at last** |
| **notion** | `/es-es/pricing` took a slot | `/plans` takes it |
| **basecamp** | 6 sources | **6 sources, unchanged** — the control |

### The features page Linear had all along

`linear.app/docs/mcp.md` is a setup guide. Read as a features page it reported `Setup`,
`Claude` and `Cursor` as capabilities of Linear (Run 8) — and worse, **it took the slot**.
Both of Linear's feature sources were documentation, because `llms.txt` outranks a probe and
`guess` classified anything under `/docs` as answering *what does this product do*.

Documentation answers *how do I use this*. Only its front page answers the other question.

With the depth rule, `linear.app/features` is read for the first time:

```text
linear.app/features   250 words   8 capabilities stated
  Planning · Building · AI-powered workflows and agents · Insights
  Mobile · Customer Requests · Linear Asks · Security
```

Eight capabilities, no dropped names, no unverifiable quotes. The page had been there for
every run since Run 5.

### A locale is a variant, not a page

Run 7 read `todoist.com/cs/pricing` and `/da/pricing` — Czech and Danish — and never read the
English page. Run 8 added `notion.com/es-es/pricing`.

**The filter that would have been wrong** is dropping localised URLs: some sites publish only
`/de/preise`, and that is then the pricing page. So variants collapse into one candidate the
way `/pricing` and `/pricing/` already did, and which one wins is a preference — no locale,
then English, then whatever the site listed.

That the Czech page was better attested than the English one is exactly why the order matters:
**which page it is comes first, how we found it comes second.**

### And a page you do something on states no facts

Todoist's sitemap lists `/cs/pricing/setup` and `/cs/pricing/upgrade`. Both classify as
pricing on the word `pricing`, both are steps in buying, and between them they took two more
of that run's five slots.

Todoist now admits three sources from eighteen paths checked, all English, and one of them is
the pricing page it publishes.

### Fewer sources is the result, not a regression

| | Sources | What they are |
|---|---|---|
| todoist, Run 7 | 8 | 4 duplicates or transaction steps |
| todoist, Run 9 | **3** | pricing, features, security |

A source that cannot answer the question it was admitted for costs a fetch, a model pass, and
a line in the report saying something we cannot stand behind. **Three we can read beats eight
we cannot.**

### What is still not right

**`linear.app/docs` gives twelve capabilities and one of them is `Popular`.** The front page of
a documentation site is partly a menu, and the menu headings survive the capability rules —
though the other eleven (`Import Issues`, `Triage`, `Parent and Sub-Issues`, `Notifications`)
are real.

**The language list is a list.** Forty codes, matched only as a leading segment with a page
under it, because two-letter segments are not all languages — `/hr` is Croatian on one site
and human resources on the next. A site using a code outside the list loses the deduplication
and nothing else.

**None of this is measured by the golden set**, which still has no discovery subjects at all.

---

## Run 8 — the second question kind, and a prompt that taught the model a fact

**Date:** 2026-08-03 · **Subjects:** basecamp, linear, notion · **Model:** Qwen3-4B Q4_K_M,
unchanged since Run 5.

Pricing was the first question. This is *what does the product do*, and
[ARCHITECTURE.md](ARCHITECTURE.md) §5.4 says how to answer it:

> | Feature lists on structured pages | **Code first**, model for normalization only |

```bash
cargo run -p landscape -- read https://basecamp.com
```

| Page | Sections named | Windows read | Capabilities reported |
|---|---|---|---|
| `basecamp.com/features` | 18 | 12 | **10** |
| `notion.com/product/ai` | 20 | 12 | **8** |
| `notion.com/product/docs` | 21 | 12 | **8** |
| `linear.app/docs/sla.md` | 12 | 12 | **8** |
| `linear.app/docs/mcp.md` | 6 | 6 | **5** |

Basecamp's ten are Message Boards, Hill Charts, Card Tables, Campfire chats, Automatic
Check-ins, Docs & Files, Reports, and three more — every one of them a real Basecamp feature.

### The guess about page shape was wrong

The plan was to read bullet lists. Four real features pages disagree:

```text
## Message Boards for announcements and discussions      basecamp.com/features
### Customer Requests                                    linear.app/features
### Capture                                              notion.com/product/docs
```

**A capability is a heading with a description under it**, and the bullet lists on those pages
are navigation — `- Pricing`, `- Log in`, `- Books we've written`. Reading bullets would have
reported Basecamp's footer as fourteen features.

That leaves the model exactly §5.4's job: `Message Boards for announcements and discussions` is
a heading a parser can find, and `Message Boards` is a name only a reader can cut out of it.

### The prompt taught the model a fact, and the model used it

The first prompt carried a worked example — *"a heading reading **Message Boards** for
announcements and discussions names a capability called Message Boards"*. On the next run,
`linear.app/docs/mcp.md` reported:

```text
MCP Server · Claude · Cursor · MCP · Message Boards
```

**Message Boards is Basecamp's feature.** It reached a Linear report through our own prompt.

`FACT_CHECKING.md` §P15 is about laundering someone else's hallucination into our citation;
this is the same failure with a shorter supply chain. **A worked example in a prompt is a
source of facts**, and it is one with no URL, no fetch date, and nothing to cite.

### Which is why a name is checked, not trusted

A price can be checked verbatim. A capability name **cannot** — naming is the normalisation the
model is there to do, so the answer is a paraphrase by design. The check that still holds is
weaker and cheap: `FeatureExtraction::name_is_from` requires every word of the name to appear
in the section.

| It caught | On |
|---|---|
| `Message Boards` | `linear.app/docs/mcp.md` — the prompt leak, before the example was removed |
| `string` — the field's own type | `linear.app/docs/sla.md`, twice |
| 10 more names across five pages | all subjects |

Removing the example fixed the first. The rest are dropped and counted, and `read` prints the
count: *"4 name(s) dropped — not words from the section"*. **A page whose windows mostly fail
this check is a page we should not be reading**, which is a signal worth having.

### A page that was already Markdown

`linear.app/docs/mcp.md` reported **no capabilities at all** before any of this. The cause was
upstream of the extractor: `llms.txt` publishes a site as Markdown, discovery follows it, and
the HTML converter turned that page into **one 2,167-word line**. A `#` is text to an HTML
parser, and there were no block tags to break lines on.

Nothing downstream could find a section in it, so the page reported nothing and looked exactly
like a page that had nothing on it. `markdown::from_body` now recognises a Markdown body.

### What is still wrong

**Two of Linear's three "features" pages are documentation.** `/docs/mcp.md` is a setup guide,
and its sections are named for other people's editors — `### Zed`, `### Windsurf`. Read as
capabilities they say Linear's product includes Zed. Sections containing a code block are now
skipped, which removes the most confident version of that answer, and `Setup` and `Claude`
still come through. **The label came from discovery, and only discovery can fix it.**

**The model shortens most names and not all.** `The Project page keeps it all together` came
back whole. Nine of twelve on Basecamp were cut correctly; the rest are the heading.

**A capability with no qualifier may still be conditional.** Nothing on these pages said
*beta* or *Business and above*, so the `qualifier` field is measured only by unit tests.

---

## Run 7 — six real pricing pages, and the one that had no prices on it

**Date:** 2026-08-03 · **Subjects:** basecamp, linear, plausible, notion, sentry, todoist ·
**Model:** Qwen3-4B Q4_K_M, unchanged since Run 5.

Run 6 got `basecamp.com/pricing` right and reported one of its two plans. This is one window
per plan — `span::every_plan` — measured against six pages, of which four had never been seen
by the code before.

```bash
cargo run -p landscape -- read https://basecamp.com
```

| Page | Plans published | Windows | Reported |
|---|---|---|---|
| `basecamp.com/pricing` | 2 | 2 | ✅ Pro Unlimited $299, Pro $15 |
| `linear.app/pricing` | 3 | 3 | ✅ Free $0, Basic $10, Business $16 |
| `plausible.io` | 3 | 3 | ✅ Starter $9, Growth $14, Business $19 |
| `notion.com/pricing` | 3 | 5 | ⚠️ Free $0, Plus $10, Business $20 — **and an add-on** |
| `sentry.io/pricing` | 4 | 6 | ✅ each plan appears twice; the duplicates are dropped |
| `todoist.com/pricing` | 3 | **0** | ✅ "no pricing content on the page" |

The plausible, notion, sentry and todoist pages were fetched *after* the heuristic was
written. Two of them changed it.

### todoist is the finding

Its prices are rendered in JavaScript, so what reaches the Markdown is the feature-comparison
table and nothing else. Forty table rows outscore any real pricing block on structure alone,
and not one of them contains a currency symbol. The window chosen from it was:

```text
|  | Beginner | Pro | Business |
|---|---|---|---|
| Personal projects | 5 | 300 | 300 for each member |
```

and the model returned **"Beginner at $5"**. The 5 is how many personal projects the plan
allows. **The HTML contains no dollar amount anywhere.**

A window must now contain a price or a contact-sales line to win at all. The page then reports
*"no pricing content on the page"*, which is both true and the number
[ARCHITECTURE.md](ARCHITECTURE.md) §5.5's JavaScript-gap counter exists to collect — the
counter cannot be honest if the pipeline guesses instead of abstaining.

### notion changed the prompt twice

**`$10 per 1,000 monthly Notion credits` came back as `$0.01`.** The model divided. It is
arithmetic rather than extraction, and it is not checkable against the page, so the prompt now
says price_usd must be a number written there.

**And the add-on stayed.** `### Custom Agents` is a heading with a price under it, which is
the definition of a plan block this code uses, so notion reports four plans where three are
real. There is no shape that separates an add-on from a plan — only meaning — and that is
worth stating rather than tuning away.

### What the page shapes turned out to be

| | |
|---|---|
| basecamp | `## Pro Unlimited` then a subtitle |
| notion | a marketing line then `### Free` |

**The same shape, upside down**, and heading levels cannot tell them apart. The shorter
heading wins: a plan name is a noun and a subtitle is a sentence. On all six pages that picks
the plan name.

### The weakest field is the billing period

`linear.app` writes `$16 per user/month` and `Billed yearly` in the same block. The price is
right on all nine plans across the three English pages; the period was wrong on one of them,
and the prompt rule that fixed the other two — *how often the price recurs, not how often the
invoice arrives* — did not fix that one.

**`BillingPeriod` conflates two facts the pages state separately.** Recorded, not fixed: it
needs a type change and golden-set coverage rather than another sentence in the prompt.

### Still not measured

**The golden set does not cover any of this.** Its subjects are single-plan fixtures, so
`every_plan` returns one window for each and the whole multi-plan path is exercised only by
unit tests and by this run. §5.4's warning stands: the six pages above are six data points and
four fixed bugs, not a validation.

---

## Run 6 — the window fixes it, and the window was wrong twice first

**Date:** 2026-08-03 · **Subjects:** `basecamp.com`, `linear.app` · **Model:** Qwen3-4B
Q4_K_M, unchanged from Run 5.

```bash
cargo run -p landscape -- read https://basecamp.com
```

Run 5 established that the model works on a span and fails on a page. This is span
pre-selection ([ARCHITECTURE.md](ARCHITECTURE.md) §5.4) built and pointed at the same page.

| Attempt | `basecamp.com/pricing` returned |
|---|---|
| Run 5 — whole page, 1729 words | ❌ "no price published" |
| First scorer — 283-word span | ❌ **"Timesheet at $50"** |
| Final scorer — 264-word span | ✅ **"Pro at $15"** |

`$15/user` is Basecamp's Pro plan. `$50/month` is the Admin Pro Pack add-on, mentioned in
the FAQ.

### The middle row is the finding

The first scorer chose the FAQ, and the reason is instructive: **the FAQ mentions prices
several times in close succession, while the actual plan states its price once.** Any scorer
summing price-shaped signals over a window prefers the denser region, and the denser region
is the one full of exceptions and hypotheticals.

What separates them is not vocabulary. It is **shape**:

| | A plan block | An FAQ |
|---|---|---|
| Distance to its heading | 1–2 lines | dozens |
| Question marks | none | one per entry |

Both are now signals, and neither is specific to Basecamp.

### Two more, found the same way

**The heading was anchored to the wrong line.** A window wide enough to hold a short page
starts at line 0, where the nearest heading above is the page title — `# Pick a package` —
rather than the one governing the price. It is now anchored to the best-scoring line.

**And it took the nearest heading rather than the most significant one.** A plan is written
as a name and then a subtitle, so the nearest heading hands the model *"All-inclusive
pricing"* as the thing `$299` belongs to. That is not a plan name.

### Running the right extractor

`linear.app/docs/mcp.md` returned **"MCP server at $0"** — a plan that does not exist, from a
documentation page. The pricing extractor was being run over every discovered source.

Discovery already labels what each page answers, so using that label costs nothing and fixes
it. It also cut the model calls per company from 8 to 2.

| | `linear.app` |
|---|---|
| `/pricing` | ✅ "Free at $0" — correct, Linear's first plan |
| `/docs/*`, `/about`, `/careers` | "not a pricing page — no extractor yet" |
| `/plans` (2 words) | "skipped — nothing to read" |

### What is still not right

**Basecamp has two plans and we report one.** `$15/user` Pro and `$299/month` Pro Unlimited
are both on that page; `PricingExtraction` models a single plan, so the window picks one and
the other is invisible. Extracting *all* plans needs a different type, and it is the next
piece of work rather than a tuning problem.

**The heuristic is `SPAN_VERSION = 1` and has been measured against two companies.** §5.4
says it *"is itself part of the golden-set evaluation, because a bad window is
indistinguishable from a bad model at the output"*. Until the golden set covers spans, this
run is two data points and a fixed bug, not a validation.

---

## Run 5 — the golden set's own warning, come true

**Date:** 2026-08-03 · **Subject:** `basecamp.com`, whole pipeline · **Model:** Qwen3-4B
Q4_K_M, the one that scores 90% on the golden set.

```bash
cargo run -p landscape -- read https://basecamp.com
```

The first run of the joined pipeline. Discovery found 6 sources, all fetched, all converted
to Markdown, all scored `good`. Then:

| Page | Words | Quality | Extracted |
|---|---|---|---|
| `basecamp.com/pricing` | 1729 | good | **"Pro, no price published"** |

**Basecamp publishes `$299/month` and `$15/user` on that page.** The model that scores 90%
on the golden set and never invents a price had just failed to find one that was plainly
there.

### Where it was lost, and where it was not

Traced stage by stage, because the point of a joined pipeline is that a wrong answer has six
possible causes:

| Stage | Verdict |
|---|---|
| Fetch | ✅ 200, full page |
| Markdown | ✅ `## Pro Unlimited` at line 11, `$299/month` at line 13 |
| Truncation | ✅ Both inside the 6000 characters sent |
| Prompt | ⚠️ Did not name a plan. Fixed — **did not help** |
| **Model, given 1729 words** | ❌ **"no price published"** |
| **Model, given the 39-word span** | ✅ **`Pro Unlimited`, `$299`, verbatim quote** |

Same model. Same prompt. Same words. **The only difference is how many of them.**

### This is the thing Run 3 said would happen

> *"It has ten subjects, all pricing, all in English, all written by the same person on the
> same afternoon — so it shares that person's blind spots, and **a model could score 100% on
> it while failing on the first real page it meets**."*

It scored 90% and failed on the first real page it met. The golden set's pages are ~100 words
of clean prose about one plan. `basecamp.com/pricing` is 1729 words, three plans, a cookie
banner and an FAQ.

**The golden set is not wrong — it is measuring a different thing than the pipeline does**,
and until this run there was nothing to reveal the difference.

### What it justifies

**Span pre-selection**, [ARCHITECTURE.md](ARCHITECTURE.md) §5.4 — feeding the model the
relevant ~400-token window rather than the page. It was already in the plan on the argument
that prefill dominates on 4 ARM cores. It now has a second and stronger argument: **without
it, extraction on real pages does not work at all.**

That reframes it from an optimisation to a correctness requirement, which changes where it
belongs in the order of work.

**And the golden set needs real pages.** `ROADMAP.md` takes it to 25 subjects in Phase 1;
those should be fetched, not written, or the set will keep predicting a performance the
pipeline does not have.

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
exactly why the *ratio* is the transferable part and the seconds are not.**

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

## Still to measure

The list below has been split by **where the answer lives**, which is the change
[ADR 0011](decisions/0011-no-experiments-on-production.md) makes. Most of it was never a
question about the deployment host.

**Locally, and nothing stops it:**

- [x] ~~`q8_0` KV cache quantization validated against the golden set~~ — Run 14. Adopted.
- [x] ~~One server at a time, so the numbers are not contended~~ — Run 14 ran the KV
      comparison CPU-only with the GPU out of the experiment.
- [ ] Prefill and generation tok/s separately, rather than end-to-end latency
- [ ] `Q4_K_M` **and** `Q4_0` — the repacking question is a property of the format and the
      CPU's instructions, and an ARM laptop or a cheap ARM VM answers it as well as anything
- [ ] Qwen3 1.7B / 4B / 8B / 14B, Gemma 3 4B/12B, Llama 3.2 3B. **Licence review first**
- [ ] Aggregate throughput at `--parallel 1/2/4/8`
- [ ] Time-to-first-token
- [ ] Resident RAM for three models against the ~17 GB budget

**From a client, against something deployed — never from inside it:**

- [ ] How long a report takes, end to end, as a user experiences it. That includes the
      network and the browser, which a stopwatch on the server cannot see.
- [ ] Whether the 90–180 second promise holds for somebody sitting in front of it.

**Not a measurement at all:**

- [ ] ~~Provision the A1 and convert to Pay-As-You-Go~~ — a deployment step, and not one that
      belongs on a benchmarking list.

Run 3 adds two of its own:

- [ ] **Field order as a lever.** llama.cpp walks properties in the order the schema
      serialises them, and `serde_json`'s sorted maps currently pick that order for us.
      `preserve_order` would hand it back. Quote-first (read, then answer) against
      quote-last (answer, then justify) is a measurable question, and a hand probe showed
      quote-first spends the whole token budget quoting — so it needs a `maxLength` too
- [ ] **Whether the 1.7B's plan confusion survives better prompting**, or is a size limit.
      This decides whether the Extractor tier can ever be a 1.7B, which is the difference
      between ~7 s and ~12 s per span on the numbers above
