//! Pixel Coverage Tracker Implementation
//!
//! Tracks which grid cells have been exercised during testing.

use serde::{Deserialize, Serialize};

/// A point in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    /// X coordinate (pixels from left)
    pub x: u32,
    /// Y coordinate (pixels from top)
    pub y: u32,
}

impl Point {
    /// Create a new point
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// A rectangular region in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    /// X coordinate of top-left corner
    pub x: u32,
    /// Y coordinate of top-left corner
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Region {
    /// Create a new region
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is within this region
    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Get the area of this region
    #[must_use]
    pub fn area(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// Grid configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GridConfig {
    /// Screen width in pixels
    pub screen_width: u32,
    /// Screen height in pixels
    pub screen_height: u32,
    /// Number of grid columns
    pub grid_cols: u32,
    /// Number of grid rows
    pub grid_rows: u32,
}

impl GridConfig {
    /// Calculate cell width in pixels
    #[must_use]
    pub fn cell_width(&self) -> u32 {
        self.screen_width / self.grid_cols
    }

    /// Calculate cell height in pixels
    #[must_use]
    pub fn cell_height(&self) -> u32 {
        self.screen_height / self.grid_rows
    }

    /// Convert screen coordinates to grid cell
    #[must_use]
    pub fn point_to_cell(&self, point: Point) -> (u32, u32) {
        let col = (point.x / self.cell_width()).min(self.grid_cols - 1);
        let row = (point.y / self.cell_height()).min(self.grid_rows - 1);
        (col, row)
    }

    /// Convert grid cell to screen region
    #[must_use]
    pub fn cell_to_region(&self, col: u32, row: u32) -> Region {
        Region::new(
            col * self.cell_width(),
            row * self.cell_height(),
            self.cell_width(),
            self.cell_height(),
        )
    }
}

/// A single coverage cell in the grid
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageCell {
    /// Number of times this cell was interacted with
    pub hit_count: u64,
    /// Coverage value (0.0 - 1.0)
    pub coverage: f32,
}

impl CoverageCell {
    /// Check if cell is covered
    #[must_use]
    pub fn is_covered(&self) -> bool {
        self.hit_count > 0
    }
}

/// Pixel coverage tracker for grid-based UI coverage
#[derive(Debug, Clone)]
pub struct PixelCoverageTracker {
    config: GridConfig,
    cells: Vec<Vec<CoverageCell>>,
    threshold: f32,
    total_interactions: u64,
}

impl PixelCoverageTracker {
    /// Create a new tracker with given resolution and grid size
    #[must_use]
    pub fn new(width: u32, height: u32, grid_cols: u32, grid_rows: u32) -> Self {
        let config = GridConfig {
            screen_width: width,
            screen_height: height,
            grid_cols,
            grid_rows,
        };

        let cells = (0..grid_rows)
            .map(|_| (0..grid_cols).map(|_| CoverageCell::default()).collect())
            .collect();

        Self {
            config,
            cells,
            threshold: 0.8,
            total_interactions: 0,
        }
    }

    /// Create a builder for more complex configuration
    #[must_use]
    pub fn builder() -> PixelCoverageTrackerBuilder {
        PixelCoverageTrackerBuilder::default()
    }

    /// Get screen resolution
    #[must_use]
    pub fn resolution(&self) -> (u32, u32) {
        (self.config.screen_width, self.config.screen_height)
    }

    /// Get grid size
    #[must_use]
    pub fn grid_size(&self) -> (u32, u32) {
        (self.config.grid_cols, self.config.grid_rows)
    }

    /// Get coverage threshold
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Get grid configuration
    #[must_use]
    pub fn grid_config(&self) -> &GridConfig {
        &self.config
    }

    /// Record an interaction at a point
    pub fn record_interaction(&mut self, point: Point) {
        let (col, row) = self.config.point_to_cell(point);
        if let Some(row_cells) = self.cells.get_mut(row as usize) {
            if let Some(cell) = row_cells.get_mut(col as usize) {
                cell.hit_count += 1;
                cell.coverage = 1.0;
                self.total_interactions += 1;
            }
        }
    }

    /// Record coverage for a region
    pub fn record_region(&mut self, region: Region) {
        let start_col = region.x / self.config.cell_width();
        let start_row = region.y / self.config.cell_height();
        let end_col = ((region.x + region.width) / self.config.cell_width())
            .min(self.config.grid_cols - 1);
        let end_row = ((region.y + region.height) / self.config.cell_height())
            .min(self.config.grid_rows - 1);

        for row in start_row..=end_row {
            for col in start_col..=end_col {
                if let Some(row_cells) = self.cells.get_mut(row as usize) {
                    if let Some(cell) = row_cells.get_mut(col as usize) {
                        cell.hit_count += 1;
                        cell.coverage = 1.0;
                    }
                }
            }
        }
        self.total_interactions += 1;
    }

    /// Record coverage for an element by ID
    pub fn record_element(&mut self, _id: &str, bounds: Region) {
        self.record_region(bounds);
    }

    /// Generate coverage report
    #[must_use]
    pub fn generate_report(&self) -> PixelCoverageReport {
        let total_cells = self.config.grid_cols * self.config.grid_rows;
        let covered_cells = self
            .cells
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_covered())
            .count() as u32;

        let overall_coverage = if total_cells > 0 {
            covered_cells as f32 / total_cells as f32
        } else {
            0.0
        };

        let min_coverage = self
            .cells
            .iter()
            .flat_map(|row| row.iter())
            .map(|c| c.coverage)
            .fold(f32::MAX, f32::min);

        let max_coverage = self
            .cells
            .iter()
            .flat_map(|row| row.iter())
            .map(|c| c.coverage)
            .fold(0.0_f32, f32::max);

        PixelCoverageReport {
            grid_width: self.config.grid_cols,
            grid_height: self.config.grid_rows,
            overall_coverage,
            covered_cells,
            total_cells,
            min_coverage: if min_coverage == f32::MAX {
                0.0
            } else {
                min_coverage
            },
            max_coverage,
            total_interactions: self.total_interactions,
            meets_threshold: overall_coverage >= self.threshold,
            uncovered_regions: self.find_uncovered_regions(),
        }
    }

    /// Get list of uncovered regions
    #[must_use]
    pub fn uncovered_regions(&self) -> Vec<Region> {
        self.find_uncovered_regions()
    }

    /// Find contiguous uncovered regions
    fn find_uncovered_regions(&self) -> Vec<Region> {
        let mut regions = Vec::new();

        for (row_idx, row) in self.cells.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if !cell.is_covered() {
                    // Convert to screen coordinates
                    let region = self
                        .config
                        .cell_to_region(col_idx as u32, row_idx as u32);
                    regions.push(region);
                }
            }
        }

