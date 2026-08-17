// crates/smith-windows/src/tools/extract.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

use uiautomation::patterns::UIValuePattern;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.extract`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractInput {
    /// Key in `ExecutionContext` containing the `SafeUIElement`.
    pub element_key: Option<String>,
    /// Element name to find (if element_key not set).
    pub name: Option<String>,
    /// UI Automation identifier.
    pub automation_id: Option<String>,
    /// Control type.
    pub control_type: Option<String>,
    /// Window class name.
    pub class_name: Option<String>,
    /// Which property to extract: `"name"` (default) or `"value"`.
    #[serde(default)]
    pub property: Option<String>,
}

/// Output of a successful extract operation.
#[derive(Debug, Serialize)]
pub struct ExtractOutput {
    pub text: String,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for extracting text from a Windows UI element.
pub struct ExtractTool;

impl ExtractTool {
    /// Creates a new `ExtractTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExtractTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExtractTool {
    type Input = ExtractInput;
    type Output = ExtractOutput;

    fn name(&self) -> &'static str {
        "windows.extract"
    }

    fn description(&self) -> &'static str {
        "Extracts text (name or value) from a UI element by context key or inline selector"
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
                "property": {
                    "type": "string",
                    "enum": ["name", "value"],
                    "description": "Property to extract: name (default) or value"
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: ExtractInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<ExtractOutput, ToolError> {
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 1. Resolve element from context key or inline selector
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
        let property = input.property.unwrap_or_else(|| "name".to_string());

        // 2. Read property in blocking thread (COM calls)
        let text = tokio::task::spawn_blocking(move || {
            let element = element_clone.inner();
            match property.as_str() {
                "value" => element
                    .get_pattern::<UIValuePattern>()
                    .and_then(|pattern| pattern.get_value())
                    .map_err(|e| ToolError::platform_error("Extract value failed", e, None)),
                _ => element
                    .get_name()
                    .map_err(|e| ToolError::platform_error("Extract name failed", e, None)),
            }
        })
        .await
        .map_err(|e| ToolError::platform_error("Extract blocking task join failed", e, None))??;

        Ok(ExtractOutput { text })
    }
}
