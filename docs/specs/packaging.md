---
status: draft
feature: Linux packaging — Flatpak (primary), AppImage, .deb, desktop entry, AppStream
phase: 5 (module packaging)
overview: docs/specs/mvp-phase5.md
date: 2026-10-15
---

# Packaging for the MVP release

## 1. Objective

Owner decision (HANDOFF): distribute via **Flatpak/Flathub (primary) + AppImage + .deb** (snap
later). Today only `cargo build`/`scripts/run.sh` and a per-user shortcut script exist;
`tauri.conf.json` would bundle `deb` + `appimage` with a generated desktop entry, but nothing
is validated and there is no Flatpak. Result: reproducible packaging files in the repository, one
command per format, offline validation in CI, and a tag-triggered workflow that builds the three
artifacts for the owner to test and publish.

## 2. Scope

**In scope**
- Canonical desktop entry and AppStream metainfo (with pt-BR localized name/summary/description).
- Tauri bundle config for `.deb` (dependency on Git ≥ 2.20, canonical desktop entry) and AppImage.
- Flatpak manifest (app id `dev.atlasgit.app`) built from the release binary produced by the
  Tauri build ("repack" manifest), with sandbox permissions a Git client needs.
- Hicolor icons generated from the existing `app/src-tauri/icons/icon.png` (no new artwork).
- Scripts: `scripts/package.sh` (deb + AppImage) and `scripts/package-flatpak.sh` (bundle).
- CI: offline validation of desktop/metainfo files in the existing `app` job; new
  `.github/workflows/release.yml` (tags `v*` + manual dispatch) that builds deb, AppImage and a
  `.flatpak` bundle and uploads them as workflow artifacts.

**Out of scope**
- Flathub submission, publishing GitHub releases, signing packages, snap, RPM.
- A from-source Flathub manifest (vendored cargo/pnpm sources) — owner question 4 in the overview.
- Changes to app code, UI or core. Version bump (stays `0.1.0`).
- Screenshots for AppStream (need the real app; `release-docs` lists it in the release checklist).

## 3. Repository context

- `app/src-tauri/tauri.conf.json`: `productName` AtlasGit, `identifier` `dev.atlasgit.app`,
  `version` 0.1.0, bundle `targets: ["deb","appimage"]`, icon `icons/icon.png` (512×512 only),
  `category: DeveloperTool`, license file `../../LICENSE`.
- Binary name `atlasgit` (`app/src-tauri/Cargo.toml` package name). It takes an optional repo
  path as first argument (`main.rs`). It re-executes itself as Git's askpass helper when
  `ATLASGIT_ASKPASS_SOCK` is set (`askpass.rs`), so the binary must be able to run itself.
- All writes go through the `git` CLI (spawned by `ak-gitcli`, override `AK_GIT`); minimum Git
  2.20, read-only below. Remote operations rely on Git's credential helpers, ssh/ssh-agent and
  the askpass bridge; signing uses the user's `gpg`/ssh config.
- `scripts/install-shortcut.sh` writes a per-user `atlasgit.desktop` (`Categories=Development;
  RevisionControl;`, `StartupWMClass=atlasgit`). Keep it working; it is a dev convenience.
- `@tauri-apps/cli` is a devDependency of `app/ui` (`pnpm --dir app/ui tauri …`).
- CI (`.github/workflows/ci.yml`): `core` matrix incl. Git 2.20.5, `app` job builds UI and the
  Tauri crate on ubuntu-24.04 with WebKitGTK 4.1 dev libs.

## 4. Requirements

### 4.1 Functional

- **FR-1 Desktop entry** `packaging/linux/dev.atlasgit.app.desktop`: `Type=Application`,
  `Name=AtlasGit`, `GenericName=Git Client` (+`[pt_BR]` "Cliente Git"), `Comment=…`
  (+`[pt_BR]`), `Exec=atlasgit %f`, `Icon=dev.atlasgit.app`, `Terminal=false`,
  `Categories=Development;RevisionControl;`, `Keywords=git;vcs;version control;…`,
  `StartupWMClass` matching what the app window really reports (verify; `install-shortcut.sh`
  uses `atlasgit`), `StartupNotify=true`. Passes `desktop-file-validate` with no errors.
