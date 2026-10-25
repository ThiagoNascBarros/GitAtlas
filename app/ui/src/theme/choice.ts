// SPDX-License-Identifier: Apache-2.0
export type ThemeChoice = "atlas-dark" | "atlas-light" | "system";

/** Storage key of the chosen theme (the language uses `ak.lang`). */
export const THEME_KEY = "ak.theme";

/** A saved theme value, or "system" when missing or not a known choice. */
export const parseThemeChoice = (raw: string | null): ThemeChoice =>
  raw === "atlas-dark" || raw === "atlas-light" ? raw : "system";
