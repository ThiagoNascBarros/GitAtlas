---
status: draft
feature: Release checklist (cut an Git Atlas release)
spec: docs/specs/release-docs.md
date: 2026-10-16
---

# Release checklist

Follow the steps in order. Copy this list into the release pull request and tick the boxes there.
`<version>` is the new version, for example `0.1.0`. Rules from `docs/HANDOFF.md` still apply:
heavy processes one at a time (check `free -m` first), never `-c safe.directory=*`, nothing
marked `approved`.

Anything marked *to be verified* has not been proven yet: do not claim it works until this
checklist has run it.

## 1. Prepare the release branch

- [ ] Owner decisions that block a real release are settled (`docs/specs/mvp-phase5.md` §8 and
      §8.1): release version, pt-BR glossary, Flathub approach, deb package/desktop file name,
      metainfo release type, `--filesystem=home`, Flatpak Git source pin.
- [ ] Create the branch from an up-to-date `develop` (Git flow):

      git switch develop && git pull
      git switch -c release/<version>

- [ ] Bump the version in all four places (it is `0.1.0` today):
  - [ ] `Cargo.toml`: `[workspace.package] version` (all three crates inherit it).
  - [ ] `app/src-tauri/tauri.conf.json`: `"version"`. `scripts/package.sh` and
        `scripts/package-flatpak.sh` read it for the file names.
  - [ ] `app/ui/package.json`: `"version"`.
  - [ ] `packaging/linux/dev.atlasgit.app.metainfo.xml`: `<release version="<version>"
        date="YYYY-MM-DD" type="stable">` with a short description. Today it is `0.1.0`,
        `2026-10-16`, `type="development"`.
  - [ ] Run `cargo build -p atlasgit --features custom-protocol` once so `Cargo.lock` picks up
        the new version, and commit `Cargo.lock` with the rest.
- [ ] `CHANGELOG.md`: rename `## [Unreleased]` to `## [<version>] - YYYY-MM-DD` and add a new
      empty `## [Unreleased]` above it.
- [ ] Owner decision on formatting: `cargo fmt --all -- --check` fails on code that predates
      Phase 5 (565 hunks; CI does not run it). Either add a `rustfmt.toml` that matches the
      current style, reformat in a separate pull request, or accept it for this release. Do not
      reformat inside the release branch.

## 2. CI is green

- [ ] Open the pull request `release/<version>` → `main`. The **CI** workflow
      (`.github/workflows/ci.yml`) must pass:
  - [ ] Core on Ubuntu 22.04 and 24.04 with the system Git, and on Ubuntu 24.04 with Git
        **2.20.5** (the oldest supported version).
  - [ ] App job: UI tests, UI build, `node scripts/contrast.mjs`, Tauri build, and packaging
        validation (`desktop-file-validate`, `appstreamcli validate --no-net`).
- [ ] SonarCloud has no new finding. The reliability finding from PR #2 is still open and needs
      the owner's login.

## 3. Local checks

Run one at a time. Test counts must not go below the Phase 5 numbers (Rust 120 passed, 2
ignored; UI 81 passed).

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `pnpm --dir app/ui test`
- [ ] `pnpm --dir app/ui build`
- [ ] `node scripts/contrast.mjs` (0 failures)
- [ ] `desktop-file-validate packaging/linux/dev.atlasgit.app.desktop` and
      `appstreamcli validate --no-net packaging/linux/dev.atlasgit.app.metainfo.xml`

## 4. Benchmarks

- [ ] Build the fixture: `tests/fixtures/make.sh wide-100k` (writes
      `tests/fixtures/out/wide-100k`).
- [ ] Open it with the release build: `scripts/run.sh --release tests/fixtures/out/wide-100k`.
- [ ] Idle memory: PSS ≤ 250 MB, measured the same way as ADR-0001 (last value: ~231 MB PSS,
      ~390 MB RSS).
- [ ] First paint: press Ctrl+Shift+P for the performance overlay and read "first paint";
      compare with the ADR-0001 target.
- [ ] Write the numbers in the release pull request, `docs/HANDOFF.md` and ADR-0001 in the vault.

## 5. Build the packages

Prefer the release workflow: its .deb and AppImage are built on Ubuntu 22.04 (glibc 2.35), so
they run on 22.04 and newer. A package built on your machine needs at least your glibc.

- [ ] Run the **Release packages** workflow (`.github/workflows/release.yml`) on the release
      branch: `gh workflow run release.yml --ref release/<version>` (or the "Run workflow" button).
      It only uploads artifacts (`atlasgit-deb`, `atlasgit-appimage`, `atlasgit-flatpak`); it
      never creates a release or pushes a tag.
