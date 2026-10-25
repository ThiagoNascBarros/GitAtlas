// SPDX-License-Identifier: Apache-2.0
const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

/** A saved pane size (raw localStorage value) kept inside the pane limits; `initial` when missing or invalid. */
export function readSize(raw: string | null, initial: number, min: number, max: number): number {
  const n = Number(raw);
  return raw !== null && n > 0 ? clamp(n, min, max) : initial;
}

/** On-screen px per css px of `el` (the global UI zoom): pointer deltas are divided by it. */
export function visualScale(el: Element | null): number {
  const w = el instanceof HTMLElement ? el.offsetWidth : 0;
  return w ? el!.getBoundingClientRect().width / w : 1;
}
