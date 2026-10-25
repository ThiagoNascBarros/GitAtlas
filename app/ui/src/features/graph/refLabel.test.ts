// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { contrast, mixHex } from "./geometry";
import { refLabelColors } from "./refLabel";

const LANES = ["#15a0bf", "#0669f7", "#8e00c2", "#c517b6", "#d90171", "#cd0101", "#f25d2e", "#f2ca33", "#7bd938", "#2ece9d"];
const THEMES = [
  { canvas: "#17151F", light: "#ECE9F2", dark: "#16102A" },
  { canvas: "#F1EFF5", light: "#FFFFFF", dark: "#1E1A27" },
];

describe("contrast", () => {
  it("is 21 for black on white and 1 for equal colours", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 1);
    expect(contrast("#336699", "#336699")).toBeCloseTo(1, 5);
  });
});

describe("refLabelColors", () => {
  it("mixes the lane colour 50% with the canvas for a normal ref", () => {
    const { bg } = refLabelColors("#15a0bf", false, THEMES[0]);
    expect(bg).toBe(mixHex("#15a0bf", "#17151F", 0.5));
  });
  it("uses the solid lane colour for the HEAD ref", () => {
    expect(refLabelColors("#15a0bf", true, THEMES[0]).bg).toBe("#15a0bf");
  });
  it("keeps label text at AA contrast for every lane, theme and HEAD state", () => {
    for (const th of THEMES) for (const lane of LANES) for (const head of [false, true]) {
      const { bg, fg } = refLabelColors(lane, head, th);
      expect(contrast(fg, bg), `${lane} head=${head} on ${th.canvas}`).toBeGreaterThanOrEqual(4.5);
    }
  });
});
