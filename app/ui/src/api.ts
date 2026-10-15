// SPDX-License-Identifier: Apache-2.0
// Typed wrappers around the core commands (SDD §4.1).
import { invoke } from "@tauri-apps/api/core";

export type RefKind = "head" | "remote" | "tag" | "stash" | "other";
export interface RefInfo { name: string; kind: RefKind; row: number | null; isHead: boolean }
export interface OpenedRepo {
  repoId: number; root: string; name: string; head: string | null; branch: string | null;
  rowCount: number; laneCount: number; refs: RefInfo[]; gitVersion: string; readOnlyReason: string | null;
}
export interface RowMeta { sha: string; summary: string; author: string; email: string; time: number }
export interface FileChange { status: string; path: string; orig_path: string | null }
export interface CommitDetails {
  sha: string; parents: string[]; author: string; email: string; time: number; message: string; files: FileChange[];
}
export interface UiError { code: string; message: string; detail?: string; suggestion?: string }
export interface StatusEntry {
  path: string; orig_path: string | null; index: string; worktree: string; conflicted: boolean; untracked: boolean;
}
export interface StatusReport { branch: string | null; oid: string | null; entries: StatusEntry[]; merging?: boolean; in_progress?: string | null; upstream?: string | null; ahead?: number; behind?: number }
export interface MergePreview { up_to_date: boolean; fast_forward: boolean; conflicts: string[] | null }
export interface ConflictVersions { base: string | null; ours: string | null; theirs: string | null; merged: string }
export interface TodoItem { sha: string; action: "pick" | "reword" | "squash" | "fixup" | "drop"; message: string }
export interface StashInfo { index: number; sha: string; message: string; time: number }
export interface OpOutcome { id: string | null; head: string | null; conflicted?: boolean }
export interface JournalEntry {
  id: string; time: number; kind: string; summary: string; undone: boolean; commands: string[]; paths: string[]; untracked: string[];
}

export const api = {
  initialPath: () => invoke<string | null>("get_initial_path"),
  open: (path: string) => invoke<OpenedRepo>("repo_open", { path }),
  refresh: (repoId: number) => invoke<OpenedRepo>("repo_refresh", { repoId }),
  status: (repoId: number) => invoke<StatusReport>("status_get", { repoId }),
  worktreeDiff: (repoId: number, path: string, staged: boolean, untracked: boolean) =>
    invoke<string>("diff_worktree", { repoId, path, staged, untracked }),
  stage: (repoId: number, paths: string[]) => invoke<OpOutcome>("op_stage", { repoId, paths }),
  unstage: (repoId: number, paths: string[]) => invoke<OpOutcome>("op_unstage", { repoId, paths }),
  stageHunk: (repoId: number, path: string, hunk: number, unstage: boolean) =>
    invoke<OpOutcome>("op_stage_hunk", { repoId, path, hunk, unstage }),
  discard: (repoId: number, paths: string[]) => invoke<OpOutcome>("op_discard", { repoId, paths }),
  commit: (repoId: number, message: string, amend: boolean) => invoke<OpOutcome>("op_commit", { repoId, message, amend }),
  branchCreate: (repoId: number, name: string, at: string | null) => invoke<OpOutcome>("op_branch_create", { repoId, name, at }),
  branchRename: (repoId: number, old: string, newName: string) => invoke<OpOutcome>("op_branch_rename", { repoId, old, new: newName }),
  branchDelete: (repoId: number, name: string, force: boolean) => invoke<OpOutcome>("op_branch_delete", { repoId, name, force }),
  checkout: (repoId: number, target: string, isBranch: boolean) => invoke<OpOutcome>("op_checkout", { repoId, target, isBranch }),
  mergePreview: (repoId: number, source: string) => invoke<MergePreview>("merge_preview", { repoId, source }),
  merge: (repoId: number, source: string) => invoke<OpOutcome>("op_merge", { repoId, source }),
  mergeAbort: (repoId: number) => invoke<OpOutcome>("op_merge_abort", { repoId }),
  conflictVersions: (repoId: number, path: string) => invoke<ConflictVersions>("conflict_versions", { repoId, path }),
  resolveWith: (repoId: number, path: string, content: string) => invoke<OpOutcome>("op_resolve_with", { repoId, path, content }),
  resolve: (repoId: number, path: string, theirs: boolean) => invoke<OpOutcome>("op_resolve", { repoId, path, theirs }),
  stashList: (repoId: number) => invoke<StashInfo[]>("stash_list", { repoId }),
  stash: (repoId: number, action: "push" | "apply" | "pop" | "drop", index = 0, message: string | null = null) =>
    invoke<OpOutcome>("op_stash", { repoId, action, index, message }),
  history: (repoId: number, action: "cherry-pick" | "revert" | "reset" | "tag" | "tag-delete", sha: string, opts: { mode?: string; name?: string } = {}) =>
    invoke<OpOutcome>("op_history", { repoId, action, sha, mode: opts.mode ?? null, name: opts.name ?? null }),
  net: (repoId: number, action: "fetch" | "pull" | "push", force = false) => invoke<OpOutcome>("op_net", { repoId, action, force }),
  credentialsAnswer: (id: number, value: string | null) => invoke<void>("credentials_answer", { id, value }),
  rebaseTodo: (repoId: number, onto: string) => invoke<TodoItem[]>("rebase_todo", { repoId, onto }),
  rebase: (repoId: number, onto: string, todo: TodoItem[] | null = null) => invoke<OpOutcome>("op_rebase", { repoId, onto, todo }),
  rebaseContinue: (repoId: number) => invoke<OpOutcome>("op_rebase_continue", { repoId }),
  undo: (repoId: number, id: string) => invoke<OpOutcome>("op_undo", { repoId, id }),
  redo: (repoId: number) => invoke<OpOutcome>("op_redo", { repoId }),
  journal: (repoId: number) => invoke<JournalEntry[]>("journal_list", { repoId }),
  pruneJournal: (repoId: number, keep: number) => invoke<number>("journal_prune", { repoId, keep }),
  close: (repoId: number) => invoke<void>("repo_close", { repoId }),
  rows: (repoId: number, start: number, count: number) => invoke<RowMeta[]>("graph_rows", { repoId, start, count }),
  window: async (repoId: number, start: number, count: number) =>
    decodeWindow(toArrayBuffer(await invoke<unknown>("graph_window", { repoId, start, count }))),
  details: (repoId: number, sha: string) => invoke<CommitDetails>("commit_details", { repoId, sha }),
  diff: (repoId: number, sha: string, path: string) => invoke<string>("diff_file", { repoId, sha, path }),
};

