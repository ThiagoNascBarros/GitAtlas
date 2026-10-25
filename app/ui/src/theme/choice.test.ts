// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import { parseThemeChoice } from "./choice";

describe("parseThemeChoice (saved theme)", () => {
  it.each(["atlas-dark", "atlas-light", "system"])("accepts %s", (v) => expect(parseThemeChoice(v)).toBe(v));
  it.each([null, "", "solarized", "ATLAS-DARK"])("falls back to system for %s", (v) => expect(parseThemeChoice(v)).toBe("system"));
});
