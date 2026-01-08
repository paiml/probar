//! WorkerBrick: Web Worker code generation from brick definitions (PROBAR-SPEC-009-P7)
//!
//! Generates both JavaScript Worker code and Rust web_sys bindings
//! from a single brick definition. Zero hand-written JavaScript.
//!
//! # Example
//!
//! ```rust,ignore
//! use probar::brick::worker::{WorkerBrick, BrickWorkerMessage, BrickWorkerMessageDirection};
//!
//! let worker = WorkerBrick::new("transcription")
//!     .message(BrickWorkerMessage::new("init", BrickWorkerMessageDirection::ToWorker)
//!         .field("modelUrl", FieldType::String)
//!         .field("buffer", FieldType::SharedArrayBuffer))
//!     .message(BrickWorkerMessage::new("ready", BrickWorkerMessageDirection::FromWorker))
//!     .transition("uninitialized", "init", "loading")
//!     .transition("loading", "ready", "ready");
//!
//! // Generate JavaScript
//! let js = worker.to_worker_js();
//!
//! // Generate Rust bindings
//! let rust = worker.to_rust_bindings();
//! ```

use super::{Brick, BrickAssertion, BrickBudget, BrickVerification};
use std::time::Duration;

/// Direction of worker message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrickWorkerMessageDirection {
    /// Message sent to worker (main → worker)
    ToWorker,
    /// Message sent from worker (worker → main)
    FromWorker,
    /// Message can be sent in either direction
    Bidirectional,
}

/// Field type for worker messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    /// JavaScript string
    String,
    /// JavaScript number (f64)
    Number,
    /// JavaScript boolean
    Boolean,
    /// SharedArrayBuffer for audio/data transfer
    SharedArrayBuffer,
    /// Float32Array for audio samples
    Float32Array,
    /// Nested object with fields
    Object(Vec<MessageField>),
    /// Optional field
    Optional(Box<FieldType>),
}

impl FieldType {
    /// Get TypeScript type annotation
    #[must_use]
    pub fn to_typescript(&self) -> String {
        match self {
            Self::String => "string".into(),
            Self::Number => "number".into(),
            Self::Boolean => "boolean".into(),
            Self::SharedArrayBuffer => "SharedArrayBuffer".into(),
            Self::Float32Array => "Float32Array".into(),
            Self::Object(fields) => {
                let field_types: Vec<_> = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, f.field_type.to_typescript()))
                    .collect();
                format!("{{ {} }}", field_types.join(", "))
            }
            Self::Optional(inner) => format!("{} | undefined", inner.to_typescript()),
        }
    }

    /// Get Rust type annotation
    #[must_use]
    pub fn to_rust(&self) -> String {
        match self {
            Self::String => "String".into(),
            Self::Number => "f64".into(),
            Self::Boolean => "bool".into(),
            Self::SharedArrayBuffer => "js_sys::SharedArrayBuffer".into(),
            Self::Float32Array => "js_sys::Float32Array".into(),
            Self::Object(_) => "serde_json::Value".into(),
            Self::Optional(inner) => format!("Option<{}>", inner.to_rust()),
        }
    }
}

/// A field in a worker message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Whether the field is required
    pub required: bool,
}

impl MessageField {
    /// Create a new required field
    #[must_use]
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: true,
        }
    }

    /// Create an optional field
    #[must_use]
    pub fn optional(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type: FieldType::Optional(Box::new(field_type)),
            required: false,
        }
    }
}

/// A worker message definition
#[derive(Debug, Clone)]
pub struct BrickWorkerMessage {
    /// Message type name (PascalCase for Rust, lowercase for JS)
    pub name: String,
    /// Direction of the message
    pub direction: BrickWorkerMessageDirection,
    /// Message fields
    pub fields: Vec<MessageField>,
    /// Include trace context for distributed tracing
    pub trace_context: bool,
}

