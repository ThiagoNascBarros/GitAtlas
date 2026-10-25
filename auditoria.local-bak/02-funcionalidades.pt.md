# Auditoria 02 — Funcionalidades e robustez

Data: 2026-10-15. IDs dos achados: `FUNC-n`. IDs da matriz de `auditoria/00-matriz-documentacao-vs-codigo.md`.

## Limitações

- Somente análise estática: código, greps e a linha de base da Fase 0 (`cargo test` 120 passaram; vitest 81 passaram). O app não foi operado de forma interativa; nenhum fluxo de rede ou de credenciais foi exercitado.
- As skills de Rust (`rust-best-practices`, `rust-async-patterns`, `rust-testing`) não foram carregadas pela ferramenta Skill; seus critérios (Result em vez de panic, sem bloqueio em async, camadas de teste) foram aplicados por conhecimento geral.
- O comportamento em crash (kill durante uma operação) foi deduzido da ordem do código, não reproduzido.
- A classificação de `unwrap/expect` cobre só código não-teste (tudo antes do primeiro `#[cfg(test)]` de cada arquivo).
- Foram usados apenas docs REA, docs internos e código. Nenhuma funcionalidade do GitKraken foi presumida.

## Revisão de status da matriz (refinamentos)

| ID | Matriz | Veredito | Evidência |
|---|---|---|---|
| F-003 | Absent | **Confirmado** | `grep -rniE "recent"` só acha texto de i18n/tema; nenhuma lista persistida de repos em `app/src-tauri/src` |
| F-005 | Absent | **Confirmado** | únicas chaves `localStorage`: `ak.split.*` (Splitter.tsx:10), colunas (columns.ts:39), idioma (i18n.ts:427), WIP aberto (WipPanel.tsx:13), glifo (shared-stash/model.ts:75). Sem lista de abas |
| F-007 | Partial | **Confirmado** | a linha WIP existe (`drawWip`, GraphView.tsx:248; `wipText` a11y.ts:50); stash é um rótulo (main.rs:~262, `RefKind::Stash` encadeado na linha base), não uma linha |
| F-012 | Absent | **Confirmado** | `file_diff` tem args fixos: `show --format= -m --first-parent --no-ext-diff --no-color` (read.rs:115), sem `-w` |
| F-013 | Absent | **Confirmado** | sem dependência/uso de highlighter |
| F-014 | Not verified | **Absent** | os dois caminhos de diff só devolvem hunks: read.rs:115 e `worktree_diff` read.rs:210-225 (contexto padrão, sem `-U<grande>`/`cat-file`); nenhum endpoint de arquivo inteiro entre os 51 comandos Tauri |
| F-029 | Absent | **Confirmado** | sem zoom/escala de fonte em app/ui/src nem tauri.conf.json |
| F-033 | Absent | **Confirmado** | ver FUNC-4 |
| F-034 | Partial | **Confirmado** | `LARGE_STASH_BYTES` só avisa (`large_bytes`, shared_stash_ops.rs:518) |
| F-036..F-041 | Absent | **Confirmado** | ver FUNC-5 |
| NF-004 | Not verified | **Não verificado** | não existe medição de cold start; não dá para medir estaticamente |
| NF-011 | Not verified | **Provavelmente atendido, não provado** | abrir/grafo/status/stage/commit usam só git local; os testes de integração do ak-core usam bare remotes locais (ops.rs:688). Rede só em `op_net` e comandos de shared stash. Sem teste offline |
| AC-001 | Not verified | **Não verificado** | sem e2e; `get_initial_path` (main.rs:284) existe |
| UI-008 | Not verified | **Implemented** | `<footer class="statusbar">` App.tsx:663 (contagem de commits, versão do Git, motivo de somente-leitura) |

## Achados

