// SPDX-License-Identifier: Apache-2.0
// Canvas commit graph (roadmap T1.8): virtualized rows, lanes, refs,
// selection, keyboard navigation and a hidden DOM mirror of the visible rows for assistive
// technology (spec accessibility A-1: ARIA grid rows, aria-activedescendant).
import { batch, createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show, untrack } from "solid-js";
import { api, FLAG_MERGE, GraphRow, OpenedRepo, RefInfo, RowMeta, SEG_IN, SEG_OUT, SEG_PASS, SharedStashInfo } from "../../api";
import { activeTheme, laneColor, token } from "../../theme/theme";
import { refLabelColors } from "./refLabel";
import { AvatarStore, avatarColor, initials, initialsInk } from "./avatar";
import { activeLocale, t } from "../../i18n";

const CHUNK = 200;
/** Lanes shown before the graph column stops growing (the rest is clipped). */
const MAX_VISIBLE_LANES = 12;
/** Row chunks kept in memory per view (PERF-03): 20 x 200 rows, enough for the viewport and overscan. */
const MAX_CACHED_CHUNKS = 20;
import { perfEnabled, perfStats, recordDraw } from "./perf";
import { columns, ColumnWidths, loadWidths, saveWidths, clampWidth, resolveWidths } from "./columns";
import { menuAnchor, rowId, rowText, visibleRange, wipText } from "./a11y";
import { isMenuKey } from "../overlay";
import { AVATAR_R, AVATAR_RING, LANE_W, ROW_H, canvasFont, centerBaseline, crisp, edgeCurve, laneX as laneXAt, rowY } from "./geometry";
import { layoutWidth, maxPan, stepZoom, wheelZoom, ZOOM_KEYS, zoomAt } from "./zoom";
import { visualScale } from "../split/size";
import { ChunkCache } from "./chunkCache";

/** Rows mirrored in the DOM above and below the viewport (spec accessibility §4.2: ≤ 20). */
const OVERSCAN = 10;

interface Chunk { rows: GraphRow[]; meta: RowMeta[] }

// Lucide "pencil-line" (ISC), drawn on the canvas for the uncommitted-changes row.
const PENCIL = typeof Path2D === "undefined" ? null : new Path2D("M13 21h8 M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z M15 5l4 4");

// Survives remounts (graph reload after commit/undo/redo) so the view keeps its place.
const savedScroll = new Map<number, number>();
let savedZoom = 1;

/** Called when a tab closes: its scroll position is no longer needed (PERF-03). */
export function forgetGraphScroll(repoId: number): void {
  savedScroll.delete(repoId);
}

// Text widths and date strings repeat on every repaint of the same rows: keep them, bounded (PERF-10).
const textWidths = new ChunkCache<number>(4000);
const dateStrings = new ChunkCache<string>(2000);
// Widths measured before a web font finished loading used the fallback face: measure again.
if (typeof document !== "undefined" && document.fonts) document.fonts.addEventListener("loadingdone", () => textWidths.clear());

function textWidth(ctx: CanvasRenderingContext2D, text: string): number {
  const key = `${ctx.font}\u0000${text}`;
  let w = textWidths.get(key);
  if (w === undefined) {
    w = ctx.measureText(text).width;
    textWidths.set(key, w);
  }
  return w;
}

function dateText(time: number, locale: string): string {
  const key = `${locale}|${time}`;
  let s = dateStrings.get(key);
  if (s === undefined) {
    s = new Date(time * 1000).toLocaleDateString(locale);
    dateStrings.set(key, s);
  }
  return s;
}

