//! Probar CLI: Command-line interface for WASM testing
//!
//! ## Usage
//!
//! ```bash
//! probar test                     # Run all tests
//! probar test --filter "game::*"  # Filter tests
//! probar record <test> --gif      # Record as GIF
//! probar report --html            # Generate HTML report
//! ```

use clap::Parser;
use probar_cli::{Cli, CliConfig, CliResult, ColorChoice, Commands, TestRunner, Verbosity};
use std::process::ExitCode;

// Re-export CoverageCell for use in create_sample_coverage_data
use jugar_probar::pixel_coverage::CoverageCell;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> CliResult<()> {
    let cli = Cli::parse();

    // Build configuration from CLI args
    let config = build_config(&cli);

    match cli.command {
        Commands::Test(args) => run_tests(config, &args),
        Commands::Record(args) => {
            run_record(&config, &args);
            Ok(())
        }
        Commands::Report(args) => {
            run_report(&config, &args);
            Ok(())
        }
        Commands::Coverage(args) => run_coverage(&config, &args),
        Commands::Init(args) => {
            run_init(&args);
            Ok(())
        }
        Commands::Config(args) => {
            run_config(&config, &args);
            Ok(())
        }
        Commands::Serve(args) => run_serve(&args),
        Commands::Build(args) => run_build(&args),
        Commands::Watch(args) => run_watch(&args),
        Commands::Playbook(args) => run_playbook(&config, &args),
    }
}

fn build_config(cli: &Cli) -> CliConfig {
    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        match cli.verbose {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    };

    let color: ColorChoice = cli.color.clone().into();

    CliConfig::new().with_verbosity(verbosity).with_color(color)
}

fn run_tests(config: CliConfig, args: &probar_cli::TestArgs) -> CliResult<()> {
    let config = config
        .with_parallel_jobs(args.parallel)
        .with_fail_fast(args.fail_fast)
        .with_coverage(args.coverage)
        .with_watch(args.watch)
        .with_output_dir(args.output.to_string_lossy().to_string());

    let mut runner = TestRunner::new(config);
    let results = runner.run(args.filter.as_deref())?;

    if results.all_passed() {
        Ok(())
    } else {
        Err(probar_cli::CliError::test_execution(format!(
            "{} test(s) failed",
            results.failed()
        )))
    }
}

fn run_record(_config: &CliConfig, args: &probar_cli::RecordArgs) {
    println!("Recording test: {}", args.test);
    println!("Format: {:?}", args.format);
    println!("FPS: {}", args.fps);
    println!("Quality: {}", args.quality);

    // Note: Full recording requires browser feature and running tests
    // This displays configuration; actual recording done via test runner
    println!("Recording configuration ready. Run test with --record flag to capture.");
}

fn run_report(_config: &CliConfig, args: &probar_cli::ReportArgs) {
    use std::fs;
    use std::io::Write;

    println!("Generating report...");
    println!("Format: {:?}", args.format);
    println!("Output: {}", args.output.display());

    // Create parent directories if needed
    if let Some(parent) = args.output.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Generate report based on format
    let report_content = match args.format {
        probar_cli::ReportFormat::Html => generate_html_report(),
        probar_cli::ReportFormat::Json => generate_json_report(),
        probar_cli::ReportFormat::Lcov => generate_lcov_report(),
        probar_cli::ReportFormat::Junit => generate_junit_report(),
        probar_cli::ReportFormat::Cobertura => generate_cobertura_report(),
    };

    match fs::File::create(&args.output) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(report_content.as_bytes()) {
                eprintln!("Failed to write report: {e}");
                return;
            }
            println!("Report generated at: {}", args.output.display());
        }
        Err(e) => {
            eprintln!("Failed to create report file: {e}");
            return;
        }
    }

    if args.open {
        println!("Opening report in browser...");
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&args.output).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(&args.output)
            .spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("start")
            .arg(&args.output)
            .spawn();
    }
}

