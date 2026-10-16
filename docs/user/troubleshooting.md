# Troubleshooting

## Reading an error

When an operation fails, a banner at the bottom of the window says what happened and, when
there is one, how to fix it. **Details** shows the raw message from Git, which is often the
fastest way to the cause. **Dismiss** closes the banner.

## Common messages

| Message | What to do |
|---|---|
| **Git does not know your name and email, so it cannot create the commit.** | Run `git config --global user.name "Your Name"` and `git config --global user.email you@example.com`. |
| **Git does not recognise that folder as a repository it can open.** | Pick the folder that contains the repository. If another user owns it, Git refuses it until you run `git config --global --add safe.directory <path>`. AtlasGit never does that for you. |
| **Git could not be found or did not run, so AtlasGit cannot open repositories.** | Install Git 2.20 or newer and make sure `git` is on your `PATH`. The .deb installs it; the Flatpak has its own. |
| **Another operation is still running on this repository.** | Wait for it to finish (for example a slow push) and try again. |
| **The remote refused your credentials.** | Check the user name and password (many hosts want a token instead of the password) or your SSH key. See [Credentials](remotes-and-rebase.md#credentials). |
| **Could not reach the remote.** | Check your network connection and the remote URL. |
| **The remote has commits you do not have yet.** | Pull, then push again. See [When the remote has new commits](remotes-and-rebase.md#when-the-remote-has-new-commits). |
| **This branch does not track a remote branch yet.** | Push the branch once; that sets its upstream. |
| **A merge is already in progress. Finish it or abort it first.** | Resolve and commit, or press **Abort** in the commit panel. |
| **That change no longer exists. The file was modified; the view will refresh.** | The file changed while you were staging a hunk. Try again on the refreshed diff. |
| **Only regular files can be discarded here. Use a terminal for links.** | Discard symbolic links with `git checkout -- <path>` or `git restore <path>` in a terminal. |
| **The branch changed since that operation, so it can no longer be undone safely.** | Something newer changed the branch. Undo the newer operations first, or leave it. |
| **Those files changed after being discarded; restoring them would overwrite newer work.** | You edited the file after discarding it. Save your new work elsewhere first if you want the old changes back. |
| **That operation cannot be undone.** | The newest journal entry is a network or shared stash operation. See [Undo and Redo](committing-and-undo.md#undo-and-redo). |
| **AtlasGit could not write one of its own files (operation journal, backups or settings).** | Check that the disk is not full and that you can write to the repository's `.git` folder. |
| **Git could not complete the operation.** | Open **Details**: it shows Git's own message. |

## Read-only (old Git)

With Git older than 2.20, repositories open read-only. The toolbar shows
**Read-only (old Git)** and the status bar says `Git older than 2.20: read-only`. You can browse
history and diffs, but every operation is disabled or refused with
**This repository is read-only because your Git is older than 2.20.** In the **Shared stash**
tab, fetching and every action are disabled.

Fix: install Git 2.20 or newer (the status bar shows the version AtlasGit found), or use the
Flatpak, which brings its own Git.

## The view does not refresh by itself

The banner **Auto-refresh is off for this repository (file watching failed); focus the window to refresh.**
means AtlasGit could not watch the folder, usually because the system limit on file watches
(inotify) is reached. Switching to another window and back refreshes the view. To raise the
limit:

```bash
echo fs.inotify.max_user_watches=524288 | sudo tee /etc/sysctl.d/90-inotify.conf
sudo sysctl --system
```

Then reopen the repository.

## A commit without a branch in the graph

The graph includes commits that only AtlasGit's own refs point to: commits kept so Undo and Redo
can bring them back (after Undo of a commit, an amend or a reset) and fetched shared stashes.
They show as rows without a branch label. They live under `refs/atlaskraken/snapshots/` (list
them with `git for-each-ref refs/atlaskraken/`). Forgetting old entries in the
[operation journal](committing-and-undo.md#the-operation-journal) deletes their main snapshot;
the refs ending in `-redo` or `-wip` stay until you delete them with `git update-ref -d <ref>`.

## Shared stash marks missing in the graph

The **shared** mark is drawn only on rows that are loaded. Scroll to the commit and the mark
appears. Also check that the layers button in the **Shared stash** tab is on.

## Flatpak limits

The Flatpak runs AtlasGit and its own Git 2.56.0 in a sandbox. It has not been tested by hand
yet; the points below come from its manifest and the GNOME runtime and are to be verified.

- System Git configuration is not visible. The sandbox does not see `/etc/gitconfig`, so
  system-wide settings (credential helper, proxy, `safe.directory`…) do not apply. Put what you
  need in `~/.gitconfig`, which is visible.
- Credential helpers set by absolute path do not run. A setting such as
  `credential.helper=/usr/lib/git-core/git-credential-libsecret` points to a file that does not
  exist in the sandbox. Git then asks for the password, and AtlasGit shows the question. The
  Flatpak bundles the libsecret helper under the short name `libsecret`, so
  `credential.helper=libsecret` works inside it, and `store` and `cache` work too. Your host Git
  only accepts the short name if a `git-credential-libsecret` is on its `PATH`.
- Repositories outside your home folder are not visible. Allow a folder with:

  ```bash
  flatpak override --user --filesystem=/path/to/folder dev.atlasgit.app
  ```

- SSH uses the SSH agent of your session and your `~/.ssh` folder (keys, `known_hosts`,
  `config`).
- GPG signing uses the gpg-agent of your session through its socket, with your `~/.gnupg`.
- Git hooks run inside the sandbox. A hook that needs a program the GNOME runtime does not have
  fails.
- The status bar shows the Git version in use (2.56.0 in the Flatpak).

## Getting more information

- Start AtlasGit from a terminal (`atlasgit /path/to/repo`) to see its messages.
- When started with `scripts/install-shortcut.sh`, its output goes to
  `~/.cache/atlasgit/launch.log`.
- The [operation journal](committing-and-undo.md#the-operation-journal) shows every Git command
  AtlasGit ran.
