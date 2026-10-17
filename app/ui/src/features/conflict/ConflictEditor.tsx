// SPDX-License-Identifier: Apache-2.0
// Full-screen conflict editor. Each block shows our lines, the base (common ancestor) and their
// lines; every line has its own checkbox. The result below is derived from the checks as they
// change. Saving goes through the operation engine (journaled, undoable).
import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { useOverlay } from "../overlay";
import {
  applyChoice,
  baseLines,
  type Choice,
  conflictsOf,
  emptyPick,
  parseSegments,
  type Pick,
  render,
  toggleLine,
  unresolvedCount,
} from "./markers";

type Run = (op: () => Promise<unknown>, opts?: { graph?: boolean; toast?: string }) => Promise<boolean>;

export function ConflictEditor(props: { repoId: number; path: string; run: Run; onClose: () => void; onError: (e: unknown) => void }) {
  // Bumped to read the file again (after it was edited in an external tool).
  const [version, setVersion] = createSignal(0);
  const [data] = createResource(
    () => ({ path: props.path, version: version() }),
    (src) =>
      api
        .conflictVersions(props.repoId, src.path)
        .then((x) => ({ segments: parseSegments(x.merged), diff3: x.merged_diff3 ? parseSegments(x.merged_diff3) : null }))
        .catch((e) => {
          props.onError(e);
          props.onClose();
          return null;
        }),
  );
  const [picks, setPicks] = createSignal<Pick[]>([]);
  const [current, setCurrent] = createSignal(0);
  const conflicts = createMemo(() => conflictsOf(data()?.segments ?? []));
  const bases = createMemo(() => baseLines(conflicts(), data()?.diff3 ?? null));
  const output = createMemo(() => render(data()?.segments ?? [], picks()));
  const open = createMemo(() => unresolvedCount(picks()));
  // A new read of the file starts again from "nothing chosen".
  createEffect(() => setPicks(conflicts().map(emptyPick)));

  let page: HTMLDivElement | undefined;
  let heading: HTMLHeadingElement | undefined;
  useOverlay(() => props.onClose(), () => heading, () => page);

  const setPick = (i: number, p: Pick) => setPicks((all) => all.map((x, j) => (j === i ? p : x)));
  const choose = (i: number, choice: Choice) => setPick(i, applyChoice(conflicts()[i], choice));
  const check = (i: number, side: "ours" | "theirs", line: number, checked: boolean) =>
    setPick(i, toggleLine(conflicts()[i], picks()[i], side, line, checked));
  const go = (i: number) => {
    const n = conflicts().length;
    if (!n) return;
    const k = (i + n) % n;
    setCurrent(k);
    page?.querySelector<HTMLElement>(`#conflict-hunk-${k}`)?.scrollIntoView({ block: "center" });
  };
  const save = async () => {
    if (await props.run(() => api.resolveWith(props.repoId, props.path, output()), { toast: t("toast.resolved", { s: props.path }) })) props.onClose();
  };
  const openExternal = () => {
    api.openExternal(props.repoId, props.path).catch((e) => props.onError(e));
  };

  return (
    <div class="conflict-page" ref={page} role="dialog" aria-modal="true" aria-labelledby="conflict-h">
      <header class="conflict-bar">
        <h2 id="conflict-h" ref={heading} tabindex="-1">{t("conflict.title", { s: props.path })}</h2>
        <Show when={conflicts().length > 0}>
          <nav class="conflict-nav" aria-label={t("conflict.title", { s: props.path })}>
            <button onClick={() => go(current() - 1)} aria-label={t("conflict.prev")}>{t("conflict.prev")}</button>
            <span aria-live="polite">{t("conflict.position", { n: current() + 1, m: conflicts().length })}</span>
            <button onClick={() => go(current() + 1)} aria-label={t("conflict.next")}>{t("conflict.next")}</button>
          </nav>
        </Show>
        <span class="grow" />
        <button onClick={openExternal}>{t("conflict.openExternal")}</button>
        <button onClick={() => setVersion((v) => v + 1)}>{t("conflict.reload")}</button>
        <button onClick={props.onClose}>{t("conflict.cancel")}</button>
        {/* Saving with markers left would commit them: every block must be resolved. */}
        <button class="primary" disabled={!data() || open() > 0} onClick={() => void save()}>{t("conflict.save")}</button>
      </header>
      <Show when={data()} fallback={<p class="conflict-loading muted">{t("loading")}</p>}>
        <div class="conflict-body">
          <div class="conflict-hunks">
            <Show when={conflicts().length > 0} fallback={<p class="muted">{t("conflict.noConflicts")}</p>}>
              <p class="conflict-hint muted">{t("conflict.hint")}</p>
            </Show>
            <For each={conflicts()}>
              {(c, i) => (
                <article
                  id={`conflict-hunk-${i()}`}
                  class="conflict-hunk"
                  classList={{ current: current() === i(), resolved: !!picks()[i()]?.resolved }}
                  aria-labelledby={`conflict-hunk-${i()}-h`}
                  onFocusIn={() => setCurrent(i())}
                >
                  <div class="conflict-hunk-head">
                    <h3 id={`conflict-hunk-${i()}-h`}>{t("conflict.chunk", { n: i() + 1 })}</h3>
                    <span class="conflict-state">{picks()[i()]?.resolved ? t("conflict.resolved") : t("conflict.pending")}</span>
                    <div class="conflict-choices">
                      <button class="mini" onClick={() => choose(i(), "ours")}>{t("conflict.onlyOurs")}</button>
                      <button class="mini" onClick={() => choose(i(), "theirs")}>{t("conflict.onlyTheirs")}</button>
                      <button class="mini" onClick={() => choose(i(), "both")}>{t("conflict.bothOurs")}</button>
                      <button class="mini" onClick={() => choose(i(), "bothTheirsFirst")}>{t("conflict.bothTheirs")}</button>
                      <button class="mini" onClick={() => choose(i(), "none")}>{t("conflict.none")}</button>
                      <button class="mini" onClick={() => choose(i(), "clear")}>{t("conflict.clear")}</button>
                    </div>
                  </div>
                  <div class="conflict-cols">
                    <section class="conflict-col ours" aria-label={t("conflict.ours")}>
                      <h4>{t("conflict.ours")}</h4>
                      <Show when={c.ours.length} fallback={<p class="muted">{t("conflict.empty")}</p>}>
                        <For each={c.ours}>
                          {(line, k) => (
                            <label class="conflict-line">
                              <input type="checkbox" checked={!!picks()[i()]?.ours[k()]} onChange={(e) => check(i(), "ours", k(), e.currentTarget.checked)} />
                              <code>{line === "" ? " " : line}</code>
                            </label>
                          )}
                        </For>
                      </Show>
                    </section>
                    <section class="conflict-col base" aria-label={t("conflict.base")}>
                      <h4>{t("conflict.base")}</h4>
                      <Show when={bases()[i()]} fallback={<p class="muted">{t("conflict.baseMissing")}</p>}>
                        {(b) => (
                          <Show when={b().length} fallback={<p class="muted">{t("conflict.empty")}</p>}>
                            <pre class="mono">{b().join("\n")}</pre>
                          </Show>
                        )}
                      </Show>
                    </section>
                    <section class="conflict-col theirs" aria-label={t("conflict.theirs")}>
                      <h4>{t("conflict.theirs")}</h4>
                      <Show when={c.theirs.length} fallback={<p class="muted">{t("conflict.empty")}</p>}>
                        <For each={c.theirs}>
                          {(line, k) => (
                            <label class="conflict-line">
                              <input type="checkbox" checked={!!picks()[i()]?.theirs[k()]} onChange={(e) => check(i(), "theirs", k(), e.currentTarget.checked)} />
                              <code>{line === "" ? " " : line}</code>
                            </label>
                          )}
                        </For>
                      </Show>
                    </section>
                  </div>
                </article>
              )}
            </For>
          </div>
          <section class="conflict-output" aria-label={t("conflict.output")}>
            <div class="conflict-output-head">
              <h3>{t("conflict.output")}</h3>
              <Show when={conflicts().length > 0}>
                <span class="muted">{t("conflict.left", { n: open() })}</span>
              </Show>
            </div>
            <pre class="mono">{output()}</pre>
          </section>
        </div>
      </Show>
    </div>
  );
}
