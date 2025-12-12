//! TUI Frame Assertions (Feature 21 - EDD Compliance)
//!
//! Provides Playwright-style assertions for TUI frames.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Poka-Yoke**: Type-safe assertions prevent invalid comparisons
//! - **Muda**: Fail-fast on first mismatch
//! - **Jidoka**: Clear error messages with visual diff

use super::backend::TuiFrame;
use crate::result::{ProbarError, ProbarResult};
use std::collections::HashMap;

/// Frame assertion builder (Playwright-style API)
#[derive(Debug)]
pub struct FrameAssertion<'a> {
    frame: &'a TuiFrame,
    soft_mode: bool,
    errors: Vec<String>,
}

impl<'a> FrameAssertion<'a> {
    /// Create a new frame assertion
    #[must_use]
    pub fn new(frame: &'a TuiFrame) -> Self {
        Self {
            frame,
            soft_mode: false,
            errors: Vec::new(),
        }
    }

    /// Enable soft assertion mode (collect errors instead of failing immediately)
    #[must_use]
    pub fn soft(mut self) -> Self {
        self.soft_mode = true;
        self
    }

    /// Assert frame contains text
    pub fn to_contain_text(&mut self, text: &str) -> ProbarResult<&mut Self> {
        if !self.frame.contains(text) {
            let msg = format!(
                "Expected frame to contain text '{}'\nFrame content:\n{}",
                text,
                self.frame.as_text()
            );
            if self.soft_mode {
                self.errors.push(msg);
            } else {
                return Err(ProbarError::AssertionFailed { message: msg });
            }
        }
        Ok(self)
    }

    /// Assert frame does not contain text
    pub fn not_to_contain_text(&mut self, text: &str) -> ProbarResult<&mut Self> {
        if self.frame.contains(text) {
            let msg = format!(
                "Expected frame NOT to contain text '{}'\nFrame content:\n{}",
                text,
                self.frame.as_text()
            );
            if self.soft_mode {
                self.errors.push(msg);
            } else {
                return Err(ProbarError::AssertionFailed { message: msg });
            }
        }
        Ok(self)
    }

    /// Assert frame matches regex pattern
    pub fn to_match(&mut self, pattern: &str) -> ProbarResult<&mut Self> {
        let matches = self.frame.matches(pattern)?;
        if !matches {
            let msg = format!(
                "Expected frame to match pattern '{}'\nFrame content:\n{}",
                pattern,
                self.frame.as_text()
            );
            if self.soft_mode {
                self.errors.push(msg);
            } else {
                return Err(ProbarError::AssertionFailed { message: msg });
            }
        }
        Ok(self)
    }

    /// Assert specific line contains text
    pub fn line_to_contain(&mut self, line_num: usize, text: &str) -> ProbarResult<&mut Self> {
        let line = self.frame.line(line_num);
        match line {
            Some(content) if content.contains(text) => Ok(self),
            Some(content) => {
                let msg = format!(
                    "Expected line {} to contain '{}'\nActual: '{}'",
                    line_num, text, content
                );
                if self.soft_mode {
                    self.errors.push(msg);
                    Ok(self)
                } else {
                    Err(ProbarError::AssertionFailed { message: msg })
                }
            }
            None => {
                let msg = format!(
                    "Line {} does not exist (frame has {} lines)",
                    line_num,
                    self.frame.height()
                );
                if self.soft_mode {
                    self.errors.push(msg);
                    Ok(self)
                } else {
                    Err(ProbarError::AssertionFailed { message: msg })
                }
            }
        }
    }

    /// Assert specific line equals exact text
    pub fn line_to_equal(&mut self, line_num: usize, expected: &str) -> ProbarResult<&mut Self> {
        let line = self.frame.line(line_num);
        match line {
            Some(content) if content == expected => Ok(self),
            Some(content) => {
                let msg = format!(
                    "Expected line {} to equal '{}'\nActual: '{}'",
                    line_num, expected, content
                );
                if self.soft_mode {
                    self.errors.push(msg);
                    Ok(self)
                } else {
                    Err(ProbarError::AssertionFailed { message: msg })
                }
            }
            None => {
                let msg = format!(
                    "Line {} does not exist (frame has {} lines)",
                    line_num,
                    self.frame.height()
                );
                if self.soft_mode {
                    self.errors.push(msg);
                    Ok(self)
                } else {
                    Err(ProbarError::AssertionFailed { message: msg })
                }
            }
        }
    }

