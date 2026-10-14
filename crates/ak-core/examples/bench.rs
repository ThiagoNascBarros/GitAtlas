// SPDX-License-Identifier: Apache-2.0
//! Phase 1 benchmark (roadmap T1.11): repo-open path timings and memory against
//! PRD NFR-1/NFR-2. Usage:
//!   cargo run --release -p ak-core --example bench -- <repo> [report.json]
//! UI paint/scroll fps are measured separately (needs a display).

use std::time::Instant;

use ak_core::graph::{self, verify, Topology};
use ak_core::read::{CliReader, RepoReader};
use ak_gitcli::{Git, Mode};
use serde_json::json;

fn vm_hwm_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM:")).map(|l| l.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0)))
        .unwrap_or(0)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let repo = args.next().expect("repo path");
    let report_path = args.next();

    let t = Instant::now();
    let reader = CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), repo.as_ref()).unwrap();
    let commits = reader.commits().unwrap();
    let refs = reader.refs().unwrap();
    let read_ms = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let topo_ms = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let layout = graph::layout(&topo);
    let layout_ms = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let first_window = graph::encode_window(&topo, &layout, 0, 200);
    let window_ms = t.elapsed().as_secs_f64() * 1e3;
    // Worst case for scrolling: a window right before a checkpoint boundary at the end.
    let t = Instant::now();
    let last = (commits.len() as u32).saturating_sub(201);
    let _ = graph::encode_window(&topo, &layout, last, 200);
    let deep_window_ms = t.elapsed().as_secs_f64() * 1e3;

    verify::check(&topo, &layout).expect("layout must be valid");

    let n = commits.len().max(1);
    let graph_bytes = topo.parents.len() * 4 + topo.parent_start.len() * 4 + topo.has_missing_parent.len() + layout.heap_bytes();
    let rss_mb = vm_hwm_kb() as f64 / 1024.0;
    // NFR-1: core layout ≤ 400 ms; first 200 rows ready ≤ 300 ms after the commit list is in memory.
    // NFR-2: ≤ 120 bytes/commit for graph topology+layout.
    let checks = json!({
        "nfr1_layout_le_400ms": (topo_ms + layout_ms) <= 400.0,
        "nfr1_first_window_le_300ms": (topo_ms + layout_ms + window_ms) <= 300.0,
        "nfr2_graph_bytes_per_commit_le_120": graph_bytes / n <= 120,
    });
    let report = json!({
        "repo": repo,
        "commits": commits.len(),
        "refs": refs.len(),
        "lanes": layout.lane_count,
        "timings_ms": { "git_read": read_ms, "topology": topo_ms, "layout": layout_ms, "first_window": window_ms, "deep_window": deep_window_ms },
        "graph_bytes_per_commit": graph_bytes / n,
        "first_window_bytes": first_window.len(),
        "peak_rss_mb_core_process": rss_mb,
        "checks": checks,
        "not_measured": ["UI first paint", "UI scroll fps", "app idle RSS (shell + WebKit)"],
    });
    let text = serde_json::to_string_pretty(&report).unwrap();
    println!("{text}");
    if let Some(p) = report_path {
        std::fs::write(p, text + "\n").unwrap();
    }
}
