// SPDX-License-Identifier: Apache-2.0
// Working-directory panel: unstaged/staged lists and the commit form (roadmap Phase 2).
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import { api, OpenedRepo, StatusReport, UiError } from "../../api";
import { t } from "../../i18n";
import { groupStatus, WipItem } from "./status";
import { Check, FileCheck2, FilePen, Minus, Plus, TriangleAlert, X } from "lucide-solid";

export interface WipFile { path: string; staged: boolean; untracked: boolean }

export function WipPanel(props: {
  repo: OpenedRepo;
  status: StatusReport | null;
  file: WipFile | null;
  onFile: (f: WipFile | null) => void;
  /** Runs an operation, then refreshes status (and the graph when `graph` is set). */
  run: (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;
  onError: (e: UiError) => void;
}) {
  const groups = createMemo(() => groupStatus(props.status?.entries ?? []));
  const [summary, setSummary] = createSignal("");
  const [body, setBody] = createSignal("");
  const [amend, setAmend] = createSignal(false);
  const [confirm, setConfirm] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const id = props.repo.repoId;

  // Amend prefills the form with the previous message.
  createEffect(
    on(amend, async (on) => {
      if (!on || !props.repo.head) return;
      try {
        const d = await api.details(id, props.repo.head);
        const [first, ...rest] = d.message.split("\n");
        setSummary(first);
        setBody(rest.join("\n").trim());
      } catch (e) {
        props.onError(e as UiError);
      }
    }),
  );

  const canCommit = () => !busy() && summary().trim() !== "" && (amend() || groups().staged.length > 0);
  const message = () => (body().trim() ? `${summary().trim()}\n\n${body().trim()}` : summary().trim());

  async function doCommit() {
    if (!canCommit()) return;
    setBusy(true);
    const wasAmend = amend();
    const msg = message();
    const ok = await props.run(() => api.commit(id, msg, wasAmend), { graph: true, toast: t("toast.committed", { s: msg.split("\n")[0] }) });
    setBusy(false);
    if (ok) {
      setSummary("");
      setBody("");
      setAmend(false);
    }
  }

  function discard(item: WipItem) {
    if (confirm() !== item.path) {
      setConfirm(item.path);
      setTimeout(() => confirm() === item.path && setConfirm(null), 3000);
      return;
    }
    setConfirm(null);
    void props.run(() => api.discard(id, [item.path]), { toast: t("toast.discarded", { s: item.path }) });
  }

  const row = (item: WipItem, staged: boolean) => (
    <li classList={{ active: props.file?.path === item.path && props.file?.staged === staged }}>
      <button
        class="file-open"
        onClick={() => props.onFile({ path: item.path, staged, untracked: item.untracked && !staged })}
        title={item.origPath ? `${item.origPath} → ${item.path}` : item.path}
      >
        <span class={`st st-${item.status === "U" ? "A" : item.status}`}>{item.status}</span>
        <span class="path">{item.path}</span>
      </button>
      <span class="row-actions">
        <Show when={!staged}>
          <button class="mini" onClick={() => props.run(() => api.stage(id, [item.path]))}><Plus size={14} aria-hidden="true" />{t("wip.stage")}</button>
          <button class="mini danger" onClick={() => discard(item)}>
            {confirm() === item.path ? t("wip.discardConfirm") : t("wip.discard")}
          </button>
        </Show>
        <Show when={staged}>
          <button class="mini" onClick={() => props.run(() => api.unstage(id, [item.path]))}><Minus size={14} aria-hidden="true" />{t("wip.unstage")}</button>
        </Show>
      </span>
    </li>
  );

  return (
    <div class="wip">
      <Show when={props.status?.merging}>
        <div class="merge-banner" role="alert">
          <span>{t("merge.banner")}</span>
          <button class="mini danger" onClick={() => void props.run(() => api.mergeAbort(id), { graph: true, toast: t("toast.mergeAborted") })}><X size={14} aria-hidden="true" />{t("merge.abort")}</button>
        </div>
      </Show>
      <Show when={groups().conflicted.length}>
        <section>
          <h3><TriangleAlert size={14} aria-hidden="true" />{t("wip.conflicts")} <span class="muted">{groups().conflicted.length}</span></h3>
          <ul class="files wip-list"><For each={groups().conflicted}>{(i) => <li>
            <span class="st st-conflict">U</span><span class="path">{i.path}</span>
            <button class="mini" onClick={() => void props.run(() => api.resolve(id, i.path, false))}>{t("merge.ours")}</button>
            <button class="mini" onClick={() => void props.run(() => api.resolve(id, i.path, true))}>{t("merge.theirs")}</button>
          </li>}</For></ul>
        </section>
      </Show>
      <section>
        <div class="files-head">
          <h3><FilePen size={14} aria-hidden="true" />{t("wip.unstaged")} <span class="muted">{groups().unstaged.length}</span></h3>
          <Show when={groups().unstaged.length}>
            <button class="mini" onClick={() => props.run(() => api.stage(id, groups().unstaged.map((i) => i.path)))}><Plus size={14} aria-hidden="true" />{t("wip.stageAll")}</button>
          </Show>
        </div>
        <ul class="files wip-list"><For each={groups().unstaged} fallback={<li class="muted pad-s">{t("wip.empty")}</li>}>{(i) => row(i, false)}</For></ul>
      </section>
      <section>
        <div class="files-head">
          <h3><FileCheck2 size={14} aria-hidden="true" />{t("wip.staged")} <span class="muted">{groups().staged.length}</span></h3>
          <Show when={groups().staged.length}>
            <button class="mini" onClick={() => props.run(() => api.unstage(id, groups().staged.map((i) => i.path)))}><Minus size={14} aria-hidden="true" />{t("wip.unstageAll")}</button>
          </Show>
        </div>
        <ul class="files wip-list"><For each={groups().staged} fallback={<li class="muted pad-s">{t("wip.empty")}</li>}>{(i) => row(i, true)}</For></ul>
      </section>
      <form
        class="commit-form"
        onSubmit={(e) => {
          e.preventDefault();
          void doCommit();
        }}
      >
        <label>
          <span class="sr-only">{t("wip.summary")}</span>
          <input
            type="text"
            placeholder={t("wip.summary")}
            value={summary()}
            onInput={(e) => setSummary(e.currentTarget.value)}
            aria-describedby="summary-count"
          />
        </label>
        <span id="summary-count" class="muted count" classList={{ over: summary().length > 72 }}>{summary().length}/72</span>
        <label>
          <span class="sr-only">{t("wip.description")}</span>
          <textarea
            rows="4"
            placeholder={t("wip.description")}
            value={body()}
            onInput={(e) => setBody(e.currentTarget.value)}
            onKeyDown={(e) => e.ctrlKey && e.key === "Enter" && doCommit()}
          />
        </label>
        <label class="check">
          <input type="checkbox" checked={amend()} disabled={!props.repo.head} onChange={(e) => setAmend(e.currentTarget.checked)} />
          {t("wip.amend")}
        </label>
        <button type="submit" class="primary wide" disabled={!canCommit()}>
          <Check size={16} aria-hidden="true" />{amend() ? t("wip.commitAmend") : t("wip.commit")}
        </button>
      </form>
    </div>
  );
}
