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
        #[allow(dead_code)]
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
