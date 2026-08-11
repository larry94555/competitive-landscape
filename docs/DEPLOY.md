# Landscape — why the deployment is shaped the way it is

**The steps are in [GO_LIVE.md](GO_LIVE.md).** This file is the argument behind them: what each
decision is protecting against, which parts are guesses, and what to check first when something
surprises you.

> **They were one document and that was a copy of a rule.** The procedure and its reasoning were
> interleaved here, which made the procedure slow to follow and — once a second document existed
> — gave the model pin, the checksum and the firewall rules two homes. Two copies of a command
> drift, and the copy that drifts is the one nobody runs. **Every command lives in
> [GO_LIVE.md](GO_LIVE.md) and nowhere else**; this file names steps rather than repeating them.

> **None of it has been run.** There is no box reachable from this repository
> ([PROJECT_STATUS.md](../PROJECT_STATUS.md) B5), so all of it is reasoned from our own code and
> from Oracle's documented behaviour rather than from a deployment that worked. The first person
> through is the one who finds out where it is wrong.

**Deploying changes nothing but secrets.** Everything environmental is already an environment
variable — `BIND_ADDR`, `DATABASE_URL`, `LLAMA_URL`, `WEB_DIR`, `SEARX_URL` — and nothing in the
code branches on where it is running. If a step ever needs a code change, that is a defect worth
fixing rather than a step worth documenting.

---

## The two things that will bite

### The URL has a cap on it, and it is not a wall

One anonymous address may start **two analyses a day** — `PRODUCT_SPEC.md` §2.1's number — after
which `POST /api/analyses` answers `429` with a sentence saying when to come back. Reading a
report, watching one arrive and reloading its URL are never capped: the cap counts where a run
*starts*, because that is the request that occupies the model for minutes.

It reads the client from `X-Forwarded-For` — **the rightmost entry, which is what Caddy
observed** — so it only applies to requests that came through the proxy. A request straight to
the API is not counted, which is why [USING_THE_SITE.md](USING_THE_SITE.md) tells you to raise
`ANONYMOUS_DAILY_LIMIT` before walking the features: that tour needs more than two runs.

**Two a day is not a wall, and this is still worth knowing.** The cap is per address, it resets
at midnight UTC — which the refusal states, rather than saying "tomorrow" — and it is held in
memory, so a restart clears it. **A failed analysis costs nothing**, including one the worker
fails after accepting it. It stops a URL being drained by strangers; it is not an accounting
record.

**Keep the allow-list anyway while you are trying this out.** Two analyses a day from each of
many addresses is still more than four ARM cores will enjoy.

### The wait is four minutes, and this box is slower than a laptop

An example idea names two companies at about two minutes each *on a developer laptop*
([BENCHMARKS.md](BENCHMARKS.md) Runs 21–23). Four ARM cores are not four laptop cores, so treat
the laptop numbers as a lower bound. First content should still be quick — the changelog needs no
model at all — but the whole report will take longer than it does at home, and finding out by how
much is one of the two reasons to deploy at all.

The other is that [ADR 0011](decisions/0011-no-experiments-on-production.md) says the end-to-end
figure can only be taken **from the client's side of a deployment**.

---

## The decisions, step by step

### One instance, not four [step 2](GO_LIVE.md#step-2--create-the-instance)

Oracle's Always Free tier gives 4 OCPU and 24 GB of Ampere A1 across your instances. Take it as
one: the model wants the memory and the extraction wants the cores, and splitting them helps
nothing. 50 GB of boot volume is comfortable — the model is ~2.5 GB and Rust's build directory
is large.

*"Out of host capacity"* is a real and common Oracle condition, not a mistake in your request.

### Two firewalls, and the allow-list is not in either [step 4](GO_LIVE.md#step-4--open-the-two-firewalls)

