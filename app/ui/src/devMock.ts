// SPDX-License-Identifier: Apache-2.0
// Dev-only: lets the UI run in a plain browser with IPC answered from fixture
// dumps (crates/ak-core/examples/dump_fixture.rs) in public/mock/<name>.json.
// Several repos can be open at once (one per repoId), like real tabs.
// Never bundled in production builds (guarded by import.meta.env.DEV in main.tsx).
import { mockIPC } from "@tauri-apps/api/mocks";

interface Fixture { open: any; rows: any[]; window: number[]; details: Record<string, any>; diffs: Record<string, string> }

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
      default: throw { code: "UNKNOWN_COMMAND", message: cmd };
    }
  });
}