/** Theme colours of the paint loop, resolved once per frame instead of once per row (PERF-10). */
function paletteNow() {
  return {
    canvas: token("color.bg.canvas"),
    selected: token("color.bg.selected"),
    text: token("color.text.primary"),
    textSecondary: token("color.text.secondary"),
    textMuted: token("color.text.muted"),
    accent: token("color.accent.default"),
    accentMuted: token("color.accent.muted"),
    raised: token("color.bg.raised"),
    onAccent: token("color.text.onAccent"),
    lanes: Array.from({ length: 10 }, (_, i) => laneColor(i)),
  };
}

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
  // Cache keys include the repo id. The component is reused across tabs, so the cache is bounded
  // (LRU) and emptied when the shown repo changes; the instance itself is discarded on unmount.
  const chunks = new ChunkCache<Chunk>(MAX_CACHED_CHUNKS);
  const pending = new Set<string>();
  let mounted = true;
  const [loadError, setLoadError] = createSignal<string | null>(null);
  const key = (chunk: number) => `${props.repo.repoId}:${chunk}`;
  createEffect(on(() => props.repo.repoId, () => chunks.clear(), { defer: true }));
  onCleanup(() => {
    mounted = false;
    chunks.clear();
  });
  const [tick, setTick] = createSignal(0);
  // w, h: css px of the view; k: on-screen px per css px (global UI zoom).
  const [size, setSize] = createSignal({ w: 800, h: 600, k: 1 });
  const measure = () => setSize({ w: scroller.clientWidth, h: scroller.clientHeight, k: visualScale(scroller) });
  // scrollTop is in scroll px, which are logical px × zoom (the spacer is scaled).
  const [scrollTop, setScrollTop] = createSignal(0);
  const [zoom, setZoom] = createSignal(savedZoom);
  const [panRaw, setPan] = createSignal(0);
  // Logical px; zooming out or widening the view pulls it back in range.
  const pan = () => Math.min(panRaw(), maxPan(size().w, zoom()));
  const panTo = (x: number) => setPan(Math.min(maxPan(size().w, zoom()), Math.max(0, x)));
  const layoutW = () => layoutWidth(size().w, zoom());
  const [zoomMsg, setZoomMsg] = createSignal("");
  const [stored, setWidths] = createSignal<ColumnWidths>(loadWidths());
  // Effective widths: an unset refs width follows the available space.
  const widths = () => resolveWidths(stored(), layoutW(), graphW());

  /** Drag a header edge to resize a column (refs grows to the right; author/date grow to the left). */
  function startResize(e: PointerEvent, col: keyof ColumnWidths) {
    e.preventDefault();
    const x0 = e.clientX, w0 = widths()[col], k = size().k * zoom();
    const dir = col === "refs" ? 1 : -1;
    const move = (ev: PointerEvent) => setWidths({ ...stored(), refs: widths().refs, [col]: clampWidth(col, w0 + dir * (ev.clientX - x0) / k) });
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
        // A chunk that arrives after the view moved to another repo (or unmounted) is dropped.
        if (mounted && props.repo.repoId === repoId) chunks.set(k, { rows: w.rows, meta });
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

  // Bumped when a photo finishes loading so the canvas repaints.
  const [avatarTick, setAvatarTick] = createSignal(0);
  const avatars = new AvatarStore((e, sha) => api.avatar(props.repo.repoId, e, sha), (b) => createImageBitmap(new Blob([b])), () => setAvatarTick((n) => n + 1));

  /** Author photo (or initials) in a circle with a lane-colour ring. */
  function drawAvatar(ctx: CanvasRenderingContext2D, x: number, y: number, name: string, email: string, sha: string, lane: string) {
    const inner = AVATAR_R - AVATAR_RING;
    const photo = avatars.get(email, sha);
    ctx.save();
    ctx.beginPath();
    ctx.arc(x, y, AVATAR_R, 0, Math.PI * 2);
    ctx.fillStyle = lane;
    ctx.fill();
    ctx.beginPath();
    ctx.arc(x, y, inner, 0, Math.PI * 2);
    ctx.clip();
    if (photo) {
      ctx.drawImage(photo, x - inner, y - inner, inner * 2, inner * 2);
    } else {
      const bg = avatarColor(email);
      ctx.fillStyle = bg;
      ctx.fillRect(x - inner, y - inner, inner * 2, inner * 2);
      ctx.fillStyle = initialsInk(bg);
      ctx.font = canvasFont(10, 600);
      ctx.textAlign = "center";
      ctx.textBaseline = "alphabetic";
      const label = initials(name), m = ctx.measureText(label);
      ctx.fillText(label, x, centerBaseline(m.actualBoundingBoxAscent, m.actualBoundingBoxDescent, y));
    }
    ctx.restore();
  }

  function draw() {
    avatarTick();
    const drawStart = performance.now();
    let painted = 0;
    tick();
    activeTheme();
    const pal = paletteNow();
    // Tracked like the theme: a language switch repaints dates and canvas labels.
    const locale = activeLocale();
    const { w: viewW, h, k } = size();
    const z = zoom();
    // Device px per css px, the global UI zoom included, so the bitmap is never stretched.
    const dpr = (window.devicePixelRatio || 1) * k;
    // Assigning width/height reallocates the backing store: only do it on real size changes.
    const bw = Math.round(viewW * dpr), bh = Math.round(h * dpr);
    if (canvas.width !== bw) canvas.width = bw;
    if (canvas.height !== bh) canvas.height = bh;
    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = pal.canvas;
    ctx.fillRect(0, 0, viewW, h);
    // From here on, logical px: the graph zoom is redrawn at scale s, panned by whole device px.
    const s = dpr * z, px = Math.round(pan() * s) / s;
    ctx.setTransform(s, 0, 0, s, -px * s, 0);
    const w = layoutW();

    const top = scrollTop() / z - off();
    const first = Math.max(0, Math.floor(top / ROW_H));
    const last = Math.min(props.repo.rowCount - 1, Math.ceil((top + h / z) / ROW_H));
    for (let c = Math.floor(first / CHUNK); c <= Math.floor(last / CHUNK); c++) ensure(c);

    const gx = widths().refs - 8;
    const laneX = (l: number) => crisp(laneXAt(gx, l), 2, s);
    const refs = refsByRow();
    ctx.font = canvasFont(13);
    ctx.textBaseline = "middle";

    for (let i = first; i <= last; i++) {
      const y = rowY(i, top);
      const cy = y + ROW_H / 2;
      const r = rowAt(i);
      if (i === props.selected) {
        ctx.fillStyle = pal.selected;
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
        ctx.strokeStyle = pal.lanes[s.color % 10];
        ctx.beginPath();
        if (s.kind === SEG_PASS) {
          ctx.moveTo(laneX(s.from), y);
          ctx.lineTo(laneX(s.from), y + ROW_H);
        } else if (s.kind === SEG_IN) {
          const k = edgeCurve("in", laneX(s.from), laneX(s.to), y);
          ctx.moveTo(...k.p0);
          ctx.bezierCurveTo(...k.c1, ...k.c2, ...k.p1);
        } else if (s.kind === SEG_OUT) {
          const k = edgeCurve("out", laneX(s.from), laneX(s.to), y);
          ctx.moveTo(...k.p0);
          ctx.bezierCurveTo(...k.c1, ...k.c2, ...k.p1);
        }
        ctx.stroke();
      }
      // Node
      const nx = laneX(r.g.lane);
      if (r.g.flags & FLAG_MERGE) {
        ctx.beginPath();
        ctx.arc(nx, cy, 4, 0, Math.PI * 2);
        ctx.fillStyle = pal.lanes[r.g.color % 10];
        ctx.fill();
        ctx.strokeStyle = pal.canvas;
        ctx.lineWidth = 2;
        ctx.stroke();
      } else {
        drawAvatar(ctx, nx, cy, r.m.author, r.m.email, r.m.sha, pal.lanes[r.g.color % 10]);
      }
      ctx.restore();
      // More lanes than fit: fade the right edge as a hint that the graph continues.
      if (props.repo.laneCount > MAX_VISIBLE_LANES) {
        const fx = gx + graphW() - 12;
        const grad = ctx.createLinearGradient(fx, 0, fx + 12, 0);
        grad.addColorStop(0, "transparent");
        grad.addColorStop(1, i === props.selected ? pal.selected : pal.canvas);
        ctx.fillStyle = grad;
        ctx.fillRect(fx, y, 12, ROW_H);
      }
      // Ref labels
      let rx = 8;
      const sharedHere = props.shared?.get(r.m.sha);
      if (sharedHere && widths().refs > 48) {
        // Same shape as the stash ref labels, so it reads as a label and not as a button.
        const label = sharedHere.length > 1 ? t("ss.glyphMany", { n: String(sharedHere.length) }) : t("ss.glyphOne");
        const tw = Math.min(textWidth(ctx, label) + 26, widths().refs - rx - 8);
        ctx.fillStyle = pal.accentMuted;
        ctx.fillRect(rx, y + 4, tw, ROW_H - 8);
        // Arrow leaving a tray: "shared out of this commit".
        const ax = rx + 11, ay = cy;
        ctx.strokeStyle = pal.accent;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(ax, ay + 2); ctx.lineTo(ax, ay - 5);
        ctx.moveTo(ax - 3, ay - 2); ctx.lineTo(ax, ay - 5); ctx.lineTo(ax + 3, ay - 2);
        ctx.moveTo(ax - 4.5, ay + 5); ctx.lineTo(ax + 4.5, ay + 5);
        ctx.stroke();
        ctx.fillStyle = pal.text;
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
        const tw = Math.min(textWidth(ctx, label) + 12, widths().refs - rx - 8);
        if (tw < 24) break;
        const lab = ref.kind === "stash"
          ? { bg: pal.accentMuted, fg: pal.text }
          : refLabelColors(pal.lanes[r.g.color % 10], ref.isHead, { canvas: pal.canvas, light: pal.text, dark: pal.onAccent });
        ctx.fillStyle = lab.bg;
        ctx.fillRect(rx, y + 4, tw, ROW_H - 8);
        ctx.fillStyle = lab.fg;
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
      ctx.fillStyle = pal.text;
      clipText(ctx, r.m.summary, col.msgX, cy, col.msgW);
      ctx.fillStyle = pal.textSecondary;
      if (col.showAuthor) clipText(ctx, r.m.author, col.authorX, cy, widths().author - 12);
      if (col.showDate) clipText(ctx, dateText(r.m.time, locale), col.dateX, cy, widths().date - 8);
      ctx.font = "12.5px 'JetBrains Mono', 'DejaVu Sans Mono', monospace";
      // Muted text is below 4.5:1 on the selection colour of Atlas Dark (3.86): use secondary there.
      ctx.fillStyle = (i === props.selected ? pal.textSecondary : pal.textMuted);
      ctx.fillText(r.m.sha.slice(0, 7), col.shaX, cy);
      ctx.font = canvasFont(13);
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
    ctx.rect(gx, 0, graphW(), size().h / zoom());
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
    ctx.font = canvasFont(13, 600);
    ctx.fillStyle = token("color.text.primary");
    const label = t("wip.row");
    ctx.fillText(label, x, cy);
    x += ctx.measureText(label).width + 10;
    ctx.font = canvasFont(11.5, 600);
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
    ctx.font = canvasFont(13);
  }


  // New repo in this view (tab switch) or reload: restore the saved position, else the top.
  createEffect(() => {
    const top = (savedScroll.get(props.repo.repoId) ?? 0) * untrack(zoom);
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
    const z = zoom(), y = (i * ROW_H + off()) * z, rh = ROW_H * z;
    if (y < scroller.scrollTop) scroller.scrollTop = y;
    else if (y + rh > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = y + rh - scroller.clientHeight;
  }

  /** Zooms to `z1`, keeping the logical point under (cx, cy) (css px in the view) in place. */
  function applyZoom(z1: number, cx: number, cy: number) {
    const z0 = zoom();
    if (z1 === z0) return;
    const next = zoomAt({ z0, z1, panX: pan(), scrollTop: scroller.scrollTop, w: size().w }, cx, cy);
    savedZoom = z1;
    batch(() => { setZoom(z1); setPan(next.panX); });
    // After the batch the spacer has its new height, so this scroll is not clamped to the old one.
    scroller.scrollTop = next.scrollTop;
    setScrollTop(scroller.scrollTop);
    savedScroll.set(props.repo.repoId, scroller.scrollTop / z1);
    setZoomMsg(t("graph.zoomLevel", { n: Math.round(z1 * 100) }));
  }

  // Set by a drag-pan so the click that ends it does not select a row.
  let dragged = false;
  /** Drag the canvas to pan (sideways only when zoomed in). Touch keeps native scrolling. */
  function startPan(e: PointerEvent) {
    dragged = false;
    if (e.button !== 0 || e.pointerType === "touch" || e.target !== canvas) return;
    const x0 = e.clientX, y0 = e.clientY, p0 = pan(), t0 = scroller.scrollTop, k = size().k;
    const move = (ev: PointerEvent) => {
      const dx = (ev.clientX - x0) / k, dy = (ev.clientY - y0) / k;
      if (!dragged && Math.hypot(dx, dy) < 4) return;
      dragged = true;
      scroller.classList.add("panning");
      panTo(p0 - dx / zoom());
      scroller.scrollTop = t0 - dy;
    };
    const up = () => {
      scroller.classList.remove("panning");
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  /** Ctrl+wheel (and touchpad pinch) zooms at the pointer; Shift+wheel or a sideways swipe pans. */
  function onWheel(e: WheelEvent) {
    const b = scroller.getBoundingClientRect(), k = size().k;
    if (e.ctrlKey) {
      e.preventDefault();
      return applyZoom(wheelZoom(zoom(), e.deltaY, e.deltaMode), (e.clientX - b.left) / k, (e.clientY - b.top) / k);
    }
    if (maxPan(size().w, zoom()) > 0 && (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY))) {
      e.preventDefault();
      panTo(pan() + (e.shiftKey ? e.deltaY : e.deltaX) / zoom());
    }
  }

  /** Menu key / Shift+F10: the selected commit's menu, under its row at the message column. */
  function openMenuFromKeyboard() {
    const i = props.selected;
    if (!props.onMenu || props.wip?.active || i === null) return;
    const r = rowAt(i);
    if (!r) return;
    reveal(i);
    // menuAnchor works in on-screen px: scale the logical geometry by the graph and UI zoom.
    const b = scroller.getBoundingClientRect(), k = size().k, zk = zoom() * k;
    const msgX = (columns(layoutW(), graphW(), widths()).msgX - pan()) * zk;
    const at = menuAnchor(i, { left: b.left, top: b.top, width: scroller.clientWidth * k, height: scroller.clientHeight * k }, scroller.scrollTop * k,
      { rowH: ROW_H * zk, offset: off() * zk, msgX, viewW: innerWidth, viewH: innerHeight });
    props.onMenu(r.m.sha, at.x, at.y);
  }

  function onKey(e: KeyboardEvent) {
    if (isMenuKey(e)) {
      e.preventDefault();
      return openMenuFromKeyboard();
    }
    // Ctrl +/-/0 zoom the graph (the app-wide UI zoom skips handled keys); arrows pan when zoomed in.
    const zdir = e.ctrlKey || e.metaKey ? ZOOM_KEYS[e.key] : undefined;
    if (zdir !== undefined) {
      e.preventDefault();
      return applyZoom(stepZoom(zoom(), zdir), size().w / 2, size().h / 2);
    }
    const sideways = { ArrowLeft: -40, ArrowRight: 40 }[e.key];
    if (sideways !== undefined && maxPan(size().w, zoom()) > 0) {
      e.preventDefault();
      return panTo(pan() + sideways / zoom());
    }
    const page = Math.floor(scroller.clientHeight / (ROW_H * zoom()));
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
    // A UI zoom change resizes the view in css px, so this also re-reads the scale.
    const ro = new ResizeObserver(measure);
    ro.observe(scroller);
    // Not passive: Ctrl+wheel must not reach the webview's own zoom or scroll.
    scroller.addEventListener("wheel", onWheel, { passive: false });
    onCleanup(() => {
      ro.disconnect();
      scroller.removeEventListener("wheel", onWheel);
    });
    // Canvas text does not trigger @font-face loading: load the faces, then repaint once.
    void Promise.all([document.fonts?.load(canvasFont(13)), document.fonts?.load(canvasFont(13, 600))])
      .then(measure)
      .catch(() => {});
    // On open, or when the reload removed the focused old graph; never steal focus from a control
    // that is still there (a sidebar or WIP button the operation started from, spec A-6).
    const a = document.activeElement;
    if (!a || a === document.body || !a.isConnected) scroller.focus();
  });

  // Accessibility mirror: one hidden row per loaded row in the viewport ± OVERSCAN (plus the
  // selected row), placed where the canvas draws it. Unloaded rows are left out, never empty.
  const mirrored = () => {
    tick();
    return visibleRange(scrollTop() / zoom(), size().h / zoom(), ROW_H, off(), props.repo.rowCount, OVERSCAN, props.selected).filter((i) => rowAt(i));
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
          const c = columns(layoutW(), graphW(), widths()), z = zoom();
          // Logical column x to header px (labels keep their size; positions follow zoom and pan).
          const at = (x: number, d = 0) => ({ left: `${(x - pan()) * z + d}px` });
          return (
            <>
              {/* The auto refs column can be narrower than its label: clip it short of the resize handle. */}
              <span style={{ ...at(8), "max-width": `${Math.max(0, (widths().refs - 8) * z - 6)}px` }}>{t("col.refs")}</span>
              <span class="col-resize" style={at(widths().refs, -6)} onPointerDown={(e) => startResize(e, "refs")} />
              <span style={at(widths().refs)}>{t("col.graph")}</span>
              {c.msgW * z > 48 && <span style={at(c.msgX)}>{t("col.message")}</span>}
              {c.showAuthor && <span style={at(c.authorX)}>{t("col.author")}</span>}
              {c.showAuthor && <span class="col-resize" style={at(c.authorX, -6)} onPointerDown={(e) => startResize(e, "author")} />}
              {c.showDate && <span style={at(c.dateX)}>{t("col.date")}</span>}
              {c.showDate && <span class="col-resize" style={at(c.dateX, -6)} onPointerDown={(e) => startResize(e, "date")} />}
              <span style={at(c.shaX)}>{t("col.sha")}</span>
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
          savedScroll.set(props.repo.repoId, scroller.scrollTop / zoom());
        }}
        onKeyDown={onKey}
        onPointerDown={startPan}
        onClick={(e) => {
          if (dragged) return;
          const rect = scroller.getBoundingClientRect();
          // On-screen px → css px (UI zoom) → logical px (graph zoom).
          const yy = ((e.clientY - rect.top) / size().k + scroller.scrollTop) / zoom() - off();
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
          const yy = ((e.clientY - rect.top) / size().k + scroller.scrollTop) / zoom() - off();
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
        <div style={{ height: `${Math.max(0, (props.repo.rowCount * ROW_H + off()) * zoom() - size().h)}px` }} />
        <div class="graph-rows" role="rowgroup">
          <Show when={props.wip}>
            {(w) => (
              <div role="row" id={rowId(props.repo.repoId, "wip")} aria-rowindex={1} aria-selected={w().active} style={{ top: "0px", height: `${ROW_H * zoom()}px` }}>
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
                      aria-selected={props.selected === i && !props.wip?.active} style={{ top: `${(i * ROW_H + off()) * zoom()}px`, height: `${ROW_H * zoom()}px` }}>
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
      <div class="sr-only" role="status">{zoomMsg()}</div>
      <Show when={zoom() !== 1}>
        <button class="graph-zoom" aria-label={t("graph.zoomReset", { n: Math.round(zoom() * 100) })}
          onClick={() => { applyZoom(1, size().w / 2, size().h / 2); scroller.focus(); }}>
          {Math.round(zoom() * 100)}%
        </button>
      </Show>
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
  if (textWidth(ctx, text) <= max) return ctx.fillText(text, x, y);
  let lo = 0, hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (textWidth(ctx, text.slice(0, mid) + "…") <= max) lo = mid;
    else hi = mid - 1;
  }
  ctx.fillText(text.slice(0, lo) + "…", x, y);
}
