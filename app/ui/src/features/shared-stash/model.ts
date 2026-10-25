// SPDX-License-Identifier: Apache-2.0
// Per-repository state of the Shared stash tab (list from the local mirrors + fetch status).
import { createSignal } from "solid-js";
import { createStore, produce, reconcile } from "solid-js/store";
import { api, FetchMode, OpErrorInfo, SharedStashListing, UiError } from "../../api";
import { FetchState, initialFetch, reduceFetch } from "./logic";

export interface RepoModel {
  listing: SharedStashListing | null;
  fetch: FetchState;
  /** The user pressed Cancel; the failure that follows is not an error. */
  cancelled: boolean;
  /** Reading the local list failed. */
  listError: UiError | null;
}

const blank = (): RepoModel => ({ listing: null, fetch: { ...initialFetch }, cancelled: false, listError: null });

const [models, setModels] = createStore<Record<number, RepoModel>>({});

export const modelOf = (repoId: number): RepoModel => models[repoId] ?? blank();

const ensure = (repoId: number) => {
  if (!models[repoId]) setModels(repoId, blank());
};

const asInfo = (e: unknown): OpErrorInfo =>
  typeof e === "object" && e && "code" in e ? (e as OpErrorInfo) : { code: "UNKNOWN", message: String(e) };

/** Reads the local mirrors (no network). */
export async function reloadList(repoId: number): Promise<void> {
  ensure(repoId);
  try {
    const listing = await api.sharedStashList(repoId);
    // Reconciled in place: rows keep their identity, so focus and open sections survive a reload.
    setModels(repoId, "listing", reconcile(listing, { key: "id" }));
    setModels(repoId, "listError", null);
  } catch (e) {
    setModels(repoId, "listError", e as UiError);
  }
}

/** Fetches one remote (or all), then re-reads the list. Failures stay inline; the last list stays visible. */
export async function fetchShared(repoId: number, remote: string | null = null): Promise<void> {
  ensure(repoId);
  if (models[repoId].fetch.loading) return;
  setModels(repoId, produce((m) => { m.cancelled = false; m.fetch = reduceFetch(m.fetch, { type: "start" }); }));
  try {
    const out = await api.sharedStashFetch(repoId, remote);
    setModels(repoId, "fetch", (f) => reduceFetch(f, { type: "done", now: Math.floor(Date.now() / 1000), remote, fetched: out.shared_stash?.fetched ?? [] }));
  } catch (e) {
    setModels(repoId, "fetch", (f) => reduceFetch(f, { type: "fail", error: asInfo(e), cancelled: models[repoId].cancelled }));
  }
  await reloadList(repoId);
}

/** Dismisses the credentials prompts; Git then fails with an auth error that is not shown. */
export function cancelFetch(repoId: number): void {
  if (!models[repoId]?.fetch.loading) return;
  setModels(repoId, "cancelled", true);
  void api.sharedStashCancel(repoId);
}

export async function setFetchMode(repoId: number, mode: FetchMode): Promise<void> {
  await api.setSharedStashFetchMode(repoId, "repo", mode);
  await reloadList(repoId);
}

export function forget(repoId: number): void {
  setModels(repoId, undefined as never);
}

const GLYPH_KEY = "gitatlas.sharedStash.glyph";
const readGlyph = () => {
  try { return localStorage.getItem(GLYPH_KEY) !== "off"; } catch { return true; }
};
/** Graph glyph on base commits (view toggle, on by default). */
const [glyph, setGlyphSignal] = createSignal(readGlyph());
export const glyphOn = glyph;
export function setGlyph(on: boolean): void {
  setGlyphSignal(on);
  try { localStorage.setItem(GLYPH_KEY, on ? "on" : "off"); } catch { /* storage unavailable */ }
}
