# Landscape — Deploying to Oracle Cloud

> **Every step below is written and none has been run.** There is no box reachable from this
> repository ([PROJECT_STATUS.md](../PROJECT_STATUS.md) B5), so this procedure is reasoned from
> the code and from Oracle's documented behaviour rather than from a deployment that worked.
> **Correct it as you go** — the first person through is the one who finds out where it is
> wrong, and a procedure nobody has walked is the deployment equivalent of a backup nobody has
> restored ([RUNBOOK.md](RUNBOOK.md) §3.1 makes the same admission about the restore drill).
>
> Where a step is a guess rather than a reading of our own code, it says so — the ACME challenge
> and the allow-list in step 8, and the iptables rule numbering in step 4, are the two I would
> check first.

**Deploying changes nothing but secrets.** Everything environmental is already an environment
variable — `BIND_ADDR`, `DATABASE_URL`, `LLAMA_URL`, `WEB_DIR` — and nothing in the code branches
on where it is running. If a step below turns out to need a code change, that is a defect worth
fixing rather than a step worth documenting.

---

## Before you start: two things that will bite

### The URL has no rate limit on it

**D6 is not built.** There is no per-IP cap, no accounts, and no anonymous quota. Every request
to `POST /api/analyses` starts a run that occupies the model for minutes on a machine with four
cores. A public URL is an open invitation to spend all of it.

Until D6 exists, the mitigation is **step 8**: a `remote_ip` allow-list in Caddy, so the
application answers one address and refuses the rest. It is one line, and it is the difference
between a demo and a box you have to rebuild.

It is deliberately *not* in the firewall, and the reason is worth knowing before you start: the
certificate authority has to reach this host, so ports 80 and 443 cannot be restricted to you.
Review found that contradiction in the first version of this document, where the two halves made
each other impossible.

### The wait is four minutes, and this box is slower than a laptop

An example idea names two companies at about two minutes each *on a developer laptop*
([BENCHMARKS.md](BENCHMARKS.md) Runs 21–23). Four ARM cores are not four laptop cores, so treat
the laptop numbers as a lower bound. First content should still be quick — the changelog needs no
model at all — but the whole report will take longer than it does at home, and finding out by how
much is one of the two reasons to deploy at all.

The other is that [ADR 0011](decisions/0011-no-experiments-on-production.md) says the end-to-end
figure can only be taken **from the client's side of a deployment**. Step 9 is that measurement.

---

## 1. The instance

Oracle's Always Free tier gives 4 OCPU and 24 GB of Ampere A1 (aarch64) across your instances.
Take it as **one** instance rather than four small ones: the model wants the memory and the
extraction wants the cores, and splitting them helps nothing here.

| | |
|---|---|
| Shape | `VM.Standard.A1.Flex` — 4 OCPU, 24 GB |
| Image | Ubuntu 22.04 or 24.04 (aarch64) |
| Boot volume | 50 GB is comfortable; the model is ~2.5 GB and Rust's build directory is large |

Add your SSH public key when you create it. Then:

```bash
ssh ubuntu@YOUR_INSTANCE_IP
```

> **If the free-tier capacity error appears** — *"Out of host capacity"* — it is a real and
> common Oracle condition, not a mistake in your request. It usually clears; some regions clear
> faster than others.

## 2. Packages

```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake git curl pkg-config libssl-dev postgresql
```

