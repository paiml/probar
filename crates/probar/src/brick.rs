//! Brick Architecture: Tests ARE the Interface (PROBAR-SPEC-009)
//!
//! This module implements the core Brick abstraction where UI components
//! are defined by their test assertions, not by their implementation.
//!
//! # Design Philosophy
//!
//! The Brick Architecture inverts the traditional UI/Test relationship:
//!
//! ```text
//! Traditional: Widget → Tests
//! Brick:       Brick(Assertions) → Widget + Tests
//! ```
//!
//! A `Brick` defines:
//! 1. **Assertions**: What must be true (contrast ratio, visibility, latency)
//! 2. **Budget**: Performance envelope (max render time in ms)
//! 3. **Events**: State transitions that trigger assertions
//!
//! # Popperian Falsification
//!
//! Each assertion is a falsifiable hypothesis. If ANY assertion fails,
//! the brick is falsified and cannot render.
//!
//! # Example
//!
//! ```rust,ignore
//! use probar::brick::{Brick, BrickAssertion, BrickBudget};
//!
//! #[derive(Brick)]
//! #[brick(
//!     html = "div.transcription",
//!     budget_ms = 100,
//!     assertions = [text_visible, contrast_ratio(4.5)]
//! )]
//! struct TranscriptionBrick {
//!     text: String,
//!     is_final: bool,
//! }
//! ```
//!
//! # References
//!
//! - Popper, K. (1959). The Logic of Scientific Discovery
//! - Beizer, B. (1990). Software Testing Techniques
//! - PROBAR-SPEC-009: Bug Hunting Probador

use std::time::Duration;

/// Brick assertion that must be verified at runtime.
///
/// Assertions are falsifiable hypotheses about the UI state.
/// If any assertion fails, the brick is falsified.
#[derive(Debug, Clone, PartialEq)]
pub enum BrickAssertion {
    /// Text content must be visible (not hidden, not zero-opacity)
    TextVisible,

    /// WCAG 2.1 AA contrast ratio requirement (4.5:1 for normal text)
    ContrastRatio(f32),

    /// Maximum render latency in milliseconds
    MaxLatencyMs(u32),

    /// Element must be present in DOM
    ElementPresent(String),

    /// Element must be focusable for accessibility
    Focusable,

    /// Custom assertion with name and validation function ID
    Custom {
        /// Assertion name for error reporting
        name: String,
        /// Validation function identifier
        validator_id: u64,
    },
}

impl BrickAssertion {
    /// Create a text visibility assertion
    #[must_use]
    pub const fn text_visible() -> Self {
        Self::TextVisible
    }

    /// Create a contrast ratio assertion (WCAG 2.1 AA)
    #[must_use]
    pub const fn contrast_ratio(ratio: f32) -> Self {
        Self::ContrastRatio(ratio)
    }

    /// Create a max latency assertion
    #[must_use]
    pub const fn max_latency_ms(ms: u32) -> Self {
        Self::MaxLatencyMs(ms)
    }

    /// Create an element presence assertion
    #[must_use]
    pub fn element_present(selector: impl Into<String>) -> Self {
        Self::ElementPresent(selector.into())
    }
}

/// Performance budget for a brick.
///
/// Budgets are enforced at runtime. Exceeding the budget triggers
/// a Jidoka (stop-the-line) alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrickBudget {
    /// Maximum time for measure phase
    pub measure_ms: u32,
    /// Maximum time for layout phase
    pub layout_ms: u32,
    /// Maximum time for paint phase
    pub paint_ms: u32,
    /// Total budget (may be less than sum of phases)
    pub total_ms: u32,
}

impl BrickBudget {
    /// Create a budget with equal distribution across phases
    #[must_use]
    pub const fn uniform(total_ms: u32) -> Self {
        let phase_ms = total_ms / 3;
        Self {
            measure_ms: phase_ms,
            layout_ms: phase_ms,
            paint_ms: phase_ms,
            total_ms,
        }
    }

