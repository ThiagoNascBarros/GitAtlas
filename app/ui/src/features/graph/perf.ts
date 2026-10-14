// SPDX-License-Identifier: Apache-2.0
// Benchmark overlay (roadmap T1.11, UI part). Toggle with Ctrl+Shift+P.
// Measures: repo open → first painted rows, per-frame draw time, scroll fps.
//
// IMPORTANT: recordDraw() runs inside the canvas draw effect. It must never
// read a signal (that would subscribe the effect to it and, since it writes
// one, loop forever). State lives in plain variables; the signal is only
// written, at most once per animation frame.
import { createSignal } from "solid-js";

interface Stats { firstPaintMs: number; drawMs: number; worstMs: number; fps: number; rows: number }

const [enabled, setEnabled] = createSignal(false);
const [stats, setStats] = createSignal<Stats>({ firstPaintMs: 0, drawMs: 0, worstMs: 0, fps: 0, rows: 0 });
export { enabled as perfEnabled, stats as perfStats };

let on = false;
let openStartedAt = 0;
let firstPaintPending = false;
let current: Stats = { firstPaintMs: 0, drawMs: 0, worstMs: 0, fps: 0, rows: 0 };
const frames: number[] = [];
const draws: { t: number; ms: number }[] = [];
let flushQueued = false;

export function togglePerf() {
  on = !on;
  setEnabled(on);
}

/** Call right before asking the core to open a repo. */
export function markOpenStart() {
  openStartedAt = performance.now();
  firstPaintPending = true;
}

/** Call after each canvas draw; `paintedRows` = rows with data drawn. */
export function recordDraw(startedAt: number, paintedRows: number) {
  const now = performance.now();
  draws.push({ t: now, ms: now - startedAt });
  while (draws.length && now - draws[0].t > 1000) draws.shift();
  current.drawMs = now - startedAt;
  current.rows = paintedRows;
  current.worstMs = draws.reduce((m, d) => Math.max(m, d.ms), 0);
  if (firstPaintPending && paintedRows > 0) {
    current.firstPaintMs = now - openStartedAt;
    firstPaintPending = false;
  }
  if (flushQueued) return;
  flushQueued = true;
  requestAnimationFrame((t) => {
    flushQueued = false;
    frames.push(t);
    while (frames.length && t - frames[0] > 1000) frames.shift();
    current.fps = frames.length;
    if (on || current.firstPaintMs !== 0) setStats({ ...current });
  });
}
