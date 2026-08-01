# Landscape — Video & Demo Guidelines

> How demo videos are written, recorded and delivered.
>
> [CODING_QUALITY.md](CODING_QUALITY.md) §9.5 sets the **policy** — when a demo is required,
> what it must contain, and how it is published. This document is the **craft**: the rules for
> writing the narration and building the artefact. Every rule here was learned by getting it
> wrong first, and the mistake is recorded alongside each one so nobody repeats it.

---

## 1. The one rule everything else serves

> **Write for someone who has never seen the product and does not work in software.**

They should finish the video able to explain, to another non-technical person, what the thing
does and why they might use it. If a sentence only makes sense to someone who already knows
the product, it has failed — however accurate it is.

---

## 2. Language

### 2.1 Say what a person does, then what happens

Narration describes the **viewer's experience**, not the system's internals. The pattern is
*action, then consequence*.

| Never | Instead |
|---|---|
| "One box. Three examples. That is the entire learning curve." | "You describe your idea in the box, in your own words. That is the only thing you have to do." |
| "Sources appear as they are fetched." | "It reads the companies' own websites. Each page shows up as it is read, so you can watch it work." |
| "Pricing is parsed deterministically, never generated." | "Prices are taken straight from each company's pricing page, so the figures are exactly what the company published." |
| "The feature matrix has five cell states." | "The comparison table is careful about what it claims. A question mark means we looked and found nothing." |

**The failure this fixes:** the first cut opened with *"One box. Three examples."* A reviewer
who had read every design document still could not tell what "one box" or "three examples"
referred to. Insider shorthand reads as confident and communicates nothing.

### 2.2 Banned vocabulary

These appear nowhere in narration. Each has a plain replacement:

| Jargon | Say |
|---|---|
| parse / parsed / extract | read, taken from |
| stream in / render | appears, shows up |
| spinner / loading state | — (describe what is visible instead) |
| diff | the before and after |
| content hash | when it was read *(or omit — see §2.4)* |
| tier | free account · a dollar a month |
| anonymous user | without signing up |
| rate limit / quota | how often you can run it |
| runtime / config | without touching code |
| degraded / truncated | nothing hidden, nothing left out |
| watermark | nothing stamped across it |
| deterministic | exactly what the company published |
| source class / attributed | where it came from |

**[CI]** A build check greps the narration for this list. The list grows; it never shrinks.

### 2.3 No ambiguous shorthand

Every noun must be resolvable from the video alone. "The box" is fine *after* you have said
what goes in it. "Three examples" is never fine, because the viewer cannot see that the chips
are examples until someone says so — and by then the sentence has moved on.

**Test:** read the caption aloud with the screen covered. If you cannot tell what it refers
to, rewrite it.

### 2.4 Cut detail that serves the author, not the viewer

The first cut said a citation shows *"the URL, the timestamp, the content hash, and the quoted
line."* Three of those four matter to an engineer; one matters to a viewer. It became:
*"you see the web page, the exact sentence, and when it was read."*

Accuracy is not the same as usefulness. Everything true is a candidate; only what changes the
viewer's understanding earns a place.

### 2.5 Complete thoughts, whole sentences

Each caption is one or two complete sentences with a subject and a verb. Fragments look
punchy written down and sound broken spoken aloud — and this narration *is* spoken aloud
(§4). No trailing em-dash clauses that a voice cannot land.

---

## 2A. Choosing the example

### 2A.1 Demonstrate the simplest path first, always

**The mistake:** the first cut opened by comparing three named products. That quietly assumed
the viewer already knew who their competitors were — which is the opposite of the problem the
product solves, and it made an advanced input look like the normal one.

**The rule:** the opening example uses **the simplest possible input, and only that**. Here it
is a plain business idea in someone's own words. Anything that requires prior knowledge —
naming rivals, pasting a website — is an advanced path and does not appear until §2A.3.

