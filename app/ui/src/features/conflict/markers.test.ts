// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { findChunks, pick } from "./markers";

const text = "a\n<<<<<<< HEAD\nmine\n||||||| base\nold\n=======\nyours\n>>>>>>> topic\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> topic\n";

describe("conflict markers", () => {
  it("finds blocks and ignores the diff3 base section", () => {
    const c = findChunks(text);
    expect(c.map((x) => [x.ours, x.theirs])).toEqual([[["mine"], ["yours"]], [["x"], ["y"]]]);
  });
  it("replaces one block with the chosen side", () => {
    expect(pick(text, 0, "theirs")).toBe("a\nyours\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> topic\n");
    expect(pick(pick(text, 1, "both"), 0, "ours")).toBe("a\nmine\nb\nx\ny\n");
  });
});
