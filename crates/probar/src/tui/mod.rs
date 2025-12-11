//! TUI Testing Support (Feature 21 - EDD Compliance)
//!
//! Test terminal user interfaces built with ratatui/crossterm.
//! Enables 100% provable UX for terminal simulations.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Poka-Yoke**: Type-safe frame capture with Buffer type
//! - **Muda**: Direct buffer comparison without rendering overhead
//! - **Genchi Genbutsu**: TestBackend reflects actual terminal behavior
//! - **Jidoka**: Fail-fast on frame mismatch

mod backend;
mod assertions;
mod snapshot;

pub use backend::{TuiTestBackend, TuiFrame};
pub use assertions::{FrameAssertion, ValueTracker};
pub use snapshot::{TuiSnapshot, FrameSequence};
