# Audit 02 — Features and robustness

Date: 2026-10-15. Finding IDs: `FUNC-n`. Matrix IDs from `auditoria/00-matriz-documentacao-vs-codigo.md`.

## Limitações

- Static analysis only: code, greps and the Phase 0 baseline (`cargo test` 120 passed; vitest 81 passed). The app was not driven interactively; no network or credential flow was exercised.
- Rust skills (`rust-best-practices`, `rust-async-patterns`, `rust-testing`) were not loaded through the Skill tool; their criteria (Result over panic, no blocking in async, test layers) were applied from general knowledge.
- Crash behaviour (kill during an operation) is reasoned from the code order, not reproduced.
- `unwrap/expect` classification covers non-test code only (everything before the first `#[cfg(test)]` of each file).
- Only REA docs, internal docs and code were used. No GitKraken feature was assumed.

## Matrix status review (refinements)

| ID | Matrix | Verdict | Evidence |
|---|---|---|---|
| F-003 | Absent | **Confirmed** | `grep -rniE "recent"` hits only i18n/theme text; no persisted repo list in `app/src-tauri/src` |
| F-005 | Absent | **Confirmed** | only `localStorage` keys: `ak.split.*` (Splitter.tsx:10), columns (columns.ts:39), language (i18n.ts:427), WIP open (WipPanel.tsx:13), glyph (shared-stash/model.ts:75). No tab list |
| F-007 | Partial | **Confirmed** | WIP row exists (`drawWip`, GraphView.tsx:248; `wipText` a11y.ts:50); stash is a label (main.rs:~262 `RefKind::Stash` chained onto base row), not a row |
| F-012 | Absent | **Confirmed** | `file_diff` args fixed: `show --format= -m --first-parent --no-ext-diff --no-color` (read.rs:115), no `-w` |
| F-013 | Absent | **Confirmed** | no highlighter dependency/usage |
| F-014 | Not verified | **Absent** | both diff paths return only hunks: read.rs:115 and `worktree_diff` read.rs:210-225 (default context, no `-U<big>`/`cat-file`); no whole-file endpoint among the 51 Tauri commands |
| F-029 | Absent | **Confirmed** | no zoom/font scale in app/ui/src or tauri.conf.json |
| F-033 | Absent | **Confirmed** | see FUNC-4 |
| F-034 | Partial | **Confirmed** | `LARGE_STASH_BYTES` only warns (`large_bytes`, shared_stash_ops.rs:518) |
| F-036..F-041 | Absent | **Confirmed** | see FUNC-5 |
| NF-004 | Not verified | **Not verified** | no cold-start measurement exists; cannot be measured statically |
| NF-011 | Not verified | **Likely met, not proven** | open/graph/status/stage/commit use local git only; ak-core integration tests use local bare remotes (ops.rs:688). Network only on `op_net` and shared-stash commands. No offline test |
| AC-001 | Not verified | **Not verified** | no e2e; `get_initial_path` (main.rs:284) exists |
| UI-008 | Not verified | **Implemented** | `<footer class="statusbar">` App.tsx:663 (commit count, Git version, read-only reason) |

## Findings

### [FUNC-1] No recent repositories and no session restore
- Severidade: ALTA
- Item da matriz: F-003, F-005
- Local: app/ui/src/App.tsx:51 (`tabs`), app/src-tauri/src/main.rs (`repo_open`)
- Evidência: tabs live in a Solid signal; no persistence code (see matrix review). After closing the app the user must reopen every repo with the dialog or CLI argument.
- Por que importa: documented in PRD §5.1; a daily-use client without "reopen what I had" is a basic usability gap, and Phase 5 shipped as MVP without it.
- Sugestão de correção: persist `{path, active}` list in app config dir (the settings.rs atomic JSON writer already exists) on tab change; add a "Recent" list on the empty state; drop paths that no longer exist at start.
- Esforço estimado: M

