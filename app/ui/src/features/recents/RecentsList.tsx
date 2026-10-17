// SPDX-License-Identifier: Apache-2.0
// Recent repositories on the start screen (FUN-05). A folder that is gone stays listed, muted, and
// can only be removed from the list.
import { For, Show } from "solid-js";
import { X } from "lucide-solid";
import type { RecentRepo } from "../../api";
import { t } from "../../i18n";
import "./recents.css";

interface Props {
  recents: RecentRepo[];
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
}

export function RecentsList(props: Props) {
  return (
    <Show when={props.recents.length > 0}>
      <section class="recents" aria-labelledby="recents-title">
        <h2 id="recents-title">{t("recents.title")}</h2>
        <ul>
          <For each={props.recents}>
            {(r) => (
              <li class="recent" classList={{ missing: !r.exists }}>
                <button class="recent-open" disabled={!r.exists} title={r.path} onClick={() => props.onOpen(r.path)}>
                  <span class="recent-name">{r.name}</span>
                  <span class="recent-path">{r.path}</span>
                  <Show when={!r.exists}>
                    <span class="recent-missing">{t("recents.missing")}</span>
                  </Show>
                </button>
                <button class="icon" title={t("recents.remove", { s: r.name })} aria-label={t("recents.remove", { s: r.name })} onClick={() => props.onRemove(r.path)}>
                  <X size={14} aria-hidden="true" />
                </button>
              </li>
            )}
          </For>
        </ul>
      </section>
    </Show>
  );
}
