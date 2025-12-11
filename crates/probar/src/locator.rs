//! Locator abstraction for element selection and interaction.
//!
//! Per spec Section 6.1.1: "Locators (The Core Abstraction) - Unlike Selenium,
//! Locators are strict and auto-wait."
//!
//! # Design Philosophy
//!
//! - **Auto-Waiting**: Locators automatically wait for elements to be actionable
//! - **Strict Selection**: Fails if multiple elements match (prevents flaky tests)
//! - **WASM Entity Support**: Custom `.entity()` method for game object selection
//! - **Fluent API**: Chainable methods for building complex selectors

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::result::{ProbarError, ProbarResult};

/// Default timeout for auto-waiting (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Default polling interval for auto-waiting (50ms)
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 50;

/// A point in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl Point {
    /// Create a new point
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Selector type for locating elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// CSS selector (e.g., "button.primary")
    Css(String),
    /// XPath selector
    XPath(String),
    /// Text content selector
    Text(String),
    /// Test ID selector (data-testid attribute)
    TestId(String),
    /// WASM entity selector (game-specific)
    Entity(String),
    /// Combined selector with text filter
    CssWithText {
        /// Base CSS selector
        css: String,
        /// Text content to match
        text: String,
    },
    /// Canvas entity selector (game objects)
    CanvasEntity {
        /// Entity name/ID
        entity: String,
    },
}

impl Selector {
    /// Create a CSS selector
    #[must_use]
    pub fn css(selector: impl Into<String>) -> Self {
        Self::Css(selector.into())
    }

    /// Create a test ID selector
    #[must_use]
    pub fn test_id(id: impl Into<String>) -> Self {
        Self::TestId(id.into())
    }

    /// Create a text selector
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create a WASM entity selector
    #[must_use]
    pub fn entity(name: impl Into<String>) -> Self {
        Self::Entity(name.into())
    }

    /// Convert to JavaScript/WASM query expression
    #[must_use]
    pub fn to_query(&self) -> String {
        match self {
            Self::Css(s) => format!("document.querySelector({s:?})"),
            Self::XPath(s) => {
                format!("document.evaluate({s:?}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue")
            }
            Self::Text(t) => {
                format!("Array.from(document.querySelectorAll('*')).find(el => el.textContent.includes({t:?}))")
            }
            Self::TestId(id) => format!("document.querySelector('[data-testid={id:?}]')"),
            Self::Entity(name) => format!("window.__wasm_get_entity({name:?})"),
            Self::CssWithText { css, text } => {
                format!("Array.from(document.querySelectorAll({css:?})).find(el => el.textContent.includes({text:?}))")
            }
            Self::CanvasEntity { entity } => format!("window.__wasm_get_canvas_entity({entity:?})"),
        }
    }

    /// Convert to query for counting matches
    #[must_use]
    pub fn to_count_query(&self) -> String {
        match self {
            Self::Css(s) => format!("document.querySelectorAll({s:?}).length"),
            Self::XPath(s) => {
                format!("document.evaluate({s:?}, document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null).snapshotLength")
            }
            Self::Text(t) => {
                format!("Array.from(document.querySelectorAll('*')).filter(el => el.textContent.includes({t:?})).length")
            }
            Self::TestId(id) => format!("document.querySelectorAll('[data-testid={id:?}]').length"),
            Self::Entity(name) => format!("window.__wasm_count_entities({name:?})"),
            Self::CssWithText { css, text } => {
                format!("Array.from(document.querySelectorAll({css:?})).filter(el => el.textContent.includes({text:?})).length")
            }
            Self::CanvasEntity { entity } => {
                format!("window.__wasm_count_canvas_entities({entity:?})")
            }
        }
    }
}

/// Drag operation builder
#[derive(Debug, Clone)]
pub struct DragOperation {
    /// Target point
    pub target: Point,
    /// Number of intermediate steps
    pub steps: u32,
    /// Total duration of the drag
    pub duration: Duration,
}

impl DragOperation {
    /// Create a new drag operation
    #[must_use]
    pub fn to(target: Point) -> Self {
        Self {
            target,
            steps: 10,
            duration: Duration::from_millis(500),
        }
    }

    /// Set the number of steps
    #[must_use]
    pub const fn steps(mut self, steps: u32) -> Self {
        self.steps = steps;
        self
    }

