# Landscape — The Demo Walkthrough

> The shot-by-shot design for the demo films — **and the file the build reads.**
> `python prototype/build.py` parses the tables in §5, computes every timing from the pacing
> rules in §3, and generates the demo steps, the chapters and the subtitles. Editing a line of
> narration here changes the film.
>
> Writing rules live in [Video_Text_Best_Practices.md](Video_Text_Best_Practices.md); film
> craft in [Video_Guidelines.md](Video_Guidelines.md).

---

## 1. What was wrong, stated plainly

The previous films failed for one reason, and every rule below exists to prevent it:

> **They showed results without ever saying why a result matters.**

| Symptom | Cause |
|---|---|
| Visuals flash past in 4 seconds | Too many beats, each too short to absorb |
| Nothing is explored in detail | The film was organised by *feature*, not by *result* |
| Abstract, not concrete | Narration described mechanisms instead of pointing at something on screen |
| Assumes the viewer already knows | No beat existed for "why this helps you" |
| The UI looks busy | Chips, URL hints and advanced options competed with the one thing that mattered |

**The correction:** every result gets *two* beats. What it is, then why it helps. That halves
what a film can cover, which is the point.

---

## 2. The arc — every film has the same five parts

| # | Part | Job |
|---|---|---|
| 1 | **recognition** | A question that makes them think *that's me* |
| 2 | **frustration** | The problem, stated so they think *yes, I need that* |
| 3 | **turn** | *Here's how it works now* — and the action happens on screen |
| 4 | **result** | What came back. Concrete, on screen, named |
| 5 | **why** | Why that result helps them |

**The payoff is the film.** A film covers **two to four results**, no more. If it needs five,
it is two films.

Every `result` must be followed by a `why`. **The build fails otherwise** — that is the one
rule worth enforcing mechanically, because it is the rule the old films broke.

The `why` line must survive: *would someone who has never thought about competitive research
understand what they can now do that they could not before?*

| Weak `why` | Strong `why` |
|---|---|
| "Sources are verifiable." | "Check any number in ten seconds, so you can put it in front of someone else." |
| "We surface unmet needs." | "Six people asked and nobody sells it. That is customers telling you what to build." |
| "Pricing is primary-sourced." | "Quote these in a plan without checking them." |

---

## 3. Pacing — computed, not authored

Timings are **derived** from these rules. No step time is written by hand.

| Rule | Value |
|---|---|
| Base beat | **4s** |
| `result` and `why` beats | **6s** |
| Any line containing a number | **at least 6s** |
| After a scroll or highlight | **+2s** to settle before the narration lands |
| Reading room | at least `characters ÷ 14` seconds, plus 1 |
| Film length | **45–75s** |
| Results per film | **2–4** |

**Scroll, settle, then speak.** The old films narrated while the page was still moving, which
is most of why nothing registered. The +2s is that rule made mechanical.

---

## 4. The UI gets simpler first

`clean` puts the prototype into demo dress: one label, one box, one button.

| Hidden by `clean` | Why |
|---|---|
| Example chips | They imply the box is fussy about input |
| The URL / competitor placeholder text | Those are other ways in — film 12 |
| Advanced options | An option you cannot yet judge reads as complexity |
| Tier pills, top nav, quota line | Operator controls, not user controls |
| The marketing sub-heading | The question is the message |
| *(kept)* The prototype banner | Honesty about it being a prototype is non-negotiable |

The rule generalises: **the screen shows what the current beat is about and as little else as
can be managed.** Everything hidden here returns in a later film, as the subject.

---

## 5. The films

Each row is one beat. `kind` drives the timing; `action` drives the picture.

**Action vocabulary** — the complete set the prototype implements:

| Action | Effect |
|---|---|
| `hold` | Nothing changes. The frame stays still |
| `clean` | Demo dress (§4), empty box |
| `type` | The example idea types into the box |
| `go` | Analyse pressed; the simulated run starts |
| `report` | A finished report appears instantly, for films starting mid-story |
| `to:<sel>` | Scroll that element into view |
| `spot:<sel>` | Scroll it in **and outline it**, dimming everything else |
| `cite:<sel>` | Open the citation card on that reference |
| `ask:<text>` | Type and send a follow-up question |
| `tier:<anon\|reg\|sub>` | Switch account level |
| `view:<name>` | Show `limit`, `notify`, `community`, `admin` or `composer` |
| `adv` | Open the advanced options |
| `diff` | Reveal the before-and-after block |

