// SPDX-License-Identifier: Apache-2.0
// Settings screen (FUN-06, LAY-04): theme, zoom, language and animations, for all repositories or
// for the open one. Values are kept in settings.json by the shell (see settings/prefs.ts).
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { RotateCcw, X, ZoomIn, ZoomOut } from "lucide-solid";
import type { EffectivePrefs, Prefs, UiError } from "../../api";
import { errorText, t } from "../../i18n";
import { clampZoom, effectivePrefs, prefsView, setPref, zoomStep, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP } from "../../settings/prefs";
import "./settings.css";

interface Props {
  /** The open repository, or null when none is open (then only the global scope is offered). */
  repoId: number | null;
  repoName: string | null;
  onClose: () => void;
}

type Scope = "global" | "repo";
type Key = keyof EffectivePrefs;

/** The shell's error text when it sent one, else a generic message. */
function failure(e: unknown): string {
  const u = e as Partial<UiError> | undefined;
  return u?.code ? errorText({ code: u.code, message: u.message ?? "" }) : t("settings.saveFailed");
}

export function SettingsPanel(props: Props) {
  let closeRef: HTMLButtonElement | undefined;
  const [scope, setScope] = createSignal<Scope>("global");
  const [error, setError] = createSignal<string | null>(null);
  // The repository scope exists only while a repository is open.
  const activeScope = (): Scope => (scope() === "repo" && props.repoId !== null ? "repo" : "global");
  /** What the scope being edited sets itself. */
  const own = (): Prefs => (activeScope() === "repo" ? prefsView()?.repo : prefsView()?.global) ?? {};
  /** The value shown: the scope's own, else the effective one (which is the inherited value). */
  const shown = <K extends Key>(key: K): EffectivePrefs[K] => (own()[key] as EffectivePrefs[K] | undefined) ?? effectivePrefs()[key];
  /** Only a repository scope can inherit: no override means the global value applies. */
  const inherits = (key: Key) => activeScope() === "repo" && own()[key] === undefined;

  async function write(patch: Record<string, unknown>) {
    setError(null);
    try {
      await setPref(activeScope(), props.repoId, patch);
    } catch (e) {
      setError(failure(e));
    }
  }

  /** Repository scope: "same as global" removes the override; turning it off copies the value shown. */
  const setInherit = (key: Key, inherit: boolean) => write({ [key]: inherit ? null : shown(key) });

  onMount(() => {
    closeRef?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") props.onClose();
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  const percent = () => Math.round(shown("zoom") * 100);

  return (
    <div class="settings-backdrop" onClick={(e) => e.target === e.currentTarget && props.onClose()}>
      <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <header class="settings-head">
          <h2 id="settings-title">{t("settings.title")}</h2>
          <button ref={closeRef} class="icon" aria-label={t("settings.close")} onClick={props.onClose}>
            <X size={16} aria-hidden="true" />
          </button>
        </header>
        <p class="settings-note muted">{t("settings.note")}</p>

        <div class="settings-row">
          <label for="settings-scope">{t("settings.scope")}</label>
          <select id="settings-scope" value={activeScope()} onChange={(e) => setScope(e.currentTarget.value as Scope)}>
            <option value="global">{t("settings.scope.global")}</option>
            <Show when={props.repoId !== null}>
              <option value="repo">{t("settings.scope.repo", { s: props.repoName ?? "" })}</option>
            </Show>
          </select>
          <span />
        </div>

        <div class="settings-row">
          <label for="settings-theme">{t("settings.theme")}</label>
          <select id="settings-theme" disabled={inherits("theme")} value={shown("theme")} onChange={(e) => void write({ theme: e.currentTarget.value })}>
            <option value="system">{t("theme.system")}</option>
            <option value="atlas-dark">{t("theme.dark")}</option>
            <option value="atlas-light">{t("theme.light")}</option>
          </select>
          <Inherit show={activeScope() === "repo"} checked={inherits("theme")} onChange={(v) => void setInherit("theme", v)} />
        </div>

        <div class="settings-row">
          <label for="settings-zoom">{t("settings.zoom")}</label>
          <div class="settings-zoom">
            <button class="icon" disabled={inherits("zoom")} aria-label={t("settings.zoomOut")} title={t("settings.zoomOut")} onClick={() => void write({ zoom: zoomStep(shown("zoom"), -1) })}>
              <ZoomOut size={16} aria-hidden="true" />
            </button>
            <input
              id="settings-zoom"
              type="range"
              min={ZOOM_MIN * 100}
              max={ZOOM_MAX * 100}
              step={ZOOM_STEP * 100}
              value={percent()}
              disabled={inherits("zoom")}
              aria-valuetext={`${percent()} %`}
              onChange={(e) => void write({ zoom: clampZoom(Number(e.currentTarget.value) / 100) })}
            />
            <button class="icon" disabled={inherits("zoom")} aria-label={t("settings.zoomIn")} title={t("settings.zoomIn")} onClick={() => void write({ zoom: zoomStep(shown("zoom"), 1) })}>
              <ZoomIn size={16} aria-hidden="true" />
            </button>
            <button class="icon" disabled={inherits("zoom")} aria-label={t("settings.zoomReset")} title={t("settings.zoomReset")} onClick={() => void write({ zoom: 1 })}>
              <RotateCcw size={16} aria-hidden="true" />
            </button>
            <output aria-live="polite">{percent()} %</output>
          </div>
          <Inherit show={activeScope() === "repo"} checked={inherits("zoom")} onChange={(v) => void setInherit("zoom", v)} />
        </div>

        <div class="settings-row">
          <label for="settings-language">{t("settings.language")}</label>
          <select id="settings-language" disabled={inherits("language")} value={shown("language")} onChange={(e) => void write({ language: e.currentTarget.value })}>
            <option value="system">{t("lang.system")}</option>
            <option value="en" lang="en">{t("lang.en")}</option>
            <option value="pt-BR" lang="pt-BR">{t("lang.ptBR")}</option>
          </select>
          <Inherit show={activeScope() === "repo"} checked={inherits("language")} onChange={(v) => void setInherit("language", v)} />
        </div>

        <div class="settings-row">
          <label for="settings-animations">{t("settings.animations")}</label>
          <input
            id="settings-animations"
            type="checkbox"
            disabled={inherits("animations")}
            checked={shown("animations") === "reduced"}
            onChange={(e) => void write({ animations: e.currentTarget.checked ? "reduced" : "on" })}
          />
          <Inherit show={activeScope() === "repo"} checked={inherits("animations")} onChange={(v) => void setInherit("animations", v)} />
        </div>

        <Show when={error()}>
          <p class="settings-error" role="alert">{error()}</p>
        </Show>
      </div>
    </div>
  );
}

/** "Same as global" checkbox, shown only for a repository scope. */
function Inherit(props: { show: boolean; checked: boolean; onChange: (inherit: boolean) => void }) {
  return (
    <Show when={props.show} fallback={<span />}>
      <label class="settings-inherit">
        <input type="checkbox" checked={props.checked} onChange={(e) => props.onChange(e.currentTarget.checked)} />
        {t("settings.inherit")}
      </label>
    </Show>
  );
}
