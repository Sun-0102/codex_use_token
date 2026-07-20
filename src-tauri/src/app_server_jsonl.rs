use std::{
    collections::VecDeque,
    fmt,
    io::{self, BufRead, BufReader, Write},
    time::Duration,
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::app_server_protocol::{ClientNotification, ClientRequest, RequestId};

#[derive(Debug)]
pub struct AppServerJsonl<R, W> {
    reader: BufReader<R>,
    writer: W,
    next_request_id: i64,
    buffered_responses: VecDeque<AppServerResponse>,
}

impl<R, W> AppServerJsonl<R, W>
where
    R: io::Read,
    W: Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
            next_request_id: 1,
            buffered_responses: VecDeque::new(),
        }
    }

    pub fn send_request<P>(
        &mut self,
        method: &str,
        params: Option<P>,
    ) -> Result<RequestId, JsonlError>
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

    pub fn request<P, T>(&mut self, method: &str, params: Option<P>) -> Result<T, JsonlError>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let id = self.send_request(method, params)?;
        self.read_response(id)
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

    pub fn read_response<T>(&mut self, expected_id: RequestId) -> Result<T, JsonlError>
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

        loop {
            match self.read_next_message()? {
                AppServerMessage::Response(response) if response.id == expected_id => {
                    return deserialize_response_result(response);
                }
                AppServerMessage::Response(response) => {
                    self.buffered_responses.push_back(response);
                }
                AppServerMessage::Notification(_) => {}
            }
        }
    }

    pub fn into_writer(self) -> W {
        self.writer
    }

    pub fn read_next_message(&mut self) -> Result<AppServerMessage, JsonlError> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Err(JsonlError::EndOfStream);
        }

        let value: Value = serde_json::from_str(line.trim_end())?;

        if value.get("id").is_some() {
            let response: AppServerResponse = serde_json::from_value(value)?;
            return Ok(AppServerMessage::Response(response));
        }

        let notification: AppServerNotification = serde_json::from_value(value)?;
        Ok(AppServerMessage::Notification(notification))
    }
}

pub fn deserialize_response_result<T>(response: AppServerResponse) -> Result<T, JsonlError>
where
    T: DeserializeOwned,
{
    match response.outcome {
        AppServerResponseOutcome::Result { result } => {
            serde_json::from_value(result).map_err(JsonlError::from)
        }
        AppServerResponseOutcome::Error { error } => Err(JsonlError::Server(ServerError {
            id: response.id,
            code: error.code,
            message: error.message,
        })),
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AppServerResponse {
    pub id: RequestId,
    #[serde(flatten)]
    outcome: AppServerResponseOutcome,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(untagged)]
enum AppServerResponseOutcome {
    Result { result: Value },
    Error { error: RawServerError },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct RawServerError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AppServerNotification {
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppServerMessage {
    Response(AppServerResponse),
    Notification(AppServerNotification),
}

#[derive(Debug)]
pub enum JsonlError {
    Io(io::Error),
    Json(serde_json::Error),
    Server(ServerError),
    Timeout { id: RequestId, timeout: Duration },
    EndOfStream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    pub id: RequestId,
    pub code: i64,
    pub message: String,
}

impl fmt::Display for JsonlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "app-server I/O error: {error}"),
            Self::Json(error) => write!(formatter, "app-server JSON error: {error}"),
            Self::Server(error) => write!(
                formatter,
                "app-server request {:?} failed with code {}: {}",
                error.id, error.code, error.message
            ),
            Self::Timeout { id, timeout } => {
                write!(
                    formatter,
                    "app-server request {id:?} timed out after {timeout:?}"
                )
            }
            Self::EndOfStream => write!(formatter, "app-server closed stdout"),
        }
    }
}

impl std::error::Error for JsonlError {}

impl From<io::Error> for JsonlError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for JsonlError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
