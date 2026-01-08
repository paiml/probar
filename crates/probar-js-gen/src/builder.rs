//! Fluent builder API for JavaScript generation.
//!
//! Provides a type-safe, ergonomic API for constructing JavaScript code.
//!
//! # Example
//!
//! ```rust,no_run
//! use probar_js_gen::prelude::*;
//!
//! let js = JsModuleBuilder::new()
//!     .comment("Generated - DO NOT EDIT")
//!     .let_decl("x", Expr::num(42)).unwrap()
//!     .build();
//! ```
//!
//! # References
//! - Bloch (2008) "Effective Java" - Builder pattern
//! - Gamma et al. (1994) "Design Patterns" - Builder

use crate::hir::*;
use crate::Result;

/// Builder for JavaScript modules.
#[derive(Debug, Default)]
pub struct JsModuleBuilder {
    statements: Vec<Stmt>,
    metadata: Option<GenerationMetadata>,
}

impl JsModuleBuilder {
    /// Create a new empty module builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set generation metadata for immutability tracking.
    #[must_use]
    pub fn metadata(mut self, metadata: GenerationMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add a statement.
    #[must_use]
    pub fn stmt(mut self, stmt: Stmt) -> Self {
        self.statements.push(stmt);
        self
    }

    /// Add multiple statements.
    #[must_use]
    pub fn stmts(mut self, stmts: impl IntoIterator<Item = Stmt>) -> Self {
        self.statements.extend(stmts);
        self
    }

    /// Add a comment.
    #[must_use]
    pub fn comment(self, text: impl Into<String>) -> Self {
        self.stmt(Stmt::Comment(text.into()))
    }

    /// Add a let declaration.
    pub fn let_decl(self, name: impl Into<String>, value: Expr) -> Result<Self> {
        let ident = Identifier::new(name)?;
        Ok(self.stmt(Stmt::Let { name: ident, value }))
    }

    /// Add a const declaration.
    pub fn const_decl(self, name: impl Into<String>, value: Expr) -> Result<Self> {
        let ident = Identifier::new(name)?;
        Ok(self.stmt(Stmt::Const { name: ident, value }))
    }

    /// Add an assignment.
    pub fn assign(self, name: impl Into<String>, value: Expr) -> Result<Self> {
        let ident = Identifier::new(name)?;
        Ok(self.stmt(Stmt::Assign { name: ident, value }))
    }

    /// Add an expression statement.
    #[must_use]
    pub fn expr(self, e: Expr) -> Self {
        self.stmt(Stmt::Expr(e))
    }

    /// Add a class definition.
    #[must_use]
    pub fn class(self, class: JsClass) -> Self {
        self.stmt(Stmt::Class(class))
    }

    /// Add a registerProcessor call.
    pub fn register_processor(
        self,
        name: impl Into<String>,
        class_name: impl Into<String>,
    ) -> Result<Self> {
        let class = Identifier::new(class_name)?;
        Ok(self.stmt(Stmt::RegisterProcessor {
            name: name.into(),
            class,
        }))
    }

    /// Build the module.
    #[must_use]
    pub fn build(self) -> JsModule {
        JsModule {
            statements: self.statements,
            metadata: self.metadata,
        }
    }
}

/// Builder for JavaScript classes.
#[derive(Debug)]
pub struct JsClassBuilder {
    name: Identifier,
    extends: Option<Identifier>,
    constructor: Option<Vec<Stmt>>,
    methods: Vec<JsMethod>,
}

impl JsClassBuilder {
    /// Create a new class builder.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        Ok(Self {
            name: Identifier::new(name)?,
            extends: None,
            constructor: None,
            methods: Vec::new(),
        })
    }

    /// Set the parent class.
    pub fn extends(mut self, parent: impl Into<String>) -> Result<Self> {
        self.extends = Some(Identifier::new(parent)?);
        Ok(self)
    }

    /// Set the constructor body.
    #[must_use]
    pub fn constructor(mut self, body: Vec<Stmt>) -> Self {
        self.constructor = Some(body);
        self
    }

    /// Add a method.
    pub fn method(
        mut self,
        name: impl Into<String>,
        params: &[&str],
        body: Vec<Stmt>,
    ) -> Result<Self> {
        let name = Identifier::new(name)?;
        let params = params
            .iter()
            .map(|p| Identifier::new(*p))
            .collect::<Result<Vec<_>>>()?;

        self.methods.push(JsMethod { name, params, body });
        Ok(self)
    }

    /// Build the class.
    #[must_use]
    pub fn build(self) -> JsClass {
        JsClass {
            name: self.name,
            extends: self.extends,
            constructor: self.constructor,
            methods: self.methods,
        }
    }
}

