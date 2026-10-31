//! Python bindings over `uidocument::analyze` (UiDocument, pipeline 127).
//!
//! UNBUILT: written against B1's documented API
//! `uidocument::analyze(&serde_json::Value) -> serde_json::Value`.
//! The bindings add no logic: JSON in, JSON out, so Python and the pipeline
//! binary give byte-identical graphs for the same input (test UD10).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

/// analyze(json_str: str) -> str
/// Parse `json_str` as the engine input, run the engine, return the graph JSON.
#[pyfunction]
fn analyze(json_str: &str) -> PyResult<String> {
    let input: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("input is not valid JSON: {e}")))?;
    let out = uidocument::analyze(&input);
    serde_json::to_string(&out)
        .map_err(|e| PyValueError::new_err(format!("engine output not serializable: {e}")))
}

/// analyze_dict(obj: dict) -> dict
/// Same as `analyze`, through Python's own json module (no extra crate):
/// keeps the dependency set to pyo3 + serde_json.
#[pyfunction]
fn analyze_dict(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    let json = PyModule::import_bound(py, "json")?;
    let text: String = json.call_method1("dumps", (obj,))?.extract()?;
    let out = analyze(&text)?;
    Ok(json.call_method1("loads", (out,))?.unbind())
}

/// version() -> str  (the binding crate version)
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _uidocument(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(analyze, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_dict, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
