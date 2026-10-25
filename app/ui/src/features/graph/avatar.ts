// SPDX-License-Identifier: Apache-2.0
// Author avatars for the graph: photo when Gravatar has one, otherwise initials on a stable colour.

import { contrast } from "./geometry";

const FALLBACK = ["#15a0bf", "#0669f7", "#8e00c2", "#c517b6", "#d90171", "#cd0101", "#f25d2e", "#c9a100", "#4aa81f", "#1fa57f"];
const norm = (e: string) => e.trim().toLowerCase();

/** First letters of the first and last word ("Ana Maria Silva" -> "AS"); "?" when there is no name. */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const letter = (w: string) => Array.from(w)[0].toLocaleUpperCase();
  if (!words.length) return "?";
  return words.length === 1 ? letter(words[0]) : letter(words[0]) + letter(words[words.length - 1]);
}

/** Background for the initials; the same e-mail always gets the same colour. */
export function avatarColor(email: string): string {
  let h = 2166136261; // FNV-1a
  for (const ch of norm(email)) h = Math.imul(h ^ ch.codePointAt(0)!, 16777619) >>> 0;
  return FALLBACK[h % FALLBACK.length];
}

/** Black or white, whichever reads better on `bg`. */
export const initialsInk = (bg: string) => (contrast("#ffffff", bg) >= contrast("#000000", bg) ? "#ffffff" : "#000000");

type Entry = ImageBitmap | "none" | "loading";

/** Lazy per-author photo cache. `get` never waits: it returns the bitmap if known and starts one load otherwise. */
export class AvatarStore {
  private entries = new Map<string, Entry>();
  private queue: { key: string; sha: string }[] = [];
  private live = 0;

  constructor(
    private fetch: (email: string, sha: string) => Promise<ArrayBuffer>,
    private decode: (bytes: ArrayBuffer) => Promise<ImageBitmap>,
    private onChange: () => void,
    private limit = 4,
  ) {}

  /** `sha` is a commit by this author; the forge APIs resolve the real account from it. */
  get(email: string, sha = ""): ImageBitmap | undefined {
    const key = norm(email);
    if (!key) return undefined;
    const e = this.entries.get(key);
    if (e === undefined) {
      this.entries.set(key, "loading");
      this.queue.push({ key, sha });
      this.pump();
      return undefined;
    }
    return typeof e === "string" ? undefined : e;
  }

  private pump() {
    while (this.live < this.limit && this.queue.length) {
      const { key, sha } = this.queue.shift()!;
      this.live++;
      this.load(key, sha).finally(() => { this.live--; this.pump(); });
    }
  }

  private async load(key: string, sha: string) {
    try {
      const bytes = await this.fetch(key, sha);
      if (!bytes.byteLength) return void this.entries.set(key, "none");
      this.entries.set(key, await this.decode(bytes));
      this.onChange();
    } catch {
      // Offline or undecodable: initials stay; the Rust cache retries on the next run.
      this.entries.set(key, "none");
    }
  }
}
