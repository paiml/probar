//! Report generation for LLM test results.
//!
//! Produces JSON and Markdown output, and can update a historical
//! `performance.md` table with new results.

use super::loadtest::LoadTestResult;
use std::path::Path;

/// Serialize a load test result to a pretty-printed JSON string.
pub fn to_json(result: &LoadTestResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string())
}

/// Produce a single Markdown table row for a load test result.
pub fn to_markdown_row(result: &LoadTestResult) -> String {
    format!(
        "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {} |",
        result.timestamp.split('T').next().unwrap_or(&result.timestamp),
        result.runtime_name,
        result.concurrency,
        result.throughput_rps,
        result.latency_p50_ms,
        result.latency_p95_ms,
        result.latency_p99_ms,
        result.ttft_p50_ms,
        result.tokens_per_sec,
        result.total_requests,
    )
}

/// Header for the performance Markdown table.
const TABLE_HEADER: &str = "\
| Date | Runtime | Concurrency | RPS | P50 (ms) | P95 (ms) | P99 (ms) | TTFT P50 (ms) | Tok/s | Requests |
|------|---------|-------------|-----|----------|----------|----------|---------------|-------|----------|";

/// Generate a complete Markdown table from multiple results.
pub fn to_markdown_table(results: &[LoadTestResult]) -> String {
    let mut lines = vec![
        "## Performance Results".to_string(),
        String::new(),
        TABLE_HEADER.to_string(),
    ];
    for r in results {
        lines.push(to_markdown_row(r));
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Append new results to an existing `performance.md` file.
///
/// If the file doesn't exist, creates it with the table header.
/// If it exists, appends new rows after the existing table.
pub fn update_performance_md(
    path: &Path,
    results: &[LoadTestResult],
) -> Result<(), std::io::Error> {
    let existing = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };

    let new_rows: Vec<String> = results.iter().map(|r| to_markdown_row(r)).collect();

    let content = if existing.is_empty() {
        // Create fresh file
        let mut lines = vec![
            "# LLM Inference Performance".to_string(),
            String::new(),
            TABLE_HEADER.to_string(),
        ];
        lines.extend(new_rows);
        lines.push(String::new());
        lines.join("\n")
    } else if existing.contains(TABLE_HEADER.lines().next().unwrap_or("")) {
        // Append rows to existing table
        let trimmed = existing.trim_end();
        let mut out = trimmed.to_string();
        for row in &new_rows {
            out.push('\n');
            out.push_str(row);
        }
        out.push('\n');
        out
    } else {
        // File exists but no table — append a new table section
        let mut out = existing;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&to_markdown_table(results));
        out
    };

    std::fs::write(path, content)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_result(runtime: &str) -> LoadTestResult {
        LoadTestResult {
            total_requests: 100,
            successful: 95,
            failed: 5,
            throughput_rps: 10.5,
            latency_p50_ms: 150.3,
            latency_p95_ms: 300.7,
            latency_p99_ms: 500.2,
            ttft_p50_ms: 80.1,
            tokens_per_sec: 200.0,
            timestamp: "2026-03-01T04:00:00Z".to_string(),
            runtime_name: runtime.to_string(),
            elapsed_secs: 10.0,
            concurrency: 4,
        }
    }

    #[test]
    fn test_to_json() {
        let result = sample_result("realizar");
        let json = to_json(&result);
        assert!(json.contains("\"runtime_name\": \"realizar\""));
        assert!(json.contains("\"total_requests\": 100"));
    }

    #[test]
    fn test_to_markdown_row() {
        let result = sample_result("ollama");
        let row = to_markdown_row(&result);
        assert!(row.starts_with("| 2026-03-01"));
        assert!(row.contains("ollama"));
        assert!(row.contains("10.5"));
        assert!(row.contains("150.3"));
    }

    #[test]
    fn test_to_markdown_table() {
        let results = vec![sample_result("realizar"), sample_result("ollama")];
        let table = to_markdown_table(&results);
        assert!(table.contains("## Performance Results"));
        assert!(table.contains("| Date |"));
        assert!(table.contains("realizar"));
        assert!(table.contains("ollama"));
    }

    #[test]
    fn test_update_performance_md_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("performance.md");
        let results = vec![sample_result("realizar")];
        update_performance_md(&path, &results).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# LLM Inference Performance"));
        assert!(content.contains("realizar"));
    }

    #[test]
    fn test_update_performance_md_append() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("performance.md");

        // First write
        update_performance_md(&path, &[sample_result("realizar")]).unwrap();

        // Second write (append)
        update_performance_md(&path, &[sample_result("ollama")]).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("realizar"));
        assert!(content.contains("ollama"));
        // Should NOT have duplicate headers
        assert_eq!(
            content.matches("# LLM Inference Performance").count(),
            1
        );
    }

    #[test]
    fn test_update_performance_md_no_table_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("performance.md");
        std::fs::write(&path, "# Some other content\n\nHello world.\n").unwrap();

        update_performance_md(&path, &[sample_result("llamacpp")]).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Some other content"));
        assert!(content.contains("llamacpp"));
        assert!(content.contains("## Performance Results"));
    }

    #[test]
    fn test_to_json_roundtrip() {
        let result = sample_result("test");
        let json = to_json(&result);
        let back: LoadTestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.runtime_name, "test");
        assert_eq!(back.total_requests, 100);
    }

    #[test]
    fn test_markdown_row_date_extraction() {
        let mut result = sample_result("x");
        result.timestamp = "2026-12-25T12:00:00Z".to_string();
        let row = to_markdown_row(&result);
        assert!(row.contains("2026-12-25"));
        assert!(!row.contains("T12:00:00Z"));
    }
}
