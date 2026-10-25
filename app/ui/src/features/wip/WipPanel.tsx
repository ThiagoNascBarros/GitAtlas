// SPDX-License-Identifier: Apache-2.0
// Working-directory panel: unstaged/staged lists and the commit form (roadmap Phase 2).
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import { api, OpenedRepo, StatusReport, UiError } from "../../api";
import { t } from "../../i18n";
import { groupStatus, WipItem } from "./status";
import { ConflictEditor } from "../conflict/ConflictEditor";
import { Check, ChevronDown, ChevronRight, FileCheck2, FilePen, GitBranch, Minus, Plus, TriangleAlert, X } from "lucide-solid";

const OPEN_KEY = "gitatlas.wip.sections";
function loadOpen(): { unstaged: boolean; staged: boolean } {
  try {
    const v = JSON.parse(localStorage.getItem(OPEN_KEY) ?? "null");
    if (v && typeof v.unstaged === "boolean" && typeof v.staged === "boolean") return v;
  } catch { /* storage unavailable or corrupt */ }
  return { unstaged: true, staged: true };
}

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
  const [editing, setEditing] = createSignal<string | null>(null);
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

  // Collapsed sections, remembered across repositories and restarts.
  const [open, setOpen] = createSignal<{ unstaged: boolean; staged: boolean }>(loadOpen());
  const toggle = (k: "unstaged" | "staged") => {
    const next = { ...open(), [k]: !open()[k] };
    setOpen(next);
    try { localStorage.setItem(OPEN_KEY, JSON.stringify(next)); } catch { /* storage unavailable */ }
  };
  const commitLabel = () => {
    if (amend()) return t("wip.commitAmend");
    const n = groups().staged.length;
    const b = props.repo.branch;
    if (!n || !b) return t("wip.commit");
    return n === 1 ? t("wip.commitOneTo", { s: b }) : t("wip.commitTo", { n, s: b });
  };
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

  // Id of a row's path text: the row's buttons are described by it ("Stage" → which file).
  const pathId = (path: string, list: string) => `wip-${list}-${encodeURIComponent(path)}`;
  const row = (item: WipItem, staged: boolean) => (
    <li classList={{ active: props.file?.path === item.path && props.file?.staged === staged }} data-focus-key={item.path}>
      <button
        class="file-open"
        onClick={() => props.onFile({ path: item.path, staged, untracked: item.untracked && !staged })}
        title={item.origPath ? `${item.origPath} → ${item.path}` : item.path}
      >
        <span class={`st st-${item.status === "U" ? "A" : item.status}`}>{item.status}</span>
        <span class="path" id={pathId(item.path, staged ? "s" : "u")}>{item.path}</span>
      </button>
      <span class="row-actions">
        <Show when={!staged}>
          <button class="mini" aria-describedby={pathId(item.path, "u")} onClick={() => props.run(() => api.stage(id, [item.path]))}><Plus size={14} aria-hidden="true" />{t("wip.stage")}</button>
          <button class="mini danger" aria-describedby={pathId(item.path, "u")} onClick={() => discard(item)}>
            {confirm() === item.path ? t("wip.discardConfirm") : t("wip.discard")}
          </button>
        </Show>
        <Show when={staged}>
          <button class="mini" aria-describedby={pathId(item.path, "s")} onClick={() => props.run(() => api.unstage(id, [item.path]))}><Minus size={14} aria-hidden="true" />{t("wip.unstage")}</button>
        </Show>
      </span>
    </li>
  );

  return (
    <div class="wip">
      <header class="wip-head">
        <h2>{t("wip.panelTitle")}</h2>
        <span class="muted"><GitBranch size={13} aria-hidden="true" />{props.repo.branch ? t("wip.onBranch", { s: props.repo.branch }) : t("wip.detached")}</span>
      </header>
      <Show when={editing()}>
        {(p) => <ConflictEditor repoId={id} path={p()} run={props.run} onClose={() => setEditing(null)} onError={(e) => props.onError(e as UiError)} />}
      </Show>
      <Show when={props.status?.merging}>
        <div class="merge-banner" role="alert">
          <span>{t("merge.banner", { s: props.status?.in_progress ?? "merge" })}</span>
          <Show when={props.status?.in_progress === "rebase"}>
            <button class="mini" onClick={() => void props.run(() => api.rebaseContinue(id), { graph: true, toast: t("toast.rebased") })}>{t("rebase.continue")}</button>
          </Show>
          <button class="mini danger" onClick={() => void props.run(() => api.mergeAbort(id), { graph: true, toast: t("toast.mergeAborted") })}><X size={14} aria-hidden="true" />{t("merge.abort")}</button>
        </div>
      </Show>
      <Show when={groups().conflicted.length}>
        <section>
          <h3><TriangleAlert size={14} aria-hidden="true" />{t("wip.conflicts")} <span class="muted">{groups().conflicted.length}</span></h3>
          <ul class="files wip-list" data-focus-group="wip-conflicted"><For each={groups().conflicted}>{(i) => <li data-focus-key={i.path}>
            <span class="st st-conflict">U</span><span class="path" id={pathId(i.path, "c")}>{i.path}</span>
            <button class="mini" aria-describedby={pathId(i.path, "c")} onClick={() => void props.run(() => api.resolve(id, i.path, false))}>{t("merge.ours")}</button>
            <button class="mini" aria-describedby={pathId(i.path, "c")} onClick={() => void props.run(() => api.resolve(id, i.path, true))}>{t("merge.theirs")}</button>
            <button class="mini" aria-describedby={pathId(i.path, "c")} onClick={() => setEditing(i.path)}>{t("merge.edit")}</button>
          </li>}</For></ul>
        </section>
      </Show>
      <section class="wip-sec unstaged" data-focus-group="wip-unstaged-section" classList={{ closed: !open().unstaged }}>
        <div class="files-head">
          <h3>
            <button class="sec-toggle" aria-expanded={open().unstaged} onClick={() => toggle("unstaged")}>
              <Show when={open().unstaged} fallback={<ChevronRight size={14} aria-hidden="true" />}><ChevronDown size={14} aria-hidden="true" /></Show>
              <FilePen size={14} aria-hidden="true" />{t("wip.unstaged")} <span class="muted">{groups().unstaged.length}</span>
            </button>
          </h3>
          <Show when={groups().unstaged.length}>
            <button class="mini" onClick={() => props.run(() => api.stage(id, groups().unstaged.map((i) => i.path)))}><Plus size={14} aria-hidden="true" />{t("wip.stageAll")}</button>
          </Show>
        </div>
        <Show when={open().unstaged}>
        <p class="wip-hint muted">{t("wip.unstagedHint")}</p>
        <ul class="files wip-list" data-focus-group="wip-unstaged"><For each={groups().unstaged} fallback={<li class="muted pad-s">{t("wip.empty")}</li>}>{(i) => row(i, false)}</For></ul>
        </Show>
      </section>
      <section class="wip-sec staged" data-focus-group="wip-staged-section" classList={{ closed: !open().staged }}>
        <div class="files-head">
          <h3>
            <button class="sec-toggle" aria-expanded={open().staged} onClick={() => toggle("staged")}>
              <Show when={open().staged} fallback={<ChevronRight size={14} aria-hidden="true" />}><ChevronDown size={14} aria-hidden="true" /></Show>
              <FileCheck2 size={14} aria-hidden="true" />{t("wip.staged")} <span class="muted">{groups().staged.length}</span>
            </button>
          </h3>
          <Show when={groups().staged.length}>
            <button class="mini" onClick={() => props.run(() => api.unstage(id, groups().staged.map((i) => i.path)))}><Minus size={14} aria-hidden="true" />{t("wip.unstageAll")}</button>
          </Show>
        </div>
        <Show when={open().staged}>
        <p class="wip-hint muted">{t("wip.stagedHint")}</p>
        <ul class="files wip-list" data-focus-group="wip-staged"><For each={groups().staged} fallback={<li class="muted pad-s">{t("wip.empty")}</li>}>{(i) => row(i, true)}</For></ul>
        </Show>
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
          <Check size={16} aria-hidden="true" />{commitLabel()}
        </button>
        <Show when={!busy() && !canCommit()}>
          <p class="wip-hint wip-why muted">{!amend() && !groups().staged.length ? t("wip.needStaged") : t("wip.needSummary")}</p>
        </Show>
      </form>
    </div>
  );
}
