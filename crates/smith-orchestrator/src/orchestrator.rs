//! Оркестратор запусков: приём роботов, отмена, история.
//!
//! Каждый запуск (`submit`) получает уникальный `JobId`, собственный
//! `CancellationToken` и выполняется в отдельной tokio-задаче поверх
//! `RobotExecutor`. Отмена (`cancel`) аннулирует токен; исполнитель
//! останавливается на ближайшей границе шага и фиксирует `Cancelled`.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

use smith_core::{ContextMap, ToolRegistry};
use smith_engine::{DebugController, ReportStatus, Robot, RobotExecutor};
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

/// Объединённое состояние: jobs + tokens + debug controllers в одном Mutex.
struct OrchestratorState {
    jobs: HashMap<u64, Job>,
    tokens: HashMap<u64, CancellationToken>,
    /// Контроллеры пошаговой отладки (только для debug-режима).
    controllers: HashMap<u64, Arc<DebugController>>,
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
                    controllers: HashMap::new(),
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

        // Атомарная вставка job + token + статус Running в один Mutex.
        {
            let mut state = lock(&self.inner.state);
            state
                .jobs
                .insert(id, Job::queued(JobId(id), robot_name.clone()));
            state.tokens.insert(id, token.clone());
            if let Some(job) = state.jobs.get_mut(&id) {
                job.status = JobStatus::Running;
            }
        }

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let report = inner.executor.execute(id, &robot, token, None).await;

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

    /// Возвращает последний снимок контекста для указанного джоба.
    #[must_use]
    pub fn context_snapshot(&self, id: JobId) -> ContextMap {
        let store = self.inner.executor.contexts();
        if let Ok(snapshots) = store.lock()
            && let Some(steps) = snapshots.get(&id.0)
        {
            return steps.last().cloned().unwrap_or_default();
        }
        ContextMap::new()
    }

