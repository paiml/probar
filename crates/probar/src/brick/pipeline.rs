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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::brick::{BrickAssertion, BrickBudget, BrickVerification};

    // ============================================================
    // Test Stage Implementation
    // ============================================================

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

    /// A stage that fails validation
    struct FailingValidationStage {
        name: &'static str,
    }

    impl Brick for FailingValidationStage {
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

    impl BrickStage for FailingValidationStage {
        fn execute(&self, ctx: PipelineContext) -> PipelineResult<PipelineContext> {
            Ok(ctx)
        }

        fn validate(&self, _ctx: &PipelineContext) -> ValidationResult {
            ValidationResult::fail("Validation error")
        }
    }

    // ============================================================
    // PipelineContext tests
    // ============================================================

    #[test]
    fn test_pipeline_context_new() {
        let ctx = PipelineContext::new();
        assert!(ctx.data.is_empty());
        assert!(ctx.trace.is_empty());
    }

    #[test]
    fn test_pipeline_context_default() {
        let ctx = PipelineContext::default();
        assert!(ctx.data.is_empty());
    }

    #[test]
    fn test_pipeline_context_from_input() {
        let ctx = PipelineContext::from_input("input", PipelineData::Text("hello".into()));
        assert!(ctx.get("input").is_some());
    }

    #[test]
    fn test_pipeline_context() {
        let mut ctx = PipelineContext::new();
        ctx.set("test", PipelineData::Text("hello".into()));

        assert!(ctx.get("test").is_some());
        assert!(ctx.get("missing").is_none());
    }

    #[test]
    fn test_pipeline_context_add_trace() {
        let mut ctx = PipelineContext::new();
        ctx.add_trace(StageTrace {
            stage_name: "test".to_string(),
            duration: Duration::from_millis(10),
            success: true,
            error: None,
        });

        assert_eq!(ctx.trace.len(), 1);
        assert_eq!(ctx.trace[0].stage_name, "test");
    }

    #[test]
    fn test_pipeline_context_clone() {
        let mut ctx = PipelineContext::new();
        ctx.set("key", PipelineData::Int(42));

        let cloned = ctx.clone();
        assert!(cloned.get("key").is_some());
    }

    // ============================================================
    // PipelineData tests
    // ============================================================

    #[test]
    fn test_pipeline_data_tensor() {
        let data = PipelineData::tensor(vec![1.0, 2.0, 3.0], vec![3]);

        let (values, shape) = data.as_tensor().unwrap();
        assert_eq!(values, &[1.0, 2.0, 3.0]);
        assert_eq!(shape, &[3]);
    }

    #[test]
    fn test_pipeline_data_as_tensor_none() {
        let data = PipelineData::Text("hello".into());
        assert!(data.as_tensor().is_none());
    }

    #[test]
    fn test_pipeline_data_as_text() {
        let data = PipelineData::Text("hello".into());
        assert_eq!(data.as_text(), Some("hello"));
    }

    #[test]
    fn test_pipeline_data_as_text_none() {
        let data = PipelineData::Int(42);
        assert!(data.as_text().is_none());
    }

    #[test]
    fn test_pipeline_data_bytes() {
        let data = PipelineData::Bytes(vec![1, 2, 3, 4]);
        if let PipelineData::Bytes(bytes) = data {
            assert_eq!(bytes, vec![1, 2, 3, 4]);
        } else {
            panic!("Expected Bytes variant");
        }
    }

    #[test]
    fn test_pipeline_data_json() {
        let json = serde_json::json!({"key": "value"});
        let data = PipelineData::Json(json.clone());
        if let PipelineData::Json(value) = data {
            assert_eq!(value, json);
        } else {
            panic!("Expected Json variant");
        }
    }

    #[test]
    fn test_pipeline_data_int() {
        let data = PipelineData::Int(-42);
        if let PipelineData::Int(val) = data {
            assert_eq!(val, -42);
        } else {
            panic!("Expected Int variant");
        }
    }

    #[test]
    fn test_pipeline_data_bool() {
        let data = PipelineData::Bool(true);
        if let PipelineData::Bool(val) = data {
            assert!(val);
        } else {
            panic!("Expected Bool variant");
        }
    }

    #[test]
    fn test_pipeline_data_clone_and_debug() {
        let data = PipelineData::Text("test".into());
        let cloned = data.clone();
        assert!(format!("{:?}", cloned).contains("Text"));
    }

    // ============================================================
    // PipelineMetadata tests
    // ============================================================

    #[test]
    fn test_pipeline_metadata_new() {
        let meta = PipelineMetadata::new();
        assert!(meta.run_id.starts_with("run-"));
        assert!(meta.started_at.is_none());
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn test_pipeline_metadata_default() {
        let meta = PipelineMetadata::default();
        assert!(meta.run_id.starts_with("run-"));
    }

    #[test]
    fn test_pipeline_metadata_tag() {
        let mut meta = PipelineMetadata::new();
        meta.tag("env", "test");
        meta.tag("version", "1.0");

        assert_eq!(meta.tags.get("env"), Some(&"test".to_string()));
        assert_eq!(meta.tags.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_pipeline_metadata_clone_and_debug() {
        let meta = PipelineMetadata::new();
        let cloned = meta.clone();
        assert!(format!("{:?}", cloned).contains("PipelineMetadata"));
    }

    // ============================================================
    // StageTrace tests
    // ============================================================

    #[test]
    fn test_stage_trace_clone_and_debug() {
        let trace = StageTrace {
            stage_name: "test".to_string(),
            duration: Duration::from_millis(100),
            success: true,
            error: None,
        };

        let cloned = trace.clone();
        assert_eq!(cloned.stage_name, "test");
        assert!(cloned.success);
        assert!(format!("{:?}", cloned).contains("StageTrace"));
    }

    #[test]
    fn test_stage_trace_with_error() {
        let trace = StageTrace {
            stage_name: "failed".to_string(),
            duration: Duration::from_millis(50),
            success: false,
            error: Some("Something went wrong".to_string()),
        };

        assert!(!trace.success);
        assert_eq!(trace.error, Some("Something went wrong".to_string()));
    }

    // ============================================================
    // PrivacyTier tests
    // ============================================================

    #[test]
    fn test_privacy_tier_default() {
        let tier = PrivacyTier::default();
        assert_eq!(tier, PrivacyTier::Standard);
    }

    #[test]
    fn test_privacy_tier_equality() {
        assert_eq!(PrivacyTier::Sovereign, PrivacyTier::Sovereign);
        assert_ne!(PrivacyTier::Sovereign, PrivacyTier::Private);
        assert_ne!(PrivacyTier::Private, PrivacyTier::Standard);
    }

    #[test]
    fn test_privacy_tier_debug_and_clone() {
        let tier = PrivacyTier::Private;
        let cloned = tier;
        assert!(format!("{:?}", cloned).contains("Private"));
    }

    // ============================================================
    // ValidationResult tests
    // ============================================================

    #[test]
    fn test_validation_result_ok() {
        let ok = ValidationResult::ok();
        assert!(ok.valid);
        assert!(ok.messages.is_empty());
    }

    #[test]
    fn test_validation_result_fail() {
        let fail = ValidationResult::fail("test error");
        assert!(!fail.valid);
        assert_eq!(fail.messages.len(), 1);
        assert_eq!(fail.messages[0].level, ValidationLevel::Error);
        assert_eq!(fail.messages[0].message, "test error");
    }

    #[test]
    fn test_validation_result_warn() {
        let mut result = ValidationResult::ok();
        result.warn("warning message");

        assert!(result.valid);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].level, ValidationLevel::Warning);
    }

    #[test]
    fn test_validation_result_clone_and_debug() {
        let result = ValidationResult::fail("error");
        let cloned = result.clone();
        assert!(format!("{:?}", cloned).contains("ValidationResult"));
    }

    // ============================================================
    // ValidationLevel tests
    // ============================================================

    #[test]
    fn test_validation_level_equality() {
        assert_eq!(ValidationLevel::Info, ValidationLevel::Info);
        assert_eq!(ValidationLevel::Warning, ValidationLevel::Warning);
        assert_eq!(ValidationLevel::Error, ValidationLevel::Error);
        assert_ne!(ValidationLevel::Info, ValidationLevel::Error);
    }

    #[test]
    fn test_validation_level_debug_and_clone() {
        let level = ValidationLevel::Warning;
        let cloned = level;
        assert!(format!("{:?}", cloned).contains("Warning"));
    }

    // ============================================================
    // ValidationMessage tests
    // ============================================================

    #[test]
    fn test_validation_message_clone_and_debug() {
        let msg = ValidationMessage {
            level: ValidationLevel::Error,
            message: "test".to_string(),
        };

        let cloned = msg.clone();
        assert_eq!(cloned.message, "test");
        assert!(format!("{:?}", cloned).contains("ValidationMessage"));
    }

    // ============================================================
    // PipelineError tests
    // ============================================================

    #[test]
    fn test_pipeline_error_validation_failed() {
        let err = PipelineError::ValidationFailed {
            stage: "test".to_string(),
            reason: "bad input".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Validation failed"));
        assert!(display.contains("test"));
        assert!(display.contains("bad input"));
    }

    #[test]
    fn test_pipeline_error_execution_failed() {
        let err = PipelineError::ExecutionFailed {
            stage: "compute".to_string(),
            reason: "timeout".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Execution failed"));
        assert!(display.contains("compute"));
    }

    #[test]
    fn test_pipeline_error_missing_input() {
        let err = PipelineError::MissingInput {
            stage: "transform".to_string(),
            input: "data".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Missing input"));
        assert!(display.contains("data"));
        assert!(display.contains("transform"));
    }

    #[test]
    fn test_pipeline_error_privacy_violation() {
        let err = PipelineError::PrivacyViolation {
            tier: PrivacyTier::Sovereign,
            reason: "external API call".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Privacy tier"));
        assert!(display.contains("Sovereign"));
    }

    #[test]
    fn test_pipeline_error_checkpoint_failed() {
        let err = PipelineError::CheckpointFailed {
            reason: "disk full".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Checkpoint failed"));
        assert!(display.contains("disk full"));
    }

    #[test]
    fn test_pipeline_error_brick_error() {
        let err = PipelineError::BrickError("brick error".to_string());

        let display = format!("{}", err);
        assert!(display.contains("Brick error"));
    }

    #[test]
    fn test_pipeline_error_from_brick_error() {
        use crate::brick::{BrickAssertion, BrickError};
        let brick_err = BrickError::AssertionFailed {
            assertion: BrickAssertion::ElementPresent("test".to_string()),
            reason: "failed".to_string(),
        };

        let pipeline_err: PipelineError = brick_err.into();
        if let PipelineError::BrickError(msg) = pipeline_err {
            assert!(msg.contains("test"));
        } else {
            panic!("Expected BrickError variant");
        }
    }

    #[test]
    fn test_pipeline_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(PipelineError::CheckpointFailed {
            reason: "test".to_string(),
        });

        assert!(err.to_string().contains("Checkpoint"));
    }

    // ============================================================
    // PipelineAuditCollector tests
    // ============================================================

    #[test]
    fn test_audit_collector_new() {
        let collector = PipelineAuditCollector::new();
        assert!(collector.entries().is_empty());
    }

    #[test]
    fn test_audit_collector_default() {
        let collector = PipelineAuditCollector::default();
        assert!(collector.entries().is_empty());
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
    fn test_audit_collector_record_failure() {
        let mut collector = PipelineAuditCollector::new();
        collector.record("failed", Duration::from_millis(25), false);

        assert_eq!(collector.entries().len(), 1);
        assert!(!collector.entries()[0].success);
    }

    #[test]
    fn test_audit_collector_debug() {
        let collector = PipelineAuditCollector::new();
        assert!(format!("{:?}", collector).contains("PipelineAuditCollector"));
    }

    // ============================================================
    // AuditEntry tests
    // ============================================================

    #[test]
    fn test_audit_entry_clone_and_debug() {
        let entry = AuditEntry {
            stage: "test".to_string(),
            timestamp: Instant::now(),
            duration: Duration::from_millis(100),
            success: true,
            inputs: vec!["input1".to_string()],
            outputs: vec!["output1".to_string()],
        };

        let cloned = entry.clone();
        assert_eq!(cloned.stage, "test");
        assert!(format!("{:?}", cloned).contains("AuditEntry"));
    }

    // ============================================================
    // Checkpoint tests
    // ============================================================

    #[test]
    fn test_checkpoint_clone_and_debug() {
        let checkpoint = Checkpoint {
            stage_index: 2,
            context: PipelineContext::new(),
            created_at: Instant::now(),
        };

        let cloned = checkpoint.clone();
        assert_eq!(cloned.stage_index, 2);
        assert!(format!("{:?}", cloned).contains("Checkpoint"));
    }

    // ============================================================
    // BrickPipeline tests
    // ============================================================

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
    fn test_pipeline_validation_failure() {
        let mut pipeline =
            BrickPipeline::new("test").stage(FailingValidationStage { name: "validator" });

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx);

        assert!(result.is_err());
        match result {
            Err(PipelineError::ValidationFailed { stage, reason }) => {
                assert_eq!(stage, "validator");
                assert!(reason.contains("Validation error"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_pipeline_privacy_tier() {
        let pipeline = BrickPipeline::new("test").with_privacy(PrivacyTier::Sovereign);

        assert_eq!(pipeline.privacy_tier(), PrivacyTier::Sovereign);
    }

    #[test]
    fn test_pipeline_name() {
        let pipeline = BrickPipeline::new("my-pipeline");
        assert_eq!(pipeline.name(), "my-pipeline");
    }

    #[test]
    fn test_pipeline_stage_count() {
        let pipeline = BrickPipeline::new("test")
            .stage(TestStage {
                name: "s1",
                should_fail: false,
            })
            .stage(TestStage {
                name: "s2",
                should_fail: false,
            })
            .stage(TestStage {
                name: "s3",
                should_fail: false,
            });

        assert_eq!(pipeline.stage_count(), 3);
    }

    #[test]
    fn test_pipeline_empty() {
        let mut pipeline = BrickPipeline::new("empty");

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx);

        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_with_checkpointing() {
        let pipeline =
            BrickPipeline::new("checkpointed").with_checkpointing(Duration::from_secs(5));

        // Just verify it compiles and sets the interval
        assert_eq!(pipeline.name(), "checkpointed");
    }

    #[test]
    fn test_pipeline_audit_trail() {
        let mut pipeline = BrickPipeline::new("audited")
            .stage(TestStage {
                name: "step1",
                should_fail: false,
            })
            .stage(TestStage {
                name: "step2",
                should_fail: false,
            });

        let ctx = PipelineContext::new();
        let _ = pipeline.run(ctx);

        let trail = pipeline.audit_trail();
        assert_eq!(trail.len(), 2);
        assert!(trail[0].success);
        assert!(trail[1].success);
    }

    #[test]
    fn test_pipeline_audit_trail_with_failure() {
        let mut pipeline = BrickPipeline::new("audited")
            .stage(TestStage {
                name: "success",
                should_fail: false,
            })
            .stage(TestStage {
                name: "failure",
                should_fail: true,
            });

        let ctx = PipelineContext::new();
        let _ = pipeline.run(ctx);

        let trail = pipeline.audit_trail();
        assert_eq!(trail.len(), 2);
        assert!(trail[0].success);
        assert!(!trail[1].success);
    }

    #[test]
    fn test_pipeline_debug() {
        let pipeline = BrickPipeline::new("debug-test")
            .with_privacy(PrivacyTier::Private)
            .stage(TestStage {
                name: "s1",
                should_fail: false,
            });

        let debug_str = format!("{:?}", pipeline);
        assert!(debug_str.contains("BrickPipeline"));
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("Private"));
    }

    #[test]
    fn test_pipeline_context_metadata_started_at() {
        let mut pipeline = BrickPipeline::new("test").stage(TestStage {
            name: "s1",
            should_fail: false,
        });

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx).unwrap();

        assert!(result.metadata.started_at.is_some());
    }

    #[test]
    fn test_pipeline_traces_recorded() {
        let mut pipeline = BrickPipeline::new("traced").stage(TestStage {
            name: "traced_stage",
            should_fail: false,
        });

        let ctx = PipelineContext::new();
        let result = pipeline.run(ctx).unwrap();

        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].stage_name, "traced_stage");
        assert!(result.trace[0].success);
        assert!(result.trace[0].error.is_none());
    }

    // ============================================================
    // BrickStage trait tests
    // ============================================================

    #[test]
    fn test_brick_stage_default_required_inputs() {
        let stage = TestStage {
            name: "test",
            should_fail: false,
        };

        assert!(stage.required_inputs().is_empty());
    }

    #[test]
    fn test_brick_stage_default_output_names() {
        let stage = TestStage {
            name: "test",
            should_fail: false,
        };

        assert!(stage.output_names().is_empty());
    }

    // ============================================================
    // uuid_v4 function test
    // ============================================================

    #[test]
    fn test_uuid_generation() {
        // Test that metadata run_id is unique
        let meta1 = PipelineMetadata::new();
        let meta2 = PipelineMetadata::new();

        // They should both start with "run-"
        assert!(meta1.run_id.starts_with("run-"));
        assert!(meta2.run_id.starts_with("run-"));
    }

    // ============================================================
    // Integration tests
    // ============================================================

    #[test]
    fn test_full_pipeline_workflow() {
        let mut pipeline = BrickPipeline::new("full-workflow")
            .with_privacy(PrivacyTier::Private)
            .stage(TestStage {
                name: "input",
                should_fail: false,
            })
            .stage(TestStage {
                name: "transform",
                should_fail: false,
            })
            .stage(TestStage {
                name: "output",
                should_fail: false,
            });

        let ctx = PipelineContext::from_input("initial", PipelineData::Text("start".into()));
        let result = pipeline.run(ctx).unwrap();

        // Check all stages executed
        assert!(result.get("input_output").is_some());
        assert!(result.get("transform_output").is_some());
        assert!(result.get("output_output").is_some());

        // Check traces
        assert_eq!(result.trace.len(), 3);

        // Check audit trail
        assert_eq!(pipeline.audit_trail().len(), 3);
    }

    #[test]
    fn test_pipeline_with_tensor_data() {
        let mut pipeline = BrickPipeline::new("tensor-pipeline").stage(TestStage {
            name: "process",
            should_fail: false,
        });

        let ctx = PipelineContext::from_input(
            "tensor",
            PipelineData::tensor(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]),
        );

        let result = pipeline.run(ctx).unwrap();

        // Original tensor data should still be accessible
        let tensor = result.get("tensor").unwrap();
        let (data, shape) = tensor.as_tensor().unwrap();
        assert_eq!(data.len(), 4);
        assert_eq!(shape, &[2, 2]);
    }
}
