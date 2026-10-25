// SPDX-License-Identifier: Apache-2.0
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
/** Value of a custom property as the cascade resolves it for :root: the last declaration wins. */
const lastDecl = (name: string) => [...css.matchAll(new RegExp(`${name}:\\s*([^;]+);`, "g"))].at(-1)?.[1].trim();

describe("panel typography", () => {
  it("--font-ui leads with Open Sans", () => {
    expect(lastDecl("--font-ui")).toMatch(/^"?Open Sans"?,/);
  });
});
