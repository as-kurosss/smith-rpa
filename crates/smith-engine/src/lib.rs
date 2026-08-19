//! smith-engine — линейный исполнитель RPA-роботов.
//!
//! MVP-философия: всё — инструменты. Робот — это JSON-список шагов,
//! каждый шаг вызывает инструмент по имени через единый `ToolRegistry`.
//!
//! Содержит:
//! - `robot` — модель Robot/Step и сериализация JSON.
//! - `interpolate` — подстановка `{{var}}` из `ExecutionContext`.
//! - `executor` — линейное исполнение шагов с `ExecutionReport`.
//! - `tools` — универсальный `HttpTool` и сборка default-реестра.

pub mod debug;
pub mod executor;
pub mod interpolate;
pub mod robot;
pub mod tools;

pub use debug::{DebugController, StepAction};
pub use executor::{ContextStore, ExecutionReport, ReportStatus, RobotExecutor, StepResult};
pub use interpolate::interpolate;
pub use robot::{Robot, Step};
pub use tools::{HttpInput, HttpOutput, HttpTool, default_registry};
