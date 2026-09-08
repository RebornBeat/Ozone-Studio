//! K-ALGORITHM contracts — shared between the host crate and pipeline crates
//! (pipelines include individual files via `#[path]`; the host includes this
//! aggregate module). See docs/CONTRACTS.md.

pub mod k_loops;
pub mod k_registry;
pub mod k_validation;
