//! smith-cli — утилита для запуска и валидации RPA-роботов.
//!
//! Подкоманды:
//! - `validate <file.json>` — проверяет JSON-модель робота без выполнения.
//! - `run <file.json>` — выполняет робота через `RobotExecutor` и печатает `ExecutionReport`.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use smith_engine::{ReportStatus, Robot, RobotExecutor, default_registry};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

/// Точка входа: парсинг аргументов и диспетчеризация подкоманды.
#[derive(Debug, Parser)]
#[command(name = "smith-cli", version, about = "RPA robot runner and validator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Подкоманды smith-cli.
#[derive(Debug, Subcommand)]
enum Command {
    /// Проверить JSON-модель робота без выполнения.
    Validate {
        /// Путь к JSON-файлу робота.
        path: PathBuf,
    },
    /// Выполнить робота и вывести отчёт.
    Run {
        /// Путь к JSON-файлу робота.
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    let cli = Cli::parse();
    match cli.command {
        Command::Validate { path } => validate(&path),
        Command::Run { path } => run(&path).await,
    }
}

/// Инициализирует tracing-subscriber с фильтром из `RUST_LOG`.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}

/// Выводит строку в stdout.
///
/// # Errors
///
/// Возвращает `io::Error`, если запись в stdout невозможна.
fn write_stdout(line: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}")
}

/// Выводит строку в stderr.
///
/// # Errors
///
/// Возвращает `io::Error`, если запись в stderr невозможна.
fn write_stderr(line: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{line}")
}

/// Читает и парсит JSON-файл робота.
///
/// # Errors
///
/// Возвращает `String` с описанием ошибки, если файл нельзя прочитать
/// или JSON не является корректной моделью `Robot`.
fn load_robot(path: &std::path::Path) -> Result<Robot, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    Robot::from_json_str(&content).map_err(|e| format!("Invalid robot JSON: {e}"))
}

/// Подкоманда `validate`: печатает сводку или ошибку, возвращает код выхода.
fn validate(path: &std::path::Path) -> ExitCode {
    match load_robot(path) {
        Ok(robot) => {
            let summary = format!(
                "OK: robot '{}' version {} ({} steps)",
                robot.name,
                robot.version,
                robot.steps.len()
            );
            let _ = write_stdout(&summary);
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = write_stderr(&message);
            ExitCode::FAILURE
        }
    }
}

/// Подкоманда `run`: выполняет робота и печатает `ExecutionReport`.
async fn run(path: &std::path::Path) -> ExitCode {
    let robot = match load_robot(path) {
        Ok(robot) => robot,
        Err(message) => {
            let _ = write_stderr(&message);
            return ExitCode::FAILURE;
        }
    };

    let executor = RobotExecutor::new(default_registry());
    let report = executor.execute(&robot, CancellationToken::new()).await;

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            let _ = write_stdout(&json);
            match report.status {
                ReportStatus::Success => ExitCode::SUCCESS,
                ReportStatus::Failed | ReportStatus::Cancelled => ExitCode::FAILURE,
            }
        }
        Err(error) => {
            let message = format!("Failed to serialize report: {error}");
            let _ = write_stderr(&message);
            ExitCode::FAILURE
        }
    }
}