Rust, for building the binary on the box:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Node, for building the web app:

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs
```

## 3. Postgres

```bash
sudo -u postgres psql <<'SQL'
CREATE USER landscape WITH PASSWORD 'pick-a-real-one';
CREATE DATABASE landscape OWNER landscape;
SQL
```

Nothing else: **the schema applies itself.** Every Postgres-backed role runs migrations on boot
before it serves anything, so there is no separate migration step to forget. If you want to see
it happen on its own, `landscape migrate` does exactly that and exits.

## 4. The firewall, which is two firewalls

This is the step that wastes an afternoon, because closed ports look identical to a broken
application.

**a. The VCN security list**, in the Oracle console — Networking → Virtual Cloud Networks → your
VCN → Security Lists → Default. Add ingress rules:

| Source | Protocol | Port |
|---|---|---|
| `0.0.0.0/0` | TCP | 80 |
| `0.0.0.0/0` | TCP | 443 |

**Open to the internet, and the allow-list moves up a layer.** The first version of this
document restricted both ports to one address, and review pointed out that this made the
procedure impossible: Let's Encrypt validates the domain by reaching this host, and it does not
come from your address. A certificate could never have been issued — and if one somehow had,
renewal would have failed sixty days later, quietly.

So the ports are open, the certificate works, and **Caddy is what refuses everybody but you** —
step 8. That is a weaker boundary than a firewall and it is the one that can actually exist: it
is one process rather than the whole box, and a refused request never reaches the application or
the model.

**b. The instance's own iptables.** Oracle's Ubuntu images ship with rules that accept SSH and
drop the rest, and they survive reboots through `netfilter-persistent`. A security list rule
alone will not get you in.

```bash
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo netfilter-persistent save
```

> **Check the rule numbering first** with `sudo iptables -L INPUT --line-numbers`. Inserting
> after the final `REJECT` does nothing, which is the failure that looks most like success.
> *(This is the step I am least able to verify from here — the exact ruleset varies by image.)*

**Nothing needs opening for 8787 or 8080.** The API binds to `127.0.0.1` and Caddy reaches it
locally; the model server never leaves the machine.

## 5. Build

On the box, in a checkout:

```bash
git clone https://github.com/larry94555/competitive-landscape.git
cd competitive-landscape
./scripts/build-release.sh
```

About ten minutes on four cores. It produces `dist/` — the binary, the built web app, and the
commit it came from.

```bash
sudo useradd --system --home /opt/landscape --shell /usr/sbin/nologin landscape || true
sudo mkdir -p /opt/landscape/{bin,web,models}
sudo cp dist/bin/landscape /opt/landscape/bin/
sudo cp -r dist/web/dist /opt/landscape/web/dist
sudo cp dist/MANIFEST /opt/landscape/
sudo chown -R root:root /opt/landscape
sudo chmod -R go-w /opt/landscape
```

**Leave it root-owned.** The services read these files and never write them, so nothing needs to
be handed over. The first version of this document chowned the lot to the service user, and
review pointed out what that buys an attacker: a compromised API or worker could replace the
binary it runs from and survive every restart afterwards. The units carry no writable path
either, so `ProtectSystem=strict` leaves the whole filesystem read-only to them.

## 6. The model

`llama-server` has to be built for aarch64; there is no package for it.

**Pinned, both of them.** An inference artefact taken from a moving branch means the same
application commit can be deployed twice and behave differently — and every quality number this
project has would be about a build nobody can reproduce.

```bash
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
git checkout b10291
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j4
sudo install -o root -g root -m 0755 build/bin/llama-server /opt/landscape/bin/
```

Then the model — **Qwen3-4B Q4_K_M**, which is the one the golden set is scored against
([MODEL_BAKEOFF.md](MODEL_BAKEOFF.md), [ADR 0007](decisions/0007-model-licences.md)). The 1.7B is
a router, not an extractor: it invented a price on a contact-sales page, which is the one failure
this product cannot ship.

The model is pinned to a revision rather than `main`, and checked afterwards:

```bash
sudo curl -L --output /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf \
  https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf

echo '7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5  /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf' | sha256sum --check
sudo chmod 0444 /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf
```

The revision, the file name and the checksum were read from Hugging Face's API rather than
guessed, and the file is 2,497,280,256 bytes. **If `sha256sum` disagrees, stop.** A model that is
not the one the golden set scored makes every quality figure on this project inapplicable to
what is running.

Record what you actually used, beside what the application was built from:

```bash
printf 'llama.cpp b10291\nmodel bc640142c66e1fdd12af0bd68f40445458f3869b\n' | sudo tee -a /opt/landscape/MANIFEST
```

## 7. The services

```bash
sudo mkdir -p /etc/landscape
sudo cp deploy/landscape.env.example /etc/landscape/landscape.env
sudo nano /etc/landscape/landscape.env      # the database password
sudo chmod 600 /etc/landscape/landscape.env
sudo chown landscape:landscape /etc/landscape/landscape.env

