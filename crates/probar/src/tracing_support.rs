//! Execution Tracing (Feature 9)
//!
//! Comprehensive tracing of test execution for debugging.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Genchi Genbutsu**: Go and see - trace shows actual execution
//! - **Jidoka**: Fail-fast with detailed context
//! - **Mieruka**: Visual representation of test flow

use crate::result::ProbarResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Instant, SystemTime};
use uuid::Uuid;

/// Configuration for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Capture screenshots during tracing
    pub capture_screenshots: bool,
    /// Capture network events
    pub capture_network: bool,
    /// Capture console output
    pub capture_console: bool,
    /// Capture performance metrics
    pub capture_performance: bool,
    /// Maximum events to store
    pub max_events: usize,
    /// Include timestamps
    pub include_timestamps: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            capture_screenshots: true,
            capture_network: true,
            capture_console: true,
            capture_performance: true,
            max_events: 10000,
            include_timestamps: true,
        }
    }
}

impl TracingConfig {
    /// Create a new config
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all capture options
    #[must_use]
    pub const fn capture_all(mut self) -> Self {
        self.capture_screenshots = true;
        self.capture_network = true;
        self.capture_console = true;
        self.capture_performance = true;
        self
    }

    /// Disable all capture options
    #[must_use]
    pub const fn capture_none(mut self) -> Self {
        self.capture_screenshots = false;
        self.capture_network = false;
        self.capture_console = false;
        self.capture_performance = false;
        self
    }

    /// Set maximum events
    #[must_use]
    pub const fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }
}

/// A traced span (a named section of execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracedSpan {
    /// Unique span ID
    pub id: String,
    /// Parent span ID (if nested)
    pub parent_id: Option<String>,
    /// Span name
    pub name: String,
    /// Start timestamp (ms since trace start)
    pub start_ms: u64,
    /// End timestamp (ms since trace start)
    pub end_ms: Option<u64>,
    /// Span duration
    pub duration_ms: Option<u64>,
    /// Span attributes
    pub attributes: HashMap<String, String>,
    /// Span status
    pub status: SpanStatus,
}

impl TracedSpan {
    /// Create a new span
    #[must_use]
    pub fn new(name: &str, start_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            name: name.to_string(),
            start_ms,
            end_ms: None,
            duration_ms: None,
            attributes: HashMap::new(),
            status: SpanStatus::Running,
        }
    }

    /// Set parent ID
    #[must_use]
    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    /// End the span
    pub fn end(&mut self, end_ms: u64) {
        self.end_ms = Some(end_ms);
        self.duration_ms = Some(end_ms.saturating_sub(self.start_ms));
        if self.status == SpanStatus::Running {
            self.status = SpanStatus::Ok;
        }
    }

    /// Mark as error
    pub fn mark_error(&mut self, message: &str) {
        self.status = SpanStatus::Error;
        self.add_attribute("error.message", message);
    }

    /// Check if span is complete
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.end_ms.is_some()
    }
}

/// Status of a span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Span is running
    Running,
    /// Span completed successfully
    Ok,
    /// Span completed with error
    Error,
    /// Span was cancelled
    Cancelled,
}

/// A traced event (a point-in-time occurrence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracedEvent {
    /// Event timestamp (ms since trace start)
    pub timestamp_ms: u64,
    /// Event name
    pub name: String,
    /// Event category
    pub category: EventCategory,
    /// Event level
    pub level: EventLevel,
    /// Event message
    pub message: String,
    /// Event attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl TracedEvent {
    /// Create a new event
    #[must_use]
    pub fn new(name: &str, category: EventCategory, timestamp_ms: u64) -> Self {
        Self {
            timestamp_ms,
            name: name.to_string(),
            category,
            level: EventLevel::Info,
            message: String::new(),
            attributes: HashMap::new(),
        }
    }

    /// Set message
    #[must_use]
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    /// Set level
    #[must_use]
    pub const fn with_level(mut self, level: EventLevel) -> Self {
        self.level = level;
        self
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, key: &str, value: serde_json::Value) {
        self.attributes.insert(key.to_string(), value);
    }
}

/// Category of traced events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    /// Test lifecycle
    Test,
    /// User interaction
    Interaction,
    /// Network activity
    Network,
    /// Console output
    Console,
    /// Screenshot capture
    Screenshot,
    /// Performance metric
    Performance,
    /// Assertion
    Assertion,
    /// Custom event
    Custom,
}

/// Level of traced events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventLevel {
    /// Trace level (most verbose)
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

