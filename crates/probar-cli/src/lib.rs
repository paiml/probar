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
mod error;
mod output;
mod runner;

pub use commands::{
    Cli, Commands, ConfigArgs, InitArgs, RecordArgs, RecordFormat, ReportArgs, ReportFormat,
    TestArgs,
};
pub use config::{CliConfig, ColorChoice, Verbosity};
pub use error::{CliError, CliResult};
pub use output::{OutputFormat, ProgressReporter};
pub use runner::TestRunner;
