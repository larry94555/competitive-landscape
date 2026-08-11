# Landscape — Code Quality Standard

> The check that every change is measured against, by humans and by agents.
> See [ROADMAP.md](ROADMAP.md) for the index.

---

## 0. The standard, in one page

This codebase is maintained by **one person and a fleet of coding agents**. That combination
fails in a specific way: agents produce code that *looks* right, passes review-by-skimming,
and accumulates into something no one can hold in their head. Six months later the founder is
the bottleneck on their own product, and the agents are making it worse faster than the human
can make it better.

Everything in this document exists to prevent that one outcome.

**The standard:** *an expert who has never seen this code should be able to open any file,
understand what it does and why, and change it correctly — without asking the author.*

That is the acceptance test. Not "does it work," not "does it pass tests," not "is it
idiomatic." **Can a competent stranger change it safely.** If the answer requires the author
to be present, the work is not done.

### The five rules that carry the most weight

1. **Enforced beats agreed.** A convention that lives only in this document is a suggestion.
   Every rule below is tagged with how it is enforced (§2), and rules that cannot be enforced
   are kept few and deliberately memorable.
2. **Deletion is a feature.** Track lines removed as carefully as lines added. A codebase
   that only grows stops being maintainable on a schedule you cannot see coming.
3. **No abstraction without a second caller that exists today.** Not one you anticipate.
   Not one the pattern implies. One that is in the repository, now (§3.3).
4. **Every non-obvious decision gets an ADR.** If a reviewer would ask "why is it like
   this?", the answer belongs in `docs/decisions/`, not in a commit message nobody will find
   (§8).
5. **Agents write code; humans decide what correct means.** Schemas, evals, thresholds,
   rubrics and ADRs are human-authored and reviewed line by line. Everything downstream of a
   good specification can be delegated aggressively (§11).

---

## Before a change is proposed

Two commands, in this order:

```bash
python3 scripts/verify.py
```

```bash
python3 scripts/mutate.py <your mutations>.json
```

The first runs every gate and takes each one's own exit status; the second reintroduces the
defects your change is meant to prevent and reports anything nothing notices.

**The second is the one that matters.** This standard describes how code should be written;
`.claude/skills/coding-mistakes/SKILL.md` is the list of places the standard did not save us, and
every mechanical defect in it was found by deliberately breaking something. A guard added without
a mutation that kills it is a guard nobody has confirmed is load-bearing.


## 1. What "slop" means here, concretely

"High quality" is unfalsifiable. This is the operational definition, drawn from what
agent-assisted codebases actually degrade into. Each item is a **review-blocking defect**,
not a style preference.

| Slop pattern | Why it is a defect | The fix |
|---|---|---|
| **Speculative abstraction** — a trait with one implementor, a factory that constructs one type, a config value nobody sets | Doubles the reading cost of every call site, forever, to buy flexibility that never arrives | Inline it. Add the seam when the second implementor exists. |
| **Ceremonial naming** — `Manager`, `Helper`, `Util`, `Service`, `Processor`, `handle()`, `process()`, `doWork()` | Names that describe nothing force the reader into the body. Multiply by 200 files. | Name for what it does to what: `PricingTableParser`, `claim_verifier`, `enqueue_watch_check`. |
| **Defensive code for impossible states** | Every branch is a claim that the state is reachable. Unreachable branches are lies that make real branches harder to find. | Make illegal states unrepresentable (§4.2). If it truly cannot happen, `unreachable!()` with a reason. |
| **Comments that restate code** — `// increment the counter` | Adds tokens, no information; goes stale and becomes actively wrong | Comment *why*, never *what*. If *what* needs a comment, rename things. |
| **Copy-paste variants** — three near-identical functions differing by a literal | Fixes get applied to one of three. Divergence is silent. | Parameterize on the third occurrence, not the second (§3.3). |
| **Mock-everything tests** | Assert that the code calls the functions it calls. Pass while the product is broken. | Test behavior through real seams; fake only what you don't own (§6.3). |
| **Swallowed errors** — `let _ = ...`, `catch {}`, `.unwrap_or_default()` on a fallible operation | Converts a loud failure into a silent wrong answer, which is this product's worst failure mode | Handle, propagate, or explicitly document why discarding is correct (§5). |
| **Grab-bag modules** — `utils.rs`, `helpers.ts`, `common/` | Become write-only. Nobody knows what is in them or dares delete anything. | Put the function next to its only caller until there are three. |
| **Long functions doing several things** | Cannot be named, tested, or reviewed as a unit | Extract until each function has one nameable job (§3.2 budgets). |
| **Stylistic drift** — every file written in a different dialect | Doubles onboarding cost; makes agent output inconsistent with itself | Formatter + lint + the "match the surrounding code" rule. |
| **Magic values** | The reader cannot tell 0.85 from 0.87, or which are tuned and which are arbitrary | Named constant, with a comment saying where the number came from. |
| **Premature configurability** — flags, env vars and options with one used value | Multiplies the state space, all of it untested | Hard-code it. Add the knob when someone needs to turn it. |

**The agent-specific failure mode**, called out because it is the most likely one here:
agents are excellent at producing *plausible* structure. A `TraitFactoryBuilder` that
compiles, passes tests and does nothing useful is the characteristic artifact. The
counter-measure is §3.3 and a reviewer who asks "what would break if this layer were
deleted?" — and deletes it when the answer is nothing.

