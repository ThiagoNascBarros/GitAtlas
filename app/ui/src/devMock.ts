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
  // ?files=N: a large working directory, to check the panel with many changes.
  const many = Number(new URLSearchParams(location.search).get("files") ?? 0);
  for (let i = 0; i < many; i++) wt.push({ path: `src/module${i % 7}/file${i}.ts`, index: i % 3 === 0 ? "M" : ".", worktree: i % 3 === 0 ? "." : "M" });
  const entry = (e: any) => ({ path: e.path, orig_path: null, index: e.index, worktree: e.worktree, conflicted: false, untracked: !!e.untracked });
  const demoDiff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n@@ -20,2 +20,3 @@\n x\n+added\n y\n";
  // Local stashes and the Shared stash tab (two remotes, one that rejects custom refs), mutated by the mocked ops.
  const hex = (n: number) => n.toString(16).padStart(40, "0");
  const localStashes = [{ index: 0, sha: hex(0xa11ce), message: "WIP on feature/shared-stash: share dialog and tab", time: Math.floor(Date.now() / 1000) - 5400 }];
  const me = { name: "Thiago Barros", email: "thiago@example.org" };
  let ssSeq = 0, ssMode = "panel";
  const ssRemotes: { name: string; probe: string | null }[] = [{ name: "origin", probe: "ok" }, { name: "upstream", probe: "ok" }, { name: "review", probe: "rejected" }];
  let shared: any[] | null = null;
  const kept: any[] = [];
  const sec = () => Math.floor(Date.now() / 1000);
  // Shas come from the open fixture so the graph glyph and the read-only diff have real commits to show.
  const ssSeed = () => {
    const fx = repos.values().next().value as Fixture;
    const small = Object.values(fx.details).filter((d: any) => d.files.length >= 2 && d.files.length <= 6).map((d: any) => d.sha as string);
    const mk = (id: string, remote: string, name: string, email: string, title: string, row: number, branch: string, ago: number, extra: object = {}) => ({
      remote, slug: name.toLowerCase().replace(/\W+/g, "-"), id, ref_name: `refs/atlasgit/shared-stash/x/${id}`, wrapper: hex(ssSeq + 100), stash: small[ssSeq++ % small.length],
      title, author_name: name, author_email: email, base: fx.rows[row].sha, base_branch: branch, created: sec() - ago, updated: sec() - ago,
      has_untracked: false, mine: email === me.email, verified: false, valid: true, invalid_reason: null, ...extra,
    });
    return [
      mk("t1", "origin", "Maria Souza", "maria@example.org", "Fix flaky retry in the sync worker", 3, "develop", 3 * 3600),
      mk("t2", "origin", "Jon Okafor", "jon@example.org", "Experiment: batch ref updates", 6, "", 2 * 86400, { valid: false, invalid_reason: "parent is not a stash commit" }),
      mk("t3", "upstream", "Ana Lima", "ana@example.org", "Spike: virtualised file tree", 1, "main", 26 * 3600, { verified: true, has_untracked: true }),
    ];
  };
  const ss = () => (shared ??= ssSeed());
  const offline = (r: string | null) => (window as any).__mockOffline === r;
  const find = (a: any) => {
    const x = ss().find((i) => i.id === a.id && i.remote === a.remote);
    if (!x) throw uiError("SHARED_STASH_GONE", "gone");
    return x;
  };
  const mine = (a: any) => {
    const x = find(a);
    if (!x.mine) throw uiError("NOT_AUTHOR", "not author");
    return x;
  };
  const ssOut = (extra: object = {}) => ({ id: null, head: null, shared_stash: { id: null, remote: null, large_bytes: null, index_dropped: false, fetched: [], ...extra } });
  const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));
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
      case "stash_list": return localStashes;
      case "op_stash":
        if (args.action === "push") localStashes.unshift({ index: 0, sha: hex(0xb0b + localStashes.length), message: args.message || "WIP on feature/shared-stash", time: sec() });
        else if (args.action !== "apply") localStashes.splice(args.index, 1);
        localStashes.forEach((x, i) => (x.index = i));
        return { id: null, head: null };
      case "shared_stash_list": return { stashes: [...ss()], kept: [...kept], remotes: ssRemotes.map((r) => ({ ...r })), defaultRemote: "origin", fetchMode: ssMode };
      case "shared_stash_fetch": {
        await delay(450);
        const names = args.remote ? [args.remote] : ssRemotes.map((r) => r.name);
        const fetched = names.map((remote: string) => ({ remote, error: offline(remote) ? { code: "NETWORK_FAILED", message: "Could not reach the remote." } : null }));
        return ssOut({ fetched });
      }
      case "shared_stash_publish": {
        const st = localStashes.find((x) => x.sha === args.stashSha);
        if (!st) throw uiError("STASH_NOT_FOUND", "no stash");
        const remote = args.remote ?? "origin";
        if (ssRemotes.find((r) => r.name === remote)?.probe === "rejected") throw uiError("REMOTE_REJECTED", "rejected");
        const fx = repos.values().next().value as Fixture;
        const id = `m${++ssSeq}`;
        const small = Object.values(fx.details).filter((d: any) => d.files.length >= 2 && d.files.length <= 6).map((d: any) => d.sha as string);
        ss().unshift({ remote, slug: "thiago-barros", id, ref_name: `refs/atlasgit/shared-stash/thiago-barros/${id}`, wrapper: hex(ssSeq + 100), stash: small[ssSeq % small.length],
          title: args.title, author_name: me.name, author_email: me.email, base: fx.rows[2 + (ssSeq % 3)].sha, base_branch: "feature/shared-stash", created: sec(), updated: sec(),
          has_untracked: false, mine: true, verified: false, valid: true, invalid_reason: null });
        return ssOut({ id, remote, large_bytes: /large/i.test(args.title) ? 12_582_912 : null });
      }
      case "shared_stash_update": {
        const x = mine(args);
        if (!args.stashSha && !args.title) throw uiError("NOTHING_TO_UPDATE", "nothing");
        if (args.title) x.title = args.title;
        x.updated = sec();
        return ssOut();
      }
      case "shared_stash_unshare": {
        const x = mine(args);
        shared = ss().filter((i) => i !== x);
        kept.unshift({ id: x.id, remote: x.remote, slug: x.slug, wrapper: x.wrapper, stash: x.stash, title: x.title, base_branch: x.base_branch, created: x.created, updated: x.updated, unshared_at: sec() });
        return ssOut();
      }
      case "shared_stash_reshare": {
        const i = kept.findIndex((k) => k.id === args.id);
        if (i < 0) throw uiError("STASH_NOT_FOUND", "no keep ref");
        const [k] = kept.splice(i, 1);
        const fx = repos.values().next().value as Fixture;
        ss().unshift({ ...k, ref_name: "", author_name: me.name, author_email: me.email, base: fx.rows[2].sha, has_untracked: false, mine: true, verified: false, valid: true, invalid_reason: null, remote: args.remote });
        return ssOut();
      }
      case "shared_stash_discard_keep": kept.splice(kept.findIndex((k) => k.id === args.id), 1); return ssOut();
      case "shared_stash_apply": {
        const x = find(args);
        if (!x.valid) throw uiError("INVALID_SHARED_STASH", "invalid");
        return ssOut();
      }
      case "shared_stash_branch": {
        const x = find(args);
        if (!x.valid) throw uiError("INVALID_SHARED_STASH", "invalid");
        return { id: "sb1", head: null, shared_stash: null };
      }
      case "shared_stash_probe_reset": for (const r of ssRemotes) if (!args.remote || r.name === args.remote) r.probe = null; return null;
      case "shared_stash_tab_open": case "shared_stash_cancel": return null;
      case "shared_stash_settings_get": case "shared_stash_settings_set":
        if (cmd.endsWith("set")) ssMode = args.mode ?? "panel";
        return { effective: ssMode, repo: ssMode, global: null };
      case "merge_preview": return { up_to_date: false, fast_forward: false, conflicts: [] };
      case "conflict_versions": return {
        base: "a\n", ours: "mine\n", theirs: "yours\n",
        merged: "<<<<<<< HEAD\nmine\n=======\nyours\n>>>>>>> topic\n",
        merged_diff3: "<<<<<<< HEAD\nmine\n||||||| base\na\n=======\nyours\n>>>>>>> topic\n",
      };
      case "open_external": return null;
      case "op_history": return { id: "h1", head: null };
      case "op_net": return { id: null, head: null };
      case "credentials_answer": return null;
      case "rebase_todo": return [{ sha: "aaaaaaa1", action: "pick", message: "First change" }, { sha: "bbbbbbb2", action: "pick", message: "Second change" }, { sha: "ccccccc3", action: "pick", message: "Third change" }];
      case "op_rebase": case "op_rebase_continue": return { id: "r1", head: null };
      case "op_merge": case "op_resolve_with": case "op_merge_abort": case "op_resolve":
      case "branch_plan": return { name: "feature/ACT-1010-Nova-branch", base: "develop" };
      case "op_branch_create": case "op_branch_create_from": case "op_branch_rename": case "op_branch_delete": case "op_checkout":
        return { id: null, head: null };
      default: throw { code: "UNKNOWN_COMMAND", message: cmd };
    }
  });
}
