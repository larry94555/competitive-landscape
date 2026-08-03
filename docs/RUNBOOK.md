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
- **In development, this is nearly always the wrong command.** `cargo run -- serve` with
  `--store memory` gives the API its own in-process store, so a separate worker never sees
  its queue. Use `cargo run -- dev`.

> **Not yet implemented:** nothing currently reclaims an analysis whose worker died. The row
> stays `running` forever. A `started_at` timeout that returns it to `queued` is needed
> before the first deploy — it is not written yet, and this is the note that says so.

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

### 2.3 Answering, but far too slowly

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
| Analyses stay `queued` | `serve` and `worker` as separate processes with `--store memory` | `cargo run -- dev` |
| `DATABASE_URL is not set` | No `.env` | `cp .env.example .env`, or add `--store memory` |
| `docker compose` hangs, no error | Docker Desktop's privileged service is stopped | `Set-Service com.docker.service -StartupType Automatic; Start-Service com.docker.service` as Administrator — or use the WSL Postgres path in `README.md` |
| Postgres tests fail on claim order | Two tests sharing a schema | Each test creates its own; if a run was killed, stale `test_*` schemas may remain |