---

## 2. The enforcement ladder

Every rule in this document carries a tag. **Rules ratchet up the ladder over time**;
a convention repeatedly missed in review should become a lint, not a louder paragraph.

| Tag | Meaning | Cost of violating |
|---|---|---|
| **[COMPILER]** | The type system makes it impossible | Cannot happen |
| **[CI]** | An automated check fails the build | Cannot merge |
| **[HOOK]** | A git hook catches it locally | Caught in seconds, before push |
| **[REVIEW]** | A checklist item a reviewer must confirm | Blocks the PR |
| **[NORM]** | Written here, enforced by taste | Rots — keep these few |

**Target: fewer than 15 [NORM] rules in this document.** Every one is a bet that someone
remembers. If a [NORM] is violated twice, promote it or delete it — a rule nobody follows and
nobody enforces is worse than no rule, because it teaches that the document is decorative.

---

## 3. Simplicity

### 3.1 The architecture rule

**[REVIEW]** The system has a documented shape ([ARCHITECTURE.md](ARCHITECTURE.md)). Any
change that alters that shape — a new crate, a new external service, a new long-lived
process, a new datastore, a new cross-cutting abstraction — requires a **design review and an
ADR before code is written** (§8, §10.1).

Adding a dependency is an architecture change. So is adding a table. So is adding a
background job type.

### 3.2 Budgets

**[CI]** Numeric, enforced, deliberately generous — they catch outliers, not style.

| Budget | Limit | Rationale |
|---|---|---|
| Function length | 60 lines | Longer than one screen means it cannot be held in view while being changed |
| File length | 500 lines | Beyond this, module boundaries are wrong |
| Cyclomatic complexity | 10 | Above this, exhaustive testing is infeasible in practice |
| Function parameters | 5 | More means a struct is hiding |
| Nesting depth | 4 | Deeper means extraction or early returns |
| Public items per module | 12 | A module with 30 exports has no interface |

Exceeding a budget is not forbidden — it requires an inline `// BUDGET: <reason>` annotation,
which CI records and reports. **The count of annotations is tracked over time.** A rising
count is the earliest available signal of decay, well before anything feels wrong.

Exempt: generated code, migrations, test fixtures, and exhaustive `match` arms over
schema enums.

### 3.3 The rule of three

**[REVIEW]** The single most load-bearing rule against agent slop.

- **First occurrence:** write it inline.
- **Second occurrence:** copy it. Yes, copy it. Two similar things are not yet a pattern, and
  the wrong abstraction costs more than the duplication.
- **Third occurrence:** now abstract, and you will know the right shape because you have three
  real examples instead of one imagined one.

Corollaries:

- **No trait with one implementor**, unless it is a deliberate seam justified in an ADR.
  There are exactly four such seams today — `LlmClient`, `SourceProvider`, `EmailSender`,
  `PaymentProvider` — each with a written justification (§4.4). A fifth requires an ADR.
- **No generic parameter with one instantiation.**
- **No configuration value with one possible setting.**
- **No wrapper that only forwards.**

The reviewer's question: *"delete this layer — what breaks?"* If the answer is "nothing, you'd
just call the thing directly," delete it.

### 3.4 Dependencies

**[CI]** `cargo-deny` (licenses, advisories, duplicates) and `knip` / `cargo-machete`
(unused). **[REVIEW]** A new dependency requires a one-line justification in the PR body
answering: what it does, what it would cost to write ourselves, its maintenance status, and
its transitive footprint.

Prefer: no dependency > 50 lines of our code > a small focused crate > a framework.
**Reject** anything that pulls in a competing async runtime, a second TLS stack, or more than
~30 transitive crates for a small job.

---

## 4. Design patterns — used deliberately, not decoratively

A pattern is justified by a problem in this repository, today. Naming a pattern is not an
argument for it.

### 4.1 Patterns we use, and why

| Pattern | Where | Justification |
|---|---|---|
| **Ports & adapters** | `LlmClient`, `SourceProvider`, `EmailSender`, `PaymentProvider` | Each has **two or more real implementations today** (local + BYOK providers; SearXNG + Brave; real + test). This is the test — not "it might be swappable." |
| **Pipeline** | `landscape-analyze`: plan → fetch → read → verify → render | The domain *is* a pipeline. Stages are independently testable, independently timed, and independently cacheable. |
| **Newtype** | `AnalysisId`, `SourceLabel`, `Secret<String>`, `PromptVersion` | Prevents the whole class of "passed the wrong String." `Secret<String>` additionally redacts `Debug`, which is a security control (§9.4). |
| **Typestate / illegal states unrepresentable** | `Analysis<Draft>` → `Analysis<Verified>` | A report that has not passed verification must be impossible to render. The compiler enforces the product's central safety rule. |
| **Repository (thin)** | `landscape-db` | Query functions returning domain types. Not an ORM, not a generic `Repository<T>`. |
| **Discriminated union + exhaustive match** | SSE events, error taxonomy, section payloads | Adding a variant becomes a compile error at every site that must handle it. |
| **Builder** | `GenRequest`, `FetchPolicy` | Only where a struct has >5 fields with sensible defaults. |

### 4.2 Making illegal states unrepresentable **[COMPILER]**

The highest-value technique available in Rust, and the one that most reduces the amount of
code a reader must verify.

