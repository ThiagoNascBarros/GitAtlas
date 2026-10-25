# 01 — Desempenho (área 1)

## Limitações
- Somente análise estática. O app não foi executado de forma interativa e esta auditoria não fez medição com profiler, fps ou RSS. A geração pesada de fixtures (`tests/fixtures/make.sh`) não foi executada (máquina com 7 GB de RAM).
- Os números citados vêm do `tests/bench/report-wide-100k.json` versionado (apenas wide-100k) e da leitura do código. Afirmações sobre custo de UI (nós DOM, re-parse) são inferidas do código, não medidas.

## Findings

### [PERF-1] Chamadas git bloqueantes rodam inline dentro de comandos Tauri `async`
- Severidade: MÉDIA
- Item da matriz: NF-002 (parcial: 60 fps / responsividade não verificados)
- Local: app/src-tauri/src/main.rs:288-505 (`repo_open`, `repo_refresh`, `graph_rows`, `commit_details`, `diff_file`, `diff_worktree`, `status_get`, `op_*`, `rebase_*`, `journal_*`)
- Evidência: apenas `op_net` (main.rs:503) e `shared_stash.rs:53` usam `tauri::async_runtime::spawn_blocking`. Todos os outros comandos são `async fn` que chamam `std::process::Command::output()` de forma síncrona, ex.: `diff_file` -> `reader.file_diff(...)` (git show), `status_get` -> `reader.status()` (git status). `grep -n spawn_blocking app/src-tauri/src` retorna só esses dois pontos. Comandos síncronos (`graph_window`, `repo_close`, `get_initial_path`) são baratos (encode em memória de no máximo 2000 linhas).
- Por que importa: isso bloqueia threads worker do Tokio (não a thread de UI). Um `git status` / `git diff` / rebase lento em repo grande ocupa um worker; com poucas chamadas concorrentes (status após evento do watcher + linhas do grafo + diff) o pool pode saturar e os demais comandos entram em fila. A UI só fica responsiva enquanto houver workers livres. Nenhum lock é mantido durante o git (ver "O que já está bom").
- Sugestão de correção: envolver os corpos que chamam git em `spawn_blocking` (helper pequeno como o de shared_stash.rs:53), ou tornar `Git::run` assíncrono com `tokio::process`.
- Esforço estimado: M

### [PERF-2] Todo refresh é um recarregamento completo; cada commit invalida o cache em disco
- Severidade: MÉDIA
- Item da matriz: NF-001, NF-002
- Local: app/src-tauri/src/main.rs:182-280 (`load`), app/src-tauri/src/main.rs:302 (`repo_refresh`), app/ui/src/App.tsx:181-232 (`refreshRepo`, `run`, `undo`, `redo`), crates/ak-core/src/cache.rs:28
- Evidência: `repo_refresh` chama `load()`: refs, head, `rev-parse --absolute-git-dir`, fingerprint, consulta ao cache, status, `Topology::from_commits`, `graph::layout`, `Engine::open`, `stashes()`. O fingerprint faz hash de todos os alvos de ref e do HEAD (cache.rs:28-35); qualquer commit/checkout/fetch o altera e todo o histórico é relido e re-serializado (`cache::save`). O bench `git_read` = 576,58 ms para 100k commits (report-wide-100k.json) é o piso por operação desse caminho em repo largo; layout (4 ms) e topologia (14,7 ms) são desprezíveis. Em App.tsx `run()` faz `await refreshRepo(...)` e depois `await loadStatus(...)` em série, e `load()` já executou `status()` uma vez, então o status roda duas vezes por operação. `refreshRepo` também zera `selected/sha/diffPath`.
- Por que importa: em repo de 100k commits cada commit/checkout/pull custa cerca de 0,6 s+ de leitura git mais uma escrita JSON completa, repetido a cada operação; em repos maiores escala linearmente. Não há atualização incremental (acrescentar commits novos).
- Sugestão de correção: atualização incremental do histórico quando a nova ponta descende da antiga (ler só `old..new` e prefixar); pular o refresh do grafo em operações que não movem refs; rodar `refreshRepo` e `loadStatus` em paralelo; reaproveitar o status já lido em `load`.
- Esforço estimado: G

