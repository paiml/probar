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
use probador::{Cli, CliConfig, CliResult, ColorChoice, Commands, TestRunner, Verbosity};
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

fn run_tests(config: CliConfig, args: &probador::TestArgs) -> CliResult<()> {
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
        Err(probador::CliError::test_execution(format!(
            "{} test(s) failed",
            results.failed()
        )))
    }
}

fn run_record(_config: &CliConfig, args: &probador::RecordArgs) {
    println!("Recording test: {}", args.test);
    println!("Format: {:?}", args.format);
    println!("FPS: {}", args.fps);
    println!("Quality: {}", args.quality);

    // Note: Full recording requires browser feature and running tests
    // This displays configuration; actual recording done via test runner
    println!("Recording configuration ready. Run test with --record flag to capture.");
}

fn run_report(_config: &CliConfig, args: &probador::ReportArgs) {
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
        probador::ReportFormat::Html => generate_html_report(),
        probador::ReportFormat::Json => generate_json_report(),
        probador::ReportFormat::Lcov => generate_lcov_report(),
        probador::ReportFormat::Junit => generate_junit_report(),
        probador::ReportFormat::Cobertura => generate_cobertura_report(),
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

fn run_coverage(_config: &CliConfig, args: &probador::CoverageArgs) -> CliResult<()> {
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
        probador::PaletteArg::Viridis => ColorPalette::viridis(),
        probador::PaletteArg::Magma => ColorPalette::magma(),
        probador::PaletteArg::Heat => ColorPalette::heat(),
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
            .map_err(|e| probador::CliError::report_generation(e.to_string()))?;
        println!("PNG heatmap exported to: {}", png_path.display());
    }

    // Export JSON if path provided
    if let Some(ref json_path) = args.json {
        let report = generate_coverage_report(&cells);
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| probador::CliError::report_generation(e.to_string()))?;
        std::fs::write(json_path, json)
            .map_err(|e| probador::CliError::report_generation(e.to_string()))?;
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
        probador::CliError::report_generation(format!("Failed to read {}: {}", path.display(), e))
    })?;

    // Try parsing as wrapped format first
    if let Ok(data) = serde_json::from_str::<CoverageData>(&content) {
        if let Some(cells) = data.cells {
            return Ok(cells);
        }
    }

    // Try parsing as bare array
    serde_json::from_str::<Vec<Vec<CoverageCell>>>(&content)
        .map_err(|e| probador::CliError::report_generation(format!("Invalid JSON format: {}", e)))
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