impl BrickWorkerMessage {
    /// Create a new worker message
    #[must_use]
    pub fn new(name: impl Into<String>, direction: BrickWorkerMessageDirection) -> Self {
        Self {
            name: name.into(),
            direction,
            fields: Vec::new(),
            trace_context: true, // Default to including trace context
        }
    }

    /// Add a field to the message
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, field_type: FieldType) -> Self {
        self.fields.push(MessageField::new(name, field_type));
        self
    }

    /// Add an optional field
    #[must_use]
    pub fn optional_field(mut self, name: impl Into<String>, field_type: FieldType) -> Self {
        self.fields.push(MessageField::optional(name, field_type));
        self
    }

    /// Disable trace context for this message
    #[must_use]
    pub fn without_trace(mut self) -> Self {
        self.trace_context = false;
        self
    }

    /// Get the JavaScript type name (lowercase)
    #[must_use]
    pub fn js_type_name(&self) -> String {
        self.name.to_lowercase()
    }

    /// Get the Rust type name (PascalCase)
    #[must_use]
    pub fn rust_type_name(&self) -> String {
        // Convert to PascalCase
        let mut result = String::new();
        let mut capitalize_next = true;
        for c in self.name.chars() {
            if c == '_' || c == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }
        result
    }
}

/// A state transition in the worker state machine
#[derive(Debug, Clone)]
pub struct WorkerTransition {
    /// Source state
    pub from: String,
    /// Message that triggers the transition
    pub message: String,
    /// Target state
    pub to: String,
    /// Optional action to execute
    pub action: Option<String>,
}

impl WorkerTransition {
    /// Create a new transition
    #[must_use]
    pub fn new(from: impl Into<String>, message: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            message: message.into(),
            to: to.into(),
            action: None,
        }
    }

    /// Add an action to the transition
    #[must_use]
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// WorkerBrick: Generates Web Worker code from brick definition
#[derive(Debug, Clone)]
pub struct WorkerBrick {
    /// Worker name
    name: String,
    /// Message definitions
    messages: Vec<BrickWorkerMessage>,
    /// State machine transitions
    transitions: Vec<WorkerTransition>,
    /// Initial state
    initial_state: String,
    /// All states
    states: Vec<String>,
}