### [PERF-3] File watcher: debounce só-trailing, watcher recriado a cada refresh, `.git` e dirs ignorados aninhados em modo recursivo
- Severidade: MÉDIA
- Item da matriz: fora da documentação (watcher; relacionado a NF-002)
- Local: app/src-tauri/src/main.rs:129-208 (`setup_watcher`), main.rs:296,306
- Evidência: (a) o laço de debounce `while let Ok(m) = rx.recv_timeout(Duration::from_millis(350)) { handle(m); }` não tem espera máxima, então eventos contínuos (saída de build em dir não ignorado, checkout longo) adiam `repo-changed` indefinidamente. (b) `start_watcher` é chamado em `repo_open` e em `repo_refresh`: cada refresh dispara `git ls-files --others --ignored --exclude-standard --directory -z`, re-registra watches inotify recursivos na árvore inteira e inicia uma nova thread; a thread do watcher antigo só termina quando seu canal fecha. (c) a detecção de ignorados é só de nível raiz: `packages/x/node_modules` ou `target/` dentro de subpasta é observado recursivamente; `.git` não é ignorado, então `.git/objects/**` é observado e filtrado em `relevant()` só depois de o evento ser entregue. (d) `is_ignored_dir` executa `git check-ignore` por novo diretório raiz dentro da thread do watcher. Ponto positivo: `GIT_OPTIONAL_LOCKS=0` (exec.rs:90) impede que `git status` toque `.git/index` e re-dispare o watcher.
- Por que importa: esgotamento de watches inotify (limites padrão 8192-65536) em monorepos grandes, CPU desperdiçada a cada refresh e possível inanição do refresh durante rajadas de build. `watch-failed` é o único fallback; o foco da janela também dispara `loadStatus` (App.tsx:291), o que mitiga estado velho.
- Sugestão de correção: manter o watcher entre refreshes (reinstalar só em repo_open); adicionar espera máxima (ex.: 2 s) ao debounce; observar explicitamente só `.git/{HEAD,index,refs,MERGE_HEAD}` em vez de todo o `.git`; respeitar ignores aninhados (ex.: `git check-ignore --stdin` em lote ou crate `ignore`).
- Esforço estimado: M

### [PERF-4] `git status --untracked-files=all` sem limite e renderizado sem virtualização (caso "muitos arquivos")
- Severidade: MÉDIA
- Item da matriz: NF-002 (caso muitos arquivos não coberto pelo bench)
- Local: crates/ak-core/src/read.rs:140-143, app/ui/src/features/wip/WipPanel.tsx:30,142-181, app/ui/src/App.tsx:162-180
- Evidência: o status roda com `["status","--porcelain=v2","-z","--branch","--untracked-files=all"]`. O WipPanel renderiza `<For each={groups().unstaged}>` / staged / conflicted como listas `<li>` simples, sem janelamento nem teto (os únicos `<For>` em features estão em WipPanel, StashPanel, RefMenu e no painel de refs do App; nenhum é virtualizado, só o grafo). `loadStatus` roda a cada `repo-changed` e a cada `focus` da janela, e faz `patchRepo` duas vezes (status e `diffRev+1`, que também recarrega o diff aberto via `createResource`).
- Por que importa: um build ou `npm install` em dir não ignorado, ou um clone novo com `.gitignore` incompleto, pode gerar dezenas de milhares de entradas: um `git status` grande a cada evento e dezenas de milhares de linhas DOM, todas re-chaveadas a cada atualização. Não medido.
- Sugestão de correção: virtualizar ou paginar as listas do WIP acima de um limite (ex.: 500) com "mostrar tudo"; limitar/colapsar diretórios não rastreados (`--untracked-files=normal` com expansão lazy); não incrementar `diffRev` quando o conteúdo do status do arquivo aberto não mudou.
- Esforço estimado: M