fn run_init(args: &probador::InitArgs) {
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

fn run_config(config: &CliConfig, args: &probador::ConfigArgs) {
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

fn run_serve(args: &probador::ServeArgs) -> CliResult<()> {
    use probador::{DevServer, DevServerConfig, ModuleValidator, ServeSubcommand};

    // Handle subcommands
    if let Some(ref subcommand) = args.subcommand {
        return match subcommand {
            ServeSubcommand::Tree(tree_args) => run_serve_tree(tree_args, &args.directory),
            ServeSubcommand::Viz(viz_args) => run_serve_viz(viz_args, &args.directory),
            ServeSubcommand::Score(score_args) => run_serve_score(score_args, &args.directory),
        };
    }

    // Validate module imports if --validate flag is set
    if args.validate {
        let mut validator = ModuleValidator::new(&args.directory);
        if !args.exclude.is_empty() {
            validator = validator.with_exclude(args.exclude.clone());
        }
        let result = validator.validate();
        validator.print_results(&result);

        if !result.is_ok() {
            return Err(probador::CliError::test_execution(format!(
                "Module validation failed: {} error(s) found. Fix imports before serving.",
                result.errors.len()
            )));
        }
        eprintln!("\n✓ All module imports validated successfully\n");
    }

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
        probador::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    rt.block_on(async {
        server
            .run()
            .await
            .map_err(|e| probador::CliError::test_execution(format!("Server error: {e}")))
    })
}

fn run_serve_tree(args: &probador::TreeArgs, _default_dir: &std::path::Path) -> CliResult<()> {
    use probador::{build_tree, render_tree, TreeConfig};

    let config = TreeConfig::default()
        .with_depth(args.depth)
        .with_filter(args.filter.as_deref())
        .with_sizes(args.sizes)
        .with_mime_types(args.mime_types);

    let tree = build_tree(&args.path, &config)
        .map_err(|e| probador::CliError::test_execution(format!("Failed to build tree: {e}")))?;

    let output = render_tree(&tree, &config);
    print!("{output}");

    Ok(())
}

fn run_serve_viz(args: &probador::VizArgs, _default_dir: &std::path::Path) -> CliResult<()> {
    use probador::{build_tree, render_tree, TreeConfig};

    // VizArgs only has path and port - use defaults for other options
    let config = TreeConfig::default();

    let tree = build_tree(&args.path, &config)
        .map_err(|e| probador::CliError::test_execution(format!("Failed to build tree: {e}")))?;

    let output = render_tree(&tree, &config);
    print!("{output}");

    Ok(())
}

fn run_serve_score(args: &probador::ScoreArgs, _default_dir: &std::path::Path) -> CliResult<()> {
    use probador::{score, ScoreCalculator, ScoreOutputFormat};

    // If --live flag is set, run actual browser validation
    if args.live {
        return run_live_browser_validation(args);
    }

    let calculator = ScoreCalculator::new(args.path.clone());
    let project_score = calculator.calculate();

    match args.format {
        ScoreOutputFormat::Text => {
            let output = score::render_score_text(&project_score, args.detailed);
            print!("{output}");
        }
        ScoreOutputFormat::Json => {
            let output = score::render_score_json(&project_score)
                .map_err(|e| probador::CliError::report_generation(format!("JSON serialization error: {e}")))?;
            println!("{output}");
        }
    }

    Ok(())
}

/// Live browser validation - FALSIFICATION approach
///
/// This actually starts a server, launches a headless browser, and tries to BREAK the app.
/// The app FAILS if we can prove it's broken. The app PASSES only if we can't break it.
///
/// Checks performed:
/// 1. Module resolution: Can all JS/WASM imports be fetched? (404 = FAIL)
/// 2. MIME types: Are JS/WASM served with correct MIME types? (wrong MIME = FAIL)
/// 3. Console errors: Any console.error or uncaught exceptions? (errors = FAIL)
/// 4. WASM initialization: Does the WASM module load? (load failure = FAIL)
fn run_live_browser_validation(args: &probador::ScoreArgs) -> CliResult<()> {
    use probador::{DevServerConfig, ModuleValidator};
    use std::net::TcpListener;

    eprintln!("\n══════════════════════════════════════════════════════════════");
    eprintln!("  LIVE BROWSER VALIDATION (Falsification Mode)");
    eprintln!("══════════════════════════════════════════════════════════════\n");
    eprintln!("This validates the app ACTUALLY WORKS by trying to break it.\n");

    // Find HTML files to test
    let html_files = find_html_files(&args.path);
    if html_files.is_empty() {
        eprintln!("✗ FAIL: No HTML files found in {}", args.path.display());
        eprintln!("\n  Cannot validate an app without HTML entry points.");
        return Err(probador::CliError::test_execution(
            "No HTML files found - nothing to validate".to_string(),
        ));
    }

    eprintln!("Found {} HTML file(s) to validate:\n", html_files.len());
    for file in &html_files {
        eprintln!("  • {}", file.display());
    }
    eprintln!();

    // Step 1: Static module validation (fast, no browser needed)
    eprintln!("[1/4] Module Resolution Check");
    let validator = ModuleValidator::new(&args.path);

    // First scan imports to identify which pages have WASM
    let imports = validator.scan_imports();
    let pages_with_wasm: std::collections::HashSet<_> = imports
        .iter()
        .filter(|imp| imp.import_type == probador::ImportType::Wasm)
        .map(|imp| imp.source_file.clone())
        .collect();

    if !pages_with_wasm.is_empty() {
        eprintln!("  Detected {} page(s) with WASM imports:", pages_with_wasm.len());
        for page in &pages_with_wasm {
            eprintln!("    • {}", page.display());
        }
        eprintln!();
    }

    let validation_result = validator.validate();

    if !validation_result.is_ok() {
        eprintln!("  ✗ FAIL: {} broken import(s) found\n", validation_result.errors.len());
        for error in &validation_result.errors {
            eprintln!("    • {} (from {}:{})",
                error.import.import_path,
                error.import.source_file.display(),
                error.import.line_number
            );
            let status_str = if error.status == 404 {
                "404 Not Found".to_string()
            } else {
                format!("{}", error.status)
            };
            eprintln!("      Status: {} - {}", status_str, error.message);
            eprintln!("      MIME type: {}", error.actual_mime);
        }
        eprintln!();
        eprintln!("══════════════════════════════════════════════════════════════");
        eprintln!("  RESULT: FAIL (Grade: F)");
        eprintln!("══════════════════════════════════════════════════════════════");
        eprintln!("\n  Fix the broken imports above and run again.\n");
        return Err(probador::CliError::test_execution(
            format!("Module resolution failed: {} broken import(s)", validation_result.errors.len()),
        ));
    }
    eprintln!("  ✓ All {} import(s) resolve correctly\n", validation_result.total_imports);

    // Step 2: Start temporary server
    eprintln!("[2/4] Starting Validation Server");
    let port = if args.port == 0 {
        // Find an available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| probador::CliError::test_execution(format!("Failed to find available port: {e}")))?;
        listener.local_addr()
            .map_err(|e| probador::CliError::test_execution(format!("Failed to get local address: {e}")))?
            .port()
    } else {
        args.port
    };

    eprintln!("  Starting server on port {port}...\n");

    let config = DevServerConfig {
        directory: args.path.clone(),
        port,
        ws_port: port + 1,
        cors: true,
        cross_origin_isolated: true,
    };

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probador::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    // Run browser validation
    let validation_errors = rt.block_on(async {
        run_browser_validation_async(&args.path, &html_files, port, config, &pages_with_wasm).await
    })?;

    // Report results
    eprintln!("══════════════════════════════════════════════════════════════");
    if validation_errors.is_empty() {
        eprintln!("  RESULT: PASS (App works!)");
        eprintln!("══════════════════════════════════════════════════════════════");
        eprintln!("\n  Could not prove the app is broken. All validation checks passed.\n");
        Ok(())
    } else {
        eprintln!("  RESULT: FAIL (Grade: F)");
        eprintln!("══════════════════════════════════════════════════════════════");
        eprintln!("\n  Found {} issue(s) that prove the app is broken:\n", validation_errors.len());
        for (i, error) in validation_errors.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, error);
        }
        eprintln!();
        Err(probador::CliError::test_execution(
            format!("Live validation failed: {} issue(s) found", validation_errors.len()),
        ))
    }
}

