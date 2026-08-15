// crates/smith-windows/src/tools/input_text.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.input_text`.
#[derive(Debug, Serialize, Deserialize)]
pub struct InputTextInput {
    /// Text to type (use {} for special keys, e.g. {enter}).
    pub text: String,
    /// Key in ExecutionContext containing a UIElement to focus first.
    pub element_key: Option<String>,
    /// Element name to find (if element_key not set).
    pub name: Option<String>,
    /// UI Automation identifier (if element_key not set).
    pub automation_id: Option<String>,
    /// Control type (if element_key not set).
    pub control_type: Option<String>,
    /// Window class name (if element_key not set).
    pub class_name: Option<String>,
    /// Optional delay before execution in milliseconds.
    #[serde(default)]
    pub delay_before_ms: Option<u64>,
    /// Optional delay after execution in milliseconds.
    #[serde(default)]
    pub delay_after_ms: Option<u64>,
}

/// Output of a successful input_text operation.
#[derive(Debug, Serialize)]
pub struct InputTextOutput {
    pub status: &'static str,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for inputting text into a Windows UI element.
///
/// If `element_key` or selector fields are specified, first focuses the element,
/// then types text via UIA `send_keys`. If no element is specified,
/// simply sends keystrokes to the active window.
pub struct InputTextTool;

impl InputTextTool {
    /// Creates a new `InputTextTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for InputTextTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for InputTextTool {
    type Input = InputTextInput;
    type Output = InputTextOutput;

    fn name(&self) -> &'static str {
        "windows.input_text"
    }

    fn description(&self) -> &'static str {
        "Types text into a UI element (or sends keystrokes if no element specified)"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to type (use {} for special keys, e.g. {enter})"
                },
                "element_key": {
                    "type": "string",
                    "description": "Key in ExecutionContext containing a UIElement to focus first"
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
            "required": ["text"]
        })
    }

    async fn execute(
        &self,
        input: InputTextInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<InputTextOutput, ToolError> {
        // 0. Optional delay before execution
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        // 1. Cancellation check (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Try to get element from context or find by selector
        let text = input.text.clone();
        let maybe_element = resolve_element_from_config(&input.element_key, &input, ctx).await?;

        // 3. Input text in blocking thread
        tokio::task::spawn_blocking(move || {
            if let Some(element) = maybe_element {
                // Focus the element, then type text
                element
                    .inner()
                    .set_focus()
                    .map_err(|e| ToolError::platform_error("Set focus failed", e, None))?;
                // Small pause after set_focus
                std::thread::sleep(std::time::Duration::from_millis(100));
                element
                    .inner()
                    .send_keys(&text, 0)
                    .map_err(|e| ToolError::platform_error("send_keys failed", e, None))?;
            } else {
                // Simply type text into the active window via the root element
                let automation = uiautomation::core::UIAutomation::new()
                    .map_err(|e| ToolError::platform_error("UIAutomation init failed", e, None))?;
                let root = automation
                    .get_root_element()
                    .map_err(|e| ToolError::platform_error("Get root element failed", e, None))?;
                root.send_keys(&text, 0)
                    .map_err(|e| ToolError::platform_error("send_keys failed", e, None))?;
            }
            Ok::<_, ToolError>(())
        })
        .await
        .map_err(|e| {
            ToolError::platform_error("Input text blocking task join failed", e, None)
        })??;

        // 4. Optional delay after execution
        if let Some(ms) = input.delay_after_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        Ok(InputTextOutput {
            status: "input_sent",
        })
    }
}

/// Resolves a UI element from input, trying in order:
/// 1. Look up `element_key` in the execution context
/// 2. Build an `ElementSelector` from selector fields and find from desktop
async fn resolve_element_from_config(
    element_key: &Option<String>,
    input: &InputTextInput,
    ctx: &ExecutionContext,
) -> Result<Option<crate::element::SafeUIElement>, ToolError> {
    use crate::selector::ElementSelector;

    // 1. Try to get element from context by element_key
    if let Some(key) = element_key {
        let value = ctx.get(key).ok_or_else(|| {
            ToolError::invalid_input(
                format!("Key '{key}' not found in context"),
                Some("element_key".into()),
                None,
            )
        })?;
        return value
            .try_as_custom::<crate::element::SafeUIElement>()
            .map(|e| Some(e.clone()));
    }

    // 2. Try to find element by selector fields
    let has_selector = input.name.is_some()
        || input.automation_id.is_some()
        || input.control_type.is_some()
        || input.class_name.is_some();

    if !has_selector {
        return Ok(None);
    }

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

    let safe_element = tokio::task::spawn_blocking(move || {
        selector
            .find_from_desktop()
            .map(crate::element::SafeUIElement::new)
    })
    .await
    .map_err(|e| ToolError::platform_error("Find element blocking task failed", e, None))?
    .map_err(|_e| ToolError::element_not_found("No element found matching selector", None))?;

    Ok(Some(safe_element))
}