        regions
    }

    /// Get cells for rendering
    #[must_use]
    pub fn cells(&self) -> &Vec<Vec<CoverageCell>> {
        &self.cells
    }

    /// Generate terminal heatmap
    #[must_use]
    pub fn terminal_heatmap(&self) -> super::heatmap::TerminalHeatmap {
        super::heatmap::TerminalHeatmap::from_tracker(self)
    }
}

/// Builder for `PixelCoverageTracker`
#[derive(Debug, Clone)]
pub struct PixelCoverageTrackerBuilder {
    width: u32,
    height: u32,
    grid_cols: u32,
    grid_rows: u32,
    threshold: f32,
}

impl Default for PixelCoverageTrackerBuilder {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            grid_cols: 64,
            grid_rows: 36,
            threshold: 0.8,
        }
    }
}

impl PixelCoverageTrackerBuilder {
    /// Set screen resolution
    #[must_use]
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set grid size
    #[must_use]
    pub fn grid_size(mut self, cols: u32, rows: u32) -> Self {
        self.grid_cols = cols;
        self.grid_rows = rows;
        self
    }

    /// Set coverage threshold
    #[must_use]
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Build the tracker
    #[must_use]
    pub fn build(self) -> PixelCoverageTracker {
        let mut tracker =
            PixelCoverageTracker::new(self.width, self.height, self.grid_cols, self.grid_rows);
        tracker.threshold = self.threshold;
        tracker
    }
}

