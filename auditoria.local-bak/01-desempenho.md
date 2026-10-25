# 01 — Performance (area 1)

## Limitações
- Static analysis only. The app was not run interactively and no profiler, fps or RSS measurement was taken by this audit. Heavy fixture generation (`tests/fixtures/make.sh`) was not run (7 GB RAM host).
- Numbers quoted come from the committed `tests/bench/report-wide-100k.json` (wide-100k only) and from code reading. Claims about UI cost (DOM nodes, re-parses) are inferred from the code, not measured.

## Findings

### [PERF-1] Blocking git calls run inline inside `async` Tauri commands
- Severidade: MÉDIA
- Item da matriz: NF-002 (partial: 60 fps / responsiveness not verified)
- Local: app/src-tauri/src/main.rs:288-505 (`repo_open`, `repo_refresh`, `graph_rows`, `commit_details`, `diff_file`, `diff_worktree`, `status_get`, `op_*`, `rebase_*`, `journal_*`)
- Evidência: only `op_net` (main.rs:503) and `shared_stash.rs:53` use `tauri::async_runtime::spawn_blocking`. All other commands are `async fn` that call `std::process::Command::output()` synchronously, e.g. `diff_file` -> `reader.file_diff(...)` (git show), `status_get` -> `reader.status()` (git status). `grep -n spawn_blocking app/src-tauri/src` returns just those two places. Sync commands (`graph_window`, `repo_close`, `get_initial_path`) are cheap (in-memory encode of at most 2000 rows).
- Por que importa: these block Tokio worker threads (not the UI thread). A slow `git status` / `git diff` / rebase on a big repo occupies a worker; with a few concurrent calls (status after watcher event + graph chunk rows + diff) the pool can saturate and other commands queue. The UI stays responsive only while workers are free. No lock is held during git (see "O que já está bom").
- Sugestão de correção: wrap git-calling bodies in `spawn_blocking` (a small helper like the one in shared_stash.rs:53), or make `Git::run` async via `tokio::process`.
- Esforço estimado: M

### [PERF-2] Every refresh is a full reload; each commit invalidates the on-disk cache
- Severidade: MÉDIA
- Item da matriz: NF-001, NF-002
- Local: app/src-tauri/src/main.rs:182-280 (`load`), app/src-tauri/src/main.rs:302 (`repo_refresh`), app/ui/src/App.tsx:181-232 (`refreshRepo`, `run`, `undo`, `redo`), crates/ak-core/src/cache.rs:28
- Evidência: `repo_refresh` calls `load()`: refs, head, `rev-parse --absolute-git-dir`, fingerprint, cache lookup, status, `Topology::from_commits`, `graph::layout`, `Engine::open`, `stashes()`. The fingerprint hashes every ref target and HEAD (cache.rs:28-35), so any commit/checkout/fetch changes it and the whole history is re-read and re-serialised (`cache::save`). Bench `git_read` = 576.58 ms for 100k commits (report-wide-100k.json) is the per-op floor for this path on a wide repo; layout (4 ms) and topology (14.7 ms) are negligible. In App.tsx `run()` does `await refreshRepo(...)` then `await loadStatus(...)` serially, and `load()` already ran `status()` once, so status runs twice per op. `refreshRepo` also resets `selected/sha/diffPath`.
- Por que importa: on a 100k-commit repo each commit/checkout/pull costs about 0.6 s+ of git read plus a full JSON write, repeated per operation; on larger repos it scales linearly. No incremental update (append new commits).
- Sugestão de correção: incremental history update when new tip descends from old tip (read only `old..new`, prepend); skip graph refresh on ops that do not move refs; run `refreshRepo` and `loadStatus` in parallel; reuse the status already read in `load`.
- Esforço estimado: G

