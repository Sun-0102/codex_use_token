#[cfg(unix)]
mod unix {
    use codex_reserve_lib::{
        app_server_connection::AppServerConnection,
        app_server_jsonl::JsonlError,
        app_server_session::{AppServerCommand, AppServerSession},
        app_server_supervisor::{AppServerSupervisor, RestartPolicy, SupervisorEvent},
    };
    use serde_json::Value;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use std::os::unix::fs::PermissionsExt;

    static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn simulated_app_server_success_response_round_trip() {
        let fake_codex = create_executable_script(
            "#!/bin/sh\nIFS= read -r _line\nprintf '%s\\n' '{\"id\":1,\"result\":{\"status\":\"ok\"}}'\n",
        );
        let mut session = start_session(fake_codex);
        let mut connection =
            AppServerConnection::from_session(&mut session).expect("create app-server connection");

        let response: Value = connection
            .request("account/read", Option::<()>::None, Duration::from_secs(5))
            .expect("successful response");

        assert_eq!(response, serde_json::json!({ "status": "ok" }));
        session.stop().expect("stop simulated app-server");
    }

    #[test]
    fn simulated_app_server_silent_response_times_out() {
        let fake_codex = create_executable_script("#!/bin/sh\nIFS= read -r _line\nsleep 2\n");
        let mut session = start_session(fake_codex);
        let mut connection =
            AppServerConnection::from_session(&mut session).expect("create app-server connection");

        let error = connection
            .request::<_, Value>(
                "account/read",
                Option::<()>::None,
                Duration::from_millis(30),
            )
            .expect_err("request timeout");

        assert!(matches!(error, JsonlError::Timeout { .. }));
        session.stop().expect("stop simulated app-server");
    }

    #[test]
    fn simulated_app_server_malformed_json_is_reported() {
        let fake_codex =
            create_executable_script("#!/bin/sh\nprintf '%s\\n' 'not-json'\nsleep 2\n");
        let mut session = start_session(fake_codex);
        let mut connection =
            AppServerConnection::from_session(&mut session).expect("create app-server connection");

        let error = connection
            .request::<_, Value>("account/read", Option::<()>::None, Duration::from_secs(5))
            .expect_err("malformed JSON error");

        assert!(
            matches!(error, JsonlError::Json(_)),
            "expected JSON error, got {error:?}"
        );
        session.stop().expect("stop simulated app-server");
    }

    #[test]
    fn simulated_app_server_exit_is_detected_by_session() {
        let fake_codex = create_executable_script("#!/bin/sh\nexit 23\n");
        let mut session = start_session(fake_codex);

        wait_until_stopped(&mut session);

        assert!(!session.is_running().expect("exited status"));
        assert_eq!(
            session.last_exit_status().expect("recorded exit").code(),
            Some(23)
        );
    }

    #[test]
    fn simulated_app_server_exit_is_restarted_by_supervisor() {
        let marker_path = unique_test_path("restart-marker");
        let fake_codex = create_executable_script(&format!(
            "#!/bin/sh\nprintf '%s\\n' \"$$\" >> {}\nexit 0\n",
            shell_quote(&marker_path)
        ));
        let mut supervisor = AppServerSupervisor::new(
            RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
            move || {
                AppServerSession::start(
                    AppServerCommand::new(fake_codex.clone()).with_args(["app-server", "--stdio"]),
                )
            },
        );

        assert_eq!(supervisor.start().expect("start"), SupervisorEvent::Started);

        let first_exit_at = wait_for_supervisor_exit(&mut supervisor);
        assert_eq!(
            supervisor.poll(first_exit_at).expect("restart"),
            SupervisorEvent::Restarted
        );

        let starts = wait_for_marker_lines(&marker_path, 2);
        assert!(starts.len() >= 2);
        supervisor.shutdown().expect("shutdown");
    }

    fn start_session(fake_codex: PathBuf) -> AppServerSession {
        AppServerSession::start(
            AppServerCommand::new(fake_codex).with_args(["app-server", "--stdio"]),
        )
        .expect("start simulated app-server")
    }

    fn wait_for_supervisor_exit<F>(
        supervisor: &mut AppServerSupervisor<F, AppServerSession>,
    ) -> Instant
    where
        F: FnMut() -> std::io::Result<AppServerSession>,
    {
        for _ in 0..240 {
            let now = Instant::now();
            match supervisor.poll(now).expect("poll supervisor") {
                SupervisorEvent::Exited { next_restart_at } => return next_restart_at,
                SupervisorEvent::Running => std::thread::sleep(Duration::from_millis(25)),
                event => panic!("unexpected supervisor event: {event:?}"),
            }
        }

        panic!("supervisor did not detect simulated app-server exit");
    }

    fn unique_test_path(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos();
        let counter = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "codex-reserve-{name}-{}-{timestamp}-{counter}",
            std::process::id()
        ))
    }

    fn create_executable_script(script: &str) -> PathBuf {
        let script_path = unique_test_path("fake-codex");
        fs::write(&script_path, script).expect("write fake codex");
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o700))
            .expect("make fake codex executable");
        script_path
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }

    fn wait_until_stopped(session: &mut AppServerSession) {
        for _ in 0..240 {
            if !session.is_running().expect("running status") {
                return;
            }

            std::thread::sleep(Duration::from_millis(25));
        }

        panic!("simulated app-server did not exit");
    }

    fn wait_for_marker_lines(path: &Path, expected_lines: usize) -> Vec<String> {
        for _ in 0..80 {
            if let Ok(content) = fs::read_to_string(path) {
                let lines = content
                    .lines()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<String>>();
                if lines.len() >= expected_lines {
                    return lines;
                }
            }

            std::thread::sleep(Duration::from_millis(25));
        }

        fs::read_to_string(path)
            .expect("restart marker to be written")
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }
}
