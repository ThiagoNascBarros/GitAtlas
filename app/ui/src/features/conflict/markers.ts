// SPDX-License-Identifier: Apache-2.0
// Conflict markers written by `git merge` (also the diff3 base section `|||||||`).

export interface Chunk { start: number; end: number; ours: string[]; theirs: string[] }

/** Finds every `<<<<<<< ... >>>>>>>` block; line indexes are inclusive. */
export function findChunks(text: string): Chunk[] {
  const lines = text.split("\n");
  const out: Chunk[] = [];
  let cur: Chunk | null = null;
  let side: "ours" | "base" | "theirs" = "ours";
  lines.forEach((l, i) => {
    if (l.startsWith("<<<<<<<")) {
      cur = { start: i, end: i, ours: [], theirs: [] };
      side = "ours";
    } else if (!cur) {
      return;
    } else if (l.startsWith("|||||||")) {
      side = "base";
    } else if (l.startsWith("=======")) {
      side = "theirs";
    } else if (l.startsWith(">>>>>>>")) {
      cur.end = i;
      out.push(cur);
      cur = null;
    } else if (side !== "base") {
      cur[side].push(l);
    }
  });
  return out;
}

/** Replaces conflict block `index` with the chosen side(s). */
export function pick(text: string, index: number, choice: "ours" | "theirs" | "both"): string {
  const c = findChunks(text)[index];
  if (!c) return text;
  const lines = text.split("\n");
  const keep = choice === "ours" ? c.ours : choice === "theirs" ? c.theirs : [...c.ours, ...c.theirs];
  lines.splice(c.start, c.end - c.start + 1, ...keep);
  return lines.join("\n");
}
