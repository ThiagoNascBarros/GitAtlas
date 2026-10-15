// SPDX-License-Identifier: Apache-2.0
import type { TodoItem } from "../../api";

/** Moves item `i` by `delta` (-1 up, +1 down); returns a new list. */
export function move(list: TodoItem[], i: number, delta: number): TodoItem[] {
  const j = i + delta;
  if (j < 0 || j >= list.length) return list;
  const out = [...list];
  [out[i], out[j]] = [out[j], out[i]];
  return out;
}

/** Squash/fixup need a kept commit before them (dropped ones do not count). */
export function firstIsSquash(list: TodoItem[]): boolean {
  const first = list.find((x) => x.action !== "drop");
  return !!first && (first.action === "squash" || first.action === "fixup");
}
