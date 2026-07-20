use serde::Serialize;
use std::{
    collections::HashSet,
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

const MINIMUM_SUPPORTED_VERSION: CliVersion = CliVersion::new(0, 144, 5);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexCliStatus {
    pub state: CodexCliState,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CodexCliState {
    Available,
    NotInstalled,
    NotLoggedIn,
    Incompatible,
    LaunchFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CliVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl CliVersion {
    const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Debug)]
struct CommandResult {
    success: bool,
    stdout: String,
    stderr: String,
}

trait CommandRunner {
    fn run(&self, executable: &Path, arguments: &[&str]) -> io::Result<CommandResult>;
}

struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, executable: &Path, arguments: &[&str]) -> io::Result<CommandResult> {
        let mut command = Command::new(executable);
        command.args(arguments);

        if let Some(executable_directory) = executable.parent() {
            let inherited_path = env::var_os("PATH").unwrap_or_default();
            let child_paths = std::iter::once(executable_directory.to_path_buf())
                .chain(env::split_paths(&inherited_path));
            if let Ok(child_path) = env::join_paths(child_paths) {
                command.env("PATH", child_path);
            }
        }

        let output = command.output()?;

        Ok(CommandResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

pub fn probe_codex_cli() -> CodexCliStatus {
    probe_discovered_executable(discover_codex_executable(), &SystemCommandRunner)
}

fn probe_discovered_executable(
    executable: Option<PathBuf>,
    runner: &impl CommandRunner,
) -> CodexCliStatus {
    let Some(executable) = executable else {
        return CodexCliStatus {
            state: CodexCliState::NotInstalled,
            executable_path: None,
            version: None,
            message: "未检测到 Codex CLI，请先安装 Codex".to_string(),
        };
    };

    probe_executable(&executable, runner)
}

fn probe_executable(executable: &Path, runner: &impl CommandRunner) -> CodexCliStatus {
    let display_path = executable.to_string_lossy().into_owned();
    let version_result = match runner.run(executable, &["--version"]) {
        Ok(result) if result.success => result,
        _ => {
            return CodexCliStatus {
                state: CodexCliState::LaunchFailed,
                executable_path: Some(display_path),
                version: None,
                message: "Codex CLI 无法启动，请检查安装权限".to_string(),
            };
        }
    };

    let Some(parsed_version) = parse_cli_version(&version_result.stdout) else {
        return CodexCliStatus {
            state: CodexCliState::Incompatible,
            executable_path: Some(display_path),
            version: None,
            message: "无法识别 Codex CLI 版本，请升级后重试".to_string(),
        };
    };

    if parsed_version < MINIMUM_SUPPORTED_VERSION {
        return CodexCliStatus {
            state: CodexCliState::Incompatible,
            executable_path: Some(display_path),
            version: Some(format_version(parsed_version)),
            message: format!(
                "Codex CLI 版本过低，需要 {} 或更高版本",
                format_version(MINIMUM_SUPPORTED_VERSION)
            ),
        };
    }

    match runner.run(executable, &["login", "status"]) {
        Ok(result) if result.success => CodexCliStatus {
            state: CodexCliState::Available,
            executable_path: Some(display_path),
            version: Some(format_version(parsed_version)),
            message: "Codex CLI 已安装并登录".to_string(),
        },
        Ok(result) if indicates_logged_out(&result) => CodexCliStatus {
            state: CodexCliState::NotLoggedIn,
            executable_path: Some(display_path),
            version: Some(format_version(parsed_version)),
            message: "Codex CLI 尚未登录，请先运行 codex login".to_string(),
        },
        Ok(_) => CodexCliStatus {
            state: CodexCliState::LaunchFailed,
            executable_path: Some(display_path),
            version: Some(format_version(parsed_version)),
            message: "Codex CLI 登录状态检查失败，请在终端运行 codex login status".to_string(),
        },
        Err(_) => CodexCliStatus {
            state: CodexCliState::LaunchFailed,
            executable_path: Some(display_path),
            version: Some(format_version(parsed_version)),
            message: "无法检查 Codex 登录状态，请重新安装或升级 Codex CLI".to_string(),
        },
    }
}

fn discover_codex_executable() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|path| path.is_file())
}

fn candidate_paths() -> Vec<PathBuf> {
    let executable_names: &[&str] = if cfg!(target_os = "windows") {
        &["codex.exe", "codex.cmd", "codex"]
    } else {
        &["codex"]
    };
    let mut candidates = Vec::new();

    if let Some(explicit_path) = env::var_os("CODEX_CLI_PATH") {
        candidates.push(PathBuf::from(explicit_path));
    }

    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            for name in executable_names {
                candidates.push(directory.join(name));
            }
        }
    }

    if let Some(home_directory) = env::var_os("HOME").map(PathBuf::from) {
        for relative_path in [".local/bin/codex", ".volta/bin/codex", ".cargo/bin/codex"] {
            candidates.push(home_directory.join(relative_path));
        }

        let nvm_versions = home_directory.join(".nvm/versions/node");
        if let Ok(entries) = fs::read_dir(nvm_versions) {
            let mut nvm_candidates = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("bin/codex"))
                .collect::<Vec<_>>();
            nvm_candidates.sort_by(|left, right| right.cmp(left));
            candidates.extend(nvm_candidates);
        }
    }

    for directory in ["/opt/homebrew/bin", "/usr/local/bin"] {
        candidates.push(PathBuf::from(directory).join("codex"));
    }

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn parse_cli_version(output: &str) -> Option<CliVersion> {
    output.split_whitespace().find_map(|part| {
        let numeric = part
            .trim_start_matches('v')
            .split_once('-')
            .map_or(part.trim_start_matches('v'), |(version, _)| version);
        let mut components = numeric.split('.');
        let major = components.next()?.parse().ok()?;
        let minor = components.next()?.parse().ok()?;
        let patch = components.next()?.parse().ok()?;

        Some(CliVersion::new(major, minor, patch))
    })
}

