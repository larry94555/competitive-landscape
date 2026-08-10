# Landscape — Benchmarks

> Real numbers, with the hardware they came from. `docs/ROADMAP.md` Phase 0 requires this
> file before a model is chosen, because every latency figure in the specification is
> otherwise an estimate.
>
> **Nothing here is measured on the deployment host, and nothing will be** —
> [ADR 0011](decisions/0011-no-experiments-on-production.md). A benchmark leaves a toolchain,
> a downloaded model and a set of assumptions behind in the thing customers depend on. What a
> laptop cannot answer — how long somebody waits — is answered from the client's side of a
> deployment, not from inside the box. These are
> laptop numbers and are labelled as such.

Two harnesses, measuring different things. Timings come from the first; whether the answers
are *true* comes from the second, and nothing in the first can tell you.

```bash
cargo run -p landscape-bench -- --runs 20 --label "what this is"
```

```bash
cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

`LLAMA_URL` selects the server for both.

A third measures neither speed nor truth but **shape** — where on a page a price actually
lives, which decides whether a headless browser is ever built:

```bash
cargo run -p landscape -- gap docs/js-gap-sample.txt
```

---

## Run 40 — where the roles actually live

**Date:** 2026-08-09 · **Where:** this laptop · **Model:** none. Discovery and the hiring
scanner are both deterministic; nothing here asks one.

`ROADMAP.md` has listed **public ATS boards** among the structured probes since Phase 1 was
written, and they were the last entry on that list still to come. Four real careers pages, read
the day this was built:

| | links to a board |
|---|---|
| `linear.app/careers` | `jobs.ashbyhq.com/Linear` |
| `front.com/jobs` | `jobs.ashbyhq.com/frontcareers` |
| `helpscout.com/company/careers/` | `jobs.ashbyhq.com/helpscout` |
| `plausible.io/jobs` | — |

**Three of four keep their vacancies somewhere else.** A careers page that links out is not a
company with nothing to say about hiring; it is a company whose list we were not reading.

### Found, never guessed

**Every board is one the company's own page linked to.** Nothing constructs
`jobs.ashbyhq.com/<the company's name>` and hopes, and that restraint is the whole safety
argument: a guessed slug belonging to somebody else would put another company's vacancies on this
company's report — cited, internally consistent, and wrong in the way `FACT_CHECKING.md` §3.1
exists to prevent. A guess is also unnecessary. The link is there, or the company has not told us
where its roles are.

**No extra request to find one.** The board is read out of the careers page's own body, which the
run has already paid for.

### A board is not the company's own server

`Disposition::Attributed`, never `Primary`. The bytes come from a stranger's host, so nothing a
board says may set a value in a comparison table — but the subject's own page is what named it,
so authorship is not in doubt either. That is what `Attributed` is for, and it means the rule
*"`Primary` is the company's own domain"* stays as it was rather than being widened to fit a
feature into it.

Discovery's off-site filter gains exactly one exception, and it is an exception to **where the
bytes come from** rather than to **what they are worth**. The laundering that filter prevents is
an off-site page arriving in the class that outranks everything, and a route whose class is fixed
below `Primary` cannot do that.

### Which hosts, and why six

Only boards addressed as `<host>/<slug>`, so a vacancy link reduces to the board's root by taking
one path segment: Ashby, Greenhouse (both hosts), Lever, Workable, SmartRecruiters. Hosts that
put the company in a **subdomain** — `acme.recruitee.com` — are deliberately absent: every
subdomain there is a different company, so a misread link is somebody else's board rather than a
404.

`boards.greenhouse.io` is a substring of `job-boards.greenhouse.io`, so a plain search finds one
board twice under two names and spends two of a page's allowance on the same vacancies. The match
is on a host boundary.

### What a board turned out to need from the scanner

Fetching a board and reading nothing off it would be a request to a stranger's server for no
information, so the two rules it needed are here rather than in a later row.

**An announcement it did not recognise.** `announces` is an exact match against ten phrases, and
Greenhouse titles the page *"Current openings at Vercel"*. It now also accepts `<phrase> at …` —
the one possessive form a hosted board uses, and deliberately not `starts_with(phrase)`, which
would let *"open roles are what we are proudest of"* announce a list that is not there.

**A list written the other way round.** A hosted board puts its roles in bullets and its
furniture — *Create a Job Alert*, a count of 83 jobs, a department heading per section — in the
prose and headings between them. A company's own careers page usually does the opposite: roles as
plain lines, with a bulleted footer underneath. Neither *"prefer bullets"* nor *"ignore bullets"*
is right; each publishes the other page's furniture as a vacancy, and three existing tests said
so the moment the first version tried. **Whichever form carries more of the titles is the list**,
counted rather than assumed, with a tie keeping the plain lines that were there before.

```text
boards.greenhouse.io/vercel   announced: true   titles found: 44   reported: 20 (the cap)
```

Every one of the twenty is a vacancy. Unscoped, the same page also published *Create a Job Alert*
and two department names.

### A board cannot win a slot on its own, and that is not a ranking bug

Round one of the cap gives each question its best page, and for hiring that is the company's own
careers page — which is how the board was found. So a board only ever competes for a **second**
slot, and the questions that sort earlier take those. On all three companies above, the board was
discovered and then dropped.

Ranking a board higher is the obvious fix and it is wrong:

| | its own careers page | its board |
|---|---|---|
| `linear.app` | lists its roles | Ashby, reads as nothing |
| `vercel.com` | navigation chrome, no list | Greenhouse, 44 titles |

**Which is which cannot be known before the page is read.** So both are admitted and the reading
decides, and the slot comes from whichever question already has the most — which is
[ADR 0010](decisions/0010-spend-the-cap-on-breadth.md)'s own argument applied to itself: a second
pricing page is the cheapest thing on the list, and a board is the only place some companies'
roles exist at all. A run that gave every question exactly one page gives nothing up.

```text
cargo run -p landscape -- discover https://vercel.com

https://vercel.com/careers/vercel-development-represent…   direction    sitemap
https://job-boards.greenhouse.io/vercel                    direction    board
```

All three companies now admit their board, measured today.

### The first frozen page that is not the subject's own

`vercel-greenhouse-board.md` joins the golden set, and it is the first fixture in it that lives
on somebody else's host. It is there to hold both rules honest, and it is the opposite shape from
`linear-careers.md` in exactly the way that matters — which is why the pair is worth having.

### A fourteenth gate, because a percentage nobody adds up drifts

`Full_Feature_List.md`'s S2 totals row read `18 / 16 / 2` while its own ten feature rows summed
to `19 / 16 / 3`. The page has therefore been publishing **89% for a state that was 84%**, and
every pull request that quoted it repeated the number — including the three before this one.
Nothing was wrong on purpose: a row was added once and the total below it was not.

`scripts/feature_totals.py` adds each table up against its own rows, checks that every row's
`done + left` is its estimate, and checks the percentage against the sum. Six tables, no test run
needed, so it costs nothing. It is the same shape as the benchmark-count gate: a number a
document must hold, checked against the thing it is derived from rather than trusted.

The corrected S2 arithmetic is **19 estimated, 17 done, 2 left — 89%**, which this row is what
moves it to.

### What this does not do

**An Ashby board still reads as nothing.** Three of the four careers pages above point at Ashby,
whose board keeps its vacancies in `window.__appData` rather than in the HTML — so the page is
fetched, the scanner finds no list, and the report says *the page does not say where its open
roles are listed*, which is the honest sentence rather than a silence. Reading it needs the
embedded-state tier `landscape-extract` already has for prices, pointed at roles; that is the
`Rendering ladder tiers 2–4` row of S3, and this run is the reason it now has a second customer.

**No board is fetched for a company that links to none.** That is not a gap: a company with no
public board has not published one.

| | Rust tests | frontend tests |
|---|---|---|
| Run 39 | 928 | 62 |
| now | **947** | **62** |

---

## Run 39 — asking whether it changed

**Date:** 2026-08-09 · **Where:** this laptop · **Model:** none. Neither half of this row asks a
model anything; both are about requests that reach somebody else's server.

This closes the last technical row `landscape-fetch` has carried open since it was written:
*"conditional GET (`Page` carries the headers; nothing re-fetches), and a cap on total fetches
per analysis — that belongs with the orchestrator."*

### The header we have been storing and never sending

`Page` has carried `etag` and `last_modified` since the first fetch in this crate. Nothing ever
sent them back, so a page whose hour was up cost a **full download** to learn that nothing about
it had changed.

| | before | now |
|---|---|---|
| inside `FRESH_FOR` | served from memory, nothing sent | unchanged |
| past `FRESH_FOR`, origin gave an `ETag` | full GET, whole body | **conditional GET; a `304` carries no body** |
| past `FRESH_FOR`, origin gave only `Last-Modified` | full GET | conditional on the date |
| past `FRESH_FOR`, origin gave neither | full GET | full GET — there is no cheap question to ask |

**The saving is theirs, not ours**, which is the argument this row has made since the fetch
cache: a `304` is the smallest thing a server can be asked for, and the bytes it does not send
are bytes it did not have to read from its own disk.

### A revalidated page is *as of now*; a merely unexpired one is not

The two paths look alike and are not, and the distinction decides what a claim is dated:

- **Cache hit.** Nobody asked anybody anything. `fetched_at` stands, and a claim drawn from the
  page dates to the fetch — the rule Run 37 introduced and a mutation still guards.
- **`304`.** We *did* ask, and the origin said the bytes are current. `fetched_at` becomes the
  moment of confirmation, because a report saying *"as of an hour ago"* about something an origin
  just confirmed **understates what is known**.

### Review: a `304` says *unchanged*, and it was read as *nothing*

RFC 9111 lets a `304` omit any header whose value has not changed, and most origins omit
`Cache-Control`. The first version re-inserted the confirmed page using **only the headers on the
`304`** — so `storable` saw no policy, fell back to our hour, and an origin's `max-age=30` became
sixty minutes the first time it was revalidated.

**Asking a publisher whether their page changed widened the policy they set on it.** That is the
opposite of what the request is for, and it is silent: the page is right, the citation is right,
only the interval is wrong.

The first fix carried the stored **duration** forward and used it whenever the `304` restated
no freshness at all. Review took that apart too, and correctly: RFC 9111 §3.2 and §4.3.4 say the
fields a `304` *supplies* replace their stored counterparts and the ones it *omits* remain — and
that is a merge **per field**, which a computed number cannot express.

```text
stored   Cache-Control: max-age=30   Expires: +1h     -> fresh for 30s
304                                  Expires: +2h     -> still 30s
read as "it said something, so read it alone"         -> two hours
```

So the entry keeps the freshness **fields** as the origin wrote them. `Policy::updated_by`
overlays what the `304` supplied, and the lifetime is recomputed from the merge:

| the `304` says | kept for |
|---|---|
| nothing | the stored policy, unchanged |
| `Expires` only, over a stored `max-age=30` | **30s** — the `max-age` is still there |
| `max-age=600` over a stored 30s | 600s — an origin extending its own policy is still the origin |
| `max-age=30` over a stored hour | 30s |
| `no-store` | **not kept, and what we held is dropped** |
| an unreadable header | not kept |

**`Date` and `Age` are deliberately not part of the policy.** They describe *a response*, not a
policy, and the stored ones belong to a response that is no longer the one in hand — carrying
them forward would charge a fresh `304` with the age of the `200` it confirms. The `304`'s own
are used, so a `304` arriving with its freshness already spent is not kept.

**And a new validator replaces the old one.** §4.3.4 again: an origin may hand back a new `ETag`
on a `304`, and keeping the one we asked with would make every later revalidation ask about a
version nobody has — so the saving would quietly stop happening.

**Keeping fields means paying for fields.** Review's third pass: the policy went into the entry
and not into `cost`, which is the same hole `cost` was written to close two runs earlier —
`Cache-Control` and `Expires` are origin-controlled heap allocations, so a budget that counted the
body and the key but not them reports a cache inside its limit while it is not. Both fields are
costed now, under the policy actually stored rather than the one being replaced, and there is a
regression that fills a cache with sixty-four-kilobyte `Cache-Control` headers and one that pads
an `Expires` — a valid `Expires` can be as long as a stranger likes, because leading space is
trimmed before it is parsed and kept in what is stored.

The last two rows are the second half of the same finding. `Storable::No` meant *do not store
this*, and the code did exactly that — while leaving the **older copy** in place to be served.
Refusing to store and refusing to serve are different things, and a cache that obeys `no-store`
by doing only the first still ignores it. `Cache::forget` is now called on every refusal, on the
`200` path as well as the `304` one.

### Review: one allowance for things that were never one question

`landscape examples` checks six independent companies. The budget was created once above the
loop, so the first two spent it and every later one came back with no pricing page — **a health
check failing on an artifact of the check before it**, which is worse than no health check.

The bound was right and its *scope* was copied from the place it was designed for to a place that
merely looked similar. One allowance per company now, and the same correction in
`landscape gap`, which measures twenty-eight independent pages. Every other construction site was
audited against the same question — *what is the unit of work here?* — and the rest are genuinely
one question each.

### `304` is a `3xx`, and that is a trap with a type for a fix

Written as two `if`s, the redirect branch comes first and swallows it: `304` has no `Location`,
so the cheapest answer an origin can give becomes `redirect with no location` — a conditional GET
strictly worse than none. Ordering is a property of how code happens to be laid out. So it is a
`match` on a type, with **no wildcard arm**:

```rust
enum Answer { NotModified, Redirect, Body }
```

**Validators travel on the first hop only.** They are the origin's proof about *the URL the
caller asked for*; a redirect lands somewhere else, and asking that somewhere else whether it
matches another page's `ETag` is a question about nothing — which an origin may cheerfully answer
`304`, handing back one page's body under another page's URL. That is the failure this whole
pipeline exists to prevent, so it has its own function and its own test.

### The cap: everything here bounded one request, and nothing bounded the run

The size cap bounds one body, the pacer bounds how often one host is asked, `robots.txt` bounds
which paths. The run was bounded only by an accident of arithmetic — eight pages a company, three
companies — **and an accident is not a bound**. A `sitemap.xml` naming ten thousand URLs, a
redirect chain per page, a `robots.txt` per host reached through search: each is a way for one
reader's question to become an unbounded number of requests to strangers.

`Budget` is per analysis, **shared by all three passes** — resolving a description, finding a
named company's rivals, and reading them are one reader's question. Sixty-four requests: three
companies at one `robots.txt`, eight discovered pages, three searched and a redirect or two is
forty-two, so the ordinary run has room and the pathological one does not. A test asserts that
arithmetic, because a bound the ordinary case trips is a bug wearing a limit's hat.

**What counts is a request that leaves the process.** A cache hit costs nothing, for the same
reason it skips the address guard. A `robots.txt` costs one — a run reaching a hundred hosts
through search fetches a hundred of them whether or not it reads a page on any. Every redirect
hop costs one, because a chain that never ends is the case this exists for.

### Running out is our doing, and never reported as theirs

Three surfaces, three different sentences, because a reader acts differently on each:

| | |
|---|---|
| `robots.txt` that could not be read | `Rules::restrictive` — *unreachable is not permission* |
| `robots.txt` we did not ask for | the error propagates. **Not** *"the site asks crawlers not to"* |
| a discovery probe not sent | `Outcome::NotAsked`, printed `not asked`, kept apart from `unreachable` |
| a page not read | *"not read - this run had already made its 64 requests"* |

A reader told *"could not fetch"* about a page this run simply declined to ask for would check it
by hand, find it fine, and stop believing the rest of the report. That is why the bound is in the
sentence: a limit nobody can see reads as a bug.

`landscape read` prints what a run cost:

```text
requests sent to strangers: 24 of 64
```

### What cannot be measured here, again

**There is still no test over a socket**, and the guard was not weakened to get one — a test
server binds loopback and the SSRF guard refuses it, absolutely and with no flag. So the `304`
handling is asserted where the decisions live rather than over the wire: `answer_to`, `confirmed`,
`conditional_on` and `asking_whether_it_changed` are each one call from an assertion, which is
where the mutation harness put them. Four of the nineteen mutations came back `MISSED` on the
first run — every one of them a rule still written inside a function that opens a socket.

The budget half **can** be exercised end to end without a network, because the allowance is
spent before a request is built: a literal address passes the guard without DNS, and `send`
refuses. That is the one thing in this crate that a bound lets us test which a limit could not.

| | Rust tests | frontend tests |
|---|---|---|
| Run 38 | 901 | 62 |
| now | **928** | **62** |

---

## Run 38 — the second reader does not pay the model

**Date:** 2026-08-09 · **Where:** this laptop · **Model:** none, and the point of the change is
that a model is asked **fewer times** rather than differently. What it says is unchanged.

### The half that was left

`ROADMAP.md` has called the pair *"the highest-leverage cache in the system"* since Phase 1. Run
37 built the fetch half and stopped the second reader of a company paying **somebody else's
server**. It left every model call in place, and the model is where a run spends nearly all of
its time.

`landscape cost` counts what a run will ask a model, by running the same span-finding code the
run uses and counting rather than reading. No model is needed, so the number is reproducible:

```bash
cargo run -p landscape -- cost https://linear.app
cargo run -p landscape -- cost https://plausible.io
```

| | every admitted page | as this run reads them |
|---|---|---|
| `linear.app` | 28 | **16** |
| `plausible.io` | 15 | **14** |

**That last column is what the second reader used to pay again**, in full, for a set of pages
whose bytes were already in memory and identical.

### Review: the cache stopped working during the outage it exists for

The readiness gate sat **above** the lookup. So a page whose answer was already held came back
as `(no model)` while `llama-server` was down — the cache failing in precisely the hour it is
worth most, and a reader depending on the health of something the run never needed to touch.

**A fallback checked after the thing it is a fallback for is not a fallback.** The gate moved
inside the miss closure, so a hit makes **no request of any kind** — not a completion, not a
health check. A miss that cannot ask returns *"not asked"* rather than an answer, so the run
still reports `(no model)` honestly and nothing is remembered.

The regression puts a flag where every model-dependent step would be and asserts a hit never
reaches it.

### Review: the key said what was asked and not who answered

A memo is a claim that two computations are the same, and that claim has two halves — *same
inputs* and *same function*. The first version wrote down only the first.

`PROMPT_VERSION` covers a prompt's wording. It does not cover the decoding settings, the schemas
the sampler is constrained by, or **the model**. And the worker outlives `llama-server`:

```text
llama-server restarts with a different model at the same LLAMA_URL
  → the next analysis reuses the previous model's answers
  → the report labels them with the current client's address
```

One model's words attributed to another, cited and internally consistent — the failure this whole
pipeline is built to prevent, arriving through the thing added to make it faster.

The static half is now `EXTRACTION_VERSION`, in the key beside `PROMPT_VERSION`: bump it when
`decode()` changes, when a schema changes shape, or when the logic judging an answer changes.

The model is a **scope** rather than a key field, and the distinction is the interesting part.
Putting the model's name in the key would mean asking the model who it is *before* answering from
memory — a request on the hit path, which the finding above forbids. Scoping the whole memory to
one identity and emptying it when that changes costs **one metadata call per analysis** and
nothing per hit:

```rust
if let Some(identity) = llm.identity().await {
    if memo.serving(&identity) { /* the model changed; everything is forgotten */ }
}
```

`LlamaClient::identity` reads llama.cpp's `/props` under its own **two-second** timeout rather
than the generous one prefill needs, and returns `None` when it cannot find out. **`None` never
forgets anything**: a question we could not ask is not evidence that the answer changed, and
throwing the memory away on an outage would empty it in the hour it is worth most.

### The content is the key, so a hit cannot be stale

`Extractions` has **no TTL**, and the absence is the design rather than an omission. A page cache
answers *"what is at this URL"*, whose answer changes without warning, so it needs a window. This
one answers *"what does **this exact text** say about this question"* — and text that changed
produces a different key and therefore a miss.

```rust
Read::of(question, url, markdown, today)   // + PROMPT_VERSION
```

A stale hit is not bounded here; **it is not expressible**. That also settles the question a
digest would raise: the key holds the whole markdown rather than a hash of it, because a 64-bit
collision is unlikely and its consequence is the one failure this pipeline exists to prevent —
one page's facts attached to another page's URL, cited, and internally consistent. The memory
that costs is bounded below; a wrong claim would not have been.

Every input is in the key because a cache key that omits one answers a question it was not asked:
the prompt set (a re-worded prompt is a different question), the question, the URL (the prompts
name it), today (`Answers::Changes` resolves *"last week"* against it), and the text.

### A failure is never remembered

This is Run 37's last finding, arriving one row later in a different unit. `Outcome` gained
`Settled`, and only `Complete` is kept:

| what happened | settled | remembered |
|---|---|---|
| every window read and answered | `Complete` | yes |
| a window's model call failed | `Partial` | **no** |
| the reader stopped waiting | `Partial` | **no** |
| nothing on the page to ask about | `Complete` | yes — reaching *"nothing here"* costs every scanner |
| a changelog or a careers page | `Complete` | yes — no model ran, so nothing can half-happen |

A cached outage would be replayed to every later reader with nothing ever going back to ask
again. The regression asks three times, gets three failures, then one success — and asserts the
model was asked four times and not five.

### Where the rule lives, decided by the harness rather than by taste

The first version put the lookup inline in `read_one`. Run 37 ended with the same lesson twice —
a rule written where no test can reach it is a rule a mutation deleting it survives — and the
harness said it again immediately: four of the sixteen mutations came back `MISSED`, all of them
about the key, all of them unreachable because `read_one` needs a fetched page and a test server
binds loopback, which the address guard refuses absolutely.

So `memo::remembering` takes a closure, and a test passes a counter where the model would be:

```rust
let second = remembering(&memo, key, || async { asked.set(asked.get() + 1); … }).await;
assert_eq!(asked.get(), 1, "the second reader paid the model again");
```

and `Read::of` builds the key, so *"the URL is not part of the question"* is one assertion rather
than an unreachable line. `read_one`'s remaining share is one call with no decision in it.

### Bounded in both units, at the third attempt in three rows

16 MiB and 1,024 entries, oldest evicted first, and one page too large for the whole budget
declined rather than allowed to empty the cache and then not fit. Every clause of that sentence
is a correction review made to the fetch cache one row earlier, applied here **before** review
had to make it again — including the "too large to keep" guard whose absence was argued for in
prose last time and disproved in one line.

Two of the bound tests earned their wording the hard way even so: a byte budget tested only with
tiny pages is bounded by the entry cap and proves nothing about bytes, and a per-entry overhead
tested against a full cache is swamped by the URLs. There is a test for each in the unit that
binds.

### What this does not do

**Nothing is shared between processes**, unchanged from Run 37: a restart is cold, and a shared
cache means a store, which the laptop rule keeps as its own decision rather than a side effect of
this one.

**`landscape cost` is now an upper bound in a warm process**, and says so in `model_calls_for`'s
own documentation. It runs as a fresh process with nothing remembered, so what it prints is exact
for the run it describes — the first one. Teaching it to consult a memo it cannot have would be
predicting a different program in the other direction, which is the mistake Run 22 recorded.

**The saving is not measured end-to-end**, and cannot be here: it needs two analyses of one
company in one long-lived process, which is the worker, with a model attached. What is asserted
is the count — *the model was asked once, not twice* — with a stub where the model goes, which is
the same instrument Run 25 used for cancellation and the one that runs on every pull request.

| | Rust tests | frontend tests |
|---|---|---|
| Run 37 | 881 | 62 |
| now | **901** | **62** |

---

## Run 37 — the request we do not send

**Date:** 2026-08-09 · **Where:** this laptop · **Model:** none. A cache changes how often a
stranger's server is asked, not what a model is asked.

`ROADMAP.md` has called the fetch cache *"the highest-leverage cache in the system — build it in
Phase 1, not later"* since Phase 1. It was not built. Worse than not built:

```text
resolve_from_description  →  Fetcher::new()
rivals_of                 →  Fetcher::new()
run_analysis              →  Fetcher::new()
```

**Three fetchers inside one run.** A `Fetcher` remembers pages, `robots.txt` rules and the
per-host delay *by itself*, so three of them share none of it. One company's `robots.txt` was
fetched up to three times, and a page read to work out *which* companies these are was read
again to extract from.

### What changed

| | before | after |
|---|---|---|
| fetchers in the worker path | **3 per analysis** | **1 per process** |
| the same page, two passes of one run | fetched twice | fetched once |
| the same page, two readers an hour apart | fetched twice | fetched once |
| `robots.txt` per host per run | up to 3 | 1 |

The cache lives on the `Fetcher`, so the fix is two things at once: a cache, and something that
holds it long enough to matter. `run_analysis`, `resolve_from_description` and `rivals_of` now
take `&Fetcher` rather than making one.

### Politeness is the point, not a side effect

The bandwidth saved is ours; **the requests not sent are somebody else's**. That puts this
beside `robots.txt` and the per-host delay as part of one commitment about how often a
stranger's machine is asked for the same bytes, rather than as an optimisation standing next to
them.

### A hit performs no network I/O, which is what makes it safe to serve

The cache is consulted **before** the address guard, `robots.txt` and the pacer. Each is fine to
skip for exactly one reason: *nothing is sent*. The guard stops us reaching an address; robots
stops us requesting a path; the pacer stops us asking too often. None protects anything when no
request leaves the process.

What makes that sound rather than convenient is the invariant on the way **in**: only a page we
were allowed to fetch is ever stored, because the insert happens after both have passed. A
disallowed path is an error, and an error is not a page. `a_refusal_leaves_nothing_behind_to_be_served_later`
pins it.

The one thing a hit can be wrong about is a `robots.txt` that changed since the fetch, and that
window is bounded by `FRESH_FOR` rather than open.

### Review, fourth pass: a bad afternoon written down and replayed

`storable` decided from the headers and only the headers, so a `500` or a `429` with no
`Cache-Control` was kept for the full hour and handed to every later reader — **without the
origin ever being asked whether it had recovered.**

The value that decides this was already in the room twice. `Page::status` was on the struct being
stored, unread. And one module away, `robots::Rules::from_status` already encodes this exact
judgement in this repository's own words: a `429` or a `5xx` means *"the site is unwell — assume
disallowed. The polite reading of 'I am struggling' is not 'carry on'."* The cache took that same
afternoon and wrote it down.

**A cached failure is worse than a slow one.** The report has a gap whose cause is no longer
live, and nothing goes back to look for an hour. The politeness argument this row leads with
points the same way: a cached `503` spares an origin nothing, because there was never going to be
a second request for a page we already failed to read.

The rule is HTTP's own — keep on our own initiative only for the statuses RFC 9110 §15.1 defines
as cacheable, and for anything else require the origin to state a freshness:

| | headerless | `max-age=30` |
|---|---|---|
| `200`, `404`, `410`, `501`, … | held for the hour | 30s |
| `429`, `500`, `503`, `400`, `403` | **not held** | 30s — their number, obeyed |

**A bare `public` is not taken as permission**, though RFC 9111 §3 would allow storing on it.
`public` says *may be stored*, not *is fresh for*, and inventing an hour of freshness for a `503`
on that basis is what costs an origin its recovery. The stricter reading costs one request.

`insert_allowed` reads the status off the `Page` rather than taking it as a parameter, so the
status stored and the status judged cannot disagree.

### Review, third pass: an argument written where a test belonged

The section below ends by explaining why the robots cache needs no "too large to keep" guard:
a `robots.txt` is read through `MAX_BYTES`, so 2 MiB of file cannot become 4 MiB of rules.
Review answered in one line:

```text
one file left 5767378 bytes held against a budget of 4194304
```

An admissible 2 MiB file of `Disallow: /` lines is 174,000 directives, and the byte counter
charges each one the 32 bytes of the tuple that holds it as well as its single character.
**The parsed form is bigger than the bytes it was parsed from**, which reasoning from the wire
size cannot see. Without the guard the eviction loop empties the map making room and then
inserts the thing that did not fit.

