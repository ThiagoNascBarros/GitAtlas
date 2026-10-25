// SPDX-License-Identifier: Apache-2.0
//! Diagnóstico de falhas (PERF-12).
//!
//! - Log em arquivo rotativo (`gitatlas.log.AAAA-MM-DD`) em `$XDG_STATE_HOME/gitatlas`
//!   (ou `~/.local/state/gitatlas`; no Windows, `%LOCALAPPDATA%\gitatlas`).
//! - Hook de pânico que grava o pânico e o backtrace em `last-crash.txt`, de forma
//!   síncrona (o worker do log pode não ter drenado antes de um abort).
//! - Marcador `session.running` com o PID da sessão: se ele sobrar e o PID não estiver
//!   mais vivo, a sessão anterior terminou sem passar pelo encerramento normal.
//! - `lock` recupera um `Mutex` envenenado em vez de propagar o pânico em cadeia.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

const LOG_FILE_PREFIX: &str = "gitatlas.log";
const MAX_LOG_FILES: usize = 14;
const CRASH_FILE: &str = "last-crash.txt";
const SESSION_FILE: &str = "session.running";

/// Pasta de estado do GitAtlas, resolvida a partir das variáveis de ambiente informadas.
fn state_dir_from(
    xdg_state: Option<PathBuf>,
    home: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> PathBuf {
    let base = xdg_state
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| home.map(|h| h.join(".local").join("state")))
        .or(local_app_data)
        .unwrap_or_else(std::env::temp_dir);
    base.join("gitatlas")
}

