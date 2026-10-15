// SPDX-License-Identifier: Apache-2.0
// Dev-only: lets the UI run in a plain browser with IPC answered from fixture
// dumps (crates/ak-core/examples/dump_fixture.rs) in public/mock/<name>.json.
// Several repos can be open at once (one per repoId), like real tabs.
// Never bundled in production builds (guarded by import.meta.env.DEV in main.tsx).
import { mockIPC } from "@tauri-apps/api/mocks";

interface Fixture { open: any; rows: any[]; window: number[]; details: Record<string, any>; diffs: Record<string, string> }

const uiError = (code: string, message: string) => Object.assign(new Error(message), { code });

export async function installDevMock(first: string, next = "hubbi") {
  const repos = new Map<number, Fixture>();
  let nextId = 0;
  const load = async (name: string) => (await (await fetch(`/mock/${name}.json`)).json()) as Fixture;
  const emptyWindow = (start: number) => {
    const b = new DataView(new ArrayBuffer(16));
    [65, 75, 82, 87].forEach((c, i) => b.setUint8(i, c));
    b.setUint16(4, 1, true);
    b.setUint32(6, start, true);
    return b.buffer;
  };
  const repo = (id: number) => {
    const r = repos.get(id);
    if (!r) throw { code: "UNKNOWN_REPO", message: String(id) };
    return r;
  };
  // Working-directory state for the first repo, mutated by the mocked operations.
  const wt = [
    { path: "src/app.ts", index: ".", worktree: "M" },
    { path: "README.md", index: "M", worktree: "." },
    { path: "notes/todo.txt", index: ".", worktree: "?", untracked: true },
  ];
  const entry = (e: any) => ({ path: e.path, orig_path: null, index: e.index, worktree: e.worktree, conflicted: false, untracked: !!e.untracked });
  const demoDiff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n@@ -20,2 +20,3 @@\n x\n+added\n y\n";
  mockIPC(async (cmd, args: any) => {
    switch (cmd) {
      case "get_initial_path": return `/mock/${first}`;
      case "plugin:dialog|open": return `/mock/${next}`;
      case "repo_open": {
        const data = await load(String(args.path).replace("/mock/", ""));
        const repoId = ++nextId;
        repos.set(repoId, data);
        return { ...data.open, repoId };
      }
      case "repo_close": repos.delete(args.repoId); return null;
      case "graph_rows": return repo(args.repoId).rows.slice(args.start, args.start + args.count);
      case "graph_window":
        return args.start === 0 ? new Uint8Array(repo(args.repoId).window).buffer : emptyWindow(args.start);
      case "commit_details": return repo(args.repoId).details[args.sha];
      case "diff_file": return repo(args.repoId).diffs[`${args.sha}|${args.path}`] ?? "";
      case "repo_refresh": return { ...repo(args.repoId).open, repoId: args.repoId };
      case "status_get": return { branch: repo(args.repoId).open.branch, oid: repo(args.repoId).open.head, entries: wt.map(entry) };
      case "diff_worktree": return args.untracked ? demoDiff.replace("@@ -1,3 +1,3 @@", "@@ -0,0 +1,3 @@") : demoDiff;
      case "op_stage": case "op_unstage": {
        for (const e of wt) if (args.paths.includes(e.path)) {
          if (cmd === "op_stage") { e.index = e.untracked ? "A" : "M"; e.worktree = "."; delete (e as any).untracked; } else { e.worktree = e.index; e.index = "."; }
        }
        return { id: null, head: null };
      }
      case "op_commit":
        if (!args.message.trim()) throw uiError("EMPTY_MESSAGE", "empty");
        if (!wt.some((e) => e.index !== ".")) throw uiError("NOTHING_TO_COMMIT", "none");
        for (const e of wt) e.index = ".";
        return { id: "demo-undo", head: null };
      case "op_discard": for (const p of args.paths) wt.splice(wt.findIndex((e) => e.path === p), 1); return { id: "d1", head: null };
      case "op_stage_hunk": return { id: null, head: null };
      case "op_redo": throw uiError("NOTHING_TO_REDO", "none");
      case "op_undo": return { id: null, head: null };
      case "journal_list": return [];
      case "journal_prune": return 0;
      default: throw { code: "UNKNOWN_COMMAND", message: cmd };
    }
  });
}
