//! Probar CLI Library (Feature 5)
//!
//! Command-line interface for the Probar testing framework.
//!
//! ## EXTREME TDD: Tests written FIRST per spec

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

mod commands;
mod config;
pub mod dev_server;
mod error;
mod output;
mod runner;

pub use commands::{
    BuildArgs, Cli, Commands, ConfigArgs, CoverageArgs, DiagramFormat, InitArgs, PaletteArg,
    PlaybookArgs, PlaybookOutputFormat, RecordArgs, RecordFormat, ReportArgs, ReportFormat,
    ServeArgs, TestArgs, WasmTarget, WatchArgs,
};
pub use config::{CliConfig, ColorChoice, Verbosity};
pub use dev_server::{
    get_mime_type, DevServer, DevServerConfig, DevServerConfigBuilder, FileWatcher,
    FileWatcherBuilder, HotReloadMessage,
};
pub use error::{CliError, CliResult};
pub use output::{OutputFormat, ProgressReporter};
pub use runner::TestRunner;
