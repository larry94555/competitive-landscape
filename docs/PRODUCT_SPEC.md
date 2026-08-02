# Landscape — Product & UX Specification

> Section **A** of the roadmap. See [ROADMAP.md](ROADMAP.md) for the index and phasing.

---

## 0. The one-sentence product

**Type anything about a product or a set of competitors into one box and get a structured,
source-cited competitive analysis — then get told when anything changes.**

> **Speed is a function of what the hosting costs.** On the free-tier launch host a complete
> report takes 90–180 seconds, with the first section on screen in 20–40s. The under-a-minute
> goal arrives at Rung 2 ([ROADMAP.md](ROADMAP.md) §6). Every timing in this document is given
> as *Rung 0 (today) → Rung 2 (funded)*, and the interface promises the former.

## 1. Design principles (enforced, not aspirational)

1. **One box.** The landing page and the app are the same page. A first-time visitor sees a
   focused textarea and three example chips. Nothing else above the fold.
2. **No prompt literacy required.** The user writes like a human ("Notion vs Coda vs
   Obsidian" or "I'm building an invoicing tool for freelancers"). The system does the
   prompting.
3. **Ask at most 3 short questions, and only when the answer would change the report.**
   Every question is answerable by clicking a chip.
4. **The report is always the same shape.** Users learn the layout once, on their first
   report, and never again.
5. **Absence is information.** "Not found in public sources" is a first-class, styled
   output — never a gap, never a guess.
6. **Every claim is clickable to its source.** No citation, no claim.
7. **Say what this is.** A calm, permanent line — not a modal, not a scare banner —
   setting the expectation: *good-enough public intelligence, generated from public web
   sources, not verified enterprise CI.*
8. **The exit is always PDF.** Users came to send something to someone. One click, one page,
   presentable.

## 2. User flows

### 2.1 First-time anonymous visitor — the first-report path

```
Land on /
  └─ Sees: headline, one textarea (autofocused), 3 example chips, one disclaimer line.
     No signup wall. No cookie modal beyond a minimal essential-only notice.
Type "Linear vs Jira vs Shortcut"  →  Enter (or click Analyze)
     Timings shown as  Rung 0 (free tier, today)  →  Rung 2 (funded)
  └─ [0.0s → 0.0s] Input collapses to a subject chip. Stage rail appears:
             Resolving → Fetching → Reading → Writing
  └─ [~2s → 0.4s] Optional ClarifyPanel appears ONLY if needed (see §3). Chips, not text
             fields. A "Skip, just analyze" button is always present and always works.
  └─ [3–10s → 1–3s] Source cards stream in: linear.app/pricing, g2.com/…, news…
  └─ [12–25s → 3–5s] **Pricing table appears first** — it is parsed from the page by code,
             not generated, so it lands before any model output. This is deliberate: the
             most-wanted section is also the fastest and the most accurate.
  └─ [20–40s → 4–8s] Positioning begins streaming, word by word.
  └─ [40–150s → 8–20s] Key features → Recent public changes → Sentiment themes → SWOT
  └─ [90–180s → ~20s] Sources section completes. PDF button turns solid.
Read / scroll
  └─ Hover any [S4] chip → source domain, page title, fetched-at timestamp, open link.
  └─ Per-section 👍 / 👎.
Act
  ├─ "Download PDF" → instant (pre-warmed). No email gate. This is the moment of value.
  ├─ "Watch for changes" → **presented at the completion moment, beside the PDF button, not
  │    buried later.** Monitoring is the retention hinge, so the first report is where it must
  │    be offered — the one-shot analysis is the demo; the watch loop is the product.
  │    Soft account prompt: "We'll email you. Where?" (email only, magic link, no password) —
  │    the ONLY place anonymous users meet friction, and only because a notification
  │    literally requires an address.
  └─ "Share" → public read-only URL.
Second analysis same day
  └─ Works. Third → "You've used your free analyses for today. Sign in (free) for 10/month."
```

**Anonymous limit: 2 analyses per day** (hashed IP + coarse fingerprint), 0 watches,
PDF allowed, share allowed. Generous enough to prove value; tight enough to survive.

**Quota counts analyses, not passes.** A pass-2 update never consumes quota — it completes
work already paid for, and charging twice for one analysis would be indefensible. Likewise a
failed analysis costs nothing (§2.4).

### 2.1A Two-pass reports — fast first, complete second

Some pages render their content with JavaScript, so a plain fetch returns a shell. Reading
those requires a headless browser, which is far too heavy to run while a user is waiting on
four shared ARM cores ([ARCHITECTURE.md](ARCHITECTURE.md) §5.5). Rather than make every
analysis slow to accommodate the minority of pages that need it, the report arrives in **two
passes**.

```
Pass 1  ~120s   Static fetch + embedded-state extraction + API discovery + archive lookup.
                A complete, usable, fully cited report. Any source that still needs
                rendering is marked — not hidden.
Pass 2  10-15m  Queued browser renders, one at a time, off-peak, yielding to live users.
                The report updates in place to v2 and the user is told.
```

**Pass 1 is a real report, not a teaser.** It is complete under everything we could read
without a browser, and for most subjects pass 2 changes nothing at all. The user is never
shown a deliberately withheld result.

**What the user sees during pass 1** — the gap is honest and specific, never a bare "not
found":

> **Shortcut pricing — retrieving this page another way.**
> This page builds its content in the browser, so our first pass could not read it. We have
> queued it and will update this report, usually within 15 minutes.
> *Checked: /pricing (JavaScript-rendered), /plans (404), homepage, docs.*

**When pass 2 completes:**

- The report becomes **v2**. **v1 stays reachable at its own URL** — a shared report whose
  content silently changes is a citation problem, so both versions persist and the share link
  resolves to the latest with a visible version marker.
- A quiet in-page banner if the reader is still on it: *"Updated — pricing for Shortcut is now
  included. [See what changed]"*
- Signed-in users get one email. Anonymous users simply revisit the URL; no account is
  required to receive the completed report.
- **The PDF waits.** The executive PDF is not pre-warmed while pass 2 is pending, because a
  user who downloads at minute two and forwards it has a stale artifact. If they ask for the
  PDF before pass 2 finishes, they get it stamped *"Provisional — 1 source still being
  retrieved"*, and it regenerates on completion.

**Pass 2 uses the same inference provider as pass 1.** If the analysis ran on a BYOK key, so
does its completion; if the key has since been removed or stopped working, pass 2 falls back
to the built-in model and says so in the update notice — the same fallback treatment as §5A.4.
Provenance stays consistent across both passes so a report is never half one model and half
another without saying so.

**When pass 2 does not run at all** — which is the common case:

- Every source read successfully on pass 1 → the report is final immediately, marked
  **Complete**, and nothing is queued.
- Rendering yields nothing new → the report stays v1 and the marker becomes
  *"we were unable to read this page"* with what was attempted
  ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5).

