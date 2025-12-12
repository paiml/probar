//! WASM Development Server
//!
//! HTTP server with hot reload support for WASM development.

use super::config::WasmRunnerConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Debug output configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DebugOutput {
    /// Show console.log from WASM
    pub console: bool,
    /// Show performance metrics
    pub metrics: bool,
    /// Show network requests
    pub network: bool,
    /// Show WASM memory usage
    pub memory: bool,
}

impl Default for DebugOutput {
    fn default() -> Self {
        Self {
            console: true,
            metrics: true,
            network: false,
            memory: false,
        }
    }
}

impl DebugOutput {
    /// Create with all output disabled
    #[must_use]
    pub fn none() -> Self {
        Self {
            console: false,
            metrics: false,
            network: false,
            memory: false,
        }
    }

    /// Create with all output enabled
    #[must_use]
    pub fn all() -> Self {
        Self {
            console: true,
            metrics: true,
            network: true,
            memory: true,
        }
    }

    /// Enable console output
    #[must_use]
    pub fn with_console(mut self) -> Self {
        self.console = true;
        self
    }

    /// Enable metrics output
    #[must_use]
    pub fn with_metrics(mut self) -> Self {
        self.metrics = true;
        self
    }

    /// Enable network output
    #[must_use]
    pub fn with_network(mut self) -> Self {
        self.network = true;
        self
    }

    /// Enable memory output
    #[must_use]
    pub fn with_memory(mut self) -> Self {
        self.memory = true;
        self
    }
}

/// Hot reload event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotReloadEvent {
    /// File changed, triggering rebuild
    FileChanged {
        /// Path to changed file
        path: String,
    },
    /// Rebuild started
    RebuildStarted,
    /// Rebuild completed successfully
    Rebuild {
        /// Build duration
        duration: Duration,
        /// State types preserved
        preserved: Vec<String>,
    },
    /// Rebuild failed
    RebuildFailed {
        /// Error messages
        errors: Vec<String>,
    },
    /// Client connected
    ClientConnected {
        /// Client ID
        id: u32,
    },
    /// Client disconnected
    ClientDisconnected {
        /// Client ID
        id: u32,
    },
}

/// Console message from WASM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    /// Log level
    pub level: ConsoleLevel,
    /// Message text
    pub text: String,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// Console log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsoleLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Log level
    Log,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

impl ConsoleLevel {
    /// Get the prefix for terminal output
    #[must_use]
    pub fn prefix(&self) -> &'static str {
        match self {
            ConsoleLevel::Debug => "\x1b[34m[DEBUG]\x1b[0m",
            ConsoleLevel::Info => "\x1b[36m[INFO]\x1b[0m",
            ConsoleLevel::Log => "[LOG]",
            ConsoleLevel::Warn => "\x1b[33m[WARN]\x1b[0m",
            ConsoleLevel::Error => "\x1b[31m[ERROR]\x1b[0m",
        }
    }
}

/// WASM development server runner
#[derive(Debug)]
pub struct WasmRunner {
    config: WasmRunnerConfig,
    debug_output: DebugOutput,
    running: bool,
    clients: Vec<u32>,
    next_client_id: u32,
}

impl WasmRunner {
    /// Create a new runner with configuration
    #[must_use]
    pub fn new(config: WasmRunnerConfig) -> Self {
        Self {
            config,
            debug_output: DebugOutput::default(),
            running: false,
            clients: Vec::new(),
            next_client_id: 1,
        }
    }

    /// Create a builder
    #[must_use]
    pub fn builder() -> WasmRunnerBuilder {
        WasmRunnerBuilder::default()
    }

    /// Get configuration
    #[must_use]
    pub fn config(&self) -> &WasmRunnerConfig {
        &self.config
    }

    /// Get debug output configuration
    #[must_use]
    pub fn debug_output(&self) -> &DebugOutput {
        &self.debug_output
    }

    /// Set debug output configuration
    pub fn set_debug_output(&mut self, output: DebugOutput) {
        self.debug_output = output;
    }

    /// Check if server is running
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get connected client count
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Simulate starting the server (for testing)
    pub fn simulate_start(&mut self) {
        self.running = true;
    }

    /// Simulate stopping the server (for testing)
    pub fn simulate_stop(&mut self) {
        self.running = false;
        self.clients.clear();
    }

    /// Simulate client connection (for testing)
    pub fn simulate_client_connect(&mut self) -> u32 {
        let id = self.next_client_id;
        self.next_client_id += 1;
        self.clients.push(id);
        id
    }

    /// Simulate client disconnection (for testing)
    pub fn simulate_client_disconnect(&mut self, id: u32) {
        self.clients.retain(|&c| c != id);
    }

    /// Format a console message for terminal output
    #[must_use]
    pub fn format_console_message(&self, msg: &ConsoleMessage) -> String {
        use chrono::{DateTime, Local};
        let timestamp: DateTime<Local> = msg.timestamp.into();
        format!(
            "[{}] {} {}",
            timestamp.format("%H:%M:%S"),
            msg.level.prefix(),
            msg.text
        )
    }

    /// Get server URL
    #[must_use]
    pub fn http_url(&self) -> String {
        format!("http://localhost:{}", self.config.http_port)
    }

