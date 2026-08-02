# Landscape — Discussion Signals & the Absence Report

> How public discussion — Hacker News, Reddit, GitHub, forums — becomes a report section
> called **"What folks are talking about"**, and how the *absence* of discussion is reported
> without it becoming a guess.
>
> This document adds a second axis to the product. Everything before it answers **"who else
> is doing this?"** — a question about *companies*. This answers **"what is the conversation
> around this problem?"** — a question about *the problem, the idea and the niche*, which may
> have no companies in it at all.
>
> Companion documents: [FACT_CHECKING.md](FACT_CHECKING.md) governs source dispositions and
> auditable negatives; [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) governs how a
> prompt becomes a search; [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md)
> governs what the report contains.

---

## 1. Why this is worth building

The competitor sections answer a question a founder asks once. This one answers questions
they ask continuously:

| Question | What it is really asking |
|---|---|
| What are people **talking about**? | Is this space alive, and moving which way? |
| What are people **complaining about**? | Which problems are unsolved, and by whom? |
| What are people **showing off**? | What has already been tried, and how far did it get? |
| What is **actively being built**? | Is my idea already someone's weekend project with 400 stars? |
| Is my idea **absent everywhere**? | Am I early, or is there no there there? |

The fourth is the one that saves the most time. Discovering that a working open-source
implementation of your idea has had commits every week for two years is worth more than any
competitor table — and it will never appear in one, because it is not a company.

**The fifth is the one nobody else reports, and it is the reason this section exists.** A
silence, honestly bounded, is a finding.

---

## 2. The governing rule for absence

> **Absence is only reportable from a venue where presence would be expected.**

This single rule is what separates a useful negative from a misleading one. "Not found on
Hacker News" is *meaningless* for a farm-to-restaurant ordering app and *highly meaningful*
for a developer tool. Reporting both identically would be dishonest.

So every venue carries a **relevance judgement for this subject**, made before the search
runs, and a negative is only published from venues that pass it:

| Venue passes when | Example |
|---|---|
| The venue's population plausibly contains people with this problem | A restaurant-supply subreddit for a farm-to-restaurant app |
| The venue has non-trivial volume on the *surrounding* topic | 400 posts about restaurant purchasing in the window |
| We can search it under terms that permit it | See §4 |

If a venue fails, it does not appear in the report at all — neither as a finding nor as a
negative. **We never pad the absence list with venues where absence means nothing.** A report
saying "not found on Hacker News, Reddit, Lobsters, Stack Overflow, Product Hunt" for a
catering app is filler wearing the costume of rigour.

### 2.1 What a silence does and does not mean

When we report a silence, we report it with its alternatives intact, and **we do not
adjudicate between them.**

A subject can be absent because:

1. Nobody has this problem *(no market)*
2. People have it but call it something else *(our query was wrong)*
3. People discuss it somewhere we did not or may not look *(private communities, Discord,
   Slack groups, WhatsApp, LinkedIn, trade press)*
4. It is too new to have surfaced
5. It is real, ordinary, and simply not interesting to talk about — **most working businesses
   are never discussed online**

Readings 1 and 5 point in opposite directions and the evidence cannot separate them. The
product's job is to hand the founder a bounded, checkable silence and let them judge. It is
explicitly **not** to say "this looks like an untapped market" or "there appears to be no
demand." Both are inferences the data does not support, and both are the kind of confident
noise the rest of this product exists to avoid.

The one thing a silence *does* establish, and it is worth a lot: **nobody has publicly
claimed this ground in the places we looked.** That is a fact about the public record, which
is exactly the kind of fact this product deals in.

### 2.2 The negative must be repeatable in two minutes

Following [FACT_CHECKING.md](FACT_CHECKING.md) §5.4, every negative shows its work — venue,
queries, window, and time of attempt — so the reader can re-run it by hand:

> **Not found: Hacker News.** Searched `farm to restaurant ordering`,
> `restaurant produce sourcing`, `farm wholesale platform` across all posts and comments
> since 2024-08-01. 0 results with more than 2 comments.
> Checked 2026-08-02 09:14 UTC. *Hacker News skews to developer tools; a restaurant supply
> product may simply not be discussed here.*

