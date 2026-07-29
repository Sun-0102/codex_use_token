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

pub(crate) fn discover_codex_executable() -> Option<PathBuf> {
    candidate_paths()
        .into_iter()
        .find(|path| path.is_file())
        .or_else(discover_codex_from_login_shell)
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
        for relative_path in [
            ".local/bin/codex",
            ".volta/bin/codex",
            ".cargo/bin/codex",
            ".asdf/shims/codex",
        ] {
            candidates.push(home_directory.join(relative_path));
        }

        let nvm_versions = home_directory.join(".nvm/versions/node");
        candidates.extend(node_version_manager_candidates(&nvm_versions, "bin/codex"));

        let fnm_versions = home_directory.join(".fnm/node-versions");
        candidates.extend(node_version_manager_candidates(
            &fnm_versions,
            "installation/bin/codex",
        ));

        let xdg_fnm_versions = home_directory.join(".local/share/fnm/node-versions");
        candidates.extend(node_version_manager_candidates(
            &xdg_fnm_versions,
            "installation/bin/codex",
        ));

        let asdf_node_installs = home_directory.join(".asdf/installs/nodejs");
        candidates.extend(node_version_manager_candidates(
            &asdf_node_installs,
            "bin/codex",
        ));
    }

    candidates.extend(windows_environment_candidates(
        env::var_os("APPDATA").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
        env::var_os("USERPROFILE").map(PathBuf::from),
    ));

    for directory in ["/opt/homebrew/bin", "/usr/local/bin"] {
        candidates.push(PathBuf::from(directory).join("codex"));
    }

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn windows_environment_candidates(
    app_data: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    user_profile: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(app_data) = app_data {
        let npm_bin = app_data.join("npm");
        candidates.extend([
            npm_bin.join("codex.cmd"),
            npm_bin.join("codex.exe"),
            npm_bin.join("codex"),
        ]);
    }

    if let Some(local_app_data) = local_app_data {
        let windows_apps = local_app_data.join("Microsoft").join("WindowsApps");
        candidates.extend([
            windows_apps.join("codex.exe"),
            windows_apps.join("codex.cmd"),
        ]);
    }

    if let Some(user_profile) = user_profile {
        let volta_bin = user_profile.join(".volta").join("bin");
        candidates.extend([
            volta_bin.join("codex.exe"),
            volta_bin.join("codex.cmd"),
            volta_bin.join("codex"),
        ]);
    }

    candidates
}

fn node_version_manager_candidates(root: &Path, relative_codex_path: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(relative_codex_path))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.cmp(left));
    candidates
}

fn discover_codex_from_login_shell() -> Option<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let mut shells = Vec::new();
        if let Some(shell) = env::var_os("SHELL") {
            shells.push(PathBuf::from(shell));
        }
        shells.extend([PathBuf::from("/bin/zsh"), PathBuf::from("/bin/bash")]);

        let mut seen = HashSet::new();
        for shell in shells
            .into_iter()
            .filter(|shell| seen.insert(shell.clone()))
        {
            let Ok(output) = Command::new(shell)
                .args(["-lc", "command -v codex"])
                .output()
            else {
                continue;
            };

            if !output.status.success() {
                continue;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let Some(path) = stdout.lines().map(str::trim).find(|line| !line.is_empty()) else {
                continue;
            };
            let candidate = PathBuf::from(path);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
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

    fn unique_probe_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "codex-reserve-cli-probe-{}-{}",
            name,
            std::process::id()
        ));
        path
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
    fn discovers_node_version_manager_candidates_newest_first() {
        let root = unique_probe_dir("node-versions");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("v20.0.0/bin")).expect("create old node bin");
        fs::create_dir_all(root.join("v24.18.0/bin")).expect("create new node bin");

        let candidates = node_version_manager_candidates(&root, "bin/codex");

        assert_eq!(
            candidates,
            vec![
                root.join("v24.18.0/bin/codex"),
                root.join("v20.0.0/bin/codex")
            ]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_common_windows_cli_install_locations() {
        let app_data = PathBuf::from("C:\\Users\\codex-user\\AppData\\Roaming");
        let local_app_data = PathBuf::from("C:\\Users\\codex-user\\AppData\\Local");
        let user_profile = PathBuf::from("C:\\Users\\codex-user");

        let candidates = windows_environment_candidates(
            Some(app_data.clone()),
            Some(local_app_data.clone()),
            Some(user_profile.clone()),
        );

        assert!(candidates.contains(&app_data.join("npm").join("codex.cmd")));
        assert!(
            candidates.contains(
                &local_app_data
                    .join("Microsoft")
                    .join("WindowsApps")
                    .join("codex.exe")
            )
        );
        assert!(candidates.contains(&user_profile.join(".volta").join("bin").join("codex.cmd")));
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
