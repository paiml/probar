//! CLI command definitions using clap

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Probador: CLI for Probar - Rust-native testing framework for WASM games
#[derive(Parser, Debug)]
#[command(name = "probador")]
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

    /// Generate coverage heatmaps
    Coverage(CoverageArgs),

    /// Initialize a new Probar project
    Init(InitArgs),

    /// Show configuration
    Config(ConfigArgs),

    /// Start WASM development server
    Serve(ServeArgs),

    /// Build WASM package
    Build(BuildArgs),

    /// Watch for changes and rebuild
    Watch(WatchArgs),

    /// Run state machine playbooks
    Playbook(PlaybookArgs),

    /// Run WASM compliance checks (C001-C010)
    ///
    /// Validates WASM application against Probar's compliance checklist:
    /// - C001: Code execution verified (not just mocked HTML)
    /// - C002: Console errors cause test failure
    /// - C003: Custom elements tested
    /// - C004: Both threading/non-threading modes tested
    /// - C005: Low memory scenario tested
    /// - C006: COOP/COEP headers present
    /// - C007: Replay hash matches
    /// - C008: Proper cache handling
    /// - C009: WASM under size limit
    /// - C010: No panic paths in WASM
    Comply(ComplyArgs),
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

/// Arguments for the coverage command
#[derive(Parser, Debug)]
pub struct CoverageArgs {
    /// Output PNG file path
    #[arg(long)]
    pub png: Option<PathBuf>,

    /// Output JSON file path
    #[arg(long)]
    pub json: Option<PathBuf>,

    /// Color palette (viridis, magma, heat)
    #[arg(long, default_value = "viridis")]
    pub palette: PaletteArg,

    /// Include legend in PNG output
    #[arg(long)]
    pub legend: bool,

    /// Highlight coverage gaps in red
    #[arg(long)]
    pub gaps: bool,

    /// Title for the heatmap
    #[arg(long)]
    pub title: Option<String>,

    /// PNG width in pixels
    #[arg(long, default_value = "800")]
    pub width: u32,

    /// PNG height in pixels
    #[arg(long, default_value = "600")]
    pub height: u32,

    /// Coverage data input file (JSON)
    #[arg(short, long)]
    pub input: Option<PathBuf>,
}

/// Color palette argument
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum PaletteArg {
    /// Viridis (colorblind-friendly)
    #[default]
    Viridis,
    /// Magma (dark to bright)
    Magma,
    /// Heat (black-red-yellow-white)
    Heat,
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

/// Arguments for the serve command
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ServeArgs {
    /// Subcommand for serve (tree, viz, score)
    #[command(subcommand)]
    pub subcommand: Option<ServeSubcommand>,

    /// Directory to serve (default: current directory)
    #[arg(short = 'd', long = "dir", default_value = ".")]
    pub directory: PathBuf,

    /// HTTP port to listen on
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// WebSocket port for hot reload
    #[arg(long, default_value = "8081")]
    pub ws_port: u16,

    /// Open browser automatically
    #[arg(long)]
    pub open: bool,

    /// Enable CORS for cross-origin requests
    #[arg(long)]
    pub cors: bool,

    /// Enable Cross-Origin Isolation (COOP/COEP headers)
    ///
    /// Required for `SharedArrayBuffer` and parallel WASM with Web Workers.
    /// Sets Cross-Origin-Opener-Policy: same-origin and
    /// Cross-Origin-Embedder-Policy: require-corp headers.
    #[arg(long)]
    pub cross_origin_isolated: bool,

    /// Enable debug mode with verbose request/response logging
    #[arg(long)]
    pub debug: bool,

    /// Enable content linting (HTML/CSS/JS validation)
    #[arg(long)]
    pub lint: bool,

    /// Enable file watching for hot reload (default: true)
    #[arg(long, default_value = "true")]
    pub watch: bool,

    /// Validate module imports before serving
    ///
    /// Scans HTML files for JS/WASM imports and verifies they resolve
    /// with correct MIME types. Fails if any imports are broken.
    #[arg(long)]
    pub validate: bool,

    /// Monitor requests and warn about issues (404s, MIME mismatches)
    #[arg(long)]
    pub monitor: bool,

    /// Exclude directories from validation (e.g., `node_modules`)
    ///
    /// Can be specified multiple times: --exclude `node_modules` --exclude vendor
    #[arg(long, value_name = "DIR")]
    pub exclude: Vec<String>,
}

/// Serve subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum ServeSubcommand {
    /// Display file tree of served directory
    Tree(TreeArgs),

    /// Interactive TUI visualization of served files
    Viz(VizArgs),

    /// Generate project testing score (0-100)
    Score(ScoreArgs),
}

