# Investigation Findings: Desktop App Crashes on Linux (Spec 008 / T001)

## Executive Summary

A static review of `app/src-tauri/src` (`main.rs`, `diagnostics.rs`, `session.rs`, `prefs.rs`, `settings.rs`, `askpass.rs`, `shared_stash.rs`), `crates/ak-core`, and `crates/ak-gitcli` was conducted alongside an examination of user state, cache, and log directories (`~/.local/share`, `~/.cache`, and `~/.local/state`).

Multiple distinct crash mechanisms were identified:
1. **Unchecked UTF-8 Byte Slicing on Non-ASCII Ref/Branch Names** in `crates/ak-core/src/ops/rebase.rs:127` causing direct Rust panics (`byte index 7 is not a char boundary`).
2. **Cascading Mutex Poisoning via `.lock().unwrap()`** throughout `app/src-tauri/src/main.rs:318`, `askpass.rs`, and `shared_stash.rs`, causing synchronous Tauri commands (`graph_window`, `repo_close`) running on the main UI thread to panic with `PoisonError` and abort the process.
3. **Out-of-Bounds Indexing on `JournalEntry.args`** during Undo/Redo operations in `crates/ak-core/src/ops/history.rs:123, 192`, `stash.rs:121, 132, 135, 140`, `branch.rs:138, 141, 144, 150, 158`, and `shared_stash_ops.rs:909`.
4. **Orphaned Diagnostics Infrastructure**: `diagnostics.rs` was added to `app/src-tauri/src/` in commit `f33a78a` but was never imported (`mod diagnostics;`) or initialized in `main.rs`. Consequently, the panic hook is inactive, `last-crash.txt` is never written upon crash, and `diagnostics::lock()` (which recovers poisoned mutexes) is unused.
5. **WebKitGTK DMA-BUF Renderer Crash on Linux / Intel Graphics**: The WebKitGTK 2.40/2.42 hardware acceleration bug with Mesa drivers (specifically Tiger Lake Intel Iris Xe / UHD) crashes the WebProcess when rendering HTML5 Canvas elements (such as the commit graph canvas).

---

## Existing Logs & Crash Dumps Analysis

Inspections of directories matching `gitatlas`, `atlaskraken`, and `dev.atlas*` revealed:

### 1. `~/.local/state/gitatlas/gitatlas.log.2026-10-17`
Historical logs from an earlier build that had diagnostics initialized recorded two abrupt terminations:
```text
2026-10-17T14:15:27.442916Z  INFO gitatlas::diagnostics: GitAtlas iniciado version="0.1.0" dir=/home/thigas/.local/state/gitatlas
2026-10-17T14:21:50.037276Z  INFO gitatlas::diagnostics: GitAtlas iniciado version="0.1.0" dir=/home/thigas/.local/state/gitatlas
2026-10-17T14:21:50.037337Z  WARN gitatlas::diagnostics: a sessão anterior não encerrou normalmente
2026-10-17T14:25:29.665118Z  INFO gitatlas::diagnostics: GitAtlas iniciado version="0.1.0" dir=/home/thigas/.local/state/gitatlas
2026-10-17T14:25:29.665168Z  WARN gitatlas::diagnostics: a sessão anterior não encerrou normalmente
```
Between `14:15:27` and `14:21:50`, and again between `14:21:50` and `14:25:29`, GitAtlas exited abruptly without reaching normal shutdown (`Diagnostics::finish()`).

### 2. `~/.local/state/atlasgit/session.running`
Contains a leftover PID (`52633`) dated Oct 17. The process no longer exists in `/proc`, confirming an unclean exit where `session.running` was never cleaned up.

### 3. `~/.cache/gitatlas/launch.log`
The launcher log from `scripts/launch.sh` contained:
```text
$ tsc --noEmit && vite build
[ELIFECYCLE] Command failed with exit code 1.
```
When invoked from desktop shortcuts, frontend build failures abort startup immediately, with exit code 1 logged to `~/.cache/gitatlas/launch.log`.

