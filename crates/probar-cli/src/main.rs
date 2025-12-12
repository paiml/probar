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
        Commands::Init(args) => {
            run_init(&args);
            Ok(())
        }
        Commands::Config(args) => {
            run_config(&config, &args);
            Ok(())
        }
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

    // TODO: Implement actual recording using media module
    // This is a placeholder for the basic CLI implementation
}

fn run_report(_config: &CliConfig, args: &probar_cli::ReportArgs) {
    println!("Generating report...");
    println!("Format: {:?}", args.format);
    println!("Output: {}", args.output.display());

    // TODO: Implement actual report generation
    // This is a placeholder for the basic CLI implementation

    if args.open {
        println!("Opening report in browser...");
    }
}

fn run_init(args: &probar_cli::InitArgs) {
    println!("Initializing Probar project in: {}", args.path.display());

    if args.force {
        println!("Force mode enabled - overwriting existing files");
    }

    // TODO: Implement actual project initialization
    // This is a placeholder for the basic CLI implementation
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
        println!("Setting: {setting}");
        // TODO: Implement config setting
    }

    if args.reset {
        println!("Resetting to default configuration");
        // TODO: Implement config reset
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
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
    fn test_build_config_quiet() {
        let cli = Cli::parse_from(["probar", "-q", "test"]);
        let config = build_config(&cli);
        assert_eq!(config.verbosity, Verbosity::Quiet);
    }
}
