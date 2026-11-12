/**
 * SettingsPanel — application settings, straight from the host config.
 *
 * Reads the live config (GET /config/get, full document) and writes typed
 * sections back (POST /config/set): models, voice, consciousness, network.
 * The desktop gets its session injected by the bridge; browsers send the
 * pairing token.
 *
 * Sectioned (Model / Voice / Consciousness / Network / Advanced) rather than
 * one long scroll — there's enough config surface now to warrant real
 * sub-navigation. Advanced is read-only: those fields (general/auth/
 * integrity/grpc/tasks) only take effect at process start, so editing them
 * live is low-value and higher-risk (e.g. changing the bound gRPC port).
 */
import React, { useEffect, useState } from "react";

type Section = "model" | "voice" | "consciousness" | "network" | "algorithms" | "advanced";

interface AvailableModelEntry {
  name: string;
  model_type: string;
  identifier: string;
  context_length?: number;
  provider?: string;
  is_free?: boolean;
  api_endpoint?: string;
}

// Providers the host can talk to directly (host config::VENDOR_PRESETS + the vendor CLI agents). The HOST owns the endpoint and wire protocol
// for each name (it fills them in when an entry is added), so this list is only the choice the operator sees. All are PAID classes:
// the host refuses them while "Allow paid models" is off.
const PROVIDER_CHOICES: { value: string; label: string; kind: "api" | "cli" }[] = [
  { value: "anthropic", label: "Anthropic (API)", kind: "api" },
  { value: "openai", label: "OpenAI (API)", kind: "api" },
  { value: "google", label: "Google Gemini (API)", kind: "api" },
  { value: "zai", label: "z.ai (API)", kind: "api" },
  { value: "minimax", label: "MiniMax (API)", kind: "api" },
  { value: "codex", label: "Codex CLI agent", kind: "cli" },
  { value: "claude", label: "Claude Code CLI agent", kind: "cli" },
  { value: "zcode", label: "ZCode CLI agent", kind: "cli" },
];

interface ModelsCfg {
  model_type?: string;
  api_provider?: string;
  api_key?: string;
  api_endpoint?: string;
  api_model?: string;
  context_length?: number;
  gpu_layers?: number | null;
  allow_user_selection?: boolean;
  allow_paid_models?: boolean;
  wire_protocol?: string;
  bitnet_cli_path?: string;
  local_model_path?: string;
  local_model_type?: string;
  available_models?: AvailableModelEntry[];
  fallback?: { order?: string[]; free_only?: boolean };
}

interface VoiceCfg {
  enabled?: boolean;
  backend?: string;
  whisper_model_path?: string;
  whisper_cpp_path?: string;
  api_endpoint?: string;
  api_key?: string;
  language?: string;
  ffmpeg_path?: string;
}

interface ConsciousnessCfg {
  enabled?: boolean;
  emotional_system_enabled?: boolean;
  experience_memory_enabled?: boolean;
  identity_system_enabled?: boolean;
  relationship_system_enabled?: boolean;
  ethical_system_enabled?: boolean;
  collective_enabled?: boolean;
  show_emotional_state?: boolean;
  show_decision_reasoning?: boolean;
  i_loop_interval_ms?: number;
  playback_enabled?: boolean;
}

interface NetworkCfg {
  enable_p2p?: boolean;
  enable_cloud_sync?: boolean;
  p2p_port?: number;
  max_peers?: number;
  enable_mdns?: boolean;
  batch_sync_interval_secs?: number;
}

interface AlgorithmsCfg {
  convergence_preset?: string;
  pairwise_preset?: string;
  convergence_options?: string[];
  pairwise_options?: string[];
}

interface AdvancedCfg {
  general?: { data_dir?: string; log_level?: string; setup_complete?: boolean };
  auth?: { session_duration_secs?: number; challenge_expiry_secs?: number };
  integrity?: { enabled?: boolean; check_interval_secs?: number; max_versions?: number };
  grpc?: { address?: string; port?: number };
  tasks?: { max_queued_tasks?: number; task_timeout_secs?: number; max_task_history?: number };
}

async function ozoneConfigGet(): Promise<any> {
  const oz = (window as any).ozone;
  if (oz?.config?.get) return oz.config.get();
  return fetch(
    `${(window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"}/config/get`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ section: "", session_token: "" }),
    },
  ).then((r) => r.json());
}

