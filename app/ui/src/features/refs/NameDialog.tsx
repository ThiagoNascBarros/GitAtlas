// SPDX-License-Identifier: Apache-2.0
import { createSignal, createUniqueId } from "solid-js";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";

/**
 * Modal with one text field (branch name, stash message). Only Cancel/Escape close it:
 * a backdrop click (or a selection dragged out of the input) must not lose typed text.
 */
export function NameDialog(props: { label: string; initial?: string; optional?: boolean; secret?: boolean; onSubmit: (v: string) => void; onCancel: () => void }) {
  const [value, setValue] = createSignal(props.initial ?? "");
  let input: HTMLInputElement | undefined;
  let dialog: HTMLDialogElement | undefined;
  const labelId = createUniqueId();
  useOverlay(() => props.onCancel(), () => input, () => dialog);
  const submit = (e: Event) => {
    e.preventDefault();
    // Passwords are sent exactly as typed.
    const v = props.secret ? value() : value().trim();
    if (v || props.optional) props.onSubmit(v);
  };
  return (
    <div class="modal-back" onContextMenu={(e) => e.preventDefault()}>
      <dialog ref={dialog} open class="modal" aria-modal="true" aria-labelledby={labelId}>
        <form onSubmit={submit}>
          <label><span id={labelId}>{props.label}</span>
            <input type={props.secret ? "password" : "text"} autocomplete="off" ref={(el) => { input = el; queueMicrotask(() => el.select()); }} value={value()} onInput={(e) => setValue(e.currentTarget.value)} />
          </label>
          <div class="modal-actions">
            <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
            <button type="submit" class="primary" disabled={!props.optional && !value()}>{t("refmenu.ok")}</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
