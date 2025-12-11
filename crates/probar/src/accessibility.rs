//! Accessibility validation for games.
//!
//! Per spec Section 6.3: "Automated a11y testing for games."
//!
//! This module provides tools for validating accessibility compliance:
//! - Color contrast analysis (WCAG 2.1 AA)
//! - Focus indicator detection
//! - Reduced motion preference handling
//! - Screen reader compatibility

use crate::result::{ProbarError, ProbarResult};

/// Minimum contrast ratio for normal text (WCAG 2.1 AA)
pub const MIN_CONTRAST_NORMAL: f32 = 4.5;

/// Minimum contrast ratio for large text (WCAG 2.1 AA)
pub const MIN_CONTRAST_LARGE: f32 = 3.0;

/// Minimum contrast ratio for UI components (WCAG 2.1 AA)
pub const MIN_CONTRAST_UI: f32 = 3.0;

/// Color represented as RGB values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl Color {
    /// Create a new color
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create from hex value (e.g., 0xFF5500)
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Get the relative luminance (per WCAG 2.1)
    #[must_use]
    pub fn relative_luminance(&self) -> f32 {
        // Convert to sRGB
        let r = srgb_to_linear(f32::from(self.r) / 255.0);
        let g = srgb_to_linear(f32::from(self.g) / 255.0);
        let b = srgb_to_linear(f32::from(self.b) / 255.0);

        // Calculate luminance
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Calculate contrast ratio with another color
    #[must_use]
    pub fn contrast_ratio(&self, other: &Self) -> f32 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();

        let lighter = l1.max(l2);
        let darker = l1.min(l2);

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check if contrast meets WCAG AA for normal text
    #[must_use]
    pub fn meets_wcag_aa_normal(&self, other: &Self) -> bool {
        self.contrast_ratio(other) >= MIN_CONTRAST_NORMAL
    }

    /// Check if contrast meets WCAG AA for large text
    #[must_use]
    pub fn meets_wcag_aa_large(&self, other: &Self) -> bool {
        self.contrast_ratio(other) >= MIN_CONTRAST_LARGE
    }

    /// Check if contrast meets WCAG AA for UI components
    #[must_use]
    pub fn meets_wcag_aa_ui(&self, other: &Self) -> bool {
        self.contrast_ratio(other) >= MIN_CONTRAST_UI
    }
}

/// Convert sRGB to linear RGB (per WCAG 2.1)
fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Results of a contrast analysis
#[derive(Debug, Clone)]
pub struct ContrastAnalysis {
    /// Minimum contrast ratio found
    pub min_ratio: f32,
    /// Maximum contrast ratio found
    pub max_ratio: f32,
    /// Average contrast ratio
    pub avg_ratio: f32,
    /// Number of color pairs analyzed
    pub pairs_analyzed: usize,
    /// Color pairs that fail WCAG AA
    pub failing_pairs: Vec<ContrastPair>,
    /// Whether the analysis passes WCAG AA
    pub passes_wcag_aa: bool,
}

impl ContrastAnalysis {
    /// Create an empty analysis
    #[must_use]
    pub fn empty() -> Self {
        Self {
            min_ratio: f32::MAX,
            max_ratio: 0.0,
            avg_ratio: 0.0,
            pairs_analyzed: 0,
            failing_pairs: Vec::new(),
            passes_wcag_aa: true,
        }
    }

    /// Add a color pair to the analysis
    pub fn add_pair(&mut self, foreground: Color, background: Color, context: impl Into<String>) {
        let ratio = foreground.contrast_ratio(&background);
        self.pairs_analyzed += 1;

        self.min_ratio = self.min_ratio.min(ratio);
        self.max_ratio = self.max_ratio.max(ratio);

        // Rolling average
        self.avg_ratio = self.avg_ratio + (ratio - self.avg_ratio) / (self.pairs_analyzed as f32);

        // Check WCAG AA
        if ratio < MIN_CONTRAST_NORMAL {
            self.passes_wcag_aa = false;
            self.failing_pairs.push(ContrastPair {
                foreground,
                background,
                ratio,
                context: context.into(),
            });
        }
    }
}

/// A pair of colors with their contrast ratio
#[derive(Debug, Clone)]
pub struct ContrastPair {
    /// Foreground color
    pub foreground: Color,
    /// Background color
    pub background: Color,
    /// Contrast ratio between them
    pub ratio: f32,
    /// Context where this pair was found
    pub context: String,
}

