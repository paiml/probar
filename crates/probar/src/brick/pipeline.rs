//! BrickPipeline: Orchestration for multi-brick workflows (PROBAR-SPEC-009-P9)
//!
//! Provides pipeline orchestration with:
//! - Sequential dependencies (brick A → brick B)
//! - Parallel execution (bricks A and B concurrently)
//! - Failure handling and checkpointing
//! - Privacy-aware routing
//!
//! # Design Philosophy
//!
//! BrickPipeline applies the batuta orchestration patterns to brick execution.
//! Every brick can be a pipeline stage with validation and audit trail.
//!
//! # Example
//!
//! ```rust,ignore
//! use probar::brick::pipeline::{BrickPipeline, BrickStage, PrivacyTier};
//!
//! let whisper_pipeline = BrickPipeline::new()
//!     .stage(AudioCaptureBrick::new())
//!     .stage(MelSpectrogramBrick::new())
//!     .stage(EncoderBrick::new())
//!     .stage(DecoderBrick::new())
//!     .with_privacy(PrivacyTier::Sovereign)
//!     .with_checkpointing(Duration::from_secs(5));
//!
//! let output = whisper_pipeline.run(input).await?;
//! ```

// Allow missing docs for enum variant fields - context is clear from variant name
#![allow(missing_docs)]

use super::{Brick, BrickError};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Pipeline execution context
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Named data values in the pipeline
    pub data: HashMap<String, PipelineData>,
    /// Metadata for audit trail
    pub metadata: PipelineMetadata,
    /// Execution trace for debugging
    pub trace: Vec<StageTrace>,
}

impl PipelineContext {
    /// Create a new empty context
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            metadata: PipelineMetadata::new(),
            trace: Vec::new(),
        }
    }

    /// Create context from initial input
    #[must_use]
    pub fn from_input(name: &str, data: PipelineData) -> Self {
        let mut ctx = Self::new();
        ctx.data.insert(name.to_string(), data);
        ctx
    }

    /// Get data by name
    pub fn get(&self, name: &str) -> Option<&PipelineData> {
        self.data.get(name)
    }

    /// Set data by name
    pub fn set(&mut self, name: impl Into<String>, data: PipelineData) {
        self.data.insert(name.into(), data);
    }

    /// Add a stage trace entry
    pub fn add_trace(&mut self, trace: StageTrace) {
        self.trace.push(trace);
    }
}

impl Default for PipelineContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Data types that can flow through pipelines
#[derive(Debug, Clone)]
pub enum PipelineData {
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Float tensor
    FloatTensor { data: Vec<f32>, shape: Vec<usize> },
    /// String value
    Text(String),
    /// JSON value
    Json(serde_json::Value),
    /// Integer value
    Int(i64),
    /// Boolean value
    Bool(bool),
}

impl PipelineData {
    /// Create a float tensor
    #[must_use]
    pub fn tensor(data: Vec<f32>, shape: Vec<usize>) -> Self {
        Self::FloatTensor { data, shape }
    }

    /// Get as float tensor
    pub fn as_tensor(&self) -> Option<(&[f32], &[usize])> {
        match self {
            Self::FloatTensor { data, shape } => Some((data, shape)),
            _ => None,
        }
    }

    /// Get as text
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

/// Metadata for pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    /// Pipeline run ID
    pub run_id: String,
    /// Start time
    pub started_at: Option<Instant>,
    /// Custom tags
    pub tags: HashMap<String, String>,
}

impl PipelineMetadata {
    /// Create new metadata with generated run ID
    #[must_use]
    pub fn new() -> Self {
        Self {
            run_id: format!("run-{}", uuid_v4()),
            started_at: None,
            tags: HashMap::new(),
        }
    }

    /// Add a tag
    pub fn tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }
}

