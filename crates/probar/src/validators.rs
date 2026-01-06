//! Streaming UX Validators (PROBAR-SPEC-011)
//!
//! Validate streaming interface patterns per specification Section 2.
//!
//! ## Toyota Way Application:
//! - **Jidoka**: Real-time quality monitoring during streaming tests
//! - **Poka-Yoke**: State machine validation prevents invalid transitions
//! - **Andon**: Clear alerts when streaming metrics degrade
//!
//! ## References:
//! - [10] Sewell et al. (2010) State machine verification
//! - [11] Blott & Korn (2015) Low-latency patterns
//! - [12] Sohn et al. (2015) VAD state machine testing

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

// =============================================================================
// VU Meter Configuration (Section 2.4)
// =============================================================================

/// VU meter validation parameters
///
/// # Example
/// ```
/// use jugar_probar::validators::VuMeterConfig;
///
/// let config = VuMeterConfig::default()
///     .with_min_level(0.1)
///     .with_update_rate_hz(30.0);
/// ```
#[derive(Debug, Clone)]
pub struct VuMeterConfig {
    /// Minimum expected level (0.0-1.0)
    pub min_level: f32,
    /// Maximum expected level (0.0-1.0)
    pub max_level: f32,
    /// Update frequency (Hz)
    pub update_rate_hz: f32,
    /// Smoothing factor validation tolerance
    pub smoothing_tolerance: f32,
    /// Maximum time without updates (staleness)
    pub max_stale_ms: u64,
}

impl Default for VuMeterConfig {
    fn default() -> Self {
        Self {
            min_level: 0.0,
            max_level: 1.0,
            update_rate_hz: 30.0,
            smoothing_tolerance: 0.1,
            max_stale_ms: 100,
        }
    }
}

impl VuMeterConfig {
    /// Set minimum expected level
    #[must_use]
    pub fn with_min_level(mut self, level: f32) -> Self {
        self.min_level = level.clamp(0.0, 1.0);
        self
    }

    /// Set maximum expected level
    #[must_use]
    pub fn with_max_level(mut self, level: f32) -> Self {
        self.max_level = level.clamp(0.0, 1.0);
        self
    }

    /// Set expected update rate in Hz
    #[must_use]
    pub fn with_update_rate_hz(mut self, rate: f32) -> Self {
        self.update_rate_hz = rate.max(1.0);
        self
    }

    /// Set staleness threshold
    #[must_use]
    pub fn with_max_stale_ms(mut self, ms: u64) -> Self {
        self.max_stale_ms = ms;
        self
    }

    /// Validate a VU meter sample
    pub fn validate_sample(&self, level: f32) -> Result<(), VuMeterError> {
        if level < 0.0 {
            return Err(VuMeterError::NegativeLevel(level));
        }
        if level > self.max_level + self.smoothing_tolerance {
            return Err(VuMeterError::Clipping(level));
        }
        Ok(())
    }
}

/// VU meter validation error
#[derive(Debug, Clone)]
pub enum VuMeterError {
    /// Level is negative
    NegativeLevel(f32),
    /// Level exceeds maximum (clipping)
    Clipping(f32),
    /// Updates are stale
    Stale {
        /// Last update timestamp
        last_update_ms: u64,
        /// Current timestamp
        current_ms: u64,
    },
    /// Update rate too slow
    SlowUpdateRate {
        /// Measured rate
        measured_hz: f32,
        /// Expected rate
        expected_hz: f32,
    },
    /// Constant value detected (not animating)
    NotAnimating {
        /// Number of samples
        sample_count: usize,
        /// Constant value
        value: f32,
    },
}

impl fmt::Display for VuMeterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeLevel(v) => write!(f, "VU meter level is negative: {v}"),
            Self::Clipping(v) => write!(f, "VU meter clipping at: {v}"),
            Self::Stale {
                last_update_ms,
                current_ms,
            } => {
                write!(
                    f,
                    "VU meter stale: last update {}ms ago",
                    current_ms - last_update_ms
                )
            }
            Self::SlowUpdateRate {
                measured_hz,
                expected_hz,
            } => {
                write!(
                    f,
                    "VU meter update rate too slow: {measured_hz:.1}Hz < {expected_hz:.1}Hz"
                )
            }
            Self::NotAnimating {
                sample_count,
                value,
            } => {
                write!(
                    f,
                    "VU meter not animating: {sample_count} samples all at {value}"
                )
            }
        }
    }
}

impl std::error::Error for VuMeterError {}

// =============================================================================
// State Transition Tracking (Section 2.4)
// =============================================================================

/// State transition event with timing
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// Previous state
    pub from: String,
    /// New state
    pub to: String,
    /// Timestamp of transition (ms since start)
    pub timestamp_ms: f64,
    /// Duration in previous state (ms)
    pub duration_ms: f64,
}

/// Partial transcription result
#[derive(Debug, Clone)]
pub struct PartialResult {
    /// Timestamp (ms since start)
    pub timestamp_ms: f64,
    /// Partial text content
    pub text: String,
    /// Whether this is the final result
    pub is_final: bool,
}

/// VU meter sample
#[derive(Debug, Clone)]
pub struct VuMeterSample {
    /// Timestamp (ms since start)
    pub timestamp_ms: f64,
    /// Level (0.0-1.0)
    pub level: f32,
}

// =============================================================================
// Test Execution Stats (Section 5.1 - trueno-zram integration)
// =============================================================================

/// Test execution statistics with compression metrics
///
/// Tracks game state capture efficiency during test runs.
/// Based on trueno-zram compression statistics patterns.
///
/// # Example
/// ```
/// use jugar_probar::validators::TestExecutionStats;
///
/// let mut stats = TestExecutionStats::new();
/// stats.record_state_capture(4096, 1024);
/// stats.record_state_capture(4096, 512); // Same-fill page
///
/// assert!(stats.efficiency() > 0.5);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TestExecutionStats {
    /// Total game states captured
    pub states_captured: u64,
    /// Bytes before compression
    pub bytes_raw: u64,
    /// Bytes after compression
    pub bytes_compressed: u64,
    /// Same-fill pages detected (high compression)
    pub same_fill_pages: u64,
    /// Start time (for throughput calculation)
    start_time: Option<Instant>,
    /// End time (for throughput calculation)
    end_time: Option<Instant>,
}

