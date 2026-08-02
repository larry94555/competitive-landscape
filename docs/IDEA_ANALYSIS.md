# Landscape — The Eight Levels of Idea Analysis

> A competitor table answers one question. A founder evaluating an idea has eight, and the
> later ones matter more than the first. This document specifies all eight, what evidence
> each can actually rest on, and — the load-bearing decision — **which of them the product is
> allowed to answer and which it may only ask.**
>
> Companion documents: [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) covers the venues and
> their access terms; [FACT_CHECKING.md](FACT_CHECKING.md) governs dispositions and
> auditable negatives; [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md)
> governs the competitor sections, which are level 4 of the eight below.

---

## 1. The central decision: two registers, never blurred

The eight levels are not the same *kind* of question, and treating them as one would break
this product. They split cleanly:

| | Levels | Bound by | Who wins |
|---|---|---|---|
| **Observation** | 4, 5, 8, and the evidence half of 1–3 | **Retrieval** — how many places you looked, how well you cited them | **Us.** Systematic search, dated links, auditable negatives, monitoring |
| **Framing** | 6, 7, and the interpretive half of 1–3 | **Reasoning** — how well you think about a business | **A frontier chatbot.** Comfortably. |

That second row is uncomfortable and it is the most important sentence in this document:

> **On levels 6 and 7, a local 8B model will produce worse output than the reader's own
> ChatGPT or Claude subscription — and the reader very likely has one.**

Every framework the user cited — *Crossing the Chasm*, *The Cold Start Problem*, *The
Tipping Point*, *The Innovator's Dilemma* — is deeply internalised by every frontier model.
"Does the Cold Start Problem apply to my idea?" is close to an ideal chatbot prompt. If we
emit confident strategic prose from a quantised 8B model, we ship something *worse than free*
under a brand built entirely on rigour, and we contaminate trust in levels 4, 5 and 8, where
we are genuinely strong.

**So the rule:**

> **Observation levels produce answers, each with a source. Framing levels produce
> *questions*, each with the evidence that raised it.**

This is not a hedge. It is a better product. A founder does not need to be told "your idea
has a network-effect dependency" — they need to be told **which four questions they now have
to answer, and what in the public record suggests they apply.** Questions are cheap to
generate, impossible to be confidently wrong about, and are the thing a chatbot will not give
you, because a chatbot always answers.

---

## 2. The eight levels

### Level 1 — Problem space

**Asks:** what problem does this address, and is there public evidence anyone has it?

**Evidence:** complaint threads, question posts, forum discussions, "how do I…" search
demand, existing tools' review complaints ([DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md)
§3.1–3.2).

**Output:** the problem stated in *sufferers' own words*, quoted and linked — never
paraphrased into vendor language. Plus the negative when it applies.

**When nothing is found, the wording is fixed and non-negotiable:**

> **The problem space is not clear from public sources.** We searched *[venues]* with
> *[queries]* and did not find people describing this problem in their own words.

**Not** "there is no problem here." The distinction is the whole product. Nobody was posting
about the absence of an iPhone in 2006. A problem can be invisible because it is unarticulated,
because sufferers are not online, because it is too ordinary to post about, or because our
queries were in the wrong register (§5 of DISCUSSION_SIGNALS). An unclear problem space is a
**prompt to go and talk to ten people**, which is exactly what it should provoke — and it is
information the founder did not have five minutes ago.

### Level 2 — Value proposition

**Asks:** what end result does this produce? Does it remove a problem, reduce its harm, reduce
its frequency, or add a benefit? Is there a payoff, and to whom?

**Evidence:** how comparable products describe their own outcomes (primary, quotable); what
users say changed for them (review and discussion sources); what people say they would pay
for.

**The one honest constraint:** we can report *claimed* value and *reported* value. We cannot
verify that value is delivered, and we never assert it. The taxonomy the user proposed —
solve / reduce harm / reduce frequency / add benefit — is a genuinely useful classification
and is applied to **each competitor's own claim**, attributed, rather than asserted about the
idea itself.

### Level 3 — The idea as a product

