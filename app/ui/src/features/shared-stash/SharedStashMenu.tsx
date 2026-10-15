// SPDX-License-Identifier: Apache-2.0
// Right-click menu of a shared stash and the small dialogs it opens (branch name, rename, update, unshare).
import { Archive, ArchiveX, GitBranchPlus, Pencil, RefreshCw, Upload } from "lucide-solid";
import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { api, SharedStashInfo } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";
import { NameDialog } from "../refs/NameDialog";
import { cleanTitle, rowActions } from "./logic";

export interface SharedMenuState { x: number; y: number; stash: SharedStashInfo }
export interface SharedActions {
  apply: (s: SharedStashInfo) => void;
  branch: (s: SharedStashInfo, name: string) => void;
  update: (s: SharedStashInfo, stashSha: string | null, title: string | null) => void;
  rename: (s: SharedStashInfo, title: string) => void;
  unshare: (s: SharedStashInfo) => void;
}

export function SharedStashMenu(props: { repoId: number; menu: SharedMenuState; actions: SharedActions; onClose: () => void }) {
  const s = () => props.menu.stash;
  const [ask, setAsk] = createSignal<"branch" | "rename" | "update" | "unshare" | null>(null);
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined);
  onMount(() => {
    if (!menuEl) return;
    const b = menuEl.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(props.menu.x, innerWidth - b.width - 4)), y: Math.max(4, Math.min(props.menu.y, innerHeight - b.height - 4)) });
  });
  const done = (f: () => void) => { props.onClose(); f(); };
  const allowed = () => rowActions(s());
  const item = (icon: unknown, label: string, onClick: () => void) => (
    <li role="none"><button role="menuitem" onClick={onClick} onMouseEnter={(e) => e.currentTarget.focus()}>{icon as never}{label}</button></li>
  );

  return (
    <Show when={!ask()} fallback={
      <>
        <Show when={ask() === "branch"}>
          <NameDialog label={t("ss.branch.prompt")} onSubmit={(n) => done(() => props.actions.branch(s(), n))} onCancel={props.onClose} />
        </Show>
        <Show when={ask() === "rename"}>
          <NameDialog label={t("ss.rename.prompt")} initial={s().title} onSubmit={(v) => done(() => props.actions.rename(s(), cleanTitle(v)))} onCancel={props.onClose} />
        </Show>
        <Show when={ask() === "update"}>
          <UpdateDialog repoId={props.repoId} stash={s()} onCancel={props.onClose}
            onSubmit={(sha, title) => done(() => props.actions.update(s(), sha, title))} />
        </Show>
        <Show when={ask() === "unshare"}>
          <UnshareDialog stash={s()} onCancel={props.onClose} onConfirm={() => done(() => props.actions.unshare(s()))} />
        </Show>
      </>
    }>
      <div class="menu-back">
        <button type="button" class="backdrop" tabindex="-1" aria-label={t("refmenu.cancel")} onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }} />
        <menu ref={menuEl} class="ctx" role="menu" aria-label={s().title} style={{ left: `${pos().x}px`, top: `${pos().y}px` }} onKeyDown={menuKeys}>
          <Show when={allowed().includes("apply")}>{item(<Archive size={14} aria-hidden="true" />, t("ss.menu.apply"), () => done(() => props.actions.apply(s())))}</Show>
          <Show when={allowed().includes("branch")}>{item(<GitBranchPlus size={14} aria-hidden="true" />, t("ss.menu.branch"), () => setAsk("branch"))}</Show>
          <Show when={allowed().includes("update")}>{item(<Upload size={14} aria-hidden="true" />, t("ss.menu.update"), () => setAsk("update"))}</Show>
          <Show when={allowed().includes("rename")}>{item(<Pencil size={14} aria-hidden="true" />, t("ss.menu.rename"), () => setAsk("rename"))}</Show>
          <Show when={allowed().includes("unshare")}>{item(<ArchiveX size={14} aria-hidden="true" />, t("ss.menu.unshare"), () => setAsk("unshare"))}</Show>
        </menu>
      </div>
    </Show>
  );
}

/** Picks one of the local stashes to replace the shared content; an optional new title. */
function UpdateDialog(props: { repoId: number; stash: SharedStashInfo; onSubmit: (sha: string | null, title: string | null) => void; onCancel: () => void }) {
  const [list] = createResource(() => props.repoId, (id) => api.stashList(id));
  const [sha, setSha] = createSignal("");
  const [title, setTitle] = createSignal("");
  let sel: HTMLSelectElement | undefined;
  useOverlay(() => props.onCancel(), () => sel);
  const chosen = () => sha() || list()?.[0]?.sha || "";
  return (
    <div class="modal-back" onContextMenu={(e) => e.preventDefault()}>
      <dialog open class="modal" aria-labelledby="ss-upd-h">
        <form onSubmit={(e) => { e.preventDefault(); if (chosen()) props.onSubmit(chosen(), cleanTitle(title()) || null); }}>
          <h2 id="ss-upd-h">{t("ss.update.title")}</h2>
          <p class="muted">{t("ss.update.hint", { s: props.stash.title })}</p>
          <Show when={(list() ?? []).length} fallback={<p class="ss-note warn">{list.loading ? t("loading") : t("ss.update.none")}</p>}>
            <label>{t("ss.update.pick")}
              <select ref={sel} value={chosen()} onChange={(e) => setSha(e.currentTarget.value)}>
                <For each={list()}>{(l) => <option value={l.sha}>{l.message}</option>}</For>
              </select>
            </label>
          </Show>
          <label>{t("ss.update.newTitle")}
            <input type="text" maxLength={120} autocomplete="off" placeholder={props.stash.title} value={title()} onInput={(e) => setTitle(e.currentTarget.value)} />
          </label>
          <div class="modal-actions">
            <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
            <button type="submit" class="primary" disabled={!chosen()}><RefreshCw size={14} aria-hidden="true" />{t("ss.update.submit")}</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}

function UnshareDialog(props: { stash: SharedStashInfo; onConfirm: () => void; onCancel: () => void }) {
  let cancel: HTMLButtonElement | undefined;
  useOverlay(() => props.onCancel(), () => cancel);
  return (
    <div class="modal-back" onContextMenu={(e) => e.preventDefault()}>
      <dialog open class="modal" aria-labelledby="ss-un-h">
        <h2 id="ss-un-h">{t("ss.unshare.title", { s: props.stash.title })}</h2>
        <p>{t("ss.unshare.body", { remote: props.stash.remote })}</p>
        <div class="modal-actions">
          <button ref={cancel} type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
          <button type="button" class="danger" onClick={props.onConfirm}>{t("ss.unshare.submit")}</button>
        </div>
      </dialog>
    </div>
  );
}
