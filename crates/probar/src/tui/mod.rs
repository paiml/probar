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

mod assertions;
mod backend;
mod snapshot;

pub use assertions::{expect_frame, FrameAssertion, MultiValueTracker, ValueTracker};
pub use backend::{TuiFrame, TuiTestBackend};
pub use snapshot::{FrameSequence, SnapshotManager, TuiSnapshot};
