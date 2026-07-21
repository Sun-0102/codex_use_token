use std::{
    io, thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    app_server_connection::AppServerConnection,
    app_server_handshake::perform_initialize_handshake_with_timeout,
    app_server_jsonl::{AppServerNotification, JsonlError},
    app_server_session::{AppServerCommand, AppServerSession},
};

pub const THREAD_TOKEN_USAGE_UPDATED: &str = "thread/tokenUsage/updated";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexThreadTokenUsageStatus {
    pub state: CodexThreadTokenUsageState,
    pub captured_at_ms: u64,
    pub usage: Option<CodexThreadTokenUsage>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexThreadTokenUsageState {
    Available,
    Waiting,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexThreadTokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawThreadTokenUsage {
    #[serde(default, alias = "input")]
    input_tokens: Option<i64>,
    #[serde(default, alias = "cachedInput")]
    cached_input_tokens: Option<i64>,
    #[serde(default, alias = "cached")]
    cached_tokens: Option<i64>,
    #[serde(default, alias = "output")]
    output_tokens: Option<i64>,
    #[serde(default, alias = "reasoning")]
    reasoning_output_tokens: Option<i64>,
    #[serde(default, alias = "reasoningTokens")]
    reasoning_tokens: Option<i64>,
    #[serde(default, alias = "total")]
    total_tokens: Option<i64>,
}

impl CodexThreadTokenUsageStatus {
    fn waiting(captured_at_ms: u64) -> Self {
        Self {
            state: CodexThreadTokenUsageState::Waiting,
            captured_at_ms,
            usage: None,
            message: "等待当前 app-server 连接的 thread/tokenUsage/updated 通知".to_string(),
        }
    }

    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CodexThreadTokenUsageState::Unavailable,
            captured_at_ms,
            usage: None,
            message: message.into(),
        }
    }
}

pub fn read_codex_thread_token_usage_status(
    handshake_timeout: Duration,
    wait_timeout: Duration,
) -> CodexThreadTokenUsageStatus {
    let captured_at_ms = unix_now_ms();

    match read_thread_token_usage_from_codex(handshake_timeout, wait_timeout, captured_at_ms) {
        Ok(status) => status,
        Err(error) => CodexThreadTokenUsageStatus::unavailable(
            safe_thread_usage_error_message(&error),
            captured_at_ms,
        ),
    }
}

fn read_thread_token_usage_from_codex(
    handshake_timeout: Duration,
    wait_timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexThreadTokenUsageStatus, JsonlError> {
    let mut session = AppServerSession::start(AppServerCommand::codex_from_environment())
        .map_err(JsonlError::from)?;
    let mut connection =
        AppServerConnection::from_session(&mut session).map_err(JsonlError::from)?;

    perform_initialize_handshake_with_timeout(&mut connection, handshake_timeout)?;

    let deadline = std::time::Instant::now() + wait_timeout;
    while std::time::Instant::now() < deadline {
        if let Some(notification) = connection.try_next_notification()
            && let Some(status) =
                thread_token_usage_status_from_notification(&notification, captured_at_ms)
        {
            return Ok(status);
        }

        thread::sleep(Duration::from_millis(25));
    }

    Ok(CodexThreadTokenUsageStatus::waiting(captured_at_ms))
}

pub fn thread_token_usage_status_from_notification(
    notification: &AppServerNotification,
    captured_at_ms: u64,
) -> Option<CodexThreadTokenUsageStatus> {
    if notification.method != THREAD_TOKEN_USAGE_UPDATED {
        return None;
    }

    let raw = serde_json::from_value::<RawThreadTokenUsage>(notification.params.clone()?).ok()?;
    let input_tokens = raw.input_tokens.unwrap_or(0);
    let cached_input_tokens = raw.cached_input_tokens.or(raw.cached_tokens).unwrap_or(0);
    let output_tokens = raw.output_tokens.unwrap_or(0);
    let reasoning_output_tokens = raw
        .reasoning_output_tokens
        .or(raw.reasoning_tokens)
        .unwrap_or(0);
    let total_tokens = raw
        .total_tokens
        .unwrap_or(input_tokens + cached_input_tokens + output_tokens + reasoning_output_tokens);

    Some(CodexThreadTokenUsageStatus {
        state: CodexThreadTokenUsageState::Available,
        captured_at_ms,
        usage: Some(CodexThreadTokenUsage {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_output_tokens,
            total_tokens,
        }),
        message: "已接收当前连接可见任务的 Token 用量通知".to_string(),
    })
}

fn safe_thread_usage_error_message(error: &JsonlError) -> String {
    match error {
        JsonlError::Timeout { .. } => "等待 Codex 线程 Token 通知超时".to_string(),
        JsonlError::EndOfStream => "Codex app-server 已关闭连接".to_string(),
        JsonlError::Server(_) => "Codex app-server 拒绝线程 Token 通知读取".to_string(),
        JsonlError::Json(_) => "Codex app-server 返回了无法解析的线程 Token 通知".to_string(),
        JsonlError::Io(error) if error.kind() == io::ErrorKind::NotFound => {
            "未检测到 Codex CLI，无法接收线程 Token 通知".to_string()
        }
        JsonlError::Io(_) => "无法启动或通信到 Codex app-server".to_string(),
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thread_token_usage_updated_notification() {
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "thread/tokenUsage/updated",
            "params": {
                "inputTokens": 100,
                "cachedInputTokens": 25,
                "outputTokens": 40,
                "reasoningOutputTokens": 9,
                "totalTokens": 174
            }
        }))
        .expect("notification");

        let status = thread_token_usage_status_from_notification(&notification, 123)
            .expect("thread token usage");

        assert_eq!(status.state, CodexThreadTokenUsageState::Available);
        assert_eq!(
            status.usage,
            Some(CodexThreadTokenUsage {
                input_tokens: 100,
                cached_input_tokens: 25,
                output_tokens: 40,
                reasoning_output_tokens: 9,
                total_tokens: 174,
            })
        );
    }

    #[test]
    fn computes_total_when_notification_omits_it() {
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "thread/tokenUsage/updated",
            "params": {
                "input": 10,
                "cached": 2,
                "output": 3,
                "reasoning": 4
            }
        }))
        .expect("notification");

        let status = thread_token_usage_status_from_notification(&notification, 456)
            .expect("thread token usage");

        assert_eq!(
            status.usage.as_ref().map(|usage| usage.total_tokens),
            Some(19)
        );
    }

    #[test]
    fn ignores_unrelated_notifications() {
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "account/rateLimits/updated",
            "params": {}
        }))
        .expect("notification");

        assert_eq!(
            thread_token_usage_status_from_notification(&notification, 789),
            None
        );
    }
}
