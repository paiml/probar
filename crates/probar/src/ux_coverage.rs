//! UX Coverage Metrics (Feature 24 - EDD Compliance)
//!
//! Provides 100% provable UX coverage metrics for WASM games and TUI apps.
//! Tracks which UI elements, interactions, and states have been tested.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application
//!
//! - **Poka-Yoke**: Type-safe coverage tracking prevents blind spots
//! - **Muda**: Efficient hit counting without overhead
//! - **Genchi Genbutsu**: Coverage reflects actual user journeys
//! - **Heijunka**: Even distribution of test coverage

use crate::result::{ProbarError, ProbarResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A unique identifier for a UI element
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementId {
    /// Element type (button, input, label, etc.)
    pub element_type: String,
    /// Unique identifier
    pub id: String,
    /// Optional parent element ID
    pub parent: Option<String>,
}

impl ElementId {
    /// Create a new element ID
    #[must_use]
    pub fn new(element_type: &str, id: &str) -> Self {
        Self {
            element_type: element_type.to_string(),
            id: id.to_string(),
            parent: None,
        }
    }

    /// Create with parent
    #[must_use]
    pub fn with_parent(element_type: &str, id: &str, parent: &str) -> Self {
        Self {
            element_type: element_type.to_string(),
            id: id.to_string(),
            parent: Some(parent.to_string()),
        }
    }

    /// Get the full path (parent/id)
    #[must_use]
    pub fn full_path(&self) -> String {
        match &self.parent {
            Some(parent) => format!("{}/{}", parent, self.id),
            None => self.id.clone(),
        }
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.element_type, self.full_path())
    }
}

/// Types of interactions that can be tracked
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionType {
    /// Element was clicked/tapped
    Click,
    /// Element received focus
    Focus,
    /// Element lost focus
    Blur,
    /// Text was entered
    Input,
    /// Element was hovered
    Hover,
    /// Element was scrolled
    Scroll,
    /// Drag operation started
    DragStart,
    /// Drag operation ended
    DragEnd,
    /// Key was pressed while element focused
    KeyPress(String),
    /// Custom interaction
    Custom(String),
}

impl fmt::Display for InteractionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Click => write!(f, "click"),
            Self::Focus => write!(f, "focus"),
            Self::Blur => write!(f, "blur"),
            Self::Input => write!(f, "input"),
            Self::Hover => write!(f, "hover"),
            Self::Scroll => write!(f, "scroll"),
            Self::DragStart => write!(f, "drag_start"),
            Self::DragEnd => write!(f, "drag_end"),
            Self::KeyPress(key) => write!(f, "keypress:{key}"),
            Self::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}

/// Tracked interaction on an element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedInteraction {
    /// Element that was interacted with
    pub element: ElementId,
    /// Type of interaction
    pub interaction: InteractionType,
    /// Number of times this interaction occurred
    pub count: u64,
}

/// UI state that can be tracked
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateId {
    /// State category (screen, modal, menu, etc.)
    pub category: String,
    /// State name
    pub name: String,
}

impl StateId {
    /// Create a new state ID
    #[must_use]
    pub fn new(category: &str, name: &str) -> Self {
        Self {
            category: category.to_string(),
            name: name.to_string(),
        }
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.category, self.name)
    }
}

/// Coverage report for a single element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementCoverage {
    /// Element ID
    pub element: ElementId,
    /// Interactions that have been tested
    pub tested_interactions: HashSet<InteractionType>,
    /// Expected interactions for full coverage
    pub expected_interactions: HashSet<InteractionType>,
    /// Whether element was visible during tests
    pub was_visible: bool,
    /// Whether element was reachable/navigable
    pub was_reachable: bool,
}

impl ElementCoverage {
    /// Create a new element coverage tracker
    #[must_use]
    pub fn new(element: ElementId) -> Self {
        Self {
            element,
            tested_interactions: HashSet::new(),
            expected_interactions: HashSet::new(),
            was_visible: false,
            was_reachable: false,
        }
    }

    /// Add an expected interaction
    pub fn expect(&mut self, interaction: InteractionType) {
        self.expected_interactions.insert(interaction);
    }

