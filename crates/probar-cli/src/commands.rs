//! CLI command definitions using clap

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Probar: Rust-native testing framework for WASM games
#[derive(Parser, Debug)]
#[command(name = "probar")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Color output (auto, always, never)
    #[arg(long, default_value = "auto", global = true)]
    pub color: ColorArg,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run tests
    Test(TestArgs),

    /// Record test execution
    Record(RecordArgs),

    /// Generate reports
    Report(ReportArgs),

    /// Initialize a new Probar project
    Init(InitArgs),

    /// Show configuration
    Config(ConfigArgs),
}

/// Arguments for the test command
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct TestArgs {
    /// Filter tests by pattern
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Number of parallel test jobs
    #[arg(short = 'j', long, default_value = "0")]
    pub parallel: usize,

    /// Enable coverage collection
    #[arg(long)]
    pub coverage: bool,

    /// Enable mutation testing
    #[arg(long)]
    pub mutants: bool,

    /// Fail fast on first error
    #[arg(long)]
    pub fail_fast: bool,

    /// Watch mode - rerun on changes
    #[arg(short, long)]
    pub watch: bool,

    /// Test timeout in milliseconds
    #[arg(long, default_value = "30000")]
    pub timeout: u64,

    /// Output directory for results
    #[arg(short, long, default_value = "target/probar")]
    pub output: PathBuf,
}

/// Arguments for the record command
#[derive(Parser, Debug)]
pub struct RecordArgs {
    /// Test to record
    pub test: String,

    /// Output format
    #[arg(short, long, default_value = "gif")]
    pub format: RecordFormat,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Frame rate for recording (for GIF/MP4)
    #[arg(long, default_value = "10")]
    pub fps: u8,

    /// Recording quality (1-100)
    #[arg(long, default_value = "80")]
    pub quality: u8,
}

/// Recording output format
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum RecordFormat {
    /// Animated GIF
    #[default]
    Gif,
    /// PNG screenshots
    Png,
    /// SVG vector graphics
    Svg,
    /// MP4 video
    Mp4,
}

/// Arguments for the report command
#[derive(Parser, Debug)]
pub struct ReportArgs {
    /// Report format
    #[arg(short, long, default_value = "html")]
    pub format: ReportFormat,

    /// Output directory
    #[arg(short, long, default_value = "target/probar/reports")]
    pub output: PathBuf,

    /// Open report in browser after generation
    #[arg(long)]
    pub open: bool,
}

/// Report output format
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ReportFormat {
    /// HTML report
    #[default]
    Html,
    /// `JUnit` XML
    Junit,
    /// LCOV coverage
    Lcov,
    /// Cobertura XML coverage
    Cobertura,
    /// JSON
    Json,
}

/// Arguments for the init command
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Project directory (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Force initialization even if files exist
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the config command
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Show current configuration
    #[arg(long)]
    pub show: bool,

    /// Set a configuration value (key=value)
    #[arg(long)]
    pub set: Option<String>,

    /// Reset to default configuration
    #[arg(long)]
    pub reset: bool,
}

/// Color argument for CLI
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ColorArg {
    /// Automatic color detection
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

impl From<ColorArg> for crate::config::ColorChoice {
    fn from(arg: ColorArg) -> Self {
        match arg {
            ColorArg::Auto => Self::Auto,
            ColorArg::Always => Self::Always,
            ColorArg::Never => Self::Never,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    mod cli_tests {
        use super::*;

        #[test]
        fn test_parse_test_command() {
            let cli = Cli::parse_from(["probar", "test"]);
            assert!(matches!(cli.command, Commands::Test(_)));
        }

        #[test]
        fn test_parse_test_with_filter() {
            let cli = Cli::parse_from(["probar", "test", "--filter", "game::*"]);
            if let Commands::Test(args) = cli.command {
                assert_eq!(args.filter, Some("game::*".to_string()));
            } else {
                panic!("expected Test command");
            }
        }

        #[test]
        fn test_parse_test_with_parallel() {
            let cli = Cli::parse_from(["probar", "test", "-j", "4"]);
            if let Commands::Test(args) = cli.command {
                assert_eq!(args.parallel, 4);
            } else {
                panic!("expected Test command");
            }
        }

        #[test]
        fn test_parse_test_with_coverage() {
            let cli = Cli::parse_from(["probar", "test", "--coverage"]);
            if let Commands::Test(args) = cli.command {
                assert!(args.coverage);
            } else {
                panic!("expected Test command");
            }
        }

        #[test]
        fn test_parse_test_with_fail_fast() {
            let cli = Cli::parse_from(["probar", "test", "--fail-fast"]);
            if let Commands::Test(args) = cli.command {
                assert!(args.fail_fast);
            } else {
                panic!("expected Test command");
            }
        }

        #[test]
        fn test_parse_record_command() {
            let cli = Cli::parse_from(["probar", "record", "test_login"]);
            if let Commands::Record(args) = cli.command {
                assert_eq!(args.test, "test_login");
            } else {
                panic!("expected Record command");
            }
        }

        #[test]
        fn test_parse_record_with_format() {
            let cli = Cli::parse_from(["probar", "record", "test_login", "--format", "png"]);
            if let Commands::Record(args) = cli.command {
                assert!(matches!(args.format, RecordFormat::Png));
            } else {
                panic!("expected Record command");
            }
        }

        #[test]
        fn test_parse_report_command() {
            let cli = Cli::parse_from(["probar", "report"]);
            assert!(matches!(cli.command, Commands::Report(_)));
        }

        #[test]
        fn test_parse_report_with_format() {
            let cli = Cli::parse_from(["probar", "report", "--format", "lcov"]);
            if let Commands::Report(args) = cli.command {
                assert!(matches!(args.format, ReportFormat::Lcov));
            } else {
                panic!("expected Report command");
            }
        }

        #[test]
        fn test_parse_init_command() {
            let cli = Cli::parse_from(["probar", "init"]);
            assert!(matches!(cli.command, Commands::Init(_)));
        }

        #[test]
        fn test_parse_config_command() {
            let cli = Cli::parse_from(["probar", "config", "--show"]);
            if let Commands::Config(args) = cli.command {
                assert!(args.show);
            } else {
                panic!("expected Config command");
            }
        }

        #[test]
        fn test_global_verbose_flag() {
            let cli = Cli::parse_from(["probar", "-vvv", "test"]);
            assert_eq!(cli.verbose, 3);
        }

        #[test]
        fn test_global_quiet_flag() {
            let cli = Cli::parse_from(["probar", "-q", "test"]);
            assert!(cli.quiet);
        }

        #[test]
        fn test_global_color_flag() {
            let cli = Cli::parse_from(["probar", "--color", "never", "test"]);
            assert!(matches!(cli.color, ColorArg::Never));
        }
    }

    mod format_tests {
        use super::*;

        #[test]
        fn test_record_format_default() {
            let format = RecordFormat::default();
            assert!(matches!(format, RecordFormat::Gif));
        }

        #[test]
        fn test_report_format_default() {
            let format = ReportFormat::default();
            assert!(matches!(format, ReportFormat::Html));
        }

        #[test]
        fn test_color_arg_conversion() {
            use crate::config::ColorChoice;

            let auto: ColorChoice = ColorArg::Auto.into();
            assert!(matches!(auto, ColorChoice::Auto));

            let always: ColorChoice = ColorArg::Always.into();
            assert!(matches!(always, ColorChoice::Always));

            let never: ColorChoice = ColorArg::Never.into();
            assert!(matches!(never, ColorChoice::Never));
        }
    }
}
