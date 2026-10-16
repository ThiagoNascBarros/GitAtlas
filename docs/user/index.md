# AtlasGit user guide

AtlasGit is a desktop Git client for Linux. It shows the commit graph of a repository and lets you
stage, commit, branch, merge, stash, fetch, push and rebase. Every change is made by running the
Git command line, so what AtlasGit does is what Git does, and the operation journal shows the
exact commands.

This guide describes version 0.1.0 (the MVP). Labels are quoted as they appear in English; in
Portuguese the same controls carry the translated text.

## Pages

1. [Install AtlasGit](install.md): .deb, AppImage, Flatpak, or from source.
2. [Getting started](getting-started.md): open a repository and find your way around the window.
3. [History and diffs](history-and-diff.md): browse the graph, read a commit, compare files.
4. [Committing and undo](committing-and-undo.md): stage files and hunks, commit, amend, discard,
   Undo/Redo and the operation journal.
5. [Branches, merges, conflicts and stashes](branches-merge-stash.md).
6. [Remotes, rebase and history operations](remotes-and-rebase.md): fetch, pull, push,
   credentials, rebase, cherry-pick, revert, reset, tags.
7. [Shared stash](shared-stash.md): hand uncommitted changes to a teammate through a remote.
8. [Settings](settings.md): theme, language, shared stash fetching, remembered layout.
9. [Keyboard and accessibility](keyboard-and-accessibility.md): every shortcut, screen readers.
10. [Troubleshooting](troubleshooting.md): error messages and their fixes, read-only mode,
    Flatpak limits.

## Requirements

- Linux with WebKitGTK 4.1 (Ubuntu 22.04 or newer, or a distribution of the same age). The
  Flatpak brings its own runtime.
- Git 2.20 or newer. With an older Git, repositories open read-only (see
  [Troubleshooting](troubleshooting.md#read-only-old-git)). The .deb installs Git for you and the
  Flatpak bundles it; the AppImage uses the Git of your system.

## What AtlasGit will not do behind your back

- It never starts Git maintenance (`git gc`, `git maintenance`) itself. Git's own automatic
  housekeeping still follows your Git settings.
- It writes inside `.git` only when you run an operation (for its journal, snapshots and
  backups; see [Settings](settings.md#where-atlasgit-keeps-its-data)).
- It never sets `safe.directory` for you. A repository owned by another user stays refused until
  you allow it in your Git configuration.
