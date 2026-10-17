// SPDX-License-Identifier: Apache-2.0
//! Askpass bridge (SDD §10): network commands run with GIT_ASKPASS/SSH_ASKPASS pointing at this
//! same executable. In that mode it only forwards the prompt over a private Unix socket to the
//! running app, which asks the user, and prints the answer for Git. Secrets never touch disk.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::diagnostics;

pub const SOCK_ENV: &str = "ATLASGIT_ASKPASS_SOCK";
const ANSWER_TIMEOUT: Duration = Duration::from_secs(300);

/// Askpass client mode: prompt in argv[1], answer on stdout. Exit code 1 = cancelled.
pub fn client(sock: &str) -> i32 {
    let prompt = std::env::args().nth(1).unwrap_or_default();
    let Ok(mut s) = UnixStream::connect(sock) else { return 1 };
    if writeln!(s, "{}", prompt.replace('\n', " ")).is_err() {
        return 1;
    }
    let mut line = String::new();
    // EOF without a line means the user cancelled.
    if BufReader::new(&s).read_line(&mut line).unwrap_or(0) == 0 {
        return 1;
    }
    print!("{line}");
    0
}

struct Waiter {
    repo_id: u32,
    tx: mpsc::Sender<Option<String>>,
}

/// Answers waiting for the user, by request id (with the repository that asked).
#[derive(Default)]
pub struct Pending(Mutex<HashMap<u64, Waiter>>);

impl Pending {
    pub fn answer(&self, id: u64, value: Option<String>) {
        if let Some(w) = diagnostics::lock(&self.0).remove(&id) {
            let _ = w.tx.send(value);
        }
    }

    /// Cancels every question `repo_id` is waiting on (Git then fails with an auth error); returns their ids.
    pub fn cancel_repo(&self, repo_id: u32) -> Vec<u64> {
        let mut map = diagnostics::lock(&self.0);
        let ids: Vec<u64> = map.iter().filter(|(_, w)| w.repo_id == repo_id).map(|(id, _)| *id).collect();
        for id in &ids {
            if let Some(w) = map.remove(id) {
                let _ = w.tx.send(None);
            }
        }
        ids
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    id: u64,
    repo_id: u32,
    prompt: String,
    secret: bool,
}

static NEXT: AtomicU64 = AtomicU64::new(1);

fn is_secret(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    ["password", "passphrase", "token", "pin"].iter().any(|w| p.contains(w))
}

/// Runs `f` with the environment that routes credential prompts to the UI.
pub fn with_bridge<T>(app: &AppHandle, pending: Arc<Pending>, repo_id: u32, f: impl FnOnce(&[(String, String)]) -> T) -> T {
    let (Ok(exe), dir) = (std::env::current_exe(), std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir)) else {
        return f(&[]);
    };
    let path = dir.join(format!("atlasgit-askpass-{}-{}.sock", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else { return f(&[]) };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    let _ = listener.set_nonblocking(true);
    let stop = Arc::new(AtomicBool::new(false));
    let current = Arc::new(Mutex::new(None::<u64>));
    let worker = {
        let (app, pending, stop, current) = (app.clone(), pending.clone(), stop.clone(), current.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, _)) => {
                        let _ = s.set_nonblocking(false);
                        serve(&app, &pending, &current, repo_id, s);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
        })
    };
    let exe = exe.to_string_lossy().into_owned();
    let env = vec![
        ("GIT_ASKPASS".to_owned(), exe.clone()),
        ("SSH_ASKPASS".to_owned(), exe),
        // OpenSSH >= 8.4: use askpass even without a TTY/DISPLAY check (we are a GUI).
        ("SSH_ASKPASS_REQUIRE".to_owned(), "force".to_owned()),
        (SOCK_ENV.to_owned(), path.to_string_lossy().into_owned()),
    ];
    let out = f(&env);
    stop.store(true, Ordering::Relaxed);
    // Git is done: drop any question still on screen so the worker can exit.
    if let Some(id) = diagnostics::lock(&current).take() {
        pending.answer(id, None);
        let _ = app.emit("credentials-cancel", id);
    }
    let _ = worker.join();
    let _ = std::fs::remove_file(&path);
    out
}

fn serve(app: &AppHandle, pending: &Pending, current: &Mutex<Option<u64>>, repo_id: u32, s: UnixStream) {
    let mut prompt = String::new();
    if BufReader::new(&s).read_line(&mut prompt).unwrap_or(0) == 0 {
        return;
    }
    let prompt = prompt.trim_end().to_owned();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = mpsc::channel();
    diagnostics::lock(&pending.0).insert(id, Waiter { repo_id, tx });
    *diagnostics::lock(current) = Some(id);
    let _ = app.emit("credentials-request", Request { id, repo_id, secret: is_secret(&prompt), prompt });
    let answer = rx.recv_timeout(ANSWER_TIMEOUT).ok().flatten();
    diagnostics::lock(&pending.0).remove(&id);
    *diagnostics::lock(current) = None;
    if let Some(a) = answer {
        let mut s = s;
        let _ = writeln!(s, "{a}");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn secret_prompts() {
        assert!(super::is_secret("Password for 'https://x@github.com': "));
        assert!(super::is_secret("Enter passphrase for key '/home/u/.ssh/id_ed25519': "));
        assert!(!super::is_secret("Username for 'https://github.com': "));
    }
}
