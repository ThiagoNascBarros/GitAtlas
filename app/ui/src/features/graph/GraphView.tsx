// SPDX-License-Identifier: Apache-2.0
// Canvas commit graph (roadmap T1.8): virtualized rows, lanes, refs,
// selection, keyboard navigation and a hidden DOM mirror of the visible rows for assistive
// technology (spec accessibility A-1: ARIA grid rows, aria-activedescendant).
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, FLAG_MERGE, GraphRow, OpenedRepo, RefInfo, RowMeta, SEG_IN, SEG_OUT, SEG_PASS, SharedStashInfo } from "../../api";
import { activeTheme, laneColor, token } from "../../theme/theme";
import { activeLocale, t } from "../../i18n";

const ROW_H = 28;
const LANE_W = 16;
const CHUNK = 200;
/** Lanes shown before the graph column stops growing (the rest is clipped). */
const MAX_VISIBLE_LANES = 12;
import { perfEnabled, perfStats, recordDraw } from "./perf";
import { columns, ColumnWidths, loadWidths, saveWidths, clampWidth, resolveWidths } from "./columns";
import { menuAnchor, rowId, rowText, visibleRange, wipText } from "./a11y";
import { isMenuKey } from "../overlay";

/** Rows mirrored in the DOM above and below the viewport (spec accessibility §4.2: ≤ 20). */
const OVERSCAN = 10;

interface Chunk { rows: GraphRow[]; meta: RowMeta[] }

// Lucide "pencil-line" (ISC), drawn on the canvas for the uncommitted-changes row.
const PENCIL = typeof Path2D === "undefined" ? null : new Path2D("M13 21h8 M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z M15 5l4 4");

// Survives remounts (graph reload after commit/undo/redo) so the view keeps its place.
const savedScroll = new Map<number, number>();

function refLabel(ref: RefInfo): string {
  if (ref.kind === "tag") return `◆ ${ref.name}`;
  if (ref.kind === "stash") return `⧉ ${ref.name}`;
  return ref.isHead ? `✓ ${ref.name}` : ref.name;
}

