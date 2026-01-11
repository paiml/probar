//! ComputeBlock Testing Utilities (PROBAR-SPEC-009)
//!
//! Test utilities for validating ComputeBlock implementations from presentar-terminal.
//! Provides assertions for SIMD-optimized panel elements like sparklines and gauges.
//!
//! ## Architecture
//!
//! ```text
//! ComputeBlock (presentar-terminal)
//!     ↓
//! ComputeBlockAssertion (probar)
//!     ↓
//! Test Results (pass/fail with diagnostics)
//! ```
//!
//! ## Toyota Way Application
//!
//! - **Jidoka**: Fail-fast on latency budget violations
//! - **Poka-Yoke**: Type-safe assertions prevent invalid tests
//! - **Muda**: Zero-copy assertions where possible
//!
//! ## Example
//!
//! ```ignore
//! use jugar_probar::tui::ComputeBlockAssertion;
//! use presentar_terminal::SparklineBlock;
//!
//! let mut block = SparklineBlock::new(60);
//! block.push(50.0);
//!
//! ComputeBlockAssertion::new(&block)
//!     .to_have_simd_support()
//!     .to_have_latency_under(100)
//!     .to_produce_valid_output();
//! ```

use std::fmt;
use std::time::{Duration, Instant};

#[cfg(feature = "compute-blocks")]
use presentar_terminal::{ComputeBlock, SimdInstructionSet};

/// Error when ComputeBlock latency exceeds budget.
#[derive(Debug, Clone)]
pub struct LatencyBudgetError {
    /// Block identifier
    pub block_id: String,
    /// Actual latency in microseconds
    pub actual_us: u64,
    /// Budget in microseconds
    pub budget_us: u64,
    /// SIMD instruction set used
    pub simd: String,
}

impl fmt::Display for LatencyBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ComputeBlock '{}' exceeded latency budget: {}μs > {}μs (SIMD: {})",
            self.block_id, self.actual_us, self.budget_us, self.simd
        )
    }
}

impl std::error::Error for LatencyBudgetError {}

/// Error when SIMD support is required but not available.
#[derive(Debug, Clone)]
pub struct SimdNotAvailableError {
    /// Block identifier
    pub block_id: String,
    /// Required instruction set
    pub required: String,
    /// Detected instruction set
    pub detected: String,
}

impl fmt::Display for SimdNotAvailableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ComputeBlock '{}' requires {} but only {} available",
            self.block_id, self.required, self.detected
        )
    }
}

impl std::error::Error for SimdNotAvailableError {}

/// ComputeBlock test assertion builder (Playwright-style API).
///
/// Provides fluent assertions for testing ComputeBlock implementations.
#[derive(Debug)]
pub struct ComputeBlockAssertion<'a, B> {
    block: &'a B,
    soft: bool,
    errors: Vec<String>,
}

#[cfg(feature = "compute-blocks")]
#[allow(clippy::panic)] // Intentional panics for test assertions
impl<'a, B: ComputeBlock> ComputeBlockAssertion<'a, B> {
    /// Create a new ComputeBlock assertion.
    pub fn new(block: &'a B) -> Self {
        Self {
            block,
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

    /// Assert the block has SIMD support available.
    pub fn to_have_simd_support(&mut self) -> &mut Self {
        if !self.block.simd_supported() {
            let msg = "ComputeBlock does not have SIMD support".to_string();
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Assert the block's latency budget is under the given microseconds.
    pub fn to_have_latency_under(&mut self, max_us: u64) -> &mut Self {
        let budget = self.block.latency_budget_us();
        if budget > max_us {
            let msg = format!(
                "ComputeBlock latency budget {}μs exceeds limit {}μs",
                budget, max_us
            );
            if self.soft {
                self.errors.push(msg);
            } else {
                panic!("{}", msg);
            }
        }
        self
    }

    /// Assert the block uses at least the given SIMD instruction set.
    pub fn to_use_simd(&mut self, min_set: SimdInstructionSet) -> &mut Self {
        let actual = self.block.simd_instruction_set();
        if actual.vector_width() < min_set.vector_width() {
            let msg = format!("ComputeBlock uses {:?} but {:?} required", actual, min_set);
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

    /// Assert no errors were collected.
    pub fn assert_no_errors(&self) {
        if !self.errors.is_empty() {
            panic!(
                "ComputeBlock had {} soft assertion failures:\n{}",
                self.errors.len(),
                self.errors.join("\n")
            );
        }
    }
}

/// Assert a ComputeBlock's latency is within budget.
///
/// Measures actual execution time and compares to the block's latency budget.
///
/// ## Example
///
/// ```ignore
/// use jugar_probar::tui::assert_compute_latency;
///
/// let mut block = SparklineBlock::new(60);
/// assert_compute_latency(&mut block, &50.0).unwrap();
/// ```
#[cfg(feature = "compute-blocks")]
pub fn assert_compute_latency<B: ComputeBlock>(
    block: &mut B,
    input: &B::Input,
) -> Result<Duration, LatencyBudgetError> {
    let budget_us = block.latency_budget_us();
    let simd = block.simd_instruction_set();

    let start = Instant::now();
    let _ = block.compute(input);
    let duration = start.elapsed();
    let actual_us = duration.as_micros() as u64;

    if actual_us <= budget_us {
        Ok(duration)
    } else {
        Err(LatencyBudgetError {
            block_id: format!("{:?}", simd),
            actual_us,
            budget_us,
            simd: simd.name().to_string(),
        })
    }
}

/// Assert SIMD is available at the required level.
///
/// ## Example
///
/// ```ignore
/// use jugar_probar::tui::assert_simd_available;
/// use presentar_terminal::SimdInstructionSet;
///
/// assert_simd_available(SimdInstructionSet::Avx2).unwrap();
/// ```
#[cfg(feature = "compute-blocks")]
pub fn assert_simd_available(
    required: SimdInstructionSet,
) -> Result<SimdInstructionSet, SimdNotAvailableError> {
    let detected = SimdInstructionSet::detect();
    if detected.vector_width() >= required.vector_width() {
        Ok(detected)
    } else {
        Err(SimdNotAvailableError {
            block_id: "system".to_string(),
            required: required.name().to_string(),
            detected: detected.name().to_string(),
        })
    }
}

/// Get the detected SIMD instruction set.
#[cfg(feature = "compute-blocks")]
pub fn detect_simd() -> SimdInstructionSet {
    SimdInstructionSet::detect()
}

/// Check if SIMD acceleration is available.
#[cfg(feature = "compute-blocks")]
pub fn simd_available() -> bool {
    SimdInstructionSet::detect().vector_width() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_budget_error_display() {
        let err = LatencyBudgetError {
            block_id: "sparkline".to_string(),
            actual_us: 150,
            budget_us: 100,
            simd: "AVX2".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("sparkline"));
        assert!(display.contains("150"));
        assert!(display.contains("100"));
        assert!(display.contains("AVX2"));
    }

    #[test]
    fn test_simd_not_available_error_display() {
        let err = SimdNotAvailableError {
            block_id: "test".to_string(),
            required: "AVX2".to_string(),
            detected: "Scalar".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("AVX2"));
        assert!(display.contains("Scalar"));
    }
}
