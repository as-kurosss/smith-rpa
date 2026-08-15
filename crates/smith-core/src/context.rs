// crates/smith-core/src/context.rs
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::tool::ToolError;

// ---------------------------------------------------------------------------
// State markers for ExecutionContext typestate (§2.3)
// ---------------------------------------------------------------------------

/// Initial state: context has been created but not yet validated.
#[derive(Debug, Clone, Copy)]
pub struct Unvalidated;

/// Ready state: context is validated and can execute tools.
#[derive(Debug, Clone, Copy)]
pub struct Ready;

// ---------------------------------------------------------------------------
// ContextValue — algebraic data type for storing values
// ---------------------------------------------------------------------------

/// Algebraic data type for storing values in context.
#[derive(Debug, Clone)]
pub enum ContextValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<ContextValue>),
    Bytes(Vec<u8>),
    /// Platform-specific objects (e.g., `UIElement` from `smith-windows`).
    Custom(Arc<dyn Any + Send + Sync>),
    Null,
}

impl PartialEq for ContextValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            (Self::Null, Self::Null) => true,
            // Custom variant: value comparison not possible through `dyn Any`
            (Self::Custom(_), Self::Custom(_)) => false,
            _ => false,
        }
    }
}

impl ContextValue {
    /// Extracts a string value.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidInput` if the value is not a `String`.
    pub fn try_as_string(&self) -> Result<&str, ToolError> {
        match self {
            ContextValue::String(s) => Ok(s.as_str()),
            _ => Err(ToolError::invalid_input("Expected String", None, None)),
        }
    }

    /// Extracts a numeric value.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidInput` if the value is not a `Number`.
    pub fn try_as_number(&self) -> Result<f64, ToolError> {
        match self {
            ContextValue::Number(n) => Ok(*n),
            _ => Err(ToolError::invalid_input("Expected Number", None, None)),
        }
    }

    /// Extracts a boolean value.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidInput` if the value is not a `Boolean`.
    pub fn try_as_boolean(&self) -> Result<bool, ToolError> {
        match self {
            ContextValue::Boolean(b) => Ok(*b),
            _ => Err(ToolError::invalid_input("Expected Boolean", None, None)),
        }
    }

    /// Extracts a custom type via `Any`.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidInput` if the value is not `Custom`
    /// or the inner type does not match the requested `T`.
    pub fn try_as_custom<T: 'static>(&self) -> Result<&T, ToolError> {
        match self {
            ContextValue::Custom(arc) => arc
                .downcast_ref::<T>()
                .ok_or_else(|| ToolError::invalid_input("Custom type mismatch", None, None)),
            _ => Err(ToolError::invalid_input("Expected Custom type", None, None)),
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionContext with typestate (§2.3)
// ---------------------------------------------------------------------------

/// Execution context with a scope stack for variable isolation.
///
/// The type parameter `State` encodes the context state at compile time:
/// - [`Unvalidated`] — freshly created, not yet ready for execution
/// - [`Ready`] — validated and ready for tool execution
///
/// # Typestate transitions
/// - `ExecutionContext<Unvalidated>::new()` → `Unvalidated`
/// - `ExecutionContext<Unvalidated>::validate()` → `ExecutionContext<Ready>`
pub struct ExecutionContext<State = Ready> {
    scopes: Vec<HashMap<String, ContextValue>>,
    _state: PhantomData<State>,
}

// ---------------------------------------------------------------------------
// Construction (Unvalidated state only)
// ---------------------------------------------------------------------------

impl ExecutionContext<Unvalidated> {
    /// Creates a new context with a global scope in the `Unvalidated` state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            _state: PhantomData,
        }
    }

    /// Validates the context and transitions to the `Ready` state.
    ///
    /// Currently a no-op; reserved for future validation logic.
    #[must_use]
    pub fn validate(self) -> ExecutionContext<Ready> {
        ExecutionContext {
            scopes: self.scopes,
            _state: PhantomData,
        }
    }
}

impl Default for ExecutionContext<Unvalidated> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// I/O operations — available in any state
// ---------------------------------------------------------------------------

impl<State> ExecutionContext<State> {
    /// Creates a new local scope (e.g., when entering a loop or function).
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Destroys the current local scope, freeing memory from temporary variables.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Writes a variable to the current (topmost) scope.
    pub fn set(&mut self, key: impl Into<String>, value: ContextValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key.into(), value);
        }
    }

    /// Reads a variable, starting search from local scope to global (LIFO).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(key) {
                return Some(value);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_scope() {
        let ctx = ExecutionContext::new();
        assert!(ctx.get("any_key").is_none());
    }

    #[test]
    fn test_set_and_get_variable() {
        let mut ctx = ExecutionContext::new();
        ctx.set("key1", ContextValue::String("value1".into()));
        assert_eq!(
            ctx.get("key1").and_then(|v| v.try_as_string().ok()),
            Some("value1")
        );
    }

    #[test]
    fn test_get_returns_none_for_missing_key() {
        let ctx = ExecutionContext::new();
        assert!(ctx.get("nonexistent").is_none());
    }

    #[test]
    fn test_push_scope_isolation() {
        let mut ctx = ExecutionContext::new();
        ctx.set("global", ContextValue::String("global_val".into()));

        ctx.push_scope();
        ctx.set("local", ContextValue::Number(42.0));

        // Both visible in inner scope
        assert_eq!(
            ctx.get("global").and_then(|v| v.try_as_string().ok()),
            Some("global_val")
        );
        assert_eq!(
            ctx.get("local").and_then(|v| v.try_as_number().ok()),
            Some(42.0)
        );

        ctx.pop_scope();

        // After pop, local is gone, global remains
        assert!(ctx.get("local").is_none());
        assert_eq!(
            ctx.get("global").and_then(|v| v.try_as_string().ok()),
            Some("global_val")
        );
    }

    #[test]
    fn test_pop_scope_does_not_remove_global() {
        let mut ctx = ExecutionContext::new();
        ctx.set("key", ContextValue::String("val".into()));
        ctx.pop_scope();
        assert_eq!(
            ctx.get("key").and_then(|v| v.try_as_string().ok()),
            Some("val")
        );
    }

    #[test]
    fn test_context_value_try_as_string() {
        let val = ContextValue::String("hello".into());
        assert_eq!(val.try_as_string().ok(), Some("hello"));

        let val = ContextValue::Number(42.0);
        assert!(val.try_as_string().is_err());
    }

    #[test]
    fn test_context_value_try_as_number() {
        let val = ContextValue::Number(std::f64::consts::PI);
        let result = val.try_as_number();
        assert!(result.is_ok_and(|n| (n - std::f64::consts::PI).abs() < f64::EPSILON));

        let val = ContextValue::Boolean(true);
        assert!(val.try_as_number().is_err());
    }

    #[test]
    fn test_context_value_try_as_boolean() {
        let val = ContextValue::Boolean(true);
        assert_eq!(val.try_as_boolean().ok(), Some(true));

        let val = ContextValue::Null;
        assert!(val.try_as_boolean().is_err());
    }

    #[test]
    fn test_context_value_null() {
        let val = ContextValue::Null;
        assert!(val.try_as_string().is_err());
        assert!(val.try_as_number().is_err());
        assert!(val.try_as_boolean().is_err());
    }
}