/// Pasta de estado real do processo.
pub fn state_dir() -> PathBuf {
    state_dir_from(
        std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

/// Guarda que mantém o escritor de log vivo; deve ficar em `main` até o fim do processo.
pub struct Diagnostics {
    _log: Option<WorkerGuard>,
    dir: PathBuf,
    /// A sessão anterior não encerrou normalmente.
    unclean_previous_exit: bool,
}

/// Inicializa log, hook de pânico e marcador de sessão. Nunca falha: sem pasta
/// gravável o app segue sem log em arquivo.
pub fn init() -> Diagnostics {
    let dir = state_dir();
    let log = if fs::create_dir_all(&dir).is_ok() {
        install_logger(&dir)
    } else {
        None
    };
    let unclean_previous_exit = claim_session(&dir);
    install_panic_hook(dir.clone());
    tracing::info!(version = env!("CARGO_PKG_VERSION"), dir = %dir.display(), "GitAtlas iniciado");
    if unclean_previous_exit {
        tracing::warn!("a sessão anterior não encerrou normalmente");
    }
    Diagnostics {
        _log: log,
        dir,
        unclean_previous_exit,
    }
}

impl Diagnostics {
    /// Marca a sessão como encerrada normalmente. Chamado em `RunEvent::Exit`.
    pub fn finish(&self) {
        tracing::info!("GitAtlas encerrado");
        let _ = fs::remove_file(self.dir.join(SESSION_FILE));
    }

    pub fn status(&self) -> DiagnosticsStatus {
        DiagnosticsStatus {
            log_dir: self.dir.to_string_lossy().into_owned(),
            unclean_previous_exit: self.unclean_previous_exit,
            previous_crash: read_crash(&self.dir),
        }
    }

    /// Descarta o aviso de queda depois que o usuário o viu.
    pub fn dismiss_crash(&self) {
        let _ = fs::remove_file(self.dir.join(CRASH_FILE));
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsStatus {
    pub log_dir: String,
    pub unclean_previous_exit: bool,
    pub previous_crash: Option<CrashReport>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    /// Texto gravado no momento da falha (mensagem, local e backtrace).
    pub details: String,
}

fn install_logger(dir: &Path) -> Option<WorkerGuard> {
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .max_log_files(MAX_LOG_FILES)
        .build(dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_env("GITATLAS_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .ok()?;
    Some(guard)
}

fn install_panic_hook(dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(sem mensagem)".to_owned());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let details = format!("{payload}\nlocal: {location}\n\n{backtrace}");
        tracing::error!(location = %location, "pânico: {payload}");
        // Gravação síncrona: o log assíncrono pode não ser drenado antes de um abort.
        let _ = write_crash(&dir, &details);
        previous(info);
    }));
}

/// Grava o relatório de falha. Separado do hook para ser testável.
fn write_crash(dir: &Path, details: &str) -> io::Result<()> {
    fs::write(dir.join(CRASH_FILE), details)
}

fn read_crash(dir: &Path) -> Option<CrashReport> {
    fs::read_to_string(dir.join(CRASH_FILE))
        .ok()
        .map(|details| CrashReport { details })
}

/// Registra a sessão atual. Devolve `true` se o marcador de uma sessão anterior
/// sobrou e o processo dele não existe mais.
fn claim_session(dir: &Path) -> bool {
    let path = dir.join(SESSION_FILE);
    let unclean = fs::read_to_string(&path)
        .map(|pid| is_stale_session(&pid, Path::new("/proc")))
        .unwrap_or(false);
    let _ = fs::write(&path, std::process::id().to_string());
    unclean
}

/// Um marcador é obsoleto quando traz um PID diferente do atual que não aparece em
/// `proc_root`. Sem `/proc` (Windows, macOS) não há como verificar: o marcador vale.
fn is_stale_session(contents: &str, proc_root: &Path) -> bool {
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return true;
    };
    if pid == std::process::id() {
        return false;
    }
    if !proc_root.is_dir() {
        return true;
    }
    !proc_root.join(pid.to_string()).exists()
}

/// Trava um `Mutex` sem entrar em pânico se outra thread já falhou com ele.
/// Os dados são devolvidos mesmo assim; o estado protegido é sempre válido para
/// leitura nos pontos em que o app usa estes locks.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| {
        tracing::warn!("mutex envenenado por uma falha anterior; seguindo com os dados atuais");
        poisoned.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn state_dir_prefers_xdg_then_home_then_local_app_data() {
        let xdg = Some(PathBuf::from("/x/state"));
        assert_eq!(
            state_dir_from(xdg, Some("/h".into()), None),
            PathBuf::from("/x/state/gitatlas")
        );
        assert_eq!(
            state_dir_from(Some(PathBuf::new()), Some("/h".into()), None),
            PathBuf::from("/h/.local/state/gitatlas")
        );
        assert_eq!(
            state_dir_from(None, None, Some("/la".into())),
            PathBuf::from("/la/gitatlas")
        );
        assert_eq!(
            state_dir_from(None, None, None),
            std::env::temp_dir().join("gitatlas")
        );
    }

    #[test]
    fn crash_report_round_trips_and_dismiss_clears_it() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_crash(dir.path()), None);
        write_crash(dir.path(), "boom\nlocal: a.rs:1").unwrap();
        assert_eq!(
            read_crash(dir.path()).unwrap().details,
            "boom\nlocal: a.rs:1"
        );
        let d = Diagnostics {
            _log: None,
            dir: dir.path().to_path_buf(),
            unclean_previous_exit: false,
        };
        d.dismiss_crash();
        assert_eq!(read_crash(dir.path()), None);
    }

    #[test]
    fn stale_marker_detection() {
        let procs = tempfile::tempdir().unwrap();
        let live = std::process::id();
        // Nosso próprio PID nunca é obsoleto.
        assert!(!is_stale_session(&live.to_string(), procs.path()));
        // PID alheio sem entrada em /proc: sessão anterior caiu.
        assert!(is_stale_session("4242", procs.path()));
        // PID alheio com entrada em /proc: outra instância ainda roda.
        fs::create_dir(procs.path().join("4242")).unwrap();
        assert!(!is_stale_session("4242", procs.path()));
        // Conteúdo ilegível conta como obsoleto.
        assert!(is_stale_session("lixo", procs.path()));
        // Sem /proc não dá para verificar: o marcador vale.
        assert!(is_stale_session("4242", &procs.path().join("missing")));
    }

    #[test]
    fn claim_session_reports_unclean_exit_once() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SESSION_FILE), "999999999").unwrap();
        let unclean = claim_session(dir.path());
        // Em Linux o PID inexistente conta como queda; em outros SOs o marcador também vale.
        assert!(unclean);
        // Depois de reclamar, o marcador passa a ser o desta sessão.
        assert!(!claim_session(dir.path()));
    }

    #[test]
    fn poisoned_mutex_is_recovered() {
        let m = Arc::new(Mutex::new(7));
        let m2 = Arc::clone(&m);
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("falha de propósito");
        })
        .join();
        assert!(m.is_poisoned());
        assert_eq!(*lock(&m), 7);
    }
}
