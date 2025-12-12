//! Coverage Report Generation
//!
//! Per spec §9.1 and Appendix B: Coverage Report Schema
//!
//! Generates comprehensive coverage reports including:
//! - Block-level coverage data
//! - Summary statistics
//! - Source location mapping
//! - Nullification test results

use super::{BlockId, CoverageViolation, TaintedBlocks};
use std::collections::HashMap;

/// Coverage summary statistics
#[derive(Debug, Clone)]
pub struct CoverageSummary {
    /// Total number of blocks
    pub total_blocks: usize,
    /// Number of covered blocks (hit_count > 0)
    pub covered_blocks: usize,
    /// Coverage percentage
    pub coverage_percent: f64,
    /// 95% confidence interval (for multiple runs)
    pub confidence_interval: Option<(f64, f64)>,
    /// Effect size (Cohen's d)
    pub effect_size: Option<f64>,
}

/// Per-block coverage information
#[derive(Debug, Clone)]
pub struct BlockCoverage {
    /// Block identifier
    pub block_id: BlockId,
    /// Number of times this block was hit
    pub hit_count: u64,
    /// Source location (e.g., "src/pong.rs:142")
    pub source_location: Option<String>,
    /// Function name containing this block
    pub function_name: Option<String>,
}

/// Coverage report containing all coverage data
#[derive(Debug)]
pub struct CoverageReport {
    /// Total number of blocks
    total_blocks: usize,
    /// Hit counts per block
    hit_counts: HashMap<BlockId, u64>,
    /// Source locations per block
    source_locations: HashMap<BlockId, String>,
    /// Function names per block
    function_names: HashMap<BlockId, String>,
    /// Tainted blocks tracker
    tainted: TaintedBlocks,
    /// Session name
    session_name: Option<String>,
    /// Tests run in this session
    tests: Vec<String>,
}

impl CoverageReport {
    /// Create a new coverage report for the given number of blocks
    #[must_use]
    pub fn new(total_blocks: usize) -> Self {
        Self {
            total_blocks,
            hit_counts: HashMap::new(),
            source_locations: HashMap::new(),
            function_names: HashMap::new(),
            tainted: TaintedBlocks::new(),
            session_name: None,
            tests: Vec::new(),
        }
    }

    /// Set the session name
    pub fn set_session_name(&mut self, name: &str) {
        self.session_name = Some(name.to_string());
    }

    /// Add a test to the report
    pub fn add_test(&mut self, name: &str) {
        self.tests.push(name.to_string());
    }

    /// Record a hit on a block
    pub fn record_hit(&mut self, block: BlockId) {
        *self.hit_counts.entry(block).or_insert(0) += 1;
    }

    /// Record multiple hits on a block
    pub fn record_hits(&mut self, block: BlockId, count: u64) {
        *self.hit_counts.entry(block).or_insert(0) += count;
    }

    /// Record a violation
    pub fn record_violation(&mut self, violation: CoverageViolation) {
        self.tainted.record_violation(violation);
    }

    /// Set source location for a block
    pub fn set_source_location(&mut self, block: BlockId, location: &str) {
        let _ = self.source_locations.insert(block, location.to_string());
    }

    /// Set function name for a block
    pub fn set_function_name(&mut self, block: BlockId, name: &str) {
        let _ = self.function_names.insert(block, name.to_string());
    }

    /// Get the hit count for a block
    #[must_use]
    pub fn get_hit_count(&self, block: BlockId) -> u64 {
        self.hit_counts.get(&block).copied().unwrap_or(0)
    }

    /// Check if a block is covered
    #[must_use]
    pub fn is_covered(&self, block: BlockId) -> bool {
        self.get_hit_count(block) > 0
    }

    /// Get the number of covered blocks
    #[must_use]
    pub fn covered_count(&self) -> usize {
        self.hit_counts.values().filter(|&&c| c > 0).count()
    }

    /// Get the coverage percentage
    #[must_use]
    pub fn coverage_percent(&self) -> f64 {
        if self.total_blocks == 0 {
            return 100.0; // Vacuously true
        }
        (self.covered_count() as f64 / self.total_blocks as f64) * 100.0
    }

    /// Get all uncovered blocks
    #[must_use]
    pub fn uncovered_blocks(&self) -> Vec<BlockId> {
        (0..self.total_blocks as u32)
            .map(BlockId::new)
            .filter(|b| !self.is_covered(*b))
            .collect()
    }

    /// Get all covered blocks
    #[must_use]
    pub fn covered_blocks(&self) -> Vec<BlockId> {
        (0..self.total_blocks as u32)
            .map(BlockId::new)
            .filter(|b| self.is_covered(*b))
            .collect()
    }

    /// Get coverage summary
    #[must_use]
    pub fn summary(&self) -> CoverageSummary {
        CoverageSummary {
            total_blocks: self.total_blocks,
            covered_blocks: self.covered_count(),
            coverage_percent: self.coverage_percent(),
            confidence_interval: None,
            effect_size: None,
        }
    }

    /// Get block coverage details
    #[must_use]
    pub fn block_coverages(&self) -> Vec<BlockCoverage> {
        (0..self.total_blocks as u32)
            .map(|i| {
                let block_id = BlockId::new(i);
                BlockCoverage {
                    block_id,
                    hit_count: self.get_hit_count(block_id),
                    source_location: self.source_locations.get(&block_id).cloned(),
                    function_name: self.function_names.get(&block_id).cloned(),
                }
            })
            .collect()
    }

    /// Get the number of violations recorded
    #[must_use]
    pub fn violation_count(&self) -> usize {
        self.tainted.violation_count()
    }

    /// Get all violations
    #[must_use]
    pub fn violations(&self) -> &[CoverageViolation] {
        self.tainted.all_violations()
    }

    /// Check if a block is tainted
    #[must_use]
    pub fn is_tainted(&self, block: BlockId) -> bool {
        self.tainted.is_tainted(block)
    }

    /// Get the total number of blocks
    #[must_use]
    pub fn total_blocks(&self) -> usize {
        self.total_blocks
    }

    /// Get the session name
    #[must_use]
    pub fn session_name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    /// Get the list of tests
    #[must_use]
    pub fn tests(&self) -> &[String] {
        &self.tests
    }

    /// Merge another report into this one
    pub fn merge(&mut self, other: &CoverageReport) {
        for (block, count) in &other.hit_counts {
            self.record_hits(*block, *count);
        }
        for (block, location) in &other.source_locations {
            if !self.source_locations.contains_key(block) {
                let _ = self.source_locations.insert(*block, location.clone());
            }
        }
        for (block, name) in &other.function_names {
            if !self.function_names.contains_key(block) {
                let _ = self.function_names.insert(*block, name.clone());
            }
        }
        for test in &other.tests {
            if !self.tests.contains(test) {
                self.tests.push(test.clone());
            }
        }
    }
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self::new(0)
    }
}
