use codex_reserve_lib::app_server_supervisor::{
    AppServerSupervisor, RestartPolicy, SupervisorEvent,
};
use std::{
    cell::Cell,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[cfg(unix)]
use codex_reserve_lib::app_server_session::{AppServerCommand, AppServerSession};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::{Command, Stdio};

static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct FakeSession {
    running: Rc<Cell<bool>>,
    stop_count: Rc<Cell<u32>>,
}

impl FakeSession {
    fn new(running: Rc<Cell<bool>>, stop_count: Rc<Cell<u32>>) -> Self {
        Self {
            running,
            stop_count,
        }
    }
}

impl codex_reserve_lib::app_server_supervisor::ManagedSession for FakeSession {
    fn is_running(&mut self) -> io::Result<bool> {
        Ok(self.running.get())
    }

    fn stop(&mut self) -> io::Result<()> {
        self.running.set(false);
        self.stop_count.set(self.stop_count.get() + 1);
        Ok(())
    }
}

#[test]
fn schedules_restart_after_an_unexpected_exit_and_restarts_when_due() {
    let now = Instant::now();
    let running = Rc::new(Cell::new(true));
    let stop_count = Rc::new(Cell::new(0));
    let starts = Rc::new(Cell::new(0));
    let mut supervisor = AppServerSupervisor::new(
        RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
        {
            let running = running.clone();
            let stop_count = stop_count.clone();
            let starts = starts.clone();
            move || {
                starts.set(starts.get() + 1);
                running.set(true);
                Ok(FakeSession::new(running.clone(), stop_count.clone()))
            }
        },
    );

    assert_eq!(supervisor.start().expect("start"), SupervisorEvent::Started);
    running.set(false);

    assert_eq!(
        supervisor.poll(now).expect("detect exit"),
        SupervisorEvent::Exited {
            next_restart_at: now + Duration::from_millis(10)
        }
    );
    assert_eq!(
        supervisor
            .poll(now + Duration::from_millis(9))
            .expect("wait"),
        SupervisorEvent::WaitingToRestart {
            next_restart_at: now + Duration::from_millis(10)
        }
    );
    assert_eq!(
        supervisor
            .poll(now + Duration::from_millis(10))
            .expect("restart"),
        SupervisorEvent::Restarted
    );
    assert_eq!(starts.get(), 2);
}

#[test]
fn restart_delay_backs_off_until_the_configured_cap() {
    let now = Instant::now();
    let running = Rc::new(Cell::new(true));
    let stop_count = Rc::new(Cell::new(0));
    let mut supervisor = AppServerSupervisor::new(
        RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(15)),
        {
            let running = running.clone();
            let stop_count = stop_count.clone();
            move || {
                running.set(true);
                Ok(FakeSession::new(running.clone(), stop_count.clone()))
            }
        },
    );

    supervisor.start().expect("start");
    running.set(false);
    assert_eq!(
        supervisor.poll(now).expect("first exit"),
        SupervisorEvent::Exited {
            next_restart_at: now + Duration::from_millis(10)
        }
    );

    supervisor
        .poll(now + Duration::from_millis(10))
        .expect("first restart");
    running.set(false);
    assert_eq!(
        supervisor
            .poll(now + Duration::from_millis(11))
            .expect("second exit"),
        SupervisorEvent::Exited {
            next_restart_at: now + Duration::from_millis(26)
        }
    );
}

#[test]
fn shutdown_stops_the_active_session_and_prevents_future_restarts() {
    let now = Instant::now();
    let running = Rc::new(Cell::new(true));
    let stop_count = Rc::new(Cell::new(0));
    let starts = Rc::new(Cell::new(0));
    let mut supervisor = AppServerSupervisor::new(
        RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
        {
            let running = running.clone();
            let stop_count = stop_count.clone();
            let starts = starts.clone();
            move || {
                starts.set(starts.get() + 1);
                running.set(true);
                Ok(FakeSession::new(running.clone(), stop_count.clone()))
            }
        },
    );

    supervisor.start().expect("start");
    assert_eq!(
        supervisor.shutdown().expect("shutdown"),
        SupervisorEvent::Stopped
    );

    assert!(!running.get());
    assert_eq!(stop_count.get(), 1);
    assert_eq!(
        supervisor.poll(now).expect("poll"),
        SupervisorEvent::Shutdown
    );
    assert_eq!(starts.get(), 1);
}

#[test]
fn dropping_supervisor_stops_the_active_session() {
    let running = Rc::new(Cell::new(true));
    let stop_count = Rc::new(Cell::new(0));
    {
        let mut supervisor = AppServerSupervisor::new(
            RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
            {
                let running = running.clone();
                let stop_count = stop_count.clone();
                move || Ok(FakeSession::new(running.clone(), stop_count.clone()))
            },
        );

        supervisor.start().expect("start");
    }

    assert!(!running.get());
    assert_eq!(stop_count.get(), 1);
}

#[cfg(unix)]
#[test]
fn shutdown_cleans_up_a_real_child_process_session() {
    let pid_path = unique_test_path("pid");
    let fake_codex = create_executable_script(&format!(
        "#!/bin/sh\nprintf '%s' \"$$\" > {}\nwhile IFS= read -r _line; do :; done\n",
        shell_quote(&pid_path)
    ));
    let mut supervisor = AppServerSupervisor::new(
        RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
        move || {
            AppServerSession::start(
                AppServerCommand::new(fake_codex.clone()).with_args(["app-server", "--stdio"]),
            )
        },
    );

    supervisor.start().expect("start");
    let process_id = wait_for_pid(&pid_path);

    assert!(process_is_alive(process_id));
    assert_eq!(
        supervisor.shutdown().expect("shutdown"),
        SupervisorEvent::Stopped
    );

    for _ in 0..20 {
        if !process_is_alive(process_id) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    assert!(!process_is_alive(process_id));
}

#[cfg(unix)]
#[test]
fn dropping_supervisor_cleans_up_a_real_child_process_session() {
    let pid_path = unique_test_path("pid");
    let fake_codex = create_executable_script(&format!(
        "#!/bin/sh\nprintf '%s' \"$$\" > {}\nwhile IFS= read -r _line; do :; done\n",
        shell_quote(&pid_path)
    ));
    let process_id = {
        let mut supervisor = AppServerSupervisor::new(
            RestartPolicy::new(Duration::from_millis(10), Duration::from_millis(100)),
            move || {
                AppServerSession::start(
                    AppServerCommand::new(fake_codex.clone()).with_args(["app-server", "--stdio"]),
                )
            },
        );

        supervisor.start().expect("start");
        wait_for_pid(&pid_path)
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
fn unique_test_path(name: &str) -> PathBuf {
    let counter = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "codex-reserve-supervisor-{name}-{}-{counter}",
        std::process::id()
    ))
}

#[cfg(unix)]
fn create_executable_script(script: &str) -> PathBuf {
    let script_path = unique_test_path("fake-codex");
    fs::write(&script_path, script).expect("write fake codex");
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o700))
        .expect("make fake codex executable");
    script_path
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(unix)]
fn wait_for_pid(path: &Path) -> u32 {
    for _ in 0..200 {
        if let Ok(content) = fs::read_to_string(path) {
            return content.parse().expect("pid");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    fs::read_to_string(path)
        .expect("pid file")
        .parse()
        .expect("pid")
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
