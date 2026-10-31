# uidocument (Python bindings)

Status: WRITTEN, UNBUILT, UNTESTED (edits-only sweep). Thin PyO3 bindings over
the UiDocument modality engine (pipeline 127, `assets/pipelines/modalities/uidocument`).
No logic here: `analyze` calls `uidocument::analyze`, the same function the
pipeline binary calls, so the farm and Ozone cannot disagree (UD10).

API: `analyze(json_str) -> str`, `analyze_dict(dict) -> dict`, `version() -> str`.
No network, no model, no persistence, no browser.

## Install (into the Browser-Farm virtualenv, build-time only)
    pip install maturin
    cd assets/pipelines/modalities/uidocument/py
    maturin develop --release        # or: maturin build --release && pip install target/wheels/*.whl
Wheel is `abi3-py39`, so one build serves Python >= 3.9.

## Dependency direction (one way, optional)
The Browser-Farm repo is NOT edited and does not depend on this package.
Only the Command Center bridge script (which lives outside this repo) does:

    try:
        import uidocument
    except ImportError:
        uidocument = None   # farm keeps working; Ozone's ingest route runs the engine instead

Typical use there: take the batches from `observer/uidocument-observer.js`,
and either forward them to Ozone's ingest route or run `analyze_dict` locally.

## Tests
`tests/test_smoke.py` (needs the wheel built). Not run in this sweep.
