# 06 · Empacotamento e distribuição

## Limitações
Análise estática de `packaging/`, `tauri.conf.json`, `.github/workflows`, `scripts/package*.sh` e `docs/specs/packaging.md`. Nada foi empacotado nesta auditoria; o build do shell Tauri não é possível no ambiente de nuvem (faltam as libs do WebKitGTK).

## Achados

### [PKG-01] Flatpak nunca foi construído
- Severidade: ALTA
- Item da matriz: critério de aceitação 12
- Local: `packaging/flatpak/dev.atlasgit.app.yml`, `scripts/package-flatpak.sh`, `docs/HANDOFF.md` ("the Flatpak was not built here")
- Evidência: o Flatpak é o canal **primário** de distribuição (PRD §5.12), mas o manifesto nunca foi executado e o repack usa o binário de release, não um build a partir do código-fonte (Flathub exige build do fonte, com fontes vendorizadas de cargo e pnpm).
- Sugestão de correção: rodar `release.yml` em um runner com `flatpak-builder`, corrigir o que falhar; depois escrever o manifesto de fonte para Flathub.
- Esforço: G

### [PKG-02] Nome do pacote `.deb` e do `.desktop` divergem do identificador
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: `tauri.conf.json` (`productName: AtlasGit`), `docs/HANDOFF.md` (deviations de empacotamento)
- Evidência: o `.deb` se chama `atlas-git` e o arquivo é `AtlasGit.desktop`, enquanto o `metainfo` usa `dev.atlasgit.app`; o `launchable` só resolve no Flatpak. O identificador termina em `.app`, o que gera aviso do Tauri.
- Sugestão de correção: renomear no bundler (`desktopTemplate` já aponta para o `.desktop`; ajustar o nome do arquivo instalado e o metainfo) e trocar o identificador por `dev.atlasgit.AtlasGit`/`io.github.thiagonascbarros.AtlasGit`.
- Esforço: P

### [PKG-03] Sem caminho de atualização
- Severidade: MÉDIA
- Item da matriz: NF-008
- Evidência: o `.deb` e o AppImage não têm repositório nem verificação de nova versão; nenhuma checagem de atualização no app (sem `tauri-plugin-updater`).
- Sugestão de correção: manter o Flatpak como canal com atualização, e para AppImage avaliar `appimageupdate` ou aviso "há versão nova" lendo os releases do GitHub (opt-in, respeita "sem telemetria").
- Esforço: M

### [PKG-04] Artefatos sem checksum nem assinatura
- Severidade: BAIXA
- Local: `.github/workflows/release.yml`
- Evidência: o workflow só envia artefatos (por desenho); não gera `SHA256SUMS` nem assina.
- Sugestão de correção: gerar e publicar `SHA256SUMS` no artefato; assinatura quando houver release público.
- Esforço: P

### [PKG-05] `cargo fmt --check` falha em 565 trechos
- Severidade: BAIXA
- Item da matriz: qualidade
- Local: `docs/HANDOFF.md` (pendente do dono)
- Evidência: sem `rustfmt.toml` e sem PR de reformatação, o CI não pode exigir formatação.
- Sugestão de correção: um PR único só de formatação, depois exigir `fmt --check` no CI.
- Esforço: P

### [PKG-06] Perfil de release e dependências do build
- Severidade: BAIXA
- Local: `Cargo.toml:19-21`, `app/src-tauri/tauri.conf.json:7-8`
- Evidência: `lto = "thin"`, `codegen-units = 4`; faltam `strip`, `panic = "abort"` (reduz o binário, mas conflita com captura de panic; ver PERF-12) e `opt-level` explícito. O build do Tauri chama `pnpm`, enquanto a auditoria e o CI usam pnpm, mas `scripts/run.sh` falha se pnpm não estiver instalado (sem mensagem amigável).
- Esforço: P

### [PKG-07] Teste manual do app empacotado pendente
- Severidade: MÉDIA
- Local: `docs/release/checklist.md §6`, HANDOFF
- Evidência: stage, hunk, commit, amend, undo, discard, Shared Stash com dois clones e demais itens do checklist ainda não foram executados no app empacotado; PRD critérios 4, 7 e 14.
- Esforço: M

## O que já está bom
- `.deb` e AppImage construídos e abertos sob Xvfb, com validação de `.desktop`/metainfo no CI.
- Ícones hicolor, categoria, licença e dependência mínima de Git declaradas.
- Workflow de release sem segredos e sem publicação automática.
- Documentação de release (`docs/release/checklist.md`) e guia do usuário em `docs/user/`.

## Não verificado
Instalação nos Ubuntu 22.04/24.04, Wayland, integração com `ssh-agent`/GPG dentro do sandbox Flatpak, e as limitações do sandbox registradas como "to be verified" em `docs/user/troubleshooting.md`.
