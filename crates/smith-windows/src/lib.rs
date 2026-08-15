// crates/smith-windows/src/lib.rs

#[cfg(windows)]
pub mod element;
#[cfg(windows)]
pub mod selector;
pub mod tools;

#[cfg(windows)]
pub use {
    element::SafeUIElement,
    selector::ElementSelector,
    tools::{ClickTool, FindTool, InputTextTool, ProcessTool, SetTextTool, WaitTool},
};
