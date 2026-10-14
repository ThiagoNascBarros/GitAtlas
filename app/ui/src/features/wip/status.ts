// SPDX-License-Identifier: Apache-2.0
// Status entries → the three lists of the working-directory panel (pure, unit-tested).
import type { StatusEntry } from "../../api";

export interface WipItem { path: string; origPath: string | null; status: string; untracked: boolean }
export interface WipGroups { staged: WipItem[]; unstaged: WipItem[]; conflicted: WipItem[] }

export function groupStatus(entries: StatusEntry[]): WipGroups {
  const g: WipGroups = { staged: [], unstaged: [], conflicted: [] };
  for (const e of entries) {
    const base = { path: e.path, origPath: e.orig_path, untracked: e.untracked };
    if (e.conflicted) {
      g.conflicted.push({ ...base, status: "U" });
      continue;
    }
    if (e.untracked) {
      g.unstaged.push({ ...base, status: "U" });
      continue;
    }
    if (e.index !== "." && e.index !== " ") g.staged.push({ ...base, status: e.index });
    if (e.worktree !== "." && e.worktree !== " ") g.unstaged.push({ ...base, status: e.worktree });
  }
  const byPath = (a: WipItem, b: WipItem) => a.path.localeCompare(b.path);
  g.staged.sort(byPath);
  g.unstaged.sort(byPath);
  return g;
}

export function changeCount(g: WipGroups): number {
  return new Set([...g.staged, ...g.unstaged, ...g.conflicted].map((i) => i.path)).size;
}
