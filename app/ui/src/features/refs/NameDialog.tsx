// SPDX-License-Identifier: Apache-2.0
import { createSignal } from "solid-js";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";

/**
 * Modal with one text field (branch name, stash message). Only Cancel/Escape close it:
 * a backdrop click (or a selection dragged out of the input) must not lose typed text.
 */
export function NameDialog(props: { label: string; initial?: string; optional?: boolean; onSubmit: (v: string) => void; onCancel: () => void }) {
  const [value, setValue] = createSignal(props.initial ?? "");
  let input: HTMLInputElement | undefined;
  useOverlay(() => props.onCancel(), () => input);
  const submit = (e: Event) => {
    e.preventDefault();
    const v = value().trim();
    if (v || props.optional) props.onSubmit(v);
  };
  return (
    <div class="modal-back" onContextMenu={(e) => e.preventDefault()}>
      <dialog open class="modal" aria-label={props.label}>
        <form onSubmit={submit}>
          <label>{props.label}
            <input ref={(el) => { input = el; queueMicrotask(() => el.select()); }} value={value()} onInput={(e) => setValue(e.currentTarget.value)} />
          </label>
          <div class="modal-actions">
            <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
            <button type="submit" class="primary" disabled={!props.optional && !value().trim()}>{t("refmenu.ok")}</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
