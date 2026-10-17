// SPDX-License-Identifier: Apache-2.0
//! Helpers compartilhados pelos testes de integração do ak-core.
#![allow(dead_code)]

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

const LOCK_TIMEOUT: Duration = Duration::from_secs(300);

/// Raiz do workspace (onde ficam `tests/fixtures`).
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Caminho de um fixture, construído por `tests/fixtures/make.sh` se ainda não existir.
///
/// Testes rodam em paralelo e vários pedem o mesmo fixture ao mesmo tempo. Sem trava,
/// um deles roda `make.sh` enquanto outro já está construindo o mesmo nome, e o
/// `rm -rf` do primeiro apaga o repositório que o segundo está usando (QA-01).
/// Por isso a construção é serializada por nome com um arquivo de trava criado com
/// `create_new` (atômico entre processos), e a conclusão é marcada por um arquivo
/// `.<nome>.ready` fora da pasta do fixture, para não aparecer como não rastreado.
pub fn fixture(name: &str) -> PathBuf {
    let out = workspace().join("tests/fixtures/out");
    let dir = out.join(name);
    let ready = out.join(format!(".{name}.ready"));
    if ready.exists() {
        return dir;
    }
    fs::create_dir_all(&out).unwrap();
    let _lock = Lock::acquire(&out.join(format!(".{name}.lock")));
    // Outro teste pode ter terminado a construção enquanto esperávamos a trava.
    if !ready.exists() {
        let make = workspace().join("tests/fixtures/make.sh");
        let ok = Command::new(make).arg(name).status().unwrap().success();
        assert!(ok, "failed to build fixture {name}");
        fs::write(&ready, b"").unwrap();
    }
    dir
}

/// Trava de arquivo: existe enquanto o dono estiver construindo o fixture.
struct Lock(PathBuf);

impl Lock {
    fn acquire(path: &Path) -> Lock {
        let start = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(_) => return Lock(path.to_path_buf()),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    assert!(
                        start.elapsed() < LOCK_TIMEOUT,
                        "fixture lock {} still held after {:?}; if no test is running, delete it",
                        path.display(),
                        LOCK_TIMEOUT
                    );
                    sleep(Duration::from_millis(50));
                }
                Err(e) => panic!("cannot create fixture lock {}: {e}", path.display()),
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