**Ports 80 and 443 are open to the internet, deliberately.** The first version of this document
restricted them to one address, and review pointed out that this made the whole procedure
impossible: Let's Encrypt validates the domain by reaching this host, and it does not come from
your address. A certificate could never have been issued — and if one somehow had, renewal would
have failed sixty days later, quietly.

So the ports are open, the certificate works, and **Caddy is what refuses everybody but you**.
That is a weaker boundary than a firewall and it is the one that can actually exist: it is one
process rather than the whole box, and a refused request never reaches the application or the
model.

**Nothing needs opening for 8787, 8080 or 8888.** The API binds to loopback and Caddy reaches it
locally; the model server and the search engine never leave the machine.

**That was a promise the compose file did not keep.** `8888:8080` publishes on every interface,
not on loopback — so the search engine was one wrong security-list rule away from being a public,
unauthenticated service, and this paragraph said otherwise. It is `127.0.0.1:8888:8080` now, and
[GO_LIVE.md](GO_LIVE.md#step-10--start-the-search-engine) checks the listening address rather
than assuming it. Review found it; a firewall that happens to be closed is not the same as a
service that is not listening.

> **This was the step I was least able to verify, and the first real box settled it.** I assumed
> Oracle's Ubuntu images ship rules that accept SSH and reject the rest, so the whole step was
> about inserting above the `REJECT`. The image actually used shipped an **empty INPUT chain
> with `policy ACCEPT`** — nothing to insert above, and the commands as written fail with
> `Index of insertion too big`.
>
> [Step 4b](GO_LIVE.md#step-4--open-the-two-firewalls) covers both now, and checks `nft` and
> `ufw` as well, because `iptables -L` only shows what iptables manages. The consequence worth
> carrying forward: on an image with no host firewall at all, **`BIND_ADDR` on loopback and the
> loopback binding of SearXNG are the only things keeping the API and the search engine off the
> internet.** That was defence in depth when it was written and is now the depth.

### The schema applies itself [step 6](GO_LIVE.md#step-6--create-the-database)

Every Postgres-backed role runs migrations on boot before it serves anything, so there is no
separate migration step to forget and no window where the binary and the schema disagree.
`landscape migrate` does exactly that and exits, if you want to watch it happen on its own.

### The artefacts stay root-owned [step 7](GO_LIVE.md#step-7--build-the-application)

The services read these files and never write them, so nothing needs to be handed over. The
first version of this document chowned the lot to the service user, and review pointed out what
that buys an attacker: a compromised API or worker could replace the binary it runs from and
survive every restart afterwards. The units carry no writable path either, so `ProtectSystem=strict`
leaves the whole filesystem read-only to them.

### Both inference artefacts are pinned [step 8](GO_LIVE.md#step-8--build-the-model-server-and-get-the-model)

An inference build taken from a moving branch means the same application commit can be deployed
twice and behave differently — and every quality number this project has would be about a build
nobody can reproduce. So `llama.cpp` is pinned to a revision, and the model to a Hugging Face
revision with a checksum.

The model is **Qwen3-4B Q4_K_M**, which is the one the golden set is scored against
([MODEL_BAKEOFF.md](MODEL_BAKEOFF.md), [ADR 0007](decisions/0007-model-licences.md)). The 1.7B is
a router, not an extractor: it invented a price on a contact-sales page, which is the one failure
this product cannot ship.

**If the checksum disagrees, stop.** A model that is not the one the golden set scored makes
every quality figure on this project inapplicable to what is running.

### The search engine is a step, not a default [step 10](GO_LIVE.md#step-10--start-the-search-engine)

`SEARX_URL` has **no default, deliberately**: a fallback to somebody's public instance would send
everything strangers type into your box to a third party.

Without it the site still works — a company's website produces a full report — and a *description*
cannot become a set of companies. The application says which of those it is rather than guessing,
and [USING_THE_SITE.md](USING_THE_SITE.md) part 9 is how to see each sentence.

SearXNG serves HTML and answers `403` to `format=json` until an instance opts in, which is why
`deploy/searxng/settings.yml` is checked in. What a first run says when it goes wrong is built
and tested — [BENCHMARKS.md](BENCHMARKS.md) Run 42 — against stand-ins rather than against a real
engine, because no engine has ever answered this application a query.

### DNS before Caddy [step 3](GO_LIVE.md#step-3--point-your-domain-at-it-now) and [step 11](GO_LIVE.md#step-11--https)

Caddy asks for a certificate on the name in its config, and the authority resolves that name and
connects to it — so with no record there is nothing to validate and no certificate. Failed
issuance is rate-limited, so the cheap order is DNS, check it resolves, then Caddy.

Two things in `deploy/Caddyfile.example` are deliberate:

**The `remote_ip` allow-list** is what the firewall could not be. Everybody else gets a 403,
which matters more than it sounds: a new certificate appears in public certificate-transparency
logs within minutes of being issued, and scanners read those logs.

**`flush_interval -1`** stops the event stream being buffered. Without it a report that fills in
over ninety seconds arrives all at once at the end — the feature, undone by the thing in front
of it.

> **⚠ The ACME challenge and the allow-list.** Caddy answers the validation request itself, ahead
> of the routes, so the 403 should not block issuance. **This is the other step I cannot verify
> from here.** If the first `journalctl -u caddy` shows a challenge failing, widen the matcher,
> let the certificate issue, then narrow it again — and confirm a renewal in the log before
> trusting it, because the failure mode is silent and sixty days away.

**Without a domain there is no certificate**, and the prompts somebody types cross the internet
in clear text. Fine for an afternoon of your own testing; not something to send anybody. One
feature also degrades: browsers refuse the clipboard outside a secure context, so **Copy as
context** puts the document in a text box instead. That path is built and tested rather than
assumed — it was the first thing a real browser did.

### Updating, and rolling back ([keeping it running](GO_LIVE.md#updating-to-a-newer-commit))

**The unit files are part of the deploy.** The first draft of the update recipe copied only the
binary, so a change to a memory cap or a startup gate would have sat in the repository for months
while the box ran the version from the first install.

**Stopping the worker mid-analysis is safe.** The row is left `running`, the staleness sweep
returns it to the queue, and another worker starts it over from nothing — that whole path is what
[BENCHMARKS.md](BENCHMARKS.md) Runs 18–20 were about. The reader sees it restart rather than sees
a lie.

**To roll back**, keep the previous binary beside the new one and put it back. There is nothing
else to undo: the schema only moves forwards, and no migration so far drops anything.

---

## The measurement this deployment exists for

[ADR 0011](decisions/0011-no-experiments-on-production.md) is why this is a browser and a
stopwatch rather than a profiler on the box: a benchmark leaves a toolchain, a downloaded model
and a set of assumptions behind in the thing customers depend on.

Open the site, pick **project management for a small design agency**, and note two things:

| | Target ([PRODUCT_SPEC.md](PRODUCT_SPEC.md) §2.1A) | Laptop, measured | Yours |
|---|---|---|---|
| First content on screen | 20–40s | 23s | ? |
| Whole report | 90–180s | ~4 min for two companies | ? |

First content should be close to the laptop figure — it is a fetch and a parse, and the model is
not involved. **The gap will be in the total**, and that number is the thing this deployment
exists to find out.

If it is bad, the levers are in [BENCHMARKS.md](BENCHMARKS.md) Run 23 and none of them is a
faster model: fewer pages, a shorter read order, and — the one the laptop already points at —
discovery, which is about 20 of those 23 seconds.

---

## When it does not work

[GO_LIVE.md](GO_LIVE.md) ends with the short table. [RUNBOOK.md](RUNBOOK.md) §6 is the long one:
a service that will not start, a page that never arrives, a queue that never moves.

The one worth memorising: **every section empty but the changelog filled means the model server
is down.** Changes is the section that needs no model, so it is what tells an inference problem
apart from a fetching one.