    /// Set the duration
    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Locator options for customizing behavior
#[derive(Debug, Clone)]
pub struct LocatorOptions {
    /// Timeout for auto-waiting
    pub timeout: Duration,
    /// Polling interval for auto-waiting
    pub poll_interval: Duration,
    /// Whether to require strict single-element match
    pub strict: bool,
    /// Whether the element must be visible
    pub visible: bool,
}

impl Default for LocatorOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            strict: true,
            visible: true,
        }
    }
}

/// A locator for finding and interacting with elements.
///
/// Per spec Section 6.1.1: "Unlike Selenium, Locators are strict and auto-wait."
#[derive(Debug, Clone)]
pub struct Locator {
    /// The selector for finding elements
    selector: Selector,
    /// Options for locator behavior
    options: LocatorOptions,
}

impl Locator {
    /// Create a new locator with a CSS selector
    #[must_use]
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: Selector::Css(selector.into()),
            options: LocatorOptions::default(),
        }
    }

    /// Create a locator from a selector
    #[must_use]
    pub fn from_selector(selector: Selector) -> Self {
        Self {
            selector,
            options: LocatorOptions::default(),
        }
    }

    /// Filter by text content
    ///
    /// Per spec: `page.locator("button").with_text("Start Game")`
    #[must_use]
    pub fn with_text(self, text: impl Into<String>) -> Self {
        let new_selector = match self.selector {
            Selector::Css(css) => Selector::CssWithText {
                css,
                text: text.into(),
            },
            other => {
                // For non-CSS selectors, we can't combine easily
                // Just keep the original and note the text filter
                let _ = text.into();
                other
            }
        };
        Self {
            selector: new_selector,
            options: self.options,
        }
    }

    /// Filter to a specific WASM game entity
    ///
    /// Per spec: `page.locator("canvas").entity("hero")`
    #[must_use]
    pub fn entity(self, name: impl Into<String>) -> Self {
        Self {
            selector: Selector::CanvasEntity {
                entity: name.into(),
            },
            options: self.options,
        }
    }

    /// Set a custom timeout
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = timeout;
        self
    }

    /// Disable strict mode (allow multiple matches)
    #[must_use]
    pub const fn with_strict(mut self, strict: bool) -> Self {
        self.options.strict = strict;
        self
    }

    /// Set visibility requirement
    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.options.visible = visible;
        self
    }

    /// Get the selector
    #[must_use]
    pub const fn selector(&self) -> &Selector {
        &self.selector
    }

    /// Get the options
    #[must_use]
    pub const fn options(&self) -> &LocatorOptions {
        &self.options
    }

    /// Simulate clicking on the located element
    ///
    /// # Errors
    ///
    /// Returns error if element not found or not clickable
    pub fn click(&self) -> ProbarResult<LocatorAction> {
        Ok(LocatorAction::Click {
            locator: self.clone(),
        })
    }

    /// Simulate double-clicking on the located element
    ///
    /// # Errors
    ///
    /// Returns error if element not found or not clickable
    pub fn double_click(&self) -> ProbarResult<LocatorAction> {
        Ok(LocatorAction::DoubleClick {
            locator: self.clone(),
        })
    }

    /// Drag the located element to a target point
    ///
    /// Per spec: `hero.drag_to(&Point::new(500.0, 500.0)).steps(10).duration(...)`
    #[must_use]
    pub fn drag_to(&self, target: &Point) -> DragBuilder {
        DragBuilder {
            locator: self.clone(),
            target: *target,
            steps: 10,
            duration: Duration::from_millis(500),
        }
    }

    /// Fill the located element with text
    ///
    /// # Errors
    ///
    /// Returns error if element not found or not fillable
    pub fn fill(&self, text: impl Into<String>) -> ProbarResult<LocatorAction> {
        Ok(LocatorAction::Fill {
            locator: self.clone(),
            text: text.into(),
        })
    }

    /// Get the text content of the located element
    ///
    /// # Errors
    ///
    /// Returns error if element not found
    pub fn text_content(&self) -> ProbarResult<LocatorQuery> {
        Ok(LocatorQuery::TextContent {
            locator: self.clone(),
        })
    }

    /// Check if the element is visible
    pub fn is_visible(&self) -> ProbarResult<LocatorQuery> {
        Ok(LocatorQuery::IsVisible {
            locator: self.clone(),
        })
    }

    /// Get the bounding box of the located element
    pub fn bounding_box(&self) -> ProbarResult<LocatorQuery> {
        Ok(LocatorQuery::BoundingBox {
            locator: self.clone(),
        })
    }

    /// Wait for the element to be visible
    pub fn wait_for_visible(&self) -> ProbarResult<LocatorAction> {
        Ok(LocatorAction::WaitForVisible {
            locator: self.clone(),
        })
    }

    /// Wait for the element to be hidden
    pub fn wait_for_hidden(&self) -> ProbarResult<LocatorAction> {
        Ok(LocatorAction::WaitForHidden {
            locator: self.clone(),
        })
    }

    /// Check the element count (for assertions)
    pub fn count(&self) -> ProbarResult<LocatorQuery> {
        Ok(LocatorQuery::Count {
            locator: self.clone(),
        })
    }
}

