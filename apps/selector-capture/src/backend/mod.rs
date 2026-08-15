//! Platform backend abstraction for selector-capture.
//!
//! The `Backend` trait defines all platform-specific operations:
//! - UI element capture (UIA on Windows, AT-SPI on Linux, CDP on browser)
//! - Input event listening (keyboard/mouse hooks)
//! - Clipboard access
//!
//! # Factory
//!
//! Call `create_backend("windows")` to get a Windows backend.
//! New backends are added behind `#[cfg(feature = "...")]` gates.

use std::sync::mpsc::Sender;

use crate::types::{BestSelector, PathNode};

// ── Event types ───────────────────────────────────────────

/// Events produced by the shared-mode input listener
/// (used by `single` and `record` subcommands).
pub enum SharedEvent {
    /// CTRL pressed alone — capture the element under cursor.
    Trigger,
    /// ESC pressed — discard last capture.
    Escape,
    /// Ctrl+Shift+F2 — stop the current mode.
    Stop,
}

/// Events produced by the series-mode input listener
/// (used by `series` subcommand).
pub enum SeriesEvent {
    /// Mouse button pressed.
    #[allow(dead_code)]
    MouseDown { button: String },
    /// Printable key pressed (text input detected).
    Input,
    /// Ctrl+Shift+F2 — stop recording.
    Stop,
}

// ── Backend trait ─────────────────────────────────────────

/// Platform-specific operations for UI element capture.
pub trait Backend: Send + Sync {
    /// Human-readable backend name (e.g. `"windows"`, `"linux"`, `"browser"`).
    fn name(&self) -> &'static str;

    /// Returns the current cursor position in screen coordinates (logical pixels).
    fn cursor_position(&self) -> (f64, f64);

    /// Captures the UI element at the given screen coordinates.
    ///
    /// Returns `(full_tree_path, best_flat_selector)`.
    fn capture_at_point(&self, x: f64, y: f64) -> Result<(Vec<PathNode>, BestSelector), String>;

    /// Spawns a background thread that listens for shared-mode input events
    /// and sends them through `tx`.
    fn spawn_shared_listener(&self, tx: Sender<SharedEvent>);

    /// Spawns a background thread that listens for series-mode input events
    /// and sends them through `tx`.
    fn spawn_series_listener(&self, tx: Sender<SeriesEvent>);

    /// Copies text to the system clipboard.
    ///
    /// Returns `true` on success, `false` if clipboard is unavailable.
    fn copy_to_clipboard(&self, text: &str) -> bool;
}

// ── Factory ───────────────────────────────────────────────

#[cfg(all(feature = "windows", windows))]
mod windows;

/// Creates a backend by name.
///
/// # Errors
///
/// Returns an error if the requested backend is not compiled in or initialisation fails.
pub fn create_backend(name: &str) -> anyhow::Result<Box<dyn Backend>> {
    match name {
        #[cfg(all(feature = "windows", windows))]
        "windows" => Ok(Box::new(windows::WindowsBackend::new()?)),
        _ => {
            let available = {
                #[cfg(all(feature = "windows", windows))]
                {
                    " windows"
                }
                #[cfg(not(all(feature = "windows", windows)))]
                {
                    ""
                }
            };
            anyhow::bail!("Unknown backend '{name}'. Available:{available}")
        }
    }
}
