/**
 * Ozone-Studio Monitor — browser plugin service worker (Manifest V3).
 *
 * The browser is the first EXTERNAL observer agent: it registers with
 * Ozone-Studio over loopback HTTP, heartbeats so the dispatch table stays
 * live, and pushes browsing activity into the monitor hub. This is the
 * browser-side seed of the movement graph — Hearth's observer transport
 * will layer capture semantics on top of this registration/activity
 * contract later.
 *
 * Config (chrome.storage.sync):
 *   ozoneUrl   — Ozone-Studio base URL (default http://127.0.0.1:50051)
 *   agentName  — agent name (default "browser-plugin")
 *   enabled    — whether activity push is on
 */

const DEFAULTS = {
  ozoneUrl: "http://127.0.0.1:50051",
  agentName: "browser-plugin",
  enabled: true,
};

let registrationTimer = null;

async function getConfig() {
  const stored = await chrome.storage.sync.get(DEFAULTS);
  return { ...DEFAULTS, ...stored };
}

function agentId() {
  // Stable per-install agent id so the host registry can track re-connects.
  return chrome.runtime.id;
}

async function apiPost(path, body) {
  const cfg = await getConfig();
  const resp = await fetch(`${cfg.ozoneUrl}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw new Error(`${path} -> HTTP ${resp.status}`);
  return resp.json();
}

async function register() {
  const cfg = await getConfig();
  if (!cfg.enabled) return;
  await apiPost("/pipelines/register", {
    pipeline_id: 9001, // observer-agent range (host treats as custom)
    name: cfg.agentName,
    execute_url: `${cfg.ozoneUrl}/noop`, // observer has no execute endpoint
  });
  await pushActivity("agent", `${cfg.agentName} registered (id 9001)`);
}

async function pushActivity(kind, message, detail) {
  const cfg = await getConfig();
  if (!cfg.enabled) return;
  try {
    await apiPost("/monitor/activity", {
      kind,
      level: "info",
      source: cfg.agentName,
      message,
      detail,
    });
  } catch (e) {
    // Monitoring must never break browsing — swallow and continue.
    console.debug("ozone-monitor: push failed", e.message);
  }
}

async function pushNavigation(tab) {
  const cfg = await getConfig();
  if (!cfg.enabled) return;
  await pushActivity("external", `Navigated: ${tab.url}`, {
    tabId: tab.id,
    url: tab.url,
    title: tab.title,
    ts: Date.now(),
  });
}

// ── lifecycle ─────────────────────────────────────────────────────────────

chrome.runtime.onInstalled.addListener(async () => {
  await register();
  // Heartbeat: re-announce so the host registry stays current across
  // service-worker suspensions.
  if (registrationTimer) clearInterval(registrationTimer);
  registrationTimer = setInterval(register, 30_000);
});

chrome.runtime.onStartup.addListener(async () => {
  await register();
  if (registrationTimer) clearInterval(registrationTimer);
  registrationTimer = setInterval(register, 30_000);
});

// Activity: page navigations (the observable skeleton of browsing).
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === "complete" && tab.url?.startsWith("http")) {
    pushNavigation(tab).catch(() => {});
  }
});

// Message API for the popup (status + manual push).
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  (async () => {
    if (msg.type === "getStatus") {
      const cfg = await getConfig();
      sendResponse({ config: cfg, registered: true });
    } else if (msg.type === "pushTest") {
      await pushActivity("external", "Test activity from browser plugin");
      sendResponse({ ok: true });
    } else if (msg.type === "reregister") {
      await register();
      sendResponse({ ok: true });
    } else {
      sendResponse({ ok: false });
    }
  })();
  return true; // async sendResponse
});
