# Landscape — The Demo Walkthrough

> The shot-by-shot design for the demo films. **This is the thing to get right before any
> footage is recorded.** Narration wording can be tuned afterwards; structure cannot.
>
> Writing rules live in [Video_Text_Best_Practices.md](Video_Text_Best_Practices.md); film
> craft in [Video_Guidelines.md](Video_Guidelines.md). This document is neither — it is the
> **script outline**: what happens, in what order, and what each beat is for.

---

## 1. What was wrong, stated plainly

The previous films failed for one reason, and it is worth naming precisely because every rule
below exists to prevent it:

> **They showed results without ever saying why a result matters.**

Symptoms, all downstream of that:

| Symptom | Cause |
|---|---|
| Visuals flash past in 4 seconds | Too many beats, each too short to absorb |
| Nothing is explored in detail | The film was organised by *feature*, not by *result* |
| Abstract, not concrete | Narration described mechanisms instead of pointing at a number on screen |
| Assumes the viewer already knows | No beat exists for "why this helps you" |
| The UI looks busy | Chips, URL hints and advanced options compete with the one thing that matters |

**The core correction:** every result gets *two* beats, not one. What it is, then why it helps.
That roughly halves how much a film can cover, which is the point — the previous films covered
too much.

---

## 2. The arc — every film has the same five parts

This shape repeats in **every** film, so a viewer who watches any one of them knows how the
next will go. It is also why each film stands alone: someone can watch film 6 without film 1.

| # | Part | Job | Length |
|---|---|---|---|
| 1 | **The recognition** | A question that makes them think *that's me* | ~5s |
| 2 | **The frustration** | The problem, stated so they think *yes, I need that* | ~5s |
| 3 | **The turn** | *Here's how it works now* — and the action happens on screen | ~6s |
| 4 | **The result** | What came back. Concrete, on screen, named | ~6s |
| 5 | **The payoff** | Each result: **what it is**, then **why it helps** | ~10s per result |

**Part 5 is the film.** Parts 1–4 exist to earn it. A film covers **two to four results**, no
more. If it needs five, it is two films.

### 2.1 The payoff beat, in detail

The pattern for every single result on screen:

```
[point]  What you are looking at.              ~8 words, names a thing visible on screen
[why]    Why that helps you.                   ~12 words, names a decision or a saving
```

**Both lines are mandatory.** The failure mode of the previous films was shipping `[point]`
alone and assuming `[why]` was obvious. It never is.

The `[why]` line must survive this test: **would a person who has never thought about
competitive research understand what they now can do that they could not before?** If the
answer needs any background, rewrite it.

| Weak `[why]` | Strong `[why]` |
|---|---|
| "Sources are verifiable." | "You can check any number in ten seconds, so you can put it in front of someone else." |
| "We surface unmet needs." | "Six people asked for this and nobody sells it. That is the closest thing to customers telling you what to build." |
| "Pricing is primary-sourced." | "Quote these in a plan without checking them." |

---

## 3. Pacing — the fix is mostly slowing down

| Rule | Value |
|---|---|
| Minimum time on a beat | **4 seconds**, and 6 for anything with a number in it |
| Results per film | **2–4** |
| Film length | **45–75 seconds** |
| Beats per film | **10–14**, not 18 |
| Time on screen after a scroll settles | at least 2s before the narration names the thing |

**The single most important pacing rule:** when narration points at something, that thing must
already be still on screen. Scroll first, settle, *then* speak. The previous films spoke while
the page was still moving, which is why nothing registered.

---

## 4. The UI has to get simpler first

The composer currently shows example chips, a URL hint, and an advanced-options toggle. All
three compete with the only thing that matters at that moment.

**Required prototype changes before recording:**

| Change | Why |
|---|---|
| Composer shows **one label: "What is your idea?"**, one box, one button | Nothing else earns its place at the moment of first contact |
| **Remove the example chips** from the demo view | They imply the box is fussy about input |
| **Remove the URL / competitor-name hints** | Those are other ways in; they belong in their own film |
| **Hide the advanced-options toggle** until film 12 | An option you cannot yet evaluate reads as complexity |
| **Hide the tier pills and the top nav** during films 1–10 | They are operator controls, not user controls |
| Keep the prototype banner | Honesty about it being a prototype is non-negotiable |

The rule generalises: **at any moment, the screen shows what the current beat is about and as
little else as can be managed.** Everything removed here reappears in a later film, where it
is the subject rather than the noise.

---

## 5. The films

Twelve short films. Each is standalone, each follows §2, each explores 2–4 results properly.

> **The short path.** Films 1–4 are the core: idea in, competitors, prices, features. Someone
> who watches only those understands the product. Films 5–12 go deeper and can be watched in
> any order.

