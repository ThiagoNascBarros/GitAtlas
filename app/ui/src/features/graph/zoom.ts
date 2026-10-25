// SPDX-License-Identifier: Apache-2.0
// Graph zoom and pan, in logical (zoom 1) css px. The canvas is redrawn at the new scale, never stretched.
export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 2;

export const clampZoom = (z: number, min = ZOOM_MIN, max = ZOOM_MAX) => Math.min(max, Math.max(min, z));

/** Ctrl +/-: the next whole 10 % step (no drift after many steps); 0 resets to 100 %. */
export const stepZoom = (z: number, dir: -1 | 0 | 1, min = ZOOM_MIN, max = ZOOM_MAX) =>
  dir === 0 ? 1 : clampZoom(Math.round(z * 10 + dir) / 10, min, max);

/** Ctrl/Cmd + these keys: zoom in, out, reset (shared by the graph and the app-wide UI zoom). */
export const ZOOM_KEYS: Record<string, -1 | 0 | 1> = { "+": 1, "=": 1, "-": -1, "_": -1, "0": 0 };

/** Ctrl+wheel: about 10 % per mouse notch, smooth on touchpads (deltaMode 1 = lines). */
export function wheelZoom(z: number, deltaY: number, deltaMode: number): number {
  const d = Math.min(100, Math.max(-100, deltaMode === 0 ? deltaY : deltaY * 33));
  return clampZoom(z * Math.exp(-d / 500));
}

/** Logical width the columns are laid out in: zooming out widens it, zooming in keeps it (and pans). */
export const layoutWidth = (w: number, z: number) => Math.max(w, w / z);

/** Largest horizontal pan for a view `w` css px wide. */
export const maxPan = (w: number, z: number) => Math.max(0, w - w / z);

/** Pan and scroll that keep the logical point under (cx, cy) (css px in the view) still while zooming. */
export function zoomAt(s: { z0: number; z1: number; panX: number; scrollTop: number; w: number }, cx: number, cy: number) {
  const gx = s.panX + cx / s.z0, gy = (s.scrollTop + cy) / s.z0;
  return { panX: Math.min(maxPan(s.w, s.z1), Math.max(0, gx - cx / s.z1)), scrollTop: Math.max(0, gy * s.z1 - cy) };
}
