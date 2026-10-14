// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { parseUnified, toSplit } from "./parse";

const PATCH = `diff --git a/src/graph.txt b/src/graph.txt
index 1..2 100644
--- a/src/graph.txt
+++ b/src/graph.txt
@@ -1,3 +1,3 @@
 lanes
-old
+new
+extra
 tail
\\ No newline at end of file
`;

describe("parseUnified", () => {
  it("numbers lines and classifies them", () => {
    const d = parseUnified(PATCH);
    expect(d.hunks).toHaveLength(1);
    const l = d.hunks[0].lines;
    expect(l.map((x) => x.kind)).toEqual(["ctx", "del", "add", "add", "ctx"]);
    expect(l[1]).toMatchObject({ oldNo: 2, newNo: null });
    expect(l[3]).toMatchObject({ newNo: 3, oldNo: null });
    expect(l[4]).toMatchObject({ oldNo: 3, newNo: 4 });
  });
  it("detects binary files", () => {
    expect(parseUnified("Binary files a/x and b/x differ\n").binary).toBe(true);
  });
  it("pairs del/add rows in split view", () => {
    const rows = toSplit(parseUnified(PATCH).hunks[0]);
    expect(rows).toHaveLength(4);
    expect(rows[1].left?.text).toBe("old");
    expect(rows[1].right?.text).toBe("new");
    expect(rows[2].left).toBeNull();
  });
});