    /// Record a tested interaction
    pub fn record(&mut self, interaction: InteractionType) {
        self.tested_interactions.insert(interaction);
    }

    /// Mark as visible
    pub fn mark_visible(&mut self) {
        self.was_visible = true;
    }

    /// Mark as reachable
    pub fn mark_reachable(&mut self) {
        self.was_reachable = true;
    }

    /// Get coverage percentage (0.0 to 1.0)
    #[must_use]
    pub fn coverage_ratio(&self) -> f64 {
        if self.expected_interactions.is_empty() {
            return 1.0;
        }
        let covered = self
            .tested_interactions
            .intersection(&self.expected_interactions)
            .count();
        covered as f64 / self.expected_interactions.len() as f64
    }

    /// Check if fully covered
    #[must_use]
    pub fn is_fully_covered(&self) -> bool {
        self.expected_interactions
            .iter()
            .all(|i| self.tested_interactions.contains(i))
    }

    /// Get uncovered interactions
    #[must_use]
    pub fn uncovered(&self) -> Vec<&InteractionType> {
        self.expected_interactions
            .iter()
            .filter(|i| !self.tested_interactions.contains(i))
            .collect()
    }
}

/// UX Coverage Tracker
#[derive(Debug, Default)]
pub struct UxCoverageTracker {
    /// Coverage by element
    elements: HashMap<String, ElementCoverage>,
    /// States that have been visited
    visited_states: HashSet<StateId>,
    /// Expected states for full coverage
    expected_states: HashSet<StateId>,
    /// Interaction counts
    interaction_counts: HashMap<String, u64>,
    /// User journeys (sequences of states)
    journeys: Vec<Vec<StateId>>,
    /// Current journey being recorded
    current_journey: Vec<StateId>,
}

impl UxCoverageTracker {
    /// Create a new UX coverage tracker
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an element with expected interactions
    pub fn register_element(&mut self, element: ElementId, expected: &[InteractionType]) {
        let key = element.to_string();
        let mut coverage = ElementCoverage::new(element);
        for interaction in expected {
            coverage.expect(interaction.clone());
        }
        self.elements.insert(key, coverage);
    }

    /// Register a button element (click expected)
    pub fn register_button(&mut self, id: &str) {
        let element = ElementId::new("button", id);
        self.register_element(element, &[InteractionType::Click]);
    }

    /// Register an input element (focus, input, blur expected)
    pub fn register_input(&mut self, id: &str) {
        let element = ElementId::new("input", id);
        self.register_element(
            element,
            &[InteractionType::Focus, InteractionType::Input, InteractionType::Blur],
        );
    }

    /// Register a clickable element
    pub fn register_clickable(&mut self, element_type: &str, id: &str) {
        let element = ElementId::new(element_type, id);
        self.register_element(element, &[InteractionType::Click]);
    }

    /// Register an expected state
    pub fn register_state(&mut self, state: StateId) {
        self.expected_states.insert(state);
    }

    /// Register a screen state
    pub fn register_screen(&mut self, name: &str) {
        self.register_state(StateId::new("screen", name));
    }

    /// Register a modal state
    pub fn register_modal(&mut self, name: &str) {
        self.register_state(StateId::new("modal", name));
    }

    /// Record an interaction
    pub fn record_interaction(&mut self, element: &ElementId, interaction: InteractionType) {
        let key = element.to_string();

        if let Some(coverage) = self.elements.get_mut(&key) {
            coverage.record(interaction.clone());
        }

        // Update interaction counts
        let count_key = format!("{}:{}", key, interaction);
        *self.interaction_counts.entry(count_key).or_insert(0) += 1;
    }

    /// Record element visibility
    pub fn record_visibility(&mut self, element: &ElementId) {
        let key = element.to_string();
        if let Some(coverage) = self.elements.get_mut(&key) {
            coverage.mark_visible();
        }
    }

    /// Record element reachability
    pub fn record_reachability(&mut self, element: &ElementId) {
        let key = element.to_string();
        if let Some(coverage) = self.elements.get_mut(&key) {
            coverage.mark_reachable();
        }
    }

