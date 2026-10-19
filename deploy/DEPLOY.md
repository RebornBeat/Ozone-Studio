# Deploying Ozone Studio on an Ubuntu VPS

This ships the host (HTTP API + orchestrator + CLI) as a single binary.
Everything — routes, MCP supervisor, security MCP, CLI — is the same binary
you run locally; `serve` is just the headless mode.

## 1. Transfer and unpack

```bash
scp ozone-studio-<ver>-linux-x64.tar.gz user@your-vps:/tmp/
ssh user@your-vps
sudo mkdir -p /opt/ozone-studio
sudo tar -xzf /tmp/ozone-studio-<ver>-linux-x64.tar.gz -C /opt/ozone-studio --strip-components=1
cd /opt/ozone-studio
```

## 2. Config

```bash
sudo -u ozone cp config.example.toml config.toml   # or craft your own
```

Key settings for a VPS:
- `[grpc] address = "127.0.0.1"` (default, secure) — expose via SSH tunnel
  (`ssh -L 50051:127.0.0.1:50051 user@vps`) or a reverse proxy. Only set
  `address = "0.0.0.0"` if you have firewall rules guarding it.
- `[general] data_dir` — where graphs/registries/history live. Keep it on a
  volume with real headroom (it grows).

## 3. Run as a service

```bash
sudo useradd -r -s /usr/sbin/nologin ozone
sudo chown -R ozone:ozone /opt/ozone-studio
sudo cp ozone-studio.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ozone-studio
systemctl status ozone-studio
```

## 4. CLI

The same binary is the operator CLI (works locally or over SSH on the VPS):

```bash
./ozone-studio version
./ozone-studio doctor                 # real environment checks
./ozone-studio serve --host 0.0.0.0   # expose (only with firewall rules)
./ozone-studio mcp list               # from the VPS: all registered tools + running state
./ozone-studio mcp autostart sec_firewall_status on   # e.g. the security monitor
./ozone-studio mcp start <tool>       # on-demand start (hook-based model)
```

## 5. Security MCP on the VPS

The security monitor (read-only: firewall status, listening ports, process
top, network connections) is the one tool with auto-start enabled by ruling.
On the VPS:

```bash
./ozone-studio mcp list               # sec_* tools should show auto=True after config
```

If it shows `auto=False` (fresh registry), flip it once:

```bash
./ozone-studio mcp autostart sec_firewall_status on
```

The supervisor then starts it and revives it if it ever dies.

## 6. Firewall baseline

```bash
sudo apt install -y ufw
sudo ufw allow OpenSSH
sudo ufw enable
# Only open 50051 if you bound to 0.0.0.0 deliberately:
# sudo ufw allow from <your-ip> to any port 50051 proto tcp
```

## 7. Pipelines (optional)

The tarball ships the host only. If the VPS should run the modality
pipeline fleet, also copy `target/pipelines/release/` to
`/opt/ozone-studio/pipelines/` and re-run `./ozone-studio doctor` — it
reports how many pipeline binaries it can see.
