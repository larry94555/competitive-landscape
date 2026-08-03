# ADR 0005 — Correlated logs and hosted error tracking, not a metrics stack

**Status:** accepted
**Date:** 2026-08-03

## Context

[ROADMAP.md](../ROADMAP.md) Phase 0 lists as an exit criterion:

> ☐ A written decision on the observability backend: `tracing` instrumentation is fixed, but
> self-hosted Prometheus/Grafana/Tempo competes for RAM with the model on a 24 GB box.

The criterion asks for a *decision*, and the reason it is a Phase 0 item rather than an
operational detail is that the answer changes what gets built. Instrumentation added later
is instrumentation added to the code paths somebody happened to remember.

**The constraint is memory, and it is not close.** The target host is an Oracle Always Free
A1: 4 OCPU, 24 GB, aarch64. [ARCHITECTURE.md](../ARCHITECTURE.md) §4.7 already spends about
17 GB of that on three resident models. What remains has to hold Postgres, the API, the
worker, Caddy, and the page cache that keeps Postgres from going to disk.

A modest self-hosted stack — Prometheus with a few weeks of retention, Grafana, Tempo for
traces — is comfortably 2–4 GB before it stores anything. There is no arrangement of that on
this machine that does not come out of the model budget, and §2.2 of
[RUNBOOK.md](../RUNBOOK.md) already records what happens when memory runs short: the process
is OOM-killed at load, or worse, the machine swaps and the site does not fail but goes a
hundred times slower while every health check passes.

**Trading a model for a dashboard is a bad trade**, and it is worth being explicit about why:
the models *are* the product. A dashboard tells us the product is unwell. Nothing on the
dashboard fixes it.

## Decision

Three things, in the order they earn their keep.

**1. Structured logs, correlated by a request id.** Every request gets an id. It goes into a
`tracing` span, so every line the request emits carries it; onto the `x-request-id` response
header; and — the part usually skipped — into the **body of a 5xx as `reference`**, so the
person looking at the screen can quote it. `crates/landscape-api/src/request_id.rs`.

**2. Hosted error tracking**, once there are users to have errors. Free tiers are ample at
this volume and the RAM cost on our box is zero. Not wired up yet; the `tracing` boundary is
where it attaches when it is.

**3. `/api/health` as the liveness signal**, which already reads `queued` from storage — so a
healthy answer proves the process is up *and* can reach its database.

**No Prometheus, no Grafana, no Tempo, no self-hosted APM, until the product is on hardware
where they are not competing with a model.**

## Consequences

**We give up aggregate questions.** "What is p95 latency this week?", "is the queue growing?",
"which section fails most often?" are not answerable from logs without a log backend that
aggregates. That is a real loss and the honest thing is to name it rather than pretend logs
cover it.

**We keep the specific ones**, which are what actually get asked at this stage. "This user
says it broke at about four" is now one search. Before this change it was unanswerable: the
detail was logged, the reader was told *"Something went wrong at our end"*, both were true,
and **nothing joined them**.

**The upgrade path is not a rewrite.** `tracing` is the instrumentation boundary, and it is
already in place. Adding an OTLP exporter is a subscriber layer and a config value; the spans
and fields being exported are the ones written today. That is the reason the decision can be
"not yet" rather than "not ever" without accruing debt.

**Reviewed when either of two things changes:** the product moves off the A1, or someone
asks an aggregate question twice in one week. The second is the real trigger — the first is
just permission.

## What was considered and rejected

**Self-hosted Prometheus + Grafana.** The default answer, and the one that costs a model.
Revisit when the box is bigger.

**Logs to a hosted aggregator now** (Loki, Better Stack, Datadog). Solves the aggregate
questions with no local RAM. Rejected *for now* on cost discipline rather than technique:
[ROADMAP.md](../ROADMAP.md) §6.4 holds infrastructure at €0/mo pre-revenue, and free tiers
here are small enough that the first busy week silently drops the logs you wanted. This is
the most likely of the rejected options to be adopted, and the cheapest to adopt — it is a
subscriber layer.

**A full UUID as the user-visible reference.** Correct, and nobody reads one out. Twelve hex
characters is unique enough for any window we retain and short enough to appear in a sentence.

**Nothing at all, and grep the journal.** What we had. It works right up to the first person
who reports a fault, which is the moment it needed to already exist.
