# Landscape — Reasoning Framework Rubrics

> Curated, versioned rubrics that let a small local model apply well-known business
> frameworks to retrieved evidence — as **a starting point for beginners and a reminder for
> experts**, never as a definitive guide.
>
> This document exists because of a specific finding in [IDEA_ANALYSIS.md](IDEA_ANALYSIS.md)
> §1: on levels 6 and 7, an 8B model loses a reasoning contest with the reader's own frontier
> chatbot. The rubrics below narrow that gap by changing what the model is asked to do.

---

## 1. Why this works — and what it does not fix

A quantized 8B model asked *"apply Crossing the Chasm to this idea"* produces a hazy,
half-remembered rendition of a book it absorbed thinly. The weakness is **recall fidelity**,
not logic. It cannot reliably reconstruct a framework it only partly encoded, and it will
state the reconstruction confidently.

Give it the framework explicitly, in context, and the task changes shape:

| | The task becomes | Model requirement |
|---|---|---|
| **Without a rubric** | Recall a book, then reason from the recollection | High-fidelity parametric memory — **the thing small models lack** |
| **With a rubric** | Match observed evidence against stated criteria, emit the questions that fire | Classification and extraction — **the thing small models do well**, and which our grammars already constrain |

This is the same move the product already makes with facts: **replace recall with
retrieval.** We do not ask the model what Linear's pricing is; we fetch the pricing page and
have the model read it. We should not ask the model what Christensen argued; we should hand
it the criteria and have it match.

**What this genuinely buys:**

- **Consistency.** The same evidence produces the same questions every run. A chatbot's
  answer varies with phrasing and luck.
- **Auditability.** The rubrics are plain files in git. A human can read the reasoning
  scaffold, disagree with it, and send a pull request. **Nobody can inspect or correct a
  frontier model's latent understanding of a book** — this is a real and underrated
  advantage.
- **Testability.** Rubrics run against golden cases in the eval suite like any other prompt
  asset.
- **Cost.** Matching against criteria is short-output work. It fits the generation budget in
  a way that essay-writing does not.

**What it does not buy, and this stays true:**

> Rubrics do not make us as good as a frontier model at strategic reasoning. They make us
> **reliable at a narrower thing**: raising the right questions, consistently, with the
> evidence that raised them attached.

So the output contract from [IDEA_ANALYSIS.md](IDEA_ANALYSIS.md) §1 is unchanged. Rubrics
generate **questions with evidence**, never conclusions. What they add is that the questions
are now the *right* questions, drawn from frameworks that have earned their standing, rather
than whatever the model half-recalls.

---

## 2. Attribution and copyright — the posture, and it is not optional

These frameworks are other people's work. The project's public-data ethos applies to ideas as
much as to pricing pages.

**The line, stated plainly:**

| We may | We must not |
|---|---|
| Use the **idea** — methods and systems are not copyrightable | Reproduce the author's **prose**, structure, or diagrams |
| Write **our own** diagnostic questions derived from the concept | Write a chapter-by-chapter summary |
| **Name and attribute** the framework and its author | Imply endorsement or affiliation |
| **Link to the book** and to the author's own free writing | Produce something that **substitutes for reading it** |

**The substitution test governs everything here:** if a reader could skip the book because our
rubric covers it, the rubric is too long. Each one is a **single page of orientation and a
pointer**, and it should leave a reader wanting the book rather than feeling done with it.

Conveniently, the design decision that produced this document also resolves the legal
question. **Questions in our own words, derived from a concept, use the uncopyrightable idea
rather than the protected expression.** Summaries would have been both worse product and
worse citizenship.

**Every rubric ships with:**
- The author, the title, the year
- A link to the book
- A link to the author's own freely published writing where it exists — Christensen's HBR
  articles and Andrew Chen's essays are both free and better than any summary we could write

Pointing readers at the source is good citizenship *and* it is the honest thing: they will get
more from the original than from us.

---

## 3. Rubric format

Each rubric is a versioned file consumed as a prompt asset. The format exists so that adding
a framework is a documentation change, not a code change.

```yaml
id: cold-start
name: The Cold Start Problem
author: Andrew Chen
year: 2021
applies_to_levels: [6, 7]

trigger: >
  One sentence: when does this framework apply at all?

signals:                    # observable in OUR retrieved evidence — this is the crucial field
  - id: two_sided_copy
    description: Competitor materials describe recruiting two distinct populations
    evidence_kind: competitor_positioning
  - id: empty_side_complaint
    description: Reviews or posts describe joining and finding the other side absent
    evidence_kind: discussion_signal

fires_when: any_of [two_sided_copy, empty_side_complaint]

questions:                  # our own words, emitted only when the rubric fires
  - Which side do you solve first, and what do you do for them while the other side is empty?
  - What is the smallest group — one city, one campus, one trade — where this is already useful?
  - What does the product do for user number one, before any network exists?

caution: >
  The failure mode this framework warns about, in one sentence.

reading:
  book: "Andrew Chen, The Cold Start Problem (2021)"
  free: "andrewchen.com — the author's essays, free"
```

