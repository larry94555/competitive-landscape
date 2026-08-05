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

**Before opening a PR**, run [the checklist](#the-checklist-before-a-pr). It is six questions and
takes two minutes.

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

---

## The checklist, before a PR

Twelve questions. Two minutes. Every one of them comes from an entry above.

1. **Lifecycle** — does anything infer "finished" from the presence of data rather than from a
   status? *(1)*
2. **Mid-operation states** — which states exist only when something fails partway, and which of
   them has a test? *(the pattern, 1, 3, 4, 12, 13, 14, 15, 18, 19)*
3. **Duplication of a derived fact** — is any fact computed in two places from different inputs?
   *(4)*
4. **Comparisons** — can I name a change my equality check cannot see? Does any validation have a
   case that should fail and does? *(2, 5)*
5. **Things that belong together** — any parallel arrays, index-paired lists, or a value separated
   from its evidence? *(7)*
6. **Honesty of the output** — are caps, drops and "found nothing" distinguishable from
   completeness? Does any prompt name a real subject? *(8, 10)*
7. **Copies I do not own** — if two writes are in flight, which lands last? Who is holding what
   I just deleted, and what tells them? Does my guard still mean the same thing on a fresh
   connection? *(12, 13, 14)*
8. **Who is asking** — can two actors be in the state this condition tests? Am I authorising a
   situation when I mean to authorise a claimant? *(15)*
9. **What am I actually shipping** — does `git status` hold anything I did not put there? Would
   the file-reading checks pass on a clean checkout of this commit? *(16)*
10. **Did my check run where I aimed it** — if I am reporting that something is *not* covered,
    have I confirmed the thing I broke is the thing I meant to break? *(17)*
11. **The unhappy branch** — does every guard still run when the step above it failed or was
    skipped? Is there a test where the dependency *errors* rather than succeeds? *(18)*
12. **Two at once** — if a second request, run or connection starts before the first finishes,
    which one writes? Does the loser touch anything, including in `finally`? *(12, 15, 19)*

## How these were found, and what that says

| Found by | Entries | |
|---|---|---|
| Review | 1, 2, 3, 4, 13, 14, 18, 19, 19b | All in error paths; 13 and 14 were successive halves of one fix |
| Using the product in a browser | the two Run 16 defects | Neither visible to 425 passing tests |
| Running the pipeline against real companies | 5, 6, 7, 8, 9, 11 | `BENCHMARKS.md` Runs 5–16 |
| Deliberately breaking the code to see if a test notices | 12, and the rearm in 14 | The store was fast, so nothing raced until one was made slow |
| Writing down what a fix does *not* cover | 15 | Named as open in Run 18's "what is still not right", fixed in Run 19 |
| CI, catching what a local run could not see | 16 | The local check read the working tree; CI reads the commit |
| Doubting a `MISSED` instead of writing it down | 17 | The mutation had been applied to a different copy of the same code |
| The test suite, before review | — | **None of the entries above** |

**That last row is the point of this file.** The suite is good at protecting what it was written
for and blind to what nobody thought of. The three things that have actually found defects here
are *reading real output*, *using the product as a client does*
([ADR 0011](../../../docs/decisions/0011-no-experiments-on-production.md)), and *somebody else
reading the diff* — and the cheapest of the three is the checklist above, applied before anyone
else has to.
