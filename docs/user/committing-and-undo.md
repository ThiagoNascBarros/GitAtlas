# Committing and undo

## See your changes

Click **Uncommitted changes**, the first row of the graph (or press Home in the graph). The
right panel becomes **Commit changes**:

- **Unstaged files**: changed but not in the next commit yet.
- **Staged files**: these go into the next commit.
- **Conflicted files**: only while a merge, rebase, cherry-pick or revert stopped on conflicts
  (see [Branches, merges, conflicts and stashes](branches-merge-stash.md#resolve-conflicts)).

Click a file to see its diff. The arrow before a section name folds it; AtlasGit remembers which
sections are folded.

## Stage and unstage

- **Stage** / **Unstage** next to a file moves that file.
- **Stage all** / **Unstage all** moves the whole section.

### Stage part of a file

Open the diff of an unstaged file. Each hunk (block of changes) has a **Stage hunk** button in
its header. In the diff of a staged file the button is **Unstage hunk**. A new, untracked file
has no hunk buttons (it shows **New file (untracked)**): stage the whole file.

## Discard changes

**Discard** throws away the changes of one unstaged file. Press it twice: the first press turns
it into **Discard? (click again)**.

AtlasGit keeps a backup, so **Undo** brings the changes back, as long as you did not edit the
file again in the meantime. Only regular files can be discarded here; for symbolic links use a
terminal.

## Commit

1. Stage what you want in the commit.
2. Write a **Summary** (one line; the counter changes colour after 72 characters) and, if you want, a
   **Description**.
3. Press the commit button. It names what will happen, for example "Commit 3 files to main".
   Ctrl+Enter in the **Description** field does the same.

When the button is disabled, the line under it says why: **Stage at least one file to commit.**
or **Write a summary to commit.**

If Git does not know who you are, the commit stops with
**Git does not know your name and email, so it cannot create the commit.** Set them once:

```bash
git config --global user.name "Your Name"
git config --global user.email you@example.com
```

Commits follow your Git configuration, including commit signing (`commit.gpgsign`) and hooks.

## Amend the last commit

Tick **Amend previous commit**. The form fills in the previous message; change it and/or stage
more files, then press **Amend commit**. You can amend the message alone, without staging
anything. **Undo** brings the previous version of the commit back.

## Undo and Redo

- **Undo** (toolbar, or Ctrl+Z outside text fields) reverts the newest operation in the journal
  that is not undone yet.
- **Redo** (toolbar, or Ctrl+Shift+Z) brings back the operation you undid last, as long as you
  did nothing new since.
- After most operations, the message at the bottom of the window has its own **Undo** button for
  that exact operation.

What can be undone: commit, amend, discard, creating/renaming/deleting a branch, checkout,
merge (also the merge made by **Pull**), **Stash** and **Drop**, cherry-pick, revert, reset,
creating/deleting a tag, rebase, and **Create branch…** from a shared stash.

What cannot:

- Staging and unstaging: just do the opposite.
- Applying or popping a stash, resolving conflicts, **Abort**.
- Network operations: **Fetch**, **Push**, **Push tag**, sharing, updating and unsharing shared
  stashes (an unshared stash is kept under **Recently unshared** instead). Undo never changes a
  remote: undoing a commit you already pushed only moves your local branch.

Undo and Redo refuse when the repository moved on since, rather than overwrite newer work. You
then see, for example, **The branch changed since that operation, so it can no longer be undone safely.**
Undo the newer operations first.

## The operation journal

**History** in the toolbar opens the **Operation journal**: every recorded operation, newest
first, with its time, an **undone** mark when it was undone, and the exact Git commands AtlasGit
ran.

The journal and the backups behind Undo stay until you clean them up. Set **Keep last** to a
number and press the "Forget all but the last …" button, then **Forget them? (click again)**.
Forgotten operations can no longer be undone. **Close** closes the journal.
