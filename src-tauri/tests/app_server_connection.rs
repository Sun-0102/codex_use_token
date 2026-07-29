use codex_reserve_lib::{
    app_server_connection::{AppServerConnection, redact_app_server_log_line},
    app_server_protocol::{InitializeResponse, method},
};
use std::{io::Cursor, time::Duration};

#[cfg(unix)]
use codex_reserve_lib::{app_server_jsonl::JsonlError, app_server_protocol::RequestId};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::time::Instant;

#[test]
fn routes_notifications_away_from_matching_responses() {
    let stdout = concat!(
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"partial\":true}}\n",
        "{\"id\":1,\"result\":{\"userAgent\":\"codex-cli/0.144.5\"}}\n"
    );
    let stderr = "Authorization: Bearer sk-live-secret\nnormal stderr\n";
    let mut connection = AppServerConnection::new(
        Cursor::new(stdout.as_bytes().to_vec()),
        Vec::<u8>::new(),
        Cursor::new(stderr.as_bytes().to_vec()),
    );

    let response: InitializeResponse = connection
        .request(
            method::INITIALIZE,
            Option::<()>::None,
            Duration::from_secs(1),
        )
        .expect("initialize response");
    let notification = connection
        .try_next_notification()
        .expect("notification available");

    assert_eq!(response.user_agent.as_deref(), Some("codex-cli/0.144.5"));
    assert_eq!(notification.method, "account/rateLimits/updated");
    assert_eq!(notification.params.expect("params")["partial"], true);
    assert_eq!(
        String::from_utf8(connection.into_writer()).expect("request jsonl"),
        "{\"id\":1,\"method\":\"initialize\"}\n"
    );
}

#[cfg(unix)]
#[test]
fn request_returns_timeout_when_no_matching_response_arrives() {
    let (stdout_reader, stdout_writer) = UnixStream::pair().expect("stdout stream pair");
    let mut connection = AppServerConnection::new(
        stdout_reader,
        Vec::<u8>::new(),
        Cursor::new(Vec::<u8>::new()),
    );
    let started_at = Instant::now();

    let error = connection
        .request::<_, InitializeResponse>(
            method::INITIALIZE,
            Option::<()>::None,
            Duration::from_millis(30),
        )
        .expect_err("request timeout");

    drop(stdout_writer);
    assert!(started_at.elapsed() >= Duration::from_millis(30));
    assert!(matches!(
        error,
        JsonlError::Timeout {
            id: RequestId::Integer(1),
            ..
        }
    ));
}

#[test]
fn stderr_log_lines_are_redacted_before_exposure() {
    let raw = concat!(
        "Authorization: Bearer sk-live-secret ",
        "access_token=abc refreshToken=def api_key: xyz ",
        "\"refresh_token\":\"json-secret\" \"apiKey\": \"json-api-key\" normal"
    );

    let sanitized = redact_app_server_log_line(raw);

    assert!(sanitized.contains("normal"));
    assert!(!sanitized.contains("sk-live-secret"));
    assert!(!sanitized.contains("abc"));
    assert!(!sanitized.contains("def"));
    assert!(!sanitized.contains("xyz"));
    assert!(!sanitized.contains("json-secret"));
    assert!(!sanitized.contains("json-api-key"));
    assert!(sanitized.contains("Authorization: Bearer [redacted]"));
    assert!(sanitized.contains("access_token=[redacted]"));
    assert!(sanitized.contains("refreshToken=[redacted]"));
    assert!(sanitized.contains("api_key: [redacted]"));
    assert!(sanitized.contains("\"refresh_token\":\"[redacted]\""));
    assert!(sanitized.contains("\"apiKey\": \"[redacted]\""));
}

#[test]
fn stderr_log_lines_are_capped_after_redaction() {
    let raw = format!("access_token={} {}", "s".repeat(100), "x".repeat(5000));

    let sanitized = redact_app_server_log_line(&raw);

    assert_eq!(sanitized.len(), 4096);
    assert!(sanitized.contains("access_token=[redacted]"));
    assert!(!sanitized.contains("ssssssssss"));
}

#[test]
fn stderr_reader_exposes_only_redacted_lines() {
    let stderr = "refresh_token=secret-token normal\n";
    let mut connection = AppServerConnection::new(
        Cursor::new(Vec::<u8>::new()),
        Vec::<u8>::new(),
        Cursor::new(stderr.as_bytes().to_vec()),
    );

    let mut line = None;
    for _ in 0..20 {
        line = connection.try_next_stderr_line();
        if line.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let line = line.expect("stderr line");
    assert!(line.contains("normal"));
    assert!(line.contains("refresh_token=[redacted]"));
    assert!(!line.contains("secret-token"));
}
