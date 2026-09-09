# Ozone-Studio Browser Plugin (Monitor Observer)

Manifest V3 browser extension — the browser-side **observer agent** for
Ozone-Studio's monitor hub. Registers with the host, heartbeats, and pushes
browsing activity into the same activity feed the dashboard reads.

This is the browser-side seed of the **movement graph** — capture semantics
(page content, selections, interaction provenance) layer on top of this
registration/activity contract later.

## Install (developer mode)

1. Open `chrome://extensions` (or `edge://extensions`)
2. Enable **Developer mode**
3. **Load unpacked** → select this directory (`tools/browser-plugin/`)

## Configure

Click the extension icon:

- **Host URL** — Ozone-Studio base URL (default `http://127.0.0.1:8080`)
- **Agent name** — defaults to `browser-plugin`

**Save + re-register** announces the plugin to the host immediately; a
heartbeat re-announces every 30 s so the registry stays current.

## What it does

| Endpoint | Direction | Purpose |
|---|---|---|
| `POST /pipelines/register` | plugin → host | announce/heartbeat (agent id 9001) |
| `POST /monitor/activity` | plugin → host | push navigation activity (`kind: external`) |
| `GET /monitor/summary` | dashboard | agents + activity feed (see ConnectedAgents/MonitoringPanel) |

## Privacy

Only URL, title, and tab metadata are pushed — **no page content**. Capture
semantics (content, selections) come later via the capture-envelope grant,
explicitly opt-in.
