# 02 · Funcionalidades e robustez

## Limitações
Análise estática. Testes do core executados (111 passando), UI 81 testes passando; o shell Tauri não compila no ambiente de auditoria. Como o dono prioriza conflitos, Git Flow e sessão, esses itens abrem o documento.

## Achados

### [FUN-01] Resolução de conflitos limitada ao bloco inteiro
- Severidade: ALTA
- Item da matriz: F-019, X-004
- Local: `app/ui/src/features/conflict/ConflictEditor.tsx:42-68`, `markers.ts:7-41`
- Evidência: o "3 painéis" mostra `ours` e `theirs` do **arquivo inteiro** em `<pre>`, sem destaque dos trechos em conflito, e um `<textarea>` para o resultado. A escolha é por bloco (`pick(text, i, "ours"|"theirs"|"both")`): não dá para marcar linhas individuais. Faltam: navegação "conflito n de m", coluna da base (merge-base), ordem de "ambas", sintaxe, "desfazer escolha", marcar arquivo como resolvido sem editar, abrir em ferramenta externa/IDE (o GitKraken oferece; REA: ux-analysis §2.4).
- Como o GitKraken faz (REA, screenshots `06a`/`06b` e feature-map): painel de arquivos em conflito à direita; ao clicar abre a ferramenta com **A (nossa) | B (deles)** em cima e **Saída** embaixo; cada lado tem **caixas de seleção por linha/hunk**; a saída se atualiza na hora; contador "conflito n de m"; botão Salvar; opção de ferramenta externa.
- Por que importa: é a maior lacuna de funcionalidade apontada pelo dono.
- Sugestão de correção: reescrever o editor como tela cheia (não modal), com `diff3` (`git checkout --conflict=diff3`) para ter a base, segmentos tipados (comum, conflito) com checkbox por linha em cada lado, saída derivada dos checks mais edição manual opcional, navegação n/m e opção "Abrir no editor" (`$EDITOR`/`xdg-open`/`code`). Manter o `markers.ts` como parser e cobrir com testes.
- Esforço: G

### [FUN-02] Resolver conflito não passa pelo journal nem guarda o conteúdo anterior
- Severidade: MÉDIA
- Item da matriz: F-019
- Local: `crates/ak-core/src/ops/merge.rs:164-171` (`resolve_with`)
- Evidência: `std::fs::write` direto no arquivo (não atômico) e `git add`; retorna `OpOutcome::default()` sem `record(...)`. A escolha do usuário não aparece no journal e não pode ser desfeita.
- Sugestão de correção: gravar via arquivo temporário + rename, guardar o texto anterior em `.git/atlaskraken/discarded/` como no discard e registrar a operação.
- Esforço: P

### [FUN-03] Não há suporte a Git Flow nem a padrão de nome de branch
- Severidade: ALTA
- Item da matriz: X-001, X-002
- Local: `app/ui/src/features/refs/NameDialog.tsx`, `App.tsx:640-650`, `ops/branch.rs:28-37`
- Evidência: criar branch abre um modal com um campo de texto. A única regra é `git check-ref-format` (`branch.rs:32`). Nada de prefixos `feature/`, `release/`, `fix/`, `hotfix/`, nem base automática (`develop`/`main`), nem finalização (merge de volta, tag).
- Proposta (alinhada ao pedido do dono):
  1. **Configuração por repositório** em `<git-dir>/atlasgit/settings.json` (o mecanismo de `settings.rs` já preserva chaves desconhecidas): `flow.enabled`, `flow.branches.main/develop`, `flow.prefixes.{feature,release,fix,hotfix}` e `flow.template`.
  2. **Modelo de nome personalizável** com variáveis, ex.: `{prefix}{ticket}-{title}` onde o usuário escolhe tipo (feature/fix/release/hotfix), informa `ACT-1010` e o título. O app aplica um `slug` configurável (espaços → `-`, minúsculas opcionais) e **mostra a prévia do nome final** antes de criar. Padrão global + sobrescrita por repo.
  3. **"Iniciar Git Flow"** no repositório (cria `develop` se faltar e grava a config) e botões "Iniciar/Finalizar feature, release, hotfix" que fazem as operações com o journal (undo) e confirmação.
  4. Reaproveitar o backend de branches (`BranchCreate { name, at }`) para tudo; a novidade é o assistente na UI e a config.
- Esforço: G

### [FUN-04] Criar branch não faz checkout e não oferece opções
- Severidade: ALTA
- Item da matriz: F-017
- Local: `ops/branch.rs:76` (`git branch <nome> <sha>`), `App.tsx:646`
- Evidência: o modal só pede o nome; a branch é criada e o usuário continua na anterior. Faltam: escolher a base (HEAD, outra branch, commit), "fazer checkout depois", "criar upstream no push", tipo (Git Flow).
- Sugestão de correção: modal "Nova branch" com: nome (com validação ao vivo), base, checkbox "Mudar para a nova branch" (padrão ligado, como GitKraken) e a seção Git Flow quando ativo.
- Esforço: M

