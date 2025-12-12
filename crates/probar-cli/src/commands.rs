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
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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

    mod record_format_tests {
        use super::*;

        #[test]
        fn test_default() {
            let format = RecordFormat::default();
            assert!(matches!(format, RecordFormat::Gif));
        }

        #[test]
        fn test_all_variants() {
            let _ = RecordFormat::Gif;
            let _ = RecordFormat::Png;
            let _ = RecordFormat::Svg;
            let _ = RecordFormat::Mp4;
        }

        #[test]
        fn test_debug() {
            let debug = format!("{:?}", RecordFormat::Gif);
            assert!(debug.contains("Gif"));
        }

        #[test]
        fn test_clone() {
            let format = RecordFormat::Mp4;
            let cloned = format;
            assert!(matches!(cloned, RecordFormat::Mp4));
        }
    }

    mod report_format_tests {
        use super::*;

        #[test]
        fn test_default() {
            let format = ReportFormat::default();
            assert!(matches!(format, ReportFormat::Html));
        }

        #[test]
        fn test_all_variants() {
            let _ = ReportFormat::Html;
            let _ = ReportFormat::Junit;
            let _ = ReportFormat::Lcov;
            let _ = ReportFormat::Cobertura;
            let _ = ReportFormat::Json;
        }

        #[test]
        fn test_debug() {
            let debug = format!("{:?}", ReportFormat::Junit);
            assert!(debug.contains("Junit"));
        }
    }

    mod test_args_tests {
        use super::*;

        #[test]
        fn test_defaults() {
            // Verify TestArgs can be created with defaults via clap
            let args = TestArgs {
                filter: None,
                parallel: 0,
                coverage: false,
                mutants: false,
                fail_fast: false,
                watch: false,
                timeout: 30000,
                output: PathBuf::from("target/probar"),
            };
            assert!(!args.coverage);
            assert_eq!(args.timeout, 30000);
        }

        #[test]
        fn test_debug() {
            let args = TestArgs {
                filter: Some("test_*".to_string()),
                parallel: 4,
                coverage: true,
                mutants: false,
                fail_fast: true,
                watch: false,
                timeout: 5000,
                output: PathBuf::from("target"),
            };
            let debug = format!("{args:?}");
            assert!(debug.contains("TestArgs"));
        }
    }

    mod record_args_tests {
        use super::*;

        #[test]
        fn test_creation() {
            let args = RecordArgs {
                test: "my_test".to_string(),
                format: RecordFormat::Gif,
                output: None,
                fps: 10,
                quality: 80,
            };
            assert_eq!(args.test, "my_test");
            assert_eq!(args.fps, 10);
        }

        #[test]
        fn test_debug() {
            let args = RecordArgs {
                test: "test".to_string(),
                format: RecordFormat::Png,
                output: Some(PathBuf::from("out.png")),
                fps: 30,
                quality: 100,
            };
            let debug = format!("{args:?}");
            assert!(debug.contains("RecordArgs"));
        }
    }

    mod report_args_tests {
        use super::*;

        #[test]
        fn test_creation() {
            let args = ReportArgs {
                format: ReportFormat::Lcov,
                output: PathBuf::from("coverage"),
                open: true,
            };
            assert!(args.open);
        }

        #[test]
        fn test_debug() {
            let args = ReportArgs {
                format: ReportFormat::Html,
                output: PathBuf::from("reports"),
                open: false,
            };
            let debug = format!("{args:?}");
            assert!(debug.contains("ReportArgs"));
        }
    }

    mod init_args_tests {
        use super::*;

        #[test]
        fn test_creation() {
            let args = InitArgs {
                path: PathBuf::from("."),
                force: false,
            };
            assert!(!args.force);
        }
    }

    mod config_args_tests {
        use super::*;

        #[test]
        fn test_creation() {
            let args = ConfigArgs {
                show: false,
                set: None,
                reset: false,
            };
            assert!(!args.show);
        }
    }

    mod cli_additional_tests {
        use super::*;

        #[test]
        fn test_cli_debug() {
            let cli = Cli {
                verbose: 0,
                quiet: false,
                color: ColorArg::Auto,
                command: Commands::Config(ConfigArgs {
                    show: true,
                    set: None,
                    reset: false,
                }),
            };
            let debug = format!("{cli:?}");
            assert!(debug.contains("Cli"));
        }
    }
}
