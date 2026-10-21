// SPDX-License-Identifier: Apache-2.0
//! T1.4: the gix reader must return exactly what the CLI reader returns.
//! Run: cargo test -p ak-core --features gix-reader --test gix_parity
#![cfg(feature = "gix-reader")]

use std::time::Instant;

use ak_core::read::{CliReader, RepoReader};
use ak_core::read_gix::GixReader;
use ak_gitcli::{Git, Mode};

mod common;
use common::fixture;

fn compare(name: &str) {
    let git = Git::discover(Mode::ReadOnly).unwrap();
    let cli = CliReader::open(git.clone(), &fixture(name)).unwrap();
    let gix = GixReader::open(git, &fixture(name)).unwrap();
    let t = Instant::now();
    let a = cli.commits().unwrap();
    let cli_ms = t.elapsed().as_millis();
    let t = Instant::now();
    let b = gix.commits().unwrap();
    let gix_ms = t.elapsed().as_millis();
    println!("{name}: {} commits, cli {cli_ms} ms, gix {gix_ms} ms", a.len());
    assert_eq!(a.len(), b.len(), "{name}: commit count");
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!(x, y, "{name}: row {i} differs");
    }
    assert_eq!(cli.refs().unwrap(), gix.refs().unwrap(), "{name}: refs");
}

#[test]
fn small_fixtures_match_cli() {
    for name in ["linear", "merges", "octopus", "criss-cross", "stash-wip", "demo"] {
        compare(name);
    }
}

#[test]
fn big_fixtures_match_cli() {
    compare("big-10k");
}

#[test]
#[ignore]
fn wide_100k_matches_cli() {
    compare("wide-100k");
}
