// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { clampZoom, DEFAULT_PREFS, zoomScope, zoomStep, ZOOM_MAX, ZOOM_MIN } from "./prefs";
import type { PrefsView } from "../api";

describe("zoom", () => {
  it("stays within the PRD range (80 % to 150 %)", () => {
    expect(clampZoom(0.5)).toBe(ZOOM_MIN);
    expect(clampZoom(3)).toBe(ZOOM_MAX);
    expect(clampZoom(1.25)).toBe(1.25);
  });
  it("moves in 10 % steps without floating-point drift", () => {
    expect(zoomStep(1, 1)).toBe(1.1);
    expect(zoomStep(1.1, 1)).toBe(1.2);
    expect(zoomStep(1.2, -1)).toBe(1.1);
    expect(zoomStep(0.8, -1)).toBe(0.8);
    expect(zoomStep(1.5, 1)).toBe(1.5);
  });
  it("defaults to 100 %", () => {
    expect(DEFAULT_PREFS.zoom).toBe(1);
  });
});

describe("zoomScope", () => {
  const view = (repo: PrefsView["repo"]): PrefsView => ({ global: {}, repo, effective: DEFAULT_PREFS });

  it("writes to the repository only when it already overrides zoom", () => {
    expect(zoomScope(view({ zoom: 1.5 }), 7)).toBe("repo");
    expect(zoomScope(view({ theme: "atlas-dark" }), 7)).toBe("global");
  });
  it("uses the global scope without an open repository or a view", () => {
    expect(zoomScope(view({ zoom: 1.5 }), null)).toBe("global");
    expect(zoomScope(null, 7)).toBe("global");
  });
});
