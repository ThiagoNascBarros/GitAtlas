// SPDX-License-Identifier: Apache-2.0
// Changed-file list as a flat path list or a directory tree (pure, unit-tested).

export interface FileItem { status: string; path: string }
export type TreeRow =
  | { kind: "dir"; name: string; depth: number; path: string }
  | { kind: "file"; name: string; depth: number; file: FileItem };

/** Flattens files into tree rows: directories first, then files, both alphabetical. */
export function toTreeRows(files: FileItem[]): TreeRow[] {
  interface Node { dirs: Map<string, Node>; files: FileItem[] }
  const root: Node = { dirs: new Map(), files: [] };
  for (const f of files) {
    const parts = f.path.split("/");
    let n = root;
    for (const p of parts.slice(0, -1)) {
      if (!n.dirs.has(p)) n.dirs.set(p, { dirs: new Map(), files: [] });
      n = n.dirs.get(p)!;
    }
    n.files.push(f);
  }
  const rows: TreeRow[] = [];
  const walk = (n: Node, depth: number, prefix: string) => {
    for (const name of [...n.dirs.keys()].sort()) {
      // Collapse single-child directory chains: "src/app" instead of "src" › "app".
      let label = name, child = n.dirs.get(name)!, full = prefix + name;
      while (child.files.length === 0 && child.dirs.size === 1) {
        const [only] = child.dirs.keys();
        label += "/" + only;
        full += "/" + only;
        child = child.dirs.get(only)!;
      }
      rows.push({ kind: "dir", name: label, depth, path: full });
      walk(child, depth + 1, full + "/");
    }
    for (const f of [...n.files].sort((a, b) => a.path.localeCompare(b.path)))
      rows.push({ kind: "file", name: f.path.split("/").pop()!, depth, file: f });
  };
  walk(root, 0, "");
  return rows;
}
