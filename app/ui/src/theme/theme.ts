// SPDX-License-Identifier: Apache-2.0
// Theme engine (design-tokens §7, SDD §9): validates theme files, writes
// tokens as `--ak-*` CSS custom properties and switches live.
import { createSignal } from "solid-js";
import dark from "../../../../themes/atlas-dark.theme.json";
import light from "../../../../themes/atlas-light.theme.json";

export interface Theme { id: string; name: string; kind: string; extends?: string; tokens: Record<string, string> }

/** Every theme must define these (completeness rule). */
export const REQUIRED_TOKENS = Object.keys(dark.tokens);

export const BUILTIN: Theme[] = [dark, light];

export function validate(theme: Theme): string[] {
  return REQUIRED_TOKENS.filter((k) => !theme.tokens[k]).map((k) => `${theme.id}: missing ${k}`);
}

export const cssVar = (token: string) => `--ak-${token.replace(/\./g, "-")}`;

export type ThemeChoice = "atlas-dark" | "atlas-light" | "system";
const [choice, setChoice] = createSignal<ThemeChoice>("system");
const [active, setActive] = createSignal<Theme>(dark);
export { active as activeTheme, choice as themeChoice };

function resolve(c: ThemeChoice): Theme {
  if (c === "system") return matchMedia("(prefers-color-scheme: light)").matches ? light : dark;
  return BUILTIN.find((t) => t.id === c) ?? dark;
}

function apply(theme: Theme) {
  const errors = validate(theme);
  const t = errors.length ? dark : theme; // invalid → fall back to the default theme
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(t.tokens)) root.setProperty(cssVar(k), v);
  document.documentElement.dataset.theme = t.kind;
  setActive(t);
}

export function setTheme(c: ThemeChoice) {
  setChoice(c);
  apply(resolve(c));
}

/** Follows system changes and applies the current choice (set by the settings store before the first render). */
export function initTheme() {
  matchMedia("(prefers-color-scheme: light)").addEventListener("change", () => {
    if (choice() === "system") apply(resolve("system"));
  });
  setTheme(choice());
}

export const laneColor = (slot: number) => active().tokens[`color.lane.${slot % 10}`];
export const token = (k: string) => active().tokens[k];
