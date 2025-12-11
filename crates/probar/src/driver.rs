//! ProbarDriver - Abstract Browser Automation Trait
//!
//! Per spec Section 3.2: Full Chromium automation via abstract `ProbarDriver` trait.
//!
//! # Architecture (from spec)
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │  ProbarDriver (Abstract Trait)                                             │
//! │  Genchi Genbutsu: "Go and see" - allows swapping implementations          │
//! ├───────────────────────────────────────────────────────────────────────────┤
//! │                                                                           │
//! │  ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────┐  │
//! │  │  ChromiumDriver     │  │  PlaywrightBridge   │  │  WasmBindgen    │  │
//! │  │  (Default)          │  │  (Fallback)         │  │  (Unit Tests)   │  │
//! │  │                     │  │                     │  │                 │  │
//! │  │  Uses CDP via       │  │  WebSocket to       │  │  In-browser     │  │
//! │  │  chromiumoxide      │  │  Playwright server  │  │  test runner    │  │
//! │  └─────────────────────┘  └─────────────────────┘  └─────────────────┘  │
//! │                                                                           │
//! │  Toyota Principle: Driver Abstraction protects against API instability   │
//! └───────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Toyota Principles Applied
//!
//! - **Genchi Genbutsu**: Abstract trait allows "going and seeing" with different browsers
//! - **Risk Mitigation**: If chromiumoxide becomes unmaintained, swap to PlaywrightBridge

#[cfg(feature = "browser")]
use crate::event::InputEvent;
use crate::locator::BoundingBox;
#[cfg(feature = "browser")]
use crate::result::{ProbarError, ProbarResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "browser")]
use async_trait::async_trait;

/// Element handle for DOM interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementHandle {
    /// Unique identifier for the element
    pub id: String,
    /// Element tag name
    pub tag_name: String,
    /// Element text content
    pub text_content: Option<String>,
    /// Bounding box if visible
    pub bounding_box: Option<BoundingBox>,
}

impl ElementHandle {
    /// Create a new element handle
    #[must_use]
    pub fn new(id: impl Into<String>, tag_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tag_name: tag_name.into(),
            text_content: None,
            bounding_box: None,
        }
    }

    /// Check if element is visible
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.bounding_box.is_some()
    }
}

/// Page performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageMetrics {
    /// Time to first paint (ms)
    pub first_paint_ms: Option<f64>,
    /// Time to first contentful paint (ms)
    pub first_contentful_paint_ms: Option<f64>,
    /// DOM content loaded time (ms)
    pub dom_content_loaded_ms: Option<f64>,
    /// Full page load time (ms)
    pub load_time_ms: Option<f64>,
    /// Total JavaScript heap size (bytes)
    pub js_heap_size_bytes: Option<u64>,
    /// Used JavaScript heap size (bytes)
    pub js_heap_used_bytes: Option<u64>,
    /// Number of DOM nodes
    pub dom_nodes: Option<u32>,
    /// Number of frames
    pub frame_count: Option<u32>,
}

/// Screenshot data with metadata
#[derive(Debug, Clone)]
pub struct Screenshot {
    /// Raw PNG data
    pub data: Vec<u8>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Device pixel ratio
    pub device_pixel_ratio: f64,
    /// Timestamp when screenshot was taken
    pub timestamp: std::time::SystemTime,
}

impl Screenshot {
    /// Create a new screenshot
    #[must_use]
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            device_pixel_ratio: 1.0,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Get the size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Check if screenshot is valid (has data)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.data.is_empty() && self.width > 0 && self.height > 0
    }
}

/// Network request interceptor configuration
#[derive(Debug, Clone, Default)]
pub struct NetworkInterceptor {
    /// URL patterns to intercept
    pub patterns: Vec<String>,
    /// Whether to block matching requests
    pub block: bool,
    /// Response to return (if overriding)
    pub response_override: Option<NetworkResponse>,
}

/// Mock network response
#[derive(Debug, Clone)]
pub struct NetworkResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Vec<u8>,
}

impl NetworkResponse {
    /// Create a JSON response
    #[must_use]
    pub fn json(status: u16, body: impl Serialize) -> Self {
        let body = serde_json::to_vec(&body).unwrap_or_default();
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body,
        }
    }

    /// Create a 404 response
    #[must_use]
    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: vec![],
            body: b"Not Found".to_vec(),
        }
    }
}

