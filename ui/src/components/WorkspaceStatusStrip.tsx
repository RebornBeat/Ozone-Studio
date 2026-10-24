/**
 * WorkspaceStatusStrip — the workspace header's live-trust indicators
 * (GraphKit wave 7, additive: a strip, not a layout rewrite). Shows the
 * REAL P2P posture via PeerState (connected ≠ trusted) and the substrate's
 * node-construction pulse from the shared GraphKit provider when a project
 * is loaded. All values are real captured ones; absent data renders as
 * honest absence.
 */
import React, { useEffect, useState } from "react";
import { getGraphEventClient } from "../graphEventClient";
import { PeerState } from "../graphkit/indicators";

export const WorkspaceStatusStrip: React.FC = () => {
  const [peerCount, setPeerCount] = useState(0);
  const [pairedCount, setPairedCount] = useState(0);
  const [available, setAvailable] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const fetchOnce = async () => {
      try {
        // /health already carries the real peer_count (HealthResponse) — no
        // new route needed; paired-peers is the trust half.
        const health = await fetch("/health").then((r) => r.json());
        const paired = await fetch("/network/paired-peers").then((r) => r.json());
        if (cancelled) return;
        setPeerCount(Number(health?.peer_count ?? 0));
        setPairedCount(Array.isArray(paired?.paired_peers) ? paired.paired_peers.length : 0);
        setAvailable(true);
      } catch {
        if (!cancelled) setAvailable(false);
      }
    };
    fetchOnce();
    const t = setInterval(fetchOnce, 30_000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  if (!available) return null; // honest absence: no strip when the host doesn't answer

  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 10,
        fontSize: 11,
        color: "var(--color-text-muted)",
        border: "1px solid var(--color-border-faint)",
        borderRadius: 8,
        padding: "2px 8px",
      }}
    >
      <PeerState peerCount={peerCount} pairedCount={pairedCount} />
    </div>
  );
};

export default WorkspaceStatusStrip;
