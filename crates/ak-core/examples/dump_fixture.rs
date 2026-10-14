// SPDX-License-Identifier: Apache-2.0
//! Dumps the read-only command responses for one repo as JSON, so the UI can
//! be exercised in a plain browser with mocked IPC (dev only).
//! Usage: cargo run -p ak-core --example dump_fixture -- <repo> > out.json

use ak_core::graph::{self, Topology};
use ak_core::read::{CliReader, RepoReader};
use ak_gitcli::{Git, Mode};
use serde_json::json;

fn main() {
    let path = std::env::args().nth(1).expect("repo path");
    let reader = CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), path.as_ref()).unwrap();
    let commits = reader.commits().unwrap();
    let refs = reader.refs().unwrap();
    let status = reader.status().unwrap();
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let layout = graph::layout(&topo);
    let window = graph::encode_window(&topo, &layout, 0, commits.len() as u32);
    let mut details = serde_json::Map::new();
    let mut diffs = serde_json::Map::new();
    for c in &commits {
        let d = reader.commit_details(&c.sha).unwrap();
        for f in &d.files {
            diffs.insert(format!("{}|{}", c.sha, f.path), json!(reader.file_diff(&c.sha, &f.path).unwrap()));
        }
        details.insert(c.sha.clone(), serde_json::to_value(d).unwrap());
    }
    let row_of: std::collections::HashMap<_, _> = commits.iter().enumerate().map(|(i, c)| (c.sha.clone(), i)).collect();
    let refs: Vec<_> = refs
        .iter()
        .map(|r| json!({ "name": r.short, "kind": r.kind, "row": row_of.get(&r.target), "isHead": r.is_head }))
        .collect();
    let out = json!({
        "open": {
            "repoId": 1, "root": reader.root(), "name": reader.root().file_name().unwrap().to_string_lossy(),
            "head": reader.head().unwrap(), "branch": status.branch, "rowCount": commits.len(),
            "laneCount": layout.lane_count, "refs": refs, "gitVersion": "mock", "readOnlyReason": null
        },
        "rows": commits.iter().map(|c| json!({"sha": c.sha, "summary": c.summary, "author": c.author, "email": c.email, "time": c.time})).collect::<Vec<_>>(),
        "window": window,
        "details": details,
        "diffs": diffs,
    });
    println!("{out}");
}
