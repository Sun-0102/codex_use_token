use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::{
    process::{Command, Output},
    sync::OnceLock,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WslCodexInstallation {
    wsl_executable: PathBuf,
    distribution: String,
    codex_executable: String,
    codex_home: Option<PathBuf>,
}

impl WslCodexInstallation {
    #[cfg(any(target_os = "windows", test))]
    pub(crate) fn new(
        wsl_executable: PathBuf,
        distribution: impl Into<String>,
        codex_executable: impl Into<String>,
        codex_home: Option<PathBuf>,
    ) -> Self {
        Self {
            wsl_executable,
            distribution: distribution.into(),
            codex_executable: codex_executable.into(),
            codex_home,
        }
    }

    pub(crate) fn wsl_executable(&self) -> &Path {
        &self.wsl_executable
    }

    pub(crate) fn distribution(&self) -> &str {
        &self.distribution
    }

    pub(crate) fn codex_executable(&self) -> &str {
        &self.codex_executable
    }

    pub(crate) fn codex_home(&self) -> Option<&Path> {
        self.codex_home.as_deref()
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn discover_wsl_codex_installations() -> Vec<WslCodexInstallation> {
    static INSTALLATIONS: OnceLock<Vec<WslCodexInstallation>> = OnceLock::new();

    INSTALLATIONS.get_or_init(discover_uncached).clone()
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn discover_wsl_codex_installations() -> Vec<WslCodexInstallation> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn discover_uncached() -> Vec<WslCodexInstallation> {
    let wsl_executable = system_wsl_executable();
    let Ok(output) = run_wsl(&wsl_executable, ["--list", "--quiet"]) else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    parse_wsl_distributions(&output.stdout)
        .into_iter()
        .filter_map(|distribution| discover_distribution(&wsl_executable, distribution))
        .collect()
}

#[cfg(target_os = "windows")]
fn discover_distribution(
    wsl_executable: &Path,
    distribution: String,
) -> Option<WslCodexInstallation> {
    let codex_output = run_wsl(
        wsl_executable,
        [
            "--distribution",
            distribution.as_str(),
            "--exec",
            "sh",
            "-lc",
            "command -v codex",
        ],
    )
    .ok()?;
    if !codex_output.status.success() {
        return None;
    }
    let codex_executable = first_nonempty_line(&codex_output.stdout)?;

    let home_output = run_wsl(
        wsl_executable,
        [
            "--distribution",
            distribution.as_str(),
            "--exec",
            "sh",
            "-lc",
            "wslpath -w \"${CODEX_HOME:-$HOME/.codex}\"",
        ],
    )
    .ok();
    let codex_home = home_output
        .filter(|output| output.status.success())
        .and_then(|output| first_nonempty_line(&output.stdout))
        .map(PathBuf::from);

    Some(WslCodexInstallation::new(
        wsl_executable.to_path_buf(),
        distribution,
        codex_executable,
        codex_home,
    ))
}

#[cfg(target_os = "windows")]
fn system_wsl_executable() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("wsl.exe"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("wsl.exe"))
}

#[cfg(target_os = "windows")]
fn run_wsl<'a>(
    wsl_executable: &Path,
    arguments: impl IntoIterator<Item = &'a str>,
) -> std::io::Result<Output> {
    let mut command = Command::new(wsl_executable);
    command.args(arguments);
    configure_no_window(&mut command);
    command.output()
}

#[cfg(target_os = "windows")]
fn configure_no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(any(target_os = "windows", test))]
fn parse_wsl_distributions(output: &[u8]) -> Vec<String> {
    decode_command_output(output)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !matches!(
                line.to_ascii_lowercase().as_str(),
                "docker-desktop" | "docker-desktop-data"
            )
        })
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(target_os = "windows")]
fn first_nonempty_line(output: &[u8]) -> Option<String> {
    decode_command_output(output)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(any(target_os = "windows", test))]
fn decode_command_output(output: &[u8]) -> String {
    let looks_like_utf16 =
        output.len().is_multiple_of(2) && output.chunks_exact(2).take(32).any(|pair| pair[1] == 0);

    if looks_like_utf16 {
        let words = output
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from_utf8_lossy(output).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16_wsl_distribution_output_and_skips_internal_distros() {
        let text = "Ubuntu\r\nDocker-Desktop\r\nDebian\r\n";
        let utf16 = text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();

        assert_eq!(
            parse_wsl_distributions(&utf16),
            vec!["Ubuntu".to_string(), "Debian".to_string()]
        );
    }

    #[test]
    fn creates_a_wsl_codex_launch_with_a_windows_accessible_home() {
        let installation = WslCodexInstallation::new(
            PathBuf::from("C:\\Windows\\System32\\wsl.exe"),
            "Ubuntu",
            "/home/codex/.local/bin/codex",
            Some(PathBuf::from(
                "\\\\wsl.localhost\\Ubuntu\\home\\codex\\.codex",
            )),
        );

        assert_eq!(installation.distribution(), "Ubuntu");
        assert_eq!(
            installation.codex_executable(),
            "/home/codex/.local/bin/codex"
        );
        assert_eq!(
            installation.codex_home(),
            Some(PathBuf::from("\\\\wsl.localhost\\Ubuntu\\home\\codex\\.codex").as_path())
        );
    }
}
