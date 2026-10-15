import { describe, expect, it } from "vitest";
import { changeCount, groupStatus } from "./status";

const e = (path: string, index: string, worktree: string, extra = {}) =>
  ({ path, orig_path: null, index, worktree, conflicted: false, untracked: false, ...extra });

describe("groupStatus", () => {
  it("splits staged, unstaged, untracked and conflicts", () => {
    const g = groupStatus([
      e("b.txt", "M", "."),
      e("a.txt", ".", "M"),
      e("both.txt", "M", "M"),
      e("new.txt", ".", "?", { untracked: true }),
      e("c.txt", "U", "U", { conflicted: true }),
    ]);
    expect(g.staged.map((i) => i.path)).toEqual(["b.txt", "both.txt"]);
    expect(g.unstaged.map((i) => [i.path, i.status])).toEqual([["a.txt", "M"], ["both.txt", "M"], ["new.txt", "U"]]);
    expect(g.conflicted.map((i) => i.path)).toEqual(["c.txt"]);
    expect(changeCount(g)).toBe(5);
  });
});