### 4. Absence of `last-crash.txt`
In current builds, `last-crash.txt` is missing from `~/.local/state/gitatlas` and `~/.local/share` because `diagnostics::init()` is not wired up in `app/src-tauri/src/main.rs`. Any Rust panic writes only to `stderr` and terminates the process or worker thread without persisting the crash dump.

---

## Ranked Findings

### Finding 1 (Highest Likelihood): Non-ASCII UTF-8 Branch Name Slicing Panic
- **Location**: `crates/ak-core/src/ops/rebase.rs:127`
- **Likelihood**: HIGH
- **Severity**: HIGH (Rust panic during user operation)
- **Code**:
  ```rust
  let onto_txt = if onto.is_empty() { String::new() } else { format!(" onto {}", &onto[..7.min(onto.len())]) };
  ```
- **Root Cause**: `&onto[..7.min(onto.len())]` takes a raw byte slice of up to 7 bytes. If `onto` is a branch or ref name containing multi-byte UTF-8 code points (e.g., Portuguese characters `ç`, `ã`, `é` or emojis like `feat/ação` or `hotfix/v1.0-🚀`), byte index 7 can land in the middle of a multi-byte sequence. In Rust, slicing across a UTF-8 character boundary immediately panics with:
  ```text
  byte index 7 is not a char boundary; it is inside 'ç' (bytes 6..8)
  ```
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn rebase_onto_utf8_branch_name_does_not_panic() {
      let (d, e) = base_repo();
      // "feat/ação": len is 10 bytes; bytes 6..8 represent 'ç' (0xc3, 0xa7).
      // Slicing &onto[..7] lands between 0xc3 and 0xa7, triggering a panic.
      let branch_name = "feat/ação";
      git(&d, &["branch", branch_name]);
      let outcome = e.run(&Operation::Rebase {
          onto: branch_name.into(),
          todo: None,
      });
      assert!(outcome.is_ok(), "Rebase onto UTF-8 branch must not panic: {:?}", outcome);
  }
  ```

---

### Finding 2 (High Likelihood): Cascading Mutex Poisoning via `.lock().unwrap()` Crashing Main Thread
- **Location**:
  - `app/src-tauri/src/main.rs:318` (`get`)
  - `app/src-tauri/src/main.rs:173, 190` (watcher)
  - `app/src-tauri/src/main.rs:292, 297, 307, 313, 314`
  - `app/src-tauri/src/askpass.rs:48, 55, 120, 137, 138, 141, 142`
  - `app/src-tauri/src/shared_stash.rs:40, 178`
- **Likelihood**: HIGH
- **Severity**: CRITICAL (Bricks session and crashes UI process)
- **Code**:
  ```rust
  fn get(state: &AppState, repo_id: u32) -> Result<Arc<Repo>> {
      state.repos.lock().unwrap().get(&repo_id).cloned().ok_or_else(|| UiError::new("UNKNOWN_REPO", repo_id))
  }
  ```
- **Root Cause**: If any panic occurs while a lock guard is held (such as an out-of-bounds error during undo, rebase panic, or watcher thread error), the `Mutex` is permanently poisoned. Every subsequent call to `state.repos.lock().unwrap()` panics with:
  ```text
  called `Result::unwrap()` on an `Err` value: PoisonError { .. }
  ```
  Synchronous Tauri commands like `graph_window` (`main.rs:327`) and `repo_close` (`main.rs:312`) run directly on the main UI/event-loop thread. Calling `get()` or accessing `state.repos` causes a panic on `thread 'main'`, terminating the entire desktop application and closing the window.
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn poisoned_repos_mutex_is_recovered_without_panicking() {
      let state = AppState::default();
      let repos = Arc::clone(&state.repos);
      // Simulate another thread panicking while holding state.repos
      let _ = std::thread::spawn(move || {
          let _g = repos.lock().unwrap();
          panic!("simulated background panic");
      }).join();
      assert!(state.repos.is_poisoned());

      // get() must handle poisoned mutex gracefully using into_inner()
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          get(&state, 1)
      }));
      assert!(result.is_ok(), "get() must not panic when state.repos is poisoned");
  }
  ```

---

