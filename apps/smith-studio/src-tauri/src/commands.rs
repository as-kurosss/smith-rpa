//! Tauri-команды студии.
//!
//! M4: проверка связи и валидация JSON-модели робота.
//! M5: запуск/отмена/история через smith-orchestrator.
//! Debug: пошаговая отладка с breakpoints.

use std::fs;

use serde::Serialize;
use serde_json::Value;
use smith_core::ContextMap;
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
pub async fn run_robot(state: State<'_, Orchestrator>, json: String) -> Result<u64, String> {
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

/// Возвращает снимок контекста для указанного джоба.
#[tauri::command]
pub fn get_context_vars(state: State<'_, Orchestrator>, id: u64) -> ContextMap {
    state.context_snapshot(JobId(id))
}

/// Сохраняет текст в файл по указанному пути.
#[tauri::command]
pub fn save_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content.as_bytes()).map_err(|e| format!("Ошибка записи: {e}"))
}

// --- Пошаговая отладка -------------------------------------------------------

/// Запускает робота в пошаговом режиме (сразу ставит паузу на первом шаге).
#[tauri::command]
pub async fn run_debug(state: State<'_, Orchestrator>, json: String) -> Result<u64, String> {
    let robot = Robot::from_json_str(&json).map_err(|e| e.to_string())?;
    Ok(state.submit_debug(robot).0)
}

/// Устанавливает точки останова (индексы шагов) для запуска.
#[tauri::command]
pub async fn set_breakpoints(
    state: State<'_, Orchestrator>,
    id: u64,
    breakpoints: Vec<usize>,
) -> Result<(), String> {
    state
        .set_breakpoints(JobId(id), breakpoints.into_iter().collect())
        .await
        .map_err(|e| e.to_string())
}

/// Снимает паузу и продолжает выполнение до следующей паузы или завершения.
#[tauri::command]
pub fn resume_execution(state: State<'_, Orchestrator>, id: u64) -> Result<(), String> {
    state.resume(JobId(id)).map_err(|e| e.to_string())
}

/// Снимает паузу и ставит паузу после следующего шага (step-over).
#[tauri::command]
pub fn step_over(state: State<'_, Orchestrator>, id: u64) -> Result<(), String> {
    state.step_over(JobId(id)).map_err(|e| e.to_string())
}

/// Текущее состояние пошаговой отладки: индекс шага + на паузе ли.
#[derive(Serialize)]
pub struct DebugStatusView {
    pub current_step: usize,
    pub is_paused: bool,
}

/// Возвращает статус пошаговой отладки (индекс следующего шага + пауза).
#[tauri::command]
pub fn debug_status(
    state: State<'_, Orchestrator>,
    id: u64,
) -> Result<Option<DebugStatusView>, String> {
    let step = state.current_step(JobId(id));
    let paused = state.is_paused(JobId(id));
    match (step, paused) {
        (Some(step), Some(paused)) => Ok(Some(DebugStatusView {
            current_step: step,
            is_paused: paused,
        })),
        _ => Ok(None),
    }
}