**Rung 2 collapses the wait.** On funded hardware, pass 1 is 15–25s and pass 2 minutes rather
than tens of minutes — the shape stays the same, the numbers shrink.

### 2.2 Registered free user

- Sign-in is **email + magic link only**. No password, no confirm-password, no captcha
  unless abuse is detected.
- After the first magic-link click, the session persists ~90 days. Sign-in should feel like
  something that happened once.
- Gets: **10 analyses/month**, **3 watches** at weekly cadence, analysis history, saved
  reports, both PDF variants, KB posting.
- A small usage meter lives in the header — a thin bar, not a nag. It turns amber at 80%.
- History (`/` shows recent analyses beneath the box once you have any) is the entire
  "dashboard." There is no separate dashboard to learn.

### 2.3 Paying user (Starter / Pro)

- Upgrade path is always contextual: the upgrade dialog appears **at the moment of the
  limit**, states exactly what was blocked, what the next tier gives, and the price. Two
  buttons: *Upgrade* and *Not now*. It never appears otherwise.
- Stripe Checkout → back to the **exact analysis that was blocked**, which then runs
  automatically. The interrupted intent is always resumed.
- Gets: higher quotas, **priority queue slot** (visible: "Priority — running now"), more
  watches at faster cadence, instant alerts for major changes, Slack/webhook delivery
  (Pro), "refresh this analysis" bypassing the cache, and the full-length PDF with
  evidence quotes.
- Billing is entirely Stripe Billing Portal. Cancel is one click, present in the UI, not
  hidden — cancellation friction is a support-load generator and a trust destroyer.

### 2.4 Failure flows (designed, not incidental)

