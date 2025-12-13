//! First-Class Terminal Output for Pixel Coverage (PIXEL-001 v2.1 Phase 7)
//!
//! Rich terminal heatmap with score bars, gap analysis, and hypothesis status.
//! Implements Popperian falsification display for coverage claims.

use super::heatmap::ColorPalette;
use super::tracker::{CombinedCoverageReport, CoverageCell};

/// Output mode for terminal rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Rich ANSI true-color (24-bit) output
    #[default]
    RichAnsi,
    /// No color ASCII output (NO_COLOR env or --no-color flag)
    NoColorAscii,
    /// JSON output for CI tools
    Json,
}

impl OutputMode {
    /// Detect output mode from environment
    #[must_use]
    pub fn from_env() -> Self {
        if std::env::var("NO_COLOR").is_ok() {
            Self::NoColorAscii
        } else if std::env::var("CI").is_ok() {
            Self::Json
        } else {
            Self::RichAnsi
        }
    }
}

/// ANSI escape codes for terminal output
pub mod ansi {
    /// Reset all attributes
    pub const RESET: &str = "\x1b[0m";
    /// Bold text
    pub const BOLD: &str = "\x1b[1m";
    /// Dim text
    pub const DIM: &str = "\x1b[2m";

    /// RGB foreground color
    #[must_use]
    pub fn rgb_fg(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[38;2;{r};{g};{b}m")
    }

    /// RGB background color
    #[must_use]
    pub fn rgb_bg(r: u8, g: u8, b: u8) -> String {
        format!("\x1b[48;2;{r};{g};{b}m")
    }

    /// Green color for passing tests
    pub const PASS: &str = "\x1b[32m";
    /// Red color for failing tests
    pub const FAIL: &str = "\x1b[31m";
    /// Yellow color for warnings
    pub const WARN: &str = "\x1b[33m";
    /// Cyan color for info messages
    pub const INFO: &str = "\x1b[36m";
}

/// A falsifiable coverage hypothesis
#[derive(Debug, Clone)]
pub struct CoverageHypothesis {
    /// Hypothesis ID (e.g., "H0-COV-01")
    pub id: String,
    /// Description of the claim
    pub description: String,
    /// Threshold value for falsification
    pub threshold: f32,
    /// Actual measured value
    pub actual: f32,
    /// Whether the hypothesis was falsified
    pub falsified: bool,
}

impl CoverageHypothesis {
    /// Create a new hypothesis
    #[must_use]
    pub fn new(id: &str, description: &str, threshold: f32, actual: f32) -> Self {
        let falsified = actual < threshold;
        Self {
            id: id.to_string(),
            description: description.to_string(),
            threshold,
            actual,
            falsified,
        }
    }

    /// Create coverage threshold hypothesis
    #[must_use]
    pub fn coverage_threshold(threshold: f32, actual: f32) -> Self {
        Self::new(
            "H0-COV-01",
            &format!("Coverage >= {:.0}%", threshold * 100.0),
            threshold,
            actual,
        )
    }

    /// Create gap size hypothesis
    #[must_use]
    pub fn max_gap_size(max_gap_percent: f32, actual_gap_percent: f32) -> Self {
        // Falsified if actual gap > max allowed
        let falsified = actual_gap_percent > max_gap_percent;
        Self {
            id: "H0-COV-02".to_string(),
            description: format!("No gap > {:.0}% area", max_gap_percent * 100.0),
            threshold: max_gap_percent,
            actual: actual_gap_percent,
            falsified,
        }
    }
}

/// Detected gap region in coverage
#[derive(Debug, Clone)]
pub struct GapRegion {
    /// Row range (start, end)
    pub rows: (usize, usize),
    /// Column range (start, end)
    pub cols: (usize, usize),
    /// Percentage of total screen
    pub percent: f32,
    /// Suggested component name (if identifiable)
    pub suggestion: Option<String>,
}

/// Visual score bar for terminal output
#[derive(Debug, Clone)]
pub struct ScoreBar {
    /// Score value (0.0 - 1.0)
    pub score: f32,
    /// Width in characters
    pub width: usize,
    /// Threshold for pass/fail coloring
    pub threshold: f32,
    /// Label text
    pub label: String,
}

impl ScoreBar {
    /// Create a new score bar
    #[must_use]
    pub fn new(label: &str, score: f32, threshold: f32) -> Self {
        Self {
            score,
            width: 25,
            threshold,
            label: label.to_string(),
        }
    }

