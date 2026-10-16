---
status: draft
feature: User guide, release checklist and changelog for the MVP
phase: 5 (module release-docs)
overview: docs/specs/mvp-phase5.md
depends-on: i18n-pt-br, packaging, accessibility (all merged first)
date: 2026-10-15
---

# Release documentation — user guide, release checklist, changelog

## 1. Objective

The roadmap lists "user docs" and a "release checklist" for Phase 5. Only the Shared Stash has
a user page (`docs/user/shared-stash.md`). Result: a user guide that covers every MVP feature as
it exists in the code, a release checklist the owner can follow to cut the MVP, a CHANGELOG, and
README/HANDOFF reflecting the real state after Phase 5.

## 2. Scope

**In scope**
- `docs/user/` guide (EN): index, installation, getting started, history & diff, committing &
  undo, branches/merge/conflicts/stash, remotes & rebase & history operations, settings
  (theme, language, shared stash fetch mode), keyboard & accessibility, troubleshooting (error
  messages with fixes, read-only mode, Flatpak sandbox limits). Link the existing
  `shared-stash.md` (do not rewrite it; fix only facts that changed).
- `docs/release/checklist.md`: the steps to cut a release (§4 FR-3).
- `CHANGELOG.md` (Keep a Changelog format) with an `Unreleased` section summarising Phases 1–5.
- `README.md` status table/Run/Install sections; `docs/HANDOFF.md` state; `docs/specs/
  mvp-phase5.md` §9 left to the coordinator.

**Out of scope**
- pt-BR user guide (owner question 3; default EN only for the MVP).
- Screenshots (need the real app on a display; listed in the checklist).
- Code changes of any kind; marking anything `approved`; editing the Obsidian vault.

## 3. Repository context

- Facts must come from the code: UI labels from the `en` table in `app/ui/src/i18n.ts`,
  keyboard map from `docs/a11y/audit.md`, install/build commands from `scripts/package*.sh`,
  `packaging/**`, `.github/workflows/release.yml`, settings from `settings.rs` and
  `i18n.ts`/`theme.ts`.
- Style of the existing user page `docs/user/shared-stash.md`: short sections, bold UI labels,
  task-oriented, plain English, no marketing.
- HANDOFF rules (heavy processes one at a time, Git ≥ 2.20, no silent `.git` writes, Git flow,
  commit trailers) must be preserved verbatim where they still apply.

## 4. Requirements

- **FR-1 Guide accuracy.** Every UI label quoted in bold exists in the `en` table; every command
  exists in the repo; every shortcut exists in the code or the a11y audit's keyboard map.
- **FR-2 Coverage.** Each MVP feature in README's status table plus Phase 5 items has a section;
  known limitations from HANDOFF/specs are stated (old Git read-only, network ops not undoable,
  glyph only on loaded rows, Flatpak sandbox limits, credential helpers by absolute path).
- **FR-3 Release checklist** — ordered, checkbox list:
  1. `release/<version>` from `develop` (Git flow); version in workspace `Cargo.toml`,
     `tauri.conf.json`, `app/ui/package.json`, metainfo `<release>`; CHANGELOG section dated.
  2. CI green (Git 2.20.5 + system Git, app job incl. packaging validation).
  3. Local: `cargo test --workspace`, `pnpm --dir app/ui test`, `node scripts/contrast.mjs`.
  4. Benchmarks: `tests/fixtures/make.sh` `wide-100k`, idle PSS ≤ 250 MB, first paint target
     (ADR-0001) — record numbers.
  5. Build artifacts (`scripts/package.sh`, `scripts/package-flatpak.sh` or the release workflow).
  6. Manual test of each package on a clean machine/VM: install, launch from the menu (icon,
     WM class), open repo, PRD criteria 4 and 7 (stage, hunk, commit, amend, Undo/Redo, discard
     + Undo, external edit, missing identity), Shared Stash two clones + local bare remote
     (PRD 14), fetch/push over HTTPS (askpass) and SSH (agent), signed commit, pt-BR switch,
     keyboard-only walkthrough, Orca spot check.
  7. AppStream screenshots, Flathub submission (later), merge `release/*` → `main` (tag
     `v<version>`) and back to `develop`; release workflow artifacts attached by the owner.
  8. Update HANDOFF and the vault (EN then PT; never `approved`).
