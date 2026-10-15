---
status: draft
feature: Shared Stash
phase: 4 addendum
refs: SDD §11, PRD criterion 14
date: 2026-10-15
---

# Shared Stash — feature spec

> Draft written from the owner decisions recorded in `docs/HANDOFF.md`. The SDD/PRD live in the
> Obsidian vault (not in this repo); reconcile §11 against this file before implementation.

## 1. Problem

A local stash (`refs/stash`) never leaves the machine. Teams want to hand someone a set of
uncommitted changes ("try my WIP", "here is the fix, don't merge it yet") without creating a
branch on the shared remote. Shared Stash publishes a stash to the repository's remote so other
AtlasGit users of the same repo can see it and apply it.

## 2. Owner decisions (fixed)

| Decision | Value |
|---|---|
| Who may update / delete a shared stash | **Only its author** |
| Expiration | **None** — lives until the author deletes it |
| What consumers can do | **Apply** or **create a branch from it**. No pop, no edit, no delete |
| Offline export/import (bundle/file) | **Later** — out of scope here |

## 3. Goals / non-goals

**Goals**
- Share a local stash with one click; list, apply and branch from teammates' shared stashes.
- Use only plain Git on the remote (works with any host: GitHub, GitLab, Gitea, bare SSH). No server component, no account.
- Same guarantees as the rest of the engine: locks, journal, undo where possible, Git >= 2.20.

**Non-goals**
- Server-side enforcement of permissions (impossible with plain Git; see §7).
- Export/import to file, expiration, comments/reviews, notifications.
- Sharing with people outside the remote (no third-party service).

## 4. Storage model

Each shared stash is one Git ref on the remote, in a namespace that hosts don't show as branches:

```
refs/atlasgit/shared-stash/<author-key>/<id>
```

- `<author-key>`: lowercase hex of the first 12 chars of SHA-1 of the normalized author email (`user.email`, trimmed, lowercased). Avoids leaking emails in ref names and illegal ref characters.
- `<id>`: ULID-like id (reuse `new_id()`), unique per stash.
- The ref points to a **wrapper commit** (created with `git commit-tree`, no working-tree touch):
  - tree: empty tree;
  - parents: `[stash_commit]` (the original `git stash` commit, which already carries index, worktree and optional untracked parents) — so one ref keeps the whole stash reachable and transferable;
  - message: line 1 = title; then a trailer block:
    ```
    AtlasGit-Shared-Stash: 1
    AtlasGit-Author-Email: <email>
    AtlasGit-Base: <sha of the commit the stash was taken on>
    AtlasGit-Base-Branch: <branch name or empty>
    AtlasGit-Created: <unix time>
    AtlasGit-Updated: <unix time>
    ```
  - author/committer: from Git config (normal identity).
- Updating a shared stash = new wrapper commit, ref moved with `--force-with-lease=<ref>:<old sha>`.
- Local mirror after fetch: `refs/atlasgit/remotes/<remote>/shared-stash/*` (never touches `refs/stash` or user branches).

## 5. Operations

All new `Operation` variants go through `Engine::run_with_env` (askpass env for network), journal entries for local effects, network on `spawn_blocking` in the shell.

