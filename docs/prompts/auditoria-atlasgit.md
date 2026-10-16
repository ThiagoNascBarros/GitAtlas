## Objetivo
Produzir um diagnóstico completo do estado atual do **AtlasGit** — cliente Git desktop para Linux, produto próprio inspirado no GitKraken (não é um clone) — comparando o que foi documentado pelo REA no início do projeto com o que existe hoje no código. O resultado é um roadmap priorizado do que falta para o app estar consolidado, cobrindo desempenho, funcionalidades, UI, acessibilidade/i18n e empacotamento.

**Esta tarefa é só de análise:** não edite código, testes, temas nem documentação existente. Escreva apenas em `./auditoria/`. Não faça commit, push, reset nem crie/apague branches.

## Contexto do projeto (leia antes de começar)
- Stack: core em Rust (`crates/ak-core`, `crates/ak-gitcli`), shell Tauri em `app/src-tauri` (`main.rs`, `settings.rs`, `askpass.rs`, `shared_stash.rs`), UI em SolidJS + Vite + TypeScript em `app/ui`.
- Features da UI em `app/ui/src/features/`: `graph`, `diff`, `wip`, `files`, `refs`, `stash`, `shared-stash`, `rebase`, `conflict`, `journal`, `split`. Ponte com o backend em `app/ui/src/api.ts`; mock de desenvolvimento em `app/ui/src/devMock.ts`; i18n em `app/ui/src/i18n.ts` + `locales/`; tema em `app/ui/src/theme` e `themes/*.theme.json`.
- Documentação interna existente (complementa a do REA): `README.md`, `CHANGELOG.md`, `docs/HANDOFF.md`, `docs/specs/*.md` (MVP fase 5, acessibilidade, i18n pt-BR, shared stash, empacotamento, release), `docs/a11y/audit.md`, `docs/user/`, `docs/screenshots/`.
- Ferramentas de verificação: `tests/bench` (benchmarks), `tests/fixtures`, `scripts/contrast.mjs` (contraste de cores), `scripts/run.sh` / `launch.sh`, `scripts/package*.sh`.
- Instruções do repositório: `AGENTS.md`. Use as skills `rust-best-practices`, `rust-async-patterns` e `rust-testing` ao avaliar código Rust.

## Fontes de entrada
| O quê | Onde |
|---|---|
| Documentação gerada pelo REA (SDD/PRD, análise do GitKraken) | [CAMINHO DA DOCUMENTAÇÃO NO OBSIDIAN] |
| Código do AtlasGit | [CAMINHO DO REPOSITÓRIO] (raiz contendo `Cargo.toml`, `crates/`, `app/`) |
| Documentação interna do repositório | `docs/`, `README.md`, `CHANGELOG.md` |
| Saídas desta tarefa | `./auditoria/` (criar se não existir) |

Se a documentação do REA divergir da documentação interna (`docs/specs`, `HANDOFF.md`), registre a divergência; a do REA é a referência de escopo, a interna é evidência do que foi decidido depois.