### [FUNC-2] Diff: no whitespace toggle, no whole-file view, no syntax highlighting
- Severidade: MÉDIA
- Item da matriz: F-012, F-013, F-014 (F-014 corrected to Absent)
- Local: crates/ak-core/src/read.rs:111-118 and :210-226; app/ui/src/features/diff/DiffView.tsx:15
- Evidência: `["show","--format=","-m","--first-parent","--no-ext-diff","--no-color",sha,"--",path]`; modes are only `unified|split`. Also no size cap: both functions return the full diff as one `String` over IPC.
- Por que importa: three documented PRD §5.4 items missing; a very large diff (generated file) is sent whole and parsed in the UI thread.
- Sugestão de correção: add `ignoreWhitespace` flag (`-w`) to both diff commands; whole-file view via `-U999999` or `git show <sha>:<path>`; cap diff bytes (e.g. 2 MB) with a "show anyway" action; highlighting can wait (roadmap T1.9).
- Esforço estimado: M

### [FUNC-3] Zoom 80-200 % absent
- Severidade: MÉDIA
- Item da matriz: F-029, NF-009
- Local: app/ui (no code), app/src-tauri/tauri.conf.json
- Evidência: `grep -rniE "zoom|font-scale|fontSize"` over app/ui/src and tauri.conf.json: no match.
- Por que importa: PRD §5.11 requirement and an accessibility aid (NF-008); the docs/a11y audit already notes font scaling is not handled.
- Sugestão de correção: root `font-size` token driven by a persisted setting plus Ctrl+= / Ctrl+- / Ctrl+0; layout uses px in places (styles.css) so verify the graph canvas row height scales.
- Esforço estimado: M

### [FUNC-4] Shared Stash publishes without secret check or hard size limit
- Severidade: ALTA
- Item da matriz: F-033, F-034
- Local: crates/ak-core/src/ops/shared_stash_ops.rs:473-521
- Evidência: `new_blob_bytes` feeds only `large_bytes` (`(bytes > LARGE_STASH_BYTES).then_some(bytes)`, :518); the push (`push_shared`, :306) runs regardless. No content scan anywhere in shared_stash*.rs.
- Por que importa: PRD §7 N0 Safety demands a secrets check and a 5 MB limit before pushing. A stash often contains `.env`/keys and is pushed to a shared remote, where it is hard to retract (history stays on the server).
- Sugestão de correção: before push, scan added blobs for common secret patterns and sensitive file names (`.env`, `id_rsa`, `*.pem`) and require explicit confirmation; make the limit configurable and blocking above the hard cap (default 5 MB per PRD).
- Esforço estimado: M

### [FUNC-5] N3-N7 and search/blame not implemented
- Severidade: MÉDIA
- Item da matriz: F-036, F-037, F-038, F-039, F-040, F-041
- Local: whole app
- Evidência: no range-compare, saved views, command palette (Ctrl+Shift+P only toggles the perf overlay, App.tsx:277), `StashPush { message, include_untracked }` without paths (ops/mod.rs:100), no identity rules, no commit search/blame (only refs filter, features/refs/filter.ts).
- Por que importa: roadmap items beyond the MVP; F-041 (commit search) is the one tied to a PRD persona and is a basic expectation of a consolidated client.
- Sugestão de correção: prioritise F-041 (search by message/author/sha over the already loaded `RowText` window or `git log --grep`), then F-039; leave N3/N4/N5/N7 for after the MVP and keep them documented as such.
- Esforço estimado: G

### [FUNC-6] Hard reset runs on one click with no confirmation
- Severidade: ALTA
- Item da matriz: F-026, NF-006
- Local: app/ui/src/features/graph/CommitMenu.tsx:51 and :34 (`reset`)
- Evidência: `const reset = (mode) => go(() => api.history(..., "reset", sha(), { mode }), ...)`; menu item "Reset current branch here (hard: discard changes)" calls it immediately. Other destructive actions (discard, stash drop, force push, journal prune) all use two-step confirmation.
- Por que importa: by the severity rule this is a destructive op without confirmation. Mitigation: history.rs:84-100 saves `stash create` + a snapshot ref, and Undo restores it (test ops.rs:650); but `stash create` does not include untracked files, and Undo is refused if the worktree is dirty (`WORKTREE_CHANGED`).
- Sugestão de correção: confirm step (or dialog listing the number of uncommitted files lost) for `hard`; keep the Undo path.
- Esforço estimado: P