/// Builder for JavaScript switch statements.
#[derive(Debug)]
pub struct JsSwitchBuilder {
    expr: Expr,
    cases: Vec<(Expr, Vec<Stmt>)>,
    default: Option<Vec<Stmt>>,
}

impl JsSwitchBuilder {
    /// Create a new switch builder.
    #[must_use]
    pub fn new(expr: Expr) -> Self {
        Self {
            expr,
            cases: Vec::new(),
            default: None,
        }
    }

    /// Add a case.
    #[must_use]
    pub fn case(mut self, value: Expr, body: Vec<Stmt>) -> Self {
        self.cases.push((value, body));
        self
    }

    /// Set the default case.
    #[must_use]
    pub fn default(mut self, body: Vec<Stmt>) -> Self {
        self.default = Some(body);
        self
    }

    /// Build the switch.
    #[must_use]
    pub fn build(self) -> JsSwitch {
        JsSwitch {
            expr: self.expr,
            cases: self.cases,
            default: self.default,
        }
    }
}

/// Expression builder helpers.
impl Expr {
    /// Create a null literal.
    #[must_use]
    pub const fn null() -> Self {
        Self::Null
    }

    /// Create a boolean literal.
    #[must_use]
    pub const fn bool(v: bool) -> Self {
        Self::Bool(v)
    }

    /// Create a number literal.
    #[must_use]
    pub fn num(v: impl Into<f64>) -> Self {
        Self::Num(v.into())
    }

