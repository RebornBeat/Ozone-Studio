/**
 * PairingPanel — multi-device onboarding, the phone as authenticator.
 * QR flow + the host's device registry, on the shared design system.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "qrcode";
import {
  DeviceInfo,
  listDevices,
  pairingStatus,
  startPairing,
} from "../ozoneClient";

type Phase = "idle" | "waiting" | "approved" | "error";

export const PairingPanel: React.FC = () => {
  const [phase, setPhase] = useState<Phase>("idle");
  const [code, setCode] = useState("");
  const [qrDataUrl, setQrDataUrl] = useState("");
  const [approveUrl, setApproveUrl] = useState("");
  const [expiresAt, setExpiresAt] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const pairingIdRef = useRef<string>("");
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshDevices = useCallback(async () => {
    try {
      const out = await listDevices();
      setDevices(out.devices ?? []);
    } catch {
      setDevices([]);
    }
  }, []);

  useEffect(() => {
    refreshDevices();
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [refreshDevices]);

  const beginPairing = async () => {
    setError("");
    try {
      const session = await startPairing("Desktop / Web UI");
      const url = QRCode.toDataURL(session.approve_url, {
        width: 220,
        margin: 1,
        color: { dark: "#0b0e14", light: "#ffffff" },
      }).catch(() => "");
      pairingIdRef.current = session.pairing_id;
      setCode(session.code);
      setQrDataUrl(await url);
      setApproveUrl(session.approve_url);
      setExpiresAt(session.expires_at);
      setPhase("waiting");

      if (pollRef.current) clearInterval(pollRef.current);
      pollRef.current = setInterval(async () => {
        try {
          const st = await pairingStatus(pairingIdRef.current);
          if (st.status === "approved" && st.session_token) {
            clearInterval(pollRef.current!);
            try {
              localStorage.setItem("ozone_session_token", st.session_token);
              localStorage.setItem("ozone_device_id", String(st.device_id ?? ""));
            } catch {
              /* storage unavailable — token still valid for this tab */
            }
            setPhase("approved");
            refreshDevices();
          } else if (st.status === "expired" || st.status === "unknown") {
            clearInterval(pollRef.current!);
            setPhase("error");
            setError("Pairing window expired — start again.");
          }
        } catch {
          /* transient — keep polling */
        }
      }, 1500);
    } catch (e: any) {
      setPhase("error");
      setError(e?.message ?? "Failed to start pairing");
    }
  };

  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className="odot ok" />
        <span className="opanel-title">Device Pairing</span>
      </div>
      <p className="opanel-sub">
        Your phone is the authenticator: scan, approve, and this device holds a
        real session — same store as key-based logins, nothing typed anywhere.
      </p>

      {phase === "idle" && (
        <button className="obtn primary" onClick={beginPairing}>
          Pair this device
        </button>
      )}

      {phase === "waiting" && (
        <div
          style={{
            display: "flex",
            gap: 24,
            alignItems: "center",
            flexWrap: "wrap",
          }}
        >
          <div
            style={{
              background: "#fff",
              padding: 10,
              borderRadius: 12,
              lineHeight: 0,
            }}
          >
            {qrDataUrl && (
              <img src={qrDataUrl} alt="Pairing QR" width={196} height={196} />
            )}
          </div>
          <div>
            <div style={{ color: "#8b98ab", fontSize: 13, marginBottom: 6 }}>
              Scan with your phone, then approve on it.
            </div>
            <div
              style={{
                fontFamily: "ui-monospace, monospace",
                fontSize: 28,
                letterSpacing: 6,
                color: "#6ec3ff",
                marginBottom: 8,
              }}
            >
              {code}
            </div>
            <div style={{ color: "#66748a", fontSize: 12, marginBottom: 8 }}>
              {expiresAt
                ? `Valid until ${new Date(expiresAt * 1000).toLocaleTimeString()}`
                : ""}{" "}
              · waiting for approval…
            </div>
            {/* Camera can't scan? Open the link on the phone directly. */}
            <div style={{ fontSize: 12 }}>
              <span style={{ color: "#8b98ab" }}>No camera? Open on your phone: </span>
              <a
                href={approveUrl}
                target="_blank"
                rel="noreferrer"
                style={{ color: "#6ec3ff", wordBreak: "break-all" }}
              >
                {approveUrl}
              </a>
            </div>
          </div>
        </div>
      )}

      {phase === "approved" && (
        <div style={{ color: "#4ade80", fontSize: 14 }}>
          ✓ Device paired — session active on this host.{" "}
          <button
            className="obtn subtle"
            onClick={() => {
              setPhase("idle");
              refreshDevices();
            }}
          >
            Pair another
          </button>
        </div>
      )}

      {phase === "error" && (
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span style={{ color: "#f87171", fontSize: 13 }}>{error}</span>
          <button className="obtn subtle" onClick={() => setPhase("idle")}>
            Back
          </button>
        </div>
      )}

      <h4
        style={{
          margin: "20px 0 8px",
          color: "#aebdce",
          fontSize: 12,
          textTransform: "uppercase",
          letterSpacing: 0.7,
        }}
      >
        Paired Devices ({devices.length})
      </h4>
      {devices.length === 0 ? (
        <div className="oempty">
          No devices yet — pair this one, or scan from the phone to anchor the
          registry.
        </div>
      ) : (
        <table className="otable">
          <thead>
            <tr>
              <th>ID</th>
              <th>Name</th>
              <th>Type</th>
              <th>Last seen</th>
            </tr>
          </thead>
          <tbody>
            {devices.map((d) => (
              <tr key={d.device_id}>
                <td className="omono">{d.device_id}</td>
                <td style={{ color: "#e8eef6" }}>{d.device_name}</td>
                <td>
                  <span className="ochip cat">{d.device_type}</span>
                </td>
                <td style={{ opacity: 0.75 }}>
                  {new Date(d.last_seen * 1000).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default PairingPanel;
