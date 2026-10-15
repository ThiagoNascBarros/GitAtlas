// SPDX-License-Identifier: Apache-2.0
// Right-click menu on a graph row: history operations on that commit (Phase 4).
import { createSignal, onMount, Show } from "solid-js";
import { Check, Copy, GitBranchPlus, Redo2, RotateCcw, Tag, Undo2 } from "lucide-solid";
import { api } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";
import { NameDialog } from "../refs/NameDialog";

export interface CommitMenuState { x: number; y: number; sha: string }
type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

export function CommitMenu(props: { repoId: number; menu: CommitMenuState; run: Run; onClose: () => void; onToast: (s: string) => void }) {
  const sha = () => props.menu.sha;
  const short = () => sha().slice(0, 7);
  const [ask, setAsk] = createSignal<"branch" | "tag" | null>(null);
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined);
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
  const item = (icon: () => unknown, label: string, onClick: () => void) => (
    <li role="none"><button role="menuitem" onClick={onClick} onMouseEnter={(e) => e.currentTarget.focus()}>{icon() as never}{label}</button></li>
  );

  return (
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
          {item(() => <Redo2 size={14} aria-hidden="true" />, t("commitmenu.cherryPick"), () => go(() => api.history(props.repoId, "cherry-pick", sha()), t("toast.cherryPicked", { s: short() })))}
          {item(() => <Undo2 size={14} aria-hidden="true" />, t("commitmenu.revert"), () => go(() => api.history(props.repoId, "revert", sha()), t("toast.reverted", { s: short() })))}
          {item(() => <RotateCcw size={14} aria-hidden="true" />, t("commitmenu.resetSoft"), () => reset("soft"))}
          {item(() => <RotateCcw size={14} aria-hidden="true" />, t("commitmenu.resetMixed"), () => reset("mixed"))}
          {item(() => <RotateCcw size={14} aria-hidden="true" />, t("commitmenu.resetHard"), () => reset("hard"))}
          {item(() => <GitBranchPlus size={14} aria-hidden="true" />, t("commitmenu.newBranch"), () => setAsk("branch"))}
          {item(() => <Tag size={14} aria-hidden="true" />, t("commitmenu.tag"), () => setAsk("tag"))}
          {item(() => <Check size={14} aria-hidden="true" />, t("commitmenu.checkout"), () => go(() => api.checkout(props.repoId, sha(), false), t("toast.checkedOutDetached", { s: short() })))}
          {item(() => <Copy size={14} aria-hidden="true" />, t("commitmenu.copySha"), () => { props.onClose(); void navigator.clipboard?.writeText(sha()).then(() => props.onToast(t("toast.copied", { s: short() }))); })}
        </menu>
      </div>
    </Show>
  );
}
