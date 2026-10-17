// SPDX-License-Identifier: Apache-2.0
// Preferences store: the shell owns the files (settings.json, global and per repository); this
// module reads the effective values, applies them and writes changes back.
import { createSignal } from "solid-js";
import { api, EffectivePrefs, Prefs, PrefsView } from "../api";
import { setLocaleChoice } from "../i18n";
import { setTheme } from "../theme/theme";

/** Zoom range from the PRD: 80 % to 200 %, in steps of 10 %. */
export const ZOOM_MIN = 0.8;
export const ZOOM_MAX = 2;
export const ZOOM_STEP = 0.1;

export const DEFAULT_PREFS: EffectivePrefs = { theme: "system", zoom: 1, language: "system", animations: "on" };

export const clampZoom = (z: number) => Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z)) * 100) / 100;
export const zoomStep = (z: number, dir: 1 | -1) => clampZoom(z + dir * ZOOM_STEP);

/**
 * Which scope a zoom change writes to: the repository when it already overrides zoom, else the
 * global scope, so a shortcut never creates a repository override by accident.
 */
export function zoomScope(view: PrefsView | null, repoId: number | null): "global" | "repo" {
  return repoId !== null && view?.repo?.zoom !== undefined ? "repo" : "global";
}

const [view, setView] = createSignal<PrefsView | null>(null);
/** What each scope sets and the effective values (null until the shell answers). */
export const prefsView = view;
/** The effective values the UI applies. */
export const effectivePrefs = (): EffectivePrefs => view()?.effective ?? DEFAULT_PREFS;

/** Zoom goes on the root element, so every px-based size scales with it. */
export function applyZoom(z: number) {
  if (typeof document !== "undefined") document.documentElement.style.zoom = String(z);
}

/** `reduced` turns CSS transitions and animations off (see settings.css). */
export function applyAnimations(a: EffectivePrefs["animations"]) {
  if (typeof document !== "undefined") document.documentElement.dataset.motion = a;
}

function applyEffective(e: EffectivePrefs) {
  setTheme(e.theme);
  setLocaleChoice(e.language);
  applyZoom(e.zoom);
  applyAnimations(e.animations);
}

/** Reads the preferences of `repoId` (or the global ones) and applies them. Unavailable shells keep the defaults. */
export async function loadPrefs(repoId: number | null = null): Promise<void> {
  try {
    const v = await api.prefsGet(repoId);
    setView(v);
    applyEffective(v.effective);
  } catch {
    if (!view()) applyEffective(DEFAULT_PREFS);
  }
}

/** Writes `patch` to a scope and applies the result. Rejects with the shell error when the write fails. */
export async function setPref(scope: "global" | "repo", repoId: number | null, patch: Record<string, unknown>): Promise<void> {
  const v = await api.prefsSet(repoId, scope, patch);
  setView(v);
  applyEffective(v.effective);
}

/** Changes the zoom by one step (or resets it to 100 %) in the scope that owns zoom. */
export async function changeZoom(repoId: number | null, change: { dir: 1 | -1 } | "reset"): Promise<void> {
  const scope = zoomScope(view(), repoId);
  const current = effectivePrefs().zoom;
  const next = change === "reset" ? 1 : zoomStep(current, change.dir);
  await setPref(scope, repoId, { zoom: next } satisfies Prefs);
}