    /// Get WebSocket URL
    #[must_use]
    pub fn ws_url(&self) -> String {
        format!("ws://localhost:{}", self.config.ws_port)
    }
}

/// Builder for `WasmRunner`
#[derive(Debug, Clone, Default)]
pub struct WasmRunnerBuilder {
    config: WasmRunnerConfig,
    debug_output: DebugOutput,
}

impl WasmRunnerBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set HTTP port
    #[must_use]
    pub fn http_port(mut self, port: u16) -> Self {
        self.config.http_port = port;
        self
    }

    /// Set WebSocket port
    #[must_use]
    pub fn ws_port(mut self, port: u16) -> Self {
        self.config.ws_port = port;
        self
    }

    /// Enable/disable hot reload
    #[must_use]
    pub fn hot_reload(mut self, enabled: bool) -> Self {
        self.config.hot_reload = enabled;
        self
    }

    /// Enable/disable state preservation
    #[must_use]
    pub fn preserve_state(mut self, enabled: bool) -> Self {
        self.config.preserve_state = enabled;
        self
    }

    /// Enable/disable source maps
    #[must_use]
    pub fn source_maps(mut self, enabled: bool) -> Self {
        self.config.source_maps = enabled;
        self
    }

    /// Set debug output configuration
    #[must_use]
    pub fn debug_output(mut self, output: DebugOutput) -> Self {
        self.debug_output = output;
        self
    }

    /// Build the runner
    #[must_use]
    pub fn build(self) -> WasmRunner {
        let mut runner = WasmRunner::new(self.config);
        runner.debug_output = self.debug_output;
        runner
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_output_default() {
        let output = DebugOutput::default();
        assert!(output.console);
        assert!(output.metrics);
        assert!(!output.network);
        assert!(!output.memory);
    }

    #[test]
    fn test_debug_output_none() {
        let output = DebugOutput::none();
        assert!(!output.console);
        assert!(!output.metrics);
        assert!(!output.network);
        assert!(!output.memory);
    }

    #[test]
    fn test_debug_output_all() {
        let output = DebugOutput::all();
        assert!(output.console);
        assert!(output.metrics);
        assert!(output.network);
        assert!(output.memory);
    }

    #[test]
    fn test_debug_output_builders() {
        let output = DebugOutput::none()
            .with_console()
            .with_network();

        assert!(output.console);
        assert!(!output.metrics);
        assert!(output.network);
        assert!(!output.memory);
    }

    #[test]
    fn test_console_level_prefix() {
        assert!(ConsoleLevel::Error.prefix().contains("[ERROR]"));
        assert!(ConsoleLevel::Warn.prefix().contains("[WARN]"));
        assert!(ConsoleLevel::Info.prefix().contains("[INFO]"));
    }

    #[test]
    fn test_wasm_runner_new() {
        let config = WasmRunnerConfig::default();
        let runner = WasmRunner::new(config);
        assert!(!runner.is_running());
        assert_eq!(runner.client_count(), 0);
    }

    #[test]
    fn test_wasm_runner_builder() {
        let runner = WasmRunnerBuilder::new()
            .http_port(9000)
            .ws_port(9001)
            .hot_reload(false)
            .build();

        assert_eq!(runner.config().http_port, 9000);
        assert_eq!(runner.config().ws_port, 9001);
        assert!(!runner.config().hot_reload);
    }

    #[test]
    fn test_wasm_runner_urls() {
        let runner = WasmRunnerBuilder::new()
            .http_port(8080)
            .ws_port(8081)
            .build();

        assert_eq!(runner.http_url(), "http://localhost:8080");
        assert_eq!(runner.ws_url(), "ws://localhost:8081");
    }

    #[test]
    fn test_wasm_runner_simulate() {
        let mut runner = WasmRunner::new(WasmRunnerConfig::default());

        runner.simulate_start();
        assert!(runner.is_running());

        let id1 = runner.simulate_client_connect();
        let id2 = runner.simulate_client_connect();
        assert_eq!(runner.client_count(), 2);

        runner.simulate_client_disconnect(id1);
        assert_eq!(runner.client_count(), 1);

        runner.simulate_stop();
        assert!(!runner.is_running());
        assert_eq!(runner.client_count(), 0);

        // Ensure IDs are unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_hot_reload_event_variants() {
        let event = HotReloadEvent::FileChanged {
            path: "src/main.rs".to_string(),
        };
        assert!(matches!(event, HotReloadEvent::FileChanged { .. }));

        let event = HotReloadEvent::Rebuild {
            duration: Duration::from_millis(500),
            preserved: vec!["State".to_string()],
        };
        assert!(matches!(event, HotReloadEvent::Rebuild { .. }));
    }

    #[test]
    fn test_console_message() {
        let msg = ConsoleMessage {
            level: ConsoleLevel::Info,
            text: "Test message".to_string(),
            timestamp: std::time::SystemTime::now(),
        };

        let runner = WasmRunner::new(WasmRunnerConfig::default());
        let formatted = runner.format_console_message(&msg);
        assert!(formatted.contains("[INFO]"));
        assert!(formatted.contains("Test message"));
    }
}