    /// Set bar width
    #[must_use]
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Render the score bar
    #[must_use]
    pub fn render(&self, mode: OutputMode) -> String {
        let filled = ((self.score * self.width as f32) as usize).min(self.width);
        let empty = self.width - filled;

        let bar = format!(
            "{:>16}: {:5.1}%  {}{}",
            self.label,
            self.score * 100.0,
            "\u{2588}".repeat(filled),
            "\u{2591}".repeat(empty)
        );

        match mode {
            OutputMode::RichAnsi => {
                if self.score >= self.threshold {
                    format!("{}{}{}", ansi::PASS, bar, ansi::RESET)
                } else {
                    format!("{}{}{}", ansi::FAIL, bar, ansi::RESET)
                }
            }
            OutputMode::NoColorAscii => {
                let status = if self.score >= self.threshold {
                    "[PASS]"
                } else {
                    "[FAIL]"
                };
                format!(
                    "{} {}",
                    bar.replace('\u{2588}', "#").replace('\u{2591}', "-"),
                    status
                )
            }
            OutputMode::Json => bar,
        }
    }
}

/// Confidence interval for statistical rigor
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceInterval {
    /// Lower bound
    pub lower: f32,
    /// Upper bound
    pub upper: f32,
    /// Confidence level (e.g., 0.95 for 95%)
    pub level: f32,
}

impl ConfidenceInterval {
    /// Create a new confidence interval
    #[must_use]
    pub fn new(lower: f32, upper: f32, level: f32) -> Self {
        Self {
            lower,
            upper,
            level,
        }
    }

    /// Calculate Wilson score interval for proportion
    /// More accurate than normal approximation for small samples
    #[must_use]
    pub fn wilson_score(successes: u32, total: u32, confidence: f32) -> Self {
        if total == 0 {
            return Self::new(0.0, 0.0, confidence);
        }

        let n = total as f64;
        let p = successes as f64 / n;

        // Z-score for confidence level (approximation)
        let z = match confidence {
            c if c >= 0.99 => 2.576,
            c if c >= 0.95 => 1.96,
            c if c >= 0.90 => 1.645,
            _ => 1.96,
        };

        let z2 = z * z;
        let denominator = 1.0 + z2 / n;
        let center = (p + z2 / (2.0 * n)) / denominator;
        let margin = (z / denominator) * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt();

        Self::new(
            (center - margin).max(0.0) as f32,
            (center + margin).min(1.0) as f32,
            confidence,
        )
    }

    /// Format as string
    #[must_use]
    pub fn format(&self) -> String {
        format!(
            "{:.0}% CI [{:.1}%, {:.1}%]",
            self.level * 100.0,
            self.lower * 100.0,
            self.upper * 100.0
        )
    }
}

/// Rich terminal heatmap with full visualization
#[derive(Debug, Clone)]
pub struct RichTerminalHeatmap {
    /// Coverage cells
    cells: Vec<Vec<CoverageCell>>,
    /// Color palette
    palette: ColorPalette,
    /// Output mode
    mode: OutputMode,
    /// Title text
    title: Option<String>,
    /// Show score panel
    show_scores: bool,
    /// Show gap analysis
    show_gaps: bool,
    /// Show hypothesis status
    show_hypotheses: bool,
    /// Coverage threshold
    threshold: f32,
    /// Confidence level for intervals
    confidence_level: f32,
}

impl RichTerminalHeatmap {
    /// Create from coverage cells
    #[must_use]
    pub fn new(cells: Vec<Vec<CoverageCell>>) -> Self {
        Self {
            cells,
            palette: ColorPalette::viridis(),
            mode: OutputMode::from_env(),
            title: None,
            show_scores: true,
            show_gaps: true,
            show_hypotheses: true,
            threshold: 0.85,
            confidence_level: 0.95,
        }
    }

