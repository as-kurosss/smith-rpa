// crates/smith-windows/src/tools/screenshot.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;
use uiautomation::screenshots::Screenshot;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.screenshot`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotInput {
    /// Destination PNG file path.
    pub path: String,
    /// Optional key of the `SafeUIElement` to capture.
    #[serde(default)]
    pub element_key: Option<String>,
    /// Element name to find.
    #[serde(default)]
    pub name: Option<String>,
    /// UI Automation identifier.
    #[serde(default)]
    pub automation_id: Option<String>,
    /// Control type.
    #[serde(default)]
    pub control_type: Option<String>,
    /// Window class name.
    #[serde(default)]
    pub class_name: Option<String>,
}

/// Output of a successful screenshot operation.
#[derive(Debug, Serialize)]
pub struct ScreenshotOutput {
    pub path: String,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for capturing a screenshot (desktop or element) to a PNG file.
pub struct ScreenshotTool;

impl ScreenshotTool {
    /// Creates a new `ScreenshotTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScreenshotTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ScreenshotTool {
    type Input = ScreenshotInput;
    type Output = ScreenshotOutput;

    fn name(&self) -> &'static str {
        "windows.screenshot"
    }

    fn description(&self) -> &'static str {
        "Captures the desktop or a UI element to a PNG file"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Destination PNG file path"
                },
                "element_key": {
                    "type": "string",
                    "description": "Key in ExecutionContext containing the UIElement to capture"
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
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        input: ScreenshotInput,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<ScreenshotOutput, ToolError> {
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        let path = input.path.clone();

        // Resolve element from context key or inline selector
        let maybe_element = super::resolve::resolve_element(
            input.element_key.as_deref(),
            super::resolve::SelectorFields {
                name: input.name.as_deref(),
                automation_id: input.automation_id.as_deref(),
                control_type: input.control_type.as_deref(),
                class_name: input.class_name.as_deref(),
            },
            ctx,
        )
        .await?;

        match maybe_element {
            Some(element) => {
                let element_clone = element.clone();

                tokio::task::spawn_blocking(move || {
                    let shot = Screenshot::capture_element(element_clone.inner()).map_err(|e| {
                        ToolError::platform_error("Element screenshot failed", e, None)
                    })?;
                    shot.save_png(&path)
                        .map_err(|e| ToolError::platform_error("Save PNG failed", e, None))
                })
                .await
                .map_err(|e| {
                    ToolError::platform_error("Screenshot blocking task join failed", e, None)
                })??;
            }
            // Capture the entire virtual desktop
            None => {
                tokio::task::spawn_blocking(move || {
                    let shot = Screenshot::capture_desktop().map_err(|e| {
                        ToolError::platform_error("Desktop screenshot failed", e, None)
                    })?;
                    shot.save_png(&path)
                        .map_err(|e| ToolError::platform_error("Save PNG failed", e, None))
                })
                .await
                .map_err(|e| {
                    ToolError::platform_error("Screenshot blocking task join failed", e, None)
                })??;
            }
        }

        Ok(ScreenshotOutput { path: input.path })
    }
}