| # | Title | Results explored |
|---|---|---|
| 1 | Do you have an idea you want to explore? | The three companies |
| 2 | What do they charge? | Price table · missing price · unverified price |
| 3 | What does each one actually do? | Matrix · the `?` · the rival claim |
| 4 | What changed, and what do people wish for? | Recent changes · no changelog · unmet wants |
| 5 | What are people saying about this? | Quotes · what is built · what was abandoned |
| 6 | Where does this idea *not* turn up? | Absence panel · unlicensed venues |
| 7 | What questions should I be asking? | Level index · trajectory questions |
| 8 | Can I check any of this? | Citation · sources · what we could not read |
| 9 | Can I just ask it things? | Answer · declined answer |
| 10 | What if something changes? | Alert email · before and after |
| 11 | What does it cost? | Anonymous · free · $1 |
| 12 | Other ways to start | Name rivals · paste a site · both |

---

### Film 1 — "Do you have an idea you want to explore?"

**Length ~60s. The only film that must be perfect** — it is the one everybody watches.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Empty composer. Nothing else. | Do you have an idea you want to explore? |
| 2 | frustration | *(unchanged, held)* | Finding out who else is doing it takes a week you do not have. |
| 3 | turn | *(unchanged)* | It should be as easy as saying the idea out loud. |
| 4 | action | Text types into the box | So type it. Here: an app that helps small farms sell straight to restaurants. |
| 5 | action | Button pressed; work begins | That is the whole input. No account, no setup. |
| 6 | wait | Pages tick past in the side panel | It goes and reads the public web. About two minutes. |
| 7 | **result** | Three company names appear | Three companies already doing this. |
| 8 | **why** | *(held, still)* | You had not heard of any of them. That is the part you cannot do yourself. |
| 9 | point | The interpretation line: *"read as: ordering software for small farms"* | It says what it decided your idea was. |
| 10 | why | *(held)* | Wrong reading, wrong report. So it shows you, and you can change it. |
| 11 | close | Report scrolls slightly, showing there is more | Everything below is what it found, and where each thing came from. |
| 12 | next | *(held)* | What they charge is next. |

**Notes.** Beat 8 is the whole film — do not rush it. Beats 9–10 exist because the single
most likely failure is a wrong category, and showing that we expose it is a trust beat that
costs 8 seconds and buys the rest of the series.

---

### Film 2 — "What do they charge?"

~65s. Three results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Price table, settled | Wondering what to charge? |
| 2 | frustration | *(held)* | Pricing pages are scattered, and half of them change quietly. |
| 3 | turn | *(held)* | Here they are side by side. |
| 4 | **result** | Highlight the three prices | $39, $49, and nothing. Copied from their own price pages. |
| 5 | **why** | *(held)* | Quote them in a plan without checking. And you can see where yours would sit. |
| 6 | **result** | Highlight the Freshroute row | Freshroute publishes no price at all. |
| 7 | **why** | *(held)* | That is a fact about them, not a hole here. Companies that hide prices are usually chasing bigger customers. |
| 8 | point | *(held)* | So it says *not published* rather than guessing. |
| 9 | **result** | Scroll to the amber block | A blog says Freshroute starts at $120. |
| 10 | **why** | *(held)* | You get the number and you get to judge it. It is not from the company, so it stays out of the table. |
| 11 | close | *(held)* | Nothing found is hidden from you. Nothing unproven is presented as fact. |
| 12 | next | | What each one actually does is next. |

**Note.** Beat 7 is the strongest line in the series: it turns a missing value into an
insight. That is the model for how every gap should be narrated.

---

### Film 3 — "What does each one actually do?"

~60s. Three results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Feature matrix, settled | Trying to work out where the gap is? |
| 2 | frustration | *(held)* | Every company's feature page says everything is included. |
| 3 | turn | *(held)* | Same features, same rows, one grid. |
| 4 | **result** | Highlight the route-planning row | Only Freshroute plans delivery routes. Barnwise says it does not. |
| 5 | **why** | *(held)* | A row where one competitor says no is a row you could win on. |
| 6 | **result** | Highlight a `?` cell | A question mark means we looked and nobody said either way. |
| 7 | **why** | *(held)* | That is not a no. It is a question you could ask them, and they have not answered publicly. |
| 8 | **result** | Highlight the triangle cell | A triangle means a rival claimed it, not the company. |
| 9 | **why** | *(held)* | Competitors describe each other badly. Worth knowing before you repeat it. |
| 10 | close | *(held)* | Five different things a box can mean. Most tools use two. |
| 11 | next | | What changed recently is next. |

---

### Film 4 — "What changed, and what do people wish for?"

