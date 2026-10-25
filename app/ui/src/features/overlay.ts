// SPDX-License-Identifier: Apache-2.0
// Shared behaviour for menus and dialogs: Escape closes, focus goes in on open and back on close,
// Tab stays inside (accessibility A-4), plus the keyboard menu key and focus recovery helpers.
import { onCleanup, onMount } from "solid-js";

/** Menu key or Shift+F10: opens the context menu of the focused item (spec accessibility A-2). */
export const isMenuKey = (e: { key: string; shiftKey: boolean }) => e.key === "ContextMenu" || (e.shiftKey && e.key === "F10");

/**
 * Focus trap step: index of the next element among `count` focusables when Tab (or Shift+Tab when
 * `back`) is pressed on `current` (-1: focus is outside the container). -1 when there is none.
 */
export function cycleIndex(count: number, current: number, back: boolean): number {
  if (count <= 0) return -1;
  if (current < 0) return back ? count - 1 : 0;
  return (current + (back ? -1 : 1) + count) % count;
}

const FOCUSABLE = "button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, a[href], [tabindex]:not([tabindex='-1'])";

/** Focusable, rendered descendants of `root`, in tab order (no positive tabindex is used in the app). */
export function focusables(root: Element): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.tabIndex >= 0 && el.getClientRects().length > 0);
}

/** Overlays with a trap, innermost last: only the innermost one handles Tab. */
const traps: object[] = [];

/** Escape closes only the innermost trapped overlay; one without a trap (not in `stack`) always handles it. */
export const handlesEscape = (stack: readonly object[], self: object) => !stack.includes(self) || stack[stack.length - 1] === self;

/**
 * `initial` gets the focus on open. With `root` (the dialog or menu element; may change while the
 * overlay is open), Tab and Shift+Tab cycle inside it, so the page behind is not reachable, and the
 * browser's own context menu is suppressed inside it.
 */
export function useOverlay(onClose: () => void, initial?: () => HTMLElement | undefined, root?: () => HTMLElement | undefined) {
  const before = document.activeElement as HTMLElement | null;
  const self = {};
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (!handlesEscape(traps, self)) return;
      e.preventDefault();
      onClose();
      return;
    }
    const el = root?.();
    if (e.key !== "Tab" || !el || traps[traps.length - 1] !== self) return;
    const items = focusables(el);
    const next = cycleIndex(items.length, items.indexOf(document.activeElement as HTMLElement), e.shiftKey);
    e.preventDefault();
    if (next >= 0) items[next].focus();
  };
  const menu = (e: MouseEvent) => {
    if (root?.()?.contains(e.target as Node)) e.preventDefault();
  };
  onMount(() => {
    document.addEventListener("keydown", key);
    if (root) {
      traps.push(self);
      document.addEventListener("contextmenu", menu);
    }
    initial?.()?.focus();
  });
  onCleanup(() => {
    document.removeEventListener("keydown", key);
    document.removeEventListener("contextmenu", menu);
    const i = traps.indexOf(self);
    if (i >= 0) traps.splice(i, 1);
    if (before?.isConnected) before.focus();
  });
}

/** Up/Down/Home/End move focus between the buttons of a menu. */
export function menuKeys(e: KeyboardEvent) {
  const items = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  const i = items.indexOf(document.activeElement as HTMLButtonElement);
  const next = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: items.length - 1 }[e.key];
  if (next === undefined || !items.length) return;
  e.preventDefault();
  items[(next + items.length) % items.length].focus();
}

/**
 * Where focus goes when the focused item of a list disappeared: the item with the same key when
 * the list was only re-rendered, else the one now at the same position (the next item), else the
 * last one; -1 for an empty list.
 */
export function neighbourIndex(keys: readonly string[], key: string, index: number): number {
  const same = keys.indexOf(key);
  if (same >= 0) return same;
  return keys.length ? Math.min(Math.max(index, 0), keys.length - 1) : -1;
}

/**
 * Keeps keyboard focus when an operation re-renders the list holding the focused control
 * (spec accessibility A-6). Items carry `data-focus-key` inside a `data-focus-group` container; when
 * the focused element is removed and focus fell to the page, it goes to the same or the
 * neighbouring item of that group, else to the commit graph. Installed once by the app.
 */
export function keepFocus(): () => void {
  let last: { el: HTMLElement; group: string | null; key: string | null; index: number } | null = null;
  const onFocus = (e: FocusEvent) => {
    const el = e.target as HTMLElement;
    const item = el.closest?.("[data-focus-key]");
    const group = el.closest?.("[data-focus-group]");
    const keys = group ? [...group.querySelectorAll("[data-focus-key]")] : [];
    last = { el, group: group?.getAttribute("data-focus-group") ?? null, key: item?.getAttribute("data-focus-key") ?? null, index: item ? keys.indexOf(item) : -1 };
  };
  const recover = () => {
    // Removed, or disabled while focused (the browser then drops focus, e.g. Commit after committing).
    if (!last || (last.el.isConnected && !last.el.matches(":disabled"))) return;
    const lost = last;
    last = null;
    const active = document.activeElement;
    if (active && active !== document.body && active !== lost.el) return;
    let target: HTMLElement | undefined;
    const group = lost.group ? document.querySelector(`[data-focus-group="${CSS.escape(lost.group)}"]`) : null;
    if (group && lost.key !== null) {
      const items = [...group.querySelectorAll("[data-focus-key]")];
      const i = neighbourIndex(items.map((x) => x.getAttribute("data-focus-key") ?? ""), lost.key, lost.index);
      if (i >= 0) target = focusables(items[i])[0] ?? (items[i] as HTMLElement);
    }
    // An emptied list: the first control of its section (e.g. the list's header toggle).
    if (!target && group) target = focusables(group)[0] ?? focusables(group.closest("section, details, aside") ?? group)[0];
    target ??= document.querySelector<HTMLElement>(".graph-scroller") ?? undefined;
    target?.focus();
  };
  document.addEventListener("focusin", onFocus);
  const mo = new MutationObserver(recover);
  mo.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["disabled"] });
  return () => {
    document.removeEventListener("focusin", onFocus);
    mo.disconnect();
  };
}
