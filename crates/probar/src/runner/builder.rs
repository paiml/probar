//! Build Coordinator for WASM Compilation
//!
//! Manages cargo build process for WASM targets with progress tracking.

use super::config::{OptLevel, RunnerConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Build status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    /// Build not started
    Pending,
    /// Build in progress
    Building,
    /// Build succeeded
    Success,
    /// Build failed
    Failed,
}

/// Build event for progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildEvent {
    /// Build started
    Started {
        /// Timestamp
        timestamp: std::time::SystemTime,
    },
    /// Compiling a crate
    Compiling {
        /// Crate name
        crate_name: String,
        /// Crate version
        version: String,
    },
    /// Build finished
    Finished {
        /// Build result
        success: bool,
        /// Duration
        duration: Duration,
    },
    /// Build error
    Error {
        /// Error message
        message: String,
    },
    /// Optimization step
    Optimizing {
        /// Step name (e.g., "wasm-opt")
        step: String,
    },
}

/// Result of a build operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// Build status
    pub status: BuildStatus,
    /// Size of output WASM in bytes
    pub size: Option<u64>,
    /// Gzipped size in bytes
    pub gzip_size: Option<u64>,
    /// Build duration
    pub duration: Option<Duration>,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
    /// Output path
    pub output_path: Option<PathBuf>,
}

impl BuildResult {
    /// Create a success result
    #[must_use]
    pub fn success(size: u64, duration: Duration) -> Self {
        Self {
            status: BuildStatus::Success,
            size: Some(size),
            gzip_size: None,
            duration: Some(duration),
            errors: Vec::new(),
            warnings: Vec::new(),
            output_path: None,
        }
    }

    /// Create a failure result
    #[must_use]
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            status: BuildStatus::Failed,
            size: None,
            gzip_size: None,
            duration: None,
            errors,
            warnings: Vec::new(),
            output_path: None,
        }
    }

    /// Check if build succeeded
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == BuildStatus::Success
    }

    /// Get size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> Option<u64> {
        self.size
    }

    /// Get size in KB
    #[must_use]
    pub fn size_kb(&self) -> Option<f64> {
        self.size.map(|s| s as f64 / 1024.0)
    }

    /// Get errors
    #[must_use]
    pub fn errors(&self) -> Option<&[String]> {
        if self.errors.is_empty() {
            None
        } else {
            Some(&self.errors)
        }
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Set output path
    pub fn set_output_path(&mut self, path: PathBuf) {
        self.output_path = Some(path);
    }

    /// Set gzip size
    pub fn set_gzip_size(&mut self, size: u64) {
        self.gzip_size = Some(size);
    }
}

/// Build coordinator for managing WASM compilation
#[derive(Debug)]
pub struct BuildCoordinator {
    config: RunnerConfig,
    opt_level: OptLevel,
    status: BuildStatus,
    last_result: Option<BuildResult>,
}

impl BuildCoordinator {
    /// Create a new build coordinator
    #[must_use]
    pub fn new(config: RunnerConfig, opt_level: OptLevel) -> Self {
        Self {
            config,
            opt_level,
            status: BuildStatus::Pending,
            last_result: None,
        }
    }

    /// Get current status
    #[must_use]
    pub fn status(&self) -> BuildStatus {
        self.status
    }

    /// Get last build result
    #[must_use]
    pub fn last_result(&self) -> Option<&BuildResult> {
        self.last_result.as_ref()
    }

    /// Build the cargo command arguments
    #[must_use]
    pub fn build_args(&self) -> Vec<String> {
        let mut args = vec![
            "build".to_string(),
            "--target".to_string(),
            self.config.target.clone(),
        ];

        if self.opt_level.is_release() {
            args.push("--release".to_string());
        }

        if let Some(ref package) = self.config.package {
            args.push("--package".to_string());
            args.push(package.clone());
        }

        for feature in &self.config.features {
            args.push("--features".to_string());
            args.push(feature.clone());
        }

        args
    }

    /// Simulate a build (for testing without actual compilation)
    pub fn simulate_build(&mut self) -> BuildResult {
        self.status = BuildStatus::Building;

        // Simulate successful build
        let result = BuildResult::success(150_000, Duration::from_millis(500));
        self.last_result = Some(result.clone());
        self.status = BuildStatus::Success;

        result
    }

    /// Mark build as started
    pub fn mark_started(&mut self) {
        self.status = BuildStatus::Building;
    }

    /// Mark build as completed
    pub fn mark_completed(&mut self, result: BuildResult) {
        self.status = result.status;
        self.last_result = Some(result);
    }

    /// Get configuration
    #[must_use]
    pub fn config(&self) -> &RunnerConfig {
        &self.config
    }

    /// Get optimization level
    #[must_use]
    pub fn opt_level(&self) -> OptLevel {
        self.opt_level
    }
}

/// Format file size for display
#[allow(dead_code)]
#[must_use]
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_build_result_success() {
        let result = BuildResult::success(1024, Duration::from_secs(1));
        assert!(result.is_success());
        assert_eq!(result.size_bytes(), Some(1024));
        assert!((result.size_kb().unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_build_result_failure() {
        let result = BuildResult::failure(vec!["error".to_string()]);
        assert!(!result.is_success());
        assert!(result.errors().is_some());
        assert_eq!(result.errors().unwrap().len(), 1);
    }

    #[test]
    fn test_build_result_add_warning() {
        let mut result = BuildResult::success(1024, Duration::from_secs(1));
        result.add_warning("test warning");
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_coordinator_new() {
        let config = RunnerConfig::default();
        let coordinator = BuildCoordinator::new(config, OptLevel::Debug);
        assert_eq!(coordinator.status(), BuildStatus::Pending);
    }

    #[test]
    fn test_coordinator_build_args() {
        let mut config = RunnerConfig::default();
        config.package = Some("myapp".to_string());
        config.features = vec!["web".to_string()];

        let coordinator = BuildCoordinator::new(config, OptLevel::Release);
        let args = coordinator.build_args();

        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--target".to_string()));
        assert!(args.contains(&"--release".to_string()));
        assert!(args.contains(&"--package".to_string()));
        assert!(args.contains(&"myapp".to_string()));
        assert!(args.contains(&"--features".to_string()));
        assert!(args.contains(&"web".to_string()));
    }

    #[test]
    fn test_coordinator_simulate_build() {
        let config = RunnerConfig::default();
        let mut coordinator = BuildCoordinator::new(config, OptLevel::Debug);

        let result = coordinator.simulate_build();
        assert!(result.is_success());
        assert_eq!(coordinator.status(), BuildStatus::Success);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.50 MB");
    }

    #[test]
    fn test_build_event_variants() {
        let event = BuildEvent::Started {
            timestamp: std::time::SystemTime::now(),
        };
        assert!(matches!(event, BuildEvent::Started { .. }));

        let event = BuildEvent::Compiling {
            crate_name: "test".to_string(),
            version: "1.0.0".to_string(),
        };
        assert!(matches!(event, BuildEvent::Compiling { .. }));
    }
}
