//! Hotkey-based recorders for UI element selectors
//!
//! ## Single mode
//! CTRL alone → capture element under cursor. ESC to cancel.
//! `--clip` → config copied to clipboard.
//! `--tool` → config generated for specific tool type.
//!
//! ## Series mode (action recorder)
//! Automatically records every mouse click and text input.
//! `Ctrl+Shift+F2` → stop and save as flow nodes.
//!
//! ## Record mode (interactive capture)
//! CTRL → capture → prompt for tool/params → repeat.
//! `Ctrl+Shift+F2` → finish and save flow.

use std::fs;
use std::io::{self, Write};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

use chrono::Utc;

use crate::backend::{Backend, SeriesEvent, SharedEvent};
use crate::format;
use crate::generate::{generate_nodes, GenerateParams};
use crate::types::{BestSelector, Capture, CaptureOutput, FlowNode, FormatType, ToolType};

// ──────────────────────────────────────────────────────────
//  SINGLE MODE — extended with --clip and --tool
// ──────────────────────────────────────────────────────────

/// Run single-capture mode.
#[allow(clippy::too_many_arguments)]
pub fn run_single_mode(
    backend: &dyn Backend,
    output: &str,
    description: Option<String>,
    clip: bool,
    tool_type: Option<ToolType>,
    output_key: Option<String>,
    text: Option<String>,
    duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    eprintln!("=== Selector Capture: Single Mode ({}) ===", backend.name());
    eprintln!("Place cursor over a UI element and press CTRL (alone) to capture.");
    eprintln!("Press ESC to cancel.");

    let (tx, rx) = mpsc::channel();
    backend.spawn_shared_listener(tx);

    print!("\n  Waiting for CTRL… ");
    io::stdout().flush()?;

    match wait_single(backend, &rx)? {
        Some(sel) => {
            let label = sel.label();
            println!("\n  ✓ Captured: {}", label);

            let tool = tool_type.unwrap_or(ToolType::Find);
            let el_key = output_key.unwrap_or_else(|| format!("el_{:02}", 1));
            let params = GenerateParams {
                output_key: el_key,
                text,
                duration_ms,
            };

            if clip {
                // For clipboard, generate find+action configs separated by ---
                let find_cfg =
                    serde_json::to_string(&generate_nodes(&sel, ToolType::Find, &params)[0].args)
                        .unwrap_or_default();
                let action_cfg = serde_json::to_string(&crate::generate::generate_action_config(
                    &sel, tool, &params,
                ))
                .unwrap_or_default();

                let clip_text = if tool.generates_two_nodes() || tool == ToolType::Find {
                    // Copy just the config (or find+action pair)
                    format!("{find_cfg}\n---\n{action_cfg}")
                } else {
                    action_cfg
                };

                if backend.copy_to_clipboard(&clip_text) {
                    println!("  📋 Copied to clipboard ({tool})");
                }
            }

            // Always save to file (Capture format for backward compat)
            let cap = Capture {
                id: format!("capture-{:016x}", timestamp_id()),
                timestamp: Utc::now().to_rfc3339(),
                description,
                full_path: Vec::new(), // not captured in this path, but kept for compat
                best_selector: sel,
            };
            append_capture(output, cap)?;
            println!("  ✓ Saved to {output}");
        }
        None => {
            println!("\n  Cancelled.");
        }
    }
    Ok(())
}

/// Wait for a single trigger in single mode.
fn wait_single(
    backend: &dyn Backend,
    rx: &Receiver<SharedEvent>,
) -> anyhow::Result<Option<BestSelector>> {
    loop {
        match rx.recv()? {
            SharedEvent::Stop => return Ok(None),
            SharedEvent::Trigger => {
                let (x, y) = backend.cursor_position();
                match backend.capture_at_point(x, y) {
                    Ok((_path, best)) => return Ok(Some(best)),
                    Err(e) => eprintln!("\n  ⚠ capture: {e}"),
                }
            }
            _ => {}
        }
    }
}

// ──────────────────────────────────────────────────────────
//  SERIES MODE — generates FlowNode sequence
// ──────────────────────────────────────────────────────────

