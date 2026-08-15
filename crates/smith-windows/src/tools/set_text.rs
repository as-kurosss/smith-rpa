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
        // 0. Optional delay before execution
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        // 1. Cancellation check (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Get element
        let text = input.text.clone();
        let element = resolve_element_from_config(&input, ctx)
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

/// Resolves a UI element from input, trying in order:
/// 1. Look up `element_key` in the execution context
/// 2. Build an `ElementSelector` from selector fields and find from desktop
async fn resolve_element_from_config(
    input: &SetTextInput,
    ctx: &ExecutionContext,
) -> Result<Option<crate::element::SafeUIElement>, ToolError> {
    use crate::element::SafeUIElement;
    use crate::selector::ElementSelector;

    // 1. Try to get element from context by element_key
    if let Some(ref key) = input.element_key {
        let value = ctx.get(key).ok_or_else(|| {
            ToolError::invalid_input(
                format!("Key '{key}' not found in context"),
                Some("element_key".into()),
                None,
            )
        })?;
        return value
            .try_as_custom::<SafeUIElement>()
            .map(|e| Some(e.clone()));
    }

    // 2. Try to find element by selector fields
    let mut selector = ElementSelector::new();
    if let Some(ref name) = input.name {
        selector = selector.name(name);
    }
    if let Some(ref aid) = input.automation_id {
        selector = selector.automation_id(aid);
    }
    if let Some(ref ct) = input.control_type {
        selector = selector.control_type(ct);
    }
    if let Some(ref cn) = input.class_name {
        selector = selector.class_name(cn);
    }

    let safe_element =
        tokio::task::spawn_blocking(move || selector.find_from_desktop().map(SafeUIElement::new))
            .await
            .map_err(|e| ToolError::platform_error("Find element blocking task failed", e, None))?
            .map_err(|_e| {
                ToolError::element_not_found("No element found matching selector", None)
            })?;

    Ok(Some(safe_element))
}