export const SEG_PASS = 0, SEG_IN = 1, SEG_OUT = 2;
export const FLAG_MERGE = 1, FLAG_MISSING_PARENT = 2;
export interface Seg { kind: number; from: number; to: number; color: number }
export interface GraphRow { row: number; lane: number; color: number; flags: number; segs: Seg[] }
export interface GraphWindow { start: number; laneCount: number; rows: GraphRow[] }

/**
 * Tauri returns raw `Response` bodies as an ArrayBuffer, a Uint8Array or a plain
 * number[] depending on the IPC path (custom protocol vs. postMessage fallback).
 */
export function toArrayBuffer(v: unknown): ArrayBuffer {
  if (v instanceof ArrayBuffer) return v;
  if (ArrayBuffer.isView(v)) return v.buffer.slice(v.byteOffset, v.byteOffset + v.byteLength) as ArrayBuffer;
  if (Array.isArray(v)) return Uint8Array.from(v).buffer;
  throw new Error(`unexpected graph window payload: ${Object.prototype.toString.call(v)}`);
}

/** Decodes the binary RowWindow (SDD §4.3; see ak-core graph/encode.rs). */
export function decodeWindow(buf: ArrayBuffer): GraphWindow {
  const v = new DataView(buf);
  const magic = String.fromCharCode(v.getUint8(0), v.getUint8(1), v.getUint8(2), v.getUint8(3));
  if (magic !== "AKRW" || v.getUint16(4, true) !== 1) throw new Error("bad RowWindow header");
  const start = v.getUint32(6, true), count = v.getUint32(10, true), laneCount = v.getUint16(14, true);
  const segBase = 16 + count * 16;
  const rows: GraphRow[] = [];
  for (let i = 0; i < count; i++) {
    const o = 16 + i * 16;
    const segOff = v.getUint32(o + 10, true), segCount = v.getUint16(o + 14, true);
    const segs: Seg[] = [];
    for (let s = 0; s < segCount; s++) {
      const so = segBase + (segOff + s) * 8;
      segs.push({ kind: v.getUint8(so), from: v.getUint16(so + 2, true), to: v.getUint16(so + 4, true), color: v.getUint16(so + 6, true) });
    }
    rows.push({ row: v.getUint32(o, true), lane: v.getUint16(o + 4, true), color: v.getUint16(o + 6, true), flags: v.getUint8(o + 8), segs });
  }
  return { start, laneCount, rows };
}
export interface CredentialsRequest { id: number; repoId: number; prompt: string; secret: boolean }
