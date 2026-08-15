//! selector-capture – cross-platform UI element selector recording utility
//!
//! Modes:
//! - `single`  – press CTRL alone to capture the element under cursor.
//! - `series`  – automatically record every mouse click & text input;
//!   press Ctrl+Shift+F2 to stop.
//! - `record`  – capture one element at a time with interactive prompts
//!   for tool type and parameters; produces ready-to-use flow node configs.
//!
//! Outputs structured JSON with full tree paths and best-effort flat selectors.
//! Can also output Rust `FlowGraph::builder(...)` code and copy to clipboard.
//!
//! Backends are selected via `--backend` (default: windows).
//! Compile with `--no-default-features --features linux` for a Linux backend.

use clap::{Parser, Subcommand};

mod backend;
mod format;
mod generate;
mod recorder;
mod types;
use types::{FormatType, ToolType};

use crate::backend::create_backend;

// ── CLI ───────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "selector-capture",
    version,
    about = "Capture UI element selectors and generate flow node configs"
)]
struct Cli {
    /// Platform backend to use (windows, linux, browser)
    #[arg(short = 'B', long, default_value = "windows")]
    backend: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Capture a single element and exit
    Single {
        /// Optional description for the capture
        #[arg(short, long)]
        description: Option<String>,

        /// Output JSON file
        #[arg(short, long, default_value = "selectors.json")]
        output: String,

        /// Copy generated config to clipboard instead of (or in addition to) file
        #[arg(short, long)]
        clip: bool,

        /// Tool type for config generation (find, click, input_text, set_text, wait)
        #[arg(short, long, value_parser = clap::value_parser!(ToolType))]
        tool: Option<ToolType>,

        /// Output key for find step (auto-generated if not set)
        #[arg(short = 'k', long)]
        output_key: Option<String>,

        /// Text for input_text / set_text
        #[arg(short, long)]
        text: Option<String>,

        /// Duration in ms for wait tool (default: 1000)
        #[arg(short = 'd', long)]
        duration_ms: Option<u64>,
    },

    /// Automatically record all clicks & inputs; Ctrl+Shift+F2 to stop
    Series {
        /// Output file (extension determines format: .rs → Rust, else JSON)
        #[arg(short, long, default_value = "selectors.json")]
        output: String,

        /// Output format (json or rust). Overrides auto-detect from extension
        #[arg(short, long)]
        format: Option<FormatType>,
    },

    /// Interactive capture: CTRL to capture, prompts for tool/params, Ctrl+Shift+F2 to finish
    Record {
        /// Output file (extension determines format: .rs → Rust, else JSON)
        #[arg(short, long, default_value = "flow.json")]
        output: String,

        /// Output format (json or rust). Overrides auto-detect from extension
        #[arg(short, long)]
        format: Option<FormatType>,
    },
}

// ── Entry point ───────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let backend = create_backend(&cli.backend)?;

    match cli.command {
        Commands::Single {
            description,
            output,
            clip,
            tool,
            output_key,
            text,
            duration_ms,
        } => {
            recorder::run_single_mode(
                &*backend,
                &output,
                description,
                clip,
                tool,
                output_key,
                text,
                duration_ms,
            )?;
        }
        Commands::Series { output, format } => {
            let fmt = format.unwrap_or_else(|| FormatType::from_extension(&output));
            recorder::run_series_mode(&*backend, &output, fmt)?;
        }
        Commands::Record { output, format } => {
            let fmt = format.unwrap_or_else(|| FormatType::from_extension(&output));
            recorder::run_record_mode(&*backend, &output, fmt)?;
        }
    }

    Ok(())
}