The guard is there now, and the regression builds a file right up against `MAX_BYTES`. The
argument that replaced it in the source says what it cost — and the sharper lesson is that the
comment cited *this file's own rule* about not writing branches no test can reach, and used it
to justify not writing a check. A principle about testability became a reason to skip a test.

### Review, third pass: `get` returns the first, and a header can arrive twice

`Cache-Control` is a list-based field, and HTTP lets it arrive as several field lines whose
values combine as though written on one line with commas. `HeaderMap::get` returns the first:

```text
Cache-Control: public
Cache-Control: no-store      ← never read
```

So a response arrived as a bare `public` and was cached against an explicit instruction, and
nothing downstream could recover it — splitting on commas does not help when the value that must
be found was dropped before being passed along. `combined()` reads `get_all` and joins, and a
value that is not readable text is reported as `no-store`: it may have *been* one, and a header
we cannot read is not permission.

The tests build a `HeaderMap` with `append` rather than a response, for the reason the section
below gives about `insert_allowed`: nothing in `fetcher.rs` can be driven over a socket, so the
header readers take the map. **The harness made that point again mid-fix.** The first version
called `combined` from inside `Fetcher::get`, and *a directive on a second Cache-Control line is
never read* came back MISSED — the choice of which reader to use is a decision, and a decision
made in that function is one nothing can reach. Which field is list-based now lives in `Said`,
one call from an assertion, and `get`'s share is a single line with no choice in it.

### Review, second pass: pruning what has expired is not a bound

The fix for the leak below made `robots::Cache` drop expired entries on insert, and the
regression beside it aged 500 hosts past the six-hour TTL, inserted one more, and asserted one
remained. It passes, and it cannot see the case that matters — **nothing expires in an
afternoon**. Asked directly:

```text
held 50000 live hosts
```

Six hours is long enough that a worker crossing many companies never expires anything, so
"prune the expired" bounded a set that was never the problem. The regression was written from
the fix's point of view rather than the defect's: it had to *arrange* expiry to observe
anything, which is the one state in which the bug is absent.

The robots cache is now bounded the same way the page cache is — expiry, **plus** a live-entry
cap of 1,024, **plus** a 4 MiB budget over the retained directives, oldest evicted first. The two
new regressions insert past each cap **without ageing anything**.

Forgetting a *live* rule set is safe, and that is what makes eviction available here at all: the
next request for that host re-fetches `robots.txt`. The cost is one extra request. What is never
done is assuming permission from a rule set that was dropped.

*This paragraph originally argued that no "too large to keep" guard was needed, because a
`robots.txt` is read through `MAX_BYTES` and one entry could not exceed a 4 MiB budget alone.
That was wrong, and the round above records how. There is a guard.*

### Review, second pass: a lifetime is not a deadline

`max-age=3600` was being read as *"keep for 3600 seconds from now"*. It is measured from the
origin's `Date`, so a CDN answering with `Age: 3590` is saying ten seconds remain:

```text
origin  ──3600s──▶  CDN (holds 3590s)  ──"3600s"──▶  us (holds 3600s more)
```

Each hop honestly obeys the number it was handed, and the response stays fresh for ever.
`storable` now subtracts the age — RFC 9111 §4.2.3 in its two useful terms, the apparent age from
`Date` and the stated `Age`, whichever is larger — and a response whose age has reached its
lifetime is not kept at all. The round-trip correction is left out: it can only make the age
larger, so omitting it errs toward serving rather than toward discarding, which is the wrong
direction to err in silently and is therefore stated here.

**`Expires` did not have this bug, and the reason is worth keeping.** It is an *instant*, so
`expires - now` already nets out the age; subtracting the age from it as well would double-count.
`max-age` is a *duration*, and a duration means nothing without the end it is measured from. A
test pins both halves, including the one that must not change.

`FRESH_FOR` is reduced by the age too, not only the origin's number: our hour is a ceiling on how
stale served bytes may be, and time spent in somebody else's cache is staleness that has already
happened.

### Review: the argument in this file, turned back on itself

The section below says a cap of *"256 pages"* bounds nothing when a page may be 2 MiB. Review
pointed the same argument the other way, and it landed:

```text
100,000 entries held.  bytes reported: 0.
```

`bytes` counted only `Page::body`, so a hundred thousand **empty** responses sat in the map
reporting nothing held, and eviction never ran. **A byte budget that ignores keys and headers
does not bound entries any more than an entry budget bounds bytes.**

`cost()` now counts the whole of what is retained — the key, the duplicated `Page::url`, the
headers, and a flat 256 bytes for the entry and the map's slot — and `MAX_ENTRIES` bounds the
count as well. **Both**, because each alone has a hole shaped exactly like the other.

The test that went with the fix asserted `bytes() > 0`, which the mutation harness then walked
through: a bodyless entry still holds its key, so that assertion passes while counting nothing
but strings — the hole again, spelt differently. It asserts `>= OVERHEAD` now.

### Review: the header a publisher states the commitment in

The section below argues the cache belongs beside `robots.txt` as one commitment about how a
stranger's server is treated. It then ignored `Cache-Control` entirely.

| The origin says | before | now |
|---|---|---|
| `no-store` | kept, and handed to a second reader | not kept |
| `no-cache` | reused with no revalidation | not kept — revalidating needs conditional GET, a later row |
| `private` | kept in a cache shared by every reader | not kept |
| `s-maxage=60` | ignored | kept for 60s — shared caches are told this one first |
| `max-age=30` | served for an hour | kept for 30s |
| `max-age=3600` with `Age: 3590` | served for an hour | kept for 10s |
| `Expires` in the past | ignored | not kept |

**Anything unparseable is treated as not cacheable.** A header we cannot read is not permission,
and being wrong that way costs one extra request rather than one ignored instruction.

**Where the decision lives is part of the fix.** The obvious shape puts the branch in
`Fetcher::get` — read the headers, decide, insert — and that is what the first version did. No
test in this repository can reach that branch: a test server binds loopback and the address
guard refuses loopback absolutely, which is the same limitation stated two sections down. The
harness said so, in the only way it can: *the origin's headers are read and then ignored* —
MISSED. The branch moved into `Cache::insert_allowed`, one call from an assertion, and the
fetcher's remaining share is reading two header strings, the same untestable line as `etag`.

### Review: what a process-long fetcher keeps

*Superseded in part by the second pass above: this was the right observation and the wrong
bound.*

Making one `Fetcher` outlive the analysis also made `robots::Cache` and `Pacer` outlive it.
Both filtered expired entries on lookup and never removed them — free when a fetcher lasted one
run, a leak when it lasts the process. A worker crossing a thousand companies kept a thousand
rule sets and a thousand elapsed instants, none of which meant anything.

Both prune on insert, which is the rare path — a host not seen before — rather than on every
request. Neither can change behaviour: an expired rule set was already ignored, and an elapsed
wait already returns zero.

### Bounded in the unit it claims to be bounded in

A cap of *"256 pages"* bounds nothing when a page may be 2 MiB — it is a cap of somewhere
between a few kilobytes and half a gigabyte. The budget is **32 MiB**, counting keys, headers and
per-entry overhead as well as bodies, alongside a cap of **4,096 entries**; oldest is evicted
first, and a page too large to sit beside anything else is declined rather than allowed to empty
the cache and then not fit.

`FRESH_FOR` is **one hour**, and it is a *ceiling* rather than a policy: it applies when the
origin says nothing, and the origin's own number wins whenever it is shorter. It is a starting
value on the same footing as `AMBIGUITY_MARGIN`. Two readers in one sitting share everything; a
pricing page edited this morning is picked up this afternoon.

**A cached page never claims to be fresher than it is.** `fetched_at` is stored with the body and
returned unchanged, so a claim's `as_of` is the moment the bytes were read rather than the moment
they were handed over. A mutation that restamps it is caught.

### What cannot be measured here, and why

The number this feature exists to change is **requests arriving at somebody else's server**, and
that cannot be counted from inside this repository: a test server binds `127.0.0.1`, and the SSRF
guard refuses loopback — deliberately, absolutely, and with no flag to turn it off, for the same
reason `--ignore-robots` does not exist. *The flag would be used, and the commitment is the
product.*

So there is **no integration test over a socket**, and none was written by weakening the guard.
What is asserted instead:

- the `Cache` directly — a hit, `fetched_at` surviving, byte-bounded eviction, replacement, and
  an oversized page declining rather than clearing everything;
- `Fetcher::get` with **the guard as the instrument** — a loopback URL that comes back as a page
  can only have come from memory, because every path that reaches the network refuses it first;
- that a refusal stores nothing, which is the invariant the ordering rests on;
- that a page stops being served once it is past `FRESH_FOR`, using a test-only clock that ages
  the entries rather than a test that waits an hour;
- that `Cache::insert_allowed` refuses a `no-store` and keeps a `max-age=30` for thirty seconds
  rather than for our hour — **the reason that function exists** rather than a branch in
  `Fetcher::get`, which is on the far side of the same guard;
- that a `max-age` arriving with an `Age` or an old `Date` is honoured to the origin's deadline
  rather than restarted, that a response already past it is not kept, and that `Expires` is
  **not** age-adjusted twice;
- that the robots cache evicts live entries past its host cap and past its byte budget, with
  nothing aged, and declines one rule set too heavy for the whole budget without emptying itself
  first;
- that a `no-store` on a second `Cache-Control` field line reaches `storable`, over a `HeaderMap`
  built with `append`;
- that a headerless `429`, `500`, `502`, `503`, `504`, `400` and `403` are none of them kept,
  that every status HTTP calls cacheable still is, and that an origin stating a `max-age` for a
  `503` is obeyed.

**One mutation was dropped rather than kept as a pin nobody can satisfy.** *"Remember the page
under where the redirect landed rather than under what was asked for"* is a real defect and the
insert path is only reachable through a completed fetch, which loopback forbids — so it came
back `MISSED` for the same reason there is no socket test. A catalogue entry that cannot fail is
worse than none: it reads as coverage. The behaviour is stated in the module doc instead.

`landscape read <origin>` prints what is held, so a person running it against a real site can see
the number this cannot.

### What this does not do yet

**Nothing is shared between processes.** Two workers on one machine, or a restart, start cold.
A shared cache means a store, and the laptop rule is that nothing requires a database —
`docs/ROADMAP.md` keeps that as its own decision rather than a side effect of this one.

**No conditional GET.** `Page` has carried `etag` and `last_modified` since the first fetch and
nothing sends them back. Turning a re-fetch into a `304` with no body is the row's other PR.

**No per-source extraction cache.** The model calls are still paid twice for one page read
twice — which cannot happen inside a run any more, but can across `PROMPT_VERSION` bumps and
across processes. That is the second half of this row.

| | Rust tests | frontend tests |
|---|---|---|
| Run 36 | 850 | 62 |
| now | **881** | **62** |

---

## Run 36 — the market decides what is searched for

**Date:** 2026-08-09 · **Where:** this laptop · **Model:** none. The vocabulary step is
deterministic; nothing here asks one.

Run 35 built the resolver and a diagnostic. **Nothing read it.** An analysis still searched for
what a reader typed, so the label was a fact about a market that changed nothing about the
report. This wires it in.

### The same idea, two rounds, two different answers

Against a stand-in engine that answers the reader's phrasing and the market's differently — as
the real web does, which is the entire premise of §4:

```text
idea      a free competitive landscape research tool
  best a free competitive landscape research tool software
  a free competitive landscape research tool tools comparison
  a free competitive landscape research tool vendors

interpreted as  competitive intelligence software
agreed on by    3 independent sites
also called     competitive intelligence, intelligence software
searched for    competitive intelligence software

company                             agreed    score  shallowest
------------------------------------------------------------------------------
crayon.co                            3/3       1.00  https://www.crayon.co/
klue.com                             3/3       1.00  https://klue.com/
kompyte.com                          3/3       1.00  https://www.kompyte.com/
```

**Round one found the pages that name the market** — G2's category, Capterra's, one vendor's
product page. **Round two found the market.** Those are different sets, and before this change
only the first existed.

### The second round happens only when the words changed

| The reader typed | Rounds | Why |
|---|---|---|
| *a free competitive landscape research tool* | **2** | the market calls it something else |
| *competitive intelligence software* | **1** | it already is the market's name — the hits in hand are the answer |
| anything, market ambiguous | **1** | nothing is searched for until somebody picks |
| anything, no engine | **0** | unchanged |

A test counts the queries for each. Three extra requests to somebody else's server is the price
of the substitution, and it is charged only when the substitution happens.

**The coverage note counts both rounds.** *"1 of the 6 searches did not complete"* is what
happened; reporting only the second would hide an outage in the first behind a tidier number.

### The interpretation is shown, not assumed

```text
Interpreted as competitive intelligence software (also: competitive intelligence,
intelligence software) — 3 independent sites use this name
```

§4 asks for exactly this line and says why: *"If the interpretation is wrong, the user sees it
immediately and fixes it in one click — which is faster and less annoying than any question we
could have asked upfront."*

**It rides on `Asked` rather than being stamped on the finished report**, because the *partial*
reports reach a reader too — `on_progress` fires after every page. A line that appeared only at
the end would tell somebody, ninety seconds in, that what they had been watching had been about
something else all along.

**And it is absent when nothing was substituted.** Repeating a reader's own words back at them
as an "interpretation" is noise, and noise is what stops the real line being read. A test pins
the absence, because that is the half that rots first.

### An ambiguous market is the clarifying question §3 wanted

`PRODUCT_SPEC.md` §3's *"who should I compare against?"* has been waiting on this. When two
markets are backed by the same number of independent sites, the run **refuses before searching**
and offers one chip per market — the same control the ambiguous-company question already uses,
so the interface has one way of asking *which did you mean* rather than two.

A market has no website, so `Choice::domain` is empty and the interface omits it rather than
rendering a blank line where a reader has learned to expect the thing that tells two choices
apart. What stands in its place is the evidence: *"2 independent sites use this name."*

**A label too short to send back is not a label.** `NewAnalysis::parse` refuses under
`MIN_PROMPT` characters, and a chip that renders and then answers the click with a `400` is
exactly register 47, one row earlier. The guard is at the source — where phrases are made — so
every later reader of a label gets one that works by construction rather than by remembering.

### Review: a guard that measured bytes where the API counts characters

The chip guard was written for register 47 — *use the real validator* — and then **re-derived
the rule anyway**:

```rust
if words.iter().map(String::len).sum::<usize>() + words.len() - 1 < MIN_PROMPT
```

`String::len` is **bytes**. `NewAnalysis::parse` counts **characters**. Two hosts titled `Ää Ää`
resolve as a tie, the label is five characters and nine bytes, the guard passes it, and the chip
renders and answers the click with a `400` — the exact failure the guard exists to prevent, for
every non-ASCII market name. It also silently ignored `MAX_PROMPT`, which the parser enforces
and a re-derived rule had no reason to know about.

**The fix is to stop deriving it.** `trimmed` now calls `NewAnalysis::parse` on the candidate
phrase: the thing that validates the click is the thing that validates the phrase, so the two
cannot come apart. Both ends are tested — a five-character phrase and one longer than a prompt
may be.

### Review: one search written three ways, treated as three

`for_market` decided *"did the words change"* by comparing two raw strings. `for_idea` normalises
through `safe_words` before interpolating, and search engines ignore case — so these are **one
search**:

```text
competitive intelligence software
Competitive Intelligence Software
competitive intelligence software!
```

The raw comparison went wrong in **both directions at once**:

| | what happened | what it cost |
|---|---|---|
| a trailing `!` | *"the words changed"* | three more requests to somebody else's server, for identical queries |
| an exact match | reused the hits correctly | and **still** told a reader their own words had been *"interpreted as"* themselves |

The second is the worse one: it contradicts the line's whole purpose, which is to disclose a
**substitution**.

**One answer, derived from the queries rather than from the strings.** `substitutes` compares
what `for_idea` would actually send, lowercased, and `Read::substituted` carries the result — so
the second round and the disclosure are the same decision made once. The worker and the CLI both
read it; neither recomputes it.

### One function, two callers

`candidates::for_market` is the whole sequence, and both the worker and `landscape candidates`
call it. *"A diagnostic that agreed with the worker only by coincidence"* is a defect this
codebase has already had once; the two still differ in what they **say**, and no longer in what
they **do**.

### What this does not do yet

**A chosen market does not stick.** Clicking a chip starts a new analysis whose description is
the market's name — which resolves to itself, so the answer holds in practice, but nothing
records that a reader chose it. §3's *"the answer is remembered for the session"* needs session
state that does not exist.