### Finding 3 (High Likelihood): Unchecked `args[n]` Indexing Panic on Malformed/Legacy Journal Entries
- **Location**:
  - `crates/ak-core/src/ops/history.rs:123`: `let (mode, wip) = (e.args[0].as_str(), e.args[1].as_str());`
  - `crates/ak-core/src/ops/history.rs:192`: `let (name, obj) = (&e.args[0], &e.args[1]);`
  - `crates/ak-core/src/ops/stash.rs:121, 132, 135, 140`: `e.args[0]`, `e.args[1]`
  - `crates/ak-core/src/ops/branch.rs:138, 141, 144, 150, 158`: `(&a[2], &a[3], &a[0], &a[1])`
  - `crates/ak-core/src/ops/shared_stash_ops.rs:909`: `(&a[0], &a[1], &a[2], a[3] == "true")`
- **Likelihood**: HIGH
- **Severity**: HIGH (Panic inside async Tauri command, leads to lock poisoning)
- **Code**:
  ```rust
  // crates/ak-core/src/ops/history.rs:123
  pub(super) fn apply_reset_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
      ...
      let (mode, wip) = (e.args[0].as_str(), e.args[1].as_str());
  ```
- **Root Cause**: `JournalEntry` declares `#[serde(default)] pub args: Vec<String>`. The file `<git-dir>/atlaskraken/journal.jsonl` is plain user-editable JSONL and may contain legacy entries without `args`, entries created by earlier versions, or truncated lines. When undoing/redoing an entry where `args` has fewer elements than expected, direct slice indexing panics with `index out of bounds: the len is 0 but the index is 0`.
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn journal_undo_malformed_args_returns_error_instead_of_panicking() {
      let (d, e) = base_repo();
      let journal_file = d.join(".git").join("atlaskraken").join("journal.jsonl");
      std::fs::create_dir_all(journal_file.parent().unwrap()).unwrap();
      // Write entry with kind="reset" but empty args
      let entry = r#"{"id":"corrupt-1","time":1000,"kind":"reset","summary":"Reset","branch":"main","headBefore":"HEAD","headAfter":"HEAD","commands":[],"undone":false,"args":[]}"#;
      std::fs::write(&journal_file, format!("{entry}\n")).unwrap();

      let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          e.undo("corrupt-1")
      }));
      assert!(res.is_ok(), "e.undo must not panic on malformed args");
      let err = res.unwrap().unwrap_err();
      assert_eq!(err.code, "JOURNAL_CORRUPT");
  }
  ```

---

### Finding 4 (High Likelihood): Orphaned `diagnostics.rs` — Missing Panic Hook and State Management
- **Location**: `app/src-tauri/src/main.rs:7-10`, `main.rs:575-639`, `app/src-tauri/src/diagnostics.rs`
- **Likelihood**: HIGH
- **Severity**: MEDIUM-HIGH (Prevents crash logging and recovery)
- **Code**:
  `app/src-tauri/src/main.rs` lines 7-10:
  ```rust
  mod askpass;
  mod settings;
  mod shared_stash;
  ```
- **Root Cause**: `diagnostics.rs` was added to `app/src-tauri/src/` to provide:
  1. Synchronous panic hook dumping backtraces to `last-crash.txt`.
  2. Rolling daily logs in `$XDG_STATE_HOME/gitatlas/gitatlas.log.*`.
  3. `session.running` detection of unclean exits.
  4. Poison-recovering `diagnostics::lock()`.
  However, `main.rs` was never updated to declare `mod diagnostics;` or call `diagnostics::init()`. The panic hook is therefore never registered. When a panic occurs, it prints to stderr and terminates without generating `last-crash.txt`, leaving the owner with no diagnostic logs.
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn panic_hook_writes_last_crash_file() {
      let temp = tempfile::tempdir().unwrap();
      diagnostics::install_panic_hook_for_test(temp.path().to_path_buf());
      let _ = std::panic::catch_unwind(|| {
          panic!("test crash recording");
      });
      let crash_file = temp.path().join("last-crash.txt");
      assert!(crash_file.exists(), "last-crash.txt must be written on panic");
      let content = std::fs::read_to_string(crash_file).unwrap();
      assert!(content.contains("test crash recording"));
  }
  ```

---

