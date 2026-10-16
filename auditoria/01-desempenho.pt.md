# 01 · Desempenho e estabilidade

## Limitações
Análise estática mais a linha de base (`00-linha-de-base.pt.md`). O app não foi executado (sem display/WebKitGTK), então **nenhum número de fps, RAM ou tempo de troca de repositório foi medido**. Onde o achado é hipótese, está marcado "Hipótese". Cada hipótese vem com a medição que a confirma.

## Achados

### [PERF-01] Toda operação recarrega o repositório inteiro
- Severidade: ALTA
- Item da matriz: NF-001, NF-002
- Local: `app/src-tauri/src/main.rs:207-281` (`load`), `:302-309` (`repo_refresh`); `app/ui/src/App.tsx:181-184,193`
- Evidência: depois de commit, checkout, merge, undo, fetch ou qualquer HEAD alterado fora do app, a UI chama `repo_refresh`, que executa `load()`: relê todas as refs, recalcula a impressão digital do cache (que **sempre muda** quando uma ref mexe), relê todo o histórico com `gix` (577 ms em 100k commits, `tests/bench/report-wide-100k.json`, `git_read`), refaz topologia e layout, roda `git status --untracked-files=all` e **recria o watcher** (`main.rs:306`, que ainda executa `git ls-files --others --ignored`). A UI então remonta o grafo inteiro (PERF-08).
- Por que importa: é a causa mais provável do "trava ao trocar de repositório / depois de operar". Em repositórios grandes cada commit custa segundos.
- Sugestão de correção: refresh incremental. Reler só refs e HEAD (`for-each-ref`), acrescentar commits novos com `rev-list <antigo>..<novo>` e recalcular o layout só das linhas afetadas; manter o watcher vivo; fazer o cache do histórico tolerar avanço de refs.
- Esforço: G

### [PERF-02] Blur de vidro (`backdrop-filter`) em painéis grandes e permanentes
- Severidade: ALTA (Hipótese)
- Item da matriz: NF-001b
- Local: `app/ui/src/styles.css:373-390` (`--lg-blur: saturate(180%) blur(28px)` aplicado a `.left`, `.right`, `.modal`, `menu.ctx`, `.toast` e grupos da barra), `:220`, `:291`
- Evidência: o mesmo `backdrop-filter` com raio de 20-28 px cobre as duas laterais inteiras e fica sobre a tela, enquanto o canvas do grafo repinta a cada scroll. Existe fallback só para `prefers-reduced-transparency` (`:317`, `:445`), mas o usuário não consegue ligá-lo.
- Por que importa: no WebKitGTK o blur de fundo é caro (composição com saturação por frame), especialmente em notebook com GPU integrada e pouca RAM. Costuma ser o primeiro suspeito de "UI pesada".
- Medição que confirma: abrir `Ctrl+Shift+P` (painel de desempenho já existe) e comparar fps e uso de CPU com `backdrop-filter: none` nas laterais.
- Sugestão de correção: fundo sólido ou levemente translúcido sem blur em `.left`/`.right`; blur só em overlay (modal/menu) e com raio ≤ 12 px; opção "Reduzir transparência" nas configurações.
- Esforço: P

### [PERF-03] Cache de chunks do grafo nunca é liberado
- Severidade: ALTA
- Item da matriz: NF-002
- Local: `app/ui/src/features/graph/GraphView.tsx:52,85-105`; `savedScroll` global em `:29`; `App.tsx:261-266` (`closeTab`)
- Evidência: `chunks` é um `Map` sem limite. O componente é reutilizado entre abas (comentário em `:51`), então os chunks de todos os repos abertos e de tudo que o usuário já rolou ficam retidos, e fechar a aba não os remove. Cada chunk guarda 200 linhas com segmentos e texto.
- Por que importa: crescimento contínuo de memória durante a sessão; em máquina de 7 GB com swap cheio (HANDOFF) isso leva a OOM e ao "app fecha do nada".
- Sugestão de correção: LRU de ~20 chunks por repositório; limpar o cache ao fechar a aba; limpar `savedScroll` junto.
- Esforço: P

### [PERF-04] Diff sem limite e sem virtualização
- Severidade: ALTA
- Item da matriz: F-012
- Local: `crates/ak-core/src/read.rs:111-118` (`git show` sem limite), `app/ui/src/features/diff/DiffView.tsx:49-` (todas as hunks e linhas no DOM)
- Evidência: o diff inteiro volta como `String`, é parseado por completo e renderizado com `<For>` em tabela. Não há teto de tamanho, aviso, nem virtualização.
- Por que importa: lockfile, arquivo minificado ou renomeação em massa congela o WebKit e estoura memória.
- Sugestão de correção: teto (ex.: 1 MB ou 5.000 linhas) com "Mostrar tudo", virtualização de linhas, e `--` com `-U` configurável.
- Esforço: M

### [PERF-05] Comandos `async` fazem trabalho bloqueante no runtime
- Severidade: MÉDIA (Hipótese)
- Item da matriz: NF-003
- Local: `main.rs:289-309` (`repo_open`/`repo_refresh`), `:333-363` (`graph_rows`, `commit_details`, `diff_*`), `:395-573` (`op_*`). Só `op_net` usa `spawn_blocking` (`:503`).
- Evidência: são `async fn`, mas chamam `Command::output()`/`wait_with_output()` sincronamente. Uma abertura longa ocupa uma thread do pool e atrasa as chamadas do grafo (`graph_window`, `graph_rows`).
- Sugestão de correção: envolver o trabalho de git em `spawn_blocking` (ou usar `tokio::process`), com um limite de concorrência.
- Esforço: M