export function GraphView(props: {
  repo: OpenedRepo;
  selected: number | null;
  onSelect: (row: number, sha: string) => void;
  /** Right-click on a row (the row is selected first). */
  onMenu?: (sha: string, x: number, y: number) => void;
  /** Shared stashes by base commit; a glyph is drawn on those rows (view toggle off: undefined). */
  shared?: Map<string, SharedStashInfo[]>;
  /** Uncommitted changes, drawn as a virtual first row above HEAD (none: no row). */
  wip?: { staged: number; unstaged: number; conflicted: number; active: boolean };
  onWip?: () => void;
}) {
  let scroller!: HTMLDivElement;
  let canvas!: HTMLCanvasElement;
  // Cache keys include the repo id: the component is reused across tabs.
  const chunks = new Map<string, Chunk>();
  const pending = new Set<string>();
  const [loadError, setLoadError] = createSignal<string | null>(null);
  const key = (chunk: number) => `${props.repo.repoId}:${chunk}`;
  const [tick, setTick] = createSignal(0);
  const [size, setSize] = createSignal({ w: 800, h: 600 });
  const [scrollTop, setScrollTop] = createSignal(0);
  const [stored, setWidths] = createSignal<ColumnWidths>(loadWidths());
  // Effective widths: an unset refs width follows the available space.
  const widths = () => resolveWidths(stored(), size().w, graphW());

  /** Drag a header edge to resize a column (refs grows to the right; author/date grow to the left). */
  function startResize(e: PointerEvent, col: keyof ColumnWidths) {
    e.preventDefault();
    const x0 = e.clientX, w0 = widths()[col];
    const dir = col === "refs" ? 1 : -1;
    const move = (ev: PointerEvent) => setWidths({ ...stored(), refs: widths().refs, [col]: clampWidth(col, w0 + dir * (ev.clientX - x0)) });
    const up = () => {
      saveWidths(stored());
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  const refsByRow = createMemo(() => {
    const m = new Map<number, RefInfo[]>();
    for (const r of props.repo.refs) if (r.row !== null) m.set(r.row, [...(m.get(r.row) ?? []), r]);
    return m;
  });
  const graphW = () => Math.min(Math.max(props.repo.laneCount, 1), MAX_VISIBLE_LANES) * LANE_W + LANE_W;

  function ensure(chunk: number) {
    const k = key(chunk);
    if (chunks.has(k) || pending.has(k) || chunk * CHUNK >= props.repo.rowCount) return;
    pending.add(k);
    const start = chunk * CHUNK;
    const repoId = props.repo.repoId;
    Promise.all([api.window(repoId, start, CHUNK), api.rows(repoId, start, CHUNK)])
      .then(([w, meta]) => {
        chunks.set(k, { rows: w.rows, meta });
        setLoadError(null);
      })
      .catch((e) => {
        // Surface the failure instead of leaving an empty graph; a later redraw retries.
        console.error("graph chunk failed", k, e);
        setLoadError(String(e?.message ?? e));
      })
      .finally(() => {
        pending.delete(k);
        setTick(tick() + 1);
      });
  }

  function rowAt(i: number): { g: GraphRow; m: RowMeta } | undefined {
    const c = chunks.get(key(Math.floor(i / CHUNK)));
    if (!c) return;
    const k = i % CHUNK;
    return c.rows[k] && { g: c.rows[k], m: c.meta[k] };
  }

  function draw() {
    const drawStart = performance.now();
    let painted = 0;
    tick();
    activeTheme();
    // Tracked like the theme: a language switch repaints dates and canvas labels.
    const locale = activeLocale();
    const { w, h } = size();
    const dpr = window.devicePixelRatio || 1;
    // Assigning width/height reallocates the backing store: only do it on real size changes.
    const bw = Math.round(w * dpr), bh = Math.round(h * dpr);
    if (canvas.width !== bw) canvas.width = bw;
    if (canvas.height !== bh) canvas.height = bh;
    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = token("color.bg.canvas");
    ctx.fillRect(0, 0, w, h);

    const top = scrollTop() - off();
    const first = Math.max(0, Math.floor(top / ROW_H));
    const last = Math.min(props.repo.rowCount - 1, Math.ceil((top + h) / ROW_H));
    for (let c = Math.floor(first / CHUNK); c <= Math.floor(last / CHUNK); c++) ensure(c);

    const gx = widths().refs - 8;
    const laneX = (l: number) => gx + LANE_W / 2 + l * LANE_W + 4;
    const refs = refsByRow();
    ctx.font = "13px Inter, Cantarell, system-ui, sans-serif";
    ctx.textBaseline = "middle";

    for (let i = first; i <= last; i++) {
      const y = i * ROW_H - top;
      const cy = y + ROW_H / 2;
      const r = rowAt(i);
      if (i === props.selected) {
        ctx.fillStyle = token("color.bg.selected");
        ctx.fillRect(0, y, w, ROW_H);
      }
      if (!r) continue;
      painted++;
      // Edges and node, clipped to the graph column so wide graphs never cover the message.
      ctx.save();
      ctx.beginPath();
      ctx.rect(gx, y, graphW(), ROW_H);
      ctx.clip();
      ctx.lineWidth = 2;
      for (const s of r.g.segs) {
        ctx.strokeStyle = laneColor(s.color);
        ctx.beginPath();
        if (s.kind === SEG_PASS) {
          ctx.moveTo(laneX(s.from), y);
          ctx.lineTo(laneX(s.from), y + ROW_H);
        } else if (s.kind === SEG_IN) {
          ctx.moveTo(laneX(s.from), y);
          ctx.bezierCurveTo(laneX(s.from), cy, laneX(s.to), y, laneX(s.to), cy);
        } else if (s.kind === SEG_OUT) {
          ctx.moveTo(laneX(s.from), cy);
          ctx.bezierCurveTo(laneX(s.from), y + ROW_H, laneX(s.to), cy, laneX(s.to), y + ROW_H);
        }
        ctx.stroke();
      }
      // Node
      const nx = laneX(r.g.lane);
      ctx.beginPath();
      ctx.arc(nx, cy, r.g.flags & FLAG_MERGE ? 4 : 6, 0, Math.PI * 2);
      ctx.fillStyle = laneColor(r.g.color);
      ctx.fill();
      ctx.strokeStyle = token("color.bg.canvas");
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.restore();
      // More lanes than fit: fade the right edge as a hint that the graph continues.
      if (props.repo.laneCount > MAX_VISIBLE_LANES) {
        const fx = gx + graphW() - 12;
        const grad = ctx.createLinearGradient(fx, 0, fx + 12, 0);
        grad.addColorStop(0, "transparent");
        grad.addColorStop(1, i === props.selected ? token("color.bg.selected") : token("color.bg.canvas"));
        ctx.fillStyle = grad;
        ctx.fillRect(fx, y, 12, ROW_H);
      }
      // Ref labels
      let rx = 8;
      const sharedHere = props.shared?.get(r.m.sha);
      if (sharedHere && widths().refs > 48) {
        // Same shape as the stash ref labels, so it reads as a label and not as a button.
        const label = sharedHere.length > 1 ? t("ss.glyphMany", { n: String(sharedHere.length) }) : t("ss.glyphOne");
        const tw = Math.min(ctx.measureText(label).width + 26, widths().refs - rx - 8);
        ctx.fillStyle = token("color.accent.muted");
        ctx.fillRect(rx, y + 4, tw, ROW_H - 8);
        // Arrow leaving a tray: "shared out of this commit".
        const ax = rx + 11, ay = cy;
        ctx.strokeStyle = token("color.accent.default");
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(ax, ay + 2); ctx.lineTo(ax, ay - 5);
        ctx.moveTo(ax - 3, ay - 2); ctx.lineTo(ax, ay - 5); ctx.lineTo(ax + 3, ay - 2);
        ctx.moveTo(ax - 4.5, ay + 5); ctx.lineTo(ax + 4.5, ay + 5);
        ctx.stroke();
        ctx.fillStyle = token("color.text.primary");
        ctx.save();
        ctx.beginPath();
        ctx.rect(rx, y, tw - 4, ROW_H);
        ctx.clip();
        ctx.fillText(label, rx + 20, cy);
        ctx.restore();
        rx += tw + 4;
      }
      for (const ref of refs.get(i) ?? []) {
        const label = refLabel(ref);
        const tw = Math.min(ctx.measureText(label).width + 12, widths().refs - rx - 8);
        if (tw < 24) break;
        ctx.fillStyle = ref.isHead ? token("color.accent.default") : ref.kind === "stash" ? token("color.accent.muted") : token("color.bg.raised");
        ctx.fillRect(rx, y + 4, tw, ROW_H - 8);
        ctx.fillStyle = ref.isHead ? token("color.text.onAccent") : token("color.text.primary");
        ctx.save();
        ctx.beginPath();
        ctx.rect(rx, y, tw - 4, ROW_H);
        ctx.clip();
        ctx.fillText(label, rx + 6, cy);
        ctx.restore();
        rx += tw + 4;
      }
      // Text columns: message, author, date, sha
      const col = columns(w, graphW(), widths());
      ctx.fillStyle = token("color.text.primary");
      clipText(ctx, r.m.summary, col.msgX, cy, col.msgW);
      ctx.fillStyle = token("color.text.secondary");
      if (col.showAuthor) clipText(ctx, r.m.author, col.authorX, cy, widths().author - 12);
      if (col.showDate) clipText(ctx, new Date(r.m.time * 1000).toLocaleDateString(locale), col.dateX, cy, widths().date - 8);
      ctx.font = "12.5px 'JetBrains Mono', 'DejaVu Sans Mono', monospace";
      ctx.fillStyle = token("color.text.muted");
      ctx.fillText(r.m.sha.slice(0, 7), col.shaX, cy);
      ctx.font = "13px Inter, Cantarell, system-ui, sans-serif";
    }
    if (props.wip) drawWip(ctx, w, top, laneX);
    recordDraw(drawStart, painted);
  }

  createEffect(draw);

  /** Height of the uncommitted-changes row (0 when there is none). */
  const off = () => (props.wip ? ROW_H : 0);
  const headRow = () => props.repo.refs.find((x) => x.isHead)?.row ?? null;

  /** Hollow node in HEAD's lane and colour, dotted line down to HEAD, label and counts. */
  function drawWip(ctx: CanvasRenderingContext2D, w: number, top: number, laneX: (l: number) => number) {
    const wip = props.wip!;
    const y = -ROW_H - top, cy = y + ROW_H / 2;
    const hr = headRow();
    const head = hr === null ? undefined : rowAt(hr);
    const lane = head?.g.lane ?? 0;
    const color = head ? laneColor(head.g.color) : token("color.text.muted");
    const nx = laneX(lane);
    const gx = widths().refs - 8;
    ctx.save();
    if (y + ROW_H > 0) {
      if (wip.active) {
        ctx.fillStyle = token("color.bg.selected");
        ctx.fillRect(0, y, w, ROW_H);
      }
    }
    // Dotted line from the WIP node to HEAD's node, over the rows in between.
    ctx.beginPath();
    ctx.rect(gx, 0, graphW(), size().h);
    ctx.clip();
    if (hr !== null) {
      const hy = hr * ROW_H - top + ROW_H / 2;
      ctx.setLineDash([3, 3]);
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(nx, cy + 6);
      ctx.lineTo(nx, hy - 6);
      ctx.stroke();
      ctx.setLineDash([]);
    }
    ctx.beginPath();
    ctx.arc(nx, cy, 6, 0, Math.PI * 2);
    ctx.fillStyle = token(wip.active ? "color.bg.selected" : "color.bg.canvas");
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.restore();
    if (y + ROW_H <= 0) return;
    // Message column: icon, label, then one pill per non-empty count.
    const col = columns(w, graphW(), widths());
    let x = col.msgX;
    if (PENCIL) {
      ctx.save();
      ctx.translate(x, cy - 7);
      ctx.scale(14 / 24, 14 / 24);
      ctx.strokeStyle = color;
      ctx.lineWidth = 2.2;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.stroke(PENCIL);
      ctx.restore();
      x += 20;
    }
    ctx.font = "600 13px Inter, Cantarell, system-ui, sans-serif";
    ctx.fillStyle = token("color.text.primary");
    const label = t("wip.row");
    ctx.fillText(label, x, cy);
    x += ctx.measureText(label).width + 10;
    ctx.font = "600 11.5px Inter, Cantarell, system-ui, sans-serif";
    const pills: [number, string, string][] = [
      [wip.conflicted, t("wip.chipConflicts", { n: wip.conflicted }), token("color.status.deleted")],
      [wip.unstaged, t("wip.chipUnstaged", { n: wip.unstaged }), token("color.status.modified")],
      [wip.staged, t("wip.chipStaged", { n: wip.staged }), token("color.status.added")],
    ];
    if (!wip.conflicted && !wip.unstaged && !wip.staged) pills.push([1, t("wip.clean"), token("color.text.muted")]);
    for (const [n, text, c] of pills) {
      if (!n) continue;
      const pw = ctx.measureText(text).width + 14;
      if (x + pw > col.shaX - 8) break;
      ctx.strokeStyle = c;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.roundRect(x + 0.5, cy - 9.5, pw - 1, 19, 9.5);
      ctx.stroke();
      ctx.fillStyle = c;
      ctx.fillText(text, x + 7, cy + 0.5);
      x += pw + 6;
    }
    ctx.font = "13px Inter, Cantarell, system-ui, sans-serif";
  }


  // New repo in this view (tab switch) or reload: restore the saved position, else the top.
  createEffect(() => {
    const top = savedScroll.get(props.repo.repoId) ?? 0;
    if (scroller) {
      scroller.scrollTop = top;
      setScrollTop(top);
    }
  });

  function select(i: number) {
    if (i < 0 || i >= props.repo.rowCount) return;
    const r = rowAt(i);
    if (!r) return ensure(Math.floor(i / CHUNK));
    props.onSelect(i, r.m.sha);
    reveal(i);
  }

  /** Scrolls just enough to show row `i`. */
  function reveal(i: number) {
    const y = i * ROW_H + off();
    if (y < scroller.scrollTop) scroller.scrollTop = y;
    else if (y + ROW_H > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = y + ROW_H - scroller.clientHeight;
  }

  /** Menu key / Shift+F10: the selected commit's menu, under its row at the message column. */
  function openMenuFromKeyboard() {
    const i = props.selected;
    if (!props.onMenu || props.wip?.active || i === null) return;
    const r = rowAt(i);
    if (!r) return;
    reveal(i);
    const b = scroller.getBoundingClientRect();
    const msgX = columns(size().w, graphW(), widths()).msgX;
    const at = menuAnchor(i, { left: b.left, top: b.top, width: scroller.clientWidth, height: scroller.clientHeight }, scroller.scrollTop,
      { rowH: ROW_H, offset: off(), msgX, viewW: innerWidth, viewH: innerHeight });
    props.onMenu(r.m.sha, at.x, at.y);
  }

  function onKey(e: KeyboardEvent) {
    if (isMenuKey(e)) {
      e.preventDefault();
      return openMenuFromKeyboard();
    }
    const page = Math.floor(scroller.clientHeight / ROW_H);
    const cur = props.selected ?? -1;
    const next = { ArrowDown: cur + 1, ArrowUp: cur - 1, PageDown: cur + page, PageUp: cur - page, Home: 0, End: props.repo.rowCount - 1 }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    if (props.wip && props.onWip && (next < 0 || e.key === "Home")) {
      scroller.scrollTop = 0;
      return props.onWip();
    }
    select(Math.max(0, Math.min(props.repo.rowCount - 1, next)));
  }

  onMount(() => {
    const ro = new ResizeObserver(() => setSize({ w: scroller.clientWidth, h: scroller.clientHeight }));
    ro.observe(scroller);
    onCleanup(() => ro.disconnect());
    // On open, or when the reload removed the focused old graph; never steal focus from a control
    // that is still there (a sidebar or WIP button the operation started from, spec A-6).
    const a = document.activeElement;
    if (!a || a === document.body || !a.isConnected) scroller.focus();
  });

  // Accessibility mirror: one hidden row per loaded row in the viewport ± OVERSCAN (plus the
  // selected row), placed where the canvas draws it. Unloaded rows are left out, never empty.
  const mirrored = () => {
    tick();
    return visibleRange(scrollTop(), size().h, ROW_H, off(), props.repo.rowCount, OVERSCAN, props.selected).filter((i) => rowAt(i));
  };
  // Never points at a row that is not in the DOM (selected but not loaded yet: none).
  const activeRow = () => {
    if (props.wip?.active) return rowId(props.repo.repoId, "wip");
    tick();
    const i = props.selected;
    return i !== null && rowAt(i) ? rowId(props.repo.repoId, i) : undefined;
  };

  return (
    <div class="graph">
      <div class="graph-header">
        {(() => {
          const c = columns(size().w, graphW(), widths());
          return (
            <>
              <span style={{ left: "8px" }}>{t("col.refs")}</span>
              <span class="col-resize" style={{ left: `${widths().refs - 6}px` }} onPointerDown={(e) => startResize(e, "refs")} />
              <span style={{ left: `${widths().refs}px` }}>{t("col.graph")}</span>
              {c.msgW > 48 && <span style={{ left: `${c.msgX}px` }}>{t("col.message")}</span>}
              {c.showAuthor && <span style={{ left: `${c.authorX}px` }}>{t("col.author")}</span>}
              {c.showAuthor && <span class="col-resize" style={{ left: `${c.authorX - 6}px` }} onPointerDown={(e) => startResize(e, "author")} />}
              {c.showDate && <span style={{ left: `${c.dateX}px` }}>{t("col.date")}</span>}
              {c.showDate && <span class="col-resize" style={{ left: `${c.dateX - 6}px` }} onPointerDown={(e) => startResize(e, "date")} />}
              <span style={{ left: `${c.shaX}px` }}>{t("col.sha")}</span>
            </>
          );
        })()}
      </div>
      <div
        ref={scroller}
        class="graph-scroller"
        tabindex="0"
        role="grid"
        aria-label={t("graph.label")}
        aria-describedby={`graph-keys-${props.repo.repoId}`}
        aria-rowcount={props.repo.rowCount + (props.wip ? 1 : 0)}
        aria-colcount={5}
        aria-multiselectable="false"
        aria-activedescendant={activeRow()}
        onScroll={() => {
          setScrollTop(scroller.scrollTop);
          savedScroll.set(props.repo.repoId, scroller.scrollTop);
        }}
        onKeyDown={onKey}
        onClick={(e) => {
          const rect = scroller.getBoundingClientRect();
          const yy = e.clientY - rect.top + scroller.scrollTop - off();
          if (yy < 0) return props.onWip?.();
          select(Math.floor(yy / ROW_H));
        }}
        onContextMenu={(e) => {
          // Raised by the keyboard (no mouse button): same as the menu key, never the row under (x, y).
          if (e.button !== 2) {
            e.preventDefault();
            return openMenuFromKeyboard();
          }
          const rect = scroller.getBoundingClientRect();
          const yy = e.clientY - rect.top + scroller.scrollTop - off();
          if (yy < 0) return;
          const i = Math.floor(yy / ROW_H);
          const r = rowAt(i);
          if (!r || !props.onMenu) return;
          e.preventDefault();
          select(i);
          props.onMenu(r.m.sha, e.clientX, e.clientY);
        }}
      >
        <canvas ref={canvas} class="graph-canvas" style={{ width: `${size().w}px`, height: `${size().h}px` }} />
        <div style={{ height: `${Math.max(0, props.repo.rowCount * ROW_H + off() - size().h)}px` }} />
        <div class="graph-rows" role="rowgroup">
          <Show when={props.wip}>
            {(w) => (
              <div role="row" id={rowId(props.repo.repoId, "wip")} aria-rowindex={1} aria-selected={w().active} style={{ top: "0px" }}>
                <span role="gridcell" /><span role="gridcell">{wipText(w())}</span><span role="gridcell" /><span role="gridcell" /><span role="gridcell" />
              </div>
            )}
          </Show>
          <For each={mirrored()}>
            {(i) => {
              const cells = () => {
                tick();
                const r = rowAt(i);
                return r && rowText(r.m, refsByRow().get(i) ?? [], props.shared?.get(r.m.sha) ?? [], activeLocale());
              };
              return (
                <Show when={cells()}>
                  {(c) => (
                    <div role="row" id={rowId(props.repo.repoId, i)} aria-rowindex={i + 1 + (props.wip ? 1 : 0)}
                      aria-selected={props.selected === i && !props.wip?.active} style={{ top: `${i * ROW_H + off()}px` }}>
                      <span role="gridcell">{c().refs}</span>
                      <span role="gridcell">{c().message}</span>
                      <span role="gridcell">{c().author}</span>
                      <span role="gridcell">{c().date}</span>
                      <span role="gridcell">{c().sha}</span>
                    </div>
                  )}
                </Show>
              );
            }}
          </For>
        </div>
      </div>
      <div id={`graph-keys-${props.repo.repoId}`} class="sr-only">{t("graph.keys")}</div>
      {loadError() && <div class="graph-error" role="alert">{loadError()}</div>}
      {perfEnabled() && (
        <div class="perf" aria-hidden="true">
          <div>first paint {perfStats().firstPaintMs.toFixed(0)} ms</div>
          <div>draw {perfStats().drawMs.toFixed(1)} ms · {perfStats().rows} rows</div>
          <div>{perfStats().fps} painted frames/s · worst {perfStats().worstMs.toFixed(1)} ms</div>
          <div>{props.repo.rowCount.toLocaleString()} commits · {props.repo.laneCount} lanes</div>
        </div>
      )}
    </div>
  );
}

function clipText(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, max: number) {
  if (max <= 0) return;
  if (ctx.measureText(text).width <= max) return ctx.fillText(text, x, y);
  let lo = 0, hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(text.slice(0, mid) + "…").width <= max) lo = mid;
    else hi = mid - 1;
  }
  ctx.fillText(text.slice(0, lo) + "…", x, y);
}