### [PERF-5] Visualização de diff: sem limite de tamanho, sem virtualização, parse não memoizado
- Severidade: MÉDIA
- Item da matriz: NF-002; F- visualização de diff (ver linhas de diff da matriz)
- Local: app/ui/src/features/diff/DiffView.tsx:29,47-96, app/ui/src/features/diff/parse.ts:9-35, crates/ak-core/src/read.rs:111-126,210-223
- Evidência: `const parsed = () => parseUnified(patch() ?? "");` é um accessor simples, não `createMemo`. É chamado em `parsed().binary` e `parsed().hunks` (linhas 47, 49), então o patch inteiro é dividido e parseado a cada avaliação desses bindings; `toSplit(h)` (linha 81) é recalculado a cada troca de modo. Cada linha vira um `<tr>` com 4 `<td>` (até 5 nós DOM por linha). O backend devolve a saída completa de `git show/diff` como uma única `String`, sem teto de tamanho.
- Por que importa: um diff de 50k linhas (lockfile, arquivo minificado ou gerado) significa cerca de 250k nós DOM mais parse repetido; espera-se travamento perceptível na thread de UI. Não medido.
- Sugestão de correção: `createMemo` para o parse; truncar ou paginar hunks acima de um orçamento de linhas com "carregar mais"; janelar hunks longos (altura de linha fixa); limitar bytes em `file_diff` com mensagem explícita de "grande demais".
- Esforço estimado: M

### [PERF-6] Cache em disco: reescrita JSON do arquivo inteiro, sem expurgo, escrito inline em `load`
- Severidade: BAIXA
- Item da matriz: NF-001 (caminho de first paint, SDD §6 passo 1)
- Local: crates/ak-core/src/cache.rs:44-66, app/src-tauri/src/main.rs:228-236
- Evidência: `save` serializa todos os `CommitRecord` (sha, parents, author, email, time, summary) com `serde_json::to_string` em uma única string, depois `fs::write`+`rename`; `load` lê e parseia o arquivo inteiro. O nome do arquivo é um hash `DefaultHasher` do git dir, um arquivo por repo em `$XDG_CACHE_HOME/atlasgit/graph/`, nunca removido nem limitado em tamanho. O fingerprint muda a qualquer movimento de ref (PERF-2), então o cache ajuda sobretudo em cold start com refs inalteradas.
- Por que importa: para 100k commits o JSON tem dezenas de MB (tamanho não medido), escrito de forma síncrona dentro de `load` a cada mudança de histórico; arquivos órfãos de repos apagados se acumulam.
- Sugestão de correção: escrever em thread de fundo; formato binário (bincode/rkyv) ou colunar compacto; apagar arquivos de cache não usados há N dias; append incremental.
- Esforço estimado: M

### [PERF-7] Cobertura de benchmark estreita; desempenho de UI não é medido nem protegido por gate
- Severidade: MÉDIA
- Item da matriz: NF-001, NF-002, NF-003, NF-004
- Local: tests/bench/report-wide-100k.json, crates/ak-core/examples/bench.rs, tests/fixtures/make.sh
- Evidência: o relatório traz `"not_measured": ["UI first paint","UI scroll fps","app idle RSS (shell + WebKit)"]`; só o repo `wide-100k` (25 lanes, 25 refs). Não há fixture nem benchmark para: histórico profundo/linear, muitas refs/tags, status com muitos arquivos, diff grande, rajadas do watcher, refresh repetido. O único conteúdo de `tests/bench` é esse JSON (sem runner). Existe um overlay de perf em Ctrl+Shift+P (features/graph/perf.ts, X-001/X-004), mas manual. O core passa: layout 4,07 ms (limite 400), primeira janela 0,077 ms (limite 300), 33 B/commit (limite 120), pico RSS do core 76,8 MB.
- Por que importa: os checks de NFR aprovados cobrem apenas a etapa pura-Rust do grafo; os custos que o usuário realmente sente (PERF-2 a PERF-5) não têm guarda contra regressão. O valor do HANDOFF PSS ~231 MB / RSS ~390 MB está perto da meta de 250 MB em idle (matriz NF-003).
- Sugestão de correção: adicionar fixtures (deep, status com muitos arquivos, diff grande) e um benchmark pequeno de `load()`, `status()`, `file_diff()`; job de CI comparando com o JSON versionado; medição automatizada de fps de scroll (Playwright/webkit) usando perf.ts.
- Esforço estimado: M

