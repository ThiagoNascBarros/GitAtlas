// SPDX-License-Identifier: Apache-2.0
// Base dialog for every modal in the app (MOD-02). Usage:
//   <Modal title="..." size="sm|md|lg|full" onClose={close} footer={<>buttons</>} initialFocus={() => el}>body</Modal>
// - Sizes: sm 400 px (confirmations, one field), md 560 px (default), lg 820 px, full (almost the whole window).
// - Header with a title and a close (X) button; the body scrolls; the footer stays visible.
// - Rendered in a Portal on <body>: panels with backdrop-filter would clip a `position: fixed` dialog.
// - Escape and X call onClose after the close fade (no delay with reduced motion).
// - A click on the backdrop does NOT close it, so typed text is not lost by a stray click.
// - Focus, Escape and the Tab trap come from features/overlay.ts (useOverlay).
import { createSignal, createUniqueId, JSX, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { X } from "lucide-solid";
import { t } from "../i18n";
import { focusables, useOverlay } from "../features/overlay";

export type ModalSize = "sm" | "md" | "lg" | "full";

/** Same as `--ak-motion-fast` in styles.css: the close fade finishes before the dialog unmounts. */
const CLOSE_MS = 90;

export interface ModalProps {
  title: string;
  size?: ModalSize;
  /** Called when the dialog should go away (Escape, X, or a button the caller wired to it). */
  onClose: () => void;
  /** Buttons of the fixed footer, usually `<button>`s in the order cancel, primary. */
  footer?: JSX.Element;
  /** Control that gets the focus on open. Default: the first control of the body. */
  initialFocus?: () => HTMLElement | undefined;
  children: JSX.Element;
}

const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The portal wrapper. The content is a child component on purpose: its onMount (focus on open) runs
 *  after the Portal has attached the dialog to the document. */
export function Modal(props: ModalProps) {
  return <Portal><ModalContent {...props} /></Portal>;
}

function ModalContent(props: ModalProps) {
  const titleId = createUniqueId();
  const [closing, setClosing] = createSignal(false);
  let dialog: HTMLDialogElement | undefined;
  const close = () => {
    if (closing()) return;
    if (reducedMotion()) return props.onClose();
    setClosing(true);
    window.setTimeout(() => props.onClose(), CLOSE_MS);
  };
  const firstControl = () => {
    if (!dialog) return undefined;
    const items = focusables(dialog);
    return items.find((el) => !el.classList.contains("modal-x")) ?? items[0] ?? dialog;
  };
  useOverlay(close, () => props.initialFocus?.() ?? firstControl(), () => dialog);
  return (
    <div class="modal-back" classList={{ closing: closing() }} onContextMenu={(e) => e.preventDefault()}>
      <dialog ref={dialog} open tabIndex={-1} class={`modal modal-base modal-${props.size ?? "md"}`} aria-modal="true" aria-labelledby={titleId}>
        <header class="modal-head">
          <h2 id={titleId}>{props.title}</h2>
          <button type="button" class="modal-x" aria-label={t("modal.close")} title={t("modal.close")} onClick={close}>
            <X size={16} aria-hidden="true" />
          </button>
        </header>
        <div class="modal-body">{props.children}</div>
        <Show when={props.footer}>
          <footer class="modal-actions modal-foot">{props.footer}</footer>
        </Show>
      </dialog>
    </div>
  );
}
