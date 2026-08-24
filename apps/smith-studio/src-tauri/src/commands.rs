//! Tauri-команды: проксируют вызовы в Python-сервер через HTTP.

use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

use crate::server::AppState;

/// Проверка доступности backend.
#[tauri::command]
pub fn health(state: State<'_, AppState>) -> Result<String, String> {
    state.proxy_get("/health")?;
    Ok("ok".into())
}

/// Парсит и валидирует JSON-модель робота.
#[tauri::command]
pub fn parse_robot(state: State<'_, AppState>, json: String) -> Result<Value, String> {
    state.proxy_post("/parse-robot", &json!({"robot_json": json}))
}

/// Запускает робота из JSON-строки и возвращает `JobId`.
#[tauri::command]
pub fn run_robot(state: State<'_, AppState>, json: String) -> Result<u64, String> {
    let robot: Value =
        serde_json::from_str(&json).map_err(|e| format!("Invalid robot JSON: {e}"))?;
    let result = state.proxy_post("/run-robot", &json!({"robot": robot}))?;
    result["job_id"]
        .as_u64()
        .ok_or_else(|| "Missing job_id in response".into())
}

/// Отменяет запуск по `JobId`.
#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.proxy_post(&format!("/cancel-job/{id}"), &json!({}))?;
    Ok(())
}

/// Возвращает текущее состояние запуска по `JobId` (или `None`).
#[tauri::command]
pub fn get_job(state: State<'_, AppState>, id: u64) -> Result<Option<Value>, String> {
    match state.proxy_get(&format!("/job/{id}")) {
        Ok(val) => {
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(val))
            }
        }
        Err(e) => {
            if e.contains("404") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Возвращает историю всех запусков.
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let val = state.proxy_get("/history")?;
    val.as_array()
        .cloned()
        .ok_or_else(|| "Expected array".into())
}

/// Возвращает снимок контекста для указанного джоба.
#[tauri::command]
pub fn get_context_vars(state: State<'_, AppState>, id: u64) -> Result<Value, String> {
    state.proxy_get(&format!("/context/{id}"))
}

/// Сохраняет текст в файл по указанному пути.
#[tauri::command]
pub fn save_file(state: State<'_, AppState>, path: String, content: String) -> Result<(), String> {
    state.proxy_post("/save-file", &json!({"path": path, "content": content}))?;
    Ok(())
}

/// Запускает робота в пошаговом режиме (сразу ставит паузу на первом шаге).
#[tauri::command]
pub fn run_debug(
    state: State<'_, AppState>,
    json: String,
    breakpoints: Vec<usize>,
) -> Result<u64, String> {
    let robot: Value =
        serde_json::from_str(&json).map_err(|e| format!("Invalid robot JSON: {e}"))?;
    let result = state.proxy_post(
        "/run-debug",
        &json!({"robot": robot, "breakpoints": breakpoints}),
    )?;
    result["job_id"]
        .as_u64()
        .ok_or_else(|| "Missing job_id in response".into())
}

/// Устанавливает точки останова (индексы шагов) для запуска.
#[tauri::command]
pub fn set_breakpoints(
    state: State<'_, AppState>,
    id: u64,
    breakpoints: Vec<usize>,
) -> Result<(), String> {
    state.proxy_post(
        &format!("/set-breakpoints/{id}"),
        &json!({"breakpoints": breakpoints}),
    )?;
    Ok(())
}

/// Снимает паузу и продолжает выполнение до следующей паузы или завершения.
#[tauri::command]
pub fn resume_execution(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.proxy_post(&format!("/resume/{id}"), &json!({}))?;
    Ok(())
}

/// Снимает паузу и ставит паузу после следующего шага (step-over).
#[tauri::command]
pub fn step_over(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    state.proxy_post(&format!("/step-over/{id}"), &json!({}))?;
    Ok(())
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
    state: State<'_, AppState>,
    id: u64,
) -> Result<Option<DebugStatusView>, String> {
    match state.proxy_get(&format!("/debug-status/{id}")) {
        Ok(val) => {
            if val.is_null() {
                return Ok(None);
            }
            let current_step = val["current_step"]
                .as_u64()
                .unwrap_or(0) as usize;
            let is_paused = val["is_paused"].as_bool().unwrap_or(false);
            Ok(Some(DebugStatusView {
                current_step,
                is_paused,
            }))
        }
        Err(e) => {
            if e.contains("404") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}