/// Browser configuration for driver
#[derive(Debug, Clone)]
pub struct DriverConfig {
    /// Run in headless mode
    pub headless: bool,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Device scale factor
    pub device_scale_factor: f64,
    /// User agent string
    pub user_agent: Option<String>,
    /// Timeout for navigation
    pub navigation_timeout: Duration,
    /// Timeout for element queries
    pub element_timeout: Duration,
    /// Enable tracing
    pub tracing: bool,
    /// Executable path override
    pub executable_path: Option<String>,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1920,
            viewport_height: 1080,
            device_scale_factor: 1.0,
            user_agent: None,
            navigation_timeout: Duration::from_secs(30),
            element_timeout: Duration::from_secs(5),
            tracing: false,
            executable_path: None,
        }
    }
}

impl DriverConfig {
    /// Create new config with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set headless mode
    #[must_use]
    pub const fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    /// Set viewport dimensions
    #[must_use]
    pub const fn viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    /// Set device scale factor
    #[must_use]
    pub const fn scale_factor(mut self, factor: f64) -> Self {
        self.device_scale_factor = factor;
        self
    }

    /// Set user agent
    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set navigation timeout
    #[must_use]
    pub const fn navigation_timeout(mut self, timeout: Duration) -> Self {
        self.navigation_timeout = timeout;
        self
    }

    /// Enable tracing
    #[must_use]
    pub const fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing = enabled;
        self
    }
}

/// Mobile device descriptor for emulation
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    /// Device name
    pub name: &'static str,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Device scale factor
    pub device_scale_factor: f64,
    /// Is mobile device
    pub is_mobile: bool,
    /// Has touch support
    pub has_touch: bool,
    /// Default user agent
    pub user_agent: &'static str,
}

impl DeviceDescriptor {
    /// iPhone 14 Pro
    pub const IPHONE_14_PRO: Self = Self {
        name: "iPhone 14 Pro",
        viewport_width: 393,
        viewport_height: 852,
        device_scale_factor: 3.0,
        is_mobile: true,
        has_touch: true,
        user_agent: "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15",
    };

    /// iPad Pro 12.9
    pub const IPAD_PRO_12_9: Self = Self {
        name: "iPad Pro 12.9",
        viewport_width: 1024,
        viewport_height: 1366,
        device_scale_factor: 2.0,
        is_mobile: true,
        has_touch: true,
        user_agent: "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15",
    };

    /// Desktop 1080p
    pub const DESKTOP_1080P: Self = Self {
        name: "Desktop 1080p",
        viewport_width: 1920,
        viewport_height: 1080,
        device_scale_factor: 1.0,
        is_mobile: false,
        has_touch: false,
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0",
    };

    /// Desktop 4K
    pub const DESKTOP_4K: Self = Self {
        name: "Desktop 4K",
        viewport_width: 3840,
        viewport_height: 2160,
        device_scale_factor: 2.0,
        is_mobile: false,
        has_touch: false,
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0",
    };

    /// Convert to driver config
    #[must_use]
    pub fn to_config(&self) -> DriverConfig {
        DriverConfig::default()
            .viewport(self.viewport_width, self.viewport_height)
            .scale_factor(self.device_scale_factor)
            .user_agent(self.user_agent)
    }
}

/// Abstract driver trait for browser automation
///
/// This trait allows swapping implementations (Genchi Genbutsu principle).
/// If chromiumoxide becomes unmaintained, implement `PlaywrightBridgeDriver`.
///
/// # Implementations
///
/// - `ChromiumDriver` - Default, uses chromiumoxide crate
/// - `PlaywrightBridgeDriver` - Fallback, WebSocket to Playwright server
/// - `MockDriver` - For unit testing
#[cfg(feature = "browser")]
#[async_trait]
pub trait ProbarDriver: Send + Sync {
    /// Navigate to URL
    async fn navigate(&mut self, url: &str) -> ProbarResult<()>;

    /// Take screenshot
    async fn screenshot(&self) -> ProbarResult<Screenshot>;

    /// Execute JavaScript in page context
    async fn execute_js(&self, script: &str) -> ProbarResult<serde_json::Value>;

    /// Query DOM element by selector
    async fn query_selector(&self, selector: &str) -> ProbarResult<Option<ElementHandle>>;

