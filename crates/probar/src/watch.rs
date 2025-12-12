//! Watch Mode with Hot Reload (Feature 6)
//!
//! Automatic test re-execution on file changes.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Jidoka**: Immediate feedback on test failures
//! - **Kaizen**: Continuous improvement through rapid iteration
//! - **Muda**: Only re-run affected tests (smart filtering)

use crate::result::{ProbarError, ProbarResult};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// Configuration for watch mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Patterns to watch (glob patterns)
    pub patterns: Vec<String>,
    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Debounce duration in milliseconds
    pub debounce_ms: u64,
    /// Whether to clear screen before re-run
    pub clear_screen: bool,
    /// Whether to run on initial start
    pub run_on_start: bool,
    /// Directories to watch
    pub watch_dirs: Vec<PathBuf>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            patterns: vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
            ignore_patterns: vec![
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
            ],
            debounce_ms: 300,
            clear_screen: true,
            run_on_start: true,
            watch_dirs: vec![PathBuf::from(".")],
        }
    }
}

impl WatchConfig {
    /// Create a new watch config
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pattern to watch
    #[must_use]
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.patterns.push(pattern.to_string());
        self
    }

    /// Add a pattern to ignore
    #[must_use]
    pub fn with_ignore(mut self, pattern: &str) -> Self {
        self.ignore_patterns.push(pattern.to_string());
        self
    }

    /// Set debounce duration
    #[must_use]
    pub const fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// Set clear screen behavior
    #[must_use]
    pub const fn with_clear_screen(mut self, clear: bool) -> Self {
        self.clear_screen = clear;
        self
    }

    /// Add a directory to watch
    #[must_use]
    pub fn with_watch_dir(mut self, dir: &Path) -> Self {
        self.watch_dirs.push(dir.to_path_buf());
        self
    }

    /// Check if a path matches watch patterns
    #[must_use]
    pub fn matches_pattern(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check ignore patterns first
        for pattern in &self.ignore_patterns {
            if Self::glob_matches(pattern, &path_str) {
                return false;
            }
        }

        // Check watch patterns
        for pattern in &self.patterns {
            if Self::glob_matches(pattern, &path_str) {
                return true;
            }
        }

        false
    }

    /// Simple glob matching (supports **, *, ?)
    fn glob_matches(pattern: &str, path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        Self::glob_match_parts(&pattern_parts, &path_parts)
    }

    fn glob_match_parts(pattern_parts: &[&str], path_parts: &[&str]) -> bool {
        if pattern_parts.is_empty() {
            return path_parts.is_empty();
        }

        let first_pattern = pattern_parts[0];

        if first_pattern == "**" {
            // ** matches zero or more path segments
            let rest_pattern = &pattern_parts[1..];
            if rest_pattern.is_empty() {
                return true;
            }

            for i in 0..=path_parts.len() {
                if Self::glob_match_parts(rest_pattern, &path_parts[i..]) {
                    return true;
                }
            }
            return false;
        }

        if path_parts.is_empty() {
            return false;
        }

        let first_path = path_parts[0];

        // Match current segment
        if Self::glob_match_segment(first_pattern, first_path) {
            Self::glob_match_parts(&pattern_parts[1..], &path_parts[1..])
        } else {
            false
        }
    }

    fn glob_match_segment(pattern: &str, segment: &str) -> bool {
        // Handle * and ? in segment matching
        let mut pattern_chars = pattern.chars().peekable();
        let mut segment_chars = segment.chars();

        while let Some(p) = pattern_chars.next() {
            match p {
                '*' => {
                    // * matches any sequence of characters
                    if pattern_chars.peek().is_none() {
                        return true;
                    }
                    // Try matching remaining pattern at each position
                    let remaining: String = pattern_chars.collect();
                    let remaining_segment: String = segment_chars.collect();
                    for i in 0..=remaining_segment.len() {
                        if Self::glob_match_segment(&remaining, &remaining_segment[i..]) {
                            return true;
                        }
                    }
                    return false;
                }
                '?' => {
                    // ? matches any single character
                    if segment_chars.next().is_none() {
                        return false;
                    }
                }
                c => {
                    if segment_chars.next() != Some(c) {
                        return false;
                    }
                }
            }
        }

        segment_chars.next().is_none()
    }
}

/// A file change event
#[derive(Debug, Clone)]
pub struct FileChange {
    /// The changed file path
    pub path: PathBuf,
    /// Type of change
    pub kind: FileChangeKind,
    /// Timestamp of the change
    pub timestamp: Instant,
}

/// Kind of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileChangeKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
    /// Unknown change type
    Other,
}