~65s. Three results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Changes section, settled | Timing matters. Is now a good moment? |
| 2 | frustration | *(held)* | Nobody announces that their customers just got unhappy. |
| 3 | **result** | Highlight the Barnwise price rise | Barnwise put its price up a quarter in May. |
| 4 | **why** | *(held)* | Customers look around after a price rise. That is a window, and it has a date. |
| 5 | **result** | Scroll to the Freshroute gap block | Freshroute publishes nothing about what it changed. |
| 6 | **why** | *(held)* | And it lists where it looked. So you know they are quiet, not that we missed it. |
| 7 | **result** | Scroll to *what people wish it did* | Six people asked for a different price list per restaurant. |
| 8 | **why** | *(held)* | Nobody sells that. It is the closest thing to customers telling you what to build. |
| 9 | point | *(held)* | Taken from the middling reviews, not the angry ones. |
| 10 | why | *(held)* | Furious people describe their day. Lukewarm ones describe the missing feature. |
| 11 | next | | What people are saying is next. |

**Note.** Beat 8 is the second-strongest line in the series. Give it room.

---

### Film 5 — "What are people saying about this?"

~65s. Three results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Section 5A header | Want to know if anyone else has this problem? |
| 2 | frustration | *(held)* | It is out there, spread across a hundred threads. |
| 3 | **result** | First quote, held still | Someone wanting to know what the chefs ordered last week. |
| 4 | **why** | *(held)* | That is your landing page, written by a customer. |
| 5 | **result** | Second quote | Someone tracking prices in six spreadsheets. |
| 6 | **why** | *(held)* | Both posts open in one click. You can go and ask them. |
| 7 | point | *(held)* | Two posts. We say two because we read two. |
| 8 | **result** | The active project row | Someone has already built this. Still working on it last week. |
| 9 | **why** | *(held)* | Either you have a competitor you did not know about, or a head start you can use. |
| 10 | **result** | The abandoned project row | Another was abandoned in 2024. |
| 11 | **why** | *(held)* | Find out why they stopped before you spend a year finding out yourself. |
| 12 | next | | Where the idea does *not* turn up is next. |

---

### Film 6 — "Where does this idea *not* turn up?"

~60s. Two results. **The most distinctive film — nothing else does this.**

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Absence panel, settled | Nobody tells you what they failed to find. |
| 2 | frustration | *(held)* | So you never know if it is quiet out there or if they just did not look. |
| 3 | **result** | Highlight the Hacker News row | Nothing on Hacker News. Here are the three searches we ran. |
| 4 | **why** | *(held)* | You can repeat them in two minutes and see for yourself. |
| 5 | **result** | Highlight the caveat line | And it says the silence proves nothing here. |
| 6 | **why** | *(held)* | Hacker News is software people. For a farming tool, they were never going to be talking. |
| 7 | point | Lobsters row | A place with nothing to say about farming is left out entirely. |
| 8 | why | *(held)* | Padding the list would make a thin search look thorough. |
| 9 | **result** | The unlicensed block | X, LinkedIn and Reddit are not licensed to us. |
| 10 | **why** | *(held)* | So we say so, and hand you the searches. You know exactly what we did not look at. |
| 11 | close | *(held)* | A report that hides its gaps is one you cannot calibrate. |
| 12 | next | | The questions it raises are next. |

---

### Film 7 — "What questions should I be asking?"

~60s. Two results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Level index, settled | New to this? You do not know what you have not thought about. |
| 2 | **result** | The eight rows | Eight things get checked. Competitors are one. |
| 3 | **why** | *(held)* | The other seven are about your idea, not their companies. |
| 4 | **result** | The two `?` rows | Two are marked with a question mark. |
| 5 | **why** | *(held)* | Those we do not answer. A small model guessing at strategy would be worse than useless. |
| 6 | turn | Scroll to trajectory block | It raises the questions instead. |
| 7 | **result** | The three questions | This one needs farms and restaurants before it works for either. |
| 8 | **why** | *(held)* | Which side you get first is the decision that sinks most two-sided businesses. |
| 9 | point | The evidence line above the questions | It says what made it think so, and links it. |
| 10 | point | The book line | The questions come from a book we did not summarise. |
| 11 | why | *(held)* | If this is your situation, read it. These are month-six questions, in front of you now. |
| 12 | next | | Checking any of it is next. |

---

### Film 8 — "Can I check any of this?"

