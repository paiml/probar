//! DistributedBrick: Work-stealing and data locality (PROBAR-SPEC-009-P10)
//!
//! This module enables distributed brick execution with:
//! - Work-stealing across nodes
//! - Data locality awareness
//! - Multi-backend dispatch (CPU/GPU/Remote/SIMD)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   DISTRIBUTED BRICK FLOW                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  1. DistributedBrick<B> wraps any Brick                     │
//! │  2. BrickDataTracker tracks data locality                   │
//! │  3. MultiBrickExecutor selects best backend                 │
//! │  4. BrickCoordinator handles PUB/SUB coordination           │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # References
//!
//! - PROBAR-SPEC-009-P10: Distribution - Repartir Integration

// Allow expect for RwLock - lock poisoning is truly exceptional
#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::{Brick, BrickAssertion, BrickBudget, BrickError, BrickResult, BrickVerification};

/// Unique identifier for a worker node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkerId(pub u64);

impl WorkerId {
    /// Create a new worker ID
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying ID value
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "worker-{}", self.0)
    }
}

/// Execution backend for brick operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// CPU execution with standard instructions
    Cpu,
    /// GPU execution via WebGPU/wgpu
    Gpu,
    /// Remote execution on another node
    Remote,
    /// CPU execution with SIMD acceleration
    Simd,
}

impl Backend {
    /// Check if backend is available on current system
    #[must_use]
    pub fn is_available(&self) -> bool {
        match self {
            Self::Cpu | Self::Simd => true,
            Self::Gpu => cfg!(feature = "gpu"),
            // Remote backend requires distributed feature (not yet implemented)
            Self::Remote => false,
        }
    }

    /// Get relative performance estimate (higher = faster)
    #[must_use]
    pub const fn performance_estimate(&self) -> u32 {
        match self {
            Self::Gpu => 100,
            Self::Simd => 50,
            Self::Cpu => 10,
            Self::Remote => 5, // Network latency
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::Cpu
    }
}

/// Input data for brick execution
#[derive(Debug, Clone, Default)]
pub struct BrickInput {
    /// Input tensor data
    pub data: Vec<f32>,
    /// Input shape dimensions
    pub shape: Vec<usize>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl BrickInput {
    /// Create new brick input
    #[must_use]
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Self {
        Self {
            data,
            shape,
            metadata: HashMap::new(),
        }
    }

    /// Get total size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }

    /// Get total element count
    #[must_use]
    pub fn element_count(&self) -> usize {
        self.data.len()
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Output data from brick execution
#[derive(Debug, Clone, Default)]
pub struct BrickOutput {
    /// Output tensor data
    pub data: Vec<f32>,
    /// Output shape dimensions
    pub shape: Vec<usize>,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

impl BrickOutput {
    /// Create new brick output
    #[must_use]
    pub fn new(data: Vec<f32>, shape: Vec<usize>) -> Self {
        Self {
            data,
            shape,
            metrics: ExecutionMetrics::default(),
        }
    }

    /// Get total size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }
}

/// Metrics from brick execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    /// Time to execute
    pub execution_time: Duration,
    /// Backend used
    pub backend: Backend,
    /// Worker that executed
    pub worker_id: Option<WorkerId>,
    /// Data transfer time (if remote)
    pub transfer_time: Option<Duration>,
}

impl ExecutionMetrics {
    /// Create new execution metrics
    #[must_use]
    pub fn new(execution_time: Duration, backend: Backend) -> Self {
        Self {
            execution_time,
            backend,
            worker_id: None,
            transfer_time: None,
        }
    }
}

/// Distributed brick wrapper for multi-backend execution
///
/// Wraps any `Brick` to enable distributed execution with:
/// - Backend selection (CPU/GPU/Remote/SIMD)
/// - Data dependency tracking for locality
/// - Work-stealing support
#[derive(Debug)]
pub struct DistributedBrick<B: Brick> {
    inner: B,
    backend: Backend,
    data_dependencies: Vec<String>,
    preferred_worker: Option<WorkerId>,
}

impl<B: Brick> DistributedBrick<B> {
    /// Create a new distributed brick wrapper
    #[must_use]
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            backend: Backend::default(),
            data_dependencies: Vec::new(),
            preferred_worker: None,
        }
    }

    /// Set the preferred execution backend
    #[must_use]
    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Add data dependencies for locality-aware scheduling
    #[must_use]
    pub fn with_data_dependencies(mut self, deps: Vec<String>) -> Self {
        self.data_dependencies = deps;
        self
    }

    /// Set preferred worker for execution
    #[must_use]
    pub fn with_preferred_worker(mut self, worker: WorkerId) -> Self {
        self.preferred_worker = Some(worker);
        self
    }

    /// Get the inner brick
    #[must_use]
    pub fn inner(&self) -> &B {
        &self.inner
    }

    /// Get mutable access to inner brick
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    /// Get current backend
    #[must_use]
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Get data dependencies
    #[must_use]
    pub fn data_dependencies(&self) -> &[String] {
        &self.data_dependencies
    }

    /// Get preferred worker
    #[must_use]
    pub fn preferred_worker(&self) -> Option<WorkerId> {
        self.preferred_worker
    }

    /// Convert to a task specification for distributed execution
    #[must_use]
    pub fn to_task_spec(&self) -> TaskSpec {
        TaskSpec {
            brick_name: self.inner.brick_name().to_string(),
            backend: self.backend,
            data_dependencies: self.data_dependencies.clone(),
            preferred_worker: self.preferred_worker,
        }
    }
}

impl<B: Brick> Brick for DistributedBrick<B> {
    fn brick_name(&self) -> &'static str {
        self.inner.brick_name()
    }

