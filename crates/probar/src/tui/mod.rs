//! TUI Testing Support (Feature 21 - EDD Compliance)
//!
//! Test terminal user interfaces with zero external dependencies.
//! Uses custom TextGrid and MockTty for 100% provable UX.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Poka-Yoke**: Type-safe frame capture with TextGrid
//! - **Muda**: Direct buffer comparison without rendering overhead
//! - **Genchi Genbutsu**: MockTty reflects actual terminal behavior
//! - **Jidoka**: Fail-fast on frame mismatch

mod assertions;
mod backend;
mod buffer;
mod snapshot;
mod tty;

pub use assertions::{expect_frame, FrameAssertion, MultiValueTracker, ValueTracker};
pub use backend::{FrameDiff, LineDiff, TuiFrame, TuiTestBackend};
pub use buffer::TextGrid;
pub use snapshot::{FrameSequence, SnapshotManager, TuiSnapshot};
pub use tty::{AnsiCommand, ClearMode, MockTty};