### [FUN-05] Sem lista de recentes nem restauração de sessão
- Severidade: ALTA
- Item da matriz: F-003, F-005, X-003
- Local: `App.tsx:51` (`tabs` em memória), `main.rs:41-49` (sem persistência), `settings.rs` (só `sharedStash.fetchMode`)
- Evidência: ao fechar ou cair, as abas se perdem. Não existe lista de recentes nem tela inicial com repositórios.
- Sugestão de correção: guardar `session.json` no diretório de configuração (abas abertas na ordem, aba ativa, scroll/seleção opcionais) gravado em cada mudança com escrita atômica; reabrir ao iniciar; tela inicial com "Recentes" (nome, caminho, branch, ahead/behind quando barato), fixar, remover. Tratar pasta que sumiu sem travar.
- Esforço: M

### [FUN-06] Sem tela de Settings; preferências ficam espalhadas e algumas não persistem
- Severidade: MÉDIA
- Item da matriz: F-024, F-034
- Local: `App.tsx:350-366` (seletores de tema e idioma na barra), `theme.ts:27-45`, `docs/HANDOFF.md` ("theme choice is not remembered")
- Evidência: o tema não é lembrado entre execuções; idioma em `localStorage` (`ak.lang`); larguras em `localStorage`; fetch mode de Shared Stash só em arquivo, sem UI.
- Sugestão de correção: tela Settings (página, não modal) com seções: Geral, Aparência (tema, zoom, reduzir transparência/animações, densidade), Git (identidade, autofetch, remoto padrão), Git Flow/Padrões de branch, Atalhos, Shared Stash, Avançado (logs, limpar cache). Persistir tudo em `settings.json` global via `settings.rs`.
- Esforço: M-G

### [FUN-07] Reset hard, force delete e cherry-pick sem confirmação
- Severidade: MÉDIA
- Item da matriz: NF-005
- Local: `CommitMenu.tsx:47-51`, `RefMenu.tsx:75-76`
- Evidência: `Reset current branch here (hard: discard changes)` e "Force delete" executam ao primeiro clique. O Undo existe e o reset hard faz `stash create` (HANDOFF), mas o PRD (feature-map, linha Reset) pede "snapshot **e confirmação**".
- Sugestão de correção: diálogo de confirmação com resumo do que será perdido, "não perguntar de novo" opcional.
- Esforço: P

### [FUN-08] Rebase: sem "Skip", sem faixas com merge, sem progresso
- Severidade: MÉDIA
- Item da matriz: F-027
- Local: `ops/rebase.rs`
- Evidência: não há `--skip`; faixas com merges são recusadas (roadmap Fase 4); sem barra de progresso de rede.
- Sugestão de correção: adicionar Skip no banner de conflito; `--rebase-merges` opcional.
- Esforço: M

### [FUN-09] Funcionalidades de cliente consolidado ausentes
- Severidade: MÉDIA
- Item da matriz: F-006, F-025, F-029 a F-033, F-035 a F-037
- Evidência: sem clone/init, zoom, fast-forward dedicado, blame/histórico de arquivo, busca no grafo, paleta de comandos, worktrees, assinatura, autofetch e stash parcial (veja a matriz).
- Sugestão de correção: priorizar clone/init, busca no grafo, blame e autofetch (maior uso diário); o resto vai para marcos posteriores.
- Esforço: G (conjunto)

### [FUN-10] Autenticação: askpass só em Unix e sem cancelamento
- Severidade: BAIXA
- Item da matriz: F-022
- Local: `app/src-tauri/src/askpass.rs`
- Evidência: socket Unix 0600 em `XDG_RUNTIME_DIR` (bom); só Unix (aceitável: o alvo é Linux). Sem timeout no prompt (PERF-11).
- Esforço: P

### [QA-01] Testes de integração do grafo geram fixtures em paralelo sem trava
- Severidade: MÉDIA
- Item da matriz: critério de aceitação 2
- Local: `crates/ak-core/tests/graph.rs:16-22` (`fixture()`), `tests/fixtures/make.sh`
- Evidência: na primeira execução de `cargo test -p ak-core -p ak-gitcli` (pasta `tests/fixtures/out` ausente), 3 testes de `graph.rs` falharam com `failed to build fixture demo/linear`; na segunda execução, com os fixtures já criados, os mesmos 7 testes passaram. Cada teste chama `make.sh` se a pasta não existe, e testes rodam em paralelo. A causa é uma hipótese forte (corrida), não confirmada por log do `make.sh`.
- Por que importa: o CI em máquina limpa pode falhar de forma intermitente e mascarar regressões reais.
- Sugestão de correção: criar os fixtures uma única vez (`std::sync::Once`/`OnceLock` ou lock de arquivo) ou em `build.rs`/script prévio do CI.
- Esforço: P

## O que já está bom
- Motor de operações com locks, journal e Undo/Redo, cobertos por testes de integração (`tests/ops.rs`, 44 testes).
- Códigos de erro estáveis com texto humano em dois idiomas e mensagem bruta em "Detalhes".
- Previsão de conflito antes do merge (`merge_preview`).
- Credenciais via helper/ssh-agent e askpass embutido.
- Shared Stash completo e testado (31 testes de integração).

## Não verificado
Todos os testes manuais do app empacotado listados em `docs/release/checklist.md §6`; comportamento de rebase em repositórios reais; `unwrap/expect` em caminhos do usuário: os encontrados em código de produção são `Mutex::lock().unwrap()` (`main.rs`, `askpass.rs`, `shared_stash.rs`) e `expect("piped stdin")` (`exec.rs:124`); os demais estão em testes.
