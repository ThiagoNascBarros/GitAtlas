// SPDX-License-Identifier: Apache-2.0
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";

const KEEP = 50;

/** Operation journal: what AtlasKraken did, with the exact Git commands, plus opt-in cleanup. */
export function JournalPanel(props: { repoId: number; onClose: () => void; onError: (e: unknown) => void }) {
  const [entries, { refetch }] = createResource(() => api.journal(props.repoId));
  let close: HTMLButtonElement | undefined;
  useOverlay(() => props.onClose(), () => close);
  const [confirm, setConfirm] = createSignal(false);
  async function prune() {
    // Pruning cannot be undone: ask for a second click.
    if (!confirm()) return setConfirm(true);
    setConfirm(false);
    try {
      await api.pruneJournal(props.repoId, KEEP);
      refetch();
    } catch (e) {
      props.onError(e);
    }
  }
  return (
    <div class="modal-back" onClick={props.onClose}>
      <dialog open class="modal" aria-label={t("journal.title")} onClick={(e) => e.stopPropagation()}>
        <h2>{t("journal.title")}</h2>
        <Show when={(entries() ?? []).length} fallback={<p class="muted">{t("journal.empty")}</p>}>
          <ul class="journal">
            <For each={[...(entries() ?? [])].reverse()}>
              {(e) => (
                <li classList={{ undone: e.undone }}>
                  <div>
                    <strong>{e.summary}</strong>
                    <span class="muted"> · {e.kind} · {new Date(e.time * 1000).toLocaleString()}</span>
                    <Show when={e.undone}><span class="badge">{t("journal.undone")}</span></Show>
                  </div>
                  <For each={e.commands}>{(c) => <code class="mono">{c}</code>}</For>
                </li>
              )}
            </For>
          </ul>
        </Show>
        <div class="modal-actions">
          <button classList={{ danger: confirm() }} onClick={prune} onBlur={() => setConfirm(false)} title={t("journal.pruneTitle", { n: KEEP })}>
            {confirm() ? t("journal.pruneConfirm") : t("journal.prune", { n: KEEP })}
          </button>
          <button ref={close} class="primary" onClick={props.onClose}>{t("journal.close")}</button>
        </div>
      </dialog>
    </div>
  );
}
