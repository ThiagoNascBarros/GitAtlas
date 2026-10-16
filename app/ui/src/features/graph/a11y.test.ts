// SPDX-License-Identifier: Apache-2.0
import { afterEach, describe, expect, it } from "vitest";
import { menuAnchor, rowId, rowText, visibleRange, wipText } from "./a11y";
import { setLocaleChoice } from "../../i18n";
import type { RefInfo, RowMeta } from "../../api";

afterEach(() => setLocaleChoice("en"));

const H = 28;

describe("visibleRange", () => {
  it("covers the viewport plus overscan at the top", () => {
    // 280 px = rows 0..9 visible; overscan 3 adds 10..12 below, nothing above row 0.
    expect(visibleRange(0, 280, H, 0, 1000, 3, null)).toEqual([...Array(13).keys()]);
  });
  it("covers the viewport plus overscan in the middle", () => {
    // scrollTop 2800 → first visible row 100; 280 px → rows 100..109.
    expect(visibleRange(2800, 280, H, 0, 1000, 2, null)).toEqual([98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111]);
  });
  it("counts a partly visible row as visible", () => {
    expect(visibleRange(14, 28, H, 0, 1000, 0, null)).toEqual([0, 1]);
  });
  it("stops at the last row", () => {
    expect(visibleRange(990 * H, 560, H, 0, 1000, 5, null)).toEqual([985, 986, 987, 988, 989, 990, 991, 992, 993, 994, 995, 996, 997, 998, 999]);
  });
  it("shifts by the WIP row offset", () => {
    // The WIP row takes the first 28 px: at scrollTop 0 rows 0..8 fit in 280 px.
    expect(visibleRange(0, 280, H, H, 1000, 0, null)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    expect(visibleRange(H, 280, H, H, 1000, 0, null)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });
  it("adds the selected row when it is outside the window, in order", () => {
    expect(visibleRange(2800, 56, H, 0, 1000, 0, 5)).toEqual([5, 100, 101]);
    expect(visibleRange(0, 56, H, 0, 1000, 0, 500)).toEqual([0, 1, 500]);
    expect(visibleRange(0, 56, H, 0, 1000, 0, 1)).toEqual([0, 1]);
  });
  it("ignores a selected row that does not exist", () => {
    expect(visibleRange(0, 56, H, 0, 10, 0, 50)).toEqual([0, 1]);
  });
  it("is empty without rows", () => {
    expect(visibleRange(0, 600, H, 0, 0, 10, null)).toEqual([]);
    expect(visibleRange(0, 600, H, H, 0, 10, 0)).toEqual([]);
  });
  it("keeps a tiny viewport usable", () => {
    expect(visibleRange(0, 0, H, 0, 1000, 0, null)).toEqual([0]);
    expect(visibleRange(0, 1, H, 0, 1000, 1, null)).toEqual([0, 1]);
  });
  it("stays bounded by the viewport, not by the row count", () => {
    const rows = visibleRange(50_000 * H, 800, H, H, 100_000, 10, 3);
    expect(rows.length).toBeLessThanOrEqual(Math.ceil(800 / H) + 1 + 2 * 10 + 1);
  });
});

describe("rowId", () => {
  it("is unique per repository and row", () => {
    expect(rowId(3, 0)).toBe("graph-row-3-0");
    expect(rowId(3, "wip")).toBe("graph-row-3-wip");
    expect(rowId(4, 0)).not.toBe(rowId(3, 0));
  });
});

describe("rowText", () => {
  const meta: RowMeta = { sha: "0123456789abcdef0123456789abcdef01234567", summary: "Fix the thing", author: "Ada", email: "a@x", time: 1700000000 };
  const refs: RefInfo[] = [
    { name: "main", kind: "head", row: 0, isHead: true },
    { name: "origin/main", kind: "remote", row: 0, isHead: false },
    { name: "v1.0", kind: "tag", row: 0, isHead: false },
    { name: "stash@{0}", kind: "stash", row: 0, isHead: false },
  ];
  it("gives the canvas texts, with words for the ref glyphs", () => {
    const r = rowText(meta, refs, [], "en");
    expect(r.refs).toBe("main (current branch), origin/main, tag v1.0, stash stash@{0}");
    expect(r.message).toBe("Fix the thing");
    expect(r.author).toBe("Ada");
    expect(r.date).toBe(new Date(1700000000 * 1000).toLocaleDateString("en"));
    expect(r.sha).toBe("0123456");
  });
  it("adds shared stashes to the refs cell", () => {
    const r = rowText(meta, [], [{ title: "Spike" }, { title: "Retry fix" }], "en");
    expect(r.refs).toBe("Shared stash: Spike, Shared stash: Retry fix");
  });
  it("follows the active language and the given locale for dates", () => {
    setLocaleChoice("pt-BR");
    const r = rowText(meta, refs.slice(0, 1), [{ title: "Spike" }], "pt-BR");
    expect(r.refs).toBe("main (branch atual), Stash compartilhado: Spike");
    expect(r.date).toBe(new Date(1700000000 * 1000).toLocaleDateString("pt-BR"));
  });
  it("leaves a missing author or no refs empty", () => {
    const r = rowText({ ...meta, author: "" }, [], [], "en");
    expect(r.author).toBe("");
    expect(r.refs).toBe("");
  });
});

describe("wipText", () => {
  it("lists the non-empty counts", () => {
    expect(wipText({ staged: 1, unstaged: 2, conflicted: 0 })).toBe("Uncommitted changes: 2 unstaged, 1 staged");
    expect(wipText({ staged: 0, unstaged: 0, conflicted: 3 })).toBe("Uncommitted changes: 3 conflicted");
  });
  it("says when there is nothing", () => {
    expect(wipText({ staged: 0, unstaged: 0, conflicted: 0 })).toBe("Uncommitted changes: No changes");
  });
});

describe("menuAnchor", () => {
  const rect = { left: 200, top: 100, width: 600, height: 400 };
  const opts = { rowH: H, offset: 0, msgX: 150, viewW: 1280, viewH: 800 };
  it("anchors at the message column, under the row", () => {
    expect(menuAnchor(2, rect, 0, opts)).toEqual({ x: 350, y: 100 + 3 * H });
  });
  it("accounts for scrolling and the WIP row", () => {
    expect(menuAnchor(10, rect, 5 * H, { ...opts, offset: H })).toEqual({ x: 350, y: 100 + (10 - 5 + 1 + 1) * H });
  });
  it("clamps inside the graph when the row is scrolled away", () => {
    expect(menuAnchor(0, rect, 50 * H, opts).y).toBe(100);
    expect(menuAnchor(500, rect, 0, opts).y).toBe(500);
  });
  it("clamps inside the viewport", () => {
    expect(menuAnchor(0, { left: 1200, top: 780, width: 600, height: 400 }, 0, opts)).toEqual({ x: 1279, y: 799 });
    expect(menuAnchor(0, { left: -50, top: -40, width: 100, height: 10 }, 0, { ...opts, msgX: 0 })).toEqual({ x: 0, y: 0 });
  });
  it("keeps the anchor inside a narrow graph", () => {
    expect(menuAnchor(0, { left: 0, top: 0, width: 100, height: 400 }, 0, opts).x).toBe(100);
  });
});