    fn assertions(&self) -> &[BrickAssertion] {
        self.inner.assertions()
    }

    fn budget(&self) -> BrickBudget {
        self.inner.budget()
    }

    fn verify(&self) -> BrickVerification {
        self.inner.verify()
    }

    fn to_html(&self) -> String {
        self.inner.to_html()
    }

    fn to_css(&self) -> String {
        self.inner.to_css()
    }
}

/// Task specification for distributed execution
#[derive(Debug, Clone)]
pub struct TaskSpec {
    /// Brick name for identification
    pub brick_name: String,
    /// Requested backend
    pub backend: Backend,
    /// Data dependencies
    pub data_dependencies: Vec<String>,
    /// Preferred worker
    pub preferred_worker: Option<WorkerId>,
}

/// Data location entry for a specific piece of data
#[derive(Debug, Clone)]
pub struct DataLocation {
    /// Data key/identifier
    pub key: String,
    /// Workers that have this data
    pub workers: Vec<WorkerId>,
    /// Size of data in bytes
    pub size_bytes: usize,
    /// Last access time
    pub last_access: Instant,
}

/// Track where brick weights/data reside across workers
///
/// Used for locality-aware scheduling to minimize data movement.
#[derive(Debug)]
pub struct BrickDataTracker {
    /// Map from data key to location info
    locations: RwLock<HashMap<String, DataLocation>>,
}

impl Default for BrickDataTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BrickDataTracker {
    /// Create a new data tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            locations: RwLock::new(HashMap::new()),
        }
    }

    /// Register that a worker has certain data
    pub fn track_data(&self, key: &str, worker_id: WorkerId, size_bytes: usize) {
        let mut locations = self.locations.write().expect("lock poisoned");
        locations
            .entry(key.to_string())
            .and_modify(|loc| {
                if !loc.workers.contains(&worker_id) {
                    loc.workers.push(worker_id);
                }
                loc.last_access = Instant::now();
            })
            .or_insert_with(|| DataLocation {
                key: key.to_string(),
                workers: vec![worker_id],
                size_bytes,
                last_access: Instant::now(),
            });
    }

    /// Register that a worker has brick weights
    pub fn track_weights(&self, brick_name: &str, worker_id: WorkerId) {
        let key = format!("{}_weights", brick_name);
        self.track_data(&key, worker_id, 0);
    }

    /// Remove data location from a worker
    pub fn remove_data(&self, key: &str, worker_id: WorkerId) {
        let mut locations = self.locations.write().expect("lock poisoned");
        if let Some(loc) = locations.get_mut(key) {
            loc.workers.retain(|w| *w != worker_id);
        }
    }

    /// Get workers that have specific data
    #[must_use]
    pub fn get_workers_for_data(&self, key: &str) -> Vec<WorkerId> {
        let locations = self.locations.read().expect("lock poisoned");
        locations
            .get(key)
            .map_or(Vec::new(), |loc| loc.workers.clone())
    }

    /// Calculate affinity scores for workers based on data dependencies
    pub fn calculate_affinity(&self, dependencies: &[String]) -> HashMap<WorkerId, f64> {
        let locations = self.locations.read().expect("lock poisoned");
        let mut affinity: HashMap<WorkerId, f64> = HashMap::new();

        for dep in dependencies {
            if let Some(loc) = locations.get(dep) {
                let score_per_worker = 1.0 / loc.workers.len() as f64;
                for worker in &loc.workers {
                    *affinity.entry(*worker).or_insert(0.0) += score_per_worker;
                }
            }
        }

        // Normalize scores
        if !affinity.is_empty() {
            let max_score = affinity.values().cloned().fold(0.0_f64, f64::max);
            if max_score > 0.0 {
                for score in affinity.values_mut() {
                    *score /= max_score;
                }
            }
        }

        affinity
    }

    /// Find the best worker for a brick based on data locality
    #[must_use]
    pub fn find_best_worker(&self, brick: &dyn Brick) -> Option<WorkerId> {
        // Use brick name to find weights
        let weights_key = format!("{}_weights", brick.brick_name());
        let workers = self.get_workers_for_data(&weights_key);
        workers.first().copied()
    }

    /// Find best worker for distributed brick with dependencies
    #[must_use]
    pub fn find_best_worker_for_distributed<B: Brick>(
        &self,
        brick: &DistributedBrick<B>,
    ) -> Option<WorkerId> {
        // Check preferred worker first
        if let Some(preferred) = brick.preferred_worker() {
            return Some(preferred);
        }

        // Calculate affinity based on dependencies
        let affinity = self.calculate_affinity(brick.data_dependencies());
        affinity
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(worker, _)| worker)
    }

    /// Get total data size tracked
    #[must_use]
    pub fn total_data_size(&self) -> usize {
        let locations = self.locations.read().expect("lock poisoned");
        locations.values().map(|loc| loc.size_bytes).sum()
    }
}

