// SPDX-License-Identifier: Apache-2.0
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { activeLocale, t } from "../../i18n";
import { ConfirmDialog, Modal } from "../../components";

const DEFAULT_KEEP = 50;

/** Operation journal: what Git Atlas did, with the exact Git commands, plus opt-in cleanup. */
export function JournalPanel(props: { repoId: number; onClose: () => void; onError: (e: unknown) => void }) {
  const [entries, { refetch }] = createResource(() => api.journal(props.repoId));
  let close: HTMLButtonElement | undefined;
  const [confirming, setConfirming] = createSignal(false);
  const [keep, setKeep] = createSignal(DEFAULT_KEEP);
  // Pruning cannot be undone, so it goes through ConfirmDialog.
  async function prune() {
    setConfirming(false);
    try {
      await api.pruneJournal(props.repoId, keep());
      refetch();
    } catch (e) {
      props.onError(e);
    }
  }
  return (
    <>
      <Modal
        title={t("journal.title")}
        size="md"
        onClose={props.onClose}
        initialFocus={() => close}
        footer={
          <>
            <label class="muted">{t("journal.keep")}{" "}
              <input type="number" min="0" step="1" value={keep()} style={{ width: "5em" }}
                onInput={(e) => setKeep(Math.max(0, Math.floor(Number(e.currentTarget.value) || 0)))} />
            </label>
            <button type="button" onClick={() => setConfirming(true)} title={t("journal.pruneTitle", { n: keep() })}>
              {t("journal.prune", { n: keep() })}
            </button>
            <button ref={close} type="button" class="primary" onClick={props.onClose}>{t("journal.close")}</button>
          </>
        }
      >
        <Show when={(entries() ?? []).length} fallback={<p class="muted">{t("journal.empty")}</p>}>
          <ul class="journal">
            <For each={[...(entries() ?? [])].reverse()}>
              {(e) => (
                <li classList={{ undone: e.undone }}>
                  <div>
                    <strong>{e.summary}</strong>
                    <span class="muted"> · {e.kind} · {new Date(e.time * 1000).toLocaleString(activeLocale())}</span>
                    <Show when={e.undone}><span class="badge">{t("journal.undone")}</span></Show>
                  </div>
                  <For each={e.commands}>{(c) => <code class="mono">{c}</code>}</For>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Modal>
      <Show when={confirming()}>
        <ConfirmDialog
          title={t("confirm.prune.title")}
          message={t("journal.pruneTitle", { n: keep() })}
          confirmLabel={t("confirm.prune.action")}
          onConfirm={() => void prune()}
          onCancel={() => setConfirming(false)}
        />
      </Show>
    </>
  );
}