That last italic clause is mandatory whenever venue fit is weak. **A negative without its
caveat is worse than no negative.**

---

## 3. What the section contains

Four blocks. Each may be empty, and each renders its own auditable negative when it is.

### 3.1 What people are asking for

Unmet needs, stated by people who have the problem. Sourced from question posts, complaint
threads, and "does anything exist that…" posts.

This is the highest-value block and the hardest to populate honestly. The bar: **a specific
want, stated by an identifiable poster, quoted verbatim, linked, dated.** Aggregate claims
("many users want X") are only made when we can point at the individual posts underneath, and
the count shown is the count of posts we actually read — never an extrapolation.

### 3.2 What people are complaining about

Same shape, negative valence, and the same rule as the existing review-themes section: **no
numeric sentiment is synthesized.** We do not compute a score, a ratio, or a trend line from
post counts. We show themes, each with its posts.

Where a complaint names a company in the competitor set, it is cross-linked to that company's
row — this is where the two axes of the report meet.

### 3.3 What people are building

Two sources, treated differently:

**GitHub — projects.** The definition of *active* matters, because the obvious metrics are
the wrong ones:

| Signal | Use it? | Why |
|---|---|---|
| Commits in the last 90 days | **Yes** | Hardest to fake, directly answers "is this alive" |
| Distinct contributors in window | **Yes** | Separates a project from a person |
| Open/closed issue activity | **Yes** | Shows users exist |
| Most recent release | **Yes** | Shows it ships |
| Stars | **Only as context, never as ranking** | Widely bought and traded; a lagging popularity signal, not an activity one |
| Forks | No | Dominated by drive-by forks |

A project is reported as active on **commit and contributor evidence**, with stars shown
beside it as context and never as the sort key. Archived and read-only repositories are
labelled as such rather than dropped — "someone built this and stopped" is a genuine finding.

**Show HN / launch posts — attempts.** What was tried, when, and what the response was. An
idea that was launched twice and died twice is important information.

### 3.4 Where the idea shows up — and where it does not

The absence panel from §2, rendered per venue, with venue-fit caveats attached.

---

## 4. Sources: what is actually permitted

This is the load-bearing section, because the answer differs sharply by venue and one of the
two the user named is a genuine problem. The project's rule holds throughout: **link rather
than reproduce where terms require it** ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.1b).

### 4.1 Hacker News — clean, use fully

The official API's own README states **"There is currently no rate limit."** It needs no key,
no account, and no authentication. The Algolia-backed search endpoint provides full-text
search across posts and comments, which the Firebase API does not.

This is the anchor venue: unrestricted, well-structured, complete back to 2007, and free.
Where a signal can be sourced from Hacker News, it is.

**Caveat that must reach the reader:** Hacker News is overwhelmingly software-industry,
English-language, and skewed toward developer and infrastructure products. It is a superb
venue for a dev tool and a poor one for most other businesses. Venue fit (§2) governs.

### 4.2 GitHub — clean, use fully

**5,000 requests per hour** authenticated with a personal access token; 60 unauthenticated.
Search endpoints are more restrictive and are documented separately — that specific number
must be confirmed against the endpoint docs during Phase 0 rather than assumed here.

Ample for our volume. Code and repository metadata are public and the API exists to be used
this way. Repository content carries its own licence, so we link and quote minimally rather
than reproducing README text at length.

### 4.3 Reddit — restricted, and this needs a decision before any code is written

**This is the honest problem in the feature request, and it should not be discovered during
implementation.**

What the secondary sources say, and how much to trust them: every accessible write-up on
current Reddit API terms comes from a vendor selling Reddit data access — SocialCrawl,
Prowlo, RedditAPIs, Octolens, Data365, Xpoz. Under this project's own trust model these are
**interested parties** (FACT_CHECKING §L7): they profit from the official API looking
painful. Reddit's own terms pages returned 403 to automated fetching, so the primary source
could not be read.

Treating them as what they are — consistent, mutually corroborating, and interested — they
report:

| Claim | Consistency across sources | Disposition |
|---|---|---|
| Free tier is 100 queries/minute per OAuth client, **non-commercial only** | High | Likely, verify |
| Commercial use requires prior written approval, 2–4 week review, not guaranteed | High | Likely, verify |
| Commercial pricing ~$0.24 per 1,000 calls with a large monthly minimum | Moderate; figures vary | **Unverified** |
| Unauthenticated public JSON endpoints began returning 403 in 2026 | High | Likely, verify |

**What this means for Landscape.** Landscape charges money. Even at $1/month it is a
commercial product, which appears to put the free tier out of reach regardless of volume. A
five-figure monthly minimum is categorically incompatible with a bootstrapped project funding
infrastructure from revenue — that is not a cost to grow into, it is a different business.

**The decision, and it is the founder's to make, not this document's:**

| Option | What it costs | What it gives |
|---|---|---|
| **A. Link-out only** *(recommended for launch)* | Nothing | Name Reddit as a venue, link searches and threads found via web search, never fetch or reproduce. Follows §3.2.1b exactly. |
| **B. Apply for commercial approval** | 2–4 weeks, likely rejection at this size | Full access if granted |
| **C. Third-party data vendor** | Recurring fee, and a dependency whose own rights are unclear | Access without Reddit's approval |
| **D. Omit Reddit entirely** | Loses the venue the user specifically asked for | Zero risk |

**Option A is the recommendation, and it is genuinely useful rather than a consolation
prize.** A web search that surfaces a Reddit thread tells us the thread exists and what it is
titled without our fetching Reddit at all — the same mechanism §3.2.1b already uses for
company-data services. The report can say *"three discussions on r/restaurateur look
relevant"* and link them, which is most of the value, without touching the API.

What Option A cannot do: quote posts, count them reliably, read comments, or assert absence.
**Under Option A, Reddit never appears in the absence panel** — we have not searched it
properly, so we cannot honestly report a negative from it. Saying "not found on Reddit" when
we only ran a web search would be exactly the kind of unearned claim this product refuses.

> **Phase 0 gate.** Before any Reddit code is written, read Reddit's Data API Terms directly,
> in a browser, and record the finding in an ADR. Everything above is secondary and interested.

### 4.4 Other venues

| Venue | Access | Notes |
|---|---|---|
| **Lobsters** | Public JSON endpoints | Small volume, high signal, developer-skewed |
| **Stack Overflow / Stack Exchange** | Documented API, key raises daily quota | Content is CC BY-SA — **attribution and share-alike obligations apply to quoting**; verify exact terms in Phase 0 |
| **Discourse forums** | Most expose public JSON | Where trade and niche communities actually live; often the best venue-fit for non-developer subjects |
| **Product Hunt** | GraphQL API, key required | Launch attempts and their reception |
| **Open web search** | Existing pipeline | The fallback that reaches venues with no API |

**Discourse forums deserve emphasis.** For the majority of subjects — trades, hospitality,
agriculture, logistics, professional services — a niche Discourse or vBulletin forum is a far
better venue than Hacker News, and its public JSON is usually readable under ordinary
robots.txt rules. Venue selection should follow the subject, not our convenience.

### 4.5 The robots.txt commitment applies here too

[FACT_CHECKING.md](FACT_CHECKING.md) commits to honouring robots.txt. Discussion venues are
no exception, and several are aggressive about crawlers. Where an API exists we use the API;
where robots.txt forbids fetching we do not fetch, and the venue is listed as **R — Not read**
with a link, exactly as any other source would be.

---

## 5. From a prompt to a search

