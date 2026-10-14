// SPDX-License-Identifier: Apache-2.0
// Benchmark overlay (roadmap T1.11, UI part). Toggle with Ctrl+Shift+P.
// Measures: repo open → first painted rows, per-frame draw time, scroll fps.
import { createSignal } from "solid-js";

const [enabled, setEnabled] = createSignal(false);
const [stats, setStats] = createSignal({ firstPaintMs: 0, drawMs: 0, worstMs: 0, fps: 0, rows: 0 });
export { enabled as perfEnabled, stats as perfStats };

let openStartedAt = 0;
let firstPaintPending = false;
const frames: number[] = [];
const draws: { t: number; ms: number }[] = [];
let frameQueued = false;

export function togglePerf() {
  setEnabled(!enabled());
}

/** Call right before asking the core to open a repo. */
export function markOpenStart() {
  openStartedAt = performance.now();
  firstPaintPending = true;
}

/** Call after each canvas draw; `paintedRows` = rows with data drawn. */
export function recordDraw(startedAt: number, paintedRows: number) {
  const now = performance.now();
  const drawMs = now - startedAt;
  draws.push({ t: now, ms: drawMs });
  while (draws.length && now - draws[0].t > 1000) draws.shift();
  const s = { ...stats(), drawMs, rows: paintedRows, worstMs: Math.max(...draws.map((d) => d.ms)) };
  if (firstPaintPending && paintedRows > 0) {
    s.firstPaintMs = now - openStartedAt;
    firstPaintPending = false;
  }
  // Several draws can happen inside one frame; count each painted frame once.
  if (!frameQueued) {
    frameQueued = true;
    requestAnimationFrame((t) => {
      frameQueued = false;
      frames.push(t);
      while (frames.length && t - frames[0] > 1000) frames.shift();
      if (enabled()) setStats({ ...stats(), fps: frames.length });
    });
  }
  s.fps = stats().fps;
  if (enabled() || s.firstPaintMs !== stats().firstPaintMs) setStats(s);
}
