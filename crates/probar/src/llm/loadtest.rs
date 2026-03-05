//! Concurrent load testing engine for LLM inference endpoints.
//!
//! Generates concurrent chat completion requests, collects timing metrics,
//! and produces percentile-based latency reports.

use super::client::{BrickTrace, ChatMessage, ChatRequest, LlmClient, LlmClientError, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Trace level for BrickProfiler data collection. Default: None.
    pub trace_level: Option<String>,
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
            trace_level: None,
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
    /// Prefill throughput: prompt_tokens / ttft_seconds. Measures prompt processing speed.
    #[serde(default)]
    pub prefill_tok_per_sec: f64,
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
    /// Aggregated BrickProfiler per-operation timing (GH-114).
    /// Present when --trace-level brick is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brick_trace_summary: Option<Vec<BrickTraceOpSummary>>,
    /// Per-request raw timing data for distribution analysis.
    /// Each entry: [latency_ms, ttft_ms, completion_tokens, itl_ms].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request_details: Vec<RequestDetail>,
}

/// Per-request timing for distribution analysis and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDetail {
    /// Total request latency (ms).
    pub latency_ms: f64,
    /// Time to first token (ms).
    pub ttft_ms: f64,
    /// Completion tokens generated.
    pub completion_tokens: u32,
    /// Prompt tokens (if reported by server).
    pub prompt_tokens: u32,
    /// Mean inter-token latency for this request (ms). 0 if < 2 tokens.
    pub itl_ms: f64,
}