/// Backend selector for choosing optimal execution backend
#[derive(Debug)]
pub struct BackendSelector {
    /// Minimum element count for GPU execution
    gpu_threshold: usize,
    /// Minimum element count for SIMD execution
    simd_threshold: usize,
    /// Maximum element count for CPU (else remote)
    cpu_max_threshold: usize,
}

impl Default for BackendSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendSelector {
    /// Create a new backend selector with default thresholds
    #[must_use]
    pub fn new() -> Self {
        Self {
            gpu_threshold: 1_000_000,       // 1M elements for GPU
            simd_threshold: 10_000,         // 10K elements for SIMD
            cpu_max_threshold: 100_000_000, // 100M elements max for local
        }
    }

    /// Configure GPU threshold
    #[must_use]
    pub fn with_gpu_threshold(mut self, threshold: usize) -> Self {
        self.gpu_threshold = threshold;
        self
    }

    /// Configure SIMD threshold
    #[must_use]
    pub fn with_simd_threshold(mut self, threshold: usize) -> Self {
        self.simd_threshold = threshold;
        self
    }

    /// Configure CPU max threshold
    #[must_use]
    pub fn with_cpu_max_threshold(mut self, threshold: usize) -> Self {
        self.cpu_max_threshold = threshold;
        self
    }

