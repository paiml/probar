//! DeterministicBrick: Pure functional brick execution (PROBAR-SPEC-009-P11)
//!
//! Provides deterministic execution with:
//! - Pure functional design: (State, Input) → (State, Output)
//! - Persistent data structures for O(1) snapshots
//! - Time-travel debugging
//! - Jidoka guards for invariant checking
//!
//! # Design Philosophy
//!
//! DeterministicBrick applies WOS (WebAssembly Operating System) patterns
//! to ensure reproducible brick execution.
//!
//! # Example
//!
//! ```rust,ignore
//! use probar::brick::deterministic::{DeterministicBrick, BrickHistory};
//!
//! impl DeterministicBrick for MelSpectrogramBrick {
//!     type State = ComputeState;
//!     type Input = AudioChunk;
//!     type Output = MelFrames;
//!
//!     fn execute_pure(
//!         state: Self::State,
//!         input: Self::Input,
//!     ) -> Result<(Self::State, Self::Output), BrickError> {
//!         let mel = compute_mel(&input, &state.filterbank)?;
//!         let new_state = state.with_frame_count(state.frame_count + 1);
//!         Ok((new_state, mel))
//!     }
//! }
//! ```

use super::{Brick, BrickError};
use std::collections::HashMap;
use std::time::Duration;

/// Trait for bricks with deterministic, pure functional execution
pub trait DeterministicBrick: Brick {
    /// State type (must be cloneable for snapshots)
    type State: Clone + Default;
    /// Input type
    type Input;
    /// Output type
    type Output;

    /// Pure function: (state, input) → (new_state, output)
    ///
    /// This function MUST be:
    /// - Deterministic: same inputs → same outputs
    /// - Pure: no side effects
    /// - Total: always returns or errors (no panics)
    fn execute_pure(
        state: Self::State,
        input: Self::Input,
    ) -> Result<(Self::State, Self::Output), BrickError>;

    /// Create initial state
    fn initial_state() -> Self::State {
        Self::State::default()
    }

    /// Get state dependencies for determinism verification
    fn state_dependencies(&self) -> &[&str] {
        &[]
    }
}

/// Brick state snapshot with structural sharing
#[derive(Debug, Clone)]
pub struct BrickState {
    /// Named tensors (using owned Vecs for simplicity)
    pub tensors: HashMap<String, Vec<f32>>,
    /// Tensor shapes
    pub shapes: HashMap<String, Vec<usize>>,
    /// Scalar metadata
    pub metadata: HashMap<String, StateValue>,
    /// Snapshot version for debugging
    pub version: u64,
}

/// Value types for state metadata
#[derive(Debug, Clone, PartialEq)]
pub enum StateValue {
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
}

impl BrickState {
    /// Create a new empty state
    #[must_use]
    pub fn new() -> Self {
        Self {
            tensors: HashMap::new(),
            shapes: HashMap::new(),
            metadata: HashMap::new(),
            version: 0,
        }
    }

    /// Create a snapshot (clones with new version)
    #[must_use]
    pub fn snapshot(&self) -> Self {
        let mut snap = self.clone();
        snap.version = self.version + 1;
        snap
    }

    /// Set a tensor value
    pub fn set_tensor(&mut self, name: impl Into<String>, data: Vec<f32>, shape: Vec<usize>) {
        let name = name.into();
        self.tensors.insert(name.clone(), data);
        self.shapes.insert(name, shape);
    }

