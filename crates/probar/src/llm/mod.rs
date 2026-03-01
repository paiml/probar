//! LLM Testing: Correctness assertions and load testing for OpenAI-compatible APIs.
//!
//! This module provides tools for testing LLM inference endpoints:
//! - **Client**: HTTP client for OpenAI-compatible chat completion APIs
//! - **Assertions**: Structural and semantic correctness checks on LLM outputs
//! - **Load testing**: Concurrent request generation with latency/throughput metrics
//! - **Reporting**: JSON and Markdown report generation with historical tracking

pub mod assertion;
pub mod client;
pub mod loadtest;
pub mod report;

pub use assertion::{LlmAssertion, LlmAssertionError, LlmAssertionResult};
pub use client::{
    ChatMessage, ChatRequest, ChatResponse, ChatResponseChoice, LlmClient, LlmClientError, Role,
    Usage,
};
pub use loadtest::{LoadTest, LoadTestConfig, LoadTestResult};
pub use report::{to_json, to_markdown_row, to_markdown_table, update_performance_md};