**Asks:** what is it, how is it used, who are the customers, who pays, why would they pay,
why would they use it, is it new, has it been tried, what changed technically?

**Two halves, and they are treated differently:**

*Observation half* — **has it been tried?** This is the single highest-value question in the
whole framework and it is pure retrieval, which means it is ours. Dead startups, archived
GitHub projects, launch posts that went nowhere, forum threads from 2018 saying "I built this
and here's why it failed." A founder who learns their idea was attempted three times and died
three times, **with links**, has received more value than any competitor table provides.

Archived and abandoned projects are surfaced *prominently*, not filtered out. "Someone built
this and stopped" is a finding, and the reasons are often written down in the final commit or
the shutdown post.

*Framing half* — who pays, why, is it new. Questions, with evidence attached.

**"Do new technologies provide new opportunities?"** is reported strictly as: what changed in
the public record, and when — a capability became cheap, an API opened, a regulation shifted,
a model class became runnable locally. Dated facts, not futurism.

### Level 4 — The idea as a business

**Asks:** do businesses exist, how long have they been around, what do they offer, how are
they doing, what do people complain about, what do they charge, how big is the market?

**This is the existing product.** Fully specified in
[COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) and
[COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md); pricing, features, changes, operating
signals, sources.

**"How well are they doing"** stays bounded by the existing rule: publicly visible since,
last public update, open roles, disclosed financials only. **No composite health score, and
private-company revenue is never estimated.** Where a market size is quoted it is attributed
to whoever published it, with its date and methodology, never synthesised
([Off-The-Napkin-Estimates.md](Off-The-Napkin-Estimates.md) governs what may be estimated).

### Level 5 — Discussions and reviews

**Asks:** what is publicly said about the problem, the value proposition, the product idea,
the market, the customers?

**This is the thoughtspace level** — the user's framing is exactly right that this is *not
about facts*. It is about what thinking already exists, so a founder can go and read it.

That changes the bar. Elsewhere we require verifiable claims. Here the artifact is **a
reading list**: dated, attributed, linked, with enough context to decide whether to open it.
We are not asserting that a blogger is correct. We are asserting that they wrote it, when,
and where — which is checkable — and letting the founder read.

