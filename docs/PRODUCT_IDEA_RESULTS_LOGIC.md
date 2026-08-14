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
| 5 | Group hits by domain | `candidates::from_results` | A first grouping |
| 6 | Read the pages a search returned | `products::split` | **What a candidate is** |
| 7 | Score and rank | `candidates::score` | **Which candidates win** |
| 8 | Fetch the front pages still unread | `candidates::from_hits` → `describe` | What each is called, **and its words** |
| 9 | Exclude on shared words | `competitors::assemble` | **Whether it is in this market** |
| 10 | Admit or refuse | `subject::decide` | Whether to report at all |
| 11 | Read each company | `landscape_analyze::analyze_many` | The six questions |

**Step 6 is new, and it is why step 5 is no longer the answer to *what a candidate is*.**
Grouping by domain made *Microsoft* one candidate that three queries agreed about; reading the
pages those queries returned makes it *Microsoft Project*, agreeing with the two that returned
Project. See §3.2.

**The ranking still works on countable things.** Agreement across queries and the depth of a URL
are the whole of it; nothing the engine *said* affects the order. What changed is **what the
agreement is counted for**.

**Step 9 is the only place a page is read for fit**, and it is a lexical test set to one shared
word — §3.4. So it is not true that the pages are discarded, and an earlier draft of this
document said so: they are read, once, for a name and a weak overlap check. What **is** discarded
is everything else the engine returned — titles beyond the market label, every snippet, and the
whole contents of the review pages that were dropped as candidates.

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

**A product, keyed on `domain#the name its page declares`.** `from_results` still groups by
registrable domain first — that is what makes `/excel`, `/excel/pricing` and a localized variant
one thing rather than three uncorroborated ones. `products::split` then reads the pages those
URLs point at and regroups by what each page calls itself, so `microsoft.com/`—`/project/`— and
`microsoft.com/`—`/teams/`— are two products and only the strongest becomes the candidate.

```
SPLIT_BUDGET = 4      extra page reads per analysis, spent in rank order
```

**Four rules were tried before this one and every one of them failed** — each either merges two
products or splits one; `BENCHMARKS.md` Run 51 has the table. The name a page declares is not a
URL rule, which is why it works and why it costs a read *before* the grouping.

**A page is grouped as one of three things, and only the middle one is a product:**

| An appearance at | Keys to | Because |
|---|---|---|
| The domain's **root** | The vendor | A front page says what the *company* is, not which product |
| A page that **declares a name** | `domain#name` | That is the identity rule |
| A page that declares **nothing** | The vendor | Nothing about it says it is a different product |

**And four things keep it from doing harm:**

| Case | What happens |
|---|---|
| Fewer than **two products** | Nothing to separate. The vendor keeps the agreement it had, named by its own front page when a query returned one. `freshbooks.com/` plus `freshbooks.com/invoice` is one company. |
| Two products with **equal support** | A question, not a choice. The vendor is kept, at the support the tie shows, with no product claimed — never the one whose heading sorts first. |
| A page that **could not be read** | Declares no identity, so it keeps the vendor as its key. An unreadable page can never split a candidate. |
| A domain costing more than the **budget** left | Left whole. Half a split attributes the unread appearances to whichever product was fetched first. |

**Agreement is counted once per query per product**, exactly as it was once per query per domain.
A query that returned a product page and its pricing page corroborated that product once.

**It does not put two products of one domain in the same report.** The strongest becomes the
candidate; the rest stop corroborating it. Two rows for one vendor needs `Candidate` to stop
being keyed on a domain, which is a larger change than this one.

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

### 3.4 What a candidate is called, and the fit test that already exists

`describe` fetches the **front page of the domain**, reads a name and a one-line description from
it, **and records which of the market's words that page uses** — `Vocabulary::Read(shared(...))`.

`competitors::assemble` then excludes any candidate whose page shares fewer than `SHARED_WORDS`
of them, as `Aside::ElsewhereEntirely`, and says so in the reader's words: *"its own front page
uses none of the words this comparison is built on"*.

