# 06 — Packaging and distribution

## Limitações
- Analysis only. No package was built (`target/release/bundle` does not exist; `scripts/package*.sh` were read, not run). Flatpak, deb and AppImage runtime behaviour is therefore not observed.
- No network: the Git tag/commit pin and the GNOME 51 runtime could not be checked.
- Only read-only validators were run (`desktop-file-validate`, `appstreamcli validate --no-net`).
- Matrix IDs from `auditoria/00-matriz-documentacao-vs-codigo.md` (F-030, NF-005, NF-007, NF-009, AC-012).

## Findings

### [PKG-1] Flatpak manifest has never been built or run
- Severidade: ALTA
- Item da matriz: F-030, AC-012
- Local: packaging/flatpak/dev.atlasgit.app.yml; scripts/package-flatpak.sh; .github/workflows/release.yml (job `flatpak`)
- Evidência: docs/HANDOFF.md:46 "the **Flatpak was not built here** (Flathub blocked by the container proxy): its first build is the release workflow". docs/release/checklist.md §5: "The Flatpak has never been built outside this workflow ... *to be verified*". Manifest pins `runtime-version: '51'`, Git `tag: v2.56.0`, `commit: a018953688f1b10bddf91bff8747068f5f4746a4`; none could be checked offline. The Flatpak is documented as the primary distribution channel (HANDOFF.md:11).
- Por que importa: the primary channel is unproven. A wrong runtime version, a tag/commit mismatch or a Git build flag (`NO_RUST`, libsecret contrib build) fails only at the first tag/dispatch.
- Sugestão de correção: run `workflow_dispatch` of release.yml now (before tagging) and record the result; fix what breaks. Keep the smoke test (git, ssh) and add `gpg --version`.
- Esforço estimado: P

### [PKG-2] `--filesystem=home` gives the app the whole home, including ~/.ssh and ~/.gnupg
- Severidade: MÉDIA
- Item da matriz: NF-007, F-030
- Local: packaging/flatpak/dev.atlasgit.app.yml (`finish-args`)
- Evidência: `- --filesystem=home` with the comment "also exposes ~/.gitconfig, ~/.config/git, ~/.ssh (keys, known_hosts, config) and ~/.gnupg. The owner may narrow this later." Checklist §1 lists it as an open owner decision.
- Por que importa: the sandbox gives almost no isolation for private keys; Flathub reviewers usually challenge it. The WebKit view is the attack surface.
- Sugestão de correção: decide and document. Option: `--filesystem=xdg-documents`/`~/Projects`-style paths plus `--filesystem=~/.ssh:ro`, `~/.config/git:ro`, and the portal file chooser for other repos. Record the trade-off in docs/user/install.md.
- Esforço estimado: M

### [PKG-3] Flatpak credential/ssh/gpg paths rest on untested assumptions
- Severidade: MÉDIA
- Item da matriz: NF-009, AC-012
- Local: packaging/flatpak/dev.atlasgit.app.yml (`--socket=ssh-auth`, `--talk-name=org.freedesktop.secrets`, `--filesystem=xdg-run/gnupg:ro`); docs/user/troubleshooting.md
- Evidência: HANDOFF.md:47 "Flatpak sandbox limits ... marked 'to be verified'". Checklist §6: "In the Flatpak this goes through the host gpg-agent: *to be verified*"; "/etc/gitconfig not applied"; "credential helper set by absolute path falls back to the dialog". gpg-agent socket is mounted `:ro`; whether `gpg` in the GNOME runtime can connect to it (and whether the agent path is `xdg-run/gnupg` on all hosts) is unproven.
- Por que importa: signed commits, SSH remotes and keyring credentials may fail only inside the sandbox, silently from the user's view.
- Sugestão de correção: run the checklist §6 Flatpak-only lines on a clean VM and turn each "to be verified" into a result (or a documented limitation).
- Esforço estimado: M

### [PKG-4] Package/desktop-file naming does not match the app id
- Severidade: MÉDIA
- Item da matriz: F-030, NF-009
- Local: app/src-tauri/tauri.conf.json (`productName`, `bundle.linux.deb.desktopTemplate`); packaging/linux/dev.atlasgit.app.metainfo.xml (`<launchable type="desktop-id">dev.atlasgit.app.desktop`)
- Evidência: docs/specs/mvp-phase5.md:151 "deb package/desktop file names come from `productName` (`atlas-git`, `AtlasGit.desktop`), so the metainfo `launchable` only resolves in the Flatpak". `identifier` is `dev.atlasgit.app`; the repo desktop file is `dev.atlasgit.app.desktop`. The deb also installs icons as `dev.atlasgit.app.png`, which only matches because the template sets `Icon=dev.atlasgit.app`. Not confirmed on a built deb (none exists).
- Por que importa: software centers cannot link the metainfo to the installed launcher in deb/AppImage; a user who has both deb and Flatpak sees inconsistent ids; `StartupWMClass=atlasgit` vs the real window class is unverified (checklist §6).
- Sugestão de correção: build the deb and inspect `dpkg-deb -c`; if the file is `AtlasGit.desktop`, rename in a post-build step or in the bundle `files` map (`/usr/share/applications/dev.atlasgit.app.desktop`), and check the window class with `xprop`/`wmctrl` on X11 and Wayland.
- Esforço estimado: P