- **FR-2 AppStream** `packaging/linux/dev.atlasgit.app.metainfo.xml`: `component type=
  desktop-application`, `id` `dev.atlasgit.app`, `metadata_license` CC0-1.0,
  `project_license` Apache-2.0, `name`, `summary`, `description` (EN + `xml:lang="pt-BR"`),
  `developer` name "AtlasGit contributors" (owner may change), `url type="homepage"`
  `https://github.com/ThiagoNascBarros/AtlasGit`, `url type="bugtracker"` (…/issues),
  `launchable type="desktop-id"`, `content_rating type="oars-1.1"`, `categories`, `provides`
  `binary atlasgit`, `releases` with `0.1.0` (type development unless the owner sets a date),
  `requires`/`recommends` display length if the validator asks. Passes
  `appstreamcli validate --no-net` with no errors (warnings listed in §12; screenshots are the
  expected one).
- **FR-3 Icons** hicolor PNGs (at least 32, 48, 64, 128, 256, 512) generated from the existing
  icon (e.g. `pnpm --dir app/ui tauri icon app/src-tauri/icons/icon.png` or ImageMagick if
  present); installed as `dev.atlasgit.app.png` in the Flatpak; Tauri bundle `icon` list updated
  to the generated files.
- **FR-4 .deb** via Tauri: `bundle.linux.deb.depends` includes `git (>= 1:2.20)`; desktop entry
  from FR-1 (Tauri `desktopTemplate` or equivalent supported key — verify against the Tauri 2
  config schema in `app/src-tauri/gen/schemas` / the installed CLI); metainfo installed to
  `/usr/share/metainfo/` if Tauri supports extra `files`; section/priority sane.
- **FR-5 AppImage** via Tauri, same desktop entry and icon. Git is **not** bundled (uses the
  host Git; documented).
- **FR-6 Flatpak** `packaging/flatpak/dev.atlasgit.app.yml`:
  - runtime/sdk: the newest stable `org.gnome.Platform`/`org.gnome.Sdk` that ships
    WebKitGTK 4.1 (Tauri 2 requirement); record the version chosen and why.
  - `command: atlasgit`; installs binary, desktop file (renamed to the app id), metainfo, icons.
  - Source: the release binary built by `scripts/package.sh` (repack). The script, not the user,
    passes the path; no absolute paths in the manifest.
  - **Git in the sandbox**: check whether the runtime contains `git` ≥ 2.20 and an `ssh`
    client. If not, add modules building them from pinned upstream tarballs with `sha256`.
    Record what the runtime provides.
  - finish-args, each with a comment saying why: `--share=ipc`, `--socket=wayland`,
    `--socket=fallback-x11`, `--device=dri`, `--share=network` (fetch/push),
    `--filesystem=home` (repositories anywhere under home; owner may narrow later),
    `--socket=ssh-auth` (ssh-agent), `--talk-name=org.freedesktop.secrets` (libsecret credential
    helper), `--filesystem=xdg-run/gnupg:ro` (gpg-agent for signed commits) — drop any that the
    validation shows is useless, and do not add others without a reason.
- **FR-7 Scripts** (bash, `set -euo pipefail`, SPDX header like the existing scripts):
  - `scripts/package.sh`: installs UI deps (frozen lockfile), runs the Tauri release bundle for
    deb + AppImage, prints artifact paths.
  - `scripts/package-flatpak.sh`: requires a prior `package.sh` (or runs it), runs
    `flatpak-builder` into `target/flatpak/…`, exports a single-file bundle
    `target/flatpak/AtlasGit-<version>.flatpak`. Fails with a clear message if
    `flatpak-builder` or the runtime is missing.
- **FR-8 CI validation** (existing `app` job in `ci.yml`): install `desktop-file-utils` and
  `appstream`, run `desktop-file-validate` and `appstreamcli validate --no-net` on the files.
  Cheap steps only; no bundling in PR CI.
- **FR-9 Release workflow** `.github/workflows/release.yml`: on `push` of tags `v*` and
  `workflow_dispatch`; job(s) on ubuntu-24.04 (deb/AppImage built on the oldest supported Ubuntu
  in CI if glibc compatibility requires it — decide and record) that run the two scripts and
  upload `*.deb`, `*.AppImage`, `*.flatpak` with `actions/upload-artifact`. No release
  publishing, no secrets.

### 4.2 Non-functional

- No new Rust/npm dependencies. Tools come from apt/flathub in CI.
- Existing behaviour unchanged: `scripts/run.sh`, `scripts/install-shortcut.sh`, `cargo build`
  and current CI jobs keep working.
- Packages must not write anything to users' repositories at install time.

## 5. Sandbox and compatibility notes (to verify and record)

| Concern | Expected handling |
|---|---|
| Host credential helpers configured by absolute path (e.g. `/usr/lib/git-core/git-credential-libsecret`) | Not present in the sandbox → documented limitation; askpass bridge still works |
| `~/.gitconfig`, `~/.ssh`, `~/.config/git` | Reachable through `--filesystem=home` |
| `/etc/gitconfig` (system config) | Not visible in the sandbox → documented |
| Askpass re-exec of `/app/bin/atlasgit` | Must work inside the sandbox; verify the socket path is writable (`$XDG_RUNTIME_DIR`) |
| Repositories outside home (e.g. `/mnt`, `/media`) | Not reachable by default → documented; user can grant with Flatseal/`flatpak override` |

