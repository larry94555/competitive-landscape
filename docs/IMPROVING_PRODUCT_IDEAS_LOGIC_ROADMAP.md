# Landscape — Improving the Product Ideas Logic

> **Every change recommended, in the order it should be made, with how many pull requests are
> left.** [`PRODUCT_IDEA_RESULTS_LOGIC.md`](PRODUCT_IDEA_RESULTS_LOGIC.md) describes what is
> true today; this describes what should change and how far along it is.
>
> **This is one plan, not a menu.** The four causes in §4 of that document are independent, and
> fixing any one alone still leaves a bad answer.
>
> **Review has corrected this plan three times.** The first draft proposed building a fit test
> and a reason that both turned out to exist; the second asserted a product-identity rule that
> breaks on the one URL it was written for. A plan written from a memory of a codebase rather
> than from reading it will do that, and the corrections are worth more than the plan was.

---

## 1. Where this stands

| | |
|---|---|
| **Pull requests in this plan** | **7** |
| **Done** | **4** |
| **Remaining** | **3** |
| **Complete** | **57%** |

**PR 1 is the two documents; PR 2 is the discovery golden set; PR 3 and PR 4 are the ones a
reader can see.** The percentage counts pull requests, not effort — PR 7 is larger than PRs 5
and 6 together.

**Two of the four causes are now closed.** A candidate is a product rather than a domain, and the
fit test is no longer one shared word. What is left is the ranking measuring appearance rather
than fit, and the first query being malformed.

**The baseline, measured rather than guessed:** mean recall **60%** across five fixtures, with
**1 impostor admitted**, scored end to end through `assemble` rather than at the ranking. Every
number in this plan from here is a move against those.

---

## 2. What is wrong

A reader typed *"project management for a small design agency"* and got **Microsoft** and
**projectplusgame.com**. A free general web answer, for the same words, returned five named
categories with a company list under each and a citation per category.

**The gap is not retrieval, it is legibility and fit.** Strip the free answer's categories that
are not competitors — *small design agencies* and *client businesses* are the reader's customers
— and its real answer is five household names with no prices, no dates and citations that are
mostly content-marketing blogs. It reads better because it is structured and instant.

So parity is a structure problem, which is mechanical and achievable. Beating it means doing the
thing an instant recalled answer structurally cannot: **a current price, from the vendor's own
page, with the date it was read.**

---

## 3. The plan

Each row is a pull request, and **each is independently mergeable and makes the decisions after
it safer.** That is the contract, and it is not the same as *each improves the answer*: PR 1 is
documents and PR 2 is measurement, and neither changes a single result. **PRs 3 to 7 are the ones
that change what a reader sees.** An earlier draft promised behavior from both prerequisites,
which is the kind of claim that makes a plan look further along than it is.

| PR | What | Where | Status |
|---|---|---|---|
| 1 | **This document and the logic document** | `docs/` | **Done** |
| 2 | **A golden set for discovery** | `landscape-golden::discovery` | **Done** |
| 3 | **Product-level candidates** — identity rule chosen by PR 2 | new `search::products` | **Done** |
| 4 | **Raise the fit test above one word** | `competitors::enough_words`, `assemble` | **Done** |
| 5 | **Candidates from page content** | new `candidates::named_in` | To do |
| 6 | **Render the reason that already exists** | `Report`, `web/src/App.tsx` | To do |
| 7 | **Breadth, and subcategories** | new `candidates::breadth`, the interface | To do |

**The comparison matrix is not in this list.** It is already a row in
[`Full_Feature_List.md`](Full_Feature_List.md) under S3, and it is the largest single thing that
would put this product ahead rather than level. It should follow PR 6 and does not depend on
PR 7.

---

## 4. Each change, in detail

### PR 2 — A golden set for discovery

**Nothing else in this plan can be judged without it.** There is a golden set for *extraction* —
ten subjects, scored against a model — and none for *discovery*. That is exactly why four causes
reached a reader.

Ten prompts, chosen to span the failure modes: a qualified description, a bare market name, a
name several products share, a market with one obvious leader, a market with none. For each, the
companies a careful person would expect, checked by hand, once.

**Fixtures first, live second.** Canned engine responses make it runnable on a laptop with no
`SEARX_URL` and in CI; an `--ignored` variant runs it against a real engine, exactly as
`against_a_model` does today.

