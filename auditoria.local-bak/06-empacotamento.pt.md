# 06 — Empacotamento e distribuição

## Limitações
- Apenas análise. Nenhum pacote foi construído (`target/release/bundle` não existe; `scripts/package*.sh` foram lidos, não executados). O comportamento em execução de Flatpak, deb e AppImage, portanto, não foi observado.
- Sem rede: o pin de tag/commit do Git e o runtime GNOME 51 não puderam ser conferidos.
- Só foram executados validadores somente-leitura (`desktop-file-validate`, `appstreamcli validate --no-net`).
- IDs da matriz em `auditoria/00-matriz-documentacao-vs-codigo.md` (F-030, NF-005, NF-007, NF-009, AC-012).

## Findings

### [PKG-1] Manifesto Flatpak nunca foi construído nem executado
- Severidade: ALTA
- Item da matriz: F-030, AC-012
- Local: packaging/flatpak/dev.atlasgit.app.yml; scripts/package-flatpak.sh; .github/workflows/release.yml (job `flatpak`)
- Evidência: docs/HANDOFF.md:46 "the **Flatpak was not built here** (Flathub blocked by the container proxy): its first build is the release workflow". docs/release/checklist.md §5: "The Flatpak has never been built outside this workflow ... *to be verified*". O manifesto fixa `runtime-version: '51'`, Git `tag: v2.56.0`, `commit: a018953688f1b10bddf91bff8747068f5f4746a4`; nada disso pôde ser checado offline. O Flatpak é documentado como canal principal (HANDOFF.md:11).
- Por que importa: o canal principal não está provado. Versão de runtime errada, tag/commit divergente ou flag de build do Git (`NO_RUST`, libsecret) só falham na primeira tag/dispatch.
- Sugestão de correção: rodar `workflow_dispatch` do release.yml agora (antes de taguear), registrar o resultado e corrigir o que quebrar. Manter o smoke test (git, ssh) e adicionar `gpg --version`.
- Esforço estimado: P

### [PKG-2] `--filesystem=home` dá ao app a home inteira, incluindo ~/.ssh e ~/.gnupg
- Severidade: MÉDIA
- Item da matriz: NF-007, F-030
- Local: packaging/flatpak/dev.atlasgit.app.yml (`finish-args`)
- Evidência: `- --filesystem=home` com o comentário "also exposes ~/.gitconfig, ~/.config/git, ~/.ssh (keys, known_hosts, config) and ~/.gnupg. The owner may narrow this later." O checklist §1 lista isso como decisão aberta do owner.
- Por que importa: o sandbox quase não isola chaves privadas; revisores do Flathub costumam questionar. A WebView é a superfície de ataque.
- Sugestão de correção: decidir e documentar. Opção: pastas específicas + `--filesystem=~/.ssh:ro`, `~/.config/git:ro` e o seletor de arquivos via portal para outros repositórios. Registrar o trade-off em docs/user/install.md.
- Esforço estimado: M

### [PKG-3] Caminhos de credencial/ssh/gpg do Flatpak dependem de suposições não testadas
- Severidade: MÉDIA
- Item da matriz: NF-009, AC-012
- Local: packaging/flatpak/dev.atlasgit.app.yml (`--socket=ssh-auth`, `--talk-name=org.freedesktop.secrets`, `--filesystem=xdg-run/gnupg:ro`); docs/user/troubleshooting.md
- Evidência: HANDOFF.md:47 "Flatpak sandbox limits ... marked 'to be verified'". Checklist §6: "In the Flatpak this goes through the host gpg-agent: *to be verified*"; "/etc/gitconfig not applied"; "credential helper set by absolute path falls back to the dialog". O socket do gpg-agent é montado `:ro`; não está provado que o `gpg` do runtime GNOME consiga conectar nele (nem que o caminho seja `xdg-run/gnupg` em todos os hosts).
- Por que importa: commits assinados, remotos SSH e credenciais no keyring podem falhar só dentro do sandbox.
- Sugestão de correção: rodar as linhas "Flatpak only" do checklist §6 numa VM limpa e transformar cada "to be verified" em resultado (ou limitação documentada).
- Esforço estimado: M

