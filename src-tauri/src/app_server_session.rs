use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
};

use crate::cli_probe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppServerCommand {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl AppServerCommand {
    pub fn codex() -> Self {
        Self::new("codex").with_args(["app-server", "--stdio"])
    }

    pub fn codex_from_environment() -> Self {
        let executable =
            cli_probe::discover_codex_executable().unwrap_or_else(|| PathBuf::from("codex"));

        Self::new(executable).with_args(["app-server", "--stdio"])
    }

    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.arguments = arguments
            .into_iter()
            .map(|argument| argument.as_ref().to_os_string())
            .collect();
        self
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug)]
pub struct AppServerSession {
    command: AppServerCommand,
    child: Child,
    last_exit_status: Option<ExitStatus>,
}

impl AppServerSession {
    pub fn start(command: AppServerCommand) -> io::Result<Self> {
        let mut child_command = Command::new(command.executable());
        child_command
            .args(command.arguments())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        prepend_executable_directory_to_path(&mut child_command, command.executable())?;

        let mut child = child_command.spawn()?;

        if child.stdin.is_none() || child.stdout.is_none() || child.stderr.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "app-server process started without piped stdio",
            ));
        }

        Ok(Self {
            command,
            child,
            last_exit_status: None,
        })
    }

    pub fn command_path(&self) -> &Path {
        self.command.executable()
    }

    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub fn is_running(&mut self) -> io::Result<bool> {
        if self.last_exit_status.is_some() {
            return Ok(false);
        }

        match self.child.try_wait()? {
            Some(status) => {
                self.last_exit_status = Some(status);
                Ok(false)
            }
            None => Ok(true),
        }
    }

    pub fn last_exit_status(&self) -> Option<ExitStatus> {
        self.last_exit_status
    }

    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if self.last_exit_status.is_some() {
            return Ok(());
        }

        if let Some(status) = self.child.try_wait()? {
            self.last_exit_status = Some(status);
            return Ok(());
        }

        self.child.kill()?;
        let status = self.child.wait()?;
        self.last_exit_status = Some(status);
        Ok(())
    }
}

impl Drop for AppServerSession {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn prepend_executable_directory_to_path(
    child_command: &mut Command,
    executable: &Path,
) -> io::Result<()> {
    let Some(executable_dir) = executable
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    else {
        return Ok(());
    };

    let mut path_entries = vec![executable_dir.to_path_buf()];
    if let Some(existing_path) = env::var_os("PATH") {
        path_entries.extend(env::split_paths(&existing_path));
    }

    let child_path = env::join_paths(path_entries).map_err(io::Error::other)?;
    child_command.env("PATH", child_path);

    Ok(())
}
