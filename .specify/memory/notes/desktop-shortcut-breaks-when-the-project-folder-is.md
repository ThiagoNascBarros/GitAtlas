---
id: desktop-shortcut-breaks-when-the-project-folder-is
type: pitfall
title: Desktop shortcut breaks when the project folder is renamed
tags: []
date: 2026-10-25
---
The launcher bakes the absolute project path into ~/.local/bin/gitatlas-launch (root=) and Path= in ~/.local/share/applications/gitatlas.desktop and ~/Área de trabalho/GitAtlas.desktop. After AtlasGit -> GitAtlas rename the click failed with "Falha ao ir para diretório". Fix: run `bash scripts/install-shortcut.sh` from the new folder. Test with `gtk-launch gitatlas`. The launcher rebuilds in release when sources are newer than target/release/gitatlas, so prebuild with: systemd-run --user --scope -p MemoryMax=2800M -p MemorySwapMax=0 nice -n 15 env CARGO_BUILD_JOBS=2 cargo build --release -p gitatlas --features custom-protocol (7GB machine).
