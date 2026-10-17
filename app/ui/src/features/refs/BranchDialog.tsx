// SPDX-License-Identifier: Apache-2.0
import { createEffect, createSignal, createUniqueId, For, onCleanup, Show } from "solid-js";
import { api } from "../../api";
import { type Key, t } from "../../i18n";
import { Modal } from "../../components";
import { canCreate, type BranchKind, DEFAULT_BRANCH_TEMPLATE, effectiveBase, isFlowKind, kindOptions } from "./branchDialog.logic";
import "./branchDialog.css";

export interface NewBranch {
  name: string;
  base: string;
  checkout: boolean;
}

const KIND_LABEL: Record<BranchKind, Key> = {
  free: "branch.kind.free",
  feature: "branch.kind.feature",
  fix: "branch.kind.fix",
  release: "branch.kind.release",
  hotfix: "branch.kind.hotfix",
};

/**
 * "New branch": type, ticket, title, base and checkout. Name and Git Flow base are computed by
 * the core (`branch_plan`) so the preview is the name that will really be created. Only Cancel
 * or Escape closes it, so typed text is not lost on a backdrop click.
 */
export function BranchDialog(props: {
  /** Local branch names, for the base choice. */
  branches: string[];
  flowEnabled: boolean;
  main: string;
  develop: string;
  template?: string;
  onSubmit: (b: NewBranch) => void;
  onCancel: () => void;
}) {
  const [kind, setKind] = createSignal<BranchKind>("free");
  const [ticket, setTicket] = createSignal("");
  const [title, setTitle] = createSignal("");
  const [chosenBase, setChosenBase] = createSignal("HEAD");
  const [checkout, setCheckout] = createSignal(true);
  // The preview is tagged with the request it answers, so a slow answer never applies to newer input.
  const [preview, setPreview] = createSignal<{ key: string; name: string; base: string } | null>(null);
  const [previewFailed, setPreviewFailed] = createSignal(false);
  const key = () => `${kind()}\n${ticket()}\n${title()}`;
  const formId = createUniqueId();
  let first: HTMLInputElement | undefined;

  createEffect(() => {
    const req = { kind: kind(), ticket: ticket(), title: title(), k: key() };
    let stale = false;
    onCleanup(() => {
      stale = true;
    });
    api
      .branchPlan(req.kind, props.template ?? DEFAULT_BRANCH_TEMPLATE, req.ticket, req.title, props.main, props.develop)
      .then((p) => {
        if (stale) return;
        setPreview({ key: req.k, name: p.name, base: p.base });
        setPreviewFailed(false);
      })
      .catch(() => {
        if (stale) return;
        setPreview(null);
        setPreviewFailed(true);
      });
  });

  // Only a preview for the current inputs counts.
  const current = () => (preview()?.key === key() ? preview() : null);
  const coreBase = () => current()?.base;
  const base = () => effectiveBase(kind(), coreBase(), chosenBase());
  const ok = () => canCreate(current(), previewFailed(), base());

  const submit = (e: Event) => {
    e.preventDefault();
    const p = current();
    const b = base();
    if (!p || b === null || !ok()) return;
    props.onSubmit({ name: p.name, base: b, checkout: checkout() });
  };

  // Modal provides the frame, Escape, the close button and focus; the form inside submits with Enter.
  return (
    <Modal
      title={t("branch.title")}
      size="md"
      onClose={props.onCancel}
      initialFocus={() => first}
      footer={
        <>
          <button type="button" onClick={props.onCancel}>{t("refmenu.cancel")}</button>
          <button type="submit" form={formId} class="primary" disabled={!ok()}>{t("branch.create")}</button>
        </>
      }
    >
        <form id={formId} class="branch-form" onSubmit={submit}>
          <label>
            <span>{t("branch.type")}</span>
            <select value={kind()} onChange={(e) => setKind(e.currentTarget.value as BranchKind)}>
              <For each={kindOptions(props.flowEnabled)}>{(k) => <option value={k}>{t(KIND_LABEL[k])}</option>}</For>
            </select>
          </label>
          <div class="branch-row">
            <label>
              <span>{t("branch.ticket")}</span>
              <input ref={(el) => { first = el; }} type="text" autocomplete="off" placeholder="ACT-1010" value={ticket()} onInput={(e) => setTicket(e.currentTarget.value)} />
            </label>
            <label>
              <span>{t("branch.titleField")}</span>
              <input type="text" autocomplete="off" value={title()} onInput={(e) => setTitle(e.currentTarget.value)} />
            </label>
          </div>
          <Show when={!isFlowKind(kind())} fallback={<p class="hint">{t("branch.flowBase", { base: coreBase() ?? "…" })}</p>}>
            <label>
              <span>{t("branch.base")}</span>
              <select value={chosenBase()} onChange={(e) => setChosenBase(e.currentTarget.value)}>
                <option value="HEAD">{t("branch.headBase")}</option>
                <For each={props.branches}>{(b) => <option value={b}>{b}</option>}</For>
              </select>
            </label>
          </Show>
          <output class="branch-preview" aria-live="polite">
            <span>{t("branch.preview")}: </span>
            <code>{current()?.name ?? ""}</code>
            <Show when={previewFailed()}><span class="branch-error" role="alert"> {t("branch.previewFailed")}</span></Show>
          </output>
          <label class="check">
            <input type="checkbox" checked={checkout()} onChange={(e) => setCheckout(e.currentTarget.checked)} />
            <span>{t("branch.checkout")}</span>
          </label>
        </form>
    </Modal>
  );
}