## 6. Implementation steps

1. Read the Tauri 2 config schema (`app/src-tauri/gen/schemas/*.json` or CLI `--help`) for
   `bundle.linux.deb` / `bundle.linux.appimage` keys; record the keys used.
2. FR-1 and FR-2 files; validate with `desktop-file-validate` / `appstreamcli validate --no-net`.
3. FR-3 icons; update `tauri.conf.json` bundle `icon`.
4. FR-4/FR-5 config; FR-7 `package.sh`; build locally (`cargo` release build is heavy: run it
   alone). Check `dpkg-deb -I` (Depends) and `dpkg-deb -c` (desktop, icons, metainfo).
   Try the AppImage build (`APPIMAGE_EXTRACT_AND_RUN=1` if FUSE is missing); if a download
   (linuxdeploy) is blocked by the network, record it as not built here.
5. FR-6 manifest + `package-flatpak.sh`. Try a real build if `flathub` and the runtime can be
   installed (`flatpak remote-add --user --if-not-exists flathub …`); if the network blocks it,
   validate the manifest with `flatpak-builder --show-manifest` and record it as not built.
6. FR-8 and FR-9 workflows; check YAML syntax (e.g. `python3 -c 'import yaml…'` if available).
7. Fill §12.

## 7. Test plan

| Check | How |
|---|---|
| Desktop entry valid | `desktop-file-validate packaging/linux/dev.atlasgit.app.desktop` exit 0 |
| Metainfo valid | `appstreamcli validate --no-net packaging/linux/dev.atlasgit.app.metainfo.xml` exit 0 |
| .deb metadata/content | `dpkg-deb -I` shows `git (>= 1:2.20)`; `dpkg-deb -c` lists binary, desktop, icons |
| .deb installs and launches | `apt install ./AtlasGit_*.deb` in the container; `atlasgit --help`-less smoke: start with `timeout` under `xvfb-run` if available, else record not run |
| AppImage | built (or blocked, recorded); `--appimage-extract` lists desktop + icon |
| Flatpak | bundle built (or manifest-only, recorded); `flatpak run --command=git dev.atlasgit.app --version` ≥ 2.20 when built |
| Regression | `pnpm --dir app/ui build`, `cargo build -p atlasgit --features custom-protocol`, existing CI steps unchanged |

## 8. Errors and edge cases

- Tauri CLI version differences in config keys → follow the installed CLI's schema, not docs from memory.
- glibc: binaries built on a newer Ubuntu do not run on older ones; choose the build image for
  deb/AppImage accordingly and record the minimum supported distro.
- WM class mismatch → shell shows a generic icon; verify with the real window if a display is
  available (`xprop WM_CLASS` under Xvfb), else record as owner check.
- `flatpak-builder` needs `--disable-rofiles-fuse` in containers without FUSE.

## 9. Impact on existing architecture

Adds a `packaging/` tree and two scripts; changes only the `bundle` section of
`tauri.conf.json` and the CI workflow files. No runtime code changes.

## 10. Definition of Done

- [ ] FR-1…FR-9 files exist; FR-1/FR-2 validators pass locally and in `ci.yml`.
- [ ] `.deb` built locally with the Git dependency and the canonical desktop entry (or a precise
      reason it could not be built here).
- [ ] AppImage and Flatpak: built, or manifest validated plus the exact blocking reason recorded.
- [ ] Release workflow exists, never publishes, uploads the three artifacts.
- [ ] No change outside the ownership list in `mvp-phase5.md` §5; no new dependency.
- [ ] §12 filled: Tauri keys used, runtime version, Git/ssh availability in the sandbox,
      finish-args kept, validator warnings, what was not verified.

## 11. Instructions for the implementing agent

- Read `AGENTS.md`, `docs/HANDOFF.md`, `docs/specs/mvp-phase5.md` and this spec; read
  `tauri.conf.json`, `scripts/*.sh`, `.github/workflows/ci.yml`, `app/src-tauri/src/askpass.rs`
  (self-exec) before writing files.
- Heavy processes one at a time (release builds, flatpak-builder).
- Do not touch `app/ui/**`, `crates/**`, `app/src-tauri/src/**`.
- Commit in small steps; do not push; do not create tags or releases. Report files changed, what
  was built and validated, what was blocked and why.

## 12. Implementation notes

(Filled by the implementing agent.)