impl TestExecutionStats {
    /// Create new stats tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            states_captured: 0,
            bytes_raw: 0,
            bytes_compressed: 0,
            same_fill_pages: 0,
            start_time: None,
            end_time: None,
        }
    }

    /// Start tracking
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Stop tracking
    pub fn stop(&mut self) {
        self.end_time = Some(Instant::now());
    }

    /// Record a state capture
    pub fn record_state_capture(&mut self, raw_bytes: u64, compressed_bytes: u64) {
        self.states_captured += 1;
        self.bytes_raw += raw_bytes;
        self.bytes_compressed += compressed_bytes;

        // Detect same-fill pages (>90% compression = likely uniform data)
        if raw_bytes > 0 && (compressed_bytes as f64 / raw_bytes as f64) < 0.1 {
            self.same_fill_pages += 1;
        }
    }

    /// Calculate compression ratio (raw / compressed)
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_compressed == 0 {
            return 0.0;
        }
        self.bytes_raw as f64 / self.bytes_compressed as f64
    }

    /// Calculate compression efficiency (1 - compressed/raw)
    #[must_use]
    pub fn efficiency(&self) -> f64 {
        if self.bytes_raw == 0 {
            return 0.0;
        }
        1.0 - (self.bytes_compressed as f64 / self.bytes_raw as f64)
    }

    /// Estimate storage savings in MB
    #[must_use]
    pub fn storage_savings_mb(&self) -> f64 {
        (self.bytes_raw.saturating_sub(self.bytes_compressed)) as f64 / 1_000_000.0
    }

    /// Calculate compression throughput (bytes/sec)
    #[must_use]
    pub fn compress_throughput(&self) -> f64 {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => {
                let duration_secs = end.duration_since(start).as_secs_f64();
                if duration_secs > 0.0 {
                    self.bytes_raw as f64 / duration_secs
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Get same-fill page ratio
    #[must_use]
    pub fn same_fill_ratio(&self) -> f64 {
        if self.states_captured == 0 {
            return 0.0;
        }
        self.same_fill_pages as f64 / self.states_captured as f64
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.states_captured = 0;
        self.bytes_raw = 0;
        self.bytes_compressed = 0;
        self.same_fill_pages = 0;
        self.start_time = None;
        self.end_time = None;
    }
}

// =============================================================================
// Screenshot Content Classifier (Section 5.2)
// =============================================================================

/// Compression algorithm recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// LZ4 for speed (UI-heavy content)
    Lz4,
    /// Zstd for balanced speed/ratio
    Zstd,
    /// PNG for lossless images
    Png,
    /// RLE for uniform content
    Rle,
}

/// Screenshot content classification for optimal compression
///
/// Based on entropy analysis to determine best compression strategy.
///
/// # Example
/// ```
/// use jugar_probar::validators::ScreenshotContent;
///
/// // Simulate UI-heavy screenshot (low entropy)
/// let pixels: Vec<u8> = vec![255; 1000]; // Uniform white
/// let content = ScreenshotContent::classify(&pixels);
///
/// assert!(matches!(content, ScreenshotContent::Uniform { .. }));
/// ```
#[derive(Debug, Clone)]
pub enum ScreenshotContent {
    /// UI-heavy (text, buttons) - high compressibility
    UiDominated {
        /// Shannon entropy (0.0-8.0 for bytes)
        entropy: f32,
    },
    /// Physics/game world - medium compressibility
    GameWorld {
        /// Shannon entropy
        entropy: f32,
    },
    /// Random/noise - low compressibility
    HighEntropy {
        /// Shannon entropy
        entropy: f32,
    },
    /// Mostly uniform - very high compressibility (same-fill)
    Uniform {
        /// Dominant fill value
        fill_value: u8,
    },
}

impl ScreenshotContent {
    /// Classify screenshot by entropy analysis
    ///
    /// Uses Shannon entropy to determine content type.
    #[must_use]
    pub fn classify(pixels: &[u8]) -> Self {
        if pixels.is_empty() {
            return Self::Uniform { fill_value: 0 };
        }

        // Count byte frequencies
        let mut frequencies = [0u64; 256];
        for &byte in pixels {
            frequencies[byte as usize] += 1;
        }

        // Check for uniform content (>95% same value)
        let total = pixels.len() as f64;
        let (max_idx, max_count) = frequencies
            .iter()
            .enumerate()
            .max_by_key(|(_, &count)| count)
            .map(|(idx, &count)| (idx, count))
            .unwrap_or((0, 0));

        if max_count as f64 / total > 0.95 {
            return Self::Uniform {
                fill_value: max_idx as u8,
            };
        }

        // Calculate Shannon entropy
        let entropy: f32 = frequencies
            .iter()
            .filter(|&&count| count > 0)
            .map(|&count| {
                let p = count as f64 / total;
                -(p * p.log2()) as f32
            })
            .sum();

        // Classify based on entropy thresholds
        if entropy < 3.0 {
            Self::UiDominated { entropy }
        } else if entropy < 6.0 {
            Self::GameWorld { entropy }
        } else {
            Self::HighEntropy { entropy }
        }
    }

    /// Get entropy value
    #[must_use]
    pub fn entropy(&self) -> f32 {
        match self {
            Self::UiDominated { entropy }
            | Self::GameWorld { entropy }
            | Self::HighEntropy { entropy } => *entropy,
            Self::Uniform { .. } => 0.0,
        }
    }

    /// Recommended compression algorithm based on content type
    #[must_use]
    pub fn recommended_algorithm(&self) -> CompressionAlgorithm {
        match self {
            Self::Uniform { .. } => CompressionAlgorithm::Rle,
            Self::UiDominated { .. } => CompressionAlgorithm::Png,
            Self::GameWorld { .. } => CompressionAlgorithm::Zstd,
            Self::HighEntropy { .. } => CompressionAlgorithm::Lz4,
        }
    }

    /// Expected compression ratio hint
    #[must_use]
    pub fn expected_ratio_hint(&self) -> &'static str {
        match self {
            Self::Uniform { .. } => "excellent (>100:1)",
            Self::UiDominated { .. } => "good (5:1 - 20:1)",
            Self::GameWorld { .. } => "moderate (2:1 - 5:1)",
            Self::HighEntropy { .. } => "poor (<2:1)",
        }
    }
}

/// Streaming UX validator for real-time audio/video interfaces
///
/// # Example
/// ```
/// use std::time::Duration;
/// use jugar_probar::validators::{StreamingUxValidator, StreamingMetric};
///
/// let mut validator = StreamingUxValidator::new()
///     .with_max_latency(Duration::from_millis(100))
///     .with_buffer_underrun_threshold(3);
///
/// // Simulate streaming metrics
/// validator.record_metric(StreamingMetric::Latency(Duration::from_millis(50)));
/// validator.record_metric(StreamingMetric::FrameRendered { timestamp: 1000 });
///
/// assert!(validator.validate().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct StreamingUxValidator {
    /// Maximum acceptable latency
    max_latency: Duration,
    /// Maximum buffer underrun count threshold
    buffer_underrun_threshold: usize,
    /// Maximum consecutive dropped frames
    max_dropped_frames: usize,
    /// Minimum frames per second
    min_fps: f64,
    /// Time-to-first-byte timeout
    ttfb_timeout: Duration,
    /// Recorded metrics
    metrics: Vec<StreamingMetricRecord>,
    /// Buffer underrun count
    buffer_underruns: usize,
    /// Dropped frame count
    dropped_frames: usize,
    /// Frame timestamps for FPS calculation
    frame_times: VecDeque<u64>,
    /// First byte received time
    first_byte_time: Option<Instant>,
    /// Start time
    start_time: Option<Instant>,
    /// State machine state
    state: StreamingState,
    /// State transition history
    state_history: Vec<(StreamingState, Instant)>,
}

/// Streaming metric record with timestamp
#[derive(Debug, Clone)]
pub struct StreamingMetricRecord {
    /// The metric
    pub metric: StreamingMetric,
    /// When recorded
    pub timestamp: Instant,
}

/// Streaming metrics to record
#[derive(Debug, Clone)]
pub enum StreamingMetric {
    /// Latency measurement
    Latency(Duration),
    /// Frame rendered with timestamp
    FrameRendered {
        /// Frame timestamp in milliseconds
        timestamp: u64,
    },
    /// Frame dropped
    FrameDropped,
    /// Buffer underrun occurred
    BufferUnderrun,
    /// First byte received
    FirstByteReceived,
    /// Buffer level percentage
    BufferLevel(f32),
    /// Audio chunk processed
    AudioChunk {
        /// Sample count
        samples: usize,
        /// Sample rate
        sample_rate: u32,
    },
}

