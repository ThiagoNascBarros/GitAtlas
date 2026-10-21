// SPDX-License-Identifier: Apache-2.0
// Right-click menu on a graph row: history operations on that commit (Phase 4).
import { createSignal, onMount, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";
import { NameDialog } from "../refs/NameDialog";
import { ConfirmDialog } from "../../components";
import { MENU_ICONS, type MenuAction } from "../icons";

/** The icon of a menu action, from the one table in features/icons.ts. */
const menuIcon = (action: MenuAction) => {
  const Icon = MENU_ICONS[action];
  return <Icon size={14} aria-hidden="true" />;
};

export interface CommitMenuState { x: number; y: number; sha: string }
type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

export function CommitMenu(props: { repoId: number; menu: CommitMenuState; run: Run; onClose: () => void; onToast: (s: string) => void; onRebase: (onto: string) => void }) {
  const sha = () => props.menu.sha;
  const short = () => sha().slice(0, 7);
  // "hard" asks for confirmation first (FUN-07); the others run at once.
  const [ask, setAsk] = createSignal<"branch" | "tag" | "hard" | null>(null);
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined, () => menuEl);
  onMount(() => {
    if (!menuEl) return;
    const b = menuEl.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(props.menu.x, innerWidth - b.width - 4)), y: Math.max(4, Math.min(props.menu.y, innerHeight - b.height - 4)) });
  });
  const go = (op: () => Promise<unknown>, toast: string) => {
    props.onClose();
    void props.run(op, { graph: true, toast });
  };
  const reset = (mode: string) => go(() => api.history(props.repoId, "reset", sha(), { mode }), t("toast.reset", { s: short() }));
  const item = (icon: () => unknown, label: string, onClick: () => void, danger = false) => (
    <li role="none"><button role="menuitem" classList={{ danger }} onClick={onClick} onMouseEnter={(e) => e.currentTarget.focus()}>{icon() as never}{label}</button></li>
  );
  const separator = () => <li role="separator" class="menu-sep" />;

  return (
    <Show when={ask() !== "hard"} fallback={
      <ConfirmDialog
        title={t("confirm.resetHard.title")}
        message={t("confirm.resetHard.message")}
        confirmLabel={t("confirm.resetHard.action")}
        onConfirm={() => reset("hard")}
        onCancel={props.onClose}
      />
    }>
      <Show when={!ask()} fallback={
        <NameDialog
          label={ask() === "tag" ? t("commitmenu.tagPrompt") : t("refmenu.namePrompt")}
          onSubmit={(n) => ask() === "tag"
            ? go(() => api.history(props.repoId, "tag", sha(), { name: n }), t("toast.tagged", { s: n }))
            : go(() => api.branchCreate(props.repoId, n, sha()), t("toast.branchCreated", { s: n }))}
          onCancel={props.onClose}
        />
      }>
        <div class="menu-back">
          <button type="button" class="backdrop" tabindex="-1" aria-label={t("refmenu.cancel")} onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }} />
          <menu ref={menuEl} class="ctx" role="menu" aria-label={short()} style={{ left: `${pos().x}px`, top: `${pos().y}px` }} onKeyDown={menuKeys}>
            {item(() => menuIcon("commit.checkout"), t("commitmenu.checkout"), () => go(() => api.checkout(props.repoId, sha(), false), t("toast.checkedOutDetached", { s: short() })))}
            {item(() => menuIcon("commit.newBranch"), t("commitmenu.newBranch"), () => setAsk("branch"))}
            {item(() => menuIcon("commit.tag"), t("commitmenu.tag"), () => setAsk("tag"))}
            {separator()}
            {item(() => menuIcon("commit.cherryPick"), t("commitmenu.cherryPick"), () => go(() => api.history(props.repoId, "cherry-pick", sha()), t("toast.cherryPicked", { s: short() })))}
            {item(() => menuIcon("commit.revert"), t("commitmenu.revert"), () => go(() => api.history(props.repoId, "revert", sha()), t("toast.reverted", { s: short() })))}
            {item(() => menuIcon("commit.rebaseInteractive"), t("commitmenu.rebaseInteractive"), () => { props.onClose(); props.onRebase(sha()); })}
            {item(() => menuIcon("commit.resetSoft"), t("commitmenu.resetSoft"), () => reset("soft"))}
            {item(() => menuIcon("commit.resetMixed"), t("commitmenu.resetMixed"), () => reset("mixed"))}
            {separator()}
            {item(() => menuIcon("commit.resetHard"), t("commitmenu.resetHard"), () => setAsk("hard"), true)}
            {separator()}
            {item(() => menuIcon("commit.copySha"), t("commitmenu.copySha"), () => { props.onClose(); void navigator.clipboard?.writeText(sha()).then(() => props.onToast(t("toast.copied", { s: short() }))); })}
          </menu>
        </div>
      </Show>
    </Show>
  );
}
