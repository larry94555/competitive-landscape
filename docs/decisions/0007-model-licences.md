# ADR 0007 — Qwen3 is the only candidate family with no strings attached

**Status:** accepted
**Date:** 2026-08-03

## Context

[ROADMAP.md](../ROADMAP.md) Phase 0 makes this a precondition rather than a footnote:

> **License review precedes benchmarking** — a model we cannot use commercially is not a
> candidate.

The bake-off list is Qwen3 1.7B/4B/8B/14B, plus Gemma 3 4B/12B and Llama 3.2 3B as
alternates. Benchmarking a model we cannot ship would waste the scarce thing here, which is
time on the A1.

Read from the primary sources on 2026-08-03 — the model cards and the licence texts
themselves, not summaries.

## What the licences actually say

### Qwen3 — Apache-2.0

Every size, and the GGUF repositories we actually pull (`Qwen/Qwen3-1.7B-GGUF` is tagged
`apache-2.0`, matching its base model).

Obligations: attribution, include the licence, state changes. Nothing else. No acceptance
click, no use-restriction policy to pass on, no user threshold, no geographic carve-out, and
**nobody who can switch it off**.

### Gemma 3 — custom Google terms, and one clause that matters more than the rest

Commercial hosting is explicitly permitted: "Distribution" includes providing Gemma
functionality *"as a hosted service via API, web access, or any other electronic or remote
means."* So it is usable. The obligations are the problem:

- **§3.1 requires us to include the use restrictions as an enforceable provision in our own
  terms of service.** Adopting Gemma means editing our ToS and enforcing Google's policy
  against our users.
- Hosted services must **notify subsequent users** that the model is subject to those
  restrictions.
- **Google "reserves the right to restrict (remotely or otherwise) usage of any of the Gemma
  Services that Google reasonably believes are in violation of this Agreement."**

That last one is the finding. [ARCHITECTURE.md](../ARCHITECTURE.md) runs models locally
partly so that no third party sits between us and our own analysis path. **A licence under
which a third party can remotely restrict our use reintroduces exactly the dependency the
local-model decision was taken to avoid** — differently shaped than an API key, and with the
same failure mode: someone else can stop the product working.

### Llama 3.2 — Community License

- **"Built with Llama" must be prominently displayed** on the website, UI, or documentation.
- Derivative models must be **named beginning with "Llama"**.
- Above **700 million monthly active users**, a separate licence must be requested from Meta.
  Not a live concern, and worth recording so nobody re-derives it.
- **The multimodal models are not licensed to companies whose principal place of business is
  in the European Union.** Text-only Llama 3.2 models are unaffected, and this project needs
  text only — so this does not currently bite. **It is flagged because this project's costs
  are denominated in euros and the domicile has never been written down.** If the company is
  EU-based, any future multimodal work is closed off on this family.

## Decision

**All three families are usable. Qwen3 is preferred, and the preference is now a licence
argument as well as a benchmark one.**

- **Qwen3** — benchmark all four sizes. No conditions to meet.
- **Gemma 3** — benchmark, but treat adoption as carrying a cost that does not appear in any
  latency table: a ToS amendment, a downstream notice obligation, and a third party with a
  remote off-switch. **If Gemma wins on numbers, that is a decision to take deliberately, not
  a default to fall into.**
- **Llama 3.2 3B** — benchmark. The attribution and naming requirements are cheap and
  permanent; note them before shipping rather than after.

## Consequences

**The bake-off is no longer "measure and pick the fastest".** Two of the three families cost
something to adopt beyond RAM and latency, and those costs are invisible to
`landscape-bench`. The comparison table in `BENCHMARKS.md` gains a licence column so the
trade is visible at the point of decision.

**Qwen3 winning would be the cheapest outcome**, and on the evidence so far it is also the
likely one — Runs 2 and 3 already establish a working Qwen3 pairing. This ADR mostly protects
against the case where a benchmark makes an alternate look attractive and the obligations are
discovered afterwards.

**Two things this does not settle.** The domicile question above, which is a founder question
and not a technical one. And whether the "Built with Llama" attribution is acceptable on a
product whose credibility rests on saying where things came from — probably yes, arguably
even on-brand, but it is a positioning call rather than a legal one.

**Re-read on any model bump.** These are not stable documents; Meta and Google have both
revised theirs. A model version bump is a licence review, not just a download.
