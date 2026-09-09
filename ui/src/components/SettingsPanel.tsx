/**
 * SettingsPanel — application settings, straight from the host config.
 *
 * Reads the live config (GET /config/get) and writes typed sections back
 * (POST /config/set): models, voice, consciousness. The desktop gets its
 * session injected by the bridge; browsers send the pairing token.
 */
import React, { useEffect, useState } from "react";

interface VoiceCfg {
  enabled?: boolean;
  backend?: string;
  whisper_model_path?: string;
  whisper_cpp_path?: string;
  api_endpoint?: string;
}
interface ModelsCfg {
  model_type?: string;
  api_provider?: string;
  api_key?: string;
  local_model_path?: string;
  local_model_type?: string;
}

export const SettingsPanel: React.FC = () => {
  const [models, setModels] = useState<ModelsCfg>({});
  const [voice, setVoice] = useState<VoiceCfg>({});
  const [consciousnessEnabled, setConsciousnessEnabled] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const oz = (window as any).ozone;
        let out: any;
        if (oz?.config?.get) {
          out = await oz.config.get();
        } else {
          out = await fetch(
            `${
              (window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"
            }/config/get`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ section: "", session_token: "" }),
            },
          ).then((r) => r.json());
        }
        const cfg = out?.config ?? out;
        setModels(cfg?.models ?? {});
        setVoice({
          enabled: cfg?.voice?.enabled ?? false,
          backend: cfg?.voice?.backend ?? "whisper_rs",
          whisper_model_path: cfg?.voice?.whisper_model_path ?? "",
          whisper_cpp_path: cfg?.voice?.whisper_cpp_path ?? "",
          api_endpoint: cfg?.voice?.api_endpoint ?? "",
        });
        setConsciousnessEnabled(cfg?.consciousness?.enabled ?? false);
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
        local_model_path: models.local_model_path || undefined,
        local_model_type: models.local_model_type,
      },
      voice: {
        enabled: voice.enabled ?? false,
        backend: voice.backend ?? "whisper_rs",
        whisper_model_path: voice.whisper_model_path || undefined,
        whisper_cpp_path: voice.whisper_cpp_path || undefined,
        api_endpoint: voice.api_endpoint || undefined,
      },
      consciousness: { enabled: consciousnessEnabled },
    };
    try {
      const oz = (window as any).ozone;
      let out: any;
      if (oz?.config?.set) {
        out = await oz.config.set(updates);
      } else {
        out = await fetch(
          `${
            (window as any).OZONE_HOST_URL || "http://127.0.0.1:50051"
          }/config/set`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ updates, session_token: "" }),
          },
        ).then((r) => r.json());
      }
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

      {error && (
        <p style={{ color: "#f87171", fontSize: 13 }}>{error}</p>
      )}

      <div className="ocards">
        <div className="ocard">
          <h4>⚡ Model</h4>
          <div className="ofield">
            <label>Model source</label>
            <select
              className="oinput"
              value={models.model_type ?? ""}
              onChange={(e) =>
                setModels({ ...models, model_type: e.target.value })
              }
            >
              <option value="zcode">ZCode (registry model)</option>
              <option value="api">API model</option>
              <option value="local">Local model</option>
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
                </select>
              </div>
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
            </>
          )}
          {models.model_type === "local" && (
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
            <p style={{ color: "#8b98ab", fontSize: 12.5, margin: 0 }}>
              Model calls dispatch to the ZCode connector registered on this
              host (roles: agent + model).
            </p>
          )}
        </div>

        <div className="ocard">
          <h4>🎤 Voice</h4>
          <div className="ofield">
            <label className="otoggle">
              <input
                type="checkbox"
                checked={voice.enabled ?? false}
                onChange={(e) =>
                  setVoice({ ...voice, enabled: e.target.checked })
                }
              />
              <span>Voice input enabled</span>
            </label>
          </div>
          <div className="ofield">
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
          {voice.backend === "whisper_cpp" && (
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
          )}
          {voice.backend === "api" && (
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
          )}
        </div>

        <div className="ocard">
          <h4>🧠 Consciousness</h4>
          <div className="ofield">
            <label className="otoggle">
              <input
                type="checkbox"
                checked={consciousnessEnabled}
                onChange={(e) => setConsciousnessEnabled(e.target.checked)}
              />
              <span>Consciousness system enabled</span>
            </label>
          </div>
          <p style={{ color: "#8b98ab", fontSize: 12.5, margin: 0 }}>
            Emotional context, experience memory, I-Loop reflection,
            relationship development, ethical framework.
          </p>
        </div>
      </div>

      <div className="osave-row">
        <button className="obtn primary" onClick={save} disabled={saving}>
          {saving ? "Saving…" : "Save settings"}
        </button>
        {saved && <span className="osaved">✓ Saved to the host</span>}
      </div>
    </div>
  );
};

export default SettingsPanel;
