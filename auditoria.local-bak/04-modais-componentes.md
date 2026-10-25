# Area 4 — Modals and components (audit)

## Limitações
- Static analysis only (code in `app/ui/src`, `crates/ak-core/src/ops`). The UI was NOT run in the Browser pane and no screenshots were taken (`auditoria/screenshots/` untouched); every behavior below is derived from code and marked as such.
- `/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/en/ux-analysis.md` does not exist at that path (read failed); the "raw stderr toast is a flaw" premise is taken from the task brief, not verified in the REA doc.
- Matrix IDs used: UI-004 (context menus), UI-005 (dialogs), UI-006 (error messages), UI-007 (WIP panel), F-021 (abort), F-022 (stash), F-023 (net), F-025 (undo/journal), F-019 (merge). Destructive operations are not individually listed in the matrix.
- Line numbers for `App.tsx` come from `grep -n` on the current tree.

## Destructive-operation confirmation map (verified in code)
| Operation | Confirm? | Names what is lost? | Recoverable? | Evidence |
|---|---|---|---|---|
| Discard file | 2nd click, 3 s timer | No (button says "Discard? (click again)"; path only in the toast afterwards) | Yes, Undo (backup) | `WipPanel.tsx:51-58` |
| Hard reset | none | Label only: "hard: discard changes" | Yes, snapshot + Undo | `CommitMenu.tsx:198,220`; `history.rs:82` |
| Delete branch / force delete | none | Label "Undo brings it back" only on force | Yes, Undo | `RefMenu.tsx:316-317` |
| Delete tag | none | no | Yes (journal `tag_delete`) | `RefMenu.tsx:310`; `branch.rs:12` |
| Stash drop | 2nd click, reset on blur only | no | Yes (journal `stash_drop`) | `StashPanel.tsx:274-278` |
| Force push | 2nd click inside error banner | Yes ("replaces the remote branch"), uses `--force-with-lease` | No (remote) | `App.tsx:607-611`; `remote.rs:116` |
| Unshare shared stash | real dialog | Yes (who loses it, how to recover) | Yes ("Recently unshared") | `SharedStashMenu.tsx` UnshareDialog |
| Discard kept shared stash | 2nd click | no | No | `SharedStashPanel.tsx:132` |
| Prune journal | 2nd click | count in label | No | `JournalPanel.tsx:148-158` |
| Abort merge/rebase/cherry-pick | none | no | NO (not journaled) | `WipPanel.tsx:136`; `merge.rs:133-139` |
| Rebase onto (non-interactive) | none | no | Yes (journal `rebase`) | `RefMenu.tsx:305` |
| Interactive rebase | editor + Start | list of commits | Yes | `RebaseEditor.tsx` |
| Merge | dialog with conflict preview | file list | Yes | `RefMenu.tsx:323-341` |

## Findings

### [MOD-1] No ErrorBoundary: a failed resource can blank the whole app
- Severidade: ALTA
- Item da matriz: UI-006
- Local: `app/ui/src/App.tsx:304` (details), `features/diff/DiffView.tsx:24`, `features/journal/JournalPanel.tsx:11`, `features/stash/StashPanel.tsx:11`, `features/shared-stash/SharedStashMenu.tsx:71`
- Evidência: `grep -rn "ErrorBoundary\|unhandledrejection" app/ui/src` returns nothing. These `createResource` fetchers have no `.catch`, unlike `RefMenu.tsx:25`, `RebaseEditor.tsx:17`, `ConflictEditor.tsx:78` which do. Example: `createResource(() => ..., ([id, sha]) => api.details(id, sha))`, then `<Show when={details()} ...>` reads it. In Solid, reading a rejected resource throws during render; with no boundary the root `render(() => <App />)` (`main.tsx:14`) unmounts. Not reproduced at runtime.
- Por que importa: a commit that vanishes after an external rewrite, a stash list read failure or a journal read failure (IO_ERROR) leaves a blank window with no message; the user loses the open tabs/state.
- Sugestão de correção: wrap `<App/>` (and each panel) in `<ErrorBoundary fallback>` showing `errorText`; add `.catch` that calls `fail(e)` in the five resources; register `unhandledrejection` to route to `fail`.
- Esforço estimado: P