### [FUNC-7] Journal entries can panic the undo path (unchecked `args[n]`)
- Severidade: ALTA
- Item da matriz: F-025
- Local: crates/ak-core/src/ops/history.rs:123,192; stash.rs:121,132,135,140; branch.rs:133 (`let a = &e.args`)
- Evidência: `let (mode, wip) = (e.args[0].as_str(), e.args[1].as_str());` while `JournalEntry.args` is `#[serde(default)]` (journal.rs) and the file is plain, user-editable JSONL. A truncated/older/edited entry with `kind:"reset"` and empty `args` panics.
- Por que importa: the panic occurs inside an `async` Tauri command; the `LockGuard` is released by unwinding, but the user gets a crash-like failure on Undo/Redo, the feature meant to rescue them. Panic = no stable error code.
- Sugestão de correção: validate `args.len()` per kind in one helper returning `UNDO_NOT_SUPPORTED`/`JOURNAL_CORRUPT`; add a test with malformed entries.
- Esforço estimado: P

### [FUNC-8] Journal durability and silent loss of corrupt lines
- Severidade: MÉDIA
- Item da matriz: F-025, NF-006
- Local: crates/ak-core/src/ops/journal.rs:44-99; ops/mod.rs:365,587 (append after the action)
- Evidência: (a) `append` has no `sync_data`; (b) `list()` uses `filter_map(... .ok())` so unreadable lines are dropped, and `set_undone`/`prune` rewrite the whole file from that list, permanently deleting those lines; (c) rewrite uses fixed `journal.tmp` with `fs::write` without fsync; (d) in `commit`/`discard` the journal is written after the destructive step and `?` turns a journal I/O error into a failed operation although the commit/discard happened (and the discard backup in `.git/atlaskraken/discarded/<id>` is orphaned).
- Por que importa: crash or disk-full between the action and the append yields an operation that exists in Git but not in the journal; a discard whose entry was never written cannot be undone from the UI (data only recoverable by hand from the backup dir).
- Sugestão de correção: write the intent first (`pending` entry) and mark it done afterwards; keep unparseable lines verbatim on rewrite; fsync file and parent dir; on append failure after success return the outcome with a warning instead of an error.
- Esforço estimado: M

### [FUNC-9] Conflicted merge/cherry-pick/revert is never journaled
- Severidade: MÉDIA
- Item da matriz: F-019, F-021, F-025
- Local: crates/ak-core/src/ops/merge.rs:~96-98 (`return Ok(OpOutcome { ..., conflicted: true })` before `journal.append`); history.rs:52-53; remote.rs:80-81 (pull)
- Evidência: on conflict the function returns before journaling. When the user later resolves and commits through the WIP box, the journaled entry is a plain `commit`; its Undo (`reset --soft head_before`, mod.rs:~415) removes the merge commit but `MERGE_HEAD` is already gone, so re-committing makes a single-parent commit.
- Por que importa: the most error-prone merges (the conflicted ones) are the ones with the weakest Undo, and the journal summary does not say "merge".
- Sugestão de correção: record head_before when the conflict starts (e.g. in a small state file) and journal the merge/pick when `MERGE_HEAD`/`CHERRY_PICK_HEAD` is concluded; or mark the commit entry kind `merge_commit` and undo with `reset --merge`.
- Esforço estimado: M

### [FUNC-10] Blocking git work on the async runtime; no timeout or cancel for local ops
- Severidade: MÉDIA
- Item da matriz: F-023, NF-002
- Local: app/src-tauri/src/main.rs:289-310 (`repo_open`, `repo_refresh` call blocking `load`), :431-434 (`run_op`); crates/ak-gitcli/src/exec.rs `run`/`run_status` (`.output()`, no timeout)
- Evidência: only `op_net` (main.rs:490) and shared-stash commands use `spawn_blocking`; every other `async fn op_*` runs `engine.run` (which spawns git and waits) directly on a Tokio worker. `Command::output()` has no deadline.
- Por que importa: opening a 100k-commit repo (`load` reads history, builds layout) or a hanging hook (pre-commit/post-checkout, slow network FS) occupies a runtime worker; with several tabs/ops the IPC can stall. A hung git also keeps `Lock::Index` held, so every later op in that repo answers `REPO_BUSY` until the app is restarted; there is no cancel for `op_net` (only `shared_stash_cancel` exists).
- Sugestão de correção: wrap `run_op`/`load` in `spawn_blocking`; add an optional timeout/kill handle in `Git::run*` and a cancel command for long ops.
- Esforço estimado: M

