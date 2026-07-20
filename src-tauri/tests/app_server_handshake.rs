use codex_reserve_lib::{
    app_server_handshake::perform_initialize_handshake,
    app_server_jsonl::{AppServerJsonl, JsonlError},
};
use std::io::Cursor;

#[test]
fn initialize_handshake_sends_initialize_then_initialized_notification() {
    let input =
        "{\"id\":1,\"result\":{\"userAgent\":\"codex-cli/0.144.5\",\"platformOs\":\"macos\"}}\n";
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let response = perform_initialize_handshake(&mut jsonl).expect("initialize handshake");

    assert_eq!(response.user_agent.as_deref(), Some("codex-cli/0.144.5"));
    assert_eq!(response.platform_os.as_deref(), Some("macos"));
    assert_eq!(
        String::from_utf8(jsonl.into_writer()).expect("utf8 jsonl"),
        concat!(
            "{\"id\":1,\"method\":\"initialize\",\"params\":",
            "{\"clientInfo\":{\"name\":\"codex-reserve\",\"title\":\"Codex Reserve\",\"version\":\"0.1.0\"},",
            "\"capabilities\":{\"experimentalApi\":true,\"requestAttestation\":false,\"optOutNotificationMethods\":[\"thread/started\"]}}}\n",
            "{\"method\":\"initialized\"}\n"
        )
    );
}

#[test]
fn initialize_handshake_does_not_send_initialized_after_server_error() {
    let input = "{\"id\":1,\"error\":{\"code\":-32000,\"message\":\"not ready\"}}\n";
    let reader = Cursor::new(input.as_bytes().to_vec());
    let writer = Vec::<u8>::new();
    let mut jsonl = AppServerJsonl::new(reader, writer);

    let error = perform_initialize_handshake(&mut jsonl).expect_err("initialize error");

    assert!(matches!(
        error,
        JsonlError::Server(server_error)
            if server_error.code == -32000 && server_error.message == "not ready"
    ));
    assert_eq!(
        String::from_utf8(jsonl.into_writer()).expect("utf8 jsonl"),
        concat!(
            "{\"id\":1,\"method\":\"initialize\",\"params\":",
            "{\"clientInfo\":{\"name\":\"codex-reserve\",\"title\":\"Codex Reserve\",\"version\":\"0.1.0\"},",
            "\"capabilities\":{\"experimentalApi\":true,\"requestAttestation\":false,\"optOutNotificationMethods\":[\"thread/started\"]}}}\n",
        )
    );
}
