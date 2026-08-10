# Landscape — everything the site does, from a browser

**A walkthrough of the working product, in order, with nothing but a browser.** No terminal, no
`curl`, no commands. Each part says what to do, what you should see, and — briefly — why it
behaves that way, because most of the behaviour here is a decision rather than an accident.

> **Deploy it first.** [GO_LIVE.md](GO_LIVE.md) takes an empty Oracle account to a running site.
> If you are on your own machine instead, `cargo run -p landscape -- dev --store memory` and
> <http://127.0.0.1:8787> does the same thing.
>
> The terminal version of this — the parts that have no browser surface, like discovery and the
> hiring scanner — is [Feature_Walkthrough.md](Feature_Walkthrough.md).

## Before you start: two things that will interrupt you

**Each analysis takes four to eight minutes on the box.** This walkthrough runs six of them. Do
part 1 to part 6 in one sitting if you have an hour; parts 7 to 10 are independent and can wait.

**There is a cap of two analyses a day per visitor,** and this tour needs more than two. It only
counts requests that arrived through Caddy, so it bites exactly when you have followed
[GO_LIVE.md](GO_LIVE.md) step 10. Raise it for the afternoon:

```
ANONYMOUS_DAILY_LIMIT=50
```

in `/etc/landscape/landscape.env`, then `sudo systemctl restart landscape-api`. **Put it back to
2 before you show anyone the URL** — it is what stands between four ARM cores and the internet.

---

## 1. The first screen

**Open the site.**

You should see:

- a heading, **What is your idea?**
- one box, already focused
- an **Analyse** button
- **Or start from one of these** — three ideas, each with two companies and a line saying why
  those two
- a sentence saying the companies in the examples were chosen by hand, and that everything the
  report says is fetched and cited when you click

**Click the first example**, *project management for a small design agency*.

**It fills the box and does not submit.** The whole sentence appears, companies included:
`project management for a small design agency - basecamp.com vs linear.app`. You can edit it
before anything is fetched.

> That is the entire point of the examples. The curated part is the *choice of companies*, and
> putting it in the box is what makes the curation visible instead of hidden. Delete a company
> from the sentence and you get a report about one.

---

## 2. Watch a report arrive

**Press Analyse.**

| When | What you should see |
|---|---|
| Immediately | The address bar changes to `/a/` and a long id. **Queued.** |
| A few seconds later | **Reading public web pages…** |
| Within about a minute | The first section, usually **Recent public changes** |
| Every 20–60 seconds after | Another section fills in |
| Four to eight minutes | **Done.** |

**Reload the page while it is running.** Nothing is lost: the report is in the database, the
page picks the stream back up, and the sections already on screen come straight back.

**Copy the URL and open it in another tab.** It is a permalink. It works tomorrow, and it works
for anyone you send it to.

> Changes usually arrive first because that section needs **no model at all** — a changelog is
> dates and headings, which is parsing rather than inference. If you ever see every section
> empty *except* that one, the model server is down, and that is how you tell it apart from a
> fetching problem.

---

## 3. Read the report the way it is meant to be read

Look at a section that filled — **Pricing & packaging** is the usual one.

Each line is a **claim**, and after it in square brackets is a **source label** like `[S1]`.
Under it is the **quote** that supports it, copied from the page.

Scroll to **Sources** at the bottom. Every label resolves there, with:

- the page's title
- **the URL**, which you can click and check yourself
- the date it was read
- one phrase about the page's standing — *the company's own page*, *an independent page whose
  author and date we confirmed*, *a page we could read but could not fully attribute*

> **There is no way to write a claim here without a quote and a source.** The type that carries
> a claim has no shape for an unsourced sentence, so one cannot reach you by accident. The
> standing matters too: only *the company's own page* is allowed to set a value in a comparison.

Now find a section that says **Nothing found in public sources.** Underneath it is a list of the
pages that were checked, with the status each returned.

> A negative you cannot repeat is not a finding. The list is what makes *"we did not find a
> price"* into something you can check in a minute rather than something to take on trust.

Look above the sections for the notes about the whole report — how many pages were read, how
many were found and not read, and whether any question went unsearched.

---

## 4. Compare two companies at once

**Go back to the home page and type:**

```
basecamp.com vs linear.app
```

**Press Analyse**, and wait.

In the finished report, claims in the same section now carry the company they came from, in
bold, before the claim.

> That label only appears when a report covers more than one company. On a report about one, it
> would be the same name on every line, which is noise. And the company travels *with* the
> claim rather than being guessed from the text, because an extractor reading Basecamp's page
> writes "Pro costs $15" — putting the company in that sentence would be the renderer's job
> leaking into the evidence.

---

## 5. Disagree with the set

On any finished report, under the notes, there is a block headed **Comparing** with one chip per
company.

Try all five of these:

| Do this | What should happen |
|---|---|
| Click the `×` on a chip | It disappears, and **Run this set** becomes clickable |
| Type `notion.so` in **Add a company** and press **Add** | It joins the row |
| Type `basecamp` (no dot) and press **Add** | *"basecamp does not look like a domain. Try example.com."* — refused **out loud** |
| Add a company that is already in the row | *"… is already in this set."* — including when you type it the way the chip spells it |
| Remove every company | The button greys out: *"Removing every company leaves nothing to compare."* |

