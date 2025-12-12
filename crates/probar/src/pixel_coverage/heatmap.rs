//! Heatmap Rendering for Pixel Coverage
//!
//! Renders coverage data as visual heatmaps for terminal and web output.

use super::tracker::{CoverageCell, PixelCoverageTracker};
use serde::{Deserialize, Serialize};

/// RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl Rgb {
    /// Create a new RGB color
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create color from hex value
    #[must_use]
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

/// Color palette for heatmap rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    /// Color for 0% coverage
    pub zero: Rgb,
    /// Color for 25% coverage
    pub low: Rgb,
    /// Color for 50% coverage
    pub medium: Rgb,
    /// Color for 75% coverage
    pub high: Rgb,
    /// Color for 100% coverage
    pub full: Rgb,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::viridis()
    }
}

impl ColorPalette {
    /// Viridis color palette (colorblind-friendly)
    #[must_use]
    pub fn viridis() -> Self {
        Self {
            zero: Rgb::from_hex(0x440154),   // Dark purple
            low: Rgb::from_hex(0x3B528B),    // Blue
            medium: Rgb::from_hex(0x21918C), // Teal
            high: Rgb::from_hex(0x5DC863),   // Green
            full: Rgb::from_hex(0xFDE725),   // Yellow
        }
    }

    /// Red-Yellow-Green palette (traffic light)
    #[must_use]
    pub fn traffic_light() -> Self {
        Self {
            zero: Rgb::from_hex(0xFF0000),   // Red
            low: Rgb::from_hex(0xFF6600),    // Orange
            medium: Rgb::from_hex(0xFFFF00), // Yellow
            high: Rgb::from_hex(0x99FF00),   // Yellow-green
            full: Rgb::from_hex(0x00FF00),   // Green
        }
    }

    /// Get color for coverage value
    #[must_use]
    pub fn color_for_coverage(&self, coverage: f32) -> Rgb {
        match coverage {
            c if c <= 0.0 => self.zero,
            c if c <= 0.25 => self.low,
            c if c <= 0.50 => self.medium,
            c if c <= 0.75 => self.high,
            _ => self.full,
        }
    }
}

/// Terminal heatmap renderer
#[derive(Debug, Clone)]
pub struct TerminalHeatmap {
    cells: Vec<Vec<f32>>,
    palette: ColorPalette,
    use_color: bool,
}

impl TerminalHeatmap {
    /// Create from coverage tracker
    #[must_use]
    pub fn from_tracker(tracker: &PixelCoverageTracker) -> Self {
        let cells = tracker
            .cells()
            .iter()
            .map(|row| row.iter().map(|c| c.coverage).collect())
            .collect();

        Self {
            cells,
            palette: ColorPalette::default(),
            use_color: true,
        }
    }

    /// Create from raw coverage values
    #[must_use]
    pub fn from_values(cells: Vec<Vec<f32>>) -> Self {
        Self {
            cells,
            palette: ColorPalette::default(),
            use_color: true,
        }
    }

    /// Set color palette
    #[must_use]
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Disable ANSI color output
    #[must_use]
    pub fn without_color(mut self) -> Self {
        self.use_color = false;
        self
    }

    /// Render to string
    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();

        for row in &self.cells {
            for &coverage in row {
                let char = Self::coverage_to_char(coverage);

                if self.use_color {
                    let color = self.palette.color_for_coverage(coverage);
                    output.push_str(&format!(
                        "\x1b[38;2;{};{};{}m{}\x1b[0m",
                        color.r, color.g, color.b, char
                    ));
                } else {
                    output.push(char);
                }
            }
            output.push('\n');
        }

        output
    }

    /// Render with border
    #[must_use]
    pub fn render_with_border(&self) -> String {
        let width = self.cells.first().map_or(0, Vec::len);
        let mut output = String::new();

        // Top border
        output.push('┌');
        for _ in 0..width {
            output.push('─');
        }
        output.push_str("┐\n");

        // Content
        for row in &self.cells {
            output.push('│');
            for &coverage in row {
                let char = Self::coverage_to_char(coverage);

                if self.use_color {
                    let color = self.palette.color_for_coverage(coverage);
                    output.push_str(&format!(
                        "\x1b[38;2;{};{};{}m{}\x1b[0m",
                        color.r, color.g, color.b, char
                    ));
                } else {
                    output.push(char);
                }
            }
            output.push_str("│\n");
        }

        // Bottom border
        output.push('└');
        for _ in 0..width {
            output.push('─');
        }
        output.push('┘');

        output
    }

    /// Convert coverage value to Unicode block character
    fn coverage_to_char(coverage: f32) -> char {
        match coverage {
            c if c <= 0.0 => ' ',    // Empty
            c if c <= 0.25 => '░',   // Light shade
            c if c <= 0.50 => '▒',   // Medium shade
            c if c <= 0.75 => '▓',   // Dark shade
            _ => '█',                // Full block
        }
    }

    /// Render legend
    #[must_use]
    pub fn legend(&self) -> String {
        let mut legend = String::from("Legend:\n");

        if self.use_color {
            let c = &self.palette;
            legend.push_str(&format!(
                "  \x1b[38;2;{};{};{}m \x1b[0m = 0% (untested)\n",
                c.zero.r, c.zero.g, c.zero.b
            ));
            legend.push_str(&format!(
                "  \x1b[38;2;{};{};{}m░\x1b[0m = 1-25%\n",
                c.low.r, c.low.g, c.low.b
            ));
            legend.push_str(&format!(
                "  \x1b[38;2;{};{};{}m▒\x1b[0m = 26-50%\n",
                c.medium.r, c.medium.g, c.medium.b
            ));
            legend.push_str(&format!(
                "  \x1b[38;2;{};{};{}m▓\x1b[0m = 51-75%\n",
                c.high.r, c.high.g, c.high.b
            ));
            legend.push_str(&format!(
                "  \x1b[38;2;{};{};{}m█\x1b[0m = 76-100%\n",
                c.full.r, c.full.g, c.full.b
            ));
        } else {
            legend.push_str("    = 0% (untested)\n");
            legend.push_str("  ░ = 1-25%\n");
            legend.push_str("  ▒ = 26-50%\n");
            legend.push_str("  ▓ = 51-75%\n");
            legend.push_str("  █ = 76-100%\n");
        }

        legend
    }
}

