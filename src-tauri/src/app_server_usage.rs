use std::{
    io::{self, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    app_server_connection::AppServerConnection,
    app_server_handshake::perform_initialize_handshake_with_timeout,
    app_server_jsonl::JsonlError,
    app_server_protocol::{AccountTokenUsageDailyBucket, AccountUsageReadResponse, method},
    app_server_session::{AppServerCommand, AppServerSession},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageStatus {
    pub state: CodexUsageState,
    pub captured_at_ms: u64,
    pub summary: Option<CodexUsageSummary>,
    pub daily_usage_buckets: Vec<CodexDailyUsageBucket>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexUsageState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageSummary {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub longest_running_turn_sec: Option<i64>,
    pub current_streak_days: Option<i64>,
    pub longest_streak_days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexDailyUsageBucket {
    pub start_date: String,
    pub tokens: i64,
}

impl CodexUsageStatus {
    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CodexUsageState::Unavailable,
            captured_at_ms,
            summary: None,
            daily_usage_buckets: Vec::new(),
            message: message.into(),
        }
    }
}

pub fn read_codex_usage_status(timeout: Duration) -> CodexUsageStatus {
    let captured_at_ms = unix_now_ms();

    match read_usage_from_codex(timeout, captured_at_ms) {
        Ok(status) => status,
        Err(error) => {
            CodexUsageStatus::unavailable(safe_usage_error_message(&error), captured_at_ms)
        }
    }
}

fn read_usage_from_codex(
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexUsageStatus, JsonlError> {
    let mut session =
        AppServerSession::start(AppServerCommand::codex()).map_err(JsonlError::from)?;
    let mut connection =
        AppServerConnection::from_session(&mut session).map_err(JsonlError::from)?;

    read_usage_from_connection(&mut connection, timeout, captured_at_ms)
}

pub fn read_usage_from_connection<W>(
    connection: &mut AppServerConnection<W>,
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexUsageStatus, JsonlError>
where
    W: Write,
{
    perform_initialize_handshake_with_timeout(connection, timeout)?;

    let response = connection.request(method::ACCOUNT_USAGE_READ, Option::<()>::None, timeout)?;

    Ok(usage_status_from_response(response, captured_at_ms))
}

fn usage_status_from_response(
    response: AccountUsageReadResponse,
    captured_at_ms: u64,
) -> CodexUsageStatus {
    let daily_usage_buckets = response
        .daily_usage_buckets
        .unwrap_or_default()
        .into_iter()
        .map(daily_bucket_from_protocol)
        .collect::<Vec<_>>();
    let bucket_count = daily_usage_buckets.len();

    CodexUsageStatus {
        state: CodexUsageState::Available,
        captured_at_ms,
        summary: Some(CodexUsageSummary {
            lifetime_tokens: response.summary.lifetime_tokens,
            peak_daily_tokens: response.summary.peak_daily_tokens,
            longest_running_turn_sec: response.summary.longest_running_turn_sec,
            current_streak_days: response.summary.current_streak_days,
            longest_streak_days: response.summary.longest_streak_days,
        }),
        daily_usage_buckets,
        message: format!("已读取 {bucket_count} 个每日 Token 用量桶"),
    }
}

fn daily_bucket_from_protocol(bucket: AccountTokenUsageDailyBucket) -> CodexDailyUsageBucket {
    CodexDailyUsageBucket {
        start_date: bucket.start_date,
        tokens: bucket.tokens,
    }
}

fn safe_usage_error_message(error: &JsonlError) -> String {
    match error {
        JsonlError::Timeout { .. } => "读取 Codex Token 用量超时".to_string(),
        JsonlError::EndOfStream => "Codex app-server 已关闭连接".to_string(),
        JsonlError::Server(_) => "Codex app-server 拒绝 Token 用量读取请求".to_string(),
        JsonlError::Json(_) => "Codex app-server 返回了无法解析的 Token 用量数据".to_string(),
        JsonlError::Io(error) if error.kind() == io::ErrorKind::NotFound => {
            "未检测到 Codex CLI，无法读取 Token 用量".to_string()
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
    use crate::app_server_protocol::ServerResponse;

    #[test]
    fn usage_status_preserves_summary_and_daily_buckets() {
        let response: ServerResponse<AccountUsageReadResponse> =
            serde_json::from_str(include_str!(
                "../tests/fixtures/app_server_protocol/0.144.5/responses/account-usage.json"
            ))
            .expect("usage response");

        let status = usage_status_from_response(response.result, 123);

        assert_eq!(status.state, CodexUsageState::Available);
        assert_eq!(status.captured_at_ms, 123);
        assert_eq!(
            status
                .summary
                .as_ref()
                .and_then(|summary| summary.lifetime_tokens),
            Some(1_234_567)
        );
        assert_eq!(
            status
                .summary
                .as_ref()
                .and_then(|summary| summary.peak_daily_tokens),
            Some(98_765)
        );
        assert_eq!(status.daily_usage_buckets.len(), 2);
        assert_eq!(status.daily_usage_buckets[1].start_date, "2026-07-20");
        assert_eq!(status.daily_usage_buckets[1].tokens, 23_456);
    }

    #[test]
    fn usage_status_accepts_sparse_summary() {
        let response: ServerResponse<AccountUsageReadResponse> =
            serde_json::from_str(include_str!(
                "../tests/fixtures/app_server_protocol/0.144.5/responses/account-usage-sparse.json"
            ))
            .expect("sparse usage response");

        let status = usage_status_from_response(response.result, 456);

        assert_eq!(status.state, CodexUsageState::Available);
        assert_eq!(
            status
                .summary
                .as_ref()
                .and_then(|summary| summary.lifetime_tokens),
            None
        );
        assert!(status.daily_usage_buckets.is_empty());
    }

    #[test]
    fn usage_connection_performs_handshake_then_usage_request() {
        let stdout = concat!(
            "{\"id\":1,\"result\":{\"userAgent\":\"fake-codex\"}}\n",
            "{\"id\":2,\"result\":{\"summary\":{\"lifetimeTokens\":42},\"dailyUsageBuckets\":[{\"startDate\":\"2026-07-20\",\"tokens\":7}]}}\n"
        )
        .as_bytes();
        let stdin = Vec::new();
        let stderr = io::empty();
        let mut connection = AppServerConnection::new(stdout, stdin, stderr);

        let status = read_usage_from_connection(&mut connection, Duration::from_secs(1), 789)
            .expect("usage");
        let written = String::from_utf8(connection.into_writer()).expect("written JSONL");

        assert_eq!(status.state, CodexUsageState::Available);
        assert_eq!(status.daily_usage_buckets[0].tokens, 7);
        assert!(written.contains("\"method\":\"initialize\""));
        assert!(written.contains("\"method\":\"initialized\""));
        assert!(written.contains("\"method\":\"account/usage/read\""));
    }
}
