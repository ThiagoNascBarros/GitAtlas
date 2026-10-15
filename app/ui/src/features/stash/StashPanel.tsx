// SPDX-License-Identifier: Apache-2.0
import { createResource, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";

type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

/** Left-panel list of stashes with apply / pop / drop. `rev` changes whenever the repo state may have. */
export function StashPanel(props: { repoId: number; rev: number; run: Run }) {
  const [list] = createResource(() => [props.repoId, props.rev] as const, ([id]) => api.stashList(id));
  const act = (action: "apply" | "pop" | "drop", index: number, toast: string) =>
    void props.run(() => api.stash(props.repoId, action, index), { toast });
  return (
    <Show when={(list() ?? []).length}>
      <details open>
        <summary><h2>{t("panel.stashes")} <span class="muted">{list()!.length}</span></h2></summary>
        <ul>
          <For each={list()}>
            {(s) => (
              <li class="stash-item">
                <span class="path" title={s.message}>{s.message}</span>
                <button class="mini" onClick={() => act("apply", s.index, t("toast.stashApplied"))}>{t("stash.apply")}</button>
                <button class="mini" onClick={() => act("pop", s.index, t("toast.stashPopped"))}>{t("stash.pop")}</button>
                <button class="mini" onClick={() => act("drop", s.index, t("toast.stashDropped"))}>{t("stash.drop")}</button>
              </li>
            )}
          </For>
        </ul>
      </details>
    </Show>
  );
}
