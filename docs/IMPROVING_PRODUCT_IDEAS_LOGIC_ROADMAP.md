# Landscape — Improving the Product Ideas Logic

> **Every change recommended, in the order it should be made, with how many pull requests are
> left.** [`PRODUCT_IDEA_RESULTS_LOGIC.md`](PRODUCT_IDEA_RESULTS_LOGIC.md) describes what is
> true today; this describes what should change and how far along it is.
>
> **This is one plan, not a menu.** The four causes in §4 of that document are independent, and
> fixing any one alone still leaves a bad answer.

---

## 1. Where this stands

| | |
|---|---|
| **Pull requests in this plan** | **7** |
| **Done** | **1** |
| **Remaining** | **6** |
| **Complete** | **14%** |

**This document and the logic document are PR 1.** No behavior has changed yet. The percentage
counts pull requests, not effort — PR 7 is larger than PRs 3 to 6 together.

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

Each row is a pull request. Each is shippable alone and improves the answer alone.

| PR | What | Where | Status |
|---|---|---|---|
| 1 | **This document and the logic document** | `docs/` | **Done** |
| 2 | **A golden set for discovery** | new `landscape-golden` fixtures | To do |
| 3 | **Product-level candidates** | `candidates::from_results`, `from_its_own_page` | To do |
| 4 | **Admit on fit, not appearance** | `candidates::from_hits`, `Vocabulary` | To do |
| 5 | **Candidates from page content** | new `candidates::named_in` | To do |
| 6 | **Carry the reason to the page** | `Candidate`, `Because`, `Report`, `web/` | To do |
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
and the four-to-eight-minute wait is unjustifiable either way. Measure it here, before building.

### PR 3 — Product-level candidates

**Keep the path.** `from_results` reduces every URL to its registrable domain; keep the URL the
engine returned and make *that* the candidate. `from_its_own_page` then fetches the page that was
actually found rather than the domain root.

Microsoft Project, with *by Microsoft* beside it, instead of Microsoft. When the root is what came
back — `asana.com` — the company and the product are one thing and one name is right.

**It also fixes attribution downstream.** A price labeled `microsoft.com` is true and useless.

*Removes cause 2. The smallest change in this plan and the largest single improvement.*

### PR 4 — Admit on fit, not appearance

The page fetch in step 7 becomes the admission test rather than a naming step: **does this page
describe something in the market being asked about?** The vocabulary-overlap machinery already
exists for the seeded path, and `Vocabulary::Read | Unreadable | NotRequested` already keeps the
three silences apart.

A page that is about something else is a **finding**, reported as one — not a silent drop.

*Removes cause 4. `projectplusgame.com` fails on its own front page, which is the cheapest place
to catch it and a page already being fetched.*

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

### PR 6 — Carry the reason to the page

Every company arrives with the sentence and the URL that admitted it, and the date it was read.
`Because` already exists for the seeded path; extend it to the described one, put it on `Report`,
and render it under each company.

**This is the last place something is asserted without evidence.** The claims inside a report
are quoted, dated and cited; the *choice of company* is not.

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