### [FUNC-1] Sem repositórios recentes e sem restauração de sessão
- Severidade: ALTA
- Item da matriz: F-003, F-005
- Local: app/ui/src/App.tsx:51 (`tabs`), app/src-tauri/src/main.rs (`repo_open`)
- Evidência: as abas vivem num signal do Solid; não há código de persistência (ver revisão da matriz). Ao fechar o app, o usuário precisa reabrir cada repo pelo diálogo ou argumento de CLI.
- Por que importa: previsto no PRD §5.1; um cliente de uso diário sem "reabrir o que eu tinha" é lacuna básica de usabilidade, e a Fase 5 saiu como MVP sem isso.
- Sugestão de correção: persistir a lista `{path, active}` no diretório de config (o gravador JSON atômico de settings.rs já existe) a cada mudança de aba; adicionar lista "Recentes" no estado vazio; descartar caminhos que não existem mais na inicialização.
- Esforço estimado: M

### [FUNC-2] Diff: sem alternância de espaços em branco, sem visão de arquivo inteiro, sem realce de sintaxe
- Severidade: MÉDIA
- Item da matriz: F-012, F-013, F-014 (F-014 corrigido para Absent)
- Local: crates/ak-core/src/read.rs:111-118 e :210-226; app/ui/src/features/diff/DiffView.tsx:15
- Evidência: `["show","--format=","-m","--first-parent","--no-ext-diff","--no-color",sha,"--",path]`; os modos são só `unified|split`. Também não há limite de tamanho: as duas funções devolvem o diff completo como uma `String` pelo IPC.
- Por que importa: três itens do PRD §5.4 ausentes; um diff muito grande (arquivo gerado) é enviado inteiro e processado na thread da UI.
- Sugestão de correção: adicionar flag `ignoreWhitespace` (`-w`) aos dois comandos de diff; visão de arquivo inteiro via `-U999999` ou `git show <sha>:<path>`; limitar bytes do diff (ex.: 2 MB) com ação "mostrar mesmo assim"; realce pode esperar (roadmap T1.9).
- Esforço estimado: M

### [FUNC-3] Zoom 80-200 % ausente
- Severidade: MÉDIA
- Item da matriz: F-029, NF-009
- Local: app/ui (sem código), app/src-tauri/tauri.conf.json
- Evidência: `grep -rniE "zoom|font-scale|fontSize"` em app/ui/src e tauri.conf.json: sem ocorrência.
- Por que importa: requisito do PRD §5.11 e auxílio de acessibilidade (NF-008); a auditoria docs/a11y já registra que a escala de fonte não é tratada.
- Sugestão de correção: token de `font-size` na raiz guiado por configuração persistida, mais Ctrl+= / Ctrl+- / Ctrl+0; o layout usa px em alguns pontos (styles.css), então verificar se a altura de linha do canvas do grafo escala.
- Esforço estimado: M

### [FUNC-4] Shared Stash publica sem verificação de segredos nem limite rígido de tamanho
- Severidade: ALTA
- Item da matriz: F-033, F-034
- Local: crates/ak-core/src/ops/shared_stash_ops.rs:473-521
- Evidência: `new_blob_bytes` alimenta apenas `large_bytes` (`(bytes > LARGE_STASH_BYTES).then_some(bytes)`, :518); o push (`push_shared`, :306) roda de qualquer forma. Não há varredura de conteúdo em nenhum shared_stash*.rs.
- Por que importa: o PRD §7 N0 Safety exige verificação de segredos e limite de 5 MB antes do push. Um stash costuma conter `.env`/chaves e vai para um remoto compartilhado, de onde é difícil retirar (o histórico permanece no servidor).
- Sugestão de correção: antes do push, varrer os blobs adicionados por padrões comuns de segredo e nomes sensíveis (`.env`, `id_rsa`, `*.pem`) exigindo confirmação explícita; tornar o limite configurável e bloqueante acima do teto (padrão 5 MB conforme o PRD).
- Esforço estimado: M

