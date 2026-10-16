# Remotes, rebase and history operations

## Fetch, pull and push

The toolbar buttons work on the current repository:

- **Fetch** downloads new commits and branches from all remotes. Remote branches that were
  deleted on the remote disappear from the sidebar.
- **Pull** fetches and merges the upstream branch into the current one (always a merge, never a
  rebase). The button shows how many commits you are behind. If the merge stops on conflicts,
  resolve them as in [Resolve conflicts](branches-merge-stash.md#resolve-conflicts).
- **Push** uploads the current branch. The button shows how many commits you are ahead. The
  first push of a branch sets its upstream on `origin` (or on the only remote, if there is one).

A branch must be checked out to pull or push. **Pull** needs an upstream: push the branch once
first, or you get **This branch does not track a remote branch yet.**

### When the remote has new commits

If **Push** is refused with **The remote has commits you do not have yet.**, pull first and
push again. To replace the remote branch with yours instead, press
**Force push (with lease)** in the error banner, then **Click again: replaces the remote branch**.
"With lease" means Git refuses if the remote branch moved since your last fetch, so you do not
overwrite work you have not seen.

Fetch and push cannot be undone (see [Undo and Redo](committing-and-undo.md#undo-and-redo)).

## Credentials

AtlasGit runs your Git, so your existing setup keeps working: credential helpers
(`credential.helper`, for example `libsecret`, `store` or `cache`), the SSH agent and your
`~/.ssh/config`.

When Git or SSH needs to ask you something (a user name, a password or token, an SSH key
passphrase, whether to trust a new host key), AtlasGit shows the question in a dialog. Passwords
are hidden while you type. **OK** answers, **Cancel** stops the operation.

If the remote rejects what you typed, you see **The remote refused your credentials.** If it
cannot be reached: **Could not reach the remote.**

In the Flatpak some helpers cannot run; see [Flatpak limits](troubleshooting.md#flatpak-limits).

## Tags

- **New tag here…** in the menu of a commit creates a lightweight tag on it.
- In the menu of a tag:
  - **Push tag** sends it to `origin` (or the only remote).
  - **Delete tag** deletes your local tag. A tag you already pushed stays on the remote.
  - **Check out (detached)** and **New branch here…** work as for branches.

Creating and deleting tags can be undone.

## Rebase

- **Rebase current branch onto this** (menu of a branch or remote branch) replays the commits of
  your current branch on top of that branch.
- **Interactive rebase onto this…** (menu of a branch or remote branch) and
  **Interactive rebase of the commits after this…** (menu of a commit in the graph) open the
  rebase editor first.

In the editor, each commit has an action:

- **Pick**: keep it.
- **Reword**: keep it with a new message; type it in the **New message** field.
- **Squash into previous**: combine it with the commit above.
- **Fixup (discard message)**: combine it with the commit above and drop its message.
- **Drop**: leave it out.

The arrow buttons (**Move up**, **Move down**) change the order. The first commit cannot be
squashed. Press **Start rebase**, or **Cancel**.

If a rebase stops on conflicts, resolve the files and press **Continue** in the banner, or
**Abort**. **Undo** puts the branch back where it was before the rebase.

The editor does not handle merge commits: when the range contains one, you see
**Those commits include merges; use a terminal for this rebase.**

## Cherry-pick, revert and reset

All three are in the menu of a commit in the graph (right-click, or select it and press the Menu
key or Shift+F10):

- **Cherry-pick onto current branch** copies that commit on top of your current branch.
- **Revert this commit** adds a new commit that undoes its changes.
- Reset moves your current branch to that commit, in three variants:
  - **Reset current branch here (soft: keep changes staged)**
  - **Reset current branch here (mixed: keep changes unstaged)**
  - **Reset current branch here (hard: discard changes)**

All three can be undone. Before a hard reset AtlasGit saves your uncommitted changes to tracked
files, so **Undo** brings both the commits and those changes back (untracked files are not
touched by a reset).

The same menu also has **New branch here…**, **New tag here…**,
**Check out this commit (detached)** and **Copy SHA**.
