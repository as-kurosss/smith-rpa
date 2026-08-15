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
    tools::{
        ClickTool, ExtractTool, FindTool, InputTextTool, ProcessTool, ScreenshotTool, SetTextTool,
        WaitTool,
    },
};
