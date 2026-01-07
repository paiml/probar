//! WASM Threading Compliance Checker
//!
//! Per `PROBAR-SPEC-WASM-001` Section 4.1, this module checks for compliance
//! with WASM threading best practices.
//!
//! ## Compliance Checks
//!
//! | Check ID | Description | Required |
//! |----------|-------------|----------|
//! | WASM-COMPLY-001 | State sync lint passes | Yes |
//! | WASM-COMPLY-002 | Mock runtime tests exist | Yes |
//! | WASM-COMPLY-003 | Property tests on actual code | Warning |
//! | WASM-COMPLY-004 | Regression tests for known bugs | Yes |

use crate::lint::StateSyncLinter;
use std::path::Path;

/// Status of a compliance check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceStatus {
    /// Check passed
    Pass,
    /// Check failed (blocking)
    Fail,
    /// Check has warnings (non-blocking)
    Warn,
    /// Check was skipped
    Skip,
}

impl std::fmt::Display for ComplianceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Warn => write!(f, "WARN"),
            Self::Skip => write!(f, "SKIP"),
        }
    }
}

/// A single compliance check result
#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    /// Check identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Status
    pub status: ComplianceStatus,
    /// Details or error message
    pub details: Option<String>,
    /// Count of issues found (if applicable)
    pub issue_count: usize,
}

impl ComplianceCheck {
    /// Create a passing check
    #[must_use]
    pub fn pass(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: ComplianceStatus::Pass,
            details: None,
            issue_count: 0,
        }
    }

    /// Create a failing check
    #[must_use]
    pub fn fail(id: &str, name: &str, details: &str, count: usize) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: ComplianceStatus::Fail,
            details: Some(details.to_string()),
            issue_count: count,
        }
    }

    /// Create a warning check
    #[must_use]
    pub fn warn(id: &str, name: &str, details: &str, count: usize) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: ComplianceStatus::Warn,
            details: Some(details.to_string()),
            issue_count: count,
        }
    }

    /// Create a skipped check
    #[must_use]
    pub fn skip(id: &str, name: &str, reason: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: ComplianceStatus::Skip,
            details: Some(reason.to_string()),
            issue_count: 0,
        }
    }
}

/// Result of compliance checking
#[derive(Debug, Default)]
pub struct ComplianceResult {
    /// Individual check results
    pub checks: Vec<ComplianceCheck>,
    /// Overall compliance status
    pub compliant: bool,
    /// Total files analyzed
    pub files_analyzed: usize,
}

impl ComplianceResult {
    /// Create a new result
    #[must_use]
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            compliant: true,
            files_analyzed: 0,
        }
    }

    /// Add a check result
    pub fn add_check(&mut self, check: ComplianceCheck) {
        if check.status == ComplianceStatus::Fail {
            self.compliant = false;
        }
        self.checks.push(check);
    }

    /// Get pass count
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == ComplianceStatus::Pass)
            .count()
    }

    /// Get fail count
    #[must_use]
    pub fn fail_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == ComplianceStatus::Fail)
            .count()
    }

    /// Get warn count
    #[must_use]
    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == ComplianceStatus::Warn)
            .count()
    }

    /// Get summary string
    #[must_use]
    pub fn summary(&self) -> String {
        let total = self.checks.len();
        let pass = self.pass_count();
        let fail = self.fail_count();
        let warn = self.warn_count();

        let status = if self.compliant {
            if warn > 0 {
                "COMPLIANT (with warnings)"
            } else {
                "COMPLIANT"
            }
        } else {
            "NON-COMPLIANT"
        };

        format!("{status}: {pass}/{total} passed, {fail} failed, {warn} warnings")
    }
}

/// WASM Threading Compliance Checker
///
/// Checks for compliance with WASM threading best practices per
/// `PROBAR-SPEC-WASM-001`.
#[derive(Debug, Default)]
pub struct WasmThreadingCompliance {
    /// State sync linter
    linter: StateSyncLinter,
}

impl WasmThreadingCompliance {
    /// Create a new compliance checker
    #[must_use]
    pub fn new() -> Self {
        Self {
            linter: StateSyncLinter::new(),
        }
    }

