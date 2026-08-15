//! Windows backend for selector-capture.
//!
//! Uses:
//! - `uiautomation` crate for UI element tree traversal and capture.
//! - `rdev` for global keyboard/mouse hook (input events).
//! - `windows` crate for `GetCursorPos` (DPI-aware coordinates).
//! - `arboard` for clipboard access.

use std::sync::mpsc::Sender;
use std::thread;

use rdev::{listen, EventType, Key};
use uiautomation::core::UIAutomation;
use uiautomation::core::{UIElement, UITreeWalker};
use uiautomation::types::ControlType;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::backend::{Backend, SeriesEvent, SharedEvent};
use crate::types::{BestSelector, PathNode};

/// Windows UI Automation backend.
pub struct WindowsBackend;

impl WindowsBackend {
    /// Creates a new Windows backend, verifying UIA is available.
    ///
    /// # Errors
    ///
    /// Returns an error if UIAutomation initialisation fails.
    pub fn new() -> anyhow::Result<Self> {
        UIAutomation::new().map_err(|e| anyhow::anyhow!("UIA init failed: {e}"))?;
        Ok(Self)
    }
}

impl Backend for WindowsBackend {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn cursor_position(&self) -> (f64, f64) {
        // SAFETY: GetCursorPos is safe; zeroed struct is properly initialised.
        unsafe {
            let mut pt = std::mem::zeroed();
            if GetCursorPos(&raw mut pt).is_ok() {
                (f64::from(pt.x), f64::from(pt.y))
            } else {
                (0.0, 0.0)
            }
        }
    }

    fn capture_at_point(&self, x: f64, y: f64) -> Result<(Vec<PathNode>, BestSelector), String> {
        let automation = UIAutomation::new().map_err(|e| format!("UIA init: {e}"))?;
        let ix = x as i32;
        let iy = y as i32;

        let root = automation
            .get_root_element()
            .map_err(|e| format!("get_root_element: {e}"))?;
        let walker = automation
            .get_control_view_walker()
            .map_err(|e| format!("get_control_view_walker: {e}"))?;

        let element = find_deepest_at_point(root, &walker, ix, iy)?;

        // Walk up the tree to desktop root
        let walker = automation
            .create_tree_walker()
            .map_err(|e| format!("create_tree_walker: {e}"))?;

        let mut path: Vec<PathNode> = Vec::new();
        let mut current = element;
        loop {
            let node = read_node(&current);
            path.push(node);
            match walker.get_parent(&current) {
                Ok(parent) => current = parent,
                Err(_) => break,
            }
        }
        path.reverse();

        let best = best_selector(&path);
        Ok((path, best))
    }

    fn spawn_shared_listener(&self, tx: Sender<SharedEvent>) {
        thread::spawn(move || {
            let mut ctrl_down = false;
            let mut shift_down = false;
            let mut blocked = false;

            let _ = listen(move |event| {
                // ── Track modifier state ──
                match event.event_type {
                    EventType::KeyPress(Key::ControlLeft | Key::ControlRight) => {
                        ctrl_down = true;
                        blocked = false;
                    }
                    EventType::KeyRelease(Key::ControlLeft | Key::ControlRight) => {
                        if ctrl_down && !blocked {
                            let _ = tx.send(SharedEvent::Trigger);
                        }
                        ctrl_down = false;
                        blocked = false;
                    }
                    EventType::KeyPress(Key::ShiftLeft | Key::ShiftRight) => {
                        shift_down = true;
                    }
                    EventType::KeyRelease(Key::ShiftLeft | Key::ShiftRight) => {
                        shift_down = false;
                    }
                    _ => {}
                }

                // ── Actions ──
                match event.event_type {
                    // Non-modifier while CTRL held → combo, suppress trigger on release
                    EventType::KeyPress(ref k) if ctrl_down && !is_modifier(*k) => {
                        blocked = true;
                    }
                    // ESC (only when CTRL not held)
                    EventType::KeyPress(Key::Escape) if !ctrl_down => {
                        let _ = tx.send(SharedEvent::Escape);
                    }
                    // Ctrl+Shift+F2 → stop
                    EventType::KeyPress(Key::F2) if ctrl_down && shift_down => {
                        let _ = tx.send(SharedEvent::Stop);
                    }
                    _ => {}
                }
            });
        });
    }

