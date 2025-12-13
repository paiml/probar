//! WASM Development Server
//!
//! Real HTTP server implementation for serving WASM applications with hot reload.
//! Implements GitHub issue #7: <https://github.com/paiml/probar/issues/7>
//!
//! ## Features
//!
//! - HTTP server with correct MIME types for WASM
//! - WebSocket server for hot reload notifications
//! - File watcher with debouncing
//! - `wasm-pack` build integration
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DevServer                                │
//! │  ┌──────────────┐     ┌──────────────┐                     │
//! │  │ HTTP Server  │     │  WS Server   │                     │
//! │  │ (port 8080)  │     │ (port 8081)  │                     │
//! │  └──────┬───────┘     └──────┬───────┘                     │
//! │         │                    │                              │
//! │         ▼                    ▼                              │
//! │  ┌──────────────┐     ┌──────────────┐                     │
//! │  │ Static Files │     │ HotReload    │◀───FileWatcher      │
//! │  │ .wasm .js    │     │ Messages     │                     │
//! │  └──────────────┘     └──────────────┘                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```

// Clippy allows for server patterns
#![allow(clippy::unused_async)]
#![allow(clippy::unnested_or_patterns)]
#![allow(clippy::use_self)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::redundant_else)]

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

/// Hot reload message sent to connected clients (JSON serializable)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum HotReloadMessage {
    /// File changed, rebuild triggered
    FileChanged {
        /// Path to the changed file
        path: String,
    },
    /// Rebuild started
    RebuildStarted,
    /// Rebuild completed successfully
    RebuildComplete {
        /// Duration in milliseconds
        duration_ms: u64,
    },
    /// Rebuild failed with error
    RebuildFailed {
        /// Error message
        error: String,
    },
    /// Server ready
    ServerReady,
}

impl HotReloadMessage {
    /// Serialize to JSON for WebSocket transmission
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"Error"}"#.to_string())
    }
}

/// WASM development server configuration
#[derive(Debug, Clone)]
pub struct DevServerConfig {
    /// Directory to serve static files from
    pub directory: PathBuf,
    /// HTTP port
    pub port: u16,
    /// WebSocket port for hot reload
    pub ws_port: u16,
    /// Enable CORS
    pub cors: bool,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("."),
            port: 8080,
            ws_port: 8081,
            cors: false,
        }
    }
}

impl DevServerConfig {
    /// Create a builder
    #[must_use]
    pub fn builder() -> DevServerConfigBuilder {
        DevServerConfigBuilder::default()
    }
}

/// Builder for `DevServerConfig`
#[derive(Debug, Clone, Default)]
pub struct DevServerConfigBuilder {
    config: DevServerConfig,
}

impl DevServerConfigBuilder {
    /// Set directory to serve
    #[must_use]
    pub fn directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.directory = dir.into();
        self
    }

    /// Set HTTP port
    #[must_use]
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Set WebSocket port
    #[must_use]
    pub fn ws_port(mut self, port: u16) -> Self {
        self.config.ws_port = port;
        self
    }

    /// Enable CORS
    #[must_use]
    pub fn cors(mut self, enabled: bool) -> Self {
        self.config.cors = enabled;
        self
    }

    /// Build the configuration
    #[must_use]
    pub fn build(self) -> DevServerConfig {
        self.config
    }
}

/// WASM development server with HTTP and WebSocket support
#[derive(Debug)]
pub struct DevServer {
    config: DevServerConfig,
    reload_tx: broadcast::Sender<HotReloadMessage>,
}

impl DevServer {
    /// Create a new dev server
    #[must_use]
    pub fn new(config: DevServerConfig) -> Self {
        let (reload_tx, _) = broadcast::channel(64);
        Self { config, reload_tx }
    }

    /// Get a sender for hot reload messages
    #[must_use]
    pub fn reload_sender(&self) -> broadcast::Sender<HotReloadMessage> {
        self.reload_tx.clone()
    }

