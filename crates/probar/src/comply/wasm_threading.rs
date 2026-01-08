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
//! | WASM-COMPLY-005 | No JS files in target/ (post-build) | Yes |
//!
//! ## Tarantula Integration
//!
//! When proptest fails, run `probar comply --wasm-threading --lcov-passed <path> --lcov-failed <path>`
//! to generate a Tarantula Hotspot Report showing suspicious lines.

use crate::comply::tarantula::TarantulaEngine;
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
    /// Tarantula fault localization engine
    tarantula: TarantulaEngine,
    /// LCOV file for passing tests (optional)
    lcov_passed: Option<std::path::PathBuf>,
    /// LCOV file for failing tests (optional)
    lcov_failed: Option<std::path::PathBuf>,
}

impl WasmThreadingCompliance {
    /// Create a new compliance checker
    #[must_use]
    pub fn new() -> Self {
        Self {
            linter: StateSyncLinter::new(),
            tarantula: TarantulaEngine::new(),
            lcov_passed: None,
            lcov_failed: None,
        }
    }

    /// Set LCOV files for Tarantula analysis
    ///
    /// When both passed and failed coverage files are provided,
    /// Tarantula will generate a hotspot report.
    pub fn with_lcov(&mut self, passed: Option<&Path>, failed: Option<&Path>) -> &mut Self {
        self.lcov_passed = passed.map(|p| p.to_path_buf());
        self.lcov_failed = failed.map(|p| p.to_path_buf());
        self
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

        // WASM-COMPLY-005: Post-build JS file check
        self.check_target_js_files(project_path, &mut result);

        result
    }

    /// Generate Tarantula hotspot report if LCOV files are configured
    ///
    /// Returns the formatted report string, or None if no coverage data.
    pub fn tarantula_report(&mut self) -> Option<String> {
        // Load LCOV files if configured
        if let Some(ref passed_path) = self.lcov_passed {
            if let Err(e) = self.tarantula.parse_lcov(passed_path, true) {
                return Some(format!("Error parsing passed LCOV: {e}"));
            }
        }

        if let Some(ref failed_path) = self.lcov_failed {
            if let Err(e) = self.tarantula.parse_lcov(failed_path, false) {
                return Some(format!("Error parsing failed LCOV: {e}"));
            }
        }

        // Generate reports
        let reports = self.tarantula.generate_all_reports();
        if reports.is_empty() {
            return None;
        }

        let mut output = String::new();
        output.push_str("═══════════════════════════════════════════════════════════════════\n");
        output.push_str("                    🕷️  TARANTULA HOTSPOT REPORT\n");
        output.push_str("═══════════════════════════════════════════════════════════════════\n\n");

        for report in reports {
            output.push_str(&report.format_hotspot_report());
            output.push('\n');
        }

        Some(output)
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

    /// Check for JS files in target/ directory (WASM-COMPLY-005)
    ///
    /// This catches CI loopholes where build.rs writes JS to target/,
    /// bypassing WASM-only compliance checks.
    fn check_target_js_files(&self, project_path: &Path, result: &mut ComplianceResult) {
        let target_path = project_path.join("target");

        if !target_path.exists() {
            result.add_check(ComplianceCheck::skip(
                "WASM-COMPLY-005",
                "No JS files in target/",
                "No target/ directory found (run cargo build first)",
            ));
            return;
        }

        let js_files = find_js_files_in_target(&target_path);

        if js_files.is_empty() {
            result.add_check(ComplianceCheck::pass(
                "WASM-COMPLY-005",
                "No JS files in target/",
            ));
        } else {
            // Filter to only suspicious JS files (not from wasm-bindgen)
            let suspicious: Vec<_> = js_files
                .iter()
                .filter(|p| !is_wasm_bindgen_output(p))
                .collect();

            if suspicious.is_empty() {
                result.add_check(ComplianceCheck::pass(
                    "WASM-COMPLY-005",
                    "No JS files in target/",
                ));
            } else {
                let file_list: Vec<_> = suspicious
                    .iter()
                    .take(5)
                    .map(|p| p.display().to_string())
                    .collect();
                result.add_check(ComplianceCheck::fail(
                    "WASM-COMPLY-005",
                    "No JS files in target/",
                    &format!(
                        "Found {} JS file(s) in target/ (possible build.rs loophole): {}{}",
                        suspicious.len(),
                        file_list.join(", "),
                        if suspicious.len() > 5 { "..." } else { "" }
                    ),
                    suspicious.len(),
                ));
            }
        }
    }
}

/// Suspicious file found in target/ directory
#[derive(Debug, Clone)]
pub struct SuspiciousFile {
    /// Path to the suspicious file
    pub path: std::path::PathBuf,
    /// Reason it's suspicious
    pub reason: SuspiciousReason,
}

/// Why a file is considered suspicious
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuspiciousReason {
    /// File has .js extension
    JsExtension,
    /// File contains JavaScript-like content
    JsContent,
}

impl std::fmt::Display for SuspiciousReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsExtension => write!(f, ".js extension"),
            Self::JsContent => write!(f, "JS content detected"),
        }
    }
}

