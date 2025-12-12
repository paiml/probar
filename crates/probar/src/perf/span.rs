//! Performance Tracing Spans
//!
//! Hierarchical span-based performance measurement.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);

/// Unique span identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl SpanId {
    /// Create a new unique span ID
    #[must_use]
    pub fn new() -> Self {
        Self(NEXT_SPAN_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

/// A performance measurement span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique identifier
    pub id: SpanId,
    /// Span name
    pub name: String,
    /// Start timestamp (nanoseconds from trace start)
    pub start_ns: u64,
    /// End timestamp (nanoseconds from trace start)
    pub end_ns: Option<u64>,
    /// Parent span ID
    pub parent: Option<SpanId>,
    /// Span category
    pub category: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl Span {
    /// Create a new span
    #[must_use]
    pub fn new(name: impl Into<String>, start_ns: u64) -> Self {
        Self {
            id: SpanId::new(),
            name: name.into(),
            start_ns,
            end_ns: None,
            parent: None,
            category: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create span with parent
    #[must_use]
    pub fn with_parent(mut self, parent: SpanId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set category
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Close the span
    pub fn close(&mut self, end_ns: u64) {
        self.end_ns = Some(end_ns);
    }

    /// Get duration in nanoseconds
    #[must_use]
    pub fn duration_ns(&self) -> Option<u64> {
        self.end_ns.map(|end| end.saturating_sub(self.start_ns))
    }

    /// Get duration as Duration
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        self.duration_ns().map(Duration::from_nanos)
    }

    /// Check if span is closed
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.end_ns.is_some()
    }
}

/// Internal tracer state for RefCell-based interior mutability
pub(crate) struct TracerState {
    pub active_spans: std::collections::HashMap<SpanId, ActiveSpan>,
    pub completed_spans: Vec<Span>,
    pub current_span: Option<SpanId>,
    pub trace_start: Option<Instant>,
}

impl TracerState {
    pub fn new() -> Self {
        Self {
            active_spans: std::collections::HashMap::new(),
            completed_spans: Vec::new(),
            current_span: None,
            trace_start: None,
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.trace_start
            .map(|start| start.elapsed().as_nanos() as u64)
            .unwrap_or(0)
    }
}

/// Shared reference to tracer state
pub(crate) type SharedTracerState = Rc<RefCell<TracerState>>;

/// RAII guard for automatic span closure
pub struct SpanGuard {
    state: SharedTracerState,
    span_id: SpanId,
    max_spans: usize,
}

impl std::fmt::Debug for SpanGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpanGuard")
            .field("span_id", &self.span_id)
            .field("max_spans", &self.max_spans)
            .finish_non_exhaustive()
    }
}

impl SpanGuard {
    /// Create a new span guard
    pub(crate) fn new(state: SharedTracerState, span_id: SpanId, max_spans: usize) -> Self {
        Self {
            state,
            span_id,
            max_spans,
        }
    }

    /// Get the span ID
    #[must_use]
    pub fn id(&self) -> SpanId {
        self.span_id
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if let Some(mut active) = state.active_spans.remove(&self.span_id) {
            let end_ns = state.elapsed_ns();
            active.span.close(end_ns);

            // Update current span to parent
            state.current_span = active.span.parent;

            // Store completed span
            if state.completed_spans.len() < self.max_spans {
                state.completed_spans.push(active.span);
            }
        }
    }
}

/// Internal span with timing information
#[derive(Debug)]
pub(crate) struct ActiveSpan {
    pub span: Span,
    #[allow(dead_code)]
    pub start_instant: Instant,
}

impl ActiveSpan {
    pub fn new(name: impl Into<String>, start_ns: u64, start_instant: Instant) -> Self {
        Self {
            span: Span::new(name, start_ns),
            start_instant,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_span_id_unique() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_span_id_as_u64() {
        let id = SpanId::new();
        assert!(id.as_u64() > 0);
    }

    #[test]
    fn test_span_new() {
        let span = Span::new("test", 1000);
        assert_eq!(span.name, "test");
        assert_eq!(span.start_ns, 1000);
        assert!(!span.is_closed());
    }

    #[test]
    fn test_span_with_parent() {
        let parent_id = SpanId::new();
        let span = Span::new("child", 2000).with_parent(parent_id);
        assert_eq!(span.parent, Some(parent_id));
    }

    #[test]
    fn test_span_with_category() {
        let span = Span::new("test", 0).with_category("render");
        assert_eq!(span.category, Some("render".to_string()));
    }

    #[test]
    fn test_span_close() {
        let mut span = Span::new("test", 1000);
        span.close(2000);
        assert!(span.is_closed());
        assert_eq!(span.duration_ns(), Some(1000));
    }

    #[test]
    fn test_span_duration() {
        let mut span = Span::new("test", 0);
        span.close(1_000_000); // 1ms in ns
        let duration = span.duration().unwrap();
        assert_eq!(duration, Duration::from_nanos(1_000_000));
    }

    #[test]
    fn test_span_metadata() {
        let mut span = Span::new("test", 0);
        span.add_metadata("key", "value");
        assert_eq!(span.metadata.get("key"), Some(&"value".to_string()));
    }
}
