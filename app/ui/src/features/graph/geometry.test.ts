// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { LANE_W, ROW_H, edgeCurve, laneX, mixHex, rowY } from "./geometry";

describe("geometry", () => {
  it("uses GitKraken's 22px row height", () => {
    expect(ROW_H).toBe(22);
  });
  it("rowY maps a row to its top, minus scroll", () => {
    expect(rowY(0, 0)).toBe(0);
    expect(rowY(3, 10)).toBe(3 * 22 - 10);
  });
  it("laneX centres lane slots from the graph column origin", () => {
    expect(laneX(100, 0)).toBe(100 + LANE_W / 2 + 4);
    expect(laneX(100, 2)).toBe(100 + LANE_W / 2 + 2 * LANE_W + 4);
  });
});

describe("mixHex", () => {
  it("mixes lane colour 50% with the canvas for ref label backgrounds", () => {
    expect(mixHex("#ff0000", "#000000", 0.5)).toBe("#800000");
    expect(mixHex("#15a0bf", "#17151f", 0.5)).toBe("#165b6f");
  });
  it("t=1 keeps the first colour, t=0 the second", () => {
    expect(mixHex("#15a0bf", "#17151f", 1)).toBe("#15a0bf");
    expect(mixHex("#15a0bf", "#17151f", 0)).toBe("#17151f");
  });
});

describe("edgeCurve", () => {
  it("'in' runs from the row top to the node centre, leaving vertically", () => {
    expect(edgeCurve("in", 10, 30, 100)).toEqual({ p0: [10, 100], c1: [10, 111], c2: [30, 100], p1: [30, 111] });
  });
  it("'out' runs from the node centre to the row bottom, arriving vertically", () => {
    expect(edgeCurve("out", 10, 30, 100)).toEqual({ p0: [10, 111], c1: [10, 122], c2: [30, 111], p1: [30, 122] });
  });
});
