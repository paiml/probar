//! Concurrent load testing engine for LLM inference endpoints.
//!
//! Generates concurrent chat completion requests, collects timing metrics,
//! and produces percentile-based latency reports.

use super::client::{ChatMessage, ChatRequest, LlmClient, LlmClientError, Role};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Configuration for a load test run.
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Number of concurrent workers.
    pub concurrency: usize,
    /// Total duration of the load test.
    pub duration: Duration,
    /// Prompts to cycle through.
    pub prompts: Vec<ChatRequest>,
    /// Name of the runtime being tested (for reporting).
    pub runtime_name: String,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 1,
            duration: Duration::from_secs(30),
            prompts: vec![default_prompt()],
            runtime_name: "unknown".to_string(),
        }
    }
}

/// Results from a load test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    /// Total requests sent.
    pub total_requests: u64,
    /// Successful completions.
    pub successful: u64,
    /// Failed requests.
    pub failed: u64,
    /// Requests per second (throughput).
    pub throughput_rps: f64,
    /// Median latency (ms).
    pub latency_p50_ms: f64,
    /// 95th percentile latency (ms).
    pub latency_p95_ms: f64,
    /// 99th percentile latency (ms).
    pub latency_p99_ms: f64,
    /// Median time to first byte (ms).
    pub ttft_p50_ms: f64,
    /// Average tokens per second across all successful requests.
    pub tokens_per_sec: f64,
    /// ISO 8601 timestamp of the run.
    pub timestamp: String,
    /// Name of the runtime tested.
    pub runtime_name: String,
    /// Total elapsed wall time (seconds).
    pub elapsed_secs: f64,
    /// Concurrency level used.
    pub concurrency: usize,
}

/// Load test executor.
#[derive(Debug)]
pub struct LoadTest {
    client: LlmClient,
    config: LoadTestConfig,
}

/// Individual request timing record.
#[derive(Debug, Clone)]
struct RequestRecord {
    latency: Duration,
    ttfb: Duration,
    tokens: u32,
    success: bool,
}

impl LoadTest {
    /// Create a new load test.
    pub fn new(client: LlmClient, config: LoadTestConfig) -> Self {
        Self { client, config }
    }

    /// Run the load test and return aggregated results.
    pub async fn run(&self) -> Result<LoadTestResult, LlmClientError> {
        let deadline = Instant::now() + self.config.duration;
        let mut handles = Vec::new();

        for worker_id in 0..self.config.concurrency {
            let client = self.client.clone();
            let prompts = self.config.prompts.clone();

            handles.push(tokio::spawn(async move {
                let mut records = Vec::new();
                let mut prompt_idx = worker_id % prompts.len().max(1);

                while Instant::now() < deadline {
                    let prompt = &prompts[prompt_idx % prompts.len()];
                    match client.send(prompt).await {
                        Ok(timed) => {
                            let tokens = timed
                                .response
                                .usage
                                .as_ref()
                                .map_or(0, |u| u.completion_tokens);
                            records.push(RequestRecord {
                                latency: timed.latency,
                                ttfb: timed.ttfb,
                                tokens,
                                success: true,
                            });
                        }
                        Err(_) => {
                            records.push(RequestRecord {
                                latency: Duration::from_millis(0),
                                ttfb: Duration::from_millis(0),
                                tokens: 0,
                                success: false,
                            });
                        }
                    }
                    prompt_idx += 1;
                }
                records
            }));
        }

        // Collect all records
        let mut all_records = Vec::new();
        for handle in handles {
            if let Ok(records) = handle.await {
                all_records.extend(records);
            }
        }

        let elapsed = self.config.duration.as_secs_f64();
        Ok(aggregate_results(
            &all_records,
            elapsed,
            &self.config.runtime_name,
            self.config.concurrency,
        ))
    }
}

/// Aggregate individual request records into summary statistics.
fn aggregate_results(
    records: &[RequestRecord],
    elapsed_secs: f64,
    runtime_name: &str,
    concurrency: usize,
) -> LoadTestResult {
    let total = records.len() as u64;
    let successful = records.iter().filter(|r| r.success).count() as u64;
    let failed = total - successful;

    let mut latencies: Vec<f64> = records
        .iter()
        .filter(|r| r.success)
        .map(|r| r.latency.as_secs_f64() * 1000.0)
        .collect();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut ttfbs: Vec<f64> = records
        .iter()
        .filter(|r| r.success)
        .map(|r| r.ttfb.as_secs_f64() * 1000.0)
        .collect();
    ttfbs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let total_tokens: u64 = records.iter().filter(|r| r.success).map(|r| u64::from(r.tokens)).sum();

    let throughput_rps = if elapsed_secs > 0.0 {
        successful as f64 / elapsed_secs
    } else {
        0.0
    };

    let tokens_per_sec = if elapsed_secs > 0.0 {
        total_tokens as f64 / elapsed_secs
    } else {
        0.0
    };

    let now = chrono::Utc::now().to_rfc3339();

    LoadTestResult {
        total_requests: total,
        successful,
        failed,
        throughput_rps,
        latency_p50_ms: percentile(&latencies, 0.50),
        latency_p95_ms: percentile(&latencies, 0.95),
        latency_p99_ms: percentile(&latencies, 0.99),
        ttft_p50_ms: percentile(&ttfbs, 0.50),
        tokens_per_sec,
        timestamp: now,
        runtime_name: runtime_name.to_string(),
        elapsed_secs,
        concurrency,
    }
}