### [MOD-2] Abort merge/rebase/cherry-pick has no confirmation and cannot be undone
- Severidade: ALTA
- Item da matriz: F-021
- Local: `app/ui/src/features/wip/WipPanel.tsx:136`, `crates/ak-core/src/ops/merge.rs:133-139`
- Evidência: `onClick={() => void props.run(() => api.mergeAbort(id), { graph: true, toast: t("toast.mergeAborted") })}`; the core runs `git <what> --abort` and returns `OpOutcome { head, ..Default::default() }` (no journal entry, so the toast has no Undo).
- Por que importa: aborting throws away every conflict resolution already saved in the working tree (including work done in the ConflictEditor); a single misclick on the danger button loses it, with no warning naming the loss.
- Sugestão de correção: confirm dialog ("Abort merge? Resolutions of N files will be lost") that defaults focus to Cancel; or snapshot with `git stash create` before aborting and journal it for Undo (as hard reset does).
- Esforço estimado: P

### [MOD-3] Hard reset runs immediately from the commit menu
- Severidade: MÉDIA
- Item da matriz: F-025
- Local: `app/ui/src/features/graph/CommitMenu.tsx:198,220`
- Evidência: `const reset = (mode) => go(() => api.history(..."reset", sha(), { mode }), ...)` and `{item(..., t("commitmenu.resetHard"), () => reset("hard"))}`; no dialog. Core mitigates: "a hard reset keeps a snapshot of the discarded changes for Undo" (`history.rs:82`). Toast Undo lasts 10 s (`App.tsx:198`); afterwards only Ctrl+Z / Journal.
- Por que importa: destructive and keyboard-reachable (ArrowDown x5 + Enter) with no preview of how many files/commits are lost; the safety net depends on the user noticing the 10 s toast.
- Sugestão de correção: confirmation dialog for `hard` only, listing the count of uncommitted files and commits that become unreachable; keep soft/mixed direct.
- Esforço estimado: P

### [MOD-4] Branch delete, force delete and tag delete have no confirmation
- Severidade: MÉDIA
- Item da matriz: UI-004
- Local: `app/ui/src/features/refs/RefMenu.tsx:310,316-317`
- Evidência: `go(() => api.branchDelete(props.repoId, r().name, true), ...)` directly from the menu item; the two delete items sit adjacent and differ only by label ("Delete" / "Delete anyway (Undo brings it back)", `i18n.ts:138`). Undoable via journal (`branch.rs:12`).
- Por que importa: force delete discards unmerged commits from branch visibility; a mis-picked adjacent item deletes immediately. Safe delete already errors with `BRANCH_NOT_MERGED` (good), so the force path could ask only then.
- Sugestão de correção: skip the confirm for safe delete; on `BRANCH_NOT_MERGED` open a dialog listing the number of unmerged commits with "Delete anyway"; confirm tag deletion or at least keep the Undo toast.
- Esforço estimado: P

### [MOD-5] Non-interactive rebase starts directly from the ref menu
- Severidade: BAIXA
- Item da matriz: UI-004
- Local: `app/ui/src/features/refs/RefMenu.tsx:305`
- Evidência: `go(() => api.rebase(props.repoId, r().name), t("toast.rebased"))`; the merge item opens a preview dialog (`RefMenu.tsx:323`) but rebase has none.
- Por que importa: history rewrite with no confirmation or conflict prediction while merge has both; inconsistent. Undoable (`rebase` in `branch.rs:12`), hence BAIXA.
- Sugestão de correção: reuse the merge dialog pattern (commits to replay, Cancel focused).
- Esforço estimado: P

### [MOD-6] Force-push confirmation state survives dismissing the banner
- Severidade: MÉDIA
- Item da matriz: F-023
- Local: `app/ui/src/App.tsx:68,607-611`
- Evidência: `onClick={() => { if (!forceConfirm()) return setForceConfirm(true); setForceConfirm(false); setError(null); void net("push", true); }}`; the Dismiss button is only `setError(null)` and never resets `forceConfirm`. After "Force push" -> Dismiss, the next `PUSH_REJECTED` banner renders already armed ("Click again: replaces the remote branch") and one click force-pushes.
- Por que importa: the armed label does warn, but the two-step protection is silently halved. `--force-with-lease` (`remote.rs:116`) limits damage.
- Sugestão de correção: reset `forceConfirm` when the error changes/dismisses (`createEffect(on(error, () => setForceConfirm(false)))`) and add a blur/timeout reset.
- Esforço estimado: P

