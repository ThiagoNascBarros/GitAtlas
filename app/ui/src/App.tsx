// SPDX-License-Identifier: Apache-2.0
// App frame (roadmap T1.7): tabs, toolbar, refs panel, graph/diff, details, status bar.
import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, OpenedRepo, StatusReport, UiError } from "./api";
import { listen } from "@tauri-apps/api/event";
import { GraphView } from "./features/graph/GraphView";
import { DiffView } from "./features/diff/DiffView";
import { setTheme, themeChoice, ThemeChoice } from "./theme/theme";
import { errorFix, errorText, t } from "./i18n";
import { JournalPanel } from "./features/journal/JournalPanel";
import { WipFile, WipPanel } from "./features/wip/WipPanel";
import { changeCount, groupStatus } from "./features/wip/status";
import { toTreeRows } from "./features/files/tree";
import { markOpenStart, togglePerf } from "./features/graph/perf";

interface Tab {
  repo: OpenedRepo; selected: number | null; sha: string | null; diffPath: string | null;
  status: StatusReport | null; wip: boolean; wipFile: WipFile | null;
  /** Bumped when history changes so the graph reloads; `diffRev` when a diff must be re-read. */
  graphRev: number; diffRev: number;
}
interface Toast { text: string; repoId: number; undoId: string | null }