fn indicates_logged_out(result: &CommandResult) -> bool {
    let output = format!("{} {}", result.stdout, result.stderr).to_ascii_lowercase();
    output.contains("not logged in") || output.contains("not logged")
}

fn format_version(version: CliVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Mutex};

    struct FakeRunner {
        results: Mutex<VecDeque<io::Result<CommandResult>>>,
    }

    impl FakeRunner {
        fn with(results: Vec<io::Result<CommandResult>>) -> Self {
            Self {
                results: Mutex::new(results.into()),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, _executable: &Path, _arguments: &[&str]) -> io::Result<CommandResult> {
            self.results.lock().unwrap().pop_front().unwrap()
        }
    }

    fn success(stdout: &str) -> io::Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }

    fn failure(stderr: &str) -> io::Result<CommandResult> {
        Ok(CommandResult {
            success: false,
            stdout: String::new(),
            stderr: stderr.to_string(),
        })
    }

    #[test]
    fn parses_current_and_prefixed_versions() {
        assert_eq!(
            parse_cli_version("codex-cli 0.144.5"),
            Some(CliVersion::new(0, 144, 5))
        );
        assert_eq!(
            parse_cli_version("codex v1.2.3-beta.1"),
            Some(CliVersion::new(1, 2, 3))
        );
    }

    #[test]
    fn reports_an_available_logged_in_cli() {
        let runner = FakeRunner::with(vec![
            success("codex-cli 0.144.5"),
            success("Logged in using ChatGPT"),
        ]);

        let status = probe_executable(Path::new("/test/codex"), &runner);

        assert_eq!(status.state, CodexCliState::Available);
        assert_eq!(status.version.as_deref(), Some("0.144.5"));
        assert_eq!(status.executable_path.as_deref(), Some("/test/codex"));
    }

    #[test]
    fn reports_when_no_cli_is_installed() {
        let runner = FakeRunner::with(vec![]);

        let status = probe_discovered_executable(None, &runner);

        assert_eq!(status.state, CodexCliState::NotInstalled);
        assert_eq!(status.executable_path, None);
        assert_eq!(status.version, None);
    }

    #[test]
    fn reports_a_cli_that_is_not_logged_in() {
        let runner = FakeRunner::with(vec![success("codex-cli 0.144.5"), failure("Not logged in")]);

        let status = probe_executable(Path::new("/test/codex"), &runner);

        assert_eq!(status.state, CodexCliState::NotLoggedIn);
    }

    #[test]
    fn does_not_misreport_an_unknown_status_error_as_logged_out() {
        let runner = FakeRunner::with(vec![
            success("codex-cli 0.144.5"),
            failure("configuration is invalid: secret diagnostic"),
        ]);

        let status = probe_executable(Path::new("/test/codex"), &runner);

        assert_eq!(status.state, CodexCliState::LaunchFailed);
        assert!(!status.message.contains("secret diagnostic"));
    }

    #[test]
    fn rejects_versions_older_than_the_tested_protocol() {
        let runner = FakeRunner::with(vec![success("codex-cli 0.143.9")]);

        let status = probe_executable(Path::new("/test/codex"), &runner);

        assert_eq!(status.state, CodexCliState::Incompatible);
        assert!(status.message.contains("0.144.5"));
    }

    #[test]
    fn reports_a_process_launch_failure_without_exposing_stderr() {
        let runner = FakeRunner::with(vec![Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "secret diagnostic",
        ))]);

        let status = probe_executable(Path::new("/test/codex"), &runner);

        assert_eq!(status.state, CodexCliState::LaunchFailed);
        assert!(!status.message.contains("secret diagnostic"));
    }
}
