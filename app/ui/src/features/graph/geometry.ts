// SPDX-License-Identifier: Apache-2.0
// Pure graph geometry; GraphView only draws with these.
export const ROW_H = 22;
export const LANE_W = 16;

/** Top y of row `i` for a canvas scrolled by `top`. */
export const rowY = (i: number, top: number) => i * ROW_H - top;

/** Centre x of lane slot `l` in a graph column starting at `gx`. */
export const laneX = (gx: number, l: number) => gx + LANE_W / 2 + l * LANE_W + 4;

/** `t` of colour `a` mixed with `1 - t` of `b` (both #rrggbb), e.g. t=0.5 for ref label backgrounds. */
export function mixHex(a: string, b: string, t: number): string {
  const ch = (h: string, i: number) => parseInt(h.slice(1 + 2 * i, 3 + 2 * i), 16);
  return "#" + [0, 1, 2].map((i) => Math.round(ch(a, i) * t + ch(b, i) * (1 - t)).toString(16).padStart(2, "0")).join("");
}
