// crates/smith-windows/src/tools/click.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.click`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickInput {
    /// Key in `ExecutionContext` containing the `SafeUIElement`.
    pub element_key: Option<String>,
    /// Element name to find (if element_key not set).
    pub name: Option<String>,
    /// UI Automation identifier.
    pub automation_id: Option<String>,
    /// Control type (e.g. Button, Edit, Window).
    pub control_type: Option<String>,
    /// Window class name.
    pub class_name: Option<String>,
    /// Optional delay before execution in milliseconds.
    #[serde(default)]
    pub delay_before_ms: Option<u64>,
    /// Optional delay after execution in milliseconds.
    #[serde(default)]
    pub delay_after_ms: Option<u64>,
}

/// Output of a successful click operation.
#[derive(Debug, Serialize)]
pub struct ClickOutput {
    pub status: &'static str,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for performing a click on a Windows UI element.
pub struct ClickTool;

impl ClickTool {
    /// Creates a new `ClickTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClickTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ClickTool {
    type Input = ClickInput;
    type Output = ClickOutput;

    fn name(&self) -> &'static str {
        "windows.click"
    }

    fn description(&self) -> &'static str {
        "Performs a click on a UI element by context key or inline selector"
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "element_key": {
                    "type": "string",
                    "description": "Key in ExecutionContext containing the UIElement"
                },
                "name": {
                    "type": "string",
                    "description": "Element name to find (if element_key not set)"
                },
                "automation_id": {
                    "type": "string",
                    "description": "UI Automation identifier (if element_key not set)"
                },
                "control_type": {
                    "type": "string",
                    "description": "Control type (if element_key not set)"
                },
                "class_name": {
                    "type": "string",
                    "description": "Window class name (if element_key not set)"
                },
                "delay_before_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Delay before execution in milliseconds"
                },
                "delay_after_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Delay after execution in milliseconds"
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: ClickInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<ClickOutput, ToolError> {
        // 0. Cancellation check before any work (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 1. Optional delay before execution
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => {}
                _ = token.cancelled() => {
                    return Err(ToolError::cancelled());
                }
            }
        }

        // 2. Resolve element from context key or inline selector
        let element = super::resolve::resolve_element(
            input.element_key.as_deref(),
            super::resolve::SelectorFields {
                name: input.name.as_deref(),
                automation_id: input.automation_id.as_deref(),
                control_type: input.control_type.as_deref(),
                class_name: input.class_name.as_deref(),
            },
            ctx,
        )
        .await?
        .ok_or_else(|| {
            ToolError::element_not_found(
                "No element found: provide element_key or selector fields".to_string(),
                None,
            )
        })?;
        let element_clone = element.clone();

        // 3. Use spawn_blocking for COM calls (§5.3)
        tokio::task::spawn_blocking(move || {
            element_clone
                .inner()
                .click()
                .map_err(|e| ToolError::platform_error("Click failed", e, None))
        })
        .await
        .map_err(|e| ToolError::platform_error("Blocking task join failed", e, None))??;

        // 4. Optional delay after execution
        if let Some(ms) = input.delay_after_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        Ok(ClickOutput { status: "clicked" })
    }
}
