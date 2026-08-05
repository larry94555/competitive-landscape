# 0012 — A claim is a number, not a state

**Status:** accepted · **Date:** 2026-08-05

## Context

An analysis is claimed by a worker, which then spends ninety to a hundred and eighty seconds on
it. A worker can be killed mid-run — a deploy, an OOM, a machine going away — and the row would
be stranded in `running` for ever, so a sweep returns anything running longer than twenty
minutes to the queue.

That sweep cannot tell a **dead** worker from a **slow** one. Nothing can: both leave a row that
has been `running` for twenty minutes, and the only honest reading of that is "probably dead".
So the sweep does the right thing and hands the row on — and now two live workers are running
the same analysis, and every piece of state we had said they were the same worker.

`status` cannot separate them. Both see `running`. Whichever finished last won, and a reader
got whichever report that was, with nothing anywhere recording that two had been produced.

The same gap has a second face on the reader's side. `BENCHMARKS.md` Runs 17 and 18 chased it
twice: a dead worker's sections stayed on screen after its run was taken away, and each fix was
a server-side judgement — *the report went away*, *this connection has already sent something* —
which is a statement about the **connection**. A reader's sections deliberately survive a
reconnect, and a fresh connection remembers nothing, so both fixes were correct until the reader
reconnected and wrong immediately after.

## Decision

**Every analysis row carries a `generation`: how many times the run has been started.**
`claim_next` raises it; `reclaim_stale` raises it.

A worker is handed the generation it claimed and quotes it on **every** write. `save_progress`,
`complete` and `fail` apply only when the number still matches, and return
`Applied::ClaimRevoked` when it does not — an outcome, not an error, because the sweep doing its
job is not a failure.

**The number goes out on the stream**, and the client compares it against the one it is holding.

## Why

**A state describes the row; a number identifies the attempt.** "Running" is true of a row no
matter which of two workers is running it, so no test on `status` can ever separate them. A
generation is the smallest thing that can: it makes "the claim I was given" a value a worker can
carry and a store can check.

**Revocation has to be checkable by the loser.** The replaced worker is still alive, still
holding a full report, and about to write it. Nothing else in the system is in a position to
stop it — the sweep has already run, the replacement does not know it exists. The only place the
conflict can be detected is at the write, which means the write has to carry enough to detect it.

**A value survives a reconnect; an edge does not.** `running → queued → running` is an edge, and
the stream polls twice a second: a sweep and a claim landing between two polls are invisible, so
a design that watched for the transition would miss exactly the restarts that happened quickly.
A generation is a value — a reader that reconnects, or one that simply blinked, still finds a
number different from the one it holds. This is the general form of the lesson in entry 14 of
`.claude/skills/coding-mistakes/SKILL.md`: **state the condition in terms of the durable thing.**

**And it puts the comparison where the knowledge is.** Only the client knows what the client is
showing. The server sends a fact — *these sections belong to run 3* — and the reader's own state
decides what that means. No server-side guess about what a connection has delivered can be right
across a connection boundary, because the boundary is where that knowledge is thrown away.

## What this costs

**A column, and a number on four signatures.** `save_progress`, `complete` and `fail` gained a
parameter and a return value, and every caller has to have a claim to quote. That is the point —
it is now impossible to write to an analysis without saying which run you are.

**Two queries on the failure path.** In Postgres, an update matching no rows is ambiguous
between "no such analysis" and "your claim is gone", so `complete` and `fail` ask. Collapsing
them would report a healthy revocation as a missing analysis, which is the more expensive
mistake by far.

## What it does not do

**It does not stop the replaced worker working.** It finds out at its next write, which for a
progress write is within a page and for `complete` is at the very end. A run that has been
replaced still spends its remaining prefill on a report that will be discarded. Stopping it
needs cancellation threaded into `analyse_with`, which is a change to the pipeline's shape
rather than to the queue's, and is worth doing on its own.

**It does not decide who is right.** The replacement wins because it holds the current number,
not because its report is better. That is the correct default — the sweep exists precisely
because the older claim looked dead — but it is a policy, and if two-workers-finishing ever
becomes common rather than pathological, it is the policy to revisit.

## Alternatives considered

**A lease the worker renews.** Correct, and the standard answer. It also means a heartbeat, a
second failure mode when the heartbeat is late but the work is fine, and tuning an interval
against the model's slowest window. The generation needs no timer and no extra traffic, and the
staleness sweep it builds on already exists.

**A `claimed_by` worker identifier.** Tells you *who*, which is useful in a log and useless as a
guard: a worker restarting under the same name would pass its own check. Counting attempts is
the property we actually need.

**Leaving it, and accepting last-write-wins.** Defensible on the numbers — twenty minutes
against a 90–180 s analysis makes it rare. But *rare and silent* is the worst combination this
product has: two workers producing two reports for one reader, and no line in any log saying so.