```rust
// NO — every consumer must remember to check, and one day one won't.
struct Report { sections: Vec<Section>, verified: bool }

// YES — an unverified report cannot reach the renderer. The compiler is the reviewer.
struct Report<S: State> { sections: Vec<Section>, _state: PhantomData<S> }
impl Report<Draft>    { fn verify(self) -> Result<Report<Verified>, VerifyError>; }
impl Report<Verified> { fn render(&self) -> Html; fn to_pdf(&self) -> Pdf; }
```

Apply to: verification state, authentication state, quota-checked requests, and any
`Option<T>` that is "always `Some` after initialization."

### 4.3 Patterns we do not use

Singletons and global mutable state (pass dependencies explicitly); DI containers (Rust has
constructors); inheritance-shaped hierarchies; `Observer` where a `tokio` channel does the
job; abstract factories; `Arc<Mutex<HashMap>>` as a substitute for thinking about ownership;
and any pattern whose motivation is "so we can swap it later" without a second implementation
in hand.

### 4.4 The four justified seams

Recorded because §3.3 forbids single-implementor traits and these are the exceptions:

- **`LlmClient`** — local llama.cpp, OpenAI-compatible, Anthropic (ARCHITECTURE §4.8). Three.
- **`SourceProvider`** — SearXNG, Brave, targeted probes, review-site adapters. Several.
- **`EmailSender`** — provider API plus a capturing test double; email is untestable otherwise.
- **`PaymentProvider`** — Stripe plus a fake clock/fake webhook double for billing tests, and
  the merchant-of-record decision is explicitly open (ARCHITECTURE §9).

A fifth seam requires an ADR arguing why the second implementation is real.

---

## 5. Error handling

**[CI]** `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` denied outside tests.

- **Libraries return typed errors** (`thiserror`), so callers can match and the API layer can
  map to correct HTTP statuses. **Binaries and tests may use `anyhow`.**
- **Every error carries context**: what was attempted, on what input, and the correlation id.
  An error a user reports must be findable in logs from the id shown on screen (§9.3).
- **No silent discards.** `let _ = fallible()` requires an inline comment stating why the
  failure is genuinely irrelevant. **[REVIEW]**
- **Errors have exactly one home.** One `AppError` enum per crate; the API layer owns the
  single mapping to HTTP problem responses.
- **User-facing messages never leak internals** — no stack traces, no SQL, no upstream
  provider bodies, no internal hostnames (this is also an SSRF-disclosure control, R10).
  Users get a plain sentence and an error id.
- **Degradation is explicit, never accidental.** This product prefers "this section could not
  be completed" over a silently thinner report — so a fallback path must set a status field
  the renderer surfaces, not just return `None`.

---

## 6. Testing

### 6.1 What each layer is actually for

The pyramid is wrong for this codebase. The shape follows the risk.