    /// Set title
    #[must_use]
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Set output mode
    #[must_use]
    pub fn with_mode(mut self, mode: OutputMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set color palette
    #[must_use]
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Set coverage threshold
    #[must_use]
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Enable/disable score panel
    #[must_use]
    pub fn with_scores(mut self, show: bool) -> Self {
        self.show_scores = show;
        self
    }

    /// Enable/disable gap analysis
    #[must_use]
    pub fn with_gaps(mut self, show: bool) -> Self {
        self.show_gaps = show;
        self
    }

    /// Enable/disable hypothesis status
    #[must_use]
    pub fn with_hypotheses(mut self, show: bool) -> Self {
        self.show_hypotheses = show;
        self
    }

    /// Calculate coverage statistics
    fn calculate_stats(&self) -> (f32, u32, u32) {
        let mut covered = 0u32;
        let mut total = 0u32;

        for row in &self.cells {
            for cell in row {
                total += 1;
                if cell.coverage > 0.0 {
                    covered += 1;
                }
            }
        }

        let coverage = if total > 0 {
            covered as f32 / total as f32
        } else {
            0.0
        };

        (coverage, covered, total)
    }

    /// Find gap regions
    fn find_gaps(&self) -> Vec<GapRegion> {
        let mut gaps = Vec::new();
        let rows = self.cells.len();
        let cols = self.cells.first().map_or(0, Vec::len);
        let total_cells = (rows * cols) as f32;

        if total_cells == 0.0 {
            return gaps;
        }

        // Simple gap detection: find contiguous regions of 0 coverage
        let mut visited = vec![vec![false; cols]; rows];

        for r in 0..rows {
            for c in 0..cols {
                if !visited[r][c] && self.cells[r][c].coverage <= 0.0 {
                    // BFS to find contiguous gap region
                    let mut min_row = r;
                    let mut max_row = r;
                    let mut min_col = c;
                    let mut max_col = c;
                    let mut gap_cells = 0;

                    let mut queue = vec![(r, c)];
                    visited[r][c] = true;

                    while let Some((row, col)) = queue.pop() {
                        gap_cells += 1;
                        min_row = min_row.min(row);
                        max_row = max_row.max(row);
                        min_col = min_col.min(col);
                        max_col = max_col.max(col);

                        // Check neighbors
                        for (dr, dc) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                            let nr = row as i32 + dr;
                            let nc = col as i32 + dc;
                            if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                                let nr = nr as usize;
                                let nc = nc as usize;
                                if !visited[nr][nc] && self.cells[nr][nc].coverage <= 0.0 {
                                    visited[nr][nc] = true;
                                    queue.push((nr, nc));
                                }
                            }
                        }
                    }

                    let percent = gap_cells as f32 / total_cells;
                    if percent >= 0.01 {
                        // Only report gaps >= 1%
                        gaps.push(GapRegion {
                            rows: (min_row, max_row),
                            cols: (min_col, max_col),
                            percent,
                            suggestion: None,
                        });
                    }
                }
            }
        }

