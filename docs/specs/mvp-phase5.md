---
status: draft
feature: Phase 5 — MVP release (overview)
refs: docs/HANDOFF.md "Next phases", roadmap `en/04-plan/roadmap.md` (Obsidian vault, not in the repo)
date: 2026-10-15
---

# Phase 5 — MVP release: overview, dependency map and coordination

This file is the entry point for the remaining MVP work. Each sub-phase has its own spec; this
file holds what must stay **centralized**: the capability map, the build order, the shared
contracts, the file ownership per agent and the integration checks.

## 1. Where the MVP stands (evidence, 2026-10-15)

| Phase | State | Evidence |
|---|---|---|
| 1 — Read-only viewer | Done | `crates/ak-core/{read,read_gix,cache,graph}`, `features/graph`, golden tests `crates/ak-core/tests/graph.rs` |
| 2 — Operation engine | Done (owner manual test of the packaged app pending) | `ops/{mod,lock,journal}.rs`, `tests/ops.rs` |
| 3 — Branching & merge | Done (same manual test pending) | `ops/{branch,merge,stash}.rs`, `features/{refs,stash,conflict}` |
| 4 — Remotes & rebase | Done | `ops/{remote,history,rebase}.rs`, `askpass.rs`, `features/rebase` |
| 4 addendum — Shared Stash | Done, merged (PR #12) | `ops/shared_stash*.rs`, `tests/shared_stash.rs` (31 tests = spec §9), `features/shared-stash` |
| **5 — MVP release** | **Not started** | see §2 |

Baseline measured on this branch before any Phase 5 change: `cargo test --workspace` 120 passed
(2 ignored), `pnpm --dir app/ui test` 32 passed, `pnpm --dir app/ui build` ok,
`cargo build -p atlasgit --features custom-protocol` ok.

## 2. Phase 5 gaps found in the code

| Roadmap item (HANDOFF) | Current state |
|---|---|
| pt-BR translation | `app/ui/src/i18n.ts` has only `en` ("pt-BR is added in Phase 5"). No locale selection. Dates use the WebView default locale. 7 error codes have no UI text (`CONFLICT`, `INVALID_PATH`, `IO_ERROR`, `GIT_NOT_FOUND`, `NOT_A_REPO`, `READ_FAILED`, `UNKNOWN_REPO`); the open-repo error shows the raw core message. |
| Accessibility audit ("canvas graph needs the hidden focus list") | Graph is a `role="grid"` with no row elements; selection is announced through a live region only. Graph, ref rows and local stash rows open their context menus only with the mouse. Dialogs are `<dialog open>` (non-modal, no focus trap, no `aria-modal`). No audit document. |
| Flatpak (primary), AppImage, .deb, `.desktop` | `tauri.conf.json` bundles `deb` + `appimage` with Tauri's generated desktop entry. No Flatpak manifest, no AppStream metainfo, no canonical `.desktop` file (only the one written by `scripts/install-shortcut.sh`). CI never bundles. |
| User docs | Only `docs/user/shared-stash.md`. |
| Release checklist | None. No CHANGELOG. |

Out of the agents' reach (owner only): manual test of the packaged app (PRD criteria 4 and 7),
the SonarCloud finding on PR #2, reconciling the Obsidian vault (SDD §11 / PRD criterion 14),
Flathub submission, publishing a release.

## 3. Capability map

| Module id | Spec | Responsibility | Depends on |
|---|---|---|---|
| `i18n-pt-br` | `docs/specs/i18n-pt-br.md` | Locale selection, pt-BR strings, missing error texts, locale-aware dates | — |
| `packaging` | `docs/specs/packaging.md` | Desktop entry, AppStream metainfo, Flatpak manifest, deb/AppImage config, CI validation, release workflow | — |
| `accessibility` | `docs/specs/accessibility.md` | Graph hidden focus list, keyboard context menus, modal dialogs, audit report | `i18n-pt-br` (adds strings in both locales) |
| `release-docs` | `docs/specs/release-docs.md` | User guide, release checklist, CHANGELOG, README/HANDOFF update | `i18n-pt-br`, `packaging`, `accessibility` |

```
i18n-pt-br ──┐
             ├──> accessibility ──┐
packaging ───┼────────────────────┼──> release-docs
             └────────────────────┘
```

Build order: wave 1 = `i18n-pt-br` ∥ `packaging`; wave 2 = `accessibility`; wave 3 = `release-docs`.

Why `accessibility` is not parallel with `i18n-pt-br`: both change `app/ui/src/i18n.ts`,
`app/ui/src/App.tsx` and `app/ui/src/styles.css`, and every string the audit adds must exist in
the pt-BR table that `i18n-pt-br` creates. `packaging` touches none of the UI files.

## 4. Shared contracts and centralized decisions

These are fixed for every agent. Changing one requires going back to the coordinator.

1. **App id** `dev.atlasgit.app` (from `tauri.conf.json` `identifier`): Flatpak app id, desktop
   file `dev.atlasgit.app.desktop`, metainfo `dev.atlasgit.app.metainfo.xml`, icon name
   `dev.atlasgit.app`. Binary name stays `atlasgit`.
2. **Version** stays `0.1.0` in this phase (workspace `Cargo.toml`, `tauri.conf.json`,
   `app/ui/package.json`). The release version is an owner decision (§8).
3. **i18n API**: `t(key, vars)`, `errorText`, `errorFix`, type `Key` in `app/ui/src/i18n.ts`;
   `en` stays the source table in `i18n.ts`; pt-BR lives in `app/ui/src/locales/pt-BR.ts`, typed
   so a missing key fails `tsc`. Any phase that adds a key adds it to both tables.
4. **No new runtime or dev dependencies** (Rust crates or npm packages). Packaging tools
   (flatpak-builder, appstreamcli, desktop-file-validate) are CI/system tools, not project deps.
5. **No engine/core changes** in Phase 5 except where a spec says so (none currently does).
   No new Tauri command unless a spec says so (none currently does).
6. **Per-viewer UI preferences** use `localStorage` wrapped in try/catch with a working default
   (existing pattern: `features/split/Splitter.tsx`, `features/wip/WipPanel.tsx`), keys prefixed
   `ak.`.
7. **Specs are `status: draft`**; never mark anything `approved` (HANDOFF rule).
8. Code, identifiers, comments and docs in English; pt-BR only inside the pt-BR string table and
   the localized entries of the desktop/metainfo files.

## 5. File ownership per agent

| Agent | May create/modify | Must not touch |
|---|---|---|
| `i18n-pt-br` | `app/ui/src/i18n.ts`, `app/ui/src/locales/**`, `app/ui/src/App.tsx`, `app/ui/src/features/**` (only to route strings/dates through i18n), `app/ui/src/styles.css` (language picker only), `app/ui/src/*.test.ts`, `app/ui/src/devMock.ts` (only if needed), its spec's notes section, `docs/screenshots/phase5/<module>/` | `crates/**`, `app/src-tauri/**`, `.github/**`, `packaging/**`, `docs/user/**` |
| `packaging` | `packaging/**` (new), `app/src-tauri/tauri.conf.json` (bundle section only), `app/src-tauri/icons/**` (sizes generated from the existing icon), `.github/workflows/**`, `scripts/` (new packaging scripts only), its spec's notes section, `docs/screenshots/phase5/<module>/` | `app/ui/**`, `crates/**`, `app/src-tauri/src/**`, existing scripts' behaviour |
| `accessibility` | `app/ui/src/**`, `docs/a11y/**` (new), its spec's notes section, `docs/screenshots/phase5/<module>/` | `crates/**`, `app/src-tauri/**`, `packaging/**`, `.github/**` |
| `release-docs` | `docs/user/**`, `docs/release/**` (new), `CHANGELOG.md` (new), `README.md`, `docs/HANDOFF.md`, its spec's notes section, `docs/screenshots/phase5/<module>/` | all code, other specs |

Each agent works in its own git worktree and commits there; the coordinator merges into the
session branch in build order. Agents never push.

## 5.1 Screenshots (owner requirement, 2026-10-15)

Every module delivers **exactly 5 PNG screenshots** of what it implemented, committed under
`docs/screenshots/phase5/<module-id>/01-….png` … `05-….png` (this path is part of each agent's
ownership). UI shots come from the dev mock (`pnpm --dir app/ui exec vite` + `devMock.ts`)
driven by Playwright/Chromium at 1280×800, or from the real app under Xvfb; command output is
rendered to an image. The agent checks each image matches its caption and lists them in its
report; the coordinator re-checks them before merging. Playwright is not added to the project
(helper scripts stay outside the repo).

## 6. Integration and validation (coordinator, after each wave)

1. Merge the wave's branches; resolve conflicts only inside the owning agent's files.
2. Run, one heavy process at a time:
   - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --features ak-core/gix-reader`
   - `pnpm --dir app/ui test`, `pnpm --dir app/ui build`, `node scripts/contrast.mjs`
   - `cargo build -p atlasgit --features custom-protocol`
   - wave 1+: `desktop-file-validate` and `appstreamcli validate --no-net` on the packaging files
3. Check every acceptance criterion of the merged specs against the code, not the agent report.
4. Check the 5 screenshots exist and show the module's work (§5.1).
5. Check no file outside the agent's ownership changed (`git diff --stat <base>..<branch>`).
6. Regression: test counts never go down; baseline in §1.

## 7. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Git, SSH, credential helpers and GPG inside the Flatpak sandbox | Remote ops/signing fail in the Flatpak only | `packaging.md` §5: verify what the runtime ships, add finish-args, document limits; owner manual test |
| Flatpak/AppImage builds need network (runtimes, linuxdeploy) | Cannot be proven in a sandboxed container | Agent validates files offline and reports exactly what was and was not built; CI release workflow builds them |
| Hidden focus list could cost frame time on 100k commits | Breaks Phase 1 perf targets | Only visible rows rendered (`accessibility.md` §6); perf overlay check |
| pt-BR terminology disagreements | Owner rewrites strings | Glossary in `i18n-pt-br.md` §6 flagged for owner review |
| Docs drift from the UI | Wrong instructions | `release-docs` runs last and quotes the `en` table |

## 8. Open questions (owner)

Blocking for a real release, not for this phase's implementation:

1. Release version for the MVP (`0.1.0` or `1.0.0`)?
2. pt-BR glossary (`i18n-pt-br.md` §6): keep Git terms (commit, branch, stash, merge, rebase…) in English?
3. User guide in pt-BR as well, or EN only for the MVP? (Default in `release-docs.md`: EN only.)
4. Flathub: build from source (requires vendored cargo/pnpm sources) or keep the binary-repack manifest for local/CI bundles only? (Default: repack for now; Flathub later.)

## 8.1 Integration log

- Wave 1 (`i18n-pt-br` + `packaging`, merged): `cargo test --workspace` 120 passed;
  `cargo clippy --workspace --all-targets -- -D warnings` clean; UI tests 48 passed; UI build,
  contrast, Tauri build, desktop/metainfo validators ok.
- Pre-existing, not a Phase 5 regression: `cargo fmt --all -- --check` reports 565 hunks in Rust
  files no Phase 5 agent touched (no `.rs` changed since `5657e39`); CI does not run it. Owner
  decision: add a `rustfmt.toml` matching the current style, or reformat in a separate PR.
- Owner decisions raised by packaging: deb package/desktop file names come from `productName`
  (`atlas-git`, `AtlasGit.desktop`), so the metainfo `launchable` only resolves in the Flatpak;
  Tauri warns that an identifier ending in `.app` clashes with macOS bundles; Flatpak Git source
  pinned by tag + commit (kernel.org tarball blocked here); `--filesystem=home` may be narrowed.

## 9. Agent status

Updated by the coordinator.

| Agent | Phase | Status | Branch / commit |
|---|---|---|---|
| agent `i18n-pt-br` (worktree) | `i18n-pt-br` | done, validated, merged | merge `5ae6256` (UI tests 48, build, contrast, Tauri build ok; 5 shots checked) |
| agent `packaging` (worktree) | `packaging` | done, validated, merged (Flatpak bundle not built here: Flathub blocked by the container proxy; built by `release.yml`) | merge `a1c00ed` (.deb + AppImage built and launched, validators ok, 5 shots checked) |
| agent `accessibility` (worktree) | `accessibility` | running (from `5ae6256`) | — |
| — | `release-docs` | waiting (needs all) | — |
