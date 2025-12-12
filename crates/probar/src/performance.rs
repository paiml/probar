//! Performance Profiling (Feature 10)
//!
//! Capture and analyze performance metrics during test execution.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Mieruka**: Visual representation of performance data
//! - **Heijunka**: Smooth performance without spikes
//! - **Kaizen**: Continuous performance improvement

use crate::result::{ProbarError, ProbarResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Types of performance metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// Frame rendering time
    FrameTime,
    /// JavaScript/WASM execution time
    ScriptTime,
    /// Layout calculation time
    LayoutTime,
    /// Paint time
    PaintTime,
    /// Network request time
    NetworkTime,
    /// Memory usage
    MemoryUsage,
    /// Garbage collection time
    GcTime,
    /// First contentful paint
    FirstContentfulPaint,
    /// Largest contentful paint
    LargestContentfulPaint,
    /// Time to interactive
    TimeToInteractive,
    /// Total blocking time
    TotalBlockingTime,
    /// Cumulative layout shift
    CumulativeLayoutShift,
    /// Custom metric
    Custom,
}

/// A single performance measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    /// Metric type
    pub metric_type: MetricType,
    /// Metric name (for custom metrics)
    pub name: String,
    /// Value
    pub value: f64,
    /// Unit (ms, bytes, etc.)
    pub unit: String,
    /// Timestamp (ms since profiling start)
    pub timestamp_ms: u64,
    /// Optional context/tags
    pub tags: HashMap<String, String>,
}

impl Measurement {
    /// Create a new measurement
    #[must_use]
    pub fn new(metric_type: MetricType, name: &str, value: f64, unit: &str) -> Self {
        Self {
            metric_type,
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            timestamp_ms: 0,
            tags: HashMap::new(),
        }
    }

    /// Create a timing measurement in milliseconds
    #[must_use]
    pub fn timing(name: &str, ms: f64) -> Self {
        Self::new(MetricType::Custom, name, ms, "ms")
    }

    /// Create a memory measurement in bytes
    #[must_use]
    pub fn memory(name: &str, bytes: u64) -> Self {
        Self::new(MetricType::MemoryUsage, name, bytes as f64, "bytes")
    }

    /// Create a frame time measurement
    #[must_use]
    pub fn frame_time(ms: f64) -> Self {
        Self::new(MetricType::FrameTime, "frame_time", ms, "ms")
    }

    /// Set timestamp
    #[must_use]
    pub const fn with_timestamp(mut self, timestamp_ms: u64) -> Self {
        self.timestamp_ms = timestamp_ms;
        self
    }

    /// Add a tag
    #[must_use]
    pub fn with_tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }
}

/// Statistics for a set of measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    /// Number of samples
    pub count: usize,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Mean value
    pub mean: f64,
    /// Median value
    pub median: f64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Sum of all values
    pub sum: f64,
}

impl MetricStats {
    /// Calculate statistics from a slice of values
    #[must_use]
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                count: 0,
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                median: 0.0,
                p95: 0.0,
                p99: 0.0,
                std_dev: 0.0,
                sum: 0.0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();
        let mean = sum / count as f64;
        let min = sorted[0];
        let max = sorted[count - 1];
        let median = Self::percentile(&sorted, 50.0);
        let p95 = Self::percentile(&sorted, 95.0);
        let p99 = Self::percentile(&sorted, 99.0);

        let variance: f64 = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        Self {
            count,
            min,
            max,
            mean,
            median,
            p95,
            p99,
            std_dev,
            sum,
        }
    }

    fn percentile(sorted: &[f64], p: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let rank = (p / 100.0) * (sorted.len() - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        if lower == upper {
            sorted[lower]
        } else {
            let weight = rank - lower as f64;
            sorted[lower] * (1.0 - weight) + sorted[upper] * weight
        }
    }
}

/// Performance threshold for assertions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThreshold {
    /// Metric name
    pub name: String,
    /// Maximum allowed value
    pub max: Option<f64>,
    /// Minimum allowed value
    pub min: Option<f64>,
    /// Maximum allowed mean
    pub max_mean: Option<f64>,
    /// Maximum allowed p95
    pub max_p95: Option<f64>,
    /// Maximum allowed p99
    pub max_p99: Option<f64>,
}