/// Network event for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    /// Timestamp (ms since trace start)
    pub timestamp_ms: u64,
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// Response status code
    pub status: Option<u16>,
    /// Request duration in ms
    pub duration_ms: Option<u64>,
    /// Request size in bytes
    pub request_size: Option<u64>,
    /// Response size in bytes
    pub response_size: Option<u64>,
    /// Content type
    pub content_type: Option<String>,
    /// Whether request failed
    pub failed: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl NetworkEvent {
    /// Create a new network event
    #[must_use]
    pub fn new(url: &str, method: &str, timestamp_ms: u64) -> Self {
        Self {
            timestamp_ms,
            url: url.to_string(),
            method: method.to_string(),
            status: None,
            duration_ms: None,
            request_size: None,
            response_size: None,
            content_type: None,
            failed: false,
            error: None,
        }
    }

    /// Complete the network event
    pub fn complete(&mut self, status: u16, duration_ms: u64) {
        self.status = Some(status);
        self.duration_ms = Some(duration_ms);
    }

    /// Mark as failed
    pub fn fail(&mut self, error: &str) {
        self.failed = true;
        self.error = Some(error.to_string());
    }
}

/// Console message for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    /// Timestamp (ms since trace start)
    pub timestamp_ms: u64,
    /// Message level
    pub level: ConsoleLevel,
    /// Message text
    pub text: String,
    /// Source URL
    pub source: Option<String>,
    /// Line number
    pub line: Option<u32>,
}

/// Console message level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsoleLevel {
    /// Log level
    Log,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
    /// Debug level
    Debug,
}

/// Metadata for a trace archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    /// Trace ID
    pub trace_id: String,
    /// Test name
    pub test_name: String,
    /// Start time
    pub start_time: SystemTime,
    /// End time
    pub end_time: Option<SystemTime>,
    /// Total duration in ms
    pub duration_ms: Option<u64>,
    /// Number of spans
    pub span_count: usize,
    /// Number of events
    pub event_count: usize,
    /// Probar version
    pub probar_version: String,
}

impl TraceMetadata {
    /// Create new metadata
    #[must_use]
    pub fn new(test_name: &str) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            test_name: test_name.to_string(),
            start_time: SystemTime::now(),
            end_time: None,
            duration_ms: None,
            span_count: 0,
            event_count: 0,
            probar_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Complete trace archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceArchive {
    /// Trace metadata
    pub metadata: TraceMetadata,
    /// All traced spans
    pub spans: Vec<TracedSpan>,
    /// All traced events
    pub events: Vec<TracedEvent>,
    /// Network events
    pub network_events: Vec<NetworkEvent>,
    /// Console messages
    pub console_messages: Vec<ConsoleMessage>,
}

impl TraceArchive {
    /// Create a new archive
    #[must_use]
    pub fn new(metadata: TraceMetadata) -> Self {
        Self {
            metadata,
            spans: Vec::new(),
            events: Vec::new(),
            network_events: Vec::new(),
            console_messages: Vec::new(),
        }
    }

    /// Save archive to JSON file
    pub fn save_json(&self, path: &Path) -> ProbarResult<()> {
        let json = serde_json::to_string_pretty(self)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, json)?;
        Ok(())
    }

    /// Load archive from JSON file
    pub fn load_json(path: &Path) -> ProbarResult<Self> {
        let json = fs::read_to_string(path)?;
        let archive: TraceArchive = serde_json::from_str(&json)?;
        Ok(archive)
    }

    /// Get spans by name
    #[must_use]
    pub fn spans_by_name(&self, name: &str) -> Vec<&TracedSpan> {
        self.spans.iter().filter(|s| s.name == name).collect()
    }

    /// Get events by category
    #[must_use]
    pub fn events_by_category(&self, category: EventCategory) -> Vec<&TracedEvent> {
        self.events
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Get failed network requests
    #[must_use]
    pub fn failed_requests(&self) -> Vec<&NetworkEvent> {
        self.network_events.iter().filter(|n| n.failed).collect()
    }

    /// Get error spans
    #[must_use]
    pub fn error_spans(&self) -> Vec<&TracedSpan> {
        self.spans
            .iter()
            .filter(|s| s.status == SpanStatus::Error)
            .collect()
    }
}

/// Execution tracer
#[derive(Debug)]
pub struct ExecutionTracer {
    config: TracingConfig,
    start_time: Instant,
    metadata: TraceMetadata,
    spans: Vec<TracedSpan>,
    events: Vec<TracedEvent>,
    network_events: Vec<NetworkEvent>,
    console_messages: Vec<ConsoleMessage>,
    current_span_id: Option<String>,
    running: bool,
}

impl ExecutionTracer {
    /// Create a new execution tracer
    #[must_use]
    pub fn new(test_name: &str, config: TracingConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            metadata: TraceMetadata::new(test_name),
            spans: Vec::new(),
            events: Vec::new(),
            network_events: Vec::new(),
            console_messages: Vec::new(),
            current_span_id: None,
            running: false,
        }
    }