    /// Assert frame has expected dimensions
    pub fn to_have_size(&mut self, width: u16, height: u16) -> ProbarResult<&mut Self> {
        let actual_width = self.frame.width();
        let actual_height = self.frame.height();

        if actual_width != width || actual_height != height {
            let msg = format!(
                "Expected frame size {}x{}, got {}x{}",
                width, height, actual_width, actual_height
            );
            if self.soft_mode {
                self.errors.push(msg);
            } else {
                return Err(ProbarError::AssertionFailed { message: msg });
            }
        }
        Ok(self)
    }

    /// Assert frame is identical to another frame
    pub fn to_be_identical_to(&mut self, other: &TuiFrame) -> ProbarResult<&mut Self> {
        if !self.frame.is_identical(other) {
            let diff = self.frame.diff(other);
            let msg = format!("Frames are not identical:\n{diff}");
            if self.soft_mode {
                self.errors.push(msg);
            } else {
                return Err(ProbarError::AssertionFailed { message: msg });
            }
        }
        Ok(self)
    }

    /// Finalize soft assertions and return any collected errors
    pub fn finalize(&self) -> ProbarResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(ProbarError::AssertionFailed {
                message: format!(
                    "{} assertion(s) failed:\n{}",
                    self.errors.len(),
                    self.errors.join("\n\n")
                ),
            })
        }
    }

    /// Get collected errors (for soft assertions)
    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

/// Create a frame assertion
#[must_use]
pub fn expect_frame(frame: &TuiFrame) -> FrameAssertion<'_> {
    FrameAssertion::new(frame)
}

/// Value tracker for monitoring changes over time
///
/// Useful for EDD (Equation-Driven Development) where you want to
/// verify that values change according to expected patterns.
#[derive(Debug, Clone)]
pub struct ValueTracker<T: Clone> {
    values: Vec<(u64, T)>, // (timestamp_ms, value)
    name: String,
}

impl<T: Clone> ValueTracker<T> {
    /// Create a new value tracker
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            values: Vec::new(),
            name: name.to_string(),
        }
    }

    /// Record a value at a timestamp
    pub fn record(&mut self, timestamp_ms: u64, value: T) {
        self.values.push((timestamp_ms, value));
    }

    /// Get the tracker name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get all recorded values
    #[must_use]
    pub fn values(&self) -> &[(u64, T)] {
        &self.values
    }

    /// Get the latest value
    #[must_use]
    pub fn latest(&self) -> Option<&T> {
        self.values.last().map(|(_, v)| v)
    }

    /// Get value at specific index
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&T> {
        self.values.get(index).map(|(_, v)| v)
    }

    /// Get the number of recorded values
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if tracker is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Clear all recorded values
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<T: Clone + PartialEq> ValueTracker<T> {
    /// Check if value changed since last recording
    #[must_use]
    pub fn has_changed(&self) -> bool {
        if self.values.len() < 2 {
            return false;
        }
        let last = &self.values[self.values.len() - 1].1;
        let prev = &self.values[self.values.len() - 2].1;
        last != prev
    }

    /// Count how many times the value changed
    #[must_use]
    pub fn change_count(&self) -> usize {
        if self.values.len() < 2 {
            return 0;
        }
        self.values.windows(2).filter(|w| w[0].1 != w[1].1).count()
    }
}

impl ValueTracker<f64> {
    /// Calculate the rate of change (delta per millisecond)
    #[must_use]
    pub fn rate_of_change(&self) -> Option<f64> {
        if self.values.len() < 2 {
            return None;
        }
        let (t1, v1) = &self.values[self.values.len() - 2];
        let (t2, v2) = &self.values[self.values.len() - 1];
        let dt = (*t2 as f64) - (*t1 as f64);
        if dt.abs() < f64::EPSILON {
            return None;
        }
        Some((v2 - v1) / dt)
    }

    /// Check if value is monotonically increasing
    #[must_use]
    pub fn is_increasing(&self) -> bool {
        if self.values.len() < 2 {
            return true;
        }
        self.values.windows(2).all(|w| w[1].1 >= w[0].1)
    }

