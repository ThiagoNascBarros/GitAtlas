---
status: draft
feature: Shared Stash
phase: 4 addendum
refs: SDD §11, PRD criterion 14
date: 2026-10-15
revision: 3 (after spec review)
---

# Shared Stash — feature spec

> Written from the owner decisions in `docs/HANDOFF.md`, the owner's answers (2026-10-15) and a
> critical review (§12). The SDD/PRD live in the Obsidian vault; reconcile §11 there with this file.

## 1. Problem

A local stash (`refs/stash`) never leaves the machine. Teams want to hand someone uncommitted
changes ("try my WIP", "here is the fix, don't merge it yet") without creating a branch on the
shared remote. Shared Stash publishes a local stash to a remote so other AtlasGit users of the
same repo can see it and apply it.

## 2. Owner decisions (fixed)

| Decision | Value |
|---|---|
| Who may update / delete | Only its author (client-enforced, see §7) |
| Expiration | None — lives until the author unshares it |
| Consumers | Apply or create a branch. No pop, edit, delete |
| Source | Only an existing **local stash** (not WIP) |
| Ref naming | **Readable name slug** of the author |
| Fetch | User setting: `panel` (default) or `always` |
| Remotes | All remotes, in a new sidebar tab "Shared stash" |
| Offline export/import | Later |

## 3. Goals / non-goals

**Goals:** share a local stash in one action; list, apply and branch from teammates' shared
stashes across all remotes; plain Git only (no server, no account); same engine guarantees
(locks, journal, Git >= 2.20).

**Non-goals:** server-side permissions; export/import to file; expiration; comments/notifications;
sharing straight from WIP; undo of network operations (v1, see §5.3).

## 4. Storage model

One ref per shared stash on the remote:

```
refs/atlasgit/shared-stash/<author-slug>/<id>
```

- `<author-slug>`: readable slug of `user.name` — lowercase ASCII, accents stripped, spaces and
  invalid ref characters → `-`, collapsed, max 40 chars; email local-part if the name is empty.
  Example `thiago-barros`. **Display only.** It is fixed at publish time and public to anyone with
  read access (the share dialog says so). Ownership and "Mine/Team" grouping use the email
  trailer, never the slug, so renaming `user.name` or two people with the same name break nothing.
- `<id>`: `new_id()`; unique, keeps refs distinct even with equal slugs.
- The ref points to a **wrapper commit** made with `git commit-tree` (no working-tree touch):
  - tree: the empty tree; parent: the original stash commit (which carries base, index and the
    optional untracked parent), so one ref transfers the whole stash;
  - message: line 1 = title (max 120 chars, single line), then trailers:
    ```
    AtlasGit-Shared-Stash: 1
    AtlasGit-Author-Email: <email>
    AtlasGit-Base: <sha the stash was taken on>
    AtlasGit-Base-Branch: <branch or empty>
    AtlasGit-Created: <unix time>
    AtlasGit-Updated: <unix time>
    ```
  - signed if the user has `commit.gpgsign`.
- Local mirror (written only by an explicit fetch): `refs/atlasgit/remotes/<remote>/shared-stash/<slug>/<id>`.
- Local keep refs for the author's own unshared stashes: `refs/atlasgit/keep/<id>` (§5.3).

### 4.1 Validation (every read)

A mirror ref is shown only if the wrapper parses (trailer `AtlasGit-Shared-Stash: 1`, required
trailers present) **and** its parent is stash-shaped: a merge commit whose `^1` equals
`AtlasGit-Base` and whose `^2` exists (`^3` optional = untracked). Otherwise it is listed as
"invalid" (greyed, no actions) and apply/branch return `INVALID_SHARED_STASH`.

## 5. Operations

New `Operation` variants through `Engine::run_with_env` (askpass env), network on
`spawn_blocking` in the shell. Local stashes are addressed by **sha**, never by index.