    /// Start tracing
    pub fn start(&mut self) {
        self.running = true;
        self.start_time = Instant::now();
        self.metadata.start_time = SystemTime::now();
    }

    /// Stop tracing and return archive
    #[must_use]
    pub fn stop(&mut self) -> TraceArchive {
        self.running = false;
        self.metadata.end_time = Some(SystemTime::now());
        self.metadata.duration_ms = Some(self.elapsed_ms());
        self.metadata.span_count = self.spans.len();
        self.metadata.event_count = self.events.len();

        // Close any open spans
        let end_ms = self.elapsed_ms();
        for span in &mut self.spans {
            if !span.is_complete() {
                span.end(end_ms);
                span.status = SpanStatus::Cancelled;
            }
        }

        TraceArchive {
            metadata: self.metadata.clone(),
            spans: self.spans.clone(),
            events: self.events.clone(),
            network_events: self.network_events.clone(),
            console_messages: self.console_messages.clone(),
        }
    }

    /// Get elapsed time in milliseconds
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Start a new span
    pub fn start_span(&mut self, name: &str) -> String {
        let span = TracedSpan::new(name, self.elapsed_ms())
            .with_parent(self.current_span_id.as_deref().unwrap_or(""));

        let id = span.id.clone();
        self.current_span_id = Some(id.clone());
        self.spans.push(span);
        id
    }

    /// End a span
    pub fn end_span(&mut self, span_id: &str) {
        let end_ms = self.elapsed_ms();
        if let Some(span) = self.spans.iter_mut().find(|s| s.id == span_id) {
            span.end(end_ms);

            // Restore parent as current
            if let Some(parent_id) = &span.parent_id {
                if !parent_id.is_empty() {
                    self.current_span_id = Some(parent_id.clone());
                } else {
                    self.current_span_id = None;
                }
            }
        }
    }

    /// Mark a span as error
    pub fn error_span(&mut self, span_id: &str, message: &str) {
        if let Some(span) = self.spans.iter_mut().find(|s| s.id == span_id) {
            span.mark_error(message);
        }
    }

    /// Record an event
    pub fn record_event(&mut self, event: TracedEvent) {
        if self.events.len() < self.config.max_events {
            self.events.push(event);
        }
    }

    /// Record a network event
    pub fn record_network(&mut self, event: NetworkEvent) {
        if self.config.capture_network && self.network_events.len() < self.config.max_events {
            self.network_events.push(event);
        }
    }

    /// Record a console message
    pub fn record_console(&mut self, message: ConsoleMessage) {
        if self.config.capture_console && self.console_messages.len() < self.config.max_events {
            self.console_messages.push(message);
        }
    }

    /// Log an info event
    pub fn info(&mut self, name: &str, message: &str) {
        let event = TracedEvent::new(name, EventCategory::Custom, self.elapsed_ms())
            .with_message(message)
            .with_level(EventLevel::Info);
        self.record_event(event);
    }

    /// Log a warning event
    pub fn warn(&mut self, name: &str, message: &str) {
        let event = TracedEvent::new(name, EventCategory::Custom, self.elapsed_ms())
            .with_message(message)
            .with_level(EventLevel::Warn);
        self.record_event(event);
    }

    /// Log an error event
    pub fn error(&mut self, name: &str, message: &str) {
        let event = TracedEvent::new(name, EventCategory::Custom, self.elapsed_ms())
            .with_message(message)
            .with_level(EventLevel::Error);
        self.record_event(event);
    }

    /// Check if tracing is running
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Get current span count
    #[must_use]
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Get current event count
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    mod tracing_config_tests {
        use super::*;

        #[test]
        fn test_default() {
            let config = TracingConfig::default();
            assert!(config.capture_screenshots);
            assert!(config.capture_network);
            assert!(config.capture_console);
            assert!(config.capture_performance);
        }

        #[test]
        fn test_capture_all() {
            let config = TracingConfig::new().capture_none().capture_all();
            assert!(config.capture_screenshots);
            assert!(config.capture_network);
        }

        #[test]
        fn test_capture_none() {
            let config = TracingConfig::new().capture_none();
            assert!(!config.capture_screenshots);
            assert!(!config.capture_network);
            assert!(!config.capture_console);
            assert!(!config.capture_performance);
        }

        #[test]
        fn test_with_max_events() {
            let config = TracingConfig::new().with_max_events(5000);
            assert_eq!(config.max_events, 5000);
        }
    }