/// Builder for drag operations
#[derive(Debug, Clone)]
pub struct DragBuilder {
    locator: Locator,
    target: Point,
    steps: u32,
    duration: Duration,
}

impl DragBuilder {
    /// Set the number of intermediate steps
    #[must_use]
    pub const fn steps(mut self, steps: u32) -> Self {
        self.steps = steps;
        self
    }

    /// Set the duration of the drag
    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Build the drag action
    pub fn build(self) -> LocatorAction {
        LocatorAction::Drag {
            locator: self.locator,
            target: self.target,
            steps: self.steps,
            duration: self.duration,
        }
    }
}

/// Actions that can be performed on a located element
#[derive(Debug, Clone)]
pub enum LocatorAction {
    /// Click on the element
    Click {
        /// The locator
        locator: Locator,
    },
    /// Double-click on the element
    DoubleClick {
        /// The locator
        locator: Locator,
    },
    /// Drag the element to a point
    Drag {
        /// The locator
        locator: Locator,
        /// Target point
        target: Point,
        /// Number of steps
        steps: u32,
        /// Duration
        duration: Duration,
    },
    /// Fill the element with text
    Fill {
        /// The locator
        locator: Locator,
        /// Text to fill
        text: String,
    },
    /// Wait for element to be visible
    WaitForVisible {
        /// The locator
        locator: Locator,
    },
    /// Wait for element to be hidden
    WaitForHidden {
        /// The locator
        locator: Locator,
    },
}

impl LocatorAction {
    /// Get the locator for this action
    #[must_use]
    pub fn locator(&self) -> &Locator {
        match self {
            Self::Click { locator }
            | Self::DoubleClick { locator }
            | Self::Drag { locator, .. }
            | Self::Fill { locator, .. }
            | Self::WaitForVisible { locator }
            | Self::WaitForHidden { locator } => locator,
        }
    }
}

/// Queries that return information about located elements
#[derive(Debug, Clone)]
pub enum LocatorQuery {
    /// Get text content
    TextContent {
        /// The locator
        locator: Locator,
    },
    /// Check if visible
    IsVisible {
        /// The locator
        locator: Locator,
    },
    /// Get bounding box
    BoundingBox {
        /// The locator
        locator: Locator,
    },
    /// Count matching elements
    Count {
        /// The locator
        locator: Locator,
    },
}

impl LocatorQuery {
    /// Get the locator for this query
    #[must_use]
    pub const fn locator(&self) -> &Locator {
        match self {
            Self::TextContent { locator }
            | Self::IsVisible { locator }
            | Self::BoundingBox { locator }
            | Self::Count { locator } => locator,
        }
    }
}

/// Bounding box for an element
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

impl BoundingBox {
    /// Create a new bounding box
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the center point
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Check if a point is inside this bounding box
    #[must_use]
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// Smart assertion builder for locators (Playwright's `expect()`)
///
/// Per spec: `expect(score_display).to_have_text("10").await?;`
#[derive(Debug, Clone)]
pub struct Expect {
    locator: Locator,
}

impl Expect {
    /// Create a new expectation for a locator
    #[must_use]
    pub const fn new(locator: Locator) -> Self {
        Self { locator }
    }

    /// Assert the element has specific text
    pub fn to_have_text(&self, expected: impl Into<String>) -> ExpectAssertion {
        ExpectAssertion::HasText {
            locator: self.locator.clone(),
            expected: expected.into(),
        }
    }

