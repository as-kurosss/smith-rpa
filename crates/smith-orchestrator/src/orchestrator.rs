//! Оркестратор запусков: приём роботов, отмена, история.
//!
//! Каждый запуск (`submit`) получает уникальный `JobId`, собственный
//! `CancellationToken` и выполняется в отдельной tokio-задаче поверх
//! `RobotExecutor`. Отмена (`cancel`) аннулирует токен; исполнитель
//! останавливается на ближайшей границе шага и фиксирует `Cancelled`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

use smith_core::ToolRegistry;
use smith_engine::{ReportStatus, Robot, RobotExecutor};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::job::{Job, JobId, JobStatus};

/// Ошибка операций оркестратора.
#[derive(Debug, Error, PartialEq)]
pub enum OrchestratorError {
    /// Запуск с указанным `JobId` не существует.
    #[error("Job not found: {id}")]
    JobNotFound { id: u64 },
    /// Запуск уже завершён и не может быть отменён.
    #[error("Job already finished: {id}")]
    JobAlreadyFinished { id: u64 },
}

/// Объединённое состояние: jobs + tokens в одном Mutex.
struct OrchestratorState {
    jobs: HashMap<u64, Job>,
    tokens: HashMap<u64, CancellationToken>,
}

/// Внутреннее состояние оркестратора (разделяется через `Arc`).
struct Inner {
    executor: Arc<RobotExecutor>,
    state: Mutex<OrchestratorState>,
    next_id: AtomicU64,
}

/// Менеджер запусков роботов.
#[derive(Clone)]
pub struct Orchestrator {
    inner: Arc<Inner>,
}

impl Orchestrator {
    /// Создаёт оркестратор поверх реестра инструментов.
    #[must_use]
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            inner: Arc::new(Inner {
                executor: Arc::new(RobotExecutor::new(registry)),
                state: Mutex::new(OrchestratorState {
                    jobs: HashMap::new(),
                    tokens: HashMap::new(),
                }),
                next_id: AtomicU64::new(0),
            }),
        }
    }

    /// Принимает робота на выполнение и возвращает `JobId`.
    ///
    /// Робот выполняется асинхронно; статус можно наблюдать через `get`.
    pub fn submit(&self, robot: Robot) -> JobId {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let token = CancellationToken::new();
        let robot_name = robot.name.clone();

        // Атомарная вставка job + token в один Mutex.
        {
            let mut state = lock(&self.inner.state);
            state
                .jobs
                .insert(id, Job::queued(JobId(id), robot_name.clone()));
            state.tokens.insert(id, token.clone());
        }

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            {
                let mut state = lock(&inner.state);
                if let Some(job) = state.jobs.get_mut(&id) {
                    job.status = JobStatus::Running;
                }
            }

            let report = inner.executor.execute(&robot, token).await;

            let status = match report.status {
                ReportStatus::Success => JobStatus::Succeeded,
                ReportStatus::Failed => JobStatus::Failed,
                ReportStatus::Cancelled => JobStatus::Cancelled,
            };

            {
                let mut state = lock(&inner.state);
                if let Some(job) = state.jobs.get_mut(&id) {
                    job.status = status;
                    job.report = Some(report);
                    job.finished_at = Some(SystemTime::now());
                }
                state.tokens.remove(&id);
            }

            debug!(job_id = id, status = ?status, "job finished");
        });

        debug!(job_id = id, robot = %robot_name, "job submitted");
        JobId(id)
    }

    /// Отменяет запуск через `CancellationToken`.
    ///
    /// # Errors
    ///
    /// Возвращает `JobNotFound`, если запуска с таким id нет,
    /// и `JobAlreadyFinished`, если запуск уже завершился.
    pub fn cancel(&self, id: JobId) -> Result<(), OrchestratorError> {
        let state = lock(&self.inner.state);
        let Some(job) = state.jobs.get(&id.0) else {
            return Err(OrchestratorError::JobNotFound { id: id.0 });
        };
        if matches!(
            job.status,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled
        ) {
            return Err(OrchestratorError::JobAlreadyFinished { id: id.0 });
        }
        let token = state.tokens.get(&id.0).cloned();
        drop(state); // Освобождаем мьютекс перед cancel (token.cancel() не требует блокировки)
        match token {
            Some(token) => {
                token.cancel();
                Ok(())
            }
            None => Err(OrchestratorError::JobAlreadyFinished { id: id.0 }),
        }
    }

    /// Возвращает текущее состояние запуска, если он существует.
    #[must_use]
    pub fn get(&self, id: JobId) -> Option<Job> {
        let state = lock(&self.inner.state);
        state.jobs.get(&id.0).cloned()
    }

    /// Возвращает историю всех запусков, отсортированную по `JobId`.
    #[must_use]
    pub fn history(&self) -> Vec<Job> {
        let state = lock(&self.inner.state);
        let mut all: Vec<Job> = state.jobs.values().cloned().collect();
        all.sort_by_key(|job| job.id.0);
        all
    }
}

