//! AST-based State Synchronization Linter (PROBAR-WASM-003)
//!
//! Uses `syn` crate for proper Rust AST traversal to detect Rc<RefCell<T>> patterns.
//! This replaces the text-based pattern matching approach that was vulnerable to:
//! - Turbofish syntax bypass (`Alias::<T>::new()`)
//! - Alternative constructors (`Rc::default()`, `Rc::from()`)
//! - Method chaining (`.to_rc()`)
//! - Unusual whitespace/formatting

use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ItemFn, ItemType, Pat, ReturnType, Type};

use super::{LintError, LintSeverity, StateSyncReport};

/// Patterns that indicate Rc creation
const RC_CONSTRUCTORS: &[&str] = &["new", "default", "from", "clone"];

/// HOTFIX PROBAR-WASM-003: Patterns that indicate unsafe Rc reconstruction
/// These methods create Rc from raw pointers - used to launder Rc past linter detection
const UNSAFE_RC_CONSTRUCTORS: &[&str] = &["from_raw", "increment_strong_count"];

/// Patterns that indicate a function creates closures
const CLOSURE_CREATOR_NAMES: &[&str] = &[
    "spawn",
    "start",
    "on_message",
    "on_click",
    "on_event",
    "set_callback",
    "register",
    "subscribe",
    "listen",
    "wrap",
];

/// AST-based visitor for detecting state sync issues
pub struct AstStateSyncVisitor<'a> {
    /// Current file being analyzed
    pub file: String,
    /// Collected lint errors
    pub errors: Vec<LintError>,
    /// Type aliases that resolve to Rc
    pub rc_type_aliases: HashSet<String>,
    /// Functions that return Rc
    pub rc_returning_functions: HashSet<String>,
    /// Current function context
    current_function: Option<String>,
    /// Whether current function likely creates closures
    fn_creates_closure: bool,
    /// Local variables that are Rc types
    local_rc_vars: HashSet<String>,
    /// Source code for line lookups
    _source: &'a str,
}

impl std::fmt::Debug for AstStateSyncVisitor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstStateSyncVisitor")
            .field("file", &self.file)
            .field("errors_count", &self.errors.len())
            .field("rc_type_aliases", &self.rc_type_aliases)
            .field("rc_returning_functions", &self.rc_returning_functions)
            .finish()
    }
}

impl<'a> AstStateSyncVisitor<'a> {
    /// Create a new AST visitor
    pub fn new(file: String, source: &'a str) -> Self {
        Self {
            file,
            errors: Vec::new(),
            rc_type_aliases: HashSet::new(),
            rc_returning_functions: HashSet::new(),
            current_function: None,
            fn_creates_closure: false,
            local_rc_vars: HashSet::new(),
            _source: source,
        }
    }