    /// Create a string literal.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Self {
        Self::Str(s.into())
    }

    /// Create an identifier reference.
    pub fn ident(name: impl Into<String>) -> Result<Self> {
        Ok(Self::Ident(Identifier::new(name)?))
    }

    /// Create `this` reference.
    #[must_use]
    pub const fn this() -> Self {
        Self::This
    }

    /// Member access: `self.prop`
    pub fn dot(self, prop: impl Into<String>) -> Result<Self> {
        Ok(Self::Member {
            object: Box::new(self),
            property: Identifier::new(prop)?,
        })
    }

    /// Computed member: `self[index]`
    #[must_use]
    pub fn index(self, idx: Expr) -> Self {
        Self::Index {
            object: Box::new(self),
            index: Box::new(idx),
        }
    }

    /// Function call.
    #[must_use]
    pub fn call(self, args: Vec<Expr>) -> Self {
        Self::Call {
            callee: Box::new(self),
            args,
        }
    }

    /// New expression.
    #[must_use]
    pub fn new_expr(self, args: Vec<Expr>) -> Self {
        Self::New {
            constructor: Box::new(self),
            args,
        }
    }

    /// Await expression.
    #[must_use]
    pub fn await_expr(self) -> Self {
        Self::Await(Box::new(self))
    }

    /// Dynamic import.
    #[must_use]
    pub fn import(path: Expr) -> Self {
        Self::Import(Box::new(path))
    }

    // Binary operations

    /// Addition: `self + other`
    #[must_use]
    pub fn add(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Add,
            right: Box::new(other),
        }
    }

    /// Subtraction: `self - other`
    #[must_use]
    pub fn sub(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Sub,
            right: Box::new(other),
        }
    }

    /// Multiplication: `self * other`
    #[must_use]
    pub fn mul(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Mul,
            right: Box::new(other),
        }
    }

    /// Division: `self / other`
    #[must_use]
    pub fn div(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Div,
            right: Box::new(other),
        }
    }

    /// Modulo: `self % other`
    #[must_use]
    pub fn modulo(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Mod,
            right: Box::new(other),
        }
    }

    /// Strict equality: `self === other`
    #[must_use]
    pub fn eq(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::EqStrict,
            right: Box::new(other),
        }
    }

    /// Strict inequality: `self !== other`
    #[must_use]
    pub fn ne(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::NeStrict,
            right: Box::new(other),
        }
    }

    /// Less than: `self < other`
    #[must_use]
    pub fn lt(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Lt,
            right: Box::new(other),
        }
    }

    /// Less than or equal: `self <= other`
    #[must_use]
    pub fn le(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Le,
            right: Box::new(other),
        }
    }

    /// Greater than: `self > other`
    #[must_use]
    pub fn gt(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Gt,
            right: Box::new(other),
        }
    }

    /// Greater than or equal: `self >= other`
    #[must_use]
    pub fn ge(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Ge,
            right: Box::new(other),
        }
    }

    /// Logical and: `self && other`
    #[must_use]
    pub fn and(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::And,
            right: Box::new(other),
        }
    }

    /// Logical or: `self || other`
    #[must_use]
    pub fn or(self, other: Expr) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: BinOp::Or,
            right: Box::new(other),
        }
    }

    // Unary operations

    /// Logical not: `!self`
    #[must_use]
    pub fn not(self) -> Self {
        Self::Unary {
            op: UnaryOp::Not,
            operand: Box::new(self),
        }
    }

    /// Negation: `-self`
    #[must_use]
    pub fn neg(self) -> Self {
        Self::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(self),
        }
    }

    /// Typeof: `typeof self`
    #[must_use]
    pub fn type_of(self) -> Self {
        Self::Unary {
            op: UnaryOp::TypeOf,
            operand: Box::new(self),
        }
    }

    /// Ternary: `self ? then : else`
    #[must_use]
    pub fn ternary(self, then_expr: Expr, else_expr: Expr) -> Self {
        Self::Ternary {
            condition: Box::new(self),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        }
    }

    /// Object literal.
    #[must_use]
    pub fn object(pairs: Vec<(&str, Expr)>) -> Self {
        Self::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    /// Array literal.
    #[must_use]
    pub fn array(items: Vec<Expr>) -> Self {
        Self::Array(items)
    }

    /// Arrow function.
    pub fn arrow(params: &[&str], body: Expr) -> Result<Self> {
        let params = params
            .iter()
            .map(|p| Identifier::new(*p))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::Arrow {
            params,
            body: Box::new(body),
        })
    }

    /// Arrow function with block body.
    pub fn arrow_block(params: &[&str], body: Vec<Stmt>) -> Result<Self> {
        let params = params
            .iter()
            .map(|p| Identifier::new(*p))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::ArrowBlock { params, body })
    }

    /// Assignment: `self = value`
    #[must_use]
    pub fn assign(self, value: Expr) -> Self {
        Self::Assign {
            target: Box::new(self),
            value: Box::new(value),
        }
    }
}

/// Statement builder helpers.
impl Stmt {
    /// Create a let declaration.
    pub fn let_decl(name: impl Into<String>, value: Expr) -> Result<Self> {
        Ok(Self::Let {
            name: Identifier::new(name)?,
            value,
        })
    }

    /// Create a const declaration.
    pub fn const_decl(name: impl Into<String>, value: Expr) -> Result<Self> {
        Ok(Self::Const {
            name: Identifier::new(name)?,
            value,
        })
    }

    /// Create an assignment.
    pub fn assign(name: impl Into<String>, value: Expr) -> Result<Self> {
        Ok(Self::Assign {
            name: Identifier::new(name)?,
            value,
        })
    }

    /// Create a member assignment.
    pub fn member_assign(obj: Expr, member: impl Into<String>, value: Expr) -> Result<Self> {
        Ok(Self::MemberAssign {
            object: obj,
            member: Identifier::new(member)?,
            value,
        })
    }

