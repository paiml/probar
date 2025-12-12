//! Performance Metrics
//!
//! Statistical analysis of performance data.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Statistical summary of values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Mean (average)
    pub mean: f64,
    /// Median value
    pub median: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
    /// Sample count
    pub count: usize,
}

impl Statistics {
    /// Calculate statistics from a slice of values
    #[must_use]
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::empty();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = values.len();
        let min = sorted[0];
        let max = sorted[count - 1];
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2]
        };

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        let p95_idx = ((count as f64 * 0.95) as usize).min(count - 1);
        let p99_idx = ((count as f64 * 0.99) as usize).min(count - 1);

        Self {
            min,
            max,
            mean,
            median,
            std_dev,
            p95: sorted[p95_idx],
            p99: sorted[p99_idx],
            count,
        }
    }

    /// Create empty statistics
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            p95: 0.0,
            p99: 0.0,
            count: 0,
        }
    }

    /// Check if within acceptable range
    #[must_use]
    pub fn within_budget(&self, budget_ms: f64) -> bool {
        self.p99 <= budget_ms
    }
}

/// Frame timing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetrics {
    /// Frame time in milliseconds
    pub frame_time_ms: f64,
    /// Frame number
    pub frame_number: u64,
    /// Timestamp
    pub timestamp_ms: f64,
}

impl FrameMetrics {
    /// Create new frame metrics
    #[must_use]
    pub fn new(frame_time_ms: f64) -> Self {
        Self {
            frame_time_ms,
            frame_number: 0,
            timestamp_ms: 0.0,
        }
    }

    /// Create with frame number
    #[must_use]
    pub fn with_frame_number(mut self, number: u64) -> Self {
        self.frame_number = number;
        self
    }

    /// Calculate FPS from frame time
    #[must_use]
    pub fn fps(&self) -> f64 {
        if self.frame_time_ms > 0.0 {
            1000.0 / self.frame_time_ms
        } else {
            0.0
        }
    }

    /// Check if frame meets target FPS
    #[must_use]
    pub fn meets_target(&self, target_fps: f64) -> bool {
        self.fps() >= target_fps
    }
}

/// Memory usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Heap bytes used
    pub heap_used: u64,
    /// Total heap size
    pub heap_total: u64,
    /// Peak memory usage
    pub peak_usage: u64,
}

impl MemoryMetrics {
    /// Create new memory metrics
    #[must_use]
    pub fn new(heap_used: u64, heap_total: u64) -> Self {
        Self {
            heap_used,
            heap_total,
            peak_usage: heap_used,
        }
    }

    /// Calculate usage percentage
    #[must_use]
    pub fn usage_percent(&self) -> f64 {
        if self.heap_total > 0 {
            (self.heap_used as f64 / self.heap_total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Format heap used for display
    #[must_use]
    pub fn heap_used_formatted(&self) -> String {
        format_bytes(self.heap_used)
    }
}

/// Aggregate performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Frame time statistics (ms)
    pub frame_times: Statistics,
    /// Memory usage snapshots
    pub memory: Option<MemoryMetrics>,
    /// Function timing by name
    pub function_times: std::collections::HashMap<String, Statistics>,
    /// Total measurement duration
    pub duration: Duration,
}

impl PerformanceMetrics {
    /// Create from a trace
    #[must_use]
    pub fn from_trace(trace: &super::trace::Trace) -> Self {
        let mut function_times = std::collections::HashMap::new();

        // Group spans by name
        let mut by_name: std::collections::HashMap<&str, Vec<f64>> = std::collections::HashMap::new();
        for span in &trace.spans {
            if let Some(dur_ns) = span.duration_ns() {
                by_name
                    .entry(&span.name)
                    .or_default()
                    .push(dur_ns as f64 / 1_000_000.0);
            }
        }

        // Calculate statistics for each
        for (name, values) in by_name {
            function_times.insert(name.to_string(), Statistics::from_values(&values));
        }

        Self {
            frame_times: Statistics::empty(),
            memory: None,
            function_times,
            duration: trace.duration.unwrap_or_default(),
        }
    }

    /// Check if all metrics within budget
    #[must_use]
    pub fn within_budget(&self, frame_budget_ms: f64) -> bool {
        self.frame_times.within_budget(frame_budget_ms)
    }
}

/// Format bytes for display
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_from_values() {
        let stats = Statistics::from_values(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        assert!((stats.min - 1.0).abs() < f64::EPSILON);
        assert!((stats.max - 5.0).abs() < f64::EPSILON);
        assert!((stats.mean - 3.0).abs() < f64::EPSILON);
        assert!((stats.median - 3.0).abs() < f64::EPSILON);
        assert_eq!(stats.count, 5);
    }

    #[test]
    fn test_statistics_empty() {
        let stats = Statistics::from_values(&[]);
        assert_eq!(stats.count, 0);
        assert!((stats.mean - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_statistics_single_value() {
        let stats = Statistics::from_values(&[42.0]);
        assert!((stats.min - 42.0).abs() < f64::EPSILON);
        assert!((stats.max - 42.0).abs() < f64::EPSILON);
        assert!((stats.mean - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_statistics_within_budget() {
        let stats = Statistics::from_values(&[10.0, 12.0, 14.0, 16.0, 18.0]);
        assert!(stats.within_budget(20.0));
        assert!(!stats.within_budget(15.0));
    }

    #[test]
    fn test_frame_metrics_fps() {
        let metrics = FrameMetrics::new(16.67);
        let fps = metrics.fps();
        assert!(fps > 59.0 && fps < 61.0);
    }

    #[test]
    fn test_frame_metrics_meets_target() {
        let metrics = FrameMetrics::new(16.67);
        assert!(metrics.meets_target(60.0));
        assert!(!metrics.meets_target(120.0));
    }

    #[test]
    fn test_memory_metrics_usage_percent() {
        let metrics = MemoryMetrics::new(512, 1024);
        assert!((metrics.usage_percent() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_memory_metrics_formatted() {
        let metrics = MemoryMetrics::new(1024 * 1024, 2 * 1024 * 1024);
        assert!(metrics.heap_used_formatted().contains("MB"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
}
