// SPDX-License-Identifier: Apache-2.0
import { afterEach, describe, expect, it, vi } from "vitest";
import { activeLocale, en, errorFix, errorText, initLocale, localeChoice, resolveLocale, setLocaleChoice, t } from "./i18n";
import { ptBR } from "./locales/pt-BR";
import { formatBytes, relativeTime } from "./features/shared-stash/logic";

afterEach(() => {
  vi.unstubAllGlobals();
  setLocaleChoice("en");
});

describe("resolveLocale", () => {
  it("returns explicit choices as they are", () => {
    expect(resolveLocale("en", ["pt-BR"])).toBe("en");
    expect(resolveLocale("pt-BR", ["en-US"])).toBe("pt-BR");
  });
  it("follows the first pt or en system language, any region", () => {
    expect(resolveLocale("system", ["pt"])).toBe("pt-BR");
    expect(resolveLocale("system", ["pt-BR"])).toBe("pt-BR");
    expect(resolveLocale("system", ["pt-PT"])).toBe("pt-BR");
    expect(resolveLocale("system", ["pt_BR"])).toBe("pt-BR");
    expect(resolveLocale("system", ["en-US", "pt-BR"])).toBe("en");
    expect(resolveLocale("system", ["fr-FR", "pt-BR"])).toBe("pt-BR");
  });
  it("falls back to en", () => {
    expect(resolveLocale("system", [])).toBe("en");
    expect(resolveLocale("system", ["fr", "de"])).toBe("en");
  });
});

describe("pt-BR table", () => {
  const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

  it("has exactly the keys of en", () => {
    expect(Object.keys(ptBR).sort()).toEqual(Object.keys(en).sort());
  });
  it("keeps the placeholders of every key", () => {
    for (const k of Object.keys(en) as (keyof typeof en)[]) {
      expect(placeholders(ptBR[k]), k).toEqual(placeholders(en[k]));
    }
  });
  it("has no empty text", () => {
    for (const [k, v] of Object.entries(ptBR)) expect(v.trim(), k).not.toBe("");
  });
  it("has a text for every error code the core and the shell can return", () => {
    for (const code of ["INVALID_PATH", "IO_ERROR", "GIT_NOT_FOUND", "NOT_A_REPO", "READ_FAILED", "UNKNOWN_REPO"]) {
      expect(en, code).toHaveProperty(`err.${code}`);
      expect(ptBR, code).toHaveProperty(`err.${code}`);
    }
  });
});

describe("t, errorText and errorFix", () => {
  it("follow the active locale live", () => {
    setLocaleChoice("en");
    expect(t("toolbar.undo")).toBe("Undo");
    setLocaleChoice("pt-BR");
    expect(activeLocale()).toBe("pt-BR");
    expect(t("toolbar.undo")).toBe(ptBR["toolbar.undo"]);
    expect(t("status.commits", { n: 3 })).toBe(ptBR["status.commits"].replace("{n}", "3"));
    expect(errorText({ code: "REPO_BUSY", message: "core text" })).toBe(ptBR["err.REPO_BUSY"]);
    expect(errorFix({ code: "BASE_MISSING" })).toBe(ptBR["err.BASE_MISSING.fix"]);
  });
  it("keep a placeholder whose value is missing", () => {
    setLocaleChoice("pt-BR");
    expect(t("tabs.close")).toContain("{s}");
  });
  it("fall back to the core text for unknown codes", () => {
    setLocaleChoice("pt-BR");
    expect(errorText({ code: "SOMETHING_NEW", message: "core text" })).toBe("core text");
    expect(errorFix({ code: "SOMETHING_NEW", suggestion: "core hint" })).toBe("core hint");
    expect(errorFix({ code: "SOMETHING_NEW" })).toBeUndefined();
  });
});

// The choice is saved in settings.json by the settings store, so the language module never touches storage.
describe("locale choice", () => {
  it("applies the chosen language and sets <html lang>", () => {
    const html = { lang: "en" };
    vi.stubGlobal("document", { documentElement: html });
    setLocaleChoice("pt-BR");
    expect(localeChoice()).toBe("pt-BR");
    expect(activeLocale()).toBe("pt-BR");
    expect(html.lang).toBe("pt-BR");
  });
  it("system follows the system languages", () => {
    vi.stubGlobal("navigator", { languages: ["pt-BR", "en"], language: "pt-BR" });
    setLocaleChoice("system");
    expect(localeChoice()).toBe("system");
    expect(activeLocale()).toBe("pt-BR");
  });
  it("initLocale re-applies the current choice and never reads or writes storage", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => { throw new Error("storage must not be read"); },
      setItem: () => { throw new Error("storage must not be written"); },
    });
    setLocaleChoice("pt-BR");
    expect(() => initLocale()).not.toThrow();
    expect(activeLocale()).toBe("pt-BR");
  });
  it("uses navigator.language when navigator.languages is empty", () => {
    vi.stubGlobal("navigator", { languages: [], language: "pt-PT" });
    setLocaleChoice("system");
    expect(activeLocale()).toBe("pt-BR");
  });
});

describe("locale-aware formatting", () => {
  it("formats sizes and old dates with the active locale", () => {
    setLocaleChoice("en");
    expect(formatBytes(12 * 1024 * 1024 + 100 * 1024)).toBe("12.1 MB");
    const old = Date.UTC(2024, 0, 31, 12) / 1000;
    expect(relativeTime(old + 86400 * 60, old)).toBe(new Date(old * 1000).toLocaleDateString("en"));
    setLocaleChoice("pt-BR");
    expect(formatBytes(12 * 1024 * 1024 + 100 * 1024)).toBe("12,1 MB");
    expect(relativeTime(old + 86400 * 60, old)).toBe(new Date(old * 1000).toLocaleDateString("pt-BR"));
    expect(relativeTime(1000, 1000 - 180)).toBe(ptBR["time.minutes"].replace("{n}", "3"));
  });
});