/// Pixel coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelCoverageReport {
    /// Grid width (columns)
    pub grid_width: u32,
    /// Grid height (rows)
    pub grid_height: u32,
    /// Overall coverage percentage (0.0 - 1.0)
    pub overall_coverage: f32,
    /// Number of covered cells
    pub covered_cells: u32,
    /// Total number of cells
    pub total_cells: u32,
    /// Minimum coverage in any cell
    pub min_coverage: f32,
    /// Maximum coverage in any cell
    pub max_coverage: f32,
    /// Total number of interactions recorded
    pub total_interactions: u64,
    /// Whether coverage meets the threshold
    pub meets_threshold: bool,
    /// List of uncovered regions
    pub uncovered_regions: Vec<Region>,
}

impl PixelCoverageReport {
    /// Get coverage as percentage (0-100)
    #[must_use]
    pub fn percent(&self) -> f32 {
        self.overall_coverage * 100.0
    }

    /// Check if coverage is complete (100%)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.covered_cells == self.total_cells
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_config_cell_dimensions() {
        let config = GridConfig {
            screen_width: 1920,
            screen_height: 1080,
            grid_cols: 64,
            grid_rows: 36,
        };

        assert_eq!(config.cell_width(), 30);
        assert_eq!(config.cell_height(), 30);
    }

    #[test]
    fn test_grid_config_point_to_cell() {
        let config = GridConfig {
            screen_width: 100,
            screen_height: 100,
            grid_cols: 10,
            grid_rows: 10,
        };

        assert_eq!(config.point_to_cell(Point::new(0, 0)), (0, 0));
        assert_eq!(config.point_to_cell(Point::new(15, 25)), (1, 2));
        assert_eq!(config.point_to_cell(Point::new(99, 99)), (9, 9));
    }

    #[test]
    fn test_coverage_cell_default() {
        let cell = CoverageCell::default();
        assert_eq!(cell.hit_count, 0);
        assert!(!cell.is_covered());
    }

    #[test]
    fn test_builder_defaults() {
        let builder = PixelCoverageTrackerBuilder::default();
        assert_eq!(builder.width, 1920);
        assert_eq!(builder.height, 1080);
        assert_eq!(builder.grid_cols, 64);
        assert_eq!(builder.grid_rows, 36);
    }

    #[test]
    fn test_report_percent() {
        let report = PixelCoverageReport {
            grid_width: 10,
            grid_height: 10,
            overall_coverage: 0.75,
            covered_cells: 75,
            total_cells: 100,
            min_coverage: 0.0,
            max_coverage: 1.0,
            total_interactions: 100,
            meets_threshold: false,
            uncovered_regions: vec![],
        };

        assert!((report.percent() - 75.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_report_is_complete() {
        let complete = PixelCoverageReport {
            grid_width: 10,
            grid_height: 10,
            overall_coverage: 1.0,
            covered_cells: 100,
            total_cells: 100,
            min_coverage: 1.0,
            max_coverage: 1.0,
            total_interactions: 100,
            meets_threshold: true,
            uncovered_regions: vec![],
        };

        assert!(complete.is_complete());

        let incomplete = PixelCoverageReport {
            covered_cells: 99,
            total_cells: 100,
            ..complete
        };

        assert!(!incomplete.is_complete());
    }

    #[test]
    fn test_region_area() {
        let region = Region::new(0, 0, 100, 50);
        assert_eq!(region.area(), 5000);
    }
}
