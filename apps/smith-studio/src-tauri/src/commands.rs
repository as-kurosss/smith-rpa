//! Tauri-команды студии.
//!
//! M4: проверка связи и валидация JSON-модели робота.
//! M5: запуск/отмена/история через smith-orchestrator.

use serde_json::Value;
use smith_engine::Robot;
use smith_orchestrator::{Job, JobId, Orchestrator};
use tauri::State;

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

/// Запускает робота из JSON-строки и возвращает `JobId`.
///
/// # Errors
///
/// Возвращает строку с описанием ошибки, если JSON не корректен.
#[tauri::command]
pub fn run_robot(state: State<'_, Orchestrator>, json: String) -> Result<u64, String> {
    let robot = Robot::from_json_str(&json).map_err(|e| e.to_string())?;
    Ok(state.submit(robot).0)
}

/// Отменяет запуск по `JobId`.
///
/// # Errors
///
/// Возвращает строку с описанием ошибки, если запуск не найден или завершён.
#[tauri::command]
pub fn cancel_job(state: State<'_, Orchestrator>, id: u64) -> Result<(), String> {
    state.cancel(JobId(id)).map_err(|e| e.to_string())
}

/// Возвращает текущее состояние запуска по `JobId` (или `None`).
#[tauri::command]
pub fn get_job(state: State<'_, Orchestrator>, id: u64) -> Result<Option<Job>, String> {
    Ok(state.get(JobId(id)))
}

/// Возвращает историю всех запусков.
#[tauri::command]
pub fn get_history(state: State<'_, Orchestrator>) -> Result<Vec<Job>, String> {
    Ok(state.history())
}
