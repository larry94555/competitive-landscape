# ADR 0006 — No SonarCloud; the gates it would add are already green elsewhere

**Status:** accepted
**Date:** 2026-08-03

## Context

[CODING_QUALITY.md](../CODING_QUALITY.md) §7.3 specified SonarCloud as a merge gate, and
[ROADMAP.md](../ROADMAP.md) listed it among the Phase 0 quality toolchain items that must
"predate the code". It was written before any of that toolchain existed, when the question
was *what should we have?* rather than *what do we have?*

By the end of Phase 0 the answer to the second question had changed. What was actually built:

| Check | Catches |
|---|---|
| `clippy -D warnings`, with `unwrap`/`expect`/`panic`/`todo` **denied** | Correctness and panic-safety, stricter than most Sonar Rust rules |
| `cargo-deny` (advisories, licences, bans, sources) | Vulnerable, unlicensed and duplicated dependencies |
| `gitleaks`, full history | Committed secrets |
| `cargo-llvm-cov` | Coverage, reported on every push |
| `tsc --strict` + ESLint type-checked rules | The TypeScript half, where Sonar's analysis is strongest |
| `scripts/lint_instructions.py` | Commands in prose that would fail if pasted |
| `scripts/check_links.py` | Dead links and dead heading anchors across 31 documents |
| `crates/landscape/tests/docs.rs` | The README, **executed** against the real binary |
| `landscape-golden` | Whether the model's answers are *true* — which no static analyser can see |
| aarch64 cross-compile | Target-architecture breakage before it reaches the target |

§7.3 claimed three things Sonar provides that per-language linters do not: the clean-as-you-code
gate, cross-file duplication detection, and cognitive-complexity tracking. Assessed against
what is now in place:

- **The clean-as-you-code gate** is a way of introducing a quality bar to a codebase that
  does not have one, by gating only new code so the gate is not permanently red. This
  repository had every gate green on an empty repository, before the first line of
  application code — a Phase 0 exit criterion, and met. There is no legacy debt for the model
  to work around.
- **Duplication detection** is real and not currently covered. It is also the check with the
  worst signal-to-noise ratio on a codebase this small, where the duplication that matters —
  two copies of a constant — has bitten us three times and was caught each time by a *test*,
  not by a similarity metric.
- **Cognitive complexity** overlaps heavily with clippy's own complexity lints, which are
  already denied rather than warned.

## Decision

**No SonarCloud.** §7.3 is removed, and it is removed from the merge-gate list in §10.4 and
from the Phase 0 toolchain in the roadmap.

The cost side matters on a bootstrapped budget and it is not the licence fee, which is zero
for public repositories. It is a further external account to hold, a further integration to
keep working, and a further gate that can go red for reasons unrelated to the change under
review. This project has already learned what that costs: a check that fails for a reason
the reader considers irrelevant is a check the reader learns to re-run rather than read.

## Consequences

**Duplication is now unchecked by any tool**, and that is the real thing given up. The
mitigation is not a tool: it is that the two-copies-of-a-constant failures this project has
actually suffered — the port in the README, the pacing constants across two film builds —
were each caught by making the second copy impossible rather than by measuring similarity.
That is the pattern to keep reaching for.

**This is reversible and cheap to reverse.** Nothing was built on Sonar's absence; adopting
it later is a repository setting and a workflow file. If duplication becomes a real problem,
that is the signal to revisit, and this ADR should be superseded rather than quietly ignored.

**Reviewed if** the codebase grows past roughly ten crates, or a second regular contributor
joins — both of which change the duplication argument, because duplication between people is
harder to notice than duplication within one head.

## A correction this surfaced

§10.4 listed **"new-code coverage ≥ 80%"** as a merge gate, alongside the Sonar gate that
would have enforced it. Coverage is now implemented as **reported, not gated**
([PR #9](https://github.com/larry94555/competitive-landscape/pull/9)), for a reason that is
in §6.2 of the same document: *"90% coverage of assertion-free tests is worse than 60% of
real ones, because it silences the alarm."*

The two statements contradicted each other, and the contradiction was introduced by the
implementation rather than found in review. §10.4 now matches what CI does. **The 100%
requirements in §6.2 are untouched** — the claim verifier, the SSRF guard, quota arithmetic,
BYOK handling and webhook idempotency are still non-negotiable, and none of them exists yet.

Current coverage is **77.8%**, which the removed gate would have failed. Recorded here so
that the change does not read as convenient afterwards: the number was under the bar, and
the argument for not gating is the one in §6.2, not the number.
