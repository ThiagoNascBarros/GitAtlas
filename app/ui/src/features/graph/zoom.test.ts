// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { layoutWidth, maxPan, stepZoom, wheelZoom, zoomAt, ZOOM_MAX, ZOOM_MIN } from "./zoom";

describe("graph zoom", () => {
  it("steps in whole 10 % without drift, clamps, and resets", () => {
    let z = 1;
    for (let i = 0; i < 3; i++) z = stepZoom(z, 1);
    expect(z).toBe(1.3);
    expect(stepZoom(ZOOM_MAX, 1)).toBe(ZOOM_MAX);
    expect(stepZoom(ZOOM_MIN, -1)).toBe(ZOOM_MIN);
    expect(stepZoom(1.7, 0)).toBe(1);
    expect(stepZoom(1.5, 1, 0.8, 1.5)).toBe(1.5);
  });
  it("wheel up zooms in, down zooms out, within bounds", () => {
    expect(wheelZoom(1, -100, 0)).toBeGreaterThan(1);
    expect(wheelZoom(1, 3, 1)).toBeLessThan(1);
    expect(wheelZoom(ZOOM_MAX, -100, 0)).toBe(ZOOM_MAX);
  });
  it("pans only when zoomed in", () => {
    expect(maxPan(800, 2)).toBe(400);
    expect(maxPan(800, 0.5)).toBe(0);
    expect(layoutWidth(800, 0.5)).toBe(1600);
    expect(layoutWidth(800, 2)).toBe(800);
  });
  it("keeps the point under the cursor still", () => {
    const s = { z0: 1, z1: 2, panX: 0, scrollTop: 100, w: 800 };
    expect(zoomAt(s, 200, 50)).toEqual({ panX: 100, scrollTop: 250 });
    // Back out: the same logical point returns under the cursor.
    expect(zoomAt({ ...s, z0: 2, z1: 1, panX: 100, scrollTop: 250 }, 200, 50)).toEqual({ panX: 0, scrollTop: 100 });
    // Never past the edges.
    expect(zoomAt({ ...s, scrollTop: 0 }, 0, 0)).toEqual({ panX: 0, scrollTop: 0 });
  });
});
