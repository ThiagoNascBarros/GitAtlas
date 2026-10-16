// SPDX-License-Identifier: Apache-2.0
import { afterEach, describe, expect, it, vi } from "vitest";
import { activeLocale, en, errorFix, errorText, initLocale, localeChoice, resolveLocale, setLocaleChoice, t } from "./i18n";
import { ptBR } from "./locales/pt-BR";
import { formatBytes, relativeTime } from "./features/shared-stash/logic";

/** An in-memory localStorage; `fail` makes every call throw (private mode, blocked storage). */
function fakeStorage(initial: Record<string, string> = {}, fail = false) {
  const data = new Map(Object.entries(initial));
  const guard = () => { if (fail) throw new Error("storage unavailable"); };
  return {
    data,
    getItem: (k: string) => (guard(), data.get(k) ?? null),
    setItem: (k: string, v: string) => { guard(); data.set(k, v); },
  };
}

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

describe("locale persistence", () => {
  it("restores a valid stored choice and sets <html lang>", () => {
    const html = { lang: "en" };
    vi.stubGlobal("localStorage", fakeStorage({ "ak.lang": "pt-BR" }));
    vi.stubGlobal("document", { documentElement: html });
    initLocale();
    expect(localeChoice()).toBe("pt-BR");
    expect(activeLocale()).toBe("pt-BR");
    expect(html.lang).toBe("pt-BR");
  });
  it("treats a missing or invalid value as system", () => {
    vi.stubGlobal("navigator", { languages: ["pt-BR", "en"], language: "pt-BR" });
    vi.stubGlobal("localStorage", fakeStorage({ "ak.lang": "klingon" }));
    initLocale();
    expect(localeChoice()).toBe("system");
    expect(activeLocale()).toBe("pt-BR");
    vi.stubGlobal("localStorage", fakeStorage());
    initLocale();
    expect(localeChoice()).toBe("system");
  });
  it("uses navigator.language when navigator.languages is empty", () => {
    vi.stubGlobal("navigator", { languages: [], language: "pt-PT" });
    vi.stubGlobal("localStorage", fakeStorage());
    initLocale();
    expect(activeLocale()).toBe("pt-BR");
  });
  it("saves the choice", () => {
    const storage = fakeStorage();
    vi.stubGlobal("localStorage", storage);
    setLocaleChoice("pt-BR");
    expect(storage.data.get("ak.lang")).toBe("pt-BR");
  });
  it("survives a throwing storage", () => {
    vi.stubGlobal("navigator", { languages: ["en-US"], language: "en-US" });
    vi.stubGlobal("localStorage", fakeStorage({}, true));
    expect(() => initLocale()).not.toThrow();
    expect(localeChoice()).toBe("system");
    expect(() => setLocaleChoice("pt-BR")).not.toThrow();
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
