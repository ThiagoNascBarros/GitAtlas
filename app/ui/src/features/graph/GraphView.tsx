// SPDX-License-Identifier: Apache-2.0
// Canvas commit graph (roadmap T1.8): virtualized rows, lanes, refs,
// selection, keyboard navigation and an accessible shadow of the selection.
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { api, FLAG_MERGE, GraphRow, OpenedRepo, RefInfo, RowMeta, SEG_IN, SEG_OUT, SEG_PASS } from "../../api";
import { activeTheme, laneColor, token } from "../../theme/theme";
import { t } from "../../i18n";

const ROW_H = 28;
const LANE_W = 16;
const CHUNK = 200;
/** Lanes shown before the graph column stops growing (the rest is clipped). */
const MAX_VISIBLE_LANES = 12;
import { perfEnabled, perfStats, recordDraw } from "./perf";
import { columns, ColumnWidths, loadWidths, saveWidths, clampWidth } from "./columns";

interface Chunk { rows: GraphRow[]; meta: RowMeta[] }

export function GraphView(props: {
  repo: OpenedRepo;
  selected: number | null;
  onSelect: (row: number, sha: string) => void;
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
  const [widths, setWidths] = createSignal<ColumnWidths>(loadWidths());

  /** Drag a header edge to resize a column (refs grows to the right; author/date grow to the left). */
  function startResize(e: PointerEvent, col: keyof ColumnWidths) {
    e.preventDefault();
    const x0 = e.clientX, w0 = widths()[col];
    const dir = col === "refs" ? 1 : -1;
    const move = (ev: PointerEvent) => setWidths({ ...widths(), [col]: clampWidth(col, w0 + dir * (ev.clientX - x0)) });
    const up = () => {
      saveWidths(widths());
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  const refsByRow = () => {
    const m = new Map<number, RefInfo[]>();
    for (const r of props.repo.refs) if (r.row !== null) m.set(r.row, [...(m.get(r.row) ?? []), r]);
    return m;
  };
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

    const top = scrollTop();
    const first = Math.floor(top / ROW_H);
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
      for (const ref of refs.get(i) ?? []) {
        const label = ref.kind === "tag" ? `◆ ${ref.name}` : ref.isHead ? `✓ ${ref.name}` : ref.name;
        const tw = Math.min(ctx.measureText(label).width + 12, widths().refs - rx - 8);
        if (tw < 24) break;
        ctx.fillStyle = ref.isHead ? token("color.accent.default") : token("color.bg.raised");
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
      if (col.showDate) clipText(ctx, new Date(r.m.time * 1000).toLocaleDateString(), col.dateX, cy, widths().date - 8);
      ctx.font = "12.5px 'JetBrains Mono', 'DejaVu Sans Mono', monospace";
      ctx.fillStyle = token("color.text.muted");
      ctx.fillText(r.m.sha.slice(0, 7), col.shaX, cy);
      ctx.font = "13px Inter, Cantarell, system-ui, sans-serif";
    }
    recordDraw(drawStart, painted);
  }

  createEffect(draw);


  // New repo in this view (tab switch): start at the top.
  createEffect(() => {
    props.repo.repoId;
    if (scroller) {
      scroller.scrollTop = 0;
      setScrollTop(0);
    }
  });

  function select(i: number) {
    if (i < 0 || i >= props.repo.rowCount) return;
    const r = rowAt(i);
    if (!r) return ensure(Math.floor(i / CHUNK));
    props.onSelect(i, r.m.sha);
    const y = i * ROW_H;
    if (y < scroller.scrollTop) scroller.scrollTop = y;
    else if (y + ROW_H > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = y + ROW_H - scroller.clientHeight;
  }

  function onKey(e: KeyboardEvent) {
    const page = Math.floor(scroller.clientHeight / ROW_H);
    const cur = props.selected ?? -1;
    const next = { ArrowDown: cur + 1, ArrowUp: cur - 1, PageDown: cur + page, PageUp: cur - page, Home: 0, End: props.repo.rowCount - 1 }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    select(Math.max(0, Math.min(props.repo.rowCount - 1, next)));
  }

  onMount(() => {
    const ro = new ResizeObserver(() => setSize({ w: scroller.clientWidth, h: scroller.clientHeight }));
    ro.observe(scroller);
    onCleanup(() => ro.disconnect());
    scroller.focus();
  });

  const selectedMeta = () => (props.selected === null ? undefined : rowAt(props.selected)?.m);

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
              <span style={{ left: `${c.msgX}px` }}>{t("col.message")}</span>
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
        aria-rowcount={props.repo.rowCount}
        onScroll={() => setScrollTop(scroller.scrollTop)}
        onKeyDown={onKey}
        onClick={(e) => {
          const rect = scroller.getBoundingClientRect();
          select(Math.floor((e.clientY - rect.top + scroller.scrollTop) / ROW_H));
        }}
      >
        <canvas ref={canvas} class="graph-canvas" style={{ width: `${size().w}px`, height: `${size().h}px` }} />
        <div style={{ height: `${Math.max(0, props.repo.rowCount * ROW_H - size().h)}px` }} />
      </div>
      {/* Accessible shadow of the canvas: announces the selected commit. */}
      {loadError() && <div class="graph-error" role="alert">{loadError()}</div>}
      {perfEnabled() && (
        <div class="perf" aria-hidden="true">
          <div>first paint {perfStats().firstPaintMs.toFixed(0)} ms</div>
          <div>draw {perfStats().drawMs.toFixed(1)} ms · {perfStats().rows} rows</div>
          <div>{perfStats().fps} painted frames/s · worst {perfStats().worstMs.toFixed(1)} ms</div>
          <div>{props.repo.rowCount.toLocaleString()} commits · {props.repo.laneCount} lanes</div>
        </div>
      )}
      <div class="sr-only" aria-live="polite" role="status">
        {selectedMeta() ? `${selectedMeta()!.summary}, ${selectedMeta()!.author}, ${selectedMeta()!.sha.slice(0, 7)}` : ""}
      </div>
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
