// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod server;

use server::{AppState, PythonServer};

fn main() {
    // Start Python server.
    let python_server = PythonServer::new();
    if let Err(e) = python_server.start() {
        eprintln!("Warning: failed to start Python server: {e}");
        eprintln!("Tauri commands will fail without the backend.");
    }

    let app_state = AppState::new();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(python_server)
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
