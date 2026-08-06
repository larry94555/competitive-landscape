# Landscape — Runbook

> What to do when something is broken, written for the person on call at 23:00, who is also
> the person who wrote it and will not remember.
>
> Every entry has the same shape: **symptom → check → fix**. It grows as things actually go
> wrong; an entry added speculatively is usually wrong in a way nobody discovers until it
> matters.

**Status: early.** Sections marked *(unverified)* describe a system that is not deployed yet.
They are here because writing them at deploy time never happens, but they have not been
rehearsed. **A procedure that has not been rehearsed is a guess.**

---

## 0. First moves, whatever is wrong

```bash
curl -s http://127.0.0.1:8787/api/health
```

`queued` in that response comes from the database, so a healthy answer proves the process is
up **and** can reach storage. No answer at all splits the problem in two immediately.

```bash
journalctl -u landscape-api  -n 100 --no-pager     # (unverified)
journalctl -u landscape-worker -n 100 --no-pager   # (unverified)
```

---

## 1. Nothing is being analysed, but the API answers

**Symptom.** Analyses accepted, `status` stays `queued` forever.

**Check.** Is a worker running, and can it see the same database?

```bash
systemctl status landscape-worker    # (unverified)
psql "$DATABASE_URL" -c "SELECT status, count(*) FROM analyses GROUP BY status"
```

**Fix.**
- No worker process → start it.
- Worker running, queue growing → it is stuck on one analysis. Find it:
  `SELECT id, started_at FROM analyses WHERE status='running' ORDER BY started_at`.
  Anything running for more than ~20 minutes is wedged; the model call is the usual reason.
- **In development, this is nearly always the wrong command.** `cargo run -p landscape -- serve` with
  `--store memory` gives the API its own in-process store, so a separate worker never sees
  its queue. Use `cargo run -p landscape -- dev`.

**A worker that died mid-analysis is handled.** The worker sweeps once a minute and returns
anything `running` for more than 20 minutes to the queue — `reclaim_stale` in
`landscape-db`. The threshold is deliberately well past a healthy run: reclaiming early
would hand a second worker a job the first is still doing.

So a row stuck at `running` for **under** 20 minutes is a slow analysis, and one stuck for
longer means the sweep is not running — check the worker is alive at all.

---

## 2. llama-server — the three failure modes

The model is a separate process. It fails in ways the application cannot fix, so recognising
which one is happening matters more than any single command.

### 2.1 It is not listening

**Symptom.** `LlmError::Unreachable`. Extraction fails; fetching still works.

**Check.** `curl -s http://127.0.0.1:8080/health` → `{"status":"ok"}`.

**Fix.** Restart the unit. If it exits immediately, it is 2.2.

### 2.2 Out of memory at load

**Symptom.** Process dies during startup, or is killed shortly after. `dmesg` shows an OOM
kill.

**Cause.** Three resident models share ~17 GB of a 24 GB box. Anything that grows — a larger
context, a bigger KV cache, a second copy started before the first exited — pushes it over.

**Fix.**
- Confirm only one process per model: `pgrep -a llama-server`.
- Check `--ctx-size` against what the units specify. A context raised for one experiment and
  never lowered is the usual cause.
- `MemoryMax=` on each unit turns a machine-wide OOM into one service failing, which is far
  easier to diagnose. Verify it is set. *(unverified — not deployed)*

**Swap must stay off.** A model that swaps does not fail, it goes a hundred times slower,
which presents as "the site is down" while every health check passes.

### 2.3 Answering, and wrong

**Symptom.** Everything is green. Health checks pass, nothing fails to parse, latency is
normal or better — and the reports contain nonsense, or prices that are not on the pages
they cite.

**This is the failure mode that looks like health.** It has happened once already: a
defective quantisation of Qwen3-1.7B was the *fastest* model on the bench and returned
schema-valid garbage. Constrained decoding guarantees the shape of an answer and nothing
about its truth, so every monitor built around shape reports fine.

**Check.** Score the model against the golden set:

```bash
LLAMA_URL=http://127.0.0.1:8080 \
  cargo test -p landscape-golden --test against_a_model -- --ignored --nocapture
```

Compare against [BENCHMARKS.md](BENCHMARKS.md) Run 3. Qwen3-4B scored 90% of fields with 0
invented prices; anything near 10% is a broken model or quantization, not a bad day.

**Fix.**
- Confirm the loaded file is the one intended: `curl -s http://127.0.0.1:8080/v1/models`.
  A model swapped by a redownload or a changed symlink is the usual cause.
- Prefer an official quantization over a third-party re-quantization. The `unsloth` Q4_K_M
  build failed where the official Q8_0 of the same model was flawless.
- Roll back to the previous model file before debugging further. A wrong answer reaches a
  reader; a stopped service does not.

**Do not judge a model swap on latency alone.** That is exactly the mistake this section
exists to prevent, and the golden set is the only instrument here that would have caught it.

### 2.4 Answering, but far too slowly

**Symptom.** Requests complete but analyses take many times the expected wall clock.

**Cause.** Prefill is the binding constraint on 4 ARM cores — not generation. A long prompt
costs more than a long answer. The usual trigger is a change that widened the span window or
stopped truncating a page.

