//! Clock Manipulation for Deterministic Tests (Feature G.6)
//!
//! Provides fake clock implementation for controlling time in tests.
//! Enables deterministic testing of time-dependent code.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Clock state for fake time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockState {
    /// Clock is running normally
    Running,
    /// Clock is paused at a fixed time
    Paused,
    /// Clock is using system time
    System,
}

/// Options for clock installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockOptions {
    /// Initial time to set (milliseconds since Unix epoch)
    pub time_ms: u64,
    /// Whether to pause immediately
    pub paused: bool,
}

impl ClockOptions {
    /// Create options with current system time
    #[must_use]
    pub fn now() -> Self {
        let time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            time_ms,
            paused: false,
        }
    }

    /// Create options with fixed time
    #[must_use]
    pub fn fixed(time_ms: u64) -> Self {
        Self {
            time_ms,
            paused: true,
        }
    }

    /// Parse ISO 8601 datetime string
    ///
    /// # Errors
    ///
    /// Returns error if parsing fails
    pub fn from_iso(iso: &str) -> Result<Self, ClockError> {
        // Simple ISO 8601 parser for common formats
        // Format: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS
        let time_ms = parse_iso_to_ms(iso)?;
        Ok(Self {
            time_ms,
            paused: false,
        })
    }

    /// Set whether to start paused
    #[must_use]
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }
}

impl Default for ClockOptions {
    fn default() -> Self {
        Self::now()
    }
}

/// Errors that can occur with clock operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockError {
    /// Invalid datetime format
    InvalidFormat(String),
    /// Clock not installed
    NotInstalled,
    /// Clock already installed
    AlreadyInstalled,
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(s) => write!(f, "Invalid datetime format: {s}"),
            Self::NotInstalled => write!(f, "Clock not installed"),
            Self::AlreadyInstalled => write!(f, "Clock already installed"),
        }
    }
}

impl std::error::Error for ClockError {}

/// Fake clock for deterministic testing
#[derive(Debug)]
pub struct FakeClock {
    /// Current time in milliseconds since Unix epoch
    current_ms: AtomicU64,
    /// Whether clock is paused
    paused: AtomicBool,
    /// Whether clock is installed
    installed: AtomicBool,
    /// Real time when clock was installed (for relative calculations)
    install_real_ms: AtomicU64,
    /// Fake time when clock was installed
    install_fake_ms: AtomicU64,
}

