// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    // Tracing: RUST_LOG=info cargo tauri dev для вывода логов.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();

    let orchestrator = smith_orchestrator::Orchestrator::new(smith_engine::default_registry());

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(orchestrator)
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::parse_robot,
            commands::run_robot,
            commands::cancel_job,
            commands::get_job,
            commands::get_history,
            commands::get_context_vars,
            commands::save_file,
            commands::run_debug,
            commands::set_breakpoints,
            commands::resume_execution,
            commands::step_over,
            commands::debug_status
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        let _ = std::io::Write::write_fmt(
            &mut std::io::stderr(),
            format_args!("Failed to run smith-studio: {error}\n"),
        );
        std::process::exit(1);
    }
}