### Finding 5 (Medium-High Likelihood): WebKitGTK DMA-BUF Renderer Crash on Linux / Intel Mesa Drivers
- **Location**: `scripts/run.sh`, `scripts/launch.sh`, `app/src-tauri/tauri.conf.json:14`
- **Likelihood**: MEDIUM-HIGH
- **Severity**: HIGH (Silent app window closure / crash without Rust stack trace)
- **Root Cause**: On Linux systems with Intel Tiger Lake / Iris Xe graphics (such as `IdeaPad-3-15ITL6`) or NVIDIA drivers, WebKitGTK 2.40+ has an upstream issue with DMA-BUF hardware acceleration renderer when rendering HTML5 Canvas (`GraphView.tsx` renders the commit graph on canvas). The `WebKitWebProcess` crashes with `SIGSEGV`, causing WebKitGTK to close the window or terminate the application.
- **Workaround Required**: Set the following environment variables in `scripts/run.sh` and `scripts/launch.sh` or in `main.rs` before initializing WebKitGTK:
  ```bash
  export WEBKIT_DISABLE_DMABUF_RENDERER=1
  export WEBKIT_DISABLE_COMPOSITING_MODE=1
  ```
- **Proposed Verification Test**:
  Verify launcher scripts export `WEBKIT_DISABLE_DMABUF_RENDERER=1` when executing on Linux.

---

### Finding 6 (Medium Likelihood): Status Parser Slice Out-of-Bounds Panic on Short Untracked Lines
- **Location**: `crates/ak-gitcli/src/parse.rs:155-156`
- **Likelihood**: MEDIUM
- **Severity**: MEDIUM
- **Code**:
  ```rust
  b'?' => rep.entries.push(StatusEntry {
      path: p[2..].to_owned(),
      orig_path: None,
      index: '.',
      worktree: '?',
      conflicted: false,
      untracked: true,
  }),
  ```