### [PERF-8] Custo do redraw do canvas do grafo e do trabalho por linha não verificado; processo `git log` por chunk
- Severidade: BAIXA
- Item da matriz: NF-002
- Local: app/ui/src/features/graph/GraphView.tsx:78-82,90-110,119-250, crates/ak-core/src/read.rs:189-206
- Evidência: pontos positivos: o canvas redesenha só as linhas visíveis (`first..last`), o backing store só é redimensionado quando o tamanho muda (linhas 125-126), o texto é buscado em chunks (`CHUNK`) com dedup via `pending`, o espelho de a11y fica limitado à viewport +- OVERSCAN (linha 408). Pontos em aberto: `refsByRow` monta o array de cada linha com `[...(m.get(r.row) ?? []), r]` (quadrático por linha, só importa com milhares de refs na mesma linha); cada chunk de metadados dispara `git log --no-walk=unsorted --stdin` (rows_text); `token(...)` é chamado por linha/pílula dentro de `draw()` e seu custo não foi inspecionado.
- Por que importa: provavelmente aceitável para 100k commits, mas scroll rápido dispara muitos spawns de processo git e lookups de estilo por frame não medidos.
- Sugestão de correção: medir com perf.ts em fixture de 100k; cachear resultados de `token()` por tema; prefetch/mesclar chunks adjacentes; usar `m.get(r.row)?.push` em vez de spread.
- Esforço estimado: P

## O que já está bom
- Nenhum `Mutex` é mantido enquanto o git roda: `get()` clona um `Arc<Repo>` e solta o lock de `repos` imediatamente (main.rs:316-318); `load()` roda sem lock.
- Operações de rede (`op_net`, shared stash) usam `spawn_blocking` e uma ponte askpass, então prompts de credencial não bloqueiam o runtime.
- Grafo: canvas virtualizado, janela binária de linhas (33 B/commit), texto lazy em chunks, `count.min(2000)` como teto; `malloc_trim` após o load; `shas` como `Vec<String>` simples, commits liberados após o layout.
- O watcher ignora dirs raiz ignorados (node_modules, target), observa a raiz de forma não recursiva, filtra ruído do `.git` (`relevant`), e `GIT_OPTIONAL_LOCKS=0` evita loop de status que se auto-dispara.
- O cache é protegido por fingerprint e escrito de forma atômica (temp + rename); qualquer erro significa "sem cache".
- Os NFRs de bench para layout do grafo/primeira janela/bytes por commit passam com grande folga.

## Não verificado
- fps real da UI, first paint, RSS/PSS em idle, cold start (NF-002, NF-003, NF-004).
- Tamanho do arquivo de cache JSON para 100k commits; comportamento de `token()` por frame; contagem real de nós DOM e tempo para um diff de 50k linhas ou status de 50k arquivos.
- Contagem de watches inotify em monorepo grande; duração de `git status` em repos grandes.
- Tamanho do pool de workers do Tokio e comportamento real de fila sob comandos concorrentes.
- Thread de execução dos comandos síncronos no Tauri 2 (assumida a main thread; eles apenas codificam dados em memória).
