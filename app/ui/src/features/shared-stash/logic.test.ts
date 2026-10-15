// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import type { SharedStashInfo } from "../../api";
import { authorSlug, basesIndex, cleanTitle, defaultTitle, formatBytes, groupStashes, initialFetch, reduceFetch, relativeTime, rowActions, visibleOrder } from "./logic";

const mk = (o: Partial<SharedStashInfo>): SharedStashInfo => ({
  remote: "origin", slug: "ana", id: "i1", ref_name: "", wrapper: "w", stash: "s", title: "T", author_name: "Ana", author_email: "a@x",
  base: "b1", base_branch: "main", created: 10, updated: 10, has_untracked: false, mine: false, verified: false, valid: true, invalid_reason: null, ...o,
});

describe("groupStashes", () => {
  it("groups by remote then Mine/Team, newest first", () => {
    const g = groupStashes(
      [mk({ id: "a", updated: 1 }), mk({ id: "b", updated: 5 }), mk({ id: "c", mine: true }), mk({ id: "d", remote: "up" })],
      [{ name: "origin", probe: null }, { name: "up", probe: null }], [],
    );
    expect(g.map((x) => x.remote)).toEqual(["origin", "up"]);
    expect(g[0].team.map((s) => s.id)).toEqual(["b", "a"]);
    expect(g[0].mine.map((s) => s.id)).toEqual(["c"]);
    expect(g[1].team.map((s) => s.id)).toEqual(["d"]);
  });
  it("keeps remotes with no stashes and adds unknown ones", () => {
    const g = groupStashes([mk({ remote: "ghost" })], [{ name: "origin", probe: "rejected" }], []);
    expect(g.map((x) => [x.remote, x.probe])).toEqual([["origin", "rejected"], ["ghost", null]]);
  });
  it("files kept stashes under their remote or the fallback", () => {
    const k = (id: string, remote: string | null) => ({ id, remote, slug: "", wrapper: "", stash: "", title: "", base_branch: "", created: 0, updated: 0, unshared_at: 1 });
    const g = groupStashes([], [{ name: "origin", probe: null }, { name: "up", probe: null }], [k("1", "up"), k("2", null)], "origin");
    expect(g[0].kept.map((x) => x.id)).toEqual(["2"]);
    expect(g[1].kept.map((x) => x.id)).toEqual(["1"]);
  });
  it("hides rows of rejected remotes from keyboard order", () => {
    const g = groupStashes([mk({ id: "a" }), mk({ id: "b", remote: "up" })], [{ name: "origin", probe: "rejected" }, { name: "up", probe: "ok" }], []);
    expect(visibleOrder(g).map((s) => s.id)).toEqual(["b"]);
  });
});

describe("basesIndex", () => {
  it("indexes valid stashes only", () => {
    const m = basesIndex([mk({ id: "a" }), mk({ id: "b", valid: false }), mk({ id: "c", base: "b2" })]);
    expect([...m.keys()].sort()).toEqual(["b1", "b2"]);
    expect(m.get("b1")!.length).toBe(1);
  });
});

describe("formatting", () => {
  it("relative time", () => {
    expect(relativeTime(1000, 990)).toBe("just now");
    expect(relativeTime(1000, 1000 - 180)).toBe("3 min ago");
    expect(relativeTime(100000, 100000 - 7200)).toBe("2 h ago");
    expect(relativeTime(1_000_000, 1_000_000 - 86400 * 3)).toBe("3 d ago");
    expect(relativeTime(10, 50)).toBe("just now");
  });
  it("bytes", () => {
    expect(formatBytes(12)).toBe("12 B");
    expect(formatBytes(2048)).toBe("2 KB");
    expect(formatBytes(12 * 1024 * 1024)).toBe("12.0 MB");
  });
  it("titles are one line and at most 120 characters", () => {
    expect(defaultTitle("  WIP on main: x\nsecond")).toBe("WIP on main: x");
    expect(cleanTitle("é".repeat(200))).toHaveLength(120);
  });
  it("slug preview", () => {
    expect(authorSlug("Thiago Barros")).toBe("thiago-barros");
    expect(authorSlug("José  D'Ávila!")).toBe("jose-d-avila");
    expect(authorSlug("", "ana.p@x.org")).toBe("ana-p");
    expect(authorSlug("   ", "")).toBe("unknown");
  });
});

describe("rowActions", () => {
  it("authors get everything, others apply and branch, invalid none", () => {
    expect(rowActions(mk({ mine: true }))).toEqual(["apply", "branch", "update", "rename", "unshare"]);
    expect(rowActions(mk({}))).toEqual(["apply", "branch"]);
    expect(rowActions(mk({ valid: false, mine: true }))).toEqual([]);
  });
});

describe("reduceFetch", () => {
  const err = { code: "NETWORK_FAILED", message: "x" };
  it("records per-remote errors and clears them on success", () => {
    let s = reduceFetch(initialFetch, { type: "start" });
    expect(s.loading).toBe(true);
    s = reduceFetch(s, { type: "done", now: 5, remote: null, fetched: [{ remote: "a", error: null }, { remote: "b", error: err }] });
    expect(s).toMatchObject({ loading: false, lastFetched: 5, errors: { b: err } });
    s = reduceFetch(s, { type: "done", now: 9, remote: "b", fetched: [{ remote: "b", error: null }] });
    expect(s.errors).toEqual({});
  });
  it("keeps other remotes' errors when one remote is refetched", () => {
    let s = reduceFetch(initialFetch, { type: "done", now: 1, remote: null, fetched: [{ remote: "a", error: err }, { remote: "b", error: err }] });
    s = reduceFetch(s, { type: "done", now: 2, remote: "a", fetched: [{ remote: "a", error: null }] });
    expect(Object.keys(s.errors)).toEqual(["b"]);
  });
  it("a failed call keeps the list state; a cancelled one is silent", () => {
    const s0 = reduceFetch(initialFetch, { type: "done", now: 3, remote: null, fetched: [{ remote: "a", error: err }] });
    const s1 = reduceFetch(reduceFetch(s0, { type: "start" }), { type: "fail", error: err, cancelled: false });
    expect(s1).toMatchObject({ loading: false, failure: err, lastFetched: 3, errors: { a: err } });
    const s2 = reduceFetch(reduceFetch(s0, { type: "start" }), { type: "fail", error: err, cancelled: true });
    expect(s2.failure).toBeNull();
    expect(s2.lastFetched).toBe(3);
  });
});