impl PerformanceThreshold {
    /// Create a new threshold
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            max: None,
            min: None,
            max_mean: None,
            max_p95: None,
            max_p99: None,
        }
    }

    /// Set maximum value
    #[must_use]
    pub const fn with_max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set minimum value
    #[must_use]
    pub const fn with_min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum mean
    #[must_use]
    pub const fn with_max_mean(mut self, max_mean: f64) -> Self {
        self.max_mean = Some(max_mean);
        self
    }

    /// Set maximum p95
    #[must_use]
    pub const fn with_max_p95(mut self, max_p95: f64) -> Self {
        self.max_p95 = Some(max_p95);
        self
    }

    /// Set maximum p99
    #[must_use]
    pub const fn with_max_p99(mut self, max_p99: f64) -> Self {
        self.max_p99 = Some(max_p99);
        self
    }

    /// Check if stats pass this threshold
    pub fn check(&self, stats: &MetricStats) -> ProbarResult<()> {
        if let Some(max) = self.max {
            if stats.max > max {
                return Err(ProbarError::AssertionError {
                    message: format!(
                        "{}: max value {:.2} exceeds threshold {:.2}",
                        self.name, stats.max, max
                    ),
                });
            }
        }

        if let Some(min) = self.min {
            if stats.min < min {
                return Err(ProbarError::AssertionError {
                    message: format!(
                        "{}: min value {:.2} below threshold {:.2}",
                        self.name, stats.min, min
                    ),
                });
            }
        }

        if let Some(max_mean) = self.max_mean {
            if stats.mean > max_mean {
                return Err(ProbarError::AssertionError {
                    message: format!(
                        "{}: mean {:.2} exceeds threshold {:.2}",
                        self.name, stats.mean, max_mean
                    ),
                });
            }
        }

        if let Some(max_p95) = self.max_p95 {
            if stats.p95 > max_p95 {
                return Err(ProbarError::AssertionError {
                    message: format!(
                        "{}: p95 {:.2} exceeds threshold {:.2}",
                        self.name, stats.p95, max_p95
                    ),
                });
            }
        }

        if let Some(max_p99) = self.max_p99 {
            if stats.p99 > max_p99 {
                return Err(ProbarError::AssertionError {
                    message: format!(
                        "{}: p99 {:.2} exceeds threshold {:.2}",
                        self.name, stats.p99, max_p99
                    ),
                });
            }
        }

        Ok(())
    }
}

/// A performance profile containing all measurements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceProfile {
    /// All measurements grouped by name
    pub measurements: HashMap<String, Vec<Measurement>>,
    /// Profile start time
    #[serde(skip)]
    pub start_time: Option<Instant>,
    /// Profile duration
    pub duration_ms: Option<u64>,
    /// Test name
    pub test_name: String,
}

impl PerformanceProfile {
    /// Create a new profile
    #[must_use]
    pub fn new(test_name: &str) -> Self {
        Self {
            measurements: HashMap::new(),
            start_time: None,
            duration_ms: None,
            test_name: test_name.to_string(),
        }
    }

    /// Add a measurement
    pub fn add(&mut self, measurement: Measurement) {
        self.measurements
            .entry(measurement.name.clone())
            .or_default()
            .push(measurement);
    }

    /// Get statistics for a metric
    #[must_use]
    pub fn stats(&self, name: &str) -> Option<MetricStats> {
        self.measurements.get(name).map(|measurements| {
            let values: Vec<f64> = measurements.iter().map(|m| m.value).collect();
            MetricStats::from_values(&values)
        })
    }

    /// Get all metric names
    #[must_use]
    pub fn metric_names(&self) -> Vec<String> {
        self.measurements.keys().cloned().collect()
    }

    /// Get total measurement count
    #[must_use]
    pub fn measurement_count(&self) -> usize {
        self.measurements.values().map(|v| v.len()).sum()
    }

    /// Check thresholds
    pub fn check_thresholds(&self, thresholds: &[PerformanceThreshold]) -> ProbarResult<()> {
        for threshold in thresholds {
            if let Some(stats) = self.stats(&threshold.name) {
                threshold.check(&stats)?;
            }
        }
        Ok(())
    }

    /// Generate a summary report
    #[must_use]
    pub fn summary(&self) -> PerformanceSummary {
        let mut metrics = HashMap::new();
        for name in self.metric_names() {
            if let Some(stats) = self.stats(&name) {
                metrics.insert(name, stats);
            }
        }
        PerformanceSummary {
            test_name: self.test_name.clone(),
            duration_ms: self.duration_ms.unwrap_or(0),
            metrics,
        }
    }
}

