// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::parse_robot
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
