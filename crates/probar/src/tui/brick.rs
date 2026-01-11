//! Brick Testing Utilities (PROBAR-SPEC-009)
//!
//! Test utilities for validating Brick implementations from presentar.
//! Provides Playwright-style assertions for Brick verification gates.
//!
//! ## Toyota Way Application
//!
//! - **Jidoka**: Fail-fast on Brick verification failures
//! - **Poka-Yoke**: Type-safe assertion methods prevent invalid tests
//! - **Genchi Genbutsu**: Test actual Brick output, not mocked data
//!
//! ## Example
//!
//! ```ignore
//! use jugar_probar::tui::{BrickAssertion, assert_brick_valid};
//! use presentar_terminal::SparklineBlock;
//!
//! let block = SparklineBlock::new(60);
//! assert_brick_valid(&block).unwrap();
//! ```

use std::fmt;
use std::time::{Duration, Instant};

#[cfg(feature = "compute-blocks")]
use presentar_core::Brick;

/// Error returned when Brick verification fails.
#[derive(Debug, Clone)]
pub struct BrickVerificationError {
    /// Name of the Brick that failed
    pub brick_name: String,
    /// Failed assertions with reasons
    pub failures: Vec<(String, String)>,
    /// Verification duration
    pub duration: Duration,
}

impl fmt::Display for BrickVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Brick '{}' verification failed:", self.brick_name)?;
        for (assertion, reason) in &self.failures {
            writeln!(f, "  - {}: {}", assertion, reason)?;
        }
        writeln!(f, "  (verified in {:?})", self.duration)?;
        Ok(())
    }
}

impl std::error::Error for BrickVerificationError {}

/// Error returned when Brick exceeds its performance budget.
#[derive(Debug, Clone)]
pub struct BudgetExceededError {
    /// Name of the Brick
    pub brick_name: String,
    /// Budget phase that exceeded (collect/layout/render)
    pub phase: String,
    /// Actual duration
    pub actual_ms: f64,
    /// Budget limit
    pub budget_ms: f64,
}

impl fmt::Display for BudgetExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Brick '{}' exceeded {} budget: {:.2}ms > {:.2}ms",
            self.brick_name, self.phase, self.actual_ms, self.budget_ms
        )
    }
}

impl std::error::Error for BudgetExceededError {}

/// Result of a single assertion check.
#[derive(Debug, Clone)]
pub struct BrickAssertionResult {
    /// Assertion name/description
    pub name: String,
    /// Whether the assertion passed
    pub passed: bool,
    /// Failure reason if applicable
    pub reason: Option<String>,
}

/// Brick test assertion builder (Playwright-style API).
///
/// Provides fluent assertions for testing Brick implementations.
///
/// ## Example
///
/// ```ignore
/// use jugar_probar::tui::BrickTestAssertion;
///
/// BrickTestAssertion::new(&my_brick)
///     .to_be_valid()
///     .to_have_budget_under(16)
///     .to_pass_all_assertions();
/// ```
#[derive(Debug)]
pub struct BrickTestAssertion<'a, B> {
    brick: &'a B,
    soft: bool,
    errors: Vec<String>,
}

#[cfg(feature = "compute-blocks")]
#[allow(clippy::panic)] // Intentional panics for test assertions
impl<'a, B: Brick> BrickTestAssertion<'a, B> {
    /// Create a new Brick assertion.
    pub fn new(brick: &'a B) -> Self {
        Self {
            brick,
            soft: false,
            errors: Vec::new(),
        }
    }

    /// Enable soft assertions (collect errors instead of failing immediately).
    #[must_use]
    pub fn soft(mut self) -> Self {
        self.soft = true;
        self
    }