### [MOD-7] Escape closes every open overlay at once (no innermost-only handling)
- Severidade: MÉDIA
- Item da matriz: UI-005
- Local: `app/ui/src/features/overlay.ts:37-41` vs the trap stack at `:27,44`
- Evidência: Tab honors `traps[traps.length - 1] !== self`, but Escape does not: `if (e.key === "Escape") { e.preventDefault(); onClose(); return; }` runs in every mounted overlay's document listener. The credentials dialog is mounted at the app root independent of other overlays (`App.tsx:633`, `credentials-request` event at any time, e.g. during a Shared Stash fetch while a menu/Journal is open).
- Por que importa: Escape meant to cancel the credentials prompt also closes the Journal/ref dialog (and vice versa, Escape on a menu answers `null` to a pending credentials request). Same for DiffView's own Escape listener (`DiffView.tsx:22`).
- Sugestão de correção: in `key`, handle Escape only if `traps[traps.length-1] === self` (register in `traps` even without `root`).
- Esforço estimado: P

### [MOD-8] "Click again" confirmations: no announcement, no minimum delay, weak wording
- Severidade: MÉDIA
- Item da matriz: UI-007
- Local: `WipPanel.tsx:51-58,77`, `StashPanel.tsx:274-278,305-306`, `JournalPanel.tsx:185`, `SharedStashPanel.tsx:132`
- Evidência: the only change is the button text (`{confirm() === item.path ? t("wip.discardConfirm") : t("wip.discard")}`) with no live region and no focus change; discard timer 3 s but stash drop resets only on blur; a double-click fires both clicks and confirms instantly; text "Discard? (click again)" does not say what is lost, and the same wording covers deleting an untracked file (`UNTRACKED` items go through the same `discard()`).
- Por que importa: screen-reader users do not hear the state change; fast double-click defeats the guard; the user is not told the file's changes (or an untracked file) vanish.
- Sugestão de correção: `aria-live` announcement or a real `alertdialog` for discard/drop; ignore the second click within ~400 ms; name the loss ("Discard 12 changed lines in a.ts" / "Delete untracked file").
- Esforço estimado: M

### [MOD-9] Dialogs are `<dialog open>` without `showModal()`; the page behind is not inert
- Severidade: BAIXA
- Item da matriz: UI-005
- Local: all dialogs, e.g. `NameDialog.tsx:156`, `RebaseEditor.tsx:32`, `ConflictEditor.tsx:98`
- Evidência: `<dialog ref={dialog} open class="modal" aria-modal="true" ...>`; containment relies on the Tab handler in `overlay.ts:43-48`.
- Por que importa: pointer/AT virtual-cursor navigation can reach the background; `docs/a11y/audit.md:36` claims Pass for keyboard only. Mouse clicks are blocked by `.modal-back` (verified in CSS `styles.css:168`).
- Sugestão de correção: set `inert` on the app root while a trap exists, or use `showModal()`.
- Esforço estimado: M

### [MOD-10] English text leaks in pt-BR for uncatalogued errors; GIT_FAILED has no localized fix
- Severidade: BAIXA
- Item da matriz: UI-006
- Local: `app/ui/src/i18n.ts:450-458`; core strings e.g. `stash.rs:96`, `history.rs:60`, `remote.rs:29`
- Evidência: `errorFix` returns `e.suggestion` (English from the core) when no `err.<CODE>.fix` key exists; `GIT_FAILED` has `err.GIT_FAILED` but no `.fix`, so "Commit or stash your local changes first." shows untranslated. `errorText` likewise falls back to `e.message`. All 38 codes emitted by `OpError::new/UiError::new` have an `err.` key (checked by script); raw stderr is only in `detail` behind a "Details" toggle (`App.tsx` banner, `showDetail`).
- Por que importa: partial i18n gap; the stderr handling itself matches the "no raw stderr toast" goal.
- Sugestão de correção: add `err.GIT_FAILED.fix`/`.fix` keys, or make the core send suggestion codes.
- Esforço estimado: P