    /// Check compliance for a project directory
    pub fn check(&mut self, project_path: &Path) -> ComplianceResult {
        let mut result = ComplianceResult::new();

        // WASM-COMPLY-001: State sync lint
        self.check_state_sync_lint(project_path, &mut result);

        // WASM-COMPLY-002: Mock runtime tests
        self.check_mock_runtime_tests(project_path, &mut result);

        // WASM-COMPLY-003: Property tests on actual code (warning only)
        self.check_property_tests(project_path, &mut result);

        // WASM-COMPLY-004: Regression tests
        self.check_regression_tests(project_path, &mut result);

        result
    }

    /// Check state sync lint (WASM-COMPLY-001)
    fn check_state_sync_lint(&mut self, project_path: &Path, result: &mut ComplianceResult) {
        let src_path = project_path.join("src");
        let lint_path = if src_path.exists() {
            src_path
        } else {
            project_path.to_path_buf()
        };

        match self.linter.lint_directory(&lint_path) {
            Ok(report) => {
                result.files_analyzed = report.files_analyzed;

                let error_count = report.error_count();
                if error_count > 0 {
                    result.add_check(ComplianceCheck::fail(
                        "WASM-COMPLY-001",
                        "State sync lint",
                        &format!("{} state sync errors found", error_count),
                        error_count,
                    ));
                } else {
                    result.add_check(ComplianceCheck::pass("WASM-COMPLY-001", "State sync lint"));
                }
            }
            Err(e) => {
                result.add_check(ComplianceCheck::skip(
                    "WASM-COMPLY-001",
                    "State sync lint",
                    &format!("Failed to run lint: {e}"),
                ));
            }
        }
    }

    /// Check for mock runtime tests (WASM-COMPLY-002)
    fn check_mock_runtime_tests(&self, project_path: &Path, result: &mut ComplianceResult) {
        let tests_path = project_path.join("tests");
        let src_path = project_path.join("src");

        let mut mock_test_count = 0;

        // Search for MockWasmRuntime or WasmCallbackTestHarness usage
        let search_patterns = [
            "MockWasmRuntime",
            "WasmCallbackTestHarness",
            "MockableWorker",
        ];

        for pattern in &search_patterns {
            mock_test_count += count_pattern_in_dir(&tests_path, pattern);
            mock_test_count += count_pattern_in_dir(&src_path, pattern);
        }

        if mock_test_count > 0 {
            result.add_check(ComplianceCheck::pass(
                "WASM-COMPLY-002",
                "Mock runtime tests",
            ));
        } else {
            result.add_check(ComplianceCheck::fail(
                "WASM-COMPLY-002",
                "Mock runtime tests",
                "No mock runtime tests found (use MockWasmRuntime or WasmCallbackTestHarness)",
                0,
            ));
        }
    }

    /// Check for property tests on actual code (WASM-COMPLY-003)
    fn check_property_tests(&self, project_path: &Path, result: &mut ComplianceResult) {
        let tests_path = project_path.join("tests");
        let src_path = project_path.join("src");

        // Look for proptest! macro usage
        let proptest_count = count_pattern_in_dir(&tests_path, "proptest!")
            + count_pattern_in_dir(&src_path, "proptest!");

        // Check if tests also use mock runtime (testing actual code)
        let mock_in_proptest = count_pattern_in_dir(&tests_path, "MockWasmRuntime")
            + count_pattern_in_dir(&tests_path, "WasmCallbackTestHarness");

        if proptest_count == 0 {
            result.add_check(ComplianceCheck::warn(
                "WASM-COMPLY-003",
                "Property tests on actual code",
                "No proptest! blocks found - consider adding property-based tests",
                0,
            ));
        } else if mock_in_proptest == 0 {
            result.add_check(ComplianceCheck::warn(
                "WASM-COMPLY-003",
                "Property tests on actual code",
                "Property tests found but may be testing models instead of actual code",
                proptest_count,
            ));
        } else {
            result.add_check(ComplianceCheck::pass(
                "WASM-COMPLY-003",
                "Property tests on actual code",
            ));
        }
    }

    /// Check for regression tests (WASM-COMPLY-004)
    fn check_regression_tests(&self, project_path: &Path, result: &mut ComplianceResult) {
        let tests_path = project_path.join("tests");
        let src_path = project_path.join("src");

        // Required regression test markers
        let required_markers = [
            "WAPR-QA-REGRESSION-005",
            "WAPR-QA-REGRESSION-006",
            "WAPR-QA-REGRESSION-007",
            "regression_", // Generic regression test prefix
        ];

        let mut found_count = 0;
        for marker in &required_markers {
            if count_pattern_in_dir(&tests_path, marker) > 0
                || count_pattern_in_dir(&src_path, marker) > 0
            {
                found_count += 1;
            }
        }

        if found_count >= 3 {
            result.add_check(ComplianceCheck::pass(
                "WASM-COMPLY-004",
                "Regression tests for known bugs",
            ));
        } else {
            result.add_check(ComplianceCheck::fail(
                "WASM-COMPLY-004",
                "Regression tests for known bugs",
                &format!(
                    "Only {} of 3 required regression test markers found",
                    found_count
                ),
                3 - found_count,
            ));
        }
    }
}