/// Summary of performance profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Test name
    pub test_name: String,
    /// Total duration
    pub duration_ms: u64,
    /// Statistics per metric
    pub metrics: HashMap<String, MetricStats>,
}

/// Performance profiler for capturing metrics
#[derive(Debug)]
pub struct PerformanceProfiler {
    /// Current profile
    profile: PerformanceProfile,
    /// Profiling start time
    start_time: Instant,
    /// Whether profiling is active
    active: bool,
    /// Thresholds to check
    thresholds: Vec<PerformanceThreshold>,
    /// Open timers
    timers: HashMap<String, Instant>,
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new("")
    }
}

impl PerformanceProfiler {
    /// Create a new profiler
    #[must_use]
    pub fn new(test_name: &str) -> Self {
        Self {
            profile: PerformanceProfile::new(test_name),
            start_time: Instant::now(),
            active: false,
            thresholds: Vec::new(),
            timers: HashMap::new(),
        }
    }

    /// Start profiling
    pub fn start(&mut self) {
        self.active = true;
        self.start_time = Instant::now();
        self.profile.start_time = Some(self.start_time);
    }

    /// Stop profiling
    pub fn stop(&mut self) -> PerformanceProfile {
        self.active = false;
        self.profile.duration_ms = Some(self.start_time.elapsed().as_millis() as u64);
        self.profile.clone()
    }

    /// Check if profiling is active
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Get elapsed time in milliseconds
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Record a measurement
    pub fn record(&mut self, measurement: Measurement) {
        if self.active {
            let measurement = measurement.with_timestamp(self.elapsed_ms());
            self.profile.add(measurement);
        }
    }

    /// Record a frame time
    pub fn record_frame_time(&mut self, ms: f64) {
        self.record(Measurement::frame_time(ms));
    }

    /// Record a custom timing
    pub fn record_timing(&mut self, name: &str, ms: f64) {
        self.record(Measurement::timing(name, ms));
    }

    /// Record memory usage
    pub fn record_memory(&mut self, name: &str, bytes: u64) {
        self.record(Measurement::memory(name, bytes));
    }

    /// Start a timer
    pub fn start_timer(&mut self, name: &str) {
        self.timers.insert(name.to_string(), Instant::now());
    }

    /// Stop a timer and record the measurement
    pub fn stop_timer(&mut self, name: &str) -> Option<f64> {
        self.timers.remove(name).map(|start| {
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            self.record_timing(name, duration_ms);
            duration_ms
        })
    }

    /// Add a threshold
    pub fn add_threshold(&mut self, threshold: PerformanceThreshold) {
        self.thresholds.push(threshold);
    }

    /// Check all thresholds
    pub fn check_thresholds(&self) -> ProbarResult<()> {
        self.profile.check_thresholds(&self.thresholds)
    }

    /// Get current profile
    #[must_use]
    pub fn profile(&self) -> &PerformanceProfile {
        &self.profile
    }

    /// Get statistics for a metric
    #[must_use]
    pub fn stats(&self, name: &str) -> Option<MetricStats> {
        self.profile.stats(name)
    }

    /// Measure a closure
    pub fn measure<F, T>(&mut self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.record_timing(name, duration_ms);
        result
    }
}

