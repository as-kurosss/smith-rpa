//! Сборка default-реестра инструментов.
//!
//! MVP: Windows UIA-инструменты (на Windows) + универсальный HTTP-инструмент.

pub mod http;

pub use http::{HttpInput, HttpOutput, HttpTool};

use smith_core::ToolRegistry;

/// Собирает реестр инструментов по умолчанию.
///
/// На Windows регистрирует UIA-инструменты (`windows.*`) и `http.request`.
/// На других платформах доступен только `http.request`.
#[must_use]
pub fn default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    #[cfg(windows)]
    {
        use smith_windows::{
            ClickTool, ExtractTool, FindTool, InputTextTool, ProcessTool, ScreenshotTool,
            SetTextTool, WaitTool,
        };
        registry.register(ClickTool::new());
        registry.register(FindTool::new());
        registry.register(InputTextTool::new());
        registry.register(ProcessTool::new());
        registry.register(SetTextTool::new());
        registry.register(WaitTool::new());
        registry.register(ExtractTool::new());
        registry.register(ScreenshotTool::new());
    }

    registry.register(HttpTool::new());
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_contains_http_tool() {
        let registry = default_registry();
        assert!(registry.get("http.request").is_some());
    }

    #[cfg(windows)]
    #[test]
    fn test_default_registry_contains_windows_tools() {
        let registry = default_registry();
        assert!(registry.get("windows.click").is_some());
        assert!(registry.get("windows.find").is_some());
        assert!(registry.get("windows.extract").is_some());
        assert!(registry.get("windows.screenshot").is_some());
    }
}