fn run_coverage(_config: &CliConfig, args: &probar_cli::CoverageArgs) -> CliResult<()> {
    use jugar_probar::pixel_coverage::{ColorPalette, CoverageCell, PngHeatmap};

    println!("Generating coverage heatmap...");

    // Load coverage data from input file or use sample data
    let cells: Vec<Vec<CoverageCell>> = if let Some(ref input) = args.input {
        println!("Loading coverage data from {}...", input.display());
        load_coverage_from_json(input)?
    } else {
        println!("No input file specified, using sample data");
        create_sample_coverage_data()
    };

    // Select palette
    let palette = match args.palette {
        probar_cli::PaletteArg::Viridis => ColorPalette::viridis(),
        probar_cli::PaletteArg::Magma => ColorPalette::magma(),
        probar_cli::PaletteArg::Heat => ColorPalette::heat(),
    };

    // Build heatmap
    let mut heatmap = PngHeatmap::new(args.width, args.height).with_palette(palette);

    if args.legend {
        heatmap = heatmap.with_legend();
    }

    if args.gaps {
        heatmap = heatmap.with_gap_highlighting();
    }

    if let Some(ref title) = args.title {
        heatmap = heatmap.with_title(title);
    }

    // Export PNG if path provided
    if let Some(ref png_path) = args.png {
        heatmap
            .export_to_file(&cells, png_path)
            .map_err(|e| probar_cli::CliError::report_generation(e.to_string()))?;
        println!("PNG heatmap exported to: {}", png_path.display());
    }

    // Export JSON if path provided
    if let Some(ref json_path) = args.json {
        let report = generate_coverage_report(&cells);
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| probar_cli::CliError::report_generation(e.to_string()))?;
        std::fs::write(json_path, json)
            .map_err(|e| probar_cli::CliError::report_generation(e.to_string()))?;
        println!("Coverage report exported to: {}", json_path.display());
    }

    // Print summary if no output specified
    if args.png.is_none() && args.json.is_none() {
        let report = generate_coverage_report(&cells);
        println!("\nCoverage Summary:");
        println!(
            "  Overall Coverage: {:.1}%",
            report.overall_coverage * 100.0
        );
        println!(
            "  Covered Cells: {}/{}",
            report.covered_cells, report.total_cells
        );
        println!(
            "  Meets Threshold: {}",
            if report.meets_threshold { "✓" } else { "✗" }
        );
        println!("\nUse --png <path> to export a heatmap image.");
    }

    Ok(())
}

/// Load coverage data from a JSON file
fn load_coverage_from_json(path: &std::path::Path) -> CliResult<Vec<Vec<CoverageCell>>> {
    // Expected JSON format: { "cells": [[{coverage: f32, hit_count: u64}, ...], ...] }
    // or just: [[{coverage: f32, hit_count: u64}, ...], ...]
    #[derive(serde::Deserialize)]
    struct CoverageData {
        cells: Option<Vec<Vec<CoverageCell>>>,
        #[serde(flatten)]
        _extra: std::collections::HashMap<String, serde_json::Value>,
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        probar_cli::CliError::report_generation(format!("Failed to read {}: {}", path.display(), e))
    })?;

    // Try parsing as wrapped format first
    if let Ok(data) = serde_json::from_str::<CoverageData>(&content) {
        if let Some(cells) = data.cells {
            return Ok(cells);
        }
    }

    // Try parsing as bare array
    serde_json::from_str::<Vec<Vec<CoverageCell>>>(&content)
        .map_err(|e| probar_cli::CliError::report_generation(format!("Invalid JSON format: {}", e)))
}

/// Check if a cell is in a gap region (no coverage)
fn is_gap_cell(row: usize, col: usize) -> bool {
    // Gap in the middle of row 5
    let middle_gap = row == 5 && (5..=7).contains(&col);
    // Gap at the end of row 2
    let end_gap = row == 2 && col > 10;
    middle_gap || end_gap
}

/// Calculate coverage value for a cell based on position
fn calculate_coverage(row: usize, col: usize, rows: usize, cols: usize) -> f32 {
    if is_gap_cell(row, col) {
        return 0.0;
    }
    let x_factor = col as f32 / (cols - 1) as f32;
    let y_factor = row as f32 / (rows - 1) as f32;
    (x_factor + y_factor) / 2.0
}