**And one number this plan depends on:** what fraction of the six questions actually fill on ten
real companies. If it is low, better discovery delivers the right companies to an empty report,
and the four-to-eight-minute wait is unjustifiable either way.

#### What it measured, and the two things it corrected

| Fixture | Recall | Set aside, and why |
|---|---|---|
| `project-management-for-agencies` | 67% | workamajig.com, notion.so — uncorroborated |
| `one-product-many-urls` | 67% | airtable.com — uncorroborated |
| `keyword-impostor` | 50% | toggl.com — uncorroborated; trackingtimemusic.com — **caught by the fit test** |
| `specialist-in-one-article` | 67% | protemos.com — uncorroborated |
| `publisher-heavy` | 50% | helpscout.com — front page unreadable |

**Mean recall 60%. One impostor admitted.** **Seven companies were set aside.** One of them is
`trackingtimemusic.com`, correctly — the fit test caught it. Of the six that should have been in
an answer, **five are the same cause**: returned by a single query, `Uncorroborated`, never
reaching the reader. The sixth is `helpscout.com`, `Unread`.

**It corrected its author three times, before PR 3 was written.** That is the argument for the
sequencing, made by the sequencing:

1. I predicted `keyword-impostor` would score (2 found, 0 missed) and it scores (1, 1). `toggl.com`
   came back from one query. Cause 3, surfacing in a fixture written for cause 4.
2. **No path-shaped identity rule works** — see PR 3 below. I had leaned on
   *strip locales and containers*; it cannot tell Excel from Project, because `microsoft-365` is
   a **suite** and suite names are not a list anybody can finish.
3. **The first scorer stopped at the ranking**, so it could not see the stage PR 4 changes at
   all — and the fixture named here as the one that would judge PR 4 could not have moved.
   Scoring end to end says the fit test *already* excludes `trackingtimemusic.com`: one shared
   word is enough for a board game that mentions *project deadlines* and not enough for a drum
   machine. **So `keyword-impostor` no longer holds an impostor to raise the bar against**, and
   PR 4 needs a fixture that survives `SHARED_WORDS = 1` — which is now part of its scope
   below, rather than an assumption it would have inherited.

**Still to do in a later slice:** the six-question fill rate on ten real companies, which needs a
model and a network and is not fixture-able.

### PR 3 — Product-level candidates

**Two identities, not one.** The first draft of this said *"make the returned URL the
candidate"*, and review found the hole: corroboration is counted by grouping URL variants under a
registrable domain, so making raw URLs the candidates turns `/project`, `/project/pricing` and a
localized variant into **three one-query candidates**, each below `CORROBORATION = 2` and all
three refused. That change would have made the answer worse, not better.

A candidate needs two things kept apart:

| | What it is | Used for |
|---|---|---|
| **Identity** | A canonical key for *the product* — **not yet decided** | Merging appearances, counting agreement |
| **Evidence URL** | The shallowest URL seen for that identity | The page to fetch, and the link a reader follows |

**Agreement is counted per identity**, exactly as it is counted per domain today, so two URLs for
one product are one candidate that agreed with two queries rather than two that agreed with one
each. That part is settled. **What a product's identity *is* is not** — and the second draft of
this section asserted an answer that does not survive the example it was written for.

#### The identity rule is a hypothesis, and PR 2 selects it

The second draft said *"registrable domain plus the first path segment"*. Here is the URL it
exists to fix:

```
microsoft.com / en-us / microsoft-365 / project
                 ^^^^^   ^^^^^^^^^^^^   ^^^^^^^
                 locale  suite          the product
```

**The first segment is a locale.** That rule groups every Microsoft page under
`microsoft.com/en-us`, merges Project with every other Microsoft 365 product, and *splits* the
same Project across `/project`, `/microsoft-365/project` and any localized variant. It reproduces
the corroboration failure it was written to prevent. Review caught it.

**So the candidates are listed, and the golden set chooses between them:**

| Rule | Fails when |
|---|---|
| Domain + first path segment | The first segment is a locale (`/en-us/`) or a container (`/products/`) |
| Domain + first segment after stripping known locale and container prefixes | The prefix list is a guess, and a missing one fails silently |
| Domain + **last** path segment | A pricing or docs page splits away from the product page |
| Domain + the product name read from the page | Needs the page *before* the merge, inverting the order the pipeline runs in |
| Domain + a canonical link or `og:title` the page declares | Only as good as what sites declare, which is uneven |

### PR 2 ran them, and none of the path-shaped rules works