impl WorkerBrick {
    /// Create a new worker brick
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            messages: Vec::new(),
            transitions: Vec::new(),
            initial_state: "uninitialized".into(),
            states: vec!["uninitialized".into()],
        }
    }

    /// Add a message definition
    #[must_use]
    pub fn message(mut self, msg: BrickWorkerMessage) -> Self {
        self.messages.push(msg);
        self
    }

    /// Add a state
    #[must_use]
    pub fn state(mut self, state: impl Into<String>) -> Self {
        let state = state.into();
        if !self.states.contains(&state) {
            self.states.push(state);
        }
        self
    }

    /// Set the initial state
    #[must_use]
    pub fn initial(mut self, state: impl Into<String>) -> Self {
        self.initial_state = state.into();
        self
    }

    /// Add a state transition
    #[must_use]
    pub fn transition(
        mut self,
        from: impl Into<String>,
        message: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        let from = from.into();
        let to = to.into();

        // Auto-add states
        if !self.states.contains(&from) {
            self.states.push(from.clone());
        }
        if !self.states.contains(&to) {
            self.states.push(to.clone());
        }

        self.transitions
            .push(WorkerTransition::new(from, message, to));
        self
    }

    /// Add a transition with action
    #[must_use]
    pub fn transition_with_action(
        mut self,
        from: impl Into<String>,
        message: impl Into<String>,
        to: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        let from = from.into();
        let to = to.into();

        if !self.states.contains(&from) {
            self.states.push(from.clone());
        }
        if !self.states.contains(&to) {
            self.states.push(to.clone());
        }

        self.transitions
            .push(WorkerTransition::new(from, message, to).with_action(action));
        self
    }

    /// Get messages sent to worker
    #[must_use]
    pub fn to_worker_messages(&self) -> Vec<&BrickWorkerMessage> {
        self.messages
            .iter()
            .filter(|m| {
                matches!(
                    m.direction,
                    BrickWorkerMessageDirection::ToWorker
                        | BrickWorkerMessageDirection::Bidirectional
                )
            })
            .collect()
    }

    /// Get messages sent from worker
    #[must_use]
    pub fn from_worker_messages(&self) -> Vec<&BrickWorkerMessage> {
        self.messages
            .iter()
            .filter(|m| {
                matches!(
                    m.direction,
                    BrickWorkerMessageDirection::FromWorker
                        | BrickWorkerMessageDirection::Bidirectional
                )
            })
            .collect()
    }

    /// Generate JavaScript Worker code
    #[must_use]
    pub fn to_worker_js(&self) -> String {
        let mut js = String::new();

        // Header
        js.push_str(&format!(
            "// {} Worker (ES Module)\n",
            to_pascal_case(&self.name)
        ));
        js.push_str("// Generated by probar - DO NOT EDIT MANUALLY\n\n");

        // State variable
        js.push_str(&format!("let workerState = '{}';\n\n", self.initial_state));

        // Message handler
        js.push_str("self.onmessage = async (e) => {\n");
        js.push_str("    const msg = e.data;\n");
        js.push_str("    const _trace = msg._trace; // Dapper trace context\n\n");
        js.push_str("    switch (msg.type) {\n");

        // Generate case for each to-worker message
        for msg in self.to_worker_messages() {
            let js_type = msg.js_type_name();

            js.push_str(&format!("        case '{}':\n", js_type));

            // Find transitions triggered by this message
            let transitions: Vec<_> = self
                .transitions
                .iter()
                .filter(|t| t.message.to_lowercase() == js_type)
                .collect();

            if transitions.is_empty() {
                js.push_str(&format!(
                    "            console.log('[Worker] Received {} (no state change)');\n",
                    js_type
                ));
            } else {
                // Generate state machine validation
                let valid_from_states: Vec<_> = transitions
                    .iter()
                    .map(|t| format!("'{}'", t.from))
                    .collect();

                js.push_str(&format!(
                    "            if (![{}].includes(workerState)) {{\n",
                    valid_from_states.join(", ")
                ));
                js.push_str(&format!(
                    "                console.warn('[Worker] Invalid state for {}: ' + workerState);\n",
                    js_type
                ));
                js.push_str("                return;\n");
                js.push_str("            }\n");

                // State transition
                if let Some(t) = transitions.first() {
                    js.push_str(&format!("            workerState = '{}';\n", t.to));
                    if let Some(ref action) = t.action {
                        js.push_str(&format!("            {};\n", action));
                    }
                }
            }

            js.push_str("            break;\n\n");
        }

        // Default case (Yuan Gate - no swallowing)
        js.push_str("        default:\n");
        js.push_str("            throw new Error('[Worker] Unknown message type: ' + msg.type);\n");
        js.push_str("    }\n");
        js.push_str("};\n\n");

        // Helper to post message back
        js.push_str("function postResult(type, data, trace) {\n");
        js.push_str("    self.postMessage({ type, ...data, _trace: trace });\n");
        js.push_str("}\n\n");

        // Log module loaded
        js.push_str(&format!(
            "console.log('[Worker] {} module loaded');\n",
            to_pascal_case(&self.name)
        ));

        js
    }

    /// Generate Rust web_sys bindings
    #[must_use]
    pub fn to_rust_bindings(&self) -> String {
        let mut rust = String::new();

        // Header
        rust.push_str(&format!(
            "//! {} Worker Bindings\n",
            to_pascal_case(&self.name)
        ));
        rust.push_str("//! Generated by probar - DO NOT EDIT MANUALLY\n\n");
        rust.push_str("use serde::{Deserialize, Serialize};\n\n");

        // ToWorker enum
        rust.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        rust.push_str("#[serde(tag = \"type\", rename_all = \"lowercase\")]\n");
        rust.push_str("pub enum ToWorker {\n");

        for msg in self.to_worker_messages() {
            let name = msg.rust_type_name();
            if msg.fields.is_empty() {
                rust.push_str(&format!("    {},\n", name));
            } else {
                rust.push_str(&format!("    {} {{\n", name));
                for field in &msg.fields {
                    let rust_type = field.field_type.to_rust();
                    rust.push_str(&format!(
                        "        {}: {},\n",
                        to_snake_case(&field.name),
                        rust_type
                    ));
                }
                rust.push_str("    },\n");
            }
        }
        rust.push_str("}\n\n");

        // FromWorker enum
        rust.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        rust.push_str("#[serde(tag = \"type\", rename_all = \"lowercase\")]\n");
        rust.push_str("pub enum FromWorker {\n");

        for msg in self.from_worker_messages() {
            let name = msg.rust_type_name();
            if msg.fields.is_empty() {
                rust.push_str(&format!("    {},\n", name));
            } else {
                rust.push_str(&format!("    {} {{\n", name));
                for field in &msg.fields {
                    let rust_type = field.field_type.to_rust();
                    rust.push_str(&format!(
                        "        {}: {},\n",
                        to_snake_case(&field.name),
                        rust_type
                    ));
                }
                rust.push_str("    },\n");
            }
        }
        rust.push_str("}\n\n");

        // State enum
        rust.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
        rust.push_str("pub enum WorkerState {\n");
        for state in &self.states {
            rust.push_str(&format!("    {},\n", to_pascal_case(state)));
        }
        rust.push_str("}\n\n");

        rust.push_str(&format!(
            "impl Default for WorkerState {{\n    fn default() -> Self {{\n        Self::{}\n    }}\n}}\n",
            to_pascal_case(&self.initial_state)
        ));

        rust
    }

    /// Generate TypeScript type definitions
    #[must_use]
    pub fn to_typescript_defs(&self) -> String {
        let mut ts = String::new();

        ts.push_str(&format!("// {} Worker Types\n", to_pascal_case(&self.name)));
        ts.push_str("// Generated by probar - DO NOT EDIT MANUALLY\n\n");

        // Trace context type
        ts.push_str("interface TraceContext {\n");
        ts.push_str("    trace_id: string;\n");
        ts.push_str("    parent_span_id: string;\n");
        ts.push_str("    span_id: string;\n");
        ts.push_str("}\n\n");

        // Message types
        for msg in &self.messages {
            ts.push_str(&format!("interface {}Message {{\n", msg.rust_type_name()));
            ts.push_str(&format!("    type: '{}';\n", msg.js_type_name()));
            for field in &msg.fields {
                let ts_type = field.field_type.to_typescript();
                if field.required {
                    ts.push_str(&format!("    {}: {};\n", field.name, ts_type));
                } else {
                    ts.push_str(&format!("    {}?: {};\n", field.name, ts_type));
                }
            }
            ts.push_str("    _trace?: TraceContext;\n");
            ts.push_str("}\n\n");
        }

        ts
    }
}

