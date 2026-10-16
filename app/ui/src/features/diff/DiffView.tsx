// SPDX-License-Identifier: Apache-2.0
// File diff: unified (hunk) or split view (roadmap T1.9).
import { Maximize2, Minimize2, X } from "lucide-solid";
import { createEffect, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { parseUnified, toSplit } from "./parse";

/** A committed file, or a working-directory file (unstaged/staged) that supports hunk staging. */
export type DiffSource =
  | { kind: "commit"; sha: string }
  | { kind: "wip"; staged: boolean; untracked: boolean; rev: number; onHunk: (index: number) => void };

export function DiffView(props: { repoId: number; source: DiffSource; path: string; onClose: () => void }) {
  const [mode, setMode] = createSignal<"unified" | "split">("unified");
  const [full, setFull] = createSignal(false);
  // Escape leaves full screen (and only that, so it does not also close dialogs underneath).
  createEffect(() => {
    if (!full()) return;
    const key = (e: KeyboardEvent) => e.key === "Escape" && setFull(false);
    document.addEventListener("keydown", key);
    onCleanup(() => document.removeEventListener("keydown", key));
  });
  const [patch] = createResource(
    () => [props.repoId, props.source, props.path, props.source.kind === "wip" ? props.source.rev : 0] as const,
    ([r, src, p]) => (src.kind === "commit" ? api.diff(r, src.sha, p) : api.worktreeDiff(r, p, src.staged, src.untracked)),
  );
  const wip = () => (props.source.kind === "wip" ? props.source : null);
  const parsed = () => parseUnified(patch() ?? "");
  const no = (n: number | null) => (n === null ? "" : String(n));

  return (
    <section class="diff" classList={{ full: full() }} aria-label={t("diff.label", { path: props.path })}>
      <header class="diff-bar">
        <strong class="diff-path">{props.path}</strong>
        <div class="seg" role="group" aria-label={t("diff.mode")}>
          <button aria-pressed={mode() === "unified"} onClick={() => setMode("unified")}>{t("diff.unified")}</button>
          <button aria-pressed={mode() === "split"} onClick={() => setMode("split")}>{t("diff.split")}</button>
        </div>
        <button class="icon" onClick={() => setFull(!full())} aria-pressed={full()} aria-label={full() ? t("diff.exitFull") : t("diff.full")} title={full() ? t("diff.exitFull") : t("diff.full")}>
          {full() ? <Minimize2 size={16} aria-hidden="true" /> : <Maximize2 size={16} aria-hidden="true" />}
        </button>
        <button class="icon" onClick={props.onClose} aria-label={t("diff.close")}><X size={16} aria-hidden="true" /></button>
      </header>
      <div class="diff-body">
        <Show when={!patch.loading} fallback={<p class="muted">{t("loading")}</p>}>
          <Show when={!parsed().binary} fallback={<p class="muted">{t("diff.binary")}</p>}>
            <Show when={wip()?.untracked}><p class="muted pad">{t("diff.untracked")}</p></Show>
            <For each={parsed().hunks}>
              {(h, hi) => (
                <div class="hunk">
                  <div class="hunk-header">
                    <span>{h.header}</span>
                    <Show when={wip() && !wip()!.untracked}>
                      <button class="hunk-btn" onClick={() => wip()!.onHunk(hi())}>
                        {wip()!.staged ? t("diff.unstageHunk") : t("diff.stageHunk")}
                      </button>
                    </Show>
                  </div>
                  <Show
                    when={mode() === "split"}
                    fallback={
                      <table class="code">
                        <tbody>
                          <For each={h.lines}>
                            {(l) => (
                              <tr class={l.kind}>
                                <td class="ln">{no(l.oldNo)}</td>
                                <td class="ln">{no(l.newNo)}</td>
                                <td class="mark">{l.kind === "add" ? "+" : l.kind === "del" ? "−" : ""}</td>
                                <td class="txt">{l.text}</td>
                              </tr>
                            )}
                          </For>
                        </tbody>
                      </table>
                    }
                  >
                    <table class="code split">
                      <tbody>
                        <For each={toSplit(h)}>
                          {(r) => (
                            <tr>
                              <td class="ln">{no(r.left?.oldNo ?? null)}</td>
                              <td class={`txt ${r.left?.kind === "del" ? "del" : ""}`}>{r.left?.text ?? ""}</td>
                              <td class="ln">{no(r.right?.newNo ?? null)}</td>
                              <td class={`txt ${r.right?.kind === "add" ? "add" : ""}`}>{r.right?.text ?? ""}</td>
                            </tr>
                          )}
                        </For>
                      </tbody>
                    </table>
                  </Show>
                </div>
              )}
            </For>
          </Show>
        </Show>
      </div>
    </section>
  );
}
