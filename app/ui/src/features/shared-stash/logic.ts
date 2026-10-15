// SPDX-License-Identifier: Apache-2.0
// Pure helpers for the Shared stash tab: grouping, formatting, fetch-state reducer.
import type { KeptSharedStash, OpErrorInfo, RemoteStatus, SharedStashInfo } from "../../api";
import { t } from "../../i18n";

export interface RemoteGroup {
  remote: string;
  /** `rejected` when the host refused custom refs (probe), `null` when unknown or fine. */
  probe: RemoteStatus["probe"];
  mine: SharedStashInfo[];
  team: SharedStashInfo[];
  /** The author's own unshared stashes for this remote (Reshare / Discard). */
  kept: KeptSharedStash[];
}

const byNewest = (a: SharedStashInfo, b: SharedStashInfo) => b.updated - a.updated || a.id.localeCompare(b.id);

/**
 * Groups by remote (listing order, then any remote that only has stashes), then Mine / Team.
 * Kept stashes without a known remote go to `fallbackRemote` (the default one) when it is listed.
 */
export function groupStashes(
  stashes: SharedStashInfo[], remotes: RemoteStatus[], kept: KeptSharedStash[], fallbackRemote: string | null = null,
): RemoteGroup[] {
  const names = remotes.map((r) => r.name);
  for (const s of stashes) if (!names.includes(s.remote)) names.push(s.remote);
  const home = (k: KeptSharedStash) => k.remote ?? fallbackRemote ?? names[0] ?? null;
  return names.map((remote) => ({
    remote,
    probe: remotes.find((r) => r.name === remote)?.probe ?? null,
    mine: stashes.filter((s) => s.remote === remote && s.mine).sort(byNewest),
    team: stashes.filter((s) => s.remote === remote && !s.mine).sort(byNewest),
    kept: kept.filter((k) => home(k) === remote).sort((a, b) => b.unshared_at - a.unshared_at),
  }));
}

/** Rows in the order they appear on screen (keyboard navigation and tests). */
export function visibleOrder(groups: RemoteGroup[]): SharedStashInfo[] {
  return groups.flatMap((g) => (g.probe === "rejected" ? [] : [...g.mine, ...g.team]));
}

/** Valid stashes by base commit, for the graph glyph. */
export function basesIndex(stashes: SharedStashInfo[]): Map<string, SharedStashInfo[]> {
  const m = new Map<string, SharedStashInfo[]>();
  for (const s of stashes) if (s.valid) m.set(s.base, [...(m.get(s.base) ?? []), s]);
  return m;
}

export function relativeTime(now: number, ts: number): string {
  const d = Math.max(0, now - ts);
  if (d < 60) return t("time.now");
  if (d < 3600) return t("time.minutes", { n: Math.floor(d / 60) });
  if (d < 86400) return t("time.hours", { n: Math.floor(d / 3600) });
  if (d < 86400 * 30) return t("time.days", { n: Math.floor(d / 86400) });
  return new Date(ts * 1000).toLocaleDateString();
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/** Title offered when sharing: the stash message, single line, at most 120 characters. */
export function defaultTitle(message: string): string {
  const line = message.split(/\r?\n/)[0].trim();
  return [...line].slice(0, 120).join("");
}

export function cleanTitle(title: string): string {
  return defaultTitle(title);
}

/** Readable slug of a Git user name, as shown in the share warning (mirrors the core, display only). */
export function authorSlug(name: string, email = ""): string {
  const fold = (s: string) =>
    s.normalize("NFKD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  const slug = fold(name).slice(0, 40).replace(/-+$/g, "");
  return slug || fold(email.split("@")[0] ?? "").slice(0, 40).replace(/-+$/g, "") || "unknown";
}

export type RowAction = "apply" | "branch" | "update" | "rename" | "unshare";

/** Menu entries for a row: authors can also update, rename and unshare; invalid rows have none. */
export function rowActions(s: SharedStashInfo): RowAction[] {
  if (!s.valid) return [];
  return s.mine ? ["apply", "branch", "update", "rename", "unshare"] : ["apply", "branch"];
}

export interface FetchState {
  loading: boolean;
  /** Last fetch error per remote; cleared when that remote fetches fine. */
  errors: Record<string, OpErrorInfo>;
  /** A failure of the whole call (not tied to one remote). The last list stays visible. */
  failure: OpErrorInfo | null;
  lastFetched: number | null;
}

export type FetchAction =
  | { type: "start" }
  | { type: "done"; now: number; remote: string | null; fetched: { remote: string; error: OpErrorInfo | null }[] }
  | { type: "fail"; error: OpErrorInfo; cancelled: boolean };

export const initialFetch: FetchState = { loading: false, errors: {}, failure: null, lastFetched: null };

export function reduceFetch(s: FetchState, a: FetchAction): FetchState {
  switch (a.type) {
    case "start":
      return { ...s, loading: true, failure: null };
    case "done": {
      const errors = { ...s.errors };
      for (const f of a.fetched) {
        if (f.error) errors[f.remote] = f.error;
        else delete errors[f.remote];
      }
      return { loading: false, errors, failure: null, lastFetched: a.now };
    }
    case "fail":
      // A cancelled prompt is not an error: keep what was shown.
      return { ...s, loading: false, failure: a.cancelled ? null : a.error };
  }
}