/// Arguments for the tree subcommand
#[derive(Parser, Debug, Clone)]
pub struct TreeArgs {
    /// Directory to display (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Maximum depth to display (0 = root only)
    #[arg(long)]
    pub depth: Option<usize>,

    /// Filter files by glob pattern (e.g., "*.html")
    #[arg(long)]
    pub filter: Option<String>,

    /// Show file sizes
    #[arg(long, default_value = "true")]
    pub sizes: bool,

    /// Show MIME types
    #[arg(long, default_value = "true")]
    pub mime_types: bool,
}

/// Arguments for the viz subcommand
#[derive(Parser, Debug, Clone)]
pub struct VizArgs {
    /// Directory to visualize (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// HTTP port for TUI server
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
}

/// Arguments for the score subcommand
#[derive(Parser, Debug, Clone)]
pub struct ScoreArgs {
    /// Project directory to score (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Minimum score threshold (exit non-zero if below)
    #[arg(long)]
    pub min: Option<u32>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: ScoreOutputFormat,

    /// Output HTML report to file
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Show detailed breakdown of all criteria
    #[arg(long)]
    pub detailed: bool,

    /// Append score to history file (JSONL)
    #[arg(long)]
    pub history: Option<PathBuf>,

    /// Show score trend over time
    #[arg(long)]
    pub trend: bool,

    /// Run LIVE browser validation (starts server, launches headless browser)
    ///
    /// This actually tests if the app works rather than just checking for files.
    /// Requires Chrome/Chromium installed. Recommended for accurate scoring.
    #[arg(long)]
    pub live: bool,

    /// Port for live validation server (default: random available port)
    #[arg(long, default_value = "0")]
    pub port: u16,
}

/// Output format for score command
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ScoreOutputFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON output for CI integration
    Json,
}

/// Arguments for the build command
#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Package directory (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Build target (web, bundler, nodejs, no-modules)
    #[arg(short, long, default_value = "web")]
    pub target: WasmTarget,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Output directory (default: pkg)
    #[arg(short, long)]
    pub out_dir: Option<PathBuf>,

    /// Enable profiling (adds names section to WASM)
    #[arg(long)]
    pub profiling: bool,
}

/// WASM build target
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum WasmTarget {
    /// ES modules for web browsers
    #[default]
    Web,
    /// `CommonJS` for bundlers like webpack
    Bundler,
    /// `Node.js` modules
    Nodejs,
    /// No ES modules (legacy)
    NoModules,
}

impl WasmTarget {
    /// Get `wasm-pack` target string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Bundler => "bundler",
            Self::Nodejs => "nodejs",
            Self::NoModules => "no-modules",
        }
    }
}

/// Arguments for the watch command
#[derive(Parser, Debug)]
pub struct WatchArgs {
    /// Package directory to watch (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Also start the dev server
    #[arg(long)]
    pub serve: bool,

    /// Server port (when --serve is used)
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// WebSocket port for hot reload
    #[arg(long, default_value = "8081")]
    pub ws_port: u16,

    /// Build target
    #[arg(short, long, default_value = "web")]
    pub target: WasmTarget,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Debounce delay in milliseconds
    #[arg(long, default_value = "500")]
    pub debounce: u64,
}

