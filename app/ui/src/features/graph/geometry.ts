// SPDX-License-Identifier: Apache-2.0
// Pure graph geometry; GraphView only draws with these.
export const ROW_H = 22;
export const LANE_W = 16;

/** Top y of row `i` for a canvas scrolled by `top`. */
export const rowY = (i: number, top: number) => i * ROW_H - top;

/** Centre x of lane slot `l` in a graph column starting at `gx`. */
export const laneX = (gx: number, l: number) => gx + LANE_W / 2 + l * LANE_W + 4;
