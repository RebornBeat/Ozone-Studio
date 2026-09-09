// Popup — config + connection status for the Ozone-Studio browser plugin.

const $ = (id) => document.getElementById(id);

async function loadConfig() {
  const cfg = await chrome.storage.sync.get({
    ozoneUrl: "http://127.0.0.1:50051",
    agentName: "browser-plugin",
  });
  $("ozoneUrl").value = cfg.ozoneUrl;
  $("agentName").value = cfg.agentName;
}

async function saveAndReregister() {
  const ozoneUrl = $("ozoneUrl").value.trim().replace(/\/$/, "");
  const agentName = $("agentName").value.trim() || "browser-plugin";
  await chrome.storage.sync.set({ ozoneUrl, agentName });
  chrome.runtime.sendMessage({ type: "reregister" }, (resp) => {
    $("status").textContent = resp?.ok
      ? "Saved — re-registered with Ozone-Studio."
      : "Saved — host unreachable, will retry on heartbeat.";
  });
}

$("save").addEventListener("click", saveAndReregister);
$("pushTest").addEventListener("click", () => {
  chrome.runtime.sendMessage({ type: "pushTest" }, (resp) => {
    $("status").textContent = resp?.ok
      ? "Test activity pushed to the monitor hub."
      : "Push failed — is Ozone-Studio running?";
  });
});

loadConfig();