### [PKG-5] CI never builds the packages on pull requests
- Severidade: MÉDIA
- Item da matriz: F-030, AC-012
- Local: .github/workflows/ci.yml; .github/workflows/release.yml
- Evidência: ci.yml `app` job only runs `cargo build -p atlasgit --features custom-protocol` plus the two validators. `package.sh` and `package-flatpak.sh` only run in release.yml (`on: push tags v*` / `workflow_dispatch`). The CI build runs on ubuntu-24.04 only, while the release .deb is built on ubuntu-22.04.
- Por que importa: a change in `tauri.conf.json`, bundle files or the manifest breaks packaging only when a tag is pushed.
- Sugestão de correção: add a weekly/PR-path-filtered job (paths: `packaging/**`, `app/src-tauri/**`, `scripts/package*.sh`) that calls the release jobs without upload retention.
- Esforço estimado: P

### [PKG-6] CI Git matrix is implicit for 2.34/latest, and there is no benchmark job
- Severidade: MÉDIA
- Item da matriz: NF-005, NF-001, NF-003
- Local: .github/workflows/ci.yml:15-24; PRD AC-13
- Evidência: matrix = `os: [ubuntu-22.04, ubuntu-24.04]`, `git: [system]` plus one `include` with `git: "2.20.5"`. No entry pins 2.34 or "latest"; "system" is whatever the runner image ships (not asserted by the job; only `git --version` is printed). `cargo test -p ak-gitcli -p ak-core ...` excludes the `atlasgit` crate. No `bench`/`wide-100k` step exists (checklist §4 benchmarks are manual); no `cargo clippy`/`fmt` step although the checklist §3 requires clippy.
- Por que importa: AC-13 asks for 2.20/2.34/latest. Regressions in a middle or newest Git, or perf regressions on 100k commits, are not caught.
- Sugestão de correção: add explicit entries `git: "2.34.1"` and `git: latest` (built from source or git-core PPA), assert the version, add a clippy step, and a manual-trigger benchmark job that stores the report.
- Esforço estimado: M

### [PKG-7] No update strategy
- Severidade: MÉDIA
- Item da matriz: F-030 (fora da documentação for updates)
- Local: app/src-tauri/tauri.conf.json (no `plugins.updater`); docs/specs/packaging.md:235
- Evidência: packaging.md:235 "harmless today (no updater)". No updater plugin in Cargo.toml/tauri.conf.json; no AppImageUpdate/zsync info, no apt repository, no Flathub remote yet (checklist §7 "Flathub submission: later"). docs/user/install.md says there is no public download page.
- Por que importa: deb and AppImage users get no update path; only Flathub would provide one, and it is not set up. Security fixes (WebKitGTK is bundled in the AppImage) would not reach users.
- Sugestão de correção: state the strategy in docs/user/install.md (Flathub first; deb/AppImage manual) and decide whether to publish a zsync AppImage or an apt repo.
- Esforço estimado: M

### [PKG-8] Release workflow has no integrity or consistency checks
- Severidade: MÉDIA
- Item da matriz: F-030
- Local: .github/workflows/release.yml; Cargo.toml `rust-version = "1.80"`
- Evidência: no `sha256sum`/checksum or signing step (checklist §5 asks the owner to compute sums by hand); no check that tag `v*` equals `tauri.conf.json` version (version lives in four manual places, checklist §1); actions are tag-pinned (`actions/checkout@v4`), not SHA-pinned; `dtolnay/rust-toolchain@stable` is unpinned and `rust-version = 1.80` is never tested; `pnpm/action-setup` `version: 12`.
- Por que importa: a tag/version mismatch produces wrongly named artifacts; unsigned artifacts with no checksums cannot be verified by users; MSRV claim is untested.
- Sugestão de correção: add a step comparing `${GITHUB_REF_NAME#v}` with the three version files, emit `SHA256SUMS` as an artifact, add a CI job on toolchain `1.80`.
- Esforço estimado: P

### [PKG-9] AppStream metainfo is incomplete for store submission
- Severidade: BAIXA
- Item da matriz: F-030
- Local: packaging/linux/dev.atlasgit.app.metainfo.xml
- Evidência: `appstreamcli validate --no-net` -> "Validação bem sucedida." (rc=0). `grep screenshot` finds nothing; release is `type="development"` dated 2026-10-16; checklist §7 states Flathub's linter requires screenshots.
- Por que importa: valid but not submittable to Flathub or shown well in GNOME Software.
- Sugestão de correção: add `<screenshots>` from the real app at stable URLs; switch release type to `stable` at tag time.
- Esforço estimado: P