    fn spawn_series_listener(&self, tx: Sender<SeriesEvent>) {
        thread::spawn(move || {
            let mut shift = false;
            let mut ctrl = false;
            let mut alt = false;

            let _ = listen(move |event| {
                // ── Update modifier states ──
                match event.event_type {
                    EventType::KeyPress(Key::ShiftLeft | Key::ShiftRight) => shift = true,
                    EventType::KeyRelease(Key::ShiftLeft | Key::ShiftRight) => shift = false,
                    EventType::KeyPress(Key::ControlLeft | Key::ControlRight) => ctrl = true,
                    EventType::KeyRelease(Key::ControlLeft | Key::ControlRight) => ctrl = false,
                    EventType::KeyPress(Key::Alt | Key::AltGr) => alt = true,
                    EventType::KeyRelease(Key::Alt | Key::AltGr) => alt = false,
                    _ => {}
                }

                match event.event_type {
                    // Stop: Ctrl+Shift+F2
                    EventType::KeyPress(Key::F2) if ctrl && shift => {
                        let _ = tx.send(SeriesEvent::Stop);
                    }
                    // Mouse clicks
                    EventType::ButtonPress(button) => {
                        let _ = tx.send(SeriesEvent::MouseDown {
                            button: format!("{button:?}"),
                        });
                    }
                    // Input: printable keys without Ctrl/Alt
                    EventType::KeyPress(ref key) if !ctrl && !alt && is_printable_key(*key) => {
                        let _ = tx.send(SeriesEvent::Input);
                    }
                    _ => {}
                }
            });
        });
    }

    fn copy_to_clipboard(&self, text: &str) -> bool {
        match arboard::Clipboard::new() {
            Ok(mut cb) => match cb.set_text(text) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("  ⚠ clipboard write failed: {e}");
                    false
                }
            },
            Err(e) => {
                eprintln!("  ⚠ clipboard init failed: {e}");
                false
            }
        }
    }
}

// ── Capture helpers ──

fn read_node(element: &UIElement) -> PathNode {
    let ct = element.get_control_type().unwrap_or(ControlType::Custom);
    PathNode {
        control_type: format!("{ct:?}"),
        class_name: element.get_classname().ok().filter(|s| !s.is_empty()),
        name: element.get_name().ok().filter(|s| !s.is_empty()),
        automation_id: element.get_automation_id().ok().filter(|s| !s.is_empty()),
    }
}

fn best_selector(path: &[PathNode]) -> BestSelector {
    let target = path.last().expect("path is non-empty");
    BestSelector {
        control_type: target.control_type.clone(),
        name: target.name.clone(),
        class_name: target.class_name.clone(),
        automation_id: target.automation_id.clone(),
    }
}

fn contains_point(element: &UIElement, x: i32, y: i32) -> bool {
    if let Ok(rect) = element.get_bounding_rectangle() {
        x >= rect.get_left()
            && x <= rect.get_right()
            && y >= rect.get_top()
            && y <= rect.get_bottom()
    } else {
        false
    }
}

fn find_deepest_at_point(
    mut current: UIElement,
    walker: &UITreeWalker,
    x: i32,
    y: i32,
) -> Result<UIElement, String> {
    loop {
        let Ok(mut child) = walker.get_first_child(&current) else {
            return Ok(current);
        };
        let mut found = false;
        loop {
            if contains_point(&child, x, y) {
                current = child;
                found = true;
                break;
            }
            match walker.get_next_sibling(&child) {
                Ok(next) => child = next,
                Err(_) => break,
            }
        }
        if !found {
            return Ok(current);
        }
    }
}

// ── Input helpers ──

fn is_modifier(k: Key) -> bool {
    matches!(
        k,
        Key::ControlLeft
            | Key::ControlRight
            | Key::ShiftLeft
            | Key::ShiftRight
            | Key::Alt
            | Key::AltGr
            | Key::MetaLeft
            | Key::MetaRight
    )
}

fn is_printable_key(key: Key) -> bool {
    matches!(
        key,
        Key::KeyA
            | Key::KeyB
            | Key::KeyC
            | Key::KeyD
            | Key::KeyE
            | Key::KeyF
            | Key::KeyG
            | Key::KeyH
            | Key::KeyI
            | Key::KeyJ
            | Key::KeyK
            | Key::KeyL
            | Key::KeyM
            | Key::KeyN
            | Key::KeyO
            | Key::KeyP
            | Key::KeyQ
            | Key::KeyR
            | Key::KeyS
            | Key::KeyT
            | Key::KeyU
            | Key::KeyV
            | Key::KeyW
            | Key::KeyX
            | Key::KeyY
            | Key::KeyZ
            | Key::Num0
            | Key::Num1
            | Key::Num2
            | Key::Num3
            | Key::Num4
            | Key::Num5
            | Key::Num6
            | Key::Num7
            | Key::Num8
            | Key::Num9
            | Key::Minus
            | Key::Equal
            | Key::LeftBracket
            | Key::RightBracket
            | Key::SemiColon
            | Key::Quote
            | Key::Comma
            | Key::Dot
            | Key::Slash
            | Key::BackSlash
            | Key::BackQuote
            | Key::Space
            | Key::Return
            | Key::Kp0
            | Key::Kp1
            | Key::Kp2
            | Key::Kp3
            | Key::Kp4
            | Key::Kp5
            | Key::Kp6
            | Key::Kp7
            | Key::Kp8
            | Key::Kp9
            | Key::KpPlus
            | Key::KpMinus
            | Key::KpMultiply
            | Key::KpDivide
            | Key::KpReturn
    )
}