- **FR-4 CHANGELOG** entries per phase, user-facing wording, no commit hashes.
- **FR-5 README** status reflects Phase 5 done (pending owner checks listed), adds "Install"
  (deb, AppImage, Flatpak bundle) and links the user guide; keeps existing sections.
- **FR-6 HANDOFF** "State" gains a Phase 5 section (what was done, where, deviations, still
  open); "Next phases" updated; obsolete "next is…" lines fixed.

## 5. Implementation steps

1. Read the merged code and the three other Phase 5 specs' "Implementation notes".
2. Write `docs/user/index.md` and topic pages; cross-check every bold label with `i18n.ts`
   (e.g. `grep -F '"<label>"' app/ui/src/i18n.ts`).
3. Write `docs/release/checklist.md`, `CHANGELOG.md`.
4. Update `README.md`, `docs/HANDOFF.md`.
5. Check relative links resolve (script or `grep -o '](…)'` + `test -e`).

## 6. Test plan

| Check | How |
|---|---|
| Labels exist | script/grep each `**Label**` in the guide against `en` values |
| Commands exist | each `scripts/…` / `pnpm …` / `cargo …` command path exists |
| Links | every relative link target exists |
| No code changes | `git diff --stat` shows docs/README/CHANGELOG only |

## 7. Errors and edge cases

- A feature described in HANDOFF but missing in code → document the code's behaviour and report
  the mismatch; do not describe what does not exist.
- Unverified packaging results (e.g. Flatpak not built in CI yet) → say "to be verified" in the
  checklist, never claim it works.

## 8. Impact on existing architecture

Documentation only.

## 9. Definition of Done

- [ ] Guide pages, checklist and CHANGELOG exist; README/HANDOFF updated.
- [ ] §6 checks pass and are reported with their output.
- [ ] No code file changed.
- [ ] 5 screenshots in `docs/screenshots/phase5/release-docs/`: rendered user guide index, one feature page, keyboard page, release checklist, README status/Install section (Markdown rendered to HTML, captured with Playwright).

## 10. Instructions for the implementing agent

- Read `AGENTS.md`, `docs/HANDOFF.md`, all `docs/specs/*.md`, `docs/a11y/audit.md`,
  `docs/user/shared-stash.md` first.
- Write plain, short, task-oriented English; match `shared-stash.md`.
- Commit in small steps; do not push. Report files, checks run, mismatches found between docs
  and code.

## 11. Implementation notes

Filled by the implementing agent (2026-10-16), from `59f8170` (all three other Phase 5 modules
merged).

**Files**
- Guide (`docs/user/`): `index.md`, `install.md`, `getting-started.md`, `history-and-diff.md`,
  `committing-and-undo.md`, `branches-merge-stash.md`, `remotes-and-rebase.md`, `settings.md`,
  `keyboard-and-accessibility.md`, `troubleshooting.md`. `shared-stash.md`: only facts that
  changed since it was written (the visible claimed/verified badges became a shield mark for
  verified only; the graph mark is a "shared" label; local stash menu by keyboard; read-only tab
  behaviour). The guide links two existing Phase 5 screenshots (main window, keyboard commit
  menu) instead of new ones.
- `docs/release/checklist.md` (FR-3, 8 sections, checkboxes), `CHANGELOG.md` (FR-4), `README.md`
  (status rows for the Shared Stash and Phase 5, pending owner checks, Install, User guide,
  Layout), `docs/HANDOFF.md` (FR-6).