**Check.** `tokens_predicted` and timing fields in the llama-server response, against the
figures in `docs/BENCHMARKS.md`.

**Fix.** Reduce input, not output: tighter span windows, fewer sources per pass. Cutting
`n_predict` addresses the smaller half of the cost.

---

## 3. Database

### 3.1 Restore from backup *(unverified — the drill has not been run)*

```bash
systemctl stop landscape-api landscape-worker
createdb landscape_restore
pg_restore -d landscape_restore /path/to/dump
# verify BEFORE swapping
psql -d landscape_restore -c "SELECT count(*), max(created_at) FROM analyses"
```

> **A backup that has never been restored is not a backup.** `ROADMAP.md` schedules a restore
> drill; until it has been done and this section updated to remove the *(unverified)* marker,
> assume this procedure is wrong somewhere.

### 3.2 Migrations will not apply

Migrations run on boot and are skipped if already applied. A failure means the database is in
a state the binary does not expect — usually a rollback to an older version after a migration
ran. **Do not hand-edit `_sqlx_migrations`.** Restore, then roll forward.

---

## 4. Safe mode

When the machine cannot serve analyses but the site should not disappear: serve existing
reports, refuse new ones, and say so plainly.

> **Not yet implemented.** `PRODUCT_SPEC.md` §2A.4 specifies the behaviour; there is no flag
> for it yet. Until there is, the honest options are a maintenance page or nothing.

---

## 5. Local development

| Symptom | Cause | Fix |
|---|---|---|
| `could not bind 127.0.0.1:8787` | Something else on the port | The error prints the command to find it |
| Analyses stay `queued` | `serve` and `worker` as separate processes with `--store memory` | `cargo run -p landscape -- dev` |
| `DATABASE_URL is not set` | No `.env` | `cp .env.example .env`, or add `--store memory` |
| `docker compose` hangs, no error | Docker Desktop's privileged service is stopped | `Set-Service com.docker.service -StartupType Automatic; Start-Service com.docker.service` as Administrator — or use the WSL Postgres path in `README.md` |
| Postgres tests fail on claim order | Two tests sharing a schema | Each test creates its own; if a run was killed, stale `test_*` schemas may remain |

---

## 6. Deployment

The procedure is [DEPLOY.md](DEPLOY.md). This section is what to do when following it does not
work — and **the first three are all the same mistake**, which is that a closed port and a
broken application look identical from a browser.

| Symptom | Cause | Fix |
|---|---|---|
| Connection times out from your machine | The VCN security list has no ingress rule | Console, Networking, Security Lists. DEPLOY.md §4a |
| Still times out with the rule in place | The instance's own iptables. Oracle's Ubuntu images drop everything but SSH and persist it | `sudo iptables -L INPUT --line-numbers` and insert **before** the final REJECT, then `netfilter-persistent save` |
| Times out only on 443 | Caddy could not get a certificate, so nothing is listening | `journalctl -u caddy` — usually port 80 is closed, which Let's Encrypt needs |
| `502 Bad Gateway` | `landscape-api` is down, or `BIND_ADDR` disagrees with the Caddyfile | `systemctl status landscape-api`; they must both say `127.0.0.1:8787` |
| `landscape-api` exits immediately | `DATABASE_URL` wrong, or Postgres not up. It applies migrations on boot and refuses to serve if it cannot | `journalctl -u landscape-api -n 50`. The message names which |
| Analyses stay `queued` | `landscape-worker` is down or cannot reach Postgres | `systemctl status landscape-worker`. Both processes need the same `DATABASE_URL` |
| Every section empty, but the report renders | `llama-server` is not up | **The changelog section will still fill**, because it needs no model. That is how you tell this apart from a fetching problem |
| `llama-server` restart-loops on boot | Four gigabytes off a free-tier volume is slow, and the unit gave up | `TimeoutStartSec` is 300s in the shipped unit; raise it rather than assume a crash |
| Sections all arrive at once at the end | A proxy is buffering the event stream | `flush_interval -1` in the Caddyfile. Without it the streaming feature is undone by the thing in front of it |
| `403 Not open yet.` from your own browser | Your address is not the one in the Caddy allow-list, or it changed | `curl -s https://ifconfig.me`, then the `@allowed remote_ip` line. A home connection's address moves |
| The certificate never issues | DNS does not point here yet, or port 80 is closed | `dig +short <name>` first; issuance is rate-limited, so check before retrying |
| The certificate issued once and expired | Renewal failed silently for sixty days | `journalctl -u caddy --since '70 days ago' \| grep -i renew`. This is why step 8 says to confirm a renewal rather than trust the first success |
| A service was killed and the journal says `oom` | `MemoryMax=` did its job | One service failing instead of the machine. Raise the cap on that unit if the journal shows it repeatedly |
| The deploy did not take | The unit files were not reinstalled | DEPLOY.md step 10 copies `deploy/*.service` and reloads. A binary-only update leaves the old unit in place |

### 6.1 A run that will not die

Stopping the worker mid-analysis is safe and is the ordinary way to deploy: the row is left
`running`, the staleness sweep returns it to the queue, and a replacement starts it from nothing
rather than finishing over it. If a row is stuck `running` with no worker alive, the sweep has
not reached it yet — it is time-based, not a signal.