/// Configuration for accessibility validation
#[derive(Debug, Clone)]
pub struct AccessibilityConfig {
    /// Check color contrast
    pub check_contrast: bool,
    /// Check focus indicators
    pub check_focus: bool,
    /// Check reduced motion
    pub check_reduced_motion: bool,
    /// Check keyboard navigation
    pub check_keyboard: bool,
    /// Minimum contrast ratio for text
    pub min_contrast_text: f32,
    /// Minimum contrast ratio for UI
    pub min_contrast_ui: f32,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            check_contrast: true,
            check_focus: true,
            check_reduced_motion: true,
            check_keyboard: true,
            min_contrast_text: MIN_CONTRAST_NORMAL,
            min_contrast_ui: MIN_CONTRAST_UI,
        }
    }
}

/// Result of an accessibility audit
#[derive(Debug, Clone)]
pub struct AccessibilityAudit {
    /// Contrast analysis results
    pub contrast: ContrastAnalysis,
    /// Whether focus indicators are present
    pub has_focus_indicators: bool,
    /// Whether reduced motion is respected
    pub respects_reduced_motion: bool,
    /// Keyboard navigation issues
    pub keyboard_issues: Vec<KeyboardIssue>,
    /// Overall accessibility score (0-100)
    pub score: u8,
    /// Issues found
    pub issues: Vec<AccessibilityIssue>,
}

impl AccessibilityAudit {
    /// Create a new empty audit
    #[must_use]
    pub fn new() -> Self {
        Self {
            contrast: ContrastAnalysis::empty(),
            has_focus_indicators: true,
            respects_reduced_motion: true,
            keyboard_issues: Vec::new(),
            score: 100,
            issues: Vec::new(),
        }
    }

    /// Check if the audit passes
    #[must_use]
    pub fn passes(&self) -> bool {
        self.issues.is_empty() && self.score >= 80
    }

    /// Add an issue
    pub fn add_issue(&mut self, issue: AccessibilityIssue) {
        // Deduct points based on severity
        let deduction = match issue.severity {
            Severity::Critical => 30,
            Severity::Major => 20,
            Severity::Minor => 10,
            Severity::Info => 0,
        };
        self.score = self.score.saturating_sub(deduction);
        self.issues.push(issue);
    }
}

impl Default for AccessibilityAudit {
    fn default() -> Self {
        Self::new()
    }
}

/// An accessibility issue found during audit
#[derive(Debug, Clone)]
pub struct AccessibilityIssue {
    /// WCAG criterion code (e.g., "1.4.3")
    pub wcag_code: String,
    /// Issue description
    pub description: String,
    /// Severity level
    pub severity: Severity,
    /// Element or context where issue was found
    pub context: Option<String>,
    /// Suggested fix
    pub fix_suggestion: Option<String>,
}

impl AccessibilityIssue {
    /// Create a new accessibility issue
    #[must_use]
    pub fn new(
        wcag_code: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            wcag_code: wcag_code.into(),
            description: description.into(),
            severity,
            context: None,
            fix_suggestion: None,
        }
    }

    /// Add context
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add fix suggestion
    #[must_use]
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix_suggestion = Some(fix.into());
        self
    }
}

/// Severity level of an accessibility issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Critical - must be fixed
    Critical,
    /// Major - should be fixed
    Major,
    /// Minor - nice to fix
    Minor,
    /// Informational
    Info,
}

/// A keyboard navigation issue
#[derive(Debug, Clone)]
pub struct KeyboardIssue {
    /// Description of the issue
    pub description: String,
    /// Element that has the issue
    pub element: Option<String>,
    /// WCAG criterion
    pub wcag: String,
}

/// Focus indicator configuration
#[derive(Debug, Clone)]
pub struct FocusConfig {
    /// Minimum focus outline width (pixels)
    pub min_outline_width: f32,
    /// Minimum contrast for focus indicator
    pub min_contrast: f32,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            min_outline_width: 2.0,
            min_contrast: 3.0,
        }
    }
}

/// Accessibility validator for game testing
///
/// Per spec Section 6.3: Validates accessibility compliance
#[derive(Debug, Clone, Default)]
pub struct AccessibilityValidator {
    config: AccessibilityConfig,
}

