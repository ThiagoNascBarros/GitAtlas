// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import type { RefInfo } from "../../api";
import { filterRefs } from "./filter";

const ref = (name: string, kind: RefInfo["kind"]): RefInfo => ({ name, kind, row: 0, isHead: false });
const refs = [ref("main", "head"), ref("Feature/Login", "head"), ref("origin/main", "remote"), ref("v1.0", "tag")];

describe("filterRefs", () => {
  it("keeps one kind and matches case-insensitively", () => {
    expect(filterRefs(refs, "head", "").map((r) => r.name)).toEqual(["main", "Feature/Login"]);
    expect(filterRefs(refs, "head", "login").map((r) => r.name)).toEqual(["Feature/Login"]);
    expect(filterRefs(refs, "remote", " MAIN ").map((r) => r.name)).toEqual(["origin/main"]);
    expect(filterRefs(refs, "tag", "x")).toEqual([]);
  });
});
