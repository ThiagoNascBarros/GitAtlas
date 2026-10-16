// SPDX-License-Identifier: Apache-2.0
//! Crash diagnostics: a panic hook that appends the panic and a backtrace to
//! `<state dir>/atlasgit/crash.log`, so "the app closed by itself" leaves evidence.

use std::io::Write;
use std::path::PathBuf;

/// `$XDG_STATE_HOME/atlasgit`, else `~/.local/state/atlasgit`.
pub fn log_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("atlasgit"))
}

fn write_entry(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Keep the file small: start over past 1 MiB.
    if std::fs::metadata(path).map(|m| m.len() > 1 << 20).unwrap_or(false) {
        std::fs::remove_file(path)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(text.as_bytes())
}

/// Installs the hook; the default hook (stderr) still runs afterwards.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = log_dir() {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let thread = std::thread::current();
            let text = format!(
                "[{secs}] panic in thread {:?}: {info}\n{}\n\n",
                thread.name().unwrap_or("<unnamed>"),
                std::backtrace::Backtrace::force_capture()
            );
            let _ = write_entry(&dir.join("crash.log"), &text);
        }
        default(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_entries_and_creates_the_directory() {
        let dir = std::env::temp_dir().join(format!("atlasgit-diag-{}", std::process::id()));
        let file = dir.join("nested").join("crash.log");
        write_entry(&file, "one\n").unwrap();
        write_entry(&file, "two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "one\ntwo\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
