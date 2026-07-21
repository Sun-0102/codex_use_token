use std::time::Duration;

use crate::{
    app_server_account, app_server_client::AppServerRuntime, app_server_rate_limits,
    app_server_thread_usage, app_server_usage, cc_switch_usage, cli_probe, desktop,
};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealth {
    app_version: &'static str,
    monitor_state: &'static str,
}

impl RuntimeHealth {
    fn foundation() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION"),
            monitor_state: "notConnected",
        }
    }
}

#[tauri::command]
pub fn runtime_health() -> RuntimeHealth {
    RuntimeHealth::foundation()
}

#[tauri::command]
pub async fn codex_cli_status() -> cli_probe::CodexCliStatus {
    tauri::async_runtime::spawn_blocking(cli_probe::probe_codex_cli)
        .await
        .unwrap_or_else(|_| cli_probe::CodexCliStatus {
            state: cli_probe::CodexCliState::LaunchFailed,
            executable_path: None,
            version: None,
            message: "Codex CLI 探测任务异常退出".to_string(),
        })
}

#[tauri::command]
pub fn codex_account_status(
    runtime: tauri::State<'_, AppServerRuntime>,
) -> app_server_account::CodexAccountStatus {
    let client = runtime.client();
    client
        .lock()
        .expect("app-server runtime lock")
        .read_account_status(Duration::from_secs(8))
}

#[tauri::command]
pub fn codex_rate_limits_status(
    runtime: tauri::State<'_, AppServerRuntime>,
) -> app_server_rate_limits::CodexRateLimitsStatus {
    let client = runtime.client();
    client
        .lock()
        .expect("app-server runtime lock")
        .read_rate_limits_status(Duration::from_secs(8))
}

#[tauri::command]
pub fn codex_usage_status(
    runtime: tauri::State<'_, AppServerRuntime>,
) -> app_server_usage::CodexUsageStatus {
    let client = runtime.client();
    client
        .lock()
        .expect("app-server runtime lock")
        .read_usage_status(Duration::from_secs(8))
}

#[tauri::command]
pub async fn cc_switch_usage_status() -> cc_switch_usage::CcSwitchUsageStatus {
    tauri::async_runtime::spawn_blocking(cc_switch_usage::read_cc_switch_usage_status)
        .await
        .unwrap_or_else(|_| cc_switch_usage::CcSwitchUsageStatus {
            state: cc_switch_usage::CcSwitchUsageState::Unavailable,
            captured_at_ms: 0,
            today: None,
            message: "cc-switch 今日 Token 统计读取任务异常退出".to_string(),
        })
}

#[tauri::command]
pub fn codex_thread_token_usage_status(
    runtime: tauri::State<'_, AppServerRuntime>,
) -> app_server_thread_usage::CodexThreadTokenUsageStatus {
    let client = runtime.client();
    client
        .lock()
        .expect("app-server runtime lock")
        .read_thread_token_usage_status(Duration::from_secs(8), Duration::from_millis(500))
}

#[tauri::command]
pub fn hide_usage_window(app: tauri::AppHandle) {
    desktop::hide_usage_window(&app);
}

#[tauri::command]
pub fn set_usage_window_mode(
    window: tauri::WebviewWindow,
    mode: desktop::WindowMode,
) -> Result<(), String> {
    desktop::apply_window_mode(&window, mode).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_tray_usage(
    app: tauri::AppHandle,
    primary_remaining_percent: Option<u8>,
    secondary_remaining_percent: Option<u8>,
) -> Result<(), String> {
    let title = format_tray_title(primary_remaining_percent, secondary_remaining_percent)?;
    let tooltip = format_tray_tooltip(primary_remaining_percent, secondary_remaining_percent)?;
    let tray = app
        .tray_by_id(desktop::USAGE_TRAY_ID)
        .ok_or_else(|| "用量状态栏尚未初始化".to_string())?;

    tray.set_title(Some(&title))
        .map_err(|error| error.to_string())?;
    tray.set_tooltip(Some(&tooltip))
        .map_err(|error| error.to_string())
}

fn format_tray_title(primary: Option<u8>, secondary: Option<u8>) -> Result<String, String> {
    validate_tray_percent(primary)?;
    validate_tray_percent(secondary)?;

    Ok(format!(
        "5h {} · W {}",
        format_tray_percent(primary),
        format_tray_percent(secondary)
    ))
}

fn format_tray_tooltip(primary: Option<u8>, secondary: Option<u8>) -> Result<String, String> {
    validate_tray_percent(primary)?;
    validate_tray_percent(secondary)?;

    Ok(format!(
        "Codex Reserve · 5 小时剩余 {} · 长周期剩余 {}",
        format_tray_percent(primary),
        format_tray_percent(secondary)
    ))
}

fn validate_tray_percent(value: Option<u8>) -> Result<(), String> {
    if value.is_some_and(|percent| percent > 100) {
        return Err("剩余用量百分比必须在 0 到 100 之间".to_string());
    }

    Ok(())
}

fn format_tray_percent(value: Option<u8>) -> String {
    value.map_or("--".to_string(), |percent| format!("{percent}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_health_reports_monitor_as_disconnected() {
        let health = RuntimeHealth::foundation();

        assert_eq!(health.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(health.monitor_state, "notConnected");
    }

    #[test]
    fn tray_title_keeps_both_quota_windows_glanceable() {
        assert_eq!(
            format_tray_title(Some(73), Some(41)),
            Ok("5h 73% · W 41%".into())
        );
    }

    #[test]
    fn tray_title_keeps_weekly_quota_visible_when_short_window_is_missing() {
        assert_eq!(
            format_tray_title(None, Some(75)),
            Ok("5h -- · W 75%".into())
        );
    }

    #[test]
    fn tray_title_rejects_invalid_percentages() {
        assert_eq!(
            format_tray_title(Some(101), Some(41)),
            Err("剩余用量百分比必须在 0 到 100 之间".into())
        );
    }
}
