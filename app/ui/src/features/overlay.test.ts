// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { cycleIndex, handlesEscape, isMenuKey, neighbourIndex } from "./overlay";

describe("isMenuKey", () => {
  it("accepts the Menu key and Shift+F10", () => {
    expect(isMenuKey({ key: "ContextMenu", shiftKey: false })).toBe(true);
    expect(isMenuKey({ key: "ContextMenu", shiftKey: true })).toBe(true);
    expect(isMenuKey({ key: "F10", shiftKey: true })).toBe(true);
  });
  it("rejects everything else", () => {
    expect(isMenuKey({ key: "F10", shiftKey: false })).toBe(false);
    expect(isMenuKey({ key: "Enter", shiftKey: true })).toBe(false);
    expect(isMenuKey({ key: "m", shiftKey: false })).toBe(false);
  });
});

describe("cycleIndex (focus trap)", () => {
  it("moves forward and wraps from the last to the first", () => {
    expect(cycleIndex(3, 0, false)).toBe(1);
    expect(cycleIndex(3, 2, false)).toBe(0);
  });
  it("moves backward and wraps from the first to the last", () => {
    expect(cycleIndex(3, 1, true)).toBe(0);
    expect(cycleIndex(3, 0, true)).toBe(2);
  });
  it("enters at the edge when focus is outside the container", () => {
    expect(cycleIndex(3, -1, false)).toBe(0);
    expect(cycleIndex(3, -1, true)).toBe(2);
  });
  it("stays on a single element", () => {
    expect(cycleIndex(1, 0, false)).toBe(0);
    expect(cycleIndex(1, 0, true)).toBe(0);
  });
  it("has no target without focusable elements", () => {
    expect(cycleIndex(0, -1, false)).toBe(-1);
  });
});

describe("neighbourIndex (focus recovery)", () => {
  it("finds the same item after a re-render", () => {
    expect(neighbourIndex(["a", "b", "c"], "b", 1)).toBe(1);
    expect(neighbourIndex(["x", "a", "b"], "b", 1)).toBe(2);
  });
  it("takes the item now at the same position when it is gone", () => {
    expect(neighbourIndex(["a", "c"], "b", 1)).toBe(1);
  });
  it("takes the last item when the gone one was last", () => {
    expect(neighbourIndex(["a", "b"], "c", 2)).toBe(1);
  });
  it("has no target in an empty list", () => {
    expect(neighbourIndex([], "a", 0)).toBe(-1);
  });
});

describe("handlesEscape (innermost overlay only)", () => {
  const a = {}, b = {};
  it("is true for the innermost trapped overlay", () => expect(handlesEscape([a, b], b)).toBe(true));
  it("is false for an outer overlay under another", () => expect(handlesEscape([a, b], a)).toBe(false));
  it("is true for an overlay without a trap (not in the stack)", () => expect(handlesEscape([a], b)).toBe(true));
});
