// SPDX-License-Identifier: Apache-2.0
// Conflict markers written by `git merge` (and the diff3 base section `|||||||`), plus the
// per-line choices the conflict editor keeps for each block. Pure functions: no DOM, no I/O.

export interface Plain { kind: "text"; lines: string[] }
/**
 * One `<<<<<<<` ... `>>>>>>>` block. `base` is set only in diff3 style. `start` and `end` are the
 * full marker lines (with their labels), kept so unresolved blocks are written back unchanged.
 */
export interface Conflict { kind: "conflict"; start: string; end: string; ours: string[]; base: string[] | null; theirs: string[] }
export type Segment = Plain | Conflict;

/** What the user kept from one block: each line of each side is checked on its own. */
export interface Pick { resolved: boolean; ours: boolean[]; theirs: boolean[]; theirsFirst: boolean }

/** Whole-block shortcuts: "none" resolves the block to nothing, "clear" reopens it. */
export type Choice = "ours" | "theirs" | "both" | "bothTheirsFirst" | "none" | "clear";

export function isConflict(s: Segment): s is Conflict {
  return s.kind === "conflict";
}

/**
 * Splits a file into plain text and conflict blocks. A `<<<<<<<` without its `>>>>>>>` is kept
 * as plain text, so nothing in the file is lost.
 */
export function parseSegments(file: string): Segment[] {
  const out: Segment[] = [];
  let plain: string[] = [];
  let open: { conflict: Conflict; raw: string[] } | null = null;
  let side: "ours" | "base" | "theirs" = "ours";
  const flush = () => {
    if (plain.length) out.push({ kind: "text", lines: plain });
    plain = [];
  };
  for (const line of file.split("\n")) {
    if (!open) {
      if (line.startsWith("<<<<<<<")) {
        open = { conflict: { kind: "conflict", start: line, end: "", ours: [], base: null, theirs: [] }, raw: [line] };
        side = "ours";
      } else {
        plain.push(line);
      }
      continue;
    }
    open.raw.push(line);
    const c = open.conflict;
    if (line.startsWith(">>>>>>>")) {
      c.end = line;
      flush();
      out.push(c);
      open = null;
    } else if (line.startsWith("|||||||")) {
      side = "base";
      c.base = [];
    } else if (line.startsWith("=======")) {
      side = "theirs";
    } else if (side === "base") {
      c.base?.push(line);
    } else if (side === "theirs") {
      c.theirs.push(line);
    } else {
      c.ours.push(line);
    }
  }
  if (open) plain.push(...open.raw);
  flush();
  return out;
}

export function conflictsOf(segments: Segment[]): Conflict[] {
  return segments.filter(isConflict);
}

/** Nothing chosen yet: every line unchecked and the block unresolved. */
export function emptyPick(c: Conflict): Pick {
  return { resolved: false, ours: c.ours.map(() => false), theirs: c.theirs.map(() => false), theirsFirst: false };
}

export function applyChoice(c: Conflict, choice: Choice): Pick {
  const all = (n: number, v: boolean) => Array.from({ length: n }, () => v);
  switch (choice) {
    case "ours":
      return { resolved: true, ours: all(c.ours.length, true), theirs: all(c.theirs.length, false), theirsFirst: false };
    case "theirs":
      return { resolved: true, ours: all(c.ours.length, false), theirs: all(c.theirs.length, true), theirsFirst: false };
    case "both":
      return { resolved: true, ours: all(c.ours.length, true), theirs: all(c.theirs.length, true), theirsFirst: false };
    case "bothTheirsFirst":
      return { resolved: true, ours: all(c.ours.length, true), theirs: all(c.theirs.length, true), theirsFirst: true };
    case "none":
      return { resolved: true, ours: all(c.ours.length, false), theirs: all(c.theirs.length, false), theirsFirst: false };
    case "clear":
      return emptyPick(c);
  }
}

/** Checks or unchecks one line; any line choice counts as resolving the block. */
export function toggleLine(c: Conflict, pick: Pick, side: "ours" | "theirs", index: number, checked: boolean): Pick {
  const next = { ...pick, resolved: true, ours: [...pick.ours], theirs: [...pick.theirs] };
  const list = side === "ours" ? next.ours : next.theirs;
  const length = side === "ours" ? c.ours.length : c.theirs.length;
  if (index >= 0 && index < length) list[index] = checked;
  return next;
}

/**
 * The file text for the current choices. Unresolved blocks keep conflict markers, so the
 * result always shows what is still open.
 */
export function render(segments: Segment[], picks: Pick[]): string {
  const out: string[] = [];
  let n = 0;
  for (const s of segments) {
    if (!isConflict(s)) {
      for (const l of s.lines) out.push(l);
      continue;
    }
    const p = picks[n++];
    if (!p || !p.resolved) {
      out.push(s.start);
      for (const l of s.ours) out.push(l);
      out.push("=======");
      for (const l of s.theirs) out.push(l);
      out.push(s.end);
      continue;
    }
    const ours = s.ours.filter((_, i) => p.ours[i]);
    const theirs = s.theirs.filter((_, i) => p.theirs[i]);
    for (const l of p.theirsFirst ? [...theirs, ...ours] : [...ours, ...theirs]) out.push(l);
  }
  return out.join("\n");
}

export function unresolvedCount(picks: Pick[]): number {
  return picks.filter((p) => !p.resolved).length;
}

/**
 * Base lines of each block, taken from a diff3 rendering of the same file. Only used when both
 * renderings have the same number of blocks; otherwise every base is `null` (not shown).
 */
export function baseLines(conflicts: Conflict[], diff3: Segment[] | null): (string[] | null)[] {
  const d = diff3 ? conflictsOf(diff3) : [];
  if (d.length !== conflicts.length) return conflicts.map(() => null);
  return d.map((c) => c.base);
}