    mod traced_span_tests {
        use super::*;

        #[test]
        fn test_new() {
            let span = TracedSpan::new("test_span", 100);
            assert_eq!(span.name, "test_span");
            assert_eq!(span.start_ms, 100);
            assert!(span.parent_id.is_none());
            assert_eq!(span.status, SpanStatus::Running);
        }

        #[test]
        fn test_with_parent() {
            let span = TracedSpan::new("child", 100).with_parent("parent_id");
            assert_eq!(span.parent_id, Some("parent_id".to_string()));
        }

        #[test]
        fn test_add_attribute() {
            let mut span = TracedSpan::new("test", 0);
            span.add_attribute("key", "value");
            assert_eq!(span.attributes.get("key"), Some(&"value".to_string()));
        }

        #[test]
        fn test_end() {
            let mut span = TracedSpan::new("test", 100);
            span.end(200);

            assert_eq!(span.end_ms, Some(200));
            assert_eq!(span.duration_ms, Some(100));
            assert_eq!(span.status, SpanStatus::Ok);
            assert!(span.is_complete());
        }

        #[test]
        fn test_mark_error() {
            let mut span = TracedSpan::new("test", 0);
            span.mark_error("Something went wrong");

            assert_eq!(span.status, SpanStatus::Error);
            assert_eq!(
                span.attributes.get("error.message"),
                Some(&"Something went wrong".to_string())
            );
        }
    }

    mod traced_event_tests {
        use super::*;

        #[test]
        fn test_new() {
            let event = TracedEvent::new("click", EventCategory::Interaction, 500);
            assert_eq!(event.name, "click");
            assert_eq!(event.category, EventCategory::Interaction);
            assert_eq!(event.timestamp_ms, 500);
        }

        #[test]
        fn test_with_message() {
            let event =
                TracedEvent::new("test", EventCategory::Test, 0).with_message("Test started");
            assert_eq!(event.message, "Test started");
        }

        #[test]
        fn test_with_level() {
            let event =
                TracedEvent::new("test", EventCategory::Test, 0).with_level(EventLevel::Error);
            assert_eq!(event.level, EventLevel::Error);
        }

        #[test]
        fn test_add_attribute() {
            let mut event = TracedEvent::new("test", EventCategory::Test, 0);
            event.add_attribute("count", serde_json::json!(42));
            assert_eq!(event.attributes.get("count"), Some(&serde_json::json!(42)));
        }
    }

    mod network_event_tests {
        use super::*;

        #[test]
        fn test_new() {
            let event = NetworkEvent::new("https://example.com", "GET", 1000);
            assert_eq!(event.url, "https://example.com");
            assert_eq!(event.method, "GET");
            assert_eq!(event.timestamp_ms, 1000);
            assert!(!event.failed);
        }

        #[test]
        fn test_complete() {
            let mut event = NetworkEvent::new("https://example.com", "GET", 1000);
            event.complete(200, 150);

            assert_eq!(event.status, Some(200));
            assert_eq!(event.duration_ms, Some(150));
        }

        #[test]
        fn test_fail() {
            let mut event = NetworkEvent::new("https://example.com", "GET", 1000);
            event.fail("Connection timeout");

            assert!(event.failed);
            assert_eq!(event.error, Some("Connection timeout".to_string()));
        }
    }

    mod trace_archive_tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn test_new() {
            let metadata = TraceMetadata::new("test");
            let archive = TraceArchive::new(metadata);

            assert!(archive.spans.is_empty());
            assert!(archive.events.is_empty());
        }

        #[test]
        fn test_spans_by_name() {
            let mut archive = TraceArchive::new(TraceMetadata::new("test"));
            archive.spans.push(TracedSpan::new("click", 0));
            archive.spans.push(TracedSpan::new("type", 100));
            archive.spans.push(TracedSpan::new("click", 200));

            let clicks = archive.spans_by_name("click");
            assert_eq!(clicks.len(), 2);
        }

        #[test]
        fn test_events_by_category() {
            let mut archive = TraceArchive::new(TraceMetadata::new("test"));
            archive
                .events
                .push(TracedEvent::new("e1", EventCategory::Network, 0));
            archive
                .events
                .push(TracedEvent::new("e2", EventCategory::Console, 100));
            archive
                .events
                .push(TracedEvent::new("e3", EventCategory::Network, 200));

            let network = archive.events_by_category(EventCategory::Network);
            assert_eq!(network.len(), 2);
        }

