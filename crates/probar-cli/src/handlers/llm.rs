//! Handler for `probador llm` subcommands.
//!
//! Implements correctness testing, load testing, and report generation
//! for OpenAI-compatible LLM inference endpoints.

use crate::error::{CliError, CliResult};
use crate::LlmLoadArgs;
use crate::LlmReportArgs;
use crate::LlmTestArgs;
use std::path::Path;
use std::time::Duration;

/// Test case loaded from YAML config.
#[derive(Debug, serde::Deserialize)]
pub struct TestConfig {
    /// List of test cases.
    pub tests: Vec<TestCase>,
}

/// A single correctness test case.
#[derive(Debug, serde::Deserialize)]
pub struct TestCase {
    /// Name of the test.
    pub name: String,
    /// Chat messages to send.
    pub messages: Vec<MessageConfig>,
    /// Assert output contains this substring.
    #[serde(default)]
    pub expect_contains: Option<String>,
    /// Assert output matches this regex pattern.
    #[serde(default)]
    pub expect_pattern: Option<String>,
    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
}

/// A message in the YAML config.
#[derive(Debug, serde::Deserialize)]
pub struct MessageConfig {
    /// Role: system, user, or assistant.
    pub role: String,
    /// Content of the message.
    pub content: String,
}

/// Result of a single correctness test.
#[derive(Debug, serde::Serialize)]
pub struct CorrectnessResult {
    /// Test name.
    pub name: String,
    /// Whether all assertions passed.
    pub passed: bool,
    /// Latency in milliseconds.
    pub latency_ms: f64,
    /// The model's output text.
    pub output: String,
    /// Details of any failures.
    pub failures: Vec<String>,
}

/// Aggregated correctness test results.
#[derive(Debug, serde::Serialize)]
pub struct CorrectnessReport {
    /// Runtime name.
    pub runtime_name: String,
    /// Timestamp.
    pub timestamp: String,
    /// Individual test results.
    pub results: Vec<CorrectnessResult>,
    /// Count of passed tests.
    pub passed: usize,
    /// Count of failed tests.
    pub failed: usize,
    /// Total tests run.
    pub total: usize,
}