/// Find all suspicious files in the target directory
///
/// This includes:
/// - Files with .js extension
/// - Text files containing JavaScript-like content (MIME-type smuggling defense)
///
/// HOTFIX PROBAR-WASM-003: Now traverses hidden directories (no more .hidden bypass)
fn find_suspicious_files_in_target(target_path: &Path) -> Vec<SuspiciousFile> {
    fn visit(dir: &Path, suspicious: &mut Vec<SuspiciousFile>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // HOTFIX: Only skip node_modules, NOT hidden directories
                    // Hidden directories in target/ are suspicious by definition
                    if name != "node_modules" {
                        visit(&path, suspicious);
                    }
                } else {
                    // Check by extension
                    if path.extension().map(|e| e == "js").unwrap_or(false) {
                        suspicious.push(SuspiciousFile {
                            path,
                            reason: SuspiciousReason::JsExtension,
                        });
                    } else {
                        // HOTFIX: Content inspection for MIME-type smuggling
                        // Check non-.js files for JavaScript content
                        if let Some(reason) = check_file_for_js_content(&path) {
                            suspicious.push(SuspiciousFile { path, reason });
                        }
                    }
                }
            }
        }
    }

    let mut suspicious = Vec::new();
    if target_path.exists() {
        visit(target_path, &mut suspicious);
    }
    suspicious
}

/// Check if a file contains JavaScript-like content
///
/// Scans the first 2048 bytes for JS keywords to detect MIME-type smuggling.
fn check_file_for_js_content(path: &Path) -> Option<SuspiciousReason> {
    // Skip known safe binary extensions
    const SAFE_BINARY_EXTENSIONS: &[&str] = &[
        "wasm",
        "png",
        "jpg",
        "jpeg",
        "gif",
        "ico",
        "webp",
        "svg",
        "ttf",
        "woff",
        "woff2",
        "eot",
        "otf",
        "zip",
        "tar",
        "gz",
        "br",
        "zst",
        "rlib",
        "rmeta",
        "so",
        "dylib",
        "dll",
        "a",
        "o",
        "d",
        "fingerprint",
        "bin",
        "dat",
    ];

    // JS keyword patterns that indicate JavaScript content
    // Using word boundaries via simple checks
    const JS_KEYWORDS: &[&str] = &[
        "function ",
        "function(",
        "const ",
        "let ",
        "var ",
        "=> {",
        "=>{",
        "class ",
        "import ",
        "export ",
        "require(",
        "module.exports",
        "window.",
        "document.",
        "console.log",
        "addEventListener",
        "setTimeout(",
        "setInterval(",
        "Promise.",
        "async ",
        "await ",
    ];

    // Only check text-like files (skip binaries, images, etc.)
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if SAFE_BINARY_EXTENSIONS.contains(&ext) {
        return None;
    }

    // Skip files that are too large (likely not hand-written JS)
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > 10 * 1024 * 1024 {
        // > 10MB
        return None;
    }

    // Read first 2048 bytes
    let content = std::fs::read(path).ok()?;
    let sample_size = content.len().min(2048);
    let sample = &content[..sample_size];

    // Check if it looks like text (not binary)
    let is_text = sample.iter().all(|&b| {
        b.is_ascii_graphic() || b.is_ascii_whitespace() || b == b'\t' || b == b'\n' || b == b'\r'
    });

    if !is_text {
        return None;
    }

    // Convert to string and check for JS keywords
    let text = std::str::from_utf8(sample).ok()?;

    // Count how many JS keywords are found
    let keyword_count = JS_KEYWORDS.iter().filter(|kw| text.contains(*kw)).count();

    // If 2+ keywords found, it's likely JS content
    if keyword_count >= 2 {
        return Some(SuspiciousReason::JsContent);
    }

    None
}