| Operation | Who | Git (sketch) | Journal |
|---|---|---|---|
| `SharedStashList` (read) | anyone | `for-each-ref refs/atlasgit/remotes/*/shared-stash` + parse/validate | — |
| `SharedStashFetch { remote: Option }` | anyone | `fetch --prune <r> +refs/atlasgit/shared-stash/*:refs/atlasgit/remotes/<r>/shared-stash/*`; all remotes when None, one failure doesn't stop the others | no |
| `SharedStashPublish { stash_sha, title, remote }` | author | probe (§5.2) → size check → `commit-tree` → `push <r> <sha>:<ref>` → fetch that ref into mirror | entry (not undoable) |
| `SharedStashUpdate { id, remote, stash_sha?, title? }` | author | fresh fetch of that ref → new wrapper → `push --force-with-lease=<ref>:<mirror sha> <r> <new>:<ref>`. Covers "rename" (title only) | entry (not undoable) |
| `SharedStashUnshare { id, remote }` | author | fresh fetch → write `refs/atlasgit/keep/<id>` → `push --force-with-lease=<ref>:<sha> <r> :<ref>` | entry (not undoable) |
| `SharedStashReshare { id, remote }` | author | push `refs/atlasgit/keep/<id>` back to its ref (create-only lease), then delete keep ref | entry |
| `SharedStashApply { id, remote }` | anyone | validate → `stash apply --index <parent>`; on index failure retry without `--index` and report "applied without staged state" | entry, not undoable (like local apply) |
| `SharedStashBranch { id, remote, name }` | anyone | validate → ensure base present → `branch <name> <base>` → checkout → apply; rollback branch on failure | entry like BranchCreate (undoable) |

### 5.1 Rules

- **Fetch mode** `sharedStash.fetchMode` (per repo, global default):
  - `panel` (default): fetch only when the "Shared stash" tab is opened, on its refresh button,
    and on Fetch/Pull while it is open;
  - `always`: also on every Fetch/Pull with the tab closed.
- **Lease:** the expected sha is always read from the mirror ref right after a fetch of that
  single ref in the same operation.
- **Author check** (update/unshare/reshare): trailer email == local `user.email` (normalized),
  else `NOT_AUTHOR`.
- **Publish remote:** default upstream remote of the current branch, else `origin`, else picker.
- **Dirty tree on apply/branch:** apply proceeds like `git stash apply` (Git refuses on
  overlapping files → `DIRTY_TREE` with hint "commit or stash first"); conflicts open the existing
  conflict banner / 3-pane tool.
- **Base missing:** fetch `AtlasGit-Base-Branch` from the same remote; if still missing →
  `BASE_MISSING`. Apply without the base is still allowed (stash apply only needs the stash
  commit, which is present); only Branch requires it.
- **Size:** warn at publish when the stash's blobs not on the remote exceed 10 MB (estimated with
  `rev-list --objects <stash> --not --remotes=<r>` + `cat-file --batch-check`). Warning only,
  never blocks.
- **Locks:** `Lock::Index` for apply/branch; new `Lock::Remote` (per repo, file-based like the
  others) for publish/update/unshare/reshare/fetch.
- **Errors:** `NO_IDENTITY`, `NO_REMOTE`, `NOT_AUTHOR`, `SHARED_STASH_GONE`,
  `STALE_SHARED_STASH`, `REMOTE_REJECTED`, `INVALID_SHARED_STASH`, `BASE_MISSING`, `DIRTY_TREE`,
  `STASH_NOT_FOUND`.

### 5.2 Remote capability probe

On the first publish to a remote, push a throwaway ref `refs/atlasgit/probe/<id>` then delete it.
Result (`ok` / `rejected`) cached in `.git/atlasgit/remotes.json` (written only by this explicit
action). `rejected` → `REMOTE_REJECTED` with a hint (host blocks custom refs: Gerrit, Bitbucket
Server, Azure DevOps, some gateways), and the tab shows a notice for that remote. "Retry check"
in the tab clears the cache.

### 5.3 Undo

Network operations are **not** undoable in v1 (a push can fail, others may already have fetched).
They are journaled for history only. Reversal is explicit: Unshare keeps the wrapper in
`refs/atlasgit/keep/<id>` (protected from gc) and the tab shows "Recently unshared" (author only)
with Reshare and Discard (deletes the keep ref).

## 6. UI

- **Sidebar tab "Shared stash"** next to "Local" and "Remote": grouped by remote, then "Mine" /
  "Team". Row: title, author name, verified/claimed badge, base branch, relative time; invalid
  rows greyed. Header: refresh, fetch-mode toggle (`panel` / `always`), "last fetched" time.
- **States:** loading (spinner, cancel), empty ("No shared stashes on <remote>"), offline/error
  per remote (inline, others still shown), host rejected (notice + Retry check).
- Auth prompts started from the tab always show a spinner with Cancel; a cancelled prompt leaves
  the last list visible.
- **Graph:** shared-stash glyph on the base commit (view toggle, default on).
- **Local stash menu:** "Share stash…" → dialog: title (prefilled), remote picker, and a warning:
  *"Everyone with read access to <remote> will see these changes, including untracked files, and
  your name `<slug>`. Clones that already fetched them keep a copy."* + size warning if any.