    /// Record a state visit
    pub fn record_state(&mut self, state: StateId) {
        self.visited_states.insert(state.clone());
        self.current_journey.push(state);
    }

    /// End current journey and start a new one
    pub fn end_journey(&mut self) {
        if !self.current_journey.is_empty() {
            self.journeys.push(std::mem::take(&mut self.current_journey));
        }
    }

    /// Get overall element coverage percentage
    #[must_use]
    pub fn element_coverage(&self) -> f64 {
        if self.elements.is_empty() {
            return 1.0;
        }
        let total_coverage: f64 = self.elements.values().map(ElementCoverage::coverage_ratio).sum();
        total_coverage / self.elements.len() as f64
    }

    /// Get state coverage percentage
    #[must_use]
    pub fn state_coverage(&self) -> f64 {
        if self.expected_states.is_empty() {
            return 1.0;
        }
        let visited = self
            .expected_states
            .iter()
            .filter(|s| self.visited_states.contains(s))
            .count();
        visited as f64 / self.expected_states.len() as f64
    }

    /// Get overall UX coverage percentage
    #[must_use]
    pub fn overall_coverage(&self) -> f64 {
        let element = self.element_coverage();
        let state = self.state_coverage();

        // Weight equally if both have expectations
        if self.elements.is_empty() {
            return state;
        }
        if self.expected_states.is_empty() {
            return element;
        }

        (element + state) / 2.0
    }

    /// Check if 100% coverage achieved
    #[must_use]
    pub fn is_complete(&self) -> bool {
        (self.overall_coverage() - 1.0).abs() < f64::EPSILON
    }

    /// Get uncovered elements
    #[must_use]
    pub fn uncovered_elements(&self) -> Vec<&ElementCoverage> {
        self.elements
            .values()
            .filter(|e| !e.is_fully_covered())
            .collect()
    }

    /// Get unvisited states
    #[must_use]
    pub fn unvisited_states(&self) -> Vec<&StateId> {
        self.expected_states
            .iter()
            .filter(|s| !self.visited_states.contains(s))
            .collect()
    }

    /// Get all recorded journeys
    #[must_use]
    pub fn journeys(&self) -> &[Vec<StateId>] {
        &self.journeys
    }

    /// Generate a coverage report
    #[must_use]
    pub fn generate_report(&self) -> UxCoverageReport {
        UxCoverageReport {
            overall_coverage: self.overall_coverage(),
            element_coverage: self.element_coverage(),
            state_coverage: self.state_coverage(),
            total_elements: self.elements.len(),
            covered_elements: self.elements.values().filter(|e| e.is_fully_covered()).count(),
            total_states: self.expected_states.len(),
            covered_states: self.visited_states.len(),
            total_interactions: self.interaction_counts.values().sum(),
            unique_journeys: self.journeys.len(),
            is_complete: self.is_complete(),
        }
    }

    /// Assert minimum coverage
    pub fn assert_coverage(&self, min_coverage: f64) -> ProbarResult<()> {
        let actual = self.overall_coverage();
        if actual >= min_coverage {
            Ok(())
        } else {
            let uncovered_elements: Vec<String> = self
                .uncovered_elements()
                .iter()
                .map(|e| e.element.to_string())
                .collect();
            let unvisited_states: Vec<String> = self
                .unvisited_states()
                .iter()
                .map(|s| s.to_string())
                .collect();

            Err(ProbarError::AssertionFailed {
                message: format!(
                    "UX coverage {:.1}% is below minimum {:.1}%\n\
                    Uncovered elements: {:?}\n\
                    Unvisited states: {:?}",
                    actual * 100.0,
                    min_coverage * 100.0,
                    uncovered_elements,
                    unvisited_states
                ),
            })
        }
    }

    /// Assert 100% coverage
    pub fn assert_complete(&self) -> ProbarResult<()> {
        self.assert_coverage(1.0)
    }
}