    /// Query all matching elements
    async fn query_selector_all(&self, selector: &str) -> ProbarResult<Vec<ElementHandle>>;

    /// Dispatch input event
    async fn dispatch_input(&self, event: InputEvent) -> ProbarResult<()>;

    /// Click element
    async fn click(&self, selector: &str) -> ProbarResult<()>;

    /// Type text into element
    async fn type_text(&self, selector: &str, text: &str) -> ProbarResult<()>;

    /// Wait for selector to appear
    async fn wait_for_selector(
        &self,
        selector: &str,
        timeout: Duration,
    ) -> ProbarResult<ElementHandle>;

    /// Get page metrics
    async fn metrics(&self) -> ProbarResult<PageMetrics>;

    /// Set network interceptor
    async fn set_network_interceptor(
        &mut self,
        interceptor: NetworkInterceptor,
    ) -> ProbarResult<()>;

    /// Get current URL
    async fn current_url(&self) -> ProbarResult<String>;

    /// Go back in history
    async fn go_back(&mut self) -> ProbarResult<()>;

    /// Go forward in history
    async fn go_forward(&mut self) -> ProbarResult<()>;

    /// Reload page
    async fn reload(&mut self) -> ProbarResult<()>;

    /// Close the browser
    async fn close(&mut self) -> ProbarResult<()>;
}

/// Mock driver for unit testing
#[derive(Debug, Default)]
pub struct MockDriver {
    /// Current URL
    pub current_url: String,
    /// Mock elements
    pub elements: Vec<ElementHandle>,
    /// JS execution results
    pub js_results: Vec<serde_json::Value>,
    /// Screenshot data
    pub screenshot_data: Option<Screenshot>,
    /// Call history for verification
    pub call_history: Vec<String>,
}

impl MockDriver {
    /// Create new mock driver
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mock element
    pub fn add_element(&mut self, element: ElementHandle) {
        self.elements.push(element);
    }

    /// Set mock JS result
    pub fn set_js_result(&mut self, result: serde_json::Value) {
        self.js_results.push(result);
    }

    /// Set mock screenshot
    pub fn set_screenshot(&mut self, screenshot: Screenshot) {
        self.screenshot_data = Some(screenshot);
    }

    /// Get call history
    #[must_use]
    pub fn history(&self) -> &[String] {
        &self.call_history
    }

    /// Check if method was called
    #[must_use]
    pub fn was_called(&self, method: &str) -> bool {
        self.call_history.iter().any(|c| c.starts_with(method))
    }
}

#[cfg(feature = "browser")]
#[async_trait]
impl ProbarDriver for MockDriver {
    async fn navigate(&mut self, url: &str) -> ProbarResult<()> {
        self.call_history.push(format!("navigate:{url}"));
        self.current_url = url.to_string();
        Ok(())
    }

    async fn screenshot(&self) -> ProbarResult<Screenshot> {
        self.screenshot_data
            .clone()
            .ok_or_else(|| ProbarError::ScreenshotError {
                message: "No mock screenshot set".to_string(),
            })
    }

    async fn execute_js(&self, script: &str) -> ProbarResult<serde_json::Value> {
        let _ = script; // Unused in mock
        self.js_results
            .first()
            .cloned()
            .ok_or_else(|| ProbarError::WasmError {
                message: "No mock JS result set".to_string(),
            })
    }

    async fn query_selector(&self, selector: &str) -> ProbarResult<Option<ElementHandle>> {
        Ok(self.elements.iter().find(|e| e.id == selector).cloned())
    }

    async fn query_selector_all(&self, _selector: &str) -> ProbarResult<Vec<ElementHandle>> {
        Ok(self.elements.clone())
    }

    async fn dispatch_input(&self, event: InputEvent) -> ProbarResult<()> {
        let _ = event; // Mock just records the call
        Ok(())
    }

    async fn click(&self, selector: &str) -> ProbarResult<()> {
        let _ = selector;
        Ok(())
    }

    async fn type_text(&self, selector: &str, text: &str) -> ProbarResult<()> {
        let _ = (selector, text);
        Ok(())
    }

    async fn wait_for_selector(
        &self,
        selector: &str,
        _timeout: Duration,
    ) -> ProbarResult<ElementHandle> {
        self.elements
            .iter()
            .find(|e| e.id == selector)
            .cloned()
            .ok_or_else(|| ProbarError::Timeout { ms: 5000 })
    }