### [FUNC-11] Askpass socket: permission race, no peer check, silent fallback
- Severidade: MÉDIA
- Item da matriz: F-024, NF-007
- Local: app/src-tauri/src/askpass.rs:85-91, :131, :140
- Evidência: `UnixListener::bind(&path)` then `set_permissions(0o600)` (two steps; the socket is created with the umask first). Directory is `$XDG_RUNTIME_DIR` (0700, fine) else `std::env::temp_dir()` (/tmp, shared); name is predictable (`atlasgit-askpass-<pid>-<n>.sock`). The accept loop never checks the peer UID and `serve` does a blocking `read_line` with no read timeout; if `bind` fails the code runs git without bridge (`else { return f(&[]) }`).
- Por que importa: in the /tmp fallback a local user could connect in the window before chmod and inject a fake prompt that the user answers (password returned to the attacker). A client that connects and never writes blocks the worker, and `worker.join()` then keeps the network op from returning. Silent fallback makes Git fail with a generic auth error.
- Sugestão de correção: create a private 0700 directory (or set umask 077 before bind); check `peer_cred` UID; set a read timeout on the stream; surface a clear `ASKPASS_UNAVAILABLE` error on bind failure.
- Esforço estimado: P

### [FUNC-12] `is_secret` heuristic: masks usernames, shows some secrets in clear
- Severidade: MÉDIA
- Item da matriz: F-024
- Local: app/src-tauri/src/askpass.rs:77-80
- Evidência: `["password","passphrase","token","pin"].iter().any(|w| p.contains(w))` on the lowercased prompt. `pin` is a substring of "spin", "pinned", "Pinterest" etc. (e.g. `Username for 'https://github.com/spin-org'` is treated as secret). Prompts such as "Verification code", "Enter passcode", "OTP" or "Enter PIN" variants without the exact words are not matched and would be echoed unmasked. The only test (askpass.rs:150+) has three cases.
- Por que importa: wrong masking either hides the username the user must read or exposes a one-time code/second factor on screen.
- Sugestão de correção: match on word boundaries (`\bpin\b`) and extend the list (passcode, verification, otp, secret); default to masked when the prompt does not start with "Username"; add table-driven tests.
- Esforço estimado: P

### [FUNC-13] `worktree_diff` accepts any path (no `check_rel`) and reads untracked files unbounded
- Severidade: MÉDIA
- Item da matriz: NF-007, AC-011
- Local: crates/ak-core/src/read.rs:210-219; main.rs:352 (`diff_worktree`)
- Evidência: for `untracked=true` it runs `git diff --no-index -- /dev/null <path>` with the caller's path as is; the write operations all call `check_rel` (mod.rs `check_rel`, rejects absolute, `..`, `.git`), the read path does not.
- Por que importa: the typed API is the security boundary (SDD §5): from the UI (or any injected script) an absolute path such as `/etc/shadow-like` file would be read and returned; a multi-GB untracked file would be loaded entirely into a `String`.
- Sugestão de correção: call the same `check_rel` in `worktree_diff` and `file_diff`; reject non-regular files; truncate output above a limit.
- Esforço estimado: P

### [FUNC-14] Silent degradation: engine, watcher and git executable
- Severidade: MÉDIA
- Item da matriz: F-004, NF-005
- Local: app/src-tauri/src/main.rs:246 (`Engine::open(git, &root).ok()`), :129-205 (`start_watcher` returns `None` via `.ok()?`), :138,:166 (`Command::new("git")`)
- Evidência: any error from `Engine::open` other than caps leaves the repo read-only with `readOnlyReason` None (reason is computed only from `caps.supported()`, :268). A failed inotify `watch` (limit reached on big trees) makes the whole watcher `None` with no message; `repo-changed` is then never emitted. The watcher shells out to a bare `git`, not the executable discovered by `Git::discover`.
- Por que importa: the user sees a repo where every action says "read-only" with no explanation, or a UI that never refreshes after external changes (documented behaviour "external edits").
- Sugestão de correção: propagate the engine error as `read_only_reason`; emit a `watcher-unavailable` event and show a "Refresh" hint; reuse `Git` for the `ls-files` call.
- Esforço estimado: P

