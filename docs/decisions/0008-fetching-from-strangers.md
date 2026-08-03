# ADR 0008 — Fetching URLs that strangers choose

**Status:** accepted
**Date:** 2026-08-03

## Context

`landscape-fetch` is the first code in this project that opens a socket to an address
somebody else picked. That changes the threat model rather than extending it: until now the
worst input was a badly-formed prompt.

**The product's purpose is the vulnerability.** We exist to fetch pages about companies a
reader names. A service that fetches arbitrary URLs from inside its own network is the
textbook SSRF target, and on the intended host — an Oracle A1 — the most valuable thing
inside that network answers at `169.254.169.254` and hands out credentials to anything that
asks.

Alongside that, [FACT_CHECKING.md](../FACT_CHECKING.md) commits us to honouring `robots.txt`
as an **ethical position, not a risk-management one**. That framing changes what "correct"
means for ambiguous cases, and it is the reason several decisions below look
over-cautious.

## Decision

Six rules, each with a test that fails if it is removed.

### 1. The guard returns the address, not a boolean

The obvious implementation — resolve, check the IP, hand the URL to an HTTP client — is
broken by **DNS rebinding**: the client resolves a second time when it connects, and an
attacker controlling the authoritative nameserver answers differently. Public for the check,
`127.0.0.1` for the connection.

`Verdict` therefore carries the `IpAddr` that was checked, and `reqwest`'s
`resolve_to_addrs` pins the connection to it. **A guard that returns only a boolean cannot
be used safely**, so the type makes the safe use the easy one.

### 2. Every address a host resolves to is checked

Not the first. A hostname with one public `A` record and one pointing at loopback is a
deliberate attack, and a guard that checks whichever the resolver returned first is a coin
flip that passes in testing.

### 3. Redirects are followed by hand, and every hop re-enters every check

Automatic following checks the URL you gave it and connects to the one at the end of the
chain. A page that passes and then redirects to the metadata endpoint has defeated a guard
that ran once. Five hops, then we conclude we are being led somewhere.

### 4. Deny by default, in ranges

Loopback, private, link-local, carrier-grade NAT, unspecified, multicast, documentation,
reserved, and **any IPv6 address embedding an IPv4 one** — including `::ffff:8.8.8.8`, whose
wrapped address is perfectly public. Too many stacks and proxies disagree about how to treat
those for the exception to be worth its risk.

`0.0.0.0` is refused specifically because Linux treats it as localhost, which makes it a
loopback bypass that does not look like one.

This refuses a handful of legitimate addresses. That is the correct direction to be wrong
in: a refused fetch shows a reader a gap, which this product is built to display honestly,
and a permitted one can exfiltrate a credential.

### 5. `robots.txt` is parsed here, and the parser is deliberately strict

Adding a dependency is an architecture change under `CODING_QUALITY.md` §3.1, and the case
for one is weaker than it looks. RFC 9309's matching rules are small — group selection,
longest match wins, `*`, `$` — and a third-party parser carries the same risk as ours minus
the ability to make it conservative *on purpose*.

The status handling is where the ethical framing shows:

| Status | Verdict | Why |
|---|---|---|
| 404, 410 | **Allowed** | No rules exist; the site has not asked for anything |
| 401, 403 | **Disallowed** | If we may not read the rules, we may not decide we pass them |
| 429, 5xx | **Disallowed** | The polite reading of "I am struggling" is not "carry on" |

The 404-vs-5xx distinction is the one most often got wrong, and getting it wrong turns a
site's bad afternoon into our crawling it against its wishes.

`Crawl-delay` is honoured though it is not in the RFC, because ignoring a site's stated
wishes on a technicality is not this product's posture. It is bounded at 300 seconds: a site
asking for a day is asking us not to crawl it, and an unbounded sleep is a hang wearing a
politeness costume.

### 6. There is no `--ignore-robots`, and there will not be

A flag that exists gets used, at 2am, by someone in a hurry with a good reason. The
commitment is part of the product rather than a setting on it.

## Consequences

**We will fetch fewer pages than a less careful crawler.** Some sites disallow us, some
addresses are refused that would have worked. Both surface as gaps, which the report already
renders honestly — that treatment was built in Phase 0 partly so this would be cheap.

**The rate limit is per-process.** `Fetcher` holds the pacer, so two processes fetching the
same host do not coordinate. Correct for the current design (one worker) and **wrong the
moment there are two**, at which point it belongs in Postgres or Redis. Recorded here rather
than discovered then.

**A client is built per request** because `resolve_to_addrs` is a builder-level override.
Wasteful, and the waste is bounded by our own rate limit — one request per host per second
is not a connection-pool problem. Revisit only if a profile says so.

**`guard.rs` is held at 100% coverage** with no exemption path, per `CODING_QUALITY.md`
§6.2. It is written pure — address in, verdict out — which is what makes that reachable
without a network, and is the main reason the module is shaped the way it is.

## What this does not cover

**Nothing yet limits total fetches per analysis.** A subject resolving to a site with a
thousand linked pages is currently bounded only by whatever calls this. That belongs with
the orchestrator, and it is the next thing to get wrong.

**No conditional GET yet.** `Page` carries `etag` and `last_modified` so the caller can
store them, but nothing sends `If-None-Match` because nothing yet re-fetches. It goes in
with the cache.

**Nothing here defends against a slow-drip response** that stays under the size cap for
ever. The 20-second overall timeout bounds it, which is adequate now and not obviously
adequate at higher concurrency.
