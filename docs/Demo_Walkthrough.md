# Landscape — The Demo Walkthrough

> The shot-by-shot design for the demo films — **and the file the build reads.**
> `python prototype/build.py` parses the tables in §5, computes every timing from the pacing
> rules in §3, and generates the demo steps, the chapters and the subtitles. Editing a line of
> narration here changes the film.
>
> Writing rules live in [Video_Text_Best_Practices.md](Video_Text_Best_Practices.md); film
> craft in [Video_Guidelines.md](Video_Guidelines.md).

---

## 1. The films explain. They do not sell.

Two faults, corrected in that order.

**The first cut showed results without saying what they were.** Visuals flashed past, nothing
was explored, and every line assumed the viewer already knew why a table mattered.

**The second cut fixed that by selling** — *"quote these without checking"*, *"a week you do
not have"*, *"that is customers telling you what to build"*. Accurate, and wrong for this.
A demo is not a pitch. Someone watching wants to know **what the thing does**, and will decide
for themselves whether that is worth anything.

> **Describe what is on screen and what it means. Never claim what it is worth.**

| Never | Instead |
|---|---|
| "Quote these in a plan without checking." | "Each figure comes from that company own price page." |
| "Finding out takes a week you do not have." | "Landscape looks for companies already doing it." |
| "Nobody sells that — it is what customers want built." | "Six reviewers asked for a price list per restaurant." |
| "You can check any number in ten seconds." | "The page, the sentence it came from, and when it was read." |

The test: **no line may mention time saved, money made, risk avoided, or what the viewer could
now do.** State the fact. Stop.

**Show it working before showing it being corrected.** Film 1 spends four beats on a report
that came out right, then two on changing a wrong reading. That order matters — the reading is
right most of the time, and leading with the caveat implies otherwise.

---

## 2. The arc — every film has the same shape

| # | Part | Job |
|---|---|---|
| 1 | **recognition** | Name what this film is about |
| 2 | **turn** | What the screen is showing, and the action |
| 3 | **result** | What is there. Concrete, on screen, named |
| 4 | **means** | What that is, or where it came from — **never what it is worth** |

**A film covers two to four results**, no more. If it needs five, it is two films.

Every `result` must be followed by a `means`. **The build fails otherwise** — that is the one
rule worth enforcing mechanically, because it is the rule the first cut broke.

The `means` line answers *what is this?*, not *why should you care?* If it could appear in a
brochure, rewrite it.

---

## 3. Pacing — computed, not authored

Timings are **derived** from these rules. No step time is written by hand.

| Rule | Value |
|---|---|
| Base beat | **3.2s** |
| `result` and `means` beats | **4.8s** |
| Any line containing a number | **at least 4.8s** |
| After a scroll or highlight | **+1.6s** to settle before the narration lands |
| Reading room | at least `characters ÷ 14` seconds, plus 0.8 |
| Film length | **35–70s** |
| Results per film | **2–4** |

Every floor above is **20% shorter than the first cut**, which left too long a gap between
points. Only the *slack* was cut: the `÷ 14` reading rate is untouched, so a caption still gets
the time it takes to read. Shortening that would make captions unreadable rather than brisk.

**Scroll, settle, then speak.** The first cut narrated while the page was still moving, which
is most of why nothing registered. The +1.6s is that rule made mechanical.

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

**Blurb.** One box, one question, and a report on the companies already doing it.

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | Do you have an idea you want to explore? |
| turn | `hold` | Landscape looks for companies already doing it. |
| point | `hold` | There is one box and one question. |
| action | `type` | You type the idea in ordinary words. |
| action | `go` | Then press Analyse. That is the only step. |
| result | `spot:#interpLine` | First it settles on what to search for. |
| means | `hold` | Ordering software for small farms, taken from your wording. |
| result | `spot:#interpLine .edit` | If that is not your market, change reopens it. |
| means | `hold` | You correct it there and the search runs again. |
| wait | `hold` | Otherwise it goes and reads public web pages. |
| result | `spot:#srcLive` | Each page is listed underneath as it is read. |
| means | `hold` | Company sites, a forum, and one page it could not open. |
| next | `to:#report` | About two minutes. The report follows, and prices are next. |

*The order on screen and in the narration is the same: your words, then the words it
searched for, then what that search found. The search line sits directly under the idea
and above the pages, so editing it clearly re-runs everything below.*

