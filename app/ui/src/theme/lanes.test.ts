// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import dark from "../../../../themes/atlas-dark.theme.json";
import light from "../../../../themes/atlas-light.theme.json";

const GK = ["#15a0bf", "#0669f7", "#8e00c2", "#c517b6", "#d90171", "#cd0101", "#f25d2e", "#f2ca33", "#7bd938", "#2ece9d"];

describe.each([["dark", dark], ["light", light]])("%s theme lane palette", (_n, theme) => {
  it("color.lane.0..9 equal the reference palette", () => {
    const t = theme.tokens as Record<string, string>;
    expect(GK.map((_, i) => t[`color.lane.${i}`].toLowerCase())).toEqual(GK);
  });
});