/// Create sample coverage data for demonstration
fn create_sample_coverage_data() -> Vec<Vec<CoverageCell>> {
    const ROWS: usize = 10;
    const COLS: usize = 15;

    (0..ROWS)
        .map(|row| {
            (0..COLS)
                .map(|col| {
                    let coverage = calculate_coverage(row, col, ROWS, COLS);
                    CoverageCell {
                        coverage,
                        hit_count: (coverage * 10.0) as u64,
                    }
                })
                .collect()
        })
        .collect()
}

/// Generate coverage report from cells
fn generate_coverage_report(
    cells: &[Vec<CoverageCell>],
) -> jugar_probar::pixel_coverage::PixelCoverageReport {
    use jugar_probar::pixel_coverage::PixelCoverageReport;

    let total_cells = cells.iter().map(|r| r.len()).sum::<usize>() as u32;
    let covered_cells = cells
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| c.coverage > 0.0)
        .count() as u32;

    let overall_coverage = if total_cells > 0 {
        covered_cells as f32 / total_cells as f32
    } else {
        0.0
    };

    PixelCoverageReport {
        grid_width: cells.first().map_or(0, |r| r.len() as u32),
        grid_height: cells.len() as u32,
        overall_coverage,
        covered_cells,
        total_cells,
        min_coverage: 0.0,
        max_coverage: 1.0,
        total_interactions: 0,
        meets_threshold: overall_coverage >= 0.8,
        uncovered_regions: Vec::new(),
    }
}

