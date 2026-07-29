use std::{
    process::ChildStdin,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    app_server_account::{self, CodexAccountState, CodexAccountStatus},
    app_server_connection::AppServerConnection,
    app_server_handshake::perform_initialize_handshake_with_timeout,
    app_server_jsonl::JsonlError,
    app_server_rate_limits::{self, CodexRateLimitsState, CodexRateLimitsStatus},
    app_server_session::{AppServerCommand, AppServerSession},
    app_server_thread_usage::{self, CodexThreadTokenUsageStatus},
    app_server_usage::{self, CodexUsageState, CodexUsageStatus},
};

pub struct AppServerRuntime {
    client: Arc<Mutex<PersistentAppServerClient>>,
}

impl Default for AppServerRuntime {
    fn default() -> Self {
        Self {
            client: Arc::new(Mutex::new(PersistentAppServerClient::default())),
        }
    }
}

impl AppServerRuntime {
    pub fn client(&self) -> Arc<Mutex<PersistentAppServerClient>> {
        Arc::clone(&self.client)
    }
}

impl Drop for AppServerRuntime {
    fn drop(&mut self) {
        if let Ok(mut client) = self.client.lock() {
            client.shutdown();
        }
    }
}

#[derive(Default)]
pub struct PersistentAppServerClient {
    session: Option<AppServerSession>,
    connection: Option<AppServerConnection<ChildStdin>>,
    latest_thread_token_usage: Option<CodexThreadTokenUsageStatus>,
}

impl PersistentAppServerClient {
    pub fn read_account_status(&mut self, timeout: Duration) -> CodexAccountStatus {
        let captured_at_ms = unix_now_ms();

        match self.with_connection(timeout, |connection| {
            app_server_account::read_account_status_from_initialized_connection(
                connection,
                timeout,
                captured_at_ms,
            )
        }) {
            Ok(status) => status,
            Err(error) => CodexAccountStatus {
                state: CodexAccountState::Unavailable,
                plan_type: None,
                account_type: None,
                captured_at_ms,
                message: app_server_account::safe_account_error_message(&error),
            },
        }
    }

    pub fn read_rate_limits_status(&mut self, timeout: Duration) -> CodexRateLimitsStatus {
        let captured_at_ms = unix_now_ms();

        match self.with_connection(timeout, |connection| {
            app_server_rate_limits::read_rate_limits_from_initialized_connection(
                connection,
                timeout,
                captured_at_ms,
            )
        }) {
            Ok(status) => status,
            Err(error) => CodexRateLimitsStatus {
                state: CodexRateLimitsState::Unavailable,
                captured_at_ms,
                buckets: Vec::new(),
                message: app_server_rate_limits::safe_rate_limits_error_message(&error),
            },
        }
    }

    pub fn read_usage_status(&mut self, timeout: Duration) -> CodexUsageStatus {
        let captured_at_ms = unix_now_ms();

        match self.with_connection(timeout, |connection| {
            app_server_usage::read_usage_from_initialized_connection(
                connection,
                timeout,
                captured_at_ms,
            )
        }) {
            Ok(status) => status,
            Err(error) => CodexUsageStatus {
                state: CodexUsageState::Unavailable,
                captured_at_ms,
                summary: None,
                daily_usage_buckets: Vec::new(),
                message: app_server_usage::safe_usage_error_message(&error),
            },
        }
    }

