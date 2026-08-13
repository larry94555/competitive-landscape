# Landscape — How a Product Idea Becomes a Set of Companies

> **This document owns the logic.** [`PRODUCT_IDEA_RESULTS.md`](PRODUCT_IDEA_RESULTS.md) owns what
> the finished page *shows*; this owns how the thing it shows is *decided* — which companies a
> description resolves to, and why each one is there.
>
> **Change this document first.** Every rule below is implemented in one named place, and every
> constant has its value here. A change to the approach that does not start here produces two
> answers to the same question, which is the mistake this repository has a register entry about.

---

## 1. Why this document exists

A reader typed *"project management for a small design agency"* and got two companies:
**Microsoft** and **projectplusgame.com**. Both are wrong, in different ways, and neither is a
bug in the sense of a line that was mistyped — each falls out of a design decision that is
reasonable read alone.

The logic had never been written down in one place. It lived in `candidates.rs`, `vocabulary.rs`,
`competitors.rs` and `subject.rs`, each of which documents its own part well and none of which
describes the whole. So the shape of the answer — *a domain, ranked by how many queries returned
it* — was never a decision anybody made; it was the sum of four local ones.

[`IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md`](IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md) is what
is being done about it. This is what is true today.

---

## 2. The pipeline, as it runs

| | Step | Implemented in | Decides |
|---|---|---|---|
| 1 | Read the prompt | `landscape_analyze::subject::subjects_in` | Description, seed, or named set |
| 2 | Ask three templated queries | `landscape_search::candidates::ask` → `for_idea` | What is asked of the engine |
| 3 | Find the market's own words | `landscape_search::vocabulary::from_titles` | Whether to substitute |
| 4 | Ask three more, if substituted | `candidates::ask` | The second round |
| 5 | Group hits into candidates | `candidates::from_results` | **What a candidate is** |
| 6 | Score and rank | `candidates::score` | **Which candidates win** |
| 7 | Fetch the top five | `candidates::from_hits` → `from_its_own_page` | What each is called |
| 8 | Admit or refuse | `subject::decide` | Whether to report at all |
| 9 | Read each company | `landscape_analyze::analyze_many` | The six questions |

**Steps 5 and 6 decide the answer**, and both work on URLs alone. Everything the engine actually
said — titles, snippets, the pages themselves — is used to count votes for domains and then
discarded. That is the single most important fact in this document.

---

## 3. The rules that hold today

### 3.1 The queries

`IDEA_TEMPLATES` is three strings, interpolated with the reader's words and nothing else:

```
best {} software
{} tools comparison
{} vendors
```

**All three are vendor-shopping phrasings.** None asks what kind of thing this is, who the buyer
is, or how the market divides — so no amount of reading the results can recover structure that
was never asked for.

**The first is not always English.** With a qualified description the result is *"best project
management for a small design agency software"*, which an engine answers by falling back to its
strongest keywords. The qualifier that made the prompt specific is the first thing lost.

### 3.2 What counts as a candidate

**The registrable domain.** `from_results` reduces every hit's URL to its registrable domain and
groups by that. `microsoft.com/microsoft-365/project` and `microsoft.com/microsoft-teams` become
one candidate.

**Known publishers are dropped.** `NOT_A_COMPANY` lists review sites, forums and blog hosts —
`g2.com`, `capterra.com`, `reddit.com`, `medium.com` and the rest. A raw IP address is dropped
too. This is right as far as it goes: they are not companies. **Their contents are never read.**

### 3.3 How a candidate is scored

Two inputs, both countable from a URL:

| Input | Meaning |
|---|---|
| **Agreement** | How many of the queries returned this domain — counted once per query, not per hit |
| **Depth** | How deep the shallowest URL was; a front page scores better than an article |

```
CORROBORATION = 2        below this, score is MINIMUM_CONFIDENCE / 2 = 0.175, which the gate refuses
NAMED         = 5        how many candidates get their page fetched
```

**Countable on purpose.** A reader asking *why is this first* gets arithmetic rather than a
shrug, which was the point. **What it measures is breadth of appearance, not fit** — a household
name in every adjacent listicle agrees with everything; a specialist named in one article scores
below the floor and is refused before anybody sees it.

### 3.4 What a candidate is called

`from_its_own_page` fetches the **front page of the domain** and reads a name and a one-line
description from it. The page is not consulted about whether the company belongs in this market.

### 3.5 The gate

`subject::decide`, with `landscape_core::subject`'s constants:

```
MINIMUM_CONFIDENCE = 0.35     below this, nothing is resolved
AMBIGUITY_MARGIN   = 0.15     two candidates this close is a question, not a choice
DESCRIBES_A_MARKET = 2        one word that several products share is ambiguity, not a market
```

**The gate is not the problem.** It refuses correctly and it will not guess between two close
candidates — but nothing above it can help when the ranking it is fed is confident and wrong.

### 3.6 The three silences, kept apart

`Vocabulary::Read | Unreadable | NotRequested` distinguishes *the page said nothing about this
market*, *the page could not be fetched*, and *this candidate ranked too low to be fetched at
all*. That distinction exists and is enforced; §4 relies on it.

---

## 4. Where this produces a wrong answer, and why

**Four independent causes.** Fixing any one alone still leaves a bad answer.

| | Cause | Symptom |
|---|---|---|
| 1 | The first query is malformed and the qualifier is dropped | The answer is about a much broader market than the one asked about |
| 2 | A domain, not a product, is the unit | **Microsoft**, scoring 3 of 3, named from `microsoft.com` |
| 3 | Ranking measures appearance, not fit | Household names win; the right specialist scores 0.175 and is refused |
| 4 | Nothing verifies a candidate is in the market | **projectplusgame.com**, clearing `CORROBORATION` on keywords alone |

**And the best sources are discarded.** Every page a general web answer cites for this question —
review sites, agency blogs — is on `NOT_A_COMPANY`. They are correctly refused *as candidates*
and never read *as evidence*, which is where the categories, the comparisons and the reasons
live. The information needed for a better answer is already arriving and is thrown away at step 5.

See [`BENCHMARKS.md`](BENCHMARKS.md) Run 50 for the full trace.

---

## 5. The rules any change must keep

These are not negotiable, and a proposal that cannot meet them is a proposal to change what this
product is.

**Nothing may be asserted that was not read.** A category has to come from pages we fetched, not
from a model's memory. A product name has to come from that product's own page. A reason has to
quote something. This is `FACT_CHECKING.md` applied to the *choice* of company rather than to a
claim about one.

**A silence must say which silence it is.** *We looked and found nothing*, *we could not look*
and *we did not look* are three different findings, and the reader acts on each differently.

**Every input to a ranking must be countable and explicable.** Not because arithmetic is better
than judgment, but because *why is this first* has to have an answer.

**The local model does not synthesize.** It is a 4B extractor scored by the golden set, and any
design that needs it to summarize or invent structure is a design that will lose to a frontier
model. Structure must come from counting what pages say.

---

## 6. How to change this

1. **Edit this document first**, in the same pull request as the code.
2. Add a row to [`IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md`](IMPROVING_PRODUCT_IDEAS_LOGIC_ROADMAP.md)
   or tick one off, so the plan and the state agree.
3. **Run the discovery golden set before and after.** A change to §3 that is not measured is the
   same kind of change as the one that produced Microsoft.
4. Record the numbers in [`BENCHMARKS.md`](BENCHMARKS.md).