/// Run series (action recorder) mode — outputs flow nodes (find+click/input pairs).
pub fn run_series_mode(
    backend: &dyn Backend,
    output: &str,
    format: FormatType,
) -> anyhow::Result<()> {
    eprintln!("=== Selector Capture: Series Mode ({}) ===", backend.name());
    eprintln!("• Mouse clicks  → captured as find + click node pairs");
    eprintln!("• Keyboard     → captured as find + input_text node pairs");
    eprintln!("• Press Ctrl+Shift+F2 to stop recording");
    eprintln!("  Output format: {format}");
    eprintln!();

    let (tx, rx) = mpsc::channel();
    backend.spawn_series_listener(tx);

    let mut nodes: Vec<FlowNode> = Vec::new();
    let mut el_counter = 0u32;
    let mut pending_input = false;
    let mut _last_element: Option<BestSelector> = None;

    loop {
        match rx.recv()? {
            SeriesEvent::Stop => {
                // Flush any pending text input before last element
                if pending_input {
                    if let Some(ref sel) = _last_element {
                        el_counter += 1;
                        let output_key = format!("el_{el_counter:02}");
                        let params = GenerateParams {
                            output_key: output_key.clone(),
                            text: None, // series doesn't capture text content
                            duration_ms: None,
                        };
                        nodes.extend(generate_nodes(sel, ToolType::InputText, &params));
                    }
                }
                break;
            }

            SeriesEvent::MouseDown { .. } => {
                // Flush pending input before handling click
                if pending_input {
                    if let Some(ref sel) = _last_element {
                        el_counter += 1;
                        let output_key = format!("el_{el_counter:02}");
                        let params = GenerateParams {
                            output_key: output_key.clone(),
                            text: None,
                            duration_ms: None,
                        };
                        nodes.extend(generate_nodes(sel, ToolType::InputText, &params));
                    }
                }
                pending_input = false;

                let (x, y) = backend.cursor_position();
                match backend.capture_at_point(x, y) {
                    Ok((_path, best)) => {
                        el_counter += 1;
                        let output_key = format!("el_{el_counter:02}");
                        let params = GenerateParams {
                            output_key: output_key.clone(),
                            text: None,
                            duration_ms: None,
                        };
                        nodes.extend(generate_nodes(&best, ToolType::Click, &params));
                        _last_element = Some(best);
                    }
                    Err(e) => {
                        eprintln!("  ⚠ could not capture element at ({x:.0}, {y:.0}): {e}");
                    }
                }
            }

            SeriesEvent::Input => {
                pending_input = true;
            }
        }
    }

    println!("\n  Recording stopped — {} nodes generated", nodes.len());

    if !nodes.is_empty() {
        let output_str = format::format_output(&nodes, format);
        fs::write(output, &output_str)?;
        let label = if output.ends_with(".rs") {
            "Rust"
        } else {
            "JSON"
        };
        println!("  ✓ Saved {label} flow to {output}");
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────
//  RECORD MODE — interactive capture with tool prompts
// ──────────────────────────────────────────────────────────

/// Run interactive record mode.
///
/// 1. Press CTRL to capture element under cursor.
/// 2. Interactively choose tool type and parameters.
/// 3. Press Ctrl+Shift+F2 to finish and write output.
pub fn run_record_mode(
    backend: &dyn Backend,
    output: &str,
    format: FormatType,
) -> anyhow::Result<()> {
    eprintln!("=== Selector Capture: Record Mode ({}) ===", backend.name());
    eprintln!("Hotkeys:");
    eprintln!("  CTRL          → capture element under cursor");
    eprintln!("  ESC           → discard last capture");
    eprintln!("  Ctrl+Shift+F2 → finish and save");
    eprintln!();

    let (tx, rx) = mpsc::channel();
    backend.spawn_shared_listener(tx);

    let mut nodes: Vec<FlowNode> = Vec::new();
    let mut el_counter = 0u32;
    let mut _last_selector: Option<BestSelector> = None;

    print!("Ready. Press CTRL over a UI element… ");
    io::stdout().flush()?;

    loop {
        match rx.recv()? {
            SharedEvent::Stop => {
                println!("\n");
                break;
            }

            SharedEvent::Escape => {
                if nodes.is_empty() {
                    println!("\n  Discarded — no captures taken.");
                    return Ok(());
                }
                // Remove the last pair of nodes (find + action)
                let removed = nodes.len().saturating_sub(2);
                let count = nodes.len() - removed;
                nodes.truncate(removed);
                println!(
                    "\n  ✗ Discarded capture #{} ({} nodes removed)",
                    el_counter, count
                );
                el_counter = el_counter.saturating_sub(1);
            }

            SharedEvent::Trigger => {
                let (x, y) = backend.cursor_position();
                match backend.capture_at_point(x, y) {
                    Ok((_path, best)) => {
                        el_counter += 1;
                        let output_key = format!("el_{el_counter:02}");
                        let label = best.label();
                        let extra = extra_info(&best);

                        println!("\n--- Capture #{el_counter} ---");
                        println!("  ✓ {label}{extra}");

                        // Prompt for tool type
                        let tool = prompt_tool_type();

                        // Prompt for parameters
                        let mut params = GenerateParams {
                            output_key,
                            text: None,
                            duration_ms: None,
                        };

                        if tool.needs_text() {
                            params.text = Some(prompt_text());
                        }
                        if tool.needs_duration() {
                            params.duration_ms = Some(prompt_duration());
                        }

                        // Generate node(s) and save
                        let new_nodes = generate_nodes(&best, tool, &params);
                        for node in &new_nodes {
                            let args_str = serde_json::to_string(&node.args).unwrap_or_default();
                            println!("    → {}: {}", node.tool, args_str);
                        }
                        nodes.extend(new_nodes);
                        _last_selector = Some(best);

                        print!("\n  [CTRL] continue  [ESC] discard  [Ctrl+Shift+F2] finish… ");
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        eprintln!("\n  ⚠ capture failed: {e}");
                        print!("\n  [CTRL] continue… ");
                        io::stdout().flush()?;
                    }
                }
            }
        }
    }

    if nodes.is_empty() {
        println!("  No captures taken.");
        return Ok(());
    }

    println!("  Generated {} nodes.", nodes.len());

    let output_str = format::format_output(&nodes, format);
    fs::write(output, &output_str)?;
    let format_name = if output.ends_with(".rs") {
        "Rust"
    } else {
        "JSON"
    };
    println!("  ✓ Saved {format_name} flow to {output}");

    Ok(())
}

// ── Prompt helpers ───────────────────────────────────────

/// Prompt for tool type on stdin.
fn prompt_tool_type() -> ToolType {
    loop {
        print!("  → Tool? [find/click/input_text/set_text/wait] ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        match input.trim().parse::<ToolType>() {
            Ok(t) => return t,
            Err(msg) => eprintln!("    {msg}"),
        }
    }
}

/// Prompt for text input (mandatory for input_text/set_text).
fn prompt_text() -> String {
    loop {
        print!("  → text: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
        eprintln!("    text cannot be empty");
    }
}

/// Prompt for duration in ms (for wait tool).
fn prompt_duration() -> u64 {
    loop {
        print!("  → duration_ms: [1000] ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return 1000;
        }
        match trimmed.parse::<u64>() {
            Ok(ms) if ms > 0 => return ms,
            _ => eprintln!("    must be a positive integer"),
        }
    }
}

/// Extra info for display: automation_id or class_name if available.
fn extra_info(sel: &BestSelector) -> String {
    if let Some(ref aid) = sel.automation_id {
        format!(" (automation_id: {aid})")
    } else if let Some(ref cn) = sel.class_name {
        format!(" (class_name: {cn})")
    } else if sel.control_type != "Custom" && !sel.control_type.is_empty() {
        format!(" (type: {})", sel.control_type)
    } else {
        String::new()
    }
}

// ──────────────────────────────────────────────────────────
//  Shared helpers
// ──────────────────────────────────────────────────────────

#[allow(clippy::cast_possible_truncation)]
fn timestamp_id() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Append a single capture to a JSON file (create or read+append).
fn append_capture(output: &str, new_cap: Capture) -> anyhow::Result<()> {
    let mut obj: CaptureOutput = fs::read_to_string(output)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(CaptureOutput {
            tool: "selector-capture".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            captures: Vec::new(),
        });
    obj.captures.push(new_cap);
    fs::write(output, serde_json::to_string_pretty(&obj)?)?;
    Ok(())
}
