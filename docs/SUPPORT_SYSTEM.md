# Landscape — Support System Design ("slash-lite" open knowledge base)

> Section **D** of the roadmap. See [ROADMAP.md](ROADMAP.md) for the index and phasing.

---

## 1. The thesis

A solo founder cannot answer the same question 400 times. The only sustainable support
model is one where **answering a question once makes it permanently available to everyone
who will ask it next** — including via Google, before they ever open the app.

So support is not a ticket queue with a public FAQ bolted on. It is a **public Q&A corpus
that happens to also handle individual requests**. Private tickets exist as an escape
hatch, not as the default path.

Three consequences that drive the design:

1. **Public is the default.** Every answer lands in indexable HTML at a stable URL.
2. **Support content is also acquisition.** "Does Landscape monitor competitor pricing
   pages?" is simultaneously a support answer and a long-tail landing page.
3. **Deflection is measurable.** If KB search sessions rise while new threads fall, the
   system is working. That ratio is a tracked metric from day one.

---

## 2. Structure

### 2.1 Objects

```
Thread   slug, title, body_md, author (user | anonymous+verified email | official),
         status: open | answered | official, tags[], views, created_at
Reply    body_md, author, is_official, votes, created_at
Tag      canonical slug, description, ~20 total, curated (not user-invented)
```

`status` transitions: `open` → `answered` (a reply was accepted or upvoted past threshold)
→ `official` (the operator edits/blesses a canonical answer and it becomes the definitive
one). `official` threads are the KB; `answered` threads are community knowledge; `open`
threads are the work queue.

### 2.2 URL & discovery surface

```
/help                        search + popular tags + recently answered
/help/t/:slug                one thread (SSR, indexable, JSON-LD QAPage)
/help/tag/:tag               tag index
/help/new                    ask
```

Threads are server-rendered with `QAPage` structured data so Google can show them as
rich results. Sitemap regenerated on write. This is the cheapest durable acquisition
channel the product has.

### 2.3 Search

Postgres FTS (`tsvector`, `websearch_to_tsquery`) + `pg_trgm` fuzzy fallback for typos.
Ranked by text match × recency × `official` boost × votes.

**Search-before-ask is enforced by the composer, not by a rule:** typing the title in
`/help/new` live-searches existing threads above the box. Roughly half of would-be posts
end at "oh, that's already answered" — which is the entire point.

Deliberately **no vector search / RAG in v1.** A few hundred threads with good titles are
served perfectly well by FTS. Revisit at ~1,000 threads, and even then consider it a
reranking layer, not a replacement.

---

## 3. Slash-lite: quick commands

Typing `/` in the ask composer opens a small menu. Selecting a command applies tags and
inserts a minimal template. **Everything is optional** — a user who ignores `/` entirely
gets a normal free-form post.