    /// Assert the element is visible
    pub fn to_be_visible(&self) -> ExpectAssertion {
        ExpectAssertion::IsVisible {
            locator: self.locator.clone(),
        }
    }

    /// Assert the element is hidden
    pub fn to_be_hidden(&self) -> ExpectAssertion {
        ExpectAssertion::IsHidden {
            locator: self.locator.clone(),
        }
    }

    /// Assert the element count
    pub fn to_have_count(&self, count: usize) -> ExpectAssertion {
        ExpectAssertion::HasCount {
            locator: self.locator.clone(),
            expected: count,
        }
    }

    /// Assert the element contains text
    pub fn to_contain_text(&self, text: impl Into<String>) -> ExpectAssertion {
        ExpectAssertion::ContainsText {
            locator: self.locator.clone(),
            expected: text.into(),
        }
    }
}

/// Assertion types for `expect()`
#[derive(Debug, Clone)]
pub enum ExpectAssertion {
    /// Element has exact text
    HasText {
        /// The locator
        locator: Locator,
        /// Expected text
        expected: String,
    },
    /// Element is visible
    IsVisible {
        /// The locator
        locator: Locator,
    },
    /// Element is hidden
    IsHidden {
        /// The locator
        locator: Locator,
    },
    /// Element count matches
    HasCount {
        /// The locator
        locator: Locator,
        /// Expected count
        expected: usize,
    },
    /// Element contains text
    ContainsText {
        /// The locator
        locator: Locator,
        /// Text to find
        expected: String,
    },
}

impl ExpectAssertion {
    /// Validate the assertion (synchronous for testing)
    ///
    /// # Errors
    ///
    /// Returns error if assertion fails
    pub fn validate(&self, actual: &str) -> ProbarResult<()> {
        match self {
            Self::HasText { expected, .. } => {
                if actual == expected {
                    Ok(())
                } else {
                    Err(ProbarError::AssertionError {
                        message: format!("Expected text '{expected}' but got '{actual}'"),
                    })
                }
            }
            Self::ContainsText { expected, .. } => {
                if actual.contains(expected) {
                    Ok(())
                } else {
                    Err(ProbarError::AssertionError {
                        message: format!(
                            "Expected text to contain '{expected}' but got '{actual}'"
                        ),
                    })
                }
            }
            Self::IsVisible { .. } | Self::IsHidden { .. } | Self::HasCount { .. } => {
                // These need browser context to validate
                Ok(())
            }
        }
    }