### [PERF-06] `git status --untracked-files=all` a cada evento e a cada foco da janela
- Severidade: MÉDIA
- Item da matriz: NF-001
- Local: `crates/ak-core/src/read.rs:143`; `App.tsx:289-291` (evento `repo-changed` e `window focus`)
- Evidência: o status completo, com todos os arquivos não rastreados listados, roda a cada rajada do watcher (debounce 350 ms, `main.rs:198`) e de novo a cada foco da janela, sem coalescer os dois.
- Sugestão de correção: `--untracked-files=normal` na listagem e expandir diretórios sob demanda; ignorar o foco se houve evento nos últimos segundos.
- Esforço: P

### [PERF-07] Watcher recursivo pode estourar o limite do inotify em monorepos
- Severidade: MÉDIA (Hipótese)
- Item da matriz: NF-010
- Local: `main.rs:166-181`
- Evidência: só diretórios ignorados de **primeiro nível** são pulados. `packages/*/node_modules` entra na observação recursiva. A falha vira o banner `watch.failed` (`App.tsx:591`) e a UI deixa de se atualizar sozinha.
- Sugestão de correção: percorrer a árvore respeitando `.gitignore` (crate `ignore`) e vigiar só diretórios não ignorados; avisar o limite `fs.inotify.max_user_watches`.
- Esforço: M

### [PERF-08] Grafo é remontado a cada refresh e relê o texto de cada janela com um processo git
- Severidade: MÉDIA
- Item da matriz: F-007
- Local: `App.tsx:471-482` (`<Show when={graphRev} keyed>`); `GraphView.tsx:91`; `read.rs:189-205` (`git log --no-walk --stdin` por chunk de 200)
- Evidência: cada refresh descarta o componente e os chunks, e cada chunk faz duas chamadas IPC e um processo `git` novo. Rolar em repositório grande gera dezenas de processos.
- Sugestão de correção: cache de texto de linhas no core (ou leitura por `gix`, que já está no crate), invalidação seletiva em vez de remontagem.
- Esforço: M

### [PERF-09] Memória por aba e RSS acima da meta
- Severidade: MÉDIA
- Item da matriz: NF-002
- Local: `main.rs:32-48` (`Repo` guarda `Vec<String>` de shas + topologia + layout para cada aba até fechar)
- Evidência: HANDOFF registra RSS ~390 MB ocioso (PSS ~231 MB); a meta do PRD é 250 MB de **RSS**, e o critério foi trocado para PSS. As 40+ bytes de cada sha em `String` custam ~8 MB só em shas a cada 100k commits, por aba.
- Sugestão de correção: shas como `[u8; 20]`; descarregar topologia de abas inativas e recarregar ao voltar; medir também os processos do WebKit.
- Esforço: M

### [PERF-10] Custo de pintura do canvas
- Severidade: MÉDIA (Hipótese)
- Item da matriz: NF-001b
- Local: `GraphView.tsx:114-250`
- Evidência: o laço de pintura chama `token(...)` (lookup em objeto) várias vezes por linha e `ctx.measureText` por rótulo por frame; `createEffect(draw)` repinta em qualquer `tick`. O benchmark não mede fps da UI (`not_measured`).
- Sugestão de correção: resolver cores uma vez por tema, cachear largura de texto, repintar só linhas sujas.
- Esforço: M

### [PERF-11] Processos git sem timeout nem cancelamento
- Severidade: MÉDIA
- Item da matriz: NF-004
- Local: `crates/ak-gitcli/src/exec.rs:105-135,171-` (`wait_with_output` sem limite)
- Evidência: um `git` pendurado (hook, rede, credencial, lock) mantém o comando e o botão ocupado (`netBusy`) para sempre; só o Shared Stash tem cancelamento (`shared_stash::shared_stash_cancel`).
- Sugestão de correção: timeout configurável por tipo de operação e botão Cancelar nas operações de rede.
- Esforço: M

### [PERF-12] Nenhum log nem diagnóstico de falhas
- Severidade: ALTA
- Item da matriz: NF-003 (estabilidade)
- Local: `app/src-tauri/src/main.rs:638` (`.expect(...)`), `:173,190,292,297,307,313,318` (`Mutex::lock().unwrap()`); busca por `tracing|log::|panic::set_hook` não encontra nada
- Evidência: não há crate de log, arquivo de log, nem `panic::set_hook`. Se uma thread entra em pânico segurando um `Mutex`, o `unwrap()` seguinte também entra em pânico em cadeia.
- Por que importa: o relato do dono é "o app fecha do nada". Hoje não existe como descobrir por quê.
- Sugestão de correção: `tracing` + arquivo rotativo em `$XDG_STATE_HOME/atlasgit`, `panic::set_hook` que grava backtrace, tratar `PoisonError`, e banner "o app encerrou inesperadamente, abrir log".
- Esforço: P-M

## O que já está bom
- Core enxuto: 33 bytes por commit, layout de 100k commits em 4 ms, pico de 77 MB de RSS no core (`report-wide-100k.json`).
- Canvas com janela virtual de linhas e leitura do histórico com cache em disco (`cache.rs`).
- Trocar de aba não recarrega nada (só `setActive`).
- Escrita só via CLI do git, com locks (`ops/lock.rs`) e journal.

## Não verificado
fps real de rolagem, tempo de abertura, RAM do app completo (WebKit incluído), efeito do blur (PERF-02), comportamento com ≥ 8.192 watches. Plano de medição no roadmap, seção "Evidências a coletar na sua máquina".
