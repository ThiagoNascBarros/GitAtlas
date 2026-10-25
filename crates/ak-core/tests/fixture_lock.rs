// SPDX-License-Identifier: Apache-2.0
//! QA-01: pedidos concorrentes do mesmo fixture ausente não podem se atrapalhar.

use std::fs;
use std::thread;

mod common;
use common::{fixture, workspace};

#[test]
fn concurrent_requests_for_a_missing_fixture_all_get_a_complete_repo() {
    let out = workspace().join("tests/fixtures/out");
    // Remove o fixture e o marcador para forçar a construção sob concorrência.
    let _ = fs::remove_dir_all(out.join("linear"));
    let _ = fs::remove_file(out.join(".linear.ready"));

    let handles: Vec<_> = (0..6)
        .map(|_| {
            thread::spawn(|| {
                let dir = fixture("linear");
                // Cada pedido deve ver um repositório completo, nunca um em construção.
                assert!(
                    dir.join(".git").exists(),
                    "fixture incompleto: {}",
                    dir.display()
                );
                dir
            })
        })
        .collect();
    for h in handles {
        h.join().expect("pedido de fixture falhou");
    }
    assert!(out.join(".linear.ready").exists());
    assert!(!out.join(".linear.lock").exists(), "trava não foi liberada");
}