/// Performance monitor for continuous monitoring
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Frame times buffer
    frame_times: Vec<f64>,
    /// Maximum buffer size
    max_buffer_size: usize,
    /// Target frame time (ms)
    target_frame_time: f64,
    /// Warning threshold (percentage above target)
    warning_threshold: f64,
    /// Frame drop count
    frame_drops: u64,
    /// Last frame time
    last_frame_time: Option<Instant>,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// Create a new monitor targeting 60 FPS
    #[must_use]
    pub fn new() -> Self {
        Self {
            frame_times: Vec::new(),
            max_buffer_size: 1000,
            target_frame_time: 16.67, // 60 FPS
            warning_threshold: 0.5,   // 50% above target
            frame_drops: 0,
            last_frame_time: None,
        }
    }

    /// Set target frame rate
    #[must_use]
    pub fn with_target_fps(mut self, fps: u32) -> Self {
        self.target_frame_time = 1000.0 / fps as f64;
        self
    }

    /// Set warning threshold
    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    /// Record a frame
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_frame_time {
            let frame_time = now.duration_since(last).as_secs_f64() * 1000.0;
            self.frame_times.push(frame_time);

            // Check for frame drop
            if frame_time > self.target_frame_time * 2.0 {
                self.frame_drops += 1;
            }

            // Keep buffer bounded
            if self.frame_times.len() > self.max_buffer_size {
                self.frame_times.remove(0);
            }
        }
        self.last_frame_time = Some(now);
    }

    /// Record a frame with explicit time
    pub fn record_frame_time(&mut self, ms: f64) {
        self.frame_times.push(ms);

        if ms > self.target_frame_time * 2.0 {
            self.frame_drops += 1;
        }

        if self.frame_times.len() > self.max_buffer_size {
            self.frame_times.remove(0);
        }
    }

    /// Get current FPS
    #[must_use]
    pub fn current_fps(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let avg_frame_time: f64 =
            self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
        if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Get frame time statistics
    #[must_use]
    pub fn frame_time_stats(&self) -> MetricStats {
        MetricStats::from_values(&self.frame_times)
    }

    /// Get frame drop count
    #[must_use]
    pub const fn frame_drops(&self) -> u64 {
        self.frame_drops
    }

    /// Get frame count
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frame_times.len()
    }

    /// Check if performance is within target
    #[must_use]
    pub fn is_within_target(&self) -> bool {
        if self.frame_times.is_empty() {
            return true;
        }
        let stats = self.frame_time_stats();
        stats.mean <= self.target_frame_time * (1.0 + self.warning_threshold)
    }

    /// Assert performance meets target
    pub fn assert_performance(&self) -> ProbarResult<()> {
        if self.frame_times.is_empty() {
            return Ok(());
        }

        let stats = self.frame_time_stats();
        let threshold = self.target_frame_time * (1.0 + self.warning_threshold);

        if stats.mean > threshold {
            return Err(ProbarError::AssertionError {
                message: format!(
                    "Mean frame time {:.2}ms exceeds threshold {:.2}ms (target: {:.2}ms @ {:.0} FPS)",
                    stats.mean,
                    threshold,
                    self.target_frame_time,
                    1000.0 / self.target_frame_time
                ),
            });
        }

        Ok(())
    }

    /// Reset the monitor
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.frame_drops = 0;
        self.last_frame_time = None;
    }
}

/// Builder for performance profiler
#[derive(Debug, Default)]
pub struct PerformanceProfilerBuilder {
    test_name: String,
    thresholds: Vec<PerformanceThreshold>,
}