### [MOD-11] No re-entrancy guard on `run()`, undo/redo and stash buttons
- Severidade: BAIXA
- Item da matriz: F-025
- Local: `App.tsx` `run()` (~line 185), toolbar Undo/Redo (`disabled={!tab()}` only), `StashPanel.tsx:303-305`
- Evidência: only `netBusy` (fetch/pull/push), `WipPanel busy` (commit) and `ShareDialog busy` exist. Repeated Ctrl+Z / double click on Apply/Pop produce concurrent calls; the core lock answers `REPO_BUSY` (`err.REPO_BUSY` exists), which then appears as an error banner.
- Por que importa: spurious errors; two quick Pops could also apply twice if the lock window is missed.
- Sugestão de correção: global `opBusy` signal set in `run/undo/redo`, disabling those buttons.
- Esforço estimado: P

### [MOD-12] Toast is single-slot, auto-hides in 10 s/4 s and does not pause on hover/focus
- Severidade: BAIXA
- Item da matriz: UI-006
- Local: `App.tsx:108,197-198,216,235,655`
- Evidência: `toastTimer = window.setTimeout(() => setToast(null), 10000)`; every `setToast` replaces the previous, including its Undo button. `<output class="toast">` is a live region (good).
- Por que importa: a second operation within 10 s removes the Undo of the first from the toast (Ctrl+Z/Journal still work); timing is hard for keyboard/AT users (WCAG 2.2.1).
- Sugestão de correção: pause timer on hover/focus, keep a short queue.
- Esforço estimado: P

### [MOD-13] Journal dialog shows "empty" while loading and has no error state
- Severidade: BAIXA
- Item da matriz: F-025
- Local: `features/journal/JournalPanel.tsx:11,33`
- Evidência: `<Show when={(entries() ?? []).length} fallback={<p class="muted">{t("journal.empty")}</p>}>` — during load `entries()` is undefined so "empty" is shown; a rejected load throws (see MOD-1). Rebase/Conflict/Merge dialogs do show `t("loading")`.
- Por que importa: wrong message flash; error not surfaced in the dialog.
- Sugestão de correção: branch on `entries.loading` / `entries.error`.
- Esforço estimado: P

### [MOD-14] Credentials prompt shows only git's raw prompt, no repo/remote context
- Severidade: BAIXA
- Item da matriz: UI-005
- Local: `App.tsx:633-635`, `app/src-tauri/src/askpass.rs:139`
- Evidência: `<NameDialog label={c.prompt || t("creds.title")} secret={c.secret} .../>`; the payload has `repo_id` but the UI shows neither repo nor remote nor operation; queued requests show one at a time without a counter. Password is not trimmed (`NameDialog.tsx:151`, good); input is `type=password` and `autocomplete=off`. Escape answers `null` (cancel).
- Por que importa: with several tabs or a Shared Stash fetch in the background, the user cannot tell why a password is requested.
- Sugestão de correção: add heading with repo name and operation; show "1 of N".
- Esforço estimado: P

## O que já está bom
- Focus trap with a stack for Tab, Escape, focus-return and menu key helpers (`overlay.ts`, tested in `overlay.test.ts`); roles/`aria-labelledby` on every dialog; menus use `role=menu/menuitem` with arrow-key navigation.
- NameDialog does not close on backdrop click (no lost text); Merge dialog focuses Cancel first and previews conflicts; Unshare dialog is a real confirmation naming consequences; ConflictEditor blocks saving while markers remain; RebaseEditor validates squash-first and empty reword.
- Journal-backed Undo covers discard, hard reset, branch/tag delete, stash drop and rebase; force push uses `--force-with-lease`.
- Errors map to stable codes with localized text (all 38 emitted codes have `err.*`), stderr hidden under "Details", `role="alert"` on the banner, `<output>` live region for toasts.
- Double-submit guards exist for commit (`busy`), network (`netBusy`) and ShareDialog (`busy`).

## Não verificado
- Real rendering, focus order, screen reader output and screenshots (UI not run).
- Whether MOD-1 truly blanks the app in the Tauri webview (Solid behavior inferred from code, not reproduced).
- GitKraken's actual behavior (REA ux-analysis.md not found).
- Whether `Undo` of hard reset / discard restores correctly (core tests exist `ops.rs`, not re-run here).
- CSS stacking of the error banner behind `.modal-back` during modal errors (e.g. ConflictEditor save failure keeps dialog open; banner is dimmed by the backdrop, `styles.css:168`).