### [PKG-10] Dev shortcut script diverges from the packaged desktop entry
- Severidade: BAIXA
- Item da matriz: NF-009
- Local: scripts/install-shortcut.sh; scripts/launch.sh; packaging/linux/dev.atlasgit.app.desktop
- Evidência: shortcut writes `~/.local/share/applications/atlasgit.desktop` with `Icon=atlasgit`, `Exec=$bin`, `Path=$root`, no `GenericName`/`Keywords`/pt_BR; launch.sh rebuilds with `scripts/run.sh --release` when sources are newer. Packaged entry is `dev.atlasgit.app.desktop`, `Icon=dev.atlasgit.app`. Root path is injected via `sed "s|^root=.*|root=\"$root\"|"` (breaks if the path contains `|` or `&`).- Por que importa: installing the deb next to the shortcut shows two "AtlasGit" entries; the launcher needs the full toolchain and a source tree at login, which end users do not have. Fine for development, misleading if linked from user docs.
- Sugestão de correção: label it dev-only in README (it already says "rebuilds"), reuse `Icon=dev.atlasgit.app`, and quote the path safely.
- Esforço estimado: P

### [PKG-11] Stale `target/` build failure is not documented
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: README.md; docs/HANDOFF.md; auditoria/00-linha-de-base.md
- Evidência: baseline: `cargo build --workspace` FAIL rc=101, tauri build script read `/home/thigas/.../AtlasKraken/target/.../app_hide.toml` (target from the old path). `grep -rn "cargo clean"`, "stale" in README.md and docs/ returns nothing about it (only the rename note in README.md:7).
- Por que importa: any contributor with a pre-rename or moved checkout hits an opaque error on the first build.
- Sugestão de correção: add one troubleshooting line to README/HANDOFF: after moving/renaming the repo run `cargo clean -p atlasgit` (or delete `target/`).
- Esforço estimado: P

### [PKG-12] Licensing headers are mostly consistent, with small gaps
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: Cargo.toml:8, app/ui/package.json:5, LICENSE, packaging/flatpak/dev.atlasgit.app.yml
- Evidência: `license = "Apache-2.0"` in workspace and package.json; LICENSE is the Apache 2.0 text (202 lines); metainfo `project_license` Apache-2.0; tauri `licenseFile` set. Tracked files without SPDX header: `app/ui/src/features/wip/status.test.ts`, packaging/linux/dev.atlasgit.app.desktop, the three Cargo.toml files, `.cargo/config.toml`, index.html. No NOTICE or third-party license list although the Flatpak builds Git (GPL-2.0) and the AppImage bundles WebKitGTK/GTK (LGPL).
- Por que importa: inconsistent policy (most `.rs/.sh/.yml` carry SPDX) and no notice for redistributed GPL/LGPL components.
- Sugestão de correção: add the header to the listed files, and a short THIRD-PARTY note (Git GPL-2.0 source offered via the pinned tag; LGPL libs in AppImage).
- Esforço estimado: P

### [PKG-13] Release profile is minimal and unmeasured
- Severidade: BAIXA
- Item da matriz: NF-004
- Local: Cargo.toml `[profile.release]`
- Evidência: `lto = "thin"`, `codegen-units = 4`; no `strip`, `panic`, `opt-level` entries. Bundle `version` 0.1.0 matches workspace. No binary or package size is recorded anywhere (no built artifact exists).
- Por que importa: package size/startup (NF-004 not measured) cannot be tuned or tracked.
- Sugestão de correção: measure the stripped release size once; consider `strip = true`; record size per release.
- Esforço estimado: P

## O que já está bom
- `desktop-file-validate` and `appstreamcli validate --no-net` pass (rc=0) and run in CI and release.
- Release job builds on ubuntu-22.04 with a documented glibc 2.35 floor, uses no secrets, only `contents: read`, and never publishes.
- deb `depends: git (>= 1:2.20)` is asserted in CI (`grep -F`); icons 32–512 px exist and are installed under hicolor with the app id.
- Flatpak Git is pinned by tag and commit; docs mark unverified sandbox claims as "to be verified".
- Checklist matches the code on versions, the 2.20.5 CI entry and the Flatpak never-built status.
- Single version `0.1.0` is consistent in Cargo.toml, tauri.conf.json and metainfo.

## Não verificado
- Any real build of .deb, AppImage or Flatpak; installation on Ubuntu 22.04/24.04; Wayland vs X11 behaviour (AC-012).
- Actual deb contents and file names, and the real window class vs `StartupWMClass=atlasgit`.
- Validity of the Git tag `v2.56.0` and commit hash, and of the GNOME 51 runtime.
- Whether `gpg`, `ssh`, `git-credential-libsecret` work inside the sandbox.
- Which Git versions the runner images "system" actually use.
- Binary size and cold-start time; Debian 12 compatibility claim.