/// Heatmap renderer trait for extensibility
pub trait HeatmapRenderer {
    /// Render heatmap to string
    fn render(&self, cells: &[Vec<CoverageCell>]) -> String;
}

/// SVG heatmap export
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SvgHeatmap {
    width: u32,
    height: u32,
    palette: ColorPalette,
}

#[allow(dead_code)]
impl SvgHeatmap {
    /// Create new SVG exporter
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            palette: ColorPalette::default(),
        }
    }

    /// Set color palette
    #[must_use]
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Export to SVG string
    #[must_use]
    pub fn export(&self, cells: &[Vec<CoverageCell>]) -> String {
        let rows = cells.len();
        let cols = cells.first().map_or(0, Vec::len);

        if rows == 0 || cols == 0 {
            return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>");
        }

        let cell_width = self.width / cols as u32;
        let cell_height = self.height / rows as u32;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            self.width, self.height, self.width, self.height
        );

        svg.push_str("\n  <style>.cell { stroke: #333; stroke-width: 0.5; }</style>\n");

        for (row_idx, row) in cells.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let x = col_idx as u32 * cell_width;
                let y = row_idx as u32 * cell_height;
                let color = self.palette.color_for_coverage(cell.coverage);

                svg.push_str(&format!(
                    r#"  <rect class="cell" x="{}" y="{}" width="{}" height="{}" fill="rgb({},{},{})"/>"#,
                    x, y, cell_width, cell_height, color.r, color.g, color.b
                ));
                svg.push('\n');
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_from_hex() {
        let red = Rgb::from_hex(0xFF0000);
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);

        let white = Rgb::from_hex(0xFFFFFF);
        assert_eq!(white.r, 255);
        assert_eq!(white.g, 255);
        assert_eq!(white.b, 255);
    }

    #[test]
    fn test_color_palette_viridis() {
        let palette = ColorPalette::viridis();
        assert_ne!(palette.zero, palette.full);
    }

    #[test]
    fn test_color_for_coverage() {
        let palette = ColorPalette::traffic_light();

        assert_eq!(palette.color_for_coverage(0.0), palette.zero);
        assert_eq!(palette.color_for_coverage(0.1), palette.low);
        assert_eq!(palette.color_for_coverage(0.4), palette.medium);
        assert_eq!(palette.color_for_coverage(0.6), palette.high);
        assert_eq!(palette.color_for_coverage(1.0), palette.full);
    }

    #[test]
    fn test_terminal_heatmap_render() {
        let cells = vec![
            vec![0.0, 0.25, 0.5],
            vec![0.75, 1.0, 0.0],
        ];

        let heatmap = TerminalHeatmap::from_values(cells).without_color();
        let rendered = heatmap.render();

        assert!(rendered.contains(' ')); // 0% coverage
        assert!(rendered.contains('█')); // 100% coverage
    }

    #[test]
    fn test_terminal_heatmap_with_border() {
        let cells = vec![
            vec![1.0, 1.0],
            vec![0.0, 0.0],
        ];

        let heatmap = TerminalHeatmap::from_values(cells).without_color();
        let rendered = heatmap.render_with_border();

        assert!(rendered.contains('┌'));
        assert!(rendered.contains('┘'));
        assert!(rendered.contains('│'));
    }

    #[test]
    fn test_coverage_to_char() {
        assert_eq!(TerminalHeatmap::coverage_to_char(0.0), ' ');
        assert_eq!(TerminalHeatmap::coverage_to_char(0.1), '░');
        assert_eq!(TerminalHeatmap::coverage_to_char(0.3), '▒');
        assert_eq!(TerminalHeatmap::coverage_to_char(0.6), '▓');
        assert_eq!(TerminalHeatmap::coverage_to_char(1.0), '█');
    }

    #[test]
    fn test_svg_export() {
        let cells = vec![
            vec![CoverageCell { hit_count: 1, coverage: 1.0 }],
        ];

        let svg = SvgHeatmap::new(100, 100).export(&cells);

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_svg_empty_cells() {
        let cells: Vec<Vec<CoverageCell>> = vec![];
        let svg = SvgHeatmap::new(100, 100).export(&cells);
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_legend() {
        let cells = vec![vec![1.0]];
        let heatmap = TerminalHeatmap::from_values(cells).without_color();
        let legend = heatmap.legend();

        assert!(legend.contains("Legend:"));
        assert!(legend.contains("░"));
        assert!(legend.contains("█"));
    }
}
