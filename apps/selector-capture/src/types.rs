use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

// ── Single-capture model (used by `single` subcommand) ────

/// A single node in the UI Automation tree path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    pub control_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
}

/// The flat optimal selector for the target element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestSelector {
    pub control_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
}

impl BestSelector {
    /// Returns a human-readable label for display.
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(self.control_type.as_str())
    }

    /// Returns `true` if the selector has any identifying field set.
    #[allow(dead_code)]
    pub fn has_any(&self) -> bool {
        self.name.is_some()
            || self.automation_id.is_some()
            || self.class_name.is_some()
            || !self.control_type.is_empty()
    }
}

/// A single capture record (single mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    pub id: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub full_path: Vec<PathNode>,
    pub best_selector: BestSelector,
}

/// Root output JSON for single captures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOutput {
    pub tool: String,
    pub version: String,
    #[serde(default)]
    pub captures: Vec<Capture>,
}

// ── Series-recording model (used by `series` subcommand) ──

/// A UIA element as captured during action recording
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CapturedElement {
    pub full_path: Vec<PathNode>,
    pub best_selector: BestSelector,
}

/// A single recorded action in a series session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
#[allow(dead_code)]
pub enum Action {
    /// User clicked a mouse button on an element
    Click {
        button: String,
        element: CapturedElement,
    },
    /// User typed text into an element (accumulated between clicks)
    Input {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        element: Option<CapturedElement>,
    },
}

/// Root output JSON for a series recording session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SeriesRecording {
    pub tool: String,
    pub version: String,
    pub timestamp_start: String,
    pub timestamp_end: String,
    pub actions: Vec<Action>,
}

// ── New: Flow config generation model ────────────────────

/// Target tool type for config generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolType {
    #[default]
    Find,
    Click,
    InputText,
    SetText,
    Wait,
}

impl ToolType {
    /// All valid tool types (for display / validation).
    #[allow(dead_code)]
    pub fn all() -> &'static [ToolType] {
        &[
            ToolType::Find,
            ToolType::Click,
            ToolType::InputText,
            ToolType::SetText,
            ToolType::Wait,
        ]
    }

    /// Returns `true` if this tool requires a text parameter.
    pub fn needs_text(self) -> bool {
        matches!(self, ToolType::InputText | ToolType::SetText)
    }

    /// Returns `true` if this tool requires a duration_ms parameter.
    pub fn needs_duration(self) -> bool {
        matches!(self, ToolType::Wait)
    }

    /// Returns `true` if this tool requires an element_key (depends on a preceding find).
    #[allow(dead_code)]
    pub fn needs_element_key(self) -> bool {
        matches!(
            self,
            ToolType::Click | ToolType::InputText | ToolType::SetText
        )
    }

    /// Returns `true` if this tool produces a find config + action config (two nodes).
    pub fn generates_two_nodes(self) -> bool {
        matches!(
            self,
            ToolType::Click | ToolType::InputText | ToolType::SetText
        )
    }
}

impl fmt::Display for ToolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolType::Find => write!(f, "find"),
            ToolType::Click => write!(f, "click"),
            ToolType::InputText => write!(f, "input_text"),
            ToolType::SetText => write!(f, "set_text"),
            ToolType::Wait => write!(f, "wait"),
        }
    }
}

impl FromStr for ToolType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "find" => Ok(ToolType::Find),
            "click" => Ok(ToolType::Click),
            "input_text" | "input" => Ok(ToolType::InputText),
            "set_text" | "set" => Ok(ToolType::SetText),
            "wait" => Ok(ToolType::Wait),
            other => Err(format!(
                "Unknown tool type '{other}'. Valid options: find, click, input_text, set_text, wait"
            )),
        }
    }
}

/// A single node in the generated flow.
#[derive(Debug, Clone, Serialize)]
pub struct FlowNode {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Full generated flow output.
#[derive(Debug, Clone, Serialize)]
pub struct FlowGraphOutput {
    pub tool: String,
    pub version: String,
    pub nodes: Vec<FlowNode>,
}

/// Output format for the generated flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    Json,
    Rust,
}

impl FormatType {
    /// Detect format from file extension.
    pub fn from_extension(path: &str) -> Self {
        if path.ends_with(".rs") {
            FormatType::Rust
        } else {
            FormatType::Json
        }
    }
}

impl fmt::Display for FormatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatType::Json => write!(f, "json"),
            FormatType::Rust => write!(f, "rust"),
        }
    }
}

impl FromStr for FormatType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "json" => Ok(FormatType::Json),
            "rust" | "rs" => Ok(FormatType::Rust),
            other => Err(format!(
                "Unknown format '{other}'. Valid options: json, rust"
            )),
        }
    }
}
