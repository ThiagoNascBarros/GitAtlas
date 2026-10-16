# Branches, merges, conflicts and stashes

Branch and tag actions live in a menu: right-click a branch or tag in the sidebar, or select it
with Tab and press the Menu key or Shift+F10.

## Create a branch

- **New branch** in the toolbar creates a branch at the current commit.
- **New branch here…** in the menu of a branch, tag or remote branch creates one at that ref.
- **New branch here…** in the menu of a commit in the graph creates one at that commit.

Type the name and press **OK**. The new branch is not checked out: check it out next (below).

## Check out

- Double-click a local branch, or choose **Check out** in its menu.
- Tags and remote branches offer **Check out (detached)**. You are then on a detached HEAD: the
  commit panel says **detached HEAD**. To work there, create a branch with **New branch here…**
  and check it out.
- In the graph, **Check out this commit (detached)** does the same for any commit.

Git refuses a checkout that would overwrite your uncommitted changes; commit or stash them first.
**Undo** goes back to the branch you were on.

## Rename and delete

- **Rename…** changes the name of a local branch.
- **Delete** removes a local branch. You cannot delete the branch you are on. If the branch has
  commits that are not merged anywhere else, AtlasGit stops; choose
  **Delete anyway (Undo brings it back)** if you are sure.

## Merge

1. Make sure the branch you want to merge into is checked out.
2. Open the menu of the other branch and choose **Merge into current branch…**, or drag that
   branch in the sidebar and drop it on the current branch (the one with the check mark).
3. The dialog predicts the result before anything changes:
   - **Already merged; nothing to do.**
   - **Fast-forward: no merge commit needed.**
   - **No conflicts expected.**
   - **These files will conflict:** followed by the files.
   - **Conflicts cannot be predicted with this Git version.**
4. Press **Merge**, or **Cancel**.

**Undo** takes the branch back to where it was before the merge.

## Resolve conflicts

When a merge (or a pull, rebase, cherry-pick or revert) stops on conflicts, the commit panel
shows a banner and a **Conflicted files** section. A stash that does not apply cleanly lists its
files there too, without a banner (Git keeps the stash in that case). For each file:

- **Keep ours** takes the version of your current branch.
- **Take theirs** takes the incoming version.
- **Edit…** opens the conflict editor: the two versions on top (**Ours (current branch)** and
  **Theirs (incoming)**), the **Result** below. For each conflict choose **Use ours**,
  **Use theirs** or **Use both**, or edit the result by hand. **Mark resolved** is enabled once
  no conflict is left.

When all files are resolved, commit as usual. During a rebase, press **Continue** in the banner
instead. **Abort** in the banner gives up and puts everything back as it was before the
merge, rebase, cherry-pick or revert started.

## Stash your changes

- **Stash** in the toolbar puts all local changes, including untracked files, aside. You can give
  the stash a message; press **OK**. **Undo** brings the changes back.
- Your stashes are listed under **Stashes** in the sidebar, and each one appears as a label on
  the commit it was made on.
  - **Apply** puts the changes back and keeps the stash.
  - **Pop** puts them back and removes the stash.
  - **Drop** deletes the stash; press it twice (**Drop? (click again)**). **Undo** brings it
    back.
- The menu of a stash (right-click the message, or Tab to it and press Enter, the Menu key or
  Shift+F10) offers **Share stash…**, **Apply** and **Apply and remove**.

To hand a stash to a teammate, see [Shared stash](shared-stash.md).
