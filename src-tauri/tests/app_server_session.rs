use codex_reserve_lib::app_server_session::AppServerCommand;
use std::path::Path;

#[cfg(unix)]
use codex_reserve_lib::app_server_session::AppServerSession;
#[cfg(unix)]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::{Command, Stdio};

#[cfg(unix)]
static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn codex_command_uses_app_server_stdio() {
    let command = AppServerCommand::codex();

    assert_eq!(command.executable(), Path::new("codex"));
    assert_eq!(
        command
            .arguments()
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["app-server", "--stdio"]
    );
}

#[cfg(unix)]
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

#[cfg(unix)]
fn unique_test_dir(name: &str) -> PathBuf {
    let path = unique_test_path(name);
    fs::create_dir(&path).expect("create test directory");
    path
}

#[cfg(unix)]
fn create_fake_codex(capture_path: &Path) -> PathBuf {
    create_executable_script(&format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\nwhile IFS= read -r _line; do :; done\n",
        shell_quote(capture_path)
    ))
}

#[cfg(unix)]
fn create_executable_script(script: &str) -> PathBuf {
    let script_path = unique_test_path("fake-codex");
    write_executable_script(&script_path, script);
    script_path
}

#[cfg(unix)]
fn write_executable_script(script_path: &Path, script: &str) {
    fs::write(script_path, script).expect("write fake codex");
    fs::set_permissions(script_path, fs::Permissions::from_mode(0o700))
        .expect("make fake codex executable");
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(unix)]
fn wait_for_file(path: &Path) -> String {
    for _ in 0..200 {
        if let Ok(content) = fs::read_to_string(path) {
            return content;
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    fs::read_to_string(path).expect("file to be created")
}

#[cfg(unix)]
fn wait_until_stopped(session: &mut AppServerSession) {
    for _ in 0..240 {
        if !session.is_running().expect("running status") {
            return;
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    panic!("simulated app-server did not exit");
}

#[cfg(unix)]
#[test]
fn starts_codex_app_server_with_stdio_and_stops_it_cleanly() {
    let capture_path = unique_test_path("args");
    let fake_codex = create_fake_codex(&capture_path);
    let command = AppServerCommand::new(fake_codex.clone()).with_args(["app-server", "--stdio"]);

    let mut session = AppServerSession::start(command).expect("start app-server session");

    std::thread::sleep(Duration::from_millis(50));

    assert!(session.is_running().expect("running status"));
    assert_eq!(session.command_path(), fake_codex.as_path());
    assert_eq!(wait_for_file(&capture_path), "app-server\n--stdio\n");

    session.stop().expect("stop app-server session");

    assert!(!session.is_running().expect("stopped status"));
}

#[cfg(unix)]
#[test]
fn starts_with_cli_directory_at_the_front_of_child_path() {
    let bin_dir = unique_test_dir("fake-codex-bin");
    let marker_path = unique_test_path("helper-output");
    let helper_path = bin_dir.join("codex-helper");
    let fake_codex = bin_dir.join("codex");

    write_executable_script(
        &helper_path,
        &format!(
            "#!/bin/sh\nprintf helper-ok > {}\n",
            shell_quote(&marker_path)
        ),
    );
    write_executable_script(
        &fake_codex,
        "#!/bin/sh\ncodex-helper\nwhile IFS= read -r _line; do :; done\n",
    );

    let command = AppServerCommand::new(fake_codex).with_args(["app-server", "--stdio"]);
    let mut session = AppServerSession::start(command).expect("start app-server session");

    assert_eq!(wait_for_file(&marker_path), "helper-ok");

    session.stop().expect("stop app-server session");
}

#[cfg(unix)]
#[test]
fn records_status_when_app_server_exits_before_stop() {
    let fake_codex = create_executable_script("#!/bin/sh\nexit 7\n");
    let command = AppServerCommand::new(fake_codex).with_args(["app-server", "--stdio"]);
    let mut session = AppServerSession::start(command).expect("start app-server session");

    wait_until_stopped(&mut session);

    assert!(!session.is_running().expect("exited status"));
    assert_eq!(
        session.last_exit_status().expect("recorded exit").code(),
        Some(7)
    );
    session.stop().expect("stop already exited session");
}

#[cfg(unix)]
#[test]
fn dropping_a_session_cleans_up_the_child_process() {
    let capture_path = unique_test_path("args");
    let fake_codex = create_fake_codex(&capture_path);
    let process_id = {
        let command = AppServerCommand::new(fake_codex).with_args(["app-server", "--stdio"]);
        let mut session = AppServerSession::start(command).expect("start app-server session");

        assert!(session.is_running().expect("running status"));
        session.process_id()
    };

    for _ in 0..20 {
        if !process_is_alive(process_id) {
            return;
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(!process_is_alive(process_id));
}

#[cfg(unix)]
fn process_is_alive(process_id: u32) -> bool {
    Command::new("kill")
        .args(["-0", &process_id.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run kill -0")
        .success()
}
