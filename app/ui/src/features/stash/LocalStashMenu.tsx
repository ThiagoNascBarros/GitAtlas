// SPDX-License-Identifier: Apache-2.0
// Right-click menu of a local stash: share it, or apply it.
import { Archive, ArchiveRestore, Share2 } from "lucide-solid";
import { createSignal, onMount } from "solid-js";
import { StashInfo } from "../../api";
import { t } from "../../i18n";
import { menuKeys, useOverlay } from "../overlay";

export interface LocalStashMenuState { x: number; y: number; stash: StashInfo }

export function LocalStashMenu(props: { menu: LocalStashMenuState; onShare: (s: StashInfo) => void; onApply: (s: StashInfo) => void; onPop: (s: StashInfo) => void; onClose: () => void }) {
  const [pos, setPos] = createSignal({ x: props.menu.x, y: props.menu.y });
  let menuEl: HTMLMenuElement | undefined;
  useOverlay(() => props.onClose(), () => menuEl?.querySelector("button") ?? undefined);
  onMount(() => {
    if (!menuEl) return;
    const b = menuEl.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(props.menu.x, innerWidth - b.width - 4)), y: Math.max(4, Math.min(props.menu.y, innerHeight - b.height - 4)) });
  });
  const go = (f: (s: StashInfo) => void) => () => { props.onClose(); f(props.menu.stash); };
  const item = (icon: unknown, label: string, onClick: () => void) => (
    <li role="none"><button role="menuitem" onClick={onClick} onMouseEnter={(e) => e.currentTarget.focus()}>{icon as never}{label}</button></li>
  );
  return (
    <div class="menu-back">
      <button type="button" class="backdrop" tabindex="-1" aria-label={t("refmenu.cancel")} onClick={props.onClose} onContextMenu={(e) => { e.preventDefault(); props.onClose(); }} />
      <menu ref={menuEl} class="ctx" role="menu" aria-label={props.menu.stash.message} style={{ left: `${pos().x}px`, top: `${pos().y}px` }} onKeyDown={menuKeys}>
        {item(<Share2 size={14} aria-hidden="true" />, t("ss.menu.share"), go(props.onShare))}
        {item(<Archive size={14} aria-hidden="true" />, t("ss.menu.localApply"), go(props.onApply))}
        {item(<ArchiveRestore size={14} aria-hidden="true" />, t("ss.menu.localPop"), go(props.onPop))}
      </menu>
    </div>
  );
}
