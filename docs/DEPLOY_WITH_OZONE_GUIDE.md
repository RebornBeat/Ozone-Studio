# Deploying and Operating with Ozone-Studio — The Zero-Knowledge Guide

> Written for a person OR an LLM that has never seen Ozone-Studio before.
> Everything needed to (1) install Ozone-Studio on a fresh Ubuntu VPS,
> (2) connect to it securely, and (3) use it as the operations brain for
> deploying and monitoring web applications — with the real security rules
> that keep the server alive on the public internet.
>
> Everything here was learned by doing: this exact flow deployed and now
> runs a real Django/React application on a live Contabo EU server, and
> repelled an active SSH brute-force environment (4,000+ attempts logged).

---

## 0. What Ozone-Studio IS (one paragraph)

Ozone-Studio is a single Linux binary that is simultaneously a **web API
server** (a graph-backed "Collective AGI Framework" with orchestration,
monitoring, and tools) and a **command-line operator tool**. You do not need
to understand the internals to use it operationally: it runs as a systemd
service, listens on localhost, exposes an HTTP API plus a WebSocket event
stream, carries a registry of "MCP tools" (small helper servers — a security
monitor, git history, etc.), and gives you a `doctor`/`mcp` CLI for
everything operational. One binary, one config file, one data directory.

---

## 1. Install on a fresh Ubuntu VPS (10 minutes)

**You need**: an Ubuntu 22.04/24.04 VPS, SSH access (password for first
contact or a key), and the release tarball
(`ozone-studio-<version>-linux-x64.tar.gz`, ~27 MB).

```bash
# From your machine:
scp ozone-studio-<ver>-linux-x64.tar.gz user@YOUR_VPS_IP:/tmp/

# On the VPS:
sudo mkdir -p /opt/ozone-studio
sudo tar -xzf /tmp/ozone-studio-<ver>-linux-x64.tar.gz -C /opt/ozone-studio --strip-components=1
cd /opt/ozone-studio
sudo cp config.example.toml config.toml
sudo useradd -r -s /usr/sbin/nologin ozone
sudo chown -R ozone:ozone /opt/ozone-studio
sudo cp ozone-studio.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ozone-studio
systemctl status ozone-studio        # expect: active
```

**First command, always**: `sudo -u ozone ./ozone-studio doctor` — it runs
real checks (config parse, data-dir writability, pipeline binaries found,
port availability) and tells you exactly what is wrong. No guessing.

---

## 2. Connect to it securely

The host binds to **127.0.0.1 only** by default — it is invisible to the
internet by design (verified: external connection refused). You reach it one
of two ways:

- **SSH tunnel (recommended)**: from your machine,
  `ssh -L 50051:127.0.0.1:50051 user@YOUR_VPS_IP`, then point apps at
  `http://127.0.0.1:50051`. The UI pairs to the host over this tunnel
  (Pairing panel → device pairing; unpaired connections see only public
  health data).
- **Direct bind**: `sudo -u ozone ./ozone-studio serve --host 0.0.0.0` —
  ONLY do this with a firewall rule restricting who can reach the port, and
  ideally a reverse proxy (nginx/Caddy) with TLS in front.

**Security rules that are not optional** (learned from an active
brute-force environment — a "help me escape" bot class that scans SSH and
tries default passwords like root:root/admin:admin/support:support):

1. `ufw` default-deny incoming; allow ONLY the ports you actually serve.
2. Install fail2ban — it bans brute-forcers automatically (ours banned
   attackers within minutes of going live; 4,000+ attempts/day is normal on
   a public IP).
