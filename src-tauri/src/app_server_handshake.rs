use std::{
    io::{self, Write},
    time::Duration,
};

use crate::{
    app_server_connection::AppServerConnection,
    app_server_jsonl::{AppServerJsonl, JsonlError},
    app_server_protocol::{
        ClientInfo, InitializeCapabilities, InitializeParams, InitializeResponse, method,
    },
};

pub fn perform_initialize_handshake<R, W>(
    jsonl: &mut AppServerJsonl<R, W>,
) -> Result<InitializeResponse, JsonlError>
where
    R: io::Read,
    W: Write,
{
    let response = jsonl.request(method::INITIALIZE, Some(default_initialize_params()))?;
    jsonl.send_notification(method::INITIALIZED, Option::<()>::None)?;

    Ok(response)
}

pub fn perform_initialize_handshake_with_timeout<W>(
    connection: &mut AppServerConnection<W>,
    timeout: Duration,
) -> Result<InitializeResponse, JsonlError>
where
    W: Write,
{
    let response = connection.request(
        method::INITIALIZE,
        Some(default_initialize_params()),
        timeout,
    )?;
    connection.send_notification(method::INITIALIZED, Option::<()>::None)?;

    Ok(response)
}

fn default_initialize_params() -> InitializeParams {
    InitializeParams {
        client_info: ClientInfo {
            name: "codex-reserve".to_string(),
            title: Some("Codex Reserve".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities: Some(InitializeCapabilities {
            experimental_api: Some(true),
            request_attestation: Some(false),
            opt_out_notification_methods: Some(vec!["thread/started".to_string()]),
            ..Default::default()
        }),
    }
}