### [FUNC-5] N3-N7 e busca/blame não implementados
- Severidade: MÉDIA
- Item da matriz: F-036, F-037, F-038, F-039, F-040, F-041
- Local: app inteiro
- Evidência: sem comparação de intervalo, vistas salvas, paleta de comandos (Ctrl+Shift+P só alterna o overlay de performance, App.tsx:277), `StashPush { message, include_untracked }` sem caminhos (ops/mod.rs:100), sem regras de identidade, sem busca de commits/blame (só filtro de refs, features/refs/filter.ts).
- Por que importa: itens de roadmap além do MVP; F-041 (busca de commits) é o ligado a uma persona do PRD e é expectativa básica de um cliente consolidado.
- Sugestão de correção: priorizar F-041 (busca por mensagem/autor/sha sobre a janela `RowText` já carregada ou `git log --grep`), depois F-039; deixar N3/N4/N5/N7 para depois do MVP mantendo-os documentados como tal.
- Esforço estimado: G

### [FUNC-6] Reset hard roda com um clique, sem confirmação
- Severidade: ALTA
- Item da matriz: F-026, NF-006
- Local: app/ui/src/features/graph/CommitMenu.tsx:51 e :34 (`reset`)
- Evidência: `const reset = (mode) => go(() => api.history(..., "reset", sha(), { mode }), ...)`; o item de menu "Reset current branch here (hard: discard changes)" chama direto. As demais ações destrutivas (discard, stash drop, force push, journal prune) usam confirmação em duas etapas.
- Por que importa: pela regra de severidade é operação destrutiva sem confirmação. Atenuante: history.rs:84-100 guarda `stash create` + ref de snapshot e o Undo restaura (teste ops.rs:650); mas `stash create` não inclui arquivos não rastreados, e o Undo é recusado se a worktree estiver suja (`WORKTREE_CHANGED`).
- Sugestão de correção: etapa de confirmação (ou diálogo com o número de arquivos não commitados que serão perdidos) para `hard`; manter o caminho de Undo.
- Esforço estimado: P

### [FUNC-7] Entradas do journal podem causar panic no undo (`args[n]` sem checagem)
- Severidade: ALTA
- Item da matriz: F-025
- Local: crates/ak-core/src/ops/history.rs:123,192; stash.rs:121,132,135,140; branch.rs:133 (`let a = &e.args`)
- Evidência: `let (mode, wip) = (e.args[0].as_str(), e.args[1].as_str());` enquanto `JournalEntry.args` é `#[serde(default)]` (journal.rs) e o arquivo é JSONL simples, editável pelo usuário. Uma entrada truncada/antiga/editada com `kind:"reset"` e `args` vazio causa panic.
- Por que importa: o panic ocorre dentro de um comando Tauri `async`; o `LockGuard` é liberado no unwinding, mas o usuário recebe uma falha tipo crash no Undo/Redo, justamente o recurso feito para socorrê-lo. Panic = sem código de erro estável.
- Sugestão de correção: validar `args.len()` por tipo num helper único que devolva `UNDO_NOT_SUPPORTED`/`JOURNAL_CORRUPT`; adicionar teste com entradas malformadas.
- Esforço estimado: P

### [FUNC-8] Durabilidade do journal e perda silenciosa de linhas corrompidas
- Severidade: MÉDIA
- Item da matriz: F-025, NF-006
- Local: crates/ak-core/src/ops/journal.rs:44-99; ops/mod.rs:365,587 (append depois da ação)
- Evidência: (a) `append` não faz `sync_data`; (b) `list()` usa `filter_map(... .ok())`, então linhas ilegíveis são descartadas, e `set_undone`/`prune` reescrevem o arquivo inteiro a partir dessa lista, apagando-as de vez; (c) a reescrita usa `journal.tmp` fixo com `fs::write` sem fsync; (d) em `commit`/`discard` o journal é escrito após o passo destrutivo e o `?` transforma erro de I/O do journal em operação falha embora o commit/discard tenha ocorrido (e o backup do discard em `.git/atlaskraken/discarded/<id>` fica órfão).
- Por que importa: crash ou disco cheio entre a ação e o append gera operação que existe no Git mas não no journal; um discard cuja entrada nunca foi escrita não pode ser desfeito pela UI (dados só recuperáveis à mão a partir do diretório de backup).
- Sugestão de correção: gravar a intenção antes (entrada `pending`) e marcar concluída depois; preservar literalmente linhas não parseáveis na reescrita; fsync do arquivo e do diretório pai; em falha de append após sucesso devolver o resultado com aviso em vez de erro.
- Esforço estimado: M

