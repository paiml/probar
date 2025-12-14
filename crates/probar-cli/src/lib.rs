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
pub mod debug;
pub mod dev_server;
mod error;
pub mod lint;
pub mod load_testing;
mod output;
mod runner;
pub mod score;
pub mod simulation;
pub mod statistics;
pub mod tracing;
pub mod tree;
pub mod visualization;
pub mod wasm_testing;

pub use commands::{
    BuildArgs, Cli, Commands, ConfigArgs, CoverageArgs, DiagramFormat, InitArgs, PaletteArg,
    PlaybookArgs, PlaybookOutputFormat, RecordArgs, RecordFormat, ReportArgs, ReportFormat,
    ScoreArgs, ScoreOutputFormat, ServeArgs, ServeSubcommand, TestArgs, TreeArgs, VizArgs,
    WasmTarget, WatchArgs,
};
pub use config::{CliConfig, ColorChoice, Verbosity};
pub use debug::{create_tracer, DebugCategory, DebugTracer, DebugVerbosity, ResolutionRule};
pub use dev_server::{
    get_mime_type, DevServer, DevServerConfig, DevServerConfigBuilder, FileChangeEvent,
    FileWatcher, FileWatcherBuilder, HotReloadMessage,
};
pub use error::{CliError, CliResult};
pub use output::{OutputFormat, ProgressReporter};
pub use runner::TestRunner;
pub use score::{
    CategoryScore, CategoryStatus, CriterionResult, Effort, Grade, ProjectScore, Recommendation,
    ScoreCalculator,
};
pub use tree::{build_tree, display_tree, render_tree, FileNode, TreeConfig};
pub use lint::{ContentLinter, LintReport, LintResult, LintSeverity, render_lint_report, render_lint_json};
pub use wasm_testing::{
    Browser, BrowserMatrix, BrowserTestResult, ComparisonStatus, KeyModifiers, MemoryGrowthEvent,
    MemoryProfile, MemorySnapshot, PerformanceBaseline, PerformanceComparison, PerformanceMetric,
    RecordedEvent, Recording, RecordingMetadata, Viewport, compare_performance,
    render_performance_report,
};
pub use load_testing::{
    AssertionResult as LoadAssertionResult, EndpointStats, HttpMethod, LatencyHistogram,
    LoadTestAssertion, LoadTestConfig, LoadTestError, LoadTestErrorKind, LoadTestOutputFormat,
    LoadTestRequest, LoadTestResult, LoadTestScenario, LoadTestStage, ResourceUsage, UserConfig,
    render_load_test_json, render_load_test_report,
};
// PROBAR-SPEC-006 Section H: Enhanced Visualization
pub use visualization::{
    ComparisonVerdict, DashboardState, DataPoint, EndpointMetrics, ExportFormat, MetricsStream,
    ReportComparison, ReportViewerConfig, StageInfo, StreamingHistogram, TimeSeries,
    render_comparison, render_dashboard,
};
// PROBAR-SPEC-006 Section I: Statistical Analysis
pub use statistics::{
    ApdexCalculator, ApdexRating, KneeDetector, LatencySample, QuantileRegression,
    StatisticalAnalysis, TailAttribution, VarianceComponent, VarianceTree,
    render_statistical_json, render_statistical_report,
};
// PROBAR-SPEC-006 Section J: Deep Tracing
pub use tracing::{
    Flamegraph, FlamegraphNode, OptimizationSuggestion, SourceHotspot, SourceLocation,
    SyscallStats, TraceAnalysis, TraceCategory, TraceConfig, TraceSpan, WasmEvent, WasmEventType,
    render_trace_json, render_trace_report,
};
// PROBAR-SPEC-006 Section K: Simulation Playback
pub use simulation::{
    ChaosObservation, ChaosResult, Distribution, FailureInjection, ImpactLevel, InjectionType,
    LatencyDistribution, MonteCarloResult, ObservationSeverity, ParameterVariation, RiskLevel,
    SensitivityFactor, SimulationConfig, SimulationMode, SimulationOutput, SlaProbability,
    render_chaos_report, render_monte_carlo_json, render_monte_carlo_report,
};