/// Arguments for the playbook command
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct PlaybookArgs {
    /// Playbook YAML file(s) to run
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Validate playbook without running
    #[arg(long)]
    pub validate: bool,

    /// Export state machine diagram
    #[arg(long, value_enum)]
    pub export: Option<DiagramFormat>,

    /// Output file for diagram export
    #[arg(long)]
    pub export_output: Option<PathBuf>,

    /// Run mutation testing (M1-M5)
    #[arg(long)]
    pub mutate: bool,

    /// Mutation classes to run (e.g., M1,M2)
    #[arg(long, value_delimiter = ',')]
    pub mutation_classes: Option<Vec<String>>,

    /// Fail fast on first error
    #[arg(long)]
    pub fail_fast: bool,

    /// Continue on step failure
    #[arg(long)]
    pub continue_on_error: bool,

    /// Output format for results
    #[arg(short, long, default_value = "text")]
    pub format: PlaybookOutputFormat,

    /// Output directory for results
    #[arg(short, long, default_value = "target/probar/playbooks")]
    pub output: PathBuf,
}

/// Diagram export format
#[derive(ValueEnum, Clone, Debug)]
pub enum DiagramFormat {
    /// DOT format (Graphviz)
    Dot,
    /// SVG format
    Svg,
}

/// Output format for playbook results
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum PlaybookOutputFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON output
    Json,
    /// `JUnit` XML
    Junit,
}

/// Arguments for the comply command
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComplyArgs {
    /// Comply subcommand (check, migrate, diff, enforce, report)
    #[command(subcommand)]
    pub subcommand: Option<ComplySubcommand>,

    /// Directory to check (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Specific checks to run (e.g., C001,C002)
    #[arg(long, value_delimiter = ',')]
    pub checks: Option<Vec<String>>,

    /// Fail on first non-compliance
    #[arg(long)]
    pub fail_fast: bool,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: ComplyOutputFormat,

    /// Maximum WASM binary size in bytes (for C009)
    #[arg(long, default_value = "5242880")]
    pub max_wasm_size: usize,

    /// Enable strict mode (all checks must pass)
    #[arg(long)]
    pub strict: bool,

    /// Generate compliance report file
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Show detailed check results
    #[arg(long)]
    pub detailed: bool,
}

/// Comply subcommands (per PROBAR-SPEC-011 Section 3.1)
#[derive(Subcommand, Debug, Clone)]
pub enum ComplySubcommand {
    /// Check WASM testing compliance
    Check(ComplyCheckArgs),

    /// Migrate to latest probador standards
    Migrate(ComplyMigrateArgs),

    /// Show changelog between versions
    Diff(ComplyDiffArgs),

    /// Install WASM quality hooks
    Enforce(ComplyEnforceArgs),

    /// Generate compliance report
    Report(ComplyReportArgs),
}

/// Arguments for comply check subcommand
#[derive(Parser, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComplyCheckArgs {
    /// Directory to check (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Exit with error if non-compliant
    #[arg(long)]
    pub strict: bool,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: ComplyOutputFormat,

    /// Specific checks to run (e.g., C001,C002)
    #[arg(long, value_delimiter = ',')]
    pub checks: Option<Vec<String>>,

    /// Show detailed check results
    #[arg(long)]
    pub detailed: bool,
}

/// Arguments for comply migrate subcommand
#[derive(Parser, Debug, Clone)]
pub struct ComplyMigrateArgs {
    /// Directory to migrate (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target version to migrate to
    #[arg(long)]
    pub version: Option<String>,

    /// Preview changes without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Force migration even with uncommitted changes
    #[arg(long)]
    pub force: bool,
}

/// Arguments for comply diff subcommand
#[derive(Parser, Debug, Clone)]
pub struct ComplyDiffArgs {
    /// From version
    #[arg(long)]
    pub from: Option<String>,

    /// To version
    #[arg(long)]
    pub to: Option<String>,

    /// Show only breaking changes
    #[arg(long)]
    pub breaking_only: bool,
}

/// Arguments for comply enforce subcommand
#[derive(Parser, Debug, Clone)]
pub struct ComplyEnforceArgs {
    /// Directory to enforce (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Skip confirmation
    #[arg(long)]
    pub yes: bool,

    /// Remove hooks instead of installing
    #[arg(long)]
    pub disable: bool,
}

