// SPDX-License-Identifier: Apache-2.0
import { createResource, createSignal, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { crashNotice } from "./logic";

/**
 * Tells the user that the previous session ended badly, with the recorded panic
 * details and a way to open the log folder (PERF-12). Failures here stay silent:
 * the banner is extra information, never a reason to break the app.
 */
export function CrashBanner() {
  const [status, { mutate }] = createResource(() => api.diagnostics().catch(() => null));
  const [details, setDetails] = createSignal(false);
  const shown = () => {
    const s = status() ?? null;
    return crashNotice(s) ? s : null;
  };
  async function dismiss() {
    mutate(null);
    try {
      await api.dismissCrash();
    } catch {
      // Nothing else to do: the banner is already hidden for this session.
    }
  }
  return (
    <Show when={shown()}>
      {(s) => (
        <output class="crash-banner">
          <span>{s().previousCrash ? t("crash.title") : t("crash.unclean")}</span>
          <Show when={s().previousCrash}>
            <button type="button" aria-expanded={details()} onClick={() => setDetails(!details())}>{t("crash.details")}</button>
          </Show>
          <button type="button" onClick={() => void api.openLogDir().catch(() => undefined)}>{t("crash.openLog")}</button>
          <button type="button" onClick={() => void dismiss()}>{t("crash.dismiss")}</button>
          <Show when={details() && s().previousCrash}>
            <pre class="crash-details">{s().previousCrash!.details}</pre>
          </Show>
        </output>
      )}
    </Show>
  );
}