    pub fn read_thread_token_usage_status(
        &mut self,
        handshake_timeout: Duration,
        wait_timeout: Duration,
    ) -> CodexThreadTokenUsageStatus {
        let captured_at_ms = unix_now_ms();

        if let Err(error) = self.ensure_connection(handshake_timeout) {
            return CodexThreadTokenUsageStatus::unavailable(
                app_server_thread_usage::safe_thread_usage_error_message(&error),
                captured_at_ms,
            );
        }

        let deadline = std::time::Instant::now() + wait_timeout;
        loop {
            self.drain_notifications(captured_at_ms);
            if let Some(status) = self.latest_thread_token_usage.clone() {
                return status;
            }

            if std::time::Instant::now() >= deadline {
                return CodexThreadTokenUsageStatus::waiting(captured_at_ms);
            }

            thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn shutdown(&mut self) {
        self.connection = None;
        if let Some(mut session) = self.session.take() {
            let _ = session.stop();
        }
    }

    fn with_connection<T>(
        &mut self,
        handshake_timeout: Duration,
        read: impl FnOnce(&mut AppServerConnection<ChildStdin>) -> Result<T, JsonlError>,
    ) -> Result<T, JsonlError> {
        self.ensure_connection(handshake_timeout)?;

        let result = {
            let connection = self
                .connection
                .as_mut()
                .expect("connection exists after ensure_connection");
            read(connection)
        };

        self.drain_notifications(unix_now_ms());

        if matches!(
            result,
            Err(JsonlError::Io(_)) | Err(JsonlError::EndOfStream) | Err(JsonlError::Json(_))
        ) {
            self.reset_connection();
        }

        result
    }

    fn ensure_connection(&mut self, handshake_timeout: Duration) -> Result<(), JsonlError> {
        if self.connection.is_some() && self.session_is_running()? {
            self.drain_notifications(unix_now_ms());
            return Ok(());
        }

        self.reset_connection();

        let mut session = AppServerSession::start(AppServerCommand::codex_from_environment())?;
        let mut connection = AppServerConnection::from_session(&mut session)?;
        perform_initialize_handshake_with_timeout(&mut connection, handshake_timeout)?;

        self.connection = Some(connection);
        self.session = Some(session);
        Ok(())
    }

    fn session_is_running(&mut self) -> Result<bool, JsonlError> {
        let Some(session) = self.session.as_mut() else {
            return Ok(false);
        };

        session.is_running().map_err(JsonlError::from)
    }

    fn reset_connection(&mut self) {
        self.connection = None;
        if let Some(mut session) = self.session.take() {
            let _ = session.stop();
        }
    }

    fn drain_notifications(&mut self, captured_at_ms: u64) {
        let Some(connection) = self.connection.as_mut() else {
            return;
        };

        while let Some(notification) = connection.try_next_notification() {
            if let Some(status) =
                app_server_thread_usage::thread_token_usage_status_from_notification(
                    &notification,
                    captured_at_ms,
                )
            {
                self.latest_thread_token_usage = Some(status);
            }
        }
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        env, fs,
        io::Read,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    #[cfg(unix)]
    fn reuses_one_app_server_for_multiple_account_requests() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let test_dir = unique_test_dir("persistent-client");
        let fake_codex = test_dir.join("codex");
        let starts_path = test_dir.join("starts");
        let script = format!(
            r#"#!/bin/sh
printf x >> {starts}
while IFS= read -r line; do
  case "$line" in
    *\"initialize\"*)
      printf '%s\n' '{{"id":1,"result":{{"userAgent":"fake-codex"}}}}'
      ;;
    *\"account/read\"*)
      printf '%s\n' '{{"id":2,"result":{{"account":{{"type":"chatgpt","planType":"pro"}},"requiresOpenaiAuth":false}}}}'
      ;;
    *\"account/rateLimits/read\"*)
      printf '%s\n' '{{"id":3,"result":{{"rateLimits":{{"limitId":"codex"}}}}}}'
      ;;
  esac
done
"#,
            starts = shell_quote(&starts_path)
        );
        write_executable_script(&fake_codex, &script);

        let previous_codex_cli_path = env::var_os("CODEX_CLI_PATH");
        // SAFETY: this test serializes access to CODEX_CLI_PATH with ENV_LOCK and
        // restores the previous value before returning.
        unsafe {
            env::set_var("CODEX_CLI_PATH", &fake_codex);
        }

        let mut client = PersistentAppServerClient::default();
        let account = client.read_account_status(Duration::from_secs(1));
        let rate_limits = client.read_rate_limits_status(Duration::from_secs(1));
        client.shutdown();

        restore_env_var("CODEX_CLI_PATH", previous_codex_cli_path);

        assert_eq!(account.state, CodexAccountState::SignedIn);
        assert_eq!(rate_limits.state, CodexRateLimitsState::Available);
        assert_eq!(read_to_string(&starts_path), "x");

        let _ = fs::remove_dir_all(test_dir);
    }

    #[cfg(unix)]
    fn unique_test_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "codex-reserve-app-server-client-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test dir");
        path
    }

    #[cfg(unix)]
    fn write_executable_script(path: &Path, contents: &str) {
        fs::write(path, contents).expect("write script");
        let mut permissions = fs::metadata(path).expect("script metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("make script executable");
    }

    #[cfg(unix)]
    fn read_to_string(path: &Path) -> String {
        let mut value = String::new();
        fs::File::open(path)
            .expect("open file")
            .read_to_string(&mut value)
            .expect("read file");
        value
    }

    #[cfg(unix)]
    fn restore_env_var(key: &str, previous_value: Option<std::ffi::OsString>) {
        // SAFETY: the caller holds ENV_LOCK, so no test in this module mutates the
        // process environment concurrently.
        unsafe {
            match previous_value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
    }

    #[cfg(unix)]
    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }
}
