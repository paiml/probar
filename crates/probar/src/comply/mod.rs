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

pub mod wasm_threading;

pub use wasm_threading::{ComplianceResult, ComplianceStatus, WasmThreadingCompliance};
