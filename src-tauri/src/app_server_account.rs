use std::{
    io::{self, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    app_server_connection::AppServerConnection,
    app_server_handshake::perform_initialize_handshake_with_timeout,
    app_server_jsonl::JsonlError,
    app_server_protocol::{AccountReadParams, AccountReadResponse, method},
    app_server_session::{AppServerCommand, AppServerSession},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountStatus {
    pub state: CodexAccountState,
    pub plan_type: Option<String>,
    pub account_type: Option<String>,
    pub captured_at_ms: u64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexAccountState {
    SignedIn,
    SignedOut,
    Unavailable,
}

impl CodexAccountStatus {
    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CodexAccountState::Unavailable,
            plan_type: None,
            account_type: None,
            captured_at_ms,
            message: message.into(),
        }
    }
}

pub fn read_codex_account_status(timeout: Duration) -> CodexAccountStatus {
    let captured_at_ms = unix_now_ms();

    match read_account_status_from_codex(timeout, captured_at_ms) {
        Ok(status) => status,
        Err(error) => {
            CodexAccountStatus::unavailable(safe_account_error_message(&error), captured_at_ms)
        }
    }
}

fn read_account_status_from_codex(
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexAccountStatus, JsonlError> {
    let mut session = AppServerSession::start(AppServerCommand::codex_from_environment())
        .map_err(JsonlError::from)?;
    let mut connection =
        AppServerConnection::from_session(&mut session).map_err(JsonlError::from)?;

    read_account_status_from_connection(&mut connection, timeout, captured_at_ms)
}

pub fn read_account_status_from_connection<W>(
    connection: &mut AppServerConnection<W>,
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexAccountStatus, JsonlError>
where
    W: Write,
{
    perform_initialize_handshake_with_timeout(connection, timeout)?;

    let response = connection.request(
        method::ACCOUNT_READ,
        Some(AccountReadParams {
            refresh_token: Some(false),
        }),
        timeout,
    )?;

    Ok(account_status_from_response(response, captured_at_ms))
}

fn account_status_from_response(
    response: AccountReadResponse,
    captured_at_ms: u64,
) -> CodexAccountStatus {
    let Some(account) = response.account else {
        return CodexAccountStatus {
            state: CodexAccountState::SignedOut,
            plan_type: None,
            account_type: None,
            captured_at_ms,
            message: if response.requires_openai_auth {
                "Codex CLI 需要重新登录 OpenAI 账户".to_string()
            } else {
                "Codex CLI 当前未返回账户信息".to_string()
            },
        };
    };

    let plan_type = account
        .plan_type
        .map(|plan_type| plan_type.as_str().to_string());
    let account_type = Some(account.account_type);
    let message = match plan_type.as_deref() {
        Some(plan_type) => format!("真实账户已连接，套餐 {plan_type}"),
        None => "真实账户已连接，套餐未提供".to_string(),
    };

    CodexAccountStatus {
        state: CodexAccountState::SignedIn,
        plan_type,
        account_type,
        captured_at_ms,
        message,
    }
}

fn safe_account_error_message(error: &JsonlError) -> String {
    match error {
        JsonlError::Timeout { .. } => "读取 Codex 账户超时".to_string(),
        JsonlError::EndOfStream => "Codex app-server 已关闭连接".to_string(),
        JsonlError::Server(_) => "Codex app-server 拒绝账户读取请求".to_string(),
        JsonlError::Json(_) => "Codex app-server 返回了无法解析的账户数据".to_string(),
        JsonlError::Io(error) if error.kind() == io::ErrorKind::NotFound => {
            "未检测到 Codex CLI，无法读取账户".to_string()
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
    fn account_status_preserves_signed_in_plan_without_credentials() {
        let response: AccountReadResponse = serde_json::from_value(serde_json::json!({
            "account": {
                "type": "chatgpt",
                "email": "person@example.invalid",
                "planType": "future_super_plan",
                "credentialSource": "auth_json"
            },
            "requiresOpenaiAuth": false
        }))
        .expect("account response");

        let status = account_status_from_response(response, 123);

        assert_eq!(status.state, CodexAccountState::SignedIn);
        assert_eq!(status.plan_type.as_deref(), Some("future_super_plan"));
        assert_eq!(status.account_type.as_deref(), Some("chatgpt"));
        assert_eq!(status.captured_at_ms, 123);
        assert!(!status.message.contains("person@example.invalid"));
        assert!(!status.message.contains("auth_json"));
    }

    #[test]
    fn account_status_reports_signed_out_when_account_is_missing() {
        let response: AccountReadResponse = serde_json::from_value(serde_json::json!({
            "account": null,
            "requiresOpenaiAuth": true
        }))
        .expect("signed out response");

        let status = account_status_from_response(response, 456);

        assert_eq!(status.state, CodexAccountState::SignedOut);
        assert_eq!(status.plan_type, None);
        assert_eq!(status.account_type, None);
        assert!(status.message.contains("重新登录"));
    }

    #[test]
    fn account_read_connection_performs_handshake_then_account_request() {
        let stdout = concat!(
            "{\"id\":1,\"result\":{\"userAgent\":\"fake-codex\"}}\n",
            "{\"id\":2,\"result\":{\"account\":{\"type\":\"chatgpt\",\"planType\":\"pro\"},\"requiresOpenaiAuth\":false}}\n"
        )
        .as_bytes();
        let stdin = Vec::new();
        let stderr = io::empty();
        let mut connection = AppServerConnection::new(stdout, stdin, stderr);

        let status =
            read_account_status_from_connection(&mut connection, Duration::from_secs(1), 789)
                .expect("account status");
        let written = String::from_utf8(connection.into_writer()).expect("written JSONL");

        assert_eq!(status.state, CodexAccountState::SignedIn);
        assert_eq!(status.plan_type.as_deref(), Some("pro"));
        assert!(written.contains("\"method\":\"initialize\""));
        assert!(written.contains("\"method\":\"initialized\""));
        assert!(written.contains("\"method\":\"account/read\""));
        assert!(written.contains("\"refreshToken\":false"));
    }
}
