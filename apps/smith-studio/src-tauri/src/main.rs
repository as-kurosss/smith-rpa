// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    let orchestrator = smith_orchestrator::Orchestrator::new(smith_engine::default_registry());

    let result = tauri::Builder::default()
        .manage(orchestrator)
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::parse_robot,
            commands::run_robot,
            commands::cancel_job,
            commands::get_job,
            commands::get_history
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