impl FakeClock {
    /// Create a new fake clock (not installed)
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_ms: AtomicU64::new(0),
            paused: AtomicBool::new(false),
            installed: AtomicBool::new(false),
            install_real_ms: AtomicU64::new(0),
            install_fake_ms: AtomicU64::new(0),
        }
    }

    /// Install the fake clock with options
    ///
    /// # Errors
    ///
    /// Returns error if clock is already installed
    pub fn install(&self, options: ClockOptions) -> Result<(), ClockError> {
        if self.installed.swap(true, Ordering::SeqCst) {
            return Err(ClockError::AlreadyInstalled);
        }

        let real_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.install_real_ms.store(real_ms, Ordering::SeqCst);
        self.install_fake_ms.store(options.time_ms, Ordering::SeqCst);
        self.current_ms.store(options.time_ms, Ordering::SeqCst);
        self.paused.store(options.paused, Ordering::SeqCst);

        Ok(())
    }

    /// Uninstall the fake clock
    pub fn uninstall(&self) {
        self.installed.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
    }

    /// Check if clock is installed
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.installed.load(Ordering::SeqCst)
    }

    /// Check if clock is paused
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Get current fake time in milliseconds
    #[must_use]
    pub fn now_ms(&self) -> u64 {
        if !self.is_installed() {
            return SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
        }

        if self.is_paused() {
            return self.current_ms.load(Ordering::SeqCst);
        }

        // Calculate elapsed time since install
        let real_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let real_elapsed = real_now.saturating_sub(self.install_real_ms.load(Ordering::SeqCst));
        let current = self.current_ms.load(Ordering::SeqCst);

        current + real_elapsed
    }

    /// Get current fake time as Duration since Unix epoch
    #[must_use]
    pub fn now(&self) -> Duration {
        Duration::from_millis(self.now_ms())
    }

    /// Pause the clock at current time
    pub fn pause(&self) {
        if self.is_installed() && !self.is_paused() {
            // Capture current time before pausing
            let current = self.now_ms();
            self.current_ms.store(current, Ordering::SeqCst);
            self.paused.store(true, Ordering::SeqCst);
        }
    }

    /// Resume the clock from paused state
    pub fn resume(&self) {
        if self.is_installed() && self.is_paused() {
            // Update install time to now
            let real_now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            self.install_real_ms.store(real_now, Ordering::SeqCst);
            self.paused.store(false, Ordering::SeqCst);
        }
    }

    /// Set clock to a fixed time (pauses clock)
    pub fn set_fixed_time(&self, time_ms: u64) {
        self.current_ms.store(time_ms, Ordering::SeqCst);
        self.paused.store(true, Ordering::SeqCst);
    }

    /// Set clock to a fixed time from ISO string
    ///
    /// # Errors
    ///
    /// Returns error if parsing fails
    pub fn set_fixed_time_iso(&self, iso: &str) -> Result<(), ClockError> {
        let time_ms = parse_iso_to_ms(iso)?;
        self.set_fixed_time(time_ms);
        Ok(())
    }

    /// Fast-forward time by duration
    pub fn fast_forward(&self, duration: Duration) {
        let current = self.current_ms.load(Ordering::SeqCst);
        let new_time = current + duration.as_millis() as u64;
        self.current_ms.store(new_time, Ordering::SeqCst);
    }

    /// Fast-forward time by milliseconds
    pub fn fast_forward_ms(&self, ms: u64) {
        self.fast_forward(Duration::from_millis(ms));
    }

    /// Pause at specific time
    pub fn pause_at(&self, time_ms: u64) {
        self.current_ms.store(time_ms, Ordering::SeqCst);
        self.paused.store(true, Ordering::SeqCst);
    }

    /// Get current state
    #[must_use]
    pub fn state(&self) -> ClockState {
        if !self.is_installed() {
            ClockState::System
        } else if self.is_paused() {
            ClockState::Paused
        } else {
            ClockState::Running
        }
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for FakeClock {
    fn clone(&self) -> Self {
        Self {
            current_ms: AtomicU64::new(self.current_ms.load(Ordering::SeqCst)),
            paused: AtomicBool::new(self.paused.load(Ordering::SeqCst)),
            installed: AtomicBool::new(self.installed.load(Ordering::SeqCst)),
            install_real_ms: AtomicU64::new(self.install_real_ms.load(Ordering::SeqCst)),
            install_fake_ms: AtomicU64::new(self.install_fake_ms.load(Ordering::SeqCst)),
        }
    }
}

/// Thread-safe clock handle
pub type Clock = Arc<FakeClock>;

/// Create a new shared clock
#[must_use]
pub fn create_clock() -> Clock {
    Arc::new(FakeClock::new())
}

/// Parse simple ISO 8601 datetime to milliseconds
fn parse_iso_to_ms(iso: &str) -> Result<u64, ClockError> {
    // Support formats:
    // - YYYY-MM-DDTHH:MM:SSZ
    // - YYYY-MM-DDTHH:MM:SS
    // - YYYY-MM-DD

    let iso = iso.trim().trim_end_matches('Z');

    let parts: Vec<&str> = if iso.contains('T') {
        iso.split('T').collect()
    } else {
        vec![iso, "00:00:00"]
    };

    if parts.is_empty() {
        return Err(ClockError::InvalidFormat(iso.to_string()));
    }

    let date_parts: Vec<u32> = parts[0]
        .split('-')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    if date_parts.len() < 3 {
        return Err(ClockError::InvalidFormat(iso.to_string()));
    }

    let year = date_parts[0];
    let month = date_parts[1];
    let day = date_parts[2];

    let (hour, minute, second) = if parts.len() > 1 {
        let time_parts: Vec<u32> = parts[1]
            .split(':')
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        (
            *time_parts.first().unwrap_or(&0),
            *time_parts.get(1).unwrap_or(&0),
            *time_parts.get(2).unwrap_or(&0),
        )
    } else {
        (0, 0, 0)
    };

    // Simple days since epoch calculation (not accounting for leap seconds)
    let days_since_epoch = days_since_unix_epoch(year, month, day);
    let seconds = days_since_epoch * 86400 + u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);

    Ok(seconds * 1000)
}

/// Calculate days since Unix epoch (1970-01-01)
fn days_since_unix_epoch(year: u32, month: u32, day: u32) -> u64 {
    let mut days: i64 = 0;

    // Years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Months
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += i64::from(month_days[(m - 1) as usize]);
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }

    // Days
    days += i64::from(day - 1);

    days.max(0) as u64
}

/// Check if year is a leap year
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Clock controller for page/context
#[derive(Debug, Clone)]
pub struct ClockController {
    clock: Clock,
}

impl ClockController {
    /// Create a new clock controller
    #[must_use]
    pub fn new() -> Self {
        Self {
            clock: create_clock(),
        }
    }

    /// Create with existing clock
    #[must_use]
    pub fn with_clock(clock: Clock) -> Self {
        Self { clock }
    }