- [ ] Check the Flatpak job's smoke test: `git --version` reports 2.56.0 and `ssh -V` runs inside
      the sandbox. The Flatpak has never been built outside this workflow (Flathub was not
      reachable from the Phase 5 build container): *to be verified* on its first run.
- [ ] Or build locally: `scripts/package.sh` (.deb + AppImage) and `scripts/package-flatpak.sh`
      (needs flatpak, flatpak-builder and the Flathub remote).
- [ ] Download the artifacts and keep their SHA-256 sums (`sha256sum`) for the release notes.

## 6. Manual test of each package

On a clean machine or VM (Ubuntu 22.04 for the .deb and AppImage; any distribution with
Flatpak for the bundle). Tick each line per package: .deb / AppImage / Flatpak.

- [ ] Install: `sudo apt install ./AtlasGit_<version>_amd64.deb`;
      `chmod +x` and run the AppImage; `flatpak install --user AtlasGit-<version>.flatpak`.
- [ ] Launch from the application menu: the Git Atlas icon shows in the menu and in the dock or
      task bar (not a generic icon), on X11 and on Wayland. Window class is `atlasgit`
      (`StartupWMClass`).
- [ ] Open a repository with **Open repository…** and with `atlasgit /path/to/repo`.
- [ ] PRD criteria 4 and 7 (still pending from Phase 2): stage and unstage a file, stage a hunk,
      commit, amend, Undo/Redo of each, discard + Undo, a file edited in another editor shows up
      by itself, commit with no `user.name`/`user.email` shows the identity message and fix.
- [ ] Shared stash, PRD criterion 14: a local bare remote (`git init --bare`) and two clones.
      Share from clone A, fetch and apply in clone B, **Create branch…** in B, then update,
      rename, unshare and reshare from A; a remote that refuses custom refs shows the notice.
- [ ] Fetch and push over HTTPS with no credential helper: the askpass dialog asks, **Cancel**
      stops cleanly, a wrong password gives **The remote refused your credentials.**
- [ ] Fetch and push over SSH with the key loaded in `ssh-agent`; also a key with a passphrase
      and a host not yet in `known_hosts` (both prompts come through the dialog).
- [ ] Signed commit with `commit.gpgsign=true`; `git log --show-signature -1` shows a good
      signature. In the Flatpak this goes through the host gpg-agent: *to be verified*.
- [ ] Language: switch to **Português (Brasil)** in the picker, check the main view, a menu, a
      dialog and an error; restart: the choice is kept; **Follow system** follows `LANG`.
- [ ] Keyboard-only walkthrough of `docs/a11y/audit.md` §5 (no mouse after launch).
- [ ] Orca spot check, `docs/a11y/audit.md` §7 items 1 and 2: graph rows are read, Shift+F10 and
      the Menu key open exactly one in-app menu (no WebKit menu on top).
- [ ] Flatpak only (*to be verified*): a repository outside home after
      `flatpak override --user --filesystem=/path dev.atlasgit.app`; a credential helper set by
      absolute path falls back to the dialog; `credential.helper=libsecret` stores the password
      in the keyring; settings from `/etc/gitconfig` are not applied.
- [ ] .deb only: `dpkg -s atlas-git` shows `Depends: git (>= 1:2.20), …`; `sudo apt remove
      atlas-git` removes the menu entry.
- [ ] AppImage only: works with the system Git; without FUSE with `APPIMAGE_EXTRACT_AND_RUN=1`.
- [ ] Write the results (pass/fail per line and package) in the release pull request.

## 7. Publish

- [ ] AppStream screenshots: take them from the real app (not the dev mock), host them at a
      stable URL, add `<screenshots>` to the metainfo and validate again. Flathub's linter
      requires them.
- [ ] Flathub submission: later (owner question: from-source manifest with vendored cargo/pnpm
      sources vs the current binary repack).
- [ ] Merge the pull request `release/<version>` → `main`.
- [ ] Tag the merge commit on `main` and push the tag (owner only):

      git tag -a v<version> -m "Git Atlas <version>"
      git push origin v<version>

      The tag starts the release workflow again; its artifacts are the release files.
- [ ] The owner creates the GitHub release by hand and attaches the .deb, AppImage and Flatpak
      artifacts, with the CHANGELOG section and the SHA-256 sums as notes.
- [ ] Merge `main` back into `develop` by pull request.

## 8. After the release

- [ ] Update `docs/HANDOFF.md` ("State": released version, benchmark numbers, test results,
      anything still open).
- [ ] Update the vault: EN first, then PT (mark a PT file `outdated` if it is not updated in the
      same change). Never mark anything `approved`.
