use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchUsageStatus {
    pub state: CcSwitchUsageState,
    pub captured_at_ms: u64,
    pub today: Option<CcSwitchDailyUsage>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CcSwitchUsageState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchDailyUsage {
    pub request_count: i64,
    pub input_tokens: i64,
    pub fresh_input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
}

impl CcSwitchUsageStatus {
    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CcSwitchUsageState::Unavailable,
            captured_at_ms,
            today: None,
            message: message.into(),
        }
    }
}

pub fn read_cc_switch_usage_status() -> CcSwitchUsageStatus {
    let captured_at_ms = unix_now_ms();
    let Some(database_path) = default_cc_switch_database_path() else {
        return CcSwitchUsageStatus::unavailable(
            "未找到 HOME，无法读取 cc-switch 统计",
            captured_at_ms,
        );
    };

    read_cc_switch_usage_status_from_database(&database_path, captured_at_ms)
        .unwrap_or_else(|error| CcSwitchUsageStatus::unavailable(error.to_string(), captured_at_ms))
}

fn default_cc_switch_database_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".cc-switch").join("cc-switch.db"))
}

fn read_cc_switch_usage_status_from_database(
    database_path: &Path,
    captured_at_ms: u64,
) -> Result<CcSwitchUsageStatus, CcSwitchUsageError> {
    if !database_path.exists() {
        return Err(CcSwitchUsageError::Unavailable(
            "未找到 cc-switch 统计数据库".to_string(),
        ));
    }

    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(database_path)
        .arg(today_usage_sql())
        .output()
        .map_err(CcSwitchUsageError::Io)?;

    if !output.status.success() {
        return Err(CcSwitchUsageError::Unavailable(
            "cc-switch 统计数据库查询失败".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let today = parse_today_usage_from_sqlite_json(&stdout)?;

    Ok(CcSwitchUsageStatus {
        state: CcSwitchUsageState::Available,
        captured_at_ms,
        today: Some(today.clone()),
        message: format!(
            "已读取 cc-switch 今日 Codex 统计：{} 个请求",
            today.request_count
        ),
    })
}

fn today_usage_sql() -> &'static str {
    r#"
SELECT
  COUNT(*) AS requestCount,
  COALESCE(SUM(input_tokens), 0) AS inputTokens,
  COALESCE(SUM(input_tokens - cache_read_tokens), 0) AS freshInputTokens,
  COALESCE(SUM(output_tokens), 0) AS outputTokens,
  COALESCE(SUM(cache_read_tokens), 0) AS cacheReadTokens,
  COALESCE(SUM(cache_creation_tokens), 0) AS cacheCreationTokens,
  COALESCE(SUM(input_tokens + output_tokens + cache_creation_tokens), 0) AS totalTokens,
  COALESCE(SUM(CAST(total_cost_usd AS REAL)), 0) AS totalCostUsd
FROM proxy_request_logs
WHERE app_type = 'codex'
  AND date(created_at, 'unixepoch', 'localtime') = date('now', 'localtime');
"#
}

fn parse_today_usage_from_sqlite_json(
    output: &str,
) -> Result<CcSwitchDailyUsage, CcSwitchUsageError> {
    let rows: Vec<CcSwitchDailyUsage> =
        serde_json::from_str(output).map_err(CcSwitchUsageError::Json)?;
    rows.into_iter()
        .next()
        .ok_or_else(|| CcSwitchUsageError::Unavailable("cc-switch 今日统计为空".to_string()))
}

#[derive(Debug)]
enum CcSwitchUsageError {
    Io(io::Error),
    Json(serde_json::Error),
    Unavailable(String),
}

impl std::fmt::Display for CcSwitchUsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) if error.kind() == io::ErrorKind::NotFound => {
                write!(formatter, "未检测到 sqlite3，无法读取 cc-switch 统计")
            }
            Self::Io(_) => write!(formatter, "无法读取 cc-switch 统计数据库"),
            Self::Json(error) => write!(formatter, "cc-switch 统计数据格式无法解析：{error}"),
            Self::Unavailable(message) => formatter.write_str(message),
        }
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
    fn parses_cc_switch_today_usage_without_double_counting_cache_read_tokens() {
        let usage = parse_today_usage_from_sqlite_json(
            r#"[{
              "requestCount": 1343,
              "inputTokens": 151257000,
              "freshInputTokens": 5619000,
              "outputTokens": 522617,
              "cacheReadTokens": 145638000,
              "cacheCreationTokens": 0,
              "totalTokens": 151779617,
              "totalCostUsd": 32.882
            }]"#,
        )
        .expect("usage");

        assert_eq!(usage.request_count, 1343);
        assert_eq!(usage.fresh_input_tokens, 5_619_000);
        assert_eq!(usage.cache_read_tokens, 145_638_000);
        assert_eq!(usage.total_tokens, 151_779_617);
    }

    #[test]
    fn rejects_empty_sqlite_json_result() {
        let error = parse_today_usage_from_sqlite_json("[]").expect_err("empty result");

        assert_eq!(error.to_string(), "cc-switch 今日统计为空");
    }
}
