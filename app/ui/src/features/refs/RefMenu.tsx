// SPDX-License-Identifier: Apache-2.0
import { createEffect, createResource, createSignal, For, on, onMount, Show } from "solid-js";
import { api, RefInfo } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";

export interface RefMenuState { x: number; y: number; ref: RefInfo }
type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

/** Context menu for a branch/tag in the left panel, plus the name prompt it can open. */
export function RefMenu(props: { repoId: number; menu: RefMenuState; run: Run; onClose: () => void; onError: (e: unknown) => void }) {
  const r = () => props.menu.ref;
  const local = () => r().kind === "head";
  const [ask, setAsk] = createSignal<"new" | "rename" | "merge" | null>(null);
  const [preview] = createResource(() => (ask() === "merge" ? r().name : null), (src) => api.mergePreview(props.repoId, src).catch((e) => { props.onError(e); props.onClose(); return null; }));
  const [name, setName] = createSignal("");
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  let cancel: HTMLButtonElement | undefined;
  let input: HTMLInputElement | undefined;
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined);
  // Keep the menu inside the window when opened near the bottom or right edge.
  onMount(() => {
    if (!menuEl) return;
    const b = menuEl.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(props.menu.x, innerWidth - b.width - 4)), y: Math.max(4, Math.min(props.menu.y, innerHeight - b.height - 4)) });
  });
  createEffect(on(ask, (a) => {
    if (a === "merge") cancel?.focus();
    else if (a) input?.select();
  }, { defer: true }));

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
    <div class="menu-back" classList={{ "modal-back": !!ask() }} onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }}>
      <Show
        when={ask()}
        fallback={
          <menu ref={menuEl} class="ctx" role="menu" style={{ left: `${pos().x}px`, top: `${pos().y}px` }} onClick={(e) => e.stopPropagation()} onKeyDown={menuKeys}>
            <Show when={local() && !r().isHead}>
              <li role="none"><button role="menuitem" onClick={() => go(() => api.checkout(props.repoId, r().name, true), t("toast.checkedOut", { s: r().name }))}>{t("refmenu.checkout")}</button></li>
            </Show>
            <Show when={!r().isHead && r().kind !== "tag"}>
              <li role="none"><button role="menuitem" onClick={() => setAsk("merge")}>{t("refmenu.merge")}</button></li>
            </Show>
            <li role="none"><button role="menuitem" onClick={() => { setName(""); setAsk("new"); }}>{t("refmenu.newBranch")}</button></li>
            <Show when={local()}>
              <li role="none"><button role="menuitem" onClick={() => { setName(r().name); setAsk("rename"); }}>{t("refmenu.rename")}</button></li>
              <Show when={!r().isHead}>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.branchDelete(props.repoId, r().name, false), t("toast.branchDeleted", { s: r().name }))}>{t("refmenu.delete")}</button></li>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.branchDelete(props.repoId, r().name, true), t("toast.branchDeleted", { s: r().name }))}>{t("refmenu.forceDelete")}</button></li>
              </Show>
            </Show>
          </menu>
        }
      >
        <dialog open class="modal" aria-label={ask() === "merge" ? t("merge.title", { s: r().name }) : t("refmenu.namePrompt")} onClick={(e) => e.stopPropagation()} onContextMenu={(e) => e.stopPropagation()}>
          <Show when={ask() === "merge"}>
            <h2>{t("merge.title", { s: r().name })}</h2>
            <Show when={preview()} fallback={<p class="muted">{t("loading")}</p>}>
              {(p) => (
                <>
                  <p>{p().up_to_date ? t("merge.upToDate") : p().fast_forward ? t("merge.ff") : p().conflicts === null ? t("merge.unknown") : p().conflicts!.length ? t("merge.conflicts") : t("merge.clean")}</p>
                  <ul><For each={p().conflicts ?? []}>{(f) => <li class="mono">{f}</li>}</For></ul>
                </>
              )}
            </Show>
            <div class="modal-actions">
              <button ref={cancel} onClick={props.onClose}>{t("refmenu.cancel")}</button>
              <button class="primary" disabled={!preview() || preview()!.up_to_date}
                onClick={() => go(() => api.merge(props.repoId, r().name), t("toast.merged", { s: r().name }))}>
                {t("merge.confirm")}
              </button>
            </div>
          </Show>
          {/* Not `hidden`: `.modal form { display: grid }` would override it. */}
          <Show when={ask() !== "merge"}>
            <form onSubmit={submit}>
              <label>{t("refmenu.namePrompt")}
                <input ref={input} value={name()} onInput={(e) => setName(e.currentTarget.value)} />
              </label>
              <div class="modal-actions">
                <button type="button" onClick={props.onClose}>{t("refmenu.cancel")}</button>
                <button type="submit" class="primary" disabled={!name().trim()}>{t("refmenu.ok")}</button>
              </div>
            </form>
          </Show>
        </dialog>
      </Show>
    </div>
  );
}
