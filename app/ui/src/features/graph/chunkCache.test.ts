// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { ChunkCache } from "./chunkCache";

describe("ChunkCache", () => {
  it("evicts the least recently used entry past the limit", () => {
    const c = new ChunkCache<number>(2);
    c.set("a", 1);
    c.set("b", 2);
    c.set("c", 3);
    expect(c.has("a")).toBe(false);
    expect(c.get("b")).toBe(2);
    expect(c.get("c")).toBe(3);
    expect(c.size).toBe(2);
  });

  it("a read promotes the entry so it survives the next eviction", () => {
    const c = new ChunkCache<number>(2);
    c.set("a", 1);
    c.set("b", 2);
    c.get("a");
    c.set("c", 3);
    expect(c.has("a")).toBe(true);
    expect(c.has("b")).toBe(false);
  });

  it("overwriting a key does not grow the cache", () => {
    const c = new ChunkCache<number>(2);
    c.set("a", 1);
    c.set("a", 2);
    c.set("b", 3);
    expect(c.size).toBe(2);
    expect(c.get("a")).toBe(2);
  });

  it("deleteWhere and clear release entries", () => {
    const c = new ChunkCache<number>(10);
    c.set("1:0", 1);
    c.set("1:1", 2);
    c.set("2:0", 3);
    c.deleteWhere((k) => k.startsWith("1:"));
    expect(c.has("1:0") || c.has("1:1")).toBe(false);
    expect(c.get("2:0")).toBe(3);
    c.clear();
    expect(c.size).toBe(0);
  });

  it("rejects a limit below one", () => {
    expect(() => new ChunkCache<number>(0)).toThrow(RangeError);
  });
});