        #[test]
        fn test_failed_requests() {
            let mut archive = TraceArchive::new(TraceMetadata::new("test"));

            let mut success = NetworkEvent::new("https://example.com", "GET", 0);
            success.complete(200, 100);
            archive.network_events.push(success);

            let mut failure = NetworkEvent::new("https://error.com", "GET", 100);
            failure.fail("404");
            archive.network_events.push(failure);

            let failed = archive.failed_requests();
            assert_eq!(failed.len(), 1);
        }

        #[test]
        fn test_error_spans() {
            let mut archive = TraceArchive::new(TraceMetadata::new("test"));

            let mut ok_span = TracedSpan::new("ok", 0);
            ok_span.end(100);
            archive.spans.push(ok_span);

            let mut error_span = TracedSpan::new("error", 100);
            error_span.mark_error("Failed");
            archive.spans.push(error_span);

            let errors = archive.error_spans();
            assert_eq!(errors.len(), 1);
        }

        #[test]
        fn test_save_and_load() {
            let temp_dir = TempDir::new().unwrap();
            let path = temp_dir.path().join("trace.json");

            let mut archive = TraceArchive::new(TraceMetadata::new("test"));
            archive.spans.push(TracedSpan::new("span1", 0));
            archive
                .events
                .push(TracedEvent::new("event1", EventCategory::Test, 0));

            archive.save_json(&path).unwrap();
            assert!(path.exists());

            let loaded = TraceArchive::load_json(&path).unwrap();
            assert_eq!(loaded.spans.len(), 1);
            assert_eq!(loaded.events.len(), 1);
        }
    }

    mod execution_tracer_tests {
        use super::*;

        #[test]
        fn test_new() {
            let tracer = ExecutionTracer::new("test", TracingConfig::default());
            assert!(!tracer.is_running());
            assert_eq!(tracer.span_count(), 0);
            assert_eq!(tracer.event_count(), 0);
        }

        #[test]
        fn test_start_stop() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();
            assert!(tracer.is_running());

            let archive = tracer.stop();
            assert!(!tracer.is_running());
            assert!(archive.metadata.duration_ms.is_some());
        }

        #[test]
        fn test_start_and_end_span() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();

            let span_id = tracer.start_span("my_span");
            assert_eq!(tracer.span_count(), 1);

            tracer.end_span(&span_id);

            let archive = tracer.stop();
            assert!(archive.spans[0].is_complete());
        }

        #[test]
        fn test_nested_spans() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();

            let parent_id = tracer.start_span("parent");
            let child_id = tracer.start_span("child");

            tracer.end_span(&child_id);
            tracer.end_span(&parent_id);

            let archive = tracer.stop();
            assert_eq!(archive.spans.len(), 2);

            let child = &archive.spans[1];
            assert!(child.parent_id.is_some());
        }

        #[test]
        fn test_info_warn_error() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();

            tracer.info("test", "Info message");
            tracer.warn("test", "Warning message");
            tracer.error("test", "Error message");

            let archive = tracer.stop();
            assert_eq!(archive.events.len(), 3);
            assert_eq!(archive.events[0].level, EventLevel::Info);
            assert_eq!(archive.events[1].level, EventLevel::Warn);
            assert_eq!(archive.events[2].level, EventLevel::Error);
        }

        #[test]
        fn test_record_network() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();

            let event = NetworkEvent::new("https://example.com", "GET", tracer.elapsed_ms());
            tracer.record_network(event);

            let archive = tracer.stop();
            assert_eq!(archive.network_events.len(), 1);
        }

        #[test]
        fn test_record_console() {
            let mut tracer = ExecutionTracer::new("test", TracingConfig::default());
            tracer.start();

            let message = ConsoleMessage {
                timestamp_ms: tracer.elapsed_ms(),
                level: ConsoleLevel::Log,
                text: "Hello".to_string(),
                source: None,
                line: None,
            };
            tracer.record_console(message);

            let archive = tracer.stop();
            assert_eq!(archive.console_messages.len(), 1);
        }

        #[test]
        fn test_max_events_limit() {
            let config = TracingConfig::default().with_max_events(3);
            let mut tracer = ExecutionTracer::new("test", config);
            tracer.start();

            for i in 0..10 {
                tracer.info("test", &format!("Event {}", i));
            }

            let archive = tracer.stop();
            assert_eq!(archive.events.len(), 3);
        }
    }
}