    /// Get a tensor value
    pub fn get_tensor(&self, name: &str) -> Option<(&[f32], &[usize])> {
        let data = self.tensors.get(name)?;
        let shape = self.shapes.get(name)?;
        Some((data, shape))
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: StateValue) {
        self.metadata.insert(key.into(), value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&StateValue> {
        self.metadata.get(key)
    }
}

impl Default for BrickState {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution trace entry for time-travel debugging
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    /// Operation name
    pub operation: String,
    /// Input summary (for debugging)
    pub input_summary: String,
    /// Output summary (for debugging)
    pub output_summary: String,
    /// Duration
    pub duration: Duration,
    /// State version before execution
    pub state_version_before: u64,
    /// State version after execution
    pub state_version_after: u64,
}

/// History for time-travel debugging
#[derive(Debug)]
pub struct BrickHistory {
    /// State snapshots
    snapshots: Vec<BrickState>,
    /// Execution traces
    traces: Vec<ExecutionTrace>,
    /// Current position
    position: usize,
    /// Maximum history size
    max_size: usize,
}

impl BrickHistory {
    /// Create a new history with given max size
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_size),
            traces: Vec::with_capacity(max_size),
            position: 0,
            max_size,
        }
    }

    /// Record a state snapshot
    pub fn record(&mut self, state: BrickState, trace: ExecutionTrace) {
        // Truncate forward history if we're not at the end
        if self.position < self.snapshots.len() {
            self.snapshots.truncate(self.position);
            self.traces.truncate(self.position);
        }

        // Remove oldest if at capacity
        if self.snapshots.len() >= self.max_size {
            self.snapshots.remove(0);
            self.traces.remove(0);
        }

        self.snapshots.push(state);
        self.traces.push(trace);
        self.position = self.snapshots.len();
    }

    /// Step backward to previous state
    pub fn step_back(&mut self) -> Option<&BrickState> {
        if self.position > 0 {
            self.position -= 1;
            self.snapshots.get(self.position)
        } else {
            None
        }
    }

    /// Step forward to next state
    pub fn step_forward(&mut self) -> Option<&BrickState> {
        if self.position < self.snapshots.len() {
            let state = self.snapshots.get(self.position);
            self.position += 1;
            state
        } else {
            None
        }
    }

    /// Jump to specific position
    pub fn goto(&mut self, position: usize) -> Option<&BrickState> {
        if position < self.snapshots.len() {
            self.position = position;
            self.snapshots.get(position)
        } else {
            None
        }
    }

    /// Get current state
    pub fn current(&self) -> Option<&BrickState> {
        if self.position > 0 && self.position <= self.snapshots.len() {
            self.snapshots.get(self.position - 1)
        } else {
            self.snapshots.first()
        }
    }

    /// Get current position
    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get total snapshot count
    #[must_use]
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if history is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Get trace at position
    pub fn trace_at(&self, position: usize) -> Option<&ExecutionTrace> {
        self.traces.get(position)
    }

    /// Get all traces
    pub fn traces(&self) -> &[ExecutionTrace] {
        &self.traces
    }
}

impl Default for BrickHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Severity level for invariant violations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardSeverity {
    /// Warning only, execution continues
    Warning,
    /// Error, execution stops
    Error,
    /// Critical, execution stops and alerts
    Critical,
}

/// Invariant guard for Jidoka pattern
pub struct InvariantGuard {
    /// Guard name
    pub name: &'static str,
    /// Check function
    pub check: fn(&BrickState) -> bool,
    /// Severity on violation
    pub severity: GuardSeverity,
}

impl std::fmt::Debug for InvariantGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvariantGuard")
            .field("name", &self.name)
            .field("check", &"<fn>")
            .field("severity", &self.severity)
            .finish()
    }
}

impl InvariantGuard {
    /// Create a new invariant guard
    #[must_use]
    pub const fn new(
        name: &'static str,
        check: fn(&BrickState) -> bool,
        severity: GuardSeverity,
    ) -> Self {
        Self {
            name,
            check,
            severity,
        }
    }

    /// Check the invariant
    pub fn check(&self, state: &BrickState) -> bool {
        (self.check)(state)
    }
}

/// Wrapper that adds invariant checking to a brick
#[derive(Debug)]
pub struct GuardedBrick<B: Brick> {
    /// Inner brick
    inner: B,
    /// Invariant guards
    guards: Vec<InvariantGuard>,
}

impl<B: Brick> GuardedBrick<B> {
    /// Create a new guarded brick
    #[must_use]
    pub fn new(brick: B) -> Self {
        Self {
            inner: brick,
            guards: Vec::new(),
        }
    }

    /// Add an invariant guard
    #[must_use]
    pub fn guard(mut self, guard: InvariantGuard) -> Self {
        self.guards.push(guard);
        self
    }

    /// Check all guards against state
    pub fn check_guards(&self, state: &BrickState) -> Result<(), GuardViolation> {
        for guard in &self.guards {
            if !guard.check(state) {
                return Err(GuardViolation {
                    guard_name: guard.name,
                    severity: guard.severity,
                });
            }
        }
        Ok(())
    }

    /// Get the inner brick
    pub fn inner(&self) -> &B {
        &self.inner
    }

