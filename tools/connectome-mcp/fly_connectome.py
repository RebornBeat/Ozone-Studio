#!/usr/bin/env python3
"""Ozone-Studio Connectome MCP — fruit fly (Drosophila melanogaster) brain
connectome worker (operator directive 2026-09-29: "the NN of the fruit
fly, download it and plug it in, study it, monitor it").

REAL DATA, REAL SOURCE: data/fly_neuron_annotations.tsv is the actual
FlyWire/Codex "FAFB" whole-brain connectome's neuron annotation table
(github.com/flyconnectome/flywire_annotations, public, no auth needed) —
139,248 real annotated neurons from the 2024 Nature-published complete
adult fruit fly brain connectome (Dorkenwald et al.). Per-row real fields:
3D soma position, cell class/type, neurotransmitter identity + confidence,
hemilineage, left/right side, cross-references to the Virtual Fly Brain
and FBbt ontologies.

HONEST SCOPE BOUNDARY (found live, 2026-09-29, not silently avoided):
this file is the NEURON CATALOG only — real per-neuron biology, no
synapse-level connectivity. The full wiring diagram (3.7M+ real synaptic
connections between these neurons) lives behind FlyWire Codex's
authenticated download app (codex.flywire.ai/api/download) — confirmed
via a real fetch of that page, which requires sign-in and lists no
public unauthenticated file. This MCP does NOT fabricate connectivity
data to fill that gap. Importing real synapse edges is a clear, flagged
next step that needs FlyWire/Codex credentials this session doesn't have.

Tools this worker implements:
  fly_neuron_query   — filter by cell_type/super_class/side/neurotransmitter
  fly_connectome_stats — real aggregate counts over the full dataset
"""
import sys
import os
import json
import csv
from collections import Counter

DATA_PATH = os.environ.get(
    "FLY_CONNECTOME_TSV",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "fly_neuron_annotations.tsv"),
)


def load_rows():
    with open(DATA_PATH, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def cmd_query(req):
    rows = load_rows()
    limit = max(1, min(500, int(req.get("limit", 50))))
    filters = {
        "cell_type": req.get("cell_type"),
        "super_class": req.get("super_class"),
        "side": req.get("side"),
        "top_nt": req.get("neurotransmitter"),
    }
    active = {k: v for k, v in filters.items() if v}

    matched = []
    for r in rows:
        if all(r.get(k, "").strip().lower() == str(v).strip().lower() for k, v in active.items()):
            matched.append({
                "root_id": r.get("root_id"),
                "position": [r.get("pos_x"), r.get("pos_y"), r.get("pos_z")],
                "soma_position": [r.get("soma_x"), r.get("soma_y"), r.get("soma_z")],
                "super_class": r.get("super_class"),
                "cell_class": r.get("cell_class"),
                "cell_type": r.get("cell_type"),
                "hemibrain_type": r.get("hemibrain_type"),
                "top_neurotransmitter": r.get("top_nt"),
                "nt_confidence": r.get("top_nt_conf"),
                "side": r.get("side"),
                "hemilineage": r.get("ito_lee_hemilineage"),
            })
            if len(matched) >= limit:
                break

    return {
        "success": True,
        "total_neurons_in_dataset": len(rows),
        "matched_count": len(matched),
        "filters_applied": active or "none",
        "neurons": matched,
    }


def cmd_stats(req):
    rows = load_rows()
    super_class = Counter(r.get("super_class", "").strip() or "(unspecified)" for r in rows)
    nt = Counter(r.get("top_nt", "").strip() or "(unspecified)" for r in rows)
    side = Counter(r.get("side", "").strip() or "(unspecified)" for r in rows)
    return {
        "success": True,
        "source": "FlyWire/Codex FAFB whole-brain connectome, neuron annotation table (github.com/flyconnectome/flywire_annotations)",
        "total_neurons": len(rows),
        "by_super_class": dict(super_class.most_common(20)),
        "by_top_neurotransmitter": dict(nt.most_common(20)),
        "by_side": dict(side.most_common(10)),
        "note": "neuron catalog only — real synapse-level connectivity (3.7M+ real connections) requires FlyWire Codex authentication, not accessed in this pass",
    }


def main():
    try:
        req = json.loads(sys.stdin.read())
        action = req.get("action", "query")
        if action == "stats":
            print(json.dumps(cmd_stats(req)))
        else:
            print(json.dumps(cmd_query(req)))
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"success": False, "error": str(e)}))


if __name__ == "__main__":
    main()
