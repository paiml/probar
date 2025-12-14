//! Playbook YAML schema types for state machine testing.
//!
//! Implements SCXML-inspired state definitions with transition-based assertions.
//! Reference: W3C SCXML Specification <https://www.w3.org/TR/scxml/>

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root playbook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    /// Schema version (must be "1.0")
    pub version: String,
    /// State machine definition
    pub machine: StateMachine,
    /// Performance budget constraints
    #[serde(default)]
    pub performance: PerformanceBudget,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// State machine definition following SCXML semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    /// Unique identifier for the machine
    pub id: String,
    /// Initial state ID (must exist in states)
    pub initial: String,
    /// State definitions keyed by ID
    pub states: HashMap<String, State>,
    /// Transition definitions
    pub transitions: Vec<Transition>,
}

/// Individual state definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// State identifier (must be unique)
    pub id: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Entry actions executed when entering this state
    #[serde(default)]
    pub on_entry: Vec<Action>,
    /// Exit actions executed when leaving this state
    #[serde(default)]
    pub on_exit: Vec<Action>,
    /// Invariant conditions that must hold while in this state
    #[serde(default)]
    pub invariants: Vec<Invariant>,
    /// Whether this is a final (accepting) state
    #[serde(default)]
    pub final_state: bool,
}

/// State transition definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// Unique transition identifier
    pub id: String,
    /// Source state ID
    pub from: String,
    /// Target state ID
    pub to: String,
    /// Event that triggers this transition
    pub event: String,
    /// Guard condition (optional)
    #[serde(default)]
    pub guard: Option<String>,
    /// Actions to execute during transition
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Expected assertions after transition
    #[serde(default)]
    pub assertions: Vec<Assertion>,
}

/// Action to execute during state entry, exit, or transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Click on an element
    #[serde(rename = "click")]
    Click { selector: String },
    /// Type text into an element
    #[serde(rename = "type")]
    Type { selector: String, text: String },
    /// Wait for a condition
    #[serde(rename = "wait")]
    Wait { condition: WaitCondition },
    /// Navigate to URL
    #[serde(rename = "navigate")]
    Navigate { url: String },
    /// Execute custom JavaScript
    #[serde(rename = "script")]
    Script { code: String },
    /// Take screenshot
    #[serde(rename = "screenshot")]
    Screenshot { name: String },
}

/// Wait condition types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WaitCondition {
    /// Wait for element to be visible
    #[serde(rename = "visible")]
    Visible { selector: String },
    /// Wait for element to be hidden
    #[serde(rename = "hidden")]
    Hidden { selector: String },
    /// Wait for fixed duration (ms)
    #[serde(rename = "duration")]
    Duration { ms: u64 },
    /// Wait for network idle
    #[serde(rename = "network_idle")]
    NetworkIdle,
    /// Wait for custom condition
    #[serde(rename = "condition")]
    Condition { expression: String },
}

/// Assertion to verify after transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Assertion {
    /// Element exists in DOM
    #[serde(rename = "element_exists")]
    ElementExists { selector: String },
    /// Element has specific text
    #[serde(rename = "text_equals")]
    TextEquals { selector: String, expected: String },
    /// Element has text containing substring
    #[serde(rename = "text_contains")]
    TextContains { selector: String, substring: String },
    /// Element has specific attribute value
    #[serde(rename = "attribute_equals")]
    AttributeEquals {
        selector: String,
        attribute: String,
        expected: String,
    },
    /// URL matches pattern
    #[serde(rename = "url_matches")]
    UrlMatches { pattern: String },
    /// Custom JavaScript assertion
    #[serde(rename = "script")]
    Script { expression: String },
}

/// Invariant condition that must hold while in a state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    /// Human-readable description
    pub description: String,
    /// Condition expression
    pub condition: String,
    /// Severity if violated
    #[serde(default)]
    pub severity: InvariantSeverity,
}