/// Безопасный доступ к мьютексу: при отравлении продолжаем с данными.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::Deserialize;
    use serde::Serialize;
    use serde_json::{Value, json};
    use smith_core::{ExecutionContext, Tool, ToolError};
    use std::time::Duration;

    // --- Тестовые инструменты -------------------------------------------------

    #[derive(Debug, Serialize, Deserialize)]
    struct EchoInput {
        text: String,
    }

    #[derive(Debug, Serialize)]
    struct EchoOutput {
        echoed: String,
    }

    /// Возвращает переданный текст.
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        type Input = EchoInput;
        type Output = EchoOutput;

        fn name(&self) -> &'static str {
            "test.echo"
        }

        fn description(&self) -> &'static str {
            "echo test tool"
        }

        fn schema(&self) -> Value {
            json!({})
        }

        async fn execute(
            &self,
            input: EchoInput,
            _ctx: &mut ExecutionContext,
            _token: CancellationToken,
        ) -> Result<EchoOutput, ToolError> {
            Ok(EchoOutput { echoed: input.text })
        }
    }

    /// Всегда падает с ошибкой.
    struct FailTool;

    #[async_trait]
    impl Tool for FailTool {
        type Input = EchoInput;
        type Output = EchoOutput;

        fn name(&self) -> &'static str {
            "test.fail"
        }

        fn description(&self) -> &'static str {
            "failing test tool"
        }

        fn schema(&self) -> Value {
            json!({})
        }

        async fn execute(
            &self,
            _input: EchoInput,
            _ctx: &mut ExecutionContext,
            _token: CancellationToken,
        ) -> Result<EchoOutput, ToolError> {
            Err(ToolError::invalid_input("boom", None, None))
        }
    }

    /// Медленный инструмент: спит `delay_ms`, затем возвращает текст.
    struct SlowTool;

    #[derive(Debug, Serialize, Deserialize)]
    struct SlowInput {
        text: String,
        delay_ms: u64,
    }

    #[async_trait]
    impl Tool for SlowTool {
        type Input = SlowInput;
        type Output = EchoOutput;

        fn name(&self) -> &'static str {
            "test.slow"
        }

        fn description(&self) -> &'static str {
            "slow test tool"
        }

        fn schema(&self) -> Value {
            json!({})
        }

        async fn execute(
            &self,
            input: SlowInput,
            _ctx: &mut ExecutionContext,
            _token: CancellationToken,
        ) -> Result<EchoOutput, ToolError> {
            tokio::time::sleep(Duration::from_millis(input.delay_ms)).await;
            Ok(EchoOutput { echoed: input.text })
        }
    }

    // --- Помощники -----------------------------------------------------------

    fn test_registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        registry.register(FailTool);
        registry.register(SlowTool);
        registry
    }

    fn robot(name: &str, actions: &[(&str, Value)]) -> Robot {
        Robot {
            name: name.to_string(),
            version: "1.0".to_string(),
            steps: actions
                .iter()
                .map(|(action, params)| smith_engine::Step {
                    action: (*action).to_string(),
                    params: params.clone(),
                    outputs: Default::default(),
                })
                .collect(),
        }
    }

    /// Ждёт достижения ожидаемого статуса (с таймаутом — явный критерий остановки).
    async fn wait_for_status(orchestrator: &Orchestrator, id: JobId, expected: JobStatus) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if orchestrator.get(id).map(|job| job.status) == Some(expected) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job did not reach expected status in time");
    }

    // --- Тесты ---------------------------------------------------------------

    #[tokio::test]
    async fn test_submit_success_reaches_succeeded_with_report() {
        let orchestrator = Orchestrator::new(test_registry());
        let id = orchestrator.submit(robot("ok", &[("test.echo", json!({ "text": "hello" }))]));

        wait_for_status(&orchestrator, id, JobStatus::Succeeded).await;

        let job = orchestrator.get(id).expect("job should exist");
        assert_eq!(job.robot_name, "ok");
        assert_eq!(job.status, JobStatus::Succeeded);
        let report = job.report.expect("report should be present");
        assert_eq!(report.steps.len(), 1);
        assert!(report.steps[0].ok);
        assert_eq!(report.steps[0].output, Some(json!({ "echoed": "hello" })));
    }

    #[tokio::test]
    async fn test_submit_failure_reaches_failed() {
        let orchestrator = Orchestrator::new(test_registry());
        let id = orchestrator.submit(robot("bad", &[("test.fail", json!({ "text": "x" }))]));

        wait_for_status(&orchestrator, id, JobStatus::Failed).await;

        let job = orchestrator.get(id).expect("job should exist");
        assert_eq!(job.status, JobStatus::Failed);
        let report = job.report.expect("report should be present");
        assert!(!report.steps[0].ok);
    }

    #[tokio::test]
    async fn test_cancel_stops_running_job() {
        let orchestrator = Orchestrator::new(test_registry());
        let id = orchestrator.submit(robot(
            "slow",
            &[
                ("test.slow", json!({ "text": "step1", "delay_ms": 300 })),
                ("test.echo", json!({ "text": "step2" })),
            ],
        ));

        orchestrator.cancel(id).expect("cancel should succeed");
        wait_for_status(&orchestrator, id, JobStatus::Cancelled).await;

        let job = orchestrator.get(id).expect("job should exist");
        assert_eq!(job.status, JobStatus::Cancelled);
        let report = job.report.expect("report should be present");
        // Исполнитель остановился на границе шага: второй шаг не выполнен.
        assert!(report.steps.len() < 2);
    }

    #[tokio::test]
    async fn test_cancel_unknown_job_returns_not_found() {
        let orchestrator = Orchestrator::new(test_registry());
        assert_eq!(
            orchestrator.cancel(JobId(999)),
            Err(OrchestratorError::JobNotFound { id: 999 })
        );
    }

    #[tokio::test]
    async fn test_cancel_finished_job_returns_already_finished() {
        let orchestrator = Orchestrator::new(test_registry());
        let id = orchestrator.submit(robot("fast", &[("test.echo", json!({ "text": "done" }))]));

        wait_for_status(&orchestrator, id, JobStatus::Succeeded).await;
        assert_eq!(
            orchestrator.cancel(id),
            Err(OrchestratorError::JobAlreadyFinished { id: id.0 })
        );
    }

    #[tokio::test]
    async fn test_get_unknown_job_returns_none() {
        let orchestrator = Orchestrator::new(test_registry());
        assert!(orchestrator.get(JobId(4242)).is_none());
    }

    #[tokio::test]
    async fn test_history_returns_all_jobs_sorted() {
        let orchestrator = Orchestrator::new(test_registry());
        let a = orchestrator.submit(robot("a", &[("test.echo", json!({ "text": "a" }))]));
        let b = orchestrator.submit(robot("b", &[("test.echo", json!({ "text": "b" }))]));

        wait_for_status(&orchestrator, a, JobStatus::Succeeded).await;
        wait_for_status(&orchestrator, b, JobStatus::Succeeded).await;

        let history = orchestrator.history();
        let ids: Vec<u64> = history.iter().map(|job| job.id.0).collect();
        assert_eq!(ids, vec![a.0, b.0]);
        assert!(history.iter().all(|job| job.status == JobStatus::Succeeded));
    }
}
