// SPDX-License-Identifier: Apache-2.0
// The one confirmation for destructive actions (MOD-04, FUN-07): reset hard, force delete, discard,
// drop stash, forget journal entries, force push. Cancel gets the focus on open, so Enter does not
// destroy anything. Replaces the old "click again to confirm" buttons.
import { JSX } from "solid-js";
import { t } from "../i18n";
import { Modal } from "./Modal";

export interface ConfirmDialogProps {
  title: string;
  /** What is lost or changed, in plain words. */
  message: JSX.Element;
  /** Label of the confirming button, naming the action (e.g. "Reset hard"). */
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  /** Red confirm button. Default true: every current use is destructive. */
  danger?: boolean;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
  let cancel: HTMLButtonElement | undefined;
  return (
    <Modal
      title={props.title}
      size="sm"
      onClose={props.onCancel}
      initialFocus={() => cancel}
      footer={
        <>
          <button ref={cancel} type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
          <button type="button" class="primary" classList={{ danger: props.danger ?? true }} onClick={props.onConfirm}>
            {props.confirmLabel}
          </button>
        </>
      }
    >
      <p>{props.message}</p>
    </Modal>
  );
}
