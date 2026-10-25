// SPDX-License-Identifier: Apache-2.0
import { contrast, mixHex } from "./geometry";

export interface LabelTheme { canvas: string; light: string; dark: string }

/** Ref label colours: lane colour mixed 50% into the canvas (solid for HEAD), text picked for contrast. */
export function refLabelColors(lane: string, isHead: boolean, th: LabelTheme): { bg: string; fg: string } {
  const bg = isHead ? lane : mixHex(lane, th.canvas, 0.5);
  const best = (c: string[]) => c.reduce((a, b) => (contrast(b, bg) > contrast(a, bg) ? b : a));
  const fg = best([th.light, th.dark]);
  // Theme text can fall short on saturated mid-tones (blue HEAD): fall back to pure black/white.
  return { bg, fg: contrast(fg, bg) >= 4.5 ? fg : best(["#ffffff", "#000000"]) };
}