### [PERF-3] File watcher: trailing-only debounce, watcher rebuilt on every refresh, `.git` and nested ignored dirs recursed
- Severidade: MÉDIA
- Item da matriz: fora da documentação (watcher; related NF-002)
- Local: app/src-tauri/src/main.rs:129-208 (`setup_watcher`), main.rs:296,306
- Evidência: (a) debounce loop `while let Ok(m) = rx.recv_timeout(Duration::from_millis(350)) { handle(m); }` has no maximum wait, so continuous events (build output in a non-ignored dir, a long checkout) postpone `repo-changed` indefinitely. (b) `start_watcher` is called in both `repo_open` and `repo_refresh`: each refresh spawns `git ls-files --others --ignored --exclude-standard --directory -z`, re-registers recursive inotify watches over the whole tree and starts a new thread; the old watcher thread ends only when its channel closes. (c) ignore detection is top-level only: `packages/x/node_modules` or `target/` inside a subfolder is watched recursively; `.git` is not ignored, so `.git/objects/**` is watched and filtered in `relevant()` only after the event is delivered. (d) `is_ignored_dir` runs `git check-ignore` per new top-level dir inside the watcher thread. Positive: `GIT_OPTIONAL_LOCKS=0` (exec.rs:90) prevents `git status` from touching `.git/index` and re-triggering the watcher.
- Por que importa: inotify watch exhaustion (default 8192-65536 limits) on big monorepos, wasted CPU on every refresh, and possible starvation of refresh during build bursts. `watch-failed` is the only fallback; window focus also triggers `loadStatus` (App.tsx:291), which mitigates stale state.
- Sugestão de correção: keep the watcher across refreshes (only reinstall on repo_open); add a max-wait (for example 2 s) to the debounce; watch only `.git/{HEAD,index,refs,MERGE_HEAD}` explicitly instead of the whole `.git`; honour nested ignores (for example `git check-ignore --stdin` batch or `ignore` crate).
- Esforço estimado: M

### [PERF-4] `git status --untracked-files=all` unbounded and rendered without virtualisation ("many files")
- Severidade: MÉDIA
- Item da matriz: NF-002 (many-files case not covered by bench)
- Local: crates/ak-core/src/read.rs:140-143, app/ui/src/features/wip/WipPanel.tsx:30,142-181, app/ui/src/App.tsx:162-180
- Evidência: status runs with `["status","--porcelain=v2","-z","--branch","--untracked-files=all"]`. WipPanel renders `<For each={groups().unstaged}>` / staged / conflicted as plain `<li>` lists, no windowing, no cap (the only `<For>` lists in features are in WipPanel, StashPanel, RefMenu, App refs panel; none is virtualised, only the graph is). `loadStatus` runs on every `repo-changed` and every window `focus`, then does `patchRepo` twice (status and `diffRev+1`, which also reloads the open diff via `createResource`).
- Por que importa: a build or `npm install` into a non-ignored dir, or a fresh clone with an unfinished `.gitignore`, can produce tens of thousands of entries: a large `git status` on every event and tens of thousands of DOM rows, all re-keyed on each update. Not measured.
- Sugestão de correção: virtualise or paginate the WIP lists past a threshold (for example 500) with a "show all" fallback; cap/collapse untracked directories (`--untracked-files=normal` plus lazy expansion); do not bump `diffRev` when status content did not change for the open file.
- Esforço estimado: M

### [PERF-5] Diff view: no size limit, no virtualisation, parse not memoised
- Severidade: MÉDIA
- Item da matriz: NF-002; F- diff view (see matrix diff rows)
- Local: app/ui/src/features/diff/DiffView.tsx:29,47-96, app/ui/src/features/diff/parse.ts:9-35, crates/ak-core/src/read.rs:111-126,210-223
- Evidência: `const parsed = () => parseUnified(patch() ?? "");` is a plain accessor, not `createMemo`. It is called in `parsed().binary` and `parsed().hunks` (lines 47, 49), so the whole patch is split and parsed on every evaluation of those bindings; `toSplit(h)` (line 81) is recomputed on every mode change. Each line becomes a `<tr>` with 4 `<td>` (up to 5 DOM nodes per line). The backend returns the full `git show/diff` output as one `String` with no size cap.
- Por que importa: a 50k-line diff (lockfile, minified or generated file) means roughly 250k DOM nodes plus repeated parsing; a perceptible freeze is expected on the UI thread. Not measured.
- Sugestão de correção: `createMemo` for the parse; truncate or paginate hunks above a line budget with "load more"; window long hunks (fixed row height); cap bytes in `file_diff` with an explicit "too large" message.
- Esforço estimado: M

### [PERF-6] On-disk cache: whole-file JSON rewrite, no eviction, written inline in `load`
- Severidade: BAIXA
- Item da matriz: NF-001 (first-paint path SDD §6 step 1)
- Local: crates/ak-core/src/cache.rs:44-66, app/src-tauri/src/main.rs:228-236
- Evidência: `save` serialises all `CommitRecord`s (sha, parents, author, email, time, summary) with `serde_json::to_string` into one string, then `fs::write`+`rename`; `load` reads and parses the whole file. File name is a `DefaultHasher` hash of the git dir, one file per repo under `$XDG_CACHE_HOME/atlasgit/graph/`, never evicted or size-bounded. Fingerprint changes on any ref move (PERF-2), so the cache mostly helps a cold start with unchanged refs.
- Por que importa: for 100k commits the JSON is tens of MB (size not measured), written synchronously inside `load` on every history change; stale files for deleted repos accumulate.
- Sugestão de correção: write on a background thread; binary format (bincode/rkyv) or compact columns; delete cache files not touched in N days; incremental append.
- Esforço estimado: M

