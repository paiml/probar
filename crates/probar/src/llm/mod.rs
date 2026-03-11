//! LLM Testing: Correctness assertions and load testing for OpenAI-compatible APIs.
//!
//! This module provides tools for testing LLM inference endpoints:
//! - **Client types**: Typed request/response structs for OpenAI-compatible APIs (feature: `llm-types`)
//! - **Assertions**: Structural and semantic correctness checks on LLM outputs (feature: `llm-types`)
//! - **Client**: HTTP client for OpenAI-compatible chat completion APIs (feature: `llm`)
//! - **Load testing**: Concurrent request generation with latency/throughput metrics (feature: `llm`)
//! - **Reporting**: JSON and Markdown report generation with historical tracking (feature: `llm`)

pub mod assertion;
#[cfg(feature = "llm")]
pub mod benchmark;
pub mod client;
pub mod experiment;
#[cfg(feature = "llm")]
pub mod gpu_telemetry;
#[cfg(feature = "llm")]
pub mod loadtest;
pub mod prompts;
#[cfg(feature = "llm")]
pub mod report;
#[cfg(feature = "llm")]
pub mod score;

pub use assertion::{LlmAssertion, LlmAssertionError, LlmAssertionResult};
pub use client::{
    BrickTrace, BrickTraceOp, ChatMessage, ChatRequest, ChatResponse, ChatResponseChoice, Role,
    StreamChunk, StreamedChatResponse, TimedChatResponse, Usage,
};
#[cfg(feature = "llm")]
pub use client::{LlmClient, LlmClientError};
pub use experiment::{
    BudgetConfig, DataAuditResult, EarlyStoppingConfig, Experiment, ExperimentRun,
    ExperimentStatus, KillCriterion, MetricSnapshot,
};
#[cfg(feature = "llm")]
pub use gpu_telemetry::{extract_host_from_url, GpuTelemetryCollector};
#[cfg(feature = "llm")]
pub use loadtest::{
    BrickTraceOpSummary, DatasetStats, DriftAnalysis, GpuTelemetry, JitterAnalysis, LatencySpike,
    LoadTest, LoadTestConfig, LoadTestResult, QualityFailure, QualityResult, RequestDetail,
    RequestRate, SweepLevel, SweepResult, TailAnalysis, TelemetryStat, ValidationMode,
};
pub use prompts::{load_from_file as load_prompts_from_file, load_profile, PromptProfile};
#[cfg(feature = "llm")]
pub use report::{to_json, to_markdown_row, to_markdown_table, update_performance_md};
#[cfg(feature = "llm")]
pub use score::{
    assign_grade, compute_metric_score, compute_scorecard, format_markdown, format_table,
    MetricScore, MetricThreshold, RuntimeScore, Scorecard, ScoringContract,
};