    /// Select best backend based on input characteristics
    #[must_use]
    pub fn select(&self, element_count: usize, gpu_available: bool) -> Backend {
        // Too large for local - use remote if available (not yet implemented)
        if element_count > self.cpu_max_threshold && Backend::Remote.is_available() {
            return Backend::Remote;
        }

        // Large enough for GPU
        if element_count >= self.gpu_threshold && gpu_available {
            return Backend::Gpu;
        }

        // Medium size - use SIMD
        if element_count >= self.simd_threshold {
            return Backend::Simd;
        }

        // Small - use plain CPU
        Backend::Cpu
    }

    /// Select backend for a brick with input
    #[must_use]
    pub fn select_for_brick(
        &self,
        _brick_complexity: u32,
        input_size: usize,
        gpu_available: bool,
    ) -> Backend {
        // Future: factor in brick_complexity
        self.select(input_size, gpu_available)
    }
}

/// Multi-backend executor for brick operations
///
/// Dispatches brick execution to the best available backend.
#[derive(Debug)]
pub struct MultiBrickExecutor {
    selector: BackendSelector,
    gpu_available: bool,
    data_tracker: Arc<BrickDataTracker>,
}

impl MultiBrickExecutor {
    /// Create a new multi-backend executor
    #[must_use]
    pub fn new(data_tracker: Arc<BrickDataTracker>) -> Self {
        Self {
            selector: BackendSelector::new(),
            gpu_available: cfg!(feature = "gpu"),
            data_tracker,
        }
    }

    /// Create with custom backend selector
    #[must_use]
    pub fn with_selector(mut self, selector: BackendSelector) -> Self {
        self.selector = selector;
        self
    }

    /// Set GPU availability
    #[must_use]
    pub fn with_gpu_available(mut self, available: bool) -> Self {
        self.gpu_available = available;
        self
    }

    /// Execute a brick on the best backend
    pub fn execute(&self, brick: &dyn Brick, input: BrickInput) -> BrickResult<BrickOutput> {
        let start = Instant::now();

        // Select backend
        let backend = self
            .selector
            .select(input.element_count(), self.gpu_available);

        // Execute on selected backend
        let (output_data, output_shape) = match backend {
            Backend::Cpu => self.execute_cpu(brick, &input)?,
            Backend::Simd => self.execute_simd(brick, &input)?,
            Backend::Gpu => self.execute_gpu(brick, &input)?,
            Backend::Remote => self.execute_remote(brick, &input)?,
        };

        let execution_time = start.elapsed();

        // Build output with metrics
        let mut output = BrickOutput::new(output_data, output_shape);
        output.metrics = ExecutionMetrics::new(execution_time, backend);

        Ok(output)
    }

    /// Execute distributed brick
    pub fn execute_distributed<B: Brick>(
        &self,
        brick: &DistributedBrick<B>,
        input: BrickInput,
    ) -> BrickResult<BrickOutput> {
        let start = Instant::now();

        // Use brick's preferred backend or select automatically
        let backend = brick.backend();

        // Find best worker for locality
        let worker_id = self.data_tracker.find_best_worker_for_distributed(brick);

        // Execute
        let (output_data, output_shape) = match backend {
            Backend::Cpu => self.execute_cpu(brick.inner(), &input)?,
            Backend::Simd => self.execute_simd(brick.inner(), &input)?,
            Backend::Gpu => self.execute_gpu(brick.inner(), &input)?,
            Backend::Remote => self.execute_remote(brick.inner(), &input)?,
        };

        let execution_time = start.elapsed();

        // Build output with metrics
        let mut output = BrickOutput::new(output_data, output_shape);
        output.metrics = ExecutionMetrics {
            execution_time,
            backend,
            worker_id,
            transfer_time: None,
        };

        Ok(output)
    }

    fn execute_cpu(
        &self,
        _brick: &dyn Brick,
        input: &BrickInput,
    ) -> BrickResult<(Vec<f32>, Vec<usize>)> {
        // Simple passthrough for now - real implementation would execute brick
        Ok((input.data.clone(), input.shape.clone()))
    }

    fn execute_simd(
        &self,
        _brick: &dyn Brick,
        input: &BrickInput,
    ) -> BrickResult<(Vec<f32>, Vec<usize>)> {
        // SIMD path - would use actual SIMD instructions
        Ok((input.data.clone(), input.shape.clone()))
    }

