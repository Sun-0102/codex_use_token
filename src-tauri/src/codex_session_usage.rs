use std::{
    collections::HashSet,
    env, fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::{DateTime, Datelike, Days, Local, NaiveDate, TimeZone};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionUsageStatus {
    pub state: CodexSessionUsageState,
    pub captured_at_ms: u64,
    pub today: Option<CodexSessionDailyUsage>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodexSessionUsageState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionDailyUsage {
    pub request_count: u64,
    pub input_tokens: u64,
    pub fresh_input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Default)]
struct CumulativeTokens {
    input: u64,
    cached_input: u64,
    output: u64,
}

#[derive(Debug, Clone, Default)]
struct DeltaTokens {
    input: u64,
    cached_input: u64,
    output: u64,
}

impl DeltaTokens {
    fn is_zero(&self) -> bool {
        self.input == 0 && self.cached_input == 0 && self.output == 0
    }
}

impl CodexSessionUsageStatus {
    fn unavailable(message: impl Into<String>, captured_at_ms: u64) -> Self {
        Self {
            state: CodexSessionUsageState::Unavailable,
            captured_at_ms,
            today: None,
            message: message.into(),
        }
    }
}

pub fn read_codex_session_usage_status() -> CodexSessionUsageStatus {
    let now = Local::now();
    let captured_at_ms = now.timestamp_millis().max(0) as u64;
    let Some(codex_dir) = default_codex_directory() else {
        return CodexSessionUsageStatus::unavailable(
            "未找到 HOME，无法读取 Codex 本地会话统计",
            captured_at_ms,
        );
    };
    let Some((day_start, day_end)) = local_day_bounds(now) else {
        return CodexSessionUsageStatus::unavailable("无法确定本地日期范围", captured_at_ms);
    };

    match summarize_codex_session_usage_from_dir(&codex_dir, day_start, day_end) {
        Ok(today) => CodexSessionUsageStatus {
            state: CodexSessionUsageState::Available,
            captured_at_ms,
            message: format!(
                "已从 Codex 本地会话日志统计今日用量：{} 个请求",
                today.request_count
            ),
            today: Some(today),
        },
        Err(error) => CodexSessionUsageStatus::unavailable(error.to_string(), captured_at_ms),
    }
}

fn default_codex_directory() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
}

fn local_day_bounds(now: DateTime<Local>) -> Option<(i64, i64)> {
    let start = Local
        .from_local_datetime(&now.date_naive().and_hms_opt(0, 0, 0)?)
        .earliest()?;
    let next_date = now.date_naive().checked_add_days(Days::new(1))?;
    let end = Local
        .from_local_datetime(&next_date.and_hms_opt(0, 0, 0)?)
        .latest()?;
    Some((start.timestamp(), end.timestamp()))
}

fn summarize_codex_session_usage_from_dir(
    codex_dir: &Path,
    day_start: i64,
    day_end: i64,
) -> Result<CodexSessionDailyUsage, CodexSessionUsageError> {
    if !codex_dir.is_dir() {
        return Err(CodexSessionUsageError::Unavailable(
            "未找到 Codex 本地会话目录".to_string(),
        ));
    }

    let files = collect_candidate_session_files(codex_dir, day_start);
    let mut today = CodexSessionDailyUsage::default();
    let mut readable_files = 0usize;
    let mut last_error = None;

    for file_path in &files {
        match summarize_session_file(file_path, day_start, day_end, &mut today) {
            Ok(()) => readable_files += 1,
            Err(error) => last_error = Some(error),
        }
    }
    if !files.is_empty()
        && readable_files == 0
        && let Some(error) = last_error
    {
        return Err(error);
    }

    today.fresh_input_tokens = today.input_tokens.saturating_sub(today.cache_read_tokens);
    today.total_tokens = today.input_tokens.saturating_add(today.output_tokens);
    Ok(today)
}

fn collect_candidate_session_files(codex_dir: &Path, day_start: i64) -> Vec<PathBuf> {
    let Some(today) = Local.timestamp_opt(day_start, 0).earliest() else {
        return Vec::new();
    };
    let today = today.date_naive();
    let yesterday = today.checked_sub_days(Days::new(1));
    let mut files = Vec::new();

    collect_partition_files(codex_dir, today, &mut files);
    if let Some(yesterday) = yesterday {
        collect_partition_files(codex_dir, yesterday, &mut files);
    }

    collect_recent_session_files(&codex_dir.join("sessions"), &mut files, 0, 3, day_start);

    let archived_dir = codex_dir.join("archived_sessions");
    if let Ok(entries) = fs::read_dir(archived_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_jsonl_file(&path) && was_modified_since(&path, day_start) {
                files.push(path);
            }
        }
    }

    files.sort();
    let mut seen_file_names = HashSet::new();
    files.retain(|path| {
        path.file_name()
            .is_some_and(|name| seen_file_names.insert(name.to_os_string()))
    });
    files
}

