// SPDX-License-Identifier: Apache-2.0
import { createResource, createSignal, For, Show } from "solid-js";
import { api, StashInfo } from "../../api";
import { t } from "../../i18n";
import { isMenuKey } from "../overlay";
import { ConfirmDialog } from "../../components";

type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

/** Left-panel list of stashes with apply / pop / drop. `rev` changes whenever the repo state may have. */
export function StashPanel(props: { repoId: number; rev: number; run: Run; onMenu?: (s: StashInfo, x: number, y: number) => void }) {
  const [list] = createResource(() => [props.repoId, props.rev] as const, ([id]) => api.stashList(id));
  const act = (action: "apply" | "pop" | "drop", index: number, toast: string) =>
    void props.run(() => api.stash(props.repoId, action, index), { graph: true, toast });
  // Dropping goes through ConfirmDialog first (Undo can still bring it back).
  const [dropping, setDropping] = createSignal<StashInfo | null>(null);
  /** Keyboard-opened menu (A-2): under the row, like the shared stash rows. */
  const menuAt = (s: StashInfo, row: Element) => {
    const b = row.getBoundingClientRect();
    props.onMenu?.(s, b.left + 24, b.bottom);
  };
  return (
    <Show when={(list() ?? []).length}>
      <details open>
        <summary><h2>{t("panel.stashes")} <span class="muted">{list()!.length}</span></h2></summary>
        <ul data-focus-group="stashes">
          <For each={list()}>
            {(s) => (
              <li class="stash-item" data-focus-key={s.sha}
                onContextMenu={(e) => { if (!props.onMenu) return; e.preventDefault(); props.onMenu(s, e.clientX, e.clientY); }}
                // Menu key / Shift+F10 on the row or any of its buttons.
                onKeyDown={(e) => { if (!props.onMenu || !isMenuKey(e)) return; e.preventDefault(); menuAt(s, e.currentTarget); }}>
                {/* Focusable only when there is a menu to open (not in read-only repositories). */}
                <span class="path" id={`stash-msg-${s.sha}`} title={s.message}
                  tabindex={props.onMenu ? 0 : undefined} role={props.onMenu ? "button" : undefined} aria-haspopup={props.onMenu ? "menu" : undefined}
                  onKeyDown={(e) => {
                    if (!props.onMenu || (e.key !== "Enter" && e.key !== " ")) return;
                    e.preventDefault();
                    menuAt(s, e.currentTarget.closest("li")!);
                  }}>{s.message}</span>
                <button class="mini" aria-describedby={`stash-msg-${s.sha}`} onClick={() => act("apply", s.index, t("toast.stashApplied"))}>{t("stash.apply")}</button>
                <button class="mini" aria-describedby={`stash-msg-${s.sha}`} onClick={() => act("pop", s.index, t("toast.stashPopped"))}>{t("stash.pop")}</button>
                <button class="mini" aria-describedby={`stash-msg-${s.sha}`} onClick={() => setDropping(s)}>{t("stash.drop")}</button>
              </li>
            )}
          </For>
        </ul>
      </details>
      <Show when={dropping()}>
        {(s) => (
          <ConfirmDialog
            title={t("confirm.dropStash.title")}
            message={t("confirm.dropStash.message")}
            confirmLabel={t("stash.drop")}
            onConfirm={() => { const x = s(); setDropping(null); act("drop", x.index, t("toast.stashDropped")); }}
            onCancel={() => setDropping(null)}
          />
        )}
      </Show>
    </Show>
  );
}
