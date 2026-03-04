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
#[cfg(feature = "llm")]
pub mod loadtest;
#[cfg(feature = "llm")]
pub mod prompts;
#[cfg(feature = "llm")]
pub mod report;

pub use assertion::{LlmAssertion, LlmAssertionError, LlmAssertionResult};
pub use client::{
    BrickTrace, BrickTraceOp, ChatMessage, ChatRequest, ChatResponse, ChatResponseChoice,
    Role, StreamChunk, StreamedChatResponse, TimedChatResponse, Usage,
};
#[cfg(feature = "llm")]
pub use client::{LlmClient, LlmClientError};
#[cfg(feature = "llm")]
pub use loadtest::{BrickTraceOpSummary, LoadTest, LoadTestConfig, LoadTestResult};
#[cfg(feature = "llm")]
pub use prompts::{load_from_file as load_prompts_from_file, load_profile, PromptProfile};
#[cfg(feature = "llm")]
pub use report::{to_json, to_markdown_row, to_markdown_table, update_performance_md};
