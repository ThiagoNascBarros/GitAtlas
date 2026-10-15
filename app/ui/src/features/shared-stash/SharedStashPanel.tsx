// SPDX-License-Identifier: Apache-2.0
// Sidebar tab "Shared stash": stashes published to the remotes, grouped by remote, then Mine / Team.
import { AlertTriangle, ArchiveRestore, Cloud, GitBranch, Layers, Loader2, RefreshCw, ShieldCheck, ShieldQuestion, User } from "lucide-solid";
import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { KeptSharedStash, SharedStashInfo } from "../../api";
import { errorText, t } from "../../i18n";
import { groupStashes, relativeTime } from "./logic";
import { cancelFetch, fetchShared, glyphOn, modelOf, setFetchMode, setGlyph } from "./model";

export interface SharedStashPanelProps {
  repoId: number;
  readOnly: boolean;
  /** Stash commit of the row whose diff is open. */
  openSha: string | null;
  onOpen: (s: SharedStashInfo) => void;
  onMenu: (s: SharedStashInfo, x: number, y: number) => void;
  onReshare: (k: KeptSharedStash) => void;
  onDiscard: (k: KeptSharedStash) => void;
  onRetryCheck: (remote: string) => void;
}

export function SharedStashPanel(props: SharedStashPanelProps) {
  const m = () => modelOf(props.repoId);
  const [now, setNow] = createSignal(Math.floor(Date.now() / 1000));
  const clock = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 30000);
  onCleanup(() => window.clearInterval(clock));
  // Opening the tab fetches (panel and always modes alike, SPEC §5.1).
  onMount(() => { if (!props.readOnly) void fetchShared(props.repoId); });

  const groups = createMemo(() => {
    const l = m().listing;
    return l ? groupStashes(l.stashes, l.remotes, l.kept, l.defaultRemote) : [];
  });
  // Keyed by remote name so a reload does not rebuild the sections (focus, open state).
  const names = createMemo(() => groups().map((g) => g.remote));
  const mode = () => m().listing?.fetchMode ?? "panel";
  const [confirmDiscard, setConfirmDiscard] = createSignal<string | null>(null);

  /** Arrow keys, Home and End move between rows; Menu / Shift+F10 opens the row's actions. */
  function onKeys(e: KeyboardEvent) {
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("[data-ss-row]")];
    const i = rows.indexOf(document.activeElement as HTMLButtonElement);
    const next = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: rows.length - 1 }[e.key];
    if (next !== undefined && rows.length) {
      e.preventDefault();
      rows[Math.max(0, Math.min(rows.length - 1, next))].focus();
    }
  }

  const Row = (p: { s: SharedStashInfo }) => {
    const s = p.s;
    const open = (e: MouseEvent | KeyboardEvent, el: HTMLElement) => {
      e.preventDefault();
      if (!s.valid || props.readOnly) return;
      const b = el.getBoundingClientRect();
      props.onMenu(s, e instanceof MouseEvent ? e.clientX : b.left + 24, e instanceof MouseEvent ? e.clientY : b.bottom);
    };
    return (
      <li>
        <button
          class="ss-row" data-ss-row classList={{ invalid: !s.valid, active: props.openSha === s.stash }}
          aria-disabled={!s.valid} aria-haspopup={s.valid ? "menu" : undefined}
          title={s.valid ? s.title : t("ss.invalidTitle", { s: s.invalid_reason ?? "" })}
          onClick={() => s.valid && props.onOpen(s)}
          onContextMenu={(e) => open(e, e.currentTarget)}
          onKeyDown={(e) => { if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) open(e, e.currentTarget); }}
        >
          <span class="ss-title">{s.title || s.id}</span>
          <span class="ss-meta">
            <span class="ss-author"><User size={12} aria-hidden="true" />{s.author_name}</span>
            <Show when={s.valid} fallback={<span class="ss-badge bad">{t("ss.invalid")}</span>}>
              <Show when={s.verified} fallback={<span class="ss-badge claimed" title={t("ss.claimedTitle")}><ShieldQuestion size={11} aria-hidden="true" />{t("ss.claimed")}</span>}>
                <span class="ss-badge verified" title={t("ss.verifiedTitle")}><ShieldCheck size={11} aria-hidden="true" />{t("ss.verified")}</span>
              </Show>
            </Show>
            <Show when={s.valid}>
              <span class="ss-base" title={s.base}><GitBranch size={12} aria-hidden="true" />{s.base_branch || t("ss.noBranch")}</span>
            </Show>
            <time datetime={new Date(s.updated * 1000).toISOString()}>{relativeTime(now(), s.updated)}</time>
          </span>
        </button>
      </li>
    );
  };

  const Sub = (p: { label: string; list: SharedStashInfo[] }) => (
    <Show when={p.list.length}>
      <h3 class="ss-sub">{p.label} <span class="muted">{p.list.length}</span></h3>
      <ul><For each={p.list}>{(s) => <Row s={s} />}</For></ul>
    </Show>
  );

  const Group = (p: { name: string }) => {
    const g = createMemo(() => groups().find((x) => x.remote === p.name) ?? { remote: p.name, probe: null, mine: [], team: [], kept: [] });
    const err = () => m().fetch.errors[g().remote];
    const total = () => g().mine.length + g().team.length;
    return (
      <details open class="ss-remote">
        <summary><Cloud size={14} aria-hidden="true" /><h2>{g().remote} <span class="muted">{total()}</span></h2></summary>
        <Show when={g().probe === "rejected"}>
          <div class="ss-note warn" role="status">
            <AlertTriangle size={14} aria-hidden="true" />
            <div><span>{t("ss.rejected", { s: g().remote })}</span>
              <button class="mini" disabled={props.readOnly} onClick={() => props.onRetryCheck(g().remote)}>{t("ss.retryCheck")}</button></div>
          </div>
        </Show>
        <Show when={err()}>
          <div class="ss-note error" role="alert">
            <AlertTriangle size={14} aria-hidden="true" />
            <div><span>{t("ss.remoteError", { s: g().remote, m: errorText(err()!) })}</span>
              <button class="mini" disabled={m().fetch.loading} onClick={() => void fetchShared(props.repoId, g().remote)}>{t("ss.retry")}</button></div>
          </div>
        </Show>
        <Show when={g().probe !== "rejected"}>
          <Show when={total()} fallback={<Show when={!m().fetch.loading && !err()}><p class="ss-empty muted">{t("ss.empty", { s: g().remote })}</p></Show>}>
            <Sub label={t("ss.mine")} list={g().mine} />
            <Sub label={t("ss.team")} list={g().team} />
          </Show>
        </Show>
        <Show when={g().kept.length}>
          <h3 class="ss-sub">{t("ss.kept")} <span class="muted">{g().kept.length}</span></h3>
          <p class="ss-hint muted">{t("ss.keptHint")}</p>
          <ul>
            <For each={g().kept}>
              {(k) => (
                <li class="ss-kept">
                  <span class="path" title={k.title}><ArchiveRestore size={12} aria-hidden="true" /> {k.title || k.id}</span>
                  <button class="mini" disabled={props.readOnly} onClick={() => props.onReshare(k)}>{t("ss.reshare")}</button>
                  <button class="mini" classList={{ danger: confirmDiscard() === k.id }} disabled={props.readOnly}
                    onBlur={() => setConfirmDiscard(null)}
                    onClick={() => { if (confirmDiscard() !== k.id) return setConfirmDiscard(k.id); setConfirmDiscard(null); props.onDiscard(k); }}>
                    {confirmDiscard() === k.id ? t("ss.discardConfirm") : t("ss.discard")}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </details>
    );
  };

  return (
    <section class="ss" aria-label={t("ss.title")}>
      <header class="ss-head">
        <div class="ss-bar">
          <strong>{t("ss.title")}</strong>
          <button class="icon" aria-pressed={glyphOn()} title={t("ss.glyphToggle")} aria-label={t("ss.glyphToggle")} onClick={() => setGlyph(!glyphOn())}><Layers size={15} /></button>
          <button class="icon" disabled={m().fetch.loading || props.readOnly} title={t("ss.refresh")} aria-label={t("ss.refresh")} onClick={() => void fetchShared(props.repoId)}>
            <RefreshCw size={15} classList={{ spin: m().fetch.loading }} />
          </button>
        </div>
        <div class="seg ss-mode" role="group" aria-label={t("ss.fetchMode")}>
          <button aria-pressed={mode() === "panel"} title={t("ss.modePanelHint")} onClick={() => void setFetchMode(props.repoId, "panel")}>{t("ss.modePanel")}</button>
          <button aria-pressed={mode() === "always"} title={t("ss.modeAlwaysHint")} onClick={() => void setFetchMode(props.repoId, "always")}>{t("ss.modeAlways")}</button>
        </div>
        <p class="ss-last muted">
          {m().fetch.lastFetched ? t("ss.lastFetched", { s: relativeTime(now(), m().fetch.lastFetched!) }) : t("ss.neverFetched")}
        </p>
      </header>
      <Show when={m().fetch.loading}>
        <div class="ss-loading" role="status"><Loader2 size={14} class="spin" aria-hidden="true" />{t("ss.loading")}
          <button class="mini" onClick={() => cancelFetch(props.repoId)}>{t("ss.cancel")}</button></div>
      </Show>
      <Show when={m().fetch.failure ?? m().listError}>
        {(e) => <div class="ss-note error" role="alert"><AlertTriangle size={14} aria-hidden="true" /><div>{t("ss.listError", { m: errorText(e()) })}
          <button class="mini" onClick={() => void fetchShared(props.repoId)}>{t("ss.retry")}</button></div></div>}
      </Show>
      <div class="ss-list" role="group" aria-label={t("ss.listLabel")} onKeyDown={onKeys}>
        <Show when={m().listing && !groups().length}><p class="ss-empty muted">{t("ss.emptyAll")}</p></Show>
        <For each={names()}>{(name) => <Group name={name} />}</For>
      </div>
    </section>
  );
}