### [FUNC-15] Shared-state `Mutex::unwrap()` can cascade after one panic
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: main.rs:173,190,292,297,307,313,314,318; askpass.rs:48,55,120,137,138,141,142; shared_stash.rs:40,178
- Evidência: `state.repos.lock().unwrap()`. `ops/lock.rs` already uses `unwrap_or_else(|e| e.into_inner())`, so the pattern exists in the codebase.
- Por que importa: one panic while a guard is held (e.g. FUNC-7 style) poisons `repos`, and every later command (`get`, main.rs:318) panics too, bricking the session.
- Sugestão de correção: a tiny `lock()` helper using `into_inner` for these maps (data is just ids/handles).
- Esforço estimado: P

### [FUNC-16] Test coverage gaps in Tauri commands, UI and failure paths
- Severidade: MÉDIA
- Item da matriz: F-001, F-002, F-004, F-024, UI-001, UI-003, AC-001
- Local: app/src-tauri/src (51 `#[tauri::command]`: 37 in main.rs, 14 in shared_stash.rs); app/ui/src
- Evidência: tests that touch src-tauri: `askpass.rs` (1 test, 3 asserts) and `settings.rs` (5). Zero tests for repo_open/refresh, watcher, `op_net` bridge, `credentials_answer`, journal commands, `shared_stash_*` commands. UI: 12 vitest files, all pure-logic (parse/tree/columns/i18n/a11y helpers/overlay); none renders App, GraphView, WipPanel, ConflictEditor UI, JournalPanel or the credentials dialog. Core: `lock.rs` tests are single-threaded (no concurrent acquire test); no test for malformed journal lines, journal write failure, or crash between action and `append`; 2 `#[ignore]` 100k tests need the heavy fixtures.
- Por que importa: findings FUNC-7, 8, 10, 14 would be caught by such tests; the Tauri layer is where error mapping (`UiError`) and the askpass bridge live.
- Sugestão de correção: (1) core tests: corrupt journal, concurrent `try_acquire` with threads, failing append; (2) refactor `load`/commands into functions testable without `AppHandle`; (3) a few component tests with the existing vitest + `devMock.ts`; (4) one e2e smoke (open CLI path -> graph) for AC-001.
- Esforço estimado: G

### [FUNC-17] "Author-only" Shared Stash rules are advisory
- Severidade: BAIXA
- Item da matriz: F-032
- Local: crates/ak-core/src/ops/shared_stash_ops.rs:296-301, :689
- Evidência: `normalize_email(&v.meta.author_email) != normalize_email(&who.email)` where `who` comes from the local `user.email`; protection against concurrent change is `--force-with-lease` (:309).
- Por que importa: anyone with push access who sets the same `user.email` can update/unshare another person's stash; the check prevents mistakes, not abuse. Fine for a trusted team, but the UI/docs should not suggest enforcement.
- Sugestão de correção: state this limit in docs/user and the spec; optionally rely on host permissions.
- Esforço estimado: P

## Operation engine review (evidence for the questions asked)

