// SPDX-License-Identifier: Apache-2.0
// Accessibility mirror of the canvas graph (spec accessibility A-1/A-2): which rows exist in the
// DOM, what they say, and where a keyboard-opened menu goes. Pure, so it is unit-tested.
import type { RefInfo, RowMeta } from "../../api";
import { t } from "../../i18n";

/**
 * Rows to mirror: the ones in the viewport plus `overscan` on each side, plus the selected row
 * when it is elsewhere (so `aria-activedescendant` always has a target). Sorted, no duplicates.
 * `offset` is the height above row 0 (the uncommitted-changes row).
 */
export function visibleRange(
  scrollTop: number, viewH: number, rowH: number, offset: number, rowCount: number, overscan: number, selected: number | null,
): number[] {
  if (rowCount <= 0) return [];
  const top = scrollTop - offset;
  const first = Math.max(0, Math.floor(top / rowH) - overscan);
  const last = Math.min(rowCount - 1, Math.floor((top + Math.max(viewH, 1) - 1) / rowH) + overscan);
  const out: number[] = [];
  const extra = selected !== null && selected >= 0 && selected < rowCount && (selected < first || selected > last) ? selected : null;
  if (extra !== null && extra < first) out.push(extra);
  for (let i = first; i <= last; i++) out.push(i);
  if (extra !== null && extra > last) out.push(extra);
  return out;
}

/** DOM id of a mirrored row; the repository id keeps ids apart when the view switches tabs. */
export const rowId = (repoId: number, row: number | "wip") => `graph-row-${repoId}-${row}`;

export interface RowText { refs: string; message: string; author: string; date: string; sha: string }

/** Cell texts of a commit row: what the canvas draws, with words instead of the ref glyphs. */
export function rowText(meta: RowMeta, refs: readonly RefInfo[], shared: readonly { title: string }[], locale: string): RowText {
  const labels = refs.map((r) =>
    r.kind === "tag" ? t("graph.refTag", { s: r.name })
    : r.kind === "stash" ? t("graph.refStash", { s: r.name })
    : r.isHead ? t("graph.refCurrent", { s: r.name })
    : r.name);
  for (const s of shared) labels.push(t("ss.glyphTitle", { s: s.title }));
  return {
    refs: labels.join(", "),
    message: meta.summary,
    author: meta.author ?? "",
    date: new Date(meta.time * 1000).toLocaleDateString(locale),
    sha: meta.sha.slice(0, 7),
  };
}

/** Text of the uncommitted-changes row: the label and its non-empty counts (as the canvas pills). */
export function wipText(w: { staged: number; unstaged: number; conflicted: number }): string {
  const parts: string[] = [];
  if (w.conflicted) parts.push(t("wip.chipConflicts", { n: w.conflicted }));
  if (w.unstaged) parts.push(t("wip.chipUnstaged", { n: w.unstaged }));
  if (w.staged) parts.push(t("wip.chipStaged", { n: w.staged }));
  return `${t("wip.row")}: ${parts.length ? parts.join(", ") : t("wip.clean")}`;
}

interface Rect { left: number; top: number; width: number; height: number }
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

/**
 * Where a keyboard-opened commit menu goes: left edge of the message column, bottom of the row,
 * kept inside the graph's box and the window (the menu then keeps its own size inside the window).
 */
export function menuAnchor(
  row: number, rect: Rect, scrollTop: number, o: { rowH: number; offset: number; msgX: number; viewW: number; viewH: number },
): { x: number; y: number } {
  const x = clamp(rect.left + o.msgX, rect.left, rect.left + rect.width);
  const y = clamp(rect.top + row * o.rowH + o.offset - scrollTop + o.rowH, rect.top, rect.top + rect.height);
  return { x: clamp(x, 0, o.viewW - 1), y: clamp(y, 0, o.viewH - 1) };
}