async function ozoneConfigSet(updates: any): Promise<any> {
  const oz = (window as any).ozone;
  if (oz?.config?.set) return oz.config.set(updates);
  return fetch(
    `${(window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"}/config/set`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updates, session_token: "" }),
    },
  ).then((r) => r.json());
}

const SECTIONS: { key: Section; label: string }[] = [
  { key: "model", label: "Model" },
  { key: "voice", label: "Voice" },
  { key: "consciousness", label: "Consciousness" },
  { key: "network", label: "Network" },
  { key: "algorithms", label: "Algorithms" },
  { key: "advanced", label: "Advanced" },
];

export const SettingsPanel: React.FC = () => {
  const [section, setSection] = useState<Section>("model");
  const [models, setModels] = useState<ModelsCfg>({});
  const [voice, setVoice] = useState<VoiceCfg>({});
  const [consciousness, setConsciousness] = useState<ConsciousnessCfg>({});
  const [network, setNetwork] = useState<NetworkCfg>({});
  const [algorithms, setAlgorithms] = useState<AlgorithmsCfg>({});
  const [advanced, setAdvanced] = useState<AdvancedCfg>({});
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");
  const [addForm, setAddForm] = useState({
    provider: "anthropic",
    name: "",
    identifier: "",
    api_key: "",
    context_length: "",
  });
  const [addNote, setAddNote] = useState("");

  // Register a provider model (API vendor or CLI agent). The host fills in the endpoint / wire protocol from its own presets.
  const addProviderModel = async () => {
    setAddNote("");
    const choice = PROVIDER_CHOICES.find((c) => c.value === addForm.provider);
    if (!choice) return;
    const identifier = addForm.identifier.trim() || (choice.kind === "cli" ? `cli/${choice.value}` : "");
    if (!identifier) {
      setAddNote("Give the model identifier (for example the vendor's model id).");
      return;
    }
    const entry: AvailableModelEntry & { api_key?: string } = {
      name: addForm.name.trim() || `${choice.label} — ${identifier}`,
      model_type: choice.kind === "cli" ? "cli" : "api",
      identifier,
      context_length: addForm.context_length ? Number(addForm.context_length) : undefined,
      provider: choice.value,
      is_free: false,
    };
    if (choice.kind === "api" && addForm.api_key.trim()) entry.api_key = addForm.api_key.trim();
    try {
      const out = await ozoneConfigSet({ models: { add_model: entry } });
      if (out?.success === false) throw new Error(out?.error ?? "Could not add the model");
      const fresh = await ozoneConfigGet();
      const cfg = fresh?.config ?? fresh;
      setModels((m) => ({ ...m, available_models: cfg?.models?.available_models ?? m.available_models }));
      setAddForm({ ...addForm, name: "", identifier: "", api_key: "" });
      setAddNote("Added. Put it in the fallback chain below to use it.");
    } catch (e: any) {
      setAddNote(e?.message ?? "Could not add the model");
    }
  };

  const removeProviderModel = async (identifier: string) => {
    setAddNote("");
    try {
      const out = await ozoneConfigSet({ models: { remove_model: identifier } });
      if (out?.success === false) throw new Error(out?.error ?? "Could not remove the model");
      setModels((m) => ({
        ...m,
        available_models: (m.available_models ?? []).filter((x) => x.identifier !== identifier),
        fallback: { ...(m.fallback ?? {}), order: (m.fallback?.order ?? []).filter((x) => x !== identifier) },
      }));
    } catch (e: any) {
      setAddNote(e?.message ?? "Could not remove the model");
    }
  };

  const chainOrder = models.fallback?.order ?? [];
  const setChain = (order: string[]) =>
    setModels({ ...models, fallback: { ...(models.fallback ?? {}), order } });
  const moveInChain = (i: number, delta: number) => {
    const j = i + delta;
    if (j < 0 || j >= chainOrder.length) return;
    const next = [...chainOrder];
    [next[i], next[j]] = [next[j], next[i]];
    setChain(next);
  };

  useEffect(() => {
    (async () => {
      try {
        const out = await ozoneConfigGet();
        const cfg = out?.config ?? out;
        setModels(cfg?.models ?? {});
        setVoice(cfg?.voice ?? {});
        setConsciousness(cfg?.consciousness ?? {});
        setNetwork(cfg?.network ?? {});
        setAdvanced({
          general: cfg?.general,
          auth: cfg?.auth,
          integrity: cfg?.integrity,
          grpc: cfg?.grpc,
          tasks: cfg?.tasks,
        });
        // Separate fetch: the "k_algorithms" section reports the LIVE
        // registry state (current default + every registered option), not
        // just the persisted config value the bulk "" fetch above returns.
        try {
          const oz = (window as any).ozone;
          const kOut = oz?.config?.get
            ? await oz.config.get("k_algorithms")
            : await fetch(
                `${(window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"}/config/get`,
                {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ section: "k_algorithms", session_token: "" }),
                },
              ).then((r) => r.json());
          setAlgorithms(kOut?.config ?? kOut ?? {});
        } catch {
          // Non-fatal — algorithms tab just shows blanks if this fails.
        }
        setLoaded(true);
      } catch (e: any) {
        setError(e?.message ?? "Failed to load config");
      }
    })();
  }, []);

  const save = async () => {
    setSaving(true);
    setSaved(false);
    setError("");
    const updates = {
      models: {
        model_type: models.model_type,
        api_provider: models.api_provider,
        api_key: models.api_key || undefined,
        api_endpoint: models.api_endpoint || undefined,
        api_model: models.api_model || undefined,
        context_length: models.context_length || undefined,
        gpu_layers: models.gpu_layers ?? undefined,
        allow_user_selection: models.allow_user_selection,
        allow_paid_models: models.allow_paid_models,
        wire_protocol: models.wire_protocol || undefined,
        bitnet_cli_path: models.bitnet_cli_path || undefined,
        local_model_path: models.local_model_path || undefined,
        local_model_type: models.local_model_type,
        fallback: models.fallback
          ? { order: models.fallback.order ?? [], free_only: models.fallback.free_only ?? false }
          : undefined,
      },
      voice: {
        enabled: voice.enabled ?? false,
        backend: voice.backend ?? "whisper_rs",
        whisper_model_path: voice.whisper_model_path || undefined,
        whisper_cpp_path: voice.whisper_cpp_path || undefined,
        api_endpoint: voice.api_endpoint || undefined,
        api_key: voice.api_key || undefined,
        language: voice.language || undefined,
        ffmpeg_path: voice.ffmpeg_path || undefined,
      },
      consciousness: {
        enabled: consciousness.enabled ?? false,
        emotional_system_enabled: consciousness.emotional_system_enabled,
        experience_memory_enabled: consciousness.experience_memory_enabled,
        identity_system_enabled: consciousness.identity_system_enabled,
        relationship_system_enabled: consciousness.relationship_system_enabled,
        ethical_system_enabled: consciousness.ethical_system_enabled,
        collective_enabled: consciousness.collective_enabled,
        show_emotional_state: consciousness.show_emotional_state,
        show_decision_reasoning: consciousness.show_decision_reasoning,
        i_loop_interval_ms: consciousness.i_loop_interval_ms,
        playback_enabled: consciousness.playback_enabled,
      },
      network: {
        enable_p2p: network.enable_p2p,
        enable_cloud_sync: network.enable_cloud_sync,
        p2p_port: network.p2p_port,
        max_peers: network.max_peers,
        enable_mdns: network.enable_mdns,
        batch_sync_interval_secs: network.batch_sync_interval_secs,
      },
      k_algorithms: {
        convergence_preset: algorithms.convergence_preset,
        pairwise_preset: algorithms.pairwise_preset,
      },
    };
    try {
      const out = await ozoneConfigSet(updates);
      if (out?.success === false) throw new Error(out?.error ?? "Save failed");
      setSaved(true);
      setTimeout(() => setSaved(false), 2600);
    } catch (e: any) {
      setError(e?.message ?? "Save failed");
    } finally {
      setSaving(false);
    }
  };

  if (!loaded && !error) {
    return (
      <div className="opanel">
        <div className="opanel-head">
          <span className="odot err" />
          <span className="opanel-title">Settings</span>
        </div>
        <div className="oempty">Loading configuration…</div>
      </div>
    );
  }

  const isLocalBackend = ["bitnet", "gguf", "onnx", "local"].includes(
    models.model_type ?? "",
  );

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className="odot ok" />
        <span className="opanel-title">Settings</span>
      </div>
      <p className="opanel-sub">
        Live host configuration — changes write through to the running
        Ozone-Studio.
      </p>

      {error && <p style={{ color: "#f87171", fontSize: 13 }}>{error}</p>}

      <div className="oseg" style={{ marginBottom: 14 }}>
        {SECTIONS.map((s) => (
          <button
            key={s.key}
            className={section === s.key ? "on" : ""}
            onClick={() => setSection(s.key)}
          >
            {s.label}
          </button>
        ))}
      </div>

      <div className="opanel-scroll">
        {section === "model" && (
          <div className="ocards">
            <div className="ocard">
              <h4>⚡ Model source</h4>
              <div className="ofield">
                <label>Model type</label>
                <select
                  className="oinput"
                  value={models.model_type ?? ""}
                  onChange={(e) =>
                    setModels({ ...models, model_type: e.target.value })
                  }
                >
                  <option value="zcode">ZCode (registered agent)</option>
                  <option value="api">API model</option>
                  <option value="bitnet">BitNet (1-bit local)</option>
                  <option value="gguf">GGUF (local, llama.cpp)</option>
                  <option value="onnx">ONNX (local)</option>
                  <option value="cli">CLI agent (codex / claude / zcode)</option>
                </select>
              </div>

              {models.model_type === "api" && (
                <>
                  <div className="ofield">
                    <label>Provider</label>
                    <select
                      className="oinput"
                      value={models.api_provider ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, api_provider: e.target.value })
                      }
                    >
                      <option value="anthropic">Anthropic</option>
                      <option value="openai">OpenAI</option>
                      <option value="google">Google</option>
                      <option value="zai">z.ai</option>
                      <option value="minimax">MiniMax</option>
                      <option value="openrouter">OpenRouter</option>
                      <option value="custom">Custom</option>
                    </select>
                  </div>
                  {models.api_provider === "custom" && (
                    <div className="ofield">
                      <label>API endpoint</label>
                      <input
                        className="oinput"
                        placeholder="https://…/v1/chat/completions"
                        value={models.api_endpoint ?? ""}
                        onChange={(e) =>
                          setModels({ ...models, api_endpoint: e.target.value })
                        }
                      />
                    </div>
                  )}
                  <div className="ofield">
                    <label>API key</label>
                    <input
                      className="oinput"
                      type="password"
                      value={models.api_key ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, api_key: e.target.value })
                      }
                    />
                  </div>
                  <div className="ofield">
                    <label>Model identifier</label>
                    <input
                      className="oinput"
                      placeholder="e.g. anthropic/claude-3.5-sonnet"
                      value={models.api_model ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, api_model: e.target.value })
                      }
                    />
                  </div>
                  <div className="ofield">
                    <label>Wire protocol</label>
                    <select
                      className="oinput"
                      value={models.wire_protocol ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, wire_protocol: e.target.value })
                      }
                    >
                      <option value="">Auto (sniff from endpoint)</option>
                      <option value="anthropic">Anthropic Messages</option>
                      <option value="chat_completions">
                        OpenAI Chat Completions
                      </option>
                    </select>
                  </div>
                </>
              )}

              {models.model_type === "cli" && (
                <>
                  <div className="ofield">
                    <label>CLI agent</label>
                    <select
                      className="oinput"
                      value={models.api_endpoint ?? "cli:codex"}
                      onChange={(e) =>
                        setModels({ ...models, api_endpoint: e.target.value })
                      }
                    >
                      <option value="cli:codex">Codex CLI</option>
                      <option value="cli:claude">Claude Code CLI</option>
                      <option value="cli:zcode">ZCode CLI</option>
                    </select>
                  </div>
                  <div className="ofield">
                    <label>Model (optional — the CLI's own default if empty)</label>
                    <input
                      className="oinput"
                      value={models.api_model ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, api_model: e.target.value })
                      }
                    />
                  </div>
                  <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "2px 0 0" }}>
                    The host runs the CLI itself in a scratch directory with a
                    cleared environment (only its own credentials pass), sends
                    the prompt on stdin and reads the answer from stdout. The
                    call is judged by the jurisdiction gate like any provider:
                    list the CLI's vendor under Marketplace → Jurisdiction →
                    providers (destination <code>cli:codex</code>,{" "}
                    <code>cli:claude</code>, <code>cli:zcode</code>) or it is
                    held for your decision. Needs "Allow paid models".
                  </p>
                </>
              )}

              {models.model_type === "bitnet" && (
                <>
                  <div className="ofield">
                    <label>BitNet CLI path (llama-cli)</label>
                    <input
                      className="oinput"
                      placeholder="/path/to/BitNet/build/bin/llama-cli"
                      value={models.bitnet_cli_path ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, bitnet_cli_path: e.target.value })
                      }
                    />
                  </div>
                  <div className="ofield">
                    <label>Model file (.gguf)</label>
                    <input
                      className="oinput"
                      value={models.local_model_path ?? ""}
                      onChange={(e) =>
                        setModels({ ...models, local_model_path: e.target.value })
                      }
                    />
                  </div>
                </>
              )}

              {(models.model_type === "gguf" || models.model_type === "onnx") && (
                <div className="ofield">
                  <label>Model file</label>
                  <input
                    className="oinput"
                    value={models.local_model_path ?? ""}
                    onChange={(e) =>
                      setModels({ ...models, local_model_path: e.target.value })
                    }
                  />
                </div>
              )}

              {models.model_type === "zcode" && (
                <p style={{ color: "var(--color-text-muted)", fontSize: 12.5, margin: 0 }}>
                  Model calls dispatch to the ZCode connector registered on
                  this host (roles: agent + model).
                </p>
              )}

              {isLocalBackend && (
                <div className="ofield">
                  <label>GPU layers</label>
                  <input
                    className="oinput"
                    type="number"
                    min={0}
                    value={models.gpu_layers ?? ""}
                    onChange={(e) =>
                      setModels({
                        ...models,
                        gpu_layers: e.target.value ? Number(e.target.value) : null,
                      })
                    }
                  />
                </div>
              )}

              <div className="ofield">
                <label>Context length</label>
                <input
                  className="oinput"
                  type="number"
                  min={1}
                  value={models.context_length ?? ""}
                  onChange={(e) =>
                    setModels({
                      ...models,
                      context_length: Number(e.target.value) || undefined,
                    })
                  }
                />
              </div>

              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={models.allow_user_selection ?? false}
                  onChange={(e) =>
                    setModels({ ...models, allow_user_selection: e.target.checked })
                  }
                />
                <span>Allow model selection from the chat UI</span>
              </label>

              <label className="otoggle" style={{ marginTop: 6 }}>
                <input
                  type="checkbox"
                  checked={models.allow_paid_models ?? false}
                  onChange={(e) =>
                    setModels({ ...models, allow_paid_models: e.target.checked })
                  }
                />
                <span>Allow paid models</span>
              </label>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "2px 0 0" }}>
                Off (default): only free OpenRouter models and the local model
                are selectable. On: paid OpenRouter models and paid direct
                providers (Anthropic, OpenAI, Google) can be chosen too.
              </p>
            </div>

            <div className="ocard">
              <h4>📋 Available models</h4>
              {(models.available_models ?? []).length === 0 ? (
                <div className="oempty">No models registered.</div>
              ) : (
                <table className="otable">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Type</th>
                      <th>Provider</th>
                      <th>Identifier</th>
                      <th>Context</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {(models.available_models ?? []).map((m, i) => (
                      <tr key={i}>
                        <td style={{ color: "#e8eef6" }}>{m.name}</td>
                        <td>
                          <span className="ochip cat">{m.model_type}</span>
                        </td>
                        <td className="omono">{m.provider || "—"}</td>
                        <td className="omono">{m.identifier}</td>
                        <td className="omono">{m.context_length ?? "—"}</td>
                        <td>
                          <button
                            className="obtn"
                            title="Remove this model from the registry (and the chain)"
                            onClick={() => removeProviderModel(m.identifier)}
                          >
                            Remove
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            <div className="ocard">
              <h4>➕ Add a provider model</h4>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "0 0 8px" }}>
                Direct vendor APIs and vendor CLI agents are paid classes: the
                host accepts them only while "Allow paid models" is on, and
                each destination is judged by the jurisdiction gate. Nothing
                here joins a fallback chain until you put it there.
              </p>
              <div className="ofield">
                <label>Provider</label>
                <select
                  className="oinput"
                  value={addForm.provider}
                  onChange={(e) => setAddForm({ ...addForm, provider: e.target.value })}
                >
                  {PROVIDER_CHOICES.map((c) => (
                    <option key={c.value} value={c.value}>
                      {c.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="ofield">
                <label>Display name (optional)</label>
                <input
                  className="oinput"
                  value={addForm.name}
                  onChange={(e) => setAddForm({ ...addForm, name: e.target.value })}
                />
              </div>
              <div className="ofield">
                <label>Model identifier</label>
                <input
                  className="oinput"
                  placeholder={
                    PROVIDER_CHOICES.find((c) => c.value === addForm.provider)?.kind === "cli"
                      ? "optional — the CLI's model id (empty = its default)"
                      : "the vendor's model id"
                  }
                  value={addForm.identifier}
                  onChange={(e) => setAddForm({ ...addForm, identifier: e.target.value })}
                />
              </div>
              {PROVIDER_CHOICES.find((c) => c.value === addForm.provider)?.kind === "api" && (
                <div className="ofield">
                  <label>API key</label>
                  <input
                    className="oinput"
                    type="password"
                    value={addForm.api_key}
                    onChange={(e) => setAddForm({ ...addForm, api_key: e.target.value })}
                  />
                </div>
              )}
              <div className="ofield">
                <label>Context length (tokens, optional)</label>
                <input
                  className="oinput"
                  value={addForm.context_length}
                  onChange={(e) => setAddForm({ ...addForm, context_length: e.target.value.replace(/[^0-9]/g, "") })}
                />
              </div>
              <button className="obtn" onClick={addProviderModel}>
                Add model
              </button>
              {addNote && (
                <p style={{ color: "var(--color-text-muted)", fontSize: 12, marginTop: 8 }}>{addNote}</p>
              )}
            </div>

            <div className="ocard">
              <h4>🔗 Fallback chain</h4>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "0 0 8px" }}>
                Your order, tried top to bottom. Nothing is added for you; when
                the chain runs out it runs out. Press Save to apply.
              </p>
              {chainOrder.length === 0 ? (
                <div className="oempty">The chain is empty (single backend).</div>
              ) : (
                <table className="otable">
                  <tbody>
                    {chainOrder.map((id, i) => (
                      <tr key={id}>
                        <td className="omono">{i + 1}</td>
                        <td className="omono">{id}</td>
                        <td>
                          <button className="obtn" onClick={() => moveInChain(i, -1)} disabled={i === 0}>
                            ↑
                          </button>{" "}
                          <button
                            className="obtn"
                            onClick={() => moveInChain(i, 1)}
                            disabled={i === chainOrder.length - 1}
                          >
                            ↓
                          </button>{" "}
                          <button className="obtn" onClick={() => setChain(chainOrder.filter((x) => x !== id))}>
                            ✕
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              <div className="ofield" style={{ marginTop: 8 }}>
                <label>Add to the chain</label>
                <select
                  className="oinput"
                  value=""
                  onChange={(e) => {
                    if (e.target.value) setChain([...chainOrder, e.target.value]);
                  }}
                >
                  <option value="">Choose a registered model…</option>
                  {(models.available_models ?? [])
                    .filter((m) => !chainOrder.includes(m.identifier))
                    .map((m) => (
                      <option key={m.identifier} value={m.identifier}>
                        {m.name} ({m.identifier})
                      </option>
                    ))}
                </select>
              </div>
              <label className="otoggle" style={{ marginTop: 6 }}>
                <input
                  type="checkbox"
                  checked={models.fallback?.free_only ?? false}
                  onChange={(e) =>
                    setModels({
                      ...models,
                      fallback: { ...(models.fallback ?? {}), free_only: e.target.checked },
                    })
                  }
                />
                <span>Free models only</span>
              </label>
            </div>
          </div>
        )}

        {section === "voice" && (
          <div className="ocards">
            <div className="ocard">
              <h4>🎤 Voice</h4>
              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={voice.enabled ?? false}
                  onChange={(e) => setVoice({ ...voice, enabled: e.target.checked })}
                />
                <span>Voice input enabled</span>
              </label>
              <div className="ofield" style={{ marginTop: 12 }}>
                <label>Backend</label>
                <select
                  className="oinput"
                  value={voice.backend ?? "whisper_rs"}
                  onChange={(e) => setVoice({ ...voice, backend: e.target.value })}
                >
                  <option value="whisper_rs">whisper-rs (integrated)</option>
                  <option value="whisper_cpp">whisper.cpp (CLI)</option>
                  <option value="api">API</option>
                </select>
              </div>
              {voice.backend !== "api" && (
                <div className="ofield">
                  <label>Whisper model file</label>
                  <input
                    className="oinput"
                    value={voice.whisper_model_path ?? ""}
                    onChange={(e) =>
                      setVoice({ ...voice, whisper_model_path: e.target.value })
                    }
                  />
                </div>
              )}
              <div className="ofield">
                <label>whisper-cli binary</label>
                <input
                  className="oinput"
                  value={voice.whisper_cpp_path ?? ""}
                  onChange={(e) =>
                    setVoice({ ...voice, whisper_cpp_path: e.target.value })
                  }
                />
              </div>
              {voice.backend === "api" && (
                <>
                  <div className="ofield">
                    <label>Transcription endpoint</label>
                    <input
                      className="oinput"
                      value={voice.api_endpoint ?? ""}
                      onChange={(e) =>
                        setVoice({ ...voice, api_endpoint: e.target.value })
                      }
                    />
                  </div>
                  <div className="ofield">
                    <label>API key</label>
                    <input
                      className="oinput"
                      type="password"
                      value={voice.api_key ?? ""}
                      onChange={(e) => setVoice({ ...voice, api_key: e.target.value })}
                    />
                  </div>
                </>
              )}
              <div className="ofield">
                <label>Language hint (optional)</label>
                <input
                  className="oinput"
                  placeholder="en"
                  value={voice.language ?? ""}
                  onChange={(e) => setVoice({ ...voice, language: e.target.value })}
                />
              </div>
              <div className="ofield">
                <label>ffmpeg binary</label>
                <input
                  className="oinput"
                  value={voice.ffmpeg_path ?? ""}
                  onChange={(e) =>
                    setVoice({ ...voice, ffmpeg_path: e.target.value })
                  }
                />
              </div>
            </div>
          </div>
        )}

        {section === "consciousness" && (
          <div className="ocards">
            <div className="ocard">
              <h4>🧠 Consciousness</h4>
              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={consciousness.enabled ?? false}
                  onChange={(e) =>
                    setConsciousness({ ...consciousness, enabled: e.target.checked })
                  }
                />
                <span>Consciousness system enabled</span>
              </label>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12.5, margin: "8px 0 4px" }}>
                Sub-systems (only meaningful while enabled above):
              </p>
              {(
                [
                  ["emotional_system_enabled", "Emotional context"],
                  ["experience_memory_enabled", "Experience memory"],
                  ["identity_system_enabled", "Identity (self-model)"],
                  ["relationship_system_enabled", "Relationship development"],
                  ["ethical_system_enabled", "Ethical reasoning"],
                  ["collective_enabled", "Collective/shared learning"],
                  ["show_emotional_state", "Show emotional state in UI"],
                  ["show_decision_reasoning", "Show decision reasoning in UI"],
                  ["playback_enabled", "Experience playback"],
                ] as [keyof ConsciousnessCfg, string][]
              ).map(([key, label]) => (
                <label className="otoggle" key={key}>
                  <input
                    type="checkbox"
                    checked={Boolean(consciousness[key] ?? false)}
                    onChange={(e) =>
                      setConsciousness({ ...consciousness, [key]: e.target.checked })
                    }
                  />
                  <span>{label}</span>
                </label>
              ))}
              <div className="ofield" style={{ marginTop: 10 }}>
                <label>I-Loop interval (ms)</label>
                <input
                  className="oinput"
                  type="number"
                  min={1000}
                  value={consciousness.i_loop_interval_ms ?? ""}
                  onChange={(e) =>
                    setConsciousness({
                      ...consciousness,
                      i_loop_interval_ms: Number(e.target.value) || undefined,
                    })
                  }
                />
              </div>
            </div>
          </div>
        )}

        {section === "network" && (
          <div className="ocards">
            <div className="ocard">
              <h4>🌐 Network</h4>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "0 0 10px" }}>
                Changes here require a restart to take effect.
              </p>
              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={network.enable_p2p ?? false}
                  onChange={(e) =>
                    setNetwork({ ...network, enable_p2p: e.target.checked })
                  }
                />
                <span>Enable P2P networking</span>
              </label>
              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={network.enable_mdns ?? false}
                  onChange={(e) =>
                    setNetwork({ ...network, enable_mdns: e.target.checked })
                  }
                />
                <span>Enable mDNS local discovery</span>
              </label>
              <label className="otoggle">
                <input
                  type="checkbox"
                  checked={network.enable_cloud_sync ?? false}
                  onChange={(e) =>
                    setNetwork({ ...network, enable_cloud_sync: e.target.checked })
                  }
                />
                <span>Enable cloud sync</span>
              </label>
              <div className="ofield" style={{ marginTop: 10 }}>
                <label>P2P port</label>
                <input
                  className="oinput"
                  type="number"
                  value={network.p2p_port ?? ""}
                  onChange={(e) =>
                    setNetwork({
                      ...network,
                      p2p_port: Number(e.target.value) || undefined,
                    })
                  }
                />
              </div>
              <div className="ofield">
                <label>Max peers</label>
                <input
                  className="oinput"
                  type="number"
                  value={network.max_peers ?? ""}
                  onChange={(e) =>
                    setNetwork({
                      ...network,
                      max_peers: Number(e.target.value) || undefined,
                    })
                  }
                />
              </div>
              <div className="ofield">
                <label>Batch sync interval (secs)</label>
                <input
                  className="oinput"
                  type="number"
                  value={network.batch_sync_interval_secs ?? ""}
                  onChange={(e) =>
                    setNetwork({
                      ...network,
                      batch_sync_interval_secs: Number(e.target.value) || undefined,
                    })
                  }
                />
              </div>
            </div>
          </div>
        )}

        {section === "algorithms" && (
          <div className="ocards">
            <div className="ocard">
              <h4>🧮 K-Algorithm presets</h4>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "0 0 12px" }}>
                Applies live — no restart needed, the orchestrator picks it up
                on its next AMT pass. Only convergence and pairwise are shown:
                the other algorithm families (validation, ordered-loop, search)
                exist in the registry but nothing reads them yet, so a control
                for them wouldn't do anything.
              </p>
              <div className="ofield">
                <label>Convergence (AMT refinement pass limit)</label>
                <select
                  className="oinput"
                  value={algorithms.convergence_preset ?? ""}
                  onChange={(e) =>
                    setAlgorithms({ ...algorithms, convergence_preset: e.target.value })
                  }
                >
                  {(algorithms.convergence_options?.length
                    ? algorithms.convergence_options
                    : ["fast", "deep"]
                  ).map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="ofield">
                <label>Pairwise (branch comparison window)</label>
                <select
                  className="oinput"
                  value={algorithms.pairwise_preset ?? ""}
                  onChange={(e) =>
                    setAlgorithms({ ...algorithms, pairwise_preset: e.target.value })
                  }
                >
                  {(algorithms.pairwise_options?.length
                    ? algorithms.pairwise_options
                    : ["default", "wide"]
                  ).map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>
        )}

        {section === "advanced" && (
          <div className="ocards">
            <div className="ocard">
              <h4>⚙️ System info (read-only)</h4>
              <p style={{ color: "var(--color-text-muted)", fontSize: 12, margin: "0 0 10px" }}>
                These only take effect at process start — edit config.toml
                directly and restart if you need to change them.
              </p>
              <div className="ostats">
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.general?.data_dir ?? "—"}
                  </div>
                  <div className="ostat-label">Data dir</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.general?.log_level ?? "—"}
                  </div>
                  <div className="ostat-label">Log level</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.grpc?.address}:{advanced.grpc?.port}
                  </div>
                  <div className="ostat-label">gRPC bind</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.tasks?.max_queued_tasks ?? "—"}
                  </div>
                  <div className="ostat-label">Max queued tasks</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.tasks?.task_timeout_secs ?? "—"}s
                  </div>
                  <div className="ostat-label">Task timeout</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.integrity?.enabled ? "On" : "Off"}
                  </div>
                  <div className="ostat-label">Integrity monitor</div>
                </div>
                <div className="ostat">
                  <div className="ostat-num" style={{ fontSize: 13 }}>
                    {advanced.auth?.session_duration_secs ?? "—"}s
                  </div>
                  <div className="ostat-label">Session duration</div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {section !== "advanced" && (
        <div className="osave-row">
          <button className="obtn primary" onClick={save} disabled={saving}>
            {saving ? "Saving…" : "Save settings"}
          </button>
          {saved && <span className="osaved">✓ Saved to the host</span>}
        </div>
      )}
    </div>
  );
};

export default SettingsPanel;