fn collect_recent_session_files(
    directory: &Path,
    files: &mut Vec<PathBuf>,
    depth: u32,
    max_depth: u32,
    day_start: i64,
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && depth < max_depth {
            collect_recent_session_files(&path, files, depth + 1, max_depth, day_start);
        } else if is_jsonl_file(&path) && was_modified_since(&path, day_start) {
            files.push(path);
        }
    }
}

fn collect_partition_files(codex_dir: &Path, date: NaiveDate, files: &mut Vec<PathBuf>) {
    let partition = codex_dir
        .join("sessions")
        .join(format!("{:04}", date.year()))
        .join(format!("{:02}", date.month()))
        .join(format!("{:02}", date.day()));
    if let Ok(entries) = fs::read_dir(partition) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_jsonl_file(&path) {
                files.push(path);
            }
        }
    }
}

fn is_jsonl_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
}

fn was_modified_since(path: &Path, day_start: i64) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .is_some_and(|modified| modified.as_secs() >= day_start.max(0) as u64)
}

fn summarize_session_file(
    path: &Path,
    day_start: i64,
    day_end: i64,
    today: &mut CodexSessionDailyUsage,
) -> Result<(), CodexSessionUsageError> {
    let file = fs::File::open(path).map_err(CodexSessionUsageError::Io)?;
    let reader = BufReader::new(file);
    let mut previous_total: Option<CumulativeTokens> = None;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        if !line.contains("\"event_msg\"") || !line.contains("\"token_count\"") {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("event_msg") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(|value| value.as_str()) != Some("token_count") {
            continue;
        }
        let Some(info) = payload.get("info").filter(|value| !value.is_null()) else {
            continue;
        };

        let delta = if let Some(total) = info
            .get("total_token_usage")
            .and_then(parse_cumulative_tokens)
        {
            let delta = compute_delta(previous_total.as_ref(), &total);
            previous_total = Some(total);
            delta
        } else if let Some(last) = info
            .get("last_token_usage")
            .and_then(parse_cumulative_tokens)
        {
            DeltaTokens {
                input: last.input,
                cached_input: last.cached_input,
                output: last.output,
            }
        } else {
            continue;
        };

        let delta = DeltaTokens {
            cached_input: delta.cached_input.min(delta.input),
            ..delta
        };
        if delta.is_zero() {
            continue;
        }

        let Some(timestamp) = value
            .get("timestamp")
            .and_then(|value| value.as_str())
            .and_then(parse_rfc3339_timestamp)
        else {
            continue;
        };
        if timestamp < day_start || timestamp >= day_end {
            continue;
        }

        today.request_count = today.request_count.saturating_add(1);
        today.input_tokens = today.input_tokens.saturating_add(delta.input);
        today.output_tokens = today.output_tokens.saturating_add(delta.output);
        today.cache_read_tokens = today.cache_read_tokens.saturating_add(delta.cached_input);
    }

    Ok(())
}

fn parse_cumulative_tokens(value: &serde_json::Value) -> Option<CumulativeTokens> {
    value.as_object()?;
    Some(CumulativeTokens {
        input: value.get("input_tokens").and_then(|value| value.as_u64())?,
        cached_input: value
            .get("cached_input_tokens")
            .or_else(|| value.get("cache_read_input_tokens"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        output: value
            .get("output_tokens")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
    })
}

fn compute_delta(previous: Option<&CumulativeTokens>, current: &CumulativeTokens) -> DeltaTokens {
    match previous {
        None => DeltaTokens {
            input: current.input,
            cached_input: current.cached_input,
            output: current.output,
        },
        Some(previous) => DeltaTokens {
            input: current.input.saturating_sub(previous.input),
            cached_input: current.cached_input.saturating_sub(previous.cached_input),
            output: current.output.saturating_sub(previous.output),
        },
    }
}

fn parse_rfc3339_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp())
}

#[derive(Debug)]
enum CodexSessionUsageError {
    Io(io::Error),
    Unavailable(String),
}