    /// Запускает робота в пошаговом режиме (сразу ставит паузу на первом шаге).
    ///
    /// Возвращает `JobId`. Статус джоба: `Paused`. Фронтенд управляет
    /// выполнением через `resume()` / `step_over()`.
    pub async fn submit_debug(&self, robot: Robot, breakpoints: HashSet<usize>) -> JobId {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let token = CancellationToken::new();
        let robot_name = robot.name.clone();
        let controller = Arc::new(DebugController::new(token.clone()));

        // Ставим паузу сразу: executor остановится перед первым шагом.
        controller.set_pause_after_step(true);

        // Вставка job + token + controllers + статус Paused.
        // Breakpoints устанавливаются в том же lock — до spawn executor task,
        // чтобы исключить race condition (executor завершается и удаляет
        // контроллер до того, как set_breakpoints найдёт его).
        {
            let mut state = lock(&self.inner.state);
            state
                .jobs
                .insert(id, Job::queued(JobId(id), robot_name.clone()));
            state.tokens.insert(id, token.clone());
            state.controllers.insert(id, controller.clone());
            if let Some(job) = state.jobs.get_mut(&id) {
                job.status = JobStatus::Paused;
            }
        }
        // Устанавливаем breakpoints в отдельном блоке (RwLock::write().await).
        tracing::info!(job_id = id, "submit_debug: about to set breakpoints");
        controller.set_breakpoints(breakpoints).await;
        tracing::info!(job_id = id, "submit_debug: breakpoints set, spawning executor");

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let report = inner
                .executor
                .execute(id, &robot, token, Some(controller.clone()))
                .await;

            // Если executor на паузе (breakpoint/step-over) — НЕ перезаписываем
            // статус Paused. Executor возвращает Success когда делает break
            // из-за паузы, но джоб всё ещё на паузе.
            let status = if controller.is_paused() {
                JobStatus::Paused
            } else {
                match report.status {
                    ReportStatus::Success => JobStatus::Succeeded,
                    ReportStatus::Failed => JobStatus::Failed,
                    ReportStatus::Cancelled => JobStatus::Cancelled,
                }
            };

            {
                let mut state = lock(&inner.state);
                if let Some(job) = state.jobs.get_mut(&id) {
                    job.status = status;
                    job.report = Some(report);
                    job.finished_at = if controller.is_paused() {
                        None
                    } else {
                        Some(SystemTime::now())
                    };
                }
                state.tokens.remove(&id);
                // Контроллер НЕ удаляется: debug_status должен возвращать
                // валидные данные даже после завершения джоба ( UI polling ).
            }

            debug!(job_id = id, status = ?status, "debug job finished");
        });

        debug!(job_id = id, robot = %robot_name, "debug job submitted");
        JobId(id)
    }

    /// Снимает паузу и продолжает выполнение до следующей паузы или завершения.
    ///
    /// # Errors
    ///
    /// Возвращает `JobNotFound`, если запуск не найден.
    pub fn resume(&self, id: JobId) -> Result<(), OrchestratorError> {
        let mut state = lock(&self.inner.state);
        let Some(controller) = state.controllers.get(&id.0) else {
            return Err(OrchestratorError::JobNotFound { id: id.0 });
        };
        controller.continue_execution();
        // Обновляем статус на Running.
        if let Some(job) = state.jobs.get_mut(&id.0)
            && job.status == JobStatus::Paused
        {
            job.status = JobStatus::Running;
        }
        Ok(())
    }

    /// Снимает паузу и ставит паузу после следующего шага (step-over).
    ///
    /// # Errors
    ///
    /// Возвращает `JobNotFound`, если запуск не найден.
    pub fn step_over(&self, id: JobId) -> Result<(), OrchestratorError> {
        let mut state = lock(&self.inner.state);
        let Some(controller) = state.controllers.get(&id.0) else {
            return Err(OrchestratorError::JobNotFound { id: id.0 });
        };
        controller.step_over();
        if let Some(job) = state.jobs.get_mut(&id.0)
            && job.status == JobStatus::Paused
        {
            job.status = JobStatus::Running;
        }
        Ok(())
    }

    /// Устанавливает точки останова для запуска.
    ///
    /// # Errors
    ///
    /// Возвращает `JobNotFound`, если запуск не найден.
    pub async fn set_breakpoints(
        &self,
        id: JobId,
        breakpoints: HashSet<usize>,
    ) -> Result<(), OrchestratorError> {
        let controller = {
            let state = lock(&self.inner.state);
            state
                .controllers
                .get(&id.0)
                .cloned()
                .ok_or(OrchestratorError::JobNotFound { id: id.0 })?
        };
        controller.set_breakpoints(breakpoints).await;
        Ok(())
    }

    /// Возвращает текущий индекс шага (0-based, «следующий к выполнению»).
    #[must_use]
    pub fn current_step(&self, id: JobId) -> Option<usize> {
        let state = lock(&self.inner.state);
        state.controllers.get(&id.0).map(|c| c.current_step())
    }

    /// Возвращает `true`, если запуск находится на паузе.
    #[must_use]
    pub fn is_paused(&self, id: JobId) -> Option<bool> {
        let state = lock(&self.inner.state);
        state.controllers.get(&id.0).map(|c| c.is_paused())
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

    #[tokio::test]
    async fn test_submit_debug_with_breakpoint_pauses_at_step() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
            .with_test_writer()
            .try_init();
        let orchestrator = Orchestrator::new(test_registry());
        let mut bp = HashSet::new();
        bp.insert(1); // Breakpoint на шаге 1
        let id = orchestrator
            .submit_debug(
                robot(
                    "bp",
                    &[
                        ("test.echo", json!({"text": "step0"})),
                        ("test.echo", json!({"text": "step1"})),
                        ("test.echo", json!({"text": "step2"})),
                    ],
                ),
                bp,
            )
            .await;

        // Ждём, пока executor выполнит шаг 0 и поставит паузу.
        // current_step должно стать 1 (следующий к выполнению).
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if orchestrator.current_step(id) == Some(1) && orchestrator.is_paused(id) == Some(true) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("executor should pause at step 1 after first step");
        tracing::info!("test: paused after submit_debug, step=1");

        // Step-over: выполняет шаг 1 (breakpoint), ставит паузу.
        // current_step остаётся 1 (breakpoint срабатывает до update_step).
        orchestrator.step_over(id).expect("step_over");
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if orchestrator.current_step(id) == Some(1) && orchestrator.is_paused(id) == Some(true) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("executor should pause at breakpoint on step 1");

        // Resume: выполняет шаг 2 и завершает.
        orchestrator.resume(id).expect("resume");
        wait_for_status(&orchestrator, id, JobStatus::Succeeded).await;
    }
}
