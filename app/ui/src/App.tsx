// SPDX-License-Identifier: Apache-2.0
// App frame (roadmap T1.7): tabs, toolbar, refs panel, graph/diff, details, status bar.
import { createResource, createSignal, For, onMount, Show } from "solid-js";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, OpenedRepo, UiError } from "./api";
import { GraphView } from "./features/graph/GraphView";
import { DiffView } from "./features/diff/DiffView";
import { setTheme, themeChoice, ThemeChoice } from "./theme/theme";
import { t } from "./i18n";
import { toTreeRows } from "./features/files/tree";
import { markOpenStart, togglePerf } from "./features/graph/perf";

interface Tab { repo: OpenedRepo; selected: number | null; sha: string | null; diffPath: string | null }

export function App() {
  const [tabs, setTabs] = createSignal<Tab[]>([]);
  const [active, setActive] = createSignal(0);
  const [error, setError] = createSignal<UiError | null>(null);
  const [fileView, setFileView] = createSignal<"path" | "tree">("path");
  const tab = () => tabs()[active()];
  const patch = (p: Partial<Tab>) => setTabs(tabs().map((x, i) => (i === active() ? { ...x, ...p } : x)));

  async function openPath(path: string) {
    try {
      markOpenStart();
      const repo = await api.open(path);
      setError(null);
      setTabs([...tabs(), { repo, selected: null, sha: null, diffPath: null }]);
      setActive(tabs().length - 1);
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
      if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        togglePerf();
      }
    });
  });

  onMount(async () => {
    const p = await api.initialPath();
    if (p) openPath(p);
  });

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
        <span class="badge">{t("toolbar.readOnly")}</span>
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
            <For each={[["head", "panel.local"], ["remote", "panel.remote"], ["tag", "panel.tags"]] as const}>
              {([kind, label]) => {
                const items = () => tab()!.repo.refs.filter((r) => r.kind === kind);
                return (
                  <Show when={items().length}>
                    <h2>{t(label)} <span class="muted">{items().length}</span></h2>
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
                  </Show>
                );
              }}
            </For>
          </aside>

          <main class="center">
            <Show
              when={tab()!.diffPath && tab()!.sha}
              fallback={
                <GraphView
                  repo={tab()!.repo}
                  selected={tab()!.selected}
                  onSelect={(row, sha) => patch({ selected: row, sha, diffPath: null })}
                />
              }
            >
              <DiffView repoId={tab()!.repo.repoId} sha={tab()!.sha!} path={tab()!.diffPath!} onClose={() => patch({ diffPath: null })} />
            </Show>
          </main>

          <aside class="right" aria-label="Commit details">
            <Show when={details()} fallback={<p class="muted pad">{t("details.select")}</p>}>
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
            </Show>
          </aside>
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
