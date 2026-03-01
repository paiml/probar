//! OpenAI-compatible HTTP client for LLM inference endpoints.
//!
//! Supports chat completions against realizar, ollama, llama.cpp,
//! and any server exposing the OpenAI `/v1/chat/completions` API.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Chat message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System prompt
    System,
    /// User message
    User,
    /// Assistant response
    Assistant,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    pub content: String,
}

/// Parameters for a chat completion request.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// Model identifier (may be ignored by some backends).
    pub model: String,
    /// The messages for the chat completion.
    pub messages: Vec<ChatMessage>,
    /// Sampling temperature (0.0 = deterministic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Usage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,
    /// Tokens generated.
    pub completion_tokens: u32,
    /// Total tokens (prompt + completion).
    pub total_tokens: u32,
}

/// A single completion choice.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatResponseChoice {
    /// Index of this choice.
    pub index: u32,
    /// The generated message.
    pub message: ChatMessage,
    /// Why generation stopped.
    pub finish_reason: Option<String>,
}

/// Response from a chat completion endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatResponse {
    /// Unique identifier for this completion.
    pub id: String,
    /// Object type (always "chat.completion").
    pub object: String,
    /// Unix timestamp of creation.
    pub created: u64,
    /// Model used.
    pub model: String,
    /// Generated choices.
    pub choices: Vec<ChatResponseChoice>,
    /// Token usage statistics.
    pub usage: Option<Usage>,
}

/// A chat response with timing metadata.
#[derive(Debug, Clone)]
pub struct TimedChatResponse {
    /// The API response.
    pub response: ChatResponse,
    /// Total request duration (time to last byte).
    pub latency: Duration,
    /// Time to first byte (approximation for non-streaming).
    pub ttfb: Duration,
}

/// Errors from the LLM client.
#[derive(Debug, thiserror::Error)]
pub enum LlmClientError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// Server returned an error status.
    #[error("API error {status}: {body}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Response body.
        body: String,
    },
    /// Health check failed.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
}

/// OpenAI-compatible HTTP client for LLM inference.
#[derive(Debug, Clone)]
pub struct LlmClient {
    base_url: String,
    client: reqwest::Client,
    model: String,
}

impl LlmClient {
    /// Create a new client pointing at the given base URL.
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the API server (e.g., `http://localhost:8081`)
    /// * `model` - Model name to include in requests
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            model: model.into(),
        }
    }

    /// Create a client with a custom reqwest client (for custom timeouts, etc.).
    pub fn with_client(
        base_url: impl Into<String>,
        model: impl Into<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            model: model.into(),
        }
    }

    /// Returns the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Send a chat completion request and return the response with timing.
    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    ) -> Result<TimedChatResponse, LlmClientError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature,
            max_tokens,
            stream: Some(false),
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let start = Instant::now();

        let resp = self.client.post(&url).json(&request).send().await?;
        let ttfb = start.elapsed();

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmClientError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let response: ChatResponse = resp.json().await?;
        let latency = start.elapsed();

        Ok(TimedChatResponse {
            response,
            latency,
            ttfb,
        })
    }

    /// Send a raw `ChatRequest` and return the timed response.
    pub async fn send(&self, request: &ChatRequest) -> Result<TimedChatResponse, LlmClientError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let start = Instant::now();

        // Use the client's model name if the request's model is empty
        let actual_request;
        let req = if request.model.is_empty() {
            actual_request = ChatRequest {
                model: self.model.clone(),
                ..request.clone()
            };
            &actual_request
        } else {
            request
        };

        let resp = self.client.post(&url).json(req).send().await?;
        let ttfb = start.elapsed();

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmClientError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let response: ChatResponse = resp.json().await?;
        let latency = start.elapsed();

        Ok(TimedChatResponse {
            response,
            latency,
            ttfb,
        })
    }

    /// Check if the server is reachable by hitting common health endpoints.
    pub async fn health_check(&self) -> Result<bool, LlmClientError> {
        // Try /health, /v1/models, then root
        for path in &["/health", "/v1/models", "/"] {
            let url = format!("{}{path}", self.base_url);
            if let Ok(resp) = self.client.get(&url).send().await {
                if resp.status().is_success() {
                    return Ok(true);
                }
            }
        }
        Err(LlmClientError::HealthCheckFailed(format!(
            "No health endpoint responded at {}",
            self.base_url
        )))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = LlmClient::new("http://localhost:8081", "qwen-coder");
        assert_eq!(client.base_url(), "http://localhost:8081");
        assert_eq!(client.model(), "qwen-coder");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = LlmClient::new("http://localhost:8081/", "model");
        assert_eq!(client.base_url(), "http://localhost:8081");
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: Role::User,
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            model: "test".to_string(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hi".to_string(),
            }],
            temperature: Some(0.0),
            max_tokens: Some(32),
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\":0.0"));
        assert!(json.contains("\"max_tokens\":32"));
        // stream is None, should be omitted
        assert!(!json.contains("stream"));
    }

    #[test]
    fn test_chat_request_omits_none_fields() {
        let req = ChatRequest {
            model: "test".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("stream"));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "qwen-coder",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-123");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello!");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_apr_response_deserialization() {
        let json = r#"{"_apr_metrics":{"latency_ms":1978,"tok_per_sec":4.14},"choices":[{"finish_reason":"stop","index":0,"message":{"content":"hello","role":"assistant"}}],"created":1772386202,"id":"chatcmpl-123","model":"test","object":"chat.completion","usage":{"completion_tokens":8,"prompt_tokens":9,"total_tokens":17}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hello");
    }

    #[test]
    fn test_gguf_response_with_name_null() {
        let json = r#"{"id":"chatcmpl-q4k-123","object":"chat.completion","created":1772385841,"model":"qwen","choices":[{"index":0,"message":{"role":"assistant","content":"4","name":null},"finish_reason":"stop"}],"usage":{"prompt_tokens":24,"completion_tokens":1,"total_tokens":25}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "4");
    }

    #[test]
    fn test_chat_response_without_usage() {
        let json = r#"{
            "id": "abc",
            "object": "chat.completion",
            "created": 0,
            "model": "m",
            "choices": []
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
        assert!(resp.choices.is_empty());
    }

    #[test]
    fn test_role_serialization_roundtrip() {
        for (role, expected) in [
            (Role::System, "\"system\""),
            (Role::User, "\"user\""),
            (Role::Assistant, "\"assistant\""),
        ] {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);
            let back: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(back, role);
        }
    }

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_client_with_custom_client() {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let client = LlmClient::with_client("http://example.com", "model", http);
        assert_eq!(client.base_url(), "http://example.com");
    }
}
