#!/usr/bin/env python3
"""Ozone-Studio Cerebrix MCP — karaone_trial tool's Python worker.

Same EEG MCP as eeg_window/Sentdex (operator, 2026-09-29: "why two
different MCPs when they are both on EEG data" — correct; this stays in
cerebrix-mcp, not a separate server), a second real data source rather
than a second server: the KaraOne imagined-speech dataset
(cs.toronto.edu/~complingweb/data/karaOne), real single-word/phoneme
"thought" vocabulary per subject — exactly the "1 word command
vocabulary... per person" the operator asked for, distinct from
Sentdex's 3-class left/right/none.

Subject MM05 downloaded and extracted in full (2.03GB archive, byte-
verified). Real trials: 165 = 11 prompts x 15 repetitions — 4 real WORDS
(pat, pot, knew, gnaw) + 7 real phonemes (/uw/ /tiy/ /iy/ /m/ /n/ /piy/
/diy/), confirmed by loading all_features_simple.mat's real `prompts`
field and counting labels directly (15 of each, exact).

HONEST CAVEAT (operator flagged this directly, not glossed over): trial
N's real EEG feature vector is assumed to correspond to `prompts[N]` —
same positional index. This is a STRONGER assumption than "two separate
files happen to have equal length": `eeg_features.thinking_feats` and
`prompts` are sibling fields of the SAME per-trial MATLAB struct
(`all_features`, shape (1,165)), so whatever code produced this file
almost certainly wrote both in one lockstep pass — but this has still
NOT been independently cross-checked against a third source (e.g.
epoch_inds.mat's `thinking_inds` timing against a raw .cnt timestamp
trace). Operator's own call: use as-is; a model that fails to learn
anything from mislabeled data is itself a real signal something's
wrong, so this is a self-correcting risk, not a blocking one.

Tools:
  karaone_trial {index?, prompt?, subject?}
    → one real trial's prompt label + real eeg_features array shape/
      summary (not the full raw feature tensor over the wire — large).
  karaone_stats {subject?}
    → real per-subject label counts, computed live from the real file.
"""
import sys
import os
import json

DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "karaone")
DEFAULT_SUBJECT = "MM05"

try:
    import scipy.io as sio
    import numpy as np
except Exception as e:  # noqa: BLE001
    print(json.dumps({"success": False, "error": f"import failed: {e}"}))
    sys.exit(0)


def _features_path(subject: str) -> str:
    # Real extracted path shape for MM05, confirmed live 2026-09-29 —
    # the original researcher's own directory layout, kept as-is rather
    # than restructured, so paths are traceable back to the real archive.
    return os.path.join(
        DATA_DIR, f"{subject}_extracted", "p", "spoclab", "users", "szhao",
        "EEG", "data", subject, "all_features_simple.mat",
    )


_CACHE = {}


def _load(subject: str):
    if subject not in _CACHE:
        path = _features_path(subject)
        if not os.path.isfile(path):
            raise FileNotFoundError(f"no extracted KaraOne data for subject {subject!r} at {path}")
        mat = sio.loadmat(path)
        af = mat["all_features"][0, 0]
        prompts = [str(p[0]) if hasattr(p, "__len__") and len(p) else str(p) for p in af["prompts"].flatten()]
        _CACHE[subject] = {"af": af, "prompts": prompts}
    return _CACHE[subject]


def cmd_stats(req):
    subject = req.get("subject", DEFAULT_SUBJECT)
    d = _load(subject)
    from collections import Counter
    counts = Counter(d["prompts"])
    return {
        "success": True,
        "subject": subject,
        "n_trials": len(d["prompts"]),
        "label_counts": dict(counts),
        "words": [p for p in counts if not p.startswith("/")],
        "phonemes": [p for p in counts if p.startswith("/")],
        "source": "KaraOne (cs.toronto.edu/~complingweb/data/karaOne), Zhao & Rudzicz 2015",
        "alignment_caveat": "prompts[i] assumed to correspond to eeg_features[i] by shared array position — not independently cross-verified against a third source; see this file's header",
    }


def cmd_trial(req):
    subject = req.get("subject", DEFAULT_SUBJECT)
    d = _load(subject)
    prompts = d["prompts"]

    if "prompt" in req:
        idx = next((i for i, p in enumerate(prompts) if p == req["prompt"]), None)
        if idx is None:
            return {"success": False, "error": f"no trial with prompt {req['prompt']!r}; real labels present: {sorted(set(prompts))}"}
    else:
        idx = int(req.get("index", 0))
        if not (0 <= idx < len(prompts)):
            return {"success": False, "error": f"index {idx} out of range (0-{len(prompts)-1})"}

    thinking_feats = d["af"]["eeg_features"][0, 0]["thinking_feats"]  # object array, shape (1, n_trials)
    eeg = thinking_feats[0, idx]  # real (62 channels, 45 features) matrix for this one trial
    return {
        "success": True,
        "subject": subject,
        "trial_index": idx,
        "prompt": prompts[idx],
        "eeg_features_shape": list(eeg.shape),
        "eeg_features_summary": {
            "mean": float(np.mean(eeg)),
            "std": float(np.std(eeg)),
            "min": float(np.min(eeg)),
            "max": float(np.max(eeg)),
        },
        "alignment_caveat": "positional assumption, not independently verified — see this file's header",
    }


def main():
    try:
        req = json.loads(sys.stdin.read() or "{}")
        action = req.get("action", "trial")
        result = cmd_stats(req) if action == "stats" else cmd_trial(req)
        print(json.dumps(result, default=str))
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"success": False, "error": str(e)}))


if __name__ == "__main__":
    main()
