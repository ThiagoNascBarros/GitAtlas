// SPDX-License-Identifier: Apache-2.0
// "Share stash…": title, remote and the public-data warning (spec §6).
import { AlertTriangle } from "lucide-solid";
import { createMemo, createSignal, For, Show } from "solid-js";
import { StashInfo } from "../../api";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";
import { authorSlug, cleanTitle, defaultTitle } from "./logic";
import { modelOf } from "./model";

export function ShareDialog(props: { repoId: number; stash: StashInfo; onSubmit: (title: string, remote: string) => void; onCancel: () => void; busy?: boolean }) {
  const listing = () => modelOf(props.repoId).listing;
  const remotes = createMemo(() => listing()?.remotes ?? []);
  const usable = () => remotes().filter((r) => r.probe !== "rejected");
  const [title, setTitle] = createSignal(defaultTitle(props.stash.message));
  const [remote, setRemote] = createSignal(
    usable().find((r) => r.name === listing()?.defaultRemote)?.name ?? usable()[0]?.name ?? "",
  );
  let input: HTMLInputElement | undefined;
  useOverlay(() => props.onCancel(), () => input);
  // Git's user.name is not exposed to the UI; the slug of an earlier own stash is the same name.
  const slug = () => {
    const mine = listing()?.stashes.find((s) => s.mine);
    return mine ? authorSlug(mine.author_name, mine.author_email) : null;
  };
  const ok = () => !!cleanTitle(title()) && !!remote() && !props.busy;
  const submit = (e: Event) => {
    e.preventDefault();
    if (ok()) props.onSubmit(cleanTitle(title()), remote());
  };
  return (
    <div class="modal-back" onContextMenu={(e) => e.preventDefault()}>
      <dialog open class="modal" aria-labelledby="ss-share-h">
        <form onSubmit={submit}>
          <h2 id="ss-share-h">{t("ss.share.title")}</h2>
          <label>{t("ss.share.titleLabel")}
            <input type="text" maxLength={120} autocomplete="off" ref={(el) => { input = el; queueMicrotask(() => el.select()); }}
              value={title()} onInput={(e) => setTitle(e.currentTarget.value)} />
          </label>
          <Show when={remotes().length} fallback={<p class="ss-note error">{t("ss.share.noRemote")}</p>}>
            <label>{t("ss.share.remote")}
              <select value={remote()} onChange={(e) => setRemote(e.currentTarget.value)}>
                <For each={remotes()}>
                  {(r) => <option value={r.name} disabled={r.probe === "rejected"}>{r.probe === "rejected" ? `${r.name} — ${t("err.REMOTE_REJECTED")}` : r.name}</option>}
                </For>
              </select>
            </label>
          </Show>
          <div class="ss-note warn" role="note">
            <AlertTriangle size={14} aria-hidden="true" />
            <div>
              <p>{slug() ? t("ss.share.warning", { remote: remote() || "…", slug: slug()! }) : t("ss.share.warningNoName", { remote: remote() || "…" })}</p>
              <p>{t("ss.share.size")}</p>
            </div>
          </div>
          <div class="modal-actions">
            <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
            <button type="submit" class="primary" disabled={!ok()}>{props.busy ? t("ss.share.busy") : t("ss.share.submit")}</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