/// Compute a percentile from a sorted slice. Returns 0.0 for empty slices.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Default prompt for load testing.
fn default_prompt() -> ChatRequest {
    ChatRequest {
        model: String::new(),
        messages: vec![ChatMessage {
            role: Role::User,
            content: "What is 2 + 2? Reply with just the number.".to_string(),
        }],
        temperature: Some(0.0),
        max_tokens: Some(16),
        stream: Some(false),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_empty() {
        assert_eq!(percentile(&[], 0.5), 0.0);
    }

    #[test]
    fn test_percentile_single() {
        assert_eq!(percentile(&[42.0], 0.5), 42.0);
        assert_eq!(percentile(&[42.0], 0.99), 42.0);
    }

    #[test]
    fn test_percentile_multiple() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        // p50 of [1..100]: index = round(99 * 0.50) = 50 → value 51
        assert_eq!(percentile(&data, 0.50), 51.0);
        assert_eq!(percentile(&data, 0.95), 95.0);
        assert_eq!(percentile(&data, 0.99), 99.0);
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate_results(&[], 10.0, "test", 1);
        assert_eq!(result.total_requests, 0);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.throughput_rps, 0.0);
        assert_eq!(result.latency_p50_ms, 0.0);
    }

    #[test]
    fn test_aggregate_all_success() {
        let records: Vec<RequestRecord> = (0..10)
            .map(|i| RequestRecord {
                latency: Duration::from_millis(100 + i * 10),
                ttfb: Duration::from_millis(50 + i * 5),
                tokens: 20,
                success: true,
            })
            .collect();
        let result = aggregate_results(&records, 10.0, "realizar", 2);
        assert_eq!(result.total_requests, 10);
        assert_eq!(result.successful, 10);
        assert_eq!(result.failed, 0);
        assert!((result.throughput_rps - 1.0).abs() < f64::EPSILON);
        assert!(result.latency_p50_ms > 0.0);
        assert!(result.tokens_per_sec > 0.0);
        assert_eq!(result.runtime_name, "realizar");
        assert_eq!(result.concurrency, 2);
    }

    #[test]
    fn test_aggregate_mixed() {
        let records = vec![
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 10,
                success: true,
            },
            RequestRecord {
                latency: Duration::from_millis(0),
                ttfb: Duration::from_millis(0),
                tokens: 0,
                success: false,
            },
        ];
        let result = aggregate_results(&records, 5.0, "ollama", 1);
        assert_eq!(result.total_requests, 2);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 1);
    }

    #[test]
    fn test_default_config() {
        let config = LoadTestConfig::default();
        assert_eq!(config.concurrency, 1);
        assert_eq!(config.duration, Duration::from_secs(30));
        assert_eq!(config.prompts.len(), 1);
    }

    #[test]
    fn test_default_prompt() {
        let p = default_prompt();
        assert_eq!(p.messages.len(), 1);
        assert_eq!(p.messages[0].role, Role::User);
        assert_eq!(p.temperature, Some(0.0));
    }

    #[test]
    fn test_load_test_result_serialization() {
        let result = LoadTestResult {
            total_requests: 100,
            successful: 95,
            failed: 5,
            throughput_rps: 10.0,
            latency_p50_ms: 150.0,
            latency_p95_ms: 300.0,
            latency_p99_ms: 500.0,
            ttft_p50_ms: 80.0,
            tokens_per_sec: 200.0,
            timestamp: "2026-03-01T00:00:00Z".to_string(),
            runtime_name: "realizar".to_string(),
            elapsed_secs: 10.0,
            concurrency: 4,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: LoadTestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_requests, 100);
        assert_eq!(back.runtime_name, "realizar");
    }

    #[test]
    fn test_percentile_boundary() {
        let data = vec![1.0, 2.0, 3.0];
        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 1.0), 3.0);
    }

    #[test]
    fn test_aggregate_zero_elapsed() {
        let records = vec![RequestRecord {
            latency: Duration::from_millis(100),
            ttfb: Duration::from_millis(50),
            tokens: 10,
            success: true,
        }];
        let result = aggregate_results(&records, 0.0, "test", 1);
        assert_eq!(result.throughput_rps, 0.0);
        assert_eq!(result.tokens_per_sec, 0.0);
    }
}