Venues, terms, and the absence panel: [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4.

### Level 6 — Power niches

**Asks:** what are the larger and smaller niches? How does this relate to adjacent problems?
Would a narrower niche be stronger?

**Framing level. Questions only.**

*What we can observe:* which adjacent categories exist, which segments competitors explicitly
name as their target, where the review complaints cluster by customer type, which niches have
discussion volume and which are silent. All sourced.

*What we do not do:* recommend a niche. We surface the segmentation visible in the public
record and pose the choice.

> Three of the four products found target restaurants with 10+ locations [S3, S7, S9].
> None state a position on single-site restaurants. The complaint threads we found are
> mostly from single-site owners [S14, S15, S18].
> **Question this raises:** is the underserved segment the one you are aiming at, and is it
> large enough? *Crossing the Chasm* frames this as beachhead selection.

Note the shape: evidence, then a question, then a named framework the founder can go and read
for themselves. **We cite the books; we do not reproduce them.** A named framework is a
reference, not content we reprint — one sentence of orientation and a pointer, never a summary
that substitutes for the book.

### Level 7 — Startup trajectory

**Asks:** does this depend on a network effect (*Cold Start Problem*)? On mass adoption
(*Tipping Point*)? On entering below the incumbents (*Innovator's Dilemma*)?

**Framing level. Questions only, and this is the level where that discipline matters most.**

The mechanism is a **cheap, deterministic classifier, not an essay.** Three yes/no/unclear
signals, each with the evidence that triggered it:

| Signal | Evidence that raises it |
|---|---|
| **Two-sided or network-dependent** | Competitors' own copy describes both sides; reviews complain about empty supply or demand |
| **Mass-adoption dependent** | Low price point, consumer positioning, virality mentioned in competitor materials |
| **Low-end entry** | Incumbents priced high with heavy feature sets; complaint themes about cost and complexity |

Each firing signal emits **the questions that framework says must be answered** — not our
answers to them.

> This idea appears two-sided: farms on one side, restaurants on the other. Two of the three
> products found describe recruiting both [S3, S7], and one review thread describes signing
> up and finding no farms listed [S15].
> **If that is right, these are the questions to answer:** which side do you solve first?
> What is the smallest geography where both sides are dense enough to be useful? What does
> the network do for the very first user, before there is a network? *The Cold Start Problem*
> is written about exactly this.

This is honest about our limits, genuinely useful, costs almost no generation budget, and is
robust to being wrong — a misfired question wastes thirty seconds; a misfired assertion costs
trust permanently.

### Level 8 — Investor interest

**Asks:** which VCs and angels have publicly expressed interest in this problem space,
technology, or market?

**Observation level, and better-evidenced than it first appears.** The open sources are
strong precisely because investors publish constantly and in structured form:

| Source | Access | What it yields |
|---|---|---|
| **VC firm blogs** | RSS/Atom, ordinary web, robots-governed | Thesis posts, "what we're looking for", market maps |
| **Partner personal blogs** | RSS | Where theses are actually stated |
| **Podcast feeds** | RSS/XML, open | Long-form thesis; show notes often list topics |
| **Portfolio pages** | Primary, on the firm's own domain | Revealed preference, which beats stated preference |
| **SEC Form D (EDGAR)** | Public, free, full-text search | Who actually raised, when, how much — filings, not press |
| **YouTube** | 100 searches/day (§4 DISCUSSION_SIGNALS) | Conference talks, interviews — link-level only |
| **X / LinkedIn** | Effectively closed to us | Link-out only, via web search |

Portfolio pages plus Form D are the strongest pair here: **what a firm funded is harder
evidence than what a partner said on a podcast**, and both are free and primary.

#### 8.1 The red flag needs a correction, and it is worth stating plainly

The user's framing was that no investor interest is a red flag. **We should report the
observation and decline that interpretation**, for three reasons:

1. **Most investor interest is private.** Public theses are a small, heavily-selected,
   marketing-shaped sample. Absence in public says little about absence in private.
2. **Not venture-shaped is not bad.** Most good businesses are not VC-fundable — wrong growth
   curve, wrong market size, wrong exit profile. A profitable business serving 400 restaurants
   is a success and will never attract a fund. Reporting "no VC interest" as a *red flag*
   silently adopts venture scale as the definition of a good idea.
3. **It would be incoherent coming from us.** This project's own roadmap explicitly rejects
   angel and VC funding and funds infrastructure from revenue. A bootstrapped product telling
   its users that a lack of VC interest is a warning sign argues against its own existence.

So the finding is scoped to what it actually measures:

> **Publicly stated investor interest: none found.** We searched 14 firm blogs, 6 podcast
> feeds and EDGAR Form D filings for this category since 2024-08. No public thesis mentions
> it, and no Form D filings match the category.
> **What this does and does not mean:** it means nobody has publicly staked out this space —
> useful if you are raising, since it means no fund is already committed to a competitor. It
> does **not** indicate the idea is weak. Most investor interest is never published, and most
> viable businesses are not venture-scale.

That paragraph is more useful than the red flag would have been, and it is defensible.

---

## 3. What this costs, and what it means for the tiers

Eight levels across a dozen venues does not fit in a 120-second pass on 24GB of shared ARM.
Being honest about that now avoids designing a product that cannot run.

| Levels | Pass | Why |
|---|---|---|
| 1, 4 (core), 5 (top signals) | **Pass 1** — ~120s | Highest value per token; mostly deterministic retrieval |
| 3 (has it been tried), 5 (full), 8 | **Pass 2** — 10–15 min queued | Search-heavy and slow, but cheap in *generation* |
| 2, 6, 7 | **Pass 2, generated last** | Depend on levels 1–5 as input; short outputs (questions, not prose) |

Two properties make this affordable, and both are already in the architecture:

- **The framing levels are the cheapest to generate**, because questions are short. The
  expensive part of levels 6–7 would have been reasoning we have decided not to do.
- **Discussion and investor sources are immutable and cache permanently.** A blog post read
  once never needs re-reading; a Form D filing never changes. Across many reports in the same
  category the marginal cost collapses — the second fintech report is far cheaper than the
  first.

**Anonymous users get all eight levels.** Restricting depth by tier would break the core
promise ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1) that the free report is not a crippled one.
The tiers differ in *volume, monitoring and history*, never in what a single report contains.

---

## 4. How the levels render

The report gains a **level index** at the top — eight rows, each showing status at a glance,
each linking to its section:

```
1  Problem space          ●  evidenced — 6 posts, 2 venues
2  Value proposition      ●  4 competitor claims, attributed
3  Idea as product        ◐  2 prior attempts found, both inactive
4  Idea as business       ●  3 companies, 11 sources
5  Discussions & reviews  ●  14 items · 3 venues not searched
6  Power niches           ?  3 questions raised
7  Startup trajectory     ?  2 questions raised — network effect likely
8  Investor interest      ○  none found publicly — see what this means
```

`●` evidenced · `◐` partial · `○` searched, nothing found · `?` questions raised, by design

**The `?` glyph is doing real work.** It tells the reader at a glance which parts of this
document are evidence and which are prompts for their own thinking — and it means levels 6–7
can never be mistaken for findings.

---

## 5. Export for the reader's own AI assistant

Most readers evaluating an idea already pay for a frontier chatbot, and the correct response
is to **feed it rather than compete with it.**

The report offers **"Copy as context"** — the full findings as clean Markdown with every
source URL and date, sized to paste into any assistant, with a suggested opening line:

> *Here is a public-evidence report on an idea I am considering. Every claim below has a
> source and a date. Argue with the framing questions in sections 6 and 7, and tell me what
> the evidence does not cover.*

This is close to free to build — the report is already structured Markdown with citations —
and it resolves the strategic question honestly. We are not a worse chatbot. We are the
**evidence file a chatbot cannot assemble**, and the reader's assistant is better with it than
without it. Levels 6 and 7 exist to hand that assistant the right questions, which is the one
part of the reasoning task retrieval genuinely improves.

---

## 6. Phasing

| Phase | Scope |
|---|---|
| **Phase 0** | Source-terms audit covers YouTube (100 searches/day, storage limits), X (pay-per-use, see §7), and RSS/robots posture. |
| **Phase 2** | Level 4 as today. Level 1 and level 5 from Hacker News + GitHub. The level index. |
| **Phase 3** | Blog and podcast RSS ingestion. Level 3 "has it been tried". Level 8 from firm blogs, portfolio pages and EDGAR. The absence panel. |
| **Phase 4** | Levels 2, 6, 7 — the framing questions. Ship last **deliberately**: they depend on levels 1–5 as input, and they are the ones that damage trust if rushed. "Copy as context." |
| **Deferred** | X, LinkedIn, Reddit beyond link-out. All three are commercially gated (§7). |

---

## 7. The honest limits

- **We do not score ideas.** No composite, no rating, no verdict. Eight levels of evidence and
  a set of questions; the judgement is the founder's. A score would be the most-clicked and
  least-defensible thing on the page.
- **We do not answer levels 6 and 7.** By design, permanently, for the reason in §1.
- **Three named venues are commercially closed to us.** X discontinued its free tier and
  charges per post read — at $0.005 a read, 200 posts costs $1, which is the entire monthly
  subscription price for a single report. LinkedIn forbids automated access. Reddit's
  commercial terms are out of reach ([DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4.3). All
  three are link-out only, and **none of them may appear in an absence panel**, because we
  have not searched them.
- **We do not reproduce the books we cite.** Named frameworks are pointers with one line of
  orientation. If a reader wants *Crossing the Chasm*, they should buy it.
- **English-language public web only**, and this constrains levels 1, 5 and 8 most.
- **Absence at every level follows the same rule** (DISCUSSION_SIGNALS §2): only reportable
  from venues where presence would be expected, always with queries shown, never interpreted.
