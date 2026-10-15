// SPDX-License-Identifier: Apache-2.0
// App frame (roadmap T1.7): tabs, toolbar, refs panel, graph/diff, details, status bar.
import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { CredentialsRequest, api, OpenedRepo, StatusReport, UiError } from "./api";
import { listen } from "@tauri-apps/api/event";
import { GraphView } from "./features/graph/GraphView";
import { DiffView } from "./features/diff/DiffView";
import { setTheme, themeChoice, ThemeChoice } from "./theme/theme";
import { errorFix, errorText, t } from "./i18n";
import { StashPanel } from "./features/stash/StashPanel";
import { CommitMenu, CommitMenuState } from "./features/graph/CommitMenu";
import { RebaseEditor } from "./features/rebase/RebaseEditor";
import { RefMenu, RefMenuState } from "./features/refs/RefMenu";
import { JournalPanel } from "./features/journal/JournalPanel";
import { NameDialog } from "./features/refs/NameDialog";
import { filterRefs } from "./features/refs/filter";
import { WipFile, WipPanel } from "./features/wip/WipPanel";
import { changeCount, groupStatus } from "./features/wip/status";
import { toTreeRows } from "./features/files/tree";
import { markOpenStart, togglePerf } from "./features/graph/perf";
import { ArrowDownToLine, ArrowUpFromLine, CloudDownload, Archive, Clock, Plus, Cloud, Files, Folder, FolderGit2, GitBranch, GitBranchPlus, GitCommitHorizontal, GitMerge, History, Laptop, List, ListTree, Redo2, Search, SunMoon, Tag, Tags, Terminal, Undo2, User, X } from "lucide-solid";
import { Dynamic } from "solid-js/web";
import { Splitter, storedSize } from "./features/split/Splitter";

interface Tab {
  repo: OpenedRepo; selected: number | null; sha: string | null; diffPath: string | null;
  status: StatusReport | null; wip: boolean; wipFile: WipFile | null;
  /** Bumped when history changes so the graph reloads; `diffRev` when a diff must be re-read. */
  graphRev: number; diffRev: number;
}
interface Toast { text: string; repoId: number; undoId: string | null }

