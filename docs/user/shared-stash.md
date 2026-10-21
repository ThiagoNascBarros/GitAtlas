# Shared stash

Hand your uncommitted changes to a teammate without creating a branch on the shared remote.
Shared stash publishes one of your local stashes to a Git remote, and anyone using Git Atlas on the
same repository can see it, apply it or start a branch from it. It uses plain Git: no server, no
account, nothing to install on the remote.

## Share a stash

1. Stash your changes (toolbar **Stash**). Only a local stash can be shared, not the working directory.
2. In the sidebar, right-click the stash under **Stashes** (or Tab to it and press Enter, the Menu
   key or Shift+F10) and choose **Share stash…**.
3. Pick a title and the remote, read the notice, and press **Share**.

The stash is then listed in the **Shared stash** tab, under that remote and under **Mine**. Your
local stash stays where it is.

**Before you share, know what leaves your machine.** Everyone with read access to the remote can see
the changes, including untracked files, and your Git user name as a lowercase slug (for example
`thiago-barros`). Clones that already fetched the stash keep a copy even if you unshare it later.
Large new files make sharing slower; if a stash adds a lot of data you are told after it is published.
Sharing is never blocked by size.

## Find and use a teammate's stash

Open the **Shared stash** tab in the sidebar. Stashes are grouped by remote, then **Mine** and
**Team**. Each row shows the title, its age, the author, a shield mark when the stash is verified
(see below) and the branch it was made on.

- Click a row to see what it changes against its base commit, in the usual file list and diff panel (read-only).
- Right-click a row (or press the Menu key / Shift+F10) for actions:
  - **Apply** puts the changes in your working directory, like `git stash apply`. If the stash had
    staged changes, they are staged again when possible; otherwise you are told they were applied unstaged.
    If a file the stash touches is already changed in your working directory, nothing is applied and
    you are asked to commit or stash first.
  - **Create branch…** creates a branch at the commit the stash was made on, checks it out and applies the stash.
    This can be undone with **Undo**.
- Arrow keys, Home and End move between rows; Enter shows the changes.
- A **shared** label on a commit in the graph shows where a shared stash was made. Turn the labels
  on or off with the layers button in the tab header.

Applying never runs anything from the stash; it only changes files.

### Rows you cannot use

A row greyed out and marked **invalid** is a ref in the shared namespace that is not a well-formed
shared stash (somebody pushed something else there, or the data is damaged). It has no actions.

## Your own stashes

For stashes under **Mine** the menu also offers:

- **Update from local stash…**: replace the content with another local stash (and optionally the title).
- **Rename…**: change the title only.
- **Unshare…**: remove it from the remote. It disappears for others after their next fetch.

Unshare is not an undo-able step, so Git Atlas keeps a protected copy under **Recently unshared**
(visible only to you, for the remote it came from). **Reshare** puts it back on the remote,
**Discard** deletes your copy for good.

If someone changed the same shared stash on the remote in the meantime, Update, Rename and Unshare
stop with "changed on the remote" and overwrite nothing. Refresh and try again.

## Ownership is declarative

"Only the author can update or unshare" is enforced by Git Atlas, not by the server. Anyone who can
push to the remote can change or delete any ref with plain Git, and the author recorded in a stash
can be forged. So:

- No mark (screen readers say **claimed**; hovering the author explains it): the author is what
  the stash says. Nothing proves it.
- Shield mark (**verified**): the stash is signed and the signature matches the author's email.
  This needs commit signing (`commit.gpgsign`) on the author's side and a working
  `git verify-commit` on yours.

Use remote permissions if you need real access control, and do not put secrets in a stash.

## Fetching

Shared stashes are not downloaded by a plain `git fetch`: they live in their own namespace
(`refs/atlasgit/shared-stash/…`). Git Atlas fetches them according to the toggle in the tab header:

- **When opened** (default): when you open the tab, press the refresh button, or run Fetch/Pull while the tab is open.
- **Always**: also on every Fetch/Pull, even with the tab closed.

The header shows when the last fetch finished. While fetching you can **Cancel** (for example a
password prompt you do not want to answer); the list you already had stays on screen. A remote that
cannot be reached shows its error inline, with **Retry**, and the other remotes are still shown.

## Host compatibility

The remote must accept pushes of custom refs under `refs/atlasgit/*`. Most hosts that serve plain Git accept them (we have not
checked every host; the check below tells you). Some are known to refuse them: Gerrit, Bitbucket Server, Azure DevOps and some
gateways. The first time you share to a remote, Git Atlas pushes and deletes a throwaway ref to check;
the result is remembered for that remote and URL. On a refusing host the tab shows a notice and
sharing is disabled for it. **Retry check** forgets the result so it runs again on your next share,
for example after the host's admin allowed the refs.

Requirements: Git 2.20 or newer (older Git can open a repository read-only; there the Shared stash tab
cannot fetch and its actions are disabled).

## Where things are stored

- On the remote: `refs/atlasgit/shared-stash/<author-slug>/<id>`, a small commit that points to your stash.
- In your clone, only after an explicit fetch: `refs/atlasgit/remotes/<remote>/shared-stash/…`.
- Your unshared copies: `refs/atlasgit/keep/<id>` and `.git/atlasgit/keep.json`.
- The host check result: `.git/atlasgit/remotes.json`.
- The fetch-mode setting: `.git/atlasgit/settings.json` (this repository) or the app settings (default for all).

Git Atlas writes there only when you share, unshare, reshare, discard, fetch shared stashes or change the setting.