**The `signals` field is what makes this work.** A rubric never fires on the model's opinion
that a framework "feels relevant." It fires on **observable evidence already retrieved by
levels 1–5**, and every emitted question carries the source labels of the signals that fired
it. That is what keeps levels 6–7 inside the product's evidence discipline instead of
floating free of it.

---

## 4. The rubrics

Written in our own words. Each is deliberately short.

### 4.1 The Innovator's Dilemma — Clayton Christensen (1997)

**Trigger:** established players serve demanding customers well, and a cheaper or simpler
entrant is *not good enough* for those customers today.

**Signals we can observe:** incumbents priced high with large feature sets; complaint themes
about cost, complexity, or paying for unused capability; competitor pages dismissing a
cheaper alternative; reviews saying the product does far more than the reviewer needs.

**Questions it raises:**
- Who are you deliberately *worse* for, and are you comfortable saying so out loud?
- What can incumbents do that you cannot — and why is that acceptable to the customer you want?
- What is your path upmarket, and what would stop an incumbent following you down?
- Is there something structural — their cost base, their sales model, their existing
  customers — that makes serving your customer unattractive to them rather than merely
  unnoticed?

**What it warns against:** assuming incumbents are asleep. Usually they are rationally
ignoring a segment that would damage their margins. That is a far more durable advantage than
complacency, and a far more fragile one to rely on if it is not actually true.

**Read:** *The Innovator's Dilemma*. Christensen's Harvard Business Review articles cover the
core argument and are freely available.

### 4.2 Crossing the Chasm — Geoffrey Moore (1991)

**Trigger:** a novel product that must move from enthusiasts to a mainstream buyer who wants
a safe, complete, referenceable purchase.

**Signals we can observe:** visible enthusiasm in developer or hobbyist venues with no
mainstream reviews; competitors targeting enterprise while complaints originate from small
operators; products described as capable but hard; a category with vocabulary that has not
settled.

**Questions it raises:**
- What is your beachhead — narrow enough that you could be the obvious choice within it?
- What does a mainstream buyer need that you do not yet ship: integrations, support,
  onboarding, someone to call?
- Who does that buyer ask before purchasing, and what would that person need to have seen?
- Which reference customer would make the next ten sales easy?

**What it warns against:** reading early-adopter enthusiasm as traction. The people who love
a rough product are systematically unlike the people who will pay for a finished one.

**Read:** *Crossing the Chasm*.

### 4.3 The Cold Start Problem — Andrew Chen (2021)

**Trigger:** value depends on other participants — two-sided markets, marketplaces,
communities, collaboration tools.

**Signals we can observe:** competitor copy recruiting two distinct populations; reviews
describing signing up and finding the other side empty; complaints of the form *"nobody in my
area"*; competitors launching city by city or vertical by vertical.

**Questions it raises:**
- Which side do you solve first, and what do you offer them while the other side is empty?
- What is the smallest network that is already useful — one city, one campus, one trade
  association?
- What does the product do for the very first user, before any network exists?
- What brings someone back in week two, before density arrives?
- Where does the network stop adding value, and what happens then?

**What it warns against:** launching broadly. Networks need density in a small space before
they need reach, and a thin national launch is usually worse than a dense local one.

**Read:** *The Cold Start Problem*. Andrew Chen's essays at andrewchen.com are free and
substantial.

### 4.4 The Tipping Point — Malcolm Gladwell (2000)

**Trigger:** adoption is expected to spread socially rather than through direct selling.

**Signals we can observe:** virality or referral mechanics in competitor materials; discussion
volume driven by a small number of high-visibility posters; category growth described as
word-of-mouth.

**Questions it raises:**
- Who carries this to other people, and what do they get from doing so?
- Is the thing memorable enough to be retold accurately by someone who is not you?
- What context makes someone more likely to adopt — a moment, a season, a trigger event?

**What it warns against:** treating virality as a plan. It is a property some products have
and most do not, and it is rarely engineered after the fact.

**An honest caveat we ship with this rubric:** the specific claim that a small number of
exceptional individuals drive most social epidemics has been **contested by later empirical
work**, notably Duncan Watts'. The framework remains a useful prompt for thinking about
diffusion; it should not be treated as settled science. Saying so is the same discipline we
apply to every other source.