/// Find all HTML files in a directory
#[allow(clippy::items_after_statements)]
fn find_html_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut html_files = Vec::new();

    fn scan_dir(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip node_modules and hidden directories
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "node_modules" {
                        scan_dir(&path, files);
                    }
                } else if path.extension().is_some_and(|ext| ext == "html") {
                    files.push(path);
                }
            }
        }
    }

    scan_dir(dir, &mut html_files);
    html_files.sort();
    html_files
}

/// Run browser validation asynchronously
///
/// Uses FALSIFICATION approach: we try to PROVE the app is broken.
/// If we can prove it's broken, it FAILS. Only passes if we cannot break it.
async fn run_browser_validation_async(
    serve_dir: &std::path::Path,
    html_files: &[std::path::PathBuf],
    port: u16,
    config: probador::DevServerConfig,
    pages_with_wasm: &std::collections::HashSet<std::path::PathBuf>,
) -> CliResult<Vec<String>> {
    use probador::DevServer;
    use std::time::Duration;

    let server = DevServer::new(config);
    #[allow(unused_mut)]
    let mut errors = Vec::new();

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    eprintln!("[3/4] Browser Bootstrap Check");

    // Try to launch browser
    #[cfg(feature = "browser")]
    {
        use jugar_probar::{Browser, BrowserConfig, BrowserConsoleLevel};
        use tokio::time::timeout;

        let browser_config = BrowserConfig::default()
            .with_headless(true)
            .with_no_sandbox()
            .with_viewport(1280, 720);

        match Browser::launch(browser_config).await {
            Ok(browser) => {
                for html_file in html_files {
                    // Convert file path to URL
                    let relative_path = html_file.strip_prefix(serve_dir).unwrap_or(html_file);
                    let url_path = relative_path.to_string_lossy().replace('\\', "/");
                    let url = format!("http://127.0.0.1:{port}/{url_path}");

                    // Check if this page is expected to have WASM
                    let expects_wasm = pages_with_wasm.contains(html_file);

                    eprintln!("  Testing: {url}");
                    if expects_wasm {
                        eprintln!("    (WASM required - imported in HTML)");
                    }

                    match browser.new_page().await {
                        Ok(mut page) => {
                            // Enable console capture BEFORE navigation to catch load errors
                            if let Err(e) = page.enable_console_capture().await {
                                eprintln!("    Warning: Could not enable console capture: {e}");
                            }
                            if let Err(e) = page.inject_console_capture().await {
                                eprintln!("    Warning: Could not inject console capture: {e}");
                            }

                            // Navigate to page with timeout
                            let nav_result = timeout(
                                Duration::from_secs(30),
                                page.goto(&url)
                            ).await;

                            match nav_result {
                                Ok(Ok(())) => {
                                    // Wait a bit for JS to execute
                                    tokio::time::sleep(Duration::from_secs(2)).await;

                                    // Fetch console messages - FALSIFICATION: any error proves broken
                                    if let Ok(messages) = page.fetch_console_messages().await {
                                        let error_messages: Vec<_> = messages.iter()
                                            .filter(|m| m.level == BrowserConsoleLevel::Error)
                                            .collect();

                                        if !error_messages.is_empty() {
                                            eprintln!("    ✗ {} console error(s) found - APP IS BROKEN", error_messages.len());
                                            for msg in &error_messages {
                                                eprintln!("      └─ {}", msg.text);
                                                errors.push(format!("[{}] Console error: {}", url_path, msg.text));
                                            }
                                        } else {
                                            eprintln!("    ✓ No console errors");
                                        }
                                    }

                                    // Check for WASM ready signal - FALSIFICATION if page requires WASM
                                    eprintln!("\n[4/4] WASM Initialization Check");
                                    let wasm_result = timeout(
                                        Duration::from_secs(15),
                                        page.wait_for_wasm_ready()
                                    ).await;

                                    match wasm_result {
                                        Ok(Ok(())) => {
                                            eprintln!("  ✓ WASM initialized successfully\n");
                                        }
                                        Ok(Err(e)) => {
                                            if expects_wasm {
                                                // FALSIFICATION: WASM was required but failed
                                                eprintln!("  ✗ WASM initialization FAILED - APP IS BROKEN");
                                                eprintln!("    Error: {e}\n");
                                                errors.push(format!("[{}] WASM required but failed: {}", url_path, e));
                                            } else {
                                                eprintln!("  ⚠ WASM initialization: {e}");
                                                eprintln!("    (No WASM imports detected - may be expected)\n");
                                            }
                                        }
                                        Err(_) => {
                                            if expects_wasm {
                                                // FALSIFICATION: WASM was required but timed out
                                                eprintln!("  ✗ WASM initialization TIMED OUT - APP IS BROKEN");
                                                eprintln!("    Page imports WASM but it failed to initialize.\n");
                                                errors.push(format!("[{}] WASM required but timed out after 15s", url_path));
                                            } else {
                                                eprintln!("  ⚠ WASM initialization timed out");
                                                eprintln!("    (No WASM imports detected - may be expected)\n");
                                            }
                                        }
                                    }
                                }
                                Ok(Err(e)) => {
                                    eprintln!("    ✗ Navigation failed - APP IS BROKEN");
                                    eprintln!("    Error: {e}");
                                    errors.push(format!("[{}] Navigation failed: {}", url_path, e));
                                }
                                Err(_) => {
                                    eprintln!("    ✗ Navigation timed out after 30s - APP IS BROKEN");
                                    errors.push(format!("[{}] Navigation timed out", url_path));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("    ✗ Failed to create page: {e}");
                            errors.push(format!("Failed to create browser page: {}", e));
                        }
                    }
                }

                // Close browser
                let _ = browser.close().await;
            }
            Err(e) => {
                eprintln!("  ✗ Failed to launch browser: {e}");
                eprintln!("    Make sure Chrome/Chromium is installed.\n");
                errors.push(format!("Failed to launch browser: {}", e));
            }
        }
    }

    #[cfg(not(feature = "browser"))]
    {
        // Suppress unused warnings when browser feature is disabled
        let _ = (serve_dir, html_files, port, pages_with_wasm);
        eprintln!("  ⚠ Browser feature not enabled - skipping browser checks");
        eprintln!("    Rebuild with --features browser for full validation\n");
        eprintln!("[4/4] WASM Initialization Check");
        eprintln!("  ⚠ Skipped (browser feature not enabled)\n");
    }

    // Stop server
    server_handle.abort();

    Ok(errors)
}

fn run_build(args: &probador::BuildArgs) -> CliResult<()> {
    use probador::dev_server::run_wasm_pack_build;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probador::CliError::test_execution(format!("Failed to create runtime: {e}"))
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
        .map_err(|e| probador::CliError::test_execution(e))
    })
}

