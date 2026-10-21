# Settings

Git Atlas has no settings window. The few choices it offers are where you use them.

## Theme

The theme picker in the toolbar offers **Follow system**, **Atlas Dark** and **Atlas Light**.
**Follow system** switches between the two when your desktop switches between light and dark.

Git Atlas starts with **Follow system** every time; a theme picked by hand lasts until you close
the app.

## Language

The language picker (next to the theme picker) offers **Follow system**, **English** and
**Português (Brasil)**.

- **Follow system** goes through your system languages in order and uses the first one that is
  Portuguese (any variant) or English. If there is neither, it uses English.
- The switch is live: no restart. Dates and numbers follow the chosen language.
- Your choice is remembered for your user account.
- In a window narrower than 1400 pixels the picker shows only `EN` or `PT`; the list still
  shows the full names.

Some text stays in English on purpose: Git terms kept in the Portuguese wording (commit, branch,
merge, rebase and others), the raw Git output shown under **Details** in error messages, and
messages that were already on screen when you switched.

## Shared stash fetching

The **Shared stash** tab has a toggle in its header: **When opened** (default) or **Always**.
See [Fetching](shared-stash.md#fetching) for what each does. The toggle applies to the current
repository and is saved in `.git/atlasgit/settings.json`.

There is no window for a default that applies to every repository. If you want one, create
`~/.config/dev.atlasgit.app/settings.json` (Flatpak:
`~/.var/app/dev.atlasgit.app/config/dev.atlasgit.app/settings.json`) with:

```json
{ "sharedStash": { "fetchMode": "always" } }
```

A repository's own setting wins over this default.

The layers button in the same header (**Show shared stashes in the graph**) turns the **shared**
marks in the graph on or off.

## What Git Atlas remembers for you

Without asking, Git Atlas remembers for your user account: the panel sizes, the graph column
widths, which sections of the commit panel are folded, the language, and whether shared stash
marks are shown. Nothing of this is stored in your repositories.

## Where Git Atlas keeps its data

Inside each repository's Git directory (usually `.git`), written only when you run an operation:

| Path | What |
|---|---|
| `.git/atlaskraken/journal.jsonl` | The operation journal |
| `.git/atlaskraken/discarded/` | Backups of discarded files, for Undo |
| `refs/atlaskraken/snapshots/` | Commits kept for Undo and Redo (amend, reset, undone commits) |
| `.git/atlasgit/` and `refs/atlasgit/` | Shared stash data and settings (see [Shared stash](shared-stash.md#where-things-are-stored)) |

`atlaskraken` is Git Atlas's former name; it was kept so data made before the rename still
works. To clean up the journal and its backups, use the [operation journal](committing-and-undo.md#the-operation-journal).

Outside the repositories: a cache of each repository's commit list in `~/.cache/atlasgit/graph/`
(Flatpak: `~/.var/app/dev.atlasgit.app/cache/atlasgit/graph/`; safe to delete, it is rebuilt),
and the shared stash default above.
