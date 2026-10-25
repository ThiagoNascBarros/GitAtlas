// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
// @ts-expect-error no @types/node in this project; vitest runs in node, so the import resolves
import { existsSync, readFileSync } from "node:fs";

const css: string = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
/** Value of a custom property as the cascade resolves it for :root: the last declaration wins. */
const lastDecl = (name: string) => [...css.matchAll(new RegExp(`${name}:\\s*([^;]+);`, "g"))].at(-1)?.[1].trim();

describe("panel typography", () => {
  it("--font-ui leads with Open Sans", () => {
    expect(lastDecl("--font-ui")).toMatch(/^"?Open Sans"?,/);
  });
});

describe("panel density", () => {
  it("--row-h is 22px (dense GitKraken rows)", () => {
    expect(lastDecl("--row-h")).toBe("22px");
  });
});

describe("control borders (UIV-5)", () => {
  const lum = (hex: string) => {
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const ratio = (a: string, b: string) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  it.each(["dark", "light"])("input/select border in %s theme is >= 3:1 against bg.raised", async (kind) => {
    const t = (await import(`../../../themes/atlas-${kind}.theme.json`)).default.tokens as Record<string, string>;
    const input = css.match(/^input:not\(\[type="checkbox"\]\)[^{]*\{([^}]*)\}/m)![1];
    const sel = css.match(/^select \{([^}]*)\}/m)![1];
    for (const rule of [input, sel]) expect(rule).toContain("border: 1px solid var(--ak-color-border-strong)");
    expect(ratio(t["color.border.strong"], t["color.bg.raised"])).toBeGreaterThanOrEqual(3);
  });
});

describe("bundled Open Sans", () => {
  const faces = [...css.matchAll(/@font-face\s*{[^}]*}/g)].map((m) => m[0]).filter((f) => /Open Sans/.test(f));

  it("declares weights 400 and 600 as font-face", () => {
    expect(faces.some((f) => /font-weight:\s*400/.test(f))).toBe(true);
    expect(faces.some((f) => /font-weight:\s*600/.test(f))).toBe(true);
  });
  it("points every face at a file shipped in public/", () => {
    expect(faces.length).toBeGreaterThan(0);
    for (const f of faces) for (const m of f.matchAll(/url\(["']?\/(fonts\/[^"')]+)["']?\)/g))
      expect(existsSync(new URL(`../public/${m[1]}`, import.meta.url)), m[1]).toBe(true);
  });
  it("ships the OFL licence next to the fonts", () => {
    expect(existsSync(new URL("../public/fonts/OFL.txt", import.meta.url))).toBe(true);
  });
});
