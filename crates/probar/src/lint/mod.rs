//! Linting Module for WASM State Synchronization
//!
//! Implements static analysis for detecting state sync anti-patterns per
//! `PROBAR-SPEC-WASM-001`.
//!
//! ## Detection Rules
//!
//! | Rule ID | Description |
//! |---------|-------------|
//! | WASM-SS-001 | Local `Rc::new()` in method with closure |
//! | WASM-SS-002 | Potential state desync (self.field and local_clone both exist) |
//! | WASM-SS-003 | Closure captures local instead of self field |
//! | WASM-SS-004 | Duplicate state fields (RefCell + non-RefCell) |
//! | WASM-SS-005 | Missing `self.*.clone()` before closure |

pub mod state_sync;

pub use state_sync::{LintError, LintResult, LintSeverity, StateSyncLinter, StateSyncReport};