/// Legacy wrapper for backward compatibility
fn find_js_files_in_target(target_path: &Path) -> Vec<std::path::PathBuf> {
    find_suspicious_files_in_target(target_path)
        .into_iter()
        .map(|s| s.path)
        .collect()
}

/// Check if a JS file is legitimate wasm-bindgen output
fn is_wasm_bindgen_output(path: &Path) -> bool {
    // wasm-bindgen outputs go into pkg/ or have specific naming patterns
    let path_str = path.display().to_string();

    // Legitimate patterns:
    // - /pkg/*.js (wasm-pack output)
    // - *_bg.js (wasm-bindgen background module)
    // - snippets/ directory (wasm-bindgen snippets)
    path_str.contains("/pkg/")
        || path_str.contains("_bg.js")
        || path_str.contains("/snippets/")
        || path_str.contains("wasm-bindgen")
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

        // Should have run all 5 checks
        assert_eq!(result.checks.len(), 5);
    }

    #[test]
    fn test_target_js_files_detection() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        // Create a suspicious JS file (not wasm-bindgen)
        let suspicious_js = target_dir.join("evil_backdoor.js");
        fs::write(&suspicious_js, "console.log('sneaky');").unwrap();

        let js_files = find_js_files_in_target(&target_dir);
        assert_eq!(js_files.len(), 1);
        assert!(!is_wasm_bindgen_output(&js_files[0]));
    }

    #[test]
    fn test_wasm_bindgen_output_detection() {
        // Legitimate wasm-bindgen outputs
        assert!(is_wasm_bindgen_output(Path::new("/target/pkg/app.js")));
        assert!(is_wasm_bindgen_output(Path::new(
            "/target/wasm32/app_bg.js"
        )));
        assert!(is_wasm_bindgen_output(Path::new(
            "/target/snippets/helper.js"
        )));
        assert!(is_wasm_bindgen_output(Path::new(
            "/target/wasm-bindgen/out.js"
        )));

        // Suspicious JS files
        assert!(!is_wasm_bindgen_output(Path::new("/target/debug/evil.js")));
        assert!(!is_wasm_bindgen_output(Path::new(
            "/target/release/backdoor.js"
        )));
    }

    #[test]
    fn test_target_js_check_passes_for_wasm_bindgen() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        let pkg_dir = target_dir.join("pkg");
        fs::create_dir_all(&pkg_dir).unwrap();

        // Create legitimate wasm-bindgen output
        let wasm_bindgen_js = pkg_dir.join("app.js");
        fs::write(&wasm_bindgen_js, "// wasm-bindgen generated").unwrap();

        let checker = WasmThreadingCompliance::new();
        let mut result = ComplianceResult::new();
        checker.check_target_js_files(temp_dir.path(), &mut result);

        // Should pass since it's legitimate wasm-bindgen output
        assert_eq!(result.checks.len(), 1);
        assert_eq!(result.checks[0].status, ComplianceStatus::Pass);
    }

    #[test]
    fn test_target_js_check_fails_for_suspicious_js() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        let debug_dir = target_dir.join("debug");
        fs::create_dir_all(&debug_dir).unwrap();

        // Create suspicious JS file (bypassing WASM-only compliance)
        let evil_js = debug_dir.join("build_script_output.js");
        fs::write(&evil_js, "// generated by build.rs - CI loophole!").unwrap();

        let checker = WasmThreadingCompliance::new();
        let mut result = ComplianceResult::new();
        checker.check_target_js_files(temp_dir.path(), &mut result);

        // Should fail since it's not legitimate wasm-bindgen output
        assert_eq!(result.checks.len(), 1);
        assert_eq!(result.checks[0].status, ComplianceStatus::Fail);
        assert!(result.checks[0]
            .details
            .as_ref()
            .unwrap()
            .contains("build.rs loophole"));
    }

    #[test]
    fn test_tarantula_report_empty_without_lcov() {
        let mut checker = WasmThreadingCompliance::new();
        let report = checker.tarantula_report();
        assert!(report.is_none());
    }
}