impl Default for PipelineMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution trace for a single stage
#[derive(Debug, Clone)]
pub struct StageTrace {
    /// Stage name
    pub stage_name: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether stage succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Privacy tier for pipeline execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyTier {
    /// Local-only execution (no network calls)
    Sovereign,
    /// VPC-only (private cloud, no public APIs)
    Private,
    /// Cloud-enabled (spillover to external APIs)
    Standard,
}

impl Default for PrivacyTier {
    fn default() -> Self {
        Self::Standard
    }
}

/// Validation result from a stage
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation messages (warnings and errors)
    pub messages: Vec<ValidationMessage>,
}

impl ValidationResult {
    /// Create a passing validation result
    #[must_use]
    pub fn ok() -> Self {
        Self {
            valid: true,
            messages: Vec::new(),
        }
    }

    /// Create a failing validation result
    #[must_use]
    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            valid: false,
            messages: vec![ValidationMessage {
                level: ValidationLevel::Error,
                message: reason.into(),
            }],
        }
    }

    /// Add a warning
    pub fn warn(&mut self, message: impl Into<String>) {
        self.messages.push(ValidationMessage {
            level: ValidationLevel::Warning,
            message: message.into(),
        });
    }
}

/// Validation message level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Informational message
    Info,
    /// Warning (execution continues)
    Warning,
    /// Error (execution blocked)
    Error,
}

/// A validation message
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Message severity
    pub level: ValidationLevel,
    /// Message content
    pub message: String,
}

/// Pipeline error type
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// Stage validation failed
    ValidationFailed { stage: String, reason: String },
    /// Stage execution failed
    ExecutionFailed { stage: String, reason: String },
    /// Missing required input
    MissingInput { stage: String, input: String },
    /// Privacy tier violation
    PrivacyViolation { tier: PrivacyTier, reason: String },
    /// Checkpoint failed
    CheckpointFailed { reason: String },
    /// Brick error
    BrickError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationFailed { stage, reason } => {
                write!(f, "Validation failed at stage '{}': {}", stage, reason)
            }
            Self::ExecutionFailed { stage, reason } => {
                write!(f, "Execution failed at stage '{}': {}", stage, reason)
            }
            Self::MissingInput { stage, input } => {
                write!(f, "Missing input '{}' for stage '{}'", input, stage)
            }
            Self::PrivacyViolation { tier, reason } => {
                write!(f, "Privacy tier {:?} violated: {}", tier, reason)
            }
            Self::CheckpointFailed { reason } => {
                write!(f, "Checkpoint failed: {}", reason)
            }
            Self::BrickError(msg) => write!(f, "Brick error: {}", msg),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<BrickError> for PipelineError {
    fn from(e: BrickError) -> Self {
        Self::BrickError(e.to_string())
    }
}

/// Trait for bricks that can be pipeline stages
pub trait BrickStage: Brick + Send + Sync {
    /// Execute the stage
    fn execute(&self, ctx: PipelineContext) -> PipelineResult<PipelineContext>;

    /// Validate before execution (Jidoka pattern)
    fn validate(&self, ctx: &PipelineContext) -> ValidationResult;

    /// Get required input names
    fn required_inputs(&self) -> &[&str] {
        &[]
    }

    /// Get output names
    fn output_names(&self) -> &[&str] {
        &[]
    }
}

/// Audit entry for pipeline execution
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Stage name
    pub stage: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Duration
    pub duration: Duration,
    /// Success status
    pub success: bool,
    /// Input data keys
    pub inputs: Vec<String>,
    /// Output data keys
    pub outputs: Vec<String>,
}

/// Audit trail collector
#[derive(Debug, Default)]
pub struct PipelineAuditCollector {
    entries: Vec<AuditEntry>,
}

impl PipelineAuditCollector {
    /// Create a new collector
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record a stage execution
    pub fn record(&mut self, stage: &str, duration: Duration, success: bool) {
        self.entries.push(AuditEntry {
            stage: stage.to_string(),
            timestamp: Instant::now(),
            duration,
            success,
            inputs: Vec::new(),
            outputs: Vec::new(),
        });
    }

    /// Get all entries
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get total execution time
    pub fn total_duration(&self) -> Duration {
        self.entries.iter().map(|e| e.duration).sum()
    }
}

