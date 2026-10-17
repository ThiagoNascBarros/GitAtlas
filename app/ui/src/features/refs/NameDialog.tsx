// SPDX-License-Identifier: Apache-2.0
import { createSignal } from "solid-js";
import { t } from "../../i18n";
import { Modal } from "../../components";

/**
 * Small modal with one text field (branch name, stash message, credential). The label is the
 * title. Only Cancel/Escape/X close it: a backdrop click must not lose typed text.
 */
export function NameDialog(props: { label: string; initial?: string; optional?: boolean; secret?: boolean; onSubmit: (v: string) => void; onCancel: () => void }) {
  const [value, setValue] = createSignal(props.initial ?? "");
  let input: HTMLInputElement | undefined;
  // The footer button is outside the form: it submits the form through requestSubmit().
  let form: HTMLFormElement | undefined;
  const submit = (e: Event) => {
    e.preventDefault();
    // Passwords are sent exactly as typed.
    const v = props.secret ? value() : value().trim();
    if (v || props.optional) props.onSubmit(v);
  };
  return (
    <Modal
      title={props.label}
      size="sm"
      onClose={props.onCancel}
      initialFocus={() => input}
      footer={
        <>
          <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
          <button type="button" class="primary" disabled={!props.optional && !value()} onClick={() => form?.requestSubmit()}>{t("refmenu.ok")}</button>
        </>
      }
    >
      <form ref={form} onSubmit={submit}>
        <input
          type={props.secret ? "password" : "text"}
          aria-label={props.label}
          autocomplete="off"
          ref={(el) => { input = el; queueMicrotask(() => el.select()); }}
          value={value()}
          onInput={(e) => setValue(e.currentTarget.value)}
        />
      </form>
    </Modal>
  );
}