impl AccessibilityValidator {
    /// Create a new validator with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: AccessibilityConfig::default(),
        }
    }

    /// Create a validator with custom config
    #[must_use]
    pub const fn with_config(config: AccessibilityConfig) -> Self {
        Self { config }
    }

    /// Analyze color contrast
    ///
    /// Per spec: `page.analyze_contrast().await?`
    #[must_use]
    pub fn analyze_contrast(&self, colors: &[(Color, Color, &str)]) -> ContrastAnalysis {
        let mut analysis = ContrastAnalysis::empty();

        for (fg, bg, context) in colors {
            analysis.add_pair(*fg, *bg, *context);
        }

        analysis
    }

    /// Check if reduced motion is respected
    ///
    /// Per spec: "Check motion preferences"
    #[must_use]
    pub fn check_reduced_motion(&self, animations_disabled_when_preferred: bool) -> bool {
        animations_disabled_when_preferred
    }

    /// Validate focus indicators
    ///
    /// # Errors
    ///
    /// Returns error if focus indicator is missing
    pub fn validate_focus(&self, has_focus_visible: bool) -> ProbarResult<()> {
        if has_focus_visible {
            Ok(())
        } else {
            Err(ProbarError::AssertionError {
                message: "Focus indicator missing".to_string(),
            })
        }
    }

    /// Run a full accessibility audit
    #[must_use]
    pub fn audit(
        &self,
        colors: &[(Color, Color, &str)],
        has_focus_indicators: bool,
        respects_reduced_motion: bool,
    ) -> AccessibilityAudit {
        let mut audit = AccessibilityAudit::new();

        // Contrast analysis
        if self.config.check_contrast {
            audit.contrast = self.analyze_contrast(colors);
            if !audit.contrast.passes_wcag_aa {
                audit.add_issue(
                    AccessibilityIssue::new(
                        "1.4.3",
                        "Color contrast is insufficient for WCAG AA",
                        Severity::Major,
                    )
                    .with_fix("Increase contrast ratio to at least 4.5:1 for normal text"),
                );
            }
        }

        // Focus indicators
        if self.config.check_focus && !has_focus_indicators {
            audit.has_focus_indicators = false;
            audit.add_issue(
                AccessibilityIssue::new(
                    "2.4.7",
                    "Focus indicators are not visible",
                    Severity::Critical,
                )
                .with_fix("Add visible focus styles using :focus-visible"),
            );
        }

        // Reduced motion
        if self.config.check_reduced_motion && !respects_reduced_motion {
            audit.respects_reduced_motion = false;
            audit.add_issue(
                AccessibilityIssue::new(
                    "2.3.3",
                    "Animations do not respect prefers-reduced-motion",
                    Severity::Major,
                )
                .with_fix("Check prefers-reduced-motion media query and disable animations"),
            );
        }

        audit
    }
}

/// Flash detection for photosensitivity protection
///
/// Per spec Section 9.3: Protect against seizure-inducing content
#[derive(Debug, Clone)]
pub struct FlashDetector {
    /// Maximum allowed flash rate (Hz)
    pub max_flash_rate: f32,
    /// Maximum red flash intensity
    pub max_red_intensity: f32,
    /// Maximum flash area (percentage of screen)
    pub max_flash_area: f32,
}

impl Default for FlashDetector {
    fn default() -> Self {
        Self {
            max_flash_rate: 3.0, // WCAG 2.3.1: < 3 flashes per second
            max_red_intensity: 0.8,
            max_flash_area: 0.25, // 25% of screen max
        }
    }
}

/// Result of flash detection
#[derive(Debug, Clone)]
pub struct FlashResult {
    /// Detected flash rate (Hz)
    pub flash_rate: f32,
    /// Whether red flash threshold was exceeded
    pub red_flash_exceeded: bool,
    /// Flash area percentage
    pub flash_area: f32,
    /// Whether the content is safe
    pub is_safe: bool,
    /// Warning message if applicable
    pub warning: Option<String>,
}

