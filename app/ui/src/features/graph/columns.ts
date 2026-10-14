// SPDX-License-Identifier: Apache-2.0
// Graph column geometry shared by the canvas and the header (pure, unit-tested).

export interface ColumnWidths { refs: number; author: number; date: number }

export const SHA_W = 72;
export const MIN_MESSAGE_W = 160;
export const DEFAULT_WIDTHS: ColumnWidths = { refs: 180, author: 140, date: 96 };
const LIMITS: Record<keyof ColumnWidths, [number, number]> = { refs: [60, 480], author: [60, 320], date: [60, 200] };
const STORAGE_KEY = "ak.graph.columns.v1";

export const clampWidth = (col: keyof ColumnWidths, w: number) => Math.round(Math.min(LIMITS[col][1], Math.max(LIMITS[col][0], w)));

/** Column x positions. Narrow views drop author first, then date; message keeps ≥ MIN_MESSAGE_W. */
export function columns(width: number, graphW: number, w: ColumnWidths = DEFAULT_WIDTHS) {
  const msgX = w.refs + graphW;
  let right = width - 8;
  const shaX = right - SHA_W;
  right = shaX;
  const showDate = right - w.date - msgX >= MIN_MESSAGE_W;
  const dateX = showDate ? right - w.date : right;
  if (showDate) right = dateX;
  const showAuthor = right - w.author - msgX >= MIN_MESSAGE_W;
  const authorX = showAuthor ? right - w.author : right;
  if (showAuthor) right = authorX;
  return { msgX, msgW: right - msgX - 12, authorX, showAuthor, dateX, showDate, shaX };
}

// Widths are a per-machine convenience: storage may be unavailable, so fall back to defaults.
export function loadWidths(): ColumnWidths {
  try {
    const v = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (v && typeof v === "object") {
      return {
        refs: clampWidth("refs", Number(v.refs) || DEFAULT_WIDTHS.refs),
        author: clampWidth("author", Number(v.author) || DEFAULT_WIDTHS.author),
        date: clampWidth("date", Number(v.date) || DEFAULT_WIDTHS.date),
      };
    }
  } catch {}
  return { ...DEFAULT_WIDTHS };
}

export function saveWidths(w: ColumnWidths) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(w));
  } catch {}
}