    async fn metrics(&self) -> ProbarResult<PageMetrics> {
        Ok(PageMetrics::default())
    }

    async fn set_network_interceptor(
        &mut self,
        _interceptor: NetworkInterceptor,
    ) -> ProbarResult<()> {
        Ok(())
    }

    async fn current_url(&self) -> ProbarResult<String> {
        Ok(self.current_url.clone())
    }

    async fn go_back(&mut self) -> ProbarResult<()> {
        self.call_history.push("go_back".to_string());
        Ok(())
    }

    async fn go_forward(&mut self) -> ProbarResult<()> {
        self.call_history.push("go_forward".to_string());
        Ok(())
    }

    async fn reload(&mut self) -> ProbarResult<()> {
        self.call_history.push("reload".to_string());
        Ok(())
    }

    async fn close(&mut self) -> ProbarResult<()> {
        self.call_history.push("close".to_string());
        Ok(())
    }
}

/// Browser controller using abstract driver
///
/// This is the main entry point for browser automation tests.
///
/// # Example
///
/// ```ignore
/// let mut browser = BrowserController::<ChromiumDriver>::launch(
///     DriverConfig::default().headless(true)
/// ).await?;
///
/// browser.goto("http://localhost:8080/game").await?;
/// let screenshot = browser.screenshot().await?;
/// ```
#[cfg(feature = "browser")]
pub struct BrowserController<D: ProbarDriver> {
    driver: D,
    config: DriverConfig,
}

#[cfg(feature = "browser")]
impl<D: ProbarDriver> BrowserController<D> {
    /// Create controller with existing driver
    pub fn new(driver: D, config: DriverConfig) -> Self {
        Self { driver, config }
    }

    /// Navigate to URL
    pub async fn goto(&mut self, url: &str) -> ProbarResult<()> {
        self.driver.navigate(url).await
    }

    /// Take screenshot
    pub async fn screenshot(&self) -> ProbarResult<Screenshot> {
        self.driver.screenshot().await
    }

    /// Execute JavaScript
    pub async fn evaluate(&self, script: &str) -> ProbarResult<serde_json::Value> {
        self.driver.execute_js(script).await
    }

    /// Query element
    pub async fn query(&self, selector: &str) -> ProbarResult<Option<ElementHandle>> {
        self.driver.query_selector(selector).await
    }

    /// Get page metrics
    pub async fn metrics(&self) -> ProbarResult<PageMetrics> {
        self.driver.metrics().await
    }

    /// Get configuration
    #[must_use]
    pub const fn config(&self) -> &DriverConfig {
        &self.config
    }

    /// Close browser
    pub async fn close(&mut self) -> ProbarResult<()> {
        self.driver.close().await
    }
}