impl FlashDetector {
    /// Create a new flash detector with default settings
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze frame transition for flashing
    #[must_use]
    pub fn analyze(
        &self,
        luminance_change: f32,
        red_intensity: f32,
        flash_area: f32,
        time_delta_secs: f32,
    ) -> FlashResult {
        // Calculate effective flash rate
        let flash_rate = if luminance_change > 0.1 && time_delta_secs > 0.0 {
            1.0 / time_delta_secs
        } else {
            0.0
        };

        let is_safe = flash_rate <= self.max_flash_rate
            && red_intensity <= self.max_red_intensity
            && flash_area <= self.max_flash_area;

        let warning = if is_safe {
            None
        } else if flash_rate > self.max_flash_rate {
            Some("Flash rate exceeds safe threshold".to_string())
        } else if red_intensity > self.max_red_intensity {
            Some("Red flash intensity exceeds safe threshold".to_string())
        } else {
            Some("Flash area exceeds safe threshold".to_string())
        };

        FlashResult {
            flash_rate,
            red_flash_exceeded: red_intensity > self.max_red_intensity,
            flash_area,
            is_safe,
            warning,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ========================================================================
    // EXTREME TDD: Tests for Accessibility validation per Section 6.3
    // ========================================================================

    mod color_tests {
        use super::*;

        #[test]
        fn test_color_from_hex() {
            let color = Color::from_hex(0x00FF_5500);
            assert_eq!(color.r, 255);
            assert_eq!(color.g, 0x55);
            assert_eq!(color.b, 0);
        }

        #[test]
        fn test_relative_luminance_black() {
            let black = Color::new(0, 0, 0);
            assert!(black.relative_luminance() < 0.01);
        }

        #[test]
        fn test_relative_luminance_white() {
            let white = Color::new(255, 255, 255);
            assert!(white.relative_luminance() > 0.99);
        }

        #[test]
        fn test_contrast_ratio_black_white() {
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);
            let ratio = black.contrast_ratio(&white);
            // Should be exactly 21:1
            assert!((ratio - 21.0).abs() < 0.1);
        }

        #[test]
        fn test_contrast_ratio_same_color() {
            let red = Color::new(255, 0, 0);
            let ratio = red.contrast_ratio(&red);
            // Same color = 1:1 ratio
            assert!((ratio - 1.0).abs() < 0.01);
        }

        #[test]
        fn test_wcag_aa_black_white() {
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);
            assert!(black.meets_wcag_aa_normal(&white));
            assert!(black.meets_wcag_aa_large(&white));
            assert!(black.meets_wcag_aa_ui(&white));
        }

        #[test]
        fn test_wcag_aa_low_contrast() {
            let light_gray = Color::new(200, 200, 200);
            let white = Color::new(255, 255, 255);
            assert!(!light_gray.meets_wcag_aa_normal(&white));
        }
    }

    mod contrast_analysis_tests {
        use super::*;

        #[test]
        fn test_empty_analysis() {
            let analysis = ContrastAnalysis::empty();
            assert_eq!(analysis.pairs_analyzed, 0);
            assert!(analysis.passes_wcag_aa);
        }

        #[test]
        fn test_add_passing_pair() {
            let mut analysis = ContrastAnalysis::empty();
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);
            analysis.add_pair(black, white, "text");
            assert_eq!(analysis.pairs_analyzed, 1);
            assert!(analysis.passes_wcag_aa);
            assert!(analysis.failing_pairs.is_empty());
        }

        #[test]
        fn test_add_failing_pair() {
            let mut analysis = ContrastAnalysis::empty();
            let gray = Color::new(150, 150, 150);
            let white = Color::new(255, 255, 255);
            analysis.add_pair(gray, white, "button");
            assert!(!analysis.passes_wcag_aa);
            assert_eq!(analysis.failing_pairs.len(), 1);
        }

        #[test]
        fn test_min_max_ratio() {
            let mut analysis = ContrastAnalysis::empty();
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);
            let gray = Color::new(128, 128, 128);

            analysis.add_pair(black, white, "high contrast");
            analysis.add_pair(gray, white, "lower contrast");

