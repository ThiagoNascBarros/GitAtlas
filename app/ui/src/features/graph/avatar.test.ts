// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { AvatarStore, avatarColor, initials, initialsInk } from "./avatar";
import { contrast } from "./geometry";

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("initials", () => {
  it("uses the first and last word", () => {
    expect(initials("Ana Maria Silva")).toBe("AS");
    expect(initials("ana")).toBe("A");
    expect(initials("  joão   pedro ")).toBe("JP");
  });
  it("falls back to ? and keeps non-ASCII letters whole", () => {
    expect(initials("")).toBe("?");
    expect(initials("Émile")).toBe("É");
    expect(initials("𝒜lice")).toBe("𝒜");
  });
});

describe("avatarColor", () => {
  it("is stable and ignores case and padding", () => {
    expect(avatarColor("a@b.c")).toBe(avatarColor(" A@B.C "));
  });
  it("spreads different e-mails over the palette", () => {
    const set = new Set(Array.from({ length: 40 }, (_, i) => avatarColor(`user${i}@x.y`)));
    expect(set.size).toBeGreaterThan(5);
  });
});

describe("AvatarStore", () => {
  const mk = (fetch: (e: string) => Promise<ArrayBuffer>, limit = 4) => {
    let changes = 0;
    const store = new AvatarStore(fetch, async (b) => ({ size: b.byteLength }) as unknown as ImageBitmap, () => changes++, limit);
    return { store, changes: () => changes };
  };

  it("returns undefined first, then the image, and asks once per author", async () => {
    let calls = 0;
    const { store, changes } = mk(async () => { calls++; return new ArrayBuffer(3); });
    expect(store.get("A@b.c")).toBeUndefined();
    expect(store.get("a@b.c")).toBeUndefined();
    await flush();
    expect(calls).toBe(1);
    expect(store.get("a@b.c")).toEqual({ size: 3 });
    expect(changes()).toBe(1);
  });

  it("treats an empty answer as no photo and never asks again", async () => {
    let calls = 0;
    const { store, changes } = mk(async () => { calls++; return new ArrayBuffer(0); });
    store.get("n@b.c");
    await flush();
    expect(store.get("n@b.c")).toBeUndefined();
    store.get("n@b.c");
    await flush();
    expect(calls).toBe(1);
    expect(changes()).toBe(0);
  });

  it("does not retry after a failed request in the same session", async () => {
    let calls = 0;
    const { store } = mk(async () => { calls++; throw new Error("down"); });
    store.get("e@b.c");
    await flush();
    store.get("e@b.c");
    await flush();
    expect(calls).toBe(1);
  });

  it("keeps at most `limit` requests in flight and drains the queue", async () => {
    let live = 0, peak = 0;
    const gates: (() => void)[] = [];
    const { store } = mk(() => new Promise<ArrayBuffer>((res) => {
      live++; peak = Math.max(peak, live);
      gates.push(() => { live--; res(new ArrayBuffer(1)); });
    }), 2);
    for (let i = 0; i < 5; i++) store.get(`u${i}@x.y`);
    await flush();
    expect(peak).toBe(2);
    while (gates.length) { gates.shift()!(); await flush(); }
    expect(peak).toBe(2);
    expect(store.get("u4@x.y")).toBeDefined();
  });
});

describe("initialsInk", () => {
  it("is black on light backgrounds and white on dark ones", () => {
    expect(initialsInk("#f2ca33")).toBe("#000000");
    expect(initialsInk("#0669f7")).toBe("#ffffff");
  });
  it("reaches 3:1 (large bold text) on every fallback colour", () => {
    for (let i = 0; i < 200; i++) {
      const bg = avatarColor(`u${i}@x.y`);
      expect(contrast(initialsInk(bg), bg), bg).toBeGreaterThanOrEqual(3);
    }
  });
});