- **Shared stash menu** — author: Apply, Create branch…, Update from local stash…, Rename,
  Unshare (confirm). Others: Apply, Create branch….
- Row click → read-only diff of stash vs base (existing diff panel).
- Keyboard: tab reachable and rows navigable like the Local/Remote tabs. Strings via `i18n.ts`.

## 7. Security & trust

- "Author-only" is **declarative**: anyone with push access can rewrite the refs with plain Git,
  and the email trailer can be forged. UI and docs never promise protection.
- Badge: **verified** when the wrapper has a valid signature matching the trailer email;
  otherwise **claimed author**.
- Apply only changes the working tree; nothing from the stash is executed.
- Invalid or malformed refs are never acted on (§4.1).

## 8. Compatibility

- Git >= 2.20: `commit-tree`, `for-each-ref`, `push --force-with-lease=<ref>:<sha>`,
  `fetch --prune` with custom refspecs, `stash apply [--index] <commit>`. Below 2.20 the tab is
  read-only.
- Writes via the CLI; `gix` only for reads. Writes to `.git` only from explicit user actions
  (mirror refs via fetch, keep refs via unshare, probe cache via publish).
- A plain `git fetch` by non-AtlasGit users doesn't fetch the namespace.

## 9. Acceptance criteria (PRD criterion 14)

E2E with local bare remotes R1, R2 and clones A, B (different `user.name`/`user.email`):

1. A stashes (incl. untracked, `-u`) and shares to R1 → ref `refs/atlasgit/shared-stash/<A-slug>/<id>` exists on R1.
2. B, mode `panel`, tab closed: Fetch does **not** download it. Opening the tab does; B sees it under "Team" with A's name, title, base branch. Mode `always`: plain Fetch downloads it.
3. A shares another stash to R2 → B sees both, grouped by remote. R2 made unreachable → R1 still listed, R2 shows an error.
4. B applies → working tree equals A's stash (tracked + untracked). Dirty overlapping file → `DIRTY_TREE`, nothing changed.
5. B creates a branch → at base, changes applied, nothing else touched.
6. B: Update/Unshare not shown; engine call returns `NOT_AUTHOR`. B with forged trailer email pushing via plain Git → shown as "claimed author" (not verified).
7. A updates (content and rename) → B sees new content after fetch. Remote ref moved meanwhile → `STALE_SHARED_STASH`, nothing overwritten.
8. A unshares → disappears for B after fetch (pruned); A sees it in "Recently unshared" and Reshare restores it.
9. A ref pointing at a non-stash commit → listed invalid, apply returns `INVALID_SHARED_STASH`.
10. Bare remote with a hook rejecting `refs/atlasgit/*` → probe gives `REMOTE_REJECTED`, cached, notice shown.
11. Two authors with the same `user.name` → distinct refs, correct "Mine" for each. A changes `user.name` → old stashes still "Mine".
12. Publish with > 10 MB new blobs shows the warning and still publishes.
13. CI runs on Git 2.20.5 and current Git.

## 10. Plan

See `docs/specs/shared-stash-plan.md`. Branch `feature/shared-stash` from `develop`, PR to `develop`.

## 11. Resolved decisions

Owner (2026-10-15): readable slug; fetch mode `panel` default + `always`; local stash only;
all remotes in a "Shared stash" sidebar tab.

