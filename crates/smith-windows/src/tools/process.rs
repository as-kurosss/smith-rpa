// crates/smith-windows/src/tools/process.rs
use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Typed input/output (§2.1)
// ---------------------------------------------------------------------------

/// Action types for process management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessAction {
    Start,
    Stop,
    Sleep,
}

/// Input parameters for `windows.process`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessInput {
    /// Action to perform.
    pub action: ProcessAction,
    /// Executable path (required for start).
    pub command: Option<String>,
    /// Command-line arguments (for start).
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// Working directory (for start).
    pub working_dir: Option<String>,
    /// Process ID to stop.
    pub pid: Option<u32>,
    /// Process image name to stop (e.g. notepad.exe).
    pub name: Option<String>,
    /// Sleep duration in milliseconds (required for sleep).
    pub duration_ms: Option<u64>,
    /// Optional delay before execution in milliseconds.
    #[serde(default)]
    pub delay_before_ms: Option<u64>,
    /// Optional delay after execution in milliseconds.
    #[serde(default)]
    pub delay_after_ms: Option<u64>,
}

/// Output of a process operation.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProcessOutput {
    Started {
        status: &'static str,
        pid: u32,
    },
    Stopped {
        status: &'static str,
        method: &'static str,
        pid: Option<u32>,
        name: Option<String>,
    },
    Slept {
        status: &'static str,
        duration_ms: u64,
    },
}

// ---------------------------------------------------------------------------
// Command allowlist
// ---------------------------------------------------------------------------

/// Whitelist of allowed executable names for process start.
///
/// This prevents arbitrary command execution via the HTTP API.
/// Only well-known Windows utilities are permitted.
/// `cmd.exe` and `powershell.exe` are intentionally excluded because they allow
/// arbitrary command execution via arguments (`/c`, `-Command`).
///
/// # Security
///
/// The daemon must NOT be exposed to untrusted networks (`--host 127.0.0.1`).
/// Comparison is case-insensitive (Windows filesystem convention).
fn is_command_allowed(cmd: &str) -> bool {
    let allowed: HashSet<&str> = HashSet::from_iter([
        "notepad.exe",
        "calc.exe",
        "mspaint.exe",
        "explorer.exe",
        "write.exe",
        "wordpad.exe",
    ]);

    // Extract the file name from the path
    let name = cmd.rsplit_once(['/', '\\']).map_or(cmd, |(_, file)| file);

    allowed.iter().any(|&a| a.eq_ignore_ascii_case(name))
}

// ---------------------------------------------------------------------------
// Tool implementation
// ---------------------------------------------------------------------------

/// Tool for managing Windows processes.
///
/// Supports actions:
/// - `Start` — launches a new process (does not wait for completion)
/// - `Stop` — stops a process by PID or name (does not wait for taskkill to finish)
/// - `Sleep` — pauses for `duration_ms` milliseconds
pub struct ProcessTool;

impl ProcessTool {
    /// Creates a new `ProcessTool` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProcessTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ProcessTool {
    type Input = ProcessInput;
    type Output = ProcessOutput;

    fn name(&self) -> &'static str {
        "windows.process"
    }

    fn description(&self) -> &'static str {
        "Manages Windows processes: start, stop, or sleep"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "stop", "sleep"],
                    "description": "Action to perform"
                },
                "command": {
                    "type": "string",
                    "description": "Executable path (required for start)"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command-line arguments (for start)"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (for start)"
                },
                "pid": {
                    "type": "integer",
                    "description": "Process ID to stop"
                },
                "name": {
                    "type": "string",
                    "description": "Process image name to stop (e.g. notepad.exe)"
                },
                "duration_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Sleep duration in milliseconds (required for sleep)"
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
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        input: ProcessInput,
        _ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<ProcessOutput, ToolError> {
        // 0. Optional delay before execution
        if let Some(ms) = input.delay_before_ms.filter(|&ms| ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        // 1. Cancellation check (§5.4)
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        // 2. Dispatch by action
        let result = match input.action {
            ProcessAction::Start => action_start(&input),
            ProcessAction::Stop => {
                let pid = input.pid;
                let name = input.name.clone();
                tokio::task::spawn_blocking(move || action_stop(pid, name.as_deref()))
                    .await
                    .map_err(|e| ToolError::platform_error("Blocking task join failed", e, None))?
            }
            ProcessAction::Sleep => {
                let duration_ms = input.duration_ms.ok_or_else(|| {
                    ToolError::invalid_input(
                        "Missing 'duration_ms' for sleep action",
                        Some("duration_ms".into()),
                        None,
                    )
                })?;
                tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                Ok(ProcessOutput::Slept {
                    status: "slept",
                    duration_ms,
                })
            }
        };

        // 3. Optional delay after execution (only on success)
        if result.is_ok()
            && let Some(ms) = input.delay_after_ms.filter(|&ms| ms > 0)
        {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        result
    }
}

fn action_start(input: &ProcessInput) -> Result<ProcessOutput, ToolError> {
    // Validation: command is required
    let cmd_str = input.command.as_deref().ok_or_else(|| {
        ToolError::invalid_input(
            "Missing 'command' for start action",
            Some("command".into()),
            None,
        )
    })?;

    // Command injection protection (Canon 10.1 Input Validation)
    if !is_command_allowed(cmd_str) {
        return Err(ToolError::invalid_input(
            format!("Command '{cmd_str}' is not in the allowed list"),
            Some("command".into()),
            None,
        ));
    }

    let mut cmd = std::process::Command::new(cmd_str);

    if let Some(args) = &input.args {
        for arg in args {
            cmd.arg(arg);
        }
    }

    if let Some(dir) = &input.working_dir {
        cmd.current_dir(dir);
    }

    let child = cmd
        .spawn()
        .map_err(|e| ToolError::platform_error("Failed to start process", e, None))?;

    let pid = child.id();
    Ok(ProcessOutput::Started {
        status: "started",
        pid,
    })
}

fn action_stop(pid: Option<u32>, name: Option<&str>) -> Result<ProcessOutput, ToolError> {
    use std::process::Stdio;

    if let Some(pid) = pid {
        let output = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| ToolError::platform_error("taskkill spawn failed", e, None))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::platform_error(
                format!("taskkill for pid {pid} failed: {stderr}"),
                std::io::Error::other(stderr.as_ref()),
                None,
            ));
        }

        Ok(ProcessOutput::Stopped {
            status: "stopped",
            method: "pid",
            pid: Some(pid),
            name: None,
        })
    } else if let Some(name) = name {
        let output = std::process::Command::new("taskkill")
            .args(["/F", "/IM", name])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| ToolError::platform_error("taskkill spawn failed", e, None))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::platform_error(
                format!("taskkill for {name} failed: {stderr}"),
                std::io::Error::other(stderr.as_ref()),
                None,
            ));
        }

        Ok(ProcessOutput::Stopped {
            status: "stopped",
            method: "name",
            pid: None,
            name: Some(name.to_string()),
        })
    } else {
        Err(ToolError::invalid_input(
            "Must provide 'pid' or 'name' for stop action",
            None,
            None,
        ))
    }
}
