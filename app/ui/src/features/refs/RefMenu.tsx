// SPDX-License-Identifier: Apache-2.0
import { createSignal, Show } from "solid-js";
import { api, RefInfo } from "../../api";
import { t } from "../../i18n";

export interface RefMenuState { x: number; y: number; ref: RefInfo }
type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

/** Context menu for a branch/tag in the left panel, plus the name prompt it can open. */
export function RefMenu(props: { repoId: number; menu: RefMenuState; run: Run; onClose: () => void; onError: (e: unknown) => void }) {
  const r = () => props.menu.ref;
  const local = () => r().kind === "head";
  const [ask, setAsk] = createSignal<"new" | "rename" | null>(null);
  const [name, setName] = createSignal("");

  const go = (op: () => Promise<unknown>, toast: string) => {
    props.onClose();
    void props.run(op, { graph: true, toast });
  };
  const submit = (e: Event) => {
    e.preventDefault();
    const n = name().trim();
    if (!n) return;
    if (ask() === "new") go(() => api.branchCreate(props.repoId, n, r().name), t("toast.branchCreated", { s: n }));
    else go(() => api.branchRename(props.repoId, r().name, n), t("toast.branchRenamed", { s: n }));
  };

  return (
    <div class="menu-back" onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }}>
      <Show
        when={ask()}
        fallback={
          <menu class="ctx" style={{ left: `${props.menu.x}px`, top: `${props.menu.y}px` }} onClick={(e) => e.stopPropagation()}>
            <Show when={local() && !r().isHead}>
              <li><button onClick={() => go(() => api.checkout(props.repoId, r().name, true), t("toast.checkedOut", { s: r().name }))}>{t("refmenu.checkout")}</button></li>
            </Show>
            <li><button onClick={() => { setName(""); setAsk("new"); }}>{t("refmenu.newBranch")}</button></li>
            <Show when={local()}>
              <li><button onClick={() => { setName(r().name); setAsk("rename"); }}>{t("refmenu.rename")}</button></li>
              <Show when={!r().isHead}>
                <li><button onClick={() => go(() => api.branchDelete(props.repoId, r().name, false), t("toast.branchDeleted", { s: r().name }))}>{t("refmenu.delete")}</button></li>
                <li><button onClick={() => go(() => api.branchDelete(props.repoId, r().name, true), t("toast.branchDeleted", { s: r().name }))}>{t("refmenu.forceDelete")}</button></li>
              </Show>
            </Show>
          </menu>
        }
      >
        <dialog open class="modal" aria-label={t("refmenu.namePrompt")} onClick={(e) => e.stopPropagation()} onKeyDown={(e) => e.key === "Escape" && props.onClose()}>
          <form onSubmit={submit}>
            <label>{t("refmenu.namePrompt")}
              <input autofocus value={name()} onInput={(e) => setName(e.currentTarget.value)} />
            </label>
            <div class="modal-actions">
              <button type="button" onClick={props.onClose}>{t("refmenu.cancel")}</button>
              <button type="submit" class="primary">{t("refmenu.ok")}</button>
            </div>
          </form>
        </dialog>
      </Show>
    </div>
  );
}