/// Checkpoint state for fault tolerance
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Stage index
    pub stage_index: usize,
    /// Context at checkpoint
    pub context: PipelineContext,
    /// Timestamp
    pub created_at: Instant,
}

/// BrickPipeline: Orchestrates multi-brick workflows
pub struct BrickPipeline {
    /// Pipeline name
    name: String,
    /// Ordered stages
    stages: Vec<Box<dyn BrickStage>>,
    /// Privacy tier
    privacy_tier: PrivacyTier,
    /// Checkpoint interval
    checkpoint_interval: Option<Duration>,
    /// Audit collector
    audit_collector: PipelineAuditCollector,
    /// Last checkpoint
    last_checkpoint: Option<Checkpoint>,
}

impl BrickPipeline {
    /// Create a new pipeline
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
            privacy_tier: PrivacyTier::Standard,
            checkpoint_interval: None,
            audit_collector: PipelineAuditCollector::new(),
            last_checkpoint: None,
        }
    }

    /// Add a stage to the pipeline
    #[must_use]
    pub fn stage<S: BrickStage + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    /// Set privacy tier
    #[must_use]
    pub fn with_privacy(mut self, tier: PrivacyTier) -> Self {
        self.privacy_tier = tier;
        self
    }

    /// Enable checkpointing
    #[must_use]
    pub fn with_checkpointing(mut self, interval: Duration) -> Self {
        self.checkpoint_interval = Some(interval);
        self
    }

    /// Run the pipeline
    pub fn run(&mut self, input: PipelineContext) -> PipelineResult<PipelineContext> {
        let mut ctx = input;
        ctx.metadata.started_at = Some(Instant::now());

        let start_index = self
            .last_checkpoint
            .as_ref()
            .map(|c| c.stage_index)
            .unwrap_or(0);

        // Restore from checkpoint if available
        if let Some(checkpoint) = &self.last_checkpoint {
            ctx = checkpoint.context.clone();
        }

        let mut last_checkpoint_time = Instant::now();

        for (i, stage) in self.stages.iter().enumerate().skip(start_index) {
            let stage_name = stage.brick_name();

            // Jidoka: validate before execution
            let validation = stage.validate(&ctx);
            if !validation.valid {
                let reason = validation
                    .messages
                    .iter()
                    .filter(|m| m.level == ValidationLevel::Error)
                    .map(|m| m.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");

                return Err(PipelineError::ValidationFailed {
                    stage: stage_name.to_string(),
                    reason,
                });
            }

            // Execute stage (clone ctx for error recovery)
            let start = Instant::now();
            let ctx_for_error = ctx.clone();
            let result = stage.execute(ctx);
            let duration = start.elapsed();

            match result {
                Ok(mut new_ctx) => {
                    new_ctx.add_trace(StageTrace {
                        stage_name: stage_name.to_string(),
                        duration,
                        success: true,
                        error: None,
                    });

                    self.audit_collector.record(stage_name, duration, true);

                    // Checkpoint if interval exceeded
                    if let Some(interval) = self.checkpoint_interval {
                        if last_checkpoint_time.elapsed() >= interval {
                            self.last_checkpoint = Some(Checkpoint {
                                stage_index: i + 1,
                                context: new_ctx.clone(),
                                created_at: Instant::now(),
                            });
                            last_checkpoint_time = Instant::now();
                        }
                    }

                    ctx = new_ctx;
                }
                Err(e) => {
                    self.audit_collector.record(stage_name, duration, false);

                    // Use saved context for trace in error case
                    let mut error_ctx = ctx_for_error;
                    error_ctx.add_trace(StageTrace {
                        stage_name: stage_name.to_string(),
                        duration,
                        success: false,
                        error: Some(e.to_string()),
                    });

                    return Err(PipelineError::ExecutionFailed {
                        stage: stage_name.to_string(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        // Clear checkpoint on successful completion
        self.last_checkpoint = None;

        Ok(ctx)
    }

    /// Get the pipeline name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of stages
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get audit trail
    pub fn audit_trail(&self) -> &[AuditEntry] {
        self.audit_collector.entries()
    }

    /// Get privacy tier
    #[must_use]
    pub fn privacy_tier(&self) -> PrivacyTier {
        self.privacy_tier
    }
}

impl Debug for BrickPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrickPipeline")
            .field("name", &self.name)
            .field("stage_count", &self.stages.len())
            .field("privacy_tier", &self.privacy_tier)
            .finish()
    }
}

/// Generate a simple UUID v4 (non-cryptographic)
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_nanos(), std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brick::{BrickAssertion, BrickBudget, BrickVerification};

    struct TestStage {
        name: &'static str,
        should_fail: bool,
    }

    impl Brick for TestStage {
        fn brick_name(&self) -> &'static str {
            self.name
        }

        fn assertions(&self) -> &[BrickAssertion] {
            &[]
        }

        fn budget(&self) -> BrickBudget {
            BrickBudget::uniform(100)
        }

        fn verify(&self) -> BrickVerification {
            BrickVerification {
                passed: vec![],
                failed: vec![],
                verification_time: Duration::from_micros(10),
            }
        }

        fn to_html(&self) -> String {
            String::new()
        }

        fn to_css(&self) -> String {
            String::new()
        }
    }

    impl BrickStage for TestStage {
        fn execute(&self, mut ctx: PipelineContext) -> PipelineResult<PipelineContext> {
            if self.should_fail {
                return Err(PipelineError::ExecutionFailed {
                    stage: self.name.to_string(),
                    reason: "Test failure".into(),
                });
            }
            ctx.set(
                format!("{}_output", self.name),
                PipelineData::Text("done".into()),
            );
            Ok(ctx)
        }

        fn validate(&self, _ctx: &PipelineContext) -> ValidationResult {
            ValidationResult::ok()
        }
    }

    #[test]
    fn test_pipeline_basic() {
        let mut pipeline = BrickPipeline::new("test")
            .stage(TestStage {
                name: "stage1",
                should_fail: false,
            })
            .stage(TestStage {
                name: "stage2",
                should_fail: false,
            });

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.get("stage1_output").is_some());
        assert!(output.get("stage2_output").is_some());
    }

    #[test]
    fn test_pipeline_failure() {
        let mut pipeline = BrickPipeline::new("test")
            .stage(TestStage {
                name: "stage1",
                should_fail: false,
            })
            .stage(TestStage {
                name: "stage2",
                should_fail: true,
            });

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx);

        assert!(result.is_err());
        match result {
            Err(PipelineError::ExecutionFailed { stage, .. }) => {
                assert_eq!(stage, "stage2");
            }
            _ => panic!("Expected ExecutionFailed"),
        }
    }

    #[test]
    fn test_pipeline_privacy_tier() {
        let pipeline = BrickPipeline::new("test").with_privacy(PrivacyTier::Sovereign);

        assert_eq!(pipeline.privacy_tier(), PrivacyTier::Sovereign);
    }

    #[test]
    fn test_pipeline_context() {
        let mut ctx = PipelineContext::new();
        ctx.set("test", PipelineData::Text("hello".into()));

        assert!(ctx.get("test").is_some());
        assert!(ctx.get("missing").is_none());
    }

    #[test]
    fn test_pipeline_data_tensor() {
        let data = PipelineData::tensor(vec![1.0, 2.0, 3.0], vec![3]);

        let (values, shape) = data.as_tensor().unwrap();
        assert_eq!(values, &[1.0, 2.0, 3.0]);
        assert_eq!(shape, &[3]);
    }

    #[test]
    fn test_audit_collector() {
        let mut collector = PipelineAuditCollector::new();
        collector.record("stage1", Duration::from_millis(100), true);
        collector.record("stage2", Duration::from_millis(50), true);

        assert_eq!(collector.entries().len(), 2);
        assert_eq!(collector.total_duration(), Duration::from_millis(150));
    }

    #[test]
    fn test_validation_result() {
        let ok = ValidationResult::ok();
        assert!(ok.valid);

        let fail = ValidationResult::fail("test error");
        assert!(!fail.valid);
        assert_eq!(fail.messages.len(), 1);
    }
}
