//! Browser control for headless testing.
//!
//! Per spec Section 6.1: Rust-native CDP (Chrome `DevTools` Protocol) implementation.
//!
//! This module provides real browser control via the Chrome `DevTools` Protocol.
//! When compiled with the `browser` feature, it uses chromiumoxide for full CDP support.
//! Without the feature, it provides a mock implementation for unit testing.

use crate::result::{ProbarError, ProbarResult};

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Run in headless mode
    pub headless: bool,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Path to chromium binary (None = auto-detect)
    pub chromium_path: Option<String>,
    /// Remote debugging port (0 = auto-assign)
    pub debug_port: u16,
    /// User agent string
    pub user_agent: Option<String>,
    /// Enable `DevTools`
    pub devtools: bool,
    /// Sandbox mode (disable for containers)
    pub sandbox: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 800,
            viewport_height: 600,
            chromium_path: None,
            debug_port: 0,
            user_agent: None,
            devtools: false,
            sandbox: true,
        }
    }
}

impl BrowserConfig {
    /// Set viewport dimensions
    #[must_use]
    pub const fn with_viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    /// Set headless mode
    #[must_use]
    pub const fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    /// Set chromium path
    #[must_use]
    pub fn with_chromium_path(mut self, path: impl Into<String>) -> Self {
        self.chromium_path = Some(path.into());
        self
    }

    /// Set user agent
    #[must_use]
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Disable sandbox (for containers/CI)
    #[must_use]
    pub const fn with_no_sandbox(mut self) -> Self {
        self.sandbox = false;
        self
    }
}

// ============================================================================
// Real CDP Implementation (when `browser` feature is enabled)
// ============================================================================