3. NEVER use default credentials (root:root, admin:admin). The password
   should be long and unique. Key-only auth is stronger; password auth is
   acceptable WITH fail2ban + a non-default password (operator's choice).
4. Services bind to 127.0.0.1 unless they must be public. Public = behind a
   proxy/TLS.

---

## 3. The operational CLI (same binary, second mode)

```bash
./ozone-studio version                  # what am I running
sudo -u ozone ./ozone-studio doctor     # environment checks — run after ANY change
./ozone-studio mcp list                 # all registered helper tools + running state
./ozone-studio mcp autostart <tool> on  # opt a tool into auto-start + crash-restart
./ozone-studio mcp start <tool>         # start one on demand (hook-based model)
./ozone-studio mcp stop <tool>
```

`mcp` commands talk to a RUNNING host over its HTTP routes — the CLI is a
client, the host is the single writer. Default target
`http://127.0.0.1:50051` (override with `--host` or `$OZONE_HOST`).

**Auto-start policy**: by default NOTHING auto-starts except what the
operator explicitly opts in. The one tool that should always be on is the
**security monitor** (`sec_firewall_status` group — read-only system/network
checks):

```bash
./ozone-studio mcp autostart sec_firewall_status on
```

---

## 4. Deploying a web application (the worked example: Django + React)

This exact flow deployed a real Django-REST + React SPA to a live VPS.

**Shape of the app**: React SPA (pre-built `frontend/build/`) +
Django REST backend (`gunicorn`) + PostgreSQL + Redis (cache/celery) +
nginx in front (static files + API proxy).

### 4.1 What to transfer — and what NEVER to transfer

Transfer ONLY: built frontend assets, backend source, requirements.txt,
`.env` (production values). **Never transfer**: `.venv/`, `node_modules/`,
`.git/`, logs, test fixtures, unrelated repositories. These are heavy
(hundreds of MB) and every dependency is rebuilt fresh on the server.

### 4.2 Server-side stack

```bash
sudo apt install -y nginx postgresql redis-server python3-venv python3-pip
sudo -u postgres psql -c "CREATE USER app_user WITH PASSWORD '<generated>'"
sudo -u postgres psql -c "CREATE DATABASE app_db OWNER app_user"
```

Generate the DB password ON THE SERVER (`openssl rand -hex 16`), store it in
a mode-600 root-only file, and write it into the app's `.env` — never print
it, never commit it.

### 4.3 Django backend

```bash
cd /opt/yourapp
python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
```

Then the production edits to `.env`:
- `DEBUG=False`, `ALLOWED_HOSTS=<server-ip>,localhost`
- DB → the local Postgres + generated password
- `CORS_ORIGINS=http://<server-ip-or-domain>`

Then:
```bash
mkdir -p logs        # if settings.py logs to ./logs — it fails without it
.venv/bin/python manage.py migrate --noinput
.venv/bin/python manage.py collectstatic --noinput
```

A gunicorn systemd unit (same hardening pattern as Ozone's own):

```ini
[Service]
ExecStart=/opt/yourapp/.venv/bin/gunicorn --workers 3 --bind 127.0.0.1:8000 yourmodule.wsgi:application
Restart=always
NoNewPrivileges=true
ProtectSystem=full
```

### 4.4 nginx (SPA + API in one server block)

```nginx
server {
    listen 80;
    server_name <ip-or-domain>;
    root /var/www/yourapp;             # the built SPA

    location /static-django/ { alias /opt/yourapp/static/; }   # collectstatic output
    location /api/    { proxy_pass http://127.0.0.1:8000;
                        proxy_set_header Host $host;
                        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for; }
    location /admin/  { proxy_pass http://127.0.0.1:8000; }     # if Django admin
    location /        { try_files $uri $uri/ /index.html; }     # SPA fallback
}
```

Then open only what you serve: `sudo ufw allow 80/tcp` (and 443 later).

### 4.5 Verify from OUTSIDE the server

curl the public IP: `/` should return the SPA's index.html, `/api/` the
backend's response, `/admin/` the admin login. Localhost-only success means
nothing — always verify from outside (or through a tunnel from outside).

---

## 5. Real lessons from the live deployment (each one bit us)

1. **Missing runtime packages surface at migrate time**: the app imported
   `dj_db_conn_pool` but `requirements.txt` didn't list it. Symptom:
   `ImproperlyConfigured: isn't an available database backend`. Fix: install
   the real package AND append it to requirements.txt so the next deploy
   doesn't trip.
2. **Excluded directories must be re-created**: `logs/` was excluded from
   transfer; Django's logging config failed on startup until `mkdir logs`.
3. **Redis is part of "the app" even if your checklist forgot it**: an
   `/api/` 500 traced to missing Redis. Install + enable it, re-verify.
4. **400 from localhost ≠ broken app**: Django's ALLOWED_HOSTS rejects
   `curl http://127.0.0.1:8000/` (Host mismatch). Test through nginx with
   the real Host header, or from outside.
5. **A frozen copy can shadow the real one**: Ozone itself hit this — the
   backend served a bootstrap-era copy of a UI file while every fix landed
   in the source. Rule: the SOURCE is the first resolution candidate;
   copies are fallbacks, never the front of the search.
6. **fail2ban earns its keep immediately**: 3 bans within the first day.

---

## 6. Monitoring it afterwards — Ozone's reason to exist on the box

The security MCP (auto-started) gives continuous read-only answers:

- `sec_firewall_status` — the real nftables/ufw ruleset, live
- `sec_listening_ports` — what is actually listening (catch surprises)
- `sec_process_top` — what is burning CPU (catch intrusions/miners)
- `sec_net_connections` — who is connected

Call them through the API (`POST /mcp/tools/<name>/...`) or check state via
`./ozone-studio mcp list`. Health: `curl 127.0.0.1:50051/health`.

## 7. Domain + TLS (when ready)

Point a domain's A record at the VPS IP, then:
```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d your-domain.com
```
Certbot rewrites the nginx block for TLS + auto-renewal (`certbot renew
--dry-run` to test). The app configuration does not change.

---

*Every claim in this guide traces to a real deployment action or a live
verified check performed on an EU (Contabo) Ubuntu 24.04 server — see
CHECKLIST.md 2026-10-19 entries for the full evidence trail.*
