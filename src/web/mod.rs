//! Web launch architecture surface.
//!
//! This module intentionally starts with framework-agnostic contracts and data
//! models. Backend lanes can layer Axum routes/WebSockets on top without
//! leaking provider credentials or duplicating TUI state semantics.

pub mod contracts;
pub mod data;

pub use contracts::*;
pub use data::*;
