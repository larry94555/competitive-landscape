# Landscape — UI Flows, Access Tiers & Community

> The seven flows the interface must support, the access model that governs them, and four
> places where these requirements change decisions already made elsewhere in the plan.
>
> **Relationship to the other documents.** [PRODUCT_SPEC.md](PRODUCT_SPEC.md) specifies the
> *analysis* experience — composer, streaming, report, PDF. This document specifies everything
> around it: who may do what and how often, the conversational follow-up layer, notifications,
> admin, and the community surface. [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) specifies the
> knowledge base that the community surface feeds.

---

## 1. What must be true of this interface

Three requirements, in priority order, because they conflict and the order decides the
conflicts:

1. **Trust before delight.** A first-time visitor must believe the output. Everything visual
   serves that: citations that invite clicking, gaps that read as rigor, timestamps that
   demonstrate freshness. A slick interface that produces unverifiable claims is worse than a
   plain one that shows its work.
2. **Curiosity without commitment.** The visitor must want to try it *before* they trust it,
   with no account and no explanation. That is the job of the one box and the three example
   chips.
3. **A good first result, even unregistered.** The anonymous run is not a crippled demo. It is
   the full nine-section report, the full PDF, the full follow-up conversation. The limit is
   *frequency*, never *quality* — a deliberately weakened free result would destroy the trust
   requirement to protect a $1 subscription.

**The learning curve is managed by having almost nothing to learn**: one textarea, one report
shape, one conversation box, one download button. Every flow below adds capability without
adding a concept the user must understand first.

---

## 2. Access model

### 2.1 The three tiers

| | **Anonymous** | **Registered** | **Subscribed ($1/mo)** |
|---|---|---|---|
| New analyses | **1 per 24h** | **1 per hour** | **5 per hour** |
| Follow-up questions | **3 per 2h** | **10 per hour** | **Unlimited** |
| Report quality | Full | Full | Full |
| PDF download | ✓ | ✓ | ✓ |
| Nine-section report | ✓ | ✓ | ✓ |
| History | Session only | Permanent | Permanent |
| Watches / notifications | ✗ | ✓ | ✓ (more, faster) |
| Community — read | ✓ | ✓ | ✓ |
| Community — post | 2/h, two channels only | ✓ all channels | ✓ all channels |

**Every number in this table is configurable at runtime** from the admin view (§6), with no
deploy. They are stored in a `limits` table with the values above as seed defaults, and every
enforcement point reads from that table rather than a constant. This is deliberate: the right
numbers are unknowable before real traffic, and the cost of guessing wrong is either a
throttled product or an exhausted free tier.

### 2.2 How limits are presented

Never as a wall arriving without warning:

- The composer shows remaining capacity **before** submission, quietly — *"1 analysis
  available · resets in 4h"*.
- On exhaustion, the message states the specific limit hit, when it resets, and exactly what
  the next tier changes — *"You've used today's analysis. It resets at 14:20. A free account
  gives you one per hour."*
- **Follow-ups and analyses are counted separately**, because they cost differently (§3.3) and
  because conflating them would make the cheap action feel as scarce as the expensive one.
- Nothing is charged for a failed analysis, and pass-2 completion never consumes quota
  ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1).

---

## 3. Flow 1–3: the analysis experience (identical across all three tiers)