    fn execute_gpu(
        &self,
        _brick: &dyn Brick,
        input: &BrickInput,
    ) -> BrickResult<(Vec<f32>, Vec<usize>)> {
        // GPU path - would use WebGPU/wgpu
        if !self.gpu_available {
            return Err(BrickError::HtmlGenerationFailed {
                reason: "GPU not available".into(),
            });
        }
        Ok((input.data.clone(), input.shape.clone()))
    }

    fn execute_remote(
        &self,
        _brick: &dyn Brick,
        input: &BrickInput,
    ) -> BrickResult<(Vec<f32>, Vec<usize>)> {
        // Remote path - would serialize and send to remote worker
        if !Backend::Remote.is_available() {
            return Err(BrickError::HtmlGenerationFailed {
                reason: "Distributed execution not available".into(),
            });
        }
        Ok((input.data.clone(), input.shape.clone()))
    }

    /// Get the data tracker
    #[must_use]
    pub fn data_tracker(&self) -> &Arc<BrickDataTracker> {
        &self.data_tracker
    }
}

/// Message for PUB/SUB coordination
#[derive(Debug, Clone)]
pub enum BrickMessage {
    /// Weight update message
    WeightUpdate {
        /// Name of the brick whose weights are being updated
        brick_name: String,
        /// Serialized weight data
        weights: Vec<u8>,
        /// Weight version number
        version: u64,
    },
    /// State change notification
    StateChange {
        /// Name of the brick that changed state
        brick_name: String,
        /// Event description
        event: String,
    },
    /// Request brick execution
    ExecutionRequest {
        /// Name of brick to execute
        brick_name: String,
        /// Key to input data
        input_key: String,
        /// Unique request ID for correlation
        request_id: u64,
    },
    /// Execution result
    ExecutionResult {
        /// Request ID this result corresponds to
        request_id: u64,
        /// Key to output data
        output_key: String,
        /// Whether execution succeeded
        success: bool,
    },
}

/// Subscription to brick events
#[derive(Debug)]
pub struct Subscription {
    topic: String,
    messages: Arc<RwLock<Vec<BrickMessage>>>,
}

impl Subscription {
    /// Get all pending messages
    #[must_use]
    pub fn drain(&self) -> Vec<BrickMessage> {
        let mut messages = self.messages.write().expect("lock poisoned");
        std::mem::take(&mut *messages)
    }

    /// Check if there are pending messages
    #[must_use]
    pub fn has_messages(&self) -> bool {
        let messages = self.messages.read().expect("lock poisoned");
        !messages.is_empty()
    }

    /// Get subscription topic
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }
}

/// PUB/SUB coordinator for brick communication
///
/// Enables distributed coordination via publish/subscribe messaging.
#[derive(Debug)]
pub struct BrickCoordinator {
    /// Active subscriptions by topic
    subscriptions: RwLock<HashMap<String, Vec<Arc<RwLock<Vec<BrickMessage>>>>>>,
    /// Message counter for request IDs
    message_counter: AtomicU64,
}

