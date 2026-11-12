/**
 * JurisdictionPolicyPanel — the instance's OWN jurisdiction policy (GET/POST /jurisdiction/*).
 *
 * Hardening is not one fixed rule: every instance has its own agreements and requirements, so every enforcement point reads this versioned,
 * durable policy. This panel shows it, lets the operator accept each requirement (re-asked when a requirement's version/text changes), decide
 * held outbound transfers, read the append-only decision log (hashes and rule vocabulary only — never request content), and run the erasure
 * path. `compact` is the setup-wizard form (requirements + residency only). Engineering input, NOT legal advice.
 */
import React, { useCallback, useEffect, useState } from "react";
import {
  JurisdictionDecision,
  JurisdictionPolicy,
  JurisdictionPolicyResponse,
  applyErasure,
  decideOutboundTransfer,
  exportJurisdictionPolicy,
  fetchJurisdictionDecisions,
  fetchJurisdictionPolicy,
  planErasure,
  saveJurisdictionPolicy,
  setJurisdictionAgreement,
  ErasurePlanResponse,
} from "../ozoneClient";

const card: React.CSSProperties = { border: "1px solid #2a3a4d", borderRadius: 8, padding: 12, background: "#101923" };
const chip: React.CSSProperties = { display: "inline-block", padding: "1px 8px", borderRadius: 10, border: "1px solid #2f4258", fontSize: 11.5, color: "#9fb6cf", marginRight: 6 };
const okChip: React.CSSProperties = { ...chip, borderColor: "#2e7d5b", color: "#7fd1a8" };
const warnChip: React.CSSProperties = { ...chip, borderColor: "#a35a2a", color: "#f0b27a" };
const btn: React.CSSProperties = { padding: "3px 10px", borderRadius: 6, border: "1px solid #3a5470", background: "#162434", color: "#d7e4f3", cursor: "pointer", fontSize: 12.5 };
const input: React.CSSProperties = { background: "#0c141c", border: "1px solid #2a3a4d", borderRadius: 6, color: "#d7e4f3", padding: "3px 8px", fontSize: 12.5 };
const label: React.CSSProperties = { fontSize: 12, color: "#8fa7bf", display: "block", marginBottom: 2 };