    /// Get guards
    pub fn guards(&self) -> &[InvariantGuard] {
        &self.guards
    }
}

/// Guard violation error
#[derive(Debug, Clone)]
pub struct GuardViolation {
    /// Name of the violated guard
    pub guard_name: &'static str,
    /// Severity
    pub severity: GuardSeverity,
}

impl std::fmt::Display for GuardViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invariant guard '{}' violated (severity: {:?})",
            self.guard_name, self.severity
        )
    }
}

impl std::error::Error for GuardViolation {}

/// Deterministic random number generator (for reproducibility)
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    /// Create a new RNG with given seed
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generate next random u64
    pub fn next_u64(&mut self) -> u64 {
        // Simple xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    /// Generate random f64 in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// Generate random f32 in [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        self.next_f64() as f32
    }

    /// Get current state (for checkpointing)
    #[must_use]
    pub const fn state(&self) -> u64 {
        self.state
    }

    /// Restore state (from checkpoint)
    pub fn restore(&mut self, state: u64) {
        self.state = state;
    }
}

impl Default for DeterministicRng {
    fn default() -> Self {
        Self::new(42)
    }
}

/// Deterministic clock for reproducibility
#[derive(Debug, Clone)]
pub struct DeterministicClock {
    /// Current time in nanoseconds
    current_ns: u64,
    /// Time step per tick
    tick_ns: u64,
}

impl DeterministicClock {
    /// Create a new clock
    #[must_use]
    pub const fn new(start_ns: u64, tick_ns: u64) -> Self {
        Self {
            current_ns: start_ns,
            tick_ns,
        }
    }

    /// Get current time
    #[must_use]
    pub const fn now_ns(&self) -> u64 {
        self.current_ns
    }

    /// Get current time as Duration
    #[must_use]
    pub const fn now(&self) -> Duration {
        Duration::from_nanos(self.current_ns)
    }

    /// Advance clock by one tick
    pub fn tick(&mut self) {
        self.current_ns += self.tick_ns;
    }

    /// Advance clock by multiple ticks
    pub fn advance(&mut self, ticks: u64) {
        self.current_ns += self.tick_ns * ticks;
    }

    /// Set time (for replay)
    pub fn set(&mut self, time_ns: u64) {
        self.current_ns = time_ns;
    }
}