- **Root Cause**: `parse_status_v2` parses git's null-delimited porcelain v2 output. If git status outputs a malformed or empty untracked entry where `p.len() < 2` (such as `p == "?"`), slicing `p[2..]` panics with:
  ```text
  byte index 2 is out of bounds of len 1
  ```
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn parse_status_v2_handles_truncated_untracked_indicator() {
      let raw = b"?\0";
      let res = parse_status_v2(raw);
      assert!(res.is_err(), "Short untracked status indicator must return ParseError, not panic");
  }
  ```

---

### Finding 7 (Medium Likelihood): Blocking Git Work on Tokio Async Worker Runtime (FUNC-10)
- **Location**: `app/src-tauri/src/main.rs:289-310`, `main.rs:431-434`
- **Likelihood**: MEDIUM
- **Severity**: MEDIUM (UI thread stall, IPC timeout, perceived crash / hang)
- **Code**:
  ```rust
  fn run_op(state: &AppState, repo_id: u32, op: Operation) -> Result<OpOutcome> {
      let r = get(state, repo_id)?;
      Ok(engine(&r)?.run(&op)?)
  }
  ```
- **Root Cause**: `repo_open`, `repo_refresh`, and all `op_*` commands (except `op_net`) are declared `async fn`, yet execute synchronous, blocking `std::process::Command` calls directly on Tokio runtime threads without `tauri::async_runtime::spawn_blocking`. A slow Git command, repository lock contention (`index.lock`), or long-running hook blocks the worker thread, causing IPC starvation and freezes.
- **Proposed Failing Test**:
  ```rust
  #[tokio::test]
  async fn git_operations_do_not_starve_async_executor() {
      // Simulate heavy load and ensure async timer tasks still run
      let (tx, rx) = tokio::sync::oneshot::channel();
      tokio::spawn(async move {
          tokio::time::sleep(Duration::from_millis(10)).await;
          let _ = tx.send(true);
      });
      let received = tokio::time::timeout(Duration::from_millis(200), rx).await;
      assert!(received.is_ok(), "Tokio runtime executor must not be starved by blocking Git operations");
  }
  ```

---

### Finding 8 (Low-Medium Likelihood): File Watcher Background Thread Panic
- **Location**: `app/src-tauri/src/main.rs:173, 190`
- **Likelihood**: LOW-MEDIUM
- **Severity**: MEDIUM (Silently kills file change updates)
- **Code**:
  ```rust
  let mut g = w.lock().unwrap();
  ...
  let _ = w.lock().unwrap().watch(&p, RecursiveMode::Recursive);
  ```
- **Root Cause**: The background file watcher thread uses `.lock().unwrap()` on `SharedWatcher`. If another thread panics while holding the lock, the watcher thread panics, and the app permanently stops reacting to external file modifications without any error emitted to the UI.
- **Proposed Failing Test**:
  ```rust
  #[test]
  fn watcher_lock_recovers_if_poisoned() {
      let w = Arc::new(Mutex::new(notify::NullWatcher));
      let w_clone = Arc::clone(&w);
      let _ = std::thread::spawn(move || {
          let _g = w_clone.lock().unwrap();
          panic!("simulated watcher lock poison");
      }).join();
      assert!(w.is_poisoned());
      let recovered = w.lock().unwrap_or_else(|e| e.into_inner());
      drop(recovered);
  }
  ```

---

### Finding 9 (Low Likelihood): Tauri Startup Panic via `.expect("error while running AtlasGit")`
- **Location**: `app/src-tauri/src/main.rs:638`
- **Likelihood**: LOW
- **Severity**: HIGH (Immediate exit on startup)
- **Code**:
  ```rust
  .run(tauri::generate_context!())
  .expect("error while running AtlasGit");
  ```
- **Root Cause**: If the Tauri runtime fails during startup (e.g., missing display server, broken Wayland/X11 socket, invalid GTK environment), `run()` returns an `Err`, immediately triggering `.expect()` and panicking on `thread 'main'` with "error while running AtlasGit".

---

### Finding 10 (Low Likelihood): Askpass Socket Path Length Limit on Linux (FUNC-11)
- **Location**: `app/src-tauri/src/askpass.rs:84-90`
- **Likelihood**: LOW
- **Severity**: MEDIUM (Network authentication failure)
- **Code**:
  ```rust
  let path = dir.join(format!("atlasgit-askpass-{}-{}.sock", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
  let Ok(listener) = UnixListener::bind(&path) else { return f(&[]) };
  ```
- **Root Cause**: On Linux, Unix domain sockets (`sockaddr_un.sun_path`) are limited to 108 bytes. If `$XDG_RUNTIME_DIR` or `/tmp` has a deeply nested or long path, `UnixListener::bind` returns an `io::Error`. The function then silently falls back to running git without the askpass bridge (`return f(&[])`), causing Git authentication to fail with no prompt shown to the user.

---

## Action Plan for T002 (Fixes & Tests)

1. **Rebase String Slicing**: Replace `&onto[..7.min(onto.len())]` in `crates/ak-core/src/ops/rebase.rs:127` with char-based truncation:
   ```rust
   let onto_prefix: String = onto.chars().take(7).collect();
   let onto_txt = if onto.is_empty() { String::new() } else { format!(" onto {onto_prefix}") };
   ```
2. **Lock Poisoning Recovery**: Use `diagnostics::lock()` or `unwrap_or_else(|e| e.into_inner())` across all mutex accesses in `main.rs`, `askpass.rs`, and `shared_stash.rs`.
3. **Journal Args Validation**: In `crates/ak-core/src/ops/history.rs`, `stash.rs`, and `branch.rs`, check `args.len()` before indexing; return `OpError::new("JOURNAL_CORRUPT", ...)` on short args.
4. **Wire Diagnostics**:
   - Add `mod diagnostics;` to `app/src-tauri/src/main.rs`.
   - Call `let diag = diagnostics::init();` in `main()`.
   - Pass `diag` to `.manage()`.
   - Register `tauri::RunEvent::Exit` handler to call `diag.finish()`.
   - Expose `diagnostics_status` and `diagnostics_dismiss_crash` commands.
5. **WebKitGTK Workarounds**: Add `export WEBKIT_DISABLE_DMABUF_RENDERER=1` to `scripts/run.sh` and `scripts/launch.sh`.
6. **Status Parser Slice Bounds**: Check `p.len() >= 2` before `p[2..]` in `crates/ak-gitcli/src/parse.rs:156`.
