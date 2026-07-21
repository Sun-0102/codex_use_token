use std::{
    collections::BTreeMap,
    io::{self, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    app_server_connection::AppServerConnection,
    app_server_handshake::perform_initialize_handshake_with_timeout,
    app_server_jsonl::{AppServerNotification, JsonlError},
    app_server_protocol::{
        CreditsSnapshot, RateLimitSnapshot, RateLimitWindow, RateLimitsReadResponse, method,
    },
    app_server_session::{AppServerCommand, AppServerSession},
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitsStatus {
    pub state: CodexRateLimitsState,
    pub captured_at_ms: u64,
    pub buckets: Vec<CodexRateLimitBucket>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexRateLimitsState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitBucket {
    pub source: CodexRateLimitBucketSource,
    pub key: Option<String>,
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub primary: Option<CodexRateLimitWindow>,
    pub secondary: Option<CodexRateLimitWindow>,
    pub credits: Option<CodexCreditsSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexRateLimitBucketSource {
    Default,
    ByLimitId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitWindow {
    pub used_percent: i32,
    pub resets_at: Option<i64>,
    pub window_duration_mins: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitNotificationMerge {
    Ignored,
    RefreshRequired,
    Merged(CodexRateLimitsStatus),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsUpdatedParams {
    #[serde(default)]
    partial: bool,
    #[serde(default)]
    rate_limits: Option<RateLimitSnapshot>,
    #[serde(default)]
    rate_limits_by_limit_id: Option<BTreeMap<String, RateLimitSnapshot>>,
}

impl CodexRateLimitsStatus {
    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CodexRateLimitsState::Unavailable,
            captured_at_ms,
            buckets: Vec::new(),
            message: message.into(),
        }
    }
}

pub fn read_codex_rate_limits_status(timeout: Duration) -> CodexRateLimitsStatus {
    let captured_at_ms = unix_now_ms();

    match read_rate_limits_from_codex(timeout, captured_at_ms) {
        Ok(status) => status,
        Err(error) => CodexRateLimitsStatus::unavailable(
            safe_rate_limits_error_message(&error),
            captured_at_ms,
        ),
    }
}

fn read_rate_limits_from_codex(
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexRateLimitsStatus, JsonlError> {
    let mut session = AppServerSession::start(AppServerCommand::codex_from_environment())
        .map_err(JsonlError::from)?;
    let mut connection =
        AppServerConnection::from_session(&mut session).map_err(JsonlError::from)?;

    read_rate_limits_from_connection(&mut connection, timeout, captured_at_ms)
}

pub fn read_rate_limits_from_connection<W>(
    connection: &mut AppServerConnection<W>,
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexRateLimitsStatus, JsonlError>
where
    W: Write,
{
    perform_initialize_handshake_with_timeout(connection, timeout)?;
    read_rate_limits_from_initialized_connection(connection, timeout, captured_at_ms)
}

pub(crate) fn read_rate_limits_from_initialized_connection<W>(
    connection: &mut AppServerConnection<W>,
    timeout: Duration,
    captured_at_ms: u64,
) -> Result<CodexRateLimitsStatus, JsonlError>
where
    W: Write,
{
    let response = connection.request(
        method::ACCOUNT_RATE_LIMITS_READ,
        Option::<()>::None,
        timeout,
    )?;

    Ok(rate_limits_status_from_response(response, captured_at_ms))
}

pub fn merge_rate_limits_notification(
    current: Option<&CodexRateLimitsStatus>,
    notification: &AppServerNotification,
    captured_at_ms: u64,
) -> RateLimitNotificationMerge {
    if notification.method != "account/rateLimits/updated" {
        return RateLimitNotificationMerge::Ignored;
    }

    let Some(params) = notification.params.as_ref() else {
        return RateLimitNotificationMerge::RefreshRequired;
    };

    let Ok(update) = serde_json::from_value::<RateLimitsUpdatedParams>(params.clone()) else {
        return RateLimitNotificationMerge::RefreshRequired;
    };

    if update.rate_limits.is_none() && update.rate_limits_by_limit_id.is_none() {
        return RateLimitNotificationMerge::RefreshRequired;
    }

    let Some(current) = current.filter(|status| status.state == CodexRateLimitsState::Available)
    else {
        return RateLimitNotificationMerge::RefreshRequired;
    };

    let mut merged = current.clone();
    merged.captured_at_ms = captured_at_ms;
    merged.message = if update.partial {
        "已合并真实限额稀疏更新".to_string()
    } else {
        "已合并真实限额更新".to_string()
    };

    if let Some(snapshot) = update.rate_limits {
        merge_bucket(
            &mut merged.buckets,
            CodexRateLimitBucketSource::Default,
            None,
            snapshot,
        );
    }

    if let Some(by_limit_id) = update.rate_limits_by_limit_id {
        for (key, snapshot) in by_limit_id {
            merge_bucket(
                &mut merged.buckets,
                CodexRateLimitBucketSource::ByLimitId,
                Some(key),
                snapshot,
            );
        }
    }

    RateLimitNotificationMerge::Merged(merged)
}

fn rate_limits_status_from_response(
    response: RateLimitsReadResponse,
    captured_at_ms: u64,
) -> CodexRateLimitsStatus {
    let mut buckets = vec![bucket_from_snapshot(
        CodexRateLimitBucketSource::Default,
        None,
        response.rate_limits,
    )];

    if let Some(by_limit_id) = response.rate_limits_by_limit_id {
        buckets.extend(by_limit_id.into_iter().map(|(key, snapshot)| {
            bucket_from_snapshot(CodexRateLimitBucketSource::ByLimitId, Some(key), snapshot)
        }));
    }

    let bucket_count = buckets.len();

    CodexRateLimitsStatus {
        state: CodexRateLimitsState::Available,
        captured_at_ms,
        buckets,
        message: format!("已读取 {bucket_count} 个真实限额桶"),
    }
}

fn merge_bucket(
    buckets: &mut Vec<CodexRateLimitBucket>,
    source: CodexRateLimitBucketSource,
    key: Option<String>,
    snapshot: RateLimitSnapshot,
) {
    let existing = buckets
        .iter_mut()
        .find(|bucket| bucket.source == source && bucket.key == key);

    match existing {
        Some(bucket) => merge_snapshot_into_bucket(bucket, snapshot),
        None => buckets.push(bucket_from_snapshot(source, key, snapshot)),
    }
}

fn merge_snapshot_into_bucket(bucket: &mut CodexRateLimitBucket, snapshot: RateLimitSnapshot) {
    if snapshot.limit_id.is_some() {
        bucket.limit_id = snapshot.limit_id;
    }
    if snapshot.limit_name.is_some() {
        bucket.limit_name = snapshot.limit_name;
    }
    if snapshot.plan_type.is_some() {
        bucket.plan_type = snapshot
            .plan_type
            .map(|plan_type| plan_type.as_str().to_string());
    }
    if snapshot.primary.is_some() {
        bucket.primary = merge_window(bucket.primary.take(), snapshot.primary);
    }
    if snapshot.secondary.is_some() {
        bucket.secondary = merge_window(bucket.secondary.take(), snapshot.secondary);
    }
    if snapshot.credits.is_some() {
        bucket.credits = snapshot.credits.map(credits_from_snapshot);
    }
}

fn merge_window(
    existing: Option<CodexRateLimitWindow>,
    update: Option<RateLimitWindow>,
) -> Option<CodexRateLimitWindow> {
    let update = update?;
    let existing = existing.unwrap_or(CodexRateLimitWindow {
        used_percent: update.used_percent,
        resets_at: None,
        window_duration_mins: None,
    });

    Some(CodexRateLimitWindow {
        used_percent: update.used_percent,
        resets_at: update.resets_at.or(existing.resets_at),
        window_duration_mins: update
            .window_duration_mins
            .or(existing.window_duration_mins),
    })
}

fn bucket_from_snapshot(
    source: CodexRateLimitBucketSource,
    key: Option<String>,
    snapshot: RateLimitSnapshot,
) -> CodexRateLimitBucket {
    CodexRateLimitBucket {
        source,
        key,
        limit_id: snapshot.limit_id,
        limit_name: snapshot.limit_name,
        plan_type: snapshot
            .plan_type
            .map(|plan_type| plan_type.as_str().to_string()),
        primary: snapshot.primary.map(window_from_snapshot),
        secondary: snapshot.secondary.map(window_from_snapshot),
        credits: snapshot.credits.map(credits_from_snapshot),
    }
}

fn window_from_snapshot(window: RateLimitWindow) -> CodexRateLimitWindow {
    CodexRateLimitWindow {
        used_percent: window.used_percent,
        resets_at: window.resets_at,
        window_duration_mins: window.window_duration_mins,
    }
}

fn credits_from_snapshot(credits: CreditsSnapshot) -> CodexCreditsSnapshot {
    CodexCreditsSnapshot {
        has_credits: credits.has_credits,
        unlimited: credits.unlimited,
        balance: credits.balance,
    }
}

pub(crate) fn safe_rate_limits_error_message(error: &JsonlError) -> String {
    match error {
        JsonlError::Timeout { .. } => "读取 Codex 限额超时".to_string(),
        JsonlError::EndOfStream => "Codex app-server 已关闭连接".to_string(),
        JsonlError::Server(_) => "Codex app-server 拒绝限额读取请求".to_string(),
        JsonlError::Json(_) => "Codex app-server 返回了无法解析的限额数据".to_string(),
        JsonlError::Io(error) if error.kind() == io::ErrorKind::NotFound => {
            "未检测到 Codex CLI，无法读取限额".to_string()
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
    fn rate_limits_status_preserves_default_and_keyed_buckets() {
        let response: ServerResponse<RateLimitsReadResponse> = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server_protocol/0.144.5/responses/rate-limits-multiple.json"
        ))
        .expect("rate limits response");

        let status = rate_limits_status_from_response(response.result, 123);

        assert_eq!(status.state, CodexRateLimitsState::Available);
        assert_eq!(status.captured_at_ms, 123);
        assert_eq!(status.buckets.len(), 3);
        assert_eq!(
            status.buckets[0].source,
            CodexRateLimitBucketSource::Default
        );
        assert_eq!(status.buckets[0].limit_id.as_deref(), Some("codex"));
        assert_eq!(
            status.buckets[0]
                .secondary
                .as_ref()
                .map(|window| window.used_percent),
            Some(59)
        );
        assert!(status.buckets.iter().any(|bucket| {
            bucket.source == CodexRateLimitBucketSource::ByLimitId
                && bucket.key.as_deref() == Some("reviews")
        }));
    }

    #[test]
    fn rate_limits_status_accepts_sparse_bucket() {
        let response: ServerResponse<RateLimitsReadResponse> = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server_protocol/0.144.5/responses/rate-limits-sparse.json"
        ))
        .expect("sparse rate limits response");

        let status = rate_limits_status_from_response(response.result, 456);

        assert_eq!(status.buckets.len(), 1);
        assert_eq!(status.buckets[0].limit_id.as_deref(), Some("codex"));
        assert_eq!(status.buckets[0].primary, None);
        assert_eq!(status.buckets[0].credits, None);
    }

    #[test]
    fn rate_limits_connection_performs_handshake_then_rate_limit_request() {
        let stdout = concat!(
            "{\"id\":1,\"result\":{\"userAgent\":\"fake-codex\"}}\n",
            "{\"id\":2,\"result\":{\"rateLimits\":{\"limitId\":\"codex\",\"primary\":{\"usedPercent\":27,\"windowDurationMins\":300,\"resetsAt\":1784548800}}}}\n"
        )
        .as_bytes();
        let stdin = Vec::new();
        let stderr = io::empty();
        let mut connection = AppServerConnection::new(stdout, stdin, stderr);

        let status = read_rate_limits_from_connection(&mut connection, Duration::from_secs(1), 789)
            .expect("rate limits");
        let written = String::from_utf8(connection.into_writer()).expect("written JSONL");

        assert_eq!(status.state, CodexRateLimitsState::Available);
        assert_eq!(status.buckets.len(), 1);
        assert_eq!(
            status.buckets[0]
                .primary
                .as_ref()
                .map(|window| window.used_percent),
            Some(27)
        );
        assert!(written.contains("\"method\":\"initialize\""));
        assert!(written.contains("\"method\":\"initialized\""));
        assert!(written.contains("\"method\":\"account/rateLimits/read\""));
    }

    #[test]
    fn sparse_rate_limit_notification_merges_with_existing_snapshot() {
        let response: ServerResponse<RateLimitsReadResponse> = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server_protocol/0.144.5/responses/rate-limits-multiple.json"
        ))
        .expect("rate limits response");
        let current = rate_limits_status_from_response(response.result, 123);
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "account/rateLimits/updated",
            "params": {
                "partial": true,
                "rateLimits": {
                    "primary": {
                        "usedPercent": 44
                    }
                }
            }
        }))
        .expect("notification");

        let merged = merge_rate_limits_notification(Some(&current), &notification, 999);

        let RateLimitNotificationMerge::Merged(status) = merged else {
            panic!("expected merged sparse notification");
        };
        let default_bucket = &status.buckets[0];
        assert_eq!(status.captured_at_ms, 999);
        assert_eq!(
            default_bucket
                .primary
                .as_ref()
                .map(|window| window.used_percent),
            Some(44)
        );
        assert_eq!(
            default_bucket
                .primary
                .as_ref()
                .and_then(|window| window.resets_at),
            Some(1_784_548_800)
        );
        assert_eq!(
            default_bucket
                .secondary
                .as_ref()
                .map(|window| window.used_percent),
            Some(59)
        );
    }

    #[test]
    fn keyed_rate_limit_notification_updates_the_matching_bucket() {
        let response: ServerResponse<RateLimitsReadResponse> = serde_json::from_str(include_str!(
            "../tests/fixtures/app_server_protocol/0.144.5/responses/rate-limits-multiple.json"
        ))
        .expect("rate limits response");
        let current = rate_limits_status_from_response(response.result, 123);
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "account/rateLimits/updated",
            "params": {
                "partial": true,
                "rateLimitsByLimitId": {
                    "reviews": {
                        "primary": {
                            "usedPercent": 15,
                            "windowDurationMins": 1440
                        }
                    }
                }
            }
        }))
        .expect("notification");

        let merged = merge_rate_limits_notification(Some(&current), &notification, 999);

        let RateLimitNotificationMerge::Merged(status) = merged else {
            panic!("expected merged keyed notification");
        };
        let reviews = status
            .buckets
            .iter()
            .find(|bucket| bucket.key.as_deref() == Some("reviews"))
            .expect("reviews bucket");
        assert_eq!(
            reviews.primary.as_ref().map(|window| window.used_percent),
            Some(15)
        );
        assert_eq!(
            reviews
                .primary
                .as_ref()
                .and_then(|window| window.window_duration_mins),
            Some(1440)
        );
    }

    #[test]
    fn empty_rate_limit_notification_requires_full_refresh() {
        let notification: AppServerNotification = serde_json::from_value(serde_json::json!({
            "method": "account/rateLimits/updated",
            "params": {
                "partial": true
            }
        }))
        .expect("notification");

        assert_eq!(
            merge_rate_limits_notification(None, &notification, 999),
            RateLimitNotificationMerge::RefreshRequired
        );
    }
}
