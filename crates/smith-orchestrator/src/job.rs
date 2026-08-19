//! Модель запуска: `JobId`, `JobStatus`, `Job`.
//!
//! `Job` — неизменяемая запись о запуске робота. Жизненный цикл:
//! `Queued` → `Running` → `Succeeded` | `Failed` | `Cancelled`.

use std::time::SystemTime;

use serde::Serialize;

use smith_engine::ExecutionReport;

/// Уникальный идентификатор запуска.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct JobId(pub u64);

/// Статус запуска (жизненный цикл джоба).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Принят, ожидает выполнения.
    Queued,
    /// Выполняется.
    Running,
    /// Завершён успешно.
    Succeeded,
    /// Завершён с ошибкой шага.
    Failed,
    /// Отменён через токен отмены.
    Cancelled,
    /// Приостановлен (пошаговая отладка).
    Paused,
}

/// Запись о запуске робота.
#[derive(Debug, Clone, Serialize)]
pub struct Job {
    /// Уникальный идентификатор.
    pub id: JobId,
    /// Имя робота (из `Robot::name`).
    pub robot_name: String,
    /// Текущий статус.
    pub status: JobStatus,
    /// Итоговый отчёт (заполняется после завершения).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ExecutionReport>,
    /// Время приёма запуска.
    pub created_at: SystemTime,
    /// Время завершения.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<SystemTime>,
}

impl Job {
    /// Создаёт запись в статусе `Queued`.
    #[must_use]
    pub fn queued(id: JobId, robot_name: String) -> Self {
        Self {
            id,
            robot_name,
            status: JobStatus::Queued,
            report: None,
            created_at: SystemTime::now(),
            finished_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_queued_initial_state() {
        let job = Job::queued(JobId(7), "Demo".into());
        assert_eq!(job.id, JobId(7));
        assert_eq!(job.robot_name, "Demo");
        assert_eq!(job.status, JobStatus::Queued);
        assert!(job.report.is_none());
        assert!(job.finished_at.is_none());
    }

    #[test]
    fn test_job_status_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&JobStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
        assert_eq!(
            serde_json::to_string(&JobStatus::Paused).unwrap(),
            "\"paused\""
        );
    }
}