### [FUNC-9] Merge/cherry-pick/revert com conflito nunca é registrado no journal
- Severidade: MÉDIA
- Item da matriz: F-019, F-021, F-025
- Local: crates/ak-core/src/ops/merge.rs:~96-98 (`return Ok(OpOutcome { ..., conflicted: true })` antes de `journal.append`); history.rs:52-53; remote.rs:80-81 (pull)
- Evidência: em conflito a função retorna antes de registrar. Quando o usuário resolve e commita depois pela caixa WIP, a entrada registrada é um `commit` simples; o Undo dela (`reset --soft head_before`, mod.rs:~415) remove o commit de merge mas o `MERGE_HEAD` já não existe, então recommitar gera commit de um só pai.
- Por que importa: os merges mais sujeitos a erro (os conflitantes) têm o Undo mais fraco, e o resumo do journal não diz "merge".
- Sugestão de correção: registrar head_before quando o conflito começa (ex.: pequeno arquivo de estado) e gravar o merge/pick no journal ao concluir `MERGE_HEAD`/`CHERRY_PICK_HEAD`; ou marcar o commit com tipo `merge_commit` e desfazer com `reset --merge`.
- Esforço estimado: M

### [FUNC-10] Trabalho bloqueante de git no runtime async; sem timeout nem cancelamento nas operações locais
- Severidade: MÉDIA
- Item da matriz: F-023, NF-002
- Local: app/src-tauri/src/main.rs:289-310 (`repo_open`, `repo_refresh` chamam `load` bloqueante), :431-434 (`run_op`); crates/ak-gitcli/src/exec.rs `run`/`run_status` (`.output()`, sem timeout)
- Evidência: só `op_net` (main.rs:490) e os comandos de shared stash usam `spawn_blocking`; todo outro `async fn op_*` roda `engine.run` (que lança git e espera) direto numa worker do Tokio. `Command::output()` não tem prazo.
- Por que importa: abrir um repo de 100k commits (`load` lê histórico e monta o layout) ou um hook travado (pre-commit/post-checkout, FS de rede lento) ocupa uma worker; com várias abas/operações o IPC pode engasgar. Um git travado também mantém `Lock::Index`, então toda operação seguinte nesse repo responde `REPO_BUSY` até reiniciar o app; não há cancelamento para `op_net` (só existe `shared_stash_cancel`).
- Sugestão de correção: envolver `run_op`/`load` em `spawn_blocking`; adicionar timeout/handle de kill opcional em `Git::run*` e um comando de cancelamento para operações longas.
- Esforço estimado: M

### [FUNC-11] Socket do askpass: corrida de permissão, sem checagem do par, fallback silencioso
- Severidade: MÉDIA
- Item da matriz: F-024, NF-007
- Local: app/src-tauri/src/askpass.rs:85-91, :131, :140
- Evidência: `UnixListener::bind(&path)` e depois `set_permissions(0o600)` (dois passos; o socket nasce com a umask). O diretório é `$XDG_RUNTIME_DIR` (0700, ok) ou, se ausente, `std::env::temp_dir()` (/tmp, compartilhado); o nome é previsível (`atlasgit-askpass-<pid>-<n>.sock`). O laço de accept não verifica o UID do par e `serve` faz `read_line` bloqueante sem timeout de leitura; se o `bind` falha o código roda o git sem ponte (`else { return f(&[]) }`).
- Por que importa: no fallback /tmp, um usuário local poderia conectar na janela antes do chmod e injetar um prompt falso que o usuário responde (senha entregue ao atacante). Um cliente que conecta e nunca escreve bloqueia a worker, e o `worker.join()` impede a operação de rede de retornar. O fallback silencioso faz o Git falhar com erro genérico de autenticação.
- Sugestão de correção: criar diretório privado 0700 (ou umask 077 antes do bind); checar UID via `peer_cred`; definir timeout de leitura no stream; devolver erro claro `ASKPASS_UNAVAILABLE` quando o bind falhar.
- Esforço estimado: P