*The search line comes out right most of the time, so it is stated plainly first and the
correction follows. Leading with the correction would suggest otherwise.*

---

### Film 2 · `prices` — What do they charge?

**Blurb.** The price table, a company that publishes none, and a figure from a blog kept separate.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This is the pricing section. |
| turn | `to:#sec-pricing` | Every plan each company publishes, in one table. |
| result | `spot:#sec-pricing table` | Croptally is thirty-nine dollars, Barnwise forty-nine. |
| means | `hold` | Each figure comes from that company own price page. |
| result | `spot:#sec-pricing tbody tr:nth-child(4)` | Freshroute publishes no price at all. |
| means | `hold` | The table says not published, and asks you to contact them. |
| point | `hold` | It does not fill the gap with a guess. |
| result | `spot:#sec-pricing .unconfirmed` | A blog gives a figure for Freshroute. |
| means | `hold` | It sits below the table, marked as not from the company, with a link. |
| next | `hold` | What each one does is next. |

---

### Film 3 · `features` — What does each one do?

**Blurb.** The feature grid, and what each of the five kinds of cell means.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This is the feature grid. |
| turn | `to:#sec-matrix` | The same questions asked of all three. |
| result | `spot:#sec-matrix tbody tr:nth-child(3)` | Freshroute plans delivery routes. Barnwise states it does not. |
| means | `hold` | A tick is a stated yes, a cross a stated no. |
| result | `spot:#sec-matrix .cell-u` | A question mark is neither. |
| means | `hold` | It means the pages were read and nobody said either way. |
| result | `spot:#sec-matrix .cell-c` | A triangle is a claim by a rival. |
| means | `hold` | Croptally says Freshroute does this. Freshroute does not say it. |
| next | `hold` | Recent changes are next. |

---

### Film 4 · `changes` — What has changed recently?

**Blurb.** Dated changes, a company that publishes none, and what reviewers ask for.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This section covers the last ninety days. |
| turn | `to:#sec-changes` | Each entry carries its date and its source. |
| result | `spot:#sec-changes p:nth-of-type(2)` | Barnwise moved its Grower plan up to forty-nine in May. |
| means | `hold` | Taken from their own page, on the date shown. |
| result | `spot:#sec-changes .gap` | Freshroute publishes no list of changes. |
| means | `hold` | The block lists the pages checked: news, blog, and the rest of the site. |
| result | `spot:#sec-sent` | Six reviewers asked for a price list per restaurant. |
| means | `hold` | Read from the middling reviews, where people describe what is missing. |
| next | `hold` | What people are saying is next. |

---

### Film 5 · `talking` — What are people saying?

**Blurb.** Posts from people with the problem, one project still active and one abandoned.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This section is about the problem, not the companies. |
| turn | `to:#sec-talk` | Public posts, open-source projects, and forums. |
| result | `spot:#sec-talk .quote` | A grower asking what the chefs ordered last week. |
| means | `hold` | Quoted in full, dated, and linked to the post. |
| result | `spot:#sec-talk .quote:nth-of-type(2)` | Another keeping prices in six spreadsheets. |
| means | `hold` | Two posts. It says two because it read two. |
| result | `spot:#sec-talk .proj` | An open-source project doing something similar. |
| means | `hold` | Twelve hundred changes, seven people, last worked on in July. |
| result | `spot:#sec-talk .proj.dead` | Another stopped in November 2024. |
| means | `hold` | Marked as no longer maintained, and still listed. |
| next | `hold` | Where the idea does not appear is next. |

---

### Film 6 · `absence` — Where does the idea not appear?

**Blurb.** Searches that found nothing, why some places are left out, and three sites not searched.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This part covers what was not found. |
| turn | `to:#sec-talk .absent` | Each place searched, and what came back. |
| result | `spot:#sec-talk .absent .row` | Nothing on Hacker News. |
| means | `hold` | The three searches used are printed, with the date and the window. |
| result | `spot:#sec-talk .caveat` | Below them, a note on what the silence is worth. |
| means | `hold` | Hacker News is mostly software people, so finding nothing means little. |
| point | `spot:#sec-talk .absent .row:nth-child(2)` | Lobsters is listed but not reported on. |
| means | `hold` | Nothing there covers farming, so it was left out rather than padded in. |
| result | `spot:#sec-talk .unsearched` | X, LinkedIn and Reddit were not searched. |
| means | `hold` | Their content is not licensed to us. The searches are offered instead. |
| next | `hold` | The questions it raises are next. |