The composer, streaming, report, citations and PDF are specified in
[PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1 and §4.2 and do not vary by tier. What is new here is
the layer after the report.

### 3.1 Conversational follow-up — the significant addition

After a report completes, a conversation box sits beneath it: *"Ask about these results."*
The user can interrogate the report in natural language, and the local model answers.

**This is a genuinely strong addition, and it changes the economics of the wait.** The
product's biggest UX liability is 90–180 seconds to first report (R12). A follow-up
conversation converts that from a cost into an *investment*: you wait once, then ask ten
questions that each answer in seconds. It reframes the product from "slow report generator"
to "research session with a fast analyst" — which is both more accurate and more defensible.

### 3.2 The grounding rule that makes follow-up safe

**A follow-up answer may only cite sources already fetched for that report.**

This single rule preserves the entire verification model
([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §2, [FACT_CHECKING.md](FACT_CHECKING.md)):
every claim in a follow-up still traces to a hashed, snapshotted, timestamped source that was
verified during the original analysis. Follow-ups run through the same Layer 3 quote
verification; an unsupported answer is dropped exactly as an unsupported report claim is.

When a question cannot be answered from the existing sources, the system says so and offers
the only honest alternative:

> That isn't covered by the sources I fetched for this report. I could run a new analysis
> focused on **pricing history** — that would use one of your analyses. [Run it] [No thanks]

**Without this rule, conversational follow-up would quietly become the product's largest
hallucination surface** — an open-ended chat with no retrieval gate, undoing the discipline
every other document is built around. With it, follow-up is the *safest* part of the system,
because it operates entirely over already-verified material.

### 3.3 Why follow-ups are cheap, and how they stay cheap

Follow-ups cost a fraction of an analysis: **no fetching, no extraction, no discovery, no
chart rendering.** They are grounded Q&A over content already in the cache. On Rung 0 that is
the difference between ~120 seconds and ~5–15 seconds.

Two controls keep them cheap as a conversation grows:

- **Context is span-selected, not accumulated.** Each follow-up retrieves the most relevant
  spans from the report's existing sources rather than replaying the whole conversation and
  every source. Otherwise prefill grows every turn — and prefill is the binding constraint on
  four ARM cores ([ARCHITECTURE.md](ARCHITECTURE.md) §4.4).
- **Conversation history is summarized past a threshold** (roughly 6 turns), keeping the last
  turns verbatim and compressing earlier ones.

### 3.4 Follow-ups and the PDF

The PDF is the report, not the conversation — a forwarded document should not contain someone
else's exploratory questions. **Optional:** *"Include my Q&A as an appendix"* on the full PDF
variant, off by default.

---

## 4. Flow 4: notifications

Specified in [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §5. The mechanism is unchanged: a change is
detected, importance-scored, and delivered with a short summary and a link back.

**Channel availability by tier:** email for registered and subscribed; **SMS is deferred** —
see §8.3 for why, and what to do instead.

The return experience matters as much as the alert. Clicking through lands on the **change
itself** — the diff, dated, with both versions — not on a generic dashboard. From there, one
click re-runs the analysis with the new information.

---

## 5. Flow 5: admin

A single role-gated area at `/admin`, available to a **hard-coded list of email addresses read
from a configuration file that is never committed**.

```toml
# /etc/landscape/admin.toml   (mode 0600, EnvironmentFile-adjacent, .gitignored)
admins = ["founder@example.com"]
```

Consistent with the existing secrets posture ([ARCHITECTURE.md](ARCHITECTURE.md) §11.4):
no admin flag in the database, no privilege escalation path through the application, and no
way to grant admin access without shell access to the host. The file is read at startup and
on `SIGHUP`; a malformed file **fails closed** (no admins) rather than open.

**What admin provides:**

| View | Contents |
|---|---|
| **Limits** | Every number in §2.1, editable, effective immediately, with an audit log of who changed what and when |
| **Usage** | Analyses and follow-ups per period by tier; queue depth; capacity headroom; cache hit rates |
| **Quality** | The daily 5-report sample ([QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) §3.4); claim drop rates; 👎 feedback |
| **Operations** | Inference health, render/archive queues, failed jobs, `llama-server` restarts |
| **Community** | Flagged posts, the support queue, promote-to-KB |
| **Billing** | Subscriber count, churn, MRR against the §6.1 infrastructure ratio |

---

## 6. Flow 6: community ("slash-lite")

### 6.1 Fixed channels

Seven, curated, not user-creatable:

| Channel | Read | Post |
|---|---|---|
| **Welcome** | Everyone | Registered |
| **General** | Everyone | **Everyone** (anonymous: 2/hour) |
| **Troubleshooting** | Everyone | Registered |
| **Sign-up Issues** | Everyone | **Everyone** (anonymous: 2/hour) |
| **Log-in Issues** | Everyone | Registered |
| **Feedback** | Everyone | Registered |
| **Social** | Everyone | Registered |

**Sign-up Issues being open to anonymous posting is the single most important rule here**, and
it is exactly right: the people who most need that channel are by definition the people who
could not create an account. Requiring registration to report a registration problem is a
closed loop that silently loses users.

**Log-in Issues is registered-only and that is a genuine trap** — someone locked out of their
account cannot post about being locked out. **Recommendation: open Log-in Issues to anonymous
posting under the same 2/hour limit as Sign-up Issues.** The two channels have identical
access logic for identical reasons.

- **Everything is publicly readable**, without an account, and server-rendered so it is
  indexable.
- **Posts are editable after publication**, Slack-style, with an `edited` marker and the
  original retained server-side for moderation. Anonymous authors can edit within their
  session.
- Anonymous posting requires a verified email before the post appears
  ([SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) §5.2), which is what makes a 2/hour limit
  enforceable at all.

### 6.2 How channels and the knowledge base fit together

These are two different things and the plan needs both:

| | **Channels** (this document) | **Knowledge base** ([SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md)) |
|---|---|---|
| Shape | Chronological conversation | Question → canonical answer |
| Optimized for | Immediacy, community, low posting friction | **Search traffic** — a compounding acquisition channel |
| Lifespan | Scrolls away | Permanent, curated, indexed |

**The reconciliation:** channels are the **intake**; the KB is the **durable artifact**. When a
channel conversation produces a good answer, the operator promotes it into an indexed KB
thread with one click — the mechanism `SUPPORT_SYSTEM.md` §4 already specifies.

This matters commercially: the KB is one of only three compounding acquisition channels in the
plan ([DISTRIBUTION.md](DISTRIBUTION.md) §4). **Channels alone would not replace it** — chat
scrolls away and does not rank in search. Building channels *instead of* the KB would quietly
remove an acquisition channel; building them as its front door adds one.

---

## 7. Flow 7: consistency across tiers

Stated as an invariant rather than a feature, because it is the kind of thing that erodes
under pressure to convert:

> **The prompt, the report, the follow-up conversation, and the PDF are identical for
> anonymous, registered and subscribed users. Only frequency, persistence and notifications
> differ.**

No watermarks on the free PDF. No truncated sections. No "upgrade to see the rest." The
product's entire argument is that its output is trustworthy; a deliberately degraded free
output would contradict that argument at the exact moment a first-time visitor is deciding
whether to believe it.

---

## 8. Four conflicts with decisions already in the plan

Raised here rather than absorbed silently, because each changes something the roadmap
currently assumes.

### 8.1 $1/month collides with payment processing fees

Stripe charges roughly **2.9% + $0.30** per transaction. On a $1 monthly charge that is
**$0.33 — about a third of the revenue** — and it recurs monthly.

| Structure | Gross | Fees | Net | Fee share |
|---|---|---|---|---|
| $1/month, billed monthly | $12/yr | ~$3.96/yr | **~$8.04** | **33%** |
| **$12/year, billed annually** | $12/yr | ~$0.65/yr | **~$11.35** | **5.4%** |

**Recommendation: keep the $1/month price point — it is an excellent "why not" number — but
bill it annually at $12.** Present it as *"$1/month, billed yearly."* Same price to the
customer, six times the net revenue.

If monthly billing is required for conversion, the fee is real but survivable at validation
scale; it should simply be a conscious choice rather than a surprise. Either way the roadmap's
revenue targets need restating: at $1/month net ~$0.67, **$2k MRR requires ~3,000 subscribers**
rather than the ~100 implied by the previous $19 tier. That is a different business — it makes
the product a volume play, which in turn makes distribution (R9) even more decisive than it
already was.

### 8.2 Hourly rate limits exceed Rung 0 capacity

Rung 0 sustains roughly **60–120 analyses per day** ([ARCHITECTURE.md](ARCHITECTURE.md) §11.1).
A registered user permitted 1 analysis per hour could consume 24 of those alone; **five active
registered users could saturate the entire box.**

The per-user limits are still correct — they exist to bound individual abuse, not to allocate
capacity. But they cannot be the only control.

**Recommendation:** keep the stated limits, and make explicit that the **global admission
controller** ([ARCHITECTURE.md](ARCHITECTURE.md) §6) is what actually protects the system —
queueing with an honest position display, reserved capacity for subscribers, and anonymous
requests shed first under load. Until Rung 1, the honest framing to users is *"1 per hour,
subject to available capacity"*, with the queue visible rather than hidden.

### 8.3 SMS notifications are not free

SMS costs roughly **$0.008 per message** via the usual providers, plus a number rental. At a
$1/month subscription netting ~$0.67, **a subscriber receiving two alerts a week would consume
~10% of their own subscription in SMS fees** — and it is a usage-scaling cost, which the cost
ladder currently has only one of ([ROADMAP.md](ROADMAP.md) §6.4).

**Recommendation: email at launch; defer SMS to Rung 2**, where revenue supports it, and offer
it as a subscriber-only option with a monthly cap. **Web push** is a free alternative worth
considering first — it reaches phones, costs nothing, and requires no phone number, though it
requires the user to have visited and granted permission.

### 8.4 The registered tier moves from monthly quota to hourly rate

The plan previously specified 10 analyses/month for registered users; this specifies 1/hour
(~720/month theoretical). That is a **~70× increase in the permitted ceiling**, and it changes
the free tier from "generous sample" to "effectively unlimited for normal use."

That may be exactly right for a validation phase, where usage is the goal and capacity is not
yet contended. It is worth being deliberate about, because it also removes most of the reason
to subscribe: if registered users can run one analysis an hour, **5/hour is not a compelling
upgrade.** The subscription's real value then rests on unlimited follow-ups, notifications and
priority — which should be what the upgrade prompt actually says.

---

## 9. What this changes elsewhere

| Document | Change required |
|---|---|
| [PRODUCT_SPEC.md](PRODUCT_SPEC.md) | Follow-up conversation UI; limits presentation; §2.2/2.3 tier descriptions restated against §2.1 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | `conversations` / `messages` tables; `limits` table; span-selection retrieval for follow-ups; admin config loading |
| [QUALITY_GUARDRAILS.md](QUALITY_GUARDRAILS.md) | Follow-up answers verified by the same Layer 3 mechanism; golden set gains follow-up cases |
| [SUPPORT_SYSTEM.md](SUPPORT_SYSTEM.md) | Channels as KB intake (§6.2); the promote-to-KB path already exists |
| [ROADMAP.md](ROADMAP.md) | Revenue model restated (§8.1); follow-up layer scheduled; community channels scheduled |
| [DISTRIBUTION.md](DISTRIBUTION.md) | Volume-play implications of $1 pricing on required signup counts |

**Suggested phasing** for the new surface, consistent with the existing plan: follow-up
conversation in **Phase 2** (it is the strongest mitigation for the latency problem and should
not wait); limits table and admin in **Phase 3** (alongside accounts); community channels in
**Phase 3** (alongside the KB they feed); subscription in **Phase 4**; notifications unchanged
in **Phase 5**.
