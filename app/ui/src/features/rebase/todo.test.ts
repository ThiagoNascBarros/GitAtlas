// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import type { TodoItem } from "../../api";
import { firstIsSquash, move } from "./todo";

const it3 = (a: TodoItem["action"][]): TodoItem[] => a.map((action, i) => ({ sha: `s${i}`, action, message: `m${i}` }));

describe("rebase todo", () => {
  it("moves items and ignores moves past the ends", () => {
    expect(move(it3(["pick", "pick", "pick"]), 2, -1).map((x) => x.sha)).toEqual(["s0", "s2", "s1"]);
    expect(move(it3(["pick"]), 0, -1).map((x) => x.sha)).toEqual(["s0"]);
  });
  it("flags a squash with nothing kept before it", () => {
    expect(firstIsSquash(it3(["drop", "squash", "pick"]))).toBe(true);
    expect(firstIsSquash(it3(["pick", "fixup"]))).toBe(false);
  });
});