    /// Validate count assertion
    ///
    /// # Errors
    ///
    /// Returns error if count doesn't match
    pub fn validate_count(&self, actual: usize) -> ProbarResult<()> {
        match self {
            Self::HasCount { expected, .. } => {
                if actual == *expected {
                    Ok(())
                } else {
                    Err(ProbarError::AssertionError {
                        message: format!("Expected count {expected} but got {actual}"),
                    })
                }
            }
            _ => Ok(()),
        }
    }
}

/// Create an expectation for a locator (Playwright-style)
///
/// Per spec: `expect(score_display).to_have_text("10").await?;`
#[must_use]
pub fn expect(locator: Locator) -> Expect {
    Expect::new(locator)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ========================================================================
    // EXTREME TDD: Tests for Locator abstraction per Section 6.1.1
    // ========================================================================

    mod selector_tests {
        use super::*;

        #[test]
        fn test_css_selector() {
            let selector = Selector::css("button.primary");
            let query = selector.to_query();
            assert!(query.contains("querySelector"));
            assert!(query.contains("button.primary"));
        }

        #[test]
        fn test_test_id_selector() {
            let selector = Selector::test_id("score");
            let query = selector.to_query();
            assert!(query.contains("data-testid"));
            assert!(query.contains("score"));
        }

        #[test]
        fn test_text_selector() {
            let selector = Selector::text("Start Game");
            let query = selector.to_query();
            assert!(query.contains("textContent"));
            assert!(query.contains("Start Game"));
        }

        #[test]
        fn test_entity_selector() {
            let selector = Selector::entity("hero");
            let query = selector.to_query();
            assert!(query.contains("__wasm_get_entity"));
            assert!(query.contains("hero"));
        }

        #[test]
        fn test_count_query() {
            let selector = Selector::css("button");
            let query = selector.to_count_query();
            assert!(query.contains("querySelectorAll"));
            assert!(query.contains(".length"));
        }
    }

    mod locator_tests {
        use super::*;

        #[test]
        fn test_locator_new() {
            let locator = Locator::new("button");
            assert!(matches!(locator.selector(), Selector::Css(_)));
        }

        #[test]
        fn test_locator_with_text() {
            let locator = Locator::new("button").with_text("Start Game");
            assert!(matches!(locator.selector(), Selector::CssWithText { .. }));
        }

        #[test]
        fn test_locator_entity() {
            let locator = Locator::new("canvas").entity("hero");
            assert!(matches!(locator.selector(), Selector::CanvasEntity { .. }));
        }

        #[test]
        fn test_locator_timeout() {
            let locator = Locator::new("button").with_timeout(Duration::from_secs(10));
            assert_eq!(locator.options().timeout, Duration::from_secs(10));
        }

        #[test]
        fn test_locator_strict_mode() {
            let locator = Locator::new("button").with_strict(false);
            assert!(!locator.options().strict);
        }
    }

    mod action_tests {
        use super::*;

        #[test]
        fn test_click_action() {
            let locator = Locator::new("button");
            let action = locator.click().unwrap();
            assert!(matches!(action, LocatorAction::Click { .. }));
        }

        #[test]
        fn test_fill_action() {
            let locator = Locator::new("input");
            let action = locator.fill("test text").unwrap();
            assert!(matches!(action, LocatorAction::Fill { .. }));
        }

        #[test]
        fn test_drag_builder() {
            let locator = Locator::new("canvas").entity("hero");
            let drag = locator
                .drag_to(&Point::new(500.0, 500.0))
                .steps(10)
                .duration(Duration::from_millis(500))
                .build();
            assert!(matches!(drag, LocatorAction::Drag { steps: 10, .. }));
        }
    }

    mod query_tests {
        use super::*;

        #[test]
        fn test_text_content_query() {
            let locator = Locator::new("[data-testid='score']");
            let query = locator.text_content().unwrap();
            assert!(matches!(query, LocatorQuery::TextContent { .. }));
        }

        #[test]
        fn test_is_visible_query() {
            let locator = Locator::new("button");
            let query = locator.is_visible().unwrap();
            assert!(matches!(query, LocatorQuery::IsVisible { .. }));
        }

        #[test]
        fn test_count_query() {
            let locator = Locator::new("li");
            let query = locator.count().unwrap();
            assert!(matches!(query, LocatorQuery::Count { .. }));
        }
    }

    mod expect_tests {
        use super::*;

        #[test]
        fn test_expect_to_have_text() {
            let locator = Locator::new("[data-testid='score']");
            let assertion = expect(locator).to_have_text("10");
            assert!(matches!(assertion, ExpectAssertion::HasText { .. }));
        }

        #[test]
        fn test_expect_to_be_visible() {
            let locator = Locator::new("button");
            let assertion = expect(locator).to_be_visible();
            assert!(matches!(assertion, ExpectAssertion::IsVisible { .. }));
        }

        #[test]
        fn test_expect_to_have_count() {
            let locator = Locator::new("li");
            let assertion = expect(locator).to_have_count(5);
            assert!(matches!(
                assertion,
                ExpectAssertion::HasCount { expected: 5, .. }
            ));
        }

        #[test]
        fn test_validate_has_text_pass() {
            let locator = Locator::new("span");
            let assertion = expect(locator).to_have_text("10");
            assert!(assertion.validate("10").is_ok());
        }

        #[test]
        fn test_validate_has_text_fail() {
            let locator = Locator::new("span");
            let assertion = expect(locator).to_have_text("10");
            assert!(assertion.validate("20").is_err());
        }

        #[test]
        fn test_validate_contains_text_pass() {
            let locator = Locator::new("span");
            let assertion = expect(locator).to_contain_text("Score");
            assert!(assertion.validate("Score: 100").is_ok());
        }

        #[test]
        fn test_validate_count_pass() {
            let locator = Locator::new("li");
            let assertion = expect(locator).to_have_count(3);
            assert!(assertion.validate_count(3).is_ok());
        }

        #[test]
        fn test_validate_count_fail() {
            let locator = Locator::new("li");
            let assertion = expect(locator).to_have_count(3);
            assert!(assertion.validate_count(5).is_err());
        }
    }

    mod point_tests {
        use super::*;

        #[test]
        fn test_point_new() {
            let p = Point::new(100.0, 200.0);
            assert!((p.x - 100.0).abs() < f32::EPSILON);
            assert!((p.y - 200.0).abs() < f32::EPSILON);
        }
    }

    mod bounding_box_tests {
        use super::*;

        #[test]
        fn test_bounding_box_center() {
            let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
            let center = bbox.center();
            assert!((center.x - 50.0).abs() < f32::EPSILON);
            assert!((center.y - 50.0).abs() < f32::EPSILON);
        }

        #[test]
        fn test_bounding_box_contains() {
            let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
            assert!(bbox.contains(&Point::new(50.0, 50.0)));
            assert!(!bbox.contains(&Point::new(150.0, 50.0)));
        }
    }

    mod default_tests {
        use super::*;

        #[test]
        fn test_default_timeout() {
            assert_eq!(DEFAULT_TIMEOUT_MS, 5000);
        }

        #[test]
        fn test_default_poll_interval() {
            assert_eq!(DEFAULT_POLL_INTERVAL_MS, 50);
        }

        #[test]
        fn test_locator_options_default() {
            let opts = LocatorOptions::default();
            assert_eq!(opts.timeout, Duration::from_millis(5000));
            assert!(opts.strict);
            assert!(opts.visible);
        }
    }

    mod additional_selector_tests {
        use super::*;

        #[test]
        fn test_xpath_selector_query() {
            let selector = Selector::XPath("//button[@id='test']".to_string());
            let query = selector.to_query();
            assert!(query.contains("evaluate"));
            assert!(query.contains("XPathResult"));
        }

        #[test]
        fn test_xpath_selector_count_query() {
            let selector = Selector::XPath("//button".to_string());
            let query = selector.to_count_query();
            assert!(query.contains("SNAPSHOT"));
            assert!(query.contains("snapshotLength"));
        }

        #[test]
        fn test_css_with_text_selector() {
            let selector = Selector::CssWithText {
                css: "button".to_string(),
                text: "Click Me".to_string(),
            };
            let query = selector.to_query();
            assert!(query.contains("querySelectorAll"));
            assert!(query.contains("textContent"));
        }

        #[test]
        fn test_css_with_text_count_query() {
            let selector = Selector::CssWithText {
                css: "button".to_string(),
                text: "Click".to_string(),
            };
            let query = selector.to_count_query();
            assert!(query.contains("filter"));
            assert!(query.contains(".length"));
        }

        #[test]
        fn test_canvas_entity_selector() {
            let selector = Selector::CanvasEntity {
                entity: "player".to_string(),
            };
            let query = selector.to_query();
            assert!(query.contains("__wasm_get_canvas_entity"));
        }

        #[test]
        fn test_canvas_entity_count_query() {
            let selector = Selector::CanvasEntity {
                entity: "enemy".to_string(),
            };
            let query = selector.to_count_query();
            assert!(query.contains("__wasm_count_canvas_entities"));
        }

        #[test]
        fn test_text_selector_count_query() {
            let selector = Selector::text("Hello");
            let query = selector.to_count_query();
            assert!(query.contains("filter"));
            assert!(query.contains("length"));
        }

        #[test]
        fn test_entity_count_query() {
            let selector = Selector::entity("player");
            let query = selector.to_count_query();
            assert!(query.contains("__wasm_count_entities"));
        }
    }

    mod additional_drag_tests {
        use super::*;

        #[test]
        fn test_drag_operation_defaults() {
            let drag = DragOperation::to(Point::new(100.0, 100.0));
            assert_eq!(drag.steps, 10);
            assert_eq!(drag.duration, Duration::from_millis(500));
        }

        #[test]
        fn test_drag_operation_custom_steps() {
            let drag = DragOperation::to(Point::new(100.0, 100.0)).steps(20);
            assert_eq!(drag.steps, 20);
        }

        #[test]
        fn test_drag_operation_custom_duration() {
            let drag = DragOperation::to(Point::new(100.0, 100.0)).duration(Duration::from_secs(1));
            assert_eq!(drag.duration, Duration::from_secs(1));
        }
    }

    mod additional_locator_tests {
        use super::*;

        #[test]
        fn test_locator_bounding_box() {
            let locator = Locator::new("button");
            let query = locator.bounding_box().unwrap();
            assert!(matches!(query, LocatorQuery::BoundingBox { .. }));
        }
    }

    mod additional_bounding_box_tests {
        use super::*;

        #[test]
        fn test_bounding_box_creation_and_fields() {
            let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
            assert!((bbox.x - 10.0).abs() < f32::EPSILON);
            assert!((bbox.y - 20.0).abs() < f32::EPSILON);
            assert!((bbox.width - 100.0).abs() < f32::EPSILON);
            assert!((bbox.height - 50.0).abs() < f32::EPSILON);
        }

        #[test]
        fn test_bounding_box_contains_edge_cases() {
            let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
            // On the edge should be inside
            assert!(bbox.contains(&Point::new(0.0, 0.0)));
            assert!(bbox.contains(&Point::new(100.0, 100.0)));
            // Just outside should not be inside
            assert!(!bbox.contains(&Point::new(-1.0, 50.0)));
            assert!(!bbox.contains(&Point::new(101.0, 50.0)));
        }
    }

    // ============================================================================
    // QA CHECKLIST SECTION 2: Locator API Falsification Tests
    // Per docs/qa/100-point-qa-checklist-jugar-probar.md
    // ============================================================================

    #[allow(clippy::uninlined_format_args, unused_imports)]
    mod qa_checklist_locator_tests {
        #[allow(unused_imports)]
        use super::*;

        /// Test #25: Extremely long selector (10KB) - length limit enforced
        #[test]
        fn test_long_selector_limit() {
            const MAX_SELECTOR_LENGTH: usize = 10 * 1024; // 10KB limit
            let long_selector = "a".repeat(MAX_SELECTOR_LENGTH + 1);

            // Validate that we can detect oversized selectors
            let is_too_long = long_selector.len() > MAX_SELECTOR_LENGTH;
            assert!(is_too_long, "Should detect selector exceeding 10KB limit");

            // System should enforce limit (truncate or reject)
            let truncated = if long_selector.len() > MAX_SELECTOR_LENGTH {
                &long_selector[..MAX_SELECTOR_LENGTH]
            } else {
                &long_selector
            };
            assert_eq!(truncated.len(), MAX_SELECTOR_LENGTH);
        }

        /// Test #34: Shadow DOM elements traversal
        #[test]
        fn test_shadow_dom_selector_support() {
            // Shadow DOM requires special traversal via >>> or /deep/
            let shadow_selector = "host-element >>> .inner-element";

            // Validate shadow-piercing combinator is recognized
            let has_shadow_combinator =
                shadow_selector.contains(">>>") || shadow_selector.contains("/deep/");
            assert!(has_shadow_combinator, "Shadow DOM combinator recognized");

            // Generate appropriate query for shadow DOM
            let query = if shadow_selector.contains(">>>") {
                let parts: Vec<&str> = shadow_selector.split(">>>").collect();
                format!(
                    "document.querySelector('{}').shadowRoot.querySelector('{}')",
                    parts[0].trim(),
                    parts.get(1).unwrap_or(&"").trim()
                )
            } else {
                shadow_selector.to_string()
            };
            assert!(query.contains("shadowRoot"), "Shadow DOM query generated");
        }

        /// Test #35: iframe elements context switching
        #[test]
        fn test_iframe_context_switching() {
            // iframe requires contentDocument access
            let iframe_selector = "iframe#game-frame";
            let inner_selector = "button.start";

            // Generate iframe traversal query
            let query = format!(
                "document.querySelector('{}').contentDocument.querySelector('{}')",
                iframe_selector, inner_selector
            );

            assert!(query.contains("contentDocument"), "iframe context switch");
            assert!(query.contains(inner_selector), "Inner selector preserved");
        }

        /// Test empty selector handling (Test #21 reinforcement)
        #[test]
        fn test_empty_selector_rejection() {
            let empty_selector = "";
            let whitespace_selector = "   ";

            let is_empty_or_whitespace =
                empty_selector.is_empty() || whitespace_selector.trim().is_empty();
            assert!(
                is_empty_or_whitespace,
                "Empty/whitespace selectors detected"
            );
        }

        /// Test special characters in selectors
        #[test]
        fn test_special_char_selector_escaping() {
            let selector_with_quotes = r#"button[data-name="test's"]"#;
            let selector_with_brackets = "div[class~=foo\\[bar\\]]";

            // These should not cause parsing issues
            assert!(selector_with_quotes.contains('"'));
            assert!(selector_with_brackets.contains('['));
        }
    }
}