#[cfg(feature = "browser")]
#[allow(
    clippy::wildcard_imports,
    clippy::redundant_clone,
    clippy::implicit_clone,
    clippy::significant_drop_tightening,
    clippy::missing_errors_doc,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::suboptimal_flops
)]
mod cdp {
    use super::*;
    use chromiumoxide::browser::{Browser as CdpBrowser, BrowserConfig as CdpConfig};
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchTouchEventParams, DispatchTouchEventType, TouchPoint,
    };
    use chromiumoxide::cdp::browser_protocol::page::{
        CaptureScreenshotFormat, CaptureScreenshotParams,
    };
    use chromiumoxide::page::Page as CdpPage;
    use futures::StreamExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Browser instance with real CDP connection
    #[derive(Debug)]
    pub struct Browser {
        config: BrowserConfig,
        inner: Arc<Mutex<CdpBrowser>>,
        handle: tokio::task::JoinHandle<()>,
    }

    impl Browser {
        /// Launch a new browser instance with real CDP
        ///
        /// # Errors
        ///
        /// Returns error if browser cannot be launched
        pub async fn launch(config: BrowserConfig) -> ProbarResult<Self> {
            let mut builder = CdpConfig::builder();

            if config.headless {
                builder = builder.with_head();
            }

            if !config.sandbox {
                builder = builder.no_sandbox();
            }

            if let Some(ref path) = config.chromium_path {
                builder = builder.chrome_executable(path);
            }

            let cdp_config = builder
                .build()
                .map_err(|e| ProbarError::BrowserLaunchError {
                    message: e.to_string(),
                })?;

            let (browser, mut handler) = CdpBrowser::launch(cdp_config).await.map_err(|e| {
                ProbarError::BrowserLaunchError {
                    message: e.to_string(),
                }
            })?;

            // Spawn handler task
            let handle = tokio::spawn(async move {
                while let Some(h) = handler.next().await {
                    if h.is_err() {
                        break;
                    }
                }
            });

            Ok(Self {
                config,
                inner: Arc::new(Mutex::new(browser)),
                handle,
            })
        }

        /// Create a new page
        ///
        /// # Errors
        ///
        /// Returns error if page cannot be created
        pub async fn new_page(&self) -> ProbarResult<Page> {
            let browser = self.inner.lock().await;
            let cdp_page =
                browser
                    .new_page("about:blank")
                    .await
                    .map_err(|e| ProbarError::PageError {
                        message: e.to_string(),
                    })?;

            // Viewport is configured at browser launch time via window_size
            // Additional viewport emulation can be done via CDP Emulation domain if needed

            Ok(Page {
                width: self.config.viewport_width,
                height: self.config.viewport_height,
                url: String::from("about:blank"),
                wasm_ready: false,
                inner: Some(Arc::new(Mutex::new(cdp_page))),
            })
        }

        /// Get the browser configuration
        #[must_use]
        pub const fn config(&self) -> &BrowserConfig {
            &self.config
        }

        /// Check if the browser handler task is still running
        #[must_use]
        pub fn is_handler_running(&self) -> bool {
            !self.handle.is_finished()
        }

        /// Close the browser
        pub async fn close(self) -> ProbarResult<()> {
            let mut browser = self.inner.lock().await;
            browser
                .close()
                .await
                .map_err(|e| ProbarError::BrowserLaunchError {
                    message: e.to_string(),
                })?;
            Ok(())
        }
    }

    /// A browser page with real CDP connection
    #[derive(Debug)]
    pub struct Page {
        /// Page width
        pub width: u32,
        /// Page height
        pub height: u32,
        /// Current URL
        pub url: String,
        /// Whether WASM is ready
        pub wasm_ready: bool,
        /// CDP page handle
        inner: Option<Arc<Mutex<CdpPage>>>,
    }

    impl Page {
        /// Create a new mock page (for testing without browser)
        #[must_use]
        pub fn new(width: u32, height: u32) -> Self {
            Self {
                width,
                height,
                url: String::from("about:blank"),
                wasm_ready: false,
                inner: None,
            }
        }

        /// Navigate to a URL
        ///
        /// # Errors
        ///
        /// Returns error if navigation fails
        pub async fn goto(&mut self, url: &str) -> ProbarResult<()> {
            if let Some(ref inner) = self.inner {
                let page = inner.lock().await;
                page.goto(url)
                    .await
                    .map_err(|e| ProbarError::NavigationError {
                        url: url.to_string(),
                        message: e.to_string(),
                    })?;
            }
            self.url = url.to_string();
            Ok(())
        }

        /// Wait for WASM to be ready
        ///
        /// # Errors
        ///
        /// Returns error if WASM fails to initialize
        pub async fn wait_for_wasm_ready(&mut self) -> ProbarResult<()> {
            if let Some(ref inner) = self.inner {
                let page = inner.lock().await;
                // Wait for WASM module to signal readiness
                page.evaluate(
                    "new Promise(resolve => { \
                    if (window.__wasm_ready) { resolve(true); } \
                    else { window.addEventListener('wasm-ready', () => resolve(true)); } \
                })",
                )
                .await
                .map_err(|e| ProbarError::WasmError {
                    message: e.to_string(),
                })?;
            }
            self.wasm_ready = true;
            Ok(())
        }

        /// Evaluate JavaScript/WASM expression
        ///
        /// # Errors
        ///
        /// Returns error if evaluation fails
        pub async fn eval_wasm<T: serde::de::DeserializeOwned>(
            &self,
            expr: &str,
        ) -> ProbarResult<T> {
            if let Some(ref inner) = self.inner {
                let page = inner.lock().await;
                let result = page
                    .evaluate(expr)
                    .await
                    .map_err(|e| ProbarError::WasmError {
                        message: e.to_string(),
                    })?;
                result.into_value().map_err(|e| ProbarError::WasmError {
                    message: e.to_string(),
                })
            } else {
                Err(ProbarError::WasmError {
                    message: "No browser connection".to_string(),
                })
            }
        }

        /// Simulate touch input
        ///
        /// # Errors
        ///
        /// Returns error if touch simulation fails
        pub async fn touch(&self, touch: crate::Touch) -> ProbarResult<()> {
            if let Some(ref inner) = self.inner {
                let page = inner.lock().await;

                match touch.action {
                    crate::TouchAction::Tap => {
                        // Touch start
                        let start_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchStart)
                            .touch_points(vec![TouchPoint::builder()
                                .x(f64::from(touch.x))
                                .y(f64::from(touch.y))
                                .build()
                                .map_err(|e| ProbarError::InputError {
                                    message: e.to_string(),
                                })?])
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(start_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        // Touch end
                        let end_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchEnd)
                            .touch_points(Vec::<TouchPoint>::new())
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(end_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;
                    }
                    crate::TouchAction::Swipe {
                        end_x,
                        end_y,
                        duration_ms,
                    } => {
                        // Simulate swipe with multiple move events
                        let steps = 10;
                        let step_delay = duration_ms / steps;

                        // Touch start
                        let start_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchStart)
                            .touch_points(vec![TouchPoint::builder()
                                .x(f64::from(touch.x))
                                .y(f64::from(touch.y))
                                .build()
                                .map_err(|e| ProbarError::InputError {
                                    message: e.to_string(),
                                })?])
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(start_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        // Move events
                        for i in 1..=steps {
                            let progress = f32::from(i as u8) / f32::from(steps as u8);
                            let x = touch.x + (end_x - touch.x) * progress;
                            let y = touch.y + (end_y - touch.y) * progress;

                            let move_params = DispatchTouchEventParams::builder()
                                .r#type(DispatchTouchEventType::TouchMove)
                                .touch_points(vec![TouchPoint::builder()
                                    .x(f64::from(x))
                                    .y(f64::from(y))
                                    .build()
                                    .map_err(|e| ProbarError::InputError {
                                        message: e.to_string(),
                                    })?])
                                .build()
                                .map_err(|e| ProbarError::InputError {
                                    message: e.to_string(),
                                })?;

                            page.execute(move_params).await.map_err(|e| {
                                ProbarError::InputError {
                                    message: e.to_string(),
                                }
                            })?;

                            tokio::time::sleep(tokio::time::Duration::from_millis(u64::from(
                                step_delay,
                            )))
                            .await;
                        }

                        // Touch end
                        let end_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchEnd)
                            .touch_points(Vec::<TouchPoint>::new())
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(end_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;
                    }
                    crate::TouchAction::Hold { duration_ms } => {
                        // Touch start
                        let start_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchStart)
                            .touch_points(vec![TouchPoint::builder()
                                .x(f64::from(touch.x))
                                .y(f64::from(touch.y))
                                .build()
                                .map_err(|e| ProbarError::InputError {
                                    message: e.to_string(),
                                })?])
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(start_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        // Wait
                        tokio::time::sleep(tokio::time::Duration::from_millis(u64::from(
                            duration_ms,
                        )))
                        .await;

                        // Touch end
                        let end_params = DispatchTouchEventParams::builder()
                            .r#type(DispatchTouchEventType::TouchEnd)
                            .touch_points(Vec::<TouchPoint>::new())
                            .build()
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;

                        page.execute(end_params)
                            .await
                            .map_err(|e| ProbarError::InputError {
                                message: e.to_string(),
                            })?;
                    }
                }
            }
            Ok(())
        }

        /// Take a screenshot
        ///
        /// # Errors
        ///
        /// Returns error if screenshot fails
        pub async fn screenshot(&self) -> ProbarResult<Vec<u8>> {
            if let Some(ref inner) = self.inner {
                let page = inner.lock().await;
                let params = CaptureScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build();

                let screenshot =
                    page.execute(params)
                        .await
                        .map_err(|e| ProbarError::ScreenshotError {
                            message: e.to_string(),
                        })?;

                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(&screenshot.data)
                    .map_err(|e| ProbarError::ScreenshotError {
                        message: e.to_string(),
                    })
            } else {
                // Return empty PNG for mock
                Ok(vec![])
            }
        }

        /// Get current URL
        #[must_use]
        pub fn current_url(&self) -> &str {
            &self.url
        }

        /// Check if WASM is ready
        #[must_use]
        pub const fn is_wasm_ready(&self) -> bool {
            self.wasm_ready
        }
    }
}

