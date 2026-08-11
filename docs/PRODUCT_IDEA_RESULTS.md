# What a reader sees when the analysis finishes

> **This document owns the results page.** It says exactly what appears after **Analyze** is
> pressed, in what order, and how each part behaves when a reader touches it. Anything not
> described here does not belong on that page.

---

## 1. Why this document exists

The page currently answers a question nobody asked. A finished run renders six sections of
claims — *Pricing & packaging*, *What it does*, *Recent public changes*, *Company facts*,
*Trust & security posture*, *Where they are investing* — each with quotes, source labels,
confidence markers and coverage notes, for every company in the set.

Every one of those is true, checkable, and hard-won. Together they are **the wrong first
screen**. A reader who typed an idea forty seconds ago wants to know *what is already out
there*. Instead they get a comparison matrix's worth of prose about companies they have not yet
decided they care about.

**The six questions are a deeper dive, and they were never a first step.** They are what a
reader asks *about a company they have already decided matters* — what does it cost, what does
it do, what has it changed lately. Asking all six of a company nobody has chosen yet is asking
a stranger their salary.

**And a question with no answer beside it is worse than no question.** Six headings, most of
them followed by *"nothing found in public sources"* and a list of what was checked, is a page
of apologies. The honest-negative treatment is right — `PRODUCT_SPEC.md` §4.3 argues for it and
this document does not touch it — but it belongs where somebody went looking for that fact, not
in front of somebody who has not yet seen a single name.

**The detail is not being deleted. It is being demoted.** The full report becomes a downloadable
PDF and a browsable detail view. The first screen becomes an answer to the question that was
actually asked.

See the shape: [`prototype/results-mockup.html`](../prototype/results-mockup.html), with
invented data and working interactions.

---

## 2. The page, in order

Exactly four blocks, in this order, and nothing else.

| | Block | Answers |
|---|---|---|
| 1 | What you asked | *"Did it understand me?"* |
| 2 | How that was interpreted | *"Is it looking for the right thing?"* |
| 3 | The count | *"Is there anything here?"* |
| 4 | The three lists | *"What is already out there?"* |

---

### 2.1 What you asked

The reader's own words, verbatim, **clamped to two lines**.

| | |
|---|---|
| Fits in two lines | Shown whole. Nothing else happens |
| Longer than two lines | Truncated at the end of the second line with a trailing `…` |
| Hovering the text | The full text appears as a tooltip |
| Clicking the `…` | The block expands in place to the full text. Clicking again collapses it |

**Two lines is a measurement, not a character count.** A clamp counted in characters is wrong at
every window width, and this page is read on a phone. The `…` appears only when the text is
actually cut.

**The tooltip and the click are not alternatives.** A tooltip is unavailable to a reader on a
touch screen and to a reader using a keyboard; the expand is what serves them. Both exist and
both show the same text.

---

### 2.2 How that was interpreted

One line, in the product's own voice:

> **Here's how I interpreted the business idea:** *competitive intelligence software*

The phrase is the one every query was actually built from — `Report::searched_as`, already
carried today. Beside it, two links:

| Link | Does |
|---|---|
| **Edit** | Lets the reader correct the phrase and run again with it |
| **Why this?** | Explains how the phrase was arrived at |

**This block is the most important thing on the page and the least obvious.** If the
interpretation is wrong, everything below it is wrong, and the reader is the only one who can
tell. `COMPETITIVE_DISCOVERY.md` §4 already argues that showing the substitution beats asking a
clarifying question first — this makes it correctable rather than merely visible.

---

### 2.3 The count

One sentence:

> **I found 12 companies, 7 open source projects, and 31 discussions on this topic.**

Plain numbers. A category with nothing found is stated as zero rather than omitted — *"and 0
discussions"* is a finding, and a missing clause is an ambiguity. `PRODUCT_SPEC.md` §4.3 and
`FACT_CHECKING.md` §5.4 both already require that an absence say so.

---

### 2.4 The three lists

Three blocks with identical mechanics and different contents.

| | Holds up to | Shows at first |
|---|---|---|
| **Here are the companies** | 25 | 5 |
| **Here are the open source projects** | 25 | 5 |
| **Here are the discussions that I found** | 25 | 5 |

Each block with more than five items ends with **`…more`**. Clicking it reveals the rest, up to
the cap of 25. There is no pagination beyond that: 25 is the whole of what this page will ever
show, and anything past it belongs in the full report.

**A block with nothing in it says so** and does not render an empty list.

#### Ordering

**Companies** and **open source projects** keep the order the pipeline already produces — the
scored order from `landscape-search::candidates`, best-supported first.

