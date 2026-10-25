# Porting GitAtlas to macOS and Windows

**Status:** plan only. GitAtlas is built, tested and packaged on Linux (Ubuntu 22.04/24.04) only.
Nothing below has been compiled or run on macOS or Windows; every item marked *verify* is an
assumption read from the code, not a measured result.

Tauri 2 already targets both platforms, and the Rust crates `ak-gitcli` and `ak-core` only spawn the
`git` executable, so most of the work is in the shell around them: one hard compile blocker,
four path conventions, packaging and CI.

## 1. Blockers and fixes, in order

| # | Where | Problem | Fix | Platform |
|---|---|---|---|---|
| 1 | `app/src-tauri/src/askpass.rs:8-9` | Uses `std::os::unix::{fs::PermissionsExt, net::UnixListener}`. Does not compile on Windows. | Put the bridge behind a small transport (`trait AskpassTransport`) with the Unix socket as one impl and a Windows named pipe (for example the `interprocess` crate) as the other. Keep the 0600-equivalent: a pipe name with a random token, and reject other users via the default DACL. | Windows |
| 2 | `askpass.rs:84` | Socket dir is `$XDG_RUNTIME_DIR`, else `temp_dir()`. macOS has no `XDG_RUNTIME_DIR`; `$TMPDIR` is long and Unix socket paths are limited to 104 bytes there. | Use a short name under `temp_dir()` (the pid + counter name is already short) and assert `path.len() < 100`, falling back to `/tmp`. *verify* | macOS |
| 3 | `crates/ak-core/src/cache.rs:35-40` | Cache dir is `$XDG_CACHE_HOME` or `$HOME/.cache`. `HOME` is normally unset on Windows, so the cache is silently disabled. | Resolve with the OS convention: macOS `~/Library/Caches/GitAtlas`, Windows `%LOCALAPPDATA%\GitAtlas\cache`. Pass the directory in from Tauri (`app.path().app_cache_dir()`) so the core crate stays free of platform code. | both |
| 4 | `app/src-tauri/src/diagnostics.rs:41-48` | Log/crash dir falls back to `~/.local/state/gitatlas` on macOS. Windows already uses `%LOCALAPPDATA%`. | macOS: `~/Library/Logs/GitAtlas` (shown by Console.app). Add one test per platform to the existing `state_dir_from` table test. | macOS |
| 5 | `tauri.conf.json` `bundle.targets` | Only `deb` and `appimage`. No `icon.icns` / `icon.ico`. | Add `dmg` + `app` and `nsis` (+ `msi` if wanted). Generate icons with `pnpm tauri icon app/src-tauri/icons/icon.png`. Keep the `linux` block as is. | both |
| 6 | `ak-gitcli` / `AK_GIT` | `git` is found through `PATH`. A macOS app started from Finder does not get the shell `PATH`; `/usr/bin/git` may be the Xcode shim that opens an installer dialog. Windows has no `git` unless Git for Windows is installed and on `PATH`. | Resolve in order: `AK_GIT`, a saved setting, then known locations (`/opt/homebrew/bin/git`, `/usr/local/bin/git`, `/usr/bin/git`, `C:\Program Files\Git\cmd\git.exe`). Show the existing "Git ≥ 2.20 required" error with an install link if none works. Do not bundle Git in the first release. | both |

## 2. Behaviour to verify (no code change known to be needed)

- **Rebase:** `rebase.rs:53` sets `GIT_EDITOR=true`. Git for Windows runs editors through its bundled `sh`, so `true` should resolve; confirm with the rebase integration tests.
- **Untracked diff:** `read.rs:216` passes `/dev/null` to `git diff --no-index`. Git for Windows maps this path; confirm.
- **Line endings:** with `core.autocrlf=true` (Windows default) the conflict editor and the rebase todo file must round-trip CRLF. Add a fixture test.
- **Case-insensitive file systems** (macOS default, Windows): branch names `Feat/x` and `feat/x` collide in `.git/refs`; two paths differing only in case collapse in the file tree. Check the file tree key and the shared-stash ref names.
- **Long paths on Windows:** enable `core.longpaths` for operations GitAtlas starts, and test a path above 260 characters.
- **Shortcuts:** `App.tsx:273,277` and `WipPanel.tsx:209` test `e.ctrlKey` only. On macOS the convention is Cmd. Use `e.ctrlKey || e.metaKey` on macOS (behind a `isMac` helper with a unit test), and update the shortcut labels in `docs/user/keyboard-and-accessibility.md`.
- **Webview differences:** WKWebView (macOS) and WebView2 (Windows) replace WebKitGTK. The commit graph is a canvas, so check scrolling at HiDPI (`crisp()` assumes `devicePixelRatio`), and check `unicode-range` font loading for the bundled Open Sans. Windows needs the WebView2 runtime; the NSIS installer can bootstrap it (`webviewInstallMode`).
- **Tauri CSP** (`default-src 'self'`) already allows the bundled fonts; confirm the `tauri://` / `http://tauri.localhost` origins on each platform.
- **Window chrome:** macOS needs a standard menu (Tauri's default menu is enough for Cut/Copy/Paste, Quit) so text fields keep working.

## 3. Build, CI and release

1. **CI matrix** (`.github/workflows/ci.yml`): add `macos-14` (arm64) and `windows-latest`. Skip the `apt-get` steps there. Run `cargo test --workspace` and `pnpm vitest run` on both. The Git-minimum job (2.20) stays Linux only.
2. **Release workflow:** one job per target producing `.dmg` (arm64 and x64, or a universal build with `--target universal-apple-darwin`) and the NSIS `.exe`. Upload as artifacts like the Linux jobs.
3. **Signing** (needs accounts, so do it last):
   - macOS: Apple Developer ID certificate, hardened runtime, notarization with `notarytool`, then staple. Unsigned apps are blocked by Gatekeeper.
   - Windows: an Authenticode or Azure Trusted Signing certificate. Unsigned installers show SmartScreen warnings.
4. **Packaging files:** `packaging/linux` and `packaging/flatpak` stay Linux only. Add `packaging/macos` (entitlements, `Info.plist` overrides) and `packaging/windows` (NSIS hooks) only if needed.
5. **Docs:** extend `docs/user/install.md` with macOS and Windows, and the release checklist (`docs/release/checklist.md`) with the signing steps.

## 4. Suggested order

| Phase | Work | Done when |
|---|---|---|
| A | Items 3, 4, 6 and the shortcut helper, with tests | `cargo test --workspace` and `vitest` pass on macOS. |
| B | Item 2 (macOS socket length), CI on `macos-14`, `.dmg` unsigned | App opens a repo, commits, and shows the graph. |
| C | Item 1 (Windows askpass transport), CI on `windows-latest`, NSIS unsigned | A push over HTTPS prompts for credentials in the UI. |
| D | Behaviour checks from section 2 on real machines | Each bullet confirmed or turned into a test. |
| E | Signing, notarization, release checklist | Signed artifacts install without OS warnings. |

macOS is the cheaper first target: item 1 is Windows only and the askpass code already compiles on Unix.

## 5. Not decided

- Bundle Git, or require it? (Section 1, item 6 assumes require.)
- Universal macOS binary or two builds.
- Minimum OS versions (Tauri 2 needs macOS 10.15+ and Windows 10 with WebView2).
- Whether to ship an auto-updater (not in the Linux release either).