export const JurisdictionPolicyPanel: React.FC<{ compact?: boolean; pollMs?: number }> = ({ compact = false, pollMs = 20000 }) => {
  const [data, setData] = useState<JurisdictionPolicyResponse | null>(null);
  const [draft, setDraft] = useState<JurisdictionPolicy | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [decisions, setDecisions] = useState<JurisdictionDecision[]>([]);
  const [needle, setNeedle] = useState("");
  const [plan, setPlan] = useState<ErasurePlanResponse | null>(null);
  const [confirm, setConfirm] = useState("");

  const load = useCallback(async () => {
    try {
      const r = await fetchJurisdictionPolicy();
      setData(r);
      setDraft((d) => d ?? r.policy);
      setErr(null);
      if (!compact) {
        const dec = await fetchJurisdictionDecisions(0);
        setDecisions([...dec.decisions].sort((a, b) => b.at - a.at));
      }
    } catch (e: any) {
      setErr(String(e?.message ?? e));
    }
  }, [compact]);

  useEffect(() => {
    load();
    const t = setInterval(load, pollMs);
    return () => clearInterval(t);
  }, [load, pollMs]);

  if (err && !data) return <div className="oempty">Jurisdiction policy could not be read: {err}</div>;
  if (!data || !draft) return <div className="oempty">Loading jurisdiction policy…</div>;

  const agree = async (key: string, accepted: boolean) => {
    const r = await setJurisdictionAgreement(key, accepted);
    setMsg(r.success ? `${accepted ? "Accepted" : "Withdrawn"}: ${key}` : `Failed: ${r.error}`);
    load();
  };
  const save = async () => {
    const r = await saveJurisdictionPolicy(draft);
    setMsg(r.success ? `Saved policy version ${r.version}` : `Not saved: ${r.error}`);
    setDraft(null);
    load();
  };
  const set = <K extends keyof JurisdictionPolicy>(k: K, v: JurisdictionPolicy[K]) => setDraft({ ...draft, [k]: v });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div className="oempty" style={{ textAlign: "left" }}>
        Every instance hardens itself: the gates read THIS policy (version {data.policy.version}, {data.policy.origin}, fingerprint {data.fingerprint}). Defaults are on; change what your own
        agreements and requirements need. This is engineering configuration, not legal advice — have qualified counsel review it.
      </div>
      {msg && <div style={{ ...card, color: "#9fd4b4" }}>{msg}</div>}
      {data.region_known === false && (
        <div style={{ ...card, color: "#f0c987" }}>
          <strong>This instance's region is not set or detected.</strong> Only Global (U.N.-level) rules apply, and the policy action for an unknown
          jurisdiction is <span className="omono">{data.policy.unknown_jurisdiction}</span>. Set <span className="omono">jurisdiction.instance_region</span> in
          the instance configuration (for example <span className="omono">IE</span> or <span className="omono">US-CA</span>) so national and local rules apply.
        </div>
      )}

      <div style={card}>
        <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>
          Requirements — accept each one ({data.unaccepted.length === 0 ? "all current requirements accepted" : `${data.unaccepted.length} awaiting you`})
        </div>
        {data.agreements.map((a) => (
          <div key={a.key} style={{ marginBottom: 10 }}>
            <div>
              <strong>{a.title}</strong> <span style={chip}>v{a.version}</span>
              {a.accepted_current ? <span style={okChip}>accepted by {a.by ?? "operator"}</span> : <span style={warnChip}>{a.stale ? "changed — re-accept" : "not accepted"}</span>}
            </div>
            <div style={{ fontSize: 12.5, color: "#c9d6e6", margin: "4px 0" }}>{a.text}</div>
            {a.source && <div style={{ fontSize: 11.5, color: "#7f94ab" }}>source: {a.source}</div>}
            <div style={{ marginTop: 4 }}>
              {a.accepted_current ? (
                <button style={btn} onClick={() => agree(a.key, false)}>Withdraw</button>
              ) : (
                <button style={btn} onClick={() => agree(a.key, true)}>I accept this requirement</button>
              )}
            </div>
          </div>
        ))}
        {data.agreements.length === 0 && <div className="oempty">This policy lists no requirements.</div>}
      </div>

      <div style={card}>
        <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Residency</div>
        <label style={label}>Hosting region (where this instance runs)</label>
        <input style={{ ...input, width: "100%" }} value={draft.hosting_region} onChange={(e) => set("hosting_region", e.target.value)} />
        <label style={{ ...label, marginTop: 8 }}>Residency note (provider, datacentre, DPA reference…)</label>
        <textarea style={{ ...input, width: "100%", minHeight: 48 }} value={draft.residency_note} onChange={(e) => set("residency_note", e.target.value)} />
        {compact && <div style={{ marginTop: 8 }}><button style={btn} onClick={save}>Save residency</button></div>}
      </div>

      {!compact && (
        <>
          <div style={card}>
            <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Outbound policy (what may leave this host, and where to)</div>
            <label style={label}>Provider regions (endpoint contains → region label)</label>
            {draft.providers.map((p, i) => (
              <div key={i} style={{ display: "flex", gap: 6, marginBottom: 4 }}>
                <input style={{ ...input, flex: 1 }} value={p.endpoint_contains} onChange={(e) => set("providers", draft.providers.map((x, j) => (j === i ? { ...x, endpoint_contains: e.target.value } : x)))} />
                <input style={{ ...input, width: 220 }} value={p.region} onChange={(e) => set("providers", draft.providers.map((x, j) => (j === i ? { ...x, region: e.target.value } : x)))} />
              </div>
            ))}
            <button style={btn} onClick={() => set("providers", [...draft.providers, { endpoint_contains: "", region: "" }])}>Add provider rule</button>
            <label style={{ ...label, marginTop: 8 }}>Regions personal data may go to (comma separated)</label>
            <input style={{ ...input, width: "100%" }} value={draft.allow_personal_data_regions.join(", ")} onChange={(e) => set("allow_personal_data_regions", e.target.value.split(",").map((s) => s.trim()).filter(Boolean))} />
            <label style={{ ...label, marginTop: 8 }}>When personal data is bound for another region</label>
            <select style={input} value={draft.on_personal_data_outbound} onChange={(e) => set("on_personal_data_outbound", e.target.value)}>
              <option value="await_operator">hold and wait for me (default)</option>
              <option value="block">block</option>
              <option value="allow_logged">allow, logged</option>
            </select>
            <label style={{ ...label, marginTop: 8 }}>When no jurisdiction is configured or detected</label>
            <select style={input} value={draft.unknown_jurisdiction} onChange={(e) => set("unknown_jurisdiction", e.target.value)}>
              <option value="global_only_flagged">Global rules only, flagged (default)</option>
              <option value="await_operator">hold and wait for me</option>
              <option value="block">block</option>
            </select>
            <div style={{ marginTop: 8, display: "flex", flexWrap: "wrap", gap: 12, fontSize: 12.5 }}>
              <label><input type="checkbox" checked={draft.enabled} onChange={(e) => set("enabled", e.target.checked)} /> outbound gate on</label>
              <label><input type="checkbox" checked={draft.require_agreements_before_outbound} onChange={(e) => set("require_agreements_before_outbound", e.target.checked)} /> require accepted requirements before third-country calls</label>
              <label><input type="checkbox" checked={draft.detection.deterministic} onChange={(e) => set("detection", { ...draft.detection, deterministic: e.target.checked })} /> detect personal-data shapes</label>
              <label><input type="checkbox" checked={draft.detection.vote} onChange={(e) => set("detection", { ...draft.detection, vote: e.target.checked })} /> yes/no vote on an in-region model</label>
            </div>
            <label style={{ ...label, marginTop: 8 }}>Vote model (identifier) and its region — the vote sends the text to that model, so it runs only if that region is allowed above</label>
            <div style={{ display: "flex", gap: 6 }}>
              <input style={{ ...input, flex: 1 }} value={draft.detection.vote_model} onChange={(e) => set("detection", { ...draft.detection, vote_model: e.target.value })} />
              <input style={{ ...input, width: 160 }} value={draft.detection.vote_model_region} onChange={(e) => set("detection", { ...draft.detection, vote_model_region: e.target.value })} />
            </div>
            <label style={{ ...label, marginTop: 8 }}>
              What this instance may publish to peers (comma separated item kinds; empty = methodology, jurisdiction_rule_set, pipeline, tool). Published items cannot be
              recalled. Add <span className="omono">jurisdiction_policy</span> only if you want to offer your own policy as a manifest (peers can only stage it).
            </label>
            <input style={{ ...input, width: "100%" }} value={draft.p2p.publish_kinds.join(", ")} onChange={(e) => set("p2p", { ...draft.p2p, publish_kinds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
            <div style={{ marginTop: 6, display: "flex", flexWrap: "wrap", gap: 12, fontSize: 12.5 }}>
              <label><input type="checkbox" checked={draft.p2p.share_policy_as_offer} onChange={(e) => set("p2p", { ...draft.p2p, share_policy_as_offer: e.target.checked })} /> allow offering my policy as a manifest</label>
              <label><input type="checkbox" checked={draft.p2p.recall_notice} onChange={(e) => set("p2p", { ...draft.p2p, recall_notice: e.target.checked })} /> show the no-recall notice</label>
            </div>
            <div style={{ marginTop: 10 }}>
              <button style={btn} onClick={save}>Save as new policy version</button>{" "}
              <button style={btn} onClick={() => setDraft(data.policy)}>Reset edits</button>{" "}
              <button style={btn} onClick={async () => { const r = await exportJurisdictionPolicy(); setMsg("Export ready (staged-file shape): " + JSON.stringify(r.content).length + " bytes — copy it from the API response to share as a manifest"); }}>Export for sharing</button>
            </div>
          </div>

          <div style={card}>
            <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Held for you ({data.holds.length})</div>
            {data.holds.length === 0 && <div className="oempty">Nothing is waiting on an operator decision.</div>}
            {data.holds.map((h) => (
              <div key={h.await_id} style={{ marginBottom: 8 }}>
                <span style={warnChip}>{h.kind}</span> <span style={{ fontSize: 12.5 }}>{h.reason}</span>
                {h.kind === "outbound_transfer" && (
                  <div style={{ marginTop: 4 }}>
                    <button style={btn} onClick={() => decideOutboundTransfer("content", h.subject, true, h.await_id).then(load)}>Approve this content</button>{" "}
                    <button style={btn} onClick={() => decideOutboundTransfer("content", h.subject, false, h.await_id).then(load)}>Deny</button>
                  </div>
                )}
              </div>
            ))}
          </div>

          <div style={card}>
            <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Peer sharing</div>
            <div style={{ fontSize: 12.5, color: "#c9d6e6" }}>
              Anything published to peers (manifests, tool findings, rule sets) reaches other instances and <strong>cannot be recalled</strong> once received. Blueprints never travel, and peers
              act only through their own operator gate. Do not publish anything containing personal data.
            </div>
          </div>

          <div style={card}>
            <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Erasure</div>
            <div style={{ fontSize: 12.5, color: "#c9d6e6", marginBottom: 6 }}>
              Plan is read-only (counts only). Apply replaces the text with [erased] in this instance's data files. It does not reach store-held container copies, indexes, peers, provider-side copies or backups — the plan lists them.
            </div>
            <input style={{ ...input, width: 260 }} placeholder="text to find (4+ characters)" value={needle} onChange={(e) => setNeedle(e.target.value)} />{" "}
            <button style={btn} onClick={async () => setPlan(await planErasure(needle))}>Plan</button>
            {plan && (
              <div style={{ marginTop: 8, fontSize: 12.5 }}>
                {plan.error && <div style={warnChip}>{plan.error}</div>}
                {plan.plan && (
                  <>
                    <div>{plan.plan.total_matches} match(es) in {plan.plan.files.length} file(s)</div>
                    {plan.plan.files.map((f) => <div key={f.path} className="omono">{f.path} · {f.matches} · {f.kind}</div>)}
                    <div style={{ marginTop: 6, color: "#f0b27a" }}>Not covered: {plan.plan.not_covered.join(" · ")}</div>
                    <div style={{ marginTop: 6 }}>
                      <input style={{ ...input, width: 140 }} placeholder='type ERASE' value={confirm} onChange={(e) => setConfirm(e.target.value)} />{" "}
                      <button style={btn} onClick={async () => { setPlan(await applyErasure(needle, confirm)); setConfirm(""); }}>Apply erasure</button>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>

          <div style={card}>
            <div className="omono" style={{ fontWeight: 600, marginBottom: 6 }}>Decision log ({decisions.length}) — append-only, never rotated, no content</div>
            <div style={{ maxHeight: 280, overflow: "auto" }}>
              {decisions.map((d, i) => (
                <div key={i} className="omono" style={{ fontSize: 11.5, color: "#b7c8da", marginBottom: 2 }}>
                  {new Date(d.at).toISOString()} · {d.entry_point} · <strong>{d.outcome}</strong> · policy v{d.policy_version}
                  {d.matched.length > 0 ? ` · ${d.matched.join(", ")}` : ""}{d.region ? ` · ${d.region}` : d.region_known ? "" : " · region n/a"}
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default JurisdictionPolicyPanel;
