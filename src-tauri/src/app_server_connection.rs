use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read, Write},
    process::ChildStdin,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app_server_jsonl::{
        AppServerJsonl, AppServerMessage, AppServerNotification, AppServerResponse, JsonlError,
        deserialize_response_result,
    },
    app_server_protocol::{ClientNotification, ClientRequest, RequestId},
    app_server_session::AppServerSession,
};
use serde::{Serialize, de::DeserializeOwned};

pub struct AppServerConnection<W> {
    writer: W,
    next_request_id: i64,
    response_rx: Receiver<ReaderEvent>,
    notification_rx: Receiver<AppServerNotification>,
    stderr_rx: Receiver<String>,
    buffered_responses: VecDeque<AppServerResponse>,
}

enum ReaderEvent {
    Response(AppServerResponse),
    Error(JsonlError),
}

impl<W> AppServerConnection<W>
where
    W: Write,
{
    pub fn new<R, E>(stdout: R, stdin: W, stderr: E) -> Self
    where
        R: Read + Send + 'static,
        E: Read + Send + 'static,
    {
        let (response_tx, response_rx) = mpsc::channel();
        let (notification_tx, notification_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        thread::spawn(move || {
            let mut jsonl = AppServerJsonl::new(stdout, io::sink());
            loop {
                match jsonl.read_next_message() {
                    Ok(AppServerMessage::Response(response)) => {
                        if response_tx.send(ReaderEvent::Response(response)).is_err() {
                            break;
                        }
                    }
                    Ok(AppServerMessage::Notification(notification)) => {
                        if notification_tx.send(notification).is_err() {
                            break;
                        }
                    }
                    Err(JsonlError::EndOfStream) => break,
                    Err(error) => {
                        let _ = response_tx.send(ReaderEvent::Error(error));
                        break;
                    }
                }
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                if stderr_tx.send(redact_app_server_log_line(&line)).is_err() {
                    break;
                }
            }
        });

        Self {
            writer: stdin,
            next_request_id: 1,
            response_rx,
            notification_rx,
            stderr_rx,
            buffered_responses: VecDeque::new(),
        }
    }

    pub fn request<P, T>(
        &mut self,
        method: &str,
        params: Option<P>,
        timeout: Duration,
    ) -> Result<T, JsonlError>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let id = self.send_request(method, params)?;
        self.read_response(id, timeout)
    }

    pub fn try_next_notification(&mut self) -> Option<AppServerNotification> {
        self.notification_rx.try_recv().ok()
    }

    pub fn try_next_stderr_line(&mut self) -> Option<String> {
        self.stderr_rx.try_recv().ok()
    }

    pub fn send_notification<P>(
        &mut self,
        method: &str,
        params: Option<P>,
    ) -> Result<(), JsonlError>
    where
        P: Serialize,
    {
        let notification = ClientNotification {
            method: method.to_string(),
            params,
        };

        serde_json::to_writer(&mut self.writer, &notification)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        Ok(())
    }

    pub fn into_writer(self) -> W {
        self.writer
    }

    fn send_request<P>(&mut self, method: &str, params: Option<P>) -> Result<RequestId, JsonlError>
    where
        P: Serialize,
    {
        let id = RequestId::Integer(self.next_request_id);
        self.next_request_id += 1;
        let request = ClientRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
        };

        serde_json::to_writer(&mut self.writer, &request)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        Ok(id)
    }

    fn read_response<T>(
        &mut self,
        expected_id: RequestId,
        timeout: Duration,
    ) -> Result<T, JsonlError>
    where
        T: DeserializeOwned,
    {
        if let Some(index) = self
            .buffered_responses
            .iter()
            .position(|response| response.id == expected_id)
        {
            let response = self
                .buffered_responses
                .remove(index)
                .expect("buffered response index exists");
            return deserialize_response_result(response);
        }

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(JsonlError::Timeout {
                    id: expected_id,
                    timeout,
                });
            }

            match self.response_rx.recv_timeout(deadline - now) {
                Ok(ReaderEvent::Response(response)) if response.id == expected_id => {
                    return deserialize_response_result(response);
                }
                Ok(ReaderEvent::Response(response)) => self.buffered_responses.push_back(response),
                Ok(ReaderEvent::Error(error)) => return Err(error),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(JsonlError::Timeout {
                        id: expected_id,
                        timeout,
                    });
                }
                Err(RecvTimeoutError::Disconnected) => return Err(JsonlError::EndOfStream),
            }
        }
    }
}

impl AppServerConnection<ChildStdin> {
    pub fn from_session(session: &mut AppServerSession) -> io::Result<Self> {
        let stdout = session
            .take_stdout()
            .ok_or_else(|| io::Error::other("app-server session stdout already taken"))?;
        let stdin = session
            .take_stdin()
            .ok_or_else(|| io::Error::other("app-server session stdin already taken"))?;
        let stderr = session
            .take_stderr()
            .ok_or_else(|| io::Error::other("app-server session stderr already taken"))?;

        Ok(Self::new(stdout, stdin, stderr))
    }
}

pub fn redact_app_server_log_line(line: &str) -> String {
    let mut sanitized = line.to_string();

    for prefix in [
        "Authorization: Bearer ",
        "authorization: Bearer ",
        "Bearer ",
        "access_token=",
        "accessToken=",
        "refresh_token=",
        "refreshToken=",
        "api_key: ",
        "apiKey: ",
        "\"access_token\":\"",
        "\"access_token\": \"",
        "\"accessToken\":\"",
        "\"accessToken\": \"",
        "\"refresh_token\":\"",
        "\"refresh_token\": \"",
        "\"refreshToken\":\"",
        "\"refreshToken\": \"",
        "\"api_key\":\"",
        "\"api_key\": \"",
        "\"apiKey\":\"",
        "\"apiKey\": \"",
    ] {
        redact_values_after_prefix(&mut sanitized, prefix);
    }

    truncate_log_line(&mut sanitized, 4096);

    sanitized
}

fn redact_values_after_prefix(value: &mut String, prefix: &str) {
    let mut search_from = 0;

    while let Some(relative_start) = value[search_from..].find(prefix) {
        let value_start = search_from + relative_start + prefix.len();
        let value_end = value[value_start..]
            .find(|character: char| {
                character.is_whitespace()
                    || character == ','
                    || character == ';'
                    || character == '"'
                    || character == '\''
            })
            .map_or(value.len(), |relative_end| value_start + relative_end);

        value.replace_range(value_start..value_end, "[redacted]");
        search_from = value_start + "[redacted]".len();
    }
}

fn truncate_log_line(value: &mut String, max_len: usize) {
    if value.len() <= max_len {
        return;
    }

    let mut truncate_at = max_len;
    while !value.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }
    value.truncate(truncate_at);
}