// ============================================================================
// Mock Implementation (when `browser` feature is NOT enabled)
// ============================================================================

#[cfg(not(feature = "browser"))]
#[allow(clippy::missing_const_for_fn)]
mod mock {
    use super::{BrowserConfig, ProbarError, ProbarResult};

    /// Browser instance for testing (mock when `browser` feature disabled)
    #[derive(Debug)]
    pub struct Browser {
        config: BrowserConfig,
    }

    impl Browser {
        /// Launch a new browser instance (mock)
        ///
        /// # Errors
        ///
        /// Returns error if browser cannot be launched
        pub fn launch(config: BrowserConfig) -> ProbarResult<Self> {
            Ok(Self { config })
        }

        /// Create a new page
        ///
        /// # Errors
        ///
        /// Returns error if page cannot be created
        pub fn new_page(&self) -> ProbarResult<Page> {
            Ok(Page::new(
                self.config.viewport_width,
                self.config.viewport_height,
            ))
        }

        /// Get the browser configuration
        #[must_use]
        pub const fn config(&self) -> &BrowserConfig {
            &self.config
        }
    }

    /// A browser page for testing (mock when `browser` feature disabled)
    #[derive(Debug)]
    pub struct Page {
        /// Page width
        pub width: u32,
        /// Page height
        pub height: u32,
        /// Current URL
        pub url: String,
        /// Whether WASM is ready
        pub wasm_ready: bool,
    }