/// Count occurrences of a pattern in all .rs files in a directory
fn count_pattern_in_dir(dir: &Path, pattern: &str) -> usize {
    fn visit(dir: &Path, pattern: &str, count: &mut usize) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "target" {
                        visit(&path, pattern, count);
                    }
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        *count += content.matches(pattern).count();
                    }
                }
            }
        }
    }

    let mut count = 0;
    if dir.exists() {
        visit(dir, pattern, &mut count);
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_compliance_status_display() {
        assert_eq!(ComplianceStatus::Pass.to_string(), "PASS");
        assert_eq!(ComplianceStatus::Fail.to_string(), "FAIL");
        assert_eq!(ComplianceStatus::Warn.to_string(), "WARN");
        assert_eq!(ComplianceStatus::Skip.to_string(), "SKIP");
    }

    #[test]
    fn test_compliance_check_constructors() {
        let pass = ComplianceCheck::pass("TEST-001", "Test check");
        assert_eq!(pass.status, ComplianceStatus::Pass);
        assert!(pass.details.is_none());

        let fail = ComplianceCheck::fail("TEST-002", "Failed check", "Error", 5);
        assert_eq!(fail.status, ComplianceStatus::Fail);
        assert_eq!(fail.issue_count, 5);

        let warn = ComplianceCheck::warn("TEST-003", "Warning check", "Warning", 2);
        assert_eq!(warn.status, ComplianceStatus::Warn);

        let skip = ComplianceCheck::skip("TEST-004", "Skipped check", "Reason");
        assert_eq!(skip.status, ComplianceStatus::Skip);
    }

    #[test]
    fn test_compliance_result_counts() {
        let mut result = ComplianceResult::new();
        result.add_check(ComplianceCheck::pass("TEST-001", "Pass"));
        result.add_check(ComplianceCheck::pass("TEST-002", "Pass"));
        result.add_check(ComplianceCheck::fail("TEST-003", "Fail", "Error", 1));
        result.add_check(ComplianceCheck::warn("TEST-004", "Warn", "Warning", 1));

        assert_eq!(result.pass_count(), 2);
        assert_eq!(result.fail_count(), 1);
        assert_eq!(result.warn_count(), 1);
        assert!(!result.compliant);
    }

    #[test]
    fn test_compliance_result_summary() {
        let mut result = ComplianceResult::new();
        result.add_check(ComplianceCheck::pass("TEST-001", "Pass"));
        result.add_check(ComplianceCheck::pass("TEST-002", "Pass"));

        let summary = result.summary();
        assert!(summary.contains("COMPLIANT"));
        assert!(summary.contains("2/2 passed"));
    }

    #[test]
    fn test_count_pattern_in_dir() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        fs::write(&test_file, "MockWasmRuntime MockWasmRuntime proptest!").unwrap();

        assert_eq!(count_pattern_in_dir(temp_dir.path(), "MockWasmRuntime"), 2);
        assert_eq!(count_pattern_in_dir(temp_dir.path(), "proptest!"), 1);
        assert_eq!(count_pattern_in_dir(temp_dir.path(), "nonexistent"), 0);
    }

    #[test]
    fn test_wasm_threading_compliance_check() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create a file with correct patterns
        let lib_file = src_dir.join("lib.rs");
        fs::write(
            &lib_file,
            r#"
use probar::mock::MockWasmRuntime;

// regression_test for state sync
fn regression_state_sync() {
    let runtime = MockWasmRuntime::new();
}

// WAPR-QA-REGRESSION-005
fn test_005() {}

// WAPR-QA-REGRESSION-006
fn test_006() {}

// WAPR-QA-REGRESSION-007
fn test_007() {}
"#,
        )
        .unwrap();

        let mut checker = WasmThreadingCompliance::new();
        let result = checker.check(temp_dir.path());

        // Should have run all 4 checks
        assert_eq!(result.checks.len(), 4);
    }
}