    /// Create a custom budget with specified phase limits
    #[must_use]
    pub const fn new(measure_ms: u32, layout_ms: u32, paint_ms: u32) -> Self {
        Self {
            measure_ms,
            layout_ms,
            paint_ms,
            total_ms: measure_ms + layout_ms + paint_ms,
        }
    }

    /// Convert to Duration
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_millis(self.total_ms as u64)
    }
}

impl Default for BrickBudget {
    fn default() -> Self {
        // Default: 16ms total for 60fps
        Self::uniform(16)
    }
}

/// Result of verifying brick assertions
#[derive(Debug, Clone)]
pub struct BrickVerification {
    /// All assertions that passed
    pub passed: Vec<BrickAssertion>,
    /// All assertions that failed with reasons
    pub failed: Vec<(BrickAssertion, String)>,
    /// Time taken to verify
    pub verification_time: Duration,
}

impl BrickVerification {
    /// Check if all assertions passed
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.failed.is_empty()
    }

    /// Get the falsification score (passed / total)
    #[must_use]
    pub fn score(&self) -> f32 {
        let total = self.passed.len() + self.failed.len();
        if total == 0 {
            1.0
        } else {
            self.passed.len() as f32 / total as f32
        }
    }
}

/// Budget violation report
#[derive(Debug, Clone)]
pub struct BudgetViolation {
    /// Name of the brick that violated
    pub brick_name: String,
    /// Budget that was exceeded
    pub budget: BrickBudget,
    /// Actual time taken
    pub actual: Duration,
    /// Phase that exceeded (if known)
    pub phase: Option<BrickPhase>,
}

/// Rendering phase for budget tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrickPhase {
    /// Measure phase (compute intrinsic size)
    Measure,
    /// Layout phase (position children)
    Layout,
    /// Paint phase (generate draw commands)
    Paint,
}

/// Core Brick trait - the foundation of the Brick Architecture.
///
/// All UI components implement this trait. The trait defines:
/// 1. Assertions that must pass for the brick to be valid
/// 2. Performance budget that must not be exceeded
/// 3. HTML/CSS generation for rendering targets
///
/// # Trait Bound
///
/// Presentar's `Widget` trait requires `Brick`:
/// ```rust,ignore
/// pub trait Widget: Brick + Send + Sync { ... }
/// ```
///
/// This ensures every widget has verifiable assertions and budgets.
pub trait Brick: Send + Sync {
    /// Get the brick's unique type name
    fn brick_name(&self) -> &'static str;

    /// Get all assertions for this brick
    fn assertions(&self) -> &[BrickAssertion];

    /// Get the performance budget
    fn budget(&self) -> BrickBudget;

    /// Verify all assertions against current state
    ///
    /// Returns a verification result with passed/failed assertions.
    fn verify(&self) -> BrickVerification;

    /// Generate HTML for this brick (WASM target)
    ///
    /// Returns the HTML string that represents this brick.
    /// Must be deterministic (same state → same output).
    fn to_html(&self) -> String;

    /// Generate CSS for this brick (WASM target)
    ///
    /// Returns the CSS rules for styling this brick.
    /// Must be deterministic and scoped to avoid conflicts.
    fn to_css(&self) -> String;

    /// Get the test ID for DOM queries
    fn test_id(&self) -> Option<&str> {
        None
    }

    /// Check if this brick can be rendered (all assertions pass)
    fn can_render(&self) -> bool {
        self.verify().is_valid()
    }
}

/// Yuan Gate: Zero-swallow error handling for bricks
///
/// Named after the Yuan dynasty's strict quality standards.
/// Every error must be explicitly handled - no silent drops.
#[derive(Debug, Clone)]
pub enum BrickError {
    /// Assertion failed during verification
    AssertionFailed {
        /// The assertion that failed
        assertion: BrickAssertion,
        /// Reason for failure
        reason: String,
    },

    /// Budget exceeded during rendering
    BudgetExceeded(BudgetViolation),