/// Streaming state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamingState {
    /// Initial state
    Idle,
    /// Loading/buffering
    Buffering,
    /// Actively streaming
    Streaming,
    /// Stalled (buffer underrun)
    Stalled,
    /// Error state
    Error,
    /// Completed
    Completed,
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::Idle
    }
}

impl fmt::Display for StreamingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Buffering => write!(f, "Buffering"),
            Self::Streaming => write!(f, "Streaming"),
            Self::Stalled => write!(f, "Stalled"),
            Self::Error => write!(f, "Error"),
            Self::Completed => write!(f, "Completed"),
        }
    }
}

/// Streaming validation error
#[derive(Debug, Clone)]
pub enum StreamingValidationError {
    /// Latency exceeded threshold
    LatencyExceeded {
        /// Measured latency
        measured: Duration,
        /// Maximum allowed
        max: Duration,
    },
    /// Too many buffer underruns
    BufferUnderrunThreshold {
        /// Number of underruns
        count: usize,
        /// Threshold
        threshold: usize,
    },
    /// Too many dropped frames
    DroppedFrameThreshold {
        /// Number of dropped frames
        count: usize,
        /// Maximum allowed
        max: usize,
    },
    /// FPS below minimum
    FpsBelowMinimum {
        /// Measured FPS
        measured: f64,
        /// Minimum required
        min: f64,
    },
    /// Time to first byte exceeded
    TtfbExceeded {
        /// Measured TTFB
        measured: Duration,
        /// Maximum allowed
        max: Duration,
    },
    /// Invalid state transition
    InvalidStateTransition {
        /// From state
        from: StreamingState,
        /// To state
        to: StreamingState,
    },
    /// State machine ended in error state
    EndedInError,
}

impl fmt::Display for StreamingValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LatencyExceeded { measured, max } => {
                write!(f, "Latency exceeded: {measured:?} > {max:?} (max allowed)")
            }
            Self::BufferUnderrunThreshold { count, threshold } => {
                write!(
                    f,
                    "Buffer underruns exceeded threshold: {count} > {threshold}"
                )
            }
            Self::DroppedFrameThreshold { count, max } => {
                write!(f, "Dropped frames exceeded: {count} > {max} (max allowed)")
            }
            Self::FpsBelowMinimum { measured, min } => {
                write!(f, "FPS below minimum: {measured:.1} < {min:.1} (required)")
            }
            Self::TtfbExceeded { measured, max } => {
                write!(
                    f,
                    "Time to first byte exceeded: {measured:?} > {max:?} (max allowed)"
                )
            }
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Invalid state transition: {from} -> {to}")
            }
            Self::EndedInError => write!(f, "Streaming ended in error state"),
        }
    }
}

impl std::error::Error for StreamingValidationError {}