### [FUNC-12] Heurística `is_secret`: mascara usuário, mostra alguns segredos às claras
- Severidade: MÉDIA
- Item da matriz: F-024
- Local: app/src-tauri/src/askpass.rs:77-80
- Evidência: `["password","passphrase","token","pin"].iter().any(|w| p.contains(w))` sobre o prompt em minúsculas. `pin` é substring de "spin", "pinned", "Pinterest" etc. (ex.: `Username for 'https://github.com/spin-org'` é tratado como segredo). Prompts como "Verification code", "Enter passcode", "OTP" ou variantes de "PIN" sem as palavras exatas não casam e seriam exibidos sem máscara. O único teste (askpass.rs:150+) tem três casos.
- Por que importa: máscara errada ou esconde o usuário que a pessoa precisa ler, ou expõe na tela um código de uso único/segundo fator.
- Sugestão de correção: casar por limite de palavra (`\bpin\b`) e ampliar a lista (passcode, verification, otp, secret); mascarar por padrão quando o prompt não começar com "Username"; adicionar testes tabulares.
- Esforço estimado: P

### [FUNC-13] `worktree_diff` aceita qualquer caminho (sem `check_rel`) e lê arquivos não rastreados sem limite
- Severidade: MÉDIA
- Item da matriz: NF-007, AC-011
- Local: crates/ak-core/src/read.rs:210-219; main.rs:352 (`diff_worktree`)
- Evidência: com `untracked=true` roda `git diff --no-index -- /dev/null <path>` com o caminho do chamador sem tratamento; todas as operações de escrita chamam `check_rel` (mod.rs `check_rel`, rejeita absoluto, `..`, `.git`), o caminho de leitura não.
- Por que importa: a API tipada é a fronteira de segurança (SDD §5): pela UI (ou script injetado) um caminho absoluto de arquivo sensível seria lido e devolvido; um arquivo não rastreado de vários GB seria carregado inteiro numa `String`.
- Sugestão de correção: chamar o mesmo `check_rel` em `worktree_diff` e `file_diff`; rejeitar não-regulares; truncar a saída acima de um limite.
- Esforço estimado: P

### [FUNC-14] Degradação silenciosa: engine, watcher e executável do git
- Severidade: MÉDIA
- Item da matriz: F-004, NF-005
- Local: app/src-tauri/src/main.rs:246 (`Engine::open(git, &root).ok()`), :129-205 (`start_watcher` devolve `None` via `.ok()?`), :138,:166 (`Command::new("git")`)
- Evidência: qualquer erro de `Engine::open` além de caps deixa o repo somente-leitura com `readOnlyReason` None (o motivo vem só de `caps.supported()`, :268). Um `watch` do inotify que falha (limite atingido em árvores grandes) torna o watcher inteiro `None` sem mensagem; `repo-changed` nunca é emitido. O watcher chama um `git` genérico, não o executável descoberto por `Git::discover`.
- Por que importa: o usuário vê um repo em que toda ação diz "somente leitura" sem explicação, ou uma UI que nunca atualiza após mudanças externas (comportamento documentado de "edições externas").
- Sugestão de correção: propagar o erro do engine como `read_only_reason`; emitir evento `watcher-unavailable` e mostrar dica de "Atualizar"; reutilizar `Git` na chamada de `ls-files`.
- Esforço estimado: P

### [FUNC-15] `Mutex::unwrap()` em estado compartilhado pode causar efeito cascata após um panic
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: main.rs:173,190,292,297,307,313,314,318; askpass.rs:48,55,120,137,138,141,142; shared_stash.rs:40,178
- Evidência: `state.repos.lock().unwrap()`. `ops/lock.rs` já usa `unwrap_or_else(|e| e.into_inner())`, então o padrão existe no código.
- Por que importa: um panic com guard seguro (ex.: estilo FUNC-7) envenena `repos`, e todo comando seguinte (`get`, main.rs:318) também dá panic, inutilizando a sessão.
- Sugestão de correção: um pequeno helper `lock()` com `into_inner` para esses mapas (os dados são só ids/handles).
- Esforço estimado: P