    impl Page {
        /// Create a new page
        #[must_use]
        pub fn new(width: u32, height: u32) -> Self {
            Self {
                width,
                height,
                url: String::from("about:blank"),
                wasm_ready: false,
            }
        }

        /// Navigate to a URL
        ///
        /// # Errors
        ///
        /// Returns error if navigation fails
        pub fn goto(&mut self, url: &str) -> ProbarResult<()> {
            self.url = url.to_string();
            Ok(())
        }

        /// Wait for WASM to be ready
        ///
        /// # Errors
        ///
        /// Returns error if WASM fails to initialize
        pub fn wait_for_wasm_ready(&mut self) -> ProbarResult<()> {
            self.wasm_ready = true;
            Ok(())
        }

        /// Evaluate WASM expression (mock returns error)
        ///
        /// # Errors
        ///
        /// Always returns error in mock mode
        pub fn eval_wasm<T: serde::de::DeserializeOwned>(&self, _expr: &str) -> ProbarResult<T> {
            Err(ProbarError::WasmError {
                message:
                    "Browser feature not enabled. Enable 'browser' feature for real CDP support."
                        .to_string(),
            })
        }

        /// Simulate touch input (mock does nothing)
        ///
        /// # Errors
        ///
        /// Returns Ok in mock mode
        pub fn touch(&self, _touch: crate::Touch) -> ProbarResult<()> {
            Ok(())
        }

        /// Take a screenshot (mock returns empty)
        ///
        /// # Errors
        ///
        /// Returns empty bytes in mock mode
        pub fn screenshot(&self) -> ProbarResult<Vec<u8>> {
            Ok(vec![])
        }

        /// Get current URL
        #[must_use]
        pub fn current_url(&self) -> &str {
            &self.url
        }

        /// Check if WASM is ready
        #[must_use]
        pub const fn is_wasm_ready(&self) -> bool {
            self.wasm_ready
        }
    }
}

// Re-export based on feature
#[cfg(feature = "browser")]
pub use cdp::{Browser, Page};