| Layer | Scope | Speed | What it protects |
|---|---|---|---|
| **Unit** | One function/module, no I/O | ms | Parsers, verifiers, normalizers, budget arithmetic — the pure logic where bugs are silent |
| **Golden / snapshot** | Fixture in → structure out | ms | Extraction and rendering. Frozen HTML → expected parse. **The regression backbone.** |
| **Integration** | Real Postgres, real HTTP via `wiremock`, fake LLM | 100s of ms | Job queue semantics, transactions, auth, quota, webhook idempotency |
| **Eval (golden set)** | Real models, frozen source fixtures | minutes | Report quality — see [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §3 |
| **E2E (Playwright)** | Browser → stack | seconds | Exactly three flows: first analysis → PDF; signup → upgrade; watch → alert |
| **Property** | Invariants over generated input | ms | Verification layer, SSRF guard, diff/SimHash normalization, quota arithmetic |

**Property tests are mandatory for the SSRF guard and the claim verifier.** Both are
security- or truth-critical, both have adversarial input, and both have invariants that are
easy to state and hard to satisfy by example: *no input resolves to a non-public address*;
*a claim whose quote is absent from its source is never `verified`*.

### 6.2 Coverage **[CI]**

- **New-code coverage ≥ 80% as a target, not a gate** — the "clean as you code" model. A
  blunt global target encourages tests on trivia; aiming the effort at new code puts it where
  change happens. It is not enforced in CI, for the reason three bullets down: a number that
  can be satisfied by assertion-free tests is a number that can be satisfied dishonestly, and
  a gate turns that from a possibility into an incentive. [ADR 0006](decisions/0006-no-sonarcloud.md).
- **100% on**: the claim verifier, the SSRF guard, quota/entitlement arithmetic, BYOK
  credential handling, webhook idempotency. Non-negotiable, no exemption path.
- Coverage is a **floor detector, not a quality measure.** 90% coverage of assertion-free
  tests is worse than 60% of real ones, because it silences the alarm.

### 6.3 Test quality **[REVIEW]**

These are reviewed as carefully as production code, because bad tests are worse than none —
they cost maintenance and provide false confidence.

- **Test behavior, not implementation.** If an internal rename breaks a test, the test was
  wrong.
- **Do not mock what you own.** Use the real module. Fake only process boundaries: HTTP
  (`wiremock`), clock, LLM, email, Stripe.
- **One reason to fail per test.** A test asserting six things tells you nothing about which
  broke.
- **Name tests as sentences**: `drops_claim_when_quote_absent_from_source`, not `test_verify_2`.
- **Arrange–Act–Assert, visibly separated.**
- **No sleeps.** Inject the clock; use deterministic scheduling.
- **Every fixed bug gets a regression test in the same PR** — asserted by the review
  checklist, and the reason the suite gets more valuable rather than merely larger.

### 6.4 Regression suites specific to this product

Four suites exist because four things silently rot in ways ordinary tests miss:

1. **Extraction goldens** — frozen HTML → expected structured parse, per site archetype.
   Catches parser drift, which QUALITY_GUARDRAILS §7 expects to be the top failure cause.
2. **Notification noise suite** — 20 recorded page-change pairs (10 material, 10 cosmetic).
   Suppression logic must not regress (ROADMAP R4).
3. **Golden-set eval** — nightly and on any prompt/model change, with hard quality gates.
4. **Structured-output parity** — the same JSON Schema must produce valid output under GBNF,
   OpenAI strict schema, and Anthropic tool schema. Runs against recorded provider responses
   in CI, and live on a schedule (ROADMAP Phase 4).

---

## 7. Linting & static analysis

### 7.1 Rust **[CI]**

```toml
# Cargo.toml [workspace.lints.rust] / [workspace.lints.clippy]
unsafe_code             = "forbid"
missing_docs            = "warn"      # public items in landscape-core
clippy::all             = "deny"
clippy::pedantic        = "warn"
clippy::unwrap_used     = "deny"      # allowed in #[cfg(test)]
clippy::expect_used     = "deny"
clippy::panic           = "deny"
clippy::todo            = "deny"
clippy::dbg_macro       = "deny"
clippy::print_stdout    = "deny"      # use tracing
clippy::cognitive_complexity = "warn"
clippy::too_many_lines  = "warn"
```

Plus `cargo fmt --check`, `cargo deny check`, `cargo machete`, and `cargo audit`.
**`unsafe` is `forbid`, not `deny`** — there is no reason for it in this codebase, and
removing the escape hatch removes the debate.

### 7.2 TypeScript **[CI]**

`tsc --noEmit` with `strict`, `noUncheckedIndexedAccess`, `noImplicitOverride`,
`exactOptionalPropertyTypes`. ESLint with `typescript-eslint` (type-checked rules),
`eslint-plugin-react-hooks`, `jsx-a11y`, and `import/no-cycle`.

**Accessibility — [CI] where checkable, [REVIEW] otherwise.** `jsx-a11y` lint plus `axe-core`
assertions in the Playwright flows. The bar: keyboard-reachable and operable without a mouse;
visible focus; correct roles and labels on every Radix primitive; **`prefers-reduced-motion`
respected by the streaming report**, which animates for 90–180 seconds and is the one surface
where motion sensitivity genuinely matters; color never the sole encoding (already required
for charts, extended to the app); and a text alternative for every chart
([COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §7.3).

Denied: `any` (use `unknown` and narrow), non-null `!`, `@ts-ignore` (`@ts-expect-error` with
a reason is fine), default exports (except route modules), barrel files (they defeat
tree-shaking and hide cycles), and hand-editing anything under `shared/types/`.

### 7.3 No third-party static-analysis service **[decided]**

This section specified SonarCloud as a merge gate. **It has been removed** — see
[ADR 0006](decisions/0006-no-sonarcloud.md).

The short version: the three things it was here for were the clean-as-you-code gate,
duplication detection, and cognitive-complexity tracking. The first is a way of introducing a
quality bar to a codebase that does not have one, and this repository had **every gate green
on an empty repository** before the first line of application code. The third overlaps
clippy's complexity lints, which are denied rather than warned.

The second — **duplication detection — is genuinely given up**, and is the honest cost. The
mitigation is not another tool: the two-copies-of-a-constant failures this project has
actually suffered were each fixed by making the second copy impossible, not by measuring
similarity.

Reversible, and cheap to reverse. If duplication becomes a real problem, supersede ADR 0006
rather than quietly ignoring it.

### 7.4 Secret scanning **[HOOK]** **[CI]**

`gitleaks` on pre-commit and in CI. Given BYOK (ROADMAP R13), one additional bespoke check:
**a CI grep that fails the build on any format string interpolating a `Secret<T>`**, plus a
test asserting `Secret`'s `Debug` and `Display` are redacting. Credential leakage is the
highest-severity failure this codebase can produce; it gets a dedicated test, not just a
convention.

---

## 8. Documenting decisions

### 8.1 Architecture Decision Records

**[REVIEW]** `docs/decisions/NNNN-short-title.md`, numbered, immutable.

```markdown
# 0007 — Postgres-backed job queue rather than a broker
Status: Accepted            (Proposed | Accepted | Superseded by 0021 | Rejected)
Date: 2026-08-14

## Context
What forced a decision. Constraints in play. What we knew and did not.

## Decision
What we are doing, in the active voice.

## Consequences
What becomes easier. What becomes harder. What we now cannot do.

## Alternatives considered
Each with why it lost — the section reviewers actually read.
```

**ADRs are never edited after Accepted.** A changed mind is a new ADR that supersedes the old
one. The history of *why* is worth more than a tidy current state — the most expensive
mistakes are the ones where someone re-litigates a decision without knowing it was already
made and rejected.

**An ADR is required for:** a new crate or top-level module; a new dependency with a large
footprint; a schema change; a new external service; a change to the verification layer; a new
port/adapter seam; a change to quality gates or budgets; anything a reviewer would question.

[ARCHITECTURE_EXPLANATION.md](ARCHITECTURE_EXPLANATION.md) is the retrospective ADR set for
every decision made before code existed. The ADR process continues it.

### 8.2 Code documentation **[CI]** for public items in `landscape-core`

- **Module docs (`//!`) are required** and answer: what lives here, what does not, and what
  the one non-obvious thing is. This is the single highest-value documentation in the
  codebase — it is what a stranger reads first.
- **Public items** get a doc comment: what, and any invariant a caller must uphold.
- **Doc examples compile** (`cargo test --doc`), so documentation cannot drift into fiction.
- **Comment why, never what.** A comment explaining *what* is a rename waiting to happen.
- **Constants carry provenance**: where the number came from, or that it is a tuned parameter
  with a link to the tuning result.

### 8.2a American spelling, everywhere **[CI]**

**One dialect, and it is American** — in prose, comments, identifiers, test names, commit
messages and product copy alike. `scripts/american_spelling.py` checks 240 words on every run.

This was not a preference until it was written down. Nobody had chosen a dialect, so the
repository grew both, and the first place it showed was the product's busiest button. Mixed
spelling is not cosmetic: `grep normalize` finds half the callers, and a reader cannot tell a
house style from a typo.

Words that are the same in both dialects are not in the list and must not be changed —
`analysis`, `analyst`, `cancellation`, `optimistic`, `emphasis`. Neither is `aria-labelledby`,
which is HTML rather than English. A passage that has to quote the other dialect to make its
point brackets itself with `american-spelling: off` and `american-spelling: on`.

### 8.3 The tutorial — `docs/TUTORIAL.md`

**A guided path that takes a competent stranger from zero to confidently changing code in
about 90 minutes.** Not API docs; a *reading order* with a question to answer at each step.

```
Chapter 0  Run it            docker compose up, a 1.7B model, one analysis. 15 min.
Chapter 1  One request       Follow a single analysis end to end, file by file, in the
                             order execution actually visits them. The spine of the tour.
Chapter 2  The schema        landscape-core: why one Rust definition drives the API, the
                             UI types, and the model's decoding grammar. The keystone.
Chapter 3  Inference         LlmClient, grammars, slots, budgets, deadlines.
Chapter 4  Verification      The claim verifier. The most important code in the repo.
Chapter 5  Jobs & schedule   The Postgres queue, watches, digests.
Chapter 6  The stream        SSE from Rust broadcast to React reducer.
Chapter 7  Change something  A guided first PR: add a field, thread it through all
                             layers, watch the type system tell you where it is missing.
```

Rules that keep it alive:

- **[CI] Documentation drift check.** A grep for banned stale terms (e.g. `seven-section` once
  the report has nine) fails the build. The documents are the product's source of truth and have
  exactly the same drift dynamics as code — every documentation inconsistency found in review to
  date was a straggler left by an otherwise-complete sweep, and all of them were greppable.
- **[CI] Every file path and symbol referenced in the tutorial is checked to exist.** A
  moved file breaks the build. This is what stops the tutorial rotting into a liability —
  documentation that is not executable is documentation that is eventually wrong.
- **[REVIEW]** A PR that changes the shape of a chapter's subject must update that chapter.
- Chapter 7 doubles as the **onboarding task for any new agent or contributor**, and as a
  smoke test of the whole toolchain.
- Each chapter links the demo video (§9.5) for the area it covers, so the tutorial can be
  watched as well as read.

---

## 9. Transparency: performance, debugging, experiments

Observability is a **code quality requirement**, not an ops afterthought. Code you cannot
observe is code you cannot maintain, and on constrained hardware
([ARCHITECTURE.md](ARCHITECTURE.md) §4.4) "where did the time go?" is the daily question.

### 9.1 Performance visibility **[REVIEW]**

- **Every operation that can exceed 100ms is a named `tracing` span.** No exceptions —
  an unnamed slow operation is invisible exactly when it matters.
- Span names are stable and hierarchical: `analysis.fetch.source`, `analysis.llm.extract`,
  `analysis.verify.claim`. Stable names are what make a trend line possible.
- Every analysis persists its own timing breakdown: per-stage duration, tokens in/out, cache
  hit/miss per layer, queue wait, and provider. Available in the admin console and via
  `?debug=1`.
- **[CI]** A performance smoke test asserts the pipeline's *shape* (stage count, absence of
  N+1 fetches, cache hit on a repeat run) rather than wall-clock, which is unstable in CI.

### 9.2 The `replay` command — the highest-leverage tool in the repo

Because sources are content-addressed and cached, any past analysis can be re-run
deterministically from its exact inputs:

```bash
landscape replay <analysis_id>                          # reproduce exactly
landscape replay <analysis_id> --prompt-version v7      # what would the new prompt do?
landscape replay <analysis_id> --model qwen3-14b        # what would a bigger model do?
landscape replay <analysis_id> --explain                # full span tree + dropped claims
```

One mechanism serves four purposes: **bug reproduction** (a user's report id is a complete
repro), **prompt and model evaluation** (offline A/B on real traffic, costing nothing),
**regression testing** (a replayed analysis becomes a golden case), and **debugging**
(`--explain` shows every claim the verifier dropped and why).

**[REVIEW]** Any change to the analysis pipeline must keep replay deterministic. Sources of
nondeterminism — wall-clock reads, RNG, unseeded sampling, iteration over `HashMap` — must be
injected, not called directly. This constraint pays for itself many times over.

### 9.3 Troubleshooting **[CI]**

- **Correlation id on every request**, propagated through jobs, logged on every span, and
  shown to the user on any error. A support thread that quotes an id must be resolvable
  without asking follow-up questions.
- **Structured logs only** — `tracing` fields, never string interpolation. Grep is not an
  observability strategy.
- **Log levels mean things**: `error` = a human must act; `warn` = degraded but handled;
  `info` = state changes worth an audit trail; `debug` = development. **[REVIEW]** `error` for
  something nobody will act on trains the operator to ignore errors.

### 9.4 A/B testing and experiments

A small framework, because the roadmap depends on it (model upgrades, prompt versions, tier
boundaries) and because ad-hoc experimentation produces confident wrong conclusions.

```rust
let variant = experiments.assign("synth_prompt_v7", subject_key);  // deterministic hash
```

- **Deterministic assignment** from a stable subject key — a user must not flip variants
  between page loads.
- **Exposure is logged, not inferred.** An experiment measures those actually exposed.
- **Every analysis records its variant set**, so any metric can be sliced retrospectively.
- **[REVIEW]** Every experiment declares, *before it starts*: the hypothesis, the primary
  metric, the minimum runtime, and the stop date. Experiments without a stop date become
  permanent forks of the codebase — the specific way experiment frameworks turn into debt.
- **Offline first.** `replay --prompt-version` answers most questions on historical traffic
  without exposing a single user. Live experiments are for what replay cannot settle.
- **Quality gates apply to every variant.** An experiment may not ship a variant that fails
  the golden set, however good its engagement metric looks.

### 9.5 Demo videos — UI in action plus a code walkthrough

**Why this exists.** Two of this product's hardest review problems are not visible in a diff.
The core UX is *progressive streaming* — a time-based experience a screenshot cannot show —
and the founder reviews their own agent-generated code, where reading for intent hides the
gap between what was asked for and what was built (§10.2). **Watching a feature work is a
different cognitive act from reading it**, and it catches "the code is right, the experience
is wrong." It is also a forcing function: a feature whose demo cannot be scripted in a few
steps probably is not reachable by a real user.

#### What a demo contains

One video, **120 seconds hard maximum** — a demo nobody watches is worse than none.

| Segment | Length | Content |
|---|---|---|
| Title card | 2s | PR number, title, commit SHA, branch |
| **UI in action** | 20–60s | The feature working, recorded from a `@demo`-tagged Playwright spec |
| **Code walkthrough** | 20–50s | 3–6 steps through the changed code: what changed, where, and why |
| End card | 3s | What the reviewer should scrutinize; link to the ADR |

**Every segment is subtitled and silent.** Captions are burned in, so the video is fully
comprehensible with sound off — which is also how GitHub autoplays it, and what makes it
accessible by default. No text-to-speech in v1; it adds a toolchain and a rendering step to
convey what the captions already convey. (If narration is wanted later for marketing,
**Piper** is the open-source option — but not on the inference box.)

#### Toolchain — all open source, nothing new to the stack

| Job | Tool | Note |
|---|---|---|
| Record the UI | **Playwright** `recordVideo` | Already in the stack for E2E (§6.1). WebM, 1280×720. |
| Render the code walkthrough | **A static HTML deck driven by Playwright** | One step per concept: syntax-highlighted diff, changed lines emphasized, scroll/highlight in CSS. Reuses the recorder we already have instead of adding a screen-capture tool. |
| CLI demos (`landscape replay`, migrations) | **VHS** (charmbracelet) | Scripted `.tape` files → MP4. The right tool for terminal-native output. |
| Concat, title cards, captions, encode | **ffmpeg** | WebM → H.264 MP4 (`-pix_fmt yuv420p -movflags +faststart`); two-pass palette for the GIF. |
| Write the walkthrough script and captions | **Claude** | See below — this is the part that genuinely needs a model. |

Claude cannot render video and should not be asked to. It writes the harness, the demo
script, and the captions; open-source tools do the recording. Building a bespoke video tool
would be precisely the speculative abstraction §1 calls a defect.

> **Craft rules** — how the narration is written, how subtitles are delivered, and how the
> voice is paced — are in [Video_Guidelines.md](Video_Guidelines.md). This section is policy:
> when a demo is required and what it must contain.

#### The narration script is a reviewed artifact, not runtime output

**[REVIEW]** Each demo has a committed script at `demos/<slug>.demo.md`:

```markdown
---
slug: watch-create-sheet
ui_spec: e2e/watch.spec.ts@creates a watch from a report
max_seconds: 90
---
## UI
- [0:00] "From a finished report, one click opens the watch sheet."
- [0:06] "Pricing page and changelog are pre-selected — no naming, no folders."

## Code
- [0:24] crates/landscape-watch/src/create.rs:41
  "Watch creation is one transaction with the first scheduled check, so a watch
   can never exist without a job that will run it."
- [0:38] crates/landscape-db/migrations/0031_watches.sql
  "The unique index on (user_id, target_url) is what makes re-watching idempotent."
```

Committing the script rather than generating captions at record time buys three things:
the video is **deterministic and reproducible** (same principle as `replay`, §9.2); the
**reviewer reads the script as part of the diff**, so a wrong explanation is caught like any
other wrong code; and a model cannot invent a description of code at render time, which is
the exact failure this whole document is built to prevent.

Timestamps and file references are **[CI]-checked**: a referenced path or spec that does not
exist fails the build, the same mechanism that keeps the tutorial honest (§8.3).

#### Pipeline

```
@demo Playwright specs ──> WebM ─┐
walkthrough HTML + Playwright ──>┤
VHS .tape (CLI demos) ──────────>┴─> ffmpeg concat
                                      ├─ burn captions (ASS styling from the .demo.md)
                                      ├─> demo.mp4  (full quality → CI artifact)
                                      ├─> demo.vtt  (sidecar, for accessibility and reuse)
                                      └─> demo.gif  (≤1MB, 15fps, inline-renderable)

demo.gif ──> orphan `demo-assets` branch at <pr>-<sha>.gif
         ──> bot comments on the PR with the raw.githubusercontent URL
```

The orphan branch matters: GIFs render inline in PR comments via `raw.githubusercontent.com`,
but committing them to `main` would bloat the repository permanently.

#### The honesty rule **[CI]**

On Rung 0 a real analysis takes 90–180 seconds ([ARCHITECTURE.md](ARCHITECTURE.md) §4.4), so
UI demos record against a stubbed inference layer. **A sped-up demo that quietly misrepresents
latency would undermine the one thing this project has been careful to be honest about.**
Therefore:

- Every stubbed demo burns in a persistent corner label:
  `Stubbed inference — not representative of real latency`.
- **One demo per release is recorded at real speed**, unedited, and linked from the changelog,
  so the actual experience stays visible to the founder and to users.
- Speed ramps are permitted only on waits, never on interactions, and the ramp factor is
  displayed.

#### When it is required **[CI]**

| Change | Demo |
|---|---|
| Touches `frontend/**`, a route, an email template, or the PDF | **Required** — merge blocked without one |
| New CLI command or operator workflow | **Required** (VHS) |
| Backend-only, refactor, dependency bump, docs, lint | Not required |

Escape hatch: a **`no-demo` label with a one-line justification**, counted and reported
alongside `// BUDGET:` annotations (§3.2). A rising count is visible rather than silent.

#### Storage & pruning **[CI]**

Free on a public repository — GitHub Actions minutes and artifact storage are unmetered for
public repos, and every tool above is open source. Two real costs remain:

- **Repository growth.** ≤1MB per GIF, so roughly 100MB/year at 100 PRs. A scheduled job
  prunes the `demo-assets` branch: keep demos for merged PRs and tagged releases, drop those
  for closed PRs, and drop anything older than two releases. **Set this up with the pipeline,
  not later** — retrofitting it means rewriting branch history.
- MP4 artifacts use 14-day retention; the GIF is the durable record.

If the repository ever goes private, Actions minutes become metered (2,000/month free) — but
the Rust build, not the demo, is what would consume them. **Do not solve that with a
self-hosted runner while the repo is public**: fork PRs can execute arbitrary code on the
runner, and the only spare machine is the production box holding the database and encrypted
user API keys.

---

## 10. Review

The hardest part of this document for a solo founder, and the part most likely to be skipped.
It is written to be honest about that rather than to describe a review process that will not
happen.

### 10.1 Design review — *before* code

Triggered by the §3.1 list. The output is an ADR. Questions, in order:

1. What problem, in one sentence, and who has it?
2. What is the smallest change that solves it?
3. What does this make harder or impossible later?
4. What existing thing does it duplicate?
5. **What can be deleted as part of this?**
6. How will we know it worked — which metric?
7. How is it tested? If the answer is "manually," the design is wrong.

**A design review may be conducted with an agent as interlocutor, but the decision and the
ADR are human.** Recommended practice: have an agent argue the *opposing* design and write
its strongest case before deciding. Cheap, and it surfaces the alternative you were going to
skip.

### 10.2 Code review — three tiers by risk

| Tier | Scope | Process |
|---|---|---|
| **Hot zone** | auth, billing, SSRF guard, BYOK credentials, the verification layer, migrations | **100% human line-by-line review, always, no exceptions.** Agent review first, then human. Never merged the same hour it was written. |
| **Standard** | features, endpoints, UI, jobs | Agent review against §10.3, then human review of the diff. Green CI is necessary, not sufficient. |
| **Mechanical** | dependency bumps, formatting, generated code, doc typos | Green CI + human skim. |

**The self-review problem, stated plainly.** One person cannot independently review their own
work, and agent-generated code reviewed by its requester gets the shallowest review of all —
you already believe it. Three mitigations, none complete:

1. **Sleep on it.** Hot-zone and standard PRs are reviewed at least a few hours after they
   were written, ideally the next morning. Distance is the closest available substitute for a
   second person.
2. **An adversarial review agent**, prompted to find defects rather than to approve, running
   §10.3 as a checklist, with an explicit instruction that finding nothing is a valid but
   *unusual* outcome. Agents told to "review this" will praise it; agents told to "find what
   breaks this" will not.
3. **Review the diff, not the intent.** Read what is there, not what you meant. Agents write
   plausible code that does something adjacent to what you asked — that gap is invisible if
   you read for intent.

### 10.3 The review checklist

Every PR. A reviewer who cannot answer yes to all of these does not approve.

**Correctness**
- [ ] I understand what every changed line does, without asking the author.
- [ ] Failure modes are handled; no swallowed errors; degradation is explicit.
- [ ] No new `unwrap`/`expect`/`panic` outside tests.
- [ ] Concurrency: no new shared mutable state; no lock held across `.await`.

**Simplicity (the anti-slop pass)**
- [ ] **Delete any layer here — what breaks?** Nothing ⇒ it goes.
- [ ] No abstraction with a single caller (§3.3).
- [ ] No name from the banned list; every name says what it does.
- [ ] No config, flag, or parameter added without a caller that sets it.
- [ ] Something was deleted, or the PR explains why nothing could be.

**Tests**
- [ ] Tests fail if the behavior regresses — verified by breaking it locally, not assumed.
- [ ] Behavior, not implementation. No mocking of code we own.
- [ ] Every fixed bug has a regression test in this PR.

**Documentation**
- [ ] Module docs updated if the module's job changed.
- [ ] ADR present if §8.1 requires one.
- [ ] Tutorial chapter updated if it describes this area.
- [ ] Comments explain *why*; none restate the code.

**Observability**
- [ ] New operations >100ms have named spans.
- [ ] New failure paths produce an actionable error with a correlation id.
- [ ] Replay determinism preserved.
- [ ] **I watched the demo, and the experience matches the intent** — not merely "a demo
      exists." The walkthrough script describes the code that is actually here (§9.5).

**Language about third parties** (blocking, wherever the change produces user-visible text)
- [ ] No sentence characterizes a publisher, a source, or a company. Every statement is about
      what **we** confirmed or could not confirm ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5).