    /// Create an add-assign.
    #[must_use]
    pub fn add_assign(target: Expr, value: Expr) -> Self {
        Self::AddAssign { target, value }
    }

    /// Create a post-increment.
    #[must_use]
    pub fn post_increment(expr: Expr) -> Self {
        Self::PostIncrement(expr)
    }

    /// Create an expression statement.
    #[must_use]
    pub fn expr(e: Expr) -> Self {
        Self::Expr(e)
    }

    /// Create a return statement.
    #[must_use]
    pub fn ret() -> Self {
        Self::Return(None)
    }

    /// Create a return with value.
    #[must_use]
    pub fn ret_val(e: Expr) -> Self {
        Self::Return(Some(e))
    }

    /// Create an if statement.
    #[must_use]
    pub fn if_then(cond: Expr, then_body: Vec<Stmt>) -> Self {
        Self::If {
            condition: cond,
            then_branch: then_body,
            else_branch: None,
        }
    }

    /// Create an if-else statement.
    #[must_use]
    pub fn if_else(cond: Expr, then_body: Vec<Stmt>, else_body: Vec<Stmt>) -> Self {
        Self::If {
            condition: cond,
            then_branch: then_body,
            else_branch: Some(else_body),
        }
    }

    /// Create a for loop.
    pub fn for_loop(
        var: impl Into<String>,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    ) -> Result<Self> {
        Ok(Self::For {
            var: Identifier::new(var)?,
            start,
            end,
            body,
        })
    }

    /// Create a while loop.
    #[must_use]
    pub fn while_loop(cond: Expr, body: Vec<Stmt>) -> Self {
        Self::While {
            condition: cond,
            body,
        }
    }

    /// Create a try-catch.
    pub fn try_catch(
        body: Vec<Stmt>,
        catch_var: impl Into<String>,
        handler: Vec<Stmt>,
    ) -> Result<Self> {
        Ok(Self::TryCatch {
            body,
            catch_var: Identifier::new(catch_var)?,
            handler,
        })
    }

    /// Create a comment.
    #[must_use]
    pub fn comment(text: impl Into<String>) -> Self {
        Self::Comment(text.into())
    }

    /// Create an onmessage handler.
    #[must_use]
    pub fn on_message(body: Vec<Stmt>) -> Self {
        Self::OnMessage(body)
    }

    /// Create a registerProcessor call.
    pub fn register_processor(name: impl Into<String>, class: impl Into<String>) -> Result<Self> {
        Ok(Self::RegisterProcessor {
            name: name.into(),
            class: Identifier::new(class)?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn module_builder_basic() {
        let module = JsModuleBuilder::new()
            .comment("test")
            .let_decl("x", Expr::num(42))
            .unwrap()
            .build();

        assert_eq!(module.statements.len(), 2);
    }

    #[test]
    fn class_builder_basic() -> Result<()> {
        let class = JsClassBuilder::new("Foo")?
            .extends("Bar")?
            .constructor(vec![])
            .method("baz", &["x", "y"], vec![Stmt::ret()])?
            .build();

        assert_eq!(class.name.as_str(), "Foo");
        assert_eq!(
            class
                .extends
                .as_ref()
                .map(super::super::hir::Identifier::as_str),
            Some("Bar")
        );
        assert_eq!(class.methods.len(), 1);
        Ok(())
    }

    #[test]
    fn expr_builder_chain() -> Result<()> {
        let expr = Expr::ident("foo")?.dot("bar")?.call(vec![Expr::num(1)]);
        match expr {
            Expr::Call { callee, args } => {
                assert_eq!(args.len(), 1);
                match *callee {
                    Expr::Member { property, .. } => {
                        assert_eq!(property.as_str(), "bar");
                    }
                    _ => panic!("expected member"),
                }
            }
            _ => panic!("expected call"),
        }
        Ok(())
    }

    #[test]
    fn identifier_validation() {
        assert!(Identifier::new("validName").is_ok());
        assert!(Identifier::new("class").is_err());
        assert!(Identifier::new("123invalid").is_err());
    }
}