/// Severity level for invariant violations.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InvariantSeverity {
    /// Warning only, test continues
    Warning,
    /// Error, test fails
    #[default]
    Error,
    /// Critical, test aborts immediately
    Critical,
}

/// Performance budget constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceBudget {
    /// Maximum time per transition (ms)
    #[serde(default)]
    pub max_transition_time_ms: Option<u64>,
    /// Maximum total playbook time (ms)
    #[serde(default)]
    pub max_total_time_ms: Option<u64>,
    /// Maximum memory usage (bytes)
    #[serde(default)]
    pub max_memory_bytes: Option<u64>,
    /// Complexity class constraint
    #[serde(default)]
    pub complexity_class: Option<ComplexityClass>,
}

/// Expected algorithmic complexity class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplexityClass {
    /// O(1) - constant time
    #[serde(rename = "O(1)")]
    Constant,
    /// O(log n) - logarithmic
    #[serde(rename = "O(log n)")]
    Logarithmic,
    /// O(n) - linear
    #[serde(rename = "O(n)")]
    Linear,
    /// O(n log n) - linearithmic
    #[serde(rename = "O(n log n)")]
    Linearithmic,
    /// O(n^2) - quadratic
    #[serde(rename = "O(n^2)")]
    Quadratic,
}

impl Playbook {
    /// Parse a playbook from YAML string.
    ///
    /// # Errors
    /// Returns error if YAML is invalid or schema validation fails.
    pub fn from_yaml(yaml: &str) -> Result<Self, PlaybookError> {
        let playbook: Playbook =
            serde_yaml::from_str(yaml).map_err(|e| PlaybookError::ParseError(e.to_string()))?;
        playbook.validate()?;
        Ok(playbook)
    }

    /// Validate the playbook structure.
    fn validate(&self) -> Result<(), PlaybookError> {
        // Validate version
        if self.version != "1.0" {
            return Err(PlaybookError::InvalidVersion(self.version.clone()));
        }

        // Validate initial state exists
        if !self.machine.states.contains_key(&self.machine.initial) {
            return Err(PlaybookError::InvalidInitialState(
                self.machine.initial.clone(),
            ));
        }

        // Validate all transition sources and targets exist
        for transition in &self.machine.transitions {
            if !self.machine.states.contains_key(&transition.from) {
                return Err(PlaybookError::InvalidTransitionSource {
                    transition_id: transition.id.clone(),
                    state_id: transition.from.clone(),
                });
            }
            if !self.machine.states.contains_key(&transition.to) {
                return Err(PlaybookError::InvalidTransitionTarget {
                    transition_id: transition.id.clone(),
                    state_id: transition.to.clone(),
                });
            }
        }

        // Check for duplicate state IDs (HashMap handles this, but explicit check)
        let state_ids: Vec<_> = self.machine.states.keys().collect();
        let unique_ids: std::collections::HashSet<_> = state_ids.iter().collect();
        if state_ids.len() != unique_ids.len() {
            return Err(PlaybookError::DuplicateStateIds);
        }

        // Check for duplicate transition IDs
        let transition_ids: Vec<_> = self.machine.transitions.iter().map(|t| &t.id).collect();
        let unique_transition_ids: std::collections::HashSet<_> = transition_ids.iter().collect();
        if transition_ids.len() != unique_transition_ids.len() {
            return Err(PlaybookError::DuplicateTransitionIds);
        }

        // Validate no empty states or transitions
        if self.machine.states.is_empty() {
            return Err(PlaybookError::EmptyStates);
        }
        if self.machine.transitions.is_empty() {
            return Err(PlaybookError::EmptyTransitions);
        }

        Ok(())
    }
}