    /// Invalid state transition
    InvalidTransition {
        /// Current state
        from: String,
        /// Attempted target state
        to: String,
        /// Reason transition is invalid
        reason: String,
    },

    /// Missing required child brick
    MissingChild {
        /// Expected child brick name
        expected: String,
    },

    /// HTML generation failed
    HtmlGenerationFailed {
        /// Reason for failure
        reason: String,
    },
}

impl std::fmt::Display for BrickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionFailed { assertion, reason } => {
                write!(f, "Assertion {assertion:?} failed: {reason}")
            }
            Self::BudgetExceeded(violation) => {
                write!(
                    f,
                    "Budget exceeded for {}: {:?} > {:?}",
                    violation.brick_name, violation.actual, violation.budget.total_ms
                )
            }
            Self::InvalidTransition { from, to, reason } => {
                write!(f, "Invalid transition {from} -> {to}: {reason}")
            }
            Self::MissingChild { expected } => {
                write!(f, "Missing required child brick: {expected}")
            }
            Self::HtmlGenerationFailed { reason } => {
                write!(f, "HTML generation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for BrickError {}

/// Result type for brick operations
pub type BrickResult<T> = Result<T, BrickError>;

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBrick {
        text: String,
        visible: bool,
    }

    impl Brick for TestBrick {
        fn brick_name(&self) -> &'static str {
            "TestBrick"
        }

        fn assertions(&self) -> &[BrickAssertion] {
            &[
                BrickAssertion::TextVisible,
                BrickAssertion::ContrastRatio(4.5),
            ]
        }

        fn budget(&self) -> BrickBudget {
            BrickBudget::uniform(16)
        }

        fn verify(&self) -> BrickVerification {
            let mut passed = Vec::new();
            let mut failed = Vec::new();

            for assertion in self.assertions() {
                match assertion {
                    BrickAssertion::TextVisible => {
                        if self.visible && !self.text.is_empty() {
                            passed.push(assertion.clone());
                        } else {
                            failed.push((assertion.clone(), "Text not visible".into()));
                        }
                    }
                    BrickAssertion::ContrastRatio(_) => {
                        // Assume pass for test
                        passed.push(assertion.clone());
                    }
                    _ => passed.push(assertion.clone()),
                }
            }

            BrickVerification {
                passed,
                failed,
                verification_time: Duration::from_micros(100),
            }
        }

        fn to_html(&self) -> String {
            format!(r#"<div class="test-brick">{}</div>"#, self.text)
        }

        fn to_css(&self) -> String {
            ".test-brick { color: #fff; background: #000; }".into()
        }
    }

    #[test]
    fn test_brick_verification_passes() {
        let brick = TestBrick {
            text: "Hello".into(),
            visible: true,
        };

        let result = brick.verify();
        assert!(result.is_valid());
        assert_eq!(result.score(), 1.0);
    }

    #[test]
    fn test_brick_verification_fails() {
        let brick = TestBrick {
            text: String::new(),
            visible: false,
        };

        let result = brick.verify();
        assert!(!result.is_valid());
        assert!(result.score() < 1.0);
    }

    #[test]
    fn test_budget_uniform() {
        let budget = BrickBudget::uniform(30);
        assert_eq!(budget.total_ms, 30);
        assert_eq!(budget.measure_ms, 10);
    }

    #[test]
    fn test_can_render_valid() {
        let brick = TestBrick {
            text: "Hello".into(),
            visible: true,
        };
        assert!(brick.can_render());
    }

    #[test]
    fn test_can_render_invalid() {
        let brick = TestBrick {
            text: String::new(),
            visible: false,
        };
        assert!(!brick.can_render());
    }

    #[test]
    fn test_brick_assertion_constructors() {
        let text_vis = BrickAssertion::text_visible();
        assert!(matches!(text_vis, BrickAssertion::TextVisible));

        let contrast = BrickAssertion::contrast_ratio(4.5);
        assert!(
            matches!(contrast, BrickAssertion::ContrastRatio(r) if (r - 4.5).abs() < f32::EPSILON)
        );

        let latency = BrickAssertion::max_latency_ms(100);
        assert!(matches!(latency, BrickAssertion::MaxLatencyMs(100)));

        let elem = BrickAssertion::element_present("div.test");
        assert!(matches!(elem, BrickAssertion::ElementPresent(s) if s == "div.test"));
    }

    #[test]
    fn test_brick_assertion_focusable() {
        let focusable = BrickAssertion::Focusable;
        assert!(matches!(focusable, BrickAssertion::Focusable));
    }

    #[test]
    fn test_brick_assertion_custom() {
        let custom = BrickAssertion::Custom {
            name: "test_assertion".into(),
            validator_id: 42,
        };
        match custom {
            BrickAssertion::Custom { name, validator_id } => {
                assert_eq!(name, "test_assertion");
                assert_eq!(validator_id, 42);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_budget_new() {
        let budget = BrickBudget::new(5, 10, 15);
        assert_eq!(budget.measure_ms, 5);
        assert_eq!(budget.layout_ms, 10);
        assert_eq!(budget.paint_ms, 15);
        assert_eq!(budget.total_ms, 30);
    }

    #[test]
    fn test_budget_default() {
        let budget = BrickBudget::default();
        assert_eq!(budget.total_ms, 16); // 60fps
    }

    #[test]
    fn test_budget_as_duration() {
        let budget = BrickBudget::uniform(100);
        let duration = budget.as_duration();
        assert_eq!(duration, Duration::from_millis(100));
    }

    #[test]
    fn test_verification_score_empty() {
        let verification = BrickVerification {
            passed: vec![],
            failed: vec![],
            verification_time: Duration::from_micros(10),
        };
        assert_eq!(verification.score(), 1.0); // Empty = perfect score
        assert!(verification.is_valid());
    }

    #[test]
    fn test_verification_score_partial() {
        let verification = BrickVerification {
            passed: vec![BrickAssertion::TextVisible],
            failed: vec![(BrickAssertion::Focusable, "Not focusable".into())],
            verification_time: Duration::from_micros(10),
        };
        assert_eq!(verification.score(), 0.5);
        assert!(!verification.is_valid());
    }

    #[test]
    fn test_brick_phase_variants() {
        let measure = BrickPhase::Measure;
        let layout = BrickPhase::Layout;
        let paint = BrickPhase::Paint;

        assert!(matches!(measure, BrickPhase::Measure));
        assert!(matches!(layout, BrickPhase::Layout));
        assert!(matches!(paint, BrickPhase::Paint));
        assert_ne!(measure, layout);
    }

    #[test]
    fn test_budget_violation() {
        let violation = BudgetViolation {
            brick_name: "TestBrick".into(),
            budget: BrickBudget::uniform(16),
            actual: Duration::from_millis(50),
            phase: Some(BrickPhase::Paint),
        };
        assert_eq!(violation.brick_name, "TestBrick");
        assert_eq!(violation.phase, Some(BrickPhase::Paint));
    }

    #[test]
    fn test_brick_to_html_css() {
        let brick = TestBrick {
            text: "Test".into(),
            visible: true,
        };
        let html = brick.to_html();
        let css = brick.to_css();

        assert!(html.contains("test-brick"));
        assert!(html.contains("Test"));
        assert!(css.contains(".test-brick"));
    }

    #[test]
    fn test_brick_name() {
        let brick = TestBrick {
            text: "Test".into(),
            visible: true,
        };
        assert_eq!(brick.brick_name(), "TestBrick");
    }

    #[test]
    fn test_brick_assertions_list() {
        let brick = TestBrick {
            text: "Test".into(),
            visible: true,
        };
        let assertions = brick.assertions();
        assert_eq!(assertions.len(), 2);
        assert!(matches!(assertions[0], BrickAssertion::TextVisible));
    }

    #[test]
    fn test_brick_budget_method() {
        let brick = TestBrick {
            text: "Test".into(),
            visible: true,
        };
        let budget = brick.budget();
        assert_eq!(budget.total_ms, 16);
    }
}