### [PERF-7] Benchmark coverage is narrow; UI perf is not measured or gated
- Severidade: MÉDIA
- Item da matriz: NF-001, NF-002, NF-003, NF-004
- Local: tests/bench/report-wide-100k.json, crates/ak-core/examples/bench.rs, tests/fixtures/make.sh
- Evidência: report `"not_measured": ["UI first paint","UI scroll fps","app idle RSS (shell + WebKit)"]`; only repo `wide-100k` (25 lanes, 25 refs). No fixture or benchmark for: deep/linear history, many refs/tags, status with many files, large diff, watcher storms, repeated refresh. The only `tests/bench` content is that JSON (no runner). A Ctrl+Shift+P perf overlay exists (features/graph/perf.ts, X-001/X-004) but is manual. Core passes: layout 4.07 ms (limit 400), first window 0.077 ms (limit 300), 33 B/commit (limit 120), core peak RSS 76.8 MB.
- Por que importa: the passing NFR checks cover only the pure-Rust graph step; the costs that users actually feel (PERF-2 to PERF-5) have no regression guard. The HANDOFF figure PSS ~231 MB / RSS ~390 MB is close to the 250 MB idle goal (matrix NF-003).
- Sugestão de correção: add fixtures (deep, many-files-status, large-diff) and a small benchmark of `load()`, `status()`, `file_diff()`; CI job comparing against the committed JSON; an automated Playwright/webkit scroll-fps measure using perf.ts.
- Esforço estimado: M

### [PERF-8] Graph canvas redraw and per-row work not verified for cost; per-chunk `git log` process
- Severidade: BAIXA
- Item da matriz: NF-002
- Local: app/ui/src/features/graph/GraphView.tsx:78-82,90-110,119-250, crates/ak-core/src/read.rs:189-206
- Evidência: positives: canvas redraws only visible rows (`first..last`), backing store resized only when size changes (line 125-126), text metadata fetched in chunks (`CHUNK`) with `pending` de-dup, a11y mirror limited to viewport +- OVERSCAN (line 408). Open points: `refsByRow` builds each row's array with `[...(m.get(r.row) ?? []), r]` (quadratic per row, only matters with thousands of refs on one row); every metadata chunk spawns `git log --no-walk=unsorted --stdin` (rows_text); `token(...)` is called per row/pill inside `draw()` and its cost was not inspected.
- Por que importa: likely fine for 100k commits, but fast scrolling triggers many git process spawns and unmeasured per-frame style lookups.
- Sugestão de correção: measure with perf.ts on a 100k fixture; cache `token()` results per theme; prefetch/merge adjacent chunks; use `m.get(r.row)?.push` instead of spread.
- Esforço estimado: P

## O que já está bom
- No `Mutex` is held while git runs: `get()` clones an `Arc<Repo>` and drops the `repos` lock immediately (main.rs:316-318); `load()` runs with no lock held.
- Network ops (`op_net`, shared stash) use `spawn_blocking` and an askpass bridge, so credential prompts do not block the runtime.
- Graph: virtualised canvas, binary row window (33 B/commit), chunked lazy text, bounded `count.min(2000)`; `malloc_trim` after load; `shas` kept as plain `Vec<String>`, commits dropped after layout.
- Watcher skips top-level ignored dirs (node_modules, target), watches the root non-recursively, filters `.git` noise (`relevant`), and `GIT_OPTIONAL_LOCKS=0` avoids a self-triggering status loop.
- Cache is fingerprint-guarded and written atomically (temp + rename); any error means "no cache".
- Bench NFRs for graph layout/first window/bytes per commit pass with a large margin.

## Não verificado
- Real UI frame rate, first paint, idle RSS/PSS, cold start (NF-002, NF-003, NF-004).
- Size of the JSON cache file for 100k commits; behaviour of `token()` per frame; actual DOM node count and time for a 50k-line diff or 50k-file status.
- inotify watch counts on a large monorepo; `git status` duration on large repos.
- Tokio worker-pool size and actual queueing behaviour under concurrent commands.
- Tauri 2 execution thread of the sync commands (assumed main thread; they only encode in-memory data).