/// Execute `probador llm test`.
pub async fn execute_llm_test(args: &LlmTestArgs) -> CliResult<()> {
    let config_str =
        std::fs::read_to_string(&args.config).map_err(|e| CliError::Generic(e.to_string()))?;
    let config: TestConfig =
        serde_yaml_ng::from_str(&config_str).map_err(|e| CliError::Generic(e.to_string()))?;

    let client = jugar_probar::llm::LlmClient::new(&args.url, &args.model);

    // Health check
    match client.health_check().await {
        Ok(true) => println!("Health check passed: {}", args.url),
        Ok(false) | Err(_) => {
            eprintln!("Warning: health check failed for {}, proceeding anyway", args.url);
        }
    }

    let mut results = Vec::new();

    for test in &config.tests {
        print!("  {} ... ", test.name);
        let messages: Vec<jugar_probar::llm::ChatMessage> = test
            .messages
            .iter()
            .map(|m| jugar_probar::llm::ChatMessage {
                role: parse_role(&m.role),
                content: m.content.clone(),
            })
            .collect();

        match client
            .chat_completion(messages, test.temperature, test.max_tokens)
            .await
        {
            Ok(timed) => {
                let mut assertion = jugar_probar::llm::LlmAssertion::new().assert_response_valid();

                if let Some(ref s) = test.expect_contains {
                    assertion = assertion.assert_contains(s);
                }
                if let Some(ref p) = test.expect_pattern {
                    assertion = assertion.assert_matches_pattern(p);
                }

                let check_results = assertion.run(&timed);
                let failures: Vec<String> = check_results
                    .iter()
                    .filter(|r| !r.passed)
                    .filter_map(|r| r.detail.clone())
                    .collect();

                let passed = failures.is_empty();
                let output = timed
                    .response
                    .choices
                    .first()
                    .map_or_else(String::new, |c| c.message.content.clone());

                if passed {
                    println!("PASS ({:.0}ms)", timed.latency.as_secs_f64() * 1000.0);
                } else {
                    println!("FAIL");
                    for f in &failures {
                        eprintln!("    {f}");
                    }
                }

                results.push(CorrectnessResult {
                    name: test.name.clone(),
                    passed,
                    latency_ms: timed.latency.as_secs_f64() * 1000.0,
                    output,
                    failures,
                });
            }
            Err(e) => {
                println!("ERROR: {e}");
                results.push(CorrectnessResult {
                    name: test.name.clone(),
                    passed: false,
                    latency_ms: 0.0,
                    output: String::new(),
                    failures: vec![e.to_string()],
                });
            }
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let total = results.len();

    println!("\nResults: {passed}/{total} passed, {failed} failed");

    let report = CorrectnessReport {
        runtime_name: args.runtime_name.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        results,
        passed,
        failed,
        total,
    };

    if let Some(ref output_path) = args.output {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| CliError::Generic(e.to_string()))?;
        std::fs::write(output_path, json).map_err(|e| CliError::Generic(e.to_string()))?;
        println!("Results written to {}", output_path.display());
    }

    if failed > 0 {
        Err(CliError::Generic(format!("{failed} test(s) failed")))
    } else {
        Ok(())
    }
}

/// Execute `probador llm load`.
pub async fn execute_llm_load(args: &LlmLoadArgs) -> CliResult<()> {
    let duration = parse_duration(&args.duration)?;
    let client = jugar_probar::llm::LlmClient::new(&args.url, &args.model);

    println!(
        "Load testing {} (concurrency={}, duration={:.0}s, runtime={})",
        args.url,
        args.concurrency,
        duration.as_secs_f64(),
        args.runtime_name,
    );

    // Health check
    match client.health_check().await {
        Ok(true) => println!("Health check passed"),
        Ok(false) | Err(_) => {
            eprintln!("Warning: health check failed, proceeding anyway");
        }
    }

    let config = jugar_probar::llm::LoadTestConfig {
        concurrency: args.concurrency,
        duration,
        runtime_name: args.runtime_name.clone(),
        ..Default::default()
    };

    let load_test = jugar_probar::llm::LoadTest::new(client, config);
    let result = load_test
        .run()
        .await
        .map_err(|e| CliError::Generic(e.to_string()))?;

    // Print summary
    println!("\n--- Load Test Results ---");
    println!("Runtime:      {}", result.runtime_name);
    println!("Requests:     {} ({} ok, {} failed)", result.total_requests, result.successful, result.failed);
    println!("Throughput:   {:.1} req/s", result.throughput_rps);
    println!("Latency P50:  {:.1} ms", result.latency_p50_ms);
    println!("Latency P95:  {:.1} ms", result.latency_p95_ms);
    println!("Latency P99:  {:.1} ms", result.latency_p99_ms);
    println!("TTFT P50:     {:.1} ms", result.ttft_p50_ms);
    println!("Tokens/sec:   {:.1}", result.tokens_per_sec);

    if let Some(ref output_path) = args.output {
        let json = jugar_probar::llm::report::to_json(&result);
        std::fs::write(output_path, json).map_err(|e| CliError::Generic(e.to_string()))?;
        println!("\nResults written to {}", output_path.display());
    }

    Ok(())
}

/// Execute `probador llm report`.
pub fn execute_llm_report(args: &LlmReportArgs) -> CliResult<()> {
    let results = load_results_from_dir(&args.results)?;

    if results.is_empty() {
        println!("No result files found in {}", args.results.display());
        return Ok(());
    }

    println!("Loaded {} result files", results.len());

    jugar_probar::llm::report::update_performance_md(&args.output, &results)
        .map_err(|e| CliError::Generic(e.to_string()))?;
    println!("Updated {}", args.output.display());

    if let Some(ref readme_path) = args.update_readme {
        let table = jugar_probar::llm::report::to_markdown_table(&results);
        update_readme_section(readme_path, &table)?;
        println!("Updated {}", readme_path.display());
    }

    Ok(())
}

/// Load all JSON result files from a directory.
fn load_results_from_dir(
    dir: &Path,
) -> CliResult<Vec<jugar_probar::llm::LoadTestResult>> {
    let mut results = Vec::new();
    let entries =
        std::fs::read_dir(dir).map_err(|e| CliError::Generic(format!("Cannot read {}: {e}", dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| CliError::Generic(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains("load"))
        {
            let content =
                std::fs::read_to_string(&path).map_err(|e| CliError::Generic(e.to_string()))?;
            match serde_json::from_str::<jugar_probar::llm::LoadTestResult>(&content) {
                Ok(result) => results.push(result),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(results)
}

/// Update a README.md with a performance section.
fn update_readme_section(path: &Path, table: &str) -> CliResult<()> {
    let existing = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| CliError::Generic(e.to_string()))?
    } else {
        String::new()
    };

    let marker_start = "<!-- PERFORMANCE_START -->";
    let marker_end = "<!-- PERFORMANCE_END -->";

    let content = if existing.contains(marker_start) && existing.contains(marker_end) {
        // Replace existing section
        let before = existing.split(marker_start).next().unwrap_or("");
        let after = existing
            .split(marker_end)
            .nth(1)
            .unwrap_or("");
        format!("{before}{marker_start}\n{table}\n{marker_end}{after}")
    } else {
        // Append section
        let mut out = existing;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("\n{marker_start}\n{table}\n{marker_end}\n"));
        out
    };

    std::fs::write(path, content).map_err(|e| CliError::Generic(e.to_string()))
}

/// Parse a role string to the LLM Role enum.
fn parse_role(s: &str) -> jugar_probar::llm::Role {
    match s.to_lowercase().as_str() {
        "system" => jugar_probar::llm::Role::System,
        "assistant" => jugar_probar::llm::Role::Assistant,
        _ => jugar_probar::llm::Role::User,
    }
}

/// Parse a duration string like "30s", "2m", "1h".
fn parse_duration(s: &str) -> CliResult<Duration> {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        let n: u64 = secs
            .parse()
            .map_err(|_| CliError::Generic(format!("Invalid duration: {s}")))?;
        Ok(Duration::from_secs(n))
    } else if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins
            .parse()
            .map_err(|_| CliError::Generic(format!("Invalid duration: {s}")))?;
        Ok(Duration::from_secs(n * 60))
    } else if let Some(hrs) = s.strip_suffix('h') {
        let n: u64 = hrs
            .parse()
            .map_err(|_| CliError::Generic(format!("Invalid duration: {s}")))?;
        Ok(Duration::from_secs(n * 3600))
    } else {
        // Try parsing as raw seconds
        let n: u64 = s
            .parse()
            .map_err(|_| CliError::Generic(format!("Invalid duration: {s}. Use 30s, 2m, or 1h")))?;
        Ok(Duration::from_secs(n))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_duration_raw_number() {
        assert_eq!(parse_duration("60").unwrap(), Duration::from_secs(60));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_parse_role() {
        assert_eq!(parse_role("system"), jugar_probar::llm::Role::System);
        assert_eq!(parse_role("user"), jugar_probar::llm::Role::User);
        assert_eq!(parse_role("assistant"), jugar_probar::llm::Role::Assistant);
        assert_eq!(parse_role("SYSTEM"), jugar_probar::llm::Role::System);
        assert_eq!(parse_role("unknown"), jugar_probar::llm::Role::User);
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
tests:
  - name: basic_math
    messages:
      - role: user
        content: "What is 7 * 8?"
    expect_contains: "56"
    max_tokens: 32
    temperature: 0.0
  - name: pattern_test
    messages:
      - role: user
        content: "Hello"
    expect_pattern: "(?i)hello"
"#;
        let config: TestConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.tests.len(), 2);
        assert_eq!(config.tests[0].name, "basic_math");
        assert_eq!(config.tests[0].expect_contains, Some("56".to_string()));
        assert_eq!(config.tests[0].max_tokens, Some(32));
        assert!(config.tests[1].expect_pattern.is_some());
    }

    #[test]
    fn test_update_readme_section_new() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(&path, "# My Project\n\nSome content.\n").unwrap();

        update_readme_section(&path, "| table data |").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- PERFORMANCE_START -->"));
        assert!(content.contains("| table data |"));
        assert!(content.contains("<!-- PERFORMANCE_END -->"));
    }

    #[test]
    fn test_update_readme_section_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(
            &path,
            "# My Project\n<!-- PERFORMANCE_START -->\nold data\n<!-- PERFORMANCE_END -->\n",
        )
        .unwrap();

        update_readme_section(&path, "| new data |").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("| new data |"));
        assert!(!content.contains("old data"));
    }

    #[test]
    fn test_correctness_result_serialization() {
        let result = CorrectnessResult {
            name: "test1".to_string(),
            passed: true,
            latency_ms: 150.5,
            output: "Hello!".to_string(),
            failures: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"name\":\"test1\""));
        assert!(json.contains("\"passed\":true"));
    }
}
