//! Visual snapshot testing.
//!
//! Per spec Section 6.2: Visual Regression Testing

/// Configuration for snapshot testing
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Whether to update snapshots on mismatch
    pub update_snapshots: bool,
    /// Difference threshold (0.0-1.0)
    pub threshold: f64,
    /// Directory to store snapshots
    pub snapshot_dir: String,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            update_snapshots: false,
            threshold: 0.01, // 1% difference allowed
            snapshot_dir: String::from("__snapshots__"),
        }
    }
}

impl SnapshotConfig {
    /// Set update mode
    #[must_use]
    pub const fn with_update(mut self, update: bool) -> Self {
        self.update_snapshots = update;
        self
    }

    /// Set threshold
    #[must_use]
    pub const fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set snapshot directory
    #[must_use]
    pub fn with_dir(mut self, dir: impl Into<String>) -> Self {
        self.snapshot_dir = dir.into();
        self
    }
}

/// A visual snapshot
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Snapshot name/identifier
    pub name: String,
    /// Raw image data
    pub data: Vec<u8>,
    /// Image width
    pub width: u32,
    /// Image height
    pub height: u32,
}

impl Snapshot {
    /// Create a new snapshot
    #[must_use]
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data,
            width: 0,
            height: 0,
        }
    }

    /// Create with dimensions
    #[must_use]
    pub const fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Compare this snapshot to another
    #[must_use]
    pub fn diff(&self, other: &Self) -> SnapshotDiff {
        // Simple byte-by-byte comparison
        let mut difference_count = 0;
        let max_len = self.data.len().max(other.data.len());

        if max_len == 0 {
            return SnapshotDiff {
                identical: true,
                difference_count: 0,
                difference_percent: 0.0,
                diff_data: Vec::new(),
            };
        }

        for i in 0..max_len {
            let a = self.data.get(i).copied().unwrap_or(0);
            let b = other.data.get(i).copied().unwrap_or(0);
            if a != b {
                difference_count += 1;
            }
        }

        #[allow(clippy::cast_precision_loss)] // Acceptable for percentage calculation
        let difference_percent = (difference_count as f64 / max_len as f64) * 100.0;

        SnapshotDiff {
            identical: difference_count == 0,
            difference_count,
            difference_percent,
            diff_data: Vec::new(), // Would contain visual diff in full impl
        }
    }

    /// Get snapshot size in bytes
    #[must_use]
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Result of comparing two snapshots
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    /// Whether snapshots are identical
    pub identical: bool,
    /// Number of differing bytes/pixels
    pub difference_count: usize,
    /// Percentage of difference
    pub difference_percent: f64,
    /// Visual diff data (highlighted differences)
    pub diff_data: Vec<u8>,
}

impl SnapshotDiff {
    /// Check if snapshots are identical
    #[must_use]
    pub const fn is_identical(&self) -> bool {
        self.identical
    }

    /// Check if difference is within threshold
    #[must_use]
    pub fn within_threshold(&self, threshold: f64) -> bool {
        self.difference_percent <= threshold * 100.0
    }
}
