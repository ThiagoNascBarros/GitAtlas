// SPDX-License-Identifier: Apache-2.0
// 3-pane conflict editor: ours | theirs on top, editable result below with per-conflict choices.
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";
import { findChunks, pick } from "./markers";

type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

export function ConflictEditor(props: { repoId: number; path: string; run: Run; onClose: () => void; onError: (e: unknown) => void }) {
  const [result, setResult] = createSignal("");
  const [v] = createResource(
    () => props.path,
    (p) => api.conflictVersions(props.repoId, p).then((x) => { setResult(x.merged); return x; }).catch((e) => { props.onError(e); props.onClose(); return null; }),
  );
  const chunks = () => findChunks(result());
  let cancel: HTMLButtonElement | undefined;
  useOverlay(() => props.onClose(), () => cancel);
  const save = async () => {
    if (await props.run(() => api.resolveWith(props.repoId, props.path, result()), { toast: t("toast.resolved", { s: props.path }) })) props.onClose();
  };
  const side = (text: string | null | undefined, label: string) => (
    <section class="conflict-side">
      <h3>{label}</h3>
      <pre class="mono">{text ?? t("conflict.missing")}</pre>
    </section>
  );
  return (
    <div class="modal-back">
      <dialog open class="modal conflict" aria-label={t("conflict.title", { s: props.path })}>
        <h2>{t("conflict.title", { s: props.path })}</h2>
        <Show when={v()} fallback={<p class="muted">{t("loading")}</p>}>
          <div class="conflict-sides">
            {side(v()!.ours, t("conflict.ours"))}
            {side(v()!.theirs, t("conflict.theirs"))}
          </div>
          <div class="conflict-chunks">
            <For each={chunks()}>
              {(_, i) => (
                <div class="conflict-chunk">
                  <span>{t("conflict.chunk", { n: i() + 1 })}</span>
                  <button class="mini" onClick={() => setResult(pick(result(), i(), "ours"))}>{t("conflict.useOurs")}</button>
                  <button class="mini" onClick={() => setResult(pick(result(), i(), "theirs"))}>{t("conflict.useTheirs")}</button>
                  <button class="mini" onClick={() => setResult(pick(result(), i(), "both"))}>{t("conflict.useBoth")}</button>
                </div>
              )}
            </For>
          </div>
          <label class="conflict-result">
            {t("conflict.result")}
            <textarea class="mono" spellcheck={false} value={result()} onInput={(e) => setResult(e.currentTarget.value)} />
          </label>
          <div class="modal-actions">
            <Show when={chunks().length}><span class="muted">{t("conflict.left", { n: chunks().length })}</span></Show>
            <button ref={cancel} onClick={props.onClose}>{t("conflict.cancel")}</button>
            {/* Saving with markers left would commit them: require every block to be resolved. */}
            <button class="primary" disabled={chunks().length > 0} onClick={() => void save()}>{t("conflict.save")}</button>
          </div>
        </Show>
      </dialog>
    </div>
  );
}
