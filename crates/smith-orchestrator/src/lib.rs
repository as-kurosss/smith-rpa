//! smith-orchestrator — менеджер запусков RPA-роботов.
//!
//! Предоставляет `Orchestrator`: приём роботов на выполнение (`submit`),
//! отмену через `CancellationToken` (`cancel`), запрос статуса (`get`)
//! и историю запусков (`history`) с итоговыми `ExecutionReport`.

pub mod job;
pub mod orchestrator;

pub use job::{Job, JobId, JobStatus};
pub use orchestrator::{Orchestrator, OrchestratorError};
