// crates/smith-windows/src/tools/wait.rs
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Input parameters for `windows.wait`.
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitInput {
    /// Delay duration in milliseconds.
    pub duration_ms: u64,
}

/// Output of a successful wait operation.
#[derive(Debug, Serialize)]
pub struct WaitOutput {
    pub status: &'static str,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for pausing (sleep) in automation scenarios.
///
/// Allows inserting a delay between steps, for example to wait for
/// UI animation, window opening, or data processing.
pub struct WaitTool;

impl WaitTool {
    /// Creates a new `WaitTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for WaitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WaitTool {
    type Input = WaitInput;
    type Output = WaitOutput;

    fn name(&self) -> &'static str {
        "windows.wait"
    }

    fn description(&self) -> &'static str {
        "Pauses execution for the specified duration (in milliseconds)"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "duration_ms": {
                    "type": "integer",
                    "description": "Delay duration in milliseconds"
                }
            },
            "required": ["duration_ms"]
        })
    }

    async fn execute(
        &self,
        input: WaitInput,
        _ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<WaitOutput, ToolError> {
        // 1. Cancellation check (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Pause with cancellation support
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(input.duration_ms)) => {}
            _ = token.cancelled() => {
                return Err(ToolError::cancelled());
            }
        }

        Ok(WaitOutput {
            status: "waited",
            duration_ms: input.duration_ms,
        })
    }
}