---

### Film 7 · `questions` — What questions does it raise?

**Blurb.** Eight levels of checking, the two it does not answer, and the questions it poses instead.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | This index sits at the top of every report. |
| result | `spot:#sec-levels .levels` | Eight levels. Competitors are the fourth. |
| means | `hold` | The others cover the problem, the idea, and what is publicly said. |
| result | `spot:#sec-levels .g-q` | Two carry a question mark. |
| means | `hold` | Those two are not answered. They raise questions instead. |
| turn | `to:#sec-traj` | This is one of them. |
| result | `spot:#sec-traj .ask ul` | Three questions about starting a two-sided service. |
| means | `hold` | Which side first, the smallest workable patch, and the first user. |
| point | `spot:#sec-traj .why` | Above them, what prompted the questions, linked. |
| point | `spot:#sec-traj .book` | Below, the book they came from, unsummarised. |
| next | `hold` | Checking the report is next. |

---

### Film 8 · `checking` — Where did each fact come from?

**Blurb.** Citations that open onto the sentence, the source list, and the page that could not be read.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Every fact in the report carries a reference. |
| turn | `to:#sec-swot` | They look like this, beside the sentence. |
| result | `cite:#sec-swot .cite` | Clicking one opens the source. |
| means | `hold` | The page, the sentence it came from, and when it was read. |
| point | `spot:#sec-sources table` | The last section lists every page read. |
| result | `spot:#sec-sources p:nth-of-type(1)` | One page could not be read. |
| means | `hold` | The site does not allow it, so the link is given instead. |
| next | `hold` | Asking questions is next. |

---

### Film 9 · `asking` — Can questions be asked afterwards?

**Blurb.** Follow-up questions answered from the pages already read, and one it declines.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | Below the report there is a box for questions. |
| turn | `to:#followup` | They are answered from the pages already read. |
| action | `ask:What would I pay for 40 orders a month?` | Here is one about cost at forty orders a month. |
| result | `hold` | It works through each plan and gives the figure. |
| means | `hold` | Every number in the answer carries its reference. |
| turn | `ask:How do they handle food safety records?` | Here is one the pages do not cover. |
| result | `hold` | It says so, rather than answering. |
| means | `hold` | And offers to go and look, which uses another report. |
| next | `hold` | Watching for changes is next. |

---

### Film 10 · `alerts` — What happens when something changes?

**Blurb.** An email when a watched page moves, showing both versions with dates.

| Kind | Action | Narration |
|---|---|---|
| recognition | `report` | A report describes one moment. |
| turn | `view:notify` | A watch keeps checking the pages behind it. |
| point | `hold` | Two clicks to set up, with the price page already ticked. |
| result | `hold` | When one changes, an email says what changed. |
| means | `hold` | One change, one email, named in the subject line. |
| point | `hold` | Prices are watched by default. Everything else is off. |
| result | `diff` | The email shows both versions. |
| means | `hold` | The old text, the new text, and the date each was read. |
| next | `hold` | What it costs is next. |

---

### Film 11 · `cost` — What does it cost?

**Blurb.** One report a day with no account, and what an account or a subscription changes.

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | Nothing so far has needed an account. |
| result | `view:limit` | Without one, it is one report a day. |
| means | `hold` | The full report, with nothing removed or marked. |
| result | `tier:reg` | A free account makes it one an hour, and keeps them. |
| means | `hold` | Past reports stay available to reopen. |
| result | `tier:sub` | A dollar a month makes it five an hour. |
| means | `hold` | The report itself is the same at all three levels. |
| next | `hold` | Other ways to start is last. |

---

### Film 12 · `ways` — Other ways to start

**Blurb.** Naming the competitors, pasting a website, or both, shown after the plain path.

| Kind | Action | Narration |
|---|---|---|
| recognition | `clean` | The box takes more than a description. |
| turn | `adv` | These options sit under the box, closed by default. |
| point | `hold` | They were kept shut until the plain path was clear. |
| result | `type:Croptally vs Barnwise` | Competitors can be named directly. |
| means | `hold` | It skips working out the market and reads them. |
| result | `type:https://croptally.example` | A website address works too. |
| means | `hold` | It reads that company first, then looks for similar ones. |
| point | `type:an app for farm-to-restaurant orders, like Croptally` | Both can go in one sentence. |
| close | `clean` | The report is the same in each case. |

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
