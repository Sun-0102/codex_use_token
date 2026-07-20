use codex_reserve_lib::{
    app_server_jsonl::{AppServerJsonl, JsonlError},
    app_server_protocol::{InitializeResponse, RequestId, method},
};
use serde_json::json;
use std::io::Cursor;

#[test]
fn sends_jsonl_request_with_incrementing_integer_ids() {
    let reader = Cursor::new(Vec::<u8>::new());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let first_id = jsonl
        .send_request(method::ACCOUNT_RATE_LIMITS_READ, Option::<()>::None)
        .expect("send first request");
    let second_id = jsonl
        .send_request(method::ACCOUNT_USAGE_READ, Option::<()>::None)
        .expect("send second request");

    assert_eq!(first_id, RequestId::Integer(1));
    assert_eq!(second_id, RequestId::Integer(2));
    assert_eq!(
        String::from_utf8(jsonl.into_writer()).expect("utf8 jsonl"),
        "{\"id\":1,\"method\":\"account/rateLimits/read\"}\n{\"id\":2,\"method\":\"account/usage/read\"}\n"
    );
}

#[test]
fn reads_until_the_response_matching_the_request_id() {
    let input = concat!(
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"partial\":true}}\n",
        "{\"id\":99,\"result\":{\"ignored\":true}}\n",
        "{\"id\":1,\"result\":{\"userAgent\":\"codex-cli/0.144.5\",\"platformOs\":\"macos\"}}\n"
    );
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let response: InitializeResponse = jsonl
        .read_response(RequestId::Integer(1))
        .expect("matching response");

    assert_eq!(response.user_agent.as_deref(), Some("codex-cli/0.144.5"));
    assert_eq!(response.platform_os.as_deref(), Some("macos"));
}

#[test]
fn buffers_out_of_order_responses_for_later_correlation() {
    let input = concat!(
        "{\"id\":1,\"result\":{\"userAgent\":\"first\"}}\n",
        "{\"id\":2,\"result\":{\"userAgent\":\"second\"}}\n"
    );
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let second: InitializeResponse = jsonl
        .read_response(RequestId::Integer(2))
        .expect("second response");
    let first: InitializeResponse = jsonl
        .read_response(RequestId::Integer(1))
        .expect("buffered first response");

    assert_eq!(second.user_agent.as_deref(), Some("second"));
    assert_eq!(first.user_agent.as_deref(), Some("first"));
}

#[test]
fn sends_request_and_correlates_typed_response() {
    let input = "{\"id\":1,\"result\":{\"userAgent\":\"codex-cli/0.144.5\"}}\n";
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let response: InitializeResponse = jsonl
        .request(
            method::INITIALIZE,
            Some(json!({"clientInfo":{"name":"codex-reserve","version":"0.1.0"}})),
        )
        .expect("typed response");

    assert_eq!(response.user_agent.as_deref(), Some("codex-cli/0.144.5"));
    assert_eq!(
        String::from_utf8(jsonl.into_writer()).expect("utf8 jsonl"),
        "{\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"codex-reserve\",\"version\":\"0.1.0\"}}}\n"
    );
}

#[test]
fn correlates_server_error_responses_by_request_id() {
    let input = concat!(
        "{\"id\":2,\"error\":{\"code\":-32601,\"message\":\"unknown method\"}}\n",
        "{\"id\":1,\"result\":{\"userAgent\":\"codex-cli/0.144.5\"}}\n"
    );
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let success: InitializeResponse = jsonl
        .read_response(RequestId::Integer(1))
        .expect("matching success");
    let error = jsonl
        .read_response::<InitializeResponse>(RequestId::Integer(2))
        .expect_err("matching server error");

    assert_eq!(success.user_agent.as_deref(), Some("codex-cli/0.144.5"));
    assert!(matches!(
        error,
        JsonlError::Server(server_error)
            if server_error.id == RequestId::Integer(2)
                && server_error.code == -32601
                && server_error.message == "unknown method"
    ));
}