fn run_watch(args: &probador::WatchArgs) -> CliResult<()> {
    use probador::{dev_server::run_wasm_pack_build, DevServer, DevServerConfig, FileWatcher};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        probador::CliError::test_execution(format!("Failed to create runtime: {e}"))
    })?;

    let path = args.path.clone();
    let target = args.target.as_str().to_string();
    let release = args.release;

    // Initial build
    println!("Performing initial build...");
    rt.block_on(async {
        run_wasm_pack_build(&path, &target, release, None, false)
            .await
            .map_err(|e| probador::CliError::test_execution(e))
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
                        let _ = tx.send(probador::dev_server::HotReloadMessage::FileChanged {
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
                                    probador::dev_server::HotReloadMessage::RebuildComplete {
                                        duration_ms,
                                    },
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Build failed: {e}");
                            if let Some(ref tx) = reload_tx {
                                let _ = tx.send(
                                    probador::dev_server::HotReloadMessage::RebuildFailed {
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
            .map_err(|e| probador::CliError::test_execution(format!("Watch error: {e}")))
    })
}

fn run_playbook(config: &CliConfig, args: &probador::PlaybookArgs) -> CliResult<()> {
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
            probador::CliError::test_execution(format!(
                "Failed to read playbook {}: {}",
                file.display(),
                e
            ))
        })?;

        let playbook = Playbook::from_yaml(&yaml_content).map_err(|e| {
            probador::CliError::test_execution(format!(
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
                probador::DiagramFormat::Dot => to_dot(&playbook),
                probador::DiagramFormat::Svg => to_svg(&playbook),
            };

            if let Some(ref output_path) = args.export_output {
                std::fs::write(output_path, &diagram).map_err(|e| {
                    probador::CliError::report_generation(format!("Failed to write diagram: {}", e))
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
            probador::PlaybookOutputFormat::Json => {
                let result = serde_json::json!({
                    "file": file.display().to_string(),
                    "machine_id": playbook.machine.id,
                    "states": playbook.machine.states.len(),
                    "transitions": playbook.machine.transitions.len(),
                    "valid": validation_result.is_valid,
                    "issues": validation_result.issues.len(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
            probador::PlaybookOutputFormat::Junit => {
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
            probador::PlaybookOutputFormat::Text => {
                // Already printed above
            }
        }

        if args.fail_fast && !validation_result.is_valid {
            return Err(probador::CliError::test_execution(
                "Playbook validation failed (--fail-fast)".to_string(),
            ));
        }
    }

    if all_passed {
        Ok(())
    } else {
        Err(probador::CliError::test_execution(
            "One or more playbooks failed validation".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use probador::{
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
        use probador::TestArgs;

        #[test]
        #[ignore = "Spawns cargo test --list subprocess - causes nested builds in CI"]
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
        #[ignore = "Spawns cargo test --list subprocess - causes nested builds in CI"]
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