            assert!(analysis.max_ratio > analysis.min_ratio);
        }
    }

    mod accessibility_issue_tests {
        use super::*;

        #[test]
        fn test_issue_creation() {
            let issue = AccessibilityIssue::new("1.4.3", "Low contrast", Severity::Major);
            assert_eq!(issue.wcag_code, "1.4.3");
            assert!(matches!(issue.severity, Severity::Major));
        }

        #[test]
        fn test_issue_with_context() {
            let issue = AccessibilityIssue::new("2.4.7", "No focus", Severity::Critical)
                .with_context("Submit button");
            assert_eq!(issue.context, Some("Submit button".to_string()));
        }

        #[test]
        fn test_issue_with_fix() {
            let issue = AccessibilityIssue::new("2.3.3", "Animations", Severity::Minor)
                .with_fix("Add reduced motion check");
            assert!(issue.fix_suggestion.is_some());
        }
    }

    mod audit_tests {
        use super::*;

        #[test]
        fn test_new_audit_passes() {
            let audit = AccessibilityAudit::new();
            assert!(audit.passes());
            assert_eq!(audit.score, 100);
        }

        #[test]
        fn test_audit_with_critical_issue() {
            let mut audit = AccessibilityAudit::new();
            audit.add_issue(AccessibilityIssue::new(
                "2.4.7",
                "No focus indicators",
                Severity::Critical,
            ));
            assert_eq!(audit.score, 70); // 100 - 30
            assert!(!audit.passes());
        }

        #[test]
        fn test_audit_with_multiple_issues() {
            let mut audit = AccessibilityAudit::new();
            audit.add_issue(AccessibilityIssue::new(
                "1.4.3",
                "Low contrast",
                Severity::Major,
            ));
            audit.add_issue(AccessibilityIssue::new(
                "2.3.3",
                "No motion",
                Severity::Minor,
            ));
            assert_eq!(audit.score, 70); // 100 - 20 - 10
        }
    }

    mod validator_tests {
        use super::*;

        #[test]
        fn test_validator_new() {
            let validator = AccessibilityValidator::new();
            assert!(validator.config.check_contrast);
            assert!(validator.config.check_focus);
        }

        #[test]
        fn test_analyze_contrast() {
            let validator = AccessibilityValidator::new();
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);

            let analysis = validator.analyze_contrast(&[(black, white, "text")]);
            assert!(analysis.passes_wcag_aa);
        }

        #[test]
        fn test_validate_focus_pass() {
            let validator = AccessibilityValidator::new();
            assert!(validator.validate_focus(true).is_ok());
        }

        #[test]
        fn test_validate_focus_fail() {
            let validator = AccessibilityValidator::new();
            assert!(validator.validate_focus(false).is_err());
        }

        #[test]
        fn test_check_reduced_motion() {
            let validator = AccessibilityValidator::new();
            assert!(validator.check_reduced_motion(true));
            assert!(!validator.check_reduced_motion(false));
        }

        #[test]
        fn test_full_audit_pass() {
            let validator = AccessibilityValidator::new();
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);

            let audit = validator.audit(
                &[(black, white, "text")],
                true, // has focus
                true, // respects reduced motion
            );

            assert!(audit.passes());
            assert_eq!(audit.score, 100);
        }

        #[test]
        fn test_full_audit_fail_contrast() {
            let validator = AccessibilityValidator::new();
            let gray = Color::new(180, 180, 180);
            let white = Color::new(255, 255, 255);

            let audit = validator.audit(&[(gray, white, "text")], true, true);

            assert!(!audit.passes());
            assert!(audit.issues.iter().any(|i| i.wcag_code == "1.4.3"));
        }

        #[test]
        fn test_full_audit_fail_focus() {
            let validator = AccessibilityValidator::new();
            let black = Color::new(0, 0, 0);
            let white = Color::new(255, 255, 255);

            let audit = validator.audit(
                &[(black, white, "text")],
                false, // no focus
                true,
            );

            assert!(!audit.passes());
            assert!(audit.issues.iter().any(|i| i.wcag_code == "2.4.7"));
        }
    }

    mod flash_detector_tests {
        use super::*;

        #[test]
        fn test_flash_detector_default() {
            let detector = FlashDetector::default();
            assert!((detector.max_flash_rate - 3.0).abs() < 0.01);
        }

        #[test]
        fn test_analyze_safe_flash() {
            let detector = FlashDetector::new();
            let result = detector.analyze(0.05, 0.2, 0.1, 0.5);
            assert!(result.is_safe);
            assert!(result.warning.is_none());
        }

        #[test]
        fn test_analyze_high_flash_rate() {
            let detector = FlashDetector::new();
            // 10 flashes per second (1/0.1)
            let result = detector.analyze(0.5, 0.2, 0.1, 0.1);
            assert!(!result.is_safe);
            assert!(result.warning.is_some());
        }

        #[test]
        fn test_analyze_high_red_intensity() {
            let detector = FlashDetector::new();
            let result = detector.analyze(0.1, 0.95, 0.1, 1.0);
            assert!(!result.is_safe);
            assert!(result.red_flash_exceeded);
        }

        #[test]
        fn test_analyze_large_flash_area() {
            let detector = FlashDetector::new();
            let result = detector.analyze(0.1, 0.2, 0.5, 1.0);
            assert!(!result.is_safe);
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_accessibility_config_default() {
            let config = AccessibilityConfig::default();
            assert!(config.check_contrast);
            assert!(config.check_focus);
            assert!(config.check_reduced_motion);
            assert!(config.check_keyboard);
        }

        #[test]
        fn test_focus_config_default() {
            let config = FocusConfig::default();
            assert!((config.min_outline_width - 2.0).abs() < 0.01);
            assert!((config.min_contrast - 3.0).abs() < 0.01);
        }
    }
}