- Screenshots: `docs/screenshots/phase5/release-docs/01…05` (pandoc 3.1.3 gfm → HTML,
  Playwright/Chromium 1280×800; helper scripts outside the repo).

**Decisions**
1. Bold = text shown in the UI. Error messages quoted in the troubleshooting table are bold too,
   so the label check also proves them. Labels with placeholders (for example "Commit {n} files
   to {s}", "Forget all but the last {n}") are quoted in plain text with a concrete example,
   never bold. The only bold that is not a UI label is the pre-existing warning sentence in
   `shared-stash.md`.
2. Strings that come from Rust and are not in the `en` table (status bar
   `Git older than 2.20: read-only`) are quoted as code.
3. CHANGELOG: one `Unreleased` section with a sub-section per phase (newest first) plus
   "Interface additions between phases" for README's "Also in" items whose phase is not
   recorded. No commit hashes.
4. HANDOFF: rules kept verbatim except two obsolete "next" sentences: "Phase 3 starts as
   `feature/branching-merge`" (replaced by a pointer to the release checklist) and the Phase 4
   "Next: Shared Stash addendum…, then Phase 5". The Shared Stash status said "PR to `develop` not
   opened yet"; `mvp-phase5.md` §1 records it merged (PR #12), so it was updated. "Next phases"
   (3–5, all done) became "Next steps".
5. The Flatpak sandbox limits are written from the manifest, the packaging notes and the
   coordinator's list, and marked "to be verified" (the Flatpak was never built here).
6. The global shared-stash fetch-mode default is documented as a file
   (`~/.config/dev.atlasgit.app/settings.json`, from `app_config_dir()` + identifier); there is
   no UI for it.

**Mismatches between docs/HANDOFF and the code (reported, documented as the code behaves)**
- The theme choice is not persisted: `initTheme()` always calls `setTheme("system")`. The guide
  says so. (The language is persisted.)
- The graph's tips are all refs except `refs/stash` (`read_gix.rs` `tips`, CLI `--all`), so
  commits only reachable from `refs/atlaskraken/snapshots/*` (amend, reset, undone commits) and
  `refs/atlasgit/*` (shared stash mirrors) appear as unlabelled rows. Documented in
  troubleshooting.
- `journal_prune` deletes each entry's `snapshot_ref` but not the `-redo` (undone commit/pull) and
  `-wip` (hard reset) refs, so "Forget" does not remove everything Undo kept. Documented.
- `shared-stash.md` described visible "claimed"/"verified" badges; since `a534f0a` only verified
  stashes show a mark (claimed is screen-reader text plus the author tooltip). Fixed in the page.
- Toolbar Undo takes the newest non-undone journal entry: after a shared stash publish (journaled
  as "passive") it reports "That operation cannot be undone." instead of reaching older entries;
  fetch/push are not journaled, so Undo after a push undoes the pushed commit locally. Documented.
- "New branch" (toolbar and menus) creates the branch without checking it out, and a branch
  created from a remote branch does not track it (`git branch <name> <sha>`); the first Push sets
  the upstream. Documented.
- The journal's "Keep last" value is not remembered (always 50). Not documented as a feature.

**Verification** (§6): label check 270 bold strings, 269 in `en`, 1 allowed non-label, 0 missing;
41 relative links (files and anchors), 0 broken; 33 paths/commands, 0 missing; a negative run with
a fake label, a missing file, a missing anchor and a missing script reported all four.
`git diff --stat 59f8170..HEAD` lists only `docs/`, `README.md`, `CHANGELOG.md` and the
screenshots.

**Open questions for the owner**
- Everything in `mvp-phase5.md` §8/§8.1 (not resolved here), plus: persist the theme choice like
  the language? Hide AtlasGit's own refs from the graph, and make "Forget" delete the `-redo`/`-wip`
  refs? Both are code changes outside this module.
