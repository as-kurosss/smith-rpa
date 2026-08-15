// crates/smith-windows/src/tools/find.rs
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ContextValue, ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

use crate::element::SafeUIElement;
use crate::selector::ElementSelector;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.find`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FindInput {
    /// Key to store the found element in execution context.
    pub output_key: String,
    /// Element name to match.
    pub name: Option<String>,
    /// UI Automation identifier.
    pub automation_id: Option<String>,
    /// Control type (e.g. Button, Edit, Window).
    pub control_type: Option<String>,
    /// Window class name.
    pub class_name: Option<String>,
    /// Process ID filter.
    pub pid: Option<u32>,
    /// Optional delay before execution in milliseconds.
    #[serde(default)]
    pub delay_before_ms: Option<u64>,
    /// Optional delay after execution in milliseconds.
    #[serde(default)]
    pub delay_after_ms: Option<u64>,
}

/// Output of a successful find operation.
#[derive(Debug, Serialize)]
pub struct FindOutput {
    pub status: &'static str,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for finding a UI element on the Windows desktop.
///
/// Accepts selector parameters (name, `automation_id`, `control_type`, `class_name`, pid)
/// and saves the found element in the context under the specified key.
pub struct FindTool;

impl FindTool {
    /// Creates a new `FindTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for FindTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FindTool {
    type Input = FindInput;
    type Output = FindOutput;

    fn name(&self) -> &'static str {
        "windows.find"
    }

    fn description(&self) -> &'static str {
        "Finds a Windows UI element matching the specified selectors and stores it in context"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Element name to match"
                },
                "automation_id": {
                    "type": "string",
                    "description": "UI Automation identifier"
                },
                "control_type": {
                    "type": "string",
                    "description": "Control type (e.g. Button, Edit, Window)"
                },
                "class_name": {
                    "type": "string",
                    "description": "Window class name"
                },
                "pid": {
                    "type": "integer",
                    "description": "Process ID filter"
                },
                "output_key": {
                    "type": "string",
                    "description": "Key to store the found element in execution context"
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
            "required": ["output_key"]
        })
    }

    async fn execute(
        &self,
        input: FindInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<FindOutput, ToolError> {
        // 0. Optional delay before execution
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        // 1. Cancellation check (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Build selector from input
        let mut selector = ElementSelector::new();
        if let Some(name) = &input.name {
            selector = selector.name(name);
        }
        if let Some(aid) = &input.automation_id {
            selector = selector.automation_id(aid);
        }
        if let Some(ct) = &input.control_type {
            selector = selector.control_type(ct);
        }
        if let Some(cn) = &input.class_name {
            selector = selector.class_name(cn);
        }
        if let Some(pid) = input.pid {
            selector = selector.pid(pid);
        }

        // 3. Search in blocking thread (COM calls).
        //    SafeUIElement is created inside spawn_blocking because UIElement is not Send.
        let safe_element = tokio::task::spawn_blocking(move || {
            selector.find_from_desktop().map(SafeUIElement::new)
        })
        .await
        .map_err(|e| ToolError::platform_error("Find element blocking task failed", e, None))?
        .map_err(|_e| {
            ToolError::element_not_found(
                "No element found matching selector".to_string(),
                Some(serde_json::to_value(&input).unwrap_or_default()),
            )
        })?;

        // 4. Save result to context
        ctx.set(
            input.output_key.clone(),
            ContextValue::Custom(Arc::new(safe_element)),
        );

        // 5. Optional delay after execution
        if let Some(ms) = input.delay_after_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        Ok(FindOutput { status: "found" })
    }
}
