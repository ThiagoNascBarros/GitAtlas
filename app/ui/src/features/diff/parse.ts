// SPDX-License-Identifier: Apache-2.0
// Unified diff → hunks/lines (pure, unit-tested).

export type LineKind = "ctx" | "add" | "del";
export interface DiffLine { kind: LineKind; text: string; oldNo: number | null; newNo: number | null }
export interface Hunk { header: string; lines: DiffLine[] }
export interface ParsedDiff { binary: boolean; hunks: Hunk[] }

export function parseUnified(patch: string): ParsedDiff {
  const hunks: Hunk[] = [];
  let cur: Hunk | null = null;
  let oldNo = 0, newNo = 0;
  let binary = false;
  for (const raw of patch.split("\n")) {
    if (raw.startsWith("Binary files ")) binary = true;
    const m = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(raw);
    if (m) {
      cur = { header: raw, lines: [] };
      hunks.push(cur);
      oldNo = Number(m[1]);
      newNo = Number(m[2]);
      continue;
    }
    if (!cur) continue;
    const c = raw[0];
    if (c === "+") cur.lines.push({ kind: "add", text: raw.slice(1), oldNo: null, newNo: newNo++ });
    else if (c === "-") cur.lines.push({ kind: "del", text: raw.slice(1), oldNo: oldNo++, newNo: null });
    else if (c === " ") cur.lines.push({ kind: "ctx", text: raw.slice(1), oldNo: oldNo++, newNo: newNo++ });
    // "\ No newline at end of file" and trailing empty line are ignored.
  }
  return { binary, hunks };
}

export interface SplitRow { left: DiffLine | null; right: DiffLine | null }

/** Pairs deletions with following additions for side-by-side view. */
export function toSplit(h: Hunk): SplitRow[] {
  const out: SplitRow[] = [];
  let dels: DiffLine[] = [], adds: DiffLine[] = [];
  const flush = () => {
    for (let i = 0; i < Math.max(dels.length, adds.length); i++) out.push({ left: dels[i] ?? null, right: adds[i] ?? null });
    dels = [];
    adds = [];
  };
  for (const l of h.lines) {
    if (l.kind === "del") {
      if (adds.length) flush();
      dels.push(l);
    } else if (l.kind === "add") adds.push(l);
    else {
      flush();
      out.push({ left: l, right: l });
    }
  }
  flush();
  return out;
}