The competitor pipeline resolves a prompt to *category vocabulary*
([COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §3). This section needs something
different: **problem vocabulary**, which is what sufferers call it, not what vendors call it.

The two diverge sharply, and searching with the wrong one produces a false silence:

| Vendor language | Sufferer language |
|---|---|
| "farm-to-table supply chain platform" | "restaurant produce ordering", "getting veg from local farms", "invoicing my chefs" |
| "observability" | "why is my server slow", "finding what broke" |
| "revenue operations" | "our sales spreadsheet is a mess" |

**Every absence claim runs at least three query formulations spanning both registers, and
the report shows all of them.** A negative from a single vendor-language query is not
publishable, because it mostly measures our phrasing. This is the single most common way an
absence report goes wrong, and the mitigation is cheap.

Query formulations are generated by the small router model, which is already resident, and
are **shown to the reader** — both because it makes the negative auditable and because the
reader will often spot the term we missed. A one-click "search this too" that re-runs the
block with the reader's own phrasing is the natural follow-on, and belongs in the same phase.

---

## 6. Ranking, and what is worth showing

Raw volume is a bad ranking signal — it selects for whatever was on the front page. Signals
are ranked by:

1. **Specificity** — a stated concrete want beats a general grumble
2. **Recency**, windowed, with the window shown
3. **Corroboration** — the same want expressed by unrelated posters in unrelated venues
4. **Engagement relative to its venue** — 40 comments in a 2,000-member forum outweighs 40 on
   the Hacker News front page

Deduplication matters more here than anywhere else in the product: a single launch is
routinely posted to Hacker News, Reddit, Lobsters and Product Hunt within an hour. These
collapse into **one** signal with multiple links, using the existing independence-grouping
machinery (FACT_CHECKING §L6). Showing them as four independent signals would manufacture
consensus out of one event — the exact failure the independence rule was written to prevent.

**Volume caps.** At most 5 items per block. This section can generate hundreds of plausible
items and a founder will read six. The cap is a product decision, not a performance one.

---

## 7. What the section looks like

Rendered, for the running example — an app helping small farms sell directly to restaurants:

```markdown
## 5A. What folks are talking about
Window: 24 months · 4 venues searched · 2 with sufficient volume to report

### People are asking for
"I'd pay for something that just tells me what the chefs ordered last week
 so I can plan planting."                    — r/smallfarms, 2026-03-11 [S14]
"Every restaurant wants a different price list and I track it in 6 spreadsheets."
                                             — Grower's Forum, 2026-01-22 [S15]
2 posts. Not a survey — these are the posts we read, quoted in full context.

### People are complaining about
Invoicing and payment terms — 4 posts across 2 venues, all describing 30–60 day
waits from restaurant buyers. Cross-references Freshroute [S3], which states
Net-30 as a feature.

### People are building
harvest-ledger        1,240 commits · 7 contributors · last commit 2026-07-28
                      Open-source CSA order management. Active. 890 stars. [S16]
farm-box-api          archived 2024-11. 3 contributors. "No longer maintained." [S17]
Ranked by commit and contributor activity. Stars shown as context only.

### Where this idea does not appear
Hacker News    Not found. Searched `farm to restaurant ordering`,
               `restaurant produce sourcing`, `farm wholesale platform`,
               all posts and comments since 2024-08-01. 0 results over 2 comments.
               Checked 2026-08-02 09:14 UTC.
               Hacker News skews to developer tools — absence here is weak evidence.

Reddit         Not searched. Reddit's API terms do not permit our use, so we
               cannot claim a negative. 3 threads found via web search that look
               relevant: [r/smallfarms] [r/restaurateur] [r/Chefit]

Lobsters       Venue not reported — no meaningful volume on food, agriculture
               or hospitality topics. Absence here would tell you nothing.

Silence in a venue is not evidence of no demand. Most working businesses are
never discussed online.
```

Note what this demonstrates: a populated block, a cross-link into the competitor axis, an
archived project shown rather than dropped, a bounded negative with its caveat, a venue we
honestly did not search, and a venue excluded for poor fit rather than padded into the list.

---

## 8. Schema

Added to the report schema in [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4.1 as section **5A**,
which avoids renumbering the existing nine sections.

```ts
interface DiscussionSignals {
  window_days: number;
  venues_considered: VenueAssessment[];
  asking_for:    Signal[];   // ≤5
  complaining:   Signal[];   // ≤5
  building:      Project[];  // ≤5
  attempts:      Signal[];   // ≤5  launch posts
}

interface VenueAssessment {
  venue: string;
  fit: 'expected' | 'plausible' | 'poor';       // §2 gate
  searched: boolean;
  not_searched_reason?: 'terms_forbid' | 'robots' | 'poor_fit' | 'unreachable';
  queries: string[];                             // shown to the reader, always
  window_start: string;
  checked_at: string;
  result_count: number;
  caveat?: string;                               // mandatory when fit != 'expected'
}

interface Signal {
  quote: string;                 // verbatim, short, attributed, linked
  venue: string;
  posted_at: string;
  url: string;
  engagement: { comments: number; venue_baseline?: number };
  independence_group: string;    // collapses cross-posts
  mentions_competitor?: string;  // cross-link into the competitor axis
}

interface Project {
  name: string; url: string; description: string;
  commits_in_window: number; contributors_in_window: number;
  last_commit: string; last_release?: string;
  archived: boolean; stars: number;   // context only, never the sort key
  licence: string;
}
```

Consistent with the rest of the schema: every item carries its source, its date and its link;
nothing is aggregated into a score.

---

## 9. Cost on the free tier

The binding constraint on Oracle's free tier is **prefill, not generation**
([ARCHITECTURE.md](ARCHITECTURE.md)). This section is prefill-hungry — discussion threads are
long, rambling and mostly irrelevant.

The mitigations are the ones the architecture already uses:

- **Deterministic filtering first.** Recency, engagement thresholds, and keyword gates are
  code, not inference. Most candidate posts never reach a model.
- **Span pre-selection.** Only the matching paragraph and its neighbours go to the extractor,
  never a whole thread.
- **The 1.7B router decides relevance**; the 4B extractor only sees what survives.
- **Cache by post ID.** Discussion posts are immutable in a way company pages are not — a
  thread read once never needs re-reading. This section caches far better than any other.

Budget: this must fit inside the existing pass-1 window without extending it. If it cannot,
it belongs in **pass 2** (the 10–15 minute queued render), which is the natural home for it
anyway given the breadth of searching involved.

---

## 10. What we do not claim

Stated plainly, because this section invites over-reading more than any other:

- **We do not claim to have read everything.** We report venues searched, queries used, and
  the window. Outside that, nothing.
- **We do not compute sentiment, momentum, or trend.** No scores, no arrows, no "rising."
- **We do not infer market size or demand from volume.** Discussion volume measures how
  online and articulate a population is, not how large or how willing to pay.
- **We do not treat silence as opportunity.** See §2.1.
- **We do not identify or profile individuals.** Posts are quoted and linked, never
  aggregated into a picture of a person.
- **We do not reproduce more than a short attributed quote**, and we honour each venue's
  licence — CC BY-SA where it applies, link-only where terms require it.

---

## 11. Phasing

| Phase | Scope |
|---|---|
| **Phase 0** | Read Reddit's Data API Terms directly and record an ADR. Confirm GitHub search rate limits and Stack Exchange licensing against primary docs. Decide Option A–D (§4.3). |
| **Phase 2** | Hacker News and GitHub only, both clean. Blocks 3.1–3.3. No absence panel yet. |
| **Phase 3** | Venue-fit gate (§2), multi-register query generation (§5), and the absence panel — the panel ships *only* once the fit gate exists, or it will produce the misleading negatives §2 was written to prevent. |
| **Phase 4** | Discourse forum discovery, Product Hunt, Stack Exchange. Reader-supplied query re-runs. |
| **Deferred** | Reddit beyond link-out, pending the §4.3 decision. |

**The ordering constraint is deliberate:** the absence panel is the feature most likely to be
quoted back at us, and the venue-fit gate is what makes it defensible. Shipping the panel
first would be shipping the liability without the control.

---

## 12. Open questions

Recorded rather than resolved, because they need the founder's judgement or a primary source:

1. **Reddit terms** — §4.3, blocking, Phase 0.
2. **Does the absence panel belong in the free tier?** It is the most expensive block to
   produce and the most compelling. Probably yes, capped to two venues.
3. **Stack Exchange share-alike** — does quoting under CC BY-SA create obligations for the
   surrounding report? Needs reading, not guessing.
4. **How far back is useful?** 24 months is a guess. The right window likely differs by
   subject and should be measured once there is usage.
5. **Non-English venues** — currently out of scope and unaddressed. For many subjects this is
   a real gap, and pretending the English-language web is the world is its own kind of
   silent error.