**Read:** *The Tipping Point*, alongside Watts' critique, which is easy to find and worth the
hour.

### 4.5 Niche strategy — narrowing to win

**Trigger:** a crowded category, or a founder proposing to serve a broad market from a
standing start.

**Signals we can observe:** several competitors describing near-identical target customers;
complaint themes clustering in a customer type nobody names as their target; discussion
volume in a sub-segment with no product addressing it.

**Questions it raises:**
- If you narrowed to one customer type, one geography, or one workflow, could you be the
  obvious choice rather than an option?
- What do the underserved complainers have in common, and is that group reachable as a group?
- What would you have to stop doing to serve them properly?
- Is the niche large enough to sustain the business you want — and if it is not, is it a
  beachhead or a dead end?

**What it warns against:** narrowing to a segment that is genuinely too small, which is the
symmetric error to serving everyone badly and gets far less attention.

> **Attribution note — needs the founder's confirmation.** This rubric was requested as
> *"Power Niche."* We could not confidently identify a single canonical book by that title, so
> this rubric is written from the general niche-strategy concept and **ships without a book
> attribution rather than guessing at one**. If a specific title was meant, name it and the
> attribution goes in. Inventing a citation would violate the rule the rest of this project
> is built on.

### 4.6 Two frameworks already implied elsewhere

Recorded here so the set stays coherent:

- **Blue Ocean Strategy** (Kim & Mauborgne) — the report's existing *Market emphasis* section
  ([COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md) §6) is a strategy canvas,
  which is their tool. That section should attribute it.
- **Jobs to be Done** (Christensen and others) — level 1's insistence on stating the problem
  in *sufferers' own words* rather than vendor language is this idea already. Level 2's
  value taxonomy is adjacent to it.

---

## 5. How the model consumes them

Consistent with the existing architecture: grammar-constrained, schema-fixed, evidence-linked.

1. **Signal matching runs first**, cheaply. The resident 1.7B router checks each rubric's
   signals against the structured output of levels 1–5. It sees extracted fields, not raw
   pages — so this step costs almost no prefill.
2. **Only fired rubrics proceed.** A typical idea fires one or two of six.
3. **Question emission is grammar-constrained** to the rubric's own question list, lightly
   contextualised with the subject's specifics. The model **cannot invent a question that is
   not in the rubric**, which is what makes the output auditable.
4. **Every question carries the source labels** of the signals that fired it.

Two consequences worth being explicit about:

- **The model cannot apply a framework we have not written down.** That is a feature. It
  bounds the output to reasoning a human has reviewed.
- **A rubric that fires wrongly is a bug with a fix.** The signal was too loose; tighten it
  and the eval suite catches the regression. Compare with a frontier model reaching a bad
  strategic conclusion, where there is nothing to fix.

---

## 6. How this renders

Framing sections stay visually distinct from evidence sections
([IDEA_ANALYSIS.md](IDEA_ANALYSIS.md) §4, the `?` glyph):

```markdown
## 7. Startup trajectory                          ? questions, not findings

### This looks like a network-effects problem
Because: two of three products describe recruiting farms and restaurants
separately [S3, S7], and one review describes signing up and finding no
farms listed [S15].

Worth answering before you build:
  · Which side do you solve first, and what do you offer them while the
    other side is empty?
  · What is the smallest network that is already useful — one city, one
    trade association?
  · What does the product do for user number one, before any network exists?

These questions come from Andrew Chen's The Cold Start Problem (2021),
which is written about exactly this situation. We have not summarized it —
if this is your situation, read it. His essays at andrewchen.com are free.

A starting point if this is new to you; a checklist if it is not. Not advice.
```

The closing line is fixed boilerplate on every framing section. It sets the expectation the
user asked for — **beginner's starting point, expert's reminder** — and it is the honest
description of what a rubric-driven question list actually is.

---

## 7. Limits

- **Six rubrics is not a strategy education.** They cover common situations, not all of them.
- **A rubric can fire on a coincidence.** The signals are heuristics over noisy evidence.
  Every fired rubric shows its evidence so the reader can dismiss it in five seconds.
- **We never rank or score which framework matters most.** If two fire, both render.
- **We do not tell the reader what to do.** No rubric emits a recommendation, and none ever
  will — that is the [IDEA_ANALYSIS.md](IDEA_ANALYSIS.md) §1 line and it does not move.
- **Rubrics are opinions about which questions matter**, written by us, reviewable in git.
  They are not neutral, and the document says so rather than presenting them as objective.