/// Generate HTML test report
fn generate_html_report() -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Probar Test Report</title>
    <style>
        body {{ font-family: system-ui, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }}
        .summary {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 20px; margin: 20px 0; }}
        .stat {{ background: #f9f9f9; padding: 20px; border-radius: 8px; text-align: center; }}
        .stat-value {{ font-size: 2em; font-weight: bold; color: #4CAF50; }}
        .stat-label {{ color: #666; margin-top: 5px; }}
        .timestamp {{ color: #999; font-size: 0.9em; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Probar Test Report</h1>
        <p class="timestamp">Generated: {timestamp}</p>
        <div class="summary">
            <div class="stat"><div class="stat-value">0</div><div class="stat-label">Tests Run</div></div>
            <div class="stat"><div class="stat-value">0</div><div class="stat-label">Passed</div></div>
            <div class="stat"><div class="stat-value">0</div><div class="stat-label">Failed</div></div>
            <div class="stat"><div class="stat-value">0ms</div><div class="stat-label">Duration</div></div>
        </div>
        <p>Run <code>probar test</code> to generate test results.</p>
    </div>
</body>
</html>"#
    )
}

/// Generate JSON test report
fn generate_json_report() -> String {
    let timestamp = chrono::Utc::now().to_rfc3339();
    format!(
        r#"{{
  "version": "1.0",
  "timestamp": "{timestamp}",
  "summary": {{
    "total": 0,
    "passed": 0,
    "failed": 0,
    "skipped": 0,
    "duration_ms": 0
  }},
  "tests": []
}}"#
    )
}

/// Generate LCOV coverage report
fn generate_lcov_report() -> String {
    "TN:\nSF:src/lib.rs\nDA:1,0\nLF:1\nLH:0\nend_of_record\n".to_string()
}

/// Generate JUnit XML report
fn generate_junit_report() -> String {
    let timestamp = chrono::Utc::now().to_rfc3339();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="probar" tests="0" failures="0" errors="0" time="0" timestamp="{timestamp}">
  <testsuite name="probar" tests="0" failures="0" errors="0" time="0">
  </testsuite>
</testsuites>"#
    )
}

/// Generate Cobertura XML report
fn generate_cobertura_report() -> String {
    let timestamp = chrono::Utc::now().timestamp();
    format!(
        r#"<?xml version="1.0" ?>
<!DOCTYPE coverage SYSTEM "http://cobertura.sourceforge.net/xml/coverage-04.dtd">
<coverage version="1.0" timestamp="{timestamp}" lines-valid="0" lines-covered="0" line-rate="0" branches-valid="0" branches-covered="0" branch-rate="0" complexity="0">
  <packages>
  </packages>
</coverage>"#
    )
}

fn run_init(args: &probar_cli::InitArgs) {
    println!("Initializing Probar project in: {}", args.path.display());

    if args.force {
        println!("Force mode enabled - overwriting existing files");
    }

    // Create project directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&args.path) {
        eprintln!("Failed to create directory: {e}");
        return;
    }

    // Create basic test file
    let test_file = args.path.join("tests").join("basic_test.rs");
    if let Some(parent) = test_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let test_content = r#"//! Basic Probar test
use jugar_probar::prelude::*;

#[test]
fn test_example() {
    let result = TestResult::pass("example_test");
    assert!(result.passed);
}
"#;
    if !test_file.exists() || args.force {
        let _ = std::fs::write(&test_file, test_content);
        println!("Created: {}", test_file.display());
    }

    println!("Probar project initialized successfully!");
}

fn run_config(config: &CliConfig, args: &probar_cli::ConfigArgs) {
    if args.show {
        println!("Current configuration:");
        println!("  Verbosity: {:?}", config.verbosity);
        println!("  Color: {:?}", config.color);
        println!("  Parallel jobs: {}", config.effective_jobs());
        println!("  Fail fast: {}", config.fail_fast);
        println!("  Coverage: {}", config.coverage);
        println!("  Output dir: {}", config.output_dir);
    }

    if let Some(ref setting) = args.set {
        // Parse key=value format
        if let Some((key, value)) = setting.split_once('=') {
            println!("Setting {key} = {value}");
            // Config persistence would require a config file (e.g., .probar.toml)
            // For now, settings are applied via CLI flags only
            println!("Note: Settings are applied via CLI flags. Use environment variables for persistence.");
        } else {
            eprintln!("Invalid setting format. Use: key=value");
        }
    }

    if args.reset {
        println!("Configuration reset to defaults:");
        let default = CliConfig::new();
        println!("  Verbosity: {:?}", default.verbosity);
        println!("  Color: {:?}", default.color);
        println!("  Parallel jobs: {}", default.effective_jobs());
        println!("  Fail fast: {}", default.fail_fast);
        println!("  Coverage: {}", default.coverage);
    }
}

// =============================================================================
// WASM Development Commands
// =============================================================================

fn run_serve(args: &probar_cli::ServeArgs) -> CliResult<()> {
    use probar_cli::{DevServer, DevServerConfig};

    let config = DevServerConfig {
        directory: args.directory.clone(),
        port: args.port,
        ws_port: args.ws_port,
        cors: args.cors,
        cross_origin_isolated: args.cross_origin_isolated,
    };

    let server = DevServer::new(config);

    // Open browser if requested
    if args.open {
        let url = format!("http://localhost:{}", args.port);
        println!("Opening browser at {url}...");
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("start").arg(&url).spawn();
    }

    // Run server (blocking)
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probar_cli::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    rt.block_on(async {
        server
            .run()
            .await
            .map_err(|e| probar_cli::CliError::test_execution(format!("Server error: {e}")))
    })
}

fn run_build(args: &probar_cli::BuildArgs) -> CliResult<()> {
    use probar_cli::dev_server::run_wasm_pack_build;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probar_cli::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    rt.block_on(async {
        run_wasm_pack_build(
            &args.path,
            args.target.as_str(),
            args.release,
            args.out_dir.as_deref(),
            args.profiling,
        )
        .await
        .map_err(|e| probar_cli::CliError::test_execution(e))
    })
}

fn run_watch(args: &probar_cli::WatchArgs) -> CliResult<()> {
    use probar_cli::{dev_server::run_wasm_pack_build, DevServer, DevServerConfig, FileWatcher};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probar_cli::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    let path = args.path.clone();
    let target = args.target.as_str().to_string();
    let release = args.release;

    // Initial build
    println!("Performing initial build...");
    rt.block_on(async {
        run_wasm_pack_build(&path, &target, release, None, false)
            .await
            .map_err(|e| probar_cli::CliError::test_execution(e))
    })?;

    // Start server if requested
    let server_handle = if args.serve {
        let config = DevServerConfig {
            directory: args.path.join("pkg"),
            port: args.port,
            ws_port: args.ws_port,
            cors: true,
            cross_origin_isolated: false,
        };
        let server = DevServer::new(config);
        let reload_tx = server.reload_sender();

        // Spawn server in background
        let server_handle = rt.spawn(async move {
            let _ = server.run().await;
        });

        Some((server_handle, reload_tx))
    } else {
        None
    };

    // File watcher
    println!("\nWatching for changes in {}...", args.path.display());
    println!("Press Ctrl+C to stop\n");

    let watcher = FileWatcher::new(args.path.clone(), args.debounce);
    let path_for_rebuild = args.path.clone();
    let target_for_rebuild = args.target.as_str().to_string();
    let release_for_rebuild = args.release;
    let reload_tx = server_handle.as_ref().map(|(_, tx)| tx.clone());

    let rebuild_in_progress = Arc::new(Mutex::new(false));

    rt.block_on(async {
        watcher
            .watch(move |changed_file| {
                let rebuild_in_progress = rebuild_in_progress.clone();
                let path = path_for_rebuild.clone();
                let target = target_for_rebuild.clone();
                let reload_tx = reload_tx.clone();

                // Use a separate runtime for the rebuild since we're in a sync callback
                let rt = tokio::runtime::Handle::current();
                rt.spawn(async move {
                    // Check if rebuild is already in progress
                    let mut in_progress = rebuild_in_progress.lock().await;
                    if *in_progress {
                        return;
                    }
                    *in_progress = true;
                    drop(in_progress);

                    println!(
                        "\n[{}] File changed: {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        changed_file
                    );

                    if let Some(ref tx) = reload_tx {
                        let _ = tx.send(probar_cli::dev_server::HotReloadMessage::FileChanged {
                            path: changed_file.clone(),
                        });
                    }

                    println!("Rebuilding...");
                    let build_start = std::time::Instant::now();
                    match run_wasm_pack_build(&path, &target, release_for_rebuild, None, false)
                        .await
                    {
                        Ok(()) => {
                            let duration_ms = build_start.elapsed().as_millis() as u64;
                            println!("Build successful!");
                            if let Some(ref tx) = reload_tx {
                                let _ = tx.send(
                                    probar_cli::dev_server::HotReloadMessage::RebuildComplete {
                                        duration_ms,
                                    },
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Build failed: {e}");
                            if let Some(ref tx) = reload_tx {
                                let _ = tx.send(
                                    probar_cli::dev_server::HotReloadMessage::RebuildFailed {
                                        error: e,
                                    },
                                );
                            }
                        }
                    }

                    let mut in_progress = rebuild_in_progress.lock().await;
                    *in_progress = false;
                });
            })
            .await
            .map_err(|e| probar_cli::CliError::test_execution(format!("Watch error: {e}")))
    })
}

fn run_playbook(config: &CliConfig, args: &probar_cli::PlaybookArgs) -> CliResult<()> {
    use jugar_probar::playbook::{
        to_dot, to_svg, MutationClass, MutationGenerator, Playbook, StateMachineValidator,
    };

    if config.verbosity != Verbosity::Quiet {
        println!("Running playbook(s)...");
    }

    let mut all_passed = true;

    for file in &args.files {
        if config.verbosity != Verbosity::Quiet {
            println!("\nProcessing: {}", file.display());
        }

        // Load playbook from YAML file
        let yaml_content = std::fs::read_to_string(file).map_err(|e| {
            probar_cli::CliError::test_execution(format!(
                "Failed to read playbook {}: {}",
                file.display(),
                e
            ))
        })?;

        let playbook = Playbook::from_yaml(&yaml_content).map_err(|e| {
            probar_cli::CliError::test_execution(format!(
                "Failed to parse playbook {}: {}",
                file.display(),
                e
            ))
        })?;

        // Validate the state machine
        let validator = StateMachineValidator::new(&playbook);
        let validation_result = validator.validate();

        if config.verbosity != Verbosity::Quiet {
            println!("  State machine: {}", playbook.machine.id);
            println!("  States: {}", playbook.machine.states.len());
            println!("  Transitions: {}", playbook.machine.transitions.len());
            println!(
                "  Valid: {}",
                if validation_result.is_valid {
                    "yes"
                } else {
                    "no"
                }
            );
        }

        if !validation_result.is_valid {
            all_passed = false;
            for issue in &validation_result.issues {
                eprintln!("  Issue: {issue:?}");
            }
        }

        // Handle --validate (dry run)
        if args.validate {
            if config.verbosity != Verbosity::Quiet {
                println!("  Validation only mode - skipping execution");
            }
            continue;
        }

        // Handle --export
        if let Some(ref format) = args.export {
            let diagram = match format {
                probar_cli::DiagramFormat::Dot => to_dot(&playbook),
                probar_cli::DiagramFormat::Svg => to_svg(&playbook),
            };

            if let Some(ref output_path) = args.export_output {
                std::fs::write(output_path, &diagram).map_err(|e| {
                    probar_cli::CliError::report_generation(format!(
                        "Failed to write diagram: {}",
                        e
                    ))
                })?;
                if config.verbosity != Verbosity::Quiet {
                    println!("  Diagram exported to: {}", output_path.display());
                }
            } else {
                println!("{diagram}");
            }
        }

        // Handle --mutate
        if args.mutate {
            let generator = MutationGenerator::new(&playbook);

            let classes_to_run: Vec<MutationClass> =
                if let Some(ref class_names) = args.mutation_classes {
                    class_names
                        .iter()
                        .filter_map(|name| match name.as_str() {
                            "M1" => Some(MutationClass::StateRemoval),
                            "M2" => Some(MutationClass::TransitionRemoval),
                            "M3" => Some(MutationClass::EventSwap),
                            "M4" => Some(MutationClass::TargetSwap),
                            "M5" => Some(MutationClass::GuardNegation),
                            _ => {
                                eprintln!("Unknown mutation class: {name}");
                                None
                            }
                        })
                        .collect()
                } else {
                    MutationClass::all()
                };

            if config.verbosity != Verbosity::Quiet {
                println!(
                    "  Running mutation testing ({} classes)...",
                    classes_to_run.len()
                );
            }

            let mut total_mutants = 0;
            for class in &classes_to_run {
                let mutants = generator.generate(*class);
                if config.verbosity != Verbosity::Quiet {
                    println!("    {}: {} mutants", class.id(), mutants.len());
                }
                total_mutants += mutants.len();
            }

            if config.verbosity != Verbosity::Quiet {
                println!("  Total mutants generated: {total_mutants}");
            }
        }

        // Output results based on format
        match args.format {
            probar_cli::PlaybookOutputFormat::Json => {
                let result = serde_json::json!({
                    "file": file.display().to_string(),
                    "machine_id": playbook.machine.id,
                    "states": playbook.machine.states.len(),
                    "transitions": playbook.machine.transitions.len(),
                    "valid": validation_result.is_valid,
                    "issues": validation_result.issues.len(),
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            }
            probar_cli::PlaybookOutputFormat::Junit => {
                let timestamp = chrono::Utc::now().to_rfc3339();
                let failures = i32::from(!validation_result.is_valid);
                let junit = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="playbook" tests="1" failures="{}" timestamp="{}">
  <testsuite name="{}" tests="1" failures="{}">
    <testcase name="validation" classname="{}">
      {}
    </testcase>
  </testsuite>
</testsuites>"#,
                    failures,
                    timestamp,
                    playbook.machine.id,
                    failures,
                    file.display(),
                    if validation_result.is_valid {
                        String::new()
                    } else {
                        format!(
                            "<failure message=\"Validation failed\">{} issues</failure>",
                            validation_result.issues.len()
                        )
                    }
                );
                println!("{junit}");
            }
            probar_cli::PlaybookOutputFormat::Text => {
                // Already printed above
            }
        }

        if args.fail_fast && !validation_result.is_valid {
            return Err(probar_cli::CliError::test_execution(
                "Playbook validation failed (--fail-fast)".to_string(),
            ));
        }
    }

    if all_passed {
        Ok(())
    } else {
        Err(probar_cli::CliError::test_execution(
            "One or more playbooks failed validation".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use probar_cli::{
        ConfigArgs, CoverageArgs, InitArgs, PaletteArg, RecordArgs, RecordFormat, ReportArgs,
        ReportFormat,
    };
    use std::path::PathBuf;

    mod build_config_tests {
        use super::*;

        #[test]
        fn test_build_config_default() {
            let cli = Cli::parse_from(["probar", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.verbosity, Verbosity::Normal);
        }

        #[test]
        fn test_build_config_verbose() {
            let cli = Cli::parse_from(["probar", "-v", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.verbosity, Verbosity::Verbose);
        }

        #[test]
        fn test_build_config_debug() {
            let cli = Cli::parse_from(["probar", "-vv", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.verbosity, Verbosity::Debug);
        }

        #[test]
        fn test_build_config_very_verbose() {
            let cli = Cli::parse_from(["probar", "-vvv", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.verbosity, Verbosity::Debug);
        }

        #[test]
        fn test_build_config_quiet() {
            let cli = Cli::parse_from(["probar", "-q", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.verbosity, Verbosity::Quiet);
        }

        #[test]
        fn test_build_config_color_never() {
            let cli = Cli::parse_from(["probar", "--color", "never", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.color, ColorChoice::Never);
        }

        #[test]
        fn test_build_config_color_always() {
            let cli = Cli::parse_from(["probar", "--color", "always", "test"]);
            let config = build_config(&cli);
            assert_eq!(config.color, ColorChoice::Always);
        }
    }

    mod run_record_tests {
        use super::*;

        #[test]
        fn test_run_record() {
            let config = CliConfig::default();
            let args = RecordArgs {
                test: "my_test".to_string(),
                format: RecordFormat::Gif,
                output: None,
                fps: 10,
                quality: 80,
            };
            run_record(&config, &args);
            // Just verify it doesn't panic
        }

        #[test]
        fn test_run_record_png() {
            let config = CliConfig::default();
            let args = RecordArgs {
                test: "another_test".to_string(),
                format: RecordFormat::Png,
                output: Some(PathBuf::from("output.png")),
                fps: 30,
                quality: 100,
            };
            run_record(&config, &args);
        }
    }

    mod run_report_tests {
        use super::*;

        #[test]
        fn test_run_report_html() {
            let config = CliConfig::default();
            let args = ReportArgs {
                format: ReportFormat::Html,
                output: PathBuf::from("/tmp/probar_test_report"),
                open: false,
            };
            run_report(&config, &args);
        }

        #[test]
        fn test_run_report_json() {
            let config = CliConfig::default();
            let args = ReportArgs {
                format: ReportFormat::Json,
                output: PathBuf::from("/tmp/probar_test_report.json"),
                open: false,
            };
            run_report(&config, &args);
        }

        #[test]
        fn test_run_report_with_open() {
            let config = CliConfig::default();
            let args = ReportArgs {
                format: ReportFormat::Html,
                output: PathBuf::from("/tmp/probar_test_report_open"),
                open: true,
            };
            run_report(&config, &args);
        }
    }

    mod run_init_tests {
        use super::*;
        use std::fs;

        #[test]
        fn test_run_init_basic() {
            let temp_dir = std::env::temp_dir().join("probar_init_test");
            let _ = fs::remove_dir_all(&temp_dir);

            let args = InitArgs {
                path: temp_dir.clone(),
                force: false,
            };
            run_init(&args);

            // Cleanup
            let _ = fs::remove_dir_all(&temp_dir);
        }

        #[test]
        fn test_run_init_force() {
            let temp_dir = std::env::temp_dir().join("probar_init_force_test");
            let _ = fs::remove_dir_all(&temp_dir);

            let args = InitArgs {
                path: temp_dir.clone(),
                force: true,
            };
            run_init(&args);

            // Run again with force
            run_init(&args);

            // Cleanup
            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    mod run_config_tests {
        use super::*;

        #[test]
        fn test_run_config_show() {
            let config = CliConfig::default();
            let args = ConfigArgs {
                show: true,
                set: None,
                reset: false,
            };
            run_config(&config, &args);
        }

        #[test]
        fn test_run_config_set_valid() {
            let config = CliConfig::default();
            let args = ConfigArgs {
                show: false,
                set: Some("key=value".to_string()),
                reset: false,
            };
            run_config(&config, &args);
        }

        #[test]
        fn test_run_config_set_invalid() {
            let config = CliConfig::default();
            let args = ConfigArgs {
                show: false,
                set: Some("invalid_format".to_string()),
                reset: false,
            };
            run_config(&config, &args);
        }

        #[test]
        fn test_run_config_reset() {
            let config = CliConfig::default();
            let args = ConfigArgs {
                show: false,
                set: None,
                reset: true,
            };
            run_config(&config, &args);
        }

        #[test]
        fn test_run_config_all_flags() {
            let config = CliConfig::default();
            let args = ConfigArgs {
                show: true,
                set: Some("test=value".to_string()),
                reset: true,
            };
            run_config(&config, &args);
        }
    }

    mod run_tests_tests {
        use super::*;
        use probar_cli::TestArgs;

        #[test]
        #[ignore] // Spawns `cargo test --list` subprocess - causes nested builds in CI
        fn test_run_tests_no_tests() {
            let config = CliConfig::default();
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
            // run_tests returns Ok when no tests are found
            let result = run_tests(config, &args);
            assert!(result.is_ok());
        }

        #[test]
        #[ignore] // Spawns `cargo test --list` subprocess - causes nested builds in CI
        fn test_run_tests_with_filter() {
            let config = CliConfig::default();
            let args = TestArgs {
                filter: Some("game::*".to_string()),
                parallel: 4,
                coverage: true,
                mutants: false,
                fail_fast: true,
                watch: false,
                timeout: 5000,
                output: PathBuf::from("target/test_output"),
            };
            let result = run_tests(config, &args);
            assert!(result.is_ok());
        }
    }

    mod run_coverage_tests {
        use super::*;

        #[test]
        fn test_run_coverage_no_output() {
            let config = CliConfig::default();
            let args = CoverageArgs {
                png: None,
                json: None,
                palette: PaletteArg::Viridis,
                legend: false,
                gaps: false,
                title: None,
                width: 400,
                height: 300,
                input: None,
            };
            let result = run_coverage(&config, &args);
            assert!(result.is_ok());
        }

        #[test]
        fn test_run_coverage_with_png() {
            let temp_dir = std::env::temp_dir();
            let png_path = temp_dir.join("test_coverage.png");

            let config = CliConfig::default();
            let args = CoverageArgs {
                png: Some(png_path.clone()),
                json: None,
                palette: PaletteArg::Magma,
                legend: true,
                gaps: true,
                title: Some("Test Coverage".to_string()),
                width: 800,
                height: 600,
                input: None,
            };

            let result = run_coverage(&config, &args);
            assert!(result.is_ok());

            // Verify PNG was created
            assert!(png_path.exists());

            // Cleanup
            let _ = std::fs::remove_file(&png_path);
        }

        #[test]
        fn test_run_coverage_with_json() {
            let temp_dir = std::env::temp_dir();
            let json_path = temp_dir.join("test_coverage.json");

            let config = CliConfig::default();
            let args = CoverageArgs {
                png: None,
                json: Some(json_path.clone()),
                palette: PaletteArg::Heat,
                legend: false,
                gaps: false,
                title: None,
                width: 640,
                height: 480,
                input: None,
            };

            let result = run_coverage(&config, &args);
            assert!(result.is_ok());

            // Verify JSON was created
            assert!(json_path.exists());

            // Verify JSON content
            let content = std::fs::read_to_string(&json_path).unwrap();
            assert!(content.contains("overall_coverage"));

            // Cleanup
            let _ = std::fs::remove_file(&json_path);
        }

        #[test]
        fn test_create_sample_coverage_data() {
            let cells = create_sample_coverage_data();
            assert_eq!(cells.len(), 10);
            assert_eq!(cells[0].len(), 15);

            // Check for gaps
            assert_eq!(cells[5][5].coverage, 0.0);
            assert_eq!(cells[5][6].coverage, 0.0);
            assert_eq!(cells[2][11].coverage, 0.0);
        }

        #[test]
        fn test_generate_coverage_report() {
            let cells = create_sample_coverage_data();
            let report = generate_coverage_report(&cells);

            assert_eq!(report.total_cells, 150);
            assert!(report.covered_cells < 150); // Some gaps exist
            assert!(report.overall_coverage > 0.0);
            assert!(report.overall_coverage < 1.0);
        }
    }
}