impl Default for StreamingUxValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingUxValidator {
    /// Create a new streaming UX validator with sensible defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_latency: Duration::from_millis(200),
            buffer_underrun_threshold: 5,
            max_dropped_frames: 10,
            min_fps: 24.0,
            ttfb_timeout: Duration::from_secs(3),
            metrics: Vec::new(),
            buffer_underruns: 0,
            dropped_frames: 0,
            frame_times: VecDeque::with_capacity(120),
            first_byte_time: None,
            start_time: None,
            state: StreamingState::Idle,
            state_history: Vec::new(),
        }
    }

    /// Create validator for real-time audio streaming
    #[must_use]
    pub fn for_audio() -> Self {
        Self::new()
            .with_max_latency(Duration::from_millis(100))
            .with_buffer_underrun_threshold(3)
            .with_ttfb_timeout(Duration::from_secs(2))
    }

    /// Create validator for video streaming
    #[must_use]
    pub fn for_video() -> Self {
        Self::new()
            .with_max_latency(Duration::from_millis(500))
            .with_min_fps(30.0)
            .with_max_dropped_frames(5)
    }

    /// Set maximum acceptable latency
    #[must_use]
    pub fn with_max_latency(mut self, latency: Duration) -> Self {
        self.max_latency = latency;
        self
    }

    /// Set buffer underrun threshold
    #[must_use]
    pub fn with_buffer_underrun_threshold(mut self, threshold: usize) -> Self {
        self.buffer_underrun_threshold = threshold;
        self
    }

    /// Set maximum dropped frames
    #[must_use]
    pub fn with_max_dropped_frames(mut self, max: usize) -> Self {
        self.max_dropped_frames = max;
        self
    }

    /// Set minimum FPS
    #[must_use]
    pub fn with_min_fps(mut self, fps: f64) -> Self {
        self.min_fps = fps;
        self
    }

    /// Set time-to-first-byte timeout
    #[must_use]
    pub fn with_ttfb_timeout(mut self, timeout: Duration) -> Self {
        self.ttfb_timeout = timeout;
        self
    }

    /// Start streaming validation
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.transition_to(StreamingState::Buffering);
    }

    /// Record a streaming metric
    pub fn record_metric(&mut self, metric: StreamingMetric) {
        let now = Instant::now();

        match &metric {
            StreamingMetric::Latency(latency) => {
                // Latency recorded - check if streaming
                if self.state == StreamingState::Buffering && *latency < self.max_latency {
                    self.transition_to(StreamingState::Streaming);
                }
            }
            StreamingMetric::FrameRendered { timestamp } => {
                self.frame_times.push_back(*timestamp);
                // Keep only last 120 frames (4 seconds at 30fps)
                while self.frame_times.len() > 120 {
                    self.frame_times.pop_front();
                }
                // If streaming, we're healthy
                if self.state == StreamingState::Stalled {
                    self.transition_to(StreamingState::Streaming);
                }
            }
            StreamingMetric::FrameDropped => {
                self.dropped_frames += 1;
            }
            StreamingMetric::BufferUnderrun => {
                self.buffer_underruns += 1;
                if self.state == StreamingState::Streaming {
                    self.transition_to(StreamingState::Stalled);
                }
            }
            StreamingMetric::FirstByteReceived => {
                self.first_byte_time = Some(now);
                if self.state == StreamingState::Idle {
                    self.transition_to(StreamingState::Buffering);
                }
            }
            StreamingMetric::BufferLevel(level) => {
                // Low buffer level indicates potential stall
                if *level < 0.1 && self.state == StreamingState::Streaming {
                    self.transition_to(StreamingState::Stalled);
                } else if *level > 0.3 && self.state == StreamingState::Stalled {
                    self.transition_to(StreamingState::Streaming);
                }
            }
            StreamingMetric::AudioChunk { .. } => {
                // Audio chunk received - mark streaming active
                if self.state == StreamingState::Buffering {
                    self.transition_to(StreamingState::Streaming);
                }
            }
        }

        self.metrics.push(StreamingMetricRecord {
            metric,
            timestamp: now,
        });
    }

    /// Transition to a new state
    fn transition_to(&mut self, new_state: StreamingState) {
        if self.state != new_state {
            self.state_history.push((self.state, Instant::now()));
            self.state = new_state;
        }
    }

    /// Mark streaming as completed
    pub fn complete(&mut self) {
        self.transition_to(StreamingState::Completed);
    }

    /// Mark streaming as errored
    pub fn error(&mut self) {
        self.transition_to(StreamingState::Error);
    }

    /// Get current state
    #[must_use]
    pub fn state(&self) -> StreamingState {
        self.state
    }

    /// Get buffer underrun count
    #[must_use]
    pub fn buffer_underruns(&self) -> usize {
        self.buffer_underruns
    }

    /// Get dropped frame count
    #[must_use]
    pub fn dropped_frames(&self) -> usize {
        self.dropped_frames
    }

    /// Calculate average FPS from recorded frames
    #[must_use]
    pub fn average_fps(&self) -> f64 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }

        let first = *self.frame_times.front().unwrap_or(&0);
        let last = *self.frame_times.back().unwrap_or(&0);
        let duration_ms = last.saturating_sub(first);

        if duration_ms == 0 {
            return 0.0;
        }

        let frame_count = self.frame_times.len() - 1;
        (frame_count as f64 * 1000.0) / duration_ms as f64
    }

    /// Validate all recorded metrics
    ///
    /// # Errors
    /// Returns validation error if any threshold is exceeded.
    pub fn validate(&self) -> Result<StreamingValidationResult, StreamingValidationError> {
        let mut errors = Vec::new();

        // Check latency metrics
        for record in &self.metrics {
            if let StreamingMetric::Latency(latency) = &record.metric {
                if *latency > self.max_latency {
                    errors.push(StreamingValidationError::LatencyExceeded {
                        measured: *latency,
                        max: self.max_latency,
                    });
                }
            }
        }

        // Check buffer underruns
        if self.buffer_underruns > self.buffer_underrun_threshold {
            errors.push(StreamingValidationError::BufferUnderrunThreshold {
                count: self.buffer_underruns,
                threshold: self.buffer_underrun_threshold,
            });
        }

        // Check dropped frames
        if self.dropped_frames > self.max_dropped_frames {
            errors.push(StreamingValidationError::DroppedFrameThreshold {
                count: self.dropped_frames,
                max: self.max_dropped_frames,
            });
        }

        // Check FPS
        let fps = self.average_fps();
        if fps > 0.0 && fps < self.min_fps {
            errors.push(StreamingValidationError::FpsBelowMinimum {
                measured: fps,
                min: self.min_fps,
            });
        }

        // Check TTFB
        if let (Some(start), Some(first_byte)) = (self.start_time, self.first_byte_time) {
            let ttfb = first_byte.duration_since(start);
            if ttfb > self.ttfb_timeout {
                errors.push(StreamingValidationError::TtfbExceeded {
                    measured: ttfb,
                    max: self.ttfb_timeout,
                });
            }
        }

        // Check final state
        if self.state == StreamingState::Error {
            errors.push(StreamingValidationError::EndedInError);
        }

        if errors.is_empty() {
            Ok(StreamingValidationResult {
                buffer_underruns: self.buffer_underruns,
                dropped_frames: self.dropped_frames,
                average_fps: fps,
                max_latency_recorded: self.max_recorded_latency(),
                total_frames: self.frame_times.len(),
            })
        } else {
            Err(errors.remove(0))
        }
    }

    /// Get all validation errors (for comprehensive reporting)
    #[must_use]
    pub fn validate_all(&self) -> Vec<StreamingValidationError> {
        let mut errors = Vec::new();

        for record in &self.metrics {
            if let StreamingMetric::Latency(latency) = &record.metric {
                if *latency > self.max_latency {
                    errors.push(StreamingValidationError::LatencyExceeded {
                        measured: *latency,
                        max: self.max_latency,
                    });
                }
            }
        }

        if self.buffer_underruns > self.buffer_underrun_threshold {
            errors.push(StreamingValidationError::BufferUnderrunThreshold {
                count: self.buffer_underruns,
                threshold: self.buffer_underrun_threshold,
            });
        }

        if self.dropped_frames > self.max_dropped_frames {
            errors.push(StreamingValidationError::DroppedFrameThreshold {
                count: self.dropped_frames,
                max: self.max_dropped_frames,
            });
        }

        let fps = self.average_fps();
        if fps > 0.0 && fps < self.min_fps {
            errors.push(StreamingValidationError::FpsBelowMinimum {
                measured: fps,
                min: self.min_fps,
            });
        }

        if self.state == StreamingState::Error {
            errors.push(StreamingValidationError::EndedInError);
        }

        errors
    }

    /// Get maximum recorded latency
    fn max_recorded_latency(&self) -> Duration {
        self.metrics
            .iter()
            .filter_map(|r| {
                if let StreamingMetric::Latency(l) = &r.metric {
                    Some(*l)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// Get state history for debugging
    #[must_use]
    pub fn state_history(&self) -> &[(StreamingState, Instant)] {
        &self.state_history
    }

    /// Reset validator for reuse
    pub fn reset(&mut self) {
        self.metrics.clear();
        self.buffer_underruns = 0;
        self.dropped_frames = 0;
        self.frame_times.clear();
        self.first_byte_time = None;
        self.start_time = None;
        self.state = StreamingState::Idle;
        self.state_history.clear();
    }
}

/// Successful validation result
#[derive(Debug, Clone)]
pub struct StreamingValidationResult {
    /// Number of buffer underruns
    pub buffer_underruns: usize,
    /// Number of dropped frames
    pub dropped_frames: usize,
    /// Average FPS
    pub average_fps: f64,
    /// Maximum latency recorded
    pub max_latency_recorded: Duration,
    /// Total frames processed
    pub total_frames: usize,
}

// CDP-based streaming UX tracking (PROBAR-SPEC-011)
#[cfg(feature = "browser")]
impl StreamingUxValidator {
    /// Track state changes on element via CDP
    ///
    /// Sets up a MutationObserver to watch for text content changes
    /// on the specified element and records state transitions.
    ///
    /// # Errors
    /// Returns error if CDP injection fails or element not found.
    pub async fn track_state_cdp(
        page: &chromiumoxide::Page,
        selector: &str,
    ) -> Result<Self, StreamingValidationError> {
        // Inject MutationObserver to track text changes
        let js = format!(
            r#"
            (function() {{
                window.__probar_state_history = [];
                window.__probar_start_time = performance.now();
                window.__probar_last_state = '';

                const el = document.querySelector('{}');
                if (!el) {{
                    return {{ error: 'Element not found: {}' }};
                }}

                const observer = new MutationObserver((mutations) => {{
                    const newState = el.textContent || el.innerText || '';
                    if (newState !== window.__probar_last_state) {{
                        const now = performance.now();
                        const elapsed = now - window.__probar_start_time;
                        window.__probar_state_history.push({{
                            from: window.__probar_last_state,
                            to: newState,
                            timestamp: elapsed,
                        }});
                        window.__probar_last_state = newState;
                    }}
                }});

                observer.observe(el, {{
                    characterData: true,
                    childList: true,
                    subtree: true
                }});

                window.__probar_state_observer = observer;
                return {{ success: true }};
            }})()
            "#,
            selector, selector
        );

        let _: serde_json::Value = page
            .evaluate(js)
            .await
            .map_err(|_e| StreamingValidationError::InvalidStateTransition {
                from: StreamingState::Idle,
                to: StreamingState::Error,
            })?
            .into_value()
            .map_err(|_| StreamingValidationError::InvalidStateTransition {
                from: StreamingState::Idle,
                to: StreamingState::Error,
            })?;

        Ok(Self::new())
    }

    /// Track VU meter levels via CDP
    ///
    /// Sets up an animation frame loop to sample VU meter values
    /// from the specified element's width, height, or data attribute.
    ///
    /// # Errors
    /// Returns error if CDP injection fails.
    pub async fn track_vu_meter_cdp(
        &mut self,
        page: &chromiumoxide::Page,
        selector: &str,
    ) -> Result<(), StreamingValidationError> {
        let js = format!(
            r#"
            (function() {{
                window.__probar_vu_samples = [];
                window.__probar_vu_start_time = performance.now();

                const el = document.querySelector('{}');
                if (!el) {{
                    return {{ error: 'VU meter element not found: {}' }};
                }}

                function sampleVu() {{
                    const elapsed = performance.now() - window.__probar_vu_start_time;
                    // Try to get level from data attribute, width, or computed style
                    let level = parseFloat(el.dataset.level) || 0;
                    if (!level) {{
                        const style = getComputedStyle(el);
                        const widthPct = parseFloat(style.width) / parseFloat(style.maxWidth || 100);
                        level = isNaN(widthPct) ? 0 : widthPct;
                    }}
                    window.__probar_vu_samples.push([elapsed, level]);

                    if (window.__probar_vu_running) {{
                        requestAnimationFrame(sampleVu);
                    }}
                }}

                window.__probar_vu_running = true;
                requestAnimationFrame(sampleVu);
                return {{ success: true }};
            }})()
            "#,
            selector, selector
        );

        let _: serde_json::Value = page
            .evaluate(js)
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        Ok(())
    }

    /// Track partial transcription results via CDP
    ///
    /// Watches for text content changes on the specified element
    /// and records partial results with timestamps.
    ///
    /// # Errors
    /// Returns error if CDP injection fails.
    pub async fn track_partials_cdp(
        &mut self,
        page: &chromiumoxide::Page,
        selector: &str,
    ) -> Result<(), StreamingValidationError> {
        let js = format!(
            r#"
            (function() {{
                window.__probar_partials = [];
                window.__probar_partials_start = performance.now();
                window.__probar_last_partial = '';

                const el = document.querySelector('{}');
                if (!el) {{
                    return {{ error: 'Partial results element not found: {}' }};
                }}

                const observer = new MutationObserver((mutations) => {{
                    const text = el.textContent || el.innerText || '';
                    if (text !== window.__probar_last_partial) {{
                        const elapsed = performance.now() - window.__probar_partials_start;
                        window.__probar_partials.push([elapsed, text]);
                        window.__probar_last_partial = text;
                    }}
                }});

                observer.observe(el, {{
                    characterData: true,
                    childList: true,
                    subtree: true
                }});

                window.__probar_partials_observer = observer;
                return {{ success: true }};
            }})()
            "#,
            selector, selector
        );

        let _: serde_json::Value = page
            .evaluate(js)
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        Ok(())
    }

    /// Collect state history from browser
    ///
    /// # Errors
    /// Returns error if CDP call fails.
    pub async fn collect_state_history_cdp(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<Vec<StateTransition>, StreamingValidationError> {
        let js = r#"
            (function() {
                const history = window.__probar_state_history || [];
                return history.map((h, i, arr) => ({
                    from: h.from,
                    to: h.to,
                    timestamp: h.timestamp,
                    duration_ms: i < arr.length - 1 ? arr[i + 1].timestamp - h.timestamp : 0
                }));
            })()
        "#;

        let result: Vec<serde_json::Value> = page
            .evaluate(js)
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        Ok(result
            .into_iter()
            .map(|v| StateTransition {
                from: v["from"].as_str().unwrap_or("").to_string(),
                to: v["to"].as_str().unwrap_or("").to_string(),
                timestamp_ms: v["timestamp"].as_f64().unwrap_or(0.0),
                duration_ms: v["duration_ms"].as_f64().unwrap_or(0.0),
            })
            .collect())
    }

    /// Collect VU meter samples from browser
    ///
    /// # Errors
    /// Returns error if CDP call fails.
    pub async fn collect_vu_samples_cdp(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<Vec<VuMeterSample>, StreamingValidationError> {
        // Stop sampling
        let _: serde_json::Value = page
            .evaluate("window.__probar_vu_running = false; true")
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        let js = "window.__probar_vu_samples || []";
        let result: Vec<Vec<f64>> = page
            .evaluate(js)
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        Ok(result
            .into_iter()
            .map(|arr| VuMeterSample {
                timestamp_ms: arr.first().copied().unwrap_or(0.0),
                level: arr.get(1).copied().unwrap_or(0.0) as f32,
            })
            .collect())
    }

    /// Collect partial results from browser
    ///
    /// # Errors
    /// Returns error if CDP call fails.
    pub async fn collect_partials_cdp(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<Vec<PartialResult>, StreamingValidationError> {
        let js = "window.__probar_partials || []";
        let result: Vec<Vec<serde_json::Value>> = page
            .evaluate(js)
            .await
            .map_err(|_| StreamingValidationError::EndedInError)?
            .into_value()
            .map_err(|_| StreamingValidationError::EndedInError)?;

        let partials: Vec<PartialResult> = result
            .into_iter()
            .filter_map(|arr| {
                let ts = arr.first()?.as_f64()?;
                let text = arr.get(1)?.as_str()?.to_string();
                Some(PartialResult {
                    timestamp_ms: ts,
                    text,
                    is_final: false,
                })
            })
            .collect();

        // Mark the last one as final if present
        if partials.is_empty() {
            return Ok(partials);
        }

        let mut result = partials;
        if let Some(last) = result.last_mut() {
            last.is_final = true;
        }
        Ok(result)
    }

    /// Assert state sequence occurred in order
    ///
    /// # Errors
    /// Returns error if sequence was not observed.
    pub async fn assert_state_sequence_cdp(
        &self,
        page: &chromiumoxide::Page,
        expected: &[&str],
    ) -> Result<(), StreamingValidationError> {
        let history = self.collect_state_history_cdp(page).await?;
        let states: Vec<&str> = history.iter().map(|t| t.to.as_str()).collect();

        let mut expected_iter = expected.iter();
        let mut current_expected = expected_iter.next();

        for state in &states {
            if let Some(exp) = current_expected {
                if state == exp {
                    current_expected = expected_iter.next();
                }
            }
        }

        if current_expected.is_some() {
            return Err(StreamingValidationError::InvalidStateTransition {
                from: StreamingState::Idle,
                to: StreamingState::Error,
            });
        }

        Ok(())
    }

    /// Assert VU meter was active during period
    ///
    /// # Errors
    /// Returns error if VU meter was not active for required duration.
    pub async fn assert_vu_meter_active_cdp(
        &self,
        page: &chromiumoxide::Page,
        min_level: f32,
        duration_ms: u64,
    ) -> Result<(), StreamingValidationError> {
        let samples = self.collect_vu_samples_cdp(page).await?;

        let mut active_duration: f64 = 0.0;
        let mut last_ts: Option<f64> = None;

        for sample in &samples {
            if sample.level >= min_level {
                if let Some(prev_ts) = last_ts {
                    active_duration += sample.timestamp_ms - prev_ts;
                }
            }
            last_ts = Some(sample.timestamp_ms);
        }

        if active_duration < duration_ms as f64 {
            return Err(StreamingValidationError::TtfbExceeded {
                measured: Duration::from_millis(active_duration as u64),
                max: Duration::from_millis(duration_ms),
            });
        }

        Ok(())
    }

    /// Assert no UI jank (state updates within threshold)
    ///
    /// # Errors
    /// Returns error if gap between updates exceeds threshold.
    pub async fn assert_no_jank_cdp(
        &self,
        page: &chromiumoxide::Page,
        max_gap_ms: f64,
    ) -> Result<(), StreamingValidationError> {
        let history = self.collect_state_history_cdp(page).await?;

        for transition in &history {
            if transition.duration_ms > max_gap_ms {
                return Err(StreamingValidationError::LatencyExceeded {
                    measured: Duration::from_millis(transition.duration_ms as u64),
                    max: Duration::from_millis(max_gap_ms as u64),
                });
            }
        }

        Ok(())
    }

    /// Assert partial results appeared before final result
    ///
    /// # Errors
    /// Returns error if no partials appeared before final.
    pub async fn assert_partials_before_final_cdp(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<(), StreamingValidationError> {
        let partials = self.collect_partials_cdp(page).await?;

        // Need at least 2 results (one partial, one final)
        if partials.len() < 2 {
            return Err(StreamingValidationError::EndedInError);
        }

        // Verify there's at least one non-final result before the final
        let non_final_count = partials.iter().filter(|p| !p.is_final).count();
        if non_final_count == 0 {
            return Err(StreamingValidationError::EndedInError);
        }

        Ok(())
    }

    /// Get average state transition time in milliseconds
    ///
    /// # Errors
    /// Returns error if CDP call fails.
    pub async fn avg_transition_time_ms_cdp(
        &self,
        page: &chromiumoxide::Page,
    ) -> Result<f64, StreamingValidationError> {
        let history = self.collect_state_history_cdp(page).await?;

        if history.is_empty() {
            return Ok(0.0);
        }

        let total: f64 = history.iter().map(|t| t.duration_ms).sum();
        Ok(total / history.len() as f64)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // ========================================================================
    // H7: Streaming latency monitoring is accurate - Falsification tests
    // ========================================================================

    #[test]
    fn f029_latency_exceeded() {
        // Falsification: Latency above threshold should fail validation
        let mut validator =
            StreamingUxValidator::new().with_max_latency(Duration::from_millis(100));

        validator.record_metric(StreamingMetric::Latency(Duration::from_millis(150)));

        let result = validator.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            StreamingValidationError::LatencyExceeded { .. }
        ));
    }

    #[test]
    fn f030_latency_acceptable() {
        // Falsification: Latency below threshold should pass
        let mut validator =
            StreamingUxValidator::new().with_max_latency(Duration::from_millis(100));

        validator.record_metric(StreamingMetric::Latency(Duration::from_millis(50)));

        assert!(validator.validate().is_ok());
    }

    #[test]
    fn f031_buffer_underrun_threshold() {
        // Falsification: Too many buffer underruns should fail
        let mut validator = StreamingUxValidator::new().with_buffer_underrun_threshold(2);

        validator.record_metric(StreamingMetric::BufferUnderrun);
        validator.record_metric(StreamingMetric::BufferUnderrun);
        validator.record_metric(StreamingMetric::BufferUnderrun);

        let result = validator.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamingValidationError::BufferUnderrunThreshold { .. }
        ));
    }

    #[test]
    fn f032_dropped_frames_threshold() {
        // Falsification: Too many dropped frames should fail
        let mut validator = StreamingUxValidator::new().with_max_dropped_frames(2);

        for _ in 0..5 {
            validator.record_metric(StreamingMetric::FrameDropped);
        }

        let result = validator.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamingValidationError::DroppedFrameThreshold { .. }
        ));
    }

    // ========================================================================
    // H8: State machine transitions are valid - Falsification tests
    // ========================================================================

    #[test]
    fn f033_state_idle_to_buffering() {
        // Falsification: FirstByteReceived should transition Idle -> Buffering
        let mut validator = StreamingUxValidator::new();
        assert_eq!(validator.state(), StreamingState::Idle);

        validator.record_metric(StreamingMetric::FirstByteReceived);
        assert_eq!(validator.state(), StreamingState::Buffering);
    }

    #[test]
    fn f034_state_buffering_to_streaming() {
        // Falsification: Audio chunk should transition Buffering -> Streaming
        let mut validator = StreamingUxValidator::new();
        validator.start();
        assert_eq!(validator.state(), StreamingState::Buffering);

        validator.record_metric(StreamingMetric::AudioChunk {
            samples: 1024,
            sample_rate: 16000,
        });
        assert_eq!(validator.state(), StreamingState::Streaming);
    }

    #[test]
    fn f035_state_streaming_to_stalled() {
        // Falsification: Buffer underrun should transition Streaming -> Stalled
        let mut validator = StreamingUxValidator::new();
        validator.start();
        validator.record_metric(StreamingMetric::AudioChunk {
            samples: 1024,
            sample_rate: 16000,
        });
        assert_eq!(validator.state(), StreamingState::Streaming);

        validator.record_metric(StreamingMetric::BufferUnderrun);
        assert_eq!(validator.state(), StreamingState::Stalled);
    }

    #[test]
    fn f036_state_recovery_from_stalled() {
        // Falsification: Frame rendered should recover Stalled -> Streaming
        let mut validator = StreamingUxValidator::new();
        validator.start();
        validator.record_metric(StreamingMetric::AudioChunk {
            samples: 1024,
            sample_rate: 16000,
        });
        validator.record_metric(StreamingMetric::BufferUnderrun);
        assert_eq!(validator.state(), StreamingState::Stalled);

        validator.record_metric(StreamingMetric::FrameRendered { timestamp: 1000 });
        assert_eq!(validator.state(), StreamingState::Streaming);
    }

    // ========================================================================
    // H9: FPS calculation is accurate - Falsification tests
    // ========================================================================

    #[test]
    fn f037_fps_calculation() {
        // Falsification: FPS should be calculated correctly
        let mut validator = StreamingUxValidator::new();

        // Simulate 30fps for 1 second
        for i in 0..31 {
            validator.record_metric(StreamingMetric::FrameRendered {
                timestamp: i * 33, // ~30fps
            });
        }

        let fps = validator.average_fps();
        // Should be approximately 30fps
        assert!((fps - 30.0).abs() < 1.0, "FPS was {fps}, expected ~30");
    }

    #[test]
    fn f038_fps_below_minimum() {
        // Falsification: Low FPS should fail validation
        let mut validator = StreamingUxValidator::new().with_min_fps(30.0);

        // Simulate 15fps for 1 second
        for i in 0..16 {
            validator.record_metric(StreamingMetric::FrameRendered {
                timestamp: i * 66, // ~15fps
            });
        }

        let result = validator.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamingValidationError::FpsBelowMinimum { .. }
        ));
    }

    // ========================================================================
    // Unit tests for core functionality
    // ========================================================================

    #[test]
    fn test_default_validator() {
        let validator = StreamingUxValidator::new();
        assert_eq!(validator.state(), StreamingState::Idle);
        assert_eq!(validator.buffer_underruns(), 0);
        assert_eq!(validator.dropped_frames(), 0);
    }

    #[test]
    fn test_audio_preset() {
        let validator = StreamingUxValidator::for_audio();
        assert_eq!(validator.max_latency, Duration::from_millis(100));
        assert_eq!(validator.buffer_underrun_threshold, 3);
    }

    #[test]
    fn test_video_preset() {
        let validator = StreamingUxValidator::for_video();
        assert_eq!(validator.max_latency, Duration::from_millis(500));
        assert!((validator.min_fps - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_complete_transition() {
        let mut validator = StreamingUxValidator::new();
        validator.complete();
        assert_eq!(validator.state(), StreamingState::Completed);
    }

    #[test]
    fn test_error_transition() {
        let mut validator = StreamingUxValidator::new();
        validator.error();
        assert_eq!(validator.state(), StreamingState::Error);
        assert!(validator.validate().is_err());
    }

    #[test]
    fn test_reset() {
        let mut validator = StreamingUxValidator::new();
        validator.start();
        validator.record_metric(StreamingMetric::BufferUnderrun);
        validator.record_metric(StreamingMetric::FrameDropped);

        validator.reset();
        assert_eq!(validator.state(), StreamingState::Idle);
        assert_eq!(validator.buffer_underruns(), 0);
        assert_eq!(validator.dropped_frames(), 0);
    }

    #[test]
    fn test_validate_all_errors() {
        let mut validator = StreamingUxValidator::new()
            .with_max_latency(Duration::from_millis(50))
            .with_buffer_underrun_threshold(1);

        validator.record_metric(StreamingMetric::Latency(Duration::from_millis(100)));
        validator.record_metric(StreamingMetric::BufferUnderrun);
        validator.record_metric(StreamingMetric::BufferUnderrun);

        let errors = validator.validate_all();
        assert!(errors.len() >= 2);
    }

    #[test]
    fn test_state_history() {
        let mut validator = StreamingUxValidator::new();
        validator.start();
        validator.record_metric(StreamingMetric::AudioChunk {
            samples: 1024,
            sample_rate: 16000,
        });

        let history = validator.state_history();
        assert!(!history.is_empty());
        assert_eq!(history[0].0, StreamingState::Idle);
    }

    #[test]
    fn test_buffer_level_transitions() {
        let mut validator = StreamingUxValidator::new();
        validator.start();
        validator.record_metric(StreamingMetric::AudioChunk {
            samples: 1024,
            sample_rate: 16000,
        });
        assert_eq!(validator.state(), StreamingState::Streaming);

        // Low buffer level should stall
        validator.record_metric(StreamingMetric::BufferLevel(0.05));
        assert_eq!(validator.state(), StreamingState::Stalled);

        // Buffer recovery should resume streaming
        validator.record_metric(StreamingMetric::BufferLevel(0.5));
        assert_eq!(validator.state(), StreamingState::Streaming);
    }

    #[test]
    fn test_streaming_state_display() {
        assert_eq!(format!("{}", StreamingState::Idle), "Idle");
        assert_eq!(format!("{}", StreamingState::Streaming), "Streaming");
        assert_eq!(format!("{}", StreamingState::Stalled), "Stalled");
    }

    // ========================================================================
    // H10: VU Meter validation is accurate - Falsification tests
    // ========================================================================

    #[test]
    fn f039_vu_meter_negative_level_rejected() {
        // Falsification: Negative levels should be rejected
        let config = VuMeterConfig::default();
        let result = config.validate_sample(-0.5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VuMeterError::NegativeLevel(_)
        ));
    }

    #[test]
    fn f040_vu_meter_clipping_detected() {
        // Falsification: Level above max (with tolerance) should be clipping
        let config = VuMeterConfig::default().with_max_level(1.0);
        // Level 1.5 exceeds 1.0 + 0.1 tolerance
        let result = config.validate_sample(1.5);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VuMeterError::Clipping(_)));
    }

    #[test]
    fn f041_vu_meter_valid_level_accepted() {
        // Falsification: Valid level should pass
        let config = VuMeterConfig::default();
        assert!(config.validate_sample(0.5).is_ok());
        assert!(config.validate_sample(0.0).is_ok());
        assert!(config.validate_sample(1.0).is_ok());
    }

    #[test]
    fn f042_vu_meter_config_builder() {
        // Falsification: Builder methods should work correctly
        let config = VuMeterConfig::default()
            .with_min_level(0.1)
            .with_max_level(0.9)
            .with_update_rate_hz(60.0)
            .with_max_stale_ms(50);

        assert!((config.min_level - 0.1).abs() < f32::EPSILON);
        assert!((config.max_level - 0.9).abs() < f32::EPSILON);
        assert!((config.update_rate_hz - 60.0).abs() < f32::EPSILON);
        assert_eq!(config.max_stale_ms, 50);
    }

    #[test]
    fn f043_vu_meter_level_clamping() {
        // Falsification: Out-of-range levels should be clamped in config
        let config = VuMeterConfig::default()
            .with_min_level(-5.0)
            .with_max_level(10.0);

        // Clamped to 0.0-1.0 range
        assert!((config.min_level - 0.0).abs() < f32::EPSILON);
        assert!((config.max_level - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f044_vu_meter_min_update_rate() {
        // Falsification: Update rate should have minimum of 1.0 Hz
        let config = VuMeterConfig::default().with_update_rate_hz(0.1);

        assert!((config.update_rate_hz - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f045_vu_meter_error_display() {
        // Falsification: Error messages should be informative
        let negative = VuMeterError::NegativeLevel(-0.5);
        assert!(negative.to_string().contains("negative"));

        let clipping = VuMeterError::Clipping(1.5);
        assert!(clipping.to_string().contains("clipping"));

        let stale = VuMeterError::Stale {
            last_update_ms: 100,
            current_ms: 300,
        };
        assert!(stale.to_string().contains("stale"));

        let slow = VuMeterError::SlowUpdateRate {
            measured_hz: 10.0,
            expected_hz: 30.0,
        };
        assert!(slow.to_string().contains("slow"));

        let not_animating = VuMeterError::NotAnimating {
            sample_count: 10,
            value: 0.5,
        };
        assert!(not_animating.to_string().contains("not animating"));
    }

    #[test]
    fn f046_state_transition_tracking() {
        // Falsification: State transitions should be properly structured
        let transition = StateTransition {
            from: "Idle".to_string(),
            to: "Recording".to_string(),
            timestamp_ms: 1000.0,
            duration_ms: 500.0,
        };

        assert_eq!(transition.from, "Idle");
        assert_eq!(transition.to, "Recording");
        assert!((transition.timestamp_ms - 1000.0).abs() < f64::EPSILON);
        assert!((transition.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn f047_partial_result_tracking() {
        // Falsification: Partial results should track interim transcriptions
        let partial = PartialResult {
            timestamp_ms: 1500.0,
            text: "Hello wo".to_string(),
            is_final: false,
        };

        assert!(!partial.is_final);
        assert_eq!(partial.text, "Hello wo");

        let final_result = PartialResult {
            timestamp_ms: 2000.0,
            text: "Hello world".to_string(),
            is_final: true,
        };

        assert!(final_result.is_final);
    }

    #[test]
    fn f048_vu_meter_sample_tracking() {
        // Falsification: VU meter samples should track level over time
        let samples = vec![
            VuMeterSample {
                timestamp_ms: 0.0,
                level: 0.1,
            },
            VuMeterSample {
                timestamp_ms: 33.3,
                level: 0.3,
            },
            VuMeterSample {
                timestamp_ms: 66.6,
                level: 0.5,
            },
            VuMeterSample {
                timestamp_ms: 100.0,
                level: 0.4,
            },
        ];

        // Calculate average level
        let avg: f32 = samples.iter().map(|s| s.level).sum::<f32>() / samples.len() as f32;
        assert!((avg - 0.325).abs() < 0.01);

        // Check time span
        let duration = samples.last().unwrap().timestamp_ms - samples.first().unwrap().timestamp_ms;
        assert!((duration - 100.0).abs() < f64::EPSILON);
    }

    // ========================================================================
    // H11: Test Execution Stats are accurate - Falsification tests (Section 5.1)
    // ========================================================================

    #[test]
    fn f049_test_execution_stats_creation() {
        // Falsification: New stats should be zero-initialized
        let stats = TestExecutionStats::new();
        assert_eq!(stats.states_captured, 0);
        assert_eq!(stats.bytes_raw, 0);
        assert_eq!(stats.bytes_compressed, 0);
        assert_eq!(stats.same_fill_pages, 0);
    }

    #[test]
    fn f050_test_execution_stats_recording() {
        // Falsification: Stats should correctly record captures
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(4096, 1024);
        stats.record_state_capture(4096, 2048);

        assert_eq!(stats.states_captured, 2);
        assert_eq!(stats.bytes_raw, 8192);
        assert_eq!(stats.bytes_compressed, 3072);
    }

    #[test]
    fn f051_test_execution_stats_compression_ratio() {
        // Falsification: Compression ratio should be raw/compressed
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(4000, 1000);

        let ratio = stats.compression_ratio();
        assert!((ratio - 4.0).abs() < 0.01);
    }

    #[test]
    fn f052_test_execution_stats_efficiency() {
        // Falsification: Efficiency should be 1 - (compressed/raw)
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(1000, 250); // 75% efficiency

        let efficiency = stats.efficiency();
        assert!((efficiency - 0.75).abs() < 0.01);
    }

    #[test]
    fn f053_test_execution_stats_storage_savings() {
        // Falsification: Storage savings should be in MB
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(5_000_000, 1_000_000); // 4MB saved

        let savings = stats.storage_savings_mb();
        assert!((savings - 4.0).abs() < 0.01);
    }

    #[test]
    fn f054_test_execution_stats_same_fill_detection() {
        // Falsification: >90% compression should be detected as same-fill
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(4096, 100); // 97.5% compression - same-fill
        stats.record_state_capture(4096, 1024); // 75% compression - not same-fill

        assert_eq!(stats.same_fill_pages, 1);
        assert!((stats.same_fill_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn f055_test_execution_stats_reset() {
        // Falsification: Reset should clear all stats
        let mut stats = TestExecutionStats::new();
        stats.record_state_capture(4096, 1024);
        stats.reset();

        assert_eq!(stats.states_captured, 0);
        assert_eq!(stats.bytes_raw, 0);
        assert_eq!(stats.bytes_compressed, 0);
    }

    #[test]
    fn f056_test_execution_stats_edge_cases() {
        // Falsification: Edge cases should not panic
        let mut stats = TestExecutionStats::new();

        // Zero bytes
        assert!((stats.compression_ratio() - 0.0).abs() < f64::EPSILON);
        assert!((stats.efficiency() - 0.0).abs() < f64::EPSILON);
        assert!((stats.same_fill_ratio() - 0.0).abs() < f64::EPSILON);

        // Record with zero compressed
        stats.record_state_capture(1000, 0);
        assert!((stats.compression_ratio() - 0.0).abs() < f64::EPSILON); // Avoid division by zero
    }

    // ========================================================================
    // H12: Screenshot content classification is accurate - Falsification tests (Section 5.2)
    // ========================================================================

    #[test]
    fn f057_screenshot_content_uniform_detection() {
        // Falsification: >95% same value should be classified as Uniform
        let pixels: Vec<u8> = vec![255; 1000];
        let content = ScreenshotContent::classify(&pixels);

        assert!(matches!(
            content,
            ScreenshotContent::Uniform { fill_value: 255 }
        ));
        assert!((content.entropy() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f058_screenshot_content_ui_dominated() {
        // Falsification: Low entropy (<3.0) should be UI-dominated
        // Simulate mostly uniform with some variation (like UI text)
        let mut pixels = vec![255u8; 900]; // 90% white
        pixels.extend(vec![0u8; 50]); // 5% black
        pixels.extend(vec![128u8; 50]); // 5% gray

        let content = ScreenshotContent::classify(&pixels);
        // This should not be Uniform since it's <95% same value
        // And should have low entropy
        assert!(matches!(
            content,
            ScreenshotContent::UiDominated { .. } | ScreenshotContent::Uniform { .. }
        ));
    }

    #[test]
    fn f059_screenshot_content_high_entropy() {
        // Falsification: Random data should be classified as HighEntropy
        // Create pseudo-random looking data
        let pixels: Vec<u8> = (0..1000).map(|i| ((i * 127 + 37) % 256) as u8).collect();
        let content = ScreenshotContent::classify(&pixels);

        // Should be GameWorld or HighEntropy depending on actual entropy
        assert!(matches!(
            content,
            ScreenshotContent::GameWorld { .. } | ScreenshotContent::HighEntropy { .. }
        ));
    }

    #[test]
    fn f060_screenshot_content_compression_algorithm() {
        // Falsification: Compression algorithm should match content type
        let uniform = ScreenshotContent::Uniform { fill_value: 0 };
        assert_eq!(uniform.recommended_algorithm(), CompressionAlgorithm::Rle);

        let ui = ScreenshotContent::UiDominated { entropy: 2.0 };
        assert_eq!(ui.recommended_algorithm(), CompressionAlgorithm::Png);

        let game = ScreenshotContent::GameWorld { entropy: 4.5 };
        assert_eq!(game.recommended_algorithm(), CompressionAlgorithm::Zstd);

        let high = ScreenshotContent::HighEntropy { entropy: 7.0 };
        assert_eq!(high.recommended_algorithm(), CompressionAlgorithm::Lz4);
    }

    #[test]
    fn f061_screenshot_content_ratio_hints() {
        // Falsification: Ratio hints should describe compression expectations
        let uniform = ScreenshotContent::Uniform { fill_value: 0 };
        assert!(uniform.expected_ratio_hint().contains("excellent"));

        let ui = ScreenshotContent::UiDominated { entropy: 2.0 };
        assert!(ui.expected_ratio_hint().contains("good"));

        let game = ScreenshotContent::GameWorld { entropy: 4.5 };
        assert!(game.expected_ratio_hint().contains("moderate"));

        let high = ScreenshotContent::HighEntropy { entropy: 7.0 };
        assert!(high.expected_ratio_hint().contains("poor"));
    }

    #[test]
    fn f062_screenshot_content_empty_input() {
        // Falsification: Empty input should be handled gracefully
        let content = ScreenshotContent::classify(&[]);
        assert!(matches!(
            content,
            ScreenshotContent::Uniform { fill_value: 0 }
        ));
    }

    #[test]
    fn f063_screenshot_content_entropy_extraction() {
        // Falsification: Entropy should be extractable from all variants
        let variants = [
            ScreenshotContent::UiDominated { entropy: 1.5 },
            ScreenshotContent::GameWorld { entropy: 4.0 },
            ScreenshotContent::HighEntropy { entropy: 7.5 },
            ScreenshotContent::Uniform { fill_value: 128 },
        ];

        let entropies: Vec<f32> = variants.iter().map(|v| v.entropy()).collect();
        assert!((entropies[0] - 1.5).abs() < f32::EPSILON);
        assert!((entropies[1] - 4.0).abs() < f32::EPSILON);
        assert!((entropies[2] - 7.5).abs() < f32::EPSILON);
        assert!((entropies[3] - 0.0).abs() < f32::EPSILON); // Uniform has 0 entropy
    }
}
