# Landscape — Benchmarks

> Real numbers, with the hardware they came from. `docs/ROADMAP.md` Phase 0 requires this
> file before a model is chosen, because every latency figure in the specification is
> otherwise an estimate.
>
> **Still nothing measured on the target hardware.** The A1 is not provisioned. These are
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
exactly why this must be re-run on the A1 before anything is concluded.**

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

## Still to measure — all of it on the A1

None of this is done. It is the remainder of Phase 0's model work.

- [ ] Provision the A1 and convert to Pay-As-You-Go
- [ ] Re-run everything above on 4 ARM cores. **Prefill is expected to behave differently**,
      and the tiering conclusion depends on it
- [ ] Prefill and generation tok/s separately, rather than end-to-end latency
- [ ] `Q4_K_M` **and** `Q4_0` — ARM repacking may make the lower-quality format faster
- [ ] Qwen3 1.7B / 4B / 8B / 14B, Gemma 3 4B/12B, Llama 3.2 3B. **Licence review first**
- [ ] Aggregate throughput at `--parallel 1/2/4/8`
- [ ] Resident RAM for three models against the ~17 GB budget
- [ ] `q8_0` KV cache quantization validated against the golden set
- [ ] Time-to-first-token
- [ ] One server at a time, so the numbers are not contended

Run 3 adds two of its own:

- [ ] **Field order as a lever.** llama.cpp walks properties in the order the schema
      serialises them, and `serde_json`'s sorted maps currently pick that order for us.
      `preserve_order` would hand it back. Quote-first (read, then answer) against
      quote-last (answer, then justify) is a measurable question, and a hand probe showed
      quote-first spends the whole token budget quoting — so it needs a `maxLength` too
- [ ] **Whether the 1.7B's plan confusion survives better prompting**, or is a size limit.
      This decides whether the Extractor tier can ever be a 1.7B, which is the difference
      between ~7 s and ~12 s per span on the numbers above