**Discussions are ordered differently, and this is the part that needs care.**

1. **Authority tier first.** Discussions from highly authoritative venues fill the 25 before any
   lower-tier venue is considered. A high-authority venue with 25 discussions leaves no room for
   a lower one, by design.
2. **Recency within a tier.** The most recent first, so the five a reader sees without clicking
   are the five most recent from the most authoritative venue that has any.

**Tier is a property of the venue, not of the post.** What defines a tier is not yet decided —
see [§4](#4-dependencies).

---

## 3. What this page does not show

Removed from the first screen. None of it is deleted; all of it moves.

| No longer on the first page | Where it goes |
|---|---|
| The six claim sections and their quotes | The detail view, and the PDF |
| Per-claim confidence and source labels | The detail view, and the PDF |
| *"Nothing found in public sources"* coverage blocks | The detail view, and the PDF |
| The sources index | The detail view, and the PDF |
| Run notes — capped subjects, skipped pages, budget | The detail view |
| **Copy as context** | Stays, but below the four blocks rather than among them |

**The full report is a downloadable PDF**, and the detail is browsable from the results page
without the reader having to download anything. Both are out of scope for this document and get
their own; this document is only the first screen.

---

## 4. Dependencies

**Two of the four blocks cannot be built today**, and one more is partly available. This section
is the honest accounting of what is missing, because a specification that assumes data the
pipeline does not produce is a wish rather than a plan.

### 4.1 Not built at all

| Capability | Needed by | State today |
|---|---|---|
| **Open source project discovery** | *Here are the open source projects*, and its share of the count | **Nothing exists.** GitHub appears in [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4.2 as a *signal* source and §3.3 as *"what people are building"* — not as a first-class result category with its own list |
| **Discussion discovery** | *Here are the discussions*, and its share of the count | **Nothing exists.** Specified in full in [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md); listed in [Full_Feature_List.md](Full_Feature_List.md) as S3, *"What people are saying — Hacker News and GitHub only"*, **2 PRs, 0 done** |
| **Venue authority tiering** | Ordering the discussions | **Does not exist, and is not the model already decided.** See §4.3 |
| **A downloadable PDF** | Everything §3 moves out of the first screen | Listed in [Full_Feature_List.md](Full_Feature_List.md) as S3, *"PDF export — Typst executive and full templates"*, **2 PRs, 0 done** |
| **A browsable detail view** | Everything §3 moves out of the first screen | Nothing exists. Today the first screen *is* the detail |

### 4.2 Partly available

| Capability | Needed by | State today |
|---|---|---|
| **The interpreted phrase** | §2.2 | **Available.** `Report::searched_as` and `Report::interpreted` carry the phrase, the alternatives that recurred, and how many independent hosts used it |
| **Why this interpretation** | The *"Why this?"* link | **The material exists, the explanation does not.** `Interpreted` holds `also` and `hosts`; nothing renders an account a reader can argue with |
| **Editing the interpretation** | The *"Edit"* link | **The pattern exists for a different thing.** `EditableSet` lets a reader correct the *company set* and re-run. Correcting the *phrase* is the same shape and does not exist |
| **A company list of up to 25** | §2.4 | **Capped far lower today.** `subject::MAX_SUBJECTS` limits a run to a handful of companies because each one is its own discovery, fetches and model calls. A list of 25 **named** companies is a much cheaper thing than 25 analyzed ones, and the distinction has not been drawn in the code |

### 4.3 A decision this document cannot make

**The ordering rule in §2.4 contradicts one already written down.**
[DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §6 ranks discussions by **specificity,
recency, corroboration, and engagement relative to venue** — explicitly rejecting raw volume —
and caps each block at **5 items**, calling the cap *"a product decision, not a performance
one"*.

This document asks for **venue authority first, then recency**, and a cap of **25**.

These are different products. The first surfaces *the most useful thing anyone said*; the second
surfaces *the most credible venues talking about this*. Both are defensible and they produce
different lists from the same data.

**This needs deciding before either is built**, and whichever wins, the other document must be
corrected rather than left standing — an instruction and its rationale are one fact in two
places, and this repository has already paid for that once.

Two further questions fall out of it, neither answered:

- **What makes a venue authoritative?** Domain age, editorial process, whether the venue is
  primary or aggregating, an explicit allow-list maintained by hand? An automatic measure that
  nobody can inspect is the kind of hidden judgment [FACT_CHECKING.md](FACT_CHECKING.md) exists
  to refuse.
- **Reddit.** [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4.3 marks it *restricted, and this
  needs a decision before any code is written*. For many subjects it is where the discussion
  actually is, so a discussions block that silently omits it is misleading about its own
  coverage.

### 4.3a The examples were never shown to produce values

**This is the assumption that should have been checked and was not**, and it is a dependency
because everything above assumes the pipeline can fill a page.

`ROADMAP.md` §2·D item D4 asks for example ideas that *really run*, and
[BENCHMARKS.md](BENCHMARKS.md) Run 22 is the check that was supposed to prove it. Its table has
a column headed **"Questions answered"** reading *"all six"* for four of the six companies.

Its own header reads: **"Model: none — this is discovery alone."**

So *"answered"* there means **a page was found and admitted for that question**. Nothing was
extracted, because nothing could be: no model ran. Discovery finding a pricing page and a model
pulling a number off it are different events, and the check measured the first while its column
name claimed the second. A reader of that table — including whoever wrote the next thing on top
of it — would reasonably conclude the examples had been shown to produce answers.

**What is actually unmeasured:** how many of the six questions yield a *claim* for a curated
example company, on the deployed box, with the model running. That number could be six and it
could be one. Nobody knows, and the first screen this document specifies is the wrong place to
find out.

### 4.4 What the search channel can and cannot reach

The channel today is one SearXNG instance, reached through `SEARX_URL`, returning general web
results. That is enough to find **companies**, which is what it was built for.

It is **not** obviously enough for the other two:

| | Why the general channel may not be enough |
|---|---|
| Open source projects | Finding repositories by topic is a query against a code host's own index — stars, language, activity, last commit. A web search returns pages *about* repositories as readily as repositories |
| Discussions | Venue coverage, post dates, comment counts and thread structure are all API-shaped. A web result gives a URL and a title, and the ranking rules above need more than that |

**Whether to add per-venue adapters or to push harder on the general channel is undecided**, and
it should be decided by trying the general channel against real subjects first — this repository
has a rule about running the real thing before designing around a limit it has not measured.

---

## 4.5 Open issues

Each of these blocks something in this document. None has an answer yet, and they are listed so
that a decision is made rather than assumed by whoever writes the code.

| # | Open issue | Blocks |
|---|---|---|
| **1** | **Discussion ordering.** Venue authority first, or specificity and corroboration first? See §4.3 — two written rules disagree | The discussions list, and the cap of 25 against 5 |
| **2** | **What makes a venue authoritative.** Domain age, editorial process, primary against aggregating, or a hand-maintained list? | Tiering, and whether the tier can be shown to a reader as a reason |
| **3** | **Reddit.** [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) §4.3 marks it restricted and needing a decision before any code | Whether the discussions block is honest about its own coverage |
| **4** | **Where open source projects come from.** A code host's API, or the general web channel? | The projects list, and its share of the count |
| **5** | **What counts as a project worth listing.** Stars, recent commits, whether it is a library or a product, whether an archived repository still counts | Ordering the projects list, and the count |
| **6** | **How many companies can be named without being analyzed.** The list wants 25; `subject::MAX_SUBJECTS` caps a *run* far lower because each company is its own discovery and model calls | The companies list at 25 |
| **7** | **What "Why this?" actually shows.** `Interpreted` holds the phrase, the alternatives and the host count — an account a reader can argue with has to be written | The interpretation block |
| **8** | **Whether the examples produce values at all.** See §4.3a — unmeasured, and the check that claimed it measured something else | Every list on the page, and whether this redesign is treating a symptom |
| **9** | **What the detail view is.** A page per company, a single scrolling report, an expander under each list row? | Everything §3 moves out of the first screen |

**Issue 8 is the one to settle first.** If a curated example yields one claim in six on the
deployed box, the problem this document is solving is a symptom and the cause is upstream — in
extraction, in the model, or in what discovery admits. A better first screen would still be
worth having, and it would be lipstick.

---

## 5. What can be built now

Honestly, from the four blocks:

| Block | Buildable today |
|---|---|
| §2.1 What you asked | **Yes, wholly.** The prompt is on the analysis |
| §2.2 How that was interpreted | **The line and the Edit link, yes.** *Why this?* needs an explanation surface |
| §2.3 The count | **Only the company count**, and a sentence with two thirds of it missing is worse than no sentence |
| §2.4 The three lists | **Companies only** |

So the first honest increment is: **the reader's words, the interpretation with an edit, and the
companies list** — with the count sentence held back until it can name all three, and the other
two lists absent rather than empty. An empty *"Here are the open source projects"* block would
be this product claiming it looked, which is the one thing it must never do.