- [ ] Where two values differ, both are shown with dates and links, and neither is adjudicated.
- [ ] Field names, enum variants and log messages follow the same rule — internal vocabulary
      leaks into interfaces.

**Security** (blocking; escalate to hot-zone review if any is unclear)
- [ ] No secret can reach a log, an error body, or a response.
- [ ] User-supplied URLs go through the SSRF guard, including redirects.
- [ ] Queries parameterized; user content sanitized at render.
- [ ] No new endpoint without authentication, authorization, and a rate limit.

### 10.4 Merge gates **[CI]**

`fmt` · `clippy -D warnings` · `tsc` · `eslint` · unit + integration tests · `cargo deny` ·
`cargo audit` · `gitleaks` · budget report · tutorial link check · American spelling (§8.2a) ·
**demo video for user-visible changes** (§9.5) · **golden-set eval for anything touching the
analysis pipeline**.

**Coverage is reported here, not gated.** This list previously said "new-code coverage ≥ 80%"
alongside the Sonar gate that would have enforced it; both are gone. The reason is §6.2's
own: *90% coverage of assertion-free tests is worse than 60% of real ones, because it
silences the alarm.* `cargo llvm-cov` prints a summary on every push, and what the number is
for is noticing a **drop**.

**The 100% requirements in §6.2 are unchanged** and are not coverage targets in this sense —
the claim verifier, the SSRF guard, quota arithmetic, BYOK handling and webhook idempotency
have no exemption path. None of them exists yet.

