// SPDX-License-Identifier: Apache-2.0
import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { api, MergePreview, RefInfo } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";
import { NameDialog } from "./NameDialog";
import { ConfirmDialog, Modal } from "../../components";
import { Check, GitBranchPlus, GitMerge, Pencil, Trash2, Upload } from "lucide-solid";

/** `merge` opens straight on the merge dialog (used by drag-and-drop of a ref onto the current branch). */
export interface RefMenuState { x: number; y: number; ref: RefInfo; merge?: boolean }
type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

/** Context menu for a branch/tag in the left panel, plus the dialogs it can open. */
function mergeText(p: MergePreview): string {
  if (p.up_to_date) return t("merge.upToDate");
  if (p.fast_forward) return t("merge.ff");
  if (p.conflicts === null) return t("merge.unknown");
  return p.conflicts.length ? t("merge.conflicts") : t("merge.clean");
}

export function RefMenu(props: { repoId: number; menu: RefMenuState; run: Run; onClose: () => void; onError: (e: unknown) => void; onRebase: (onto: string) => void }) {
  const r = () => props.menu.ref;
  const local = () => r().kind === "head";
  const [ask, setAsk] = createSignal<"new" | "rename" | "merge" | "force" | null>(props.menu.merge ? "merge" : null);
  const [preview] = createResource(() => (ask() === "merge" ? r().name : null), (src) => api.mergePreview(props.repoId, src).catch((e) => { props.onError(e); props.onClose(); return null; }));
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  let cancel: HTMLButtonElement | undefined;
  // Trap: the menu (the dialogs installed on top of it take over Tab while they show).
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined, () => menuEl);
  // Keep the menu inside the window when opened near the bottom or right edge.
  onMount(() => {
    if (!menuEl) return;
    const b = menuEl.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(props.menu.x, innerWidth - b.width - 4)), y: Math.max(4, Math.min(props.menu.y, innerHeight - b.height - 4)) });
  });

  const go = (op: () => Promise<unknown>, toast: string) => {
    props.onClose();
    void props.run(op, { graph: true, toast });
  };
  const submit = (n: string) => {
    if (ask() === "new") go(() => api.branchCreate(props.repoId, n, r().name), t("toast.branchCreated", { s: n }));
    else go(() => api.branchRename(props.repoId, r().name, n), t("toast.branchRenamed", { s: n }));
  };

  // The merge dialog closes only via its buttons/Escape/X, like the name dialog.
  const mergeDialog = () => (
    <Modal
      title={t("merge.title", { s: r().name })}
      size="md"
      onClose={props.onClose}
      initialFocus={() => cancel}
      footer={
        <>
          <button ref={cancel} type="button" onClick={props.onClose}>{t("refmenu.cancel")}</button>
          <button type="button" class="primary" disabled={!preview() || preview()!.up_to_date}
            onClick={() => go(() => api.merge(props.repoId, r().name), t("toast.merged", { s: r().name }))}>
            {t("merge.confirm")}
          </button>
        </>
      }
    >
      <Show when={preview()} fallback={<p class="muted">{t("loading")}</p>}>
        {(p) => (
          <>
            <p>{mergeText(p())}</p>
            <ul><For each={p().conflicts ?? []}>{(f) => <li class="mono">{f}</li>}</For></ul>
          </>
        )}
      </Show>
    </Modal>
  );

  return (
    <Show when={ask() !== "new" && ask() !== "rename"} fallback={
      <NameDialog label={t("refmenu.namePrompt")} initial={ask() === "rename" ? r().name : ""} onSubmit={submit} onCancel={props.onClose} />
    }>
      <Show when={ask() !== "force"} fallback={
        <ConfirmDialog
          title={t("confirm.forceDelete.title")}
          message={t("confirm.forceDelete.message")}
          confirmLabel={t("confirm.forceDelete.action")}
          onConfirm={() => go(() => api.branchDelete(props.repoId, r().name, true), t("toast.branchDeleted", { s: r().name }))}
          onCancel={props.onClose}
        />
      }>
        <Show when={ask() !== "merge"} fallback={mergeDialog()}>
          <div class="menu-back">
            <button type="button" class="backdrop" tabindex="-1" aria-label={t("refmenu.cancel")} onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }} />
            <menu ref={menuEl} class="ctx" role="menu" aria-label={r().name} style={{ left: `${pos().x}px`, top: `${pos().y}px` }} onClick={(e) => e.stopPropagation()} onKeyDown={menuKeys}
              // Hover moves focus, so mouse and keyboard share one highlighted item (as in native menus).
              onPointerOver={(e) => (e.target as HTMLElement).closest<HTMLButtonElement>("button:not(:disabled)")?.focus()}>
              <Show when={local() && !r().isHead}>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.checkout(props.repoId, r().name, true), t("toast.checkedOut", { s: r().name }))}><Check size={14} aria-hidden="true" />{t("refmenu.checkout")}</button></li>
              </Show>
              <Show when={r().kind === "tag" || r().kind === "remote"}>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.checkout(props.repoId, r().name, false), t("toast.checkedOutDetached", { s: r().name }))}><Check size={14} aria-hidden="true" />{t("refmenu.checkoutDetached")}</button></li>
              </Show>
              <Show when={!r().isHead && r().kind !== "tag"}>
                <li role="none"><button role="menuitem" onClick={() => setAsk("merge")}><GitMerge size={14} aria-hidden="true" />{t("refmenu.merge")}</button></li>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.rebase(props.repoId, r().name), t("toast.rebased"))}><GitMerge size={14} aria-hidden="true" />{t("refmenu.rebase")}</button></li>
                <li role="none"><button role="menuitem" onClick={() => { props.onClose(); props.onRebase(r().name); }}><GitMerge size={14} aria-hidden="true" />{t("refmenu.rebaseInteractive")}</button></li>
              </Show>
              <Show when={r().kind === "tag"}>
                <li role="none"><button role="menuitem" onClick={() => go(() => api.net(props.repoId, "push-tag", false, r().name), t("toast.tagPushed", { s: r().name }))}><Upload size={14} aria-hidden="true" />{t("refmenu.pushTag")}</button></li>
                <li role="none"><button role="menuitem" classList={{ danger: true }} onClick={() => go(() => api.history(props.repoId, "tag-delete", "", { name: r().name }), t("toast.tagDeleted", { s: r().name }))}><Trash2 size={14} aria-hidden="true" />{t("refmenu.deleteTag")}</button></li>
              </Show>
              <li role="none"><button role="menuitem" onClick={() => setAsk("new")}><GitBranchPlus size={14} aria-hidden="true" />{t("refmenu.newBranch")}</button></li>
              <Show when={local()}>
                <li role="none"><button role="menuitem" onClick={() => setAsk("rename")}><Pencil size={14} aria-hidden="true" />{t("refmenu.rename")}</button></li>
                <Show when={!r().isHead}>
                  <li role="separator" class="menu-sep" />
                  <li role="none"><button role="menuitem" classList={{ danger: true }} onClick={() => go(() => api.branchDelete(props.repoId, r().name, false), t("toast.branchDeleted", { s: r().name }))}><Trash2 size={14} aria-hidden="true" />{t("refmenu.delete")}</button></li>
                  {/* Deletes commits that are not merged: confirmed first (FUN-07). */}
                  <li role="none"><button role="menuitem" classList={{ danger: true }} onClick={() => setAsk("force")}><Trash2 size={14} aria-hidden="true" />{t("refmenu.forceDelete")}</button></li>
                </Show>
              </Show>
            </menu>
          </div>
        </Show>
      </Show>
    </Show>
  );
}
