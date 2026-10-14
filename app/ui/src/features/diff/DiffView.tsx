// SPDX-License-Identifier: Apache-2.0
// File diff: unified (hunk) or split view (roadmap T1.9).
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { parseUnified, toSplit } from "./parse";

export function DiffView(props: { repoId: number; sha: string; path: string; onClose: () => void }) {
  const [mode, setMode] = createSignal<"unified" | "split">("unified");
  const [patch] = createResource(() => [props.repoId, props.sha, props.path] as const, ([r, s, p]) => api.diff(r, s, p));
  const parsed = () => parseUnified(patch() ?? "");
  const no = (n: number | null) => (n === null ? "" : String(n));

  return (
    <section class="diff" aria-label={t("diff.label", { path: props.path })}>
      <header class="diff-bar">
        <strong class="diff-path">{props.path}</strong>
        <div class="seg" role="group" aria-label={t("diff.mode")}>
          <button aria-pressed={mode() === "unified"} onClick={() => setMode("unified")}>{t("diff.unified")}</button>
          <button aria-pressed={mode() === "split"} onClick={() => setMode("split")}>{t("diff.split")}</button>
        </div>
        <button class="icon" onClick={props.onClose} aria-label={t("diff.close")}>✕</button>
      </header>
      <div class="diff-body">
        <Show when={!patch.loading} fallback={<p class="muted">{t("loading")}</p>}>
          <Show when={!parsed().binary} fallback={<p class="muted">{t("diff.binary")}</p>}>
            <For each={parsed().hunks}>
              {(h) => (
                <div class="hunk">
                  <div class="hunk-header">{h.header}</div>
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
