# Install Git Atlas

Git Atlas 0.1.0 comes as a .deb package, an AppImage and a Flatpak bundle. There is no public
download page yet: the packages are built from this repository with the scripts below, or taken
from the artifacts of the "Release packages" workflow (`.github/workflows/release.yml`) on
GitHub.

All three install the same program, `atlasgit`, with the menu entry **Git Atlas** (category
Development).

## .deb (Ubuntu, Debian)

```bash
sudo apt install ./AtlasGit_0.1.0_amd64.deb
```

- Package name: `atlas-git`. It depends on `git (>= 1:2.20)`, WebKitGTK 4.1 and GTK 3, which
  `apt` installs if they are missing.
- Needs Ubuntu 22.04 or newer (glibc 2.35 or newer) when it comes from the release workflow. A
  package you build yourself needs at least the glibc of the machine that built it.
- Remove it with `sudo apt remove atlas-git`.

## AppImage

```bash
chmod +x AtlasGit_0.1.0_amd64.AppImage
./AtlasGit_0.1.0_amd64.AppImage /path/to/repo
```

- The AppImage does not contain Git. Install Git 2.20 or newer from your distribution.
- It needs FUSE 2 to mount itself. Without it, run it with `APPIMAGE_EXTRACT_AND_RUN=1`
  in front of the command.
- It does not add a menu entry by itself; use your AppImage integration tool if you want one.

## Flatpak bundle

```bash
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user AtlasGit-0.1.0.flatpak
flatpak run dev.atlasgit.app
```

- App id: `dev.atlasgit.app`. The bundle uses the GNOME 51 runtime from Flathub (installed on the
  first install) and brings its own Git 2.56.0 with the libsecret credential helper.
- Git Atlas is not on Flathub yet.
- The sandbox changes a few things (system Git configuration, credential helpers, folders outside
  your home). Read [Flatpak limits](troubleshooting.md#flatpak-limits) before you rely on it.
- Remove it with `flatpak uninstall --user dev.atlasgit.app`.

## Build the packages yourself

From a clone of the repository (needs Rust 1.80 or newer, Node 22 or newer, pnpm and the
WebKitGTK 4.1 development libraries):

```bash
scripts/package.sh               # .deb and AppImage
scripts/package-flatpak.sh       # Flatpak bundle (needs flatpak and flatpak-builder)
```

- `scripts/package.sh` prints where it put the files:
  `target/release/bundle/deb/AtlasGit_0.1.0_amd64.deb` and
  `target/release/bundle/appimage/AtlasGit_0.1.0_amd64.AppImage`. Pass `deb` or `appimage` to
  build only one.
- `scripts/package-flatpak.sh` repacks the release binary that `scripts/package.sh` built (it
  runs it first if needed) and writes `target/flatpak/AtlasGit-0.1.0.flatpak`. When the GNOME
  runtime is missing and no Flathub remote is configured, it stops and prints the command that
  adds Flathub.

## Run from source

For development, or to try a branch without packaging:

```bash
scripts/run.sh /path/to/repo             # debug build
scripts/run.sh --release /path/to/repo   # optimized build
scripts/install-shortcut.sh              # menu entry that runs your clone (release build)
```

The shortcut opens Git Atlas without a repository and rebuilds the release binary when the sources
changed. Its log is `~/.cache/atlasgit/launch.log`.

## Open a repository from the command line

Every package accepts a folder as the first argument:

```bash
atlasgit /path/to/repo
flatpak run dev.atlasgit.app /path/to/repo
```

Next: [Getting started](getting-started.md).