/// UX Coverage Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UxCoverageReport {
    /// Overall UX coverage percentage (0.0 to 1.0)
    pub overall_coverage: f64,
    /// Element interaction coverage
    pub element_coverage: f64,
    /// State/screen coverage
    pub state_coverage: f64,
    /// Total elements registered
    pub total_elements: usize,
    /// Elements fully covered
    pub covered_elements: usize,
    /// Total states expected
    pub total_states: usize,
    /// States visited
    pub covered_states: usize,
    /// Total interactions recorded
    pub total_interactions: u64,
    /// Number of unique user journeys
    pub unique_journeys: usize,
    /// Whether 100% coverage achieved
    pub is_complete: bool,
}

impl UxCoverageReport {
    /// Format as text summary
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "UX Coverage Report\n\
            ==================\n\
            Overall Coverage: {:.1}%\n\
            Element Coverage: {:.1}% ({}/{} elements)\n\
            State Coverage:   {:.1}% ({}/{} states)\n\
            Interactions:     {}\n\
            User Journeys:    {}\n\
            Status:           {}",
            self.overall_coverage * 100.0,
            self.element_coverage * 100.0,
            self.covered_elements,
            self.total_elements,
            self.state_coverage * 100.0,
            self.covered_states,
            self.total_states,
            self.total_interactions,
            self.unique_journeys,
            if self.is_complete { "COMPLETE" } else { "INCOMPLETE" }
        )
    }
}

impl fmt::Display for UxCoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Builder for defining UX coverage requirements
#[derive(Debug, Default)]
pub struct UxCoverageBuilder {
    tracker: UxCoverageTracker,
}

