# Landscape — from an empty Oracle account to a working site

**Follow this top to bottom and the last step ends with a URL you can use.** Every command is
written out; every value you have to choose is named where you choose it, with what to put.
Nothing here needs a search engine, a forum, or a decision you have not been given the
information to make.

> **What this is not.** [DEPLOY.md](DEPLOY.md) is the same deployment with the reasoning — why
> the ports are open, why the artefacts stay root-owned, what each unit file is protecting
> against. Read it when something surprises you. This file is the sequence.
>
> **Being walked for the first time.** It was written from our own code and from Oracle's
> documented behaviour rather than from a deployment that worked, so **correct it as you go**.
> Two steps are still marked **⚠ least certain**; a third is not, because
> [step 4b](#step-4--open-the-two-firewalls) has now met a real box and been rewritten around
> what it found.

## What you will end up with

| | |
|---|---|
| One Oracle Always Free ARM instance | 4 OCPU, 24 GB, Ubuntu |
| Five things running on it | Postgres, the model server, the API, the worker, SearXNG |
| Reachable at | `https://` your own domain |
| Cost | £0/month for the instance. The domain is the only recurring cost |

## What you need before you start

| | |
|---|---|
| An Oracle Cloud account | Free tier is enough. Sign-up needs a card for identity; the Always Free shapes are not charged |
| A terminal | macOS or Linux: the built-in one. Windows: PowerShell, which has `ssh` built in |
| About 90 minutes | Most of it waiting: ~10 minutes building the app, ~25 building the model server, ~5 downloading the model |
| Your domain name | And the ability to add a DNS record at whoever you registered it with. If you do not have one, [Appendix A](#appendix-a--running-without-a-domain) is the version without |

---

## Step 1 — Make an SSH key

**On your own machine**, not on Oracle.

**macOS or Linux**, in a terminal:

```bash
mkdir -p ~/.ssh && chmod 700 ~/.ssh
ssh-keygen -t ed25519 -C "landscape" -f ~/.ssh/id_ed25519 -N ""
cat ~/.ssh/id_ed25519.pub
```

**Windows**, in PowerShell — `~` is not expanded when it is handed to a program rather than
to PowerShell itself, so the path is written out:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.ssh" | Out-Null
ssh-keygen -t ed25519 -C "landscape" -f "$env:USERPROFILE\.ssh\id_ed25519" -N '""'
Get-Content "$env:USERPROFILE\.ssh\id_ed25519.pub"
```

> **The first line is not optional on a machine that has never used SSH.** `ssh-keygen` writes
> the key to the path you give it and does not create the directory above it: without `.ssh`
> it exits with *"No such file or directory"*, which reads like a missing program rather than a
> missing folder.

Either way the last command prints one line beginning `ssh-ed25519`. **Copy the whole line** —
you paste it into Oracle in the next step.

> If the key already exists, `ssh-keygen` asks whether to overwrite it. Answer `n` and just run
> the second command.

> **Everything from [step 5](#step-5--install-what-the-build-needs) onwards runs on the box**,
> over SSH, where the shell is bash whatever you connected from.

---

## Step 2 — Create the instance

In the Oracle Cloud console:

1. Menu → **Compute** → **Instances** → **Create instance**.
2. **Name**: `landscape`.
3. **Image and shape** → **Edit**:
   - **Change image** → **Canonical Ubuntu** → **24.04**. Confirm.
   - **Change shape** → **Ampere** → **VM.Standard.A1.Flex** → set **4 OCPUs** and **24 GB**
     memory. Confirm.
4. **Networking**: leave the defaults. It creates a VCN and gives the instance a public IPv4
   address, which is what you want.
5. **Add SSH keys** → **Paste public keys** → paste the line from [step 1](#step-1--make-an-ssh-key).
6. **Boot volume** → tick **Specify a custom boot volume size** → **50** GB.
7. **Create**.

Wait for the state to go from **PROVISIONING** to **RUNNING**, then copy the **Public IP
address** from the instance page. Everything below calls it `YOUR_IP`.

> **If you see "Out of host capacity"** — that is Oracle being full in your region, not a
> mistake in your request. It clears; try again later, or create the instance in a different
> availability domain from the same page.

Connect:

```bash
ssh ubuntu@YOUR_IP
```

Answer `yes` to the fingerprint question.

---

## Step 3 — Point your domain at it, now

**Do this before the long steps, not after them.** DNS takes minutes to hours to propagate, and
doing it here means it is ready by the time you need it in [step 11](#step-11--https) rather than being the
you wait on at the end.

At whoever you registered the domain with, find **DNS**, **DNS records**, or **Manage DNS**, and
add one record:

| Type | Name | Value | TTL |
|---|---|---|---|
| `A` | `@` for the bare domain, or `landscape` for `landscape.your-domain` | `YOUR_IP` | leave the default |

**Delete or edit any existing `A` record with the same name**, including the parking page most
registrars add. Two `A` records for one name send visitors to both.

Then check from **your own machine** — this may take a while, so carry on with [step 4](#step-4--open-the-two-firewalls) and come
back to it:

```bash
nslookup landscape.your-domain
```

It must eventually answer with `YOUR_IP`. **Do not start [step 11](#step-11--https) until it does**: certificate
requests are rate-limited, and asking for one before DNS is ready spends a retry for nothing.

From here on, `YOUR_DOMAIN` means whatever you just pointed at the box — `landscape.example.com`
or `example.com`, whichever you chose.

---

## Step 4 — Open the two firewalls

There are two, and a closed port looks exactly like a broken application.

**a. Oracle's, in the console.** Networking → **Virtual Cloud Networks** → your VCN →
**Security Lists** → **Default Security List** → **Add Ingress Rules**. Add two, one at a time:

| Source CIDR | IP Protocol | Destination Port Range |
|---|---|---|
| `0.0.0.0/0` | TCP | `80` |
| `0.0.0.0/0` | TCP | `443` |

Leave everything else on those rules as it comes.

**b. Ubuntu's, on the box** — **and it may not exist.** Which of these you have depends on the
image, so look before you type:

```bash
sudo iptables -L INPUT --line-numbers
```

Read the first line for the **policy**, and the rows under it for the **rules**:

```text
Chain INPUT (policy ACCEPT)
num  target     prot opt source               destination
```

That is an empty chain — no rules, default allow — and it is what the first box this guide
was walked on had. **Nothing on the machine is blocking anything**, so there is nothing to add.
Skip to *"Whichever you have"* below.

If instead there are numbered rows ending in a `REJECT` or `DROP`, that is the older Oracle
ruleset: SSH allowed, everything else refused. Add yours **above** the refusal — `iptables`
stops at the first rule that matches, so anything below a `REJECT` is never reached. If the
`REJECT` is line 6, these are right as written; if it is a different number, use that number in
both:

```bash
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo netfilter-persistent save
```

Then run `sudo iptables -L INPUT --line-numbers` again and check both `ACCEPT` lines are above
the `REJECT`. **Inserting below it does nothing at all, and that failure looks exactly like
success.**

> **On an empty chain those commands fail** with `Index of insertion too big`: position 6 does
> not exist when there are no rules. That error means you are in the first case, not that
> something is wrong.

**Whichever you have, check the other two places a rule can hide.** `iptables -L` shows only
what iptables manages, and Ubuntu has two other front ends to the same kernel:

```bash
sudo nft list ruleset
sudo ufw status
```

| What you get | What to do |
|---|---|
| `nft` prints nothing, or only chains that say `policy accept` with no rules | Nothing. This is the ordinary case |
| `nft` shows tables named `ip nat`, `ip filter` or `ip raw` full of rules mentioning `br-…`, `docker0` or `172.x.x.x` addresses | **Nothing.** Those are Docker's own, and they are protective rather than restrictive — see below |
| `nft` shows an **`input`** chain containing `drop` or `reject` that is not about a container address | The rules are real but not where `iptables -L` looks. Add yours with `sudo nft add rule inet filter input tcp dport {80, 443} accept`, then `sudo nft list ruleset` to confirm they sit above the drop |
| `ufw` says `Status: inactive`, or the command is not installed | Nothing |
| `ufw` says `Status: active` | `sudo ufw allow 80/tcp && sudo ufw allow 443/tcp` |

> **Docker's rules look alarming and are on your side.** Anything with Docker installed shows a
> page of them, typically in `ip nat` and `ip raw`, and the ones that say `drop` read like a
> firewall. They are not. Two shapes are common:
>
> ```text
> iifname != "br-55b8…" ip daddr 172.18.0.3 counter drop
> iifname != "lo" meta l4proto tcp ip daddr 127.0.0.1 tcp dport 5432 counter drop
> ```
>
> The first stops anything reaching a container by routing straight to its private address
> instead of coming through the bridge. The second makes a port published to `127.0.0.1`
> genuinely loopback-only — the same property [step 10](#step-10--start-the-search-engine) has
> you check for SearXNG. Neither mentions 80 or 443, both sit in `prerouting` rather than in an
> `input` filter, and **neither is yours to change.**
>
> What they *do* tell you is that containers are already running here. If one of them publishes
> **5432**, it will collide with the Postgres this guide installs —
> [step 5](#step-5--install-what-the-build-needs) says how to find out.

**If everything is empty, Oracle's security list from 4a is your only firewall.** That is
genuinely enough for what this guide does — it is the boundary that decides what reaches the
machine at all — but it makes one thing matter more than it otherwise would:

> **With an empty chain and `policy ACCEPT`, anything bound to `0.0.0.0` is on the internet the
> moment a security-list rule allows its port.** `BIND_ADDR` staying on `127.0.0.1`
> ([step 9](#step-9--configure-and-start-the-three-services)) and SearXNG being bound to
> loopback ([step 10](#step-10--start-the-search-engine)) are then the *only* things keeping the
> API and the search engine off the public internet. Do not change either to `0.0.0.0` without
> reading [Appendix A](#appendix-a--running-without-a-domain), which is the one place this guide
> asks you to.

Nothing needs opening for 8787, 8080 or 8888: all three listen on `127.0.0.1` only, and
[step 10](#step-10--start-the-search-engine) checks that rather than taking it on trust.

---

## Step 5 — Install what the build needs

```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake git curl pkg-config libssl-dev postgresql
```

Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Node:

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs
```

Check all three answer:

```bash
rustc --version && node --version && psql --version
```

**And check the database *server* is running, which is a different question.** `psql --version`
is the client program; the next step needs the server behind it:

```bash
systemctl is-active postgresql
sudo -u postgres psql -c 'SELECT version();'
```

You want `active`, then a line naming a PostgreSQL version. Installing the package normally
starts it, so this usually passes — but when it does not, [step 6](#step-6--create-the-database)
fails with `could not connect to server`, which reads like a missing database rather than a
stopped service.

| If | Then |
|---|---|
| `systemctl is-active postgresql` says `inactive` or `failed` | `sudo systemctl start postgresql`, then `journalctl -u postgresql -n 30` if it will not |
| the journal says the address is already in use | Something else holds port 5432 — see below |

**Then find out which port your server is on**, because it is not necessarily 5432:

```bash
pg_lsclusters
```

```text
Ver Cluster Port Online Owner    Data directory              Log file
16  main    5432 online postgres /var/lib/postgresql/16/main /var/log/postgresql/…
```

**Write down the number in the `Port` column.** Everything below calls it `PGPORT`, and it goes
into `DATABASE_URL` at [step 9](#step-9--configure-and-start-the-three-services). Carrying it in
the shell saves retyping:

```bash
PGPORT=$(pg_lsclusters --no-header | awk '{print $3}' | head -1)
echo "$PGPORT"
```

> **It is 5433 or higher when something else already had 5432.** Ubuntu's packaging picks the
> next free port for a new cluster rather than failing, which is a kindness that is invisible
> until it confuses you: your socket commands work regardless of port, while anything using a
> URL goes wherever 5432 actually points.
>
> **That "something else" may not be yours to stop.** This box may run other applications — the
> first one this guide was walked on had another project's database on 5432, up for three weeks.
> Nothing here justifies stopping it, and nothing here needs to: the port lives in one
> environment variable and the application has no opinion about it.

| If `Online` says | Then |
|---|---|
| `online` | Carry on |
| `down` | `sudo pg_ctlcluster VERSION main start`, with the version from the first column, then `journalctl -u postgresql -n 20 --no-pager` if it will not |

> **This project's own `docker-compose.yml` is the likeliest container**, and it is built to
> catch you out: its `db` service ships a role called `landscape`, a database called
> `landscape`, and the password `landscape`. So a collision does not fail loudly — it succeeds
> against the wrong server.
>
> **Stop it by name, not with compose.** The checkout does not exist yet — it arrives in
> [step 7](#step-7--build-the-application) — so `docker compose down` has no project to read and
> `cd ~/competitive-landscape` fails:
>
> ```bash
> sudo docker ps
> ```
>
> Find the row whose image is a `postgres`, then, with its name or the first characters of its
> id:
>
> ```bash
> sudo docker stop NAME_OR_ID
> sudo systemctl restart postgresql
> sudo ss -ltnp | grep 5432
> ```
>
> `ss` should now name `postgres`. **If Postgres will not start**, look at
> `sudo journalctl -u postgresql -n 20 --no-pager`: a server that could not bind the port when
> it was installed has been down ever since, and anything you have run over the socket since
> then failed too.
>
> **The compose file publishes on `127.0.0.1` now** rather than every interface, so a container
> and the native server can no longer both look reachable from outside. They still cannot share
> the port.

---

## Step 6 — Create the database

**Generate the password into a shell variable**, so that the rest of this step uses it without
you retyping it anywhere:

```bash
cd /tmp
PGPASS=$(openssl rand -hex 24)
echo "$PGPASS"
```

**Write down what `echo` prints.** [Step 9](#step-9--configure-and-start-the-three-services)
needs the value, and `$PGPASS` is gone the moment you close this SSH session — if that happens
before step 9, come back and run the `ALTER USER` line below with a fresh one.

**Hexadecimal on purpose.** That password goes into three places with three different escaping
rules: a SQL string, a URL (`postgres://landscape:PASSWORD@...`), and a systemd environment
file. `/`, `?`, `#`, `%`, `@`, a space or a backslash changes how at least one of them parses —
and the failure is a service that will not start, or worse, one that connects somewhere else.
Letters and digits mean the same thing in all three.

Then, exactly as written — the shell puts the password in, so there is nothing to edit:

```bash
sudo -u postgres psql <<SQL
CREATE USER landscape WITH PASSWORD '$PGPASS';
CREATE DATABASE landscape OWNER landscape;
SQL
```

Two lines, and they are the whole of it: the first makes the **role** the application logs in
as, the second makes the **database** it logs in to, owned by that role.

> **`<<SQL` and not `<<'SQL'`.** The quotes around the marker are what stop a shell expanding
> anything inside a heredoc; without them `$PGPASS` becomes the password, which is the point.
> Nothing else in those two lines contains a `$`.

> **`could not change directory to "/home/ubuntu": Permission denied` is a warning, not a
> failure.** `sudo -u postgres` keeps your current directory and the `postgres` user cannot read
> your home. `psql` runs anyway, and everything printed after that line is the real result. The
> `cd /tmp` above is only there to silence it.

> **`role "landscape" already exists` means this step has run before.** Nothing is broken and
> nothing needs undoing — but `CREATE USER` failing means it did **not** set the password, so
> what is in force is whatever the earlier run used, which you may no longer know. Settle it
> rather than guess, with the same variable:
>
> ```bash
> sudo -u postgres psql -c "ALTER USER landscape WITH PASSWORD '$PGPASS';"
> ```
>
> Double quotes there, so your shell expands `$PGPASS` before `sudo` runs.
>
> `database "landscape" already exists` needs nothing at all: it is owned by that role and the
> application creates its own tables.

**Now prove it works from the outside**, with the same password, over TCP, exactly as the
services will:

```bash
psql "postgres://landscape:$PGPASS@127.0.0.1:$PGPORT/landscape" -c '\conninfo'
```

```text
You are connected to database "landscape" as user "landscape" on host "127.0.0.1" at port "5432".
```

**This is the check worth not skipping.** It proves four separate things at once — the server is
up, the role exists, the password is right, and the URL parses the way you meant — and it is the
string you are about to paste into `/etc/landscape/landscape.env` at
[step 9](#step-9--configure-and-start-the-three-services). Finding out here costs a minute;
finding out there costs a service that will not start and a journal line about authentication.

| If it says | Then |
|---|---|
| `could not connect to server` | The server is not running — see the table at the end of [step 5](#step-5--install-what-the-build-needs) |
| `password authentication failed` | The role exists and its password is not `$PGPASS` — which is what happens when the role predates this attempt. The `ALTER USER` line above sets it, and then this command passes |
| `database "landscape" does not exist` | The second SQL line did not run. Run it on its own |

> **`password authentication failed` right after you set the password almost always means you
> are talking to a different server.** `sudo -u postgres psql` goes over the Unix socket and
> reaches your cluster whatever port it is on; a URL goes over TCP, to whoever holds the port
> you named. `ALTER USER` changed the password on one and the check authenticated against the
> other.
>
> **The message will not tell you which.** PostgreSQL answers *password authentication failed
> for user "landscape"* whether or not that role exists — deliberately, so that it cannot be
> used to discover valid usernames. It is not evidence that the role is there.
>
> So check the port rather than reading the error:
>
> ```bash
> echo "$PGPORT"
> pg_lsclusters
> sudo ss -ltnp | grep "$PGPORT"
> ```
>
> If `$PGPORT` is empty you skipped that part of
> [step 5](#step-5--install-what-the-build-needs). If `ss` names `docker-proxy` on the port your
> cluster claims, two things are fighting over it and `pg_lsclusters` will say your cluster is
> `down`.
>
> **The `\conninfo` check is what catches all of this**, because it is the only command in this
> step that takes the same route the services will.

There is no schema step. **The application applies its own migrations on boot**, so there is
nothing to run and nothing to forget — the tables appear the first time
[step 9](#step-9--configure-and-start-the-three-services) starts a service.

---

## Step 7 — Build the application

```bash
cd ~
git clone https://github.com/larry94555/competitive-landscape.git
cd competitive-landscape
./scripts/build-release.sh
```

**About ten minutes.** It ends with a `dist/` directory. Install it:

```bash
sudo useradd --system --home /opt/landscape --shell /usr/sbin/nologin landscape || true
sudo mkdir -p /opt/landscape/bin /opt/landscape/web /opt/landscape/models
sudo cp dist/bin/landscape /opt/landscape/bin/
sudo cp -r dist/web/dist /opt/landscape/web/dist
sudo cp dist/MANIFEST /opt/landscape/
sudo chown -R root:root /opt/landscape
sudo chmod -R go-w /opt/landscape
```

Everything stays owned by `root`. The services only ever read these files, so a compromised
service cannot replace the binary it runs from.

---

## Step 8 — Build the model server, and get the model

`llama-server` has no package for ARM, so it is built here. **This is the slow step — about
twenty-five minutes.**

```bash
cd ~
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
git checkout b10291
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j4
sudo install -o root -g root -m 0755 build/bin/llama-server /opt/landscape/bin/
```

The revision is pinned on purpose: an inference build taken from a moving branch means the same
application commit can behave differently on two deployments.

Then the model — **Qwen3-4B Q4_K_M**, which is the one this project's quality numbers were
measured against ([MODEL_BAKEOFF.md](MODEL_BAKEOFF.md)):

```bash
sudo curl -L --output /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf \
  https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf

echo '7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5  /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf' | sha256sum --check
sudo chmod 0444 /opt/landscape/models/Qwen3-4B-Q4_K_M.gguf
```

The `sha256sum` line must print `OK`. **If it does not, stop and download it again** — a
different model makes every quality figure this project publishes inapplicable to what you are
running.

Record what you used:

```bash
printf 'llama.cpp b10291\nmodel bc640142c66e1fdd12af0bd68f40445458f3869b\n' | sudo tee -a /opt/landscape/MANIFEST
```

---

## Step 9 — Configure and start the three services

```bash
cd ~/competitive-landscape
sudo mkdir -p /etc/landscape
sudo cp deploy/landscape.env.example /etc/landscape/landscape.env
sudo nano /etc/landscape/landscape.env
```

In that file, change the `DATABASE_URL` line so it carries **two** things from
[step 6](#step-6--create-the-database): the password in place of `CHANGE_ME`, and the port your
cluster is actually on in place of `5432`. It is the string you already proved works:

```text
DATABASE_URL=postgres://landscape:THE_PASSWORD@127.0.0.1:THE_PORT/landscape
```

Save with `Ctrl+O`, `Enter`, then `Ctrl+X`.

> **The port is `5432` only if `pg_lsclusters` said so.** If another application already had
> that port when PostgreSQL was installed, your cluster is on 5433 or higher, and this line is
> the one place that has to know.

**If you still have the same SSH session open**, you do not have to retype it at all:

```bash
echo "$PGPASS"
```

and if that prints nothing, the session has been reconnected since step 6 — use what you wrote
down, or set a fresh one with the `ALTER USER` line from that step and use that.

```bash
sudo chmod 600 /etc/landscape/landscape.env
sudo chown landscape:landscape /etc/landscape/landscape.env

sudo cp deploy/*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now llama-server landscape-api landscape-worker
```

Check. The model server takes a minute or two to load the model the first time:

```bash
systemctl is-active llama-server landscape-api landscape-worker
curl -s http://127.0.0.1:8080/health
curl -s http://127.0.0.1:8787/api/health
```

You want three `active` lines, then two JSON responses. The second one reads the queue out of
Postgres, so a healthy answer also proves the application can reach the database.

> **If a service is not active**, `journalctl -u landscape-api -n 50` says why. The usual cause
> is the database password in `/etc/landscape/landscape.env` not matching [step 6](#step-6--create-the-database).

---

## Step 10 — Start the search engine

**Without this the site works, and one whole half of it does not.** Typing a company's website
produces a full report either way. Typing a *description* — "project management for a small
design agency" — needs a search engine to find the companies, and without one the site says so
rather than guessing.

SearXNG is a metasearch front end you host. It has no account, no key and no quota.

```bash
curl -fsSL https://get.docker.com | sudo sh
cd ~/competitive-landscape
sudo docker compose --profile search up -d searxng
```

Wait about thirty seconds, then check three things.

**It answers in the format the application asks for:**

```bash
curl -s 'http://127.0.0.1:8888/search?q=test&format=json' | head -c 100
```

**You want JSON.** If you get HTML or nothing, the container has not finished starting — wait
and try again. If it answers `403`, the settings file did not mount; `sudo docker compose
--profile search logs searxng` will say so.

**It is not listening to the internet:**

```bash
sudo ss -ltnp | grep 8888
```

The address must be `127.0.0.1:8888`, not `0.0.0.0:8888` or `*:8888`. SearXNG has no
authentication in front of it and nothing outside this box has any business reaching it.

**And it comes back by itself:**

```bash
sudo docker inspect -f '{{.HostConfig.RestartPolicy.Name}}' $(sudo docker compose --profile search ps -q searxng)
```

That must print `unless-stopped`. Without it the container stays down after the first reboot,
and every description quietly loses its search channel.

Now tell the application about it:

```bash
echo 'SEARX_URL=http://127.0.0.1:8888' | sudo tee -a /etc/landscape/landscape.env
sudo systemctl restart landscape-api landscape-worker
```

> **There is deliberately no default for `SEARX_URL`.** A fallback to somebody's public
> instance would send everything strangers type into your box to a third party.

---

## Step 11 — HTTPS

**Check DNS has arrived first.** On your own machine:

```bash
nslookup YOUR_DOMAIN
```

It must answer with `YOUR_IP`. If it does not, wait — asking for a certificate before the name
resolves spends a rate-limited retry for nothing.

### 11a. Find the address you browse from

**On your own machine, not the box:**

```bash
curl -s https://ifconfig.me
```

Write down what it prints. If your home address changes from time to time, note that this is the
one thing you may have to come back and edit.

### 11b. Install Caddy and point it at your domain

On the box:

```bash
sudo apt-get install -y caddy
sudo cp ~/competitive-landscape/deploy/Caddyfile.example /etc/caddy/Caddyfile
sudo nano /etc/caddy/Caddyfile
```

Change exactly two things in that file:

| Find | Replace with |
|---|---|
| `landscape.example.com` | your domain |
| `203.0.113.4` | the address `ifconfig.me` printed |

Save with `Ctrl+O`, `Enter`, then `Ctrl+X`. Then:

```bash
sudo systemctl restart caddy
sudo journalctl -u caddy -f
```

Watch until the log mentions a certificate being obtained for your domain — usually under a
minute. `Ctrl+C` stops watching; the service keeps running.

> **⚠ Least certain.** The allow-list refuses everybody but you, and the certificate authority is
> not you. Caddy answers the validation request ahead of those routes, so it should issue — but
> if the log shows a challenge failing, comment out the two `@allowed` lines and the `handle`
> blocks around them, restart, let the certificate issue, then put them back.

> **Why an allow-list at all:** a new certificate appears in public transparency logs within
> minutes of being issued, and scanners read those logs. Four ARM cores are not something to hand
> to the internet on day one. When you are ready for an audience, delete the `@allowed` matcher
> and the `handle` block that 403s everyone else.

---

## Step 12 — You are done. Check it.

**On your own machine, open `https://YOUR_DOMAIN`.**

The padlock should be there. If the browser warns about the certificate, Caddy has not finished
— `sudo journalctl -u caddy -n 30` on the box says where it got to.

You should see **What is your idea?**, a text box, an **Analyse** button, and three example
ideas underneath.

Then run the one check that exercises everything:

1. Click the example **project management for a small design agency**. It fills the box; it does
   not submit.
2. Press **Analyse**.
3. The address bar changes to `/a/…` — that is the permalink, and it works on reload.
4. Within about a minute the first section appears. **Recent public changes** usually fills
   first: it needs no model at all.
5. Between four and eight minutes later, **Done.** appears.

**If every section says "Nothing found in public sources" but the changelog filled**, the model
server is not answering — `systemctl status llama-server`. That is the one symptom that tells a
model problem apart from a fetching problem, because the changelog is the section that needs no
model.

Now go to **[USING_THE_SITE.md](USING_THE_SITE.md)**, which walks every feature the site has
from the browser.

---

## Keeping it running

### Updating to a newer commit

```bash
cd ~/competitive-landscape && git pull && ./scripts/build-release.sh
sudo systemctl stop landscape-api landscape-worker
sudo cp dist/bin/landscape /opt/landscape/bin/
sudo rm -rf /opt/landscape/web/dist && sudo cp -r dist/web/dist /opt/landscape/web/dist
sudo cp dist/MANIFEST /opt/landscape/
sudo cp deploy/*.service /etc/systemd/system/
sudo chown -R root:root /opt/landscape && sudo chmod -R go-w /opt/landscape
sudo systemctl daemon-reload
sudo systemctl start landscape-api landscape-worker
```

Migrations apply on boot, so there is never a moment where the binary and the schema disagree.
Stopping the worker mid-analysis is safe: the run returns to the queue and starts over, and the
reader watches it restart rather than watching a lie.

### When something is wrong

| What you see | Where to look |
|---|---|
| The browser times out | Both firewalls — [step 4](#step-4--open-the-two-firewalls), and the `iptables` rule numbering |
| `502` from Caddy | `systemctl status landscape-api` |
| The page loads but nothing works | The API is down; the page is being served by the fallback |
| Analyses stay **Queued.** for ever | `systemctl status landscape-worker` |
| Every section empty, changelog filled | `systemctl status llama-server` |
| "no search engine is configured" on a report | [Step 10](#step-10--start-the-search-engine) — `SEARX_URL` is missing or the container is down |

[RUNBOOK.md](RUNBOOK.md) is the longer version of that table.

### What it costs to run

Nothing, on the Always Free shapes, as long as you keep to one instance of 4 OCPU and 24 GB.
Your domain is the only recurring cost.

---

## Appendix A — running without a domain

Only if you skipped [step 3](#step-3--point-your-domain-at-it-now) and [step 11](#step-11--https). It works, and two things are worse:

- everything typed and read crosses the internet in clear text
- **"Copy as context" cannot reach the clipboard** — browsers only allow that on `https://`. The
  button still works: it puts the report in a text box on the page and tells you to copy it by
  hand

Make the API listen on the public interface and open its port:

```bash
sudo sed -i 's|^BIND_ADDR=.*|BIND_ADDR=0.0.0.0:8787|' /etc/landscape/landscape.env
sudo systemctl restart landscape-api
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 8787 -j ACCEPT
sudo netfilter-persistent save
```

Add an ingress rule for TCP `8787` in the Oracle security list, exactly as in [step 4a](#step-4--open-the-two-firewalls). The site
is then at `http://YOUR_IP:8787`.

**There is no allow-list in this version**, so anyone who finds the address can spend your four
cores. The per-visitor cap does not apply either: it counts the address Caddy observed, and
without Caddy there is no such header. Use it for an afternoon of your own testing and not for
anything you send to anybody.
