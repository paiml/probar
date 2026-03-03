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
    /// Duration of warmup phase (excluded from metrics). Default: zero (no warmup).
    pub warmup_duration: Duration,
    /// Use SSE streaming for per-token TPOT measurement. Default: false.
    pub stream: bool,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 1,
            duration: Duration::from_secs(30),
            prompts: vec![default_prompt()],
            runtime_name: "unknown".to_string(),
            warmup_duration: Duration::ZERO,
            stream: false,
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
    /// Total tokens per second (sum of all completion tokens / wall time).
    /// NOTE: Not comparable across backends with different response lengths.
    pub tokens_per_sec: f64,
    /// Average completion tokens per response (GH-23).
    #[serde(default)]
    pub avg_tok_per_req: f64,
    /// Median inter-token latency in ms: (latency - ttft) / (tokens - 1) (GH-23).
    /// Comparable across backends regardless of response length.
    #[serde(default)]
    pub itl_p50_ms: f64,
    /// Decode throughput: 1000 / itl_p50_ms. True generation speed (GH-23).
    #[serde(default)]
    pub decode_tok_per_sec: f64,
    /// ISO 8601 timestamp of the run.
    pub timestamp: String,
    /// Name of the runtime tested.
    pub runtime_name: String,
    /// Total elapsed wall time (seconds).
    pub elapsed_secs: f64,
    /// Concurrency level used.
    pub concurrency: usize,
    // --- Extended percentiles (Benchmarking v2.0) ---
    /// TTFT 90th percentile (ms).
    #[serde(default)]
    pub ttft_p90_ms: f64,
    /// TTFT 95th percentile (ms).
    #[serde(default)]
    pub ttft_p95_ms: f64,
    /// TTFT 99th percentile (ms).
    #[serde(default)]
    pub ttft_p99_ms: f64,
    /// Time per output token P50 (ms). TPOT = (latency - TTFB) / (tokens - 1).
    #[serde(default)]
    pub tpot_p50_ms: f64,
    /// TPOT 90th percentile (ms).
    #[serde(default)]
    pub tpot_p90_ms: f64,
    /// TPOT 95th percentile (ms).
    #[serde(default)]
    pub tpot_p95_ms: f64,
    /// TPOT 99th percentile (ms).
    #[serde(default)]
    pub tpot_p99_ms: f64,
    /// Minimum latency (ms).
    #[serde(default)]
    pub latency_min_ms: f64,
    /// Maximum latency (ms).
    #[serde(default)]
    pub latency_max_ms: f64,
    /// Latency standard deviation (ms).
    #[serde(default)]
    pub latency_stddev_ms: f64,
    /// Error rate: failed / total.
    #[serde(default)]
    pub error_rate: f64,
    /// Total prompt (input) tokens across all requests.
    #[serde(default)]
    pub prompt_tokens_total: u64,
    /// Total completion (output) tokens across all requests.
    #[serde(default)]
    pub completion_tokens_total: u64,
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
    prompt_tokens: u32,
    success: bool,
    /// Per-token timestamps from SSE streaming (empty if non-streaming).
    token_timestamps: Vec<Duration>,
}

impl LoadTest {
    /// Create a new load test.
    pub fn new(client: LlmClient, config: LoadTestConfig) -> Self {
        Self { client, config }
    }

    /// Run the load test and return aggregated results.
    pub async fn run(&self) -> Result<LoadTestResult, LlmClientError> {
        // Warmup phase: send requests but discard results
        if self.config.warmup_duration > Duration::ZERO {
            self.run_phase(self.config.warmup_duration).await?;
        }

        // Measurement phase: use actual wall time
        let measure_start = Instant::now();
        let all_records = self.run_phase(self.config.duration).await?;
        let elapsed = measure_start.elapsed().as_secs_f64();

        Ok(aggregate_results(
            &all_records,
            elapsed,
            &self.config.runtime_name,
            self.config.concurrency,
        ))
    }