    /// Convert a span to line number
    fn span_to_line(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    /// Check if a type is Rc-like
    fn is_rc_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                return name == "Rc" || name == "Arc";
            }
        }
        false
    }

    /// Check if a type path refers to an Rc type alias
    #[allow(dead_code)] // Reserved for future use
    fn is_rc_alias(&self, path: &syn::Path) -> bool {
        if let Some(segment) = path.segments.first() {
            let name = segment.ident.to_string();
            return self.rc_type_aliases.contains(&name);
        }
        false
    }

    /// Extract the base type name from a path (handles turbofish)
    #[allow(dead_code)] // Reserved for future use
    fn extract_type_name(path: &syn::Path) -> Option<String> {
        path.segments.first().map(|s| s.ident.to_string())
    }

    /// Check if this is an Rc constructor call
    fn is_rc_constructor_call(&self, expr: &ExprCall) -> Option<(String, String)> {
        if let Expr::Path(path_expr) = &*expr.func {
            let path = &path_expr.path;

            // Check for Rc::new, Rc::default, Rc::from
            if path.segments.len() >= 2 {
                let type_name = path.segments[0].ident.to_string();
                let method_name = path.segments.last().map(|s| s.ident.to_string());

                // Direct Rc::new() or Arc::new()
                if (type_name == "Rc" || type_name == "Arc")
                    && method_name
                        .as_ref()
                        .map(|m| RC_CONSTRUCTORS.contains(&m.as_str()))
                        .unwrap_or(false)
                {
                    return Some((type_name, method_name.unwrap_or_default()));
                }

                // HOTFIX PROBAR-WASM-003: Detect Rc::from_raw - raw pointer laundering
                if (type_name == "Rc" || type_name == "Arc" || type_name == "Weak")
                    && method_name
                        .as_ref()
                        .map(|m| UNSAFE_RC_CONSTRUCTORS.contains(&m.as_str()))
                        .unwrap_or(false)
                {
                    return Some((type_name, method_name.unwrap_or_default()));
                }

                // Type alias ::new() - handles turbofish like Alias::<T>::new()
                if self.rc_type_aliases.contains(&type_name)
                    && method_name
                        .as_ref()
                        .map(|m| RC_CONSTRUCTORS.contains(&m.as_str()))
                        .unwrap_or(false)
                {
                    return Some((type_name, method_name.unwrap_or_default()));
                }
            }

            // Check for single-segment paths that match known Rc-returning functions
            if path.segments.len() == 1 {
                let fn_name = path.segments[0].ident.to_string();
                if self.rc_returning_functions.contains(&fn_name) {
                    return Some((fn_name, "call".to_string()));
                }
            }
        }
        None
    }

    /// Check if a method is an unsafe Rc reconstruction (WASM-SS-009)
    fn is_unsafe_rc_reconstruction(&self, method_name: &str) -> bool {
        UNSAFE_RC_CONSTRUCTORS.contains(&method_name)
    }

    /// HOTFIX PROBAR-WASM-003: Unwrap unsafe blocks to detect Rc::from_raw
    /// Pattern: `unsafe { Rc::from_raw(ptr) }` wraps the call in Expr::Unsafe
    fn unwrap_unsafe_block(expr: &Expr) -> &Expr {
        match expr {
            Expr::Unsafe(unsafe_block) => {
                // Get the last expression from the unsafe block (the return value)
                if let Some(syn::Stmt::Expr(inner_expr, _)) = unsafe_block.block.stmts.last() {
                    // Recursively unwrap in case of nested unsafe
                    return Self::unwrap_unsafe_block(inner_expr);
                }
                expr
            }
            Expr::Block(block_expr) => {
                // Handle regular block expressions too
                if let Some(syn::Stmt::Expr(inner_expr, _)) = block_expr.block.stmts.last() {
                    return Self::unwrap_unsafe_block(inner_expr);
                }
                expr
            }
            other => other,
        }
    }

    /// Check if a method call returns Rc (e.g., .to_rc(), .clone())
    fn is_rc_method_call(&self, method_call: &ExprMethodCall) -> bool {
        let method_name = method_call.method.to_string();
        // Methods that commonly return Rc
        method_name == "to_rc"
            || method_name == "into_rc"
            || method_name == "as_rc"
            || method_name == "wrap_rc"
            // clone() on an Rc variable
            || (method_name == "clone" && self.expr_is_rc(&method_call.receiver))
    }

    /// Check if an expression refers to an Rc variable
    fn expr_is_rc(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    return self.local_rc_vars.contains(&ident.to_string());
                }
                false
            }
            Expr::Field(field) => {
                // Check if it's self.something that's an Rc
                if let Expr::Path(path) = &*field.base {
                    if path.path.is_ident("self") {
                        // Assume self fields might be Rc - this is conservative
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if function name suggests it creates closures
    fn function_creates_closure(name: &str) -> bool {
        CLOSURE_CREATOR_NAMES
            .iter()
            .any(|&pattern| name.contains(pattern))
    }

    /// Record an error for local Rc creation
    fn report_local_rc(&mut self, var_name: &str, constructor: &str, line: usize, rule: &str) {
        let fn_name = self
            .current_function
            .clone()
            .unwrap_or_else(|| "<unknown>".to_string());

        self.errors.push(LintError {
            rule: rule.to_string(),
            message: format!(
                "Local `{var_name}` created via `{constructor}` in `{fn_name}()` - \
                 if captured by closure, it will be disconnected from self"
            ),
            file: self.file.clone(),
            line,
            column: 1,
            severity: LintSeverity::Error,
            suggestion: Some(format!(
                "Use `let {var_name}_clone = self.{var_name}.clone()` instead"
            )),
        });
    }
}

impl<'ast> Visit<'ast> for AstStateSyncVisitor<'_> {
    fn visit_item_type(&mut self, node: &'ast ItemType) {
        // Detect: type Foo = Rc<...> or type Foo<T> = Rc<RefCell<T>>
        if Self::is_rc_type(&node.ty) {
            let alias_name = node.ident.to_string();
            self.rc_type_aliases.insert(alias_name.clone());

            self.errors.push(LintError {
                rule: "WASM-SS-006".to_string(),
                message: format!(
                    "Type alias `{alias_name}` wraps Rc - usage with constructors may cause state desync"
                ),
                file: self.file.clone(),
                line: self.span_to_line(node.ident.span()),
                column: 1,
                severity: LintSeverity::Info,
                suggestion: Some("Consider using self.field.clone() pattern instead".to_string()),
            });
        }

        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.visit_signature(&node.sig);

        // Check return type for Rc
        if let ReturnType::Type(_, ty) = &node.sig.output {
            if Self::is_rc_type(ty) {
                let fn_name = node.sig.ident.to_string();
                self.rc_returning_functions.insert(fn_name.clone());

                self.errors.push(LintError {
                    rule: "WASM-SS-007".to_string(),
                    message: format!(
                        "Function `{fn_name}` returns Rc - callers may create disconnected state"
                    ),
                    file: self.file.clone(),
                    line: self.span_to_line(node.sig.ident.span()),
                    column: 1,
                    severity: LintSeverity::Info,
                    suggestion: Some(
                        "Document that callers should use self.field.clone() instead".to_string(),
                    ),
                });
            }
        }

        // Enter function context
        let fn_name = node.sig.ident.to_string();
        self.current_function = Some(fn_name.clone());
        self.fn_creates_closure = Self::function_creates_closure(&fn_name);
        self.local_rc_vars.clear();

        // Visit function body
        syn::visit::visit_item_fn(self, node);

        // Exit function context
        self.current_function = None;
        self.fn_creates_closure = false;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        // Check return type for Rc
        if let ReturnType::Type(_, ty) = &node.sig.output {
            if Self::is_rc_type(ty) {
                let fn_name = node.sig.ident.to_string();
                self.rc_returning_functions.insert(fn_name.clone());

                self.errors.push(LintError {
                    rule: "WASM-SS-007".to_string(),
                    message: format!(
                        "Method `{fn_name}` returns Rc - callers may create disconnected state"
                    ),
                    file: self.file.clone(),
                    line: self.span_to_line(node.sig.ident.span()),
                    column: 1,
                    severity: LintSeverity::Info,
                    suggestion: Some(
                        "Document that callers should use self.field.clone() instead".to_string(),
                    ),
                });
            }
        }

        // Enter method context
        let fn_name = node.sig.ident.to_string();
        self.current_function = Some(fn_name.clone());
        self.fn_creates_closure = Self::function_creates_closure(&fn_name);
        self.local_rc_vars.clear();

        // Visit method body
        syn::visit::visit_impl_item_fn(self, node);

        // Exit method context
        self.current_function = None;
        self.fn_creates_closure = false;
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        // Check for local variable assignments
        if let Some(init) = &node.init {
            let var_name = match &node.pat {
                Pat::Ident(pat_ident) => Some(pat_ident.ident.to_string()),
                Pat::Type(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(var_name) = var_name {
                // Extract the actual expression, unwrapping unsafe blocks
                let inner_expr = Self::unwrap_unsafe_block(&init.expr);

                // Check for Rc constructor calls
                if let Expr::Call(call) = inner_expr {
                    if let Some((type_name, method)) = self.is_rc_constructor_call(call) {
                        self.local_rc_vars.insert(var_name.clone());

                        // Only report if in closure-creating context
                        if self.fn_creates_closure {
                            // HOTFIX PROBAR-WASM-003: Detect from_raw laundering
                            let rule = if self.is_unsafe_rc_reconstruction(&method) {
                                "WASM-SS-009" // Unsafe Rc reconstruction from raw pointer
                            } else if type_name == "Rc" || type_name == "Arc" {
                                "WASM-SS-001" // Direct Rc::new
                            } else if self.rc_type_aliases.contains(&type_name) {
                                "WASM-SS-006" // Type alias
                            } else {
                                "WASM-SS-007" // Function returning Rc
                            };

                            self.report_local_rc(
                                &var_name,
                                &format!("{type_name}::{method}"),
                                self.span_to_line(node.let_token.span),
                                rule,
                            );
                        }
                    }
                }

                // Check for method calls that return Rc (.to_rc(), etc.)
                if let Expr::MethodCall(method_call) = &*init.expr {
                    if self.is_rc_method_call(method_call) {
                        self.local_rc_vars.insert(var_name.clone());

                        if self.fn_creates_closure {
                            let method_name = method_call.method.to_string();
                            self.errors.push(LintError {
                                rule: "WASM-SS-008".to_string(),
                                message: format!(
                                    "Method `.{method_name}()` returns Rc - local `{var_name}` \
                                     may cause state desync if captured in closure"
                                ),
                                file: self.file.clone(),
                                line: self.span_to_line(node.let_token.span),
                                column: 1,
                                severity: LintSeverity::Warning,
                                suggestion: Some(
                                    "Clone from self instead of creating new Rc".to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        syn::visit::visit_local(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        // When we encounter a closure, mark that this function creates closures
        self.fn_creates_closure = true;

        // Continue visiting
        syn::visit::visit_expr_closure(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        // Check for Closure::wrap, Closure::once etc.
        if let Expr::Path(path) = &*node.func {
            if let Some(segment) = path.path.segments.first() {
                if segment.ident == "Closure" {
                    self.fn_creates_closure = true;
                }
            }
        }

        syn::visit::visit_expr_call(self, node);
    }
}

/// Parse and lint source code using AST analysis
pub fn lint_source_ast(source: &str, file: &str) -> Result<StateSyncReport, String> {
    let syntax = syn::parse_file(source).map_err(|e| format!("Parse error: {e}"))?;

    let mut visitor = AstStateSyncVisitor::new(file.to_string(), source);

    // First pass: collect type aliases and function signatures
    for item in &syntax.items {
        visitor.visit_item(item);
    }

    Ok(StateSyncReport {
        errors: visitor.errors,
        files_analyzed: 1,
        lines_analyzed: source.lines().count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_detect_type_alias() {
        let source = r#"
type StatePtr = Rc<RefCell<State>>;
type GenericWrapper<T> = Rc<RefCell<T>>;
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        let ss006: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-006")
            .collect();

        assert!(
            ss006.len() >= 2,
            "Should detect both type aliases. Found: {:?}",
            ss006
        );
    }

    #[test]
    fn test_ast_detect_turbofish_constructor() {
        let source = r#"
type MyWrapper<T> = Rc<RefCell<T>>;

fn spawn() {
    let state = MyWrapper::<State>::new(RefCell::new(State {}));
    let cb = move || { state.borrow_mut(); };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        // Should detect the type alias usage even with turbofish
        let errors: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-006" && e.message.contains("MyWrapper"))
            .collect();

        assert!(
            !errors.is_empty(),
            "Should detect turbofish type alias usage. Errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn test_ast_detect_rc_default() {
        let source = r#"
fn spawn() {
    let state: Rc<RefCell<i32>> = Rc::default();
    let cb = move || { *state.borrow_mut() += 1; };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        let ss001: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-001")
            .collect();

        assert!(
            !ss001.is_empty(),
            "Should detect Rc::default(). Errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn test_ast_detect_rc_returning_function() {
        let source = r#"
fn make_state() -> Rc<RefCell<State>> {
    Rc::new(RefCell::new(State {}))
}

fn spawn() {
    let state = make_state();
    let cb = move || { state.borrow_mut(); };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        let ss007: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-007")
            .collect();

        assert!(
            !ss007.is_empty(),
            "Should detect function returning Rc. Errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn test_ast_detect_method_chain() {
        let source = r#"
trait ToRc {
    fn to_rc(self) -> Rc<RefCell<Self>> where Self: Sized;
}

fn spawn() {
    let state = value.to_rc();
    let cb = move || { state.borrow_mut(); };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        let ss008: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-008")
            .collect();

        assert!(
            !ss008.is_empty(),
            "Should detect .to_rc() method chain. Errors: {:?}",
            report.errors
        );
    }

    #[test]
    fn test_ast_correct_pattern_no_error() {
        let source = r#"
impl Worker {
    fn spawn(&mut self) {
        // CORRECT: Clone from self
        let state_clone = self.state.clone();
        let cb = move || { state_clone.borrow_mut(); };
    }
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        let errors: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.severity == LintSeverity::Error)
            .collect();

        assert!(
            errors.is_empty(),
            "Should not report errors for correct pattern. Errors: {:?}",
            errors
        );
    }

    /// HOTFIX PROBAR-WASM-003: Test detection of raw pointer laundering
    #[test]
    fn test_ast_detect_from_raw_laundering() {
        let source = r#"
fn spawn() {
    // ATTACK: Launder Rc through raw pointers
    let ptr = Rc::into_raw(Rc::new(RefCell::new(0)));
    let state = unsafe { Rc::from_raw(ptr) };

    let cb = move || { state.borrow_mut(); };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        // Should detect WASM-SS-009 for Rc::from_raw
        let ss009: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-009")
            .collect();

        assert!(
            !ss009.is_empty(),
            "Should detect Rc::from_raw() laundering. Errors: {:?}",
            report.errors
        );
    }

    /// HOTFIX PROBAR-WASM-003: Test detection of Weak::from_raw
    #[test]
    fn test_ast_detect_weak_from_raw() {
        let source = r#"
fn spawn() {
    let weak_ptr = Weak::into_raw(Rc::downgrade(&rc));
    let weak = unsafe { Weak::from_raw(weak_ptr) };

    let cb = move || { weak.upgrade(); };
}
"#;

        let report = lint_source_ast(source, "test.rs").expect("parse failed");

        // Should detect WASM-SS-009 for Weak::from_raw
        let ss009: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.rule == "WASM-SS-009")
            .collect();

        assert!(
            !ss009.is_empty(),
            "Should detect Weak::from_raw() laundering. Errors: {:?}",
            report.errors
        );
    }
}