export function App() {
  const [leftW, setLeftW, saveLeftW] = storedSize("left", 232);
  const [rightW, setRightW, saveRightW] = storedSize("right", 360);
  const [diffH, setDiffH, saveDiffH] = storedSize("diff", 320);
  const [tabs, setTabs] = createSignal<Tab[]>([]);
  const [active, setActive] = createSignal(0);
  const [error, setError] = createSignal<UiError | null>(null);
  const [fileView, setFileView] = createSignal<"path" | "tree">("path");
  const [toast, setToast] = createSignal<Toast | null>(null);
  const [refFilter, setRefFilter] = createSignal("");
  const [refMenu, setRefMenu] = createSignal<RefMenuState | null>(null);
  const [rebaseOnto, setRebaseOnto] = createSignal<string | null>(null);
  const [commitMenu, setCommitMenu] = createSignal<CommitMenuState | null>(null);
  const [creds, setCreds] = createSignal<CredentialsRequest[]>([]);
  const [netBusy, setNetBusy] = createSignal<string | null>(null);
  const [forceConfirm, setForceConfirm] = createSignal(false);
  async function net(action: "fetch" | "pull" | "push", force = false) {
    setNetBusy(action);
    const toast = { fetch: t("toast.fetched"), pull: t("toast.pulled"), push: force ? t("toast.forcePushed") : t("toast.pushed") }[action];
    await run(() => api.net(tab().repo.repoId, action, force), { graph: true, toast });
    setNetBusy(null);
  }
  const answerCreds = (id: number, value: string | null) => {
    setCreds(creds().filter((c) => c.id !== id));
    void api.credentialsAnswer(id, value);
  };
  const [showJournal, setShowJournal] = createSignal(false);
  const [watchFailed, setWatchFailed] = createSignal<number[]>([]);
  const [askName, setAskName] = createSignal<"branch" | "stash" | null>(null);
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
      const still = (f: WipFile | null) => {
        if (!f) return null;
        const list = f.staged ? g.staged : g.unstaged;
        return list.some((i) => i.path === f.path) ? f : null;
      };
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
      const out = (await op()) as { id?: string | null; conflicted?: boolean } | undefined;
      if (out?.conflicted) opts = { graph: true, toast: t("toast.opConflicts") };
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
      if (st?.entries.length) patchRepo(repo.repoId, () => ({ wip: true }));
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
        if (e.shiftKey) void redo(tab().repo.repoId);
        else void undo(tab().repo.repoId, null);
      } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        togglePerf();
      }
    });
  });

  onMount(async () => {
    try {
      // The core watches the working tree and emits this (debounced) on any change.
      await listen<CredentialsRequest>("credentials-request", (e) => setCreds([...creds(), e.payload]));
      await listen<number>("credentials-cancel", (e) => setCreds(creds().filter((c) => c.id !== e.payload)));
      await listen<number>("repo-changed", (e) => loadStatus(e.payload));
      await listen<number>("watch-failed", (e) => setWatchFailed([...watchFailed().filter((id) => id !== e.payload), e.payload]));
      window.addEventListener("focus", () => tab() && loadStatus(tab().repo.repoId));
    } catch { /* no event bridge (plain browser) */ }
    const p = await api.initialPath();
    if (p) openPath(p);
  });

  const wipLabel = () => {
    if (!tab()?.status) return "";
    return wipCount() ? t("wip.changes", { n: wipCount() }) : t("wip.clean");
  };
  const wipCount = () => (tab()?.status ? changeCount(groupStatus(tab()!.status!.entries)) : 0);

  const [details] = createResource(
    () => (tab()?.sha ? ([tab()!.repo.repoId, tab()!.sha!] as const) : null),
    ([id, sha]) => api.details(id, sha),
  );

  return (
    <div class="app">
      <header class="toolbar">
        <div class="crumb">
          <Show when={tabs().length} fallback={<strong class="repo">{t("app.name")}</strong>}>
            <nav class="tabs" role="tablist" aria-label={t("tabs.label")}>
              <For each={tabs()}>
                {(x, i) => (
                  <div class="tab" role="tab" aria-selected={i() === active()} classList={{ active: i() === active() }}>
                    <button class="tab-name repo" onClick={() => setActive(i())}>{x.repo.name}</button>
                    <button class="icon" aria-label={t("tabs.close", { s: x.repo.name })} onClick={() => closeTab(i())}><X size={14} /></button>
                  </div>
                )}
              </For>
              <button class="icon tab-add" title={t("open.button")} aria-label={t("open.button")} onClick={pick}><Plus size={16} /></button>
            </nav>
          </Show>
          <Show when={tab()?.repo.branch}>
            <span class="branch" title={tab()!.repo.branch!}><GitBranch size={14} aria-hidden="true" />{tab()!.repo.branch}</span>
          </Show>
          <Show when={tab()?.repo.readOnlyReason}><span class="badge">{t("toolbar.readOnly")}</span></Show>
        </div>
        <div class="group">
          <button class="tool" disabled={!tab()} title={t("toolbar.undoTitle")} onClick={() => undo(tab().repo.repoId, null)}><Undo2 size={16} /><span class="lbl">{t("toolbar.undo")}</span></button>
          <button class="tool" disabled={!tab()} title={t("toolbar.redoTitle")} onClick={() => redo(tab().repo.repoId)}><Redo2 size={16} /><span class="lbl">{t("toolbar.redo")}</span></button>
        </div>
        <div class="group">
          <button class="tool" disabled={!tab() || !!netBusy() || !!tab()?.repo.readOnlyReason} aria-busy={netBusy() === "fetch"} title={t("toolbar.fetchTitle")} onClick={() => void net("fetch")}><CloudDownload size={16} /><span class="lbl">{t("toolbar.fetch")}</span></button>
          <button class="tool" disabled={!tab() || !!netBusy() || !!tab()?.repo.readOnlyReason} aria-busy={netBusy() === "pull"} title={t("toolbar.pullTitle")} onClick={() => void net("pull")}>
            <ArrowDownToLine size={16} /><span class="lbl">{t("toolbar.pull")}</span><Show when={tab()?.status?.behind}><span class="count">{tab()!.status!.behind}</span></Show>
          </button>
          <button class="tool" disabled={!tab() || !!netBusy() || !!tab()?.repo.readOnlyReason} aria-busy={netBusy() === "push"} title={t("toolbar.pushTitle")} onClick={() => void net("push")}>
            <ArrowUpFromLine size={16} /><span class="lbl">{t("toolbar.push")}</span><Show when={tab()?.status?.ahead}><span class="count">{tab()!.status!.ahead}</span></Show>
          </button>
          <button class="tool" disabled={!tab() || !!tab()?.repo.readOnlyReason} title={t("toolbar.newBranchTitle")} onClick={() => setAskName("branch")}><GitBranchPlus size={16} /><span class="lbl">{t("toolbar.newBranch")}</span></button>
          <button class="tool" disabled={!tab() || !!tab()?.repo.readOnlyReason} title={t("toolbar.stashTitle")}
            onClick={() => setAskName("stash")}><Archive size={16} /><span class="lbl">{t("toolbar.stash")}</span></button>
        </div>
        <div class="group">
          <button class="tool" disabled={!tab()} title={t("toolbar.journalTitle")} onClick={() => setShowJournal(true)}><History size={16} /><span class="lbl">{t("toolbar.journal")}</span></button>
          <label class="theme-pick" title={t("toolbar.theme")}>
            <SunMoon size={16} aria-hidden="true" /><span class="sr-only">{t("toolbar.theme")}</span>
            <select value={themeChoice()} onChange={(e) => setTheme(e.currentTarget.value as ThemeChoice)}>
              <option value="system">{t("theme.system")}</option>
              <option value="atlas-dark">{t("theme.dark")}</option>
              <option value="atlas-light">{t("theme.light")}</option>
            </select>
          </label>
        </div>
      </header>

      <Show
        when={tab()}
        fallback={
          <main class="empty">
            <FolderGit2 size={48} stroke-width={1.25} class="muted" aria-hidden="true" />
            <h1>{t("open.title")}</h1>
            <p class="muted">{t("open.hint")}</p>
            <Show when={error()}>
              <div class="error" role="alert"><strong>{t("error.title")}</strong><p>{error()!.message}</p></div>
            </Show>
            <button class="primary" onClick={pick}>{t("open.button")}</button>
          </main>
        }
      >
        <div class="body" style={{ "--left-w": `${leftW()}px`, "--right-w": `${rightW()}px` }}>
          <aside class="left" aria-label={t("panel.refs")}>
            <div class="search"><Search size={14} aria-hidden="true" /><input class="ref-filter" type="search" placeholder={t("panel.filter")} aria-label={t("panel.filter")}
              value={refFilter()} onInput={(e) => setRefFilter(e.currentTarget.value)} /></div>
            <For each={[["head", "panel.local"], ["remote", "panel.remote"], ["tag", "panel.tags"]] as const}>
              {([kind, label]) => {
                const items = () => filterRefs(tab()!.repo.refs, kind, refFilter());
                return (
                  <Show when={items().length}>
                    <details open={kind !== "remote" || !!refFilter()}>
                    <summary><Dynamic component={{ head: Laptop, remote: Cloud, tag: Tags }[kind]} size={14} aria-hidden="true" /><h2>{t(label)} <span class="muted">{items().length}</span></h2></summary>
                    <ul>
                      <For each={items()}>
                        {(r) => (
                          <li>
                            <button
                              classList={{ head: r.isHead }}
                              aria-disabled={r.row === null}
                              title={r.row === null ? t("panel.refHidden") : undefined}
                              onClick={() => r.row !== null && patch({ selected: r.row, sha: null })}
                              onDblClick={() => r.kind === "head" && !r.isHead && void run(() => api.checkout(tab().repo.repoId, r.name, true), { graph: true, toast: t("toast.checkedOut", { s: r.name }) })}
                              onContextMenu={(e) => { e.preventDefault(); setRefMenu({ x: e.clientX, y: e.clientY, ref: r }); }}
                              draggable={r.kind !== "tag" && !r.isHead}
                              onDragStart={(e) => e.dataTransfer?.setData("text/x-atlas-ref", JSON.stringify(r))}
                              onDragOver={(e) => r.isHead && e.dataTransfer?.types.includes("text/x-atlas-ref") && e.preventDefault()}
                              onDrop={(e) => {
                                // Dropping a ref onto the checked-out branch offers to merge it in (with the conflict preview).
                                const src = e.dataTransfer?.getData("text/x-atlas-ref");
                                if (!r.isHead || !src) return;
                                e.preventDefault();
                                setRefMenu({ x: e.clientX, y: e.clientY, ref: JSON.parse(src), merge: true });
                              }}
                            >
                              <Dynamic component={r.kind === "tag" ? Tag : GitBranch} size={14} class="ref-icon" aria-hidden="true" />
                              <span class="path">{r.name}</span>
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
            <StashPanel repoId={tab()!.repo.repoId} rev={tab()!.diffRev} run={run} />
          </aside>
          <Splitter label={t("split.left")} axis="x" value={leftW()} min={140} max={420} onChange={setLeftW} onCommit={saveLeftW} />

          <main class="center" style={{ "--diff-h": `${diffH()}px` }}>
            <button
              class="wip-row"
              classList={{ active: tab()!.wip }}
              onClick={() => patch({ wip: true, selected: null, sha: null, diffPath: null })}
            >
              <strong>{t("wip.row")}</strong>
              <span class="muted">
                {wipLabel()}
              </span>
            </button>
            <Show when={tab()!.graphRev} keyed>
              {(_) => (
                <GraphView
                  repo={tab()!.repo}
                  selected={tab()!.selected}
                  onSelect={(row, sha) => patch({ selected: row, sha, diffPath: null, wip: false })}
                  onMenu={tab()!.repo.readOnlyReason ? undefined : (sha, x, y) => setCommitMenu({ sha, x, y })}
                />
              )}
            </Show>
            <Show when={(tab()!.wip && tab()!.wipFile) || (!tab()!.wip && tab()!.diffPath && tab()!.sha)}>
              <Splitter label={t("split.diff")} axis="y" dir={-1} value={diffH()} min={120} max={Math.max(160, innerHeight - 260)} onChange={setDiffH} onCommit={saveDiffH} />
            </Show>
            <Show when={tab()!.wip && tab()!.wipFile}>
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
            <Show when={!tab()!.wip && tab()!.diffPath && tab()!.sha}>
              <DiffView
                repoId={tab()!.repo.repoId}
                source={{ kind: "commit", sha: tab()!.sha! }}
                path={tab()!.diffPath!}
                onClose={() => patch({ diffPath: null })}
              />
            </Show>
          </main>

          <Splitter label={t("split.right")} axis="x" dir={-1} value={rightW()} min={240} max={600} onChange={setRightW} onCommit={saveRightW} />
          <aside class="right" aria-label={t("details.label")}>
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
                  <p class="sha mono"><GitCommitHorizontal size={14} aria-hidden="true" />{d().sha.slice(0, 10)}</p>
                  <h2 class="message">{d().message.split("\n")[0]}</h2>
                  <Show when={d().message.includes("\n")}><pre class="body-text">{d().message.split("\n").slice(1).join("\n").trim()}</pre></Show>
                  <p class="meta"><User size={14} aria-hidden="true" />{d().author}</p>
                  <p class="meta"><Clock size={14} aria-hidden="true" /><time datetime={new Date(d().time * 1000).toISOString()}>{new Date(d().time * 1000).toLocaleString()}</time></p>
                  <p class="meta"><GitMerge size={14} aria-hidden="true" />{t("details.parents")}: <span class="mono">{d().parents.map((p) => p.slice(0, 7)).join(", ") || "—"}</span></p>
                  <div class="files-head">
                    <h3><Files size={14} aria-hidden="true" />{t("details.files")} <span class="muted">{d().files.length}</span></h3>
                    <div class="seg" role="group" aria-label={t("files.view")}>
                      <button aria-pressed={fileView() === "path"} onClick={() => setFileView("path")}><List size={14} aria-hidden="true" />{t("files.path")}</button>
                      <button aria-pressed={fileView() === "tree"} onClick={() => setFileView("tree")}><ListTree size={14} aria-hidden="true" />{t("files.tree")}</button>
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
                            <li class="dir" style={{ "padding-left": `${8 + r.depth * 14}px` }}><Folder size={14} aria-hidden="true" /> {r.name}</li>
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

      <Show when={tab() && watchFailed().includes(tab()!.repo.repoId)}>
        <output class="banner">
          <div>{t("watch.failed")}</div>
          <button class="mini" onClick={() => setWatchFailed(watchFailed().filter((id) => id !== tab()!.repo.repoId))}>{t("err.dismiss")}</button>
        </output>
      </Show>
      <Show when={error() && tab()}>
        <div class="banner error" role="alert">
          <div>
            <strong>{errorText(error()!)}</strong>
            <Show when={errorFix(error()!)}><p>{errorFix(error()!)}</p></Show>
            <Show when={showDetail() && error()!.detail}><pre class="mono">{error()!.detail}</pre></Show>
          </div>
          <Show when={error()!.detail}><button class="mini" onClick={() => setShowDetail(!showDetail())}>{t("err.details")}</button></Show>
          <Show when={error()!.code === "PUSH_REJECTED"}>
            {/* Overwrites remote commits: needs a second click. */}
            <button class="mini danger" onClick={() => { if (!forceConfirm()) return setForceConfirm(true); setForceConfirm(false); setError(null); void net("push", true); }}>
              {forceConfirm() ? t("push.forceConfirm") : t("push.force")}
            </button>
          </Show>
          <button class="mini" onClick={() => setError(null)}>{t("err.dismiss")}</button>
        </div>
      </Show>
      <Show when={tab() && rebaseOnto()} keyed>
        {(onto) => <RebaseEditor repoId={tab()!.repo.repoId} onto={onto} label={/^[0-9a-f]{40}$/.test(onto) ? onto.slice(0, 7) : onto}
          run={run} onClose={() => setRebaseOnto(null)} onError={(e) => fail(e)} />}
      </Show>
      <Show when={tab() && commitMenu()} keyed>
        {(m) => <CommitMenu repoId={tab()!.repo.repoId} menu={m} run={run} onClose={() => setCommitMenu(null)}
          onToast={(text) => setToast({ text, repoId: tab()!.repo.repoId, undoId: null })} onRebase={setRebaseOnto} />}
      </Show>
      <Show when={creds()[0]} keyed>
        {(c) => <NameDialog label={c.prompt || t("creds.title")} secret={c.secret} onSubmit={(v) => answerCreds(c.id, v)} onCancel={() => answerCreds(c.id, null)} />}
      </Show>
      {/* Keyed: the menu's actions still read their ref after closing it (refMenu() is null by then). */}
      <Show when={tab() && refMenu()} keyed>
        {(m) => <RefMenu repoId={tab()!.repo.repoId} menu={m} run={run} onClose={() => setRefMenu(null)} onError={(e) => fail(e)} onRebase={setRebaseOnto} />}
      </Show>
      <Show when={tab() && askName()} keyed>
        {(kind) => {
          const repoId = tab()!.repo.repoId;
          const done = (v: string) => {
            setAskName(null);
            if (kind === "stash") void run(() => api.stash(repoId, "push", 0, v || null), { graph: true, toast: t("toast.stashed") });
            else void run(() => api.branchCreate(repoId, v, null), { graph: true, toast: t("toast.branchCreated", { s: v }) });
          };
          return <NameDialog label={kind === "stash" ? t("stash.messagePrompt") : t("refmenu.namePrompt")} optional={kind === "stash"} onSubmit={done} onCancel={() => setAskName(null)} />;
        }}
      </Show>
      <Show when={showJournal() && tab()}>
        <JournalPanel repoId={tab()!.repo.repoId} onClose={() => setShowJournal(false)} onError={(e) => fail(e)} />
      </Show>
      <Show when={toast()}>
        <output class="toast">
          <span>{toast()!.text}</span>
          <Show when={toast()!.undoId}>
            <button onClick={() => { const x = toast()!; setToast(null); void undo(x.repoId, x.undoId); }}>{t("toast.undo")}</button>
          </Show>
        </output>
      </Show>

      <footer class="statusbar">
        <Show when={tab()}>
          <span><GitCommitHorizontal size={14} aria-hidden="true" />{t("status.commits", { n: tab()!.repo.rowCount.toLocaleString() })}</span>
          <span><Terminal size={14} aria-hidden="true" />{t("status.git", { v: tab()!.repo.gitVersion })}</span>
          <Show when={tab()!.repo.readOnlyReason}><span class="warn">{tab()!.repo.readOnlyReason}</span></Show>
        </Show>
      </footer>
    </div>
  );
}