#[cfg(not(feature = "browser"))]
pub use mock::{Browser, Page};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    mod browser_config_tests {
        use super::*;

        #[test]
        fn test_default() {
            let config = BrowserConfig::default();
            assert!(config.headless);
            assert_eq!(config.viewport_width, 800);
            assert_eq!(config.viewport_height, 600);
            assert!(config.chromium_path.is_none());
            assert_eq!(config.debug_port, 0);
            assert!(config.user_agent.is_none());
            assert!(!config.devtools);
            assert!(config.sandbox);
        }

        #[test]
        fn test_with_viewport() {
            let config = BrowserConfig::default().with_viewport(1920, 1080);
            assert_eq!(config.viewport_width, 1920);
            assert_eq!(config.viewport_height, 1080);
        }

        #[test]
        fn test_with_headless() {
            let config = BrowserConfig::default().with_headless(false);
            assert!(!config.headless);
        }

        #[test]
        fn test_with_chromium_path() {
            let config = BrowserConfig::default().with_chromium_path("/usr/bin/chromium");
            assert_eq!(config.chromium_path, Some("/usr/bin/chromium".to_string()));
        }

        #[test]
        fn test_with_user_agent() {
            let config = BrowserConfig::default().with_user_agent("Custom UA");
            assert_eq!(config.user_agent, Some("Custom UA".to_string()));
        }

        #[test]
        fn test_with_no_sandbox() {
            let config = BrowserConfig::default().with_no_sandbox();
            assert!(!config.sandbox);
        }

        #[test]
        fn test_clone() {
            let config = BrowserConfig::default()
                .with_viewport(1024, 768)
                .with_headless(false);
            let cloned = config.clone();
            assert_eq!(config.viewport_width, cloned.viewport_width);
            assert_eq!(config.headless, cloned.headless);
        }

        #[test]
        fn test_debug() {
            let config = BrowserConfig::default();
            let debug = format!("{:?}", config);
            assert!(debug.contains("BrowserConfig"));
            assert!(debug.contains("headless"));
        }
    }

    #[cfg(not(feature = "browser"))]
    mod mock_browser_tests {
        use super::*;

        #[test]
        fn test_browser_launch() {
            let config = BrowserConfig::default();
            let browser = Browser::launch(config).unwrap();
            assert_eq!(browser.config().viewport_width, 800);
        }

        #[test]
        fn test_browser_new_page() {
            let config = BrowserConfig::default().with_viewport(1024, 768);
            let browser = Browser::launch(config).unwrap();
            let page = browser.new_page().unwrap();
            assert_eq!(page.width, 1024);
            assert_eq!(page.height, 768);
        }

        #[test]
        fn test_browser_debug() {
            let config = BrowserConfig::default();
            let browser = Browser::launch(config).unwrap();
            let debug = format!("{:?}", browser);
            assert!(debug.contains("Browser"));
        }
    }

    #[cfg(not(feature = "browser"))]
    mod mock_page_tests {
        use super::*;

        #[test]
        fn test_page_new() {
            let page = Page::new(800, 600);
            assert_eq!(page.width, 800);
            assert_eq!(page.height, 600);
            assert_eq!(page.url, "about:blank");
            assert!(!page.wasm_ready);
        }

        #[test]
        fn test_page_goto() {
            let mut page = Page::new(800, 600);
            page.goto("https://example.com").unwrap();
            assert_eq!(page.current_url(), "https://example.com");
        }

        #[test]
        fn test_page_wait_for_wasm_ready() {
            let mut page = Page::new(800, 600);
            assert!(!page.is_wasm_ready());
            page.wait_for_wasm_ready().unwrap();
            assert!(page.is_wasm_ready());
        }

        #[test]
        fn test_page_eval_wasm_error() {
            let page = Page::new(800, 600);
            let result: Result<String, _> = page.eval_wasm("test");
            assert!(result.is_err());
        }

        #[test]
        fn test_page_touch() {
            let page = Page::new(800, 600);
            let touch = crate::Touch {
                x: 100.0,
                y: 100.0,
                action: crate::TouchAction::Tap,
            };
            page.touch(touch).unwrap();
        }

        #[test]
        fn test_page_screenshot() {
            let page = Page::new(800, 600);
            let screenshot = page.screenshot().unwrap();
            assert!(screenshot.is_empty()); // Mock returns empty
        }

        #[test]
        fn test_page_debug() {
            let page = Page::new(800, 600);
            let debug = format!("{:?}", page);
            assert!(debug.contains("Page"));
        }
    }

    // =========================================================================
    // H₀ EXTREME TDD: Browser Tests (Feature F P0)
    // =========================================================================

    mod h0_browser_config_tests {
        use super::*;

        #[test]
        fn h0_browser_01_config_default_headless() {
            let config = BrowserConfig::default();
            assert!(config.headless);
        }

        #[test]
        fn h0_browser_02_config_default_viewport_width() {
            let config = BrowserConfig::default();
            assert_eq!(config.viewport_width, 800);
        }

        #[test]
        fn h0_browser_03_config_default_viewport_height() {
            let config = BrowserConfig::default();
            assert_eq!(config.viewport_height, 600);
        }

        #[test]
        fn h0_browser_04_config_default_no_chromium_path() {
            let config = BrowserConfig::default();
            assert!(config.chromium_path.is_none());
        }

        #[test]
        fn h0_browser_05_config_default_debug_port() {
            let config = BrowserConfig::default();
            assert_eq!(config.debug_port, 0);
        }

        #[test]
        fn h0_browser_06_config_default_no_user_agent() {
            let config = BrowserConfig::default();
            assert!(config.user_agent.is_none());
        }

        #[test]
        fn h0_browser_07_config_default_devtools_off() {
            let config = BrowserConfig::default();
            assert!(!config.devtools);
        }

        #[test]
        fn h0_browser_08_config_default_sandbox_on() {
            let config = BrowserConfig::default();
            assert!(config.sandbox);
        }

        #[test]
        fn h0_browser_09_config_with_viewport() {
            let config = BrowserConfig::default().with_viewport(1920, 1080);
            assert_eq!(config.viewport_width, 1920);
            assert_eq!(config.viewport_height, 1080);
        }

        #[test]
        fn h0_browser_10_config_with_headless_false() {
            let config = BrowserConfig::default().with_headless(false);
            assert!(!config.headless);
        }
    }

    mod h0_browser_config_builder_tests {
        use super::*;

        #[test]
        fn h0_browser_11_config_with_chromium_path() {
            let config = BrowserConfig::default().with_chromium_path("/path/to/chromium");
            assert_eq!(config.chromium_path, Some("/path/to/chromium".to_string()));
        }

        #[test]
        fn h0_browser_12_config_with_user_agent() {
            let config = BrowserConfig::default().with_user_agent("Test UA");
            assert_eq!(config.user_agent, Some("Test UA".to_string()));
        }

        #[test]
        fn h0_browser_13_config_with_no_sandbox() {
            let config = BrowserConfig::default().with_no_sandbox();
            assert!(!config.sandbox);
        }

        #[test]
        fn h0_browser_14_config_builder_chain() {
            let config = BrowserConfig::default()
                .with_viewport(1024, 768)
                .with_headless(false)
                .with_no_sandbox()
                .with_user_agent("Custom");
            assert_eq!(config.viewport_width, 1024);
            assert!(!config.headless);
            assert!(!config.sandbox);
            assert_eq!(config.user_agent, Some("Custom".to_string()));
        }

        #[test]
        fn h0_browser_15_config_clone() {
            let config = BrowserConfig::default().with_viewport(800, 600);
            let cloned = config;
            assert_eq!(cloned.viewport_width, 800);
        }

        #[test]
        fn h0_browser_16_config_string_conversion() {
            let config = BrowserConfig::default().with_chromium_path(String::from("/usr/bin/chrome"));
            assert!(config.chromium_path.is_some());
        }

        #[test]
        fn h0_browser_17_config_small_viewport() {
            let config = BrowserConfig::default().with_viewport(320, 240);
            assert_eq!(config.viewport_width, 320);
            assert_eq!(config.viewport_height, 240);
        }

        #[test]
        fn h0_browser_18_config_large_viewport() {
            let config = BrowserConfig::default().with_viewport(3840, 2160);
            assert_eq!(config.viewport_width, 3840);
        }

        #[test]
        fn h0_browser_19_config_debug_format() {
            let config = BrowserConfig::default();
            let debug = format!("{:?}", config);
            assert!(debug.contains("headless"));
        }

        #[test]
        fn h0_browser_20_config_user_agent_unicode() {
            let config = BrowserConfig::default().with_user_agent("UA/テスト");
            assert_eq!(config.user_agent, Some("UA/テスト".to_string()));
        }
    }

    #[cfg(not(feature = "browser"))]
    mod h0_mock_browser_tests {
        use super::*;

        #[test]
        fn h0_browser_21_launch() {
            let config = BrowserConfig::default();
            let browser = Browser::launch(config);
            assert!(browser.is_ok());
        }

        #[test]
        fn h0_browser_22_launch_config_preserved() {
            let config = BrowserConfig::default().with_viewport(1024, 768);
            let browser = Browser::launch(config).unwrap();
            assert_eq!(browser.config().viewport_width, 1024);
        }

        #[test]
        fn h0_browser_23_new_page() {
            let browser = Browser::launch(BrowserConfig::default()).unwrap();
            let page = browser.new_page();
            assert!(page.is_ok());
        }

        #[test]
        fn h0_browser_24_new_page_dimensions() {
            let config = BrowserConfig::default().with_viewport(1280, 720);
            let browser = Browser::launch(config).unwrap();
            let page = browser.new_page().unwrap();
            assert_eq!(page.width, 1280);
            assert_eq!(page.height, 720);
        }

        #[test]
        fn h0_browser_25_debug_format() {
            let browser = Browser::launch(BrowserConfig::default()).unwrap();
            let debug = format!("{:?}", browser);
            assert!(debug.contains("Browser"));
        }
    }

    #[cfg(not(feature = "browser"))]
    mod h0_mock_page_tests {
        use super::*;

        #[test]
        fn h0_browser_26_page_new() {
            let page = Page::new(800, 600);
            assert_eq!(page.width, 800);
        }

        #[test]
        fn h0_browser_27_page_initial_url() {
            let page = Page::new(800, 600);
            assert_eq!(page.url, "about:blank");
        }

        #[test]
        fn h0_browser_28_page_initial_wasm_not_ready() {
            let page = Page::new(800, 600);
            assert!(!page.wasm_ready);
        }

        #[test]
        fn h0_browser_29_page_goto() {
            let mut page = Page::new(800, 600);
            let result = page.goto("http://localhost:8080");
            assert!(result.is_ok());
        }

        #[test]
        fn h0_browser_30_page_goto_updates_url() {
            let mut page = Page::new(800, 600);
            page.goto("http://test.com").unwrap();
            assert_eq!(page.current_url(), "http://test.com");
        }

        #[test]
        fn h0_browser_31_page_wait_for_wasm() {
            let mut page = Page::new(800, 600);
            let result = page.wait_for_wasm_ready();
            assert!(result.is_ok());
        }

        #[test]
        fn h0_browser_32_page_wasm_ready_after_wait() {
            let mut page = Page::new(800, 600);
            page.wait_for_wasm_ready().unwrap();
            assert!(page.is_wasm_ready());
        }

        #[test]
        fn h0_browser_33_page_eval_wasm_fails() {
            let page = Page::new(800, 600);
            let result: Result<i32, _> = page.eval_wasm("1 + 1");
            assert!(result.is_err());
        }

        #[test]
        fn h0_browser_34_page_touch_tap() {
            let page = Page::new(800, 600);
            let touch = crate::Touch {
                x: 50.0,
                y: 50.0,
                action: crate::TouchAction::Tap,
            };
            assert!(page.touch(touch).is_ok());
        }

        #[test]
        fn h0_browser_35_page_screenshot_empty() {
            let page = Page::new(800, 600);
            let screenshot = page.screenshot().unwrap();
            assert!(screenshot.is_empty());
        }
    }

    #[cfg(not(feature = "browser"))]
    mod h0_mock_page_advanced_tests {
        use super::*;

        #[test]
        fn h0_browser_36_page_touch_swipe() {
            let page = Page::new(800, 600);
            let touch = crate::Touch {
                x: 100.0,
                y: 100.0,
                action: crate::TouchAction::Swipe {
                    end_x: 200.0,
                    end_y: 200.0,
                    duration_ms: 100,
                },
            };
            assert!(page.touch(touch).is_ok());
        }

        #[test]
        fn h0_browser_37_page_touch_hold() {
            let page = Page::new(800, 600);
            let touch = crate::Touch {
                x: 100.0,
                y: 100.0,
                action: crate::TouchAction::Hold { duration_ms: 500 },
            };
            assert!(page.touch(touch).is_ok());
        }

        #[test]
        fn h0_browser_38_page_debug() {
            let page = Page::new(800, 600);
            let debug = format!("{:?}", page);
            assert!(debug.contains("Page"));
        }

        #[test]
        fn h0_browser_39_page_current_url_method() {
            let page = Page::new(800, 600);
            assert_eq!(page.current_url(), "about:blank");
        }

        #[test]
        fn h0_browser_40_page_is_wasm_ready_method() {
            let page = Page::new(800, 600);
            assert!(!page.is_wasm_ready());
        }

        #[test]
        fn h0_browser_41_page_multiple_goto() {
            let mut page = Page::new(800, 600);
            page.goto("http://first.com").unwrap();
            page.goto("http://second.com").unwrap();
            assert_eq!(page.current_url(), "http://second.com");
        }

        #[test]
        fn h0_browser_42_page_zero_dimensions() {
            let page = Page::new(0, 0);
            assert_eq!(page.width, 0);
            assert_eq!(page.height, 0);
        }

        #[test]
        fn h0_browser_43_page_large_dimensions() {
            let page = Page::new(7680, 4320);
            assert_eq!(page.width, 7680);
        }

        #[test]
        fn h0_browser_44_config_overwrite_viewport() {
            let config = BrowserConfig::default()
                .with_viewport(800, 600)
                .with_viewport(1024, 768);
            assert_eq!(config.viewport_width, 1024);
        }

        #[test]
        fn h0_browser_45_config_overwrite_headless() {
            let config = BrowserConfig::default()
                .with_headless(false)
                .with_headless(true);
            assert!(config.headless);
        }
    }

    mod h0_browser_edge_cases {
        use super::*;

        #[test]
        fn h0_browser_46_config_empty_chromium_path() {
            let config = BrowserConfig::default().with_chromium_path("");
            assert_eq!(config.chromium_path, Some(String::new()));
        }

        #[test]
        fn h0_browser_47_config_empty_user_agent() {
            let config = BrowserConfig::default().with_user_agent("");
            assert_eq!(config.user_agent, Some(String::new()));
        }

        #[test]
        fn h0_browser_48_config_viewport_square() {
            let config = BrowserConfig::default().with_viewport(1000, 1000);
            assert_eq!(config.viewport_width, config.viewport_height);
        }

        #[test]
        fn h0_browser_49_config_viewport_portrait() {
            let config = BrowserConfig::default().with_viewport(600, 800);
            assert!(config.viewport_height > config.viewport_width);
        }

        #[test]
        fn h0_browser_50_config_viewport_landscape() {
            let config = BrowserConfig::default().with_viewport(1920, 1080);
            assert!(config.viewport_width > config.viewport_height);
        }
    }
}
