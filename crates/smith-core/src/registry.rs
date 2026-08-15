// crates/smith-core/src/registry.rs
use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

use crate::context::{ExecutionContext, Ready};
use crate::tool::{DynTool, Tool, ToolError};

/// Tool registry for centralized management and execution.
///
/// Uses `DynTool` (object-safe wrapper) for dynamic dispatch, allowing
/// the registry to work with any `T: Tool` via its blanket `DynTool` impl.
pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn DynTool>>,
}

impl ToolRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Registers a new tool by the name returned by `Tool::name()`.
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name();
        self.tools.insert(name, Box::new(tool));
    }

    /// Returns a reference to a registered tool by name, if it exists.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn DynTool> {
        self.tools.get(name).map(AsRef::as_ref)
    }

    /// Executes a tool by name with JSON parameters.
    ///
    /// Only available when the `ExecutionContext` is in the [`Ready`] state (§2.3).
    ///
    /// # Arguments
    /// * `name` - Tool name (e.g., `"windows.click"`)
    /// * `config` - JSON Value parameters
    /// * `ctx` - Execution context in Ready state
    /// * `token` - Cancellation token for graceful shutdown
    ///
    /// # Errors
    ///
    /// Returns `ToolError::InvalidInput` if the tool is not found,
    /// or the tool execution error.
    pub async fn execute(
        &self,
        name: &str,
        config: serde_json::Value,
        ctx: &mut ExecutionContext<Ready>,
        token: CancellationToken,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self.get(name).ok_or_else(|| {
            ToolError::invalid_input(
                format!("Tool '{name}' not found"),
                None,
                Some(config.clone()),
            )
        })?;
        tool.execute(config, ctx, token).await
    }

    /// Returns the list of names of all registered tools.
    #[must_use]
    pub fn list_tools(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::json;

    use crate::context::{ContextValue, Unvalidated};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestInput {
        param: i32,
    }

    #[derive(Debug, Serialize)]
    struct TestOutput {
        status: &'static str,
    }

    struct TestTool {
        name: &'static str,
    }

    #[async_trait]
    impl Tool for TestTool {
        type Input = TestInput;
        type Output = TestOutput;

        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            "A test tool"
        }

        fn schema(&self) -> serde_json::Value {
            json!({})
        }

        async fn execute(
            &self,
            _input: TestInput,
            ctx: &mut ExecutionContext,
            _token: CancellationToken,
        ) -> Result<TestOutput, ToolError> {
            ctx.set("executed", ContextValue::String(self.name.into()));
            Ok(TestOutput { status: "ok" })
        }
    }

    #[test]
    fn test_new_creates_empty_registry() {
        let registry = ToolRegistry::new();
        assert!(registry.list_tools().is_empty());
    }

    #[test]
    fn test_register_and_get_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool { name: "test.click" });

        let tool = registry.get("test.click");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "test.click");
    }

    #[test]
    fn test_get_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.get("nonexistent");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_execute_success() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool { name: "test.click" });

        let mut ctx: ExecutionContext<Ready> = ExecutionContext::<Unvalidated>::new().validate();
        let token = CancellationToken::new();

        let result = registry
            .execute("test.click", json!({"param": 1}), &mut ctx, token)
            .await;

        assert!(result.is_ok());
        assert_eq!(
            ctx.get("executed").and_then(|v| v.try_as_string().ok()),
            Some("test.click")
        );
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let mut ctx: ExecutionContext<Ready> = ExecutionContext::<Unvalidated>::new().validate();
        let token = CancellationToken::new();

        let result = registry
            .execute("nonexistent", json!({}), &mut ctx, token)
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::InvalidInput { .. })));
    }

    #[test]
    fn test_list_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool { name: "tool.a" });
        registry.register(TestTool { name: "tool.b" });

        let mut tools = registry.list_tools();
        tools.sort();
        assert_eq!(tools, vec!["tool.a", "tool.b"]);
    }

    #[test]
    fn test_default_is_empty() {
        let registry: ToolRegistry = Default::default();
        assert!(registry.list_tools().is_empty());
    }
}