    /// Check if value is monotonically decreasing
    #[must_use]
    pub fn is_decreasing(&self) -> bool {
        if self.values.len() < 2 {
            return true;
        }
        self.values.windows(2).all(|w| w[1].1 <= w[0].1)
    }

    /// Get the minimum value
    #[must_use]
    pub fn min(&self) -> Option<f64> {
        self.values
            .iter()
            .map(|(_, v)| *v)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Get the maximum value
    #[must_use]
    pub fn max(&self) -> Option<f64> {
        self.values
            .iter()
            .map(|(_, v)| *v)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Get the average value
    #[must_use]
    pub fn average(&self) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let sum: f64 = self.values.iter().map(|(_, v)| v).sum();
        Some(sum / self.values.len() as f64)
    }
}

impl ValueTracker<i64> {
    /// Check if value is monotonically increasing
    #[must_use]
    pub fn is_increasing(&self) -> bool {
        if self.values.len() < 2 {
            return true;
        }
        self.values.windows(2).all(|w| w[1].1 >= w[0].1)
    }

    /// Check if value is monotonically decreasing
    #[must_use]
    pub fn is_decreasing(&self) -> bool {
        if self.values.len() < 2 {
            return true;
        }
        self.values.windows(2).all(|w| w[1].1 <= w[0].1)
    }
}

/// Multi-value tracker for monitoring multiple named values
#[derive(Debug, Default)]
pub struct MultiValueTracker {
    trackers: HashMap<String, ValueTracker<f64>>,
}

impl MultiValueTracker {
    /// Create a new multi-value tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            trackers: HashMap::new(),
        }
    }

    /// Record a value for a named tracker
    pub fn record(&mut self, name: &str, timestamp_ms: u64, value: f64) {
        let tracker = self
            .trackers
            .entry(name.to_string())
            .or_insert_with(|| ValueTracker::new(name));
        tracker.record(timestamp_ms, value);
    }

    /// Get a specific tracker
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ValueTracker<f64>> {
        self.trackers.get(name)
    }

    /// Get all tracker names
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.trackers.keys().map(String::as_str).collect()
    }

    /// Check if all tracked values are within expected bounds
    pub fn assert_bounds(&self, bounds: &HashMap<String, (f64, f64)>) -> ProbarResult<()> {
        let mut errors = Vec::new();

        for (name, (min, max)) in bounds {
            if let Some(tracker) = self.trackers.get(name) {
                for (ts, value) in tracker.values() {
                    if value < min || value > max {
                        errors.push(format!(
                            "{}: value {} at {}ms is outside bounds [{}, {}]",
                            name, value, ts, min, max
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ProbarError::AssertionFailed {
                message: errors.join("\n"),
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    mod frame_assertion_tests {
        use super::*;

        #[test]
        fn test_to_contain_text_pass() {
            let frame = TuiFrame::from_lines(&["Hello World", "Goodbye"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_contain_text("World").is_ok());
        }

        #[test]
        fn test_to_contain_text_fail() {
            let frame = TuiFrame::from_lines(&["Hello World"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_contain_text("Missing").is_err());
        }

        #[test]
        fn test_not_to_contain_text_pass() {
            let frame = TuiFrame::from_lines(&["Hello World"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.not_to_contain_text("Missing").is_ok());
        }

        #[test]
        fn test_not_to_contain_text_fail() {
            let frame = TuiFrame::from_lines(&["Hello World"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.not_to_contain_text("World").is_err());
        }

        #[test]
        fn test_to_match_pass() {
            let frame = TuiFrame::from_lines(&["Score: 100"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_match(r"Score: \d+").is_ok());
        }

        #[test]
        fn test_to_match_fail() {
            let frame = TuiFrame::from_lines(&["Score: abc"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_match(r"Score: \d+").is_err());
        }

        #[test]
        fn test_line_to_contain_pass() {
            let frame = TuiFrame::from_lines(&["First", "Second", "Third"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.line_to_contain(1, "Sec").is_ok());
        }

        #[test]
        fn test_line_to_contain_fail() {
            let frame = TuiFrame::from_lines(&["First", "Second"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.line_to_contain(0, "Second").is_err());
        }

        #[test]
        fn test_line_to_contain_invalid_line() {
            let frame = TuiFrame::from_lines(&["Only one line"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.line_to_contain(5, "text").is_err());
        }

        #[test]
        fn test_line_to_equal_pass() {
            let frame = TuiFrame::from_lines(&["Exact Match"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.line_to_equal(0, "Exact Match").is_ok());
        }

        #[test]
        fn test_line_to_equal_fail() {
            let frame = TuiFrame::from_lines(&["Exact Match"]);
            let mut assertion = expect_frame(&frame);
            assert!(assertion.line_to_equal(0, "Different").is_err());
        }

        #[test]
        fn test_to_have_size_pass() {
            let frame = TuiFrame::from_lines(&["12345", "12345"]); // 5x2
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_have_size(5, 2).is_ok());
        }

        #[test]
        fn test_to_have_size_fail() {
            let frame = TuiFrame::from_lines(&["123"]); // 3x1
            let mut assertion = expect_frame(&frame);
            assert!(assertion.to_have_size(10, 10).is_err());
        }

        #[test]
        fn test_to_be_identical_to_pass() {
            let frame1 = TuiFrame::from_lines(&["Same", "Content"]);
            let frame2 = TuiFrame::from_lines(&["Same", "Content"]);
            let mut assertion = expect_frame(&frame1);
            assert!(assertion.to_be_identical_to(&frame2).is_ok());
        }

        #[test]
        fn test_to_be_identical_to_fail() {
            let frame1 = TuiFrame::from_lines(&["Different"]);
            let frame2 = TuiFrame::from_lines(&["Content"]);
            let mut assertion = expect_frame(&frame1);
            assert!(assertion.to_be_identical_to(&frame2).is_err());
        }

        #[test]
        fn test_soft_assertions_collect_errors() {
            let frame = TuiFrame::from_lines(&["Hello"]);
            let mut assertion = expect_frame(&frame).soft();

            let _ = assertion.to_contain_text("Missing1");
            let _ = assertion.to_contain_text("Missing2");

            assert_eq!(assertion.errors().len(), 2);
            assert!(assertion.finalize().is_err());
        }

        #[test]
        fn test_soft_assertions_no_errors() {
            let frame = TuiFrame::from_lines(&["Hello World"]);
            let mut assertion = expect_frame(&frame).soft();

            let _ = assertion.to_contain_text("Hello");
            let _ = assertion.to_contain_text("World");

            assert!(assertion.errors().is_empty());
            assert!(assertion.finalize().is_ok());
        }

        #[test]
        fn test_chained_assertions() {
            let frame = TuiFrame::from_lines(&["Score: 100", "Lives: 3"]);
            let mut assertion = expect_frame(&frame);

            assert!(assertion
                .to_contain_text("Score")
                .and_then(|a| a.to_contain_text("Lives"))
                .and_then(|a| a.to_match(r"\d+"))
                .is_ok());
        }
    }

    mod value_tracker_tests {
        use super::*;

        #[test]
        fn test_new() {
            let tracker: ValueTracker<f64> = ValueTracker::new("score");
            assert_eq!(tracker.name(), "score");
            assert!(tracker.is_empty());
        }

        #[test]
        fn test_record_and_latest() {
            let mut tracker: ValueTracker<i64> = ValueTracker::new("score");
            tracker.record(0, 100);
            tracker.record(100, 200);

            assert_eq!(tracker.len(), 2);
            assert_eq!(tracker.latest(), Some(&200));
        }

        #[test]
        fn test_at() {
            let mut tracker: ValueTracker<i64> = ValueTracker::new("test");
            tracker.record(0, 10);
            tracker.record(100, 20);
            tracker.record(200, 30);

            assert_eq!(tracker.at(0), Some(&10));
            assert_eq!(tracker.at(1), Some(&20));
            assert_eq!(tracker.at(2), Some(&30));
            assert_eq!(tracker.at(3), None);
        }

        #[test]
        fn test_has_changed() {
            let mut tracker: ValueTracker<i64> = ValueTracker::new("test");

            assert!(!tracker.has_changed()); // Empty

            tracker.record(0, 100);
            assert!(!tracker.has_changed()); // Only one value

            tracker.record(100, 100);
            assert!(!tracker.has_changed()); // Same value

            tracker.record(200, 200);
            assert!(tracker.has_changed()); // Different value
        }

        #[test]
        fn test_change_count() {
            let mut tracker: ValueTracker<i64> = ValueTracker::new("test");
            tracker.record(0, 1);
            tracker.record(100, 1);
            tracker.record(200, 2);
            tracker.record(300, 2);
            tracker.record(400, 3);

            assert_eq!(tracker.change_count(), 2); // 1->2 and 2->3
        }

        #[test]
        fn test_clear() {
            let mut tracker: ValueTracker<f64> = ValueTracker::new("test");
            tracker.record(0, 1.0);
            tracker.record(100, 2.0);

            tracker.clear();
            assert!(tracker.is_empty());
        }
    }

    mod value_tracker_f64_tests {
        use super::*;

        #[test]
        fn test_rate_of_change() {
            let mut tracker = ValueTracker::new("position");
            tracker.record(0, 0.0);
            tracker.record(1000, 100.0);

            let rate = tracker.rate_of_change().unwrap();
            assert!((rate - 0.1).abs() < 0.001); // 100 units / 1000 ms = 0.1 per ms
        }

        #[test]
        fn test_rate_of_change_no_time() {
            let mut tracker = ValueTracker::new("test");
            tracker.record(100, 0.0);
            tracker.record(100, 100.0); // Same timestamp

            assert!(tracker.rate_of_change().is_none());
        }

        #[test]
        fn test_is_increasing() {
            let mut tracker = ValueTracker::new("score");
            tracker.record(0, 0.0);
            tracker.record(100, 10.0);
            tracker.record(200, 20.0);

            assert!(tracker.is_increasing());
        }

        #[test]
        fn test_is_not_increasing() {
            let mut tracker = ValueTracker::new("health");
            tracker.record(0, 100.0);
            tracker.record(100, 80.0);
            tracker.record(200, 90.0);

            assert!(!tracker.is_increasing());
        }

        #[test]
        fn test_is_decreasing() {
            let mut tracker = ValueTracker::new("health");
            tracker.record(0, 100.0);
            tracker.record(100, 80.0);
            tracker.record(200, 60.0);

            assert!(tracker.is_decreasing());
        }

        #[test]
        fn test_min_max_average() {
            let mut tracker = ValueTracker::new("test");
            tracker.record(0, 10.0);
            tracker.record(100, 20.0);
            tracker.record(200, 30.0);

            assert!((tracker.min().unwrap() - 10.0).abs() < f64::EPSILON);
            assert!((tracker.max().unwrap() - 30.0).abs() < f64::EPSILON);
            assert!((tracker.average().unwrap() - 20.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_empty_stats() {
            let tracker: ValueTracker<f64> = ValueTracker::new("empty");
            assert!(tracker.min().is_none());
            assert!(tracker.max().is_none());
            assert!(tracker.average().is_none());
            assert!(tracker.rate_of_change().is_none());
        }
    }

    mod multi_value_tracker_tests {
        use super::*;

        #[test]
        fn test_record_and_get() {
            let mut multi = MultiValueTracker::new();
            multi.record("score", 0, 100.0);
            multi.record("health", 0, 100.0);

            assert!(multi.get("score").is_some());
            assert!(multi.get("health").is_some());
            assert!(multi.get("missing").is_none());
        }

        #[test]
        fn test_names() {
            let mut multi = MultiValueTracker::new();
            multi.record("a", 0, 1.0);
            multi.record("b", 0, 2.0);

            let names = multi.names();
            assert_eq!(names.len(), 2);
            assert!(names.contains(&"a"));
            assert!(names.contains(&"b"));
        }

        #[test]
        fn test_assert_bounds_pass() {
            let mut multi = MultiValueTracker::new();
            multi.record("health", 0, 100.0);
            multi.record("health", 100, 80.0);

            let mut bounds = HashMap::new();
            bounds.insert("health".to_string(), (0.0, 100.0));

            assert!(multi.assert_bounds(&bounds).is_ok());
        }

        #[test]
        fn test_assert_bounds_fail() {
            let mut multi = MultiValueTracker::new();
            multi.record("health", 0, 150.0); // Above max

            let mut bounds = HashMap::new();
            bounds.insert("health".to_string(), (0.0, 100.0));

            assert!(multi.assert_bounds(&bounds).is_err());
        }
    }
}