impl From<EventKind> for FileChangeKind {
    fn from(kind: EventKind) -> Self {
        match kind {
            EventKind::Create(_) => Self::Created,
            EventKind::Modify(_) => Self::Modified,
            EventKind::Remove(_) => Self::Deleted,
            EventKind::Other => Self::Other,
            EventKind::Any | EventKind::Access(_) => Self::Other,
        }
    }
}

/// Watch mode handler trait
pub trait WatchHandler: Send + Sync {
    /// Called when files change
    fn on_change(&self, changes: &[FileChange]) -> ProbarResult<()>;

    /// Called when watch starts
    fn on_start(&self) -> ProbarResult<()> {
        Ok(())
    }

    /// Called when watch stops
    fn on_stop(&self) -> ProbarResult<()> {
        Ok(())
    }
}

/// Simple closure-based watch handler
pub struct FnWatchHandler<F>
where
    F: Fn(&[FileChange]) -> ProbarResult<()> + Send + Sync,
{
    handler: F,
}

impl<F> std::fmt::Debug for FnWatchHandler<F>
where
    F: Fn(&[FileChange]) -> ProbarResult<()> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnWatchHandler").finish_non_exhaustive()
    }
}

impl<F> FnWatchHandler<F>
where
    F: Fn(&[FileChange]) -> ProbarResult<()> + Send + Sync,
{
    /// Create a new function-based handler
    #[must_use]
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> WatchHandler for FnWatchHandler<F>
where
    F: Fn(&[FileChange]) -> ProbarResult<()> + Send + Sync,
{
    fn on_change(&self, changes: &[FileChange]) -> ProbarResult<()> {
        (self.handler)(changes)
    }
}

/// File system watcher for watch mode
pub struct FileWatcher {
    config: WatchConfig,
    watcher: Option<RecommendedWatcher>,
    receiver: Option<Receiver<Result<Event, notify::Error>>>,
    last_trigger: Option<Instant>,
    pending_changes: Vec<FileChange>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatchConfig) -> ProbarResult<Self> {
        Ok(Self {
            config,
            watcher: None,
            receiver: None,
            last_trigger: None,
            pending_changes: Vec::new(),
        })
    }

    /// Start watching
    pub fn start(&mut self) -> ProbarResult<()> {
        let (tx, rx): (
            Sender<Result<Event, notify::Error>>,
            Receiver<Result<Event, notify::Error>>,
        ) = channel();

        let watcher_config = Config::default().with_poll_interval(Duration::from_millis(100));

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                // Ignore send errors (receiver may have dropped)
                let _ = tx.send(res);
            },
            watcher_config,
        )
        .map_err(|e| {
            ProbarError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create watcher: {e}"),
            ))
        })?;

        // Watch configured directories
        for dir in &self.config.watch_dirs {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::Recursive).map_err(|e| {
                    ProbarError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to watch directory {:?}: {e}", dir),
                    ))
                })?;
            }
        }

        self.watcher = Some(watcher);
        self.receiver = Some(rx);
        Ok(())
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher = None;
        self.receiver = None;
    }

    /// Check for changes (non-blocking)
    pub fn check_changes(&mut self) -> Option<Vec<FileChange>> {
        let receiver = self.receiver.as_ref()?;
        let now = Instant::now();

        // Collect all pending events
        while let Ok(result) = receiver.try_recv() {
            if let Ok(event) = result {
                for path in event.paths {
                    if self.config.matches_pattern(&path) {
                        self.pending_changes.push(FileChange {
                            path,
                            kind: event.kind.into(),
                            timestamp: now,
                        });
                    }
                }
            }
        }

        // Check if we should trigger (debounce)
        if self.pending_changes.is_empty() {
            return None;
        }

        let should_trigger = match self.last_trigger {
            Some(last) => now.duration_since(last).as_millis() >= self.config.debounce_ms as u128,
            None => true,
        };

        if should_trigger {
            self.last_trigger = Some(now);
            let changes = std::mem::take(&mut self.pending_changes);

            // Deduplicate by path
            let unique_paths: HashSet<_> = changes.iter().map(|c| c.path.clone()).collect();
            let deduped: Vec<FileChange> = unique_paths
                .into_iter()
                .filter_map(|path| changes.iter().find(|c| c.path == path).cloned())
                .collect();

            if deduped.is_empty() {
                None
            } else {
                Some(deduped)
            }
        } else {
            None
        }
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &WatchConfig {
        &self.config
    }

    /// Check if watcher is running
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }
}

impl std::fmt::Debug for FileWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileWatcher")
            .field("config", &self.config)
            .field("is_running", &self.is_running())
            .field("pending_changes", &self.pending_changes.len())
            .finish()
    }
}

/// Watch session state
#[derive(Debug, Clone, Default)]
pub struct WatchStats {
    /// Number of times tests were triggered
    pub trigger_count: u64,
    /// Number of file changes detected
    pub change_count: u64,
    /// Total run time
    pub total_runtime: Duration,
    /// Time of last trigger
    pub last_trigger: Option<Instant>,
}