/// Errors that can occur during playbook parsing and validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlaybookError {
    #[error("Failed to parse YAML: {0}")]
    ParseError(String),

    #[error("Invalid version '{0}', expected '1.0'")]
    InvalidVersion(String),

    #[error("Initial state '{0}' does not exist")]
    InvalidInitialState(String),

    #[error("Transition '{transition_id}' references non-existent source state '{state_id}'")]
    InvalidTransitionSource {
        transition_id: String,
        state_id: String,
    },

    #[error("Transition '{transition_id}' references non-existent target state '{state_id}'")]
    InvalidTransitionTarget {
        transition_id: String,
        state_id: String,
    },

    #[error("Duplicate state IDs detected")]
    DuplicateStateIds,

    #[error("Duplicate transition IDs detected")]
    DuplicateTransitionIds,

    #[error("States cannot be empty")]
    EmptyStates,

    #[error("Transitions cannot be empty")]
    EmptyTransitions,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_PLAYBOOK: &str = r##"
version: "1.0"
machine:
  id: "login_flow"
  initial: "logged_out"
  states:
    logged_out:
      id: "logged_out"
      description: "User is not authenticated"
      invariants:
        - description: "Login button visible"
          condition: "document.querySelector('#login-btn') !== null"
    logged_in:
      id: "logged_in"
      description: "User is authenticated"
      final_state: true
  transitions:
    - id: "t1"
      from: "logged_out"
      to: "logged_in"
      event: "login_success"
      assertions:
        - type: element_exists
          selector: "#welcome-message"
"##;

    #[test]
    fn test_parse_valid_playbook() {
        let playbook = Playbook::from_yaml(VALID_PLAYBOOK).expect("Should parse valid playbook");
        assert_eq!(playbook.version, "1.0");
        assert_eq!(playbook.machine.id, "login_flow");
        assert_eq!(playbook.machine.initial, "logged_out");
        assert_eq!(playbook.machine.states.len(), 2);
        assert_eq!(playbook.machine.transitions.len(), 1);
    }

    #[test]
    fn test_reject_invalid_version() {
        let yaml = VALID_PLAYBOOK.replace("version: \"1.0\"", "version: \"2.0\"");
        let result = Playbook::from_yaml(&yaml);
        assert!(matches!(result, Err(PlaybookError::InvalidVersion(_))));
    }

    #[test]
    fn test_reject_invalid_initial_state() {
        let yaml = VALID_PLAYBOOK.replace("initial: \"logged_out\"", "initial: \"nonexistent\"");
        let result = Playbook::from_yaml(&yaml);
        assert!(matches!(result, Err(PlaybookError::InvalidInitialState(_))));
    }

    #[test]
    fn test_reject_invalid_transition_source() {
        let yaml = VALID_PLAYBOOK.replace("from: \"logged_out\"", "from: \"nonexistent\"");
        let result = Playbook::from_yaml(&yaml);
        assert!(matches!(
            result,
            Err(PlaybookError::InvalidTransitionSource { .. })
        ));
    }

    #[test]
    fn test_reject_invalid_transition_target() {
        let yaml = VALID_PLAYBOOK.replace("to: \"logged_in\"", "to: \"nonexistent\"");
        let result = Playbook::from_yaml(&yaml);
        assert!(matches!(
            result,
            Err(PlaybookError::InvalidTransitionTarget { .. })
        ));
    }

    #[test]
    fn test_reject_empty_states() {
        let yaml = r#"
version: "1.0"
machine:
  id: "test"
  initial: "start"
  states: {}
  transitions:
    - id: "t1"
      from: "start"
      to: "end"
      event: "go"
"#;
        let result = Playbook::from_yaml(yaml);
        // Empty states causes initial state validation to fail first
        assert!(result.is_err());
        // Could be EmptyStates or InvalidInitialState depending on validation order
        assert!(matches!(
            result,
            Err(PlaybookError::EmptyStates) | Err(PlaybookError::InvalidInitialState(_))
        ));
    }

    #[test]
    fn test_reject_empty_transitions() {
        let yaml = r#"
version: "1.0"
machine:
  id: "test"
  initial: "start"
  states:
    start:
      id: "start"
  transitions: []
"#;
        let result = Playbook::from_yaml(yaml);
        assert!(matches!(result, Err(PlaybookError::EmptyTransitions)));
    }
}
