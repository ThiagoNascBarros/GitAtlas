// SPDX-License-Identifier: Apache-2.0
//! Graph layout against fixture repos (golden files) and random DAGs.
//! Fixtures are built by `tests/fixtures/make.sh`; missing ones are built on demand.

use std::path::{Path, PathBuf};
use std::process::Command;

use ak_core::graph::{self, verify, Layout, Topology};
use ak_core::read::{CliReader, RepoReader};
use ak_gitcli::{Git, Mode};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn fixture(name: &str) -> PathBuf {
    let dir = workspace().join("tests/fixtures/out").join(name);
    if !dir.exists() {
        let ok = Command::new(workspace().join("tests/fixtures/make.sh")).arg(name).status().unwrap().success();
        assert!(ok, "failed to build fixture {name}");
    }
    dir
}

fn load(name: &str) -> (Vec<ak_gitcli::parse::CommitRecord>, Topology, Layout) {
    let reader = CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), &fixture(name)).unwrap();
    let commits = reader.commits().unwrap();
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let lay = graph::layout(&topo);
    (commits, topo, lay)
}

/// Human-readable dump: one line per row, e.g. `1 |o  | Merge x | in 0>1 pass 2 out 1>1`.
fn dump(commits: &[ak_gitcli::parse::CommitRecord], l: &Layout) -> String {
    let mut s = String::new();
    for (i, c) in commits.iter().enumerate() {
        let segs: Vec<String> = l
            .segments_of(i as u32)
            .iter()
            .map(|g| format!("{:?} {}>{}", g.kind, g.from, g.to).to_lowercase())
            .collect();
        s.push_str(&format!("{:>3} lane {} | {} | {}\n", i, l.lane[i], c.summary, segs.join(", ")));
    }
    s
}

fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap();
    assert_eq!(expected, actual, "golden mismatch for {name}; rerun with UPDATE_GOLDEN=1 after review");
}

#[test]
fn fixtures_layout_is_valid_and_matches_golden() {
    for name in ["linear", "merges", "octopus", "criss-cross", "stash-wip", "demo"] {
        let (commits, topo, lay) = load(name);
        verify::check(&topo, &lay).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut shas: Vec<_> = commits.iter().map(|c| &c.sha).collect();
        shas.sort();
        shas.dedup();
        assert_eq!(shas.len(), commits.len(), "{name}: duplicate commits");
        golden(name, &dump(&commits, &lay));
    }
}

#[test]
fn fixture_shapes() {
    let (c, _, l) = load("linear");
    assert_eq!(c.len(), 5);
    assert_eq!(l.lane_count, 1);
    let (c, t, _) = load("octopus");
    assert_eq!(t.parents_of(0).len(), 4, "{}", c[0].summary);
    let (c, _, _) = load("stash-wip");
    assert_eq!(c.len(), 2, "stash commits must not be part of the graph");
}

#[test]
fn commit_details_and_diff() {
    let reader = CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), &fixture("demo")).unwrap();
    let commits = reader.commits().unwrap();
    let edges = commits.iter().find(|c| c.summary == "Draw graph edges").unwrap();
    let d = reader.commit_details(&edges.sha).unwrap();
    assert_eq!(d.message, "Draw graph edges");
    assert_eq!(d.files.len(), 1);
    assert_eq!((d.files[0].status, d.files[0].path.as_str()), ('M', "src/graph.txt"));
    let patch = reader.file_diff(&edges.sha, "src/graph.txt").unwrap();
    assert!(patch.contains("@@") && patch.contains("+edges"), "{patch}");
    // Merge commits list changes against the first parent.
    let merge = commits.iter().find(|c| c.summary == "Merge feature/graph").unwrap();
    assert!(!reader.commit_details(&merge.sha).unwrap().files.is_empty());
    // Root commit has no parent but still shows its files.
    let root = commits.last().unwrap();
    assert_eq!(reader.commit_details(&root.sha).unwrap().files[0].status, 'A');
    // Option injection is rejected.
    assert!(reader.commit_details("--output=/tmp/x").is_err());
}

/// Small deterministic PRNG so the test needs no extra crates.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn random_dags_have_valid_layouts() {
    let mut rng = Lcg(42);
    for case in 0..300 {
        let n = 2 + (rng.next() % 120) as usize;
        // Row i's parents are drawn from rows below it (display order).
        let shas: Vec<String> = (0..n).map(|i| format!("{i:040}")).collect();
        let parents: Vec<Vec<String>> = (0..n)
            .map(|i| {
                let below = n - 1 - i;
                let k = if below == 0 { 0 } else { (rng.next() % 4).min(below as u64) as usize };
                let mut ps: Vec<String> = (0..k).map(|_| shas[i + 1 + (rng.next() as usize % below)].clone()).collect();
                ps.dedup();
                ps
            })
            .collect();
        let topo = Topology::from_commits(shas.iter().map(String::as_str).zip(parents.iter().map(Vec::as_slice)));
        let lay = graph::layout(&topo);
        verify::check(&topo, &lay).unwrap_or_else(|e| panic!("case {case} (n={n}): {e}"));
    }
}

#[test]
fn window_encoding_roundtrip_header() {
    let (_, topo, lay) = load("demo");
    let w = graph::encode_window(&topo, &lay, 2, 5);
    assert_eq!(&w[0..4], b"AKRW");
    assert_eq!(u32::from_le_bytes(w[6..10].try_into().unwrap()), 2);
    assert_eq!(u32::from_le_bytes(w[10..14].try_into().unwrap()), 5);
    let segs: usize = (2..7).map(|r| lay.segments_of(r).len()).sum();
    assert_eq!(w.len(), 16 + 5 * 16 + segs * 8);
    // Window past the end is clamped.
    let tail = graph::encode_window(&topo, &lay, 1000, 10);
    assert_eq!(u32::from_le_bytes(tail[10..14].try_into().unwrap()), 0);
}

/// NFR-1: 100k-commit layout ≤ 400 ms (release). Run with:
/// `cargo test --release -p ak-core --test graph -- --ignored big_100k`
#[test]
#[ignore]
fn big_100k_layout_time() {
    let (commits, _, _) = load("big-100k");
    let t0 = std::time::Instant::now();
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let lay = graph::layout(&topo);
    let ms = t0.elapsed().as_millis();
    verify::check(&topo, &lay).unwrap();
    let bytes = topo.parents.len() * 4 + topo.parent_start.len() * 4 + lay.lane.len() * 4 + lay.segments.len() * 8 + lay.seg_start.len() * 4;
    println!("100k: topology+layout {ms} ms, {} bytes/commit, {} lanes", bytes / commits.len(), lay.lane_count);
    assert_eq!(commits.len(), 100_000);
    assert!(ms <= 400, "layout took {ms} ms");
    assert!(bytes / commits.len() <= 120);
}