sudo cp deploy/*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now llama-server landscape-api landscape-worker
```

Check all three, in the order they depend on each other:

```bash
systemctl status llama-server landscape-api landscape-worker
curl -s http://127.0.0.1:8080/health
curl -s http://127.0.0.1:8787/api/health
```

`/api/health` reads the queue depth **out of the database**, so a healthy answer also proves the
process can reach Postgres. A health check that only proves the process is running would report
healthy while every request failed.

## 8. DNS, then TLS

**The domain has to point here first.** Caddy asks for a certificate on the name in its config,
and the authority resolves that name and connects to it — so with no record there is nothing to
validate and no certificate. This step was missing from the first version of this document.

At your registrar or DNS host:

| Type | Name | Value |
|---|---|---|
| `A` | `landscape` (or `@`) | your instance's public IPv4 |

Check it resolves from your machine — `dig +short landscape.example.com` — **before** starting
Caddy. Failed issuance is rate-limited by Let's Encrypt, so the cheap order is DNS, check, then
Caddy.

```bash
sudo apt-get install -y caddy
sudo cp deploy/Caddyfile.example /etc/caddy/Caddyfile
sudo nano /etc/caddy/Caddyfile        # your domain, and your address in the allow-list
sudo systemctl restart caddy
journalctl -u caddy -f                # watch the certificate arrive
```

Two things in that config are deliberate.

**The `remote_ip` allow-list** is what step 4 moved up here — the application is behind a list of
one, because the ports cannot be. Put the address you will browse from; `curl -s https://ifconfig.me`
tells you what it is. Everybody else gets a 403, which matters more than it sounds: a new
certificate appears in public certificate-transparency logs within minutes of being issued, and
scanners read those logs.

**`flush_interval -1`** stops the event stream being buffered. Without it a report that fills in
over ninety seconds arrives all at once at the end — the feature, undone by the thing in front
of it.

> **The ACME challenge and the allow-list.** Caddy answers the validation request itself, ahead
> of the routes above, so the 403 should not block issuance. **This is the step I am least able
> to verify from here.** If the first `journalctl -u caddy` shows a challenge failing, widen the
> matcher, let the certificate issue, then narrow it again — and confirm a renewal in the log
> before trusting it, because the failure mode is silent and sixty days away.

**Without a domain there is no certificate.** You can point a browser at `http://YOUR_IP:8787`
after changing `BIND_ADDR` to `0.0.0.0:8787` and opening that port — but then the prompts
somebody types and the reports they read cross the internet in clear text, and there is no
allow-list in front of the application at all. Fine for ten minutes of your own testing; not
something to send anybody.

## 9. Now measure it, from here rather than there

[ADR 0011](decisions/0011-no-experiments-on-production.md) is the reason this step is a browser
and a stopwatch rather than a profiler on the box: a benchmark leaves a toolchain, a downloaded
model and a set of assumptions behind in the thing customers depend on.

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

## 10. Updating

```bash
cd competitive-landscape && git pull && ./scripts/build-release.sh
sudo systemctl stop landscape-api landscape-worker
sudo cp dist/bin/landscape /opt/landscape/bin/
sudo rm -rf /opt/landscape/web/dist && sudo cp -r dist/web/dist /opt/landscape/web/dist
sudo cp dist/MANIFEST /opt/landscape/
sudo chown -R root:root /opt/landscape && sudo chmod -R go-w /opt/landscape

# **The unit files are part of the deploy.** The first draft of this recipe copied only the
# binary, so a change to a memory cap or a startup gate would have sat in the repository for
# months while the box ran the version from the first install.
sudo cp deploy/*.service /etc/systemd/system/
sudo systemctl daemon-reload

sudo systemctl start landscape-api landscape-worker
```

If `deploy/llama-server.service` changed, restart that one too. It is left out above because
reloading the model costs minutes and most updates do not touch it:

```bash
sudo systemctl restart llama-server
```

Migrations apply on boot, so there is no separate step and no window where the binary and the
schema disagree.

**Stopping the worker mid-analysis is safe.** The row is left `running`, the staleness sweep
returns it to the queue, and another worker starts it over from nothing — that whole path is
what [BENCHMARKS.md](BENCHMARKS.md) Runs 18–20 were about. The reader sees it restart rather than
sees a lie.

**To roll back**, keep the previous binary beside the new one and put it back. There is nothing
else to undo: the schema only moves forwards, and no migration so far drops anything.

---

## When it does not work

[RUNBOOK.md §6](RUNBOOK.md) covers the failures this procedure produces — a service that will not
start, a page that never arrives, a queue that never moves. The short version:

| Symptom | Look at |
|---|---|
| Connection times out from your machine | Both firewalls. §4, and check the rule numbering |
| `502` from Caddy | `landscape-api` is down, or `BIND_ADDR` is not what Caddy proxies to |
| The page loads, every API call 404s | `WEB_DIR` is right but the API is not running — the fallback is serving `index.html` for `/api/*` |
| Analyses stay `queued` for ever | `landscape-worker` is down, or it cannot reach Postgres |
| Reports come back with every section empty | `llama-server` is not up. The changelog section will still fill, because it needs no model — that is how you tell this apart from a fetching problem |