## Fase 0: Linha de base (verificação objetiva)
Antes de analisar, rode o que for possível e registre comando, resultado e duração em `./auditoria/00-linha-de-base.md`:
- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`
- Em `app/ui`: `npm ci`, `npm run build` (inclui `tsc --noEmit`), `npm test` (vitest)
- `node scripts/contrast.mjs` (se aplicável) e os benchmarks em `tests/bench` (se houver instruções de execução)
- Tentar executar o app (`scripts/run.sh`). Se não for possível (sem display, dependências do WebKitGTK etc.), registre o motivo; a UI pode ser inspecionada via `npm run dev` com `devMock.ts`.

Comando que falhar não interrompe a tarefa: registre a falha como achado e siga.

## Fase 1: Matriz documentação × código
- **Entrada:** documentação do REA + documentação interna.
- **Tarefa:** liste todas as funcionalidades, telas, componentes de UI e requisitos não funcionais descritos. Para cada item, registre: Implementado, Parcial, Ausente ou Não verificado, citando `arquivo:linha` (ou módulo/comando Tauri) que comprova. "Implementado" exige evidência no código **e**, quando existir, teste cobrindo.
- **Saída:** `./auditoria/00-matriz-documentacao-vs-codigo.md` — tabela: `ID | item | origem na doc | status | evidência no código | teste?`. Use IDs estáveis (`F-001`, `NF-001`, `UI-001`) para os agentes referenciarem.
- **Critério de pronto:** todo item da documentação aparece na tabela com status; itens que existem no código mas não na doc ficam numa seção "Fora da documentação".

## Fase 2: Agentes especializados (só após a Fase 1)
Crie um subagente por área, todos em paralelo, cada um recebendo como contexto a matriz da Fase 1, a linha de base da Fase 0 e a seção "Contexto do projeto" acima.

| # | Agente | Escopo | Onde olhar primeiro | Saída |
|---|---|---|---|---|
| 1 | Desempenho | travamentos; operações bloqueantes no thread principal do Tauri ou da UI; comandos Tauri síncronos que chamam git; file watcher; renderização do grafo de commits e diffs grandes (virtualização); memória/CPU; repositórios grandes | `crates/`, `app/src-tauri/src`, `features/graph`, `features/diff`, `tests/bench` | `./auditoria/01-desempenho.md` |
| 2 | Funcionalidades e robustez | o que a doc prevê e falta; o que falta para um cliente Git consolidado; tratamento de erros (`Result`, `unwrap`/`expect` em caminhos de usuário); motor de operações (locks, journal, undo/redo); autenticação (askpass); cobertura de testes | `crates/ak-core`, `crates/ak-gitcli`, `features/journal`, `features/rebase`, `features/conflict` | `./auditoria/02-funcionalidades.md` |
| 3 | UI visual, acessibilidade e i18n | cores, temas claro/escuro, tipografia, ícones, consistência; contraste (WCAG AA); navegação por teclado e foco; textos sem tradução ou fixos no código | `styles.css`, `theme/`, `themes/`, `locales/`, `docs/a11y/audit.md`, `docs/specs/accessibility.md`, `docs/screenshots/` | `./auditoria/03-ui-visual.md` |
| 4 | Modais e componentes | modais, diálogos, formulários, menus de contexto; estados vazio/carregando/erro; confirmação de operações destrutivas; mensagens de erro vindas do git | `features/*`, `overlay.ts` | `./auditoria/04-modais-componentes.md` |
| 5 | Layout e responsividade | layouts, redimensionamento da janela, painéis divisíveis (`split`), tamanho mínimo, telas pequenas e HiDPI, escala de fonte | `features/split`, `App.tsx`, `styles.css`, `tauri.conf` | `./auditoria/05-layout-responsividade.md` |
| 6 | Empacotamento e distribuição | build de release, Flatpak/pacotes, atalho de desktop, dependências de sistema, atualização, documentação de release | `packaging/`, `scripts/package*.sh`, `scripts/install-shortcut.sh`, `docs/specs/packaging.md`, `docs/specs/release-docs.md` | `./auditoria/06-empacotamento.md` |

### Instruções comuns a todos os agentes
Cada achado segue este formato:

    ### [ID-área-nº] Título curto
    - Severidade: CRÍTICA | ALTA | MÉDIA | BAIXA
    - Item da matriz: F-001 (ou "fora da documentação")
    - Local: arquivo:linha / componente / tela
    - Evidência: trecho de código, saída de comando, medição ou captura de tela
    - Por que importa:
    - Sugestão de correção:
    - Esforço estimado: P (< 1 dia) | M (1–3 dias) | G (> 3 dias)

Critérios de severidade:
- **CRÍTICA:** perda de dados, corrupção do repositório, crash, operação destrutiva sem confirmação, falha de segurança.
- **ALTA:** funcionalidade central da doc ausente ou quebrada; travamento perceptível; inacessível por teclado.
- **MÉDIA:** funcionalidade parcial, inconsistência visível, degradação em repositórios grandes.
- **BAIXA:** polimento, estética, melhorias opcionais.

Cada arquivo começa com "Limitações" (ex.: app não executado, só análise estática) e termina com "O que já está bom" (pontos fortes a preservar) e "Não verificado" (o que não deu para confirmar e por quê).

- **Critério de pronto por agente:** arquivo existe, segue o formato, toda afirmação tem evidência, e há pelo menos um achado ou a declaração explícita de que nada foi encontrado na área.

## Fase 3: Consolidação (só após os 6 arquivos existirem e não estarem vazios)
- **Tarefa:** una os achados em um roadmap priorizado por severidade e esforço, removendo duplicatas entre agentes (mantenha o ID do achado principal e liste os duplicados).
- **Saída:** `./auditoria/07-roadmap-consolidacao.md` com:
  1. Resumo executivo (máx. 10 linhas).
  2. Lista priorizada (Crítico → Baixo), com ID, área, esforço.
  3. "Ganhos rápidos": severidade ALTA/CRÍTICA com esforço P.
  4. Ordem sugerida de execução em marcos (ex.: estabilidade → funcionalidades da doc → UI → empacotamento), indicando dependências entre itens.
  5. Definição de "consolidado": checklist objetivo do que falta para o app ser considerado consolidado.
  6. Sugestão de branches no fluxo do projeto (`fix/`, `feature/` a partir de `develop`) para os primeiros itens.
- **Critério de pronto:** todo achado dos arquivos 01 a 06 está no roadmap ou marcado como duplicado de outro.

## Idioma
Gere cada arquivo em duas versões: inglês (`*.md`, lido pelos agentes) e português do Brasil (`*.pt.md`, lido por mim). Conteúdo equivalente; IDs, caminhos e trechos de código idênticos nas duas.

## Restrições
- Não edite código, testes, temas nem a documentação existente; escreva apenas em `./auditoria/`. Comandos de build/teste só podem gerar artefatos em `target/`, `node_modules/` e `dist/`.
- Nenhum comando git que altere estado (commit, push, reset, checkout, clean, branch -D, stash).
- Não invente funcionalidades do GitKraken nem do AtlasGit: use só a documentação do REA, a documentação interna ou o código. O que não puder ser verificado vai em "Não verificado".
- Não analise licença/EULA do GitKraken.
- Se um arquivo, caminho ou comando não existir ou falhar, registre no arquivo de saída e siga com o que estiver disponível, sem parar a tarefa.
- Ao final, informe: número de achados por severidade e por área, itens da matriz por status, quais comandos da Fase 0 falharam e qualquer arquivo de saída que ficou incompleto.