| Command | Tags applied | Template inserted |
|---|---|---|
| `/bug` | `bug` | What happened / What you expected / Analysis ID (auto-filled if in context) |
| `/quota` | `limits`, `billing` | Which limit you hit / your plan (auto-filled if signed in) |
| `/pricing` | `billing` | — |
| `/watch` | `notifications` | Which page / how often / what you expected |
| `/source` | `sources`, `quality` | Which claim / which source / what's wrong |
| `/quality` | `quality` | Analysis ID / which section / what was wrong |
| `/byok` | `byok`, `models` | Provider / what you expected / the error shown (**never** the key — the composer's secret detector blocks anything key-shaped) |
| `/feature` | `feature-request` | What you're trying to do (not what to build) |
| `/private` | — | Converts to a **private** email-backed thread |

Auto-attached context when the user arrives from in-app: analysis ID, plan, model id,
prompt version, browser, and the error id if any. It is shown before posting and is
**redactable in one click** — the user always sees exactly what they're publishing.

Why commands rather than a dropdown: they're faster for the people who use them, invisible
to those who don't, and they give the operator clean tag data for free. Tags are curated
(~20) — user-invented tags produce a long tail nobody can browse.

---

## 4. How the operator works the queue

A single `/admin/support` view, ordered by: private threads → open public threads with no
reply → threads with community replies needing blessing → flagged content.

Answering workflow, designed for ~15 minutes/day:

1. Read the thread.
2. **Check whether an `official` thread already covers it.** If so, reply with the link and
   close. (The reply itself is public, so the duplicate becomes another search entry point.)
3. If not: answer publicly. If the answer is general, click **"Promote to official"** — the
   operator edits it into a clean canonical form, retitles it as a question people actually
   search for, and it becomes KB.
4. If the thread revealed a product problem, click **"Open issue"** — links the thread to a
   GitHub issue and posts a public "tracking this" note. Users are told when it ships.

**Canned answers with variables** (`{{plan}}`, `{{quota_reset}}`, `{{analysis_id}}`) live in
admin. They exist to make the *public* answer fast, not to make private replies fast.

### 4.1 Private threads

`/private` (and `support@`) create a private thread. Rules:

- Private is for account, billing, security, legal, or data-deletion issues — anything with
  personal or payment data in it.
- After resolving a private thread, the operator is prompted: **"Publish a generalized
  version?"** — one click creates a public thread with identifiers stripped, authored as
  `official`. This is how private load converts into public deflection instead of
  evaporating.
- Target: private threads stay under 20% of total support volume by month 6.
- SLA published on `/help`: private within 2 business days; public threads usually same-day.
  Under-promise, over-deliver — an unmet SLA generates a second, angrier ticket.

---

## 5. Seeding, moderation, and quality

### 5.1 Seeding (before real users exist)

Launch `/help` with **25–30 `official` threads written by the founder**, titled as real
search queries:

- "Where does Landscape get its data?"
- "Why does my report say 'not found in public sources'?"
- "How accurate is this compared to a paid CI tool?"
- "Does Landscape respect robots.txt?"
- "How often are watched pages checked?"
- "Why did I get an alert about a change that looks cosmetic?"
- "Can I export a report to PDF or share it?"
- "What happens when I hit my monthly analysis limit?"
- "How do I cancel my subscription?"
- "Do you use my inputs to train a model?" (Answer: no — see [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md).)
- "Which AI model does Landscape use, and does my data leave your server?"
- "Why does a report take two minutes?" (Answer: honest — free hardware, and here is what
  we do to make the wait useful.)
- "Can I use my own OpenAI or Anthropic key?" (Yes — and here is exactly what leaves our
  servers if you do.)
- "My report says it fell back to the built-in model. Why?"
- "My report says a page is 'being retrieved another way' — what does that mean?"
- "My report changed after I read it. Why, and can I get the original back?" (Yes — v1 stays
  at its own URL.)
- …

An empty help section reads as an abandoned product. A seeded one reads as a considered
one, and starts earning search traffic before launch day.

### 5.2 Moderation (spam-resistant, low-effort)

- Anonymous asks require **email verification** before the post goes live.
- New accounts: rate-limited (3 posts/day), first post held for review, no links until a
  reply is accepted.
- Markdown rendered to a **safe subset** — no raw HTML, no iframes, `rel="nofollow ugc"` on
  links.
- Community flagging (`spam` / `abusive` / `off-topic`) hides content at 2 flags pending
  review.
- Public posts carry a visible reminder: *"This is public. Don't paste API keys, invoices,
  or anything private — use `/private` for that."* Plus a client-side secret-pattern
  detector that warns before posting.

### 5.3 Answer quality

The corpus is only an asset if it stays true. Two mechanisms:

- **Staleness sweep**: any `official` thread older than 180 days, or touched by a shipped
  change linked to it, appears in an admin "review" list. Stale threads show an
  *"Answer last verified 2026-03-01"* line — honest, and it makes staleness visible.
- **"Did this answer your question?"** on every thread. Two consecutive "no" votes on an
  `official` thread flags it for rewrite.

---

## 6. How support load falls over time

The intended curve, and how each mechanism bends it:

| Mechanism | Effect |
|---|---|
| Public answers indexed by Google | Users self-serve *before* contacting anyone. |
| Search-before-ask in the composer | Cuts duplicate posts at the source. |
| Contextual `?` deep-links to the exact thread | Answers the question at the moment it forms. |
| Promote-to-official | Each novel question is answered well once, forever. |
| Publish-generalized-version from private threads | Private load becomes public deflection. |
| Thread → GitHub issue → "shipped" note | Fixes the *cause*; recurring questions stop recurring. |
| Learned alert thresholds & clear limit dialogs | Removes the two largest predictable ticket sources (noise, surprise limits). |

**Targets:** by month 6, ≥60% of `/help` sessions end without a new post; new threads per
100 analyses trending down month over month; median founder support time ≤15 min/day.
If any of these move the wrong way, the fix is almost always a *product* change surfaced by
the tag distribution — which is why tags are curated and reviewed monthly.

---

## 7. Build cost

Deliberately small. The KB is roughly **one Rust module + four React routes**:
threads/replies CRUD, Postgres FTS, tags, votes, flags, SSR rendering, an admin queue, and
the slash menu. No Discourse, no Intercom, no Zendesk — each of those adds a monthly bill,
a second identity system, and content the product does not own or rank for.

Estimated: **~1.5 weeks of implementation** in Phase 3, plus ~2 days of seed-content
writing. Ongoing cost is the founder's 15 minutes a day, which is the number the whole
design exists to protect.