    /// Install fake clock
    ///
    /// # Errors
    ///
    /// Returns error if already installed
    pub fn install(&self, options: ClockOptions) -> Result<(), ClockError> {
        self.clock.install(options)
    }

    /// Uninstall fake clock
    pub fn uninstall(&self) {
        self.clock.uninstall();
    }

    /// Fast-forward time
    pub fn fast_forward(&self, duration: Duration) {
        self.clock.fast_forward(duration);
    }

    /// Set fixed time
    pub fn set_fixed_time(&self, time_ms: u64) {
        self.clock.set_fixed_time(time_ms);
    }

    /// Set fixed time from ISO string
    ///
    /// # Errors
    ///
    /// Returns error if parsing fails
    pub fn set_fixed_time_iso(&self, iso: &str) -> Result<(), ClockError> {
        self.clock.set_fixed_time_iso(iso)
    }

    /// Pause at specific time
    pub fn pause_at(&self, time_ms: u64) {
        self.clock.pause_at(time_ms);
    }

    /// Pause clock
    pub fn pause(&self) {
        self.clock.pause();
    }

    /// Resume clock
    pub fn resume(&self) {
        self.clock.resume();
    }

    /// Get current time in milliseconds
    #[must_use]
    pub fn now_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    /// Get current state
    #[must_use]
    pub fn state(&self) -> ClockState {
        self.clock.state()
    }

    /// Get inner clock
    #[must_use]
    pub fn inner(&self) -> &Clock {
        &self.clock
    }
}

