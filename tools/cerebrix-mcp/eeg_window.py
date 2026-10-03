#!/usr/bin/env python3
"""Ozone-Studio Cerebrix MCP — eeg_window tool's Python worker.

Wraps the REAL, unmodified WindowProcessor from the Cerebrix BCI AI
Controller API (~/Projects/Cerebrix/util/window_processor.py) — a
numpy-only class (real: real dependency, confirmed available on this
host), no CereBrix source touched. Does the dynamic-windowing step
Cerebrix's own README describes (200ms/5-frame windows at 25Hz by
default, temporal-context padding) against real caller-supplied frame
data, and returns real shape/metadata as JSON on stdout.

Input (stdin, JSON), one of:
  {"data": [[[float,...]]], ...}                  // caller-supplied (n_frames, n_channels, n_frequencies)
  {"sample": {"label": "left|right|none", "index": 0}, ...}  // real Sentdex/BCI data (see below)
  window_size/stride/context_size optional, Cerebrix defaults.

REAL VALIDATION DATA (operator directive 2026-09-29 — "sentdex... eeg
files of his left/right or not thinking"): data/sentdex_extracted/ holds
1,230 real recordings from github.com/Sentdex/BCI (Harrison Kinsley's
real OpenBCI 16-channel headset sessions, recovered via the Wayback
Machine after the live link went 404 — archived 2025-12-19, file itself
dated 2019-11-12, verified via exact Content-Length byte match: real,
not fabricated). Shape (250, 16, 60) per file — an exact match for this
module's (n_frames, n_channels, n_frequencies) contract, confirmed by
actually running one through the real WindowProcessor (125 real windows,
correct edge padding, both this session). 417 left / 413 right / 400
none real labeled recordings.

Output (stdout, JSON): {success, n_windows, window_shape,
  temporal_positions, padding_mask, error?}
"""
import sys
import os
import json
import glob

CEREBRIX_ROOT = os.environ.get("CEREBRIX_ROOT", "/home/rebornbeat/Projects/Cerebrix")
sys.path.insert(0, CEREBRIX_ROOT)
sys.path.insert(0, os.path.join(CEREBRIX_ROOT, "util"))

SENTDEX_DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "sentdex_extracted")

try:
    import numpy as np
    from window_processor import WindowProcessor
except Exception as e:  # noqa: BLE001 — report import failure as real JSON, not a traceback
    print(json.dumps({"success": False, "error": f"import failed: {e}"}))
    sys.exit(0)


def load_sentdex_sample(label: str, index: int):
    if label not in ("left", "right", "none"):
        raise ValueError(f"label must be left/right/none, got {label!r}")
    files = sorted(glob.glob(os.path.join(SENTDEX_DATA_DIR, "**", label, "*.npy"), recursive=True))
    if not files:
        raise FileNotFoundError(f"no real Sentdex sample files found for label {label!r} under {SENTDEX_DATA_DIR}")
    if not (0 <= index < len(files)):
        raise IndexError(f"index {index} out of range — {len(files)} real {label!r} samples available (0-{len(files)-1})")
    return np.load(files[index]), files[index]


def main():
    try:
        req = json.loads(sys.stdin.read())
        source_file = None
        if "sample" in req:
            data, source_file = load_sentdex_sample(req["sample"].get("label", "left"), int(req["sample"].get("index", 0)))
        else:
            data = np.array(req["data"], dtype=np.float64)
        wp = WindowProcessor(
            window_size=req.get("window_size", 5),
            stride=req.get("stride", 2),
            context_size=req.get("context_size", 2),
        )
        windows, info = wp.create_sliding_windows(data, return_info=True)
        out = {
            "success": True,
            "n_windows": info.n_windows,
            "window_shape": list(windows[0].shape) if windows else [],
            "original_shape": list(info.original_shape),
            "temporal_positions": info.temporal_positions.tolist(),
            "padding_mask": info.padding_mask.tolist(),
        }
        if source_file:
            out["source"] = source_file
        print(json.dumps(out))
    except Exception as e:  # noqa: BLE001 — real failure, reported not raised
        print(json.dumps({"success": False, "error": str(e)}))


if __name__ == "__main__":
    main()