| Situation | What the user sees |
|---|---|
| Subject unresolvable | "I couldn't identify a product from that. Do you mean one of these?" + 3 candidates + the box, still filled. |
| Few sources found | Report runs anyway, marked **Thin evidence** with source count and an explicit list of what could not be found. |
| Sources found but not used | "3 sources found and not used" — collapsed, expandable to **what we were and were not able to confirm**, phrased per [FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5 ("we could not confirm an author or a publication date"; "we were unable to reconcile $6.00 with the vendor's own current page, $8.50 as of 2026-07-31" — both figures shown, neither adjudicated). "Show what we found" opens a quarantined panel: never the report body, never the PDF, never a shared link. |
| Section empty only because of strictness | "2 unattributed sources mention pricing. Include them?" — one click, scoped to this analysis. The only place the strictness setting ever interrupts. |
| Site blocks the bot | Source card shows "not accessible to automated retrieval" and is listed in Sources with that reason. |
| Model/section timeout | That section shows "couldn't be completed within the time budget — retry" with a retry button. The rest of the report is delivered. |
| Queue congestion | "2 analyses ahead of you — about 4 minutes." Honest countdown in real units, not a spinner. On the free tier this is common and must read as normal, not as breakage. |
| Hard failure | Nothing is charged against quota. One-click retry. Error id shown for support. |

---

## 2A. Mobile and returning-user states

**Mobile is a reading surface, not a second app.** Reports are forwarded and opened on phones;
the composer is used on desktop. The spec therefore differs by surface rather than duplicating:

- **Composer on mobile:** the textarea and example chips, full width, keyboard-aware. Nothing
  else above the fold. Identical behaviour, not a reduced feature set.
- **Report on mobile:** sections stack; the **feature matrix scrolls horizontally inside its own
  container** with the competitor column pinned (the page body never scrolls sideways); charts
  reflow to full width; citation chips open a sheet rather than a hover card, because hover
  does not exist.
- **Long waits on mobile** are the hardest case — a locked phone must not lose a report. The
  `Last-Event-ID` replay (ARCHITECTURE §2.4) exists for exactly this, and it is tested on a
  real device, not only in a headless browser.

**Returning-user states**, none of which is an empty dashboard:

| State | What `/` shows |
|---|---|
| First visit | Composer only |
| Has history | Composer, with recent analyses beneath it |
| Analysis still running | Composer, with the in-flight analysis pinned at the top and resumable |
| Pass 2 pending | Same, with the "completing" marker — revisiting is how anonymous users collect the finished report |
| Watches firing | A single line above history: *"3 changes across your watches this week"* — not a badge, not a notification centre |

---

## 3. Clarifying questions

Asked **only** when the input is genuinely insufficient, at most **3**, always progressive
(one at a time), always chip-answerable, always skippable.

**Questions fire only when discovery fails to converge**, not when the prompt looks
incomplete — see [COMPETITIVE_DISCOVERY.md](COMPETITIVE_DISCOVERY.md) §2. A cheap discovery
probe runs first; most prompts need no question because the probe resolves them. Where
convergence is merely *marginal*, the product proceeds and shows an editable interpretation
line instead of asking (§6.3 there) — direct manipulation beats interrogation.

Trigger conditions (evaluated after the probe, by a grammar-constrained call to the 1.7B
router model — ~2s on the free tier, <1s on Rung 2):

| Condition | Question | Chips |
|---|---|---|
| Ambiguous brand name (multiple real products match) | "Which *Notion* do you mean?" | the candidates |
| No competitors named and no product URL | "Who should I compare against?" | 3 auto-suggested competitors + "Find them for me" |
| Product described but no market/segment | "Who's the buyer?" | SMB / Mid-market / Enterprise / Consumer |
| Intent unclear (positioning vs pricing vs feature gap) | "What matters most right now?" | Positioning / Pricing / Features / Everything |

Rules: never ask something inferable from the input or from a fetched homepage; never ask
two questions on one screen; "Skip, just analyze" always produces a complete report; and
the answer is remembered for the session so a follow-up analysis doesn't re-ask.

---

## 4. The report schema (fixed, every time)

> **Scope note.** This section specifies the *schema*. What the report should **contain** —
> the nine sections, the chart catalogue, the evidence classes, and what is deliberately
> excluded — is specified in
> [COMPETITIVE_ANALYSIS_REPORT.md](COMPETITIVE_ANALYSIS_REPORT.md). The schema below covers
> seven sections and must be expanded to nine (adding **Feature comparison matrix** and
> **Market emphasis**) plus a `Chart` payload per section; that expansion is ADR-worthy and
> lands in Phase 2. §4.2's worked example already shows the **nine-section** output the
> schema must reach.

The schema is defined once in Rust (`landscape-core`) and generates the TypeScript types,
the JSON Schema, and the GBNF decoding grammar. **The model cannot emit a shape other than
this one.**

### 4.1 Structure

```
Header
  subject, subject_type (single | comparison), competitors[],
  generated_at (UTC), model_id, prompt_version,
  evidence_strength: strong | moderate | thin,
  source_count, disclaimer
1. Positioning
2. Pricing signals
3. Key features
4. Recent public changes
5. Review & sentiment themes
5A. What folks are talking about        ← discussion signals + the absence panel
6. SWOT-style summary
7. Sources
```

> **5A is the second axis.** Sections 1–5 describe *companies*. 5A describes *the problem,
> the idea and the niche* — public discussion, active open-source projects, and the venues
> where the idea does **not** appear. It is specified in full, including which venues may
> legally be read, in [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md). It is numbered 5A
> rather than 6 to avoid renumbering the existing nine sections.

Every section carries `status: populated | partial | not_found_in_public_sources` and
`notes[]`. Every factual statement is a `Claim`:

```ts
interface Claim {
  text: string;              // ≤400 chars, one assertion
  source_label: string;      // "S1".."S14" — required
  evidence_quote: string;    // verbatim span from the source, required
  confidence: 'high' | 'medium' | 'low';
  as_of: string;             // ISO-8601 UTC, from the source or the fetch time
}
```

Section payloads:

| Section | Payload |
|---|---|
| **Positioning** | `summary: Claim[]` (2–4), `target_segments: Claim[]`, `stated_differentiators: Claim[]`, `category_language: string[]` (their words, quoted) |
| **Pricing signals** | `tiers: PricingTier[]` (`name, price_display, billing_period, seat_or_usage_basis, notable_limits[], source_label, source_class, as_of`), `free_tier: bool \| unknown`, `trial: Claim?`, `enterprise_pricing: 'public' \| 'contact_sales' \| 'unknown'`, `observed_changes: Claim[]`, `reported_unconfirmed: Claim[]` — secondary-sourced figures shown beneath the table, never in it |
| **Key features** | `features: { name, description, evidence: Claim, category }[]` (8–15), `notable_gaps: Claim[]` — *gaps only when a source explicitly says so; never inferred from absence* |
| **Recent public changes** | `changes: { date, headline, detail, kind: release\|pricing\|positioning\|funding\|personnel\|policy, evidence: Claim }[]`, `lookback_window_days`, `coverage_note` |
| **Review & sentiment themes** | `themes: { theme, valence: positive\|negative\|mixed, frequency: 'often'\|'sometimes'\|'rarely', representative_quotes: Claim[] }[]`, `platforms_covered[]`, `volume_caveat` — **no numeric ratings are synthesized**, only reported with a source |
| **What folks are talking about** | `window_days`, `venues_considered: VenueAssessment[]`, `asking_for/complaining/attempts: Signal[]` (≤5 each), `building: Project[]` (≤5). Every venue records `fit: expected\|plausible\|poor`, whether it was searched, the **queries used**, and a `caveat` that is *mandatory* whenever fit is not `expected`. Absence is only published from venues where presence would be expected; no sentiment, momentum or trend is computed. Full schema and the source-by-source access position in [DISCUSSION_SIGNALS.md](DISCUSSION_SIGNALS.md) |
| **SWOT-style summary** | `strengths/weaknesses/opportunities/threats: Claim[]` (2–4 each). Opportunities/Threats are **explicitly labelled `interpretation`** and must each cite the observed facts they rest on. This is the one place inference is allowed, and it is visually marked as such. |
| **Sources** | `sources: { label, url, title, host, source_class: P\|A\|U, attribution_signals_confirmed[], independence_group, fetched_at, content_hash, extraction_quality, status: ok\|blocked_by_robots\|unreachable\|paywalled }[]`, `sources_not_used: { url, host, not_used_reason: unverified_by_our_criteria\|could_not_reconcile, signals_confirmed[], what_it_stated, primary_value_and_date }[]`, `strictness_setting` |

### 4.2 Rendered example — the full nine-section output (abridged)

This is the specification's most load-bearing artifact: the shape everything else must
produce. Note what it demonstrates beyond layout — source classes on every value, five-state
matrix cells, a disclosed not-used source, an auditable negative, and interpretation that is
visually separated from observation.

```markdown
# Linear vs Jira vs Shortcut
Comparison · Generated 2026-07-31 14:22 UTC · 11 sources (9 independent groups)
Evidence: strong · Sources: primary + attributed · Model: qwen3-8b · Prompt v4 · Complete
Public-web analysis. Not verified enterprise competitive intelligence.

## 1. Positioning
Linear describes itself as "purpose-built for modern product development." [S1·P] (high)
Jira describes itself as "agile project management at scale." [S4·P] (high)
Shortcut describes itself as "planning without the overhead." [S7·P] (medium)

Category language — Linear: "issue tracking, project planning, roadmaps" [S1];
Jira: "agile project management at scale" [S4]; Shortcut: "project management for
software teams" [S7].

  [Positioning map — INTERPRETATION]
  Axes: published entry price (observed) x stated target segment (observed ordinal).
  Method printed on chart. Omitted entirely if neither axis can be grounded.

## 2. Pricing & packaging
| Product  | Tier  | Price         | Basis    | Notable limits            | As of      | Src   |
|----------|-------|---------------|----------|---------------------------|------------|-------|
| Linear   | Free  | $0            | per user | 250 issues [S2]           | 2026-07-31 | S2·P  |
| Linear   | Basic | $8/user/mo    | per user | annual billing shown [S2] | 2026-07-31 | S2·P  |
| Jira     | Free  | $0            | per user | up to 10 users [S5]       | 2026-07-30 | S5·P  |
| Shortcut | Team  | $8.50/user/mo | per user | -                         | 2026-07-31 | S8·P  |

Enterprise pricing: contact sales for all three [S2][S5][S8].

  [Cost-at-scale curve — DERIVED]
  Monthly cost vs seats (1/5/10/25/50/100), step-shaped at tier boundaries.
  Contact-sales tiers appear as a labelled gap, never an estimate.

> Reported elsewhere, unconfirmed: a third-party article (Feb 2026) states Shortcut
> Business at $16/user/mo. We were unable to confirm this on Shortcut's own site, so it
> is not in the table above. [S10·A]

## 3. Feature comparison matrix
| Capability          | Linear | Jira  | Shortcut | Note                          |
|---------------------|--------|-------|----------|-------------------------------|
| Issue tracking      | ✓ P    | ✓ P   | ✓ P      |                               |
| Roadmaps            | ✓ P    | ✓ P   | ◐ P      | Shortcut: "basic" [S7]        |
| Native time tracking| ? —    | ✓ P   | ✗ P      | Shortcut docs state not incl. |
| Self-hosting        | ✗ P    | ✓ P   | ? —      | Linear states cloud-only [S1] |
| SAML SSO            | ✓ P    | ✓ P   | ▲ A      | Per Jira's comparison page    |

  ? = we found no public statement. Hover lists the pages checked.
  ▲ = claimed by a rival's comparison page; never presented as neutral.

## 4. Recent public changes (lookback: 90 days)
2026-07-14 — Linear shipped "Initiatives" for multi-project planning. [S3·P]
2026-06-02 — Jira announced a Premium price increase effective Sept 2026. [S6·P]
Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
/releases (404), blog (90d). Not "no changes."

  [Shipping velocity — DERIVED]  Releases/month, 12 months. Shortcut shown as
  "no public changelog," never as zero.

## 5. Review & sentiment themes
Speed / responsiveness — positive, often (14 mentions, 2 platforms).
  "The fastest tracker I've used." [S9·A]
Migration difficulty — negative, sometimes (Jira, 6 mentions). [S9·A]

Unmet needs (from 2-3 star reviews):
  Bulk export — 7 reviewers across 2 platforms mention wanting it [S9][S11]
  Offline mode — 4 reviewers [S9]

Platforms covered: G2, Reddit r/ProductManagement. Volume caveat: 31 reviews sampled.
Ratings reported with source and count only; no composite score is synthesised.

## 6. Market emphasis (strategy canvas)
  [Value curve — DERIVED, presented as interpretation]
  Factors from the union of their own marketing language; Y = prominence in their own
  public copy. Method printed on chart. This plots EMPHASIS, not capability.

## 7. SWOT-style summary   [INTERPRETATION — inference from the facts above]
Strengths  · Linear's shipping cadence is visibly high — 6 changelog entries in 90 days [S3].
Weaknesses · Shortcut publishes no changelog, so release activity cannot be assessed [S8].
Opportunities · Cost-sensitive Jira teams may evaluate alternatives before Sept 2026 —
             inference from the announced increase [S6], not a stated Jira position.
Threats    · Jira's self-hosting option [S5] covers a requirement Linear states it does
             not serve [S1].

## 5A. What folks are talking about
Window: 24 months · 4 venues searched · 2 with enough volume to report

People are asking for
  "I'd pay for something that tells me what the chefs ordered last week."
                                      — r/smallfarms, 2026-03-11 [S14]
  2 posts. Not a survey — these are the posts we read.

People are building
  harvest-ledger   1,240 commits · 7 contributors · last commit 2026-07-28 [S16]
  farm-box-api     archived 2024-11 · "No longer maintained." [S17]
  Ranked by commit activity. Stars shown as context, never as the sort key.

Where this idea does not appear
  Hacker News  Not found. Searched `farm to restaurant ordering`,
               `restaurant produce sourcing`, `farm wholesale platform`
               since 2024-08-01. 0 results over 2 comments. 09:14 UTC.
               HN skews to developer tools — absence here is weak evidence.
  Reddit       Not searched: their API terms do not permit our use, so we
               cannot claim a negative. 3 threads found via web search. [links]
  Lobsters     Not reported — no meaningful volume on this topic. Absence
               here would tell you nothing.

Silence is not evidence of no demand. Most working businesses are never
discussed online.

## 7A. Operating signals
| Company  | Publicly visible since | Last public update | Open roles | Financial standing |
|----------|------------------------|--------------------|------------|--------------------|
| Linear   | 2019 (archive) ·       | 2026-07-14 [S3]    | 14 [S12]   | Privately held;    |
|          | states founded 2019    |                    |            | not disclosed      |
| Shortcut | 2014 (archive)         | not found          | not found  | Not disclosed      |

No composite health score. Private-company revenue is never estimated.

## 8. Sources
S1  linear.app — Homepage — P — fetched 2026-07-31 14:21 UTC — hash 8f3c…a91d — group G1
S2  linear.app/pricing — Pricing — P — 14:21 UTC — hash 4b7e…22c0 — group G1
…
S12 boards.greenhouse.io/linear — Careers — P — 14:23 UTC — group G7

Not used (3): [Show what we found]
  example.com/best-tools-2026 — we were unable to verify this source against our
  criteria: we could not confirm an author, a publication date, or cited sources.
  capterra.com/… — not accessible under our fetching rules (robots.txt).
```

### 4.3 The "not found" treatment

A section with nothing verifiable renders as a calm bordered block, not an error:

> **No public pricing found.** Shortcut's pricing page returned 403 and no pricing was
> stated on the homepage or in the last 90 days of the blog. We do not estimate prices.
> *Checked: /pricing, /plans, homepage, blog (90d), G2 listing.*

Listing **what was checked** converts a gap into evidence of thoroughness. This is a
deliberate trust mechanism, not a consolation message.

---

## 5. Notification UX

### 5.1 Creating a watch — two clicks, no forms

From a finished report, a "Watch for changes" button opens a sheet pre-populated with the
sources already fetched, sensibly pre-selected:

```
Watch Linear for changes
 [x] Pricing page          linear.app/pricing        ← pre-checked
 [x] Changelog             linear.app/changelog      ← pre-checked
 [ ] Homepage              linear.app
 [ ] News mentions         (search-based)
How often?   ( Daily )  Weekly     [Free plan: weekly, 3 watches]
Email:       larry@example.com
                                              [Start watching]
```

Nothing else is asked. No naming, no folders, no thresholds — thresholds are learned (§5.4).
Watches can also be created from `/watch` by pasting a URL.

### 5.2 The alert email

Subject lines state the change, not the product:
`Linear raised Basic to $10/user/mo` — never `You have 1 new alert`.

```
┌───────────────────────────────────────────────┐
│ Linear · Pricing page changed                 │
│ Detected 2026-08-14 09:02 UTC   [ Major ]     │
├───────────────────────────────────────────────┤
│ Basic went from $8 to $10 per user/month.     │
│ The free tier's 250-issue limit was removed.  │
│                                               │
│ Why it may matter                             │
│ A 25% increase on their entry paid tier, plus │
│ a more generous free tier — consistent with   │
│ moving upmarket while widening the top of the │
│ funnel.                                       │
│                                               │
│ [ See the diff ]  [ Re-run full analysis ]    │
│                                               │
│ Useful?   👍   👎     (one click, no login)    │
├───────────────────────────────────────────────┤
│ Source: linear.app/pricing · fetched 09:02 UTC│
│ Pause this watch · Unsubscribe                │
└───────────────────────────────────────────────┘
```

- **Weekly digest is the free-tier default**; one email, grouped by product, cosmetic
  changes collapsed into a single line.
- Paid tiers get **instant alerts for `major` pricing/positioning changes** and a daily
  digest for everything else.
- "See the diff" opens a side-by-side rendered diff with the changed regions highlighted —
  the receipt behind the AI summary. Users who verify once tend to trust thereafter.
- The 👍/👎 in the email is a signed one-click link, no login. It is the primary training
  signal for §5.4.

### 5.3 Zero-noise defaults

Never alert on: timestamps, rotating testimonials, customer-count widgets, blog sidebars,
cache-busting parameters, cookie-banner variants, A/B-test copy shuffles, or any diff whose
normalized SimHash distance is below threshold. **Silence is the default state of a good
monitoring product.** A watch that fires weekly is broken.

### 5.4 Learned thresholds

Two 👎 on a watch → its importance threshold rises automatically and the user is told:
*"We'll only send major changes for this page from now on."* No settings screen required;
the setting is still there in `/watch/:id` for anyone who wants it.

---

## 5A. Bring your own model (BYOK)

**Default: the built-in model. Nobody is asked about this, ever, unless they go looking.**
A first-time visitor must never see the words "API key," "provider," or "model." The feature
exists for the minority who want it and is invisible to everyone else — that is the only way
it can coexist with a zero-learning-curve product.

### 5A.1 Where it surfaces — two places, both earned

**1. In `/account` → "Model," for people who go looking.** A short form: provider dropdown
(Built-in · OpenAI-compatible · Anthropic), key field, optional model name, optional custom
base URL behind a disclosure. Test button. That's it.

**2. Contextually, when the wait is the problem.** After a user's *third* analysis, or when
they abandon mid-stream, a single dismissible line appears under the stage rail:

> Reports take about two minutes on our free hardware. If you have an OpenAI or Anthropic
> key, you can use it for your own analyses and get results in about 20 seconds.
> [Use my own key] · [No thanks]

Dismissed twice, never shown again. This is the one place the product acknowledges its own
slowness, and offering a fix is more honest than staying quiet about it.

### 5A.2 The privacy disclosure — mandatory, unmissable, at the point of choice

The product's differentiator is that research never leaves the server. BYOK breaks that, by
the user's own choice, so the choice must be informed:

> **Using your own key sends your query and the fetched page text to OpenAI.**
> That data leaves our servers and is subject to their terms, not ours. Your built-in
> analyses stay local. You can switch back at any time.
> [ ] I understand

Unchecked, the key is not accepted. This is not a dark pattern in reverse — a user who
enables BYOK without understanding it has been mis-sold the product's core promise.

### 5A.3 Session-only by default

The key is held for the session and never written to disk. Persisting it is a separate
checkbox, and the copy says exactly why you would want to:

> [ ] Remember this key so my **watch alerts** also use it.
>     Stored encrypted. We never show it again, and you can delete it in one click.

Most users need only the session behaviour. Only background jobs require persistence, and
saying so lets people choose the smaller risk.

### 5A.4 On the report

A small provider chip beside the timestamp: **`Built-in model`** or **`Your OpenAI key ·
gpt-…`**. Same treatment in the PDF footer. If a key failed and the analysis fell back:

> **Your key didn't work, so we used the built-in model.** OpenAI returned "insufficient
> quota." Nothing was charged to your account, and this report is complete.
> [Check my key]

The report is still delivered. A failed key must never mean a missing report or a missing
watch alert.

### 5A.5 What it changes for the user

| | Built-in | BYOK |
|---|---|---|
| Speed | 90–180s (Rung 0) | Whatever their provider does — typically 15–25s |
| Analysis quota | Plan limit | **Substantially higher** — inference is no longer ours |
| Cost | Free / plan price | Their provider bill, which we show tokens for but cannot see |
| Privacy | Never leaves our server | Leaves, to their provider |
| Watch count, alert cadence, webhooks, API, PDF | Plan limits | **Unchanged** — these aren't inference costs |
| Source grounding, citation checks, "not found" honesty | Identical | **Identical** |

That last row is the important one. Bringing a frontier model does not switch off any
verification: an unsupported claim is deleted whether Claude wrote it or Qwen3-8B did.

### 5A.6 Cost transparency

Every BYOK analysis shows tokens submitted and returned, and `/account` shows a 30-day
total. We never see their spend and say so. Nobody should discover this feature through a
provider invoice.

---

## 6. Support UX — the "slash-lite" open knowledge base

Full design in [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md). The user-facing shape:

- `/help` is **public, indexable, and searchable without an account**. Search box on top,
  popular tags, recently answered threads.
- Every product surface that can confuse someone has a contextual `?` linking to a
  *specific* KB thread, not to the help index.
- Asking a question is one textarea plus optional email. Signed-in users post with their
  handle; anonymous asks are allowed with email verification to prevent spam.
- **Slash commands in the composer** (`/bug`, `/pricing`, `/quota`, `/watch`, `/source`,
  `/feature`, `/private`) auto-apply tags and pre-fill a short structured template.
  Typing `/` opens the menu; ignoring it entirely still works.
- Answers stay public forever. Official answers are badged and pinned above community ones.
- `/private` is the escape hatch: it converts the post into an email-backed private thread.
  It exists, it works, and it is deliberately the *second* option on the menu.

---

## 7. How near-zero learning curve is maintained at every step

| Surface | Mechanism |
|---|---|
| Landing | One autofocused textarea. Three example chips that run real analyses. No signup wall, no video, no feature grid above the fold. |
| Input | Free-form natural language. URLs, names, or a paragraph all work. No syntax, no operators, no "prompt tips" link. |
| Clarification | ≤3 questions, one at a time, chips not fields, always skippable to a complete report. |
| Waiting | Progressive streaming with a named stage rail, sources appearing live, and the deterministically-parsed pricing table landing before any generated text. The user reads while it writes — which is what makes free-tier latency survivable. |
| Scanning | The feature matrix and pricing charts carry the comparison, so the reader gets the answer without reading paragraphs. Every chart ships with its data table, so nobody has to trust a picture. |
| Reading | Identical 7-section structure every time. Learned once. |
| Trust | Inline `[S4]` chips with hover cards; "what we checked" on every gap. |
| Acting | One primary button per context: **Download PDF** on a report, **Start watching** in the watch sheet, **Upgrade** in the limit dialog. |
| Accounts | Magic link only. Account creation happens *because* the user asked for something that needs one (an alert), never as a gate. |
| Model choice | **Not a choice, by default.** BYOK (§5A) is invisible until a user goes looking or repeatedly hits the wait. Nobody is asked to pick a model to get a report. |
| Source strictness | **Not a choice, by default.** The source-strictness setting ([FACT_CHECKING.md](FACT_CHECKING.md) §3.2.3) has a sensible default and lives in `/account`; it surfaces contextually only when a section is empty under the current setting. |
| Limits | Never surprise the user: usage meter visible; limit dialogs state exactly what was blocked and resume the blocked action after upgrade. |
| Notifications | Two clicks with sane pre-selection. Thresholds are learned from 👍/👎, not configured. |
| Support | Public KB reachable from every screen; slash commands are discoverable but never required. |
| Errors | Every failure names what happened, what it cost (nothing), and offers exactly one recovery action. |

**The measurable definition:** a first-time visitor with no instructions reaches a downloaded
PDF **without abandoning**, having read no documentation and made no account. On Rung 0 that
means surviving a 90–180s wait, which is why the perceived-latency ladder in §2.1 is a
P0 requirement rather than polish: sources landing at 3–10s and a parsed pricing table at
12–25s are what make the wait legible as work rather than as a hang. Tested with unmoderated
5-person usability runs every phase (see Metrics, §F).