> **The short path.** Films 1–4 are the core: idea in, competitors, prices, features. Someone
> who watches only those understands the product.

---

### Film 1 · `idea` — Do you have an idea you want to explore?

**Blurb.** Someone types a business idea in plain words and finds three companies already doing it.

*The only film that must be perfect — it is the one everybody watches.*

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | Do you have an idea you want to explore? |
| frustration | `hold` | Finding out who else is doing it takes a week you do not have. |
| turn | `hold` | It should be as easy as saying the idea out loud. |
| action | `type` | So type it. Here: an app helping small farms sell straight to restaurants. |
| action | `go` | That is the whole input. No account, no setup. |
| wait | `hold` | It goes and reads the public web. About two minutes. |
| result | `spot:#srcLive` | Three companies already doing this. |
| why | `hold` | You had not heard of any of them. That is the part you cannot do yourself. |
| result | `spot:#interpLine` | It shows what it decided your idea was. |
| why | `hold` | Wrong reading, wrong report. So you get to see it, and change it. |
| close | `to:#report` | Everything below is what it found, and where each piece came from. |
| next | `hold` | What they charge is next. |

*Beat 8 is the film. Beats 9–10 cost eight seconds and buy the rest of the series: a wrong
category reading silently ruins everything, so showing that we expose it is the trust beat.*

---

### Film 2 · `prices` — What do they charge?

**Blurb.** Three prices, one company that publishes none, and a figure from a blog that stays out of the table.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Wondering what to charge? |
| frustration | `to:#sec-pricing` | Pricing pages are scattered, and half of them change quietly. |
| turn | `hold` | Here they are side by side. |
| result | `spot:#sec-pricing table` | Thirty-nine dollars, forty-nine, and nothing. Copied from their own price pages. |
| why | `hold` | Quote them in a plan without checking. And you can see where yours would sit. |
| result | `spot:#sec-pricing tbody tr:nth-child(4)` | Freshroute publishes no price at all. |
| why | `hold` | That is a fact about them, not a hole here. Companies that hide prices chase bigger customers. |
| point | `hold` | So it says not published, rather than guessing. |
| result | `spot:#sec-pricing .unconfirmed` | A blog says Freshroute starts at a hundred and twenty. |
| why | `hold` | You get the number and you get to judge it. Not from the company, so it stays out of the table. |
| next | `hold` | Nothing unproven is dressed up as fact. What each one does is next. |

*Beat 7 is the strongest line in the series: it turns a missing value into an insight. That is
the model for how every gap should be narrated.*

---

### Film 3 · `features` — What does each one actually do?

**Blurb.** The feature grid, the cells that mean "nobody said", and the ones where only a rival is claiming it.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Trying to work out where the gap is? |
| frustration | `to:#sec-matrix` | Every company's feature page says everything is included. |
| turn | `hold` | Same rows, same questions, one grid. |
| result | `spot:#sec-matrix tbody tr:nth-child(3)` | Only Freshroute plans delivery routes. Barnwise says it does not. |
| why | `hold` | A row where a competitor says no is a row you could win on. |
| result | `spot:#sec-matrix .cell-u` | A question mark means we looked and nobody said either way. |
| why | `hold` | That is not a no. It is a question they have never answered in public. |
| result | `spot:#sec-matrix .cell-c` | A triangle means a rival claimed it, not the company. |
| why | `hold` | Competitors describe each other badly. Worth knowing before you repeat it. |
| close | `hold` | Five things a box can mean. Most comparisons manage two. |
| next | `hold` | What changed recently is next. |

---

### Film 4 · `changes` — What changed, and what do people wish for?

**Blurb.** A price rise with a date, a company that publishes nothing, and six people asking for a feature nobody sells.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Timing matters. Is now a good moment? |
| frustration | `to:#sec-changes` | Nobody announces that their customers just got unhappy. |
| result | `spot:#sec-changes p:nth-of-type(2)` | Barnwise put its price up a quarter in May. |
| why | `hold` | Customers look around after a price rise. That is a window, and it has a date. |
| result | `spot:#sec-changes .gap` | Freshroute publishes nothing about what it changed. |
| why | `hold` | And it lists where it looked. So you know they are quiet, not that we missed it. |
| result | `spot:#sec-sent` | Six people asked for a different price list per restaurant. |
| why | `hold` | Nobody sells that. It is the closest thing to customers telling you what to build. |
| point | `hold` | Taken from the middling reviews, not the angry ones. |
| why | `hold` | Furious people describe their day. Lukewarm ones describe the missing feature. |
| next | `hold` | What people are saying is next. |