~55s. Three results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Report body | Would you put this in front of an investor? |
| 2 | frustration | *(held)* | Not if you cannot say where any of it came from. |
| 3 | **result** | Click a citation; card opens and holds | Every number opens like this. |
| 4 | **why** | *(held)* | The page, the exact sentence, and the minute it was read. |
| 5 | why | *(held)* | Ten seconds to check one. So you can hand the whole thing to someone else. |
| 6 | **result** | Sources list | Every page it read, listed. |
| 7 | **result** | The could-not-read line | Including one it was not allowed to read. |
| 8 | **why** | *(held)* | It still gives you the link. A person can open what a program cannot. |
| 9 | close | *(held)* | Nothing found is withheld. |
| 10 | next | | Asking it questions is next. |

---

### Film 9 — "Can I just ask it things?"

~50s. Two results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Follow-up box | Reports never answer the exact thing you wanted. |
| 2 | turn | Question types in | So ask. What would I pay for 40 orders a month? |
| 3 | **result** | Answer appears with citations | It works it out from what it already read. |
| 4 | **why** | *(held)* | The pages are read once. Answers come back in seconds, not minutes. |
| 5 | turn | Second question types in | Now something it cannot know. |
| 6 | **result** | The declined answer | It says the pages do not cover that. |
| 7 | **why** | *(held)* | It offers to go and look rather than inventing something plausible. |
| 8 | close | *(held)* | A tool that says *I do not know* is one you can trust when it does answer. |
| 9 | next | | Watching for changes is next. |

---

### Film 10 — "What if something changes?"

~50s. Two results.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Report actions | This is true today. |
| 2 | frustration | *(held)* | Competitors change prices without telling anyone. |
| 3 | turn | Watch created in two clicks | Two clicks to watch it. |
| 4 | **result** | The alert email | An email when something you care about moves. |
| 5 | **why** | *(held)* | Not a digest. One change, one email, the thing that changed in the subject line. |
| 6 | **result** | Before-and-after block | It shows both versions, both dated. |
| 7 | **why** | *(held)* | You can see it happened rather than take our word for it. |
| 8 | next | | What it costs is next. |

---

### Film 11 — "What does it cost?"

~45s.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Composer, anonymous | You have not signed in to anything yet. |
| 2 | **result** | The quota line | One report a day without an account. |
| 3 | **why** | *(held)* | The whole report. Nothing held back, nothing stamped across it. |
| 4 | **result** | Free account state | An account gets you one an hour, and keeps your history. |
| 5 | **result** | Subscribed state | A dollar a month gets five an hour. |
| 6 | **why** | *(held)* | Same report at every level. You pay for how often, never for how much. |
| 7 | next | | Other ways to start is last. |

---

### Film 12 — "Other ways to start"

~45s. **This is where the advanced options finally appear** — after the viewer knows what a
report is and can judge whether they need them.

| # | Beat | On screen | Narration |
|---|---|---|---|
| 1 | recognition | Composer | If you already know the market, say so. |
| 2 | turn | Advanced options open | These were hidden until now on purpose. |
| 3 | why | *(held)* | An option you cannot judge yet just looks like complexity. |
| 4 | **result** | Competitor names typed | Name the rivals yourself. |
| 5 | **result** | URL typed | Or paste one website and let it find the rest. |
| 6 | **result** | Both typed | Or both. |
| 7 | **why** | *(held)* | Same report either way. The box was never the hard part. |
| 8 | close | Back to the empty box | One question. Now try it yourself. |

---

## 6. What changes in the build

| Change | Where |
|---|---|
| Composer reduced to one label, one box, one button | `ui-prototype.html` — new `clean` mode |
| Chips, URL hints, advanced toggle hidden by default | same |
| Tier pills and nav hidden during films 1–10 | same |
| Highlight helper — briefly outline the element being discussed | new; the films depend on it |
| Scroll-then-settle: 2s pause between scroll and narration | `DEMO` step timings |
| 12 chapters replacing 4 | `CHAPTERS` |
| ~130 narration lines replacing 49 | `narration.md` |

**The highlight helper is the one genuinely new mechanism.** Films 2–8 all depend on pointing
at a specific row, cell, or block while the narration names it. Without it the viewer is
hunting for what is being described, which is a large part of why the current films feel
abstract.

---

## 7. The standard this has to meet

Before recording, every film is checked against this. Any *no* is a rewrite, not a tweak.

- [ ] Does beat 1 make someone think **that's me**?
- [ ] Does beat 2 name a frustration they have actually had?
- [ ] Is the UI at beat 3 showing **only** what beat 3 is about?
- [ ] Does **every** result have both a `[point]` and a `[why]`?
- [ ] Does every `[why]` name a decision, a saving, or a risk avoided?
- [ ] Is anything being pointed at **still** on screen before it is named?
- [ ] Are there **four or fewer** results?
- [ ] Could someone who has never used a research tool follow it start to finish?
- [ ] Is there a single sentence that assumes prior knowledge? Remove it.
- [ ] Read aloud: does any beat feel rushed? Add two seconds.