impl Brick for WorkerBrick {
    fn brick_name(&self) -> &'static str {
        "WorkerBrick"
    }

    fn assertions(&self) -> &[BrickAssertion] {
        // WorkerBrick assertions are verified by JS validator
        &[]
    }

    fn budget(&self) -> BrickBudget {
        // Worker code generation is not render-bound
        BrickBudget::uniform(1000)
    }

    fn verify(&self) -> BrickVerification {
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        // Verify state machine completeness
        for transition in &self.transitions {
            if !self.states.contains(&transition.from) {
                failed.push((
                    BrickAssertion::Custom {
                        name: "state_exists".into(),
                        validator_id: 1,
                    },
                    format!("State '{}' not defined", transition.from),
                ));
            }
            if !self.states.contains(&transition.to) {
                failed.push((
                    BrickAssertion::Custom {
                        name: "state_exists".into(),
                        validator_id: 1,
                    },
                    format!("State '{}' not defined", transition.to),
                ));
            }
        }

        // Verify messages have corresponding transitions
        for msg in self.to_worker_messages() {
            let has_transition = self
                .transitions
                .iter()
                .any(|t| t.message.to_lowercase() == msg.js_type_name());

            if has_transition {
                passed.push(BrickAssertion::Custom {
                    name: format!("message_{}_handled", msg.name),
                    validator_id: 2,
                });
            } else {
                failed.push((
                    BrickAssertion::Custom {
                        name: format!("message_{}_handled", msg.name),
                        validator_id: 2,
                    },
                    format!("Message '{}' has no state transition", msg.name),
                ));
            }
        }

        if failed.is_empty() {
            passed.push(BrickAssertion::Custom {
                name: "state_machine_valid".into(),
                validator_id: 3,
            });
        }

        BrickVerification {
            passed,
            failed,
            verification_time: Duration::from_micros(100),
        }
    }

    fn to_html(&self) -> String {
        // WorkerBrick doesn't generate HTML
        String::new()
    }

    fn to_css(&self) -> String {
        // WorkerBrick doesn't generate CSS
        String::new()
    }

    fn test_id(&self) -> Option<&str> {
        None
    }
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '-' {
            result.push('_');
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_brick_basic() {
        let worker = WorkerBrick::new("transcription")
            .message(BrickWorkerMessage::new(
                "init",
                BrickWorkerMessageDirection::ToWorker,
            ))
            .message(BrickWorkerMessage::new(
                "ready",
                BrickWorkerMessageDirection::FromWorker,
            ))
            .transition("uninitialized", "init", "ready");

        assert_eq!(worker.name, "transcription");
        assert_eq!(worker.messages.len(), 2);
        assert_eq!(worker.transitions.len(), 1);
    }

    #[test]
    fn test_worker_brick_js_generation() {
        let worker = WorkerBrick::new("test")
            .message(BrickWorkerMessage::new(
                "ping",
                BrickWorkerMessageDirection::ToWorker,
            ))
            .message(BrickWorkerMessage::new(
                "pong",
                BrickWorkerMessageDirection::FromWorker,
            ))
            .transition("uninitialized", "ping", "ready");

        let js = worker.to_worker_js();

        assert!(js.contains("self.onmessage"));
        assert!(js.contains("case 'ping':"));
        assert!(js.contains("workerState = 'ready'"));
        assert!(js.contains("Generated by probar"));
    }

    #[test]
    fn test_worker_brick_rust_bindings() {
        let worker = WorkerBrick::new("test")
            .message(
                BrickWorkerMessage::new("init", BrickWorkerMessageDirection::ToWorker)
                    .field("url", FieldType::String),
            )
            .message(BrickWorkerMessage::new(
                "ready",
                BrickWorkerMessageDirection::FromWorker,
            ))
            .transition("uninitialized", "init", "ready");

        let rust = worker.to_rust_bindings();

        assert!(rust.contains("pub enum ToWorker"));
        assert!(rust.contains("pub enum FromWorker"));
        assert!(rust.contains("pub enum WorkerState"));
        assert!(rust.contains("url: String"));
    }

    #[test]
    fn test_worker_brick_verification() {
        let worker = WorkerBrick::new("test")
            .message(BrickWorkerMessage::new(
                "init",
                BrickWorkerMessageDirection::ToWorker,
            ))
            .transition("uninitialized", "init", "ready");

        let result = worker.verify();
        assert!(result.is_valid());
    }

    #[test]
    fn test_field_type_typescript() {
        assert_eq!(FieldType::String.to_typescript(), "string");
        assert_eq!(FieldType::Number.to_typescript(), "number");
        assert_eq!(FieldType::Boolean.to_typescript(), "boolean");
        assert_eq!(
            FieldType::SharedArrayBuffer.to_typescript(),
            "SharedArrayBuffer"
        );
    }

    #[test]
    fn test_field_type_rust() {
        assert_eq!(FieldType::String.to_rust(), "String");
        assert_eq!(FieldType::Number.to_rust(), "f64");
        assert_eq!(FieldType::Boolean.to_rust(), "bool");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("helloWorld"), "HelloWorld");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("model-url"), "model_url");
    }
}