*Beat 8 is the second-strongest line in the series. Give it room.*

---

### Film 5 · `talking` — What are people saying about this?

**Blurb.** People describing the problem in their own words, one project still being built, and one abandoned.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Want to know whether anyone else has this problem? |
| frustration | `to:#sec-talk` | It is out there, spread across a hundred threads. |
| result | `spot:#sec-talk .quote` | Someone wanting to know what the chefs ordered last week. |
| why | `hold` | That is your landing page, written by a customer. |
| result | `spot:#sec-talk .quote:nth-of-type(2)` | Someone else keeping prices in six spreadsheets. |
| why | `hold` | Two posts, both one click away. We say two because we read two. |
| result | `spot:#sec-talk .proj` | Somebody has already built this. Still working on it last week. |
| why | `hold` | Either you have a competitor you did not know about, or a head start you can use. |
| result | `spot:#sec-talk .proj.dead` | Another was abandoned in 2024. |
| why | `hold` | Find out why they stopped before you spend a year finding out yourself. |
| next | `hold` | Where the idea does not turn up is next. |

---

### Film 6 · `absence` — Where does this idea not turn up?

**Blurb.** The searches that found nothing, why that sometimes proves nothing, and the three places we have not licensed.

*The most distinctive film — nothing else on the market does this.*

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Nobody tells you what they failed to find. |
| frustration | `to:#sec-talk .absent` | So you never learn whether it is quiet out there or they just did not look. |
| result | `spot:#sec-talk .absent .row` | Nothing on Hacker News. Here are the three searches we ran. |
| why | `hold` | Repeat them in two minutes and see for yourself. |
| result | `spot:#sec-talk .caveat` | And it says the silence proves nothing here. |
| why | `hold` | Hacker News is software people. For a farming tool they were never going to be talking. |
| point | `spot:#sec-talk .absent .row:nth-child(2)` | Somewhere with nothing to say about farming is left out entirely. |
| why | `hold` | Padding the list would make a thin search look thorough. |
| result | `spot:#sec-talk .unsearched` | X, LinkedIn and Reddit are not licensed to us. |
| why | `hold` | So we say so, and hand you the searches. You know what we did not look at. |
| next | `hold` | The questions it raises are next. |

---

### Film 7 · `questions` — What questions should I be asking?

**Blurb.** Eight levels of checking, the two we deliberately do not answer, and the questions raised instead.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | New to this? You do not know what you have not thought about. |
| result | `spot:#sec-levels .levels` | Eight things get checked. Competitors are one of them. |
| why | `hold` | The other seven are about your idea, not their companies. |
| result | `spot:#sec-levels .g-q` | Two are marked with a question mark. |
| why | `hold` | Those we do not answer. A small model guessing at strategy would be worse than useless. |
| turn | `to:#sec-traj` | So it raises the questions instead. |
| result | `spot:#sec-traj .ask ul` | This one needs farms and restaurants before it works for either. |
| why | `hold` | Which side you get first is the decision that sinks most two-sided businesses. |
| point | `spot:#sec-traj .why` | It says what made it think so, and links it. |
| point | `spot:#sec-traj .book` | The questions come from a book we did not summarise. Read it. |
| next | `hold` | These are month-six questions, now. Checking any of it is next. |

---

### Film 8 · `checking` — Can I check any of this?

**Blurb.** Every number opens onto the sentence it came from, including the one page we were not allowed to read.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Would you put this in front of an investor? |
| frustration | `to:#sec-swot` | Not if you cannot say where any of it came from. |
| result | `cite:#sec-swot .cite` | Every number opens like this. |
| why | `hold` | The page, the exact sentence, and the minute it was read. |
| why | `hold` | Ten seconds to check one. So you can hand the whole thing to somebody else. |
| point | `spot:#sec-sources table` | Every page it read, listed. |
| result | `spot:#sec-sources p:nth-of-type(1)` | Including one it was not allowed to read. |
| why | `hold` | It still gives you the link. A person can open what a program cannot. |
| close | `hold` | Nothing it found is withheld from you. |
| next | `hold` | Asking it questions is next. |

---

### Film 9 · `asking` — Can I just ask it things?

