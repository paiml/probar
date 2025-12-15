//! GPU Pixel Testing: Atomic verification of CUDA kernel correctness
//!
//! Inspired by pixel-level GUI testing, GPU pixel testing verifies the smallest
//! testable units of GPU kernel behavior to catch bugs like:
//! - Shared memory addressing (u64 vs u32)
//! - Loop branch direction (START vs END)
//! - Kernel name mismatches
//! - Tile/thread bounds violations
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    GPU Pixel Testing                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │   ┌────────────┐    ┌────────────┐    ┌────────────┐            │
//! │   │ PTX Static │    │ Kernel     │    │ Regression │            │
//! │   │ Analysis   │───►│ Pixel      │───►│ Detection  │            │
//! │   │            │    │ Tests      │    │            │            │
//! │   └────────────┘    └────────────┘    └────────────┘            │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

mod kernel_pixels;
mod ptx_analysis;
mod regression;

pub use kernel_pixels::{
    standard_pixel_tests, GpuPixelResult, GpuPixelTest, GpuPixelTestSuite, KernelPixelConfig,
};
pub use ptx_analysis::{PtxAnalyzer, PtxBug, PtxBugClass, PtxValidationResult};
pub use regression::{
    run_regression_suite, GpuRegressionSuite, RegressionConfig, RegressionResult,
};

/// Quick validation of PTX for common GPU kernel bugs
///
/// # Example
/// ```ignore
/// use probar::gpu_pixels::validate_ptx;
///
/// let ptx = kernel.emit_ptx();
/// let result = validate_ptx(&ptx);
/// assert!(result.is_valid(), "PTX bugs: {:?}", result.bugs);
/// ```
#[must_use]
pub fn validate_ptx(ptx: &str) -> PtxValidationResult {
    PtxAnalyzer::default().analyze(ptx)
}

/// Run all GPU pixel tests for a kernel
///
/// # Example
/// ```ignore
/// use probar::gpu_pixels::run_kernel_pixels;
///
/// let results = run_kernel_pixels("gemm_tiled", &ptx, &config);
/// assert!(results.all_passed());
/// ```
pub fn run_kernel_pixels(
    kernel_name: &str,
    ptx: &str,
    config: &KernelPixelConfig,
) -> GpuPixelTestSuite {
    let mut suite = GpuPixelTestSuite::new(kernel_name);

    // PTX static analysis
    let ptx_result = validate_ptx(ptx);
    suite.add_result(GpuPixelResult::from_ptx_validation(&ptx_result));

    // Kernel-specific pixel tests
    suite.run_kernel_pixels(ptx, config);

    suite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ptx_empty() {
        let result = validate_ptx("");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_ptx_minimal_valid() {
        let ptx = r#"
.version 8.0
.target sm_70
.address_size 64

.visible .entry test_kernel() {
    ret;
}
"#;
        let result = validate_ptx(ptx);
        assert!(result.is_valid());
    }

    #[test]
    fn test_shared_memory_u64_bug_detected() {
        let ptx = r#"
.version 8.0
.target sm_70
.address_size 64

.visible .entry buggy_kernel() {
    .reg .u64 %rd<5>;
    .reg .f32 %f<2>;
    .shared .align 16 .b8 smem[4096];

    st.shared.f32 [%rd0], %f0;
    ret;
}
"#;
        let result = validate_ptx(ptx);
        assert!(!result.is_valid());
        assert!(result
            .bugs
            .iter()
            .any(|b| matches!(b.class, PtxBugClass::SharedMemU64Addressing)));
    }

    #[test]
    fn test_shared_memory_u32_valid() {
        let ptx = r#"
.version 8.0
.target sm_70
.address_size 64

.visible .entry valid_kernel() {
    .reg .u32 %r<5>;
    .reg .f32 %f<2>;
    .shared .align 16 .b8 smem[4096];

    st.shared.f32 [%r0], %f0;
    ret;
}
"#;
        let result = validate_ptx(ptx);
        // Should not have shared memory addressing bug
        assert!(!result
            .bugs
            .iter()
            .any(|b| matches!(b.class, PtxBugClass::SharedMemU64Addressing)));
    }
}
