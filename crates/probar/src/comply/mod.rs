//! Compliance Checking Module
//!
//! Per `PROBAR-SPEC-WASM-001`, this module provides compliance checking
//! for WASM threading and state synchronization patterns.
//!
//! ## Usage
//!
//! ```bash
//! probar comply --wasm-threading ./src
//! ```

pub mod tarantula;
pub mod wasm_threading;

pub use tarantula::{TarantulaEngine, TarantulaReport};
pub use wasm_threading::{ComplianceResult, ComplianceStatus, WasmThreadingCompliance};