    /// Run a single phase (warmup or measurement) for the given duration.
    async fn run_phase(&self, duration: Duration) -> Result<Vec<RequestRecord>, LlmClientError> {
        let deadline = Instant::now() + duration;
        let mut handles = Vec::new();
        let use_stream = self.config.stream;

        for worker_id in 0..self.config.concurrency {
            let client = self.client.clone();
            let prompts = self.config.prompts.clone();

            handles.push(tokio::spawn(async move {
                let mut records = Vec::new();
                let mut prompt_idx = worker_id % prompts.len().max(1);

                while Instant::now() < deadline {
                    let prompt = &prompts[prompt_idx % prompts.len()];
                    if use_stream {
                        match client.chat_completion_stream(prompt).await {
                            Ok(streamed) => {
                                let token_count = streamed.token_timestamps.len() as u32;
                                let usage_tokens = streamed.usage.as_ref().map_or(token_count, |u| u.completion_tokens);
                                let prompt_tokens = streamed.usage.as_ref().map_or(0, |u| u.prompt_tokens);
                                records.push(RequestRecord {
                                    latency: streamed.latency,
                                    ttfb: streamed.ttft,
                                    tokens: usage_tokens,
                                    prompt_tokens,
                                    success: true,
                                    token_timestamps: streamed.token_timestamps,
                                });
                            }
                            Err(_) => {
                                records.push(RequestRecord {
                                    latency: Duration::from_millis(0),
                                    ttfb: Duration::from_millis(0),
                                    tokens: 0,
                                    prompt_tokens: 0,
                                    success: false,
                                    token_timestamps: Vec::new(),
                                });
                            }
                        }
                    } else {
                        match client.send(prompt).await {
                            Ok(timed) => {
                                let usage = timed.response.usage.as_ref();
                                let tokens = usage.map_or(0, |u| u.completion_tokens);
                                let prompt_tokens = usage.map_or(0, |u| u.prompt_tokens);
                                records.push(RequestRecord {
                                    latency: timed.latency,
                                    ttfb: timed.ttfb,
                                    tokens,
                                    prompt_tokens,
                                    success: true,
                                    token_timestamps: Vec::new(),
                                });
                            }
                            Err(_) => {
                                records.push(RequestRecord {
                                    latency: Duration::from_millis(0),
                                    ttfb: Duration::from_millis(0),
                                    tokens: 0,
                                    prompt_tokens: 0,
                                    success: false,
                                    token_timestamps: Vec::new(),
                                });
                            }
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

        Ok(all_records)
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
    let total_prompt_tokens: u64 = records
        .iter()
        .filter(|r| r.success)
        .map(|r| u64::from(r.prompt_tokens))
        .sum();

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

    // GH-23: Normalized metrics for cross-backend comparison
    let avg_tok_per_req = if successful > 0 {
        total_tokens as f64 / successful as f64
    } else {
        0.0
    };

    // Inter-token latency and decode throughput.
    // GH-24: When streaming token_timestamps are available, compute real per-token ITL.
    // Otherwise fall back to request-level approximation.
    let multi_token_records: Vec<&RequestRecord> = records
        .iter()
        .filter(|r| r.success && r.tokens >= 2)
        .collect();

    // Check if we have real streaming timestamps
    let has_streaming_timestamps = multi_token_records
        .iter()
        .any(|r| r.token_timestamps.len() >= 2);

    let is_streaming = has_streaming_timestamps
        || multi_token_records.iter().any(|r| {
            let ratio = r.ttfb.as_secs_f64() / r.latency.as_secs_f64().max(1e-9);
            ratio < 0.95
        });

    let mut itls: Vec<f64> = if has_streaming_timestamps {
        // GH-24: Real per-token ITL from SSE timestamps
        multi_token_records
            .iter()
            .filter(|r| r.token_timestamps.len() >= 2)
            .flat_map(|r| {
                r.token_timestamps
                    .windows(2)
                    .map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0)
                    .collect::<Vec<_>>()
            })
            .collect()
    } else if is_streaming {
        // Streaming without timestamps: ITL from decode phase
        multi_token_records
            .iter()
            .map(|r| {
                let decode_ms =
                    (r.latency.as_secs_f64() - r.ttfb.as_secs_f64()) * 1000.0;
                decode_ms / (r.tokens as f64 - 1.0)
            })
            .collect()
    } else {
        // Non-streaming: per-request throughput as ITL proxy
        multi_token_records
            .iter()
            .map(|r| r.latency.as_secs_f64() * 1000.0 / r.tokens as f64)
            .collect()
    };
    itls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let itl_p50_ms = percentile(&itls, 0.50);
    let decode_tok_per_sec = if itl_p50_ms > 0.0 {
        1000.0 / itl_p50_ms
    } else {
        0.0
    };

    // TPOT: Time per output token
    // GH-24: When streaming timestamps available, use mean of per-token deltas per request.
    // Otherwise: (latency - TTFB) / (tokens - 1).
    let mut tpots: Vec<f64> = if has_streaming_timestamps {
        multi_token_records
            .iter()
            .filter(|r| r.token_timestamps.len() >= 2)
            .map(|r| {
                let deltas: Vec<f64> = r
                    .token_timestamps
                    .windows(2)
                    .map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0)
                    .collect();
                deltas.iter().sum::<f64>() / deltas.len() as f64
            })
            .collect()
    } else {
        multi_token_records
            .iter()
            .map(|r| {
                let decode_ms =
                    (r.latency.as_secs_f64() - r.ttfb.as_secs_f64()) * 1000.0;
                decode_ms / (r.tokens as f64 - 1.0)
            })
            .collect()
    };
    tpots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Latency statistics
    let latency_min_ms = latencies.first().copied().unwrap_or(0.0);
    let latency_max_ms = latencies.last().copied().unwrap_or(0.0);
    let latency_stddev_ms = stddev(&latencies);

    let error_rate = if total > 0 {
        failed as f64 / total as f64
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
        avg_tok_per_req,
        itl_p50_ms,
        decode_tok_per_sec,
        timestamp: now,
        runtime_name: runtime_name.to_string(),
        elapsed_secs,
        concurrency,
        // Extended percentiles
        ttft_p90_ms: percentile(&ttfbs, 0.90),
        ttft_p95_ms: percentile(&ttfbs, 0.95),
        ttft_p99_ms: percentile(&ttfbs, 0.99),
        tpot_p50_ms: percentile(&tpots, 0.50),
        tpot_p90_ms: percentile(&tpots, 0.90),
        tpot_p95_ms: percentile(&tpots, 0.95),
        tpot_p99_ms: percentile(&tpots, 0.99),
        latency_min_ms,
        latency_max_ms,
        latency_stddev_ms,
        error_rate,
        prompt_tokens_total: total_prompt_tokens,
        completion_tokens_total: total_tokens,
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

/// Compute standard deviation of a slice of f64 values.
fn stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt()
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
        assert_eq!(result.error_rate, 0.0);
        assert_eq!(result.prompt_tokens_total, 0);
        assert_eq!(result.completion_tokens_total, 0);
    }

    #[test]
    fn test_aggregate_all_success() {
        let records: Vec<RequestRecord> = (0..10)
            .map(|i| RequestRecord {
                latency: Duration::from_millis(100 + i * 10),
                ttfb: Duration::from_millis(50 + i * 5),
                tokens: 20,
                prompt_tokens: 10,
                success: true,
                token_timestamps: Vec::new(),
            })
            .collect();
        let result = aggregate_results(&records, 10.0, "realizar", 2);
        assert_eq!(result.total_requests, 10);
        assert_eq!(result.successful, 10);
        assert_eq!(result.failed, 0);
        assert!((result.throughput_rps - 1.0).abs() < f64::EPSILON);
        assert!(result.latency_p50_ms > 0.0);
        assert!(result.tokens_per_sec > 0.0);
        // GH-23: normalized metrics
        assert!((result.avg_tok_per_req - 20.0).abs() < f64::EPSILON);
        assert!(result.itl_p50_ms > 0.0);
        assert!(result.decode_tok_per_sec > 0.0);
        assert_eq!(result.runtime_name, "realizar");
        assert_eq!(result.concurrency, 2);
        // Extended percentiles
        assert!(result.ttft_p90_ms > 0.0);
        assert!(result.ttft_p95_ms > 0.0);
        assert!(result.ttft_p99_ms > 0.0);
        assert!(result.tpot_p50_ms > 0.0);
        assert!(result.latency_min_ms > 0.0);
        assert!(result.latency_max_ms >= result.latency_min_ms);
        assert!(result.latency_stddev_ms >= 0.0);
        assert!((result.error_rate).abs() < f64::EPSILON);
        assert_eq!(result.prompt_tokens_total, 100);
        assert_eq!(result.completion_tokens_total, 200);
    }

    #[test]
    fn test_aggregate_mixed() {
        let records = vec![
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 10,
                prompt_tokens: 5,
                success: true,
                token_timestamps: Vec::new(),
            },
            RequestRecord {
                latency: Duration::from_millis(0),
                ttfb: Duration::from_millis(0),
                tokens: 0,
                prompt_tokens: 0,
                success: false,
                token_timestamps: Vec::new(),
            },
        ];
        let result = aggregate_results(&records, 5.0, "ollama", 1);
        assert_eq!(result.total_requests, 2);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 1);
        assert!((result.error_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_config() {
        let config = LoadTestConfig::default();
        assert_eq!(config.concurrency, 1);
        assert_eq!(config.duration, Duration::from_secs(30));
        assert_eq!(config.prompts.len(), 1);
        assert_eq!(config.warmup_duration, Duration::ZERO);
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
            avg_tok_per_req: 15.0,
            itl_p50_ms: 5.0,
            decode_tok_per_sec: 200.0,
            timestamp: "2026-03-01T00:00:00Z".to_string(),
            runtime_name: "realizar".to_string(),
            elapsed_secs: 10.0,
            concurrency: 4,
            ttft_p90_ms: 90.0,
            ttft_p95_ms: 95.0,
            ttft_p99_ms: 99.0,
            tpot_p50_ms: 6.0,
            tpot_p90_ms: 8.0,
            tpot_p95_ms: 9.0,
            tpot_p99_ms: 12.0,
            latency_min_ms: 50.0,
            latency_max_ms: 800.0,
            latency_stddev_ms: 120.0,
            error_rate: 0.05,
            prompt_tokens_total: 950,
            completion_tokens_total: 1425,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: LoadTestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_requests, 100);
        assert_eq!(back.runtime_name, "realizar");
        assert!((back.avg_tok_per_req - 15.0).abs() < f64::EPSILON);
        assert!((back.itl_p50_ms - 5.0).abs() < f64::EPSILON);
        assert!((back.decode_tok_per_sec - 200.0).abs() < f64::EPSILON);
        assert!((back.tpot_p50_ms - 6.0).abs() < f64::EPSILON);
        assert!((back.error_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(back.prompt_tokens_total, 950);
        assert_eq!(back.completion_tokens_total, 1425);
    }

    #[test]
    fn test_load_test_result_backwards_compat() {
        // Old JSON without new fields should deserialize with defaults
        let json = r#"{
            "total_requests": 50,
            "successful": 50,
            "failed": 0,
            "throughput_rps": 5.0,
            "latency_p50_ms": 100.0,
            "latency_p95_ms": 200.0,
            "latency_p99_ms": 300.0,
            "ttft_p50_ms": 50.0,
            "tokens_per_sec": 100.0,
            "timestamp": "2026-01-01T00:00:00Z",
            "runtime_name": "old",
            "elapsed_secs": 10.0,
            "concurrency": 1
        }"#;
        let result: LoadTestResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.total_requests, 50);
        assert_eq!(result.tpot_p50_ms, 0.0);
        assert_eq!(result.error_rate, 0.0);
        assert_eq!(result.prompt_tokens_total, 0);
    }

    #[test]
    fn test_percentile_boundary() {
        let data = vec![1.0, 2.0, 3.0];
        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 1.0), 3.0);
    }

    #[test]
    fn test_itl_streaming() {
        // GH-23: Streaming mode — ITL = (latency - ttfb) / (tokens - 1)
        // Request: 200ms latency, 50ms ttfb, 16 tokens
        // ttfb/latency = 0.25 < 0.95 → streaming detected
        // Decode time = 200 - 50 = 150ms, ITL = 150 / 15 = 10ms
        let records = vec![RequestRecord {
            latency: Duration::from_millis(200),
            ttfb: Duration::from_millis(50),
            tokens: 16,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert!((result.itl_p50_ms - 10.0).abs() < 0.1);
        assert!((result.decode_tok_per_sec - 100.0).abs() < 1.0);
        assert!((result.avg_tok_per_req - 16.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_itl_non_streaming() {
        // GH-23: Non-streaming — ttfb ≈ latency, fallback to latency/tokens
        // Request: 1600ms latency, 1599ms ttfb, 16 tokens
        // ttfb/latency = 0.999 > 0.95 → non-streaming detected
        // ITL proxy = 1600 / 16 = 100ms
        let records = vec![RequestRecord {
            latency: Duration::from_millis(1600),
            ttfb: Duration::from_millis(1599),
            tokens: 16,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert!((result.itl_p50_ms - 100.0).abs() < 0.1);
        assert!((result.decode_tok_per_sec - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_itl_single_token_excluded() {
        // GH-23: Requests with < 2 tokens should be excluded from ITL
        // (can't compute inter-token latency with 0 or 1 token)
        let records = vec![RequestRecord {
            latency: Duration::from_millis(100),
            ttfb: Duration::from_millis(100),
            tokens: 1,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert_eq!(result.itl_p50_ms, 0.0);
        assert_eq!(result.decode_tok_per_sec, 0.0);
        assert!((result.avg_tok_per_req - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregate_zero_elapsed() {
        let records = vec![RequestRecord {
            latency: Duration::from_millis(100),
            ttfb: Duration::from_millis(50),
            tokens: 10,
            prompt_tokens: 5,
            success: true,
            token_timestamps: Vec::new(),
        }];
        let result = aggregate_results(&records, 0.0, "test", 1);
        assert_eq!(result.throughput_rps, 0.0);
        assert_eq!(result.tokens_per_sec, 0.0);
    }

    #[test]
    fn test_stddev() {
        assert_eq!(stddev(&[]), 0.0);
        assert_eq!(stddev(&[5.0]), 0.0);
        // [10, 20, 30]: mean=20, var=((100+0+100)/2)=100, stddev=10
        let sd = stddev(&[10.0, 20.0, 30.0]);
        assert!((sd - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_tpot_computation() {
        // TPOT = (latency - ttfb) / (tokens - 1)
        // Streaming: 200ms latency, 50ms ttfb, 16 tokens
        // TPOT = (200 - 50) / 15 = 10ms
        let records = vec![RequestRecord {
            latency: Duration::from_millis(200),
            ttfb: Duration::from_millis(50),
            tokens: 16,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert!((result.tpot_p50_ms - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_latency_min_max_stddev() {
        let records = vec![
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 10,
                prompt_tokens: 5,
                success: true,
                token_timestamps: Vec::new(),
            },
            RequestRecord {
                latency: Duration::from_millis(300),
                ttfb: Duration::from_millis(100),
                tokens: 10,
                prompt_tokens: 5,
                success: true,
                token_timestamps: Vec::new(),
            },
        ];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert!((result.latency_min_ms - 100.0).abs() < 0.1);
        assert!((result.latency_max_ms - 300.0).abs() < 0.1);
        assert!(result.latency_stddev_ms > 0.0);
    }

    #[test]
    fn test_prompt_tokens_tracking() {
        let records = vec![
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 10,
                prompt_tokens: 20,
                success: true,
                token_timestamps: Vec::new(),
            },
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 15,
                prompt_tokens: 25,
                success: true,
                token_timestamps: Vec::new(),
            },
        ];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert_eq!(result.prompt_tokens_total, 45);
        assert_eq!(result.completion_tokens_total, 25);
    }

    #[test]
    fn test_tpot_from_streaming_timestamps() {
        // GH-24: When token_timestamps are available, TPOT uses real per-token deltas.
        // 5 tokens arriving at 50ms, 60ms, 70ms, 80ms, 90ms
        // Inter-token deltas: 10ms, 10ms, 10ms, 10ms → mean TPOT = 10ms
        let records = vec![RequestRecord {
            latency: Duration::from_millis(100),
            ttfb: Duration::from_millis(50),
            tokens: 5,
            prompt_tokens: 10,
            success: true,
            token_timestamps: vec![
                Duration::from_millis(50),
                Duration::from_millis(60),
                Duration::from_millis(70),
                Duration::from_millis(80),
                Duration::from_millis(90),
            ],
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        // Real TPOT from timestamps: mean of [10, 10, 10, 10] = 10ms
        assert!((result.tpot_p50_ms - 10.0).abs() < 0.1);
        // ITL also uses real timestamps
        assert!((result.itl_p50_ms - 10.0).abs() < 0.1);
        assert!((result.decode_tok_per_sec - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_tpot_mixed_streaming_and_non_streaming() {
        // GH-24: When some records have timestamps and some don't,
        // only records with timestamps >= 2 are used for streaming TPOT.
        let records = vec![
            RequestRecord {
                latency: Duration::from_millis(200),
                ttfb: Duration::from_millis(50),
                tokens: 4,
                prompt_tokens: 10,
                success: true,
                token_timestamps: vec![
                    Duration::from_millis(50),
                    Duration::from_millis(70),
                    Duration::from_millis(90),
                    Duration::from_millis(110),
                ],
            },
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 5,
                prompt_tokens: 10,
                success: true,
                token_timestamps: Vec::new(), // non-streaming request
            },
        ];
        let result = aggregate_results(&records, 1.0, "test", 1);
        // Only the first record with timestamps is used for TPOT
        // Deltas: [20, 20, 20] → mean TPOT = 20ms
        assert!((result.tpot_p50_ms - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_stream_config_default() {
        let config = LoadTestConfig::default();
        assert!(!config.stream);
    }
}