/// Arguments for comply report subcommand
#[derive(Parser, Debug, Clone)]
pub struct ComplyReportArgs {
    /// Directory to report on (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: ComplyReportFormat,

    /// Output file (default: stdout)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

/// Output format for comply report
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ComplyReportFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON output
    Json,
    /// Markdown
    Markdown,
    /// HTML
    Html,
}

/// Output format for comply command
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ComplyOutputFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON output for CI integration
    Json,
    /// `JUnit` XML for CI systems
    Junit,
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

    mod coverage_tests {
        use super::*;

        #[test]
        fn test_parse_coverage_command() {
            let cli = Cli::parse_from(["probar", "coverage"]);
            assert!(matches!(cli.command, Commands::Coverage(_)));
        }

        #[test]
        fn test_parse_coverage_with_png() {
            let cli = Cli::parse_from(["probar", "coverage", "--png", "output.png"]);
            if let Commands::Coverage(args) = cli.command {
                assert_eq!(args.png, Some(PathBuf::from("output.png")));
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_with_palette() {
            let cli = Cli::parse_from(["probar", "coverage", "--palette", "magma"]);
            if let Commands::Coverage(args) = cli.command {
                assert!(matches!(args.palette, PaletteArg::Magma));
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_with_legend() {
            let cli = Cli::parse_from(["probar", "coverage", "--legend"]);
            if let Commands::Coverage(args) = cli.command {
                assert!(args.legend);
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_with_gaps() {
            let cli = Cli::parse_from(["probar", "coverage", "--gaps"]);
            if let Commands::Coverage(args) = cli.command {
                assert!(args.gaps);
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_with_title() {
            let cli = Cli::parse_from(["probar", "coverage", "--title", "My Coverage"]);
            if let Commands::Coverage(args) = cli.command {
                assert_eq!(args.title, Some("My Coverage".to_string()));
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_with_dimensions() {
            let cli = Cli::parse_from(["probar", "coverage", "--width", "1024", "--height", "768"]);
            if let Commands::Coverage(args) = cli.command {
                assert_eq!(args.width, 1024);
                assert_eq!(args.height, 768);
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_parse_coverage_full_options() {
            let cli = Cli::parse_from([
                "probar",
                "coverage",
                "--png",
                "heatmap.png",
                "--palette",
                "heat",
                "--legend",
                "--gaps",
                "--title",
                "Test Coverage",
                "--width",
                "1920",
                "--height",
                "1080",
            ]);
            if let Commands::Coverage(args) = cli.command {
                assert_eq!(args.png, Some(PathBuf::from("heatmap.png")));
                assert!(matches!(args.palette, PaletteArg::Heat));
                assert!(args.legend);
                assert!(args.gaps);
                assert_eq!(args.title, Some("Test Coverage".to_string()));
                assert_eq!(args.width, 1920);
                assert_eq!(args.height, 1080);
            } else {
                panic!("expected Coverage command");
            }
        }

        #[test]
        fn test_palette_default() {
            let palette = PaletteArg::default();
            assert!(matches!(palette, PaletteArg::Viridis));
        }

        #[test]
        fn test_coverage_args_defaults() {
            let args = CoverageArgs {
                png: None,
                json: None,
                palette: PaletteArg::default(),
                legend: false,
                gaps: false,
                title: None,
                width: 800,
                height: 600,
                input: None,
            };
            assert_eq!(args.width, 800);
            assert_eq!(args.height, 600);
            assert!(matches!(args.palette, PaletteArg::Viridis));
        }

        #[test]
        fn test_coverage_args_debug() {
            let args = CoverageArgs {
                png: Some(PathBuf::from("test.png")),
                json: None,
                palette: PaletteArg::Magma,
                legend: true,
                gaps: true,
                title: Some("Test".to_string()),
                width: 640,
                height: 480,
                input: None,
            };
            let debug = format!("{args:?}");
            assert!(debug.contains("CoverageArgs"));
        }
    }

    mod playbook_tests {
        use super::*;

        #[test]
        fn test_parse_playbook_command() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml"]);
            assert!(matches!(cli.command, Commands::Playbook(_)));
        }

        #[test]
        fn test_parse_playbook_multiple_files() {
            let cli = Cli::parse_from(["probar", "playbook", "a.yaml", "b.yaml", "c.yaml"]);
            if let Commands::Playbook(args) = cli.command {
                assert_eq!(args.files.len(), 3);
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_validate() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--validate"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(args.validate);
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_export_dot() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--export", "dot"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(matches!(args.export, Some(DiagramFormat::Dot)));
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_export_svg() {
            let cli = Cli::parse_from([
                "probar",
                "playbook",
                "test.yaml",
                "--export",
                "svg",
                "--export-output",
                "diagram.svg",
            ]);
            if let Commands::Playbook(args) = cli.command {
                assert!(matches!(args.export, Some(DiagramFormat::Svg)));
                assert_eq!(args.export_output, Some(PathBuf::from("diagram.svg")));
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_mutate() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--mutate"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(args.mutate);
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_mutation_classes() {
            let cli = Cli::parse_from([
                "probar",
                "playbook",
                "test.yaml",
                "--mutate",
                "--mutation-classes",
                "M1,M2,M3",
            ]);
            if let Commands::Playbook(args) = cli.command {
                assert!(args.mutate);
                let classes = args.mutation_classes.expect("mutation classes");
                assert_eq!(classes.len(), 3);
                assert!(classes.contains(&"M1".to_string()));
                assert!(classes.contains(&"M2".to_string()));
                assert!(classes.contains(&"M3".to_string()));
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_fail_fast() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--fail-fast"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(args.fail_fast);
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_continue_on_error() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--continue-on-error"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(args.continue_on_error);
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_format_json() {
            let cli = Cli::parse_from(["probar", "playbook", "test.yaml", "--format", "json"]);
            if let Commands::Playbook(args) = cli.command {
                assert!(matches!(args.format, PlaybookOutputFormat::Json));
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_parse_playbook_output_dir() {
            let cli =
                Cli::parse_from(["probar", "playbook", "test.yaml", "--output", "results/pb"]);
            if let Commands::Playbook(args) = cli.command {
                assert_eq!(args.output, PathBuf::from("results/pb"));
            } else {
                panic!("expected Playbook command");
            }
        }

        #[test]
        fn test_playbook_args_defaults() {
            let args = PlaybookArgs {
                files: vec![PathBuf::from("test.yaml")],
                validate: false,
                export: None,
                export_output: None,
                mutate: false,
                mutation_classes: None,
                fail_fast: false,
                continue_on_error: false,
                format: PlaybookOutputFormat::default(),
                output: PathBuf::from("target/probar/playbooks"),
            };
            assert!(!args.validate);
            assert!(!args.mutate);
            assert!(matches!(args.format, PlaybookOutputFormat::Text));
        }

        #[test]
        fn test_playbook_args_debug() {
            let args = PlaybookArgs {
                files: vec![PathBuf::from("login.yaml")],
                validate: true,
                export: Some(DiagramFormat::Svg),
                export_output: Some(PathBuf::from("out.svg")),
                mutate: true,
                mutation_classes: Some(vec!["M1".to_string(), "M2".to_string()]),
                fail_fast: true,
                continue_on_error: false,
                format: PlaybookOutputFormat::Json,
                output: PathBuf::from("output"),
            };
            let debug = format!("{args:?}");
            assert!(debug.contains("PlaybookArgs"));
        }

        #[test]
        fn test_diagram_format_debug() {
            let dot_debug = format!("{:?}", DiagramFormat::Dot);
            assert!(dot_debug.contains("Dot"));

            let svg_debug = format!("{:?}", DiagramFormat::Svg);
            assert!(svg_debug.contains("Svg"));
        }

        #[test]
        fn test_playbook_output_format_default() {
            let format = PlaybookOutputFormat::default();
            assert!(matches!(format, PlaybookOutputFormat::Text));
        }

        #[test]
        fn test_playbook_output_format_all_variants() {
            let _ = PlaybookOutputFormat::Text;
            let _ = PlaybookOutputFormat::Json;
            let _ = PlaybookOutputFormat::Junit;
        }
    }
}