export function App() {
  const [tabs, setTabs] = createSignal<Tab[]>([]);
  const [active, setActive] = createSignal(0);
  const [error, setError] = createSignal<UiError | null>(null);
  const [fileView, setFileView] = createSignal<"path" | "tree">("path");
  const [toast, setToast] = createSignal<Toast | null>(null);
  const [refFilter, setRefFilter] = createSignal("");
  const [showJournal, setShowJournal] = createSignal(false);
  const [showDetail, setShowDetail] = createSignal(false);
  let toastTimer: number | undefined;
  const tab = () => tabs()[active()];
  const patch = (p: Partial<Tab>) => setTabs(tabs().map((x, i) => (i === active() ? { ...x, ...p } : x)));
  const patchRepo = (repoId: number, f: (x: Tab) => Partial<Tab>) =>
    setTabs(tabs().map((x) => (x.repo.repoId === repoId ? { ...x, ...f(x) } : x)));

  function fail(e: unknown) {
    setShowDetail(false);
    setError(typeof e === "object" && e && "code" in e ? (e as UiError) : { code: "UNKNOWN", message: String(e) });
  }

  async function loadStatus(repoId: number) {
    try {
      const status = await api.status(repoId);
      const cur = tabs().find((x) => x.repo.repoId === repoId);
      if (!cur) return;
      // History moved outside the app (terminal, another tool): reload the graph too.
      if (status.oid !== cur.repo.head || status.branch !== cur.repo.branch) await refreshRepo(repoId);
      patchRepo(repoId, () => ({ status }));
      const g = groupStatus(status.entries);
      const still = (f: WipFile | null) =>
        f && (f.staged ? g.staged : g.unstaged).some((i) => i.path === f.path) ? f : null;
      patchRepo(repoId, (x) => ({ wipFile: still(x.wipFile), diffRev: x.diffRev + 1 }));
    } catch (e) {
      fail(e);
    }
  }

  async function refreshRepo(repoId: number) {
    const repo = await api.refresh(repoId);
    patchRepo(repoId, (x) => ({ repo, graphRev: x.graphRev + 1, selected: null, sha: null, diffPath: null }));
  }

  /** Runs an operation, then brings status (and optionally history) up to date. */
  async function run(op: () => Promise<unknown>, opts: { graph?: boolean; toast?: string } = {}): Promise<boolean> {
    const repoId = tab().repo.repoId;
    try {
      const out = (await op()) as { id?: string | null } | undefined;
      setError(null);
      if (opts.graph) await refreshRepo(repoId);
      await loadStatus(repoId);
      if (opts.toast) {
        clearTimeout(toastTimer);
        setToast({ text: opts.toast, repoId, undoId: out?.id ?? null });
        toastTimer = window.setTimeout(() => setToast(null), 10000);
      }
      return true;
    } catch (e) {
      fail(e);
      await loadStatus(repoId);
      return false;
    }
  }

  async function redo(repoId: number) {
    try {
      await api.redo(repoId);
      setError(null);
      await refreshRepo(repoId);
      await loadStatus(repoId);
      clearTimeout(toastTimer);
      setToast({ text: t("toast.redone"), repoId, undoId: null });
      toastTimer = window.setTimeout(() => setToast(null), 4000);
    } catch (e) {
      fail(e);
    }
  }

  async function undo(repoId: number, id: string | null) {
    try {
      if (!id) {
        const last = (await api.journal(repoId)).reverse().find((e) => !e.undone);
        if (!last) return setToast({ text: t("toast.nothingToUndo"), repoId, undoId: null });
        id = last.id;
      }
      await api.undo(repoId, id);
      setError(null);
      await refreshRepo(repoId);
      await loadStatus(repoId);
      setToast({ text: t("toast.undone"), repoId, undoId: null });
      clearTimeout(toastTimer);
      toastTimer = window.setTimeout(() => setToast(null), 4000);
    } catch (e) {
      fail(e);
    }
  }

  async function openPath(path: string) {
    try {
      markOpenStart();
      const repo = await api.open(path);
      setError(null);
      setTabs([...tabs(), { repo, selected: null, sha: null, diffPath: null, status: null, wip: false, wipFile: null, graphRev: 1, diffRev: 0 }]);
      setActive(tabs().length - 1);
      await loadStatus(repo.repoId);
      const st = tabs().find((x) => x.repo.repoId === repo.repoId)?.status;
      if (st && st.entries.length) patchRepo(repo.repoId, () => ({ wip: true }));
    } catch (e) {
      setError(e as UiError);
    }
  }

  async function pick() {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir === "string") openPath(dir);
  }

  function closeTab(i: number) {
    api.close(tabs()[i].repo.repoId);
    setTabs(tabs().filter((_, k) => k !== i));
    setActive(Math.max(0, Math.min(active(), tabs().length - 1)));
  }

  onMount(() => {
    window.addEventListener("keydown", (e) => {
      const typing = /^(INPUT|TEXTAREA|SELECT)$/.test((e.target as HTMLElement)?.tagName ?? "");
      if (e.ctrlKey && !typing && e.key.toLowerCase() === "z" && tab()) {
        e.preventDefault();
        if (e.shiftKey) redo(tab().repo.repoId);
        else undo(tab().repo.repoId, null);
      } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        togglePerf();
      }
    });
  });

  onMount(async () => {
    try {
      // The core watches the working tree and emits this (debounced) on any change.
      await listen<number>("repo-changed", (e) => loadStatus(e.payload));
      window.addEventListener("focus", () => tab() && loadStatus(tab().repo.repoId));
    } catch { /* no event bridge (plain browser) */ }
    const p = await api.initialPath();
    if (p) openPath(p);
  });

  const wipCount = () => (tab()?.status ? changeCount(groupStatus(tab()!.status!.entries)) : 0);

  const [details] = createResource(
    () => (tab()?.sha ? ([tab()!.repo.repoId, tab()!.sha!] as const) : null),
    ([id, sha]) => api.details(id, sha),
  );

  return (
    <div class="app">
      <nav class="tabbar" role="tablist" aria-label="Repositories">
        <For each={tabs()}>
          {(x, i) => (
            <div class="tab" role="tab" aria-selected={i() === active()} classList={{ active: i() === active() }}>
              <button class="tab-name" onClick={() => setActive(i())}>{x.repo.name}</button>
              <button class="icon" aria-label={`Close ${x.repo.name}`} onClick={() => closeTab(i())}>✕</button>
            </div>
          )}
        </For>
        <button class="icon tab-add" aria-label={t("open.button")} onClick={pick}>＋</button>
      </nav>

      <header class="toolbar">
        <div class="crumb">
          <strong>{tab()?.repo.name ?? t("app.name")}</strong>
          <Show when={tab()?.repo.branch}><span class="muted">›</span><span>{tab()!.repo.branch}</span></Show>
        </div>
        <Show when={tab()?.repo.readOnlyReason}><span class="badge">{t("toolbar.readOnly")}</span></Show>
        <button disabled={!tab()} title={t("toolbar.undoTitle")} onClick={() => undo(tab().repo.repoId, null)}>↶ {t("toolbar.undo")}</button>
        <button disabled={!tab()} title={t("toolbar.redoTitle")} onClick={() => redo(tab().repo.repoId)}>↷ {t("toolbar.redo")}</button>
        <button disabled={!tab()} title={t("toolbar.journalTitle")} onClick={() => setShowJournal(true)}>{t("toolbar.journal")}</button>
        <label class="theme-pick">
          {t("toolbar.theme")}
          <select value={themeChoice()} onChange={(e) => setTheme(e.currentTarget.value as ThemeChoice)}>
            <option value="system">{t("theme.system")}</option>
            <option value="atlas-dark">{t("theme.dark")}</option>
            <option value="atlas-light">{t("theme.light")}</option>
          </select>
        </label>
        <button class="primary" onClick={pick}>{t("open.button")}</button>
      </header>

      <Show
        when={tab()}
        fallback={
          <main class="empty">
            <h1>{t("open.title")}</h1>
            <p class="muted">{t("open.hint")}</p>
            <Show when={error()}>
              <div class="error" role="alert"><strong>{t("error.title")}</strong><p>{error()!.message}</p></div>
            </Show>
            <button class="primary" onClick={pick}>{t("open.button")}</button>
          </main>
        }
      >
        <div class="body">
          <aside class="left" aria-label={t("panel.refs")}>
            <input class="ref-filter" type="search" placeholder={t("panel.filter")} aria-label={t("panel.filter")}
              value={refFilter()} onInput={(e) => setRefFilter(e.currentTarget.value)} />
            <For each={[["head", "panel.local"], ["remote", "panel.remote"], ["tag", "panel.tags"]] as const}>
              {([kind, label]) => {
                const items = () => tab()!.repo.refs.filter((r) => r.kind === kind && r.name.toLowerCase().includes(refFilter().toLowerCase()));
                return (
                  <Show when={items().length}>
                    <details open={kind !== "remote" || !!refFilter()}>
                    <summary><h2>{t(label)} <span class="muted">{items().length}</span></h2></summary>
                    <ul>
                      <For each={items()}>
                        {(r) => (
                          <li>
                            <button
                              classList={{ head: r.isHead }}
                              disabled={r.row === null}
                              onClick={() => r.row !== null && patch({ selected: r.row, sha: null })}
                            >
                              {r.name}
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                    </details>
                  </Show>
                );
              }}
            </For>
          </aside>

          <main class="center">
            <button
              class="wip-row"
              classList={{ active: tab()!.wip }}
              onClick={() => patch({ wip: true, selected: null, sha: null, diffPath: null })}
            >
              <strong>{t("wip.row")}</strong>
              <span class="muted">
                {tab()!.status ? (wipCount() ? t("wip.changes", { n: wipCount() }) : t("wip.clean")) : ""}
              </span>
            </button>
            <Show
              when={tab()!.wip && tab()!.wipFile}
              fallback={
                <Show
                  when={tab()!.diffPath && tab()!.sha}
                  fallback={
                    <Show when={tab()!.graphRev} keyed>
                      {(_) => (
                        <GraphView
                          repo={tab()!.repo}
                          selected={tab()!.selected}
                          onSelect={(row, sha) => patch({ selected: row, sha, diffPath: null, wip: false })}
                        />
                      )}
                    </Show>
                  }
                >
                  <DiffView
                    repoId={tab()!.repo.repoId}
                    source={{ kind: "commit", sha: tab()!.sha! }}
                    path={tab()!.diffPath!}
                    onClose={() => patch({ diffPath: null })}
                  />
                </Show>
              }
            >
              <DiffView
                repoId={tab()!.repo.repoId}
                path={tab()!.wipFile!.path}
                source={{
                  kind: "wip",
                  staged: tab()!.wipFile!.staged,
                  untracked: tab()!.wipFile!.untracked,
                  rev: tab()!.diffRev,
                  onHunk: (i) => run(() => api.stageHunk(tab()!.repo.repoId, tab()!.wipFile!.path, i, tab()!.wipFile!.staged)),
                }}
                onClose={() => patch({ wipFile: null })}
              />
            </Show>
          </main>

          <aside class="right" aria-label="Commit details">
            <Show when={tab()!.wip}>
              <WipPanel
                repo={tab()!.repo}
                status={tab()!.status}
                file={tab()!.wipFile}
                onFile={(f) => patch({ wipFile: f })}
                run={run}
                onError={fail}
              />
            </Show>
            <Show when={!tab()!.wip}><Show when={details()} fallback={<p class="muted pad">{t("details.select")}</p>}>
              {(d) => (
                <div class="details">
                  <p class="sha mono">{d().sha.slice(0, 10)}</p>
                  <h2 class="message">{d().message.split("\n")[0]}</h2>
                  <Show when={d().message.includes("\n")}><pre class="body-text">{d().message.split("\n").slice(1).join("\n").trim()}</pre></Show>
                  <p class="meta">{d().author} · {new Date(d().time * 1000).toLocaleString()}</p>
                  <p class="meta">{t("details.parents")}: <span class="mono">{d().parents.map((p) => p.slice(0, 7)).join(", ") || "—"}</span></p>
                  <div class="files-head">
                    <h3>{t("details.files")} <span class="muted">{d().files.length}</span></h3>
                    <div class="seg" role="group" aria-label={t("files.view")}>
                      <button aria-pressed={fileView() === "path"} onClick={() => setFileView("path")}>{t("files.path")}</button>
                      <button aria-pressed={fileView() === "tree"} onClick={() => setFileView("tree")}>{t("files.tree")}</button>
                    </div>
                  </div>
                  <ul class="files">
                    <Show
                      when={fileView() === "tree"}
                      fallback={
                        <For each={d().files}>
                          {(f) => (
                            <li>
                              <button classList={{ active: tab()!.diffPath === f.path }} onClick={() => patch({ diffPath: f.path })}>
                                <span class={`st st-${f.status}`}>{f.status}</span>
                                <span class="path">{f.path}</span>
                              </button>
                            </li>
                          )}
                        </For>
                      }
                    >
                      <For each={toTreeRows(d().files)}>
                        {(r) =>
                          r.kind === "dir" ? (
                            <li class="dir" style={{ "padding-left": `${8 + r.depth * 14}px` }}>▾ {r.name}</li>
                          ) : (
                            <li>
                              <button
                                style={{ "padding-left": `${8 + r.depth * 14}px` }}
                                classList={{ active: tab()!.diffPath === r.file.path }}
                                onClick={() => patch({ diffPath: r.file.path })}
                                title={r.file.path}
                              >
                                <span class={`st st-${r.file.status}`}>{r.file.status}</span>
                                <span class="path">{r.name}</span>
                              </button>
                            </li>
                          )
                        }
                      </For>
                    </Show>
                  </ul>
                </div>
              )}
            </Show></Show>
          </aside>
        </div>
      </Show>

      <Show when={error() && tab()}>
        <div class="banner error" role="alert">
          <div>
            <strong>{errorText(error()!)}</strong>
            <Show when={errorFix(error()!)}><p>{errorFix(error()!)}</p></Show>
            <Show when={showDetail() && error()!.detail}><pre class="mono">{error()!.detail}</pre></Show>
          </div>
          <Show when={error()!.detail}><button onClick={() => setShowDetail(!showDetail())}>{t("err.details")}</button></Show>
          <button onClick={() => setError(null)}>{t("err.dismiss")}</button>
        </div>
      </Show>
      <Show when={showJournal() && tab()}>
        <JournalPanel repoId={tab()!.repo.repoId} onClose={() => setShowJournal(false)} onError={(e) => fail(e)} />
      </Show>
      <Show when={toast()}>
        <div class="toast" role="status">
          <span>{toast()!.text}</span>
          <Show when={toast()!.undoId}>
            <button onClick={() => { const x = toast()!; setToast(null); undo(x.repoId, x.undoId); }}>{t("toast.undo")}</button>
          </Show>
        </div>
      </Show>

      <footer class="statusbar">
        <Show when={tab()}>
          <span>{t("status.commits", { n: tab()!.repo.rowCount.toLocaleString() })}</span>
          <span>{t("status.git", { v: tab()!.repo.gitVersion })}</span>
          <Show when={tab()!.repo.readOnlyReason}><span class="warn">{tab()!.repo.readOnlyReason}</span></Show>
        </Show>
      </footer>
    </div>
  );
}