### [PKG-4] Nome do pacote/arquivo desktop não bate com o app id
- Severidade: MÉDIA
- Item da matriz: F-030, NF-009
- Local: app/src-tauri/tauri.conf.json (`productName`, `bundle.linux.deb.desktopTemplate`); packaging/linux/dev.atlasgit.app.metainfo.xml (`<launchable type="desktop-id">dev.atlasgit.app.desktop`)
- Evidência: docs/specs/mvp-phase5.md:151 "deb package/desktop file names come from `productName` (`atlas-git`, `AtlasGit.desktop`), so the metainfo `launchable` only resolves in the Flatpak". O `identifier` é `dev.atlasgit.app`; o desktop do repo é `dev.atlasgit.app.desktop`. O deb instala ícones como `dev.atlasgit.app.png`, que só casa porque o template usa `Icon=dev.atlasgit.app`. Não confirmado em deb construído (nenhum existe).
- Por que importa: centrais de software não ligam a metainfo ao lançador instalado em deb/AppImage; quem tem deb e Flatpak vê ids inconsistentes; `StartupWMClass=atlasgit` vs a classe real da janela não foi verificada (checklist §6).
- Sugestão de correção: construir o deb e inspecionar `dpkg-deb -c`; se o arquivo for `AtlasGit.desktop`, renomear via mapa `files` do bundle (`/usr/share/applications/dev.atlasgit.app.desktop`) ou passo pós-build, e checar a classe da janela com `xprop`/`wmctrl` em X11 e Wayland.
- Esforço estimado: P

### [PKG-5] O CI nunca constrói os pacotes em pull requests
- Severidade: MÉDIA
- Item da matriz: F-030, AC-012
- Local: .github/workflows/ci.yml; .github/workflows/release.yml
- Evidência: o job `app` do ci.yml só roda `cargo build -p atlasgit --features custom-protocol` e os dois validadores. `package.sh` e `package-flatpak.sh` só rodam no release.yml (`on: push tags v*` / `workflow_dispatch`). O build do CI usa só ubuntu-24.04; o .deb de release é feito em ubuntu-22.04.
- Por que importa: mudança em `tauri.conf.json`, arquivos do bundle ou manifesto só quebra o empacotamento quando uma tag é enviada.
- Sugestão de correção: job semanal/por path-filter (`packaging/**`, `app/src-tauri/**`, `scripts/package*.sh`) que execute os jobs de release sem retenção de artefatos.
- Esforço estimado: P

### [PKG-6] Matriz de Git do CI é implícita para 2.34/latest e não há job de benchmark
- Severidade: MÉDIA
- Item da matriz: NF-005, NF-001, NF-003
- Local: .github/workflows/ci.yml:15-24; PRD AC-13
- Evidência: matriz = `os: [ubuntu-22.04, ubuntu-24.04]`, `git: [system]` mais um `include` com `git: "2.20.5"`. Nenhuma entrada fixa 2.34 ou "latest"; "system" é o que a imagem do runner traz (o job só imprime `git --version`, não afirma). `cargo test -p ak-gitcli -p ak-core ...` exclui o crate `atlasgit`. Não existe passo `bench`/`wide-100k` (benchmarks do checklist §4 são manuais); não há `cargo clippy`/`fmt` embora o checklist §3 exija clippy.
- Por que importa: o AC-13 pede 2.20/2.34/latest. Regressões em Git intermediário/mais novo ou de performance em 100k commits não são pegas.
- Sugestão de correção: entradas explícitas `git: "2.34.1"` e `git: latest` (build do fonte ou PPA git-core), asserção de versão, passo de clippy e job de benchmark disparado manualmente que guarde o relatório.
- Esforço estimado: M