impl Default for ClockController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // =========================================================================
    // H₀-CLOCK-01: FakeClock creation
    // =========================================================================

    #[test]
    fn h0_clock_01_new() {
        let clock = FakeClock::new();
        assert!(!clock.is_installed());
        assert!(!clock.is_paused());
    }

    #[test]
    fn h0_clock_02_state_system_when_not_installed() {
        let clock = FakeClock::new();
        assert_eq!(clock.state(), ClockState::System);
    }

    // =========================================================================
    // H₀-CLOCK-03: Clock installation
    // =========================================================================

    #[test]
    fn h0_clock_03_install_success() {
        let clock = FakeClock::new();
        let options = ClockOptions::fixed(1_000_000);
        clock.install(options).unwrap();

        assert!(clock.is_installed());
        assert!(clock.is_paused());
    }

    #[test]
    fn h0_clock_04_install_already_installed() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::now()).unwrap();

        let result = clock.install(ClockOptions::now());
        assert!(matches!(result, Err(ClockError::AlreadyInstalled)));
    }

    #[test]
    fn h0_clock_05_uninstall() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::now()).unwrap();
        clock.uninstall();

        assert!(!clock.is_installed());
    }

    // =========================================================================
    // H₀-CLOCK-06: Time retrieval
    // =========================================================================

    #[test]
    fn h0_clock_06_now_ms_when_paused() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(1_705_312_800_000)).unwrap(); // 2024-01-15T10:00:00Z

        let time = clock.now_ms();
        assert_eq!(time, 1_705_312_800_000);
    }

    #[test]
    fn h0_clock_07_now_returns_duration() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(1000)).unwrap();

        let duration = clock.now();
        assert_eq!(duration.as_millis(), 1000);
    }

    // =========================================================================
    // H₀-CLOCK-08: Fast forward
    // =========================================================================

    #[test]
    fn h0_clock_08_fast_forward() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(1000)).unwrap();

        clock.fast_forward(Duration::from_secs(60));

        assert_eq!(clock.now_ms(), 61_000);
    }

    #[test]
    fn h0_clock_09_fast_forward_ms() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(0)).unwrap();

        clock.fast_forward_ms(5000);

        assert_eq!(clock.now_ms(), 5000);
    }

    // =========================================================================
    // H₀-CLOCK-10: Pause and resume
    // =========================================================================

    #[test]
    fn h0_clock_10_pause() {
        let clock = FakeClock::new();
        clock
            .install(ClockOptions {
                time_ms: 1000,
                paused: false,
            })
            .unwrap();

        clock.pause();

        assert!(clock.is_paused());
        assert_eq!(clock.state(), ClockState::Paused);
    }

    #[test]
    fn h0_clock_11_resume() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(1000)).unwrap();

        clock.resume();

        assert!(!clock.is_paused());
        assert_eq!(clock.state(), ClockState::Running);
    }

    #[test]
    fn h0_clock_12_pause_at() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::now()).unwrap();

        clock.pause_at(5000);

        assert!(clock.is_paused());
        assert_eq!(clock.now_ms(), 5000);
    }

    // =========================================================================
    // H₀-CLOCK-13: Set fixed time
    // =========================================================================

    #[test]
    fn h0_clock_13_set_fixed_time() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::now()).unwrap();

        clock.set_fixed_time(9999);

        assert!(clock.is_paused());
        assert_eq!(clock.now_ms(), 9999);
    }

    #[test]
    fn h0_clock_14_set_fixed_time_iso() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::now()).unwrap();

        clock.set_fixed_time_iso("2024-01-15T12:00:00Z").unwrap();

        assert!(clock.is_paused());
        // Should be around Jan 15, 2024 12:00:00 UTC
        assert!(clock.now_ms() > 1_705_000_000_000);
    }

    // =========================================================================
    // H₀-CLOCK-15: ClockOptions
    // =========================================================================

    #[test]
    fn h0_clock_15_options_now() {
        let options = ClockOptions::now();
        assert!(!options.paused);
        assert!(options.time_ms > 0);
    }

    #[test]
    fn h0_clock_16_options_fixed() {
        let options = ClockOptions::fixed(1234);
        assert!(options.paused);
        assert_eq!(options.time_ms, 1234);
    }

    #[test]
    fn h0_clock_17_options_from_iso() {
        let options = ClockOptions::from_iso("2024-01-01T00:00:00Z").unwrap();
        assert!(options.time_ms > 1_704_000_000_000);
    }

    #[test]
    fn h0_clock_18_options_paused_builder() {
        let options = ClockOptions::now().paused(true);
        assert!(options.paused);
    }

    // =========================================================================
    // H₀-CLOCK-19: ISO parsing
    // =========================================================================

    #[test]
    fn h0_clock_19_parse_iso_full() {
        let ms = parse_iso_to_ms("2024-01-15T10:30:00Z").unwrap();
        // Should be roughly Jan 15, 2024 10:30:00 UTC
        assert!(ms > 1_705_000_000_000);
    }

    #[test]
    fn h0_clock_20_parse_iso_date_only() {
        let ms = parse_iso_to_ms("2024-01-15").unwrap();
        assert!(ms > 1_705_000_000_000);
    }

    #[test]
    fn h0_clock_21_parse_iso_invalid() {
        let result = parse_iso_to_ms("invalid");
        assert!(result.is_err());
    }

    // =========================================================================
    // H₀-CLOCK-22: ClockController
    // =========================================================================

    #[test]
    fn h0_clock_22_controller_new() {
        let controller = ClockController::new();
        assert_eq!(controller.state(), ClockState::System);
    }

    #[test]
    fn h0_clock_23_controller_install() {
        let controller = ClockController::new();
        controller.install(ClockOptions::fixed(1000)).unwrap();

        assert_eq!(controller.state(), ClockState::Paused);
        assert_eq!(controller.now_ms(), 1000);
    }

    #[test]
    fn h0_clock_24_controller_fast_forward() {
        let controller = ClockController::new();
        controller.install(ClockOptions::fixed(0)).unwrap();

        controller.fast_forward(Duration::from_secs(30));

        assert_eq!(controller.now_ms(), 30_000);
    }

    #[test]
    fn h0_clock_25_controller_pause_resume() {
        let controller = ClockController::new();
        controller
            .install(ClockOptions {
                time_ms: 1000,
                paused: false,
            })
            .unwrap();

        controller.pause();
        assert_eq!(controller.state(), ClockState::Paused);

        controller.resume();
        assert_eq!(controller.state(), ClockState::Running);
    }

    // =========================================================================
    // H₀-CLOCK-26: Clone
    // =========================================================================

    #[test]
    fn h0_clock_26_clone() {
        let clock = FakeClock::new();
        clock.install(ClockOptions::fixed(5000)).unwrap();

        let cloned = clock;

        assert!(cloned.is_installed());
        assert_eq!(cloned.now_ms(), 5000);
    }

    // =========================================================================
    // H₀-CLOCK-27: Leap year
    // =========================================================================

    #[test]
    fn h0_clock_27_is_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900));
    }

    // =========================================================================
    // H₀-CLOCK-28: Error display
    // =========================================================================

    #[test]
    fn h0_clock_28_error_display() {
        let err = ClockError::InvalidFormat("bad".to_string());
        assert!(err.to_string().contains("Invalid datetime"));

        let err = ClockError::NotInstalled;
        assert!(err.to_string().contains("not installed"));

        let err = ClockError::AlreadyInstalled;
        assert!(err.to_string().contains("already installed"));
    }

    // =========================================================================
    // H₀-CLOCK-29: Shared clock
    // =========================================================================

    #[test]
    fn h0_clock_29_create_clock() {
        let clock = create_clock();
        assert!(!clock.is_installed());
    }

    #[test]
    fn h0_clock_30_controller_with_clock() {
        let clock = create_clock();
        clock.install(ClockOptions::fixed(1234)).unwrap();

        let controller = ClockController::with_clock(clock);
        assert_eq!(controller.now_ms(), 1234);
    }
}
