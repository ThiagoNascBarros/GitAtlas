// SPDX-License-Identifier: Apache-2.0
// Draggable seam between two panes. Keyboard: arrows move 16 px, Home/End jump to the limits.
import { createSignal } from "solid-js";

export const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

/** A pane size in px, remembered per key in localStorage (a per-viewer convenience; falls back to `initial`). */
export function storedSize(key: string, initial: number) {
  let saved = NaN;
  try { saved = Number(localStorage.getItem(`ak.split.${key}`)); } catch { /* storage unavailable */ }
  const [size, setSize] = createSignal(saved > 0 ? saved : initial);
  const save = () => { try { localStorage.setItem(`ak.split.${key}`, String(size())); } catch { /* ignore */ } };
  return [size, setSize, save] as const;
}

interface Props {
  label: string;
  /** "x": vertical seam resizing a width; "y": horizontal seam resizing a height. */
  axis: "x" | "y";
  value: number;
  min: number;
  max: number;
  /** -1 when the resized pane sits after the seam (dragging toward it grows it). */
  dir?: 1 | -1;
  onChange: (v: number) => void;
  onCommit: () => void;
}

export function Splitter(props: Props) {
  const dir = () => props.dir ?? 1;
  const set = (v: number) => props.onChange(clamp(Math.round(v), props.min, props.max));
  function start(e: PointerEvent) {
    e.preventDefault();
    const p0 = props.axis === "x" ? e.clientX : e.clientY, v0 = props.value;
    document.body.classList.add(`resizing-${props.axis}`);
    const move = (ev: PointerEvent) => set(v0 + dir() * ((props.axis === "x" ? ev.clientX : ev.clientY) - p0));
    const up = () => {
      document.body.classList.remove(`resizing-${props.axis}`);
      props.onCommit();
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
  function key(e: KeyboardEvent) {
    const fwd = props.axis === "x" ? "ArrowRight" : "ArrowDown", back = props.axis === "x" ? "ArrowLeft" : "ArrowUp";
    const step = { [fwd]: 16 * dir(), [back]: -16 * dir() }[e.key];
    if (step !== undefined) set(props.value + step);
    else if (e.key === "Home") set(props.min);
    else if (e.key === "End") set(props.max);
    else return;
    e.preventDefault();
    props.onCommit();
  }
  return (
    <div
      class={`splitter splitter-${props.axis}`}
      role="separator"
      tabindex="0"
      aria-label={props.label}
      aria-orientation={props.axis === "x" ? "vertical" : "horizontal"}
      aria-valuenow={props.value}
      aria-valuemin={props.min}
      aria-valuemax={props.max}
      onPointerDown={start}
      onKeyDown={key}
    />
  );
}