The same discipline applies to the interface itself. The examples offered on screen are all
the simple kind; the other ways in live behind a closed disclosure that most people will never
open.

### 2A.2 Pick an example that makes the product look necessary

**The mistake:** the first example was three project-management tools. A reviewer's verdict was
"pretty flat," and they were right — everyone has heard of those products, so the demo showed
the tool doing work the viewer could have done themselves.

**The rule:** choose a subject where the viewer **cannot already name the competitors**. The
current example — an app helping small farms sell to restaurants — surfaces three companies
most viewers will never have heard of. That single fact demonstrates the hardest capability in
the product, and no narration is needed to explain why it is useful.

**Test:** if a viewer could have listed the competitors themselves before pressing play, the
example is teaching nothing.

### 2A.3 Advanced paths go last, and are named as alternatives

Show them **after** the complete simple journey, briefly, one at a time:

1. name the competitors, if you already know them;
2. paste a company's website;
3. or put either in the same sentence as the idea.

Say explicitly that each works alone **or in combination**. Listing them separately keeps each
one legible; bundling them into one example makes all three look like requirements.

### 2A.4 Invent the companies in the example

Demo data is fabricated by definition. Attaching invented prices and features to **real named
companies** — even behind a "simulated data" banner — is exactly what
[FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5 forbids the product itself from doing, and it
carries the same trade-libel exposure as R13.

Use coined names on the reserved `.example` domain. It is unmistakably fictional, it costs
nothing, and it means the demo never makes a claim about a real business.

---

## 3. Subtitles

### 3.1 Player-rendered, never burned into the picture

**The mistake:** the first two cuts drew captions into the video frame. They looked fine at
full size and became unreadable the moment a player scaled the video down — the text shrinks
with the picture, because it *is* the picture.

**The rule:** captions ship as a **WebVTT track that the player renders**, on by default. They
stay legible at any player size, can be turned off, can be selected and copied, and are
available to screen readers.

The raw video file therefore has no captions in it. The `.vtt` ships beside it.

### 3.2 Generated from source, never transcribed

The narration lives in the demo script in the prototype source. `make-captions.py` parses it
and emits the WebVTT. **Nobody types the captions twice.**

Two consequences worth having:

- Captions cannot drift from what is actually demonstrated.
- **Rewriting the narration does not require re-recording.** Text and timing are separate:
  `t:` values drive the visuals, `c:` strings drive the words. This document's own rewrite of
  every caption cost one command and no new footage.

### 3.3 Styled for reading, not for decoration

Large (`font-size: 1.35em`), high contrast, dark plate behind the text, generous line height.
Subtitles are the primary channel, not a fallback.

---

## 4. Narration

### 4.1 Optional, and off by default

The video must be fully comprehensible **silent**. Most people watch muted, and the recording
has no audio track at all. Narration is a switch, not a requirement.

### 4.2 The voice owns the pacing

**The mistake:** the first version spoke each caption when its cue appeared and cancelled it
when the next arrived. Sentences were cut off mid-thought. It sounded broken, and it was.

**The rule:** on each caption the video **pauses**, the line is spoken to completion, there is
a beat (~700ms), and then playback resumes.

Narrated playback therefore runs longer than the silent version. **That is correct.** Letting
the voice finish a sentence is worth more than matching a runtime. The page states the
difference rather than hiding it.

### 4.3 One sentence per utterance

Captions are split on sentence boundaries and spoken in sequence with a short gap. This gives
a natural pause between thoughts, and keeps each utterance short enough to avoid the ~15-second
cutoff in Chromium's speech engine.

### 4.4 Use the viewer's own speech engine

Web Speech, with a preference for a neural voice where the browser exposes one. It costs
nothing, ships nothing, and needs no service. Voice quality varies by platform — say so rather
than pretending otherwise.

---

## 5. Picture

### 5.1 Record with the interface scaled up

**The mistake:** the first cut was recorded at native scale. The text was correct and too
small to read once the video sat inside a page.

**The rule:** record with the UI scaled (currently `zoom: 1.35`) so text is large *relative to
the frame*. More pixels alone does not help — a 4K recording of small text is still small text.

### 5.2 Frame size is a negotiation with file size

The demo page inlines the video as a data URI, and the artifact limit is **16MB** — so the
video must stay under roughly **11.5MB**. At 1360×850 a two-and-a-half minute demo lands near
11MB.

Because subtitles are drawn by the player (§3.1), the frame only has to carry the *interface*
legibly. That is what the zoom does, not the pixel count — so when the file is too large, cut
the frame size and keep the zoom. Cutting resolution costs almost nothing; cutting the zoom
would make the text unreadable again.

### 5.3 Give the viewer a way to make it bigger

Full-bleed layout up to 88vh, plus an explicit fullscreen button. Do not rely on people
finding the player's own control.

---

## 6. Audio

**No background music.** Procedurally generated ambient beds sound cheap — too structured to
be texture, too thin to be music. A silent video with good captions is better than a video
with filler audio.

If music is ever added, it should be composed or licensed, not synthesised as a nicety. The
bar is that a viewer would notice its absence.

---

## 7. Honesty

Carried over from [CODING_QUALITY.md](CODING_QUALITY.md) §9.5 and
[FACT_CHECKING.md](FACT_CHECKING.md) §3.2.5, because a demo is a public claim about the
product:

- **Label anything not real.** Stubbed inference, faked data and compressed timings are stated
  on screen and in the surrounding page. The current demo runs at 8× and says so.
- **One demo per release at real speed**, unedited, so the actual experience stays visible.
- **Speed ramps only on waits, never on interactions**, and the factor is displayed.
- **In comparisons, publish the dimensions where the product loses**, not only where it wins.
- **Describe what a tool did, never what a company is.**

---

## 8. Structure and length

| | |
|---|---|
| Hard cap | **120 seconds** silent. A demo nobody finishes is worse than none. |
| Opening | What the product answers, in one sentence, before any interface appears |
| Body | One idea per caption, in the order a real user meets them |
| Close | What to remember, not a summary of features |

The current close — *"Facts you can check. Gaps it owns up to. And one thing to type to get
started."* — is three short claims, not a recap. That is the target.

---

## 9. The pipeline

Three scripts, roughly 200 lines total, **no ffmpeg required**:

| Script | Does |
|---|---|
| `prototype/record-demo.mjs` | Drives the prototype with Playwright, records WebM |
| `prototype/make-captions.py` | Parses the narration out of the demo script → WebVTT |
| `prototype/make-demo-page.py` | Builds a self-contained page: video + track inlined, narration and controls in JS |

Rebuild after a copy change:

```bash
python prototype/make-captions.py && python prototype/make-demo-page.py
```

Re-record only when the **visuals** change:

```bash
node prototype/record-demo.mjs
```

---

## 10. Review checklist

Blocking, alongside the §10.3 checklist in [CODING_QUALITY.md](CODING_QUALITY.md):

- [ ] The opening example is the **simplest input**, with no advanced options mixed in.
- [ ] A viewer could **not** have named the competitors themselves before pressing play.
- [ ] Advanced paths appear only at the end, listed separately, described as optional.
- [ ] Companies in the example are invented, on `.example` — no real business is given
      fabricated prices or features.
- [ ] Every caption is understandable to someone outside software.
- [ ] No word from the §2.2 banned list.
- [ ] No noun that cannot be resolved from the video alone.
- [ ] Every caption is a complete sentence, and lands when read aloud.
- [ ] Subtitles render at the player's size, on by default.
- [ ] Watched silent, start to finish, and it made sense.
- [ ] Watched with narration, and no sentence was cut off.
- [ ] Anything faked, stubbed or sped up is labelled on screen.
- [ ] Under the length cap.
