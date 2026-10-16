// SPDX-License-Identifier: Apache-2.0
// Interactive rebase editor: choose an action per commit, reorder, reword (Phase 4).
import { createEffect, createResource, createSignal, For, Show } from "solid-js";
import { ArrowDown, ArrowUp } from "lucide-solid";
import { api, TodoItem } from "../../api";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";
import { firstIsSquash, move } from "./todo";

type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;
const ACTIONS: TodoItem["action"][] = ["pick", "reword", "squash", "fixup", "drop"];

export function RebaseEditor(props: { repoId: number; onto: string; label: string; run: Run; onClose: () => void; onError: (e: unknown) => void }) {
  const [list, setList] = createSignal<TodoItem[]>([]);
  const [loaded] = createResource(
    () => props.onto,
    (o) => api.rebaseTodo(props.repoId, o).then((x) => { setList(x); return true; }).catch((e) => { props.onError(e); props.onClose(); return false; }),
  );
  let cancel: HTMLButtonElement | undefined;
  let dialog: HTMLDialogElement | undefined;
  // While loading, the title holds focus (inside the dialog); Cancel takes it once it exists.
  let heading: HTMLHeadingElement | undefined;
  useOverlay(() => props.onClose(), () => cancel ?? heading, () => dialog);
  createEffect(() => { if (loaded() && document.activeElement === heading) cancel?.focus(); });
  const set = (i: number, patch: Partial<TodoItem>) => setList(list().map((x, j) => (j === i ? { ...x, ...patch } : x)));
  const start = async () => {
    props.onClose();
    await props.run(() => api.rebase(props.repoId, props.onto, list()), { graph: true, toast: t("toast.rebased") });
  };
  return (
    <div class="modal-back">
      <dialog ref={dialog} open class="modal rebase" aria-modal="true" aria-labelledby="rebase-h">
        <h2 id="rebase-h" ref={heading} tabindex="-1">{t("rebase.title", { s: props.label })}</h2>
        <Show when={loaded()} fallback={<p class="muted">{t("loading")}</p>}>
          <Show when={list().length} fallback={<p class="muted">{t("rebase.empty")}</p>}>
            <ol class="rebase-list">
              <For each={list()}>
                {(item, i) => (
                  <li classList={{ dropped: item.action === "drop" }}>
                    <select aria-label={item.message} value={item.action} onChange={(e) => set(i(), { action: e.currentTarget.value as TodoItem["action"] })}>
                      <For each={ACTIONS}>{(a) => <option value={a}>{t(`rebase.${a}` as never)}</option>}</For>
                    </select>
                    <span class="mono sha">{item.sha.slice(0, 7)}</span>
                    <Show when={item.action === "reword"} fallback={<span class="msg">{item.message}</span>}>
                      <input class="msg" aria-label={t("rebase.message")} value={item.message} onInput={(e) => set(i(), { message: e.currentTarget.value })} />
                    </Show>
                    <button class="icon" aria-label={t("rebase.up")} disabled={i() === 0} onClick={() => setList(move(list(), i(), -1))}><ArrowUp size={14} aria-hidden="true" /></button>
                    <button class="icon" aria-label={t("rebase.down")} disabled={i() === list().length - 1} onClick={() => setList(move(list(), i(), 1))}><ArrowDown size={14} aria-hidden="true" /></button>
                  </li>
                )}
              </For>
            </ol>
          </Show>
          <div class="modal-actions">
            <Show when={firstIsSquash(list())}><span class="muted">{t("rebase.firstSquash")}</span></Show>
            <button ref={cancel} onClick={props.onClose}>{t("refmenu.cancel")}</button>
            <button class="primary" disabled={!list().length || firstIsSquash(list()) || list().some((x) => x.action === "reword" && !x.message.trim())} onClick={() => void start()}>{t("rebase.start")}</button>
          </div>
        </Show>
      </dialog>
    </div>
  );
}
