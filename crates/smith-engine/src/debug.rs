//! Пошаговая отладка: контроллер пауз, breakpoints, step-over.
//!
//! `DebugController` — shared-объект (`Arc`), связывающий executor-loop
//! с внешним API (оркестратор → Tauri → фронтенд).
//! Использует `tokio::sync::watch` для управления потоком выполнения
//! и `tokio::sync::Notify` для пробуждения executor после паузы.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Решение executor-loop на каждой итерации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    /// Выполнить следующий шаг.
    Execute,
    /// Приостановиться (breakpoint или step-over).
    Pause,
}

/// Разделяемое состояние контроллера (вынесено в `Arc` для безопасного
/// клонирования — клон разделяет одни и те же атомики и breakpoints).
struct SharedState {
    /// Точки останова: индексы шагов, на которых нужно сделать паузу.
    breakpoints: RwLock<HashSet<usize>>,
    /// Текущий индекс шага (0-based, «следующий к выполнению»).
    current_step: AtomicUsize,
    /// Находится ли executor на паузе.
    paused: AtomicBool,
    /// Флаг: поставить паузу после завершения текущего шага (step-over).
    pause_after_step: AtomicBool,
    /// Флаг возобновления: устанавливается continue/step_over.
    /// Позволяет `wait_for_resume` выйти, даже если watch-сигнал был отправлен до входа в wait.
    resume_flag: AtomicBool,
}

/// Контроллер пошаговой отладки.
///
/// Связывает executor (consumer) с фронтендом (producer).
/// Потокобезопасен: может быть обёрнут в `Arc` и разделяться между задачами.
pub struct DebugController {
    shared: Arc<SharedState>,
    /// Токен отмены: позволяет разбудить executor при отмене во время паузы.
    token: CancellationToken,
}

impl DebugController {
    /// Создаёт контроллер в состоянии «не на паузе».
    #[must_use]
    pub fn new(token: CancellationToken) -> Self {
        Self {
            shared: Arc::new(SharedState {
                breakpoints: RwLock::new(HashSet::new()),
                current_step: AtomicUsize::new(0),
                paused: AtomicBool::new(false),
                pause_after_step: AtomicBool::new(false),
                resume_flag: AtomicBool::new(false),
            }),
            token,
        }
    }

    /// Устанавливает точки останова (полная замена набора).
    pub async fn set_breakpoints(&self, points: HashSet<usize>) {
        *self.shared.breakpoints.write().await = points;
    }

    /// Добавляет одну точку останова.
    pub async fn add_breakpoint(&self, index: usize) {
        self.shared.breakpoints.write().await.insert(index);
    }

    /// Удаляет одну точку останова.
    pub async fn remove_breakpoint(&self, index: usize) {
        self.shared.breakpoints.write().await.remove(&index);
    }

    /// Проверяет, должна ли выполнение приостановиться перед шагом `step_index`.
    ///
    /// Если да — выставляет флаг `paused` и блокируется до получения
    /// сигнала `Continue` или `StepOver`. Если на паузе без breakpoint —
    /// ждёт сигнала. Если не нужно паузиться — возвращает `Execute`.
    ///
    /// # Arguments
    ///
    /// * `step_index` — индекс следующего шага (0-based).
    ///
    /// # Returns
    ///
    /// `StepAction::Execute` если можно продолжать, `StepAction::Pause` если
    /// выполнение приостановлено (после возобновления тоже возвращает `Pause`,
    /// чтобы executor знал о паузе).
    pub async fn should_continue(&self, step_index: usize) -> StepAction {
        // Если уже на паузе — проверяем resume_flag.
        // Если флаг установлен (resume/step_over) — НЕ снимаем паузу здесь:
        // пусть check_step_over снимет её после выполнения шага.
        if self.shared.paused.load(Ordering::SeqCst) {
            if self.shared.resume_flag.swap(false, Ordering::SeqCst) {
                tracing::info!(step_index, "debug: should_continue — paused but resume_flag set, allowing step");
                return StepAction::Execute;
            }
            tracing::info!(step_index, "debug: should_continue — paused, waiting");
            self.wait_for_resume().await;
            return StepAction::Pause;
        }

        // Проверяем breakpoint.
        {
            let bp = self.shared.breakpoints.read().await;
            tracing::info!(step_index, breakpoints = ?bp.iter().copied().collect::<Vec<_>>(), "debug: should_continue — checking breakpoints");
            if bp.contains(&step_index) {
                drop(bp);
                tracing::info!(step_index, "debug: should_continue — BREAKPOINT HIT at step {step_index}");
                self.shared.paused.store(true, Ordering::SeqCst);
                self.wait_for_resume().await;
                return StepAction::Pause;
            }
        }

        tracing::info!(step_index, "debug: should_continue — no breakpoint, Execute");
        StepAction::Execute
    }

    /// Обновляет текущий индекс шага (вызывается executor после каждого шага).
    pub fn update_step(&self, index: usize) {
        self.shared.current_step.store(index, Ordering::SeqCst);
    }

    /// Проверяет, нужно ли поставить паузу после завершения шага (step-over).
    ///
    /// Если флаг установлен — выставляет `paused = true` и возвращает `true`.
    pub fn check_step_over(&self) -> bool {
        let was_set = self
            .shared
            .pause_after_step
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        tracing::info!(
            was_set,
            paused = self.shared.paused.load(Ordering::SeqCst),
            "debug: check_step_over"
        );
        if was_set {
            self.shared.paused.store(true, Ordering::SeqCst);
        }
        was_set
    }