// ============================================================================
// EXTREME TDD: Tests written FIRST per spec Section 6.1
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod element_handle_tests {
        use super::*;

        #[test]
        fn test_element_handle_creation() {
            let elem = ElementHandle::new("btn-1", "button");
            assert_eq!(elem.id, "btn-1");
            assert_eq!(elem.tag_name, "button");
            assert!(elem.text_content.is_none());
        }

        #[test]
        fn test_element_handle_visibility() {
            let mut elem = ElementHandle::new("elem", "div");
            assert!(!elem.is_visible());

            elem.bounding_box = Some(BoundingBox::new(0.0, 0.0, 100.0, 100.0));
            assert!(elem.is_visible());
        }
    }

    mod screenshot_tests {
        use super::*;

        #[test]
        fn test_screenshot_creation() {
            let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
            let screenshot = Screenshot::new(data.clone(), 100, 100);
            assert_eq!(screenshot.width, 100);
            assert_eq!(screenshot.height, 100);
            assert_eq!(screenshot.data, data);
        }

        #[test]
        fn test_screenshot_size_bytes() {
            let screenshot = Screenshot::new(vec![0; 1024], 100, 100);
            assert_eq!(screenshot.size_bytes(), 1024);
        }

        #[test]
        fn test_screenshot_is_valid() {
            let valid = Screenshot::new(vec![1, 2, 3], 100, 100);
            assert!(valid.is_valid());

            let empty = Screenshot::new(vec![], 100, 100);
            assert!(!empty.is_valid());

            let zero_width = Screenshot::new(vec![1], 0, 100);
            assert!(!zero_width.is_valid());
        }
    }

    mod network_response_tests {
        use super::*;

        #[test]
        fn test_json_response() {
            let response = NetworkResponse::json(200, serde_json::json!({"ok": true}));
            assert_eq!(response.status, 200);
            assert!(response
                .headers
                .iter()
                .any(|(k, v)| k == "Content-Type" && v.contains("json")));
        }

        #[test]
        fn test_not_found_response() {
            let response = NetworkResponse::not_found();
            assert_eq!(response.status, 404);
        }
    }

    mod driver_config_tests {
        use super::*;

        #[test]
        fn test_config_default() {
            let config = DriverConfig::default();
            assert!(config.headless);
            assert_eq!(config.viewport_width, 1920);
            assert_eq!(config.viewport_height, 1080);
        }

        #[test]
        fn test_config_builder() {
            let config = DriverConfig::new()
                .headless(false)
                .viewport(800, 600)
                .scale_factor(2.0)
                .user_agent("test-agent");

            assert!(!config.headless);
            assert_eq!(config.viewport_width, 800);
            assert_eq!(config.viewport_height, 600);
            assert!((config.device_scale_factor - 2.0).abs() < f64::EPSILON);
            assert_eq!(config.user_agent, Some("test-agent".to_string()));
        }

        #[test]
        fn test_config_with_tracing() {
            let config = DriverConfig::new().with_tracing(true);
            assert!(config.tracing);
        }
    }

    mod device_descriptor_tests {
        use super::*;

        #[test]
        fn test_iphone_14_pro() {
            let device = DeviceDescriptor::IPHONE_14_PRO;
            assert_eq!(device.viewport_width, 393);
            assert!(device.is_mobile);
            assert!(device.has_touch);
        }

        #[test]
        fn test_desktop_1080p() {
            let device = DeviceDescriptor::DESKTOP_1080P;
            assert_eq!(device.viewport_width, 1920);
            assert!(!device.is_mobile);
            assert!(!device.has_touch);
        }

        #[test]
        fn test_device_to_config() {
            let config = DeviceDescriptor::IPHONE_14_PRO.to_config();
            assert_eq!(config.viewport_width, 393);
            assert_eq!(config.viewport_height, 852);
        }
    }

    mod mock_driver_tests {
        use super::*;

        #[test]
        fn test_mock_driver_creation() {
            let driver = MockDriver::new();
            assert!(driver.elements.is_empty());
            assert!(driver.current_url.is_empty());
        }

        #[test]
        fn test_mock_driver_add_element() {
            let mut driver = MockDriver::new();
            driver.add_element(ElementHandle::new("test", "div"));
            assert_eq!(driver.elements.len(), 1);
        }

        #[test]
        fn test_mock_driver_history() {
            let driver = MockDriver::new();
            assert!(driver.history().is_empty());
        }

        #[test]
        fn test_mock_driver_was_called() {
            let driver = MockDriver::new();
            assert!(!driver.was_called("navigate"));
        }
    }

    mod page_metrics_tests {
        use super::*;

        #[test]
        fn test_metrics_default() {
            let metrics = PageMetrics::default();
            assert!(metrics.first_paint_ms.is_none());
            assert!(metrics.dom_nodes.is_none());
        }
    }

    #[cfg(feature = "browser")]
    mod async_driver_tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_driver_navigate() {
            let mut driver = MockDriver::new();
            driver.navigate("https://example.com").await.unwrap();
            assert_eq!(driver.current_url, "https://example.com");
            assert!(driver.was_called("navigate"));
        }

        #[tokio::test]
        async fn test_mock_driver_history_tracking() {
            let mut driver = MockDriver::new();
            driver.go_back().await.unwrap();
            driver.go_forward().await.unwrap();
            driver.reload().await.unwrap();
            driver.close().await.unwrap();

            assert!(driver.was_called("go_back"));
            assert!(driver.was_called("go_forward"));
            assert!(driver.was_called("reload"));
            assert!(driver.was_called("close"));
        }

        #[tokio::test]
        async fn test_browser_controller_with_mock() {
            let driver = MockDriver::new();
            let mut controller = BrowserController::new(driver, DriverConfig::default());

            controller.goto("https://test.com").await.unwrap();
            let url = controller.driver.current_url().await.unwrap();
            assert_eq!(url, "https://test.com");
        }
    }
}