```
SHARED_WORDS = 1     one content word is enough to be admitted
```

**One word is the whole test**, and it is why `projectplusgame.com` is in the answer: its page
uses *project*, the description contains *project*, and one is the bar. So the fit gate is not
missing — it is a lexical test set to its weakest possible value.

**An earlier draft of this document said the page was consulted only for a name.** That was
wrong, and review caught it. It matters: *there is no test* and *there is a test that admits
anything sharing one word* have different fixes, and only the second is true.

### 3.5 The reason a company is in the set, which already exists

`Because::Found { agreed, asked, shares }` is built on the described path and reads:

> *"3 of the 3 searches returned it, and its own front page uses ‘project’"*

`Because::Named` covers a company the reader typed. **`Aside` has five variants, and every
exclusion is one of them:**

| Variant | Why it was left out |
|---|---|
| `Uncorroborated { agreed, asked }` | Fewer than `CORROBORATION` queries returned it |
| `Unconvincing` | It scored below what is worth putting in a report |
| `ElsewhereEntirely { looked_for }` | Its front page shares none of the market's words |
| `Unread` | Its front page could not be fetched, so nothing could be checked |
| `BeyondTheFetchBudget { budget }` | It ranked below `NAMED`, so no page was requested |

**`Uncorroborated` is the one §4's cause 3 is about.** A specialist named in a single
specialist article is excluded by *that* variant, and an earlier draft of this section listed
three of the five and left that one out — in the paragraph nearest the case it explains.
Review caught it.

**Every company in the set already carries a countable reason, and every one left out carries the
reason it is not there.** What is missing is that
[`PRODUCT_IDEA_RESULTS.md`](PRODUCT_IDEA_RESULTS.md)'s page does not render any of it.

### 3.6 The gate

`subject::decide`, with constants from two crates — which this document got wrong once, and
which matters because it promises where every rule lives:

```
landscape_core::subject
    MINIMUM_CONFIDENCE = 0.35     below this, nothing is resolved
    AMBIGUITY_MARGIN   = 0.15     two candidates this close is a question, not a choice

landscape_search::competitors
    DESCRIBES_A_MARKET = 2        one word that several products share is ambiguity,
                                  not a market
    SHARED_WORDS       = 1        the fit test in §3.4
    CORROBORATION      = 2        in `candidates`, and the floor `Uncorroborated` reports
```

**The gate is not the problem.** It refuses correctly and it will not guess between two close
candidates — but nothing above it can help when the ranking it is fed is confident and wrong.

### 3.7 The three silences, kept apart

`Vocabulary::Read | Unreadable | NotRequested` distinguishes *the page said nothing about this
market*, *the page could not be fetched*, and *this candidate ranked too low to be fetched at
all*. That distinction exists and is enforced; §4 relies on it.

---

## 4. Where this produces a wrong answer, and why

**Four independent causes, one of them now fixed.** Fixing any one alone still leaves a bad
answer, which is why the plan is seven pull requests rather than one.

| | Cause | Symptom |
|---|---|---|
| 1 | The first query is malformed and the qualifier is dropped | The answer is about a much broader market than the one asked about |
| 2 | ~~A domain, not a product, is the unit~~ **Fixed, §3.2** | **Microsoft**, 3 of 3, named from `microsoft.com`. Now *Microsoft Project*, 2 of 3, named from a page a query returned |
| 3 | Ranking measures appearance, not fit | Household names win; the right specialist scores 0.175 and is refused |
| 4 | The fit test is one shared content word | **projectplusgame.com** — its page uses *project*, and `SHARED_WORDS = 1` |

**Cause 4 is a threshold, not an absence**, which is the correction review made to this
document. The machinery to exclude a company for being in another market is built, tested, and
reports itself honestly. It is set to admit anything that shares a single word with the prompt.

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
