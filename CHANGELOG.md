# Changelog

All notable changes to Git Atlas are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). The first release (the MVP) has not been cut yet;
everything below is in `Unreleased` until then (version `0.1.0` in the sources).

## [Unreleased]

### Phase 5: MVP release

#### Added
- Brazilian Portuguese interface, with a language picker in the toolbar (**Follow system**,
  **English**, **Português (Brasil)**). The choice switches live and is remembered; dates and
  numbers follow the chosen language.
- Plain-language messages and fixes for errors that used to show raw text (Git not found, folder
  not a repository, read failures, Git Atlas's own files not writable, closed repository).
- Keyboard access to every context menu: the Menu key or Shift+F10 opens the menu of the
  selected commit, branch, tag, stash or shared stash.
- Screen reader support for the commit graph: each visible commit is read with its branches and
  tags, message, author, date and SHA.
- Dialogs keep the keyboard inside until they are closed, and focus returns to where it was.
- Desktop entry and AppStream metadata (`dev.atlasgit.app`), app icons in every size.
- Packages: .deb (`atlas-git`, depends on Git 2.20 or newer), AppImage, and a Flatpak bundle with
  its own Git and the libsecret credential helper. Build scripts `scripts/package.sh` and
  `scripts/package-flatpak.sh`, and a release workflow that builds all three.
- User guide (`docs/user/`), release checklist (`docs/release/checklist.md`) and this changelog.

#### Fixed
- The graph's keyboard focus ring is visible again.
- Short commit hashes on the selected graph row have enough contrast in Atlas Dark.
- After an operation, keyboard focus stays on the list you were using instead of jumping to the
  graph.

### Shared stash

#### Added
- Share a local stash through a Git remote, without a branch: teammates see it in the
  **Shared stash** tab, apply it or create a branch from it.
- Update, rename and unshare your own shared stashes; recently unshared ones can be shared again.
- Signed stashes are marked as verified; marks in the graph show where shared stashes were made.
- Choose when shared stashes are fetched (**When opened** or **Always**), per repository.
- Hosts that refuse custom refs are detected once and explained.

### Phase 4: Remotes and rebase

#### Added
- **Fetch**, **Pull** and **Push** in the toolbar, with ahead/behind counts and force push with
  lease after a rejected push.
- Credentials through Git's own helpers and the SSH agent; questions from Git or SSH (passwords,
  passphrases, host keys) appear in an Git Atlas dialog.
- Rebase and an interactive rebase editor (pick, reword, squash, fixup, drop, reorder).
- Cherry-pick, revert, reset (soft, mixed, hard) and tags (create, delete, push) from the graph
  and the sidebar; all of them can be undone except pushing.

### Phase 3: Branching and merge

#### Added
- Create, rename, delete and check out branches; detached checkout of tags, remote branches and
  commits.
- Merge with a preview of the result and the files that will conflict; drag a branch onto the
  current one to merge it.
- Conflict handling: keep ours, take theirs, or a three-pane conflict editor; abort a merge,
  rebase, cherry-pick or revert.
- Stash (including untracked files), apply, pop and drop; stashes appear on their commit in the
  graph.
- Branch and tag filter in the sidebar.

### Phase 2: Operation engine

#### Added
- Stage and unstage files and single hunks, commit, amend, discard.
- Undo and Redo for commits, amends, discards and, later, every local history operation, backed
  by an operation journal and snapshots.
- Operation journal window with the exact Git commands, and opt-in cleanup of old entries.
- The view refreshes by itself when files or the repository change outside Git Atlas.

### Phase 1: Read-only viewer

#### Added
- Open one or more repositories in tabs and browse the full commit graph; a graph cache makes
  large repositories open quickly.
- Commit details and file diffs.
- Atlas Dark and Atlas Light themes, following the system by default.
- Repositories open read-only when Git is older than 2.20.

### Interface additions between phases

#### Added
- Resizable panels and graph columns, remembered between sessions.
- Changed files as a path list or a folder tree; unified and split diffs; full-screen diff.
- `scripts/install-shortcut.sh`: a menu entry with the app icon for a source checkout.
