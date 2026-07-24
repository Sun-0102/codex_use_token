use std::{io, thread, time::Duration};

use tauri::{AppHandle, Emitter, Runtime};

pub const USAGE_REFRESH_EVENT: &str = "codex-reserve://usage-refresh";
pub const USAGE_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

pub fn start_usage_refresh_ticker<R: Runtime>(app: AppHandle<R>) -> io::Result<()> {
    thread::Builder::new()
        .name("usage-refresh-ticker".to_string())
        .spawn(move || {
            loop {
                thread::sleep(USAGE_REFRESH_INTERVAL);
                if app.emit(USAGE_REFRESH_EVENT, ()).is_err() {
                    break;
                }
            }
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_interval_is_thirty_seconds() {
        assert_eq!(USAGE_REFRESH_INTERVAL, Duration::from_secs(30));
    }
}