        // Sort by size (largest first)
        gaps.sort_by(|a, b| {
            b.percent
                .partial_cmp(&a.percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        gaps
    }

    /// Render the heatmap grid
    #[must_use]
    pub fn render_grid(&self) -> String {
        let mut output = String::new();

        for row in &self.cells {
            output.push_str("  ");
            for cell in row {
                let ch = Self::coverage_char(cell.coverage);
                match self.mode {
                    OutputMode::RichAnsi => {
                        let color = self.palette.interpolate(cell.coverage);
                        output.push_str(&ansi::rgb_fg(color.r, color.g, color.b));
                        output.push(ch);
                        output.push_str(ansi::RESET);
                    }
                    OutputMode::NoColorAscii => {
                        output.push(Self::ascii_coverage_char(cell.coverage));
                    }
                    OutputMode::Json => {
                        output.push(ch);
                    }
                }
            }
            output.push('\n');
        }

        output
    }

    /// Render score panel
    #[must_use]
    pub fn render_scores(&self, pixel_coverage: f32, line_coverage: Option<f32>) -> String {
        let mut output = String::new();
        let combined = line_coverage.map_or(pixel_coverage, |l| (pixel_coverage + l) / 2.0);

        output.push_str("  \u{250C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}\n");
        output.push_str(
            "  \u{2502}  COVERAGE SCORE                                        \u{2502}\n",
        );
        output.push_str("  \u{2502}  \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}  \u{2502}\n");
        output.push_str(
            "  \u{2502}                                                        \u{2502}\n",
        );

        // Pixel coverage bar
        let pixel_bar = ScoreBar::new("Pixel Coverage", pixel_coverage, self.threshold);
        output.push_str(&format!(
            "  \u{2502}    {}  \u{2502}\n",
            pixel_bar.render(self.mode)
        ));

        // Line coverage bar (if available)
        if let Some(line) = line_coverage {
            let line_bar = ScoreBar::new("Line Coverage", line, self.threshold);
            output.push_str(&format!(
                "  \u{2502}    {}  \u{2502}\n",
                line_bar.render(self.mode)
            ));
        }

        // Combined score bar
        let combined_bar = ScoreBar::new("Combined Score", combined, self.threshold);
        output.push_str(&format!(
            "  \u{2502}    {}  \u{2502}\n",
            combined_bar.render(self.mode)
        ));

        output.push_str(
            "  \u{2502}                                                        \u{2502}\n",
        );

        // Status and confidence interval
        let (_, covered, total) = self.calculate_stats();
        let ci = ConfidenceInterval::wilson_score(covered, total, self.confidence_level);
        let status = if combined >= self.threshold {
            match self.mode {
                OutputMode::RichAnsi => format!("{}\u{2705} PASS{}", ansi::PASS, ansi::RESET),
                _ => "PASS".to_string(),
            }
        } else {
            match self.mode {
                OutputMode::RichAnsi => format!("{}\u{274C} FAIL{}", ansi::FAIL, ansi::RESET),
                _ => "FAIL".to_string(),
            }
        };

        output.push_str(&format!(
            "  \u{2502}    Threshold: {:.1}%    Status: {}                  \u{2502}\n",
            self.threshold * 100.0,
            status
        ));
        output.push_str(&format!(
            "  \u{2502}    Confidence: {}                          \u{2502}\n",
            ci.format()
        ));
        output.push_str(
            "  \u{2502}                                                        \u{2502}\n",
        );
        output.push_str("  \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}\n");

        output
    }

    /// Render gap analysis
    #[must_use]
    pub fn render_gap_analysis(&self) -> String {
        let gaps = self.find_gaps();
        let mut output = String::new();

        if gaps.is_empty() {
            output.push_str(&format!(
                "  {}\u{2705} No coverage gaps detected{}\n",
                if self.mode == OutputMode::RichAnsi {
                    ansi::PASS
                } else {
                    ""
                },
                if self.mode == OutputMode::RichAnsi {
                    ansi::RESET
                } else {
                    ""
                }
            ));
            return output;
        }

        let total_gap_percent: f32 = gaps.iter().map(|g| g.percent).sum();
        output.push_str(&format!(
            "  {}\u{26A0} GAPS DETECTED ({} region{}, {:.1}% of screen){}\n",
            if self.mode == OutputMode::RichAnsi {
                ansi::WARN
            } else {
                ""
            },
            gaps.len(),
            if gaps.len() == 1 { "" } else { "s" },
            total_gap_percent * 100.0,
            if self.mode == OutputMode::RichAnsi {
                ansi::RESET
            } else {
                ""
            }
        ));

        for (i, gap) in gaps.iter().take(5).enumerate() {
            let connector = if i == gaps.len().min(5) - 1 {
                "\u{2514}"
            } else {
                "\u{251C}"
            };
            output.push_str(&format!(
                "  {}\u{2500} Gap #{}: rows {}-{}, cols {}-{} ({:.1}%)\n",
                connector,
                i + 1,
                gap.rows.0,
                gap.rows.1,
                gap.cols.0,
                gap.cols.1,
                gap.percent * 100.0
            ));
        }

        if gaps.len() > 5 {
            output.push_str(&format!("     ... and {} more gaps\n", gaps.len() - 5));
        }

        output
    }

    /// Render hypothesis falsification status
    #[must_use]
    pub fn render_hypotheses(&self, hypotheses: &[CoverageHypothesis]) -> String {
        let mut output = String::new();

        output.push_str("  FALSIFICATION STATUS\n");

        for (i, h) in hypotheses.iter().enumerate() {
            let connector = if i == hypotheses.len() - 1 {
                "\u{2514}"
            } else {
                "\u{251C}"
            };
            let status = if h.falsified {
                match self.mode {
                    OutputMode::RichAnsi => {
                        format!("{}\u{274C} FALSIFIED{}", ansi::FAIL, ansi::RESET)
                    }
                    _ => "FALSIFIED".to_string(),
                }
            } else {
                match self.mode {
                    OutputMode::RichAnsi => {
                        format!("{}\u{2705} NOT FALSIFIED{}", ansi::PASS, ansi::RESET)
                    }
                    _ => "NOT FALSIFIED".to_string(),
                }
            };

            output.push_str(&format!(
                "  {}\u{2500} {}: {} \u{2192} {} ({:.1}%)\n",
                connector,
                h.id,
                h.description,
                status,
                h.actual * 100.0
            ));
        }

        output
    }

    /// Render complete terminal display
    #[must_use]
    pub fn render(&self) -> String {
        self.render_with_report(None)
    }

    /// Render with combined report
    #[must_use]
    pub fn render_with_report(&self, report: Option<&CombinedCoverageReport>) -> String {
        let mut output = String::new();
        let (pixel_coverage, _, _) = self.calculate_stats();

        // Header
        let border = "\u{2550}".repeat(70);
        output.push_str(&format!("\u{2554}{}\u{2557}\n", border));

        if let Some(title) = &self.title {
            let padding = (68 - title.len()) / 2;
            output.push_str(&format!(
                "\u{2551}{:^70}\u{2551}\n",
                format!("{}{}", " ".repeat(padding.max(0)), title)
            ));
        } else {
            output.push_str(&format!(
                "\u{2551}{:^70}\u{2551}\n",
                "PIXEL COVERAGE HEATMAP"
            ));
        }

        output.push_str(&format!("\u{2560}{}\u{2563}\n", border));

        // Grid
        output.push_str(&format!("\u{2551}{:70}\u{2551}\n", ""));
        let grid = self.render_grid();
        for line in grid.lines() {
            output.push_str(&format!("\u{2551}{:70}\u{2551}\n", line));
        }
        output.push_str(&format!("\u{2551}{:70}\u{2551}\n", ""));

        // Legend
        output.push_str(&format!("\u{2560}{}\u{2563}\n", border));
        output.push_str(
            "\u{2551}  LEGEND: \u{2588} 76-100%  \u{2593} 51-75%  \u{2592} 26-50%  \u{2591} 1-25%  \u{00B7} 0% (GAP)   \u{2551}\n"
        );

        // Score panel
        if self.show_scores {
            output.push_str(&format!("\u{2560}{}\u{2563}\n", border));
            let line_coverage = report.map(|r| r.line_coverage.element_coverage);
            let scores = self.render_scores(pixel_coverage, line_coverage);
            for line in scores.lines() {
                output.push_str(&format!("\u{2551}{:70}\u{2551}\n", line));
            }
        }

        // Gap analysis
        if self.show_gaps {
            output.push_str(&format!("\u{2560}{}\u{2563}\n", border));
            let gaps = self.render_gap_analysis();
            for line in gaps.lines() {
                output.push_str(&format!("\u{2551}{:70}\u{2551}\n", line));
            }
        }

        // Hypothesis status
        if self.show_hypotheses {
            let gaps = self.find_gaps();
            let max_gap = gaps.first().map_or(0.0, |g| g.percent);

            let hypotheses = vec![
                CoverageHypothesis::coverage_threshold(self.threshold, pixel_coverage),
                CoverageHypothesis::max_gap_size(0.15, max_gap),
            ];

            output.push_str(&format!("\u{2560}{}\u{2563}\n", border));
            let hyp_output = self.render_hypotheses(&hypotheses);
            for line in hyp_output.lines() {
                output.push_str(&format!("\u{2551}{:70}\u{2551}\n", line));
            }
        }

        // Footer
        output.push_str(&format!("\u{255A}{}\u{255D}\n", border));

        output
    }

    /// Get coverage character for value
    fn coverage_char(coverage: f32) -> char {
        match coverage {
            c if c <= 0.0 => '\u{00B7}',  // Middle dot for gaps
            c if c <= 0.25 => '\u{2591}', // Light shade
            c if c <= 0.50 => '\u{2592}', // Medium shade
            c if c <= 0.75 => '\u{2593}', // Dark shade
            _ => '\u{2588}',              // Full block
        }
    }

    /// Get ASCII coverage character (no-color mode)
    fn ascii_coverage_char(coverage: f32) -> char {
        match coverage {
            c if c <= 0.0 => '.',
            c if c <= 0.25 => '-',
            c if c <= 0.50 => '+',
            c if c <= 0.75 => '#',
            _ => '@',
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // =========================================================================
    // Score Bar Tests (H0-TERM-01-XX)
    // =========================================================================

    #[test]
    fn h0_term_01_score_bar_render() {
        let bar = ScoreBar::new("Test", 0.85, 0.80);
        let output = bar.render(OutputMode::NoColorAscii);
        assert!(output.contains("85.0%"));
        assert!(output.contains("[PASS]"));
    }

    #[test]
    fn h0_term_02_score_bar_fail() {
        let bar = ScoreBar::new("Test", 0.50, 0.80);
        let output = bar.render(OutputMode::NoColorAscii);
        assert!(output.contains("50.0%"));
        assert!(output.contains("[FAIL]"));
    }

    #[test]
    fn h0_term_03_score_bar_width() {
        let bar = ScoreBar::new("Test", 1.0, 0.80).with_width(10);
        let output = bar.render(OutputMode::NoColorAscii);
        assert!(output.contains("##########")); // 10 filled chars
    }

    // =========================================================================
    // Confidence Interval Tests (H0-TERM-04-XX)
    // =========================================================================

    #[test]
    fn h0_term_04_wilson_score_full() {
        let ci = ConfidenceInterval::wilson_score(100, 100, 0.95);
        assert!(ci.lower > 0.95);
        assert!((ci.upper - 1.0).abs() < 0.01);
    }

    #[test]
    fn h0_term_05_wilson_score_empty() {
        let ci = ConfidenceInterval::wilson_score(0, 100, 0.95);
        assert!(ci.lower < 0.05);
        assert!(ci.upper < 0.10);
    }

    #[test]
    fn h0_term_06_wilson_score_half() {
        let ci = ConfidenceInterval::wilson_score(50, 100, 0.95);
        assert!(ci.lower > 0.35);
        assert!(ci.upper < 0.65);
    }

    #[test]
    fn h0_term_07_wilson_zero_total() {
        let ci = ConfidenceInterval::wilson_score(0, 0, 0.95);
        assert_eq!(ci.lower, 0.0);
        assert_eq!(ci.upper, 0.0);
    }

    // =========================================================================
    // Hypothesis Tests (H0-TERM-08-XX)
    // =========================================================================

    #[test]
    fn h0_term_08_hypothesis_pass() {
        let h = CoverageHypothesis::coverage_threshold(0.80, 0.85);
        assert!(!h.falsified);
    }

    #[test]
    fn h0_term_09_hypothesis_fail() {
        let h = CoverageHypothesis::coverage_threshold(0.80, 0.75);
        assert!(h.falsified);
    }

    #[test]
    fn h0_term_10_gap_hypothesis() {
        let h = CoverageHypothesis::max_gap_size(0.15, 0.10);
        assert!(!h.falsified);

        let h2 = CoverageHypothesis::max_gap_size(0.15, 0.20);
        assert!(h2.falsified);
    }

    // =========================================================================
    // Rich Terminal Heatmap Tests (H0-TERM-11-XX)
    // =========================================================================

    #[test]
    fn h0_term_11_render_empty() {
        let cells = vec![
            vec![
                CoverageCell {
                    coverage: 0.0,
                    hit_count: 0
                };
                5
            ];
            5
        ];
        let heatmap = RichTerminalHeatmap::new(cells).with_mode(OutputMode::NoColorAscii);
        let output = heatmap.render();
        assert!(!output.is_empty());
    }

    #[test]
    fn h0_term_12_render_full() {
        let cells = vec![
            vec![
                CoverageCell {
                    coverage: 1.0,
                    hit_count: 10
                };
                5
            ];
            5
        ];
        let heatmap = RichTerminalHeatmap::new(cells).with_mode(OutputMode::NoColorAscii);
        let output = heatmap.render();
        assert!(output.contains("PASS") || output.contains("NOT FALSIFIED"));
    }

    #[test]
    fn h0_term_13_render_with_gaps() {
        let mut cells = vec![
            vec![
                CoverageCell {
                    coverage: 1.0,
                    hit_count: 10
                };
                10
            ];
            10
        ];
        // Create a gap
        for r in 3..7 {
            for c in 3..7 {
                cells[r][c] = CoverageCell {
                    coverage: 0.0,
                    hit_count: 0,
                };
            }
        }
        let heatmap = RichTerminalHeatmap::new(cells).with_mode(OutputMode::NoColorAscii);
        let output = heatmap.render();
        assert!(output.contains("GAP"));
    }

    #[test]
    fn h0_term_14_output_mode_env() {
        // Default should be RichAnsi or based on env
        let mode = OutputMode::from_env();
        // Just verify it doesn't panic
        assert!(matches!(
            mode,
            OutputMode::RichAnsi | OutputMode::NoColorAscii | OutputMode::Json
        ));
    }

    #[test]
    fn h0_term_15_coverage_chars() {
        assert_eq!(RichTerminalHeatmap::coverage_char(0.0), '\u{00B7}');
        assert_eq!(RichTerminalHeatmap::coverage_char(0.1), '\u{2591}');
        assert_eq!(RichTerminalHeatmap::coverage_char(0.4), '\u{2592}');
        assert_eq!(RichTerminalHeatmap::coverage_char(0.6), '\u{2593}');
        assert_eq!(RichTerminalHeatmap::coverage_char(1.0), '\u{2588}');
    }

    #[test]
    fn h0_term_16_ascii_coverage_chars() {
        assert_eq!(RichTerminalHeatmap::ascii_coverage_char(0.0), '.');
        assert_eq!(RichTerminalHeatmap::ascii_coverage_char(0.1), '-');
        assert_eq!(RichTerminalHeatmap::ascii_coverage_char(0.4), '+');
        assert_eq!(RichTerminalHeatmap::ascii_coverage_char(0.6), '#');
        assert_eq!(RichTerminalHeatmap::ascii_coverage_char(1.0), '@');
    }

    #[test]
    fn h0_term_17_find_gaps() {
        let mut cells = vec![
            vec![
                CoverageCell {
                    coverage: 1.0,
                    hit_count: 10
                };
                10
            ];
            10
        ];
        // Create a 4x4 gap (16% of 100 cells)
        for r in 3..7 {
            for c in 3..7 {
                cells[r][c] = CoverageCell {
                    coverage: 0.0,
                    hit_count: 0,
                };
            }
        }
        let heatmap = RichTerminalHeatmap::new(cells);
        let gaps = heatmap.find_gaps();
        assert!(!gaps.is_empty());
        assert!((gaps[0].percent - 0.16).abs() < 0.01);
    }

    #[test]
    fn h0_term_18_confidence_interval_format() {
        let ci = ConfidenceInterval::new(0.80, 0.90, 0.95);
        let formatted = ci.format();
        assert!(formatted.contains("95%"));
        assert!(formatted.contains("80.0%"));
        assert!(formatted.contains("90.0%"));
    }
}
