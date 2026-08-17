// crates/smith-windows/src/tools/set_text.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.set_text`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SetTextInput {
    /// Text value to set.
    pub text: String,
    /// Key in ExecutionContext containing a UIElement.
    pub element_key: Option<String>,
    /// Element name to find (if element_key not set).
    pub name: Option<String>,
    /// UI Automation identifier.
    pub automation_id: Option<String>,
    /// Control type.
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

/// Output of a successful set_text operation.
#[derive(Debug, Serialize)]
pub struct SetTextOutput {
    pub status: &'static str,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for setting text via UI Automation `ValuePattern`.
///
/// Unlike `InputTextTool`, this tool directly sets the
/// text field value via `ValuePattern::set_value()`, which
/// works faster but does not simulate real keyboard input.
pub struct SetTextTool;

impl SetTextTool {
    /// Creates a new `SetTextTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for SetTextTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SetTextTool {
    type Input = SetTextInput;
    type Output = SetTextOutput;

    fn name(&self) -> &'static str {
        "windows.set_text"
    }

    fn description(&self) -> &'static str {
        "Sets the text value of a UI element via ValuePattern"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text value to set"
                },
                "element_key": {
                    "type": "string",
                    "description": "Key in ExecutionContext containing a UIElement"
                },
                "name": {
                    "type": "string",
                    "description": "Element name to find (if element_key not set)"
                },
                "automation_id": {
                    "type": "string",
                    "description": "UI Automation identifier"
                },
                "control_type": {
                    "type": "string",
                    "description": "Control type"
                },
                "class_name": {
                    "type": "string",
                    "description": "Window class name"
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
            "required": ["text"],
            "anyOf": [
                { "required": ["element_key"] },
                { "required": ["name"] },
                { "required": ["automation_id"] },
                { "required": ["control_type"] },
                { "required": ["class_name"] }
            ]
        })
    }

    async fn execute(
        &self,
        input: SetTextInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<SetTextOutput, ToolError> {
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

        // 2. Get element
        let text = input.text.clone();
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
            ToolError::invalid_input("Missing 'element_key' or selector fields", None, None)
        })?;

        // 3. Set text via ValuePattern in blocking thread
        tokio::task::spawn_blocking(move || {
            let pattern = element
                .inner()
                .get_pattern::<uiautomation::patterns::UIValuePattern>()
                .map_err(|e| ToolError::platform_error("Get ValuePattern failed", e, None))?;
            pattern
                .set_value(&text)
                .map_err(|e| ToolError::platform_error("Set value failed", e, None))?;
            Ok::<_, ToolError>(())
        })
        .await
        .map_err(|e| ToolError::platform_error("Set text blocking task join failed", e, None))??;

        // 4. Optional delay after execution
        if let Some(ms) = input.delay_after_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        Ok(SetTextOutput { status: "text_set" })
    }
}