### [PKG-7] Não há estratégia de atualização
- Severidade: MÉDIA
- Item da matriz: F-030 (atualização: fora da documentação)
- Local: app/src-tauri/tauri.conf.json (sem `plugins.updater`); docs/specs/packaging.md:235
- Evidência: packaging.md:235 "harmless today (no updater)". Sem plugin updater em Cargo.toml/tauri.conf.json; sem AppImageUpdate/zsync, sem repositório apt, Flathub ainda não configurado (checklist §7 "Flathub submission: later"). docs/user/install.md diz que não há página pública de download.
- Por que importa: usuários de deb e AppImage ficam sem caminho de atualização; só o Flathub daria um, e não está pronto. Correções de segurança (WebKitGTK vai no AppImage) não chegariam.
- Sugestão de correção: declarar a estratégia em docs/user/install.md (Flathub primeiro; deb/AppImage manual) e decidir sobre AppImage com zsync ou repositório apt.
- Esforço estimado: M

### [PKG-8] O workflow de release não tem verificações de integridade/consistência
- Severidade: MÉDIA
- Item da matriz: F-030
- Local: .github/workflows/release.yml; Cargo.toml `rust-version = "1.80"`
- Evidência: sem checksum/assinatura (checklist §5 pede soma manual do owner); sem checagem de que a tag `v*` bate com a versão de `tauri.conf.json` (a versão está em quatro lugares manuais, checklist §1); actions fixadas por tag (`actions/checkout@v4`), não por SHA; `dtolnay/rust-toolchain@stable` não fixa versão e `rust-version = 1.80` nunca é testado; `pnpm/action-setup` `version: 12`.
- Por que importa: divergência tag/versão gera artefatos mal nomeados; artefatos sem checksum/assinatura não são verificáveis; a promessa de MSRV não é testada.
- Sugestão de correção: passo comparando `${GITHUB_REF_NAME#v}` com os arquivos de versão, gerar `SHA256SUMS` como artefato, job de CI com toolchain `1.80`.
- Esforço estimado: P

### [PKG-9] Metainfo AppStream incompleta para submissão em loja
- Severidade: BAIXA
- Item da matriz: F-030
- Local: packaging/linux/dev.atlasgit.app.metainfo.xml
- Evidência: `appstreamcli validate --no-net` -> "Validação bem sucedida." (rc=0). `grep screenshot` não encontra nada; release com `type="development"` datado 2026-10-16; checklist §7 afirma que o linter do Flathub exige screenshots.
- Por que importa: válida, mas não submetível ao Flathub nem bem exibida no GNOME Software.
- Sugestão de correção: adicionar `<screenshots>` do app real em URLs estáveis; trocar o tipo para `stable` ao taguear.
- Esforço estimado: P

### [PKG-10] Script de atalho de desenvolvimento diverge da entrada desktop empacotada
- Severidade: BAIXA
- Item da matriz: NF-009
- Local: scripts/install-shortcut.sh; scripts/launch.sh; packaging/linux/dev.atlasgit.app.desktop
- Evidência: o atalho grava `~/.local/share/applications/atlasgit.desktop` com `Icon=atlasgit`, `Exec=$bin`, `Path=$root`, sem `GenericName`/`Keywords`/pt_BR; launch.sh recompila com `scripts/run.sh --release` quando há fontes mais novas. A entrada empacotada é `dev.atlasgit.app.desktop`, `Icon=dev.atlasgit.app`. O caminho é injetado com `sed "s|^root=.*|root=\"$root\"|"` (quebra se o caminho tiver `|` ou `&`).
- Por que importa: instalar o deb junto do atalho mostra duas entradas "AtlasGit"; o lançador exige toolchain completa e árvore de fontes no login, o que usuário final não tem. Ok para desenvolvimento, enganoso se citado em docs de usuário.
- Sugestão de correção: rotular como somente-dev no README, reutilizar `Icon=dev.atlasgit.app` e proteger o quoting do caminho.
- Esforço estimado: P

