// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { clampWidth, columns, DEFAULT_WIDTHS, MIN_MESSAGE_W, resolveWidths } from "./columns";

describe("columns", () => {
  it("shows every column when wide", () => {
    const c = columns(1400, 64);
    expect(c.showAuthor && c.showDate).toBe(true);
    expect(c.msgX).toBe(DEFAULT_WIDTHS.refs + 64);
    expect(c.authorX).toBeLessThan(c.dateX);
    expect(c.dateX).toBeLessThan(c.shaX);
  });
  it("drops author first, then date, keeping the message readable", () => {
    const mid = columns(650, 64);
    expect(mid.showAuthor).toBe(false);
    expect(mid.showDate).toBe(true);
    const narrow = columns(420, 64);
    expect(narrow.showDate).toBe(false);
    for (const w of [420, 650, 900, 1400]) {
      const c = columns(w, 64);
      if (c.showAuthor || c.showDate) expect(c.msgW + 12).toBeGreaterThanOrEqual(MIN_MESSAGE_W);
    }
  });
  it("follows custom widths and clamps them", () => {
    const c = columns(1400, 64, { refs: 300, author: 200, date: 100 });
    expect(c.msgX).toBe(364);
    expect(c.dateX - c.authorX).toBe(200);
    expect(clampWidth("refs", 5)).toBe(60);
    expect(clampWidth("author", 9999)).toBe(320);
  });
});

describe("resolveWidths", () => {
  it("shrinks an auto refs column so the message stays readable", () => {
    const auto = { ...DEFAULT_WIDTHS, refs: 0 };
    expect(resolveWidths(auto, 1400, 64).refs).toBe(DEFAULT_WIDTHS.refs);
    const narrow = resolveWidths(auto, 400, 64);
    expect(narrow.refs).toBeLessThan(DEFAULT_WIDTHS.refs);
    expect(columns(400, 64, narrow).msgW + 12).toBeGreaterThanOrEqual(MIN_MESSAGE_W);
    expect(resolveWidths(auto, 100, 64).refs).toBe(60);
  });
  it("keeps a manual refs width", () => {
    expect(resolveWidths({ ...DEFAULT_WIDTHS, refs: 300 }, 520, 64).refs).toBe(300);
  });
});
