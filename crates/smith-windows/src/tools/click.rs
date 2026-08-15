// crates/smith-windows/src/tools/click.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

use crate::element::SafeUIElement;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.click`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickInput {
    /// Key in `ExecutionContext` containing the `SafeUIElement`.
    pub element_key: String,
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
        "Performs a click on a UI element stored in the execution context"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "element_key": {
                    "type": "string",
                    "description": "Key in ExecutionContext containing the UIElement"
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
            "required": ["element_key"]
        })
    }

    async fn execute(
        &self,
        input: ClickInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<ClickOutput, ToolError> {
        // 0. Optional delay before execution (§5.4 — check before work)
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        // 1. Cancellation check before heavy operation (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Retrieve element from context
        let value = ctx.get(&input.element_key).ok_or_else(|| {
            ToolError::invalid_input(
                format!("Key '{}' not found in context", input.element_key),
                Some("element_key".into()),
                None,
            )
        })?;

        let wrapper = value.try_as_custom::<SafeUIElement>()?;
        let element_clone = wrapper.clone();

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