### [PKG-11] Falha de build por `target/` obsoleto não está documentada
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: README.md; docs/HANDOFF.md; auditoria/00-linha-de-base.md
- Evidência: baseline: `cargo build --workspace` FAIL rc=101, o build script do tauri leu `/home/thigas/.../AtlasKraken/target/.../app_hide.toml` (target do caminho antigo). `grep -rn "cargo clean"` e "stale" em README.md e docs/ não retorna nada sobre isso (só a nota de rename em README.md:7).
- Por que importa: quem tem checkout anterior ao rename ou movido recebe erro opaco no primeiro build.
- Sugestão de correção: uma linha de troubleshooting no README/HANDOFF: após mover/renomear o repositório, rodar `cargo clean -p atlasgit` (ou apagar `target/`).
- Esforço estimado: P

### [PKG-12] Licenciamento é majoritariamente consistente, com pequenas lacunas
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: Cargo.toml:8, app/ui/package.json:5, LICENSE, packaging/flatpak/dev.atlasgit.app.yml
- Evidência: `license = "Apache-2.0"` no workspace e no package.json; LICENSE é o texto Apache 2.0 (202 linhas); metainfo `project_license` Apache-2.0; tauri `licenseFile` definido. Arquivos rastreados sem cabeçalho SPDX: `app/ui/src/features/wip/status.test.ts`, packaging/linux/dev.atlasgit.app.desktop, os três Cargo.toml, `.cargo/config.toml`, index.html. Não há NOTICE nem lista de licenças de terceiros, embora o Flatpak construa o Git (GPL-2.0) e o AppImage inclua WebKitGTK/GTK (LGPL).
- Por que importa: política inconsistente (a maioria de `.rs/.sh/.yml` tem SPDX) e sem aviso para componentes GPL/LGPL redistribuídos.
- Sugestão de correção: adicionar o cabeçalho nos arquivos listados e uma nota THIRD-PARTY curta (fonte do Git via tag fixada; libs LGPL no AppImage).
- Esforço estimado: P

### [PKG-13] Perfil de release mínimo e não medido
- Severidade: BAIXA
- Item da matriz: NF-004
- Local: Cargo.toml `[profile.release]`
- Evidência: `lto = "thin"`, `codegen-units = 4`; sem `strip`, `panic`, `opt-level`. A `version` do bundle 0.1.0 bate com o workspace. Nenhum tamanho de binário/pacote é registrado (não há artefato construído).
- Por que importa: tamanho do pacote e partida (NF-004 não medido) não podem ser ajustados nem acompanhados.
- Sugestão de correção: medir o tamanho do release com strip uma vez; considerar `strip = true`; registrar o tamanho por release.
- Esforço estimado: P

## O que já está bom
- `desktop-file-validate` e `appstreamcli validate --no-net` passam (rc=0) e rodam no CI e no release.
- O job de release usa ubuntu-22.04 com piso de glibc 2.35 documentado, sem secrets, só `contents: read`, e nunca publica.
- `depends: git (>= 1:2.20)` do deb é verificado no CI (`grep -F`); ícones de 32 a 512 px existem e são instalados em hicolor com o app id.
- O Git do Flatpak é fixado por tag e commit; a documentação marca como "to be verified" o que não foi provado.
- O checklist bate com o código em versões, na entrada 2.20.5 do CI e no estado "Flatpak nunca construído".
- Versão única `0.1.0` consistente em Cargo.toml, tauri.conf.json e metainfo.

## Não verificado
- Qualquer build real de .deb, AppImage ou Flatpak; instalação em Ubuntu 22.04/24.04; Wayland vs X11 (AC-012).
- Conteúdo e nomes reais do deb e a classe real da janela vs `StartupWMClass=atlasgit`.
- Validade da tag `v2.56.0` do Git, do hash do commit e do runtime GNOME 51.
- Se `gpg`, `ssh`, `git-credential-libsecret` funcionam dentro do sandbox.
- Quais versões de Git o "system" das imagens de runner realmente usa.
- Tamanho do binário e partida a frio; a alegação de compatibilidade com Debian 12.
