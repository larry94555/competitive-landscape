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

Four blocks, in this order, and nothing else. **Blocks 1 and 2 always render. Blocks 3 and 4
depend on whether anything could be looked for at all** — see
[§2.5](#25-when-a-source-could-not-be-reached).

| | Block | Answers | Always? |
|---|---|---|---|
| 1 | What you asked | *"Did it understand me?"* | Yes |
| 2 | How that was interpreted | *"Is it looking for the right thing?"* | Yes |
| 3 | The count | *"Is there anything here?"* | Yes, though its wording changes |
| 4 | The three lists | *"What is already out there?"* | One heading per category that was searched |

**When every source failed**, the page is blocks 1 and 2, then one sentence in place of the
count — *"I could not reach any of the sources for this. Nothing below is a finding about your
idea."* — and **no fourth block at all**: no headings, no empty lists.

**When some failed**, every category that was *searched* keeps its heading, whatever it found. A
category that could not be searched keeps its heading too, with the line saying so, because a
missing heading is indistinguishable from a feature that does not exist. **Only the all-failed
case removes the block**, and it removes the whole of it rather than leaving three apologies
where three lists were.

An earlier draft of this section said *"exactly four blocks and nothing else"* without
qualification, which contradicted §2.5 one screen later — two owning rules producing two
different pages, and an implementer forced to pick which to break.

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

**It is never `Report::searched_as`.** An earlier draft named that field and review caught it:
`searched_as` holds the *origins* — `origins.join(", ")` on the multi-company path and the single
origin on the other — and the interface already renders it separately as *"Searched as
basecamp.com, linear.app"*. Following it would have produced *"Here's how I interpreted the
business idea: https://basecamp.com"*, and an **Edit** that let somebody rewrite a domain as
though it were a market phrase.

**And it is not one field either.** A second draft said the phrase is `interpreted.label` full
stop, which is right for one of the three cases below and unbuildable for another — the
unchanged-description row exists *precisely* when `interpreted` is `None`. The source is per
case:

| The reader gave | The phrase shown is | Where it comes from |
|---|---|---|
| A description, substituted | The market's words | `interpreted.label` |
| A description, unchanged | The reader's own words | The analysis prompt, or a field added to carry it — see [§4.5](#45-what-the-report-does-not-carry) |
| One or more names | The names | The subject set |

Beside it, two links:

| Link | Does |
|---|---|
| **Edit** | Lets the reader correct the phrase and run again with it |
| **Why this?** | Explains how the phrase was arrived at |

**This block is the most important thing on the page and the least obvious.** If the
interpretation is wrong, everything below it is wrong, and the reader is the only one who can
tell. `COMPETITIVE_DISCOVERY.md` §4 already argues that showing the substitution beats asking a
clarifying question first — this makes it correctable rather than merely visible.

#### Three cases, and the first question is not the one about substitution

An earlier draft of this section read `interpreted: None` as *"the reader named companies"*.
**It does not mean that**, and review caught it. `interpreted` is `None` whenever **nothing was
substituted**, and a reader who types a phrase the market already uses — *competitive
intelligence software* — takes the description path, has those exact words searched for,
produces a discovered set, and stores `None`. Telling them *"You named these directly"* would
be false about the one thing they are being asked to check, and would point **Edit** at a set
they never wrote.

**Two independent facts decide this block**, and they must be read in this order:

| | Where it comes from |
|---|---|
| **What class of thing the reader gave** | `subject::subjects_in` — `Describe`, `Seed(one)`, or `Exactly(several)` |
| **Whether their words were replaced** | `interpreted`, which is `Some` only on the describe path and only when `substituted` was true |

| The reader gave | Substituted | The block says | **Edit** acts on |
|---|---|---|---|
| A description | Yes | **Here's how I interpreted the business idea:** *the phrase*, with **Why this?** | The phrase |
| A description | No | **I searched for your words as you wrote them:** *the words* | The phrase |
| One or more names | — | **You named these directly:** the names | The **set**, with the machinery `EditableSet` already has |

**The middle row is the one that was missing**, and it is not a rare case: a reader who already
knows their market types its name. They interpreted nothing, so there is nothing to justify and
no *"Why this?"* — but the phrase is still what every query was built from, so it is still the
thing they may want to change.

**A reader who typed `basecamp.com` has not had an idea interpreted** and is not shown a block
implying they did.

---

### 2.3 The count

One sentence:

> **I found 12 companies, 7 open source projects, and 31 discussions on this topic.**

Plain numbers. A category with nothing found is stated as zero rather than omitted — *"and 0
discussions"* is a finding, and a missing clause is an ambiguity. `PRODUCT_SPEC.md` §4.3 and
`FACT_CHECKING.md` §5.4 both already require that an absence say so.

**"Found" is a claim about provenance, and it is false for two of the three input classes.**
`Subjects::Exactly` hands the named domains straight through — `(named, None)`, no discovery of
any kind — and `Seed` supplies one company and discovers the rest. Saying *"I found 3
companies"* to somebody who typed all three is the product taking credit for reading a list.

| The reader gave | The companies clause reads |
|---|---|
| A description | *"I found 12 companies"* |
| One name | See below — the rival search has three outcomes |
| Several names | *"You named 3 companies"* — and no discovery number, because there was none |

Projects and discussions are always discovered, whatever the reader typed, so their clauses are
unaffected.

#### One name, and what happened to the search for its rivals

**The seed is never in doubt; the rivals are.** `competitors::NoRivals` has four variants, and
they are four different facts:

| Variant | What actually happened |
|---|---|
| `NoEngine` | **Nothing was asked.** No search engine is configured, so nothing off the company's own site was reachable |
| `NothingToCompare(why)` | **Nothing was asked.** The company's own page gave no words to judge a rival against |
| `SearchIncomplete { failed, sent, sought, fault }` | Some searches went out and some did not come back |
| `NobodyHeldUp { sought }` | **Every search ran.** What came back did not hold up — the only one of the four that is a statement about the world |

**The page does not write these sentences.** `NoRivals::sentence()` already does, it already
carries the reason *and* the remedy, and it is what the tests assert. Restating it here would put
the wording in two places and let them drift — and the second copy would be the one nobody
re-reads. So the page composes: **the seed clause it owns, then the pipeline's own sentence.**

| What the rival search returned | The clause |
|---|---|
| Rivals, every search finished | *"You named basecamp.com, and I found 11 more like it"* |
| Rivals, `SearchIncomplete` | *"You named basecamp.com, and I found **at least** 4 more like it."* + `NoRivals::sentence()` |
| `NobodyHeldUp` | *"You named basecamp.com."* + `NoRivals::sentence()` |
| `NothingToCompare` | *"You named basecamp.com."* + `NoRivals::sentence()` |
| `NoEngine` | *"You named basecamp.com."* + `NoRivals::sentence()` |

**The bottom three rows must not share a sentence, and an earlier draft of this table gave two
of them one** — four lines below a paragraph saying the page must not collapse what the pipeline
keeps apart. The distinction is not decorative: **configuring a search engine fixes `NoEngine`
and cannot fix `NothingToCompare`**, whose seed page had nothing quotable on it. One is a
deployment problem and the other is a fact about that company's website, and a reader who is
told the wrong one goes and does the wrong thing.

`SearchIncomplete` carries a `Fault` for the same reason, and `Fault::advice()` is why a reader
whose engine refuses every query permanently is not told to try again.

**The mixed case maps to the same coverage number §2.5 uses:** searches attempted against
searches answered — `failed` and `sent`, already on the variant — which is what makes
*"at least"* honest rather than a hedge.

---

### 2.4 The three lists

Three blocks with identical mechanics and different contents.

| Heading | Holds up to | Shows at first |
|---|---|---|
| **Companies** | 25 | 5 |
| **Open source projects** | 25 | 5 |
| **Discussions** | 25 | 5 |

**The heading is the noun and nothing else.** *"Here are the…"* three times running is the same
three words asking to be read three times, and once the heading carries real weight the noun
does the work on its own. The count sits at the other end of the header rule as a chip, because
it is a fact about the list rather than part of its name.

Each block with more than five items ends with **`…more`**. Clicking it reveals the rest, up to
the cap of 25. There is no pagination beyond that: 25 is the whole of what this page will ever
show, and anything past it belongs in the full report.

**The count and the cap are different numbers and both are shown.** If 31 discussions were
found, the count sentence says **31** — that is what was found — and the `…more` control names
what it can actually reveal, **20**, not the 26 that are not on screen.

**And the "showing" line counts what is on screen now, not what will be.** Before the click it
reads *"Showing 5 of 31"*; after it, *"Showing 25 of 31"*. A line that says 25 while five rows
are visible is false in the state a reader spends most of their time in, which is the one before
they touch anything.

A control or a count that promises more than is there is the same defect as a progress bar that
retreats: checkable by the person reading it, and wrong.

**A block with nothing in it says so** and does not render an empty list.

**Each heading is a rule, not a line of prose** — set large, closed off from its rows by an
accent rule, with the count at the far end. The three lists are what a reader scans for, so
finding them must not require reading them. See
[`prototype/results-mockup.html`](../prototype/results-mockup.html).

#### Ordering

**Companies are ordered by where they came from**, and one of the three orders is not ours to
choose:

| The reader gave | Order |
|---|---|
| A description | The scored order from `landscape-search::candidates`, best-supported first |
| One name | The named company first, then the discovered rivals in scored order |
| Several names | **Exactly the order they wrote**, never re-sorted |

**The third is an instruction, not a default.** `Subjects::Exactly` is documented as *"exactly
these, in the order written"*, and somebody comparing `basecamp.com vs linear.app` has put the
one they care about first. Re-scoring that list would silently overrule them in the one place
they were most explicit.

**Open source projects have no order yet, and this document must not invent one.** An earlier
draft said projects kept the `candidates` order too. That function ranks *company* candidates
from URL shape and cross-query agreement; it never produced a repository and cannot be the
existing order for a category that does not exist. What ranks a project — stars, recent
commits, whether it is a library or a product, whether an archived repository still counts — is
[open issue 5](#47-open-issues), and **nothing should be built against a guess about it**.

**Discussions are ordered differently, and this is the part that needs care.**

1. **Authority tier first.** Discussions from highly authoritative venues fill the 25 before any
   lower-tier venue is considered. A high-authority venue with 25 discussions leaves no room for
   a lower one, by design.
2. **Recency within a tier.** The most recent first, so the five a reader sees without clicking
   are the five most recent from the most authoritative venue that has any.

**Tier is a property of the venue, not of the post.** What defines a tier is not yet decided —
see [§4](#4-dependencies). The rule itself is stated under the discussions heading, in a line a
reader can check the list against: *"Most authoritative venues first, most recent within
each."*

---

### 2.5 When a source could not be reached

**Four states per category.** *Found some and finished*, *found some and did not finish*, *found
none*, and *could not look* are four different findings. The first draft of this section had
three of them; the second had the table below and this sentence still saying three, which is the
document disagreeing with itself in the space of a paragraph.

That distinction is not new here. A company search already reports `nothing_found`,
`search_incomplete` and `search_refused` as separate outcomes, because only one of them is
fixed by trying again and only one is fixed by changing the words. With three independent
channels — the web, a code host, and whatever reaches discussions — one of them being down while
the others answer is the *ordinary* case rather than the edge one.

| State | The count sentence | The block |
|---|---|---|
| Found some, and the search finished | *"12 companies"* | The list |
| **Found some, and the search did not finish** | *"at least 12 companies"* | The list, and one line naming what did not come back |
| Found none, and the search finished | *"0 open source projects"* | The heading, and one line: *"Searched, and found none."* |
| Could not look at all | The clause is **replaced**, not zeroed | The heading, and one line saying which source and what a reader can do |

**Four states, because a non-empty answer is not the same as a complete one.** An earlier draft
of this section had three and treated anything non-empty as finished — which reintroduces, one
row down, the exact ambiguity the section was written to remove.

**Partial success is reachable today, not just with future adapters.**
`resolve_from_description` records the queries that failed, and it can return the companies the
*successful* queries found. A bare *"12 companies"* over that is a definite number about an
indefinite search: the thirteenth may not exist, or may be behind the query that timed out, and
the reader cannot tell which.

*"At least 12"* costs one word and is true. The line beneath it says what was missed — *"2 of
8 searches did not come back"* — because a reader deciding whether to re-run needs to know
whether re-running would plausibly change the answer.

Worked through, with the projects channel down and the other two fine:

> **I found 12 companies and 31 discussions on this topic. I could not reach the code host, so
> I have not looked for open source projects.**

**A zero is a claim that we looked.** Rendering *"0 open source projects"* during a code-host
outage is the product asserting an absence it never established, which is the single thing
`FACT_CHECKING.md` §5.4 and `PRODUCT_SPEC.md` §4.3 exist to prevent — and it is worse here than
in a report section, because a count carries more authority than a paragraph.

**Omitting the clause is also wrong**, and it is what §2.3's original rule forced: a reader who
sees two categories named cannot tell whether the third found nothing, was not searched, or
does not exist as a feature. The clause has to be *replaced* by a sentence that says what
happened.

**Every category failing is not a special case.** It reads: *"I could not reach any of the
sources for this. Nothing below is a finding about your idea."* — and the three blocks are
absent rather than empty.

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

**Three of the four blocks cannot be built today**, either because nothing produces the data or
because the finished report does not carry what the page needs to describe it honestly. This
section is the accounting, because a specification that assumes data the pipeline does not
produce is a wish rather than a plan.

Two whole result categories have no pipeline ([§4.1](#41-not-built-at-all)); the interpretation
block and the count both need facts the report never carries
([§4.5](#45-what-the-report-does-not-carry)); and whether the pipeline yields values at all is
unmeasured ([§4.4](#44-the-examples-were-never-shown-to-produce-values)). [§5](#5-what-can-be-built-now)
states what survives all of that.

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
| **The interpreted phrase, when one exists** | §2.2, first row | **Available.** `Report::interpreted` carries `label` — the phrase every query was built from — with the alternatives that recurred and how many independent hosts used it. **Not `searched_as`**, which carries the origins and none of the vocabulary |
| **The phrase when nothing was substituted** | §2.2, second row | **Not carried as a phrase.** `interpreted` is `None` there by design, and the prompt is the nearest thing — which stops being the same thing the moment a prompt also names companies. See [§4.5](#45-what-the-report-does-not-carry) |
| **Which class of thing the reader gave** | §2.2, §2.3, §2.4 | **Not carried at all.** `subjects_in` runs in the worker and only the prompt survives — [§4.5](#45-what-the-report-does-not-carry) |
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

### 4.4 The examples were never shown to produce values

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

### 4.5 What the report does not carry

**This document now asks the page to know two things the finished analysis cannot tell it**, and
that is a dependency rather than an implementation detail. Review found it after the rest of the
section had been called an honest accounting, which it was not.

| The page needs | Where it lives today | Reaches the client? |
|---|---|---|
| The input class — `Describe`, `Seed`, `Exactly` | `subjects_in(&analysis.prompt)`, evaluated **inside the worker** | **No.** Only the prompt survives |
| The reader's unchanged phrase, when nothing was substituted | The prompt | Only as the raw prompt, which is not the same thing once a prompt names companies |
| How many searches did not come back | `read.queried.failed`, **logged and dropped** — `Read` returns `decided` and `interpreted` and nothing else | **No** |

**Both fixes are the same shape: stop deriving in the client what the worker already knew.**
Re-running `subjects_in` in TypeScript would put a business rule in two languages, which is the
mistake this repository has a register entry about; and no amount of client-side parsing can
recover a query that failed inside the worker an hour ago.

So the contract has to carry them. The smallest honest version:

- **the input class**, as a field on the report, set where `subjects_in` is already called;
- **discovery coverage** — how many searches were attempted and how many answered — which is
  the same number §2.5's *"at least 12"* needs, and which `Read` currently throws away.

**Until that exists, §2.3's per-class wording and §2.5's partial state cannot be built at all**,
and §5 says so rather than implying they are a rendering exercise.

### 4.6 What the search channel can and cannot reach

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

### 4.7 Open issues

**11. A partial search says how much was missed and not what to do about it.** `Searches` carries
two counts, so the page can say *"2 of 8 searches did not come back"* and stop there. Which of
three things happened — the engine refused, asked us to slow down, or never answered — is what
decides whether trying again is worth anything, and `landscape_search::competitors::NoRivals`
already keeps those apart. It reaches the page only through `Report::notes`, and only when the
set has **one** member: `alone_because` returns `None` as soon as there is a comparison, so a
seeded search that found rivals *and* dropped queries has the count and no remedy.

**Carrying it is a contract change rather than a rendering one** § `Searches` is counts, and the
fault is an enum with a reader-facing sentence attached. It is named here rather than
half-built, because collapsing *refused*, *rate-limited* and *never answered* into one sentence
is the un-making this project has already paid for three times.

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
| **8** | **Whether the examples produce values at all.** See §4.4 — unmeasured, and the check that claimed it measured something else | Every list on the page, and whether this redesign is treating a symptom |
| **9** | **What the detail view is.** A page per company, a single scrolling report, an expander under each list row? | Everything §3 moves out of the first screen |
| **10** | **The report contract.** The page needs the input class and discovery coverage; the finished analysis carries neither — [§4.5](#45-what-the-report-does-not-carry). Deciding the shape is the first piece of work, before any of the four blocks | §2.2's second and third rows, §2.3 entirely, §2.4's ordering, §2.5's partial state |

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
| §2.2 How that was interpreted | **The substituted case only.** The other two rows need the input class, which does not reach the client — §4.5. *Why this?* needs an explanation surface besides |
| §2.3 The count | **No.** Even the company clause needs the input class to avoid claiming it *found* companies the reader named, and the partial state needs discovery coverage — neither is on the report |
| §2.4 The three lists | **The companies list, in scored order, for a description.** Named sets need the input class to keep the reader's order |

So the first honest increment is smaller than the last draft of this section claimed, and it
starts one step earlier:

1. **Extend the report contract** — input class and discovery coverage (§4.5). Without it, three
   of the four blocks are either unbuildable or buildable only in a way that misstates
   provenance.
2. **Then**: the reader's words, the interpretation, and the companies list in the right order
   for how they were arrived at.
3. **The count sentence stays out** until it can name all three categories, and the other two
   lists are **absent rather than empty**. An empty *"Open source projects"* block would be this
   product claiming it looked, which is the one thing it must never do.

**The contract work is not a prerequisite somebody can skip and revisit.** Every shortcut around
it — re-parsing the prompt in the browser, inferring the class from whether `interpreted` is
set, treating any non-empty result as complete — is a business rule in a second place or a
claim about a search nobody watched. Both are on the list of things this repository has already
paid for once.

---

## 6. What was built, and where it departs from §5

Steps 1 and 2 above shipped together. `landscape_core::given` carries `Given` — the input class,
set where `subjects_in` is already called — and `Searches` — how many searches were sent and how
many came back. Both are on `Report`, both reach the browser, and every sentence on the page that
depends on provenance reads them rather than re-deriving anything.

**On both discovery paths, which took a second pass.** The first version populated the coverage
where a *description* was resolved and not where a *named company's* rivals were searched for,
so a seed whose search finished completely was still hedged as *"at least"*. The arithmetic now
lives once, in the crate that owns the query counts, rather than at each caller.

**And *"nothing was asked"* is decided there too, not at each caller.** `of_company` returns
before it sends anything when the seed's own page gave no vocabulary, so a configured engine
that was never asked a question would otherwise report a coverage of nought out of nought — a
search that came back empty, which is a finding about the market rather than about us. Review
found the same run serializing two different ways depending only on whether an unused engine
happened to be set. `Queried::coverage()` returns `None` whenever `sent()` is zero.

**[§2.5](#25-when-a-source-could-not-be-reached)'s line beneath the hedge is built.** *"At least
12"* says the number is soft; *"2 of 8 searches did not come back"* says whether re-running
would plausibly change it, and one failed search out of eight and six out of eight are the same
word and very different decisions. The first version of this page shipped the word without the
line, which is this document's own requirement unimplemented.

**Step 3 shipped differently, and this section is the reason.** §5 said the count sentence stays
out until it can name all three categories. `prototype/results-mockup.html` — which is the
instruction this page was built to — shows the sentence and all three headings. Rather than pick
one, the page says what is true of each category separately:

> I found 12 companies. **I have not looked for open source projects or discussions — neither
> search is built yet.**

and the two categories keep their headings with a **not built** chip and one line in place of a
list.

**This is §2.5's *could not look* state, one step further out.** That row was written for a
source that was unreachable on this run; these two were never reachable at all. Both are facts
about us rather than about the reader's market, and both are worse than useless as a zero — *"0
open source projects"* is a claim that we looked. The distinction §5 was protecting — never
show an empty list — is intact; what changed is that the headings stay, because a missing
heading is indistinguishable from a feature that does not exist, and a reader cannot ask for
what they cannot see is missing.

### 6.1 Still outstanding

| | Why |
|---|---|
| **Why this?** on the interpretation | Needs an explanation surface. [Open issue 3](#47-open-issues) |
| **Edit** on the interpretation | `EditableSet` edits the *set*; editing the *phrase* is a different affordance and a different re-run |
| The separate detail view and the downloadable report (§3) | [Open issue 9](#47-open-issues). Until they exist the six claim sections sit behind a disclosure on the same page — off the first screen, and nothing thrown away |
| Both missing pipelines | [§4.1](#41-not-built-at-all) |
| **The remedy for a partial search** | [Open issue 11](#47-open-issues) |

