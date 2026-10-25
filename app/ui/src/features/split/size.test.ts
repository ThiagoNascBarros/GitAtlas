// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { readSize } from "./size";

describe("readSize (saved splitter size)", () => {
  it("keeps an in-range saved value", () => expect(readSize("300", 232, 140, 420)).toBe(300));
  it("clamps a saved value that is now out of range", () => {
    expect(readSize("9000", 232, 140, 420)).toBe(420);
    expect(readSize("20", 232, 140, 420)).toBe(140);
  });
  it("falls back to the initial size for missing or invalid values", () => {
    expect(readSize(null, 232, 140, 420)).toBe(232);
    expect(readSize("abc", 232, 140, 420)).toBe(232);
    expect(readSize("-5", 232, 140, 420)).toBe(232);
  });
});