impl UxCoverageBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a button
    #[must_use]
    pub fn button(mut self, id: &str) -> Self {
        self.tracker.register_button(id);
        self
    }

    /// Add an input field
    #[must_use]
    pub fn input(mut self, id: &str) -> Self {
        self.tracker.register_input(id);
        self
    }

    /// Add a clickable element
    #[must_use]
    pub fn clickable(mut self, element_type: &str, id: &str) -> Self {
        self.tracker.register_clickable(element_type, id);
        self
    }

    /// Add a screen state
    #[must_use]
    pub fn screen(mut self, name: &str) -> Self {
        self.tracker.register_screen(name);
        self
    }

    /// Add a modal state
    #[must_use]
    pub fn modal(mut self, name: &str) -> Self {
        self.tracker.register_modal(name);
        self
    }

    /// Add a custom element with expected interactions
    #[must_use]
    pub fn element(mut self, element: ElementId, expected: &[InteractionType]) -> Self {
        self.tracker.register_element(element, expected);
        self
    }

    /// Add a custom state
    #[must_use]
    pub fn state(mut self, category: &str, name: &str) -> Self {
        self.tracker.register_state(StateId::new(category, name));
        self
    }

    /// Build the tracker
    #[must_use]
    pub fn build(self) -> UxCoverageTracker {
        self.tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod element_id_tests {
        use super::*;

        #[test]
        fn test_new() {
            let id = ElementId::new("button", "submit");
            assert_eq!(id.element_type, "button");
            assert_eq!(id.id, "submit");
            assert!(id.parent.is_none());
        }

        #[test]
        fn test_with_parent() {
            let id = ElementId::with_parent("button", "ok", "dialog");
            assert_eq!(id.parent, Some("dialog".to_string()));
        }

        #[test]
        fn test_full_path() {
            let id1 = ElementId::new("button", "submit");
            assert_eq!(id1.full_path(), "submit");

            let id2 = ElementId::with_parent("button", "ok", "dialog");
            assert_eq!(id2.full_path(), "dialog/ok");
        }

        #[test]
        fn test_display() {
            let id = ElementId::new("button", "submit");
            assert_eq!(format!("{}", id), "button:submit");
        }
    }

    mod interaction_type_tests {
        use super::*;

        #[test]
        fn test_display() {
            assert_eq!(format!("{}", InteractionType::Click), "click");
            assert_eq!(format!("{}", InteractionType::Focus), "focus");
            assert_eq!(format!("{}", InteractionType::KeyPress("Enter".to_string())), "keypress:Enter");
            assert_eq!(format!("{}", InteractionType::Custom("swipe".to_string())), "custom:swipe");
        }
    }

    mod element_coverage_tests {
        use super::*;

        #[test]
        fn test_new() {
            let element = ElementId::new("button", "test");
            let coverage = ElementCoverage::new(element);

            assert!(coverage.tested_interactions.is_empty());
            assert!(coverage.expected_interactions.is_empty());
        }

        #[test]
        fn test_coverage_ratio() {
            let element = ElementId::new("button", "test");
            let mut coverage = ElementCoverage::new(element);

            coverage.expect(InteractionType::Click);
            coverage.expect(InteractionType::Hover);

            assert!((coverage.coverage_ratio() - 0.0).abs() < f64::EPSILON);

            coverage.record(InteractionType::Click);
            assert!((coverage.coverage_ratio() - 0.5).abs() < f64::EPSILON);

            coverage.record(InteractionType::Hover);
            assert!((coverage.coverage_ratio() - 1.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_is_fully_covered() {
            let element = ElementId::new("button", "test");
            let mut coverage = ElementCoverage::new(element);

            coverage.expect(InteractionType::Click);
            assert!(!coverage.is_fully_covered());

            coverage.record(InteractionType::Click);
            assert!(coverage.is_fully_covered());
        }

        #[test]
        fn test_uncovered() {
            let element = ElementId::new("button", "test");
            let mut coverage = ElementCoverage::new(element);

            coverage.expect(InteractionType::Click);
            coverage.expect(InteractionType::Hover);
            coverage.record(InteractionType::Click);

            let uncovered = coverage.uncovered();
            assert_eq!(uncovered.len(), 1);
            assert_eq!(uncovered[0], &InteractionType::Hover);
        }
    }

    mod ux_coverage_tracker_tests {
        use super::*;

        #[test]
        fn test_new() {
            let tracker = UxCoverageTracker::new();
            assert!(tracker.is_complete()); // Empty tracker is "complete"
        }

        #[test]
        fn test_register_button() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("submit");

            assert_eq!(tracker.elements.len(), 1);
            assert!((tracker.element_coverage() - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_record_interaction() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("submit");

            let element = ElementId::new("button", "submit");
            tracker.record_interaction(&element, InteractionType::Click);

            assert!((tracker.element_coverage() - 1.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_register_state() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_screen("home");
            tracker.register_screen("settings");

            assert_eq!(tracker.expected_states.len(), 2);
            assert!((tracker.state_coverage() - 0.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_record_state() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_screen("home");
            tracker.register_screen("settings");

            tracker.record_state(StateId::new("screen", "home"));
            assert!((tracker.state_coverage() - 0.5).abs() < f64::EPSILON);

            tracker.record_state(StateId::new("screen", "settings"));
            assert!((tracker.state_coverage() - 1.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_overall_coverage() {
            let mut tracker = UxCoverageTracker::new();

            // Register 2 buttons and 2 screens
            tracker.register_button("btn1");
            tracker.register_button("btn2");
            tracker.register_screen("home");
            tracker.register_screen("settings");

            // Cover 1 button and 1 screen
            tracker.record_interaction(&ElementId::new("button", "btn1"), InteractionType::Click);
            tracker.record_state(StateId::new("screen", "home"));

            // 50% element + 50% state = 50% overall
            assert!((tracker.overall_coverage() - 0.5).abs() < f64::EPSILON);
        }

        #[test]
        fn test_journeys() {
            let mut tracker = UxCoverageTracker::new();

            tracker.record_state(StateId::new("screen", "home"));
            tracker.record_state(StateId::new("screen", "settings"));
            tracker.end_journey();

            tracker.record_state(StateId::new("screen", "home"));
            tracker.record_state(StateId::new("screen", "profile"));
            tracker.end_journey();

            assert_eq!(tracker.journeys().len(), 2);
        }

        #[test]
        fn test_assert_coverage_pass() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("btn");
            tracker.record_interaction(&ElementId::new("button", "btn"), InteractionType::Click);

            assert!(tracker.assert_coverage(1.0).is_ok());
        }

        #[test]
        fn test_assert_coverage_fail() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("btn");

            assert!(tracker.assert_coverage(1.0).is_err());
        }

        #[test]
        fn test_uncovered_elements() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("btn1");
            tracker.register_button("btn2");
            tracker.record_interaction(&ElementId::new("button", "btn1"), InteractionType::Click);

            let uncovered = tracker.uncovered_elements();
            assert_eq!(uncovered.len(), 1);
        }

        #[test]
        fn test_unvisited_states() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_screen("home");
            tracker.register_screen("settings");
            tracker.record_state(StateId::new("screen", "home"));

            let unvisited = tracker.unvisited_states();
            assert_eq!(unvisited.len(), 1);
        }
    }

    mod ux_coverage_report_tests {
        use super::*;

        #[test]
        fn test_generate_report() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("btn1");
            tracker.register_button("btn2");
            tracker.register_screen("home");

            tracker.record_interaction(&ElementId::new("button", "btn1"), InteractionType::Click);
            tracker.record_state(StateId::new("screen", "home"));

            let report = tracker.generate_report();
            assert_eq!(report.total_elements, 2);
            assert_eq!(report.covered_elements, 1);
            assert_eq!(report.total_states, 1);
            assert_eq!(report.covered_states, 1);
            assert!(!report.is_complete);
        }

        #[test]
        fn test_complete_report() {
            let mut tracker = UxCoverageTracker::new();
            tracker.register_button("btn");
            tracker.record_interaction(&ElementId::new("button", "btn"), InteractionType::Click);

            let report = tracker.generate_report();
            assert!(report.is_complete);
        }
    }

    mod ux_coverage_builder_tests {
        use super::*;

        #[test]
        fn test_builder() {
            let tracker = UxCoverageBuilder::new()
                .button("submit")
                .button("cancel")
                .input("username")
                .screen("login")
                .screen("dashboard")
                .build();

            assert_eq!(tracker.elements.len(), 3);
            assert_eq!(tracker.expected_states.len(), 2);
        }

        #[test]
        fn test_custom_element() {
            let tracker = UxCoverageBuilder::new()
                .element(
                    ElementId::new("canvas", "game"),
                    &[InteractionType::Click, InteractionType::Hover],
                )
                .build();

            assert_eq!(tracker.elements.len(), 1);
        }
    }

    mod pong_game_coverage_tests {
        use super::*;

        #[test]
        fn test_pong_full_coverage() {
            // Define expected coverage for a Pong game
            let mut tracker = UxCoverageBuilder::new()
                .button("start_game")
                .button("pause")
                .button("restart")
                .clickable("paddle", "player")
                .screen("title")
                .screen("playing")
                .screen("paused")
                .screen("game_over")
                .build();

            // Simulate a test session that covers everything
            // Title screen
            tracker.record_state(StateId::new("screen", "title"));
            tracker.record_interaction(&ElementId::new("button", "start_game"), InteractionType::Click);

            // Playing
            tracker.record_state(StateId::new("screen", "playing"));
            tracker.record_interaction(&ElementId::new("paddle", "player"), InteractionType::Click);
            tracker.record_interaction(&ElementId::new("button", "pause"), InteractionType::Click);

            // Paused
            tracker.record_state(StateId::new("screen", "paused"));

            // Resume and game over
            tracker.record_state(StateId::new("screen", "game_over"));
            tracker.record_interaction(&ElementId::new("button", "restart"), InteractionType::Click);

            // Verify 100% coverage
            assert!(tracker.assert_complete().is_ok());
        }

        #[test]
        fn test_pong_partial_coverage() {
            let mut tracker = UxCoverageBuilder::new()
                .button("start_game")
                .button("pause")
                .screen("title")
                .screen("playing")
                .build();

            // Only cover some things
            tracker.record_state(StateId::new("screen", "title"));
            tracker.record_interaction(&ElementId::new("button", "start_game"), InteractionType::Click);

            let report = tracker.generate_report();
            assert!(!report.is_complete);
            assert!((report.element_coverage - 0.5).abs() < f64::EPSILON);
            assert!((report.state_coverage - 0.5).abs() < f64::EPSILON);
        }
    }
}
