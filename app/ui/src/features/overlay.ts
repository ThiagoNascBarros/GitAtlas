// SPDX-License-Identifier: Apache-2.0
// Shared behaviour for menus and dialogs: Escape closes, focus goes in on open and back on close.
import { onCleanup, onMount } from "solid-js";

export function useOverlay(onClose: () => void, initial?: () => HTMLElement | undefined) {
  const before = document.activeElement as HTMLElement | null;
  const key = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.preventDefault();
    onClose();
  };
  onMount(() => {
    document.addEventListener("keydown", key);
    initial?.()?.focus();
  });
  onCleanup(() => {
    document.removeEventListener("keydown", key);
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