| Operation | Who | Git commands (sketch) | Journal / undo |
|---|---|---|---|
| `SharedStashList` (read) | anyone | `git for-each-ref refs/atlasgit/remotes/<r>/shared-stash` + parse trailers | — |
| `SharedStashFetch` | anyone | `git fetch <r> +refs/atlasgit/shared-stash/*:refs/atlasgit/remotes/<r>/shared-stash/*` (also run as part of normal Fetch/Pull, prune stale) | no |
| `SharedStashPublish { index, title }` | author | `commit-tree` wrapper → `git push <r> <sha>:refs/atlasgit/shared-stash/<key>/<id>` | journaled; undo = delete (author) |
| `SharedStashUpdate { id, index, title }` | **author only** | new wrapper → push with `--force-with-lease` | journaled; undo = push previous sha with lease |
| `SharedStashDelete { id }` | **author only** | `git push <r> --force-with-lease=<ref>:<sha> :<ref>` | journaled; undo = re-push the old sha (kept locally) |
| `SharedStashApply { id }` | anyone | `git stash apply <stash_commit>` (via wrapper's parent); `--index` attempted, falls back without | not undoable (same as local apply); conflicts reuse the conflict banner/3-pane tool |
| `SharedStashBranch { id, name }` | anyone | `git stash branch`-equivalent: `branch <name> <AtlasGit-Base>` + checkout + apply, all-or-nothing | journaled like BranchCreate |

Rules:
- Publish requires a configured `user.email` and a remote (default: upstream of current branch, else `origin`; picker if ambiguous).
- Locks: `Lock::Index` for apply/branch; a new `Lock::Remote` (or existing network lock) for publish/update/delete.
- Base missing locally (consumer hasn't fetched that history): fetch the base first; if still missing, apply is refused with a clear `OpError` ("Base commit not available — fetch the branch it was made on").
- Error codes: `NO_IDENTITY`, `NOT_AUTHOR`, `SHARED_STASH_GONE`, `STALE_SHARED_STASH` (lease failed), `REMOTE_REJECTED` (host blocks custom refs), `BASE_MISSING`.

## 6. UI

- **Left panel → "Shared stashes"** section under Stashes, grouped "Mine" / "Team"; each row: title, author name, base branch, relative time.
- **Graph:** shared stashes shown as a distinct stash glyph on their base commit (toggle in view menu, default on; reuse stash-row rendering).
- **Local stash context menu:** "Share stash…" (title prefilled with stash message, remote picker).
- **Shared stash context menu:**
  - author: Apply, Create branch…, Update from local stash…, Rename, Delete (confirm);
  - others: Apply, Create branch… only (other actions not shown).
- Click a row → diff view of the stash vs its base (reuse diff panel, read-only).
- Fetch/Pull refresh the list; credentials go through the existing askpass bridge.
- All strings via `i18n.ts` (pt-BR comes in Phase 5).

## 7. Security & trust

- "Author-only" is **enforced by the client**, by comparing the trailer `AtlasGit-Author-Email` (and the wrapper commit's committer email) with local `user.email`. Anyone with push access can still rewrite the ref with plain Git; the UI says so in the docs. Signed wrapper commits (if the user has `commit.gpgsign`) are honored and shown as "verified".
- Applying someone else's stash only changes the working tree; never runs hooks or scripts from it.
- No emails in ref names (hashed key). Emails remain visible inside commits, like any commit.
- If the host rejects pushes outside `refs/heads|tags` → `REMOTE_REJECTED` with hint; feature gracefully hidden for that remote.

## 8. Compatibility

- Git >= 2.20: `commit-tree`, `for-each-ref`, `push --force-with-lease`, `stash apply <commit>` all available. Below 2.20 the app is read-only — shared stashes listed but actions disabled.
- Writes through the CLI; `gix` only for reads. No silent writes to `.git` beyond the `refs/atlasgit/remotes/*` mirror created by an explicit fetch.
- Ordinary `git fetch` by non-AtlasGit users does not fetch the namespace, so nothing changes for them.

## 9. Acceptance criteria (PRD criterion 14)

E2E with **one local bare remote and two clones (A, B)**, distinct `user.email`:
1. A stashes changes (incl. an untracked file with `-u`) and shares it → ref exists on the bare remote.
2. B fetches → sees the stash under "Team" with A's name, title, base branch.
3. B applies → B's working tree equals A's stashed changes (tracked + untracked).
4. B creates a branch from it → branch at the base commit, changes applied, nothing else touched.
5. B does not see Update/Delete; calling the engine directly returns `NOT_AUTHOR`.
6. A updates the stash → B fetches and sees the new content; A deletes it → after fetch it disappears for B (pruned).
7. A updates while the remote ref moved (lease) → `STALE_SHARED_STASH`, nothing overwritten.
8. Undo of publish/update/delete works for A; journal entries recorded.
9. CI runs the test on Git 2.20.5 and current Git.

## 10. Implementation plan

1. `ak-core/src/ops/shared_stash.rs`: model + trailer parse/format (unit + property tests for parsing).
2. Operations + error codes + journal/undo; integration tests with temp bare remote (criteria 1–8).
3. Shell: Tauri commands, wire into Fetch/Pull, askpass.
4. UI: `features/stash` section, context menus, graph glyph, diff view; `devMock.ts` minimal additions.
5. Docs: update HANDOFF, user docs note on client-side permissions.

Branch: `feature/shared-stash` from `develop`, PR to `develop`.

## 11. Open questions

1. Hashed author key vs readable slug in ref names?
2. Should normal Fetch/Pull always fetch shared stashes, or only when the panel is open (bandwidth on big teams)?
3. Allow sharing changes directly from WIP (without creating a local stash first)?
4. Multi-remote repos: list shared stashes from all remotes or only the default one?
