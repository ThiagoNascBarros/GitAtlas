// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { canCreate, effectiveBase, flowBranches, isFlowKind, kindOptions } from "./branchDialog.logic";

describe("kindOptions", () => {
  it("offers Git Flow types only when develop exists", () => {
    expect(kindOptions(false)).toEqual(["free", "fix"]);
    expect(kindOptions(true)).toEqual(["free", "feature", "fix", "release", "hotfix"]);
  });
});

describe("effectiveBase", () => {
  it("takes the core's base for Git Flow kinds and the chosen one otherwise", () => {
    expect(effectiveBase("feature", "develop", "HEAD")).toBe("develop");
    expect(effectiveBase("hotfix", "main", "develop")).toBe("main");
    expect(effectiveBase("fix", "HEAD", "main")).toBe("main");
    expect(effectiveBase("free", undefined, "HEAD")).toBe("HEAD");
  });

  it("has no base for a Git Flow kind before the core answered", () => {
    expect(isFlowKind("release")).toBe(true);
    expect(effectiveBase("release", undefined, "HEAD")).toBeNull();
  });
});

describe("canCreate", () => {
  it("needs a non-empty preview, no failure and a base", () => {
    expect(canCreate({ name: "feature/ACT-1" }, false, "develop")).toBe(true);
    expect(canCreate({ name: "" }, false, "HEAD")).toBe(false);
    expect(canCreate(null, false, "HEAD")).toBe(false);
    expect(canCreate({ name: "x" }, true, "HEAD")).toBe(false);
    expect(canCreate({ name: "x" }, false, null)).toBe(false);
  });
});

describe("flowBranches", () => {
  it("enables the flow when develop exists and finds main or master", () => {
    expect(flowBranches(["main", "develop"])).toEqual({ flowEnabled: true, main: "main", develop: "develop" });
    expect(flowBranches(["master"])).toEqual({ flowEnabled: false, main: "master", develop: "develop" });
    expect(flowBranches(["feat"])).toEqual({ flowEnabled: false, main: "main", develop: "develop" });
  });
});