impl PerformanceProfilerBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            thresholds: Vec::new(),
        }
    }

    /// Add a threshold
    #[must_use]
    pub fn threshold(mut self, threshold: PerformanceThreshold) -> Self {
        self.thresholds.push(threshold);
        self
    }

    /// Add a max frame time threshold
    #[must_use]
    pub fn max_frame_time(self, max_ms: f64) -> Self {
        self.threshold(PerformanceThreshold::new("frame_time").with_max(max_ms))
    }

    /// Add a mean frame time threshold
    #[must_use]
    pub fn mean_frame_time(self, max_mean_ms: f64) -> Self {
        self.threshold(PerformanceThreshold::new("frame_time").with_max_mean(max_mean_ms))
    }

    /// Build the profiler
    #[must_use]
    pub fn build(self) -> PerformanceProfiler {
        let mut profiler = PerformanceProfiler::new(&self.test_name);
        for threshold in self.thresholds {
            profiler.add_threshold(threshold);
        }
        profiler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod measurement_tests {
        use super::*;

        #[test]
        fn test_new() {
            let m = Measurement::new(MetricType::Custom, "test", 42.0, "ms");
            assert_eq!(m.name, "test");
            assert!((m.value - 42.0).abs() < f64::EPSILON);
            assert_eq!(m.unit, "ms");
        }

        #[test]
        fn test_timing() {
            let m = Measurement::timing("render", 16.5);
            assert_eq!(m.name, "render");
            assert_eq!(m.unit, "ms");
        }

        #[test]
        fn test_memory() {
            let m = Measurement::memory("heap", 1024);
            assert!(matches!(m.metric_type, MetricType::MemoryUsage));
            assert_eq!(m.unit, "bytes");
        }

        #[test]
        fn test_frame_time() {
            let m = Measurement::frame_time(16.67);
            assert!(matches!(m.metric_type, MetricType::FrameTime));
        }

        #[test]
        fn test_with_timestamp() {
            let m = Measurement::timing("test", 10.0).with_timestamp(1000);
            assert_eq!(m.timestamp_ms, 1000);
        }

        #[test]
        fn test_with_tag() {
            let m = Measurement::timing("test", 10.0).with_tag("component", "renderer");
            assert_eq!(m.tags.get("component"), Some(&"renderer".to_string()));
        }
    }

    mod metric_stats_tests {
        use super::*;

        #[test]
        fn test_empty() {
            let stats = MetricStats::from_values(&[]);
            assert_eq!(stats.count, 0);
            assert!((stats.mean - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_single_value() {
            let stats = MetricStats::from_values(&[42.0]);
            assert_eq!(stats.count, 1);
            assert!((stats.min - 42.0).abs() < f64::EPSILON);
            assert!((stats.max - 42.0).abs() < f64::EPSILON);
            assert!((stats.mean - 42.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_multiple_values() {
            let stats = MetricStats::from_values(&[1.0, 2.0, 3.0, 4.0, 5.0]);
            assert_eq!(stats.count, 5);
            assert!((stats.min - 1.0).abs() < f64::EPSILON);
            assert!((stats.max - 5.0).abs() < f64::EPSILON);
            assert!((stats.mean - 3.0).abs() < f64::EPSILON);
            assert!((stats.median - 3.0).abs() < f64::EPSILON);
            assert!((stats.sum - 15.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_percentiles() {
            let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
            let stats = MetricStats::from_values(&values);
            assert!((stats.p95 - 95.0).abs() < 1.0);
            assert!((stats.p99 - 99.0).abs() < 1.0);
        }

        #[test]
        fn test_std_dev() {
            let stats = MetricStats::from_values(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
            // Mean = 5, variance should be 4, std_dev should be 2
            assert!((stats.std_dev - 2.0).abs() < 0.1);
        }
    }

    mod performance_threshold_tests {
        use super::*;

        #[test]
        fn test_new() {
            let t = PerformanceThreshold::new("frame_time");
            assert_eq!(t.name, "frame_time");
            assert!(t.max.is_none());
        }

        #[test]
        fn test_with_max() {
            let t = PerformanceThreshold::new("frame_time").with_max(16.67);
            assert_eq!(t.max, Some(16.67));
        }

        #[test]
        fn test_check_passes() {
            let t = PerformanceThreshold::new("test")
                .with_max(100.0)
                .with_max_mean(50.0);

            let stats = MetricStats::from_values(&[10.0, 20.0, 30.0]);
            assert!(t.check(&stats).is_ok());
        }

        #[test]
        fn test_check_max_fails() {
            let t = PerformanceThreshold::new("test").with_max(20.0);
            let stats = MetricStats::from_values(&[10.0, 30.0]);
            assert!(t.check(&stats).is_err());
        }

        #[test]
        fn test_check_mean_fails() {
            let t = PerformanceThreshold::new("test").with_max_mean(15.0);
            let stats = MetricStats::from_values(&[20.0, 20.0, 20.0]);
            assert!(t.check(&stats).is_err());
        }

        #[test]
        fn test_check_p95_fails() {
            let t = PerformanceThreshold::new("test").with_max_p95(50.0);
            let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
            let stats = MetricStats::from_values(&values);
            assert!(t.check(&stats).is_err());
        }
    }

    mod performance_profile_tests {
        use super::*;

        #[test]
        fn test_new() {
            let profile = PerformanceProfile::new("test");
            assert_eq!(profile.test_name, "test");
            assert!(profile.measurements.is_empty());
        }

        #[test]
        fn test_add() {
            let mut profile = PerformanceProfile::new("test");
            profile.add(Measurement::timing("render", 16.0));
            profile.add(Measurement::timing("render", 17.0));

            assert_eq!(profile.measurement_count(), 2);
        }

        #[test]
        fn test_stats() {
            let mut profile = PerformanceProfile::new("test");
            profile.add(Measurement::timing("render", 10.0));
            profile.add(Measurement::timing("render", 20.0));
            profile.add(Measurement::timing("render", 30.0));

            let stats = profile.stats("render").unwrap();
            assert_eq!(stats.count, 3);
            assert!((stats.mean - 20.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_metric_names() {
            let mut profile = PerformanceProfile::new("test");
            profile.add(Measurement::timing("render", 10.0));
            profile.add(Measurement::timing("update", 5.0));

            let names = profile.metric_names();
            assert_eq!(names.len(), 2);
        }

        #[test]
        fn test_check_thresholds() {
            let mut profile = PerformanceProfile::new("test");
            profile.add(Measurement::timing("render", 10.0));

            let thresholds = vec![PerformanceThreshold::new("render").with_max(20.0)];
            assert!(profile.check_thresholds(&thresholds).is_ok());
        }

        #[test]
        fn test_summary() {
            let mut profile = PerformanceProfile::new("test");
            profile.add(Measurement::timing("render", 10.0));
            profile.duration_ms = Some(1000);

            let summary = profile.summary();
            assert_eq!(summary.test_name, "test");
            assert_eq!(summary.duration_ms, 1000);
        }
    }

    mod performance_profiler_tests {
        use super::*;

        #[test]
        fn test_new() {
            let profiler = PerformanceProfiler::new("test");
            assert!(!profiler.is_active());
        }

        #[test]
        fn test_start_stop() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.start();
            assert!(profiler.is_active());

            let profile = profiler.stop();
            assert!(!profiler.is_active());
            assert!(profile.duration_ms.is_some());
        }

        #[test]
        fn test_record() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.start();
            profiler.record_frame_time(16.67);

            let profile = profiler.stop();
            assert_eq!(profile.measurement_count(), 1);
        }

        #[test]
        fn test_record_when_inactive() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.record_frame_time(16.67);

            assert_eq!(profiler.profile().measurement_count(), 0);
        }

        #[test]
        fn test_timer() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.start();

            profiler.start_timer("operation");
            std::thread::sleep(std::time::Duration::from_millis(10));
            let duration = profiler.stop_timer("operation");

            assert!(duration.is_some());
            assert!(duration.unwrap() >= 10.0);
        }

        #[test]
        fn test_measure() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.start();

            let result = profiler.measure("calculation", || {
                std::thread::sleep(std::time::Duration::from_millis(5));
                42
            });

            assert_eq!(result, 42);
            assert!(profiler.stats("calculation").is_some());
        }

        #[test]
        fn test_add_threshold() {
            let mut profiler = PerformanceProfiler::new("test");
            profiler.add_threshold(PerformanceThreshold::new("frame_time").with_max(20.0));
            profiler.start();
            profiler.record_frame_time(15.0);
            profiler.stop();

            assert!(profiler.check_thresholds().is_ok());
        }
    }

    mod performance_monitor_tests {
        use super::*;

        #[test]
        fn test_new() {
            let monitor = PerformanceMonitor::new();
            assert_eq!(monitor.frame_count(), 0);
            assert_eq!(monitor.frame_drops(), 0);
        }

        #[test]
        fn test_with_target_fps() {
            let monitor = PerformanceMonitor::new().with_target_fps(30);
            assert!((monitor.target_frame_time - 33.33).abs() < 0.1);
        }

        #[test]
        fn test_record_frame_time() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.0);
            monitor.record_frame_time(17.0);

            assert_eq!(monitor.frame_count(), 2);
        }

        #[test]
        fn test_current_fps() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.67);
            monitor.record_frame_time(16.67);

            let fps = monitor.current_fps();
            assert!((fps - 60.0).abs() < 1.0);
        }

        #[test]
        fn test_frame_drops() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.0); // Normal
            monitor.record_frame_time(50.0); // Frame drop (> 2x target)

            assert_eq!(monitor.frame_drops(), 1);
        }

        #[test]
        fn test_is_within_target() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.0);
            monitor.record_frame_time(17.0);

            assert!(monitor.is_within_target());
        }

        #[test]
        fn test_is_not_within_target() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(30.0);
            monitor.record_frame_time(35.0);

            assert!(!monitor.is_within_target());
        }

        #[test]
        fn test_assert_performance() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.0);
            monitor.record_frame_time(17.0);

            assert!(monitor.assert_performance().is_ok());
        }

        #[test]
        fn test_reset() {
            let mut monitor = PerformanceMonitor::new();
            monitor.record_frame_time(16.0);
            monitor.record_frame_time(50.0);

            monitor.reset();

            assert_eq!(monitor.frame_count(), 0);
            assert_eq!(monitor.frame_drops(), 0);
        }
    }

    mod performance_profiler_builder_tests {
        use super::*;

        #[test]
        fn test_builder() {
            let profiler = PerformanceProfilerBuilder::new("test")
                .max_frame_time(33.33)
                .mean_frame_time(16.67)
                .build();

            assert_eq!(profiler.thresholds.len(), 2);
        }
    }
}
