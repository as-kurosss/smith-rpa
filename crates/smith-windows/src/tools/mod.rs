// crates/smith-windows/src/tools/mod.rs
#[cfg(windows)]
pub mod click;
#[cfg(windows)]
pub mod find;
#[cfg(windows)]
pub mod input_text;
#[cfg(windows)]
pub mod process;
#[cfg(windows)]
pub mod set_text;
#[cfg(windows)]
pub mod wait;

#[cfg(windows)]
pub use click::{ClickInput, ClickOutput, ClickTool};
#[cfg(windows)]
pub use find::{FindInput, FindOutput, FindTool};
#[cfg(windows)]
pub use input_text::{InputTextInput, InputTextOutput, InputTextTool};
#[cfg(windows)]
pub use process::{ProcessInput, ProcessOutput, ProcessTool};
#[cfg(windows)]
pub use set_text::{SetTextInput, SetTextOutput, SetTextTool};
#[cfg(windows)]
pub use wait::{WaitInput, WaitOutput, WaitTool};