After review (author, 2026-10-15): keep the wrapper commit (metadata must be updatable without
touching the stash's own message format); keep readable slug, with public-name warning and
email-based grouping; network ops not undoable in v1 — Unshare/Reshare with keep refs instead;
validate stash shape before acting; size is a warning only; capability probe per remote;
declarative ownership with verified/claimed badge.

## 12. Review log

Critical review by a subagent on 2026-10-15. Accepted: validation, undo, sensitive-data
warning, claimed/verified, sha addressing, lease source, dirty tree, `Lock::Remote`, probe,
UI states, extra criteria. Rejected: dropping the wrapper commit; "hosts may gc custom refs"
(refs keep objects reachable); hard size limit.

## 13. Implementation notes (M1-M4)

Choices made while implementing, where the spec was silent, ambiguous or not feasible with Git as it behaves.

- **Apply needs the base commit.** `git stash apply` merges against the stash's first parent, so Apply cannot run without the base, contrary to §5.1. Apply and Branch both try the base branch once on the same remote, then return `BASE_MISSING`.
- **`DIRTY_TREE` is checked before Git runs.** Git restores the untracked files of a stash even when it then refuses the tracked part, so "nothing changed" (§9.4) needs a check first: if any path the stash touches (tracked, staged or untracked) is already changed in the index or working tree, or an untracked path exists on disk, the apply is refused untouched. Git's own messages are still mapped to `DIRTY_TREE` as a safety net.
- **`--index` only when it can work.** `git stash apply --index` fails when the index already has staged changes, and then unstages them. So `--index` is used only if the stash has staged changes and the index is clean; otherwise the stash is applied without it and `index_dropped` is reported. A failed `--index` run that is not a conflict or overlap also falls back (§5 table).
- **Branch undo.** Recorded as `shared_stash_branch` (undoable). Undo needs HEAD still at the base on the new branch, checks out the previous branch/commit and deletes the new branch; the applied changes stay as ordinary local changes (nothing is discarded). Redo recreates and checks out the branch without applying the stash again.
- **Keep metadata.** The wrapper has neither remote nor slug, so unshare writes `.git/atlasgit/keep.json` (`id -> remote, slug, unshared_at`) next to the keep ref; Reshare falls back to the slug of the wrapper's author name if the entry is gone. Written only by Unshare/Reshare/Discard.
- **Probe.** Pushes an unrelated commit (empty tree, fixed identity) so no user history is uploaded. A host that accepts the push but refuses the delete counts as `rejected` (Unshare would fail there too). Network/auth errors are errors, never cached. The cache entry is keyed by remote name and URL; a URL change invalidates it.
- **Stale detection.** `STALE_SHARED_STASH` covers both Git's client-side `stale info` and the server-side `failed to update ref` seen when the ref moves between the fresh fetch and the push.
- **Publish/update.** Title defaults to the stash message. Update with neither content nor title returns `NOTHING_TO_UPDATE` (new code). Reshare/discard of a missing keep ref returns `STASH_NOT_FOUND`. `Created` is kept on update.
- **Verified badge.** `git verify-commit` succeeds and the signer (`%GS`) contains the trailer email; only checked for wrappers that carry a signature header.
- **Journal.** Publish, update, unshare, reshare, discard-keep and apply are journaled but not undoable (`UNDO_NOT_SUPPORTED`) and do not clear the redo chain; fetch is not journaled. `OpOutcome::id` is `None` for them; extra results are in `OpOutcome::shared_stash`.
- **Listing.** Mirrors of remotes that no longer exist are ignored. `Engine::open` refuses Git < 2.20, so the list is not available there either (§8 says read-only; not implemented).
- **Fetch mode (shell).** Stored as `sharedStash.fetchMode` in `<app config dir>/settings.json` (global) and `<git-dir>/atlasgit/settings.json` (per repo, written only when changed). `shared_stash_tab_open` takes the repo id. A plain Fetch/Pull fetches shared stashes when the effective mode is `always` or the tab is open; its failures are reported per remote and never fail the Fetch/Pull.
- **Cancel.** `shared_stash_cancel` dismisses the credentials prompts of the repository (Git then fails with an auth error). A running Git process is not killed, and `Lock::Remote` is held until it ends.
- **Large stash.** Threshold is 10 MiB of blobs missing from the remote's known branches; best effort, a failure to estimate means no warning.

## 14. Implementation notes (M5-M6, UI)

- **Sidebar tab.** The sidebar has a two-button switch, "Branches" (the Local/Remote/Tags/Stashes sections) and "Shared stash". Opening the tab fetches once and tells the shell (`shared_stash_tab_open`); leaving it, switching repository or closing it reports closed.
- **Row click** selects the shared stash's stash commit in the details panel (files list, then the existing read-only diff). The diff is the stash against its base (first parent); untracked files (`^3`) are not part of it.
- **Share dialog.** No command exposes `user.name`, so the warning shows the slug of the user's earlier shared stash when there is one and otherwise says "your Git user name (as a lowercase slug)". The size warning cannot be computed before publishing (the engine reports it in the publish outcome), so the dialog states the rule and a toast reports the size after publishing.
- **Failed share** closes the dialog and the error shows in the banner (a dialog would cover it).
- **Graph glyph** is drawn on the rows of loaded graph chunks whose sha equals a valid stash's base (no sha-to-row command exists); stashes sharing a base show a count. View toggle (default on) is stored in `localStorage` only.
- **Kept stashes** are listed under the remote recorded in `keep.json`, else the default remote.
- **Per-remote errors and last-fetched time** are UI state (not stored); a cancelled fetch leaves the previous list and shows no error. A rejected remote hides its rows and offers "Retry check" (`shared_stash_probe_reset`).
- **Read-only repositories** do not fetch on tab open and disable the refresh button and the mutating actions.