`landscape_golden::discovery::Identity` implements four of the five and the fixtures score them.
The result is not *"one is best"*:

| Rule | Joins one product's pages? | Separates two products in a suite? |
|---|---|---|
| Domain (today) | Yes | **No** — this is cause 2 |
| First path segment | **No** — splits on locale | **No** — groups on locale |
| First meaningful segment | Yes | **No** — both key to `microsoft.com/microsoft-365` |
| Last path segment | **No** — splits `/excel` from `/excel/pricing` | Yes |

**Each one either merges two products or splits one.** That is a result about the approach rather
than a threshold to tune, and it says the answer is the fifth candidate: **the vendor's domain
plus the identity the page declares about itself** — a canonical link, an `og:title`, the product
name in its own words. That cannot be decided from a URL, so PR 3 has to fetch before it merges,
which inverts the order the pipeline runs in today.

**The domain half is not decoration, and leaving it out was a real defect.** `Identity::declared_for`
first keyed on the declared name alone, and review found what that does: two vendors who both
call their product *Invoicing* become one company. **That is a wider failure than the
domain-collapse it replaces** — this one crosses a vendor boundary. `specialist-in-one-article`
now holds the pair, and the rule is scored on four cases, every one of them a page of a URL the
fixtures' own queries returned:

| Case | Two locales of one product | Two products of one suite | A product and its pricing page | One name, two vendors |
|---|---|---|---|---|
| Required | joined | apart | joined | apart |

**PR 3 is therefore larger than this plan first said, and its shape is now known rather than
assumed.** The assertions are in `tests/discovery.rs`; a rule that ever passes both columns of
the table above will fail that test and have to be argued for.

#### What shipped, and what it moved

`landscape_search::products` reads the pages behind the results and regroups each domain by what
its pages call themselves. **The rule now lives in the crate it judges and the golden set
delegates to it**, because a scorer holding its own copy of a rule measures itself.

| | Before | After |
|---|---|---|
| The reported failure's first company | **Microsoft**, 3 of 3 | **Asana** |
| What Microsoft is called | *Microsoft* | ***Microsoft Project***, 2 of 3 |
| `one-product-many-urls` | *Microsoft*, *Google* | ***Microsoft Excel***, ***Google Sheets*** |
| Mean recall | 60% | 60% |
| Impostors admitted | 1 | 1 |

**Recall did not move and was never going to.** Cause 2 is about what a company is *called* and
what its agreement is *for*, and every host in the answer is the same host. That is why `Scored`
now records the ordered set and the name of every member: a scorecard of recall and impostors
would have reported this change as doing nothing at all.

**Three guards, each with a test that fails without it:** a domain returned at its own root is
never split, an unreadable page can never split anybody, and a domain costing more than
`SPLIT_BUDGET = 4` extra reads is left whole rather than split on half its evidence.

**And a rule for the root, which is safe whichever wins.** When the evidence URL is the domain
root — `asana.com` — the company and the product are one thing and one name is right. Only a
non-root URL can produce *Microsoft Project, by Microsoft*.

**It also fixes attribution downstream.** A price labeled `microsoft.com` is true and useless.

*Aimed at cause 2, and no longer claimed to remove it until a rule is chosen by measurement. The
merge is the work, the fetch is the easy part, and the identity is an open question.*

### PR 4 — Raise the fit test above one word

**This test exists and is set to its weakest possible value.** The first draft of this roadmap
proposed building it; review found it built. `describe` records which of the market's words a
candidate's front page uses, `assemble` excludes on fewer than `SHARED_WORDS`, and the exclusion
reaches the reader as `Aside::ElsewhereEntirely`: *"its own front page uses none of the words
this comparison is built on"*. The machinery is right. The number is **1**.

`projectplusgame.com` uses *project*. The prompt contains *project*. One word is the bar, so it
passed.

**The work is the test, not the plumbing:**

**It needed a fixture first, and needed it twice over.** `keyword-impostor` was written to be
this PR's judge and, measured, admitted no impostor at all. Worse, its market was three words
wide — and the rule that shipped asks for half the market's words, so a three-word market asks
for one, exactly what it asked for before. **The fixture named as this PR's judge could not have
moved either way.** It now describes *"time tracking for independent consultants"* and holds
`timezonecheck.com`, a world clock whose page shares `time` and nothing else: it passes a bar of
one and fails a bar of two.

#### Four rules, scored

