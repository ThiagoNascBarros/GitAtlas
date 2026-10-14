// SPDX-License-Identifier: Apache-2.0
// All UI strings live here (PRD NFR-9). pt-BR is added in Phase 5.
const en = {
  "app.name": "AtlasKraken",
  "open.title": "Open a repository",
  "open.hint": "Choose a folder that contains a Git repository, or start AtlasKraken with a path: atlaskraken /path/to/repo",
  "open.button": "Open repository…",
  "toolbar.readOnly": "Read-only preview (Phase 1)",
  "toolbar.theme": "Theme",
  "theme.system": "Follow system",
  "theme.dark": "Atlas Dark",
  "theme.light": "Atlas Light",
  "col.refs": "Branch / tag",
  "col.graph": "Graph",
  "col.message": "Message",
  "col.author": "Author",
  "col.date": "Date",
  "col.sha": "SHA",
  "graph.label": "Commit graph. Use arrow keys to move between commits.",
  "panel.refs": "References",
  "panel.local": "Local",
  "panel.remote": "Remote",
  "panel.tags": "Tags",
  "details.select": "Select a commit to see its details.",
  "details.parents": "Parents",
  "details.files": "Changed files",
  "diff.label": "Diff of {path}",
  "diff.mode": "Diff layout",
  "diff.unified": "Unified",
  "diff.split": "Split",
  "diff.close": "Close diff",
  "diff.binary": "Binary file — no text diff.",
  "loading": "Loading…",
  "status.commits": "{n} commits",
  "status.git": "Git {v}",
  "error.title": "Could not open the repository",
} as const;

export type Key = keyof typeof en;

export function t(key: Key, vars: Record<string, string | number> = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? `{${k}}`));
}