Merges are **squash-only**, with a message explaining *why*. The commit log is documentation.

---

## 11. Working with coding agents

The codebase is optimized for agents as first-class contributors, which mostly means being
ruthlessly explicit about things a human would infer.

### 11.1 `AGENTS.md` / `CLAUDE.md` at the repository root

Kept short — a long context file is a skimmed context file. It contains: the build and test
commands, the three or four conventions most often violated, the hot-zone list, and a pointer
here. Not a copy of this document.

### 11.2 The contract

- **Agents may:** implement against a written spec, write tests, refactor within a module,
  write adapters against an existing trait, fix lint, upgrade dependencies, draft
  documentation and tutorial chapters.
- **Agents may not, without human authorship and review:** change the report schema, change
  quality gates or thresholds, touch the verification layer, touch hot-zone code, add a
  dependency, add a port/adapter seam, or write an ADR's Decision section.
- **Every agent PR states which tier it falls into** and links the spec or ADR it implements.
  A PR that cannot name what it implements is not ready for review.

### 11.3 Why the gates matter more with agents

Agents produce more code, faster, with more plausible-looking structure than a human writing
by hand. Every mechanism in this document — budgets, rule of three, ADRs, the delete-a-layer
question — exists because the volume is high and the plausibility is high. **The eval suite
and the type system are what make agent contributions safe to accept**: they are the only
reviewers that never get tired and never assume good intent.

Corollary worth stating: **the highest-leverage work the founder does is not writing code.**
It is writing the schema, the evals, the thresholds, and the ADRs — the artifacts that define
what correct means. Everything downstream of those can be delegated. Nothing upstream can.

---

## 12. Debt, refactoring, and keeping this document honest

- **Debt is tracked, not felt.** A `DEBT:` annotation with an issue link; the count is
  reported by CI alongside budget annotations. Rising counts are the early-warning signal.
- **The boy-scout rule, bounded**: leave the file better, but unrelated refactors go in a
  separate PR. Mixed PRs cannot be reviewed properly and are where regressions hide.
- **A refactor PR changes no behavior** and says so in its title. Its tests must be unchanged
  — if they had to change, it was not a refactor.
- **Quarterly**: review the budget-annotation and debt counts, the [NORM] rules (promote or
  delete), the dependency tree, and this document. **A quality standard nobody has read in
  six months is not a standard.**
- **This document is itself under review.** It is wrong somewhere. When a rule fights real
  work more than it prevents real defects, change it in a PR with the evidence — do not
  quietly ignore it. A rule that is routinely ignored teaches that all the rules are optional,
  which is how a standard dies.