Now put the set back the way it was. **Run this set** greys out again — the set on screen is not
a different question, and re-running it would spend eight minutes redrawing a page you are
looking at.

Finally, remove one company and press **Run this set**. A **new analysis** starts, with its own
permalink. The report you were reading stays where it was.

> The button hands back a whole prompt — the same thing a reader could have typed. Nothing about
> what counts as a company is decided in the browser; that rule lives in one place on the server,
> and a second copy here would be a rule that agrees today.

---

## 6. Hand the whole report to your own assistant

Under the set, on any finished report: **Copy as context**.

**Press it.**

| Where you are | What happens |
|---|---|
| On `https://`, which is what [GO_LIVE.md](GO_LIVE.md) leaves you with | The button says **Copied**, and the whole report is on your clipboard |
| On `http://` — Appendix A of that guide | *"This browser would not let us reach the clipboard. Here it is — select it and copy."* and the document appears in a text box |

Either way you have the document. Paste it somewhere and read the top:

```
Here is a public-evidence report on an idea I am considering. Every claim below has a
source and a date. Tell me what the evidence does not cover, and where you would want
more before believing it.
```

Then the report as Markdown: every claim with the quote it came from, every source with its URL
and the date it was read.

Check three things in what you pasted:

- every `[S1]` in a claim has a matching `[S1]` in the Sources list
- no address of the machine it runs on appears anywhere
- the quotes are whole — nothing is shortened, nothing has an ellipsis

> This is the product's argument for itself. Most people evaluating an idea already pay for a
> chatbot; the honest response is to feed it rather than compete with it. What a chatbot cannot
> do is assemble forty pages of public evidence with a URL and a date against every sentence —
> so that is the thing to hand it.

---

## 7. Describe an idea instead of naming companies

**This part needs the search engine** — [GO_LIVE.md](GO_LIVE.md) step 9. Without it you will get
a refusal that says so, which is part 9.

**Type a description with no company in it:**

```
privacy-friendly website analytics
```

**Press Analyse.** This one takes longer: it searches first, then reads.

In the finished report, look for:

- **Interpreted as** — the phrase the searches actually used, if it differs from yours, with how
  many independent sites used it
- **Why each one is here** — for every company in the report, in countable terms: how many of
  the searches returned it, and which of your words its own front page uses
- companies that were **found and left out**, each with which of five things happened: only one
  search found it, we do not believe the score, its front page is about another market, we could
  not read its front page, or we never asked for it

> The substitution is shown rather than assumed because it decides every query underneath it. If
> the reading is wrong, everything below is about a different market — and you are the only one
> who can tell.

---

## 8. A name that matches several companies

**Type a bare word that several products share:**

```
notion
```

You get a refusal rather than a report, and under it **one button per company** — each with the
name read off that company's own front page, its domain, and a line telling it apart from the
others.

**Click one.** It starts an analysis of that company. You did not have to retype anything.

> There is deliberately no *"skip, just analyse"* here. Skipping means guessing between two
> companies that share a name, and a report about the wrong Notion looks exactly like a report
> about the right one.

---

## 9. Every way it tells you no

These are worth seeing once, because each one asks you to do something different — and the whole
design of this part is that they are never collapsed into one sentence.

| To see it | What it says |
|---|---|
| Type `a tool for small farms` | *"We could not work out which company you meant. Try naming its website…"* |
| Type `notion` (part 8) | *"That name matches more than one company… Pick the one you meant:"* |
| Stop the search engine on the box, then type a description | *"The search did not finish… This is usually temporary — try again."* |
| Remove `SEARX_URL` and restart, then type a description | The report says no search engine is configured, so nothing off the company's own site could be reached |

To stop and restart the search engine, on the box:

```bash
sudo docker compose --profile search stop searxng     # then try a description
sudo docker compose --profile search start searxng    # put it back
```

> Only one of those situations is fixed by waiting, and only one is fixed by changing what you
> typed. A single sentence covering all of them would send you to fix something that was never
> wrong.

---

## 10. A report about one company

**Type a single company's website:**

```
basecamp.com
```

The report covers it alone, and says so in its first note — including **why** there is nobody
beside it. That is one of four different facts:

- no search engine is configured here
- its own front page gave nothing to judge a competitor against
- the searching did not finish
- we searched and nobody held up

> Only the last is a statement about the market. The others are statements about us, and
> confusing the two is how a tool quietly tells you a market is empty when what happened is that
> a query timed out.

---

## What you have just seen, and what is not here yet

| Working today | |
|---|---|
| A report from a company's website | Cited, quoted, and dated |
| A report from a description | Companies found, scored, and each one's reason shown |
| A comparison of several companies | With every claim attributed |
| Editing the set and re-running it | As a new analysis with its own permalink |
| The whole report as Markdown | For your own assistant |
| Permalinks | Shareable, reloadable, resumable mid-run |
| Six kinds of refusal | Each with the one thing that would help |

| Not built yet | Where it is on the plan |
|---|---|
| The feature comparison matrix | [Full_Feature_List.md](Full_Feature_List.md), S3 |
| Charts, and PDF export | S3 |
| Quote verification against the source | S3 — `landscape-verify` |
| Three more report sections | S3 — positioning, sentiment, SWOT |
| Accounts, saved sets, alerts | S5 and S6 |

The honest summary of where this is: [PROJECT_STATUS.md](../PROJECT_STATUS.md).
