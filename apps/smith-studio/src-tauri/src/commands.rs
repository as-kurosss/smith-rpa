//! Tauri-команды студии.
//!
//! M4: минимальные команды для проверки связи с фронтендом и валидации
//! JSON-модели робота. M5 добавляет запуск/отмену через smith-orchestrator.

use serde_json::Value;
use smith_engine::Robot;

/// Проверка доступности backend.
#[tauri::command]
pub fn health() -> String {
    "ok".into()
}

/// Парсит и валидирует JSON-модель робота.
///
/// # Errors
///
/// Возвращает строку с описанием ошибки, если JSON не корректен.
#[tauri::command]
pub fn parse_robot(json: String) -> Result<Value, String> {
    let robot = Robot::from_json_str(&json).map_err(|e| e.to_string())?;
    serde_json::to_value(robot).map_err(|e| e.to_string())
}
