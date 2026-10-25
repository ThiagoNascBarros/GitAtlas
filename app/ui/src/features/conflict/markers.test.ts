// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import {
  applyChoice,
  baseLines,
  conflictsOf,
  emptyPick,
  parseSegments,
  render,
  toggleLine,
  unresolvedCount,
  type Conflict,
} from "./markers";

const merged = "a\n<<<<<<< HEAD\nmine\n=======\nyours\n>>>>>>> topic\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> topic\n";
const diff3 = "a\n<<<<<<< HEAD\nmine\n||||||| base\nold\n=======\nyours\n>>>>>>> topic\nb\n<<<<<<< HEAD\nx\n||||||| base\n=======\ny\n>>>>>>> topic\n";

function blocks(text: string): Conflict[] {
  return conflictsOf(parseSegments(text));
}

describe("parseSegments", () => {
  it("splits text and blocks, and round-trips when nothing is picked", () => {
    const segs = parseSegments(merged);
    expect(segs.map((s) => s.kind)).toEqual(["text", "conflict", "text", "conflict", "text"]);
    expect(blocks(merged).map((c) => [c.ours, c.theirs, c.base])).toEqual([
      [["mine"], ["yours"], null],
      [["x"], ["y"], null],
    ]);
    // Re-rendering with all blocks unresolved gives the markers back, byte for byte.
    const picks = blocks(merged).map(emptyPick);
    expect(render(segs, picks)).toBe(merged);
  });

  it("keeps the diff3 base section of each block", () => {
    const [first, second] = blocks(diff3);
    expect(first.base).toEqual(["old"]);
    expect(second.base).toEqual([]);
    expect(first.theirs).toEqual(["yours"]);
  });

  it("keeps an unclosed marker as plain text instead of dropping it", () => {
    const text = "a\n<<<<<<< HEAD\nmine\nb\n";
    expect(blocks(text)).toEqual([]);
    expect(render(parseSegments(text), [])).toBe(text);
  });
});

describe("choices", () => {
  const [c0, c1] = blocks(merged);

  it("whole-block shortcuts pick one side, both, or nothing", () => {
    const seg = parseSegments(merged);
    expect(render(seg, [applyChoice(c0, "theirs"), applyChoice(c1, "ours")])).toBe("a\nyours\nb\nx\n");
    expect(render(seg, [applyChoice(c0, "both"), applyChoice(c1, "bothTheirsFirst")])).toBe("a\nmine\nyours\nb\ny\nx\n");
    expect(render(seg, [applyChoice(c0, "none"), applyChoice(c1, "none")])).toBe("a\nb\n");
  });

  it("checks lines one by one on each side", () => {
    const multi: Conflict = { kind: "conflict", start: "<<<<<<< HEAD", end: ">>>>>>> topic", ours: ["o1", "o2", "o3"], base: null, theirs: ["t1", "t2"] };
    let p = emptyPick(multi);
    p = toggleLine(multi, p, "ours", 0, true);
    p = toggleLine(multi, p, "ours", 2, true);
    p = toggleLine(multi, p, "theirs", 1, true);
    expect(p.resolved).toBe(true);
    expect(render(parseSegments("<<<<<<< HEAD\no1\no2\no3\n=======\nt1\nt2\n>>>>>>> topic\n"), [p])).toBe("o1\no3\nt2\n");
    // Unchecking the only line keeps the block resolved (it is deliberately empty).
    p = toggleLine(multi, p, "ours", 0, false);
    p = toggleLine(multi, p, "ours", 2, false);
    p = toggleLine(multi, p, "theirs", 1, false);
    expect(p.resolved).toBe(true);
    expect(p.ours.some(Boolean) || p.theirs.some(Boolean)).toBe(false);
  });

  it("clear reopens a block and counts what is still open", () => {
    const picks = [applyChoice(c0, "ours"), emptyPick(c1)];
    expect(unresolvedCount(picks)).toBe(1);
    expect(unresolvedCount([applyChoice(c1, "clear")])).toBe(1);
    expect(render(parseSegments(merged), [applyChoice(c0, "ours"), emptyPick(c1)])).toContain("<<<<<<<");
  });
});

describe("baseLines", () => {
  it("pairs the base of each block with the diff3 rendering of the same block", () => {
    expect(baseLines(blocks(merged), parseSegments(diff3))).toEqual([["old"], []]);
  });
  it("gives no base when the two renderings do not line up", () => {
    expect(baseLines(blocks(merged), null)).toEqual([null, null]);
    expect(baseLines(blocks(merged), parseSegments("<<<<<<< HEAD\na\n=======\nb\n>>>>>>> t\n"))).toEqual([null, null]);
  });
});