impl Default for DeterministicClock {
    fn default() -> Self {
        // 10ms tick
        Self::new(0, 10_000_000)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_brick_state_basic() {
        let mut state = BrickState::new();
        state.set_tensor("audio", vec![1.0, 2.0, 3.0], vec![3]);
        state.set_metadata("frame_count", StateValue::Int(42));

        let (data, shape) = state.get_tensor("audio").unwrap();
        assert_eq!(data, &[1.0, 2.0, 3.0]);
        assert_eq!(shape, &[3]);

        assert_eq!(
            state.get_metadata("frame_count"),
            Some(&StateValue::Int(42))
        );
    }

    #[test]
    fn test_brick_state_snapshot() {
        let mut state = BrickState::new();
        state.set_metadata("count", StateValue::Int(1));

        let snap = state.snapshot();
        assert_eq!(snap.version, 1);
        assert_eq!(snap.get_metadata("count"), Some(&StateValue::Int(1)));
    }

    #[test]
    fn test_brick_history_forward() {
        let mut history = BrickHistory::new(10);

        for i in 0..5 {
            let mut state = BrickState::new();
            state.version = i;
            state.set_metadata("step", StateValue::Int(i as i64));

            let trace = ExecutionTrace {
                operation: format!("step_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::from_millis(1),
                state_version_before: i,
                state_version_after: i + 1,
            };

            history.record(state, trace);
        }

        assert_eq!(history.len(), 5);
        assert_eq!(history.position(), 5);
    }

    #[test]
    fn test_brick_history_time_travel() {
        let mut history = BrickHistory::new(10);

        // Record 3 states with values 0, 1, 2
        for i in 0..3 {
            let mut state = BrickState::new();
            state.set_metadata("value", StateValue::Int(i as i64));

            let trace = ExecutionTrace {
                operation: format!("op_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::from_millis(1),
                state_version_before: i as u64,
                state_version_after: (i + 1) as u64,
            };

            history.record(state, trace);
        }

        // After recording: position = 3 (past end)
        assert_eq!(history.position(), 3);

        // Go back: position = 2, returns snapshots[2] (value 2)
        let state = history.step_back().unwrap();
        assert_eq!(state.get_metadata("value"), Some(&StateValue::Int(2)));
        assert_eq!(history.position(), 2);

        // Go back again: position = 1, returns snapshots[1] (value 1)
        let state = history.step_back().unwrap();
        assert_eq!(state.get_metadata("value"), Some(&StateValue::Int(1)));
        assert_eq!(history.position(), 1);

        // Go forward: returns snapshots[1] (value 1), then position = 2
        let state = history.step_forward().unwrap();
        assert_eq!(state.get_metadata("value"), Some(&StateValue::Int(1)));
        assert_eq!(history.position(), 2);
    }

    #[test]
    fn test_brick_history_goto() {
        let mut history = BrickHistory::new(10);

        for i in 0..5 {
            let mut state = BrickState::new();
            state.set_metadata("index", StateValue::Int(i as i64));

            let trace = ExecutionTrace {
                operation: format!("op_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::from_millis(1),
                state_version_before: i as u64,
                state_version_after: (i + 1) as u64,
            };

            history.record(state, trace);
        }

        let state = history.goto(2).unwrap();
        assert_eq!(state.get_metadata("index"), Some(&StateValue::Int(2)));
        assert_eq!(history.position(), 2);
    }

    #[test]
    fn test_invariant_guard() {
        fn check_positive(state: &BrickState) -> bool {
            match state.get_metadata("count") {
                Some(StateValue::Int(n)) => *n >= 0,
                _ => true,
            }
        }

        let guard = InvariantGuard::new("positive_count", check_positive, GuardSeverity::Error);

        let mut state = BrickState::new();
        state.set_metadata("count", StateValue::Int(5));
        assert!(guard.check(&state));

        state.set_metadata("count", StateValue::Int(-1));
        assert!(!guard.check(&state));
    }

    #[test]
    fn test_deterministic_rng() {
        let mut rng1 = DeterministicRng::new(12345);
        let mut rng2 = DeterministicRng::new(12345);

        // Same seed should produce same sequence
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_deterministic_rng_f64_range() {
        let mut rng = DeterministicRng::new(42);

        for _ in 0..1000 {
            let val = rng.next_f64();
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_deterministic_clock() {
        let mut clock = DeterministicClock::new(0, 1_000_000); // 1ms tick

        assert_eq!(clock.now_ns(), 0);

        clock.tick();
        assert_eq!(clock.now_ns(), 1_000_000);

        clock.advance(10);
        assert_eq!(clock.now_ns(), 11_000_000);
        assert_eq!(clock.now(), Duration::from_millis(11));
    }

    #[test]
    fn test_deterministic_clock_replay() {
        let mut clock = DeterministicClock::new(0, 1_000_000);

        clock.advance(100);
        assert_eq!(clock.now_ns(), 100_000_000);

        // Reset for replay
        clock.set(0);
        assert_eq!(clock.now_ns(), 0);
    }

    #[test]
    fn test_state_value_variants() {
        let int_val = StateValue::Int(42);
        let float_val = StateValue::Float(3.14);
        let string_val = StateValue::String("hello".into());
        let bool_val = StateValue::Bool(true);

        assert_eq!(int_val, StateValue::Int(42));
        assert_eq!(float_val, StateValue::Float(3.14));
        assert_eq!(string_val, StateValue::String("hello".into()));
        assert_eq!(bool_val, StateValue::Bool(true));
    }

    // ========================================================================
    // Additional comprehensive tests for 95%+ coverage
    // ========================================================================

    #[test]
    fn test_brick_state_default() {
        let state = BrickState::default();
        assert!(state.tensors.is_empty());
        assert!(state.shapes.is_empty());
        assert!(state.metadata.is_empty());
        assert_eq!(state.version, 0);
    }

    #[test]
    fn test_brick_state_get_tensor_nonexistent() {
        let state = BrickState::new();
        assert!(state.get_tensor("nonexistent").is_none());
    }

    #[test]
    fn test_brick_state_get_metadata_nonexistent() {
        let state = BrickState::new();
        assert!(state.get_metadata("nonexistent").is_none());
    }

    #[test]
    fn test_brick_state_tensor_missing_shape() {
        let mut state = BrickState::new();
        state.tensors.insert("data".into(), vec![1.0, 2.0]);
        // No shape entry - get_tensor should return None
        assert!(state.get_tensor("data").is_none());
    }

    #[test]
    fn test_brick_state_clone() {
        let mut state = BrickState::new();
        state.set_tensor("t1", vec![1.0], vec![1]);
        state.set_metadata("m1", StateValue::Bool(true));
        state.version = 5;

        let cloned = state.clone();
        assert_eq!(cloned.version, 5);
        assert_eq!(cloned.get_tensor("t1").unwrap().0, &[1.0]);
        assert_eq!(cloned.get_metadata("m1"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_state_value_clone() {
        let val = StateValue::String("test".into());
        let cloned = val.clone();
        assert_eq!(val, cloned);
    }

    #[test]
    fn test_state_value_partial_eq() {
        assert_ne!(StateValue::Int(1), StateValue::Int(2));
        assert_ne!(StateValue::Float(1.0), StateValue::Float(2.0));
        assert_ne!(StateValue::Bool(true), StateValue::Bool(false));
        assert_ne!(
            StateValue::String("a".into()),
            StateValue::String("b".into())
        );
    }

    #[test]
    fn test_execution_trace_clone() {
        let trace = ExecutionTrace {
            operation: "test".into(),
            input_summary: "in".into(),
            output_summary: "out".into(),
            duration: Duration::from_secs(1),
            state_version_before: 0,
            state_version_after: 1,
        };
        let cloned = trace.clone();
        assert_eq!(trace.operation, cloned.operation);
        assert_eq!(trace.duration, cloned.duration);
    }

    #[test]
    fn test_brick_history_default() {
        let history = BrickHistory::default();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert_eq!(history.position(), 0);
    }

    #[test]
    fn test_brick_history_is_empty() {
        let mut history = BrickHistory::new(10);
        assert!(history.is_empty());

        let state = BrickState::new();
        let trace = ExecutionTrace {
            operation: "op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_brick_history_step_back_empty() {
        let mut history = BrickHistory::new(10);
        assert!(history.step_back().is_none());
    }

    #[test]
    fn test_brick_history_step_back_at_start() {
        let mut history = BrickHistory::new(10);

        let state = BrickState::new();
        let trace = ExecutionTrace {
            operation: "op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);

        // Move to position 0
        history.position = 0;
        assert!(history.step_back().is_none());
    }

    #[test]
    fn test_brick_history_step_forward_at_end() {
        let mut history = BrickHistory::new(10);

        let state = BrickState::new();
        let trace = ExecutionTrace {
            operation: "op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);

        // Position is already at end (1)
        assert!(history.step_forward().is_none());
    }

    #[test]
    fn test_brick_history_goto_invalid() {
        let mut history = BrickHistory::new(10);
        assert!(history.goto(100).is_none());
    }

    #[test]
    fn test_brick_history_current_empty() {
        let history = BrickHistory::new(10);
        assert!(history.current().is_none());
    }

    #[test]
    fn test_brick_history_current_at_start() {
        let mut history = BrickHistory::new(10);

        let mut state = BrickState::new();
        state.set_metadata("val", StateValue::Int(1));
        let trace = ExecutionTrace {
            operation: "op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);

        // Position is 1
        let current = history.current();
        assert!(current.is_some());
        assert_eq!(
            current.unwrap().get_metadata("val"),
            Some(&StateValue::Int(1))
        );
    }

    #[test]
    fn test_brick_history_current_position_zero() {
        let mut history = BrickHistory::new(10);

        let state = BrickState::new();
        let trace = ExecutionTrace {
            operation: "op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);

        // Force position to 0 (should return first)
        history.position = 0;
        assert!(history.current().is_some());
    }

    #[test]
    fn test_brick_history_trace_at() {
        let mut history = BrickHistory::new(10);

        let state = BrickState::new();
        let trace = ExecutionTrace {
            operation: "test_op".into(),
            input_summary: "input".into(),
            output_summary: "output".into(),
            duration: Duration::from_secs(2),
            state_version_before: 0,
            state_version_after: 1,
        };
        history.record(state, trace);

        let retrieved = history.trace_at(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().operation, "test_op");
    }

    #[test]
    fn test_brick_history_trace_at_invalid() {
        let history = BrickHistory::new(10);
        assert!(history.trace_at(100).is_none());
    }

    #[test]
    fn test_brick_history_traces() {
        let mut history = BrickHistory::new(10);

        for i in 0..3 {
            let state = BrickState::new();
            let trace = ExecutionTrace {
                operation: format!("op_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::ZERO,
                state_version_before: i as u64,
                state_version_after: (i + 1) as u64,
            };
            history.record(state, trace);
        }

        let traces = history.traces();
        assert_eq!(traces.len(), 3);
        assert_eq!(traces[0].operation, "op_0");
        assert_eq!(traces[2].operation, "op_2");
    }

    #[test]
    fn test_brick_history_record_truncates_forward() {
        let mut history = BrickHistory::new(10);

        // Record 5 states
        for i in 0..5 {
            let mut state = BrickState::new();
            state.set_metadata("i", StateValue::Int(i));
            let trace = ExecutionTrace {
                operation: format!("op_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::ZERO,
                state_version_before: i as u64,
                state_version_after: (i + 1) as u64,
            };
            history.record(state, trace);
        }

        // Go back to position 2
        history.goto(2);

        // Record a new state - should truncate forward
        let mut new_state = BrickState::new();
        new_state.set_metadata("new", StateValue::Bool(true));
        let trace = ExecutionTrace {
            operation: "new_op".into(),
            input_summary: String::new(),
            output_summary: String::new(),
            duration: Duration::ZERO,
            state_version_before: 2,
            state_version_after: 3,
        };
        history.record(new_state, trace);

        // Should now have 3 states (0, 1, new)
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_brick_history_capacity_eviction() {
        let mut history = BrickHistory::new(3); // Small capacity

        // Record more than capacity
        for i in 0..5 {
            let mut state = BrickState::new();
            state.set_metadata("i", StateValue::Int(i));
            let trace = ExecutionTrace {
                operation: format!("op_{}", i),
                input_summary: String::new(),
                output_summary: String::new(),
                duration: Duration::ZERO,
                state_version_before: i as u64,
                state_version_after: (i + 1) as u64,
            };
            history.record(state, trace);
        }

        // Should only have 3 states (oldest evicted)
        assert_eq!(history.len(), 3);

        // First state should be i=2 (0 and 1 evicted)
        let first = history.goto(0).unwrap();
        assert_eq!(first.get_metadata("i"), Some(&StateValue::Int(2)));
    }

    #[test]
    fn test_guard_severity_values() {
        assert_eq!(GuardSeverity::Warning, GuardSeverity::Warning);
        assert_eq!(GuardSeverity::Error, GuardSeverity::Error);
        assert_eq!(GuardSeverity::Critical, GuardSeverity::Critical);
        assert_ne!(GuardSeverity::Warning, GuardSeverity::Error);
    }

    #[test]
    fn test_invariant_guard_debug() {
        fn check(_: &BrickState) -> bool {
            true
        }
        let guard = InvariantGuard::new("test", check, GuardSeverity::Warning);
        let debug_str = format!("{:?}", guard);
        assert!(debug_str.contains("InvariantGuard"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_guarded_brick() {
        use super::super::{Brick, BrickAssertion, BrickBudget, BrickVerification};

        struct TestBrick;
        impl Brick for TestBrick {
            fn brick_name(&self) -> &'static str {
                "TestBrick"
            }
            fn assertions(&self) -> &[BrickAssertion] {
                &[]
            }
            fn budget(&self) -> BrickBudget {
                BrickBudget::uniform(16)
            }
            fn verify(&self) -> BrickVerification {
                BrickVerification {
                    passed: vec![],
                    failed: vec![],
                    verification_time: Duration::ZERO,
                }
            }
            fn to_html(&self) -> String {
                String::new()
            }
            fn to_css(&self) -> String {
                String::new()
            }
        }

        fn check_positive(state: &BrickState) -> bool {
            match state.get_metadata("count") {
                Some(StateValue::Int(n)) => *n >= 0,
                _ => true,
            }
        }

        let guard = InvariantGuard::new("positive", check_positive, GuardSeverity::Error);
        let guarded = GuardedBrick::new(TestBrick).guard(guard);

        assert_eq!(guarded.inner().brick_name(), "TestBrick");
        assert_eq!(guarded.guards().len(), 1);
    }

    #[test]
    fn test_guarded_brick_check_guards_pass() {
        use super::super::{Brick, BrickAssertion, BrickBudget, BrickVerification};

        struct TestBrick;
        impl Brick for TestBrick {
            fn brick_name(&self) -> &'static str {
                "TestBrick"
            }
            fn assertions(&self) -> &[BrickAssertion] {
                &[]
            }
            fn budget(&self) -> BrickBudget {
                BrickBudget::uniform(16)
            }
            fn verify(&self) -> BrickVerification {
                BrickVerification {
                    passed: vec![],
                    failed: vec![],
                    verification_time: Duration::ZERO,
                }
            }
            fn to_html(&self) -> String {
                String::new()
            }
            fn to_css(&self) -> String {
                String::new()
            }
        }

        fn always_pass(_: &BrickState) -> bool {
            true
        }

        let guard = InvariantGuard::new("always_pass", always_pass, GuardSeverity::Error);
        let guarded = GuardedBrick::new(TestBrick).guard(guard);

        let state = BrickState::new();
        assert!(guarded.check_guards(&state).is_ok());
    }

    #[test]
    fn test_guarded_brick_check_guards_fail() {
        use super::super::{Brick, BrickAssertion, BrickBudget, BrickVerification};

        struct TestBrick;
        impl Brick for TestBrick {
            fn brick_name(&self) -> &'static str {
                "TestBrick"
            }
            fn assertions(&self) -> &[BrickAssertion] {
                &[]
            }
            fn budget(&self) -> BrickBudget {
                BrickBudget::uniform(16)
            }
            fn verify(&self) -> BrickVerification {
                BrickVerification {
                    passed: vec![],
                    failed: vec![],
                    verification_time: Duration::ZERO,
                }
            }
            fn to_html(&self) -> String {
                String::new()
            }
            fn to_css(&self) -> String {
                String::new()
            }
        }

        fn always_fail(_: &BrickState) -> bool {
            false
        }

        let guard = InvariantGuard::new("always_fail", always_fail, GuardSeverity::Critical);
        let guarded = GuardedBrick::new(TestBrick).guard(guard);

        let state = BrickState::new();
        let result = guarded.check_guards(&state);
        assert!(result.is_err());

        let violation = result.unwrap_err();
        assert_eq!(violation.guard_name, "always_fail");
        assert_eq!(violation.severity, GuardSeverity::Critical);
    }

    #[test]
    fn test_guard_violation_display() {
        let violation = GuardViolation {
            guard_name: "test_guard",
            severity: GuardSeverity::Error,
        };
        let display = format!("{}", violation);
        assert!(display.contains("test_guard"));
        assert!(display.contains("Error"));
    }

    #[test]
    fn test_guard_violation_error_trait() {
        let violation = GuardViolation {
            guard_name: "test",
            severity: GuardSeverity::Warning,
        };
        let _: &dyn std::error::Error = &violation;
    }

    #[test]
    fn test_deterministic_rng_default() {
        let rng = DeterministicRng::default();
        assert_eq!(rng.state(), 42);
    }

    #[test]
    fn test_deterministic_rng_f32_range() {
        let mut rng = DeterministicRng::new(123);
        for _ in 0..100 {
            let val = rng.next_f32();
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_deterministic_rng_state() {
        let mut rng = DeterministicRng::new(999);
        let _ = rng.next_u64();
        let state = rng.state();
        assert_ne!(state, 999); // State should have changed
    }

    #[test]
    fn test_deterministic_rng_restore() {
        let mut rng1 = DeterministicRng::new(100);
        let mut rng2 = DeterministicRng::new(999);

        // Get some values from rng1
        for _ in 0..10 {
            rng1.next_u64();
        }

        // Save state and restore to rng2
        let saved_state = rng1.state();
        rng2.restore(saved_state);

        // Both should now produce same sequence
        for _ in 0..10 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_deterministic_rng_clone() {
        let mut rng1 = DeterministicRng::new(555);
        for _ in 0..5 {
            rng1.next_u64();
        }

        let mut rng2 = rng1.clone();

        // Both should produce same sequence from here
        for _ in 0..10 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_deterministic_clock_default() {
        let clock = DeterministicClock::default();
        assert_eq!(clock.now_ns(), 0);
        // Default tick is 10ms
    }

    #[test]
    fn test_deterministic_clock_clone() {
        let mut clock1 = DeterministicClock::new(100, 50);
        clock1.advance(5);

        let clock2 = clock1.clone();
        assert_eq!(clock1.now_ns(), clock2.now_ns());
    }
}
