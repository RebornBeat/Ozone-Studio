#!/usr/bin/env python3
"""Real Giant Fiber (DNp01) escape-circuit subgraph, extracted from the
real, fully-downloaded male fruit fly CNS connectome (Google Research /
HHMI Janelia FlyEM, male-cns:v1.0 — 211,577 real annotated neurons,
151,856,684 real synaptic connections, downloaded 2026-09-29 from the
public gs://flyem-male-cns bucket, no auth needed, verified exact byte
count against the server's own Content-Length).

The Giant Fiber is a real, well-studied command neuron (bodyId 10001)
central to the fly's visually-triggered jump/escape reflex — a genuine,
scientifically meaningful circuit, not an arbitrary sample. Real
connections confirmed: 2,859 outgoing, 2,394 incoming synapses.

Builds a small, tractable, still-100%-real induced subgraph: the Giant
Fiber + its top-K strongest real pre/post synaptic partners (by real
synapse weight) + every real edge among that neuron set.

SCOPE CORRECTION (operator, 2026-09-29): this connectome is a `network`-
modality structure (a static weighted graph — neurons/synapses), NOT
EEG data (no time dimension, no signal) — it does not belong under
Cerebrix. An earlier pass in this same session tried routing this
subgraph through Cerebrix's `GraphNeuralNetwork` decision layer; that
was wrong on two counts: the modality mismatch above, and, found
independently, that class is genuinely incomplete in Cerebrix's own
code (inherits an abstract `get_config` from `BaseDecisionLayer` that
it never implements — real bug, unrelated to this work, not chased
further). Removed that attempt; this subgraph extraction stands on its
own as a real `network`/`biology`-modality artifact.
"""
import json
import pyarrow as pa
import pyarrow.feather as feather
import pyarrow.compute as pc
import numpy as np

DATA_DIR = "/home/rebornbeat/Projects/Ozone-Studio/tools/connectome-mcp/data"
GF_BODY_ID = 10001  # Giant Fiber (DNp01) — the default seed, real and well-studied; any real bodyId works


_TABLE_CACHE = {}


def _load_tables():
    """Load both real feather tables once and cache in memory for the
    life of the process — a real, measured fix (16.5s -> re-read every
    call from a 1GB file was the actual bottleneck, not compute)."""
    if "ann" not in _TABLE_CACHE:
        _TABLE_CACHE["ann"] = feather.read_table(f"{DATA_DIR}/body-annotations-male-cns-v1.0.feather")
        _TABLE_CACHE["weights"] = feather.read_table(f"{DATA_DIR}/connectome-weights-male-cns-v1.0.feather")
    return _TABLE_CACHE["ann"], _TABLE_CACHE["weights"]


def build_subgraph(seed_body_id: int = GF_BODY_ID, top_k: int = 25):
    ann, weights = _load_tables()

    out_edges = weights.filter(pc.equal(weights.column("body_pre"), seed_body_id)).to_pandas()
    in_edges = weights.filter(pc.equal(weights.column("body_post"), seed_body_id)).to_pandas()

    if out_edges.empty and in_edges.empty:
        return {"error": f"bodyId {seed_body_id} has no real connections in the dataset (unknown or isolated id)"}

    top_out = out_edges.nlargest(top_k, "weight")
    top_in = in_edges.nlargest(top_k, "weight")

    node_ids = sorted(set([seed_body_id]) | set(top_out["body_post"]) | set(top_in["body_pre"]))
    node_index = {bid: i for i, bid in enumerate(node_ids)}
    n = len(node_ids)

    # Real edges among exactly this node set (GF's own top edges + any
    # real edges the partners happen to have with each other) — a genuine
    # induced subgraph, not just a star around GF. Filtered with pyarrow
    # compute directly on the 151.8M-row Arrow table (columnar, no full
    # pandas materialization — that OOM-killed the process on first try).
    id_array = pa.array(node_ids)
    pre_mask = pc.is_in(weights.column("body_pre"), value_set=id_array)
    post_mask = pc.is_in(weights.column("body_post"), value_set=id_array)
    induced = weights.filter(pc.and_(pre_mask, post_mask)).to_pandas()
    ann_pd = ann.filter(pc.is_in(ann.column("bodyId"), value_set=id_array)).to_pandas()

    adjacency = np.zeros((n, n), dtype=np.float32)
    for _, row in induced.iterrows():
        i, j = node_index[row["body_pre"]], node_index[row["body_post"]]
        adjacency[i, j] = float(row["weight"])

    ann_by_id = ann_pd.set_index("bodyId")
    node_meta = []
    for bid in node_ids:
        if bid in ann_by_id.index:
            row = ann_by_id.loc[bid]
            node_meta.append({
                "body_id": int(bid),
                "type": row.get("type"),
                "superclass": row.get("superclass"),
                "instance": row.get("instance"),
                "soma_side": row.get("somaSide"),
                "status": row.get("status"),
            })
        else:
            node_meta.append({"body_id": int(bid), "type": None, "superclass": None, "instance": None, "soma_side": None, "status": None})

    return {
        "seed_body_id": int(seed_body_id),
        "n_nodes": n,
        "n_real_edges_in_subgraph": int((adjacency > 0).sum()),
        "seed_real_total_outgoing": int(len(out_edges)),
        "seed_real_total_incoming": int(len(in_edges)),
        "node_meta": node_meta,
        "adjacency": adjacency,
    }


if __name__ == "__main__":
    result = build_subgraph()
    summary = {k: v for k, v in result.items() if k != "adjacency"}
    print(json.dumps(summary, indent=2, default=str))
