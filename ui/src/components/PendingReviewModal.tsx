/**
 * PendingReviewModal — the ONLY place a human decides a gate: reviews the
 * host marks human-actionable (jurisdiction RequireConfirmation). Consciousness
 * /meta gates are never shown here as a decision; they await meta.
 * Resolving calls /consciousness/reviews/resolve, which resumes the walk.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { fetchPendingReviews, resolveReview, type PendingReviewRow } from "../ozoneClient";

const isHuman = (r: PendingReviewRow) =>
  r.resolver === "human" || (r.review_kind ?? "").toLowerCase().includes("jurisdiction") ||
  (r.decision ?? "").toLowerCase().includes("confirm");

export const PendingReviewModal: React.FC = () => {
  const [rows, setRows] = useState<PendingReviewRow[]>([]);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const seen = useRef<Set<string>>(new Set());

  const refresh = useCallback(async () => {
    try {
      const r = await fetchPendingReviews();
      const human = (r.rows ?? []).filter(isHuman);
      setRows(human);
      const fresh = human.some((x) => {
        const id = x.review_id ?? x.id ?? "";
        if (seen.current.has(id)) return false;
        seen.current.add(id);
        return true;
      });
      if (fresh) setOpen(true);
    } catch {
      /* route gated/absent: stay silent, no invented state */
    }
  }, []);

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 10000);
    const onOpen = () => setOpen(true);
    window.addEventListener("ozone:open-pending-reviews", onOpen);
    return () => { clearInterval(t); window.removeEventListener("ozone:open-pending-reviews", onOpen); };
  }, [refresh]);

  const resolve = async (r: PendingReviewRow, decision: "Proceed" | "Decline") => {
    const id = r.review_id ?? r.id;
    if (!id) return;
    setBusy(id); setErr(null);
    try {
      const res = await resolveReview(id, decision);
      if (res?.error) setErr(String(res.error));
      await refresh();
    } catch (e: any) {
      setErr(e?.message ?? String(e));
    } finally { setBusy(null); }
  };

  if (!open || rows.length === 0) return null;
  return (
    <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,.55)", zIndex: 9999, display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div style={{ background: "var(--color-surface, #1b1b1f)", border: "1px solid var(--color-border)", borderRadius: 8, padding: 16, width: 560, maxHeight: "80vh", overflow: "auto" }}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
          <strong>Confirmation required ({rows.length})</strong>
          <button onClick={() => setOpen(false)} style={{ cursor: "pointer" }}>Later</button>
        </div>
        {err && <div style={{ color: "#ff8a8a", fontSize: 12, marginBottom: 6 }}>{err}</div>}
        {rows.map((r) => {
          const id = r.review_id ?? r.id ?? "";
          return (
            <div key={id} style={{ borderTop: "1px solid var(--color-border)", padding: "8px 0", fontSize: 12 }}>
              <div style={{ opacity: 0.7 }}>{r.review_kind ?? "review"} · {r.ts ?? ""}</div>
              <div style={{ margin: "4px 0" }}>{r.task_summary_preview ?? "(no summary recorded)"}</div>
              {r.reasoning_preview && <div style={{ opacity: 0.8 }}>{r.reasoning_preview}</div>}
              <div style={{ marginTop: 6, display: "flex", gap: 8 }}>
                <button disabled={busy === id} onClick={() => resolve(r, "Proceed")}>Proceed</button>
                <button disabled={busy === id} onClick={() => resolve(r, "Decline")}>Decline</button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default PendingReviewModal;
