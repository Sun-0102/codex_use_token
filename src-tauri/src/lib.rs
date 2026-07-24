pub mod app_server_account;
pub mod app_server_client;
pub mod app_server_connection;
pub mod app_server_handshake;
pub mod app_server_jsonl;
pub mod app_server_protocol;
pub mod app_server_rate_limits;
pub mod app_server_session;
pub mod app_server_supervisor;
pub mod app_server_thread_usage;
pub mod app_server_usage;
mod cli_probe;
pub mod codex_session_usage;
mod commands;
mod desktop;
mod monitor_refresh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_positioner::init());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(app_server_client::AppServerRuntime::default())
        .setup(desktop::setup)
        .on_window_event(desktop::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::runtime_health,
            commands::codex_cli_status,
            commands::codex_account_status,
            commands::codex_rate_limits_status,
            commands::codex_usage_status,
            commands::codex_session_usage_status,
            commands::codex_thread_token_usage_status,
            commands::hide_usage_window,
            commands::set_usage_window_mode,
            commands::update_tray_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