### [FUNC-16] Lacunas de cobertura de testes em comandos Tauri, UI e caminhos de falha
- Severidade: MÉDIA
- Item da matriz: F-001, F-002, F-004, F-024, UI-001, UI-003, AC-001
- Local: app/src-tauri/src (51 `#[tauri::command]`: 37 em main.rs, 14 em shared_stash.rs); app/ui/src
- Evidência: testes que tocam src-tauri: `askpass.rs` (1 teste, 3 asserts) e `settings.rs` (5). Zero testes para repo_open/refresh, watcher, ponte `op_net`, `credentials_answer`, comandos de journal, comandos `shared_stash_*`. UI: 12 arquivos vitest, todos de lógica pura (parse/tree/columns/i18n/helpers a11y/overlay); nenhum renderiza App, GraphView, WipPanel, UI do ConflictEditor, JournalPanel ou o diálogo de credenciais. Core: os testes de `lock.rs` são single-thread (sem teste de `try_acquire` concorrente); sem teste de linhas malformadas no journal, falha de escrita do journal ou crash entre a ação e o `append`; 2 testes `#[ignore]` de 100k exigem as fixtures pesadas.
- Por que importa: os achados FUNC-7, 8, 10, 14 seriam pegos por esses testes; a camada Tauri é onde ficam o mapeamento de erros (`UiError`) e a ponte do askpass.
- Sugestão de correção: (1) testes no core: journal corrompido, `try_acquire` concorrente com threads, append falhando; (2) refatorar `load`/comandos em funções testáveis sem `AppHandle`; (3) alguns testes de componente com o vitest + `devMock.ts` existentes; (4) um smoke e2e (abrir caminho via CLI -> grafo) para o AC-001.
- Esforço estimado: G

### [FUNC-17] Regras "somente o autor" do Shared Stash são consultivas
- Severidade: BAIXA
- Item da matriz: F-032
- Local: crates/ak-core/src/ops/shared_stash_ops.rs:296-301, :689
- Evidência: `normalize_email(&v.meta.author_email) != normalize_email(&who.email)` onde `who` vem do `user.email` local; a proteção contra mudança concorrente é `--force-with-lease` (:309).
- Por que importa: qualquer pessoa com permissão de push que configure o mesmo `user.email` pode atualizar/remover o stash de outra; a checagem evita erros, não abuso. Serve para equipe de confiança, mas UI/docs não devem sugerir imposição.
- Sugestão de correção: declarar esse limite em docs/user e na spec; opcionalmente apoiar-se nas permissões do host.
- Esforço estimado: P

## Revisão do motor de operações (evidência para as perguntas feitas)

- **Locks** (`ops/lock.rs`): tabela em memória por caminho do repo, `try_acquire` tudo-ou-nada, liberada no `Drop`, tolerante a poison. Não há arquivos de lock, então **não existem locks obsoletos após crash**; o custo é proteger só dentro de um processo (duas janelas do AtlasGit ou um `git` externo dependem do `index.lock` do próprio Git, e `GIT_OPTIONAL_LOCKS=0` é definido, exec.rs). Um git travado mantém o lock até reiniciar (FUNC-10). O caminho canônico vem de `rev-parse --show-toplevel`, então caminhos com symlink mapeiam para uma chave.
- **HEAD_MOVED / REF_MOVED**: o undo de `commit/commit_amend` exige `HEAD == head_after` (mod.rs:411), o redo exige `HEAD == head_before` (:456); entradas de ref usam `REF_MOVED`; o discard compara conteúdo (`WORKTREE_CHANGED`, mod.rs:606+, :484+). A lógica é sólida e testada (ops.rs:120). Lacuna: operação feita fora do app move o HEAD e bloqueia permanentemente o undo de entradas anteriores (a mensagem manda desfazer as mais novas, que podem não existir no journal).
- **Crash durante a operação**: a ordem é ação -> snapshot/backup -> journal. O discard copia backups antes de tocar nos arquivos (bom, mod.rs:~560), o reset salva refs de snapshot antes de mover. A janela insegura é o último passo (FUNC-8). O rebase grava `.git/atlaskraken/rebase/` e só remove nos caminhos de sucesso (`rebase_continue` limpa; `merge_abort` sobre rebase não), deixando pequenos órfãos.
- **Pilha de redo**: `redo_candidate` limpa o redo após qualquer entrada não-passiva mais nova; correto e testado (ops.rs:287,308).
- **Journal prune** deixa refs `-redo`/`-wip` (conhecido, HANDOFF): `prune_journal` apaga só `snapshot_ref` (mod.rs:~205), então essas refs se acumulam e mantêm objetos vivos.