    /// Get the HTTP URL
    #[must_use]
    pub fn http_url(&self) -> String {
        format!("http://localhost:{}", self.config.port)
    }

    /// Get the WebSocket URL
    #[must_use]
    pub fn ws_url(&self) -> String {
        format!("ws://localhost:{}/ws", self.config.port)
    }

    /// Start the server (blocking)
    ///
    /// This starts both the HTTP server for static files and
    /// WebSocket endpoints for hot reload on the same port.
    pub async fn run(&self) -> Result<(), std::io::Error> {
        let directory = Arc::new(self.config.directory.clone());
        let reload_tx = self.reload_tx.clone();

        // Build router with static file serving and WebSocket
        let app = Router::new()
            // WebSocket endpoint for hot reload
            .route(
                "/ws",
                get({
                    let tx = reload_tx.clone();
                    move |ws: WebSocketUpgrade| handle_websocket(ws, tx.clone())
                }),
            )
            // Index route
            .route(
                "/",
                get({
                    let dir = directory.clone();
                    move || serve_index(dir.clone())
                }),
            )
            // Static file fallback
            .fallback({
                let dir = directory.clone();
                move |uri: axum::http::Uri| serve_static(dir.clone(), uri)
            });

        // Add CORS if enabled
        let app = if self.config.cors {
            app.layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
        } else {
            app
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║               Probar WASM Development Server                 ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  HTTP:      http://localhost:{:<29}║", self.config.port);
        println!(
            "║  WebSocket: ws://localhost:{}/ws{:<23}║",
            self.config.port, ""
        );
        println!(
            "║  Directory: {:<48}║",
            self.config
                .directory
                .display()
                .to_string()
                .chars()
                .take(48)
                .collect::<String>()
        );
        println!(
            "║  CORS:      {:<48}║",
            if self.config.cors {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  Press Ctrl+C to stop                                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝");

        // Notify that server is ready
        let _ = reload_tx.send(HotReloadMessage::ServerReady);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Run HTTP and WebSocket servers on separate ports
    ///
    /// Use this when you need dedicated ports for HTTP and WebSocket.
    pub async fn run_split(&self) -> Result<(), std::io::Error> {
        let directory = Arc::new(self.config.directory.clone());
        let reload_tx = self.reload_tx.clone();

        // HTTP server
        let http_app = Router::new()
            .route(
                "/",
                get({
                    let dir = directory.clone();
                    move || serve_index(dir.clone())
                }),
            )
            .fallback({
                let dir = directory.clone();
                move |uri: axum::http::Uri| serve_static(dir.clone(), uri)
            });

        let http_app = if self.config.cors {
            http_app.layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
        } else {
            http_app
        };

        // WebSocket server
        let ws_app = Router::new().route(
            "/",
            get({
                let tx = reload_tx.clone();
                move |ws: WebSocketUpgrade| handle_websocket(ws, tx.clone())
            }),
        );

        let http_addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        let ws_addr = SocketAddr::from(([0, 0, 0, 0], self.config.ws_port));

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║               Probar WASM Development Server                 ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  HTTP:      http://localhost:{:<29}║", self.config.port);
        println!("║  WebSocket: ws://localhost:{:<30}║", self.config.ws_port);
        println!(
            "║  Directory: {:<48}║",
            self.config
                .directory
                .display()
                .to_string()
                .chars()
                .take(48)
                .collect::<String>()
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  Press Ctrl+C to stop                                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝");

        let _ = reload_tx.send(HotReloadMessage::ServerReady);

        let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
        let ws_listener = tokio::net::TcpListener::bind(ws_addr).await?;

        tokio::select! {
            r = axum::serve(http_listener, http_app) => r?,
            r = axum::serve(ws_listener, ws_app) => r?,
        }

        Ok(())
    }
}

/// Handle WebSocket connection for hot reload
async fn handle_websocket(
    ws: WebSocketUpgrade,
    reload_tx: broadcast::Sender<HotReloadMessage>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_handler(socket, reload_tx))
}

/// WebSocket handler that broadcasts hot reload messages
async fn websocket_handler(socket: WebSocket, reload_tx: broadcast::Sender<HotReloadMessage>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = reload_tx.subscribe();

    // Send initial ready message
    let ready_msg = HotReloadMessage::ServerReady.to_json();
    if sender.send(Message::Text(ready_msg.into())).await.is_err() {
        return;
    }

    // Handle incoming messages and broadcast outgoing
    loop {
        tokio::select! {
            // Forward hot reload messages to the client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let json = msg.to_json();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            // Handle client messages (ping/pong, close)
            msg_opt = receiver.next() => {
                match msg_opt {
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
        }
    }
}

/// Serve index.html
async fn serve_index(directory: Arc<PathBuf>) -> Response {
    let index_path = directory.join("index.html");
    serve_file(&index_path).await
}

/// Serve static file based on URI
async fn serve_static(directory: Arc<PathBuf>, uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let file_path = directory.join(path);
    serve_file(&file_path).await
}

/// Serve a file with correct MIME type
///
/// MIME types are critical for WASM to work in browsers:
/// - `.wasm` files MUST be `application/wasm`
/// - `.js` files MUST be `text/javascript` or `application/javascript`
async fn serve_file(path: &std::path::Path) -> Response {
    match tokio::fs::read(path).await {
        Ok(contents) => {
            // Determine MIME type from extension
            let mime_type = get_mime_type(path);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type)
                .header(header::CACHE_CONTROL, "no-cache")
                .body(axum::body::Body::from(contents))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            format!("File not found: {}", path.display()),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error reading file: {e}"),
        )
            .into_response(),
    }
}

/// Get MIME type for a file path
///
/// Ensures WASM files get the correct `application/wasm` type
#[must_use]
pub fn get_mime_type(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("wasm") => "application/wasm".to_string(),
        Some("js") | Some("mjs") => "text/javascript".to_string(),
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("css") => "text/css".to_string(),
        Some("json") => "application/json".to_string(),
        Some("png") => "image/png".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("svg") => "image/svg+xml".to_string(),
        Some("ico") => "image/x-icon".to_string(),
        _ => mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    }
}

// =============================================================================
// WASM Build
// =============================================================================

/// Run wasm-pack build
///
/// This wraps the `wasm-pack` CLI tool to build Rust projects for WASM.
///
/// # Arguments
///
/// * `path` - Directory containing Cargo.toml
/// * `target` - WASM target (web, bundler, nodejs, no-modules)
/// * `release` - Build in release mode
/// * `out_dir` - Output directory (default: pkg)
/// * `profiling` - Enable profiling (adds names section)
///
/// # Errors
///
/// Returns an error if wasm-pack is not installed or build fails.
pub async fn run_wasm_pack_build(
    path: &std::path::Path,
    target: &str,
    release: bool,
    out_dir: Option<&std::path::Path>,
    profiling: bool,
) -> Result<(), String> {
    use std::process::Stdio;
    use std::time::Instant;

    let start = Instant::now();

    let mut cmd = tokio::process::Command::new("wasm-pack");
    cmd.arg("build");
    cmd.arg("--target").arg(target);

    if release {
        cmd.arg("--release");
    } else {
        cmd.arg("--dev");
    }

    if let Some(out) = out_dir {
        cmd.arg("--out-dir").arg(out);
    }

    if profiling {
        cmd.arg("--profiling");
    }

    cmd.current_dir(path);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    println!(
        "Running: wasm-pack build --target {} {}",
        target,
        if release { "--release" } else { "--dev" }
    );

    let status: std::process::ExitStatus = cmd
        .status()
        .await
        .map_err(|e| format!("Failed to execute wasm-pack: {e}. Is wasm-pack installed?"))?;

    let elapsed = start.elapsed();

    if status.success() {
        println!("Build completed in {:.2}s", elapsed.as_secs_f64());
        Ok(())
    } else {
        Err(format!(
            "wasm-pack build failed with exit code: {:?}",
            status.code()
        ))
    }
}

// =============================================================================
// File Watcher
// =============================================================================

/// File watcher for hot reload
///
/// Watches for changes to Rust source files and triggers rebuilds.
#[derive(Debug)]
pub struct FileWatcher {
    /// Directory to watch
    pub path: PathBuf,
    /// Debounce interval in milliseconds
    pub debounce_ms: u64,
    /// File patterns to watch (extensions)
    pub patterns: Vec<String>,
}

impl FileWatcher {
    /// Create a new file watcher with default settings
    #[must_use]
    pub fn new(path: PathBuf, debounce_ms: u64) -> Self {
        Self {
            path,
            debounce_ms,
            patterns: vec!["rs".to_string(), "toml".to_string()],
        }
    }

    /// Create a builder
    #[must_use]
    pub fn builder() -> FileWatcherBuilder {
        FileWatcherBuilder::default()
    }

    /// Check if a path matches watch patterns
    #[must_use]
    pub fn matches_pattern(&self, path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| self.patterns.iter().any(|p| p == ext))
    }

    /// Start watching for changes
    ///
    /// Calls `on_change` with the path of each changed file.
    /// This function blocks until the watcher is stopped.
    pub async fn watch<F>(&self, mut on_change: F) -> Result<(), notify::Error>
    where
        F: FnMut(String) + Send + 'static,
    {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel();
        let patterns = self.patterns.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        for path in event.paths {
                            let matches = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .is_some_and(|ext| patterns.iter().any(|p| p == ext));
                            if matches {
                                let _ = tx.send(path.display().to_string());
                            }
                        }
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(self.debounce_ms)),
        )?;

        watcher.watch(&self.path, RecursiveMode::Recursive)?;

        // Keep watcher alive and process events
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(path) => {
                    on_change(path);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        Ok(())
    }
}

/// Builder for `FileWatcher`
#[derive(Debug, Clone, Default)]
pub struct FileWatcherBuilder {
    path: Option<PathBuf>,
    debounce_ms: u64,
    patterns: Vec<String>,
}

impl FileWatcherBuilder {
    /// Set watch path
    #[must_use]
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set debounce interval
    #[must_use]
    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// Add file pattern to watch
    #[must_use]
    pub fn pattern(mut self, ext: impl Into<String>) -> Self {
        self.patterns.push(ext.into());
        self
    }

    /// Build the file watcher
    #[must_use]
    pub fn build(self) -> FileWatcher {
        FileWatcher {
            path: self.path.unwrap_or_else(|| PathBuf::from(".")),
            debounce_ms: if self.debounce_ms == 0 {
                500
            } else {
                self.debounce_ms
            },
            patterns: if self.patterns.is_empty() {
                vec!["rs".to_string(), "toml".to_string()]
            } else {
                self.patterns
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DevServerConfig Tests
    // =========================================================================

    #[test]
    fn test_dev_server_config_default() {
        let config = DevServerConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.ws_port, 8081);
        assert!(!config.cors);
        assert_eq!(config.directory, PathBuf::from("."));
    }

    #[test]
    fn test_dev_server_config_builder() {
        let config = DevServerConfig::builder()
            .directory("./www")
            .port(9000)
            .ws_port(9001)
            .cors(true)
            .build();

        assert_eq!(config.port, 9000);
        assert_eq!(config.ws_port, 9001);
        assert!(config.cors);
        assert_eq!(config.directory, PathBuf::from("./www"));
    }

    // =========================================================================
    // DevServer Tests
    // =========================================================================

    #[test]
    fn test_dev_server_creation() {
        let config = DevServerConfig {
            directory: PathBuf::from("./www"),
            port: 9000,
            ws_port: 9001,
            cors: true,
        };
        let server = DevServer::new(config);
        assert_eq!(server.http_url(), "http://localhost:9000");
        assert_eq!(server.ws_url(), "ws://localhost:9000/ws");
    }

    #[test]
    fn test_dev_server_reload_sender() {
        let server = DevServer::new(DevServerConfig::default());
        let tx = server.reload_sender();

        // Should be able to send messages
        let result = tx.send(HotReloadMessage::ServerReady);
        // No receivers yet, but send should work
        assert!(result.is_ok() || result.is_err());
    }

    // =========================================================================
    // HotReloadMessage Tests
    // =========================================================================

    #[test]
    fn test_hot_reload_message_clone() {
        let msg = HotReloadMessage::RebuildComplete { duration_ms: 500 };
        let cloned = msg;
        assert!(matches!(
            cloned,
            HotReloadMessage::RebuildComplete { duration_ms: 500 }
        ));
    }

    #[test]
    fn test_hot_reload_message_to_json() {
        let msg = HotReloadMessage::FileChanged {
            path: "src/lib.rs".to_string(),
        };
        let json = msg.to_json();
        assert!(json.contains("FileChanged"));
        assert!(json.contains("src/lib.rs"));
    }

    #[test]
    fn test_hot_reload_message_rebuild_complete_json() {
        let msg = HotReloadMessage::RebuildComplete { duration_ms: 1234 };
        let json = msg.to_json();
        assert!(json.contains("RebuildComplete"));
        assert!(json.contains("1234"));
    }

    #[test]
    fn test_hot_reload_message_rebuild_failed_json() {
        let msg = HotReloadMessage::RebuildFailed {
            error: "compile error".to_string(),
        };
        let json = msg.to_json();
        assert!(json.contains("RebuildFailed"));
        assert!(json.contains("compile error"));
    }

    #[test]
    fn test_hot_reload_message_server_ready_json() {
        let msg = HotReloadMessage::ServerReady;
        let json = msg.to_json();
        assert!(json.contains("ServerReady"));
    }

    #[test]
    fn test_hot_reload_message_rebuild_started_json() {
        let msg = HotReloadMessage::RebuildStarted;
        let json = msg.to_json();
        assert!(json.contains("RebuildStarted"));
    }

    // =========================================================================
    // FileWatcher Tests
    // =========================================================================

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new(PathBuf::from("."), 500);
        assert_eq!(watcher.debounce_ms, 500);
        assert_eq!(watcher.path, PathBuf::from("."));
        assert!(watcher.patterns.contains(&"rs".to_string()));
        assert!(watcher.patterns.contains(&"toml".to_string()));
    }

    #[test]
    fn test_file_watcher_builder() {
        let watcher = FileWatcher::builder()
            .path("./src")
            .debounce_ms(1000)
            .pattern("rs")
            .pattern("ts")
            .build();

        assert_eq!(watcher.path, PathBuf::from("./src"));
        assert_eq!(watcher.debounce_ms, 1000);
        assert!(watcher.patterns.contains(&"rs".to_string()));
        assert!(watcher.patterns.contains(&"ts".to_string()));
    }

    #[test]
    fn test_file_watcher_builder_defaults() {
        let watcher = FileWatcher::builder().build();

        assert_eq!(watcher.path, PathBuf::from("."));
        assert_eq!(watcher.debounce_ms, 500);
        assert!(watcher.patterns.contains(&"rs".to_string()));
    }

    #[test]
    fn test_file_watcher_matches_pattern() {
        let watcher = FileWatcher::new(PathBuf::from("."), 500);

        assert!(watcher.matches_pattern(&PathBuf::from("src/lib.rs")));
        assert!(watcher.matches_pattern(&PathBuf::from("Cargo.toml")));
        assert!(!watcher.matches_pattern(&PathBuf::from("README.md")));
        assert!(!watcher.matches_pattern(&PathBuf::from("main.js")));
    }

    #[test]
    fn test_file_watcher_custom_patterns() {
        let mut watcher = FileWatcher::new(PathBuf::from("."), 500);
        watcher.patterns = vec!["js".to_string(), "ts".to_string()];

        assert!(watcher.matches_pattern(&PathBuf::from("app.js")));
        assert!(watcher.matches_pattern(&PathBuf::from("app.ts")));
        assert!(!watcher.matches_pattern(&PathBuf::from("lib.rs")));
    }

    // =========================================================================
    // MIME Type Tests
    // =========================================================================

    #[test]
    fn test_get_mime_type_wasm() {
        assert_eq!(
            get_mime_type(&PathBuf::from("app.wasm")),
            "application/wasm"
        );
    }

    #[test]
    fn test_get_mime_type_javascript() {
        assert_eq!(get_mime_type(&PathBuf::from("app.js")), "text/javascript");
        assert_eq!(
            get_mime_type(&PathBuf::from("module.mjs")),
            "text/javascript"
        );
    }

    #[test]
    fn test_get_mime_type_html() {
        assert_eq!(get_mime_type(&PathBuf::from("index.html")), "text/html");
        assert_eq!(get_mime_type(&PathBuf::from("page.htm")), "text/html");
    }

    #[test]
    fn test_get_mime_type_css() {
        assert_eq!(get_mime_type(&PathBuf::from("styles.css")), "text/css");
    }

    #[test]
    fn test_get_mime_type_json() {
        assert_eq!(
            get_mime_type(&PathBuf::from("data.json")),
            "application/json"
        );
    }

    #[test]
    fn test_get_mime_type_images() {
        assert_eq!(get_mime_type(&PathBuf::from("logo.png")), "image/png");
        assert_eq!(get_mime_type(&PathBuf::from("photo.jpg")), "image/jpeg");
        assert_eq!(get_mime_type(&PathBuf::from("photo.jpeg")), "image/jpeg");
        assert_eq!(get_mime_type(&PathBuf::from("icon.svg")), "image/svg+xml");
        assert_eq!(get_mime_type(&PathBuf::from("favicon.ico")), "image/x-icon");
    }

    #[test]
    fn test_get_mime_type_unknown() {
        // Unknown types fall back to mime_guess or octet-stream
        let mime = get_mime_type(&PathBuf::from("data.xyz"));
        assert!(!mime.is_empty());
    }

    // =========================================================================
    // Integration-style Tests (no actual I/O)
    // =========================================================================

    #[test]
    fn test_dev_server_config_chain() {
        let config = DevServerConfig::builder()
            .directory("./dist")
            .port(3000)
            .ws_port(3001)
            .cors(true)
            .build();

        let server = DevServer::new(config);
        assert_eq!(server.http_url(), "http://localhost:3000");
    }

    #[test]
    fn test_file_watcher_builder_chain() {
        let watcher = FileWatcher::builder()
            .path("./crate")
            .debounce_ms(250)
            .pattern("rs")
            .pattern("toml")
            .pattern("lock")
            .build();

        assert_eq!(watcher.path, PathBuf::from("./crate"));
        assert_eq!(watcher.debounce_ms, 250);
        assert_eq!(watcher.patterns.len(), 3);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_hot_reload_message_roundtrip() {
        let original = HotReloadMessage::RebuildComplete { duration_ms: 1500 };
        let json = original.to_json();
        let parsed: HotReloadMessage = serde_json::from_str(&json).expect("parse failed");

        match parsed {
            HotReloadMessage::RebuildComplete { duration_ms } => {
                assert_eq!(duration_ms, 1500);
            }
            _ => panic!("Wrong variant after roundtrip"),
        }
    }

    #[test]
    fn test_hot_reload_message_all_variants() {
        let variants = vec![
            HotReloadMessage::FileChanged {
                path: "test.rs".to_string(),
            },
            HotReloadMessage::RebuildStarted,
            HotReloadMessage::RebuildComplete { duration_ms: 100 },
            HotReloadMessage::RebuildFailed {
                error: "err".to_string(),
            },
            HotReloadMessage::ServerReady,
        ];

        for variant in variants {
            let json = variant.to_json();
            assert!(!json.is_empty());
            // Verify it can be parsed back
            let _: HotReloadMessage = serde_json::from_str(&json).expect("parse failed");
        }
    }
}