impl WatchStats {
    /// Create new stats
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a trigger
    pub fn record_trigger(&mut self, change_count: usize) {
        self.trigger_count += 1;
        self.change_count += change_count as u64;
        self.last_trigger = Some(Instant::now());
    }
}

/// Builder for creating watch mode configurations
#[derive(Debug)]
pub struct WatchBuilder {
    config: WatchConfig,
}

impl Default for WatchBuilder {
    fn default() -> Self {
        Self {
            config: WatchConfig {
                patterns: Vec::new(),
                ignore_patterns: Vec::new(),
                debounce_ms: 300,
                clear_screen: true,
                run_on_start: true,
                watch_dirs: vec![std::path::PathBuf::from(".")],
            },
        }
    }
}

impl WatchBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Watch Rust files
    #[must_use]
    pub fn rust_files(mut self) -> Self {
        self.config.patterns.push("**/*.rs".to_string());
        self
    }

    /// Watch TOML files
    #[must_use]
    pub fn toml_files(mut self) -> Self {
        self.config.patterns.push("**/*.toml".to_string());
        self
    }

    /// Watch test files only
    #[must_use]
    pub fn test_files(mut self) -> Self {
        self.config.patterns.push("**/tests/**/*.rs".to_string());
        self.config.patterns.push("**/*_test.rs".to_string());
        self.config.patterns.push("**/test_*.rs".to_string());
        self
    }

    /// Watch source directory
    #[must_use]
    pub fn src_dir(mut self) -> Self {
        self.config.watch_dirs.push(PathBuf::from("src"));
        self
    }

    /// Ignore target directory
    #[must_use]
    pub fn ignore_target(mut self) -> Self {
        self.config.ignore_patterns.push("**/target/**".to_string());
        self
    }

    /// Set debounce duration
    #[must_use]
    pub const fn debounce(mut self, ms: u64) -> Self {
        self.config.debounce_ms = ms;
        self
    }

    /// Build the configuration
    #[must_use]
    pub fn build(self) -> WatchConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod watch_config_tests {
        use super::*;

        #[test]
        fn test_default() {
            let config = WatchConfig::default();
            assert!(!config.patterns.is_empty());
            assert!(!config.ignore_patterns.is_empty());
            assert_eq!(config.debounce_ms, 300);
        }

        #[test]
        fn test_with_pattern() {
            let config = WatchConfig::new().with_pattern("**/*.js");
            assert!(config.patterns.contains(&"**/*.js".to_string()));
        }

        #[test]
        fn test_with_ignore() {
            let config = WatchConfig::new().with_ignore("**/dist/**");
            assert!(config.ignore_patterns.contains(&"**/dist/**".to_string()));
        }

        #[test]
        fn test_with_debounce() {
            let config = WatchConfig::new().with_debounce(500);
            assert_eq!(config.debounce_ms, 500);
        }

        #[test]
        fn test_matches_pattern_rust_file() {
            let config = WatchConfig::default();
            assert!(config.matches_pattern(Path::new("src/main.rs")));
            assert!(config.matches_pattern(Path::new("tests/test.rs")));
        }

        #[test]
        fn test_matches_pattern_toml_file() {
            let config = WatchConfig::default();
            assert!(config.matches_pattern(Path::new("Cargo.toml")));
        }

        #[test]
        fn test_ignores_target() {
            let config = WatchConfig::default();
            assert!(!config.matches_pattern(Path::new("target/debug/main.rs")));
        }

        #[test]
        fn test_ignores_git() {
            let config = WatchConfig::default();
            assert!(!config.matches_pattern(Path::new(".git/config")));
        }
    }

    mod glob_matching_tests {
        use super::*;

        #[test]
        fn test_exact_match() {
            assert!(WatchConfig::glob_match_segment("main.rs", "main.rs"));
            assert!(!WatchConfig::glob_match_segment("main.rs", "test.rs"));
        }

        #[test]
        fn test_star_wildcard() {
            assert!(WatchConfig::glob_match_segment("*.rs", "main.rs"));
            assert!(WatchConfig::glob_match_segment("*.rs", "test.rs"));
            assert!(!WatchConfig::glob_match_segment("*.rs", "main.js"));
        }

        #[test]
        fn test_question_wildcard() {
            assert!(WatchConfig::glob_match_segment("?.rs", "a.rs"));
            assert!(!WatchConfig::glob_match_segment("?.rs", "ab.rs"));
        }

        #[test]
        fn test_double_star() {
            assert!(WatchConfig::glob_matches("**/*.rs", "src/main.rs"));
            assert!(WatchConfig::glob_matches("**/*.rs", "src/lib/mod.rs"));
            assert!(WatchConfig::glob_matches(
                "**/target/**",
                "crates/probar/target/debug/build"
            ));
        }

        #[test]
        fn test_prefix_pattern() {
            assert!(WatchConfig::glob_matches("src/**/*.rs", "src/lib.rs"));
            assert!(WatchConfig::glob_matches(
                "src/**/*.rs",
                "src/modules/test.rs"
            ));
            assert!(!WatchConfig::glob_matches("src/**/*.rs", "tests/test.rs"));
        }
    }

    mod file_change_tests {
        use super::*;

        #[test]
        fn test_file_change_kind_from_event() {
            assert_eq!(
                FileChangeKind::from(EventKind::Create(notify::event::CreateKind::File)),
                FileChangeKind::Created
            );
            assert_eq!(
                FileChangeKind::from(EventKind::Modify(notify::event::ModifyKind::Data(
                    notify::event::DataChange::Content
                ))),
                FileChangeKind::Modified
            );
            assert_eq!(
                FileChangeKind::from(EventKind::Remove(notify::event::RemoveKind::File)),
                FileChangeKind::Deleted
            );
        }
    }

    mod file_watcher_tests {
        use super::*;

        #[test]
        fn test_new() {
            let config = WatchConfig::default();
            let watcher = FileWatcher::new(config);
            assert!(watcher.is_ok());
        }

        #[test]
        fn test_is_running_before_start() {
            let config = WatchConfig::default();
            let watcher = FileWatcher::new(config).unwrap();
            assert!(!watcher.is_running());
        }

        #[test]
        fn test_check_changes_before_start() {
            let config = WatchConfig::default();
            let mut watcher = FileWatcher::new(config).unwrap();
            assert!(watcher.check_changes().is_none());
        }
    }

    mod watch_stats_tests {
        use super::*;

        #[test]
        fn test_new() {
            let stats = WatchStats::new();
            assert_eq!(stats.trigger_count, 0);
            assert_eq!(stats.change_count, 0);
        }

        #[test]
        fn test_record_trigger() {
            let mut stats = WatchStats::new();
            stats.record_trigger(3);

            assert_eq!(stats.trigger_count, 1);
            assert_eq!(stats.change_count, 3);
            assert!(stats.last_trigger.is_some());
        }

        #[test]
        fn test_multiple_triggers() {
            let mut stats = WatchStats::new();
            stats.record_trigger(2);
            stats.record_trigger(5);

            assert_eq!(stats.trigger_count, 2);
            assert_eq!(stats.change_count, 7);
        }
    }

    mod watch_builder_tests {
        use super::*;

        #[test]
        fn test_new() {
            let builder = WatchBuilder::new();
            let config = builder.build();
            assert!(config.patterns.is_empty());
        }

        #[test]
        fn test_rust_files() {
            let config = WatchBuilder::new().rust_files().build();
            assert!(config.patterns.contains(&"**/*.rs".to_string()));
        }

        #[test]
        fn test_toml_files() {
            let config = WatchBuilder::new().toml_files().build();
            assert!(config.patterns.contains(&"**/*.toml".to_string()));
        }

        #[test]
        fn test_test_files() {
            let config = WatchBuilder::new().test_files().build();
            assert!(config.patterns.contains(&"**/tests/**/*.rs".to_string()));
            assert!(config.patterns.contains(&"**/*_test.rs".to_string()));
        }

        #[test]
        fn test_ignore_target() {
            let config = WatchBuilder::new().ignore_target().build();
            assert!(config.ignore_patterns.contains(&"**/target/**".to_string()));
        }

        #[test]
        fn test_debounce() {
            let config = WatchBuilder::new().debounce(500).build();
            assert_eq!(config.debounce_ms, 500);
        }

        #[test]
        fn test_chained_builder() {
            let config = WatchBuilder::new()
                .rust_files()
                .toml_files()
                .ignore_target()
                .debounce(200)
                .build();

            assert!(config.patterns.contains(&"**/*.rs".to_string()));
            assert!(config.patterns.contains(&"**/*.toml".to_string()));
            assert!(config.ignore_patterns.contains(&"**/target/**".to_string()));
            assert_eq!(config.debounce_ms, 200);
        }
    }

    mod fn_watch_handler_tests {
        use super::*;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        #[test]
        fn test_on_change() {
            let counter = Arc::new(AtomicU32::new(0));
            let counter_clone = Arc::clone(&counter);

            let handler = FnWatchHandler::new(move |_changes| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });

            let changes = vec![FileChange {
                path: PathBuf::from("test.rs"),
                kind: FileChangeKind::Modified,
                timestamp: Instant::now(),
            }];

            handler.on_change(&changes).unwrap();
            assert_eq!(counter.load(Ordering::SeqCst), 1);
        }
    }
}