- **Locks** (`ops/lock.rs`): in-memory table keyed by repo path, all-or-nothing `try_acquire`, released on `Drop`, poison-tolerant. There are no lock files, so **no stale locks after a crash**; the cost is that it only protects within one process (two AtlasGit windows or an external `git` rely on Git's own `index.lock`, and `GIT_OPTIONAL_LOCKS=0` is set, exec.rs). A hung git keeps the lock until restart (FUNC-10). Canonical repo path is `rev-parse --show-toplevel`, so symlinked paths map to one key.
- **HEAD_MOVED / REF_MOVED**: undo of `commit/commit_amend` requires `HEAD == head_after` (mod.rs:411), redo requires `HEAD == head_before` (:456); ref-kind entries use `REF_MOVED`; discard uses content comparison (`WORKTREE_CHANGED`, mod.rs:606+, :484+). Logic is sound and tested (ops.rs:120). Gap: an operation done outside the app moves HEAD and permanently blocks undo of earlier entries (message tells the user to undo newer ones, which may not exist in the journal).
- **Crash during an op**: ordering is action -> snapshot/backup -> journal. Discard copies backups before touching files (good, mod.rs:~560), reset saves snapshot refs before moving. The unsafe window is the last step (FUNC-8). Rebase writes `.git/atlaskraken/rebase/` and removes it only on success paths (`rebase_continue` cleans it; `merge_abort` on a rebase does not), leaving small orphans.
- **Redo stack**: `redo_candidate` clears redo after any newer non-passive entry; correct and tested (ops.rs:287,308).
- **Journal prune** leaves `-redo`/`-wip` refs (known, HANDOFF): `prune_journal` deletes only `snapshot_ref` (mod.rs:~205), so those refs accumulate and keep objects alive.

## Authentication (askpass) summary

- Socket: `$XDG_RUNTIME_DIR` else temp dir, mode 0600 after bind, removed on exit (`remove_file`, askpass.rs:123); secrets not written to disk (comment and code agree: answer goes socket -> stdout of the client).
- Cancel/timeout: UI answer `None` closes the stream -> client exit 1; `ANSWER_TIMEOUT` 300 s; `cancel_repo` and the "Git is done" cleanup emit `credentials-cancel`. Good.
- Gaps: FUNC-11, FUNC-12; the typed answer is a plain `String` (not zeroized) and travels via Tauri IPC to the webview - acceptable, but note it for the threat model.

## Classification of non-test `unwrap/expect/panic` (≈30 sites)

| Class | Sites | Verdict |
|---|---|---|
| Mutex poisoning `lock().unwrap()` | main.rs:173,190,292,297,307,313,314,318; shared_stash.rs:40,178; askpass.rs:48,55,120,137,138,141,142 | Low risk alone, cascades after any panic (FUNC-15) |
| Invariant (just checked) | settings.rs:66,71 `as_object_mut().expect("object")`; read_gix.rs:106,122 (`indegree` keys come from the same `nodes`); exec.rs:124 `child.stdin.take().expect("piped stdin")` (set one line above) | Safe by construction; could be `let ... else` |
| Fatal at startup | main.rs:638 `expect("error while running AtlasGit")` | Acceptable |
| User-reachable panic | not unwrap, but indexing `e.args[n]` (FUNC-7) | Real risk |

No `panic!`/`unreachable!`/`todo!` in non-test code.

## O que já está bom

- Every working-tree change goes through `Operation` with a stable `OpError.code`; UI maps codes to i18n text (UI-006).
- Paths from the UI are validated (`check_rel`, `:(literal)` pathspecs, chunking by 100) on all write ops; revisions are hex-checked (`check_rev`).
- Network ops use `--force-with-lease` (remote.rs:116, shared stash) and the UI asks confirmation for force push, discard, stash drop and prune.
- Discard backs up before deleting and refuses symlinks/non-regular files (test ops.rs:372); undo refuses to overwrite newer work.
- `Git` wrapper denies write subcommands in read-only mode and never passes `safe.directory=*` (tests exec.rs:204+).
- Lock table is all-or-nothing and poison-tolerant; 59 core integration tests + 31 shared-stash e2e tests with two clones.

## Não verificado

- Real behaviour on kill -9 during an operation, with a huge repo, or with a hanging hook (reasoned, not executed).
- NF-004 cold start, AC-001, NF-002 UI measures; offline mode (NF-011) only inferred.
- Whether the WebView can really call `diff_worktree` with arbitrary paths (depends on Tauri capability scope; code shows no server-side check).
- Spec text `docs/specs/shared-stash.md` on author-only enforcement and the 10 MiB choice (not re-read in full).
- Orca/AT behaviour of the credentials dialog.
- Whether `tokio` worker count and Tauri's blocking pool make FUNC-10 perceptible in practice (no measurement).