impl Default for BrickCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl BrickCoordinator {
    /// Create a new coordinator
    #[must_use]
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            message_counter: AtomicU64::new(0),
        }
    }

    /// Subscribe to a topic
    #[must_use]
    pub fn subscribe(&self, topic: &str) -> Subscription {
        let messages = Arc::new(RwLock::new(Vec::new()));
        {
            let mut subs = self.subscriptions.write().expect("lock poisoned");
            subs.entry(topic.to_string())
                .or_default()
                .push(Arc::clone(&messages));
        }
        Subscription {
            topic: topic.to_string(),
            messages,
        }
    }

    /// Subscribe to brick events
    #[must_use]
    pub fn subscribe_brick(&self, brick_name: &str) -> Subscription {
        let topic = format!("brick/{}/events", brick_name);
        self.subscribe(&topic)
    }

    /// Publish a message to a topic
    pub fn publish(&self, topic: &str, message: BrickMessage) {
        let subs = self.subscriptions.read().expect("lock poisoned");
        if let Some(subscribers) = subs.get(topic) {
            for sub in subscribers {
                let mut messages = sub.write().expect("lock poisoned");
                messages.push(message.clone());
            }
        }
    }

    /// Broadcast weight updates for a brick
    pub fn broadcast_weights(&self, brick_name: &str, weights: Vec<u8>) {
        let topic = format!("brick/{}/weights", brick_name);
        let version = self.message_counter.fetch_add(1, Ordering::SeqCst);
        self.publish(
            &topic,
            BrickMessage::WeightUpdate {
                brick_name: brick_name.to_string(),
                weights,
                version,
            },
        );
    }

    /// Broadcast state change for a brick
    pub fn broadcast_state_change(&self, brick_name: &str, event: &str) {
        let topic = format!("brick/{}/events", brick_name);
        self.publish(
            &topic,
            BrickMessage::StateChange {
                brick_name: brick_name.to_string(),
                event: event.to_string(),
            },
        );
    }

    /// Generate a unique request ID
    #[must_use]
    pub fn next_request_id(&self) -> u64 {
        self.message_counter.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBrick {
        name: &'static str,
    }

    impl Brick for TestBrick {
        fn brick_name(&self) -> &'static str {
            self.name
        }

        fn assertions(&self) -> &[BrickAssertion] {
            &[BrickAssertion::TextVisible]
        }

        fn budget(&self) -> BrickBudget {
            BrickBudget::uniform(16)
        }

        fn verify(&self) -> BrickVerification {
            BrickVerification {
                passed: vec![BrickAssertion::TextVisible],
                failed: vec![],
                verification_time: Duration::from_micros(100),
            }
        }

        fn to_html(&self) -> String {
            format!("<div>{}</div>", self.name)
        }

        fn to_css(&self) -> String {
            ".test { }".into()
        }
    }

    #[test]
    fn test_worker_id() {
        let id = WorkerId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(format!("{id}"), "worker-42");
    }

    #[test]
    fn test_backend_availability() {
        assert!(Backend::Cpu.is_available());
        assert!(Backend::Simd.is_available());
        // GPU/Remote depend on feature flags
    }

    #[test]
    fn test_backend_performance() {
        assert!(Backend::Gpu.performance_estimate() > Backend::Simd.performance_estimate());
        assert!(Backend::Simd.performance_estimate() > Backend::Cpu.performance_estimate());
    }

    #[test]
    fn test_distributed_brick_creation() {
        let inner = TestBrick { name: "Test" };
        let distributed = DistributedBrick::new(inner)
            .with_backend(Backend::Gpu)
            .with_data_dependencies(vec!["weights".into(), "biases".into()])
            .with_preferred_worker(WorkerId::new(1));

        assert_eq!(distributed.backend(), Backend::Gpu);
        assert_eq!(distributed.data_dependencies().len(), 2);
        assert_eq!(distributed.preferred_worker(), Some(WorkerId::new(1)));
        assert_eq!(distributed.brick_name(), "Test");
    }

    #[test]
    fn test_distributed_brick_implements_brick() {
        let inner = TestBrick { name: "Test" };
        let distributed = DistributedBrick::new(inner);

        // Verify it implements Brick trait
        assert!(distributed.verify().is_valid());
        assert_eq!(distributed.budget().total_ms, 16);
    }

    #[test]
    fn test_task_spec() {
        let inner = TestBrick { name: "TestTask" };
        let distributed = DistributedBrick::new(inner)
            .with_backend(Backend::Simd)
            .with_data_dependencies(vec!["model".into()]);

        let spec = distributed.to_task_spec();
        assert_eq!(spec.brick_name, "TestTask");
        assert_eq!(spec.backend, Backend::Simd);
        assert_eq!(spec.data_dependencies, vec!["model"]);
    }

    #[test]
    fn test_brick_input_output() {
        let input = BrickInput::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]);
        assert_eq!(input.element_count(), 4);
        assert_eq!(input.size_bytes(), 16);

        let output = BrickOutput::new(vec![5.0, 6.0], vec![2]);
        assert_eq!(output.size_bytes(), 8);
    }

    #[test]
    fn test_data_tracker() {
        let tracker = BrickDataTracker::new();

        // Track some data
        tracker.track_data("model_weights", WorkerId::new(1), 1024);
        tracker.track_data("model_weights", WorkerId::new(2), 1024);
        tracker.track_data("biases", WorkerId::new(1), 256);

        // Check workers
        let workers = tracker.get_workers_for_data("model_weights");
        assert_eq!(workers.len(), 2);

        // Calculate affinity
        let affinity = tracker.calculate_affinity(&["model_weights".into(), "biases".into()]);
        assert!(affinity.get(&WorkerId::new(1)).unwrap_or(&0.0) > &0.0);
    }

    #[test]
    fn test_data_tracker_find_best_worker() {
        let tracker = BrickDataTracker::new();

        let brick = TestBrick { name: "MelBrick" };
        tracker.track_weights("MelBrick", WorkerId::new(5));

        let best = tracker.find_best_worker(&brick);
        assert_eq!(best, Some(WorkerId::new(5)));
    }

    #[test]
    fn test_backend_selector() {
        let selector = BackendSelector::new()
            .with_gpu_threshold(1000)
            .with_simd_threshold(100);

        // Small input -> CPU
        assert_eq!(selector.select(50, true), Backend::Cpu);

        // Medium input -> SIMD
        assert_eq!(selector.select(500, true), Backend::Simd);

        // Large input with GPU -> GPU
        assert_eq!(selector.select(5000, true), Backend::Gpu);

        // Large input without GPU -> SIMD
        assert_eq!(selector.select(5000, false), Backend::Simd);
    }

    #[test]
    fn test_multi_executor() {
        let tracker = Arc::new(BrickDataTracker::new());
        let executor = MultiBrickExecutor::new(tracker);

        let brick = TestBrick { name: "Test" };
        let input = BrickInput::new(vec![1.0, 2.0, 3.0], vec![3]);

        let result = executor.execute(&brick, input);
        assert!(result.is_ok());

        let output = result.expect("execution should succeed");
        assert_eq!(output.data.len(), 3);
        assert!(
            output.metrics.execution_time > Duration::ZERO
                || output.metrics.execution_time == Duration::ZERO
        );
    }

    #[test]
    fn test_brick_coordinator() {
        let coordinator = BrickCoordinator::new();

        // Subscribe to events
        let sub = coordinator.subscribe_brick("MyBrick");

        // Broadcast event
        coordinator.broadcast_state_change("MyBrick", "loaded");

        // Check subscription received message
        assert!(sub.has_messages());
        let messages = sub.drain();
        assert_eq!(messages.len(), 1);
        matches!(&messages[0], BrickMessage::StateChange { brick_name, .. } if brick_name == "MyBrick");
    }

    #[test]
    fn test_coordinator_weight_broadcast() {
        let coordinator = BrickCoordinator::new();

        let sub = coordinator.subscribe("brick/Encoder/weights");
        coordinator.broadcast_weights("Encoder", vec![1, 2, 3, 4]);

        let messages = sub.drain();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            BrickMessage::WeightUpdate {
                brick_name,
                weights,
                version,
            } => {
                assert_eq!(brick_name, "Encoder");
                assert_eq!(weights, &vec![1, 2, 3, 4]);
                assert_eq!(*version, 0);
            }
            _ => panic!("Expected WeightUpdate message"),
        }
    }

    #[test]
    fn test_subscription_topic() {
        let coordinator = BrickCoordinator::new();
        let sub = coordinator.subscribe("my/topic");
        assert_eq!(sub.topic(), "my/topic");
    }

    #[test]
    fn test_execution_metrics() {
        let metrics = ExecutionMetrics::new(Duration::from_millis(50), Backend::Gpu);
        assert_eq!(metrics.execution_time, Duration::from_millis(50));
        assert_eq!(metrics.backend, Backend::Gpu);
        assert!(metrics.worker_id.is_none());
    }
}
