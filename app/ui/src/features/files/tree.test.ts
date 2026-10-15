// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { toTreeRows } from "./tree";

describe("toTreeRows", () => {
  it("groups by directory, dirs before files, collapsing single-child chains", () => {
    const rows = toTreeRows([
      { status: "M", path: "README.md" },
      { status: "A", path: "src/ui/app.ts" },
      { status: "M", path: "src/ui/graph.ts" },
      { status: "D", path: "docs/guide/intro.md" },
    ]);
    expect(rows.map((r) => `${r.depth}:${r.kind}:${r.name}`)).toEqual([
      "0:dir:docs/guide",
      "1:file:intro.md",
      "0:dir:src/ui",
      "1:file:app.ts",
      "1:file:graph.ts",
      "0:file:README.md",
    ]);
  });
  it("handles an empty list", () => expect(toTreeRows([])).toEqual([]));
});

describe("visibleRows", () => {
  it("hides what is inside a collapsed directory, keeping the directory row", async () => {
    const { toTreeRows, visibleRows } = await import("./tree");
    const rows = toTreeRows([{ status: "M", path: "a/b/x.ts" }, { status: "M", path: "a/c.ts" }, { status: "A", path: "z.ts" }]);
    const names = (c: string[]) => visibleRows(rows, new Set(c)).map((r) => r.name);
    expect(names([])).toEqual(["a", "b", "x.ts", "c.ts", "z.ts"]);
    expect(names(["a/b"])).toEqual(["a", "b", "c.ts", "z.ts"]);
    expect(names(["a"])).toEqual(["a", "z.ts"]);
  });
});