/// Aggregated BrickProfiler operation timing across benchmark requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickTraceOpSummary {
    /// Operation name (e.g., "attention_qkv", "mlp_gate_up").
    pub name: String,
    /// Mean time in microseconds across all requests.
    pub mean_us: f64,
    /// Minimum time in microseconds.
    pub min_us: f64,
    /// Maximum time in microseconds.
    pub max_us: f64,
    /// Percentage of total inference time.
    pub pct_of_total: f64,
    /// Number of samples.
    pub samples: usize,
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
    /// Brick trace data from response (when trace_level was set).
    brick_trace: Option<BrickTrace>,
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
        let trace_level = self.config.trace_level.clone();

        for worker_id in 0..self.config.concurrency {
            let client = self.client.clone();
            let prompts = self.config.prompts.clone();
            let trace_level = trace_level.clone();

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
                                    brick_trace: None,
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
                                    brick_trace: None,
                                });
                            }
                        }
                    } else {
                        // GH-114: Use send_with_trace when trace_level is set
                        let result = if let Some(ref tl) = trace_level {
                            client.send_with_trace(prompt, tl).await
                        } else {
                            client.send(prompt).await
                        };
                        match result {
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
                                    brick_trace: timed.brick_trace,
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
                                    brick_trace: None,
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
        // Per-request mean ITL from SSE timestamps: (last_ts - first_ts) / (n - 1).
        // Robust to token batching (servers often send 2+ tokens per SSE chunk,
        // which would make flat_map of individual deltas bimodal: [0, 113, 0, 113...]).
        multi_token_records
            .iter()
            .filter(|r| r.token_timestamps.len() >= 2)
            .map(|r| {
                let first = r.token_timestamps.first().unwrap();
                let last = r.token_timestamps.last().unwrap();
                let decode_ms = (*last - *first).as_secs_f64() * 1000.0;
                let n_intervals = (r.token_timestamps.len() - 1) as f64;
                decode_ms / n_intervals
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
    // When streaming without timestamps: (latency - TTFB) / (tokens - 1).
    // Non-streaming: TTFB ≈ latency so (latency - TTFB) ≈ 0 — use latency/tokens as proxy.
    let mut tpots: Vec<f64> = if has_streaming_timestamps {
        // Per-request mean TPOT: (last_ts - first_ts) / (n - 1).
        // Same as ITL when using timestamps — robust to token batching.
        multi_token_records
            .iter()
            .filter(|r| r.token_timestamps.len() >= 2)
            .map(|r| {
                let first = r.token_timestamps.first().unwrap();
                let last = r.token_timestamps.last().unwrap();
                let decode_ms = (*last - *first).as_secs_f64() * 1000.0;
                let n_intervals = (r.token_timestamps.len() - 1) as f64;
                decode_ms / n_intervals
            })
            .collect()
    } else if is_streaming {
        multi_token_records
            .iter()
            .map(|r| {
                let decode_ms =
                    (r.latency.as_secs_f64() - r.ttfb.as_secs_f64()) * 1000.0;
                decode_ms / (r.tokens as f64 - 1.0)
            })
            .collect()
    } else {
        // Non-streaming: per-request throughput as TPOT proxy (same as ITL fallback)
        multi_token_records
            .iter()
            .map(|r| r.latency.as_secs_f64() * 1000.0 / r.tokens as f64)
            .collect()
    };
    tpots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Prefill throughput: prompt_tokens / ttft_seconds for each request, then median
    let mut prefill_rates: Vec<f64> = records
        .iter()
        .filter(|r| r.success && r.prompt_tokens > 0 && r.ttfb > Duration::ZERO)
        .map(|r| r.prompt_tokens as f64 / r.ttfb.as_secs_f64())
        .collect();
    prefill_rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let prefill_tok_per_sec = percentile(&prefill_rates, 0.50);

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

    // GH-114: Aggregate brick trace data across requests
    let brick_trace_summary = aggregate_brick_traces(records);

    // Per-request raw data for distribution analysis
    let request_details: Vec<RequestDetail> = records
        .iter()
        .filter(|r| r.success)
        .map(|r| {
            let itl_ms = if r.token_timestamps.len() >= 2 {
                let first = r.token_timestamps.first().unwrap();
                let last = r.token_timestamps.last().unwrap();
                let decode_ms = (*last - *first).as_secs_f64() * 1000.0;
                decode_ms / (r.token_timestamps.len() - 1) as f64
            } else if r.tokens >= 2 {
                let decode_ms = (r.latency.as_secs_f64() - r.ttfb.as_secs_f64()) * 1000.0;
                decode_ms / (r.tokens as f64 - 1.0)
            } else {
                0.0
            };
            RequestDetail {
                latency_ms: r.latency.as_secs_f64() * 1000.0,
                ttft_ms: r.ttfb.as_secs_f64() * 1000.0,
                completion_tokens: r.tokens,
                prompt_tokens: r.prompt_tokens,
                itl_ms,
            }
        })
        .collect();

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
        prefill_tok_per_sec,
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
        brick_trace_summary,
        request_details,
    }
}

/// Aggregate BrickProfiler traces across requests (GH-114).
fn aggregate_brick_traces(records: &[RequestRecord]) -> Option<Vec<BrickTraceOpSummary>> {
    let traces: Vec<&BrickTrace> = records
        .iter()
        .filter(|r| r.success)
        .filter_map(|r| r.brick_trace.as_ref())
        .collect();

    if traces.is_empty() {
        return None;
    }

    // Collect per-operation times across all requests
    let mut op_times: HashMap<String, Vec<f64>> = HashMap::new();
    let mut total_time_sum: f64 = 0.0;

    for trace in &traces {
        total_time_sum += trace.total_time_us as f64;
        for op in &trace.breakdown {
            op_times
                .entry(op.name.clone())
                .or_default()
                .push(op.time_us as f64);
        }
    }

    let avg_total = total_time_sum / traces.len() as f64;
    let mut summaries: Vec<BrickTraceOpSummary> = op_times
        .into_iter()
        .map(|(name, times)| {
            let n = times.len();
            let sum: f64 = times.iter().sum();
            let mean = sum / n as f64;
            let min = times.iter().copied().fold(f64::INFINITY, f64::min);
            let max = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let pct = if avg_total > 0.0 { (mean / avg_total) * 100.0 } else { 0.0 };
            BrickTraceOpSummary {
                name,
                mean_us: mean,
                min_us: min,
                max_us: max,
                pct_of_total: pct,
                samples: n,
            }
        })
        .collect();

    // Sort by percentage descending (hottest first)
    summaries.sort_by(|a, b| b.pct_of_total.partial_cmp(&a.pct_of_total).unwrap_or(std::cmp::Ordering::Equal));
    Some(summaries)
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
                brick_trace: None,
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
                brick_trace: None,
            },
            RequestRecord {
                latency: Duration::from_millis(0),
                ttfb: Duration::from_millis(0),
                tokens: 0,
                prompt_tokens: 0,
                success: false,
                token_timestamps: Vec::new(),
                brick_trace: None,
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
            prefill_tok_per_sec: 0.0,
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
            brick_trace_summary: None,
            request_details: Vec::new(),
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
            brick_trace: None,
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
            brick_trace: None,
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
            brick_trace: None,
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
            brick_trace: None,
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
            brick_trace: None,
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
                brick_trace: None,
            },
            RequestRecord {
                latency: Duration::from_millis(300),
                ttfb: Duration::from_millis(100),
                tokens: 10,
                prompt_tokens: 5,
                success: true,
                token_timestamps: Vec::new(),
                brick_trace: None,
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
                brick_trace: None,
            },
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 15,
                prompt_tokens: 25,
                success: true,
                token_timestamps: Vec::new(),
                brick_trace: None,
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
            brick_trace: None,
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
                brick_trace: None,
            },
            RequestRecord {
                latency: Duration::from_millis(100),
                ttfb: Duration::from_millis(50),
                tokens: 5,
                prompt_tokens: 10,
                success: true,
                token_timestamps: Vec::new(), // non-streaming request
                brick_trace: None,
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

    #[test]
    fn test_tpot_non_streaming_uses_latency_per_token() {
        // Non-streaming: ttfb ≈ latency → TPOT should use latency/tokens (not near-zero).
        // Before fix: TPOT = (latency - ttfb)/(tokens-1) = (1600-1599)/15 = 0.067ms (WRONG)
        // After fix: TPOT = latency/tokens = 1600/16 = 100ms (correct, matches ITL)
        let records = vec![RequestRecord {
            latency: Duration::from_millis(1600),
            ttfb: Duration::from_millis(1599),
            tokens: 16,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
            brick_trace: None,
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        // Both TPOT and ITL should be latency/tokens = 100ms
        assert!((result.tpot_p50_ms - 100.0).abs() < 0.1, "tpot={}", result.tpot_p50_ms);
        assert!((result.itl_p50_ms - 100.0).abs() < 0.1, "itl={}", result.itl_p50_ms);
    }

    #[test]
    fn test_itl_robust_to_token_batching() {
        // Server sends tokens in pairs (batch=2): timestamps are [100, 100, 200, 200, 300]
        // Old code (flat_map): deltas = [0, 100, 0, 100] → P50 = 50ms (bimodal, fragile)
        // New code (per-request mean): (300-100)/4 = 50ms (robust)
        // With batch=3: timestamps = [100, 100, 100, 300, 300, 300]
        // Old code: deltas = [0, 0, 200, 0, 0] → P50 = 0ms (WRONG)
        // New code: (300-100)/5 = 40ms (correct)
        let records = vec![RequestRecord {
            latency: Duration::from_millis(350),
            ttfb: Duration::from_millis(100),
            tokens: 6,
            prompt_tokens: 10,
            success: true,
            token_timestamps: vec![
                Duration::from_millis(100), // batch 1
                Duration::from_millis(100),
                Duration::from_millis(100),
                Duration::from_millis(300), // batch 2
                Duration::from_millis(300),
                Duration::from_millis(300),
            ],
            brick_trace: None,
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        // Per-request mean: (300-100)/5 = 40ms
        assert!((result.itl_p50_ms - 40.0).abs() < 0.1, "itl={}", result.itl_p50_ms);
        assert!((result.tpot_p50_ms - 40.0).abs() < 0.1, "tpot={}", result.tpot_p50_ms);
        assert!((result.decode_tok_per_sec - 25.0).abs() < 0.5, "decode={}", result.decode_tok_per_sec);
    }

    #[test]
    fn test_request_details_populated() {
        let records = vec![RequestRecord {
            latency: Duration::from_millis(200),
            ttfb: Duration::from_millis(50),
            tokens: 16,
            prompt_tokens: 10,
            success: true,
            token_timestamps: Vec::new(),
            brick_trace: None,
        }];
        let result = aggregate_results(&records, 1.0, "test", 1);
        assert_eq!(result.request_details.len(), 1);
        let detail = &result.request_details[0];
        assert!((detail.latency_ms - 200.0).abs() < 0.1);
        assert!((detail.ttft_ms - 50.0).abs() < 0.1);
        assert_eq!(detail.completion_tokens, 16);
        assert_eq!(detail.prompt_tokens, 10);
        assert!(detail.itl_ms > 0.0);
    }
}
