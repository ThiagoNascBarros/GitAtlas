// SPDX-License-Identifier: Apache-2.0
// Dev-only: lets the UI run in a plain browser with IPC answered from a
// fixture dump (crates/ak-core/examples/dump_fixture.rs). Never bundled in
// production builds (guarded by import.meta.env.DEV in main.tsx).
import { mockIPC } from "@tauri-apps/api/mocks";

export async function installDevMock(fixture: string) {
  const data = await (await fetch(`/mock/${fixture}.json`)).json();
  const windowBuf = new Uint8Array(data.window).buffer;
  const emptyWindow = (start: number) => {
    const b = new DataView(new ArrayBuffer(16));
    [65, 75, 82, 87].forEach((c, i) => b.setUint8(i, c));
    b.setUint16(4, 1, true);
    b.setUint32(6, start, true);
    return b.buffer;
  };
  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case "get_initial_path": return "/mock/" + fixture;
      case "repo_open": return data.open;
      case "repo_close": return null;
      case "graph_rows": return data.rows.slice(args.start, args.start + args.count);
      case "graph_window": return args.start === 0 ? windowBuf : emptyWindow(args.start);
      case "commit_details": return data.details[args.sha];
      case "diff_file": return data.diffs[`${args.sha}|${args.path}`] ?? "";
      case "plugin:dialog|open": return null;
      default: throw { code: "UNKNOWN_COMMAND", message: cmd };
    }
  });
}