    /// Пробуждает executor: снимает паузу и отправляет сигнал `Continue`.
    pub fn continue_execution(&self) {
        self.shared.resume_flag.store(true, Ordering::SeqCst);
        self.shared.paused.store(false, Ordering::SeqCst);
    }

    /// Снимает паузу и ставит флаг «пауза после следующего шага» (step-over).
    /// Параметр `paused` НЕ снимается здесь: `should_continue` проверит
    /// `resume_flag` и вернёт Execute, а `check_step_over` поставит паузу
    /// после выполнения шага.
    pub fn step_over(&self) {
        self.shared.pause_after_step.store(true, Ordering::SeqCst);
        self.shared.resume_flag.store(true, Ordering::SeqCst);
    }

    /// Возвращает текущий индекс шага («следующий к выполнению»).
    #[must_use]
    pub fn current_step(&self) -> usize {
        self.shared.current_step.load(Ordering::SeqCst)
    }

    /// Возвращает `true`, если executor находится на паузе.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::SeqCst)
    }

    /// Устанавливает флаг «пауза после следующего шага» (для submit_debug).
    pub fn set_pause_after_step(&self, value: bool) {
        self.shared.pause_after_step.store(value, Ordering::SeqCst);
    }

    /// Блокируется до получения сигнала `Continue`, `StepOver`
    /// или отмены через `CancellationToken`.
    ///
    /// Использует yield_now() для协作ативного ожидания — надёжнее чем
    /// Notify/watch каналы (нет гонок "notify до subscribe").
    async fn wait_for_resume(&self) {
        loop {
            if self.token.is_cancelled() {
                self.shared.paused.store(false, Ordering::SeqCst);
                return;
            }
            if self.shared.resume_flag.swap(false, Ordering::SeqCst) {
                self.shared.paused.store(false, Ordering::SeqCst);
                return;
            }
            if !self.shared.paused.load(Ordering::SeqCst) {
                return;
            }
            tokio::task::yield_now().await;
        }
    }
}

impl Default for DebugController {
    fn default() -> Self {
        Self::new(CancellationToken::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Ручной Clone для тестов: shared state разделяется через Arc,
    /// каналы тоже разделяют состояние (watch).
    impl Clone for DebugController {
        fn clone(&self) -> Self {
            Self {
                shared: Arc::clone(&self.shared),
                token: self.token.clone(),
            }
        }
    }

    #[tokio::test]
    async fn test_no_breakpoints_returns_execute() {
        let dc = DebugController::default();
        assert_eq!(dc.should_continue(0).await, StepAction::Execute);
        assert_eq!(dc.should_continue(1).await, StepAction::Execute);
        assert_eq!(dc.should_continue(5).await, StepAction::Execute);
    }

    #[tokio::test]
    async fn test_breakpoint_pauses_and_resumes() {
        let dc = DebugController::default();
        dc.add_breakpoint(2).await;

        // Шаги 0, 1 — Execute.
        assert_eq!(dc.should_continue(0).await, StepAction::Execute);
        assert_eq!(dc.should_continue(1).await, StepAction::Execute);

        // Запускаем should_continue(2) в фоне — breakpoint ставит паузу.
        let dc2 = dc.clone();
        let handle = tokio::spawn(async move { dc2.should_continue(2).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(dc.is_paused());

        // continue_execution снимает паузу.
        dc.continue_execution();
        let result = tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("task should complete")
            .expect("ok");
        assert_eq!(result, StepAction::Pause);
    }

    #[tokio::test]
    async fn test_step_over_pauses_after_next_step() {
        let dc = DebugController::default();
        assert!(!dc.is_paused());

        // step_over: выставляет pause_after_step + resume_flag.
        dc.step_over();
        assert!(!dc.is_paused());

        // executor вызывает check_step_over после выполнения шага.
        assert!(dc.check_step_over());
        assert!(dc.is_paused());

        // should_continue с resume_flag вернёт Execute (шаг выполняется).
        let dc2 = dc.clone();
        let result = tokio::time::timeout(Duration::from_secs(1), async {
            dc2.should_continue(1).await
        })
        .await
        .expect("timeout");
        assert_eq!(result, StepAction::Execute);
    }

    #[tokio::test]
    async fn test_set_breakpoints_replaces_all() {
        let dc = DebugController::default();
        dc.add_breakpoint(0).await;
        dc.add_breakpoint(1).await;

        let mut new_set = HashSet::new();
        new_set.insert(5);
        dc.set_breakpoints(new_set).await;

        // Шаг 0 и 1 больше не breakpoints.
        assert_eq!(dc.should_continue(0).await, StepAction::Execute);
        assert_eq!(dc.should_continue(1).await, StepAction::Execute);

        // Шаг 5 — breakpoint.
        let dc2 = dc.clone();
        let handle = tokio::spawn(async move { dc2.should_continue(5).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(dc.is_paused());
        dc.continue_execution();
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("task should complete")
            .expect("ok");
    }

    #[tokio::test]
    async fn test_update_step_and_current_step() {
        let dc = DebugController::default();
        assert_eq!(dc.current_step(), 0);
        dc.update_step(3);
        assert_eq!(dc.current_step(), 3);
    }
}