    /// Assert the Brick passes verification.
    pub fn to_be_valid(&mut self) -> &mut Self {
        let verification = self.brick.verify();
        if !verification.is_valid() {
            let msg = format!(
                "Brick '{}' failed verification: {:?}",
                self.brick.brick_name(),
                &verification.failed
            );
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Assert the Brick's total budget is under the given milliseconds.
    pub fn to_have_budget_under(&mut self, max_ms: u32) -> &mut Self {
        let budget = self.brick.budget();
        let total = budget.total_ms;
        if total > max_ms {
            let msg = format!(
                "Brick '{}' budget {}ms exceeds limit {}ms",
                self.brick.brick_name(),
                total,
                max_ms
            );
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Assert all Brick assertions pass.
    pub fn to_pass_all_assertions(&mut self) -> &mut Self {
        let verification = self.brick.verify();
        let failed = &verification.failed;
        if !failed.is_empty() {
            let msg = format!(
                "Brick '{}' has {} failed assertions: {:?}",
                self.brick.brick_name(),
                failed.len(),
                failed
            );
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Assert the Brick can render (Jidoka gate passes).
    pub fn to_be_renderable(&mut self) -> &mut Self {
        if !self.brick.can_render() {
            let msg = format!(
                "Brick '{}' cannot render (Jidoka gate failed)",
                self.brick.brick_name()
            );
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Get collected errors (for soft assertions).
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Assert no errors were collected (for soft assertions).
    pub fn assert_no_errors(&self) {
        if !self.errors.is_empty() {
            panic!(
                "Brick '{}' had {} soft assertion failures:\n{}",
                self.brick.brick_name(),
                self.errors.len(),
                self.errors.join("\n")
            );
        }
    }
}

/// Assert a Brick passes verification.
///
/// ## Example
///
/// ```ignore
/// use jugar_probar::tui::assert_brick_valid;
///
/// assert_brick_valid(&my_brick).unwrap();
/// ```
#[cfg(feature = "compute-blocks")]
pub fn assert_brick_valid<B: Brick>(brick: &B) -> Result<(), BrickVerificationError> {
    let start = Instant::now();
    let verification = brick.verify();
    let duration = start.elapsed();

    if verification.is_valid() {
        Ok(())
    } else {
        Err(BrickVerificationError {
            brick_name: brick.brick_name().to_string(),
            failures: verification
                .failed
                .iter()
                .map(|(a, r)| (format!("{:?}", a), r.clone()))
                .collect(),
            duration,
        })
    }
}

/// Assert a Brick's execution time is within budget.
///
/// ## Example
///
/// ```ignore
/// use jugar_probar::tui::assert_brick_budget;
///
/// assert_brick_budget(&my_brick, || {
///     my_brick.render();
/// }, "render").unwrap();
/// ```
#[cfg(feature = "compute-blocks")]
pub fn assert_brick_budget<B: Brick, F: FnOnce()>(
    brick: &B,
    operation: F,
    phase: &str,
) -> Result<Duration, BudgetExceededError> {
    let budget = brick.budget();
    let limit_ms = match phase {
        "measure" => budget.measure_ms as f64,
        "layout" => budget.layout_ms as f64,
        "paint" => budget.paint_ms as f64,
        "total" => budget.total_ms as f64,
        _ => budget.total_ms as f64,
    };

    let start = Instant::now();
    operation();
    let duration = start.elapsed();
    let actual_ms = duration.as_secs_f64() * 1000.0;

    if actual_ms <= limit_ms {
        Ok(duration)
    } else {
        Err(BudgetExceededError {
            brick_name: brick.brick_name().to_string(),
            phase: phase.to_string(),
            actual_ms,
            budget_ms: limit_ms,
        })
    }
}

/// Measure Brick verification score (0.0 - 1.0).
///
/// Returns the ratio of passed assertions to total assertions.
#[cfg(feature = "compute-blocks")]
pub fn brick_verification_score<B: Brick>(brick: &B) -> f64 {
    let verification = brick.verify();
    f64::from(verification.score())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brick_verification_error_display() {
        let err = BrickVerificationError {
            brick_name: "TestBrick".to_string(),
            failures: vec![("MinWidth".to_string(), "too narrow".to_string())],
            duration: Duration::from_millis(5),
        };
        let display = format!("{}", err);
        assert!(display.contains("TestBrick"));
        assert!(display.contains("MinWidth"));
        assert!(display.contains("too narrow"));
    }

    #[test]
    fn test_budget_exceeded_error_display() {
        let err = BudgetExceededError {
            brick_name: "TestBrick".to_string(),
            phase: "render".to_string(),
            actual_ms: 20.5,
            budget_ms: 16.0,
        };
        let display = format!("{}", err);
        assert!(display.contains("TestBrick"));
        assert!(display.contains("render"));
        assert!(display.contains("20.50"));
        assert!(display.contains("16.00"));
    }

    #[test]
    fn test_assertion_result() {
        let result = BrickAssertionResult {
            name: "MinWidth".to_string(),
            passed: true,
            reason: None,
        };
        assert!(result.passed);
        assert!(result.reason.is_none());
    }
}