**No `[Change]` control.** §4's line ends with one; editing an interpretation in place is
[F2](../PROJECT_STATUS.md#f2--editing-the-product-idea), still R0. Retyping works today.

**Nothing measures whether the label is right.** Unchanged, and now one step more consequential:
the label decides the queries. [B1](../PROJECT_STATUS.md#4-blockers).

| | Rust tests | frontend tests |
|---|---|---|
| Run 35 | 840 | 58 |
| now | **850** | **62** |

---

## Run 35 — the market's words, not the reader's

**Date:** 2026-08-08 · **Where:** this laptop · **Model:** none, and that is the design —
`COMPETITIVE_DISCOVERY.md` §9 budgets this step at *"deterministic, no model"*.

Somebody types **a free competitive landscape research tool**. Here is what we sent:

```text
best a free competitive landscape research tool software
a free competitive landscape research tool tools comparison
a free competitive landscape research tool vendors
```

**Nobody in that market uses that phrase.** Those queries find blog posts about the idea of
competitive research. The market says *competitive intelligence software*, and searching its
words finds the companies.

### What it does now

`landscape vocabulary "a free competitive landscape research tool"`, against a stand-in engine
serving five plausible first-page results:

```text
interpreted as  competitive intelligence software
agreed on by    3 independent sites
also called     competitive intelligence, intelligence software

Every query a report builds would use the label, not the words
above it. The other phrasings are shown and never searched.
```

### Counted once per site, never per page

`FACT_CHECKING.md` §6, and it is the whole reason the number means anything. A content farm with
a title template can put one phrase in forty titles, and forty is not agreement — the test feeds
exactly that and requires the answer to be *"the market has no word for this"*:

| Input | Answer |
|---|---|
| 40 pages, 1 host, all saying *battlecard automation platform* | `TheirWords` — nothing recurred |
| 5 pages, 5 hosts, 3 saying *competitive intelligence software* | `Market` — 3 independent sites |

### Review: two markets, and a coin flip presented as a fact

Two hosts titled *Email Marketing Software*, two titled *Project Management Software*:

```text
Market { label: "email marketing software", hosts: 2 }
```

The other market — backed by exactly as many independent sites — was **not returned, not
offered, and not even in the `also` list**, which had been consumed by overlapping fragments of
the winner. It won because `e` sorts before `p`.

`landscape_core::subject::AMBIGUITY_MARGIN` already has the words for this: *"picking the top
one is a coin flip presented to the reader as a fact"*. Here it is worse than picking the wrong
company, because **the label decides every query downstream** — the whole report would be about
a market nobody asked for.

`Resolved::Ambiguous { between }` now carries the competing categories. The grouping is by
**containment**, the relation `most_specific` already used: *email marketing*, *marketing
software* and *email marketing software* are one market said three ways, and *project management
software* is not.

### Review again: the fix had a bridge in it

Connected components over every containment edge was wrong, and the counterexample is better
than the original:

```text
inventory management software   2 hosts   ─┐
project management software     2 hosts   ─┤
management software             4 hosts   ─┴─ contained in BOTH
```

`management software` is on **all four** hosts *because* it is the part two different markets
have in common. Transitivity made one cluster of the lot, and the arbitrary choice came straight
back wearing a bigger number.

**A market is a phrase nothing else extends** — §4 step 4's *most specific term* read as a
definition rather than a search. A shorter phrase belongs to the one market that extends it; a
phrase extended by **more than one** belongs to neither, because it is the overlap between them
and not evidence for either. Dropping it leaves two clusters of two hosts each: a tie, and a
question.

That also removes the fragment problem the first version needed transitivity for. *marketing
software* is extended by exactly one market, so it joins it rather than becoming a third.

**A fractional margin is the wrong instrument.** These are counts of two to five, where 15% is
either zero or everything. Equal corroboration is a tie, and a tie is a question.

### Review: a digit inside a word is not a separator

```text
phrases_in("B2B Marketing Automation Software")  ->  "b marketing automation software"
phrases_in("3D Modeling Software")               ->  "d modeling software"
```

Breaking a run on every digit was right for the `10` in *Top 10 CRM Software* and wrong for
every category that contains one. Two hosts using either title would have promoted a word that
does not exist to a market label.

**A whole word made of digits is a separator; a digit inside a word is part of it.** `10` is the
first; `b2b` and `3d` are the second. Both behaviours now have their own test, because fixing
one at the cost of the other is the obvious way to get this wrong.

### The most specific term that still recurs widely

§4 step 4 in one sentence, and the hard half is *"still recurs widely"*. `competitive
intelligence` is on four hosts; `competitive intelligence software` is on three. The longer one
wins — but only because it **extends the phrase everybody already agreed on**, which is what
keeps *battlecard automation platform* from winning on a lucky two.

**A ratio was the obvious alternative and is worse.** "Widely" as a fraction of the top count
needs a number nobody can defend, and it would move every time the query count moved.
Containment needs none.

### A title is not evidence, and this is not a claim

`Hit::title` says plainly it is the engine's account of a page and *"not evidence of anything"*.
That rule is intact: **nothing this module produces can reach a report as a fact about a
company.** A claim is about one company and must come from that company's own page; a category
label is about *the words a market uses*, and the only way to observe those is across many
strangers' pages at once. Counting them is not trusting any one of them — and §4 requires the
label be shown to a reader as an interpretation they can overrule, precisely because it was
inferred.

### Review sites are the best source here and the worst source next door

`candidates` excludes `g2.com` and `capterra.com` by name — they are places people talk about
companies, not companies. This module deliberately does **not** share that list, because §4 step
5 calls their curated category trees *"the strongest signal available"*: they **are** the
market's own vocabulary. One question is *who is a company*; the other is *what is this called*.
A test pins the inversion, so sharing the list later fails loudly instead of quietly costing the
label its two best sources.

### Our own boilerplate cannot become the answer

We send `best {} software` and `{} vendors`. A title reading *"Software Vendors"* is this
codebase's phrasing looking back at us, and counting it would let a template vote. The guard
reads `IDEA_TEMPLATES` rather than a retyped copy — a second list would go stale the first time
a template changed, and silently.

### A gate for a defect no compiler can see

Writing this found the fourth instance of something this repository keeps producing:

```text
"...worker, migrate, fetch, gap, discover,              search, candidates, ..."
```

A `\` continuation in a Rust string strips the newline **and** the indentation. Lose the
backslash and the indentation stays, baked into a sentence somebody is shown. It compiles and
every test passes.

`scripts/no_lost_continuations.py` is the **thirteenth gate**. It found ten — a CLI message,
five test assertions in `interrupted_runs.rs`, two in `conformance.rs` and two more in
`stages.rs` — all of them closed here. Telling it apart from deliberate column padding needed
two signals together, because this codebase pads columns constantly:

| | gap | what follows |
|---|---|---|
| `"none    no price found"` | 4 | a word — but the gap is small |
| `"{:<58} answers      may set a table value"` | 6 | a word — small again |
| `"  fits in 120s        {:.0} extractions"` | 8 | a format specifier |
| `"...discover,              search, candidates"` | **14** | a word |

Nine spaces or more **and** letters on both sides. A continuation carries the whole indentation
of the line that followed it, which inside a function inside a block is never small; a column
that wide would be unreadable, which is why nobody writes one.

### Review: the gate was blind where it mattered most

The first version matched `"…"` **per line** — and a `\` continuation is *by definition* a
literal that spans several. The three model prompts in `landscape-analyze::stages`, the longest
multi-line literals in the repository, were invisible to the check written to find exactly this.
Reading literals whole, with a small state machine that skips comments, char literals and
lifetimes, found all three.

They are still **deferred**, because editing a prompt needs `PROMPT_VERSION` bumped and the
golden set re-run against a real model — a benchmark, not a lint fix. But deferred **by the
digest of the exact literal**, not by file:

| | before | after |
|---|---|---|
| a new damaged string in `stages.rs` | silently counted as deferred | **fails** |
| one of the three prompts edited | silently accepted | **fails** — that is the decision point |
| a deferred string finally fixed | nothing said | **fails**, so the exemption is removed |

Both failure modes were checked by making them happen.

### What this does not do yet

**It is not wired into an analysis.** This is the first of the row's two PRs: the resolver and
its diagnostic. Nothing a reader sees changes yet — the queries an analysis sends are still
built from their own words.

**It costs nothing to wire.** `resolve` sends exactly `candidates::for_idea`'s queries, which an
analysis already sends to find companies, so the second PR reads titles it is already holding —
zero extra round trips — and adds the *"Interpreted as …"* header §4 asks for.

**Nothing measures whether the label is right**, and there is no golden set for it. The tests
here pin the *arithmetic*, not the taste. [B1](../PROJECT_STATUS.md#4-blockers) again, one step
earlier in the pipeline than usual.

| | Rust tests | frontend tests |
|---|---|---|
| Run 34 | 822 | 58 |
| now | **840** | **58** |

---

## Run 34 — the question, handed back as three buttons

**Date:** 2026-08-08 · **Where:** this laptop · **Model:** none — this is the boundary again,
not the pipeline.

Run 33 gave *"that name matches more than one company"* its own sentence. It could not say
**which** companies, because nothing carried them past the gate:

```text
That name matches more than one company, and we will not guess between them.
Name the one you mean — a website works.
```

The gate had already resolved them. `Resolution::Ambiguous` holds every candidate, with a name
read off its own front page, a canonical domain and a line telling it apart from the others —
and `decide` threw all of it away and kept a `Failure`. So a reader was asked to work out, and
retype, exactly what we had in hand and declined to choose between.

### What it costs to drop a question you already know the answer to

| | before | after |
|---|---|---|
| what the reader is told | a name matched several companies | *which* several |
| what it takes to answer | retype the idea with a company bolted onto it | **one click** |
| what the click sends | — | the candidate's canonical domain, verbatim |
| what an unanswered run leaves behind | nothing | nothing — the list is cleared on the next attempt |

`PRODUCT_SPEC.md` §3 prices a clarification at *one click* and names this exact row —
*"Which Notion do you mean?"*, chips being *"the candidates"*. That is either true in the code
or it is a sentence in a document.

### The choices needed a column, and not the one that was there

`migrations/0001_init.sql` says `failure_reason` is for operators and **never shown verbatim**.
The chips are shown, verbatim, to a reader — so they cannot ride in it, and
`0005_clarification_choices.sql` adds `clarification jsonb` beside it. Two fields, because they
have two audiences; a human-readable field is an output, not a source.

**jsonb rather than a table.** These are read as a whole or not at all, never queried across
analyses, and never joined — a `clarification_choices` table would buy indexing nobody needs and
cost a second write on the failure path.

### The chip carries a prompt, not a name

```rust
prompt: format!("https://{}", c.canonical_domain),
```

A name goes back through the search that produced the tie and can return the same question. A
domain is the one input `subjects_in` reads as *"this company, definitely"*, so a click resolves
in one step. And the whole prompt is built on the server for the same reason `Example.prompt` is:
which words join an idea to a company is one decision, and the parser that reads them back lives
on that side of the wire.

### Review: a chip that rendered, and then refused the click

It shipped as the bare domain, and **`box.com` is seven characters**:

```text
POST /api/analyses  {"prompt":"box.com"}
400  a prompt must contain at least 8 characters, got 7
```

`MIN_PROMPT` is eight. So every company with a short domain got a button that looked like one
click and was a dead end — about a company *we* had resolved and *we* had put in front of the
reader. The chip and the validator are on two sides of one wire, and neither knew the other's
rule.

**`https://` is eight characters by itself**, so an origin is long enough whatever the domain is:
the prompt is valid **by construction** rather than for most inputs. It is also exactly what
`Set::origins` already builds from the same field, so a chip now hands the pipeline the shape it
uses internally instead of a second spelling of it.

**The alternative was to relax the minimum for things that look like domains**, and it is worse
for a reason this repository keeps rediscovering: the rule would then live in two places, and
the second copy is the one that goes stale.

| | asserted where | what it pins |
|---|---|---|
| `a_chip_a_short_domain_cannot_send_is_not_one_click` | `landscape-analyze` | every chip parses **and** resolves back to its own company |
| `every_chip_a_reader_can_click_is_a_prompt_this_endpoint_accepts` | `landscape-api` | the real `choices_from` output through the real `POST`, `201` |

Both run `choices_from` rather than a string retyped to look like it — a hand-written copy is
what stops noticing when the real one changes. A mutation restoring the bare domain is caught by
both crates in one run.

### Four positional arguments became one struct

`fail(id, generation, kind, reason)` was already carrying two descriptions of one refusal, and
the choices would have made five. `Refused { kind, reason, choices }` — a caller passing the
right value in the wrong position is the defect that shape removes, and every `Store` decorator
in the tree now forwards one value instead of three.

### The sentence changes with the affordance

With chips under it, *"name the one you mean — a website works"* asks somebody to type what is
already a button. `describe` takes the count and picks:

```text
That name matches more than one company, and we will not guess between them.
Pick the one you meant:
```

The frontend test asserts the negative as well as the positive — with chips on screen the page
must **not** say *"a website works"* — because an instruction that survives its own affordance
is how a page comes to tell a reader to do the work twice.

### Two places clear the question, and neither trusts the other

A row can hold the choices an earlier attempt left behind. `analysis_from_row` zeroes them unless
the status is `Failed`, and the page reads them only for a failed, ambiguous analysis. Either one
alone would be enough today; both, because offering a chip under a finished report invites
somebody to re-run something they can already read, and the two sides settle it independently
rather than one relying on the other having done so.

### Where this deviates from the specification, deliberately

§3 says *"Skip, just analyze" always produces a complete report*. **It cannot, here.** Skipping
this question means guessing between two companies that share a name, which is precisely what
the gate refuses to do — a report about the wrong Notion is worse than no report, and it would
be indistinguishable from a right one. Every other trigger in that table has a defensible
default; this one has none, so there is no skip on it. Recorded rather than quietly ignored.

### What still does not happen

**One question, not three.** §3 allows up to three, progressive, one at a time. Only the
ambiguous-name trigger has candidates behind it today; the buyer and intent questions need a
router model, and *"who should I compare against?"* needs vocabulary resolution — the last open
row of this group.

**No session memory.** §3 wants an answer remembered so a follow-up does not re-ask. A click
starts a new analysis with nothing carried across, which is right for a URL somebody can share
and wrong for somebody asking twice.

**Nothing measures whether the set is right**, unchanged.
[B1](../PROJECT_STATUS.md#4-blockers).

| | Rust tests | frontend tests |
|---|---|---|
| Run 33 | 816 | 54 |
| now | **822** | **58** |

---

## Run 33 — five situations, and the reader saw one

**Date:** 2026-08-08 · **Where:** this laptop · **Model:** none — this is the boundary, not the
pipeline.

Runs 29 to 32 spent four changes keeping four silences apart inside the analysis: an outage is
not an empty market, a missing engine is not a market with nobody in it, a page we could not read
is not a page about something else. Every one of those decisions is tested, mutated and
documented.

**And then all of them arrived at a reader as this:**

```text
We could not work out which company you meant. Try naming its website — for example,
basecamp.com.
```

`Failure` had two values, `no_subject` and `internal`. Every refusal `decide` produces is a
`no_subject`, and the interface has one sentence for it.

### What that sentence does to each situation

| The run | The reader is told | What that costs |
|---|---|---|
| several products share a name | *"try naming its website"* | the question they could have answered in **one word** is thrown away |
| the search did not finish | *"try naming its website"* | they go and fix a prompt that was **never wrong** |
| we searched and found nobody | *"try naming its website"* | roughly right, by accident |
| no engine is configured | *"try naming its website"* | exactly right — this is the one it was written for |

**The instruction is right for one of the four.** It was written when `no_subject` meant only the
last row, and three situations moved in behind it as the search channel was built.

### The set stays closed, and small

`migrations/0001_init.sql` is explicit that `failure_reason` is for operators and **never shown
verbatim**, because what somebody is told about a failure is a presentation decision rather than
a database field. This change keeps that rule rather than working around it: nothing internal
leaks, and the five sentences are still written in the interface.

What moved is the *situation*. A value earns a place when a reader would **do something
different** — that is the whole test, and it is why *"we found companies and rejected them"*
shares `nothing_found` with *"we found nobody"*. Both are statements about a market and both are
answered the same way; the difference between them is exactly what `failure_reason` is for.

### The situation travels with the sentence

`Decided::Refuse` carries both. They are one decision — the code that picks the words is the code
that knows which of five things happened — and splitting them is how the boundary came to render
all five the same way in the first place.

### One of them must not say "try again"

Only `search_incomplete` is fixed by waiting, and the frontend test asserts the negative on all
four: the ambiguous case must **not** say *"try again"*, the timed-out case must **not** say
*"naming its website"*. A wording test that only checks what a sentence says would pass with two
situations sharing an instruction; the assertion that matters is the one nobody thinks to write.

### Review: the guard was shaped like a verdict, not like the fact

`decide` gave failed queries precedence **inside the `NothingFound` arm**, and there is a way
round it: two queries corroborate a candidate, the third fails, the gate *resolves* it — and
`assemble` then sets it aside because its front page could not be read.

```text
one query failed, the resolved candidate was set aside
kind = NothingFound
```

`Resolved` with an empty set falls past a verdict-shaped check and lands on *"we searched and
found no company we could stand behind"*: a conclusion about a market drawn while a query was
still unanswered, and the one refusal worth retrying offered as one that is not. It contradicts
the invariant written two arms above it.

**The guard is now shaped like the fact it protects.** `set.is_empty() && !queried.failed
.is_empty()` — no members surviving, whatever the gate concluded on the way. A **non-empty** set
still beats an outage, because an answer we already have is worth more than telling somebody to
come back for it, and that half has its own test so it stays a rule rather than a coincidence.

### What still does not happen

**The candidates are not carried to the client.** *"That name matches more than one company"* is
now its own sentence, and it cannot yet name them — `PRODUCT_SPEC.md` §3 wants three chips and
one click. That is the second half of this row, and it needs a column: `failure_reason` is the
operator's, deliberately, so the choices cannot ride in it.

**Nothing measures whether the set is right**, unchanged.
[B1](../PROJECT_STATUS.md#4-blockers).

| | Rust tests | frontend tests |
|---|---|---|
| Run 32 | 806 | 51 |
| now | **816** | **54** |

---

## Run 32 — a report about one company says why

**Date:** 2026-08-08 · **Where:** this laptop · **Model:** none for the set; the analysis that
follows uses one exactly as it always has.

Runs 30 and 31 made both inputs produce a comparison. **What neither made honest was a
comparison that did not happen.** A lone company arrived with this note:

```text
You named basecamp.com, so we searched for the companies it competes with. This report
compares Basecamp (basecamp.com).
```

A comparison claimed and not made — and in three of the four ways it happens, **a search claimed
and not run**. With no `SEARX_URL` nothing was asked at all, and the page read exactly as it did
for a run that asked three times and found nobody.

### Four facts, and a reader acts on each differently

| `NoRivals` | What it says | What a reader does |
|---|---|---|
| `NoEngine` | nothing is configured, so nothing off this company's own site can be reached | configure one |
| `NothingToCompare(why)` | its own page gave us nothing to judge a rival against | name a different company |
| `SearchIncomplete { failed, sent }` | some searches did not come back | wait, and try again |
| `NobodyHeldUp` | we searched, and nothing that came back held up | nothing — this is the answer |

**Only the last is a statement about the market.** The other three are about us or about one
company's website, and a single sentence covering all four is worse than no sentence, because a
reader acts on it and gets the same silence back. That refusal is why this row was left standing
through three PRs rather than half-built inside them — Runs 29, 30 and 31 each name it as
deliberately deferred. This is it paid off.

`NothingToCompare` carries the `NoVocabulary` from Run 31's review, so *"we could not read its
front page"* and *"its front page has no sentence describing what it does"* stay apart the whole
way to the reader. And it says out loud what it is **not** claiming: *"This says nothing about
who else is out there."* Nothing was asked, so nothing about the market has been established.

### One decision for both paths

`alone_because(members, queried, nothing_asked)` is a pure function, and both paths call it. A
description that resolved to a single company and a named company nobody could be found for are
the same shape from a reader's side — one company, in a tool that promises a comparison — and
the two would otherwise each guess at the reason.

`nothing_asked` is a parameter because [`Queried`] cannot tell *"we sent none"* from *"there were
none to send"*: no engine and no vocabulary are both decided before a query exists. That is the
same distinction the whole row keeps making, one level further in.

**More than one company returns `None` whatever else went wrong.** A comparison is a comparison
even if two of three searches failed on the way to it, and a reason for having nobody printed
beside *"this report compares X and Y"* would contradict the line above it.

### The note stops claiming what it did not do

```text
You named basecamp.com, so this report is about it.
We did not look for companies it competes with: no search engine is configured here, so
nothing off this company's own site can be reached.
```

The first note now branches on whether the set is alone, in both the seeded and described cases,
so the word *"compares"* appears only when there is a comparison.

### And the diagnostic shows it too

`landscape candidates` prints the reason under the set, through one `set_as_printed` shared by
both its paths. A diagnostic that showed the comparison but not the reason for its size would
show **less than the report does** — which is exactly the drift Run 31's last two review rounds
were about, so it was closed in the same change rather than found later.

### A guard that compares text, and a formatter that rewrites it

Running `cargo fmt --all` while a catalogue held a file open stopped that catalogue and kept its
backup, exactly as Run 31's review intended. What it left behind was the mutated file **after
`rustfmt` reflowed it** — and `scripts/no_live_mutations.py` reported a clean tree, because the
swap was no longer the recorded string character for character.

The obvious fix is wrong in an instructive way: collapsing runs of whitespace before comparing
would **not** have caught this one, because `rustfmt` drops the trailing comma when it joins
lines as well as removing the newlines. A normalisation is a claim about what the other tool
does, and that one was a guess — its own self-check is what said so.

What closes it needs no guess. A `*.mutate-backup` on disk means exactly two things, both of them
*"this tree is not what it looks like"*: a run died holding a file, or the harness refused to
restore over an edit. Neither depends on what the mutated text looks like afterwards. It is one
`glob`, and it is the first thing that gate now checks.

### Review: one silence, two searches, and a remedy that could not work

**A reason written for one path, printed on the other.** `NoRivals::NobodyHeldUp` said *"we
searched for companies it competes with and none of what came back held up"* — true on the
seeded path, and on a description's report it produced two adjacent notes contradicting each
other:

```text
You described a market ... This report is about Plausible (plausible.io).
We searched for companies it competes with and none of what came back held up.
Why each one is here: Plausible - 3 of the 3 searches returned it, ...
```

Plausible **is** what came back, and it held up; and neither search was for Plausible's rivals.
`Sought` is carried on the two variants that describe a search, rather than passed to
`sentence()`, so a caller cannot supply the wrong one — each path states it once, where it knows
it. The description's version of that silence is *"only one company matched that description
well enough to report on"*, which is the same fact without the contradiction.

**And the no-engine branch blamed the engine for a page it had already failed to read.**

```text
seed.words()  = Err(Unreadable)
the report says:     ... no search engine is configured here ...
the diagnostic says: ... nothing would have been asked with it either ...
```

Configuring `SEARX_URL` would have changed nothing — `of_company` sends zero queries without
vocabulary — so the report recommended a remedy that cannot work, while the diagnostic beside it
said so correctly. **A reader who acts on a report and gets the same silence has been sent
somewhere for nothing**, which is the failure this whole row exists to prevent, appearing inside
the row that prevents it.

`NoRivals::when_no_engine_is_configured` gives the missing vocabulary precedence, because it is
the fact that makes the engine irrelevant. The regression asserts the **parity**: across the
three seed shapes, the report blames the engine exactly when the diagnostic says an engine would
help.

### What still does not happen

**Chips.** A one-word name the gate cannot choose between is still refused in prose, naming the
candidates. `PRODUCT_SPEC.md` §3 wants three chips and one click.

**Nothing measures whether the set is right.** Unchanged, and now the only thing about this
feature that is not honest about itself: `NoRivals::NobodyHeldUp` is a statement about a market
made from three searches, and there is no recall number behind it.
[B1](../PROJECT_STATUS.md#4-blockers).

**And no engine has been asked**, still: Docker's daemon does not start where this was built, so
every test is a canned provider over the real code path.

| | Rust tests | frontend tests |
|---|---|---|
| Run 31 | 792 | 51 |
| now | **806** | **51** |

---

## Run 31 — a company brings its rivals

**Date:** 2026-08-08 · **Where:** this laptop · **Model:** none for the set; the analysis that
follows uses one exactly as it always has.

Run 30 turned a *description* into a set. A **named** company still produced a report about
itself alone — `basecamp.com` typed into a competitive landscape tool got a profile of Basecamp,
which is answering a different question from the one that was asked.

### The case P22 was written for, rather than the exception to it

`FACT_CHECKING.md` P22 requires *"templated queries from resolved entities, not user phrasing"*.
[`candidates::for_idea`] documents at length why it cannot comply: there is no resolved entity
when all anybody has typed is a description, so the reader's words go to an engine exactly once,
to produce a *set of companies* and never a fact.

Here there is a resolved entity — the reader named the domain — and the name in the query comes
off the company's **own front page**:

```text
company   Basecamp (basecamp.com)
it says   Project management and team communication
query set 2026-08-08.1
  Basecamp alternatives
  Basecamp competitors
  companies like Basecamp
```

Nothing a reader phrased reaches the engine. It is the first retrieval path in this codebase that
complies with P22 without an argument attached.

### Reading one box three ways

| Typed | Read as | Why |
|---|---|---|
| a description | search for the companies | Run 30 |
| **one** domain | seed a landscape and find the rest | this run |
| **two or more** domains | exactly those | a reader has said what to compare |

**The third row is a decision, not an omission.** `basecamp.com vs linear.app` is somebody
saying which comparison they want; adding a third company we happened to find would be overruling
them. That rule is `subject::subjects_in` — a function with a name, because it is a judgement
about what somebody meant and worth being able to assert. Four tests, three mutations.

### The seed is dropped from its own results

A search for *"Basecamp alternatives"* returns Basecamp, and it outscores every other host on the
page — it is the subject of all three queries. Left in, one company appears twice: once as *"you
named it"* and once as *"3 of the 3 searches returned it"*, the second of which is circular.

### Two kinds of reason, and `Because` became an enum for it

```text
Basecamp (basecamp.com)      you named it
Linear (linear.app)          3 of the 3 searches returned it, and its own front page uses
                             "project", "management"
```

`Because` was a struct with `agreed`, `asked` and `shares`. It could only say the second thing —
so putting a named company in the same list would have meant inventing a search nobody ran, in
the field a reader is shown as evidence. **A reader who typed a domain has already answered the
question**; a company we searched for has to justify itself, and those are not the same kind of
fact.

### The vocabulary floor, pointed at the only words there are

On the description path a rival has to share a word with what the reader typed. On this path the
reader typed a domain, so the words come from the **seed's own one-line description of itself**
— the same `SHARED_WORDS = 1` floor, making the same narrow claim: a company whose front page has
nothing in common with the seed's is not obviously in the same market. A bakery that a search for
*"Basecamp alternatives"* somehow returned is set aside and **named**, as on the other path.

### What the anchor gate caught on its first working day

Extracting `from_its_own_page` so `describe` and `named_seed` share one copy of *"we were unable
to read its front page"* moved the line an existing mutation pinned.
`scripts/mutation_anchors.py` — added in Run 30's review — reported it before the harness was
run at all:

```text
crates/landscape-search/src/candidates.rs
  a candidate we could not read vanishes from the list instead of saying so   missing
```

That is the class it was written for, caught in a second rather than in a twenty-minute run whose
output would have read *"26 of 27"*.

### Two mutations that came back MISSED, and both were real

`named_seed` had no test at all: every rival test passed a hand-built seed straight to
`of_company`, so a version that fetched the front page and then ignored it passed the suite. And
nothing pinned that a rival's score divides by the queries **sent** rather than the ones that
answered — the rule Run 29's review established, which has to hold on both paths and was only
tested on one.

### What still does not happen

**The seeded path has no honest empty.** With no `SEARX_URL`, or with every query failing, a
named company gets a report about itself and nothing on the page says we looked. That is *"no
public information at the level of a whole competitor set"* — its own row of S2, deliberately not
half-built here: one sentence covering *no engine*, *the search did not finish* and *we looked and
found nobody* is precisely the collapse this project has un-made three times, in Runs 29, 30 and
30's review. **Built in Run 32**, as four sentences.

**Nothing measures whether the set is right**, on either path. Three searches agreeing and a
shared word is the whole of the evidence, and there is no recall number. That is
[B1](../PROJECT_STATUS.md#4-blockers) and it is now the only thing holding this feature back that
is about the feature rather than about the product around it.

**And no engine has been asked**, still: Docker's daemon does not start where this was built, so
every test is a canned provider over the real code path.

### And the harness put a file back over two of those tests

Writing them took one command; only one of the two survived. A **different** catalogue was still
running, `mutate.py` had copied `candidates.rs` before mutating it, and the `finally` that puts
the copy back predated the edit. The sibling test in `competitors.rs` lived, because that file
was not in the catalogue that was running — so one of two tests written together disappeared,
which looks like nothing at all. `cargo nextest` passed: a test that does not exist does not fail.

**The only thing that noticed was the mutation the test had been written for.** The harness,
aimed at the harness.

`mutate.py` now reads the file back before restoring and compares it against what it wrote. If
they differ, the backup is **not** put back:

```text
CHANGED      crates/landscape-search/src/candidates.rs was edited while this ran.
    The original is in candidates.rs.mutate-backup and has NOT been put back, because
    restoring it would delete whatever was just written. Merge it by hand.
```

Register entry 43, and the third about the same window: 41 was a run that died holding a file,
42 was a pin that came loose under one, this is one that wrote over somebody's work. **A tool
that edits source in place is a second author**, and every rule about two authors sharing a file
applies to it.

### Review: our own error message, used as evidence about a company

The vocabulary a rival has to match comes from the seed's one-line description of itself. When
the seed's front page cannot be read, that field holds *our sentence about the failure*:

```text
seed.what_it_is = "we were unable to read its front page"
content_words   = ["were", "unable", "read", "front", "page"]
a rival page saying "# Toolbox\n\nRead more on this page."
shares          = ["read", "page"]
```

A corroborated but unrelated company joins the comparison, and `Because::sentence` tells the
reader *"its own front page uses \"read\", \"page\""* — citing our error message as the
company's evidence. Same shape as the substring finding in Run 30's review, from a direction
nothing was watching.

**The root cause is where the fact was kept.** `named_seed` returned a bare `Candidate` and put
*"we could not read this"* into `what_it_is`, a field written for a reader — so anything deriving
data from it inherited the sentence's second job. It is a `Seed { candidate, read: bool }` now.
A fact about whether a fetch succeeded belongs in a `bool`, never in prose.

**And with no vocabulary, no rivals — and no queries either.** Searching, then fetching five
front pages on other people's servers, to conclude that none of them can be judged is work
nobody should pay for. The regression asserts `queried.sent() == 0`, not merely that the members
list is short.

### Review: a warning is not an abort

The concurrent-edit guard above printed `CHANGED` and carried on. Execution reached the normal
result handling, `main` kept going, and a catalogue could finish `all 17 caught` and **exit 0**
having announced that every result after that point was uncontrolled — and a later mutation of
the same file would have copied over the preserved backup.

Reproduced with two mutations on a scratch file, the first one's `run` editing the file while it
"tests":

```text
  STOPPED      subject.rs was edited while this ran.
      The original is in subject.rs.mutate-backup and has NOT been put back...

exit code: 3
backup kept:      True
second entry ran: False
```

`SourceChanged` is raised rather than counted, because a result measured against a file somebody
else wrote is not a result. Raised **after** the `finally` so it cannot mask a failure in the run
above; a pre-existing `.mutate-backup` aborts at the top of `apply` for the same reason; and
`restore()` is a function rather than a branch, with a `self_check()` exercising both directions
on real files every invocation — the same discipline `verify.py` applies to its own comparison.

### Review, again: the gate was on the wrong question, and the sentence named nobody

One run reproduced both:

```text
read       = true
what_it_is = ""
words      = []
queries sent = 3
  set aside linear.app -> ElsewhereEntirely
     says: its own front page uses none of the words you typed
```

**`read` describes what happened to the request; the gate is about what there is to compare
against.** Those coincide until a page is reachable and says nothing quotable — `naming` wants a
line of four words before it will quote one, so `# Basecamp` is `read == true` with an empty
`what_it_is`. Three queries went out, a front page was fetched, and every corroborated rival came
back `ElsewhereEntirely`: **a negative claim about those companies, made out of missing evidence
about the seed.** The same shape as the two findings before it — evidence about A produced from
an absence at B.

One predicate now, `Seed::vocabulary()`, folding both ways of having nothing, and computed before
the first query because that is the only place *"no queries either"* can be true. It returns the
words rather than a `bool`: the caller needs them anyway, so there is no second place to derive
the answer slightly differently.

**And the exclusion sentence blamed the reader for words we chose.** *"None of the words you
typed"* is true when a description was searched for and false on the seeded path, where the
words come off the seed company's own front page. It named none of them, so nothing could check
it. `Aside::ElsewhereEntirely` carries them now:

```text
its own front page uses none of the words this comparison is built on: "project", "management"
```

True on both paths, and a reader can judge the exclusion rather than take it — the same reason
`Because::Found` names its count and its shared words.

### Review, a third time: the diagnostic asked a different question

```text
the worker asks:      seed.vocabulary().is_empty() = true
the diagnostic asks:  !seed.read                   = false

worker      -> 0 queries sent
diagnostic  -> prints 3 queries, and says nothing about it
```

`of_company` was fixed and `landscape candidates <domain>` was not. **Two predicates for one
decision**, disagreeing exactly on the case the fix had just added — and this command exists to
be the auditable view of what the worker really does, so a version that agrees by coincidence is
worse than none, because it is believed.

`Seed::vocabulary() -> Vec<String>` became `Seed::words() -> Result<Vec<String>, NoVocabulary>`.
Both callers match on it, and the return type is a **reason** rather than a `bool` so the two
cannot describe the same outcome differently. The two silences are told apart at last: a page
nobody could read is about the company's server, a page with no prose is about what the company
chose to put on it, and only the first is worth trying again.

The diagnostic's query section is `what_would_be_asked(&seed) -> Vec<String>` — lines rather than
a run of `println!`s, so it can be asserted against the decision it describes. The regression is
the **equality**: for each of the three seed shapes, *"nothing is asked"* appears exactly when
`words()` fails, and a query is shown exactly when one would be sent. A test that pins today's
wording would not have closed this; one that pins the equality makes a future change move both
sides.

### And the footer under it still disagreed

```text
asking    nothing - its front page has no sentence describing what it does
...
SEARX_URL is not set, so nothing was asked. The queries above are what would go.
```

There are no queries above — and the worse half is the implication, that installing an engine
would change the outcome. It would not: this company's own page gave nothing to compare a rival
against, so an engine would be asked nothing either. **A reader who acts on a diagnostic and gets
the same silence has been sent somewhere for nothing**, which is the failure this command exists
to prevent.

The footer reads `seed.words()` too now, and the regression is the whole output rather than one
section: for each of the three seed shapes, neither a query nor the phrase *"queries above"*
appears unless `words()` succeeded. The previous round asserted that equality for the query
section alone, and the hole was the surface next to it.

**The description path has a byte-identical footer**, and the first edit replaced that one
instead. It failed to compile, which is the only reason it was noticed. That one is correct
without a condition — `queries` is non-empty there, guaranteed by an early return four lines
above — and now says so, because the next person to grep that sentence will find two.

| | Rust tests | frontend tests |
|---|---|---|
| Run 30 | 767 | 51 |
| now | **792** | **51** |

---

## Run 30 — one idea, several companies

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none for the set; the analysis that
follows uses one exactly as it always has.

A description produced **one** company. This product is a competitive landscape tool, so that
was the wrong number, and it was the wrong number for a reason worth writing down.

### The gate's most common answer for a market was "which one did you mean?"

Three companies that all three differently-worded searches returned score identically:

```text
0.8 x 3/3 + 0.2 = 1.0    Fathom
0.8 x 3/3 + 0.2 = 1.0    Plausible
0.8 x 3/3 + 0.2 = 1.0    Simple Analytics
```

`resolve` compares them against `AMBIGUITY_MARGIN`, finds them tied, and asks the reader to pick
one. **That is correct for the question it was asked.** Three products sharing a name is exactly
what `PRODUCT_SPEC.md` §3 wants a chip for, and one chip click really does prevent an entire
wrong report.

It is also, for a market description, the *usual* answer — so answering *"privacy-friendly
website analytics"* meant answering a question about a landscape with a question about a company.

### Telling a market from a name, with arithmetic

| The reader typed | Content words | What a tie means |
|---|---|---|
| `Notion` | 1 | Several products share a name. Ask. |
| `privacy-friendly website analytics` | 4 | Several companies share a market. Compare them. |

`DESCRIBES_A_MARKET = 2` is that rule and it is deliberately crude. It is checkable, it is
explainable to a reader in one line — `landscape candidates` prints *"reading a market"* or
*"reading a name"* — and it needs no model. **What it cannot do** is tell *"Notion project
management"* from a two-word brand, and nothing here pretends otherwise: that is a chip, and
chips are the clarifying-question row.

**No protection was traded for the feature.** The gate keeps its job for the name case, and it
still stops a report when nothing was found or the searching did not finish. What changed is
what happens *after* it has no objection.

### Why each company is in the report, in countable terms

```text
Fathom (usefathom.com)
    3 of the 3 searches returned it, and its own front page uses "privacy", "analytics"
```

Two numbers and a list of the reader's own words. Nothing here is a summary somebody would have
to trust, for the same reason the score is arithmetic over URLs: a reader asking *why is this
company in my report* deserves an answer rather than a shrug.

The divisor is the searches **sent**, not the ones that answered — the rule Run 29's review
established for the score, carried into the sentence. An engine that answered twice out of three
must not produce *"2 of the 2 searches returned it"*.

### Five ways to be left out, and three of them look identical from outside

| Set aside | What it says |
|---|---|
| `Uncorroborated` | One search found it, so nothing corroborates it |
| `Unconvincing` | Corroborated and still below the floor |
| `ElsewhereEntirely` | Its front page **was read** and uses none of your words |
| `Unread` | Its front page was requested and could not be read |
| `BeyondTheFetchBudget` | It ranked below the front pages we were willing to fetch |

**The last three all produce an empty list of shared words and only the first is about the
company.** `Described::shares` started as `Option<Vec<String>>`, where `None` meant *the page
could not be read* — and review then found a company that was never fetched at all, with nowhere
to put it that was not a lie about one of the other two. It is a three-state `Vocabulary` now:
`Read(words)`, `Unreadable`, `NotRequested`. An `Option` could hold two of those, which is why it
held the wrong two.

Every company left out is **named** in the report. A competitor dropped in silence is the defect
this row exists to remove, one company further out than the one it removed first.

### What the reader sees

Three notes, in this order, above everything else:

```text
You described a market rather than naming companies, so we searched for them. This report
compares Fathom (usefathom.com) and Plausible (plausible.io). ...

Why each one is here: Fathom - 3 of the 3 searches returned it, ...

Also found and not compared: Notion Press (notionpress.example) - its own front page uses
none of the words you typed.
```

Three rather than one paragraph, because they are three facts and a reader who stops after the
first still knows what they are looking at. The note names only the companies the run actually
**read**: `MAX_SUBJECTS` is three, and naming five while comparing three would be the silent-drop
defect with better prose in front of it. There is a test for that specifically.

### The stop list that could not grow

The tempting additions to the grammar-word list are `software`, `platform`, `tool` and `app` —
they appear on every front page and carry almost nothing. Dropping them would turn `tax software`
into **one** content word, which `DESCRIBES_A_MARKET` then reads as a brand name and answers with
a question. A weak content word is still a content word, and the list stayed at grammar.

The same restraint applies to `SHARED_WORDS = 1`. It is a floor, not a filter: the only claim it
makes is that a company whose front page uses *none* of the words somebody typed is not obviously
in the market they described. It cannot tell a good competitor from a mediocre one and does not
try.

### What still does not happen

**A named company still produces a report about itself alone.** `basecamp.com` gets Basecamp, not
Basecamp and its competitors — the set is derived from a *description*. That is the next PR of
this row and the more useful half.

**Nothing measures whether the set is right.** Three searches agreeing and a shared word is the
whole of the evidence. There is no recall number, so nobody can say how many real competitors
were missed. That is [B1](../PROJECT_STATUS.md#4-blockers), and with the search channel finished
it is the only thing holding this feature back on its own merits.

**And no engine has been asked**, still: Docker's daemon does not start where this was built, so
every test is a canned provider over the real code path.

### Review: the cap that pre-empted the guarantee, and evidence made of spelling

Two [P1]s, both reproduced before anything changed.

**`from_results` truncated to five before the set could see the list.** Six corroborated
companies in, five out — and the sixth was neither compared nor reported as excluded:

```text
six corroborated companies searched; from_results kept 5
   c0.example agreed 3   c1.example agreed 3   c2.example agreed 3
   c3.example agreed 3   c4.example agreed 3
```

That cap was written for *a list a reader picks one company from*, where five is generous. A set
is a different consumer of the same list, and against it the cap was exactly the silent drop this
row exists to remove — the guarantee was being broken one line above the code that makes it.

The budget itself is real: every name costs a request against somebody's server. So it moved from
*truncating* to *fetching*. `from_results` returns the complete ranked list, `describe` fetches
the first `NAMED = 5` and marks the rest `Vocabulary::NotRequested`, and `assemble` turns those
into `Aside::BeyondTheFetchBudget` — **a named exclusion a reader can act on**, rather than an
absence nothing records.

**And `shared` matched substrings.** `content_words("art marketplace")` yields `art`; a front page
reading *"Tools for startups"* contains those three letters inside `startups`:

```text
words   = ["art", "marketplace"]
shares  = ["art"]
because = 3 of the 3 searches returned it, and its own front page uses "art"
```

One shared word is enough to admit a company, so a corroborated but unrelated result entered the
report and the sentence explaining why cited a word the page never used. **Evidence manufactured
out of a coincidence of spelling** — the failure this project's whole quoting discipline exists
to prevent, reached by a different route.

The doc comment defending the substring match said it was needed to find `analytics` inside `web
analytics` and `Analytics.` — and **it never was**: splitting on non-alphanumerics finds both,
plus `(analytics)` and `analytics/`. The justification had never been checked against the
alternative it rejected. What is genuinely lost is plurals, and being strict is the safe
direction for evidence a reader is shown.

**One note changed shape as a consequence.** With nothing truncated, every one-hit host reaches
`set_aside`, and naming twenty of them turns a report note into a search results page. The line
is corroboration: a company two searches agreed on **could** have been in the report and is named
with its reason; the rest are counted — *"3 further sites returned by only one search"* — which is
a number rather than a silence, and `landscape candidates` still prints every one.

### And then the fix broke the caller the cap was written for

The next review round found the same defect pointing the other way. Removing the truncation so the
*set* could see every company also handed every company to the *gate*:

```text
the reader is offered 6 choices:
   Company at https://c0.example/ (c0.example)
   ...
   c5.example (c5.example)
```

Six choices where `PRODUCT_SPEC.md` §3 asks for three, bounded by how many results a provider felt
like returning — and the last is a bare domain printed twice, because candidates past the fetch
budget were never named from their own pages. *"Which of these did you mean: c5.example
(c5.example)?"* is not a question anybody can answer.

**One number was being asked two questions.** *How many companies did we find* and *how many can a
reader be asked to choose between* are different, and no value of a single constant is right for
both:

| Consumer | Gets | Because |
|---|---|---|
| `competitors::assemble` | every company found | one it cannot see is one nothing can report as excluded |
| `subject::resolve` | the fetched-and-named subset | a candidate with no name of its own is not a choice |

The split is `Described::was_requested()` — a property of the candidate rather than a second
comparison against `NAMED` somewhere else, because a caller redoing that arithmetic is how the
two lists drift apart again. `NAMED`'s doc comment now names **both** jobs, which is the whole of
what went wrong the first time.

### And a harness result that was about code nobody wrote

Confirming the fix nearly produced a worse mistake than the fix. The catalogue takes longer than
ten minutes; a foreground run with a ten-minute timeout was killed mid-mutation, and
`mutate.py`'s restore is a `finally` that a `SIGTERM` does not always reach. The mutated line was
left in the tree.

**The re-run then made that the baseline.** Every entry copies the current file and restores it
afterwards, so thirty-eight mutations were measured against a tree with the defect in it — all
reporting `caught`, counts adding up, output indistinguishable from a clean run. The only signal
was the thirty-ninth reporting `NOT APPLIED`, which reads like a stale anchor.

`scripts/no_live_mutations.py` exists for exactly this and opens by saying *"this exists because
it happened"*. It is `verify.py`'s first gate — which runs long after every mutation result has
been read and believed. So the same check now runs at the **start** of `mutate.py`, which refuses
to measure anything while a recorded defect is live and says which one. A harness whose
correctness depends on remembering to check something by hand is the defect this project keeps
recording, one level up.

### Five regression pins that had come loose

Review then found the other way a catalogue stops meaning anything: `cargo fmt` had collapsed a
`subject::resolve(...)` call onto one line, so the mutation pinning the ambiguity behaviour could
no longer be applied. The harness prints `NOT APPLIED` and carries on; the run ends *26 of 27*,
which reads like a coverage gap rather than a pin that came loose.

**Running every catalogue after every change is not the fix** — it takes tens of minutes, which
is why `verify.py` never ran them at all. Checking that each `old` still appears in its file
exactly once takes a moment. That is `scripts/mutation_anchors.py`, now the **second** gate, and
what it found on its first clean run is the point — not one loose pin, **five**, across four
catalogues:

| Pin | Loosened by |
|---|---|
| the ambiguity call | `cargo fmt` reflowing it, two commits back |
| a changelog needs no model | `Answers::Direction` joining the `matches!`, Run 24 |
| the model's health | a local becoming a struct field, Run 27 |
| a company dropped in silence | `let notes` becoming `let mut notes` |
| a trust page reaching its extractor | the dispatch moving to another file entirely |

Each of those catalogues reported *"all caught"* on the day it was written and had been proving
less ever since. **A suite of regression pins decays exactly like a suite of tests, except that
nothing fails when it does.**

**Two of the five, once retargeted, came back `MISSED`** — the pins had been loose long enough
that the properties underneath had lost their coverage:

* **A changelog read while the model is down.** The gate sits inside `read_one`, reachable only
  through a real fetch, and the fetcher refuses `127.0.0.1` — so no test in the crate got there.
  It is `can_read(question, model_ready) -> Option<bool>` now: `Some(true)` for a deterministic
  question whatever the model is doing, `None` when nobody has asked yet. The case that matters
  is assertable at last — *the model is known to be down and the changelog is still read*.
* **A trust page reaching its extractor.** `trust-posture.json` pinned what
  `assurance::every_assurance` finds and what the judge accepts; nothing asserted that
  `Answers::Trust` still *arrives* there. An empty outcome for a trust page passed the whole
  suite — a page fetched, read and thrown away in silence.

All five retargeted, both gaps closed, and 203 anchors now check in a second.

| | Rust tests | frontend tests |
|---|---|---|
| Run 29 | 738 | 51 |
| now | **767** | **51** |

---

## Run 29 — typing an idea runs

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none for the resolution; the analysis
that follows uses one exactly as it always has.

`PROJECT_STATUS.md` has said *"a business idea does not run at all"* in the S2 row since the row
existed. **It no longer does.** A prompt naming no domain is searched for, grouped into
companies, scored, named, and put through the disambiguation gate — and when the gate returns one
company, the run proceeds against it.

### Three verdicts, three different things to tell a reader

The gate has always had three answers. Until now the worker had one:

```text
this prompt does not name a website, and finding one from a description needs the
search channel that is not built yet
```

That sentence had also gone stale — the channel is built. What replaces it is one refusal per
verdict, because a reader can act on all three and the actions differ:

| | What a reader is told |
|---|---|
| **Resolved** | Nothing. The report runs, and its **first note** says which company was chosen and that they did not name it |
| **Ambiguous** | The companies, by name and domain: *"that description matches more than one company and we will not guess between them"* |
| **Nothing found** | That we looked and found nobody, which is not the same as having no way to look |
| **No engine** | That finding a company from a description needs one, which is the only case where the answer is *install something* |

**The first and the last used to be the same sentence.** *"We have no way to look"* and *"we
looked and found nothing"* are different facts about the world, and the honest-negative
discipline this project applies to a section applies to a subject.

### The note is first, and that is not styling

A reader who typed a description **never said this company's name**. Every other note on a report
is about what the run did — which pages the budget skipped, which questions were searched. This
one is about *who the report is about*, and it is the single thing a reader cannot check by
reading further down the page. It is inserted at position zero and there is a test that it is,
not merely that it exists.

### One engine, read once

The worker asked the environment for `SEARX_URL` twice: once to resolve a description, once to
fill a company's gaps. Two answers that can disagree, for one question — the second source of
truth this codebase keeps deleting. It is read once and passed to both.

### And clippy found the shape before a reader did

`analyse_many` reached **eight positional arguments**, four of them `Option`s or timestamps.
The lint was right, and the codebase already had the answer: `Reading` exists because *"threading
seven arguments through each of those is how a caller ends up passing the right value in the
wrong position"*. The new `Asked` is that, for the same reason, rather than an `#[allow]`.

### Two lost line continuations, one of them shipped

Writing the new note produced a string with a run of eighteen spaces in the middle of a sentence
— a `\` continuation eaten on the way into the file. The test caught it. **The note beside it
had the same defect and had been shipping it**: *"Each company is its own              discovery"*
is what a reader saw whenever a fourth company was dropped. Both fixed.

The same slip is in three of the model prompts and is **not** fixed here: changing a prompt
changes what the model reads, so it needs a `PROMPT_VERSION` bump and a golden-set run rather
than a quiet edit. Flagged rather than folded in.

### Review: an outage reported as an empty market

The first version returned a count of failures and did nothing with it. With all three provider
calls returning `Err`, `suggest` produced no candidates, `resolve` returned `NothingFound`, and a
reader was told:

```text
we searched for companies matching that description and found none
```

That is a conclusion about a market drawn from a conclusion about nothing — and the `checked`
list handed over as evidence of the looking held **all three queries**, including the two that
never reached an engine. `FACT_CHECKING.md` §5.4 says a negative nobody can check is not a
finding; a query that never ran is not a check.

`Queried { completed, failed }` replaces the count, and it exists because **two rules here pull
in opposite directions and both are right**:

| | Divides / lists by | Why |
|---|---|---|
| The confidence score | queries **sent** | An engine that answered once of three has produced no agreement. Dividing by what answered lets an outage manufacture unanimity. |
| The audit trail | queries **completed** | It is shown to a reader as *"we checked these"*, and a query that never ran checked nothing. |

The refusal splits in two as well. `search_incomplete` is about **us**, is retryable, and names
the counts — *"we could not complete 3 of the 3 searches"* — where `NOTHING_RESOLVED` remains a
statement about the market, reachable only when the searching finished.

Ambiguity deliberately outranks the outage: several real candidates are still asked about even
when a query failed, because telling somebody to try again would throw away an answer we already
have. There is a test for that branch specifically.

**The match moved out of the binary.** It lived in `main.rs`, where a mutation aimed at the
outage arm reported `NOT APPLIED` and no test could reach it. `landscape_analyze::subject::decide`
is the same three-into-four decision somewhere it can be asserted — five unit tests, three
mutations, all caught. That is the same lesson as `worth_searching` and `ToRead` before it: a
decision inside a worker is a decision nobody can test.

### What still does not happen

**A description produces one company, not a set.** Competitor-set derivation is its own row, and
until it lands a description gets you one company's report rather than a comparison —
`landscape candidates` remains the way to see the whole list.

**And no engine has been asked**, still: Docker's daemon does not start where this was built, so
every test is a canned provider over the real code path.

| | Rust tests | frontend tests |
|---|---|---|
| Run 28 | 725 | 51 |
| now | **738** | **51** |

---

## Run 28 — a description becomes companies, and the gate finally has something to gate

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none, and that is the finding as much
as the feature: **nothing in this step asks a model anything.**

`landscape-core::subject::resolve` — the disambiguation gate — was built in Phase 1 *before*
anything could feed it, deliberately: written afterwards it would have been a check bolted onto a
pipeline that already ran without it. It has had nothing to gate for six phases. This is
`FACT_CHECKING.md` §3.1 step 2, and it is what the gate was waiting for.

### The score is arithmetic over URLs, and that is a decision

The gate compares two scores against `AMBIGUITY_MARGIN` and decides whether to interrupt a
reader. **A score nobody can explain makes that decision unaccountable**, so every input to it is
countable from a URL:

| Signal | Weight | Why it is evidence |
|---|---|---|
| Queries that returned this host | 0.8 | Agreement between differently-worded questions is the closest thing to corroboration available before a page is read |
| Depth of its shallowest URL | 0.2 max | A company's front page is `/`; a page *about* a company is four levels into somebody's blog |

**Nothing reads a title or a snippet.** `Hit` carries both so a person running the diagnostic can
see what came back, and `admit::Found` drops them so engine prose cannot reach a report. A score
is the same problem with a shorter supply chain: letting an engine's summary move a company up a
list a reader chooses from is laundering somebody else's opinion into our recommendation.

Three consequences fall out of the arithmetic, and each has a test:

* **Volume is not agreement.** One query returning a host five times has said one thing. Counted
  per query, not per hit, or a listicle farm outranks a company two queries both found.
* **An outage is not unanimity.** The divisor is the number of queries *sent*, not the number
  that answered — otherwise two failed searches turn a single hit into a confident answer.
* **Agreement can never be outweighed by a shallow URL.** A front page found once must not beat a
  company every query agreed on.

### The reader's own words, once, and the reason that is allowed here

`FACT_CHECKING.md` P22 requires **"templated queries from resolved entities, not user phrasing"**.
This module interpolates the reader's description, and that is not an exception to the rule: there
is no resolved entity yet, and finding one is the job. The words go to an engine once, to produce a
*set of companies* — never to produce a fact — and everything downstream is templated from the
domain the gate resolves to.

The asymmetry is what makes it safe. A biased query puts the wrong company on a list a reader then
looks at and can reject. A biased query against a resolved company puts a biased **fact** in a
report, where nobody can see it.

That description is also **the freest text this system accepts**, arriving from a stranger's text
box, and SearXNG reads `!!`, `:fr` and `<99` as its own control language before anything is
searched for. The grammar that disarms a company name now disarms a description too — one
function, two callers, because two copies would leave the riskier one to be forgotten.

### A name a reader can choose between comes from the company, not the engine

A disambiguation chip is the one place a reader is asked to decide, and choosing between three
summaries an engine wrote is choosing between an engine's opinions. Each surviving host's front
page is fetched — at most five, stated — and the heading becomes the name, the first line of prose
the distinguisher.

A host whose front page cannot be read **keeps its host as its name and says so**, rather than
being dropped: a candidate silently missing from a list a reader is choosing from is the silent
truncation this project keeps deleting.

### And the harness caught a test passing by luck

`candidates_that_tie_come_back_in_the_same_order_every_time` used **two** tying hosts, and the
mutation that deletes the tie-break survived: with two entries a `HashMap` iterates in the sorted
order often enough that the assertion held anyway. It uses eight now, which turns agreement by
chance from a coin flip into a one-in-forty-thousand event, and asserts the whole order rather
than the first element.

The property matters because the gate compares the **first two** scores against
`AMBIGUITY_MARGIN`. Which two those are cannot be luck.

### Three things review found, and all three were about identity

The score was defensible and the **thing being scored** was wrong three times over — which is
worse, because entity resolution is the one step where a mistake is invisible downstream: a
report about the wrong company is internally consistent, fully cited, and wrong in every section.

**One company was three.** `registrable()` lowercased and stripped `www.` and stopped there, so
`example.com`, `app.example.com` and `docs.example.com` were three candidates. That divides the
cross-query agreement the whole score rests on, and spends three of the five slots a reader
chooses from on one vendor — turning a clear subject into an ambiguous one.

**And the first fix for it was worse in the direction that matters.** A hand-written list of
thirty multi-label suffixes covered `co.uk` and missed `github.io`:

```text
alpha.github.io  ->  github.io
beta.github.io   ->  github.io      one "company", agreed = 2
```

Two unrelated tenants, seen by two different queries, **manufacturing the corroboration** the
rule above was added to require — one round after it went in. A missing suffix does not merely
merge two companies; it forges the evidence that lets the merged thing auto-resolve.

That is why this is a dependency. **A thirty-entry sample cannot be completed by adding entries,
because the failure is not knowing what is missing** — which is exactly what a maintained list is
for. `psl` embeds the Public Suffix List **including its private section**, which is what knows
that `github.io` is a suffix and `github.com` is a company. It is offline, needs no data file at
runtime, and clears `cargo deny` on advisories, licences and sources.

**And the list will pretend an address is a domain.** `psl::domain_str("127.0.0.1")` is
`Some("0.1")` — and so is `psl::domain_str("10.0.0.1")`, so two unrelated addresses forged the
same agreement the private suffixes did. Addresses are recognised before the list is consulted
now, and never reach a reader's list at all: nobody publishes a company at `93.184.216.34`, and
offering one as a choice between five names is offering nonsense. Keeping them out at that step
rather than at the grouping step also means the home-page URL never has to decide whether an IPv6
literal needs brackets — the case cannot arrive.

**The doc comment for that fallback said an IP address was "returned unchanged".** True of IPv6,
false of IPv4, and the test I wrote for it covered `localhost` and the empty string — the branch
where the claim happened to hold. Three times in this pull request a test has covered a different
branch from the one it was named for, and this is the one where the claim was in a doc comment
rather than only in a test name.

I had written the argument against my own fix into its own doc comment: *"the worst outcome is
two candidates where there should be one"* and *"an unlisted suffix groups one label too short,
which merges"* are both in there, and only the second one was true.

**The page that named a company was not its front page.** `describe` fetched the shallowest
*search result*, which for a real company is often `/pricing` — whose first heading is `Pricing`.
A reader would have been asked to choose between three companies, one of them called *Pricing*.
The home page is built from the host now.

**And a single search could resolve a subject.** Three queries sent, two failing, and the one hit
from the third scored `0.8 × 1/3 + 0.2 = 0.47` — over `MINIMUM_CONFIDENCE`, so the gate resolved
it and an analysis would have run against a company that appeared in one search. Corroboration is
required: below two agreeing queries the score is **derived from the gate's own floor** rather
than chosen to look low, so the two cannot drift apart in silence.

**The test for that last one asserted `confidence < 0.5`** and passed the whole time, because
0.47 is less than 0.5 and also more than 0.35. A number instead of the behaviour, which is the
register's own rule about asserting on the surface — the regression runs
`suggest → describe → resolve` and asserts the *verdict* now.

### And the fix put a hole in the tests above it

Adding the corroboration floor left every existing score test comparing a **corroborated**
candidate against a **floored** one — so nothing distinguished two-of-three agreement from
three-of-three, and the divisor could have been anything at all. The harness said so on the next
run: a mutation that had been caught for two rounds went missed on a change that fixed something
else. A fix is a change, and the tests around it are part of what it changes.

### What is not measured here

**No engine has been asked.** Docker's daemon does not start where this was built, which is the
same limit `landscape search` carries and the same one `DEPLOY.md` carries — recorded rather than
worked around. Every number above is from a canned provider over the real code path; what a real
SearXNG returns for a real description is the first thing the next person through should print,
and `landscape candidates "<description>"` prints it.

**And no analysis calls this.** Typing a description into the box still fails with `no_subject`.
The join needs the gate's *ambiguous* branch to have somewhere to ask, which is the
clarifying-question row of S2 — one piece of work away, and named rather than implied.

| | Rust tests | frontend tests |
|---|---|---|
| Run 27 | 687 | 51 |
| now | **725** | **51** |

---

## Run 27 — the join, and the sharper gap it made available

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none. The join is arithmetic over
what a run already produced, and every number below is counted without one.

Run 26 built the search channel and said plainly what it was not: *"no analysis calls it."* This
is the call. It also closes the limit that run named rather than leaving it for somebody to
discover.

### A question is unanswered when nothing filled it, not when nothing was admitted

`landscape search <origin>` computes gaps from what **discovery admitted**. That is the only
measure available to a command that never reads anything, and it is the weaker one:

```text
helpscout.com/blog   admitted for `changes`   yields no dated entry
```

Under the old measure that question is answered — a page was found — and no search happens for a
section that will be blank. Under the new one it is a gap, because **no claim exists**, and it is
searched for. The measure moved because the join made the sharper one available: an analysis
knows what filled, and the CLI cannot.

That is one line of code and the whole reason this belongs in the orchestrator:

```rust
.filter(|question| !self.claims.iter().any(|(q, _)| q == question))
```

### What a search may cost, and what it may never do

| | |
|---|---|
| Queries | one per gap, templated and versioned. A company whose own pages answered everything sends none |
| Pages added | at most **3**, spent round-robin across questions so one gap cannot starve another |
| Model calls | whatever those pages cost, and they are *not* in `landscape cost`'s prediction — which now says so rather than quietly under-reporting a run |
| A page on the subject's own domain | `Primary`. The company said it, however we came to find the page |
| Any other page | `Unverified`, marked `(U)` **in the report body beside the claim**, with the reason on the source line |

**The marking is the part review would have found.** `FACT_CHECKING.md` §3.2.4 puts an
unverified source in the report body *"marked unverified, with why"*, and until this change every
source in every report was the company's own page — so `Disposition` had never once rendered as
anything but `P`, and a code beside a URL two screens below a claim was enough. It is not enough
the moment a stranger's page can produce a claim:

```text
- (U) states it offers unlimited seats [S4·M]

[S4] … — https://someblog.example/… (U — a page we could read but could not fully attribute)
```

### The refusals, and where each one lives

Three things had to be impossible, and each is one place rather than a rule to remember:

* **Search cannot lead.** The gaps are computed from claims that exist, so the ordering
  `FACT_CHECKING.md` §3.3 requires is structural rather than commented.
* **A run nobody is waiting for spends nothing more.** `worth_searching` refuses both a run the
  reader abandoned and a run with no gaps, in one function, for two stated reasons.
* **A page cannot be parted from its standing.** The URL and its disposition were two arguments
  to the reading function until the mutation harness labelled every search result `Primary` and
  the whole suite passed. They travel as one value now — entry 7 of the register, found by
  tooling rather than by review.

### And three things review found in the reporting

The reading was right and the *account of it* was wrong in three places, each the same shape:
a sentence written from what was intended rather than from what happened.

**A page admitted is not a page read.** The note was written straight after admission and said
every admitted hit *"was read"* — so a dead host, a page below the quality floor, and a run the
reader walked away from all counted as reads. It is computed after the loop now, from the pages
that reached an extractor, and what was admitted and never opened is counted separately:
*"1 further page(s) were found and not read."*

**Coverage is built from discovery, and search was the first thing ever to add a page discovery
had not.** A question whose only page came from search reported:

```text
nothing was checked - our gap, not theirs
```

— the report telling a reader we did not look, immediately after looking. Admitted search pages
now join the coverage for their question, so the four silences stay four.

**And a count is not evidence.** The first fix for the paragraph above extended
`Coverage.sources` and stopped there — but `sources` is a number, and the *"Checked:"* line a
reader actually sees is rendered from `attempts`, which discovery still owned alone. So the note
became `read 1 page(s), none stated anything. Checked: nothing`: a page opened, and nowhere for a
reader to go and look at it. Searched pages join the attempts now, carrying **the whole URL**
rather than a path, because a page on somebody else's host is not the subject's and the host is
the part that matters about it.

That surfaced one more, a branch over and older than this change: *"N page(s) found and not read"*
named a count and no pages. Every other silence in that function says what was checked. It does
too now.

**And a note that says "this company" cannot survive a merge.** `analyse_many` joins each
subject's notes and drops duplicates, so two companies with the same gaps collapsed into one
ambiguous sentence and the other was silently discarded. Every search note names its company.

### What did not change

**Nothing, without `SEARX_URL`.** The laptop default reads exactly the pages discovery planned,
and the report says which questions were left unsearched rather than letting them read as
questions somebody looked into. That distinction is the same one the coverage note has always
drawn, applied to a new kind of silence.

| | Rust tests | frontend tests |
|---|---|---|
| Run 26 | 669 | 51 |
| now | **687** | **51** |

---

## Run 26 — the first road off the subject's own domain

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none. Nothing here asks one, and
nothing here needs one — this is retrieval, and the model does not run until a page has been
read.

Every page this pipeline has ever read was on the subject's own website. `landscape-discover`
guesses paths, reads `sitemap.xml` and `llms.txt`, and stops at the edge of one domain. That is
the whole of [B2](../PROJECT_STATUS.md#4-blockers) and the first row of S2: **there is no path
from a description to a company, and no source that is not the company itself.**

`landscape-search` is the first piece of that. It is one of four, and the three it does not do
are named at the bottom.

### What it costs: nothing on four of the six demo companies

The rule is `FACT_CHECKING.md` §3.3's — *search fills gaps; it does not lead* — and it is
structural here rather than a comment: `queries::for_questions` takes the questions **discovery
came back empty on**, so a company whose probes answered everything asks nothing. Run against the
six curated domains with `landscape search`, no engine configured, so the only thing measured is
how many questions are left over:

| | questions discovery found a page for | queries a search would ask |
|---|---|---|
| linear.app | 6 of 6 | **0** |
| usefathom.com | 6 of 6 | **0** |
| front.com | 6 of 6 | **0** |
| helpscout.com | 6 of 6 | **0** |
| basecamp.com | 5 of 6 | **1** — *changes* |
| simpleanalytics.com | 5 of 6 | **1** — *direction* |

**Two queries across six companies.** That number is the argument for building the deterministic
half first, made after the fact: a design that searched first and probed second would have made
thirty-six round trips to learn what fifteen seconds of politeness already knew.

Basecamp is the same gap Run 11 found and named. Its coverage note reads *"no page found.
Checked: /changelog (404), /releases (404), /blog (404)"*, and the query that follows from it is
`"basecamp.com" changelog OR "release notes"` — the first time this codebase has asked anybody
outside a company about that company.

### The column heading is the finding

It says *"questions discovery found a page for"*, and the first draft of this table said
*"answered"*. They are not the same thing, and running the six is what showed it.

**Help Scout is the case.** It shows 6 of 6 here and asks nothing — because `/blog` is admitted
for *changes*. Run 25 recorded that Help Scout *"publishes no changelog this pipeline finds"*,
and both statements are true: a page was found, and the extractor got nothing dated out of it.
So a section that will come back empty produces **no search**, because search is currently
triggered by an unadmitted question rather than by an unfilled section.

That is the honest limit of this slice, and it is the right limit for it: `Coverage` already
distinguishes four silences — our gap, the company's gap, a page found and never opened, and a
page read that stated nothing — and only the first two are gaps a search can close. Wiring the
trigger to coverage instead of to admission needs the orchestrator, which is the next PR. **What
would be wrong is claiming the sharper number now**, so the column says what was actually
counted.

### What a result is allowed to become

A hit is three strings from a machine we did not write, about a page nobody has read. Three
things stop it being treated as more than that, and they fail differently, so they are three
separate checks rather than one.

**Its standing comes from its host, never from its rank.** The subject's own domain — including
subdomains, because `docs.linear.app` is Linear speaking — is `Primary`. Everything else is
`Unverified`. Since only a primary source may set a value in a comparison table, **search buys
pages to report beside the table and cannot buy a cell inside it.** A price on an aggregator
stays a price on an aggregator.

The dot in that rule is load-bearing, and the mutation that proves it is the sharpest one in the
set: `host.ends_with(subject)` without it makes `linear.app.attacker.test` the subject's own
domain, and hands the strongest standing this codebase grants to whoever registers the name.

**The engine's prose does not travel.** `Hit` carries a title and a snippet so a person running
the diagnostic can see what came back; `Found` has no field for either. This is Run 8's finding
with a shorter supply chain — a worked example in a prompt became a fact in a report — and the
same answer: cut the path rather than remember to avoid it. A snippet is not a quote; it is
assembled from the page, from an older copy of it, or from somebody else's description of it.

**A URL from a stranger is checked before it is held.** Every hit goes through
`landscape_fetch::Target::parse` at admission, so `file:///etc/passwd` and `javascript:` results
are refused before anything carries them. The SSRF guard would already refuse to *fetch* them;
this refuses to put them in a list a later change might loop over.

### Two things reused rather than written again

Discovery already answers *"are these two URLs one page"* and already spends a cap round-robin
across questions. Both are reused: hits become `Candidate`s with a new `Via::Search`, ordered
**below** every existing variant, and `rank::admit` does the rest. So a page reached by both
channels keeps the probe, `/changelog` and `/changelog/` are one page here for the same reason
they are one page there, and four hits about pricing cannot starve the one about changes.

That was entry 4 of the mistakes register applied deliberately — one fact derived two ways is
how the two implementations drift the first time either learns something.

### The guard that was deleted rather than tested

The first draft refused an empty subject explicitly. Mutating that guard away changed no test and
no behaviour: `host == ""` is false for every host `Target::parse` admits, and no host survives
`strip_www` ending in a dot. Per `docs/mutations/README.md`, **a guard nothing needs is deleted
rather than given a test that keeps it for ever** — so it was, and the mutation was inverted. It
now puts the *wrong* rule in (`subject.is_empty() ||` …, making every page on the web primary)
and a test fails on it. Entry 32 of the register is the precedent.

### What was run, and what was not

```bash
cargo run -p landscape -- search https://basecamp.com
cargo run -p landscape -- search https://simpleanalytics.com
python3 scripts/mutate.py docs/mutations/the-search-channel.json   # 16 of 16 caught
```

### Seven defects review found, and what each of them was

The first version of this passed all eleven gates, 658 tests and ten mutations. **Review found
seven things none of them could see**, which is the register's own thesis arriving on schedule.
Recorded here because six of the seven are classes rather than typos.

**A trust boundary reading its host from a query string.** `Target::parse` ended the authority
at the first `/`, so `https://evil.test?@linear.app` kept the query inside the authority and the
credential strip then read `linear.app` out of it. `disposition_for` would have called a page on
evil.test **Primary** — the one disposition permitted to set a value in a comparison table. The
inverse was wrong too: `https://linear.app?x=1` gave a host of `linear.app?x=1`, matching
nothing. Fixed in `landscape-fetch`, where every other caller benefits, and no test there had a
URL with a query and no path.

**Phrase quotes are not an escape.** Wrapping the subject in `"` and stripping its own quotes
was the wrong model of the thing being defended against: SearXNG splits `q` on whitespace and
reads its own control language off the tokens **before** searching. A name containing `!!`
leaves a bare `!!` token, which means *redirect to the first result* — and the client followed
redirects, so a hostile name could have this process fetch an arbitrary page before `admit` or
the SSRF guard saw a URL. `!google`, `:fr` and `<99` select an engine, a language and a timeout.
Replaced with a deliberate grammar — letters, digits and `-._&'+`, everything else becomes a
space — and the transport now refuses redirects as well, because one guard and no second is how
a bypass goes quiet.

**A value separated from its evidence, in the function that was written to keep them together.**
The same URL returned for two questions overwrote its `Found` while the ranked `Candidate` kept
the first question, so a page came out labelled with a question it was not found for, quoting a
query that did not find it. The test named for this used two different URLs and could never
collide. Entry 7 of the register.

**A cap that capped the wrong thing.** `HITS_PER_QUERY` truncates *after* the body is read and
`serde` has materialised every row, so the comment claiming it stopped a hostile engine deciding
this process's workload was the opposite of true. `MAX_RESPONSE_BYTES` now refuses while the
bytes arrive, and the test drives a server that will not stop sending.

**A configuration file that did not do what it said.** `use_default_settings: true` merges the
`engines:` list by name and leaves every other default engine enabled, so *"only the engines a
competitive analysis has a use for"* was three configured and the rest still running. Now
`use_default_settings.engines.keep_only`.

**A normaliser that lowercased paths.** Pre-existing in discovery and harmless while it had one
caller; making it the shared answer for two channels made it a page silently dropped as
*"discovery already has it"* when `/Docs` and `/docs` are different resources. Only the scheme
and host are case-insensitive.

**An error swallowed into a weaker object.** `.build().unwrap_or_default()` replaced a client
carrying the eight-second timeout with one carrying none. Now fallible.

**41 tests — 34 unit and seven over a socket.** `tests/against_a_server.rs` stands up a listener,
points a real client at it, and asserts on **what arrived** as well as what came back — the
percent-encoding of a template full of quotes, the endpoint the query is appended to, a 403
carrying its number, a dead port becoming `Unreachable` rather than a hang, a redirect that is
reported rather than chased to `169.254.169.254`, and a body that will not stop being abandoned
part-way rather than after. Every other test in the crate exercises one side of that join, and
the register's recurring finding is that the joins are where the defects live.

**No SearXNG has been run against this, and the compose service is written rather than walked.**
Docker is not available in the workspace this was built in. The service, its checked-in
`settings.yml` and the healthcheck that asks for `format=json` are in the repository; the first
person to run `docker compose --profile search up -d searxng` is the one who finds out where they
are wrong. That is D3's posture, for the same reason: an instruction nobody has followed is a
draft.

The one failure mode most likely to be met first is already named in the code: SearXNG answers
**403** to `format=json` until the instance opts in, so `SearchError::Status` carries the number
rather than flattening it into *"search failed"*.

### Still open — the other three quarters of B2

- **Candidate generation.** `landscape-core::subject` holds a disambiguation gate that has
  never been given a candidate. Turning hits into scored candidates is what feeds it.
- **Competitor set derivation.** One idea to several companies, with why each was chosen.
- **Vocabulary resolution.** A reader's words to a category the pipeline can search.

Until those exist, *"an app that helps small farms sell to local restaurants"* still fails with
`no_subject` and a remedy line. **This PR does not move that**, and the honest summary is that
the road now exists and nothing drives on it yet: search is wired to a command, not to the
orchestrator.

| | Rust tests | frontend tests |
|---|---|---|
| Run 25 | 624 | 51 |
| now | **669** | **51** |

---

## Run 25 — the sixth question, for nothing

**Date:** 2026-08-07 · **Where:** this laptop · **Model:** none — every number below is counted
without one, which is the point of this one twice over.

*Where they are investing* was the last of the six questions with no extractor. A careers page
was admitted by discovery, and then `has_extractor` returned false, the page was never fetched,
and the section carried a coverage note for ever. This reads it, and **asks no model anything**:
a job title is a line somebody wrote down on purpose, so `ARCHITECTURE.md` §5.4 puts it on the
same side of the line as a date.

### What it costs: nothing, on all six demo companies

`landscape cost` counts the calls a run would make, with no model running. Both columns are the
same command on the same six domains, before and after.

| | model calls, before | after | first chance of content |
|---|---|---|---|
| basecamp.com | 14 | 14 | 1 call, unchanged |
| linear.app | 16 | 16 | 0 calls, and now from two pages rather than one |
| usefathom.com | 17 | 17 | 0 calls |
| simpleanalytics.com | 17 | 17 | 0 calls |
| **helpscout.com** | **19** | **19** | **1 call → 0** |
| front.com | 17 | 17 | 0 calls |

**A whole section, at no cost in calls, on every one of them.** And Help Scout moved: it
publishes no changelog this pipeline finds, so its first content used to wait on the model being
up and answering. Its careers page is deterministic, so it now goes first and the wait for
something on screen is a fetch. **Five of the six demo companies now get their first content
with no model call**, up from four.

Basecamp is the honest exception. `basecamp.com/jobs` is admitted and yields no roles, so it is
read, costs nothing, and says so — which is a fact about Basecamp rather than a gap in the
report.

### The design decision, and the three pages that made it

The obvious version of this extractor sorts titles into functions and reports *engineering: 7,
sales: 3*. Three real careers pages, frozen into the golden set, say not to:

* Linear files **Product Marketing Manager** under *Product Management* and **Developer
  Relations** under *Marketing*. A keyword table puts both in the other bucket.
* Front's **AI Engineer - GTM / Operations** is filed under *G&A*.
* Linear's own group labels include **Magic**.

So the titles are reported as the page writes them and the reader does the sorting. On
linear.app that reads as *five titles carrying "Engineer", four carrying "Designer", and one
"Senior Counsel"* — an investment signal made of twenty lines, each of which can be found on the
page it came from.

### Everything dangerous was outside the list

Each of the three pages carries a short line, with a job word in it and no full stop, that is
not a vacancy:

```text
The Pragmatic Engineer                                 linear.app/careers - a podcast
Staff Java Engineer in Brno, Czechia, started in 2015  helpscout.com - an employee
Kelsey Weber , Engineering Manager                     front.com - a testimonial byline
```

All three sit *above* the page's own `## Open roles` heading, which is why the scan is scoped to
it and stops at the next heading of the same level — Help Scout's list is followed by *"Our
Hiring Process"*, which explains what a hiring manager does.

**Front's list is the last heading on its page**, so there the scan reaches the footer and only
the shape rules stand between a navigation label and a job. The first run over it reported
*"Become a Partner"* — a reseller programme — as an open role. `partner`, `associate` and `lead`
left the word list; no title on any of the three pages needs them, because a real title carrying
one carries another word too: *Lead/Principal Product Manager*.

### And a page that never says where its list is, is not read at all

Review found what the unscoped path published. When no recognised heading is present the scan
used to fall back to the whole page and let the shape rules decide — and every shape rule says
yes to `Kelsey Weber , Engineering Manager`: five words, no full stop, a job word on a word
boundary. Off front.com's *Open positions* heading that line is scoped away. Off a page with no
such heading it reached the report as **`lists an open role: Kelsey Weber , Engineering
Manager`**, at high confidence, about a person who already works there.

**The run log said the page was unscoped, and nothing carried that into the report.** That is the
same mistake as Run 24's: a line nobody reads standing in where a check belongs.

So the fallback is a refusal. The shape rules clean up *inside* a list somebody has pointed at;
they were never strong enough to find one, and using them as the only defence was reading a
diagnostic as a safeguard. **The cost is stated rather than hidden**: a careers page whose
heading this list does not recognise now yields nothing, and real vacancies on it are lost. A
missed vacancy is a thin section; an invented one is a false sentence about a named person.

The two silences no longer read alike, which is what `Coverage` exists for:

```text
no open roles listed on the page                                    news about the company
the page does not say where its open roles are listed, so none were read     our gap
```

### And the harness deleted a rule for free

Twenty-five mutations, and one of them could not be caught: putting back a defect in the
plural-matching rule broke no test. Looking for the test to write showed there was none to
write. **No title on any of the three pages is plural** — a job advertisement names one job —
and every line the rule did reach was a navigation label: `Developers`, `Partners`, `Managers`.
It widened the false-positive surface and bought nothing, so it is gone, and the mutation now
puts it *back* and a test fails.

That is the second time this harness has removed code rather than adding a test for it, and it
is the more useful of the two outcomes.

### The string that said "no extractor yet" is gone too

With all six covered, `has_extractor` would have returned `true` for every question — a
predicate no caller can make false, which is the register's commonest defect shape. It is
deleted, and so is the branch behind it that fetched nothing and wrote a line to a run log.
`stages::extract` now matches all six questions **with no wildcard arm**, so a seventh question
is a build error rather than a string nobody reads.

| | Rust tests | frontend tests |
|---|---|---|
| Run 24 | 586 | 51 |
| Run 25 | 624 | 51 |

---

## Run 24 — the fifth question, and the page it was reading instead

**Date:** 2026-08-06 · **Where:** this laptop · **Model:** none — every number below is counted
without one.

Two of the six questions had no extractor. A page admitted for *Trust & security posture* was
discovered, fetched, and then skipped with `no extractor yet for trust pages` — the fetch spent,
the section permanently empty. This closes the first of the two.

### The vocabulary is closed, which changes the design

A capability can be called anything and a price can be any number, but **a company does not
invent a compliance standard.** It names one of a few dozen that already exist, and spells them
the way the auditors do.

So the scanner finds the mention deterministically, from a named list, and the model is asked
one small question about each: *do they say they have it, or that they are working towards it?*

```text
SOC 2 Type II                          found by the scanner, no model
…report available under NDA            read by the model: holds, or pursuing?
```

**That split is the whole argument.** A model asked to *"list the certifications on this page"*
will return SOC 2 for a page that never mentions it, and grounding would be the only thing
between that and a published claim that a real company holds a certification it does not. Here
the name comes from the page by construction — nothing can be reported that the scanner did not
first find written down — and the model only judges a sentence it has in front of it.

### What it costs

Model calls per company, counted by `landscape cost` with no model running:

| Company | Before | After | Standards named |
|---|---:|---:|---|
| basecamp.com | 14 | **14** | none |
| linear.app | 12 | **17** | SOC 2 Type II, GDPR, HIPAA, ISO 27001 |
| usefathom.com | 15 | **17** | 2 |
| simpleanalytics.com | 15 | **18** | 3 |
| helpscout.com | 18 | **19** | 1 |
| front.com | 14 | **18** | 4 |
| **total** | **88** | **103** | |

**+17%, and the trade is stated rather than buried.** A section that could never fill now fills
for five of the six, and the sixth is the honest case: basecamp.com's security page names no
standard from the list, so the extractor reports nothing and the coverage note says the page was
read and stated nothing. That is a fact about Basecamp, not a gap in the reader's report.

A page of reassurance with nothing named costs **zero** calls — the scanner runs first, so the
model is never asked about a page that has no answer on it.

### The measurement found something the tests could not

`basecamp.com` was reading `/status` and skipping `/security`.

Both are admitted for the trust question, only the first is read
([Run 23](#run-23--the-wait-and-the-two-decisions-that-are-ours)'s one-page-per-question budget),
and discovery ranked them equally. So the extractor would have shipped unable to fire on a
company that publishes a security page — **the feature working in tests and doing nothing in the
product**, which is precisely what Phase D exists to stop happening again.

A status page is not a broad page; it is a page about a *different question*. It reports whether
the service is up now. `specificity` ranks it after `/security` now, and `usefathom.com` went
from 0 standards to 2 on that change alone.

### Freezing a real security page paid for itself immediately

`linear-security.md` is the first frozen trust page, and the first regeneration showed the
extractor reporting **both `SOC 2` and `SOC 2 Type II`** — a banner and the section that explains
it, the same certification told to a reader twice, at two precisions, for two model calls.

The more precise spelling now wins wherever it appears. Four standards, one each.

| Reintroduced defect | Caught? |
|---|---|
| the model *names* a different standard in the same window | **unrepresentable** |
| the model *answers about* a different standard in the same window | **yes** |
| evidence naming no standard is refused along with the rest | **yes** |
| an unsupported claim is published instead of dropped | **yes** |
| a status with no quote at all is published as a claim | **yes** |
| `SOC 2 Type 2` and `SOC 2 Type II` are treated as different standards | **yes** |
| a precision difference is treated as a different standard | **yes** |
| a roadmap item is reported the same way as a certification | **yes** |
| the report asserts the certification rather than the claim | **yes** |
| a standard mentioned without a claim is reported as one | **yes** |
| a standard named without a claim is dropped instead of reported | **yes** |
| the same standard at two precisions is reported twice | **yes** |
| a shorter spelling wins over the precise one | **yes** |
| a name that runs into another word is read as that name | **yes** |
| the window is the line alone, without the sentence that carries the claim | **yes** |
| the cap is applied and the number the page offered is lost | **yes** |
| a status page outranks the security page for the trust question | **yes** |
| a trust page is fetched and skipped as if it had no extractor | **yes** |
| a trust page costs nothing in the prediction the cost command prints | **yes** |

Twenty-two, all caught — including the ones that guard the wiring, the pairing of a claim to its
standard, and the deletion of a claim whose quote is not on the page. Every one of those was
added after review broke it and watched every test pass.

### What review found, and why two of those rows now say *unrepresentable*

**The model was still being asked for the standard's name.** The check was containment — does
the answer appear somewhere in this three-line window — and a window that mentions two standards
defeats it completely: a response naming `ISO 27001`, claiming `holds`, and quoting a verbatim
sentence about SOC 2 passed both independent checks and would have published a certification
claim about the wrong standard. Returning `SOC 2` for a scanned `SOC 2 Type II` also passed, and
quietly undid the precise-spelling rule.

The model is no longer asked for the name. `AssuranceClaim` carries a status and a quote and
**has no name field**; the scanner's standard is attached afterwards.

**And that was not enough, which review found in the regression written to prove it was.** Taking
the name away stopped the model *labelling* an answer; it did not stop it *answering about the
wrong thing*. A window is three lines and two standards often sit within three lines of each
other — or on one line — so the same window is handed over twice, once per standard. A model
asked about ISO 27001 can reply `holds` while quoting the sentence about SOC 2, and every check
passed: the quote is verbatim in the section, and the name is the scanner's. The report then said
*"states ISO 27001"* on evidence about a different certification.

So the evidence is checked against the standard it is supposed to be about: a quote naming a
recognised standard that is **not** this one keeps the mention and drops the claim, and the run
log says how many. A quote naming no standard at all — *"We are certified and audited
annually."* — is the ordinary honest case and is kept.

### And a claim whose quote is not on the page is deleted, which is already the rule

`ARCHITECTURE.md` says it plainly: *"a claim whose evidence quote is absent from its cited source
is deleted"*. This counted such a quote and **published the status anyway**, on the argument that
the standard was real so the finding was worth keeping. What that ships is *"states ISO 27001"*
against a page whose only sentence is *"Questions about ISO 27001? Contact us."* — an unsupported
compliance claim, with a line in a run log nobody reads standing in for a check. Review found it,
and my test had locked the behaviour in.

An unsupported claim now takes the same path as one whose evidence is about a neighbour: the
mention survives, the claim does not. **A status with no quote at all takes it too** — "there is
no quote to check" had been reading as "the quote is fine".

**And two spellings of one report were two standards.** `SOC 2 Type 2` and `SOC 2 Type II` are
what an auditor writes either way; prefix comparison called them different, which counted one
certification twice on a page using both and let the evidence check discard a correct quote for
choosing the other form. Compared canonically now — Roman numerals folded to digits — while the
page's own spelling is still what a report shows.

**And two boundary defects the suite could not see.** `to_lowercase` is Unicode-aware and can
change a string's length, so an offset found in the lowered copy pointed past the end of the
original and slicing it panicked — a line beginning with `İ` was a crash. Every standard is
ASCII, so the fold is ASCII now and offsets are preserved by construction.

The other: `considered` promises the number of distinct standards a page named, and the
have-I-seen-this comparison was made against the **capped** list — so past eight, a repeat of the
ninth counted again, and nine standards plus one repeat reported ten. The record of what was seen
is uncapped now; only the windows that will be read are capped.

### And the row below this paragraph was wrong

It said 583 where the suite ran 586, and I had reached it by **adding two to the number I
remembered** rather than by reading the line `verify.py` prints. Review caught it — the last
finding on a change whose every other number was measured.

So `verify.py` gained a gate that reads this row back against the counts the gates above just
produced, and fails when they disagree. The table stays, because a benchmark file is a record of
history and cannot be deduplicated away like the second copies elsewhere in this run — it is
checked instead, by the run that produced it.

| | Rust tests | frontend tests |
|---|---|---|
| Run 23 | 509 | 51 |
| now | **586** | **51** |

---

## Run 23 — the wait, and the two decisions that are ours

**Date:** 2026-08-06 · **Where:** this laptop · **Model:** none — every number below is counted
or measured without one, which is the point.

`PRODUCT_SPEC.md` §2.1A asks for **content in twenty to forty seconds** and a whole report
inside ninety to a hundred and eighty. Neither was met, and the last row of S1 existed to fix
it. The interesting part is that **the model's speed is not the lever.** What a run costs is how
many times the pipeline asks — every extractor works a window at a time and makes one call per
window — and *when* it asks decides how long somebody stares at nothing.

Both are decisions this repository makes, so both can be measured with no model running.

```bash
cargo run -p landscape -- cost https://linear.app
```

### The page that needs no model goes first

A changelog is parsed, not generated. Discovery hands its pages back pricing-first, so the first
thing a reader waited for was a chain of model calls; the deterministic page now goes in front.

**Measured, with no model at all:**

```text
first content 23s - whole report 26s
```

That is `linear.app`, and the content is seven real dated changes. **23 seconds, inside §2.1A's
window, and it does not depend on the model** — which is the property worth having, because the
model is the part that will be slowest on four ARM cores.

Of those 23 seconds, about 20 are discovery. **That is now the first-content bottleneck**, and
it is the next lever rather than this one.

### One page a question, and the rest are named

Discovery admits eight pages round-robin across six questions, so a company gets a first page
for each and a *second* page for some. The second is where the wait doubles — another dozen
model calls for a question that already has an answer.

Each question is now worth one page that needs a model, and **the pages that are not read are
named on the report**, the same rule the capability cap and the subject cap already follow.

### What that costs the six companies the demo offers

Model calls, counted by running the real span-finders over the real pages:

| Company | Before | After | Before a first chance | After |
|---|---:|---:|---:|---:|
| basecamp.com | 14 | **14** | 1 | **1** |
| linear.app | 24 | **12** | 1 | **0** |
| usefathom.com | 15 | **15** | 1 | **0** |
| simpleanalytics.com | 24 | **15** | 1 | **0** |
| helpscout.com | 25 | **18** | 1 | **1** |
| front.com | 26 | **14** | 1 | **0** |
| **total** | **128** | **88** | | |

**31% fewer model calls, and it is not uniform — which is the honest part.**

- **basecamp.com changes by nothing at all.** It publishes no changelog and discovery found no
  second page for any question, so there was nothing to reorder and nothing to skip. A company
  this does nothing for must run exactly as it did, and it does.
- **usefathom.com keeps all fifteen calls** and still gets its first content free: no second
  pages, but a changelog that parses.
- **helpscout.com still spends a call before content.** Its `changes` page is `/blog`, and the
  blog yields no dated entries the parser accepts, so the first chance falls back to pricing.
  Deterministic-first pays only when the deterministic page actually answers.

**The right-hand columns are one call, not several, and review is why.** The first version of
this table counted *every* window on the first page that could show something — twelve calls for
a twelve-window page. Extractors report progress after **each** window, so a reader's first
chance comes after one call rather than after the page. The old numbers made the pipeline look
worse than it was and made an approximation look precise; both are corrected above, and the
column is now named *a first chance* because that is what it is. Whether a call answers, and
whether the answer survives grounding, is unknowable without running it.

So the first-content win here is **one model call and a dependency**, not a large number of
calls. The dependency is the part that matters, and the next section is about it.

Per example — two companies each — that is 26, 30 and 32 calls, against 38, 39 and 51.

### The dependency this removes, which review found still in place

`analyse_with` used to ask `llm.is_ready()` **before it built the plan and before it fetched
anything.** That health request runs on the same client as everything else, and that client
waits 180 seconds — the whole report budget — because prefill on four ARM cores is slow.

So a model endpoint that accepted the connection and then stalled would have spent the entire
wait *before the changelog was opened*. **The one guarantee this change exists to make would have
failed in exactly the situation it is for**: the model being unhealthy.

Health is now asked for at most once, and not until a page needs it — which, because the
deterministic page is read first, means not until after the first content has already gone out.
The regression stands up a listener that accepts connections and never answers, and asserts a
run that needs no model finishes in well under the timeout. It takes 0.05 seconds; before the
fix it took 180.

### The number this cannot give you

**The seconds.** One call's cost belongs to the model and the machine, not to this pipeline, and
[ADR 0011](decisions/0011-no-experiments-on-production.md) puts the end-to-end figure on the
client's side of a deployment rather than anywhere here. `landscape cost` counts the calls;
`landscape read` now prints `first content Ns - whole report Ns` for a run you take yourself.
Multiplying is deliberately left to whoever has the model.

### The honesty this could most easily have cost

A shorter wait bought by reading less is a report that quietly says less. Two things stop that:

The skipped pages are **named on the report**, above the sections, where they change what the
sections mean. And `Coverage::note` used to say *"3 fact(s) from 2 source(s)"* counting pages
**admitted** — so a run that read one of two pricing pages would have credited its facts to a
page nobody opened. It counts pages *read* now. That was already wrong whenever a fetch failed;
this change would have made it the ordinary case.

| Reintroduced defect | Caught? |
|---|---|
| the page that needs no model is read in discovery's order like everything else | **yes** |
| a changelog is treated as needing a model, so it is capped and read late | **yes** |
| every admitted page is read, so the wait is whatever discovery found | **yes** |
| the budget drops a question's only page | **yes** |
| pages are left unread and the report does not say so | **yes** |
| the merged report drops each company's note about what it did not read | **yes** |
| facts are credited to pages that were admitted rather than read | **yes** |
| the model's health is asked for before anything is read | **yes** |
| a page the run would skip for its quality is counted as a model call | **yes** |
| a whole first page is counted as the wait before content, not one call | **yes** |

**And `cost` was counting a page the run would not open.** `analyse_with` skips a page below the
quality floor before any extractor sees it; the counter handed the same markdown straight to the
span finders, so a two-line page with a plan name and a price reported one call where the real
run makes none. The quality gate is inside the shared counting function now — a prediction of the
run has to include everything the run decides, or it is a prediction of a different program.

| | Rust tests | frontend tests |
|---|---|---|
| Run 22 | 498 | 51 |
| now | **514** | **51** |

---

## Run 22 — the six companies the demo offers, checked against the live web

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — this is discovery alone.

`ROADMAP.md` §2·D item D4 asks for example ideas that *really run*. The word doing the work is
"really": a list of plausible-looking domains in a source file is a promise about somebody
else's website, and it is the one promise in this repository that can be broken by people who
have never heard of it. A site drops its sitemap, moves pricing behind an anchor, or starts
refusing our user agent, and the first person to find out is whoever was sent the link.

So the list is checkable, by a command with no model and no database behind it:

```bash
cargo run -p landscape -- examples
```

### What it found

| Idea | Company | Sources | Discovery | Questions answered |
|---|---|---|---:|---:|---|
| project management | basecamp.com | 6 | 16s | pricing, features, identity, trust, direction |
| | linear.app | 8 | 20s | **all six** |
| website analytics | usefathom.com | 8 | 18s | **all six** |
| | simpleanalytics.com | 8 | 32s | pricing, features, changes, identity, trust |
| shared inbox | helpscout.com | 8 | 35s | **all six** |
| | front.com | 8 | 25s | **all six** |

Six for six on pricing, which is the one the command treats as fatal. **A missing changelog is
a coverage note and a coverage note is the product working** — *"no page found. Checked:
/changelog (404), /releases (404)"* is a finding. A missing pricing page is a demo that opens
on a page of coverage notes, so `landscape examples` exits non-zero for that and only that.

Basecamp has no changelog to find. That is true of Basecamp rather than a defect here, and it
is worth keeping in the demo for exactly that reason: one of the six ideas shows the honest
negative beside real prices.

### The wait these imply, and why two companies rather than three

Discovery is 16–35 seconds a company. The model is the rest: about **two minutes a company** on
this laptop, from Run 21, which no part of this run changes.

| Companies in an example | Rough wait, here |
|---|---|
| 2 | **~4 min** |
| 3 | ~6 min |

`MAX_SUBJECTS` allows three. Every example ships **two**, and the difference between those two
numbers is the whole argument: three is what the analyser will do, two is what somebody who
clicked a link will sit through. `PRODUCT_SPEC.md` §2.1A asks for ninety to a hundred and
eighty seconds and neither number is inside it — which is the standing item Phase D's risk
section names, and no part of this feature pretends otherwise.

### What was curated, and what a test can hold

Only the choice of companies. Every claim is fetched, quoted and cited when somebody clicks;
nothing in the catalogue is an answer, a cached report, or text that reaches a claim.

That distinction is enforced rather than promised: `Example::prompt` puts the companies **into
the sentence the reader sees and can edit**, and the test that matters runs each prompt through
`origins_in` — the parser the worker really calls — and asserts the companies come back out.

| Reintroduced defect | Caught? |
|---|---|
| a chip expands to companies the reader never saw | **yes** |
| the words joining an idea to its companies glue two domains into one | **yes** |
| an example names a fourth company and is silently capped when it runs | **yes** |
| an example names one company, so the demo shows a profile | **yes** |
| the list is served without the sentence that qualifies it | **yes** |
| the interface says nothing about what is curated | **yes** |
| clicking an idea starts four minutes of work nobody asked for | **yes** |
| a malformed catalogue takes the first screen down with it | **yes** |
| the ideas stay under the report and invite a click that throws it away | **yes** |
| the entries inside a well-formed list are trusted rather than checked | **yes** |
| an entry's companies are taken on trust once the other fields look right | **yes** |
| the ideas render without the sentence that qualifies them | **yes** |

Twelve mutations, all caught — **three of them only after the mutation itself was corrected.**
One appended text to an idea that `origins_in` steps over as its own word, so it broke nothing;
one removed one of two guards while the second still carried the case; and one stopped being a
defect at all when review's fix made its target redundant, so it was re-aimed at a rule that was
real and unenforced. All three reported `MISSED` about code that was fine, which is what the
register says to check first and is why the harness is a committed file rather than something
typed once.

### What review found: a guard that validates the box, not what is in it

`getExamples` checked that `examples` was an array and `note` a string, then cast the whole
payload. **One entry with `companies: null` passes that untouched** and throws in
`companies.join(" vs ")` — while the first screen is drawing itself, on the one path designed to
degrade in silence. The fixture missed it by being malformed at the top level, which is the
shape the guard already handled. Register entry 26.

| | Rust tests | frontend tests |
|---|---|---|
| Run 21 | 485 | 41 |
| now | **498** | **51** |

---

## Run 21 — a comparison, not a profile

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — this is the join.

`PROJECT_STATUS.md` has said since it was written that the product is *"a single-company public
profile, not a competitive analysis"*. The reason was one line: `origin_in` returned the **first**
site named in a prompt and the rest were dropped in silence.

```text
"compare basecamp.com vs linear.app"   ->   a report about Basecamp
```

Nothing on the page said the second company had been ignored, which is the shape of wrong answer
that is hardest to notice: it looks exactly like a right answer to a different question.

### Every site named is a subject

`origins_in` returns them all, in the order written, without repeats — `basecamp.com` and
`www.basecamp.com` are one company — **capped at three**.

**The cap is about the wait, and it is the number to argue with.** Each subject is its own
discovery, fetches and model calls, so a report on three takes roughly three times as long as
one. At about two minutes a company on this laptop, three is already at the edge of
`PRODUCT_SPEC.md` §2.1A's ninety-to-a-hundred-and-eighty seconds, and four is past it.

| Subjects | Rough wait, here |
|---|---|
| 1 | ~2 min |
| 3 | **~6 min** |

**That is the honest problem with this feature**, and no part of it is solved by the join. It is
what the next item — read order, and how many pages each subject is worth — exists to fix.

### One section, every company

Sections merge by question, so *What it costs* holds every company's prices. That is what makes
the output a comparison rather than several reports stapled together, and it is the shape the
feature matrix will eventually be built from.

The pipeline for one company is untouched. `analyse_many` runs the existing, well-tested path
once per subject and joins the results — a bug in the joining cannot reach into the reading.

### The defect the join could most easily have introduced

Each run numbers its own sources from `S1`. Merging without renaming gives two companies the same
label, and **a reader following the citation for one company's price arrives at another
company's pricing page.** Evidence attached to the wrong claim is the worst output this product
can produce, and it would have looked perfectly well-formed.

Labels are reassigned as they merge. Three mutations, all caught:

| Reintroduced defect | Caught? |
|---|---|
| labels are not reassigned when merging | **yes** |
| sections are appended rather than merged by question | **yes**, 2 tests |
| the company still being read is left out of the report | **yes** |

| | Rust tests | frontend tests |
|---|---|---|
| Run 20 | 460 | 39 |
| now | **485** | **41** |

### What review found: a comparison you cannot read

Three things, and the first is the one that mattered.

**No claim said whose it was.** The extractors produce what the page says — `Pro costs $15` —
and never name the company, which is right: putting the subject into the sentence would be the
renderer's job leaking into the data. But once one section holds several companies, a reader saw
two prices and a bare `[S1]`, and the web UI renders neither `sources` nor a URL. The comparison
was unreadable.

**And the join tests hid it**, because their fixtures invented `A costs $10` and `B costs $20` —
claim text that names its company, which no extractor produces. A fixture more convenient than
reality is a fixture that tests the convenience.

`Claim` carries a `subject` now, stamped where the reports are merged rather than where the
claims are built — the merge is the function with tests around it. Shown beside each claim only
when a report covers more than one company, because repeating one name down a single-company
report is the noise that teaches people to stop reading labels.

**A fourth company was dropped as silently as the second used to be.** `origins_in` capped at
three and stopped scanning, which is the same defect at a higher count. The parser returns
everything now and the *caller* applies the cap, so it can say what it left out — a note on the
report, above the sections, where it changes what they mean.

**Coverage was concatenated, not merged.** `Analysis::render` zips sections with coverage, and
the merged report has one section per question however many subjects it covers — so the renderer
consumed the first company's six records and silently ignored every other company's. A section
that found nothing would have described only the first site's attempts, which is the
honest-negative treatment failing exactly where it matters most.

| Reintroduced defect | Caught? |
|---|---|
| the merge does not stamp the subject | **yes** |
| the interface does not name the company | **yes** |
| coverage is concatenated rather than merged | **yes** |
| a dropped company is not mentioned | **yes** |

The last two are asserted against `analyse_many` itself rather than its helpers, by pointing it
at origins in `.invalid` — which by RFC 2606 can never resolve, so every fetch fails fast and
what is left is the joining. It costs about six seconds of the suite, and it is the only way
those two call sites can be reached without a model.

### What is still not right

**Nothing discovers a competitor.** This analyses the companies a reader names. Turning an idea
into a set is `landscape-search`, and it is still the largest unbuilt thing in the project.

**Three companies is six minutes**, per the table above. The wait is now the binding constraint
on this feature rather than a background worry.

**The report is labelled, not grouped.** Each claim names its company, which is enough to read;
a matrix that puts them side by side is `landscape-charts`, in S3.

---

## Run 20 — stopping work nobody wants

**Date:** 2026-08-04 · **Where:** this laptop · **Model:** a stub, on purpose — see below.

Run 19 gave a claim a number, so a worker the staleness sweep has replaced is refused at every
write. It said plainly what it did not do:

> A replaced worker keeps working. It finds out at its next write — within a page for progress,
> at the very end for `complete` — and until then it is spending prefill on a report that will
> be discarded.

**Prefill is the only scarce resource this machine has.** A run is 90–180 seconds of it, and a
replaced worker was spending all of the remainder on a report nothing would accept.

### The stop

The progress callback returns an answer now:

```rust
pub enum Wanted { Yes, No }
```

`Wanted::No` breaks the loop — the page loop in `analyse_with`, and the window loop in each of
the three model-backed stages. **The callback is the right place for it because the answer is
already there**: a worker learns its claim is gone *by writing*, and the progress callback is
the write.

The answer `record` gives is as of the last write that **landed**, so it lags by a window. That
is soon enough: what it saves is everything the run had not started yet.

### Counting what it saves

A model call is the expensive thing, so a model call is the unit. The stub `llama-server`
answers instantly and counts requests — and for this question a stub is a *better* instrument
than a real model, not a worse one: the question is not what the model says, it is **how many
times it is asked**. It also runs on every pull request, which `against_a_model` cannot.

Against the real Basecamp features page — twelve capability windows, the page that made a reader
wait four minutes in Run 16:

| | model calls |
|---|---|
| told to stop after the first answer | **1** |
| nobody stopping it | **12** |

And the Notion pricing page, five plan windows: **1** against **5**.

### What review found: the stop only worked when the model worked

`so_far` was called from the branch where a window **succeeded**, and from nowhere else. So a
run whose model calls were returning unparseable output, or whose answers were being dropped as
ungrounded, never asked whether it was still wanted — and read all twelve windows anyway.

**That is the worst place to miss it.** A run producing errors or ungrounded answers is a slow
run, and a slow run is exactly the one the twenty-minute sweep takes away. The saving was absent
precisely where it was largest.

Reproduced with the same twelve-window fixture and a stub returning `not json at all`: the
callback was invoked **zero** times and the stub received **twelve** requests.

The question is asked after every window now, whatever became of it — and the features loop's
`continue` on an ungrounded name, which stepped straight past it, is an `else` instead.

| The path | before | after |
|---|---|---|
| the model answered and it grounded | 1 call | 1 call |
| the model answered and it did not ground | 12 | **1** |
| the model returned nothing usable | 12 | **1** |

### Does it hold up?

Five ways of getting it wrong, put back one at a time:

| Reintroduced defect | Caught? |
|---|---|
| the features loop ignores the answer | **yes** |
| the pricing loop ignores the answer | **yes** |
| the features check moves back inside the success branch | **yes**, 2 tests |
| the identity loop stops asking | **yes** |
| `record` never reports the claim gone | **yes** |
| the writer never sets the flag | **yes** |
| *every* run stops after one window | **yes** — the control |

That last row is the one that matters most. A stop that fires always looks like a saving in
every number above, and would quietly turn every report into its first window.

| | tests |
|---|---|
| Run 19 | 456 |
| now | **463** |

### Why the stub, and not the pipeline

`analyse_with` needs a `Fetcher`, whose SSRF guard refuses loopback **on purpose** — so a local
page server is unreachable by design, and the whole-pipeline version of this test cannot be
written without a seam that does not exist. `stages::extract` takes the Markdown directly, which
is where every model call is made, so that is where the counting happens.

### What is still not right

**The page-loop break has no test.** Four of the five mutations above are caught; the fifth
thing — `analyse_with` breaking *between pages* rather than between windows — is one line, and
nothing exercises it, for the reason in the paragraph above. It is subsumed in practice (a
revoked claim stops the window loop first) but that is an argument, not a test.

**Nothing measures how often this fires.** The saving is real and unquantified: twenty minutes
against a 90–180 s analysis makes a revoked claim rare, and there is no counter that would tell
us if that changed.

---

## Run 19 — telling two workers apart

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — this is the queue, the store
and the stream.

Run 18 left two things open and said so. Both were the same hole seen from different ends, and
this closes it: **there was nothing that identified a claim.**

The sweep returns a row that has been `running` for twenty minutes to the queue, because a row
that has been running that long is probably a dead worker. It cannot tell a *dead* worker from a
*slow* one — nothing can — so when the slow case happens, two live workers are running the same
analysis and `status` says `running` for both. Whichever finished last won, and a reader got
whichever report that was, with nothing anywhere recording that two had been produced.

The reader's side is the same gap: Runs 17 and 18 chased a dead worker's sections off the screen
twice, and each fix was a server-side judgement about **the connection** — *the report went
away*, *this connection has already sent something*. A reader's sections survive a reconnect on
purpose, and a fresh connection remembers nothing, so both were correct until the reader
reconnected and wrong immediately after.

### A claim is a number

Every row carries a `generation`: how many times the run has been started. `claim_next` raises
it, `reclaim_stale` raises it, and a worker quotes the number it was given on every write.

```text
save_progress(id, generation, report) -> Applied::Yes | Applied::ClaimRevoked
```

`ClaimRevoked` is an outcome and not an error, because the sweep doing its job is not a failure.
The worker is told once, at the end, rather than per write.

[ADR 0012](decisions/0012-a-claim-is-a-number.md) has the reasoning. The short version is that
**a state describes the row and a number identifies the attempt**: "running" is true no matter
which of two workers is running it, so no test on `status` can ever separate them.

### And the number goes to the reader

The stream announces it, and the **client** compares against the one it is holding — which is
the part the previous two attempts could not do, because only the client knows what the client
is showing.

It also fixes something subtler that fell out while testing. `running` → `queued` → `running` is
an *edge*, and the stream polls twice a second: in one of these tests the sweep and the
replacement claim both landed between two polls, and **the status never said `queued` at all.**

| | survives a reconnect | survives a poll gap |
|---|---|---|
| watching for `running` → `queued` | no | **no** |
| the report going to `NULL` | no | yes |
| a generation the client compares | **yes** | **yes** |

A design that watched for the transition would have missed exactly the restarts that happened
quickly.

### Does any of it hold up?

Eight ways of getting it wrong, put back into the code one at a time:

| Reintroduced defect | Caught? |
|---|---|
| claiming does not raise the generation | **yes** |
| the sweep does not raise the generation | **yes**, 2 tests |
| `save_progress` ignores the generation | **yes** |
| `complete` ignores the generation | **yes** |
| the stream never says which run it is watching | **yes**, 3 tests |
| a new generation does not clear the sent-payload memory | **yes** |
| the client ignores a change of generation | **yes**, 3 frontend tests |
| the recovery fetch does not check the generation | **yes**, 1 frontend test |

| | Rust tests | frontend tests |
|---|---|---|
| Run 18 | 454 | 27 |
| now | **456** | **27** |

The frontend count is unchanged because the three tests that covered this were rewritten rather
than added to: they used to send a `reset` event and now send a generation, which is the same
scenario asserted against a better signal.

### What is still not right

**A replaced worker keeps working.** It finds out at its next write — within a page for progress,
at the very end for `complete` — and until then it is spending prefill on a report that will be
discarded. Stopping it needs cancellation threaded into `analyse_with`, which changes the
pipeline's shape rather than the queue's and is worth doing on its own.

**Postgres was not exercised here.** Docker would not start on this machine, so the `generation`
column, the four `WHERE generation = $n` predicates and the two-query failure path are verified
by CI's `rust (against postgres)` job rather than by me. The in-memory store and the Postgres
store share one conformance body, which is what makes that a reasonable division of labour, but
it is not the same as having run it.

**The replacement wins because it holds the current number, not because its report is better.**
That is the right default — the sweep exists precisely because the older claim looked dead — but
it is a policy, and nothing measures how often it fires.

---

## Run 18 — the states a run only reaches when something goes wrong

**Date:** 2026-08-05 · **Where:** this laptop · **Model:** none — every defect here is in the
worker, the store and the stream.

Run 17 froze what a *page* yields. This is the other gap it left, and the one the
coding-mistakes register argues is larger: **nine of that register's eleven entries are states
that only exist when something goes wrong mid-operation**, and three of them were introduced by
the fix for the previous one. A suite built from complete runs cannot see any of them, because a
complete run never enters them.

So: drive the orchestrator through a worker that dies, a run that is reclaimed, and a store that
is slower than the pipeline feeding it. **Two defects, both invisible to 442 passing tests.**

### A slow write overwrote a newer report

The worker spawned a task per progress snapshot, with this reasoning beside it:

> Ordering is not a concern because each write is the whole report so far, not a delta.

**That is true of the payloads and false of the writes.** Two spawned tasks are two concurrent
round-trips over two pooled connections, and nothing makes the first land first. Given a store
whose first write is slow:

```text
writes landed ["second", "first"]
  left: "first"
 right: "second"
```

The store keeps the **older** report. So a section that had four claims goes back to three, and
`PagePricing::assembled` replacing *"Free is listed with no published price"* with *"Free costs
$0"* is undone in front of a reader who already saw the correction. It repairs itself at
`complete` — which means the window where it is visible is exactly the ninety seconds somebody
is watching.

This is Run 16's `claims.len()` defect wearing a different hat: a correction that arrives and
then un-arrives.

Progress now goes through one writer task fed by a `watch` channel. Writes are strictly ordered,
and a run producing snapshots faster than the store can take them **coalesces** rather than
queueing — every snapshot is a whole report, so the newest is strictly better than a backlog of
stale ones, and a queue that grows under load is how a slow database becomes a slow reader.
`finish()` waits for the write in flight before `complete`, so the final report cannot be
overtaken by a partial one.

### A reclaimed run kept the dead worker's half-report

`reclaim_stale` returns a stranded row to the queue, which is right. It also left the partial
report attached, which is not: a **queued** analysis served sections from a worker that no
longer exists, and the run that replaced it started from an empty report — so a reader watching
saw answers they had already been shown blank themselves out. That reads as a retraction rather
than a restart.

Both stores now clear the report with the requeue, and the assertion lives in the shared
conformance contract so Postgres and the in-memory store cannot disagree about it.

### What review found: clearing the row is not retracting the answer

The fix above was half of one. `reclaim_stale` clearing the partial report fixes a fresh `GET`
and a reconnection — and does nothing for **the connection that is already open**, which has
already sent those sections. `sent_sections` still held them, the transition sent only
`status: queued`, and the client's `onStatus` left its section state untouched. Reproduced
through `App`: after a `$15` section and then `queued`, the dead worker's claim stayed on screen.

The interval is not small. The row has to be picked up by a worker polling once a second, then
discovery and a fetch and a model call before the replacement has anything to say — call it
thirty to ninety seconds. And **if the second run never reaches that question**, because the page
404s this time or discovery picks different pages, nothing overwrites that key and the retracted
claim stays until the run ends.

The test could not see any of it, because it asserted the *eventual* value was the second run's.
The defect is entirely in the interval.

So the stream now sends a **`reset`** when the report goes away under a live connection, and
clears `sent_sections` with it. The client throws away both copies it holds: the sections the
stream sent, and the partial report a recovery fetch cached on the analysis.

`reset` rather than `done` matters. A reader told a run finished does not reconnect, and a
reclaimed run is the opposite of finished — *keep watching, and forget what you have*.

**And review found the fix for that was itself half a fix.** It sent the retraction only when
*this connection* had already sent something — and the reader's sections survive a reconnect on
purpose, while the server's record of what it has sent starts empty on every new stream. Drop,
reclaim, reconnect, and the guard suppressed the retraction on the one connection that needed it.

The condition is the **row's** state now, not the connection's: *no report on the row means
nothing backs what the reader holds*, whoever is connected. The same rule is applied at the other
boundary, in the recovery fetch, which is the first thing to find out in that sequence. An
ordinary new analysis is also report-less, so a run now opens by retracting nothing — correct,
and free.

Once it fires per *episode* rather than per poll, something has to rearm it. Nothing tested that,
and a flag that is set and never cleared works for the first reclaim and is silent for the
second — so a run that loses two workers leaves the second one's answers on screen with no sign
anything is wrong. There is a test for two reclaims on one connection because breaking it on
purpose was the only thing that noticed.

Clearing `sent_sections` is not tidiness either: without it, a replacement run that reaches the
**same** answer would be suppressed as a duplicate, and a reader whose screen had just been
cleared would sit in front of an empty section for the whole second run. That is the failure mode
of every de-duplicating cache — correct until the thing it deduplicates against is cleared
somewhere else.

### Are the new tests worth anything?

Same question as Run 17, same method — break it on purpose and see:

| Reintroduced defect | Caught? |
|---|---|
| the stream treats any non-`running` status as the end | **yes** |
| a failed run closes the connection without a `done` | **yes**, 2 tests |
| empty sections are streamed before their question is answered | **yes** |
| a reclaimed run keeps the dead worker's partial report | **yes** |
| the row is cleared but the wire sends no retraction | **yes**, 2 tests |
| the retraction is sent but the already-sent memory is kept | **yes** |
| the memory is cleared but the reader is never told | **yes**, 2 tests |
| a `done` is sent instead of a retraction | **yes**, 2 tests |
| the client ignores the retraction | **yes**, 2 frontend tests |
| the retraction is guarded on the connection rather than the row | **yes**, 2 tests |
| it fires on every poll instead of once per episode | **yes** |
| the flag never rearms, so a second reclaim is silent | **yes** |
| the recovery fetch keeps stale sections across a reconnect | **yes**, 1 frontend test |

The first row is the one worth pausing on. A reclaimed run goes `running` → **`queued`** →
`running`, which is a *backwards* status transition no healthy analysis ever makes. A stream
that ended on "not running any more" would tell a reader the report was finished when a second
worker was about to start it — and a reader who has been told it finished does not reconnect.

Until now the stream's tests all checked *helpers*: does this section serialise, is a correction
a different payload. The loop itself — where all four of Run 16's defects lived — was driven by
nothing. It now runs against the real router with the store being mutated underneath it.

| | Rust tests | frontend tests |
|---|---|---|
| before | 442 | 24 |
| after | **454** | **27** |

### What is still not right

**There is no claim token, so a slow worker can still finish over its replacement.** If a run
takes longer than the twenty-minute staleness threshold, the sweep hands it to a second worker
while the first is alive; the first then calls `complete` and its report wins. `save_progress`
is guarded — both stores refuse a write to a row that is not `running` — but `complete` cannot
tell the two workers apart, because nothing identifies a claim. Fixing it properly is a
generation column threaded through `claim_next`, `save_progress` and `complete`, which is a
schema change and its own piece of work rather than a line in this one. Twenty minutes against a
90–180 s analysis makes it unlikely, not impossible.

**Nothing drives a worker that dies *while* the pipeline is mid-page.** These tests drive the
store and the stream through that sequence; the worker's own loop is still only exercised by a
run that completes.

**The 500 ms poll is in the tests' wall-clock.** Four tests take 2.5 s of the suite's 6 s. That
is worth it here and would not be at ten times the count.

---

## Run 17 — the sixteen defects, written down, and a seventeenth found while writing them

**Date:** 2026-08-05 · **Subjects:** ten frozen pages from six companies · **Where:** this
laptop · **Model:** none, and that is the point.

Runs 5 to 16 found sixteen defects. Every one was found the same way — point the pipeline at a
real company, read the output, notice something wrong — and **not one of them is asserted
anywhere.** The pipeline could regress to its Run 5 behaviour today and 437 tests would pass.

The half of that method which needs no GPU is now a test. Ten real pages are frozen as
Markdown in `crates/landscape-golden/pages/` (222 KB with their expectations), each beside a
JSON file saying what the deterministic passes must make of it: **every plan window a pricing
page yields, heading and body**; every capability window a features page names; the date, title
and quote of every entry a changelog carries; the window each fact on an about page was read
from.

```bash
cargo test -p landscape-golden --test the_pages
```

**0.11 s, no model, no network.** It runs on every pull request; `against_a_model` still cannot.

### Does it actually catch anything?

A golden set that has never failed is a set nobody has calibrated. So five of the sixteen
defects were put back into the code, one at a time, and the set was asked:

| The defect, as it originally was | Caught? |
|---|---|
| Run 7 — the scoring floor that let Basecamp's FAQ outrank both its plans | **yes**, 2 pages |
| Run 7 — a window with no price in it could still win | **yes**, 1 page |
| Run 8 — a page footer read as fourteen capabilities | **yes**, 1 page |
| Run 12 — the identity window's floor | **yes**, 1 page |
| Run 10 — a date mid-sentence filed as a shipped change | no |
| *(added after review)* the window body silently shortened, `WINDOW_CHARS` 1600 → 900 | **yes**, 1 page |

**Five of six.** The one that gets away is caught by a unit test in `landscape-extract` that
already existed, so the suite as a whole stops all six — but no frozen page carries a line that
begins with a date and runs into prose, and that is a real gap rather than a rounding error. It
is recorded here rather than papered over.

The last row is there because of what review found, below.

The failures name the page, quote its reason for being in the set, and print expected against
got for every page that moved, not just the first:

```text
2 of 10 frozen pages no longer read the way they did:

basecamp-pricing.md  (Pricing)
  why it is here: Two plans on one page, which is the finding Run 7 exists for…
  plan windows
  expected ["## Pro Unlimited", "## Pro"]
  got      ["## Pro Unlimited", "## Pro", "## I have pricing questions&hellip;"]
```

### What review found: freezing the heading is not freezing the window

The first version of this froze **only which headings won**. Review took one try to show what
that leaves out: change Linear Business from `$16 per user/month` to `$999` in the frozen page,
and all five tests stay green. Heading selection was unchanged, so nothing noticed.

That is the wrong half. §5.4's argument is that a bad window and a bad model **cannot be told
apart at the output**, which is the entire reason span selection is a stage — and a set that
freezes the label on a window but not its contents asserts the cheap part. Every expectation now
carries the body, stored one line per entry so a change reads as a changed line in review:

```text
linear-pricing.md  (Pricing)
  the window under `### Business`
  line 1
    expected "$16 per user/month $16 per user/month"
    got      "$999 per user/month $999 per user/month"
```

The `WINDOW_CHARS` row in the table above is the mutation that measures this, and it is one the
first version could not have caught: shrinking a window does not change which heading wins.

Review found a second thing in the subjects — the Linear Free subject expected
`billing_period: monthly` from a window that states only `$0` and *"Free for everyone"*. The
cadence belonged to the plan **below** it. That expectation would have marked the honest null
answer wrong and rewarded a model for copying a neighbour's period, which is not a hypothetical
failure: Run 3's scorecard has a model reporting `monthly` for plans with no published price at
all. The subject expects `null` now, and a new check asserts every expected period is stated in
**the part of the page about the plan being asked**, not merely somewhere on it.

### The seventeenth defect, found by building the set

Cloudflare's changelog was fetched as a tenth page. It reported **0 entries out of 0
considered** — a page of two dozen releases read as a page with no dates on it.

Every date on it is a `<time datetime="2026-08-04">Aug 4, 2026</time>`, which converts to:

```text
2026-08-04 Aug 4, 2026
```

Run 10 added a rule that a date must be followed by a separator or nothing, because *"Starting
August 11 2026, Workers will run…"* is a promise, not a release. Here the thing following the
date is **the same date, written for a person** — and the rule read it as prose and threw the
line away. The most explicitly machine-readable date shape on the web was the one we could not
read.

| | entries read |
|---|---|
| before | **0** of 0 |
| after | **24** of 24 |

Two renderings of one date are now stepped over as one date. The guard is untouched for every
other case, including the two-different-dates line — `2026-08-04 Aug 10, 2026 scheduled
changes` is a publication date and a scheduled date, and which the entry belongs to is exactly
what this parser refuses to guess.

### The model-facing set, from real pages at last

`docs/ROADMAP.md` has said since Run 5 that the new subjects **should be fetched pages, not
written ones** — because the synthetic set was predicting a performance the pipeline does not
have. Five of the fifteen subjects now are: each `source` is the verbatim window
`span::every_plan` cuts from a frozen page, so the model is scored on the text it will actually
be handed.

| Subject | From | The thing it makes hard |
|---|---|---|
| `flat-price-for-the-whole-company` | Basecamp Pro Unlimited | $299/month flat, beside a storage figure written as `10x Pro` |
| `per-user-price-among-service-numbers` | Basecamp Pro | $15/user competing with 500 GB, `24/7/365` and two trial lengths |
| `free-tier-with-real-quotas` | Linear Free | `$0` competing with 2 teams and 250 issues |
| `price-buried-under-its-own-feature-list` | Notion Plus | the answer is line two; eighty words of features follow |
| `a-pricing-page-that-states-no-price` | Todoist | says `per user/month` with **no figure** — the page that produced `Beginner at $5`, where 5 is a project limit |

A test asserts they are still verbatim in the frozen pages, because a subject a model keeps
failing invites a quiet edit to the page text, and a subject edited until it passes measures
nothing.

### What this run does not claim

It measures **us**, not a model. No accuracy number moved, because none was taken — the model
half of the set still needs a `llama-server` and is still `#[ignore]`d. Ten pages from six
companies is not a representative sample of the web, and four of the ten answer the pricing
question. The claim is narrower and worth having anyway: the specific things sixteen runs of
hand-reading found now fail a build instead of being remembered.

### What is still not right

**The date-in-prose rule has no frozen page**, per the table above.

**Nothing freezes a page against the model half.** The five real subjects are scored only when
somebody runs the bake-off by hand.

**Four of ten pages are pricing.** Changes has three, features two, identity one — and identity
is the extractor with the fewest rules and the most guesswork.

---

## Run 16 — watched in a browser, which found two things the tests could not

**Date:** 2026-08-04 · **Subject:** plausible.io · **Where:** a browser, on this laptop ·
**Store:** in-memory.

Run 15 measured the stream with `curl`. This is the same run watched the way a reader watches
it — [ADR 0011](decisions/0011-no-experiments-on-production.md)'s rule that testing happens
from the client's side, applied to our own interface for the first time.

Both defects below were invisible to 425 passing tests and to the `curl` transcript.

### The first section took four minutes

`PRODUCT_SPEC.md` §2.1A asks for content in twenty to forty seconds. The screen said *"Reading
the first pages…"* for **four minutes**.

Nothing was broken. Progress was written **per page**, and `plausible.io`'s first page is
twelve capability windows read one at a time at ~15 s each. A page is simply too large a unit
to deliver anything.

Progress is per *window* now — one fact is enough to put a section on screen:

| | first section on screen |
|---|---|
| per page | **~4 min** |
| per window | **~2 min** |

Still short of §2.1A, and the remainder is now visible rather than hidden: ~20 s of discovery,
a fetch, and one model call. What would close it is reading the cheap questions first — the
changelog needs no model at all.

### Then the section froze

With the first fix in, the screen showed *"What it does — 1 item"* and **stayed that way for
two minutes** while eight more capabilities were read.

A section was sent once, when it first had claims. **A section that arrives and then stops
changing reads as a section that is finished**, so a reader would have taken nine capabilities
for one. It is now sent again whenever it grows, and the client replaces rather than skips.

### What the finished report looks like to a reader

```text
compare plausible.io for me
Searched as https://plausible.io                      Done.

Pricing & packaging     Nothing found in public sources.
                        /pricing (404) · /plans (404) · /pricing/ (404)
What it does            9 capabilities, each quoted
Recent public changes   11 dated entries
Company facts           founded 2018 · based in EU · 10 people
Trust & security        Nothing found. /security (200) · /status (200)
```

Every claim carries its source label and the page's own words underneath.

### And failures got an audience

A prompt that names no company used to reach the reader as *"This one did not finish. Nothing
you did caused it."* — which was **both wrong and a dead end**, because they had done
something and could fix it.

`core::Failure` is a closed set of situations, not the operator's text:
`migrations/0001_init.sql` is explicit that `failure_reason` is never shown verbatim, and what
somebody is told is a presentation decision. The interface now writes:

> We could not work out which company you meant. Try naming its website — for example,
> basecamp.com.

### Two more the review found

Neither would have shown up in a happy-path run, and both are the same class of thing — a
reader left looking at something that is no longer true.

**A partial report is not a finished one.** The client treated *"we have a report"* as
settled, but `save_progress` gives a running analysis a report from its first fact. So when a
stream dropped or hit its ten-minute cutoff, the one recovery fetch came back *running*, the
client called it settled, and nothing ever reconnected — a half-written report, for ever.
Settlement is a **terminal status** now, and a non-terminal `done` reopens the stream after a
second.

**A section can change without growing.** The stream tracked `claims.len()`, and
`PagePricing::assembled` replaces a plan when a later window supplies the price the first one
lacked: *"Free is listed with no published price"* becomes *"Free costs $0"* at the same
length. The correction was suppressed and the retracted claim stayed on screen until the run
ended. It compares the payload now.

**And two more in the reconnect that fixed the first.** The recovery fetch stores a *partial*
report, and the view preferred `report.sections` the moment one existed — so the page froze at
the instant of the fetch, ignoring everything the reconnected stream sent, and rendered the
partial report's placeholder sections as *"Nothing found in public sources"* for questions
still being read. Mid-run the view now merges the stream with only those fetched sections that
have claims. Separately, the stream's last word before a drop is *"running"*, and it outranked
a recovery fetch that came back **complete** — a finished report under a "Reading…" line. A
terminal stored status now wins.

**Four defects, all in the same twenty lines, none visible to a passing test suite.** The
pattern is worth naming: every one of them is a state that only exists when something goes
*wrong mid-run*, and nothing about the happy path exercises it.

### What is still not right

**Two minutes is not forty seconds.** The order pages are read in decides what a reader sees
first, and nothing chooses it deliberately.

**A reload loses the analysis.** There is no URL for one, so a refresh mid-run leaves a reader
with nothing to return to.

**Nobody has watched this on a slow connection or a phone**, which is where a stream that
depends on a proxy not buffering is most likely to behave differently.

---

## Run 15 — the queue, end to end, with a reader watching

**Date:** 2026-08-04 · **Subject:** plausible.io · **Store:** in-memory, no database ·
**Model:** Qwen3-4B Q4_K_M.

Every previous run drove the pipeline from `landscape read`, a command. This one goes through
the product's own path: **POST an analysis, and watch the stream.**

```bash
cargo run --release -p landscape -- dev --store memory
curl -X POST localhost:8787/api/analyses -H 'content-type: application/json' \
     -d '{"prompt":"compare plausible.io for me"}'
curl -N localhost:8787/api/analyses/<id>/events
```

```text
event: status    queued
event: status    running
event: section   features    9 claims
event: section   changes    11 claims
event: section   identity    3 claims
event: status    complete
event: done
```

**Three sections arrived while it was still running.** `PRODUCT_SPEC.md` §2.1A asks for first
content in twenty to forty seconds rather than everything at ninety, and this is the shape of
it: a reader watches a report fill in.

The finished report:

| section | |
|---|---|
| pricing | *no page found. Checked: /pricing (404), /plans (404), /pricing/ (404)* |
| **features** | **9 claims** |
| **changes** | **11 claims** |
| **identity** | **3 claims** |
| trust · direction | *pages found and not read — our gap, not theirs* |

Four sources cited, every claim traceable.

### What the worker does with a prompt it cannot resolve

`NewAnalysis` takes free text, and the pipeline reads a **site**. `FACT_CHECKING.md` §3.1's
entity resolution needs candidates, candidates come from search, and §3.3 puts search last on
purpose. So the worker reads a URL or a bare domain out of the prompt, and when there is none:

```text
status: failed
reason: this prompt does not name a website, and finding one from a description needs
        the search channel that is not built yet (FACT_CHECKING.md §3.3).
        Try a domain — for example: basecamp.com
```

**A failure, not an empty report.** Guessing a domain from *"an app that helps small farms
sell to restaurants"* would produce something correctly cited and about the wrong company,
which is the most expensive wrong answer this product can make.

### Streaming without a broker

The worker and the API are two processes sharing one row. A message broker between them would
deliver events a few hundred milliseconds sooner and would be **a second piece of
infrastructure to run, supervise and lose events through** — on a box with no spare memory
([ADR 0005](decisions/0005-observability-on-a-24gb-box.md)).

So the endpoint reads the row every 500 ms and sends the difference. The queue already lives in
the database for exactly this reason. **A reader cannot tell**, and there is one fewer thing to
fail.

Two rules fell out of writing it:

- **A section is sent once**, when it first has claims. A section that is still empty carries a
  *"we found nothing"* note that is only true at the end, and sending it early would tell a
  reader we had finished looking.
- **A late write cannot resurrect a finished row.** Both stores refuse progress unless the
  analysis is still running — a worker that lost a race must not overwrite the report somebody
  is reading.

### What is still not right

**The frontend does not consume the stream.** The endpoint exists and `web/` still polls.

**`searched_as` is the origin, not a resolution.** Until search exists, what we searched for
and what the user typed are the same string in every report.

**Nothing measured how long the reader waited.** That is the client-side measurement
[ADR 0011](decisions/0011-no-experiments-on-production.md) describes, and it belongs against a
deployment rather than a laptop.

---

## Run 14 — the bake-off, run here

**Date:** 2026-08-04 · **Machine:** this laptop — 13th Gen Intel i7-1360P, 16 threads, 16 GB.
**Not the deployment host, on purpose** —
[ADR 0011](decisions/0011-no-experiments-on-production.md).

Two Phase 0 exit criteria had been waiting on somebody opening a terminal on a server. Both of
them turn out to be questions about weights and flags rather than about hardware, and both
were answered in an afternoon.

### The three-model choice — exit criterion 1

| Model | fields | perfect | **invented prices** | span median |
|---|---|---|---|---|
| **Qwen3-4B Q4_K_M** | **90%** | **8/10** | **0** | 16.4 s |
| Qwen3-1.7B Q8_0 | 87% | 7/10 | **1** | **5.8 s** |
| `unsloth` 1.7B Q4_K_M *(control)* | 10% | 0/10 | 3 | 5.5 s |

**Three points between them, and the three points are the whole decision.** The 1.7B is 2.8×
faster and it invented a price — on `contact-sales`, which the golden set describes as *"the
single most common real case on a competitor's pricing page, and the one where an invented
number does the most damage"*:

```text
expected  price None
returned  price Some(100.0), period Some(Yearly)
```

There is no page on the open web where that number appears. **The 4B has never invented one**,
across Runs 4, 12 and this one.

So the extraction tier stays the 4B, and the 1.7B is not promoted to it. Its speed is real and
it can have work where a wrong number is cheap — routing, classification — but not the tier
that fills a table a reader will check.

### The control did its job

`unsloth/Qwen3-1.7B-Q4_K_M` scored **10%, 0/10, three invented prices** — the same numbers as
Run 3, on a harness that has changed a great deal since. An instrument that still separates a
defective quantisation from a good one is an instrument the other two rows can be believed
from.

### `q8_0` KV cache — exit criterion 5

Same weights, same prompts, `-ngl 0` on both sides so the GPU is out of the experiment.

| | fields | perfect | invented | private memory |
|---|---|---|---|---|
| KV `f16` | 90% | 8/10 | 0 | 2.52 GB |
| **KV `q8_0`** | **90%** | **8/10** | **0** | **1.73 GB** |

**Identical scorecards, subject by subject — and 0.79 GB back.** The rule this was measured
against was written before the numbers existed: *take it if the score is unchanged and memory
drops; do not if the score moves at all.* The score did not move. **Adopt `q8_0` KV.**

Three resident models make that ~2.4 GB across the fleet, which is the difference between the
three-model design fitting a 24 GB box and being an argument.

### What this run cannot tell anyone

**How long a report takes in production.** Not on 16 x86 threads, and not from inside a
server either. The ratios transfer — the 4B is 2.8× the 1.7B on the span shape and will be on
any CPU — and the seconds do not.

That number is a **client-side measurement**: use the deployed product, time the wait, and
include the network and the browser, because the user does. Phase 0's latency criterion has
been restated to say so.

### Reproducing it

[MODEL_BAKEOFF.md](MODEL_BAKEOFF.md), which unlike the document it replaced has been run end
to end on the machine it was written on.

---

## Run 13 — a report, and what assembling one revealed

**Date:** 2026-08-04 · **Subject:** plausible · **Model:** Qwen3-4B Q4_K_M, unchanged since
Run 5.

Twelve runs of a pipeline printing what each stage decided. This is the first one that
produces **a report**:

```bash
cargo run -p landscape -- read https://plausible.io
```

```text
# https://plausible.io
Read 2026-08-04 19:58 UTC · 4 source(s) cited · prompts v1

## Pricing & packaging
no page found. Checked: /pricing (404), /plans (404), /pricing/ (404)

## Recent public changes
- Add notes to your traffic chart with annotations [S2·H]
  > - Jul 14, 2026

## Company facts
- says it was founded in 2018 [S3·H]
  > Uku Taht started Plausible in December 2018, building it alone…

## Trust & security posture
2 page(s) found and not read - our gap, not theirs
```

| | a run log | a report |
|---|---|---|
| Ordered by | page | **question** — which is what a reader arrived with |
| Its silence | invisible | a section carrying its coverage note |
| Its facts | uncited | **a source label and a verbatim quote, each** |

### The report found two things the run log had hidden

**A quote under the wrong fact.** `PageIdentity` held one list of quotes and paired them to
facts by position, and the assembled report rendered *"says it is based in the EU"* above a
sentence about web analytics drifting from its purpose. The run log had printed the facts and
the quote count separately and looked perfectly healthy.

**Evidence for a claim it does not support is the one thing this codebase must never render.**
Each fact now carries the quote it arrived with — `core::Stated<T>` — and the type makes the
old arrangement unrepresentable.

**And then: a quote that does not contain its fact.** With the pairing fixed, the headquarters
claim still cited a sentence that does not mention the EU — the model had picked a
neighbouring line out of the window it was given. The quote is now kept only if it contains
the value. The fact survives, because it was checked against the whole window; **the quote is
the smaller loss**.

That is the pattern this run is about. Assembling facts into claims is itself a check, and it
caught two things twelve runs of printing had not.

### What the sections are

Six, one per question discovery asks. `PRODUCT_SPEC.md` §4 fixes nine, and the other three —
positioning, sentiment themes, the SWOT — are **interpretation over sources this pipeline does
not gather**. A section that exists and cannot be filled teaches a reader to skim past empty
sections, which is the one habit this product cannot afford.

### Confidence is the fact's, not the model's

| | |
|---|---|
| a price, quoted verbatim | **High** |
| a dated change, parsed | **High**, and `as_of` the date the page states |
| a capability name | **Medium**, and it can never be more |

A capability name is a paraphrase by design — shortening a heading is what the model is *for*
here — so it cannot carry the confidence a quoted number does.

### The report checks itself

`Report::every_claim_is_traceable` refuses a citation that does not resolve, and the renderer
prints **"this report is not publishable"** rather than showing one. A citation that looks
checkable and is not is worse than no citation at all.

### What is still not right

**Two questions have no extractor**, and the report says so six times: *"2 page(s) found and
not read — our gap, not theirs."*

**`searched_as` is the origin the user typed.** The entity gate exists and this does not call
it, so the report's own account of what it looked for is a copy of its input.

**Nothing is stored.** The report is assembled, rendered and dropped. The queue, the worker and
the API have been waiting for something to carry since Phase 1 began.

---

## Run 12 — facts a page states by accident, and three checks that were not checks

**Date:** 2026-08-04 · **Subjects:** plausible, linear, basecamp, notion · **Model:** Qwen3-4B
Q4_K_M, unchanged since Run 5.

The fourth question kind: *who they are, where, how big*. It is the first one whose answers a
page is not built to publish — a pricing page exists to state a price, and **an about page
exists to tell a story**.

```bash
cargo run -p landscape -- read https://plausible.io
```

| Subject | Facts stated | What came out |
|---|---|---|
| `plausible.io/about` | **3 of 3** | founded 2018 · based in the EU · 10 people |
| `linear.app/about` | **1 of 3** | founded 2019 |
| `basecamp.com/about` | **0 of 3** | *"no stated facts about the company"* |
| `notion.com/about` | **0 of 3** | *"read 1 page, none stated anything"* |

All four are correct. Basecamp's about page says *"We're here for them, **23 years and
running**"* and never names a year — and 2003 is **arithmetic, not reading**. Nothing here
computes it.

### Three checks that were not checks

**A substring test on a number.** The model answered **"0 people"** for a page reading *"Today
Plausible is a team of 10"*, and the grounding check passed it: `"10".contains("0")`. Numbers
are matched as whole tokens now, and the test that would have caught it is in
`landscape-core`.

**All-or-nothing grounding.** The check cleared an *extraction* rather than a *field*, so one
carrying a correct founding year and an invented headquarters was discarded whole — throwing
away the answer that window had been asked for. `plausible.io/about` reported two facts of the
three it states because of it. Fields are cleared individually now, and the count is printed.

**A phrase that assumed word order.** `started in` does not match *"Uku Taht **started**
Plausible **in** December 2018"* — a name sits between the verb and the preposition. The
window went elsewhere, the model answered 2018 from somewhere it had not been shown, and the
grounding check dropped a year the page states plainly. Both halves were wrong and they
cancelled out to look like one right answer.

### And a source that was not ours

`notion.com`'s `llms.txt` lists **`linkedin.com/company/notionhq`**, and it took the identity
slot for that company. It could not be fetched, so the run printed *"could not fetch"* and the
question went unanswered.

Had it been fetched, it would have been worse. `FACT_CHECKING.md` §3.3 defines a primary source
as a page on the subject's own domain, and **only a primary source may set a value in a
comparison table**. Admitting somebody else's page here would launder it into the class that
outranks everything. Everything discovery admits is now on the subject's own site — subdomains
included, lookalike domains not.

With that fixed, notion reads its own `/about`, which states none of the three.

### What the fourth question is really like

| | pricing | identity |
|---|---|---|
| The page's purpose | state the price | tell a story |
| Where the fact is | a plan card | a subordinate clause |
| Checkable | exactly | **only that the words are on the page** |

The grounding check does more work here than anywhere else, and for a specific reason: **a
model has read about these companies**. Ask it where Notion is based over a page of prose and
the answer is available to it whether or not the page says so. Every value reported now has to
be written in the window it came from.

### What is still not right

**One window per fact means one chance per fact.** `linear.app/about` states more than a
founding year; the headquarters and headcount lines either did not score or did not survive.

**`headquarters` is a free-text field.** *"the EU"*, *"Chicago, Illinois"* and *"remote"* are
all valid answers and none of them compares to another company's.

**Nothing measures any of this.** Four about pages read by hand, no golden-set subject, and
the facts are the hardest of the four kinds to state a right answer for.

---

## Run 11 — the note that reads back what was checked, and what it found first

**Date:** 2026-08-04 · **Subjects:** basecamp, linear, notion · **Model:** Qwen3-4B Q4_K_M for
pricing and features; none for changes.

Ten runs have ended with a version of the same gap: `read` printed what it found and was silent
about what it looked for. [FACT_CHECKING.md](FACT_CHECKING.md) §5.4 has the rule — *a negative
nobody can check is not a finding* — and [PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 the output:

> Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
> /releases (404), blog (90d). **Not "no changes."**

```bash
cargo run -p landscape -- read https://basecamp.com
```

```text
question     coverage
----------------------------------------------------------------------------------------------
pricing      2 fact(s) from 1 source(s)
features     10 fact(s) from 1 source(s)
changes      no page found. Checked: /changelog (404), /releases (404), /blog (404)
identity     1 page(s) found and not read - our gap, not theirs
trust        2 page(s) found and not read - our gap, not theirs
direction    1 page(s) found and not read - our gap, not theirs
```

**Basecamp publishes no changelog.** Run 10 knew that and said nothing; this states it with the
three paths that were tried and what each returned.

### Four silences, and no two of them mean the same thing

| The note | What it means |
|---|---|
| `nothing was checked` | our gap, and the only honest thing is to say so |
| `no page found. Checked: …` | **the company publishes none of this** |
| `found and not read` | our gap again, and a different one |
| `read N page(s), none stated anything` | the page exists and does not say |

The third one exists because the fourth was lying. Four of the six questions have **no
extractor yet**, so their pages are admitted and never opened — and the first version of this
note reported *"read 1 page, it stated nothing"* about `basecamp.com/about`, a page nothing had
opened. **That is this feature committing the exact error it was built to prevent**, and it
survived one run before the numbers were separated.

### What the note found on its first honest run

```text
linear   changes   read 1 page(s), none stated anything.
                   Checked: /changelog (200), /releases (404), /blog (200)
```

`linear.app/changelog` **answers 200 and had never been read.** The page that held the changes
slot was `/docs/releases.md` — documentation about a feature called Releases, the same page Run
10 recorded as correctly reporting no dated entries. It won the slot because it was named in
`llms.txt`, and `llms.txt` outranked a probe.

So the ordering changed to match the rule locale preference already follows in Run 9: **which
page it is comes before how we found it.** Provenance is evidence *about* a page; it is not
evidence that the page is the right one.

| `linear.app` | Before | After |
|---|---|---|
| changes | `/docs/releases.md` → no dated entries | **`/changelog` → 7 changes in the window** |
| trust | `/docs/security.md` | `/security` |

Seven dated changes — *Coding sessions on mobile*, *Introducing Loops*, *Initiative
properties* — from a page that had been reachable and unread since Run 5.

### The counts, after

| | pricing | features | changes |
|---|---|---|---|
| basecamp | 2 | 10 | **none published** |
| linear | 3 | 20 | **7** |
| notion | 4 | 19 | 8 |

### What is still not right

**Three questions have no extractor, and the note now says so six times a run.** That is the
correct output and an uncomfortable one: identity, trust and direction are found on every
subject and opened on none.

**`/blog (200)` under *changes* does not say the blog was read and rejected**, only that the
path answered. The attempts list carries what discovery saw, not what extraction did with it.

**The note is per question, not per section.** `PRODUCT_SPEC.md` §4 wants it under the section
in the report; this is the CLI, and `Coverage::to_section` is the piece that will carry it
there — written and tested, not yet used by anything that renders a report.

---

## Run 10 — the question a model never sees

**Date:** 2026-08-04 · **Subjects:** notion, plausible, linear, basecamp · **Model:** none, and
that is the point.

The third question kind is *what shipped recently*, and
[ARCHITECTURE.md](ARCHITECTURE.md) §5.4 answers it without asking anything:

> | Changelog entries, release dates, version numbers | **Code** — heading + `<time>` + date
> regex | **Dates are the most common LLM fabrication in "recent changes"** and are trivially
> verifiable. |

```bash
cargo run -p landscape -- read https://www.notion.com
```

```text
www.notion.com/releases   3136 words   8 change(s) in 90 days, 0 older
  2026-07-31  AI Meeting Notes can now trigger Custom Agents
  2026-07-30  High contrast mode
  2026-07-24  Workers, now in your Notion credits dashboard
  2026-07-16  New calendar tools for your agent
  ... and 4 more inside the window
```

| Page | In the 90-day window | Older | On the page |
|---|---|---|---|
| `notion.com/releases` | **8** | 0 | 8 |
| `plausible.io/changelog` | **4** | 36 | 70 |
| `plausible.io/blog` | **7** | 33 | 104 |
| `linear.app/docs/releases.md` | — | — | **"no dated entries on the page"** |
| `basecamp.com` | — | — | **no changelog discovered at all** |

Every date above was checked against the page. The last two rows are the interesting ones.

### "No dated entries" is not "no changes"

`linear.app/docs/releases.md` is **documentation about a feature called Releases**. It has no
dates on it, and the honest output is to say so — which is what
[PRODUCT_SPEC.md](PRODUCT_SPEC.md) §4 insists on for exactly this case:

> Coverage note: no public changelog found for Shortcut in this window — /changelog (404),
> /releases (404), blog (90d). **Not "no changes."**

The same discipline shows up in the numbers beside each page. Plausible's changelog has 70
dated entries and four of them are inside the window; a report that printed four and stopped
would read as a quiet quarter for a company that ships weekly.

### The line has to *be* a date

Notion's changelog contains this sentence:

> *"Starting August 11 2026, Workers will run on Notion credits."*

A parser that takes any date it finds files a **future price change as a shipped feature**.
That is the single worst thing this could do, and the rule that prevents it is small: a date is
an entry only when it starts the line — alone, or followed by a separator and a title. The
six formats real changelogs use are all read; a date inside a sentence is not one of them.

### Two more discovery findings, both from running this

**A blog post is not a section answer.** `plausible.io/blog/product-hunt-launch` classified as
*features* on the word `product`, and `/blog/changelog-podcast` as *changes* on the word
`changelog` — a podcast episode. Both were admitted, both were read, and both cost a slot. The
index of a publication still counts; a post inside it does not.

**The page that names the question outranks the broad one.** `notion.com` publishes `/releases`
and `/blog`. Both answer *what changed*, both came from `llms.txt`, and the tiebreak was URL
length — so `/blog` went into the report with **no dated entry on it** and the changelog went
unread. Length is now the last word rather than the second.

### What is still not right

**The window is counted, not the coverage.** *"4 in 90 days, 36 older"* says what the page
holds. It does not say whether the page is the company's only changelog, which is the other
half of §4's coverage note.

**Basecamp has no discovered changelog**, and the run says nothing about that at all — the
absence of a source is currently invisible, where §4 wants it stated with the paths that were
checked.

**A job posting is still an article.** `plausible.io/jobs/customer-success` is admitted as
*direction*, which is the same shape as the blog-post problem in a place the rule does not
reach.

---

## Run 9 — the wrong pages, and what reading the right ones changed

**Date:** 2026-08-04 · **Subjects:** linear, todoist, notion, basecamp · **Model:** Qwen3-4B
Q4_K_M, unchanged since Run 5.

Runs 7 and 8 each ended with a finding nothing downstream could fix. An extractor pointed at
the wrong page has two options, a wrong answer and silence, and it had been producing both.

```bash
cargo run -p landscape -- discover https://linear.app
```

| Subject | Before | After |
|---|---|---|
| **linear** | two setup documents as the feature sources | **`/features` and `/docs`** |
| **todoist** | 8 sources: Czech, Danish, and two steps in buying | **3, all English, `/pricing` read at last** |
| **notion** | `/es-es/pricing` took a slot | `/plans` takes it |
| **basecamp** | 6 sources | **6 sources, unchanged** — the control |

### The features page Linear had all along

`linear.app/docs/mcp.md` is a setup guide. Read as a features page it reported `Setup`,
`Claude` and `Cursor` as capabilities of Linear (Run 8) — and worse, **it took the slot**.
Both of Linear's feature sources were documentation, because `llms.txt` outranks a probe and
`guess` classified anything under `/docs` as answering *what does this product do*.

Documentation answers *how do I use this*. Only its front page answers the other question.

With the depth rule, `linear.app/features` is read for the first time:

```text
linear.app/features   250 words   8 capabilities stated
  Planning · Building · AI-powered workflows and agents · Insights
  Mobile · Customer Requests · Linear Asks · Security
```

Eight capabilities, no dropped names, no unverifiable quotes. The page had been there for
every run since Run 5.

### A locale is a variant, not a page

Run 7 read `todoist.com/cs/pricing` and `/da/pricing` — Czech and Danish — and never read the
English page. Run 8 added `notion.com/es-es/pricing`.

**The filter that would have been wrong** is dropping localised URLs: some sites publish only
`/de/preise`, and that is then the pricing page. So variants collapse into one candidate the
way `/pricing` and `/pricing/` already did, and which one wins is a preference — no locale,
then English, then whatever the site listed.

That the Czech page was better attested than the English one is exactly why the order matters:
**which page it is comes first, how we found it comes second.**

### And a page you do something on states no facts

Todoist's sitemap lists `/cs/pricing/setup` and `/cs/pricing/upgrade`. Both classify as
pricing on the word `pricing`, both are steps in buying, and between them they took two more
of that run's five slots.

Todoist now admits three sources from eighteen paths checked, all English, and one of them is
the pricing page it publishes.

### Fewer sources is the result, not a regression

| | Sources | What they are |
|---|---|---|
| todoist, Run 7 | 8 | 4 duplicates or transaction steps |
| todoist, Run 9 | **3** | pricing, features, security |

A source that cannot answer the question it was admitted for costs a fetch, a model pass, and
a line in the report saying something we cannot stand behind. **Three we can read beats eight
we cannot.**

### What is still not right

**`linear.app/docs` gives twelve capabilities and one of them is `Popular`.** The front page of
a documentation site is partly a menu, and the menu headings survive the capability rules —
though the other eleven (`Import Issues`, `Triage`, `Parent and Sub-Issues`, `Notifications`)
are real.

**The language list is a list.** Forty codes, matched only as a leading segment with a page
under it, because two-letter segments are not all languages — `/hr` is Croatian on one site
and human resources on the next. A site using a code outside the list loses the deduplication
and nothing else.

**None of this is measured by the golden set**, which still has no discovery subjects at all.

---

## Run 8 — the second question kind, and a prompt that taught the model a fact

**Date:** 2026-08-03 · **Subjects:** basecamp, linear, notion · **Model:** Qwen3-4B Q4_K_M,
unchanged since Run 5.

Pricing was the first question. This is *what does the product do*, and
[ARCHITECTURE.md](ARCHITECTURE.md) §5.4 says how to answer it:

> | Feature lists on structured pages | **Code first**, model for normalization only |

```bash
cargo run -p landscape -- read https://basecamp.com
```

| Page | Sections named | Windows read | Capabilities reported |
|---|---|---|---|
| `basecamp.com/features` | 18 | 12 | **10** |
| `notion.com/product/ai` | 20 | 12 | **8** |
| `notion.com/product/docs` | 21 | 12 | **8** |
| `linear.app/docs/sla.md` | 12 | 12 | **8** |
| `linear.app/docs/mcp.md` | 6 | 6 | **5** |

Basecamp's ten are Message Boards, Hill Charts, Card Tables, Campfire chats, Automatic
Check-ins, Docs & Files, Reports, and three more — every one of them a real Basecamp feature.

### The guess about page shape was wrong

The plan was to read bullet lists. Four real features pages disagree:

```text
## Message Boards for announcements and discussions      basecamp.com/features
### Customer Requests                                    linear.app/features
### Capture                                              notion.com/product/docs
```

**A capability is a heading with a description under it**, and the bullet lists on those pages
are navigation — `- Pricing`, `- Log in`, `- Books we've written`. Reading bullets would have
reported Basecamp's footer as fourteen features.

That leaves the model exactly §5.4's job: `Message Boards for announcements and discussions` is
a heading a parser can find, and `Message Boards` is a name only a reader can cut out of it.

### The prompt taught the model a fact, and the model used it

The first prompt carried a worked example — *"a heading reading **Message Boards** for
announcements and discussions names a capability called Message Boards"*. On the next run,
`linear.app/docs/mcp.md` reported:

```text
MCP Server · Claude · Cursor · MCP · Message Boards
```

**Message Boards is Basecamp's feature.** It reached a Linear report through our own prompt.

`FACT_CHECKING.md` §P15 is about laundering someone else's hallucination into our citation;
this is the same failure with a shorter supply chain. **A worked example in a prompt is a
source of facts**, and it is one with no URL, no fetch date, and nothing to cite.

### Which is why a name is checked, not trusted

A price can be checked verbatim. A capability name **cannot** — naming is the normalisation the
model is there to do, so the answer is a paraphrase by design. The check that still holds is
weaker and cheap: `FeatureExtraction::name_is_from` requires every word of the name to appear
in the section.

| It caught | On |
|---|---|
| `Message Boards` | `linear.app/docs/mcp.md` — the prompt leak, before the example was removed |
| `string` — the field's own type | `linear.app/docs/sla.md`, twice |
| 10 more names across five pages | all subjects |

Removing the example fixed the first. The rest are dropped and counted, and `read` prints the
count: *"4 name(s) dropped — not words from the section"*. **A page whose windows mostly fail
this check is a page we should not be reading**, which is a signal worth having.

### A page that was already Markdown

`linear.app/docs/mcp.md` reported **no capabilities at all** before any of this. The cause was
upstream of the extractor: `llms.txt` publishes a site as Markdown, discovery follows it, and
the HTML converter turned that page into **one 2,167-word line**. A `#` is text to an HTML
parser, and there were no block tags to break lines on.

Nothing downstream could find a section in it, so the page reported nothing and looked exactly
like a page that had nothing on it. `markdown::from_body` now recognises a Markdown body.

### What is still wrong

**Two of Linear's three "features" pages are documentation.** `/docs/mcp.md` is a setup guide,
and its sections are named for other people's editors — `### Zed`, `### Windsurf`. Read as
capabilities they say Linear's product includes Zed. Sections containing a code block are now
skipped, which removes the most confident version of that answer, and `Setup` and `Claude`
still come through. **The label came from discovery, and only discovery can fix it.**

**The model shortens most names and not all.** `The Project page keeps it all together` came
back whole. Nine of twelve on Basecamp were cut correctly; the rest are the heading.

**A capability with no qualifier may still be conditional.** Nothing on these pages said
*beta* or *Business and above*, so the `qualifier` field is measured only by unit tests.

---

## Run 7 — six real pricing pages, and the one that had no prices on it

**Date:** 2026-08-03 · **Subjects:** basecamp, linear, plausible, notion, sentry, todoist ·
**Model:** Qwen3-4B Q4_K_M, unchanged since Run 5.

Run 6 got `basecamp.com/pricing` right and reported one of its two plans. This is one window
per plan — `span::every_plan` — measured against six pages, of which four had never been seen
by the code before.

```bash
cargo run -p landscape -- read https://basecamp.com
```

| Page | Plans published | Windows | Reported |
|---|---|---|---|
| `basecamp.com/pricing` | 2 | 2 | ✅ Pro Unlimited $299, Pro $15 |
| `linear.app/pricing` | 3 | 3 | ✅ Free $0, Basic $10, Business $16 |
| `plausible.io` | 3 | 3 | ✅ Starter $9, Growth $14, Business $19 |
| `notion.com/pricing` | 3 | 5 | ⚠️ Free $0, Plus $10, Business $20 — **and an add-on** |
| `sentry.io/pricing` | 4 | 6 | ✅ each plan appears twice; the duplicates are dropped |
| `todoist.com/pricing` | 3 | **0** | ✅ "no pricing content on the page" |

The plausible, notion, sentry and todoist pages were fetched *after* the heuristic was
written. Two of them changed it.

### todoist is the finding

Its prices are rendered in JavaScript, so what reaches the Markdown is the feature-comparison
table and nothing else. Forty table rows outscore any real pricing block on structure alone,
and not one of them contains a currency symbol. The window chosen from it was:

```text
|  | Beginner | Pro | Business |
|---|---|---|---|
| Personal projects | 5 | 300 | 300 for each member |
```

and the model returned **"Beginner at $5"**. The 5 is how many personal projects the plan
allows. **The HTML contains no dollar amount anywhere.**

A window must now contain a price or a contact-sales line to win at all. The page then reports
*"no pricing content on the page"*, which is both true and the number
[ARCHITECTURE.md](ARCHITECTURE.md) §5.5's JavaScript-gap counter exists to collect — the
counter cannot be honest if the pipeline guesses instead of abstaining.

### notion changed the prompt twice

**`$10 per 1,000 monthly Notion credits` came back as `$0.01`.** The model divided. It is
arithmetic rather than extraction, and it is not checkable against the page, so the prompt now
says price_usd must be a number written there.

**And the add-on stayed.** `### Custom Agents` is a heading with a price under it, which is
the definition of a plan block this code uses, so notion reports four plans where three are
real. There is no shape that separates an add-on from a plan — only meaning — and that is
worth stating rather than tuning away.

### What the page shapes turned out to be

| | |
|---|---|
| basecamp | `## Pro Unlimited` then a subtitle |
| notion | a marketing line then `### Free` |

**The same shape, upside down**, and heading levels cannot tell them apart. The shorter
heading wins: a plan name is a noun and a subtitle is a sentence. On all six pages that picks
the plan name.

### The weakest field is the billing period

`linear.app` writes `$16 per user/month` and `Billed yearly` in the same block. The price is
right on all nine plans across the three English pages; the period was wrong on one of them,
and the prompt rule that fixed the other two — *how often the price recurs, not how often the
invoice arrives* — did not fix that one.

**`BillingPeriod` conflates two facts the pages state separately.** Recorded, not fixed: it
needs a type change and golden-set coverage rather than another sentence in the prompt.

### Still not measured

**The golden set does not cover any of this.** Its subjects are single-plan fixtures, so
`every_plan` returns one window for each and the whole multi-plan path is exercised only by
unit tests and by this run. §5.4's warning stands: the six pages above are six data points and
four fixed bugs, not a validation.

---

## Run 6 — the window fixes it, and the window was wrong twice first

**Date:** 2026-08-03 · **Subjects:** `basecamp.com`, `linear.app` · **Model:** Qwen3-4B
Q4_K_M, unchanged from Run 5.

```bash
cargo run -p landscape -- read https://basecamp.com
```

Run 5 established that the model works on a span and fails on a page. This is span
pre-selection ([ARCHITECTURE.md](ARCHITECTURE.md) §5.4) built and pointed at the same page.

| Attempt | `basecamp.com/pricing` returned |
|---|---|
| Run 5 — whole page, 1729 words | ❌ "no price published" |
| First scorer — 283-word span | ❌ **"Timesheet at $50"** |
| Final scorer — 264-word span | ✅ **"Pro at $15"** |

`$15/user` is Basecamp's Pro plan. `$50/month` is the Admin Pro Pack add-on, mentioned in
the FAQ.

### The middle row is the finding

The first scorer chose the FAQ, and the reason is instructive: **the FAQ mentions prices
several times in close succession, while the actual plan states its price once.** Any scorer
summing price-shaped signals over a window prefers the denser region, and the denser region
is the one full of exceptions and hypotheticals.

What separates them is not vocabulary. It is **shape**:

| | A plan block | An FAQ |
|---|---|---|
| Distance to its heading | 1–2 lines | dozens |
| Question marks | none | one per entry |

Both are now signals, and neither is specific to Basecamp.

### Two more, found the same way

**The heading was anchored to the wrong line.** A window wide enough to hold a short page
starts at line 0, where the nearest heading above is the page title — `# Pick a package` —
rather than the one governing the price. It is now anchored to the best-scoring line.

**And it took the nearest heading rather than the most significant one.** A plan is written
as a name and then a subtitle, so the nearest heading hands the model *"All-inclusive
pricing"* as the thing `$299` belongs to. That is not a plan name.

### Running the right extractor

`linear.app/docs/mcp.md` returned **"MCP server at $0"** — a plan that does not exist, from a
documentation page. The pricing extractor was being run over every discovered source.

Discovery already labels what each page answers, so using that label costs nothing and fixes
it. It also cut the model calls per company from 8 to 2.

| | `linear.app` |
|---|---|
| `/pricing` | ✅ "Free at $0" — correct, Linear's first plan |
| `/docs/*`, `/about`, `/careers` | "not a pricing page — no extractor yet" |
| `/plans` (2 words) | "skipped — nothing to read" |

### What is still not right

**Basecamp has two plans and we report one.** `$15/user` Pro and `$299/month` Pro Unlimited
are both on that page; `PricingExtraction` models a single plan, so the window picks one and
the other is invisible. Extracting *all* plans needs a different type, and it is the next
piece of work rather than a tuning problem.

**The heuristic is `SPAN_VERSION = 1` and has been measured against two companies.** §5.4
says it *"is itself part of the golden-set evaluation, because a bad window is
indistinguishable from a bad model at the output"*. Until the golden set covers spans, this
run is two data points and a fixed bug, not a validation.

---

## Run 5 — the golden set's own warning, come true

**Date:** 2026-08-03 · **Subject:** `basecamp.com`, whole pipeline · **Model:** Qwen3-4B
Q4_K_M, the one that scores 90% on the golden set.

```bash
cargo run -p landscape -- read https://basecamp.com
```

The first run of the joined pipeline. Discovery found 6 sources, all fetched, all converted
to Markdown, all scored `good`. Then:

| Page | Words | Quality | Extracted |
|---|---|---|---|
| `basecamp.com/pricing` | 1729 | good | **"Pro, no price published"** |

**Basecamp publishes `$299/month` and `$15/user` on that page.** The model that scores 90%
on the golden set and never invents a price had just failed to find one that was plainly
there.

### Where it was lost, and where it was not

Traced stage by stage, because the point of a joined pipeline is that a wrong answer has six
possible causes:

| Stage | Verdict |
|---|---|
| Fetch | ✅ 200, full page |
| Markdown | ✅ `## Pro Unlimited` at line 11, `$299/month` at line 13 |
| Truncation | ✅ Both inside the 6000 characters sent |
| Prompt | ⚠️ Did not name a plan. Fixed — **did not help** |
| **Model, given 1729 words** | ❌ **"no price published"** |
| **Model, given the 39-word span** | ✅ **`Pro Unlimited`, `$299`, verbatim quote** |

Same model. Same prompt. Same words. **The only difference is how many of them.**

### This is the thing Run 3 said would happen

> *"It has ten subjects, all pricing, all in English, all written by the same person on the
> same afternoon — so it shares that person's blind spots, and **a model could score 100% on
> it while failing on the first real page it meets**."*

It scored 90% and failed on the first real page it met. The golden set's pages are ~100 words
of clean prose about one plan. `basecamp.com/pricing` is 1729 words, three plans, a cookie
banner and an FAQ.

**The golden set is not wrong — it is measuring a different thing than the pipeline does**,
and until this run there was nothing to reveal the difference.

### What it justifies

**Span pre-selection**, [ARCHITECTURE.md](ARCHITECTURE.md) §5.4 — feeding the model the
relevant ~400-token window rather than the page. It was already in the plan on the argument
that prefill dominates on 4 ARM cores. It now has a second and stronger argument: **without
it, extraction on real pages does not work at all.**

That reframes it from an optimisation to a correctness requirement, which changes where it
belongs in the order of work.

**And the golden set needs real pages.** `ROADMAP.md` takes it to 25 subjects in Phase 1;
those should be fetched, not written, or the set will keep predicting a performance the
pipeline does not have.

---

## Run 4 — the JavaScript-rendering gap, measured

**Date:** 2026-08-03 · **Sample:** 28 real pricing pages, `docs/js-gap-sample.txt` ·
**Question:** [ARCHITECTURE.md](ARCHITECTURE.md) §5.5's two counters.

```bash
cargo run -p landscape -- gap docs/js-gap-sample.txt
```

| Where the price was | Pages | Share |
|---|---|---|
| **Tier 1** — visible in static HTML | 24 | **85.7%** |
| **Tier 2** — recovered from embedded JSON (JSON-LD) | 1 | 3.6% |
| Residual — neither | 3 | 10.7% |
| *Unreachable, excluded* | *1* | — |

### The residual is two different things, and separating them is the finding

| Page | What it is |
|---|---|
| `hetzner.com/cloud` | **A genuine JS-rendered page** — no `€` amount and no price-shaped JSON key anywhere in the bytes |
| `databricks.com/company/contact` | Publishes no price. Control group |
| `palantir.com/platforms/foundry/` | Publishes no price. Control group |

**So the JavaScript-rendering gap is 1 page in 28 — 3.6% — and the residual is 10.7%.**
Those fall on opposite sides of §5.5's ~5% threshold, and only the first is the number the
rule is about. No headless browser renders a price that was never written.

**Tier 5 is not built.** [ADR 0009](decisions/0009-no-headless-browser.md).

### Two things this run says about the plan

**"The big one" is smaller than expected.** §5.5 predicted embedded state would close most of
the gap. It recovered exactly one page — because the gap it was aimed at barely exists.
**85.7% of pricing pages simply print their prices in HTML.** The prediction is not wrong so
much as pointed at a problem that turned out to be small.

**The control group caught a bug in the instrument on its first run.**
`databricks.com/company/contact` was reported as *priced*, because the detector matched
*"Learn professional Data and AI tools for free"*. Without two deliberately price-free URLs in
the sample, the tier-1 count would have been quietly one too high and nobody would have
looked.

### How far to trust it

**Twenty-eight pages chosen by one person tells 40% from 4% and nothing finer** — which is
the decision in front of us, and it is not close. It is **not a market statistic.**

The sample skews toward companies that publish prices, because those are the pages this
product reads. Tiers 3–4 were not measured, so 3.6% is an **upper bound**. And one page's
classification moves the figure by 3.6 points, which is why §5.5 requires a Phase 2
re-measurement rather than treating this as settled.

---

## Run 3 — the golden set, and what shape tests could never see

**Date:** 2026-08-03 · **Host:** same laptop, three `llama-server` processes resident ·
**Task:** ten frozen pricing pages, one plan each, scored against hand-written references.

```bash
LLAMA_URL=http://127.0.0.1:8080 \
  cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

Prompt **v2**. Seed 7, temperature 0. Verdicts are reproducible — two consecutive 4B runs
produced byte-identical verdict tables, differing only in latency.

| Model | Fields correct | Perfect subjects | **Invented prices** | Any fabrication | Median |
|---|---|---|---|---|---|
| Qwen3-4B Q4_K_M | **90%** | 8 / 10 | **0** | 1 | 11.7 s |
| Qwen3-1.7B Q8_0 | 87% | 7 / 10 | **1** | 2 | 7.5 s |
| ~~Qwen3-1.7B Q4_K_M (`unsloth`)~~ | **10%** | 0 / 10 | **3** | 8 | 5.7 s |

### The result this was built for

**Run 2 gave the defective quantisation a clean bill of health on every measure it had:**
0/20 unparseable, and the *fastest* median in the table. Only a hand-written note recorded
that its output was garbage.

The golden set scores it at **10%**, against 87% for the official quantisation of the same
model at the same size and the same speed. That gap is now a number a test can fail on
rather than a comment somebody has to read.

The value is not that we caught this one — we already had. It is that the next one gets
caught by machinery instead of by luck.

### The 4B is the only model here that can be trusted with a price

The single assertion in the golden-set test is that no price is returned for a plan whose
page publishes none. **Qwen3-4B passes it. Qwen3-1.7B Q8_0 does not**, and the way it fails
is worth reading:

| | expected | returned |
|---|---|---|
| `contact-sales`, plan `Enterprise` | no price | **$49 / monthly** |

$49 is the price of *Grower*, the plan directly above it on the page, and the model quoted
that line verbatim. It did not hallucinate — it answered a neighbouring question and
supported the answer honestly. Every fact in the output is true of *something* on the page.

That failure mode matters more than a hallucination would. A fabricated number often looks
wrong; a correctly-quoted number attached to the wrong plan looks exactly like a correct
answer, including to the quote-fidelity check, which passes it.

**So the tiering thesis from Run 2 survives, but with a boundary Run 2 could not see.** The
1.7B is 1.6× faster and nearly as accurate in aggregate — and aggregate accuracy is the
wrong measure for the tier that assigns a dollar figure to a named company. A 1.7B router
picking which spans to read is still well supported. A 1.7B extractor is not.

### What v2 of the prompt changed, including what it cost

Both good models returned `billing_period: "monthly"` for plans they had *correctly*
reported as having no published price — an invalid state, and one the type cannot forbid
(see [ADR 0004](decisions/0004-require-every-property.md) for why the tagged-union fix
measured worse). v2 states the invariant in prose instead.

| | v1 | v2 |
|---|---|---|
| Qwen3-4B — perfect subjects | 7 / 10 | **8 / 10** |
| Qwen3-4B — fabrications | 3 | **1** |
| Qwen3-1.7B Q8_0 — fields correct | 73% | **87%** |

**It was not free.** The 4B now returns nothing at all for `plain-table` — the easiest
subject in the set, a price in a plain table — where v1 answered it correctly. Reproducible
across both runs. Three added lines of caution bought two fewer fabrications and one new
silence on the simplest possible case, which is the trade constrained extraction seems to
offer everywhere: abstention and coverage move together.

Recorded rather than tuned away. Chasing it with a fourth prompt revision would be fitting
the prompt to ten pages we wrote ourselves.

### What the set does not tell us

It has ten subjects, all pricing, all in English, all written by the same person on the same
afternoon — so it shares that person's blind spots, and a model could score 100% on it while
failing on the first real page it meets. It is a floor, not a certificate. `ROADMAP.md`
takes it to 25 subjects in Phase 1 and 50 in Phase 2, and the first user-reported error
belongs in it the day it arrives.

---

## Run 2 — the tiering thesis, tested

**Date:** 2026-08-03 · **Host:** Windows 11 laptop, x86-64, 8 threads, CPU only — **not the
target hardware** · **Task:** extract a `PricingFact` (string, f64, 3-variant enum,
`Option<u32>`) under constrained decoding.

> [!WARNING]
> **Confound, stated up front.** These runs happened with **three `llama-server` processes
> resident** on one laptop, competing for the same cores. Absolute latencies are therefore
> pessimistic. The *comparison* between models is fair — all three ran under the same
> contention — but do not read any single figure as what one model alone would do. Run 1
> measured the same 4B task at **10.9s** median with only one server up, against **12.6s**
> here.

| Model | Shape | Median | p95 | Unparseable | Wrong contents |
|---|---|---|---|---|---|
| **Qwen3-1.7B Q8_0** | span (~400 tok) | **3.8 s** | 4.4 s | 0/20 | 0/20 |
| Qwen3-1.7B Q8_0 | sentence | 4.6 s | 5.7 s | 0/20 | 0/20 |
| Qwen3-4B Q4_K_M | span (~400 tok) | 12.6 s | 18.5 s | 0/20 | 0/20 |
| Qwen3-4B Q4_K_M | sentence | 16.6 s | 25.6 s | 0/20 | 0/20 |
| ~~Qwen3-1.7B Q4_K_M (`unsloth`)~~ | either | 12–16 s | — | 1/20 | **20/20** |

### What this settles

**The tiering thesis holds.** A 1.7B router is **~3.3× faster** than the 4B on the realistic
span shape, at no measured cost in accuracy on this task. `ARCHITECTURE.md` §4.7's three-tier
design is worth building.

**In budget terms:** 120 seconds buys roughly **32 extractions** on the 1.7B against **10** on
the 4B — before any fetching, and under three-way contention. Run 1's pessimism about the
90–180s promise was premature: it measured a 4B doing a 1.7B's job.

**Longer prompts did not cost proportionally more.** The span shape is roughly 13× the
sentence shape in characters, and was *faster* on both models. Prefill on x86 with 8 threads
is not the binding constraint the architecture expects it to be on 4 ARM cores — **which is
exactly why the *ratio* is the transferable part and the seconds are not.**

### Two findings that cost more than the timings

**A defective quantisation produces schema-valid garbage.** The `unsloth` Q4_K_M build of
Qwen3-1.7B returned things like:

```json
{"plan_name": "/:D!01:56:G>!#9*2-@1F-08@E5A0'(5,#0#D>9G", "price_usd": 9, ...}
```

Perfectly shaped. Entirely wrong. **Constrained decoding guarantees shape, never content** —
so a broken model or quantisation sails straight through the one mechanism that looks like it
should catch it. The official Q8_0 of the *same model* was flawless.

The practical consequence: **a model swap needs an accuracy check, not just a latency
check.** `landscape-bench` now reports `wrong contents` separately for that reason, and the
golden set exists to make the check meaningful.

**`schemars` does not bound its integers, and llama.cpp does not infer the bound.** A `u32`
becomes `{"type":"integer","format":"uint32","minimum":0}` — with no `maximum`. The grammar
therefore permits integers no `u32` can hold, and the 1.7B produced
`"order_limit": 1000000000000000` in **6 of 20** runs. `serde` rejected them, surfacing as
`LlmError::Unparseable` — which reads as *"constrained decoding is broken"* when the
constraint was simply never told the real limit.

`landscape-llm` now adds the missing bounds before sending the schema. Same model, same
prompts, after the fix: **0 of 20**. See [ADR 0003](decisions/0003-bound-integer-schemas.md).

The larger model happened not to hit it. That is precisely how a bug like this reaches
production.

---

## Run 1 — constrained decoding, the exit criterion

**Date:** 2026-08-03 · Qwen3-4B-Q4_K_M, single server, same laptop.

| Measure | Value |
|---|---|
| **Parse failures** | **0 / 100** |
| Content mismatches | 0 / 100 |
| Median latency | 10.9 s |
| p95 | 17.2 s |

Plus ten runs at temperature 0.9 in which a three-variant enum never wandered outside its
three variants.

**This is the Phase 0 exit criterion for constrained decoding, and it is met.** Rust struct →
`schemars` schema → llama.cpp's GBNF → constrained sample → parsed back, with no retry logic
and no defensive re-parsing anywhere in the path.

```bash
cargo test -p landscape-llm -- --ignored --nocapture
```

---

## Still to measure

The list below has been split by **where the answer lives**, which is the change
[ADR 0011](decisions/0011-no-experiments-on-production.md) makes. Most of it was never a
question about the deployment host.

**Locally, and nothing stops it:**

- [x] ~~`q8_0` KV cache quantization validated against the golden set~~ — Run 14. Adopted.
- [x] ~~One server at a time, so the numbers are not contended~~ — Run 14 ran the KV
      comparison CPU-only with the GPU out of the experiment.
- [ ] Prefill and generation tok/s separately, rather than end-to-end latency
- [ ] `Q4_K_M` **and** `Q4_0` — the repacking question is a property of the format and the
      CPU's instructions, and an ARM laptop or a cheap ARM VM answers it as well as anything
- [ ] Qwen3 1.7B / 4B / 8B / 14B, Gemma 3 4B/12B, Llama 3.2 3B. **Licence review first**
- [ ] Aggregate throughput at `--parallel 1/2/4/8`
- [ ] Time-to-first-token
- [ ] Resident RAM for three models against the ~17 GB budget

**From a client, against something deployed — never from inside it:**

- [ ] How long a report takes, end to end, as a user experiences it. That includes the
      network and the browser, which a stopwatch on the server cannot see.
- [ ] Whether the 90–180 second promise holds for somebody sitting in front of it.

**Not a measurement at all:**

- [ ] ~~Provision the A1 and convert to Pay-As-You-Go~~ — a deployment step, and not one that
      belongs on a benchmarking list.

Run 3 adds two of its own:

- [ ] **Field order as a lever.** llama.cpp walks properties in the order the schema
      serialises them, and `serde_json`'s sorted maps currently pick that order for us.
      `preserve_order` would hand it back. Quote-first (read, then answer) against
      quote-last (answer, then justify) is a measurable question, and a hand probe showed
      quote-first spends the whole token budget quoting — so it needs a `maxLength` too
- [ ] **Whether the 1.7B's plan confusion survives better prompting**, or is a size limit.
      This decides whether the Extractor tier can ever be a 1.7B, which is the difference
      between ~7 s and ~12 s per span on the numbers above
