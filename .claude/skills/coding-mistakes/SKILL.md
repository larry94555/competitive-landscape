---
name: coding-mistakes
description: >
  CODING_MISTAKES — the register of defects that reached a pull request in this repository, what
  class each belongs to, and the check that would have caught it. Use this before writing or
  reviewing code that involves progress/partial state, streams and reconnection, change
  detection, cached snapshots beside a live source, model output that must be grounded in a
  source, or any "derive one fact in two places" situation. Also use it when a review finds a
  defect, to add the entry — this file is a record that grows, not a static style guide.
---

# CODING_MISTAKES

**Every entry here is a defect that passed our own tests, our own review, and reached a pull
request.** It was written down because the class it belongs to will occur again, and the second
occurrence is cheaper to prevent than to find.

This is not a style guide. `docs/CODING_QUALITY.md` is the standard; this is the list of places
the standard did not save us, with the specific question that would have.

## How to use it

**Before writing** code that touches any of the classes below, read that class's rule and its
"ask this" line.

**Before opening a PR**, run [the two commands and eight questions](#before-a-pr-two-commands-and-eight-questions).
The commands are the part that does not depend on remembering; the questions take two minutes.

**After a review finds something**, add an entry the same day. Format: what was written, what a
person would have seen, why the tests missed it, the rule. Keep the failure concrete — the
*symptom a reader would experience* is what makes an entry memorable a year later.

---

## The pattern behind most of them

**Nine of the eleven entries below are states that only exist when something goes wrong
mid-operation**, or after an operation has partly succeeded. A run that completes normally never
enters them, so a test suite built from complete runs cannot see them.

> **Ask this:** *what states exist between "started" and "finished", and which of them did I
> write a test for?*

The second pattern is narrower and sharper: **three defects were introduced by the fix for the
previous one** (entries 1 → 3 → 4). A fix that adds a recovery path adds states, and the new
states are of exactly the kind that the original bug was.

> **Ask this:** *my fix added a path. What is true on that path that was not true before?*

---

## 1. "Has data" is not "is finished"

**Written:** `const settled = analysis?.report != null || analysis?.failure != null;`

**What a person saw:** their report stopped updating half-written, for ever. The stream had
dropped, the single recovery fetch returned an analysis that was *still running but already
carried a partial report* — so the client called it settled, cleaned up the effect, and never
reconnected.

**Why tests missed it:** every test drove the happy path, where a report appears only at the end.
`save_progress` had made that assumption false one commit earlier, in a different crate.

**Rule:** **derive lifecycle state from the state machine, never from the presence of a payload.**
If a type has a status field, the status is the answer. A nullness check is a proxy, and a proxy
holds only until someone makes the payload arrive earlier — which is exactly what a progress
feature does.

> **Ask this:** *is this `x != null` really asking "is it finished"? What happens the day `x`
> exists sooner?*

## 2. Change detection on a summary rather than the value

**Written:** `if sent_sections.get(&key) != Some(&section.claims.len())`

**What a person saw:** a claim we had already retracted stayed on screen. `PagePricing::assembled`
*replaces* a plan when a later window supplies the price the first one lacked, so
*"Free is listed with no published price"* became *"Free costs $0"* — a different claim, at the
same length. The count was unchanged, so the correction was suppressed.

**Why tests missed it:** the tests covered "a section grows", which is what we had designed for.
Nothing covered "a section is corrected", because we had not thought of correction as a case.

**Rule:** **compare what you are about to send, not a cheap summary of it** — unless the summary
provably covers every field that can change. Serialising the payload and comparing strings costs
nothing at this scale and cannot drift from the thing it guards.

> **Ask this:** *name a change this comparison cannot see. If naming one takes under a minute, it
> is not a comparison.*

## 3. A snapshot preferred over the live source

**Written:** `const showing = report?.sections ?? sections;`

**What a person saw:** after a dropped stream reconnected, the page froze at the instant of the
recovery fetch. Every section the new connection delivered was ignored, because the fetched
partial report was now non-null and won the `??`.

**Why tests missed it:** the two sources are never both present on the happy path — a fetched
report means the run is over, so there is nothing live left to lose.

**Rule:** when the same data has a **live source and a snapshot**, say explicitly which wins *in
each phase*, and write the phase into the expression. `??` picks by nullness, which is not a
phase.

> **Ask this:** *both of these are populated at once — when, and which should the reader see
> then?*

## 4. One fact derived two ways

**Written:** the effect settled on `isTerminal(analysis.status)`; the view rendered
`status ?? analysis.status`.

**What a person saw:** a finished report sitting under *"Reading public web pages…"*, with a
line promising more sections. The stream's last word before dropping was `running`, and it
outranked a recovery fetch that came back `complete`.

**Why tests missed it:** each half was tested against its own source. Nothing asserted that the
two agreed.

**Rule:** **derive it once, read it everywhere.** Two expressions computing "is it finished" from
different inputs will disagree, and the disagreement will be invisible until the inputs diverge —
which is precisely what an error path does.

> **Ask this:** *is this fact computed anywhere else? Do the two agree on every path, including
> the ones that fail?*

## 5. A check that cannot fail

**Written:** `haystack.contains(&count.to_string())` — grounding a model's number against the
window it read.

**What a person saw:** *"0 people"* reported for a page that says *"a team of 10"*. `"10"`
contains `"0"`, so the check passed.

**Why tests missed it:** the check was tested with values that were present and values that were
absent — never with a value that is a *substring* of a present one.

**Rule:** a validation test needs a case that **should fail and does**. Substring matching on
numbers, prefixes or identifiers is almost always wrong: compare whole tokens.

> **Ask this:** *what is the nearest input that must be rejected? Is it in the tests?*

## 6. All-or-nothing validation throwing away the good part

**Written:** `if !got.is_from(&window.text) { dropped += 1; continue; }` — one bad field
discarded the whole extraction.

**What a person saw:** a founding year the page states plainly was reported as missing, because
the same model answer also carried a headquarters it had invented.

**Rule:** validate at the granularity of the thing you can act on. If three fields arrive
independently, they fail independently.

> **Ask this:** *if one field is wrong, what happens to the others — and is that what a reader
> would want?*

## 7. Evidence attached by position

**Written:** `PageIdentity { evidence: Vec<String> }`, with quotes paired to facts by index.

**What a person saw:** *"says it is based in the EU"* rendered above a sentence about web
analytics drifting from its purpose. **Evidence for a claim it does not support** — the single
worst output this product can produce.

**Why tests missed it:** the assembled type was tested for the facts it held, not for whether
each fact's quote was *its own*.

**Rule:** if two pieces of data belong together, **put them in the same value**
(`Stated<T> { value, quote }`). Parallel collections joined by index are a bug waiting for the
first case where one of them is shorter.

> **Ask this:** *can these two lists ever be different lengths, or ordered differently? Then they
> are one list of pairs.*

**It came back.** `Analysis::render` zips `sections` with `coverage`, and merging several
companies' reports produced six sections and *N×6* coverage records — so the renderer read the
first company's and silently dropped the rest, and a section that found nothing described only
the first site's attempts. Same shape, two years of intent apart: **two collections joined by
position, where one of them changed length.** Found by review, in the pull request that changed
the length.

## 8. A worked example in a prompt is a source of facts

**Written:** a capability prompt containing *"a heading reading **Message Boards** for
announcements and discussions names a capability called Message Boards"*.

**What a person saw:** **Message Boards**, a Basecamp feature, reported for a page of Linear's
documentation.

**Rule:** no real name from any real subject in a prompt — company, product, plan, person.
Illustrate with the shape, not with an instance. `FACT_CHECKING.md` §P15 is about laundering
somebody else's hallucination; this is the same failure with a shorter supply chain, and the
example has no URL and nothing to cite.

> **Ask this:** *if the model repeated this instruction back as an answer, would it be a
> fabrication?* A test in `stages.rs` now asserts no prompt names a real company.

## 9. A rule that assumed word order

**Written:** matching `"started in"` to find a founding year.

**What a person saw:** nothing — for a page that says *"Uku Taht **started** Plausible **in**
December 2018"*. A name sits between the verb and the preposition, so the window went elsewhere,
the model answered from somewhere it had not been shown, and the grounding check dropped a year
the page states plainly. **Two failures cancelling into one plausible silence.**

**Rule:** phrase matching over prose is a heuristic; test it against the sentence a real page
writes, not the sentence you would write.

> **Ask this:** *is this rule tested against text I copied from a real page, or text I invented?*

## 10. A cap applied silently

**Written:** `windows.truncate(MAX_CAPABILITIES)` with no record of what was dropped.

**Rule:** a limit a reader is not told about reads as completeness. Carry the number considered
alongside the number kept, and print both.

> **Ask this:** *does the output distinguish "this is all there was" from "this is all we
> looked at"?*

## 11. A window that could win with nothing in it

**Written:** a span scorer where structural signals (table rows) could outscore every window
containing an actual price.

**What a person saw:** *"Beginner at $5"* for `todoist.com`, whose HTML contains no dollar amount
at all. The 5 was how many personal projects the plan allows.

**Rule:** when a scorer selects a candidate to answer a question, require the candidate to
contain **something that can answer it**. A score is a ranking, not a qualification.

> **Ask this:** *can the top-scoring candidate be one that contains no answer? What does the
> caller do then?*

## 12. Concurrent writes of a whole value, believed to be order-free

**Written:** a `tokio::spawn` per progress snapshot, with the reasoning beside it:

```rust
// Ordering is not a concern because each write is the whole report so far, not a delta.
```

**What a person saw:** nothing, on a fast store. Given a store whose first write is slow, the
older report lands last and is what is kept — so a section loses a claim, and a correction the
reader has already been shown is undone in front of them. It repairs itself at the end, which
puts the visible window exactly on the ninety seconds somebody is watching.

**Why the tests missed it:** every test wrote to an in-memory store that completed instantly, so
there was never a second write in flight to overtake the first.

**Rule:** "the payload is idempotent" is not "the writes are ordered". Whole-value writes are
order-free only if something *makes* them ordered. One writer, fed by a channel, is the cheap
version — and it can coalesce, which a queue of stale snapshots cannot.

> **Ask this:** *if two of these are in flight at once and the slow one finishes last, what is
> left behind?*

## 13. Clearing the source without retracting what was already sent

**Written:** `reclaim_stale` clearing a dead worker's partial report from the row — and stopping
there.

**What a person saw:** the dead worker's claims still on screen. The store was right, a fresh GET
was right, a reconnection was right; the connection **already open** had sent those sections and
nothing took them back. If the replacement run never reached that question — a page that 404s
this time — the retracted claim stayed until the run ended.

**Why the tests missed it:** the test asserted the *eventual* value was the second run's. The
defect is entirely in the interval, and an assertion about the end cannot see it.

**Rule:** state that has been pushed to a reader lives in two places, and clearing the source
only fixes the next reader. A withdrawal needs its own message — and any "already sent this"
memory used to suppress duplicates has to be cleared with it, or the replacement's identical
answer is suppressed too and the reader is left looking at nothing.

> **Ask this:** *who else is holding a copy of what I just deleted, and what tells them?*

## 14. A guard written about the connection, not about the state

**Written:** the fix for 13, sending the withdrawal only when *this connection* had already sent
something:

```rust
if analysis.report.is_none() && !sent_sections.is_empty() { … }
```

**What a person saw:** the same stale claim as 13, one reconnect later. The reader's sections
survive a reconnect **on purpose** — that is what stops a dropped stream wiping the page — while
the server's record of what it has sent starts empty on every new connection. Drop, reclaim,
reconnect, and the guard suppressed the withdrawal on the one connection that needed it.

**Why the tests missed it:** every test drove a single connection. The defect only exists at the
seam between two.

**Rule:** when a guard's subject is *the connection* and the thing it protects is *the reader*,
it is wrong at every reconnect. State the condition in terms of the durable thing — here, the
row: **no report on the row means nothing backs what the reader holds**, whoever is connected.
The price is that an ordinary new run also opens by withdrawing nothing, which is correct and
free.

**And once it is per-episode rather than per-poll, something has to rearm it.** A flag that is
set and never cleared works for the first occurrence and is silent for the second.

> **Ask this:** *does this condition mean the same thing on a fresh connection as on the one
> that has been open for a minute? If it fires once per episode, what starts the next episode?*

**Resolved.** 13 and 14 were two attempts to describe *"the run started over"* with something
other than a name for the run. The third attempt gave it one — a `generation` on the row, raised
by both the claim and the sweep, sent to the client so the client can compare
([ADR 0012](../../../docs/decisions/0012-a-claim-is-a-number.md)). Keep the entries: the two
wrong answers are the useful part, and the shape they share — *a condition about the connection,
protecting something that outlives connections* — is the thing to recognise next time.

## 15. A state that two things can be in at once

**Written:** `WHERE status = 'running'` as the guard on who may write to an analysis.

**What a person saw:** nothing yet, and that is why it is here. The staleness sweep cannot tell a
**dead** worker from a **slow** one — a row that has been `running` for twenty minutes looks the
same either way — so when the slow case happens, two live workers are running the same analysis
and `status` says `running` for both. Whichever finished last won. A reader would have got a
report, correctly assembled and correctly cited, from a run that had been abandoned, with
nothing in any log recording that two had been produced.

**Why the tests missed it:** every test had one worker. The state is only reachable when a
timeout fires against work that is still going, which no test had a reason to arrange.

**Rule:** a guard has to name **the actor**, not the situation. "Is it running" is a property of
the row and true for both claimants; "is this still *my* claim" needs something identifying the
claim, which means a value the claimant carries and the store can check. The cheapest such value
is a counter that goes up whenever the work is handed on.

**And the same value is what a reader needs.** A status transition is an *edge* — miss the poll
and it never happened. A generation is a *value*, so a client that reconnected, or blinked, still
finds it different from the one it holds. That is what finally settled 13 and 14.

> **Ask this:** *can two different actors be in this state at the same time? If so, what does
> this condition actually authorise?*

## 16. A check that read the working tree instead of the commit

**Written:** `python scripts/check_links.py`, run locally, green.

**What a person saw:** CI red on the link checker for a file that exists on my disk. An
untracked `PROJECT_STATUS.md` was in the working tree along with an uncommitted README row
pointing at it; `git add -A` took the row and not the file, and the local checker resolved the
link against a file that was never going to be checked out.

**Why the tests missed it:** they did not miss it — they cannot see it. Every local check reads
the working tree, and the working tree is not what is pushed. The gap only exists when the two
differ, which is exactly when `git add -A` is used on a tree somebody else has been editing.

**Rule:** for any check that reads *files* rather than code — links, generated assets,
documentation that references paths — verify against a clean checkout of the commit, not the
directory you are sitting in:

```bash
git worktree add --detach /tmp/check HEAD && cd /tmp/check && python3 scripts/check_links.py
```

And do not `git add -A` a tree you did not start clean. Read `git status` first, and commit
someone else's work with them rather than around them.

> **Ask this:** *would this check still pass on a fresh clone of what I am about to push?*

## 17. A mutation that silently hit the wrong copy

**Written:** a harness that reintroduces a defect by replacing the first occurrence of a snippet,
pointed at a `break` that three stages had just been given identical copies of.

**What a person saw:** `MISSED` — reported against the loop I meant to break, when what had
actually been broken was a different loop in the same file. I was one sentence away from writing
"this is a real gap" into `BENCHMARKS.md` about a case that was covered.

**Why the tests missed it:** the harness *is* the test of the tests. Nothing checks it, and its
failure mode is a green-looking word rather than an error.

**Rule:** a mutation has to be **verified to have landed where it was aimed** — anchor it on
something unique to the site, and treat `MISSED` as a claim to check rather than a result to
report. A "not caught" that is really "not applied" is the most expensive possible outcome of
this technique, because it manufactures a gap that then gets written down as fact.

> **Ask this:** *is the string I am replacing unique? If the same fix was applied in three
> places, does my anchor say which one?*

## 18. A guard that only runs when the work succeeded

**Written:** the cancellation check placed inside the `Ok` arm of each extraction loop, with an
`Err` arm beside it that only logged, and a `continue` that stepped past it when an answer was
dropped as ungrounded.

**What a person saw:** a worker whose claim had been revoked reading all twelve windows of a
page anyway — the exact cost the change existed to remove — whenever the model was returning
unparseable output or answers that would not ground.

**Why the tests missed it:** every test had the model succeeding. The stub answered valid JSON,
so the only path exercised was the one the check was on.

**Rule:** a check that decides *whether to keep going* belongs at the end of the iteration, not
inside the branch where the iteration went well. **The failure path is usually the one that most
needs it** — here a run erroring is a slow run, and a slow run is the one the staleness sweep
takes away, so the guard was missing precisely where its value was highest.

The same shape shows up as `continue`: an early exit written to skip the *body* also skips
everything after it, including code added later that has nothing to do with the reason for
skipping.

> **Ask this:** *what happens on the error branch and the `continue`? If this guard matters at
> all, does it still run when the thing above it failed?*

## 19. An async result applied without asking whether it is still wanted

**Written:** a fetch started on navigation, with `then`, `catch` and `finally` that all wrote to
state without checking the address bar still named the thing they had asked for.

**What a person saw:** press Back while a slow report is loading, and the report renders under
the wrong URL. Two more faces of the same race: a slow report overwriting the one asked for
*next*, and an overtaken request's `finally` clearing the "still opening" state of a newer one —
so the reader gets an empty box while their report is on its way.

**Why the tests missed it:** every test resolved its fetch immediately. A race needs one request
still in flight when the next event arrives, which a stub that answers instantly can never
produce.

**Rule:** an async result is a message from the past. Before it writes anything, it has to ask
whether it is still the current one — **carry a number and let only the newest write.** This is
the same shape as [the claim generation](../../../docs/decisions/0012-a-claim-is-a-number.md):
a state ("we are loading") cannot tell two loads apart, and a number can.

**And the cleanup path needs the check too.** `finally` runs for the loser as well as the
winner, so an unguarded one reaches across and clears the winner's state — the quietest of the
three, because nothing wrong is displayed, only something right removed.

> **Ask this:** *if a second one of these starts before the first comes back, which of them
> writes — and does the loser touch anything on its way out?*

### 19b. …and the guard has to outlive the thing it guards

**Written:** the fix for 19, with the counter declared *inside* the effect:

```tsx
useEffect(() => {
  let newest = 0;            // one per effect instance
  …
}, []);
```

**What a person saw:** the same overwrite, on a plain page load. **React Strict Mode runs setup
→ cleanup → setup on mount**, and the production entry point renders under it — so there were
two setups, each with its own counter starting at zero, and the discarded mount's request still
looked current when it answered.

**Why the tests missed it:** every test rendered `<App />` bare. The double-invoke only happens
under `<StrictMode>`, which is what `main.tsx` uses and no test did.

**Rule:** a guard's lifetime has to be at least as long as the thing it guards. An in-flight
request outlives the effect that started it, so the counter belongs in a `ref` — and cleanup
should invalidate it, which covers a real unmount as well as the discarded half of a double
mount.

**And test the arrangement production uses.** If the entry point wraps the app in something,
at least one test has to wrap it too, or the tests are exercising a configuration nobody runs.

> **Ask this:** *what is the scope of this guard, and what is the scope of the work it is
> guarding? Does anything in `main.tsx` change how the component mounts?*

## 20. A fixture that carried the property being asserted

**Written:** a test of merging two companies' reports, with claim text invented as
`A costs $10` and `B costs $20`.

**What a person saw:** two prices in one section with no way to tell whose either was. The real
extractors produce `Pro costs $15` — what the page says, and nothing about who said it — so the
merged output carried no company anywhere, and the interface rendered only the text and a bare
`[S1]`.

**Why the tests missed it:** the fixture named the company *inside the claim text*, which is the
thing the code was supposed to provide and does not. The assertion passed by reading data the
test had put there itself.

**Rule:** a fixture has to be shaped like what the real producer emits, especially in the field
under test. If the assertion would still pass when the code does nothing, the fixture is
answering the question instead of the code.

**A cheap check:** assert the fixture *lacks* the property, right where you assert the code adds
it — `assert!(!claim.text.contains("a.com"))` beside the check that the subject is `a.com`.

> **Ask this:** *where did this value come from — the code, or the test?*

## 21. Testing the helper and not the call site

**Written:** merge functions with thorough tests, called from a function nothing could reach.

**What a person saw:** nothing, and that is the point. Mutating `analyse_many` to concatenate
coverage instead of merging it, and to drop the truncation notice, left every test green —
because the tests called the merge helpers directly with hand-built inputs.

**Why the tests missed it:** the helpers were tested; the *decision to use them* was not. A
one-line call site is exactly the code most likely to be edited by someone who has not read the
helper's doc comment.

**Rule:** if a function is hard to reach, make it reachable rather than testing around it. Here
that meant driving the real entry point with origins in `.invalid`, which RFC 2606 guarantees
can never resolve — every fetch fails fast, and what is left is the joining. Six seconds of the
suite for two call sites that could not otherwise be asserted at all.

> **Ask this:** *if I deleted the call to the thing I just tested, would anything fail?*

## 22. A layer that wrapped only what existed when it was added

**Written:** the request-id middleware attached inside `router()`, with the single-page fallback
added afterwards by `with_ui()`.

**What a person saw:** every page and every asset came back with **no `x-request-id`, no span and
no access line** — ADR 0005's invariant broken on the one surface a visitor actually touches, and
on the only URL anybody types. The API kept its ids, so nothing looked wrong.

**Why the tests missed it:** the API tests asserted the header on API routes, which still had it.
Nothing asked the question about the surface that had just been added.

**Rule:** `Router::layer` wraps the routes present *when it is called* and nothing added later,
and the same is true of most middleware and decorator APIs. **Build the whole thing, then wrap
it** — and where that is awkward, make the wrapping the last step in one function so there is
exactly one place it can be got wrong. Here the routes are built without middleware and the layer
goes on outermost, so `with_ui` cannot forget.

> **Ask this:** *what did this wrapper actually enclose — everything, or everything that existed
> on the line above it?*

## 23. Deriving a fact from the evidence for it

**Written:** *"does this report cover several companies?"* answered by counting the distinct
subjects **on the claims**.

**What a person saw:** ask about Basecamp and Linear, have Linear yield no pricing, and
`Pro costs $15` renders with no company beside it — because one company produced claims, so the
report stopped calling itself a comparison. The label disappears from exactly the report that
needs it, since a reader who asked about two companies and sees one price cannot tell which one
it is.

**Why the tests missed it:** every multi-company test had every company producing a claim. The
proxy and the fact agree on the happy path and come apart when something is silent — which is
the register's oldest pattern wearing new clothes.

**Rule:** *"did anything come back for X"* is not *"was X asked about"*. When a decision depends
on **intent**, carry the intent — here `Report::subjects`, the list the run set out to cover —
rather than inferring it from the results, which are the thing that may legitimately be empty.

> **Ask this:** *am I counting the thing, or counting the evidence the thing produced? What does
> this say when the answer is none?*

## 24. Asserting a shape the producer does not emit — again, one comment later

**Written:** a merge test for coverage attempts, with fixtures built as
`"https://a.com/pricing"`.

**What a person saw:** `Discovered::attempts_for` stores `path_of(&c.url)` — **`/pricing`**, not
a URL, and the field's own doc comment explains why: *"the note is read under a heading that
already names the company"*. Multi-company reports broke that assumption, so merging gave two
companies an identical `/pricing (404)` each and a reader could not tell whose gap was whose. I
had written *"attribution survives because every path is a URL"* in the pull request. It is not,
and the test that was supposed to prove it had built the URLs itself.

**Why this one is here rather than folded into 20:** it happened **in the same pull request that
registered 20**, one review comment later. Knowing the rule did not help; the fixture was written
before the rule was, and nothing re-read it.

**Rule:** entry 20's rule, plus the part that makes it operational — **go and look at the
producer.** Open the function that builds the value and copy its shape, rather than writing what
the value "obviously" is. A doc comment on the field is usually where the assumption you are
about to break is written down.

> **Ask this:** *have I read the code that produces this, in this pull request — or am I
> remembering what it produces?*

## 25. Fixing the model and never looking at the surface

**Written:** `Attempt.subject`, merged `Coverage`, a passing test on the merged coverage — and
a web report still rendering `/pricing (404)` twice.

**What a person saw:** review, again, one round after the fix. The structured data was right;
the interface renders `section.checked`, which is a **list of strings each company rendered
before it knew there would be another one**, and `joined()` concatenated those strings. My tests
asserted the merged `Coverage` and called `to_section()` themselves — neither of which is the
value the interface reads.

Two more of the same shape landed in the same review. `several` was correct on the finished
report and guessed from the claims **during the stream**, because `subjects` lives on a report
that is not fetched until the run ends — so the label was wrong for the ninety seconds a reader
is actually watching. And `<strong>{subject}</strong>{claim.text}` rendered
`basecamp.comPro costs $15`, because JSX drops the whitespace around a newline and every
assertion so far had matched the two halves **separately**.

**Rule:** a fact has as many surfaces as it has renderings, and fixing the one that holds the
data fixes none of the others. Fix a fact and then **list the surfaces**: the CLI, the merged
report, the live stream, the rendered line. Assert on the value each surface actually reads —
`report.sections[*].checked`, not the `Coverage` behind it; the whole line's text, not the two
halves that compose it.

And the timing half: a fact that arrives with the finished report is absent for the entire wait.
If a reader looks at something before the report exists, whatever decides how it is rendered has
to reach them before it does.

> **Ask this:** *what does each surface read — and does the one a person looks at longest have
> this yet?*

## 26. Validating the container and trusting the leaves

**Written:** a body checked with *"is `examples` an array, is `note` a string"*, then
`return body as Examples`.

**What a person saw:** review. One entry with `companies: null` passes that guard untouched,
reaches `companies.join(" vs ")` in the renderer, and throws **while the first screen is drawing
itself** — on the one code path whose whole design is to degrade in silence. The guard reads as
validation. It validates the box the data came in.

**Why the test missed it:** my malformed fixture was malformed *at the top level*
(`examples: "not a list"`), so it exercised the container check and never an entry. A fixture
that is broken in the shape the guard already handles cannot find the shape it does not.

**Rule:** a cast is a claim about **every field of every element**, so validate to the depth the
renderer reads. `as` after a partial check is the same lie as `as` after no check. And when a
list can be partly bad, decide which way it degrades: dropping bad entries costs a reader one
row, dropping the list makes one broken row look like an outage.

> **Ask this:** *what does the renderer actually touch — and is every one of those reached by
> my check and by my fixture?*

### And the tail of this one, which is about the harness

Fixing it made an existing mutation stop being a defect. Removing the `Array.isArray` check no
longer changes anything, because a non-array now throws inside `.filter` and the surrounding
`try` returns the same `null`. **A mutation whose defect has become unreachable is a check that
cannot fail**, so it was re-aimed at a rule that is real and was not enforced: a list that
arrives without the sentence qualifying it must render **no chips at all**, not chips without
the qualification. Retiring a mutation is a normal outcome; leaving it in place to keep a green
line is not.

## 27. A decision made in four steps, with an await between each

**Written:** read what this address started, ask the store which of those still count, enqueue,
record the new id — a per-client cap, in four `await`s.

**What a person saw:** review sent twenty simultaneous requests carrying one address at a limit
of two, and **nine were accepted.** Every one of them read the same empty list before any of
them had recorded anything. Each step was right; the sequence was not atomic, and a cap is a
decision about a sequence.

**What made it mine rather than unlucky:** the *previous* version of this held a single
`Mutex` and did all its counting inside it — atomic by accident. Replacing the reservation with
"ask the store, then decide" introduced `await` points into what had been one critical section,
and I did not notice that I had spent the property that made it correct.

**Rule:** check-then-act across an `await` is check-then-act across a gap somebody else fits
through. When a fix turns a synchronous decision into an asynchronous one, ask what was true
because it was synchronous. Serialise the whole sequence — per key, so one caller does not wait
behind another's store reads.

> **Ask this:** *between reading and acting, does anything yield — and what happens if a second
> request arrives exactly there?*

### And the regression for it was flaky, which is nearly as bad

The first version of the concurrency test caught the missing lock on one run and missed it on
the next. `MemoryStore::get` takes a lock and returns **without ever awaiting**, so twenty
concurrent requests can run to completion one after another and a genuine race looks fine.
Postgres does I/O and yields every time.

**A test double that is faster than production hides exactly the bugs concurrency tests exist to
find.** The fix was a store wrapper that yields between its steps, putting the interleaving point
back where a deployment has it — and the mutation then failed five times out of five instead of
three.

> **Ask this:** *is my double faster than the real thing in a way the test depends on?*

### And the third round: the clock the decision carries

Serialising the sequence was still not the end. Each request captures the date **before** it
waits for its gate and for the store, so one admitted a second before midnight finishes a second
after it — and a rollover written as *"reset whenever the date differs"* let that request wind
the day **backwards** and clear what the new day had already recorded. The next request rolled
forward into an empty day and was handed a fresh allowance. Review reproduced it deterministically
through the public API.

**A value captured before an await is a value from the past by the time it is used.** Rollover is
monotonic now, and a caller carrying an old date is ignored rather than allowed to rewrite
history — which costs at most one uncounted run per address per midnight, bounded and in the
forgiving direction, where losing the day's record was neither.

> **Ask this:** *what did this request capture before it waited, and is it still true?*

### And the fourth round: harmless is not the same as bounded

Making the stale path harmless was still not the end, and the reason is worth the entry on its
own. I wrote that ignoring a stale caller *"costs at most one uncounted run per address per
midnight"* — a bound, stated in a comment, that I had not checked.

It was false. A gate is asked for **before** it is awaited, so any number of requests can take
the old day's gate and queue. When one new-day request advances the day and clears the gate map,
every waiter still holds the old `Arc`: each then reads an empty list, has its writes dropped,
and is admitted free. **A queue built up before midnight was admitted whole**, at a boundary
anybody can predict. Review found it.

The fix is to notice the day moved *after* acquiring the gate, and start again under the current
one. What genuinely crosses is the single request already admitted when the day turned — which
is what I had claimed without having earned it.

**Rule:** "this can only happen once" is a quantity, and a quantity in a comment is a claim. If
it is worth writing down, it is worth a test that counts — mine asserts *how many* got through,
not merely that the mechanism exists.

> **Ask this:** *I have written "at most one" — one what, and what stops the second?*

---

## 28. The harness that could not fail either

**Written:** `scripts/mutate.py`, reporting `MISSED` for fourteen mutations in a row.

**What a person saw:** me, believing the suite had gone blind. It had not. A hung test process
was holding the compiled binary, so every link failed with `LNK1104`, **no test ran at all** —
and the harness read "no test reported a failure" as "no test noticed". Its only guard against
that was a check for `error[`, which catches rustc's own diagnostics and not a linker error, a
locked executable, or a runner that died.

**The tool built to find checks that cannot fail contained one.**

Worse, the interrupted run left a deliberate defect in the tree — `analysis.status !=
AnalysisStatus::Failed`, the inverse of the rule the pull request existed to add — and `git add
-A` committed it. I then checked the one file I suspected, found and fixed it there, and did not
check the others. A second file had it too.

**Rules, three:**

- **A verdict needs positive evidence that the thing ran.** `mutate.py` now requires a test
  runner's summary line before it will report anything at all; without one it says `BROKEN`.
- **Never `git add -A` while a harness is editing the tree.** The mutation files are a list of
  defects this repository can recognise, so `scripts/no_live_mutations.py` reads them the other
  way round and refuses a tree containing one. It is the first gate in `verify.py`, and the only
  one that runs against the **working tree** rather than a clean checkout — because its job is
  to stop something reaching a commit, not to notice afterwards.
- **When a tool has corrupted the tree once, check all of it.** I fixed the file I expected and
  shipped the one I did not, which is the same error twice in five minutes.

> **Ask this:** *did this actually run — and is the tree still the tree I think it is?*

## 29. Checking two sources instead of having one

**Written:** three defects in one extractor, and review found all three.

- The model was asked for a compliance standard's **name**, and the answer was checked for
  *containment* in the window. A window naming two standards let an answer about the other one
  through, with a verbatim quote about the first — a fabricated certification claim about a
  real company, fully cited.
- Match offsets were found in a `to_lowercase()` copy and applied to the **original** string.
  `to_lowercase` is Unicode-aware and can change length, so slicing panicked.
- A distinct-standard count was kept by comparing against a list the **cap had truncated**, so a
  repeat past the cap counted twice.

**What they have in common:** in each case there were two sources of truth and a check between
them. The fix in each case was to delete the second source, not to compare more carefully — take
the name from the scanner so the model cannot supply one, fold ASCII so offsets cannot move, keep
an uncapped record so the comparison is against everything.

**Rule:** entry 4 says *do not derive one fact in two places*. This is its sharper form: **when
you find yourself writing a check between two sources, ask whether one of them can simply not
exist.** A guarantee by construction cannot be forgotten, cannot drift, and does not need a test
to keep it honest — a check needs all three.

> **Ask this:** *am I comparing two sources of this, and can I delete one?*

## 30. "This cannot be tested" is a claim like any other

**Written:** by me, in a reply to a reviewer — that the extraction stage could not be tested
without a running model, and that a mutation guarding its most dangerous line was therefore
being dropped as untestable.

**What a person saw:** the reviewer replaced `claim.about(&named.standard)` with a hard-coded
standard, ran the suite, and watched all 572 tests pass. The class of defect the previous round
had removed could walk straight back in.

**There is a `StubModel` forty lines below the code I was editing**, used by nine existing tests
in the same file. It serves canned JSON over a real socket, and the stage runs against it in
milliseconds. I did not look.

**Rule:** the words *"this cannot be tested"* are load-bearing — they are usually the reason a
guard is missing — so they need the same evidence as any other claim in a pull request. Before
writing them: read the test module of the file you are editing. The harness for the thing you
think is untestable is very often already there, written by you, for the thing next to it.

> **Ask this:** *what does the test module of this file already stand up — and did I look before
> saying I could not?*

### The sequel, in the same pull request

Removing the model's ability to *name* the standard was written up — by me, in a comment, in
`BENCHMARKS.md`, and in a reply — as making the failure **unrepresentable**. Review then found
the failure still live, *inside the regression I had written to prove it was closed*.

Taking the name away stopped the model **labelling** an answer. It did not stop it **answering
about the wrong thing**: a window is three lines, two standards often sit within three lines of
each other, so the same window is handed over twice and an answer about the first can be
relabelled as the second. Every check passed — the quote was verbatim, the name was the
scanner's — and the report published a certification claim on evidence about a different
certification.

**Rule:** *"unrepresentable"* is the strongest claim available, so it needs the strongest
evidence. Removing one route to a defect is not removing the defect. Before writing the word,
name the property exactly — *the model cannot choose the label* — rather than the family it
belongs to, because the gap between those two sentences is where the next finding lives.

> **Ask this:** *have I closed the class, or one door into it — and can I say which in one
> sentence?*

## 31. A number written down instead of read back

**Written:** by me, in `docs/BENCHMARKS.md` and in three replies on one pull request — *583 Rust
tests (up 2)*.

**What a person saw:** the reviewer ran `python3 scripts/verify.py` at the same commit and it
printed `586 tests run`. Everything else on the pull request was correct; the only wrong thing in
it was a number I had produced by **adding two to the last one I remembered** rather than by
reading the run that had just finished on my own screen.

**It is entry 29 wearing different clothes.** The suite is the source of truth for how many tests
there are; the table was a second copy, updated by arithmetic. The same shape as every other
finding in that pull request — assert the property, do not establish it — except this time the
tool that establishes it had already printed the answer and I did not look at the line.

**The fix is not "be careful with numbers".** `verify.py` now reads the `| now |` row of
`BENCHMARKS.md` back against the counts the gates just produced, and fails when they disagree.
The table stays, because a benchmark file is a record of history and cannot be deduplicated away
— so it is *checked* instead, by the run that produced it.

**Rule:** any count that appears in a document is a claim about a command's output. Paste it from
the command. If it is going to be repeated — a test count, a gate count, a mutation count — the
cheapest honest version is a check that reads it back, because the alternative is remembering to
re-run something every time the number moves.

> **Ask this:** *did I copy this number from the tool that produces it, on this commit — or did I
> compute it?*

## 32. A passing test that holds a different property than its name

**Written:** by me, in `landscape-extract::hiring`, twice in one file.

**What the harness saw:** two mutations survived. `a_sentence_about_the_list_is_not_in_it`
deleted the full-stop rule and still passed; `a_one_word_navigation_label_is_not_a_role` deleted
the two-word minimum and still passed.

**Both tests were green, both assertions were right, and neither was testing what it said.** The
sentence — front.com's *"Browse our open positions and find your dream job."* — was rejected by
the **word list**, because `positions` and `job` are not job words; the full stop never got a
turn. The navigation label — Linear's `Developers` — was rejected by the **plural rule**, not by
the length floor. Each test named a guard and exercised a different one, so either guard could
have been deleted in silence.

**And one of them should have been.** Looking for the missing test on a third mutation showed
there was none to write: no title on any of the three frozen pages is plural, and every line the
plural rule reached was a navigation label. The rule widened the false-positive surface and
bought nothing, so it went. **The harness removed code rather than gaining a test**, which is
the more useful of its two outcomes and the one that never happens if a survived mutation is
answered by reflex with a new assertion.

**Rule:** a test named after a guard has to fail when *that* guard is removed, and only the
mutation harness can tell you whether it does. When a mutation survives, ask which of the two
things is true before writing anything: *the guard is untested*, or **the guard is unnecessary**.
Reaching for a test first is how a rule nothing needs acquires a test that keeps it for ever.

> **Ask this:** *if I delete the guard this test is named after, does this test fail — or is
> something else rejecting my fixture?*

## 33. A fallback that runs the safe path's rules on unsafe input

**Written:** by me, in `landscape-extract::hiring`, in the same file, in the same pull request.

**What a reviewer saw:** a careers page with no recognised *Open roles* heading published
`lists an open role: Kelsey Weber , Engineering Manager` — a testimonial byline — as a
**high-confidence** claim about a company hiring a person who already works there.

**The scoped path was correct and the fallback reused its rules.** A title is short, has no
terminal full stop, and carries a job word: that is enough to clean up *inside* a list somebody
has pointed at, and nowhere near enough to *find* one. When no heading matched, the scan fell
back to the whole page and those same three rules were the only thing left. **I had written the
counterexample into the module's own documentation** — that exact line, named as the danger — and
then let the fallback run past it.

**And the honesty was in the wrong place.** The stage emitted a run-log line saying the page had
been read unscoped. Nothing carried it into the report, so the reader saw a confident claim and
the caveat lived in a diagnostic. That is Run 24's mistake exactly, one question later.

**Rule:** when the happy path is *"somebody told us where to look"*, the fallback is not the same
code over a wider range — it is a **different problem**, and usually a refusal. Before writing
one, ask what the narrow path's rules were relied upon to do, and whether they can carry the
weight alone. And a caveat that lives only in a log is not a caveat: if the output cannot state
it, the output should not exist.

> **Ask this:** *what is my fallback assuming that my main path was given? If the answer is
> "where to look", why is reading everything the right answer instead of reading nothing?*

## 34. A sentence written from the plan, not from the run

**Written:** by me, in `landscape-analyze`, three times in one pull request, each a different
surface of the same habit.

**What a reviewer saw:**

* The search note said *"3 page(s) were read"* — computed immediately after admission, before
  a single one had been fetched. A dead host, a page below the quality floor, and a run the
  reader walked away from all counted as reads.
* A question whose only page came from search reported *"nothing was checked - our gap, not
  theirs"*, because `Coverage` is built from what discovery admitted and search was the first
  thing ever to add a page discovery had not.
* Both note branches said *"this company"*, and `analyse_many` merges every subject's notes and
  drops duplicates — so two companies with the same gaps collapsed into one ambiguous sentence
  and the other was thrown away.

**None of these is a wording problem, and treating them as one is the trap.** The first is a
count taken from a list of intentions; the second is a summary derived from one of the two
sources that now feed it; the third is a string that was correct in the only context it had ever
been rendered in. The reading itself was right in all three cases, which is what made them easy
to miss: the *account* of the run was assembled from what the code meant to do.

**Rule:** a sentence about what a run did has to be computed from the record of what it did —
after it did it. When you add a second producer of anything a summary reads, the summary is part
of the change, not a caller of it. And a note that will be merged with somebody else's has to
name whose it is, because deduplication cannot ask.

> **Ask this:** *is this number counted from what happened, or from what I asked for — and if I
> add a second source of these, does everything that summarises them know?*

### The sequel, and it is the older rule failing

The fix for the coverage half of that entry extended `Coverage.sources` — and **I asserted on
`Coverage.sources`**, which is the field, not the surface. Review ran the same state through
`Coverage::note()` and got:

```text
read 1 page(s), none stated anything. Checked: nothing
```

`sources` is a count; `attempts` is the list a reader is shown, and only discovery wrote to it.
So the report said a page had been opened and offered nothing to go and look at — for a page on
somebody else's host, where the host is the only thing that identifies it.

**Entry 25 is the rule I already had**: *assert on the value the surface reads, not the structure
behind it.* I had it, and I still reached for the field, because the field was the thing I had
just changed. The tests assert on `note()`, on `to_section().checked`, and on `render()` now.

> **Ask this:** *is my assertion on the thing a person sees, or on the thing I just edited?*

## 35. A threshold asserted as a number instead of as the decision it drives

**Written:** by me, in `landscape-search::candidates`.

```rust
assert!(found[0].confidence < 0.5, "an outage produced confidence: {found:#?}");
```

**What a reviewer saw:** three queries sent, two failing, and the single hit from the third
scoring **0.47** — which is under 0.5, so the test passed, and over
`subject::MINIMUM_CONFIDENCE = 0.35`, so the gate **resolved it** and an analysis would have run
against a company that appeared in one search. The test was named for the property *"an outage is
not unanimity"* and asserted a number that had nothing to do with the floor the number is
compared against.

**A threshold has exactly one meaning, and it is downstream.** `0.5` was a number I picked while
writing the test because it looked comfortably low. The only number that means anything about
that score is the one the gate refuses at — and it lives in another crate, which is exactly why
reaching for a literal felt reasonable.

**The fix has two halves.** The regression runs `suggest → describe → resolve` and asserts the
**verdict**, not the score. And the score for an uncorroborated candidate is now *derived from*
`MINIMUM_CONFIDENCE` rather than chosen to sit below it, so moving the gate's floor cannot
silently make uncorroborated candidates resolvable again.

**Rule:** when a value exists to be compared against a threshold, a test that asserts anything
other than the comparison's *outcome* is testing a number nobody uses. And when code has to sit
on one side of somebody else's constant, derive it from that constant — a literal chosen to be
"clearly below" is a second source of truth with no way to notice it drifting.

> **Ask this:** *what decision does this number drive, and am I asserting the decision or the
> number?*

### The sequel: a sample of a list is not a small version of the list

The fix for the first half of that entry — one company arriving as three subdomains — was a
**hand-written list of thirty multi-label public suffixes**. It covered `co.uk` and missed
`github.io`, so two unrelated tenants became one company:

```text
alpha.github.io  ->  github.io
beta.github.io   ->  github.io      one "company", agreed = 2
```

**Two different queries, each finding a different tenant, forging the corroboration the entry
above had just made mandatory.** A missing suffix does not merely merge two companies; it
manufactures the evidence that lets the merged thing auto-resolve, one round after the rule went
in to prevent exactly that.

**And I had written the counterargument into the list's own doc comment.** It said the worst
outcome was *"two candidates where there should be one"* — the safe direction — and, four lines
later, that an unlisted suffix *"groups one label too short, which merges rather than splits"* —
the unsafe one. Both sentences, one comment, and I shipped the justification rather than the
contradiction.

**Rule:** a curated subset of a maintained list is not a smaller version of it — the entries you
know to add are exactly the ones that were never the problem. Where the failure mode is *not
knowing what is missing*, the list is the dependency, and the argument for hand-rolling it has to
survive being written down beside its own exceptions.

> **Ask this:** *does my comment justifying this contain the sentence that refutes it — and which
> direction does being wrong fail in?*

## 36. A fixture with one element cannot test an ordering

**Written:** by me, testing that a report says *who it is about* before it says anything else.

```rust
assert_eq!(notes.iter().position(|n| n.contains("You described")), Some(0));
```

**Why it could not fail:** the fixture analysed **one** company, so `notes` held exactly one
note. `insert(0, ..)` and `insert(len(), ..)` put it in the same place, and the mutation that
moved the sentence to the bottom of the report passed the whole suite. The assertion named a
position and the fixture had no positions.

**This is the fourth time in three pull requests** that a test covered something other than what
its name claimed — a sentence rejected by the wrong rule, a page absent from `pages` rather than
present-and-unread, a helper tested while its call site was not, and now an ordering with nothing
to order. Each one was found by the mutation harness and none by reading the test.

**Rule:** a property about *position*, *precedence*, *first*, *worst* or *nearest* needs a
fixture with at least two candidates for the position to be about — and ideally one where the
wrong answer is the one a naive implementation would give. Before asserting an index, ask what
else is in the collection; if the answer is "nothing", the assertion is about a constant.

> **Ask this:** *if my fixture had one element, would this assertion mean anything?*

---

## 37. A failure counted and then dropped, and the silence that followed reported as a fact

**Written:** by me, resolving a description into a company through three search queries.

```rust
let (found, failures) = suggest(engine, description).await;
if failures > 0 {
    tracing::warn!(failures, "some candidate searches did not complete");
}
match verdict {
    Resolution::NothingFound { .. } => Chosen::None(NOTHING_RESOLVED.to_owned()),
    ...
}
```

**What a reader got when all three queries failed:** *"we searched for companies matching that
description and found none"* — and, as the evidence of the looking, a `checked` list holding all
three queries, none of which had reached an engine.

**Why it is not a logging bug.** The count was correct, was carried out of the function, and was
even written down. It just never reached the sentence. **A failure that is observed and then not
allowed to change any decision is the same as a failure that was swallowed** — the log line makes
it worse, because it reads like handling.

Two rules had been collapsed into one number, and they pull in opposite directions:

| | Counts | Because |
|---|---|---|
| The score | queries **sent** | One answer out of three is not agreement. Divide by what answered and an outage manufactures unanimity. |
| The audit trail | queries **completed** | It is shown as *"we checked these"*, and a query that never ran checked nothing. |

A single `usize` cannot serve both, and whichever rule you write first, the other silently gets
the wrong answer. `Queried { completed, failed }` is the smallest type that can.

**And the refusals split too.** *"We looked and found nobody"* and *"we could not finish
looking"* are different facts about different subjects — one about a market, one about us — and
only the second is fixed by waiting. This is the third time this project has found two sentences
sharing one branch; each time the fix was a sentence, not a flag.

**Rule:** when a failure produces a count, find the sentence that count must change before
writing the log line. If no sentence changes, the count is decoration. And prefer a small struct
over a number the moment two callers want to divide by different halves of it.

> **Ask this:** *if every one of these calls failed, what would a reader be told — and is it true?*

---

## 38. A `MISSED` that meant "the wrong suite ran", not "nothing tested this"

**Written:** by me, cataloguing the mutations for competitor-set derivation.

```json
{
  "name": "a company found and left out is dropped in silence",
  "file": "crates/landscape-search/src/competitors.rs",
  "new": "            (Some(_), _) => {}",
  "run": ["cargo", "nextest", "run", "-p", "landscape-analyze"]
}
```

`MISSED`. The obvious reading is *there is no test for this*, and the obvious response is to
write one — which would have added a second test for a property already covered, and left the
catalogue entry still lying.

**What had actually happened:** the mutated code is in `landscape-search`; the test that catches
it is in `landscape-search`; the `run` said `landscape-analyze`. That suite compiles the mutated
crate, passes, and reports nothing wrong — because nothing in it exercises the mutated line.

**Why this is worse than `NOT APPLIED`.** A wrong `file` or a stale anchor is *loud*: the harness
says the anchor is not there and you go and look. A wrong `run` is **silent and looks exactly
like a real finding**, and the natural response to it — write another test — makes the suite
bigger, makes the catalogue *stay* wrong, and produces a green run that proves less than it did
before. This is entry 17's sibling: there the mutation had been applied to a different copy of
the code; here it was applied to the right code and checked by the wrong suite.

**The check is one question and it costs nothing:** for every `MISSED`, before writing a test,
grep the crate the mutation lives in for a test that names the property. If one exists, the
catalogue is wrong, not the code. Two of this catalogue's four `MISSED`/`NOT APPLIED` reports on
the first run were catalogue errors rather than coverage gaps.

**Rule:** a mutation's `run` is part of the mutation, not a convenience. Default it to the crate
the `file` is in, and only widen it when the *caller* in another crate is what you are testing —
in which case say so in the name, because a reader of the catalogue cannot see the difference
either.

> **Ask this:** *is this `MISSED` telling me about my code, or about my catalogue entry?*

---

## 39. A cap written for one consumer, still applied when a second one arrived

**Written:** by me, deriving a competitor set from candidates that were already being scored.

```rust
found.truncate(MAX_CANDIDATES);   // "the most companies worth putting in front of a reader"
```

That line was **correct** when it was written. The only consumer was a disambiguation chip list,
and twenty candidates in a chip list is a search results page with your name on it.

Then a second consumer arrived — a *set*, whose entire promised guarantee was *"a company we
found and did not compare is named, never dropped"*. Six corroborated companies in, five out, and
the sixth was neither a member nor an exclusion. **The guarantee was being broken one function
above the code that makes it**, by a line nobody had touched and whose comment still read true.

**Why the review caught it and the tests did not.** The test asserted `found.len() ==
MAX_CANDIDATES` — the cap was *pinned*, and there was a mutation defending it. Every test agreed
the truncation should happen, because every one of them predated the consumer that made it wrong.
A green suite here means *"this still does what it used to"*, which is the one question that
cannot detect this class of defect.

**What the fix is not.** The budget was real — each name costs a request against a stranger's
server — so deleting it would have traded one defect for a politer one. It moved from
*truncating the list* to *bounding the fetch*, and what it costs became something a reader is
told: a fifth `Aside`, `BeyondTheFetchBudget`, naming the company and the number.

**Rule:** when you add a consumer to an existing pipeline, read every `truncate`, `take`, `min`,
`filter` and early `return` upstream of it and ask *whose question was this answering?* A limit
justified by presentation is not a limit on data, and the comment above it will not say so —
because when it was written there was no difference.

> **Ask this:** *this cap was right for the old caller. Is my new caller the same kind of caller?*

### 39b. And then I deleted it, which broke the caller it was written for

**One review round later.** The fix above removed `found.truncate(..)` so the set could see every
company. The *same list* still fed the disambiguation gate, so the gate got all of them too:

```text
the reader is offered 6 choices:
   Company at https://c0.example/ (c0.example)
   ...
   c5.example (c5.example)
```

Six choices where `PRODUCT_SPEC.md` §3 asks for three — bounded now by *how many results a
provider felt like returning* — and the last one is a bare domain printed twice, because
candidates past the fetch budget were never named from their own pages. **The same defect, from
the opposite direction, introduced by the fix for it.**

**Why I did not see it.** I checked what the removal gave the new consumer and never asked what
it took from the old one. Deleting a constraint feels like a smaller act than adding one, and it
is not: a shared limit is load-bearing for every caller downstream of it, and there is nothing at
the deletion site that names them.

**What was actually wrong both times: one number serving two questions.** *How many companies did
we find* and *how many can a reader be asked to choose between* are different, and no value of
one constant is right for both. The answer was two lists off one budget — the complete ranked
list for the set, and the fetched-and-named subset for the gate, with the split expressed as
`Described::was_requested()` rather than as a second comparison against `NAMED` somewhere else.

**Rule:** removing a shared limit needs the same list of callers as adding one. Before deleting a
`truncate`, `take` or `filter`, enumerate everything downstream that reads the result and say out
loud what each one now receives. If two of them want different amounts, the fix is two lists, not
a different number.

> **Ask this:** *who else was relying on this being small?*

---

## 40. A justification in a doc comment that had never been tested against its alternative

**Written:** by me, matching a reader's words against a company's front page.

```rust
/// Substring matching on a lowercased page, so `analytics` is found inside `web analytics` and
/// inside `Analytics.` — a page is prose, not a token stream, and requiring whole-word equality
/// would miss a company whose front page is one styled sentence.
pub fn shared(words: &[String], page: &str) -> Vec<String> {
    let lowered = page.to_lowercase();
    words.iter().filter(|w| lowered.contains(w.as_str())).cloned().collect()
}
```

**What it did:** `content_words("art marketplace")` yields `art`. A front page reading *"Tools
for startups"* contains those three letters inside `startups`. One shared word is enough to admit
a company, so an unrelated company entered the report — and the sentence explaining why told the
reader *"its own front page uses \"art\""*, citing a word that page never used. **Evidence
manufactured out of a coincidence of spelling.**

**The defect is in the paragraph, not the line.** Both examples the comment offers —
`web analytics` and `Analytics.` — are found *perfectly well* by splitting on non-alphanumerics.
The justification described a cost that the rejected alternative does not have. It was written
from the shape of the idea rather than from trying it, and then it sat there being persuasive:
the test underneath it was named `sharing_is_not_case_sensitive_and_reads_inside_a_word`, so the
wrong behaviour had a test asserting it was intended.

This is [entry 34](#34-a-sentence-written-from-the-plan-not-from-the-run)'s failure aimed at a
rationale instead of at output. A confident *"X would not work because Y"* is a claim about Y,
and it is checkable in about a minute.

**Rule:** a doc comment that rejects an alternative has to name a case where the alternative
actually fails, and that case belongs in a test. If you cannot write the test, the comment is a
preference wearing an argument's clothes — write *"chosen because"* rather than *"because the
other one would"*.

> **Ask this:** *have I run the thing this comment says would not work?*

---

## 41. A killed mutation run leaves the defect in your tree, and the next run believes it

**Found:** while answering the review round above, by noticing a `git diff` hunk I had not written.

The catalogue takes longer than ten minutes, and I ran it in the foreground with a ten-minute
timeout. It was killed mid-mutation. `mutate.py` restores the file in a `finally`, and a
`SIGTERM` at the wrong moment does not run it — so the tree was left holding the mutated line:

```rust
// what I wrote
let choices: Vec<Candidate> = named.into_iter().filter(Described::was_requested)...
// what was in the file afterwards - the mutation's `new`
let choices: Vec<Candidate> = named.into_iter().map(|d| d.candidate).collect();
```

**Then I re-ran the catalogue, and that is where it got dangerous.** Every mutation copies the
current file as its backup and restores it afterwards, so the second run's baseline *was the
mutated code*. Thirty-eight entries were measured against a tree with the defect in it. The
thirty-ninth — the one whose anchor the kill had eaten — reported `NOT APPLIED`, which reads like
a stale anchor rather than *"the code you are testing is not the code you wrote"*.

**Nothing in the run says so.** `caught` still prints, the counts still add up, and the summary
line still says how many are covered. A poisoned baseline produces a report that looks exactly
like a clean one.

**This is the second time, and the first time already has a script.**
`scripts/no_live_mutations.py` opens by saying *"this exists because it happened"* — a cut-short
run once left an inverted rule in the tree and `git add -A` would have committed it. That guard
is `verify.py`'s **first** gate, it works, and it is what finally confirmed the tree was clean
here. The gap is *when* it runs: at verify time, which is after every mutation result has already
been read and believed. A `git diff` before re-running would also have shown a hunk I did not
write, and that is what eventually found it — several steps later than it should have.

**The mechanical fix, not the resolution to be careful:** `mutate.py` now refuses to start while
any catalogued mutation is live in the tree, and says which one and how to restore it. A harness
whose correctness depends on remembering to check something by hand is the same defect this file
keeps recording, one level up.

**Rule:** run the catalogue in the background or with a timeout longer than it takes — never with
one that can kill it. And treat *any* interrupted run as having poisoned the tree: `git diff`
before doing anything else, because the next thing that reads those files will believe them.

> **Ask this:** *the last run of this died. Is the tree the tree I think it is?*

---

## 42. A regression pin that came loose, in a catalogue nobody had reason to re-run

**Found:** by review, at `a084775`.

```text
1 of 27 not caught - each one is a test that cannot fail
   NOT APPLIED  a description that matches two companies picks one of them
```

`cargo fmt` had collapsed a `subject::resolve(...)` call onto one line two commits earlier. The
mutation pinning the ambiguity behaviour still expected the multi-line form, so the harness could
not apply it — and printed `NOT APPLIED`, which reads like a stale anchor and is *also* the thing
that stops a real property from being covered.

**The process failure is the interesting part.** I re-ran the catalogue I was working on, saw all
39 caught, and never ran the other five. There was no reason to think a change in
`landscape-search` could loosen a pin in `read-order.json` — and it can, because `cargo fmt`
reflows whatever it touches and a mutation's anchor is a *verbatim string*.

**Running every catalogue is not the fix**, because it takes tens of minutes and that is exactly
why `scripts/verify.py` never ran them at all. Reading each `old` out of the JSON and checking it
appears in its file **exactly once** takes a moment and catches the whole class:
`scripts/mutation_anchors.py`, now the second gate.

**What it found on its first run is the point.** Not one loose pin — **five**, across four
catalogues, rotting since earlier phases:

| Pin | Loosened by |
|---|---|
| the ambiguity call | `cargo fmt` reflowing it, two commits back |
| a changelog needs no model | `Answers::Direction` joining the `matches!`, PR #44 |
| the model's health | a local becoming a struct field, PR #46 |
| a company dropped in silence | `let notes` becoming `let mut notes` |
| a trust page reaching its extractor | the dispatch moving to another file entirely |

Every one of those catalogues had reported *"all caught"* on the day it was written and had been
quietly proving less ever since. **A suite of regression pins decays exactly like a suite of
tests, except that nothing fails when it does** — the harness prints a line and returns 1, and
nobody runs it.

**And two of the five, once retargeted, came back `MISSED`** — which is the more useful half of
the finding. The pins had been loose long enough that the properties underneath had quietly lost
their coverage: nothing asserted that a changelog is read while the model is down (the gate sits
behind a fetch the test fetcher refuses), and nothing asserted that a trust page reaches its
extractor at all (the catalogue pinned what the extractor *finds*, never that the dispatch still
arrives there). Both are now tested. This is entry 38's question — *is this `MISSED` about my
code or my catalogue?* — coming out the other way for once.

**Rule:** any artefact that pins code by *quoting* it — mutation anchors, golden-file excerpts,
documentation snippets, `expect_test` blocks — needs a cheap mechanical check that the quote is
still there, and that check has to run every time, not when somebody suspects something. If
verifying the artefact properly is slow, verify that it is still *applicable* quickly. And when
a loose pin is retightened, **run it** — a pin nobody could apply is a pin nobody was covering.

> **Ask this:** *this pin quotes code. What tells me it still quotes code that exists?*

---

## 43. The harness put a file back, and took two tests with it

**Found:** by the harness, one run later.

Two mutations came back `MISSED`. Both were real gaps, so I wrote the tests, watched 780 pass,
and re-ran the catalogue. One of the two was still `MISSED`.

```bash
$ grep -c a_named_company_takes_its_name crates/landscape-search/src/candidates.rs
0
```

**The test was gone.** A *different* catalogue was still running in the background when I wrote
it. `scripts/mutate.py` copies each file before mutating it and moves the copy back in a
`finally` — and the copy it held for `candidates.rs` predated my edit. The restore was correct
by its own lights and silently deleted work.

**Two things made it nearly invisible.** The sibling edit to `competitors.rs` survived, because
that file was not in the catalogue that was running — so *one of two tests written in one command
disappeared*, which looks like nothing at all. And `cargo nextest` passed: a test that does not
exist does not fail.

**The only thing that noticed was the mutation the test was written for.** That is the harness
doing the job it exists for, aimed at the harness.

**The mechanical fix.** `mutate.py` now reads the file back before restoring and compares it
against the text it wrote. If they differ, somebody else has edited it since, and the backup is
**not** put back:

```text
CHANGED      crates/landscape-search/src/candidates.rs was edited while this ran.
    The original is in candidates.rs.mutate-backup and has NOT been put back, because
    restoring it would delete whatever was just written. Merge it by hand.
```

This is the third entry about the same window — **41** was a run that died holding a file, **42**
was a pin that came loose under one, and this is one that put a file back over somebody's work.
**A tool that edits your source in place is a second author**, and every rule about two authors
touching one file applies to it.

**Rule:** while a mutation run is going, the working tree belongs to the harness. Edit docs, write
the PR, read code — do not write source. If a background job is long enough that you will forget
it is running, that is the argument for the machine checking rather than for remembering.

> **Ask this:** *is anything running right now that owns the file I am about to write?*

---

## 44. A sentence written for a reader, mined for data by something else

**Written:** by me, giving a named company's competitors something to be judged against.

```rust
let words = content_words(&seed.what_it_is);   // the seed's own description of itself
```

Which is exactly right, until the seed's front page cannot be read — at which point
`what_it_is` holds the fallback sentence *"we were unable to read its front page"*:

```text
content_words = ["were", "unable", "read", "front", "page"]
a rival page saying "Read more on this page."
shares        = ["read", "page"]
```

An unrelated company entered the report, and the reason shown to the reader — *"its own front
page uses \"read\", \"page\""* — cited **our error message** as that company's evidence.

**The field had two jobs and only one of them was declared.** `what_it_is` is prose written for a
person, and it also carried a fact: *whether anybody read the page*. Nothing at the call site
said so, because the fact was implicit in the wording. Every consumer that treated it as data
inherited the second job without knowing it existed.

The fix is not a better sentence or a check for that sentence — both leave the coupling in place
and the next fallback wording breaks it again. The fact moved out:

```rust
pub struct Seed { pub candidate: Candidate, pub read: bool }
```

**This is the same class as the substring match one review round earlier**, and both produced the
same symptom: *evidence, manufactured from something that was never a claim about the company*.
That is the failure mode this project's whole quoting discipline exists to prevent, arrived at
twice from directions nothing was watching.

**Rule:** a human-readable field is an output, not a source. If code needs to know *why* a field
says what it says, that reason is a separate field — and a fallback value is the place this goes
wrong, because it is the one wording nobody thinks of as data.

> **Ask this:** *what does this field say when the thing it describes did not happen — and is
> anything downstream reading it?*

---

## 45. A guard that compares text, and a formatter that rewrites it

**Found:** by running `cargo fmt --all` while a mutation catalogue held a file open — the fourth
finding about that same window, after 41, 42 and 43.

The catalogue stopped and kept its backup, exactly as entry 43's fix intended. What it left in
the tree was the mutated file **after `rustfmt` had reflowed it**:

```rust
// what mutate.py wrote                    // what was on disk afterwards
const TEMPLATES: [&str; 3] = [             const TEMPLATES: [&str; 3] =
    r#"{} alternatives"#,                      [r#"{} alternatives"#, r#"{} competitors"#,
    r#"{} competitors"#,                        r#"{} rivals"#];
    r#"{} rivals"#,
];
```

**`scripts/no_live_mutations.py` then reported a clean tree.** It matches the recorded `new`
character for character, and this was no longer that string — not just different whitespace, but
a dropped trailing comma too. The one gate written to stop a deliberate defect reaching a commit
could not see the defect sitting in front of it.

**My first fix was wrong in an instructive way.** I collapsed runs of whitespace before
comparing, which reads like the obvious answer and would not have caught this one: `rustfmt` also
removes the trailing comma when it joins lines, so the two strings still differ after every space
is normalised. I only found that out because the self-check I wrote alongside it failed. **A
normalisation is a claim about what the other tool does**, and mine was a guess.

**What actually closes it needs no guess.** A `*.mutate-backup` on disk means exactly two things,
both of them *"this tree is not what it looks like"*: a run died holding a file, or the harness
refused to restore over somebody's edit. Neither depends on what the mutated text looks like
afterwards, so the check is sound rather than usually right, and it is one `glob`.

**Rule:** a guard that recognises code by its exact text is defeated by anything allowed to
rewrite that text — a formatter, a linter's autofix, an IDE on save. Before hardening the
comparison, look for a signal that is not the code at all: a lock file, a marker, a leftover
artefact. If the only available check is textual, say in the docstring what it cannot see.

> **Ask this:** *what else is allowed to edit this file, and would my check still recognise it?*

---

## 46. A `MISSED` at a seam no test can reach, closed by deleting the seam

**Found:** by the mutation harness, on the change that added clarification chips.

The worker handed a refusal to the store in pieces:

```rust
refuse(store, analysis, kind, &why, &choices).await;
```

A mutation replaced `&choices` with `&[]` — the question decided and then dropped on the way to
the database — and **nothing failed**. Five arguments, three of them describing one refusal, and
the third silently discardable.

**The obvious fix was to write a test, and it would have been a bad test.** That arm needs a live
search engine to reach with a non-empty list; `Searx::from_env()` is read inside `run_analysis`
and is not injectable. Anything I wrote to make that mutation fail would have had to reach around
the code under test — which is a test of the mutation, not of the behaviour, and it leaves the
same defect available at the next call site somebody adds.

**So the seam went away instead.** `Decided::Refuse(Refusal { why, kind, choices })`, and
`refuse(store, analysis, &refusal)`. There is no third argument to drop, so there is nothing for
a mutation to remove and nothing for a future caller to forget. The same argument one layer down
turned `fail(id, generation, kind, reason)` into `fail(id, generation, Refused { .. })`.

**The retargeted mutation is the honest one.** It now removes `choices` inside `refuse` itself,
where the value is still in scope and a direct test does reach — and it is caught.

**Rule:** a `MISSED` is a question about the shape of the code, not only about the tests. Before
writing a test to catch it, ask whether the mutation describes a mistake a caller could actually
make. If it does and the call site is unreachable from a test, **make the mistake unrepresentable
rather than detectable** — several arguments that always travel together are one value.

> **Ask this:** *is this `MISSED` telling me a test is missing, or that the signature lets a
> caller drop something?*

---

## 47. A value that satisfied the other side's rule for the fixtures I happened to pick

**Found:** by review, on the clarification chips.

`Choice::prompt` was the company's bare canonical domain, and the endpoint it is posted to
rejects anything shorter than `MIN_PROMPT`:

```text
POST /api/analyses  {"prompt":"box.com"}
400  a prompt must contain at least 8 characters, got 7
```

`notion.so` is nine characters. `notionenergy.com` is sixteen. **Every fixture I wrote happened
to clear a bar I had not noticed existed**, so nine tests and a mutation catalogue all passed
over a button that rendered for `box.com`, `wix.com` and `hey.com` and answered the click with an
error — about a company we had resolved ourselves and put in front of the reader.

**The producer and the validator were on two sides of one wire and neither knew the other's
rule.** Nothing was wrong with either half. What was missing was any test that ran the real
output of the first through the real input of the second.

**The fix is a construction, not a check.** `format!("https://{domain}")` — `https://` is eight
characters by itself, so the prompt clears the minimum whatever the domain is. The alternative,
relaxing the minimum for things that look like domains, puts the rule in two places, and the
second copy is the one that goes stale (see 44, and 39's cap).

**Two regressions, and both call the producer rather than quote it.** One asserts every chip's
prompt parses *and* resolves back to its own company; one posts `choices_from`'s real output
through the real route and expects `201`. A string retyped into a test to look like the
producer's output is exactly what stops noticing when the producer changes.

**Rule:** when one component builds a value another component validates, a fixture that passes
proves the fixture, not the boundary. Run the real producer's output through the real validator,
and choose inputs at the *edge* of what the producer can emit — the shortest name, the empty
list, the one with a hyphen. Better still, build the value so the constraint cannot be violated
rather than so it usually is not.

> **Ask this:** *what is the smallest, longest or strangest thing this can produce, and does the
> thing that receives it still accept that one?*

---

## 48. A negative asserted as one spelling of the defect, not as the rule

**Found:** by the mutation harness, on vocabulary resolution — and the fix was to the test.

Phrases are cut at grammar words, so *competitive intelligence software **for** product
marketing* yields two phrases and none that spans the `for`. The test said:

```rust
assert!(!found.iter().any(|p| p.contains("software product")));
```

A mutation deleting the break reported **MISSED**. It had worked exactly as intended: without
the break the run produces `software for product` — the defect, spelled the other way round —
and the assertion looked for the one spelling it does not have.

**The rule is not about those two words.** It is *no phrase may contain a grammar word at all*,
and written that way it needs no guess about which words end up either side of one:

```rust
assert!(!found.iter().any(|p| p.split(' ').any(|w| NOT_CONTENT.contains(&w))));
```

**This is entry 35's shape on a negative.** That one was a threshold asserted as a number
instead of as the decision it drives; this is an invariant asserted as a sample of what its
violation might look like. Both pass while the thing they exist to protect is broken, and both
read like real tests until something tries to break the code underneath them.

**Rule:** when asserting that something must *not* happen, state the property that must hold —
quantified over everything the function can produce — rather than one string the failure might
contain. A negative built from a guess is only as good as the guess, and there is no feedback
when the guess is wrong: it passes either way.

### The sequel, in the next PR, in a test written *after* reading this entry

Numbers separate a listicle's headline from its category, so `Top 10 CRM Software` yields
`crm software` and not `top crm`. The assertion:

```rust
assert!(!found.iter().any(|p| p.contains("top")));
```

**MISSED again.** Deleting the break leaves `top`, `10`, `crm`, `software` in one run — and the
furniture trim removes `top` from every phrase anyway, so the assertion passes while
`10 crm software` sits in the output. A bare number had been glued to a category, which is the
whole defect.

The rule is *no phrase may contain a word that is all digits*, and it does not care what the
neighbouring words are.

**Writing this entry did not stop me making the same mistake eight hours later**, which is worth
recording as plainly as the mistake: a negative assertion aimed at a *word* is the reflex, and
the reflex is wrong. The only reliable tell is that the assertion names something specific from
the example rather than something universal about the output.

> **Ask this:** *if this code were broken in a way I have not thought of, would this assertion
> still fail?*

---

## 49. A check written to find a defect, blind to the shape that defect takes

**Found:** by review, on the gate added in the same PR as the defect it was written for.

`scripts/no_lost_continuations.py` looks for a run of spaces inside a string literal — the
wreckage of a lost `\` continuation. It found eight and closed them. It extracted literals with
a per-line regex:

```python
for lit in re.findall(QUOTED, line):   # one line at a time
```

**A `\` continuation is, by definition, a literal that spans several lines.** The three model
prompts in `landscape-analyze::stages` — the longest multi-line literals in the repository, and
the place where damage matters most because a mangled prompt changes what a model is asked —
were invisible to the check written to find exactly this. I had even *documented* them as
deferred, on the strength of an old note rather than the gate's own output; the gate had never
seen them, and my exemption list was a guess dressed as a finding.

**The exemption made it worse.** It named a *file*, so a new damaged string in that file would
increment a counter and pass. The place the check most needed to look was the one place it had
been told not to.

**Both halves are fixed the same way: read what the defect actually looks like.** Literals are
now parsed whole with a small state machine (skipping comments, char literals and lifetimes),
and exemptions are keyed on the **digest of the exact literal** — so a new damaged string fails,
editing a deferred one fails, and fixing one fails until its entry is removed. Every one of
those was checked by making it happen.

**Rule:** a check for a defect has to be tested against the defect, on real inputs, before it is
believed. "It found eight" is evidence it finds *some*; it is not evidence of coverage. And an
exemption must name the thing exempted, never the place it lives — a path-shaped exemption grows
silently, and it grows fastest exactly where somebody once had a good reason to add it.

> **Ask this:** *what does this defect look like in the worst file I have, and would my check see
> it there?*

---

## 50. A fix for an arbitrary choice, with the arbitrary choice inside it

**Found:** by review, in the fix for the defect review had found one round earlier.

A tie between two categories was being resolved alphabetically. The fix grouped phrases into
markets by **containment** — *email marketing*, *marketing software* and *email marketing
software* are one thing said three ways — and refused to choose when two markets tied.

Grouping was connected components over every containment edge. Two rounds later:

```text
inventory management software   2 hosts   ─┐
project management software     2 hosts   ─┤
management software             4 hosts   ─┴─ contained in BOTH
```

**`management software` is on all four hosts precisely *because* it is what two different
markets have in common.** One containment edge to each, transitivity does the rest, and the two
markets became one cluster — with the arbitrary choice back, now backed by a bigger number than
either real category had.

**The relation was right and the closure over it was wrong.** "A contains B" says B is a less
precise way of saying A. It does not survive being chained: A contains B and C contains B says
nothing whatever about A and C, and treating it as if it did is what merged them.

**A market is a phrase nothing else extends.** A shorter phrase belongs to the one market that
extends it; a phrase extended by more than one belongs to neither, because it is their overlap
rather than evidence for either. No transitivity, and the fragment case that seemed to need it
falls out for free.

**Both rounds of this defect were the same shape** — a choice made where the evidence does not
support one — and the second was introduced *by the fix for the first*, in the same file, the
same afternoon. A fix for "we chose arbitrarily" is exactly the code most likely to contain a
new arbitrary choice, because it is where the tie-breaking lives.

**Rule:** when you group things by a relation, ask what the *transitive closure* of that
relation means, in words, about the things at either end of a chain. If the sentence is not one
you would defend — *"these two markets are the same because they share a word"* — the closure is
not the grouping you want. And test a fix for an arbitrary choice with an input where the
arbitrary choice would be **profitable**: two candidates that share something big.

> **Ask this:** *what does A-to-B-to-C mean here, and would I say it out loud?*

---

## 51. Register 47's own rule, re-derived instead of called — twice

**Found:** by review, in the guard written to satisfy register 47.

Entry 47 says: a value one component builds and another validates must be **built so the
constraint cannot be violated**, and the way to do that is to use the real validator. The guard
written for it did this:

```rust
if words.iter().map(String::len).sum::<usize>() + words.len() - 1 < MIN_PROMPT
```

**`String::len` is bytes; `NewAnalysis::parse` counts characters.** `Ää Ää` is five characters
and nine bytes, so the guard passed a label the API rejects — the exact rendered-chip-then-`400`
failure entry 47 exists to prevent, now for every non-ASCII market name. And it silently ignored
`MAX_PROMPT`, because a rule re-derived from one constant has no reason to know about the other.

**The same PR did it again, in a different shape.** Whether the market's words differed from the
reader's was decided by `words == description` — comparing two raw strings, when the thing that
decides is `for_idea`, which normalises through `safe_words` first and produces queries an engine
reads case-insensitively. A trailing `!` therefore counted as a change and bought three
redundant requests, while an exact match reused the hits *and still* told the reader their own
words had been "interpreted as" themselves.

**Both are the same mistake:** a rule that already exists, written out again in the new code's
own terms. The re-derivation is always *nearly* right, which is why it survives review and the
tests written beside it — it fails on the inputs the author did not have in mind, and those are
by definition the ones nobody wrote a fixture for.

The fixes call the real thing. `trimmed` runs `NewAnalysis::parse`; `substitutes` compares the
output of `for_idea`. Neither can drift, because neither holds a copy.

**Rule:** when a constraint already has an implementation, *call it*. Not "match it", not
"mirror it" — call it. If it cannot be called from where you are, that is a fact worth fixing or
naming, and it is a much smaller problem than a second copy that agrees today. And note the
tell: the re-derived version is usually **shorter and faster**, which is exactly why it looks
like an improvement.

> **Ask this:** *am I writing this rule down, or asking the thing that owns it?*

---

## 52. A bound in the wrong unit, and the rule put where nothing can reach it

**Found:** the first half by review; the second half by the mutation harness, in the fix.

### The bound

This file's own argument, one entry earlier in the same PR: *a cap of "256 pages" bounds nothing
when a page may be 2 MiB*. So the cache was bounded in **bytes**. Review pointed the identical
argument the other way and it landed on the first try:

```text
100,000 entries held.  bytes reported: 0.
```

The byte counter summed `Page::body` and nothing else. A hundred thousand empty responses cost
zero, so eviction never ran, and the map holding the keys grew without any number describing it.

**A budget in one unit does not bound the other, in either direction.** The fix counts what is
actually retained — key, duplicated URL, headers, and a flat per-entry figure — *and* caps the
entry count, because each bound alone has a hole shaped exactly like the other.

### The rule nothing could reach

Review's second finding was that `Cache-Control` was ignored entirely, and the fix read the
header in `Fetcher::get` and branched there:

```rust
if let Storable::For(fresh_for) = keep { cache.insert_for(url, page, fresh_for); }
```

Correct, reviewed, and **untestable**: a test server binds loopback, and the address guard
refuses loopback absolutely, on purpose, with no flag. Nothing in the repository can drive
`Fetcher::get` to that line. The harness said so in the only way it can — *the origin's headers
are read and then ignored*: **MISSED**.

The fix was not another test. It was moving the branch into `Cache::insert_allowed`, which takes
the two header values and returns whether it stored — one call away from an assertion. What is
left in the fetcher is reading two strings off a response, the same untestable line as `etag`,
with no decision in it.

### And the assertion that agreed with the defect

The test written beside the byte fix asserted `cache.bytes() > 0`. A bodyless entry still holds
its key, so that passes while counting nothing but strings — *the defect itself*, spelt
differently. It asserts `>= OVERHEAD` now: the entry costs at least what an entry costs.

**Rule:** put a rule where something can call it, and prefer moving the rule to writing a test
that cannot exist. An untestable branch is not "covered by review"; it is a line whose deletion
nothing notices. And when a bound is the point, assert against the constant that expresses it,
not against zero — `> 0` is satisfied by the part that was never in question.

> **Ask this:** *can a test call this rule without a network, a clock or a socket? And does my
> assertion fail if the bound is removed, or only if everything is?*

---

## 53. A bound that only removes what was already dead, and a clock started at the wrong end

**Found:** by review, in the fix for entry 52 — the round immediately after.

### Removing the expired is not a bound

Entry 52's fix made the process-long `robots::Cache` prune on insert:

```rust
self.by_host.retain(|_, (_, at)| at.elapsed() < CACHE_TTL);
```

and the regression beside it inserted 500 hosts, **aged all of them past the TTL**, inserted one
more, and asserted one remained. It passes. It is also blind to the only case that matters:
`CACHE_TTL` is six hours, so nothing expires in an afternoon, and a worker crossing many hosts
holds every one of them. Asked directly, it held **50,000** live rule sets.

**The regression was written from the fix's point of view, not the defect's.** The fix was "drop
what has expired", so the test aged everything to make things expire — which is exactly the state
in which the bug cannot appear. A test built out of the mechanism can only confirm the mechanism.

The bound is now the same shape as the page cache's: expiry, *plus* a live-entry cap, *plus* a
byte cap, with oldest-first eviction. The new regressions insert past each cap **without ageing
anything**.

### A lifetime is not a deadline

The same round: `max-age=3600` was turned into "keep for 3600 seconds **from now**". But
`max-age` is measured from the origin's `Date`, and a CDN answering with `Age: 3590` is saying
ten seconds remain. Restarting the clock on arrival lets a chain of caches hold one response
fresh for ever, one hop at a time — each honestly obeying the number it was handed.

```text
origin  ──3600s──▶  CDN (holds 3590s)  ──"3600s"──▶  us (holds 3600s more)
```

`Expires` did not have the bug, and the reason is worth keeping: it is an **instant**, so
`expires - now` already nets out the age. `max-age` is a **duration**, and a duration is
meaningless without the end it is measured from. The fix subtracts the age from durations only —
subtracting it from `Expires` too would have double-counted, which is the same mistake mirrored.

**Rule:** when a fix bounds or expires something, write the regression in the state the *defect*
needs, not the state the *fix* creates — if the test has to arrange the fix's precondition to
observe anything, it is testing the fix. And when a value crosses a boundary as a duration, ask
what clock it started on; if that clock is not yours, the elapsed part is already gone.

> **Ask this:** *does my regression still fail if I delete the setup line that makes the fix
> apply? And is this number a duration or a deadline?*

---

## 54. An argument written where a test belonged, and the first value taken for all of them

**Found:** by review, in the fix for entry 53 — the third consecutive round on one cache.

### Prose is not a proof, and a correct principle can be used to skip a check

The page cache declines a single entry too large for its whole budget, or the eviction loop
empties the map making room and then inserts the thing that did not fit. The robots cache was
given the same budget and **no such guard**, with a comment explaining why one was unnecessary:

> a `robots.txt` is read through `MAX_BYTES`, so a single entry cannot reach 2 MiB of retained
> directives and cannot exceed this budget on its own. A guard for a case that cannot arise is a
> branch no test can reach.

Every clause of that is confident and the conclusion is false. Review answered it in one line:

```text
one file left 5767378 bytes held against a budget of 4194304
```

An admissible 2 MiB file of `Disallow: /` lines is 174,000 directives, and the byte counter —
**mine, three paragraphs above the comment** — charges each one the 32 bytes of the tuple that
holds it as well as its single character. The argument reasoned about the size on the wire; the
budget counts the size in memory. *The parsed form is bigger than the bytes it was parsed from.*

The worst part is the second sentence. Entry 52's rule — do not write branches no test can reach
— is right, and it was used here to justify *not writing a check*, on the strength of an argument
about reachability that was never run. **A principle about testability became a reason to skip a
test.** Deleting a guard needs the same evidence as adding one, and the evidence is a test, not
a paragraph.

### `get` returns the first, and a header can arrive twice

`Cache-Control` is a list-based field, and HTTP lets it arrive as several field lines whose
values combine as though written on one line with commas. `HeaderMap::get` returns the **first**:

```text
Cache-Control: public
Cache-Control: no-store      ← never read
```

so a response arrived as a bare `public` and was cached against an explicit instruction. Nothing
downstream could recover it: splitting on commas does not help when the value that must be found
was dropped before it was passed along. The fix reads `get_all` and joins.

Note the shape — this is the same family as the round before it. A part standing in for the
whole: the first field line for the whole field, the wire size for the whole cost.

**Rule:** when you remove or omit a bound, guard or check, write the test that proves it
redundant — an argument in a comment carries no weight against a defect and is worse than
silence, because it discourages the next reader from checking. And when an API offers `get` and
`get_all`, find out which of the two the *protocol* means before picking the shorter name.

> **Ask this:** *is my reason for leaving this out a measurement or a paragraph? And can this
> thing legitimately occur more than once?*

---

## 55. A judgement this codebase already makes, not consulted by the new code

**Found:** by review, in the same cache, fourth round.

`storable` decided whether a response could be kept from its headers, and only its headers. So a
`500` or a `429` with no `Cache-Control` was kept for the full hour and replayed to every later
reader — **without the origin ever being asked whether it had recovered.**

The value that decides this was already in the room, twice over:

- `Page::status` was on the very struct being stored, unread.
- One module away, `robots::Rules::from_status` already encodes this exact judgement, in this
  repository's own words: a `429` or a `5xx` means *"the site is unwell — assume disallowed. The
  polite reading of 'I am struggling' is not 'carry on'."*

The cache took that same afternoon and wrote it down. **A cached failure is worse than a slow
one**: the report has a gap whose cause is no longer live, and nothing will go and look again for
an hour. The politeness argument the module leads with points the same way — a cached `503` does
not spare an origin anything, because there was never going to be a second request for a page we
already failed to read.

The fix is HTTP's own rule: keep on our own initiative only for the statuses RFC 9110 §15.1
defines as cacheable, and for anything else require the origin to state a freshness. `storable`
takes the status, and `insert_allowed` reads it off the `Page` rather than from a parameter, so
the two cannot disagree.

**Rule:** before writing a policy, ask whether this codebase already has an opinion about the
same value — and whether the type you are handed is already carrying the input you need. A new
component that reaches a different verdict from an existing one about the same fact is a
contradiction, not a feature, and the older one is usually the one that was thought about.

> **Ask this:** *does anything else here already decide something about this value? And am I
> ignoring a field of the thing I was handed?*

---

## 56. A cache keyed on the question but not on who answers it — and put behind the failure it exists to survive

**Found:** by review, in the extraction cache, first round.

A memo is a claim that **two computations are the same**. That claim has two halves — *same
inputs* and *same function* — and the first version wrote down only the first.

### The key described the question and not the answerer

`Read` carried every input: the prompt version, the question, the URL, the day, and the page's
whole text. It said nothing about **what would answer it**. Two things were missing:

- **the decoding settings and schemas** — a prompt can be word-for-word identical and mean
  something different at another temperature or under another constraint;
- **the model itself.** The worker outlives `llama-server`, so that server can restart with a
  different model at the same `LLAMA_URL`. The next analysis would reuse the previous model's
  answers, and the report would label them with the current client's address. *One model's words
  attributed to another, cited, and internally consistent* — the exact failure the whole pipeline
  is built to prevent, arriving through the thing added to make it faster.

The static half became `EXTRACTION_VERSION` in the key. The model became a **scope** rather than
a key field, and that distinction is the interesting part: putting the model's name in the key
means asking the model who it is *before* answering from memory — a request on the hit path,
which the other half of this entry says must not exist. Scoping the whole memory to one identity
and emptying it when that changes costs one metadata call per analysis and nothing per hit.

### The fast path was placed behind the slow path's precondition

The lookup sat **after** the model-readiness gate. So a page whose answer was already held came
back as `(no model)` while `llama-server` was down — the cache failing in precisely the hour it
exists for, and the reader depending on the health of a thing it never needed to touch.

**A fallback checked after the thing it is a fallback for is not a fallback.** The gate moved
inside the miss closure: a hit now makes no request of any kind, and a miss that cannot ask
returns "not asked" rather than an answer, so nothing is remembered.

**Rule:** a cache key must identify **the function as well as the argument** — version the
behaviour, not only the input, and if part of the behaviour is a live external thing, scope the
cache to its identity rather than keying on it. And a memory that exists to survive an outage
must be consulted **before** anything that the outage can fail.

> **Ask this:** *if the thing that produced this answer were swapped out, would my key notice?
> And can this hit be served when everything else is down?*

---

## 57. Ordering as a substitute for a type, and an accident mistaken for a bound

**Found:** by writing the change, with the harness deciding where four of the rules had to live.

Two habits, both of which look fine in a diff.

### `304` is a `3xx`

The obvious shape for a conditional GET is to add one branch to the loop that already reads a
status:

```rust
if (300..400).contains(&status) { /* follow the Location */ }
```

`304 Not Modified` is in that range and has **no `Location`**, so the redirect branch swallows it
and the cheapest answer an origin can give becomes `redirect with no location`. A conditional GET
that turns every revalidation into a transport error is strictly worse than never sending one.

The fix is not *"put the `304` check first"*. **Ordering is a property of how the code happens to
be laid out**, and the next person to add a branch has no way to know that this one is load
bearing. A `match` on a named type with **no wildcard arm** makes it a property of the status:

```rust
enum Answer { NotModified, Redirect, Body }
```

Now a fourth kind of answer is a build error, and the rule can be tested without a socket — which
is the same reason four other rules in this change moved out of the function that opens one.

### An accident of arithmetic is not a bound

Nothing capped the number of requests one analysis sent to strangers. It *looked* bounded —
eight pages a company, three companies — but that number is the product of two unrelated
decisions, and neither was chosen to bound anything. A `sitemap.xml` naming ten thousand URLs, a
redirect chain per page, a `robots.txt` per host reached through search: each turns one reader's
question into an unbounded number of requests to somebody else's servers.

Two things follow, and both are easy to get wrong:

- **Count what leaves the process.** A bound on "pages the caller asked for" bounds the thing
  nobody was worried about. `robots.txt` is a request; every redirect hop is a request; a cache
  hit is not, because nothing was sent.
- **Running out is your doing, and must never be reported as theirs.** The first version let a
  spent allowance fail the `robots.txt` fetch, which every other failure turns into
  `Rules::restrictive` — so the report would have said *"the site asks crawlers not to"* about a
  site that had said nothing. A reader acts on that sentence. Our own limit needed its own error,
  its own discovery outcome, and its own words on the page.

**Rule:** when a check's correctness depends on running before another check, give it a type
instead of a position. And when you believe something is already bounded, name the number and the
decision that set it — if you cannot, it is an accident, and accidents do not hold.

> **Ask this:** *would a new branch here silently break an old one? And is the limit I am relying
> on something somebody chose, or something that fell out?*

---

## 58. Silence read as absence, and one budget across things that were never one thing

**Found:** by review, in the change that introduced both.

### A `304` says *"unchanged"*, and I read it as *"nothing"*

RFC 9111 lets a `304` omit any header whose value has not changed, and most origins omit
`Cache-Control`. The revalidation branch re-inserted the confirmed page using **only the headers
on the `304`** — so `storable` saw no policy, fell back to our hour, and an origin's
`max-age=30` became sixty minutes the first time it was revalidated.

**Asking a publisher whether their page changed widened the policy they set on it.** That is the
opposite of what the request was for, and it is silent: the page is correct, the citation is
correct, only the interval is wrong, and nothing on any surface says so.

The general shape: **an absent field in an update is not the same as an absent field in a
creation.** A partial update carries deltas; reading it as a whole record replaces everything it
did not mention with a default. The fix is to carry the stored value and let the update override
it — which meant `Stale` had to carry the freshness it was stored under, not only its body and
validators.

It also exposed a second half of the same error. `Storable::No` meant *do not store this*, and
the code did exactly that — while leaving the **older copy** in place to be served. Refusing to
store and refusing to serve are different, and a cache that obeys `no-store` by only doing the
first still ignores it.

### One allowance for things that were never one question

A per-analysis fetch budget is right for an analysis. It was created once at the top of a
health-check command that loops over six independent companies, so the first two spent it and
every later one came back *"no pricing page"* — **a check failing on an artifact of the check
before it**, which is worse than no check at all.

The bound was correct; its *scope* was copied from the place it was designed for to a place that
merely looked similar. When a resource is scoped to a unit of work, every construction site has
to answer *"what is the unit of work here?"* — and a loop over independent targets is not one
unit however much it resembles one.

**The first fix was not enough, and the second round is the more useful half.** I carried the
stored *duration* forward and used it whenever the `304` restated nothing. Review took that apart
too: RFC 9111 §3.2 says the fields a response *supplies* replace their stored counterparts and
the ones it *omits* remain, so the merge is **per field**, and a computed number cannot express
it. A page stored with `max-age=30` **and** an `Expires` an hour out is fresh for thirty seconds;
a `304` updating only `Expires` is still thirty seconds; "it said something, so read it alone"
gives two hours.

**Merging is a field-level operation, and a derived value has already thrown the fields away.**
Keep the fields, overlay what arrived, recompute. And the same round found the same shape in a
neighbour: the validators. A `304` may supply a new `ETag`, and keeping the one we asked with
would make every later revalidation ask about a version nobody has.

**And keeping fields means paying for them.** The policy went into the entry and not into the
cache's `cost`, which is the hole that same function had been written to close two rounds
earlier: `Cache-Control` and `Expires` are origin-controlled heap allocations, so a byte budget
counting the body and the key but not them reports a cache inside its limit while it is not.
**Anything added to a bounded structure is added to its bound in the same commit** — the two are
one change, and a review that has already made this point about one field will make it about the
next.

**Rule:** when consuming an update, ask what a missing field means — *unchanged* or *unset* —
and carry the stored value when it means unchanged. Carry the **fields**, not what you computed
from them: a merge you cannot perform field by field is a merge you are not performing. When
refusing to keep something, ask whether you are also refusing to serve what you already kept. And
when a limit is scoped to a unit of work, name the unit at every place you construct it.

> **Ask this:** *does silence here mean "same as before" or "none"? Can I merge this field by
> field, or have I already reduced it to a number? And is this loop one piece of work or
> several?*

---

## 59. A feature that is built, admitted, and then never reached

**Found:** by running the thing against three real sites before writing it up.

Discovery learned to find the applicant-tracking board a company's careers page links to. Every
part worked: the link was extracted, the URL reduced to the board's root, the candidate admitted
past the off-site filter with its own standing. Then `landscape discover` was run against the
three real companies it had been built for, and **not one of them showed a board**.

The cap admits one page per question first and spends what is left in the same order. Hiring's
first slot goes to the company's own careers page — which is *how the board was found* — so a
board can only ever compete for a second slot, and the questions that sort earlier take those.
The feature was complete and inert.

Nothing about that is visible in a diff, in a unit test, or in a mutation catalogue: every piece
was individually correct and the composition was not. **The only thing that finds it is running
the feature the way a user gets it**, which for this project is one command against a real site.

The tempting fix — rank a board above the page that named it — is also wrong, and the same three
sites say so: `linear.app/careers` lists its roles and its board reads as nothing, while
`vercel.com/careers` is navigation chrome and its roles are entirely on Greenhouse. **Which is
which cannot be known before the page is read**, so both are admitted and the reading decides.

**It happened again inside the fix.** Review pointed out that scanning raw HTML for a host is not
the same as finding a link, which was right. The fix paired quotes across the document to find
values — and paired quotes go out of phase the moment a page nests JSON inside JSON, which the
very page it was built for does. Every unit test passed and **all three real sites went back to
finding nothing**. Anchoring on the quote immediately before a URL needs no pairing at all.

Twice in one change, the same shape: **a composition that only the real input exercises.** A
parser tested on fragments is tested on fragments.

**Rule:** before writing up a feature, run it end to end and look at the output with your own
eyes — and run it again after every fix, including the ones review asked for. A test proves a unit does what you meant; only the real thing proves the units add up to a
feature a reader sees. And when a ranking has to choose between two sources, check whether the
information needed to choose exists yet — if it does not, admit both rather than guessing.

> **Ask this:** *have I actually seen this work, or only seen its parts pass? And am I ranking on
> something I will not know until later?*

## 60. A second answer to a question already answered — and a gate that checked everything except the row people read

**Found:** by the mutation harness, twice, and by adding a table up by hand once.

A component showed the set of companies a report was built from, and let a reader change it. The
set can be replaced underneath — a corrected run finishes and the report is about somebody else —
so the first version kept the two in step with a `useEffect` copying the prop into state. The
harness emptied that effect and **nothing failed**. The reflex was to write the missing test; the
second answer was to replace the effect with a `key`, which is the idiomatic version of the same
idea. The harness broke that too, and again nothing failed.

Both were dead. The set is only offered **once a run is over**, and correcting it **starts a new
run** — so between the two reports the component is unmounted and the second set is read fresh.
One guard already enforced the rule, and the sync was a second enforcement of it that no input
could reach.

**A `MISSED` on a rule that is genuinely enforced somewhere else is not a missing test.** It is a
second answer to a settled question, and two answers can one day disagree — the effect and the
guard would have had to keep agreeing about a case neither could produce. The catalogue entry was
removed as a duplicate of the guard's, and the code with it. This is entry 46's shape arriving
from the other direction: there, a seam nobody used; here, a rule stated twice.

**And the same day, in the same change, the gate written one run earlier to stop numbers drifting
was found not to check the numbers anybody reads.** `scripts/feature_totals.py` added up each
state's feature rows against that state's totals row — correctly — and never looked at the
**Summary table at the top of the page**, which is derived from those totals, is the first thing
a reader sees, and is what every pull request quotes. It had been saying S2 was 78% for four
changes while S2's own table said 89%.

A gate that checks the derived numbers but not the number derived from *those* leaves the
most-read row unchecked. The blind spot is not an oversight about regular expressions; it is that
**a summary is data too**, and the reason it exists — that nobody wants to add the tables up — is
exactly the reason nobody notices when it is wrong.

**And review found the same shape a third time, on both sides at once.** The page refused to
re-run *"the set already on screen"* and did not refuse *adding a company already in the set* — a
longer list, so the change check said yes, and the run put the duplicate back together and
returned the same report. The guard was right and did not cover the way in. Meanwhile the gate
checked each Summary row against its own table, and the total against the sum of the rows, so a
state listed **twice** with the total inflated to match passed everything: every number right,
the page false. **A per-row check cannot see a duplicated row**, and neither guard was reachable
from the thing it was written to protect.

The gate now breaks itself seven ways before it reads the real page, because the version that had
this hole would still have exited 0 for ever.

**And the first fix for the page was refused, correctly.** It compared the stored strings, and the
defence written beside it was that two *spellings* of one company belong to the resolver and this
side does not get an opinion. That is a good rule and it was quoted against the wrong case: the
interface **asks for** the schemeless spelling — the chip renders it, the input box suggests it —
so the normal way in was the way that slipped past. Comparing what the page *renders* is not a
copy of the resolver's rule; it is the claim the page already published when it drew the chip, and
holding it in one function turned up a second hole nothing had tested: two edits that cancel out
left the button enabled. **A principle about not duplicating a rule is not a licence to compare
in a representation the reader never sees.**

**Rule:** when a mutation survives, ask *what else already enforces this* before asking *what test
is missing* — and delete rather than pin, when the answer is "the thing above it". And when a gate
checks a document's numbers, check every number in the document that is derived from another,
starting with the one printed largest. When a rule is stated as *"do not spend this twice"*, list
the ways in rather than the one that prompted it — and when a check compares things one at a
time, ask what a **duplicate** would do to it.

> **Ask this:** *is this the only thing making the rule true, or the second? Which number on this
> page would a reader quote — is that the one I am checking? What does my one-at-a-time check do
> when the same thing appears twice? And am I comparing in the form the interface asked for, or in
> the form I happen to store?*

---

---

## 61. Advice that was true of one case, attached to all of them

**Found:** by running the thing against a server that answers `403`, after every unit test passed.

Every path that reported a failed search ended with the same sentence: *"that is usually
temporary - try again."* It is true when an engine times out. The most likely first experience of
a **configured** engine is not a timeout: `deploy/searxng/settings.yml` is checked in precisely
because SearXNG serves HTML and answers `403` to `format=json` until an instance opts in, so a
first run without that file refuses every query, for ever, and the report recommended waiting.

The reason was known at the moment of failure and thrown away one line later — `Queried.failed`
held the query text and not the error. **A value parted from its evidence**, which is register
entry 7, arriving for the third time in a different crate.

**The mutation harness found the gap that made it invisible.** Replacing the read of the error's
kind with a constant broke nothing, because every test of the resulting sentence built its input
by hand. The wording was pinned at both ends and the wire between them was not: a pair kept
together is only kept together if something reads both halves of it.

**And the surface that mattered most had no test at all.** With the notes and the report
sentences fixed and the whole suite green, pointing the application at a refusing server showed
the *whole-run refusal* — the first thing a reader meets — still saying *"this is usually
temporary."* It renders from a `Failure` kind rather than from a sentence, so nothing in the
sentence-level work could reach it. That is entry 59's rule paying off again: **run it, then
believe it.**

**The split that fixed it is not a taxonomy of one provider.** *Did the engine answer at all?*
Anything that came back is a decision and the same decision comes back next time; only silence,
and the two answers that explicitly mean *later*, are worth waiting on. A rule shaped like HTTP
rather than like SearXNG is one a second provider inherits without an edit.

**Rule:** when one sentence ends a family of error paths, check it against the *most likely*
member of that family rather than the one you were thinking of when you wrote it — and when a
reason is available at the point of failure, carry it, because the sentence that needs it is
always further away than it looks.

> **Ask this:** *is this advice true of every case that reaches it? And which surface renders
> from a kind rather than from the string I just fixed?*

---

## Before a PR: two commands and eight questions

**The commands come first, because they are the part that does not depend on remembering.**

```bash
python3 scripts/verify.py
```

Every gate, each judged by its **own** exit code, with the file-reading checks run against a
clean checkout of `HEAD` rather than your working tree. Entries 16 and 17 are both here: a link
that resolved only because of an untracked file, and a `| tail && echo OK` that printed success
over a broken build.

```bash
python3 scripts/mutate.py mutations.json
```

**Put back every defect your change is supposed to prevent, and confirm something fails.** Write
one mutation per guard you added. This is the only mechanical check that has ever found a defect
here — and a `MISSED` exits non-zero, because a test that cannot fail is a finding rather than a
line in a table.

### Then eight questions

Grouped by what has actually gone wrong, commonest first.

1. **What happens between two of my `await`s?** If a decision reads and then acts, what does a
   second request arriving in the gap see? Was this correct only while it was synchronous? And
   **what did it capture before it waited** — a clock, a version, a count — that may not be true
   any more? And if I have written *"at most one"* anywhere, what stops the second? *(27)*

2. **Am I checking two sources, or do I have one?** If a value is compared against where it
   came from, ask whether the second copy can be deleted instead — and if it cannot be deleted,
   as a record of history cannot, is it read back from the tool that produced it? *(4, 29, 31)*

3. **What states exist between "started" and "finished"?** Which of them has a test — a worker
   replaced mid-run, a stream that drops, a request still in flight when the next one starts, a
   step whose dependency *errored* rather than succeeded? Ten of the entries above live here.
   *(1, 3, 4, 12, 13, 14, 15, 18, 19, 19b)*

4. **Which of my checks cannot fail?** **Open the producer and copy its shape** — do not write
   what the value obviously is. **Assert on the value the surface reads**, not the structure
   behind it, and on the whole line rather than the halves. Is my malformed fixture broken in
   the shape the guard *already* handles, or in the shape it does not? *(26)* Does the fixture already contain the
   thing I am asserting? If I deleted the call to the function I just tested, would anything
   fail? Is there a case that *should* fail and does? **And when a mutation survives, is the
   guard untested or unnecessary?** **And if this value exists to cross a threshold, am I
   asserting the threshold's outcome or a number of my own?** *(5, 20, 21, 24, 25, 32, 35)*

5. **What is the scope of each guard, and of each wrapper?** A condition about *the connection*
   is wrong at every reconnect; a counter inside an effect does not survive a remount; a layer
   wraps what existed when it was added. Does the guard outlive the thing it guards? *(14, 19b,
   22)*

6. **What travels together, and what is joined by position?** Any parallel collection, index
   pairing, or value separated from its evidence — and does anything still line up when one of
   them changes length? *(7)*

7. **Have I listed the surfaces?** One fact, rendered by the CLI, the merged report, the live
   stream and the interface. Which of them still has the old shape — and does the one a reader
   looks at *longest*, the stream, have this before it needs it? *(25)*

8. **Is the output honest about what it does not know?** Are caps, drops, truncation and "found
   nothing" distinguishable from completeness? Does a merged view say which subject each part
   belongs to — **including when one subject produced nothing**? Am I counting the thing or the
   evidence for it? Does any prompt name a real company? **And is any caveat I have written
   living only in a log, where the reader of the claim will never meet it?** *(2, 6, 8, 9, 10,
   11, 23, 33, 34)*

## How these were found, and what that says

| Found by | Entries | |
|---|---|---|
| Review | 1, 2, 3, 4, 13, 14, 18, 19, 19b, 20, 22, 23, 24, 25, 26, 27, 29, 30, 31, 33, 34, 35, 47, 49, 50, 51 |
| The mutation harness | 32, 36, 46, 48 | The only ones it found before review did. 36 deleted a rule rather than adding a test; 46 deleted an argument; 48 was a defect in a test |
| Its own tooling | 28 | Almost all in error paths; 13 and 14 were successive halves of one fix, and so were 19 and 19b |
| Using the product in a browser | the two Run 16 defects | Neither visible to 425 passing tests |
| Running the pipeline against real companies | 5, 6, 7, 8, 9, 11 | `BENCHMARKS.md` Runs 5–16 |
| Deliberately breaking the code to see if a test notices | 12, the rearm in 14, and 21 | The store was fast, so nothing raced until one was made slow. Now `scripts/mutate.py` |
| Writing down what a fix does *not* cover | 15 | Named as open in Run 18's "what is still not right", fixed in Run 19 |
| CI, catching what a local run could not see | 16 | The local check read the working tree; CI reads the commit. Now `scripts/verify.py` |
| Doubting a `MISSED` instead of writing it down | 17 | The mutation had been applied to a different copy of the same code |
| The test suite, before review | — | **None of the entries above** |

**That last row is the point of this file.** The suite is good at protecting what it was written
for and blind to what nobody thought of.

### What that says about where to spend effort

Four things have found defects here. Ranked by how much they cost:

| | Cost | Finds |
|---|---|---|
| **Breaking the code on purpose** | minutes, and now one command | Guards nobody tested, fixtures that answer their own question |
| **Reading real output** | an afternoon per run | Everything in `BENCHMARKS.md` Runs 5–16 |
| **Using the product as a client does** | a browser and ten minutes | What 425 passing tests could not see ([ADR 0011](../../../docs/decisions/0011-no-experiments-on-production.md)) |
| **Somebody else reading the diff** | another person | The largest share, and every one in an error path |

**Only the first is mechanical, and it is the one that was being done by hand and thrown away.**
It is `scripts/mutate.py` now. Writing a mutation for each guard a change adds is the closest
thing here to a way of *not* needing the review that follows.

The rest of this file is a memory aid for the other three. It works only if it is read before
the code is written, which is why the two commands are at the top of the checklist and the
questions are underneath them.