**Blurb.** Follow-up questions answered from what it already read, and one it refuses to answer.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Reports never answer the exact thing you wanted. |
| turn | `to:#followup` | So ask. |
| action | `ask:What would I pay for 40 orders a month?` | What would I pay for forty orders a month? |
| result | `hold` | It works it out from what it already read. |
| why | `hold` | The pages are read once. Answers come back in seconds, not minutes. |
| turn | `ask:How do they handle food safety records?` | Now something it cannot know. |
| result | `hold` | It says the pages do not cover that. |
| why | `hold` | And offers to go and look, rather than inventing something plausible. |
| close | `hold` | A tool that says I do not know is one you can trust when it answers. |
| next | `hold` | Watching for changes is next. |

---

### Film 10 · `alerts` — What if something changes?

**Blurb.** An email when a competitor moves, showing both versions with dates.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This is all true today. |
| frustration | `hold` | Competitors change prices without telling anybody. |
| turn | `view:notify` | Two clicks to watch it. Nothing to configure. |
| result | `hold` | An email when something you care about moves. |
| why | `hold` | Not a digest. One change, one email, the thing that changed in the subject line. |
| point | `hold` | Off by default, except for prices. |
| why | `hold` | A tool that mails you every day gets filtered, and then it is worth nothing. |
| result | `diff` | It shows both versions, both dated. |
| why | `hold` | You can see that it happened rather than take our word for it. |
| next | `hold` | What it costs is next. |

---

### Film 11 · `cost` — What does it cost?

**Blurb.** One report a day with no account, and what an account or a dollar a month changes.

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | You have not signed in to anything yet. |
| frustration | `hold` | Most tools want an account before they show you anything. |
| result | `view:limit` | One report a day without an account. |
| why | `hold` | The whole report. Nothing held back, nothing stamped across it. |
| result | `tier:reg` | An account gets you one an hour, and keeps your history. |
| why | `hold` | You only sign up when you want the second report, not before. |
| result | `tier:sub` | A dollar a month gets you five an hour. |
| why | `hold` | The same report at every level. You pay for how often, never for how much. |
| next | `hold` | Other ways to start is last. |

---

### Film 12 · `ways` — Other ways to start

**Blurb.** Naming the rivals, pasting a website, or both — revealed only once you know what a report is.

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | If you already know the market, say so. |
| turn | `adv` | These were hidden until now, on purpose. |
| why | `hold` | An option you cannot judge yet just looks like complexity. |
| result | `type:Croptally vs Barnwise` | Name the rivals yourself. |
| why | `hold` | It skips working out who competes with you and goes straight to reading them. |
| result | `type:https://croptally.example` | Or paste one website. |
| why | `hold` | It reads that company, works out the market, and finds the others itself. |
| point | `type:an app for farm-to-restaurant orders, like Croptally` | Or both at once, in one sentence. |
| close | `clean` | The same report either way. One question. Now try it yourself. |

---

## 6. How the build works

```
docs/Demo_Walkthrough.md          ← the only place films are authored
        │
        │  python prototype/build.py
        ▼
prototype/ui-prototype.html       generated DEMO + CHAPTERS block
prototype/video/*.vtt             one subtitle track per film
prototype/demo-*.html             one page per film
```

`build.py --check` validates without writing, and **fails** on:

- a `result` beat with no `why` after it
- more than 4 results in a film
- a film outside 45–75 seconds
- a caption too long to read in its own slot
- banned vocabulary ([Video_Guidelines.md](Video_Guidelines.md) §2.2)
- an action referring to a selector the prototype does not contain

`--preview` lists every film with its computed timings, so pacing can be judged without
recording anything. In the browser, `ui-prototype.html?film=<id>` plays one film at real speed
with a beat inspector.

**Re-record only when the picture changes.** Wording, timing and ordering all flow from this
file without new footage.

---

## 7. The standard

Checked before recording. Anything failing is a rewrite, not a tweak. The first five are
enforced by `--check`; the rest need a person.

- [ ] Every `result` is followed by a `why`
- [ ] Four or fewer results per film
- [ ] Every film lands between 45 and 75 seconds
- [ ] Every caption is readable in the time it is on screen
- [ ] Every selector exists in the prototype
- [ ] Does beat 1 make someone think *that's me*?
- [ ] Does beat 2 name a frustration they have actually had?
- [ ] Does every `why` name a decision, a saving, or a risk avoided?
- [ ] Could someone who has never used a research tool follow it start to finish?
- [ ] Any sentence assuming prior knowledge? Remove it.
- [ ] Read aloud: does any beat feel rushed?