`landscape_golden::discovery::Fit` implements all four and the fixtures score them, in the shape
PR 3's identity comparison took:

| Rule | Impostors admitted | Real companies lost |
|---|---|---|
| One word (what ran before) | **2** | 0 |
| A flat two | 0 | **2** — microsoft.com, intuit.com |
| Half the market, rounded up | 0 | **1** — intuit.com |
| **Half the market, rounded down** | **0** | **0** |

**The losses are the point.** A flat two asks a two-word market for all of it, and *"# Microsoft
Excel / The spreadsheet."* is a terse front page rather than a company in another market.
Rounding down never asks a short description for all of itself, and a four-word market is the
first that has to give two.

**Five fixtures is a small set and this number is fitted to them.** Said in the code, in the
logic document and here, because the next person to move it should know what it rests on.

*Removes cause 4. The cheapest change in the plan, and the one most easily got wrong by guessing
at a threshold — which is why it came after the measurement, and why the measurement had to be
widened before it could judge anything.*

### PR 5 — Candidates from page content

**The reframe, and the largest idea in this plan.** Today the search engine is the source of
companies, and the answer is *which domains came back most often*. It should be the way to find
**the market's literature**, with the companies coming from what those pages say.

A company enters because **two independent hosts name it in this market** — the same
independence rule `FACT_CHECKING.md` §L6 already applies to claims. `NOT_A_COMPANY` inverts from
a filter that discards our best sources into an index of them: still never candidates, now read
as evidence.

*Removes cause 3, and most of cause 1 — a malformed query matters far less when the answer comes
from what the returned pages say rather than from which domains they were.*

### PR 6 — Render the reason that already exists

**The reason already exists, on both paths.** `Because::Found { agreed, asked, shares }` is
built for described candidates and reads *"3 of the 3 searches returned it, and its own front
page uses 'project'"*. `Aside` has five variants and every exclusion is one of them —
`Uncorroborated`, `Unconvincing`, `ElsewhereEntirely`, `Unread` and `BeyondTheFetchBudget`; see
[`PRODUCT_IDEA_RESULTS_LOGIC.md`](PRODUCT_IDEA_RESULTS_LOGIC.md) §3.5 for the whole table.
Review corrected the first draft here too, which had this as extending a seeded-only type.

**So the work is carrying it to the page.** `Because` reaches the CLI and the report's notes; it
does not reach the four blocks. Put it on `Report` beside each subject, and render it under each
company with the date the page was read.

**This is the last place something is asserted without its evidence being shown.** The claims
inside a report are quoted, dated and cited. The reason a company is in the set is computed,
correct, and invisible.

### PR 7 — Breadth, and subcategories

Ask a breadth question before shopping for vendors — *types of X*, *X categories*, *X for Y* —
read those pages, and decide whether this is one market or several.

```
a category is OFFERED when
    it is named on >= 2 independent hosts
    and it has >= 1 candidate company under it

the prompt is TOO GENERAL when
    >= 2 categories clear that bar
    and no single category holds > 60% of the candidates
```

Both thresholds are starting points and must be labeled as such, exactly as
`MINIMUM_CONFIDENCE` and `AMBIGUITY_MARGIN` are. When it is several, the reader is offered the
categories and picks — reusing the ambiguity chips exactly as they are, each carrying a whole
prompt so a click is a new run with its own URL.

*Removes the rest of cause 1.*

> **This one is a research problem, and should be built last and measured hardest.** Deciding
> that a question has several answers, from pages, without a model inventing the categories, is
> not solved by anything in this repository today. The thresholds above are a hypothesis. If PRs
> 3 to 6 land and the answer is good, this may turn out to be worth less than it looks.

---

## 5. What this will not do

**It will not be fast.** Four to eight minutes against an instant answer is the trade this
product makes, and no item here changes it. That trade is only worth taking if what comes back is
something an instant answer cannot give — which is why PR 6 and the comparison matrix matter more
than they look.

**It will not write better prose.** The local model is a 4B extractor. Any design that needs it
to summarize or invent structure loses to a frontier model, which is why every rule in §4 counts
what pages say rather than asking the model what it thinks. See
[`PRODUCT_IDEA_RESULTS_LOGIC.md`](PRODUCT_IDEA_RESULTS_LOGIC.md) §5.

**It does not fix empty sections.** If PR 2's yield number is low, that is a different problem
with its own fix, and it is upstream of whether any of this is worth doing.