## Autenticação (askpass) — resumo

- Socket: `$XDG_RUNTIME_DIR` ou diretório temporário, modo 0600 após o bind, removido ao sair (`remove_file`, askpass.rs:123); segredos não vão ao disco (comentário e código concordam: a resposta vai do socket para o stdout do cliente).
- Cancelamento/timeout: resposta `None` da UI fecha o stream -> cliente sai com 1; `ANSWER_TIMEOUT` de 300 s; `cancel_repo` e a limpeza "Git terminou" emitem `credentials-cancel`. Bom.
- Lacunas: FUNC-11, FUNC-12; a resposta digitada é uma `String` simples (sem zeroize) e trafega pelo IPC do Tauri até a webview - aceitável, mas anotar no modelo de ameaças.

## Classificação de `unwrap/expect/panic` fora de testes (≈30 pontos)

| Classe | Pontos | Veredito |
|---|---|---|
| Poison de Mutex `lock().unwrap()` | main.rs:173,190,292,297,307,313,314,318; shared_stash.rs:40,178; askpass.rs:48,55,120,137,138,141,142 | Risco baixo isolado, cascata após qualquer panic (FUNC-15) |
| Invariante (recém checado) | settings.rs:66,71 `as_object_mut().expect("object")`; read_gix.rs:106,122 (chaves de `indegree` vêm do mesmo `nodes`); exec.rs:124 `child.stdin.take().expect("piped stdin")` (configurado uma linha acima) | Seguro por construção; poderia ser `let ... else` |
| Fatal na inicialização | main.rs:638 `expect("error while running AtlasGit")` | Aceitável |
| Panic alcançável pelo usuário | não é unwrap, mas indexação `e.args[n]` (FUNC-7) | Risco real |

Não há `panic!`/`unreachable!`/`todo!` em código não-teste.

## O que já está bom

- Toda mudança na árvore de trabalho passa por `Operation` com `OpError.code` estável; a UI mapeia códigos para texto i18n (UI-006).
- Caminhos vindos da UI são validados (`check_rel`, pathspecs `:(literal)`, blocos de 100) em todas as operações de escrita; revisões são checadas como hex (`check_rev`).
- Operações de rede usam `--force-with-lease` (remote.rs:116, shared stash) e a UI pede confirmação para force push, discard, stash drop e prune.
- O discard faz backup antes de apagar e recusa symlinks/não-regulares (teste ops.rs:372); o undo recusa sobrescrever trabalho mais novo.
- O wrapper `Git` nega subcomandos de escrita em modo somente-leitura e nunca passa `safe.directory=*` (testes exec.rs:204+).
- A tabela de locks é tudo-ou-nada e tolerante a poison; 59 testes de integração do core + 31 testes e2e de shared stash com dois clones.

## Não verificado

- Comportamento real com kill -9 durante uma operação, com repo enorme ou com hook travado (deduzido, não executado).
- NF-004 cold start, AC-001, medidas de UI do NF-002; modo offline (NF-011) apenas inferido.
- Se a WebView realmente consegue chamar `diff_worktree` com caminhos arbitrários (depende do escopo de capabilities do Tauri; o código não mostra checagem no servidor).
- Texto de `docs/specs/shared-stash.md` sobre imposição de autoria e a escolha de 10 MiB (não relido por inteiro).
- Comportamento de Orca/TA no diálogo de credenciais.
- Se a quantidade de workers do Tokio e o pool bloqueante do Tauri tornam o FUNC-10 perceptível na prática (sem medição).
