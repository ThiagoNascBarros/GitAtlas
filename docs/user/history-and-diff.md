# History and diffs

## Browse the graph

The graph shows the history of all your branches, tags and remote branches, newest first (plus
the commits Git Atlas keeps for Undo and shared stashes; see
[Troubleshooting](troubleshooting.md#a-commit-without-a-branch-in-the-graph)). Lines and colours
show which commits belong to which line of work; merge commits join two lines.

- Click a commit, or use the arrow keys, Page Up/Page Down and Home/End, to select it.
- Labels in the **Branch / tag** column name the branches and tags on that commit. The current
  branch has a check mark. A stash appears as a label on the commit it was made on, and a
  **shared** mark shows where a [shared stash](shared-stash.md) was made.
- The text of each row is read as you scroll, so large repositories open quickly.
- Drag the small handles in the header (after **Branch / tag**, before **Author** and before
  **Date**) to change column widths. Git Atlas remembers them.

## Find a branch or tag

In the sidebar (**Branches** view):

- Type in **Filter branches and tags** to narrow the lists. The **Remote** section is folded
  until you filter or open it.
- Click a branch or tag to select its commit in the graph.
- A greyed-out entry is **Not shown in the graph**: its commit is not part of the loaded history.

## Read a commit

The right panel shows the selected commit: short SHA, message, author, date, **Parents** and the
**Changed files**.

- **Path** lists the files with their full path; **Tree** groups them in folders you can fold.
- The letter before each file is its status: `A` added, `M` modified, `D` deleted, `R` renamed.

## Compare a file

Click a changed file to open its diff under the graph.

- **Unified** shows old and new lines in one column; **Split** shows them side by side.
- The full-screen button gives the diff the whole window; Escape brings it back.
- **Close diff** (the `×` button) closes it.
- Binary files show **Binary file — no text diff.**

The same diff view is used for your uncommitted files, where it also lets you stage single
hunks (see [Committing and undo](committing-and-undo.md#stage-part-of-a-file)).