impl std::fmt::Display for CodexSessionUsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "无法读取 Codex 本地会话日志：{error}"),
            Self::Unavailable(message) => formatter.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_today_from_codex_session_token_events() {
        let root = unique_test_dir("today-summary");
        let date = NaiveDate::from_ymd_opt(2026, 7, 24).expect("date");
        let (day_start, day_end) = test_day_bounds(date);
        let sessions = session_partition(&root, date);
        fs::create_dir_all(&sessions).expect("create sessions directory");
        fs::write(
            sessions.join("rollout-test.jsonl"),
            format!(
                "{{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":100,\"cached_input_tokens\":60,\"output_tokens\":20}}}}}}}}\n\
                 {{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":250,\"cached_input_tokens\":160,\"output_tokens\":50}}}}}}}}\n",
                test_timestamp(day_start + 600),
                test_timestamp(day_start + 1_200),
            ),
        )
        .expect("write session");

        let usage =
            summarize_codex_session_usage_from_dir(&root, day_start, day_end).expect("usage");

        assert_eq!(
            usage,
            CodexSessionDailyUsage {
                request_count: 2,
                input_tokens: 250,
                fresh_input_tokens: 90,
                output_tokens: 50,
                cache_read_tokens: 160,
                cache_creation_tokens: 0,
                total_tokens: 300,
            }
        );

        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn uses_previous_day_totals_as_the_baseline_without_counting_them_today() {
        let root = unique_test_dir("previous-baseline");
        let date = NaiveDate::from_ymd_opt(2026, 7, 24).expect("date");
        let previous_date = date.checked_sub_days(Days::new(1)).expect("previous date");
        let (day_start, day_end) = test_day_bounds(date);
        let sessions = session_partition(&root, previous_date);
        fs::create_dir_all(&sessions).expect("create sessions directory");
        fs::write(
            sessions.join("rollout-overnight.jsonl"),
            format!(
                "{{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":100,\"cached_input_tokens\":80,\"output_tokens\":20}}}}}}}}\n\
                 {{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"total_token_usage\":{{\"input_tokens\":160,\"cached_input_tokens\":120,\"output_tokens\":30}}}}}}}}\n",
                test_timestamp(day_start - 600),
                test_timestamp(day_start + 600),
            ),
        )
        .expect("write session");

        let usage =
            summarize_codex_session_usage_from_dir(&root, day_start, day_end).expect("usage");

        assert_eq!(usage.request_count, 1);
        assert_eq!(usage.input_tokens, 60);
        assert_eq!(usage.cache_read_tokens, 40);
        assert_eq!(usage.fresh_input_tokens, 20);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.total_tokens, 70);

        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn ignores_message_content_and_malformed_lines() {
        let root = unique_test_dir("safe-filtering");
        let date = NaiveDate::from_ymd_opt(2026, 7, 24).expect("date");
        let (day_start, day_end) = test_day_bounds(date);
        let sessions = session_partition(&root, date);
        fs::create_dir_all(&sessions).expect("create sessions directory");
        fs::write(
            sessions.join("rollout-filtered.jsonl"),
            format!(
                "{{\"timestamp\":\"{}\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"content\":\"token_count credentials should not be parsed\"}}}}\n\
                 not-json token_count event_msg\n\
                 {{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":30,\"cached_input_tokens\":20,\"output_tokens\":5}}}}}}}}\n",
                test_timestamp(day_start + 60),
                test_timestamp(day_start + 120),
            ),
        )
        .expect("write session");

        let usage =
            summarize_codex_session_usage_from_dir(&root, day_start, day_end).expect("usage");

        assert_eq!(usage.request_count, 1);
        assert_eq!(usage.total_tokens, 35);

        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn counts_matching_active_and_archived_rollouts_only_once() {
        let root = unique_test_dir("active-archive-dedup");
        let date = Local::now().date_naive();
        let (day_start, day_end) = test_day_bounds(date);
        let sessions = session_partition(&root, date);
        let archived = root.join("archived_sessions");
        fs::create_dir_all(&sessions).expect("create sessions directory");
        fs::create_dir_all(&archived).expect("create archive directory");
        let event = format!(
            "{{\"timestamp\":\"{}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":30,\"cached_input_tokens\":20,\"output_tokens\":5}}}}}}}}\n",
            test_timestamp(day_start + 120),
        );
        fs::write(sessions.join("rollout-same.jsonl"), &event).expect("write active session");
        fs::write(archived.join("rollout-same.jsonl"), event).expect("write archived session");

        let usage =
            summarize_codex_session_usage_from_dir(&root, day_start, day_end).expect("usage");

        assert_eq!(usage.request_count, 1);
        assert_eq!(usage.total_tokens, 35);

        fs::remove_dir_all(root).expect("remove test directory");
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "codex-reserve-session-usage-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn test_day_bounds(date: NaiveDate) -> (i64, i64) {
        let midday = Local
            .from_local_datetime(&date.and_hms_opt(12, 0, 0).expect("midday"))
            .earliest()
            .expect("local midday");
        local_day_bounds(midday).expect("day bounds")
    }

    fn test_timestamp(timestamp: i64) -> String {
        DateTime::from_timestamp(timestamp, 0)
            .expect("timestamp")
            .to_rfc3339()
    }

    fn session_partition(root: &Path, date: NaiveDate) -> PathBuf {
        root.join("sessions")
            .join(format!("{:04}", date.year()))
            .join(format!("{:02}", date.month()))
            .join(format!("{:02}", date.day()))
    }
}
