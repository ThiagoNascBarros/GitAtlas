# Área 4 — Modais e componentes (auditoria)

## Limitações
- Somente análise estática (código em `app/ui/src`, `crates/ak-core/src/ops`). A UI NÃO foi executada no painel Browser e nenhuma captura de tela foi feita (`auditoria/screenshots/` intocado); todo comportamento abaixo deriva do código.
- `/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/en/ux-analysis.md` não existe nesse caminho (leitura falhou); a premissa "toast com stderr cru é uma falha" vem do briefing da tarefa, não foi verificada no doc REA.
- IDs da matriz usados: UI-004 (menus de contexto), UI-005 (diálogos), UI-006 (mensagens de erro), UI-007 (painel WIP), F-021 (abortar), F-022 (stash), F-023 (rede), F-025 (undo/journal), F-019 (merge). Operações destrutivas não constam individualmente na matriz.
- Números de linha de `App.tsx` vêm de `grep -n` na árvore atual.

## Mapa de confirmação de operações destrutivas (verificado no código)
| Operação | Confirma? | Nomeia o que se perde? | Recuperável? | Evidência |
|---|---|---|---|---|
| Descartar arquivo | 2º clique, timer 3 s | Não (botão diz "Discard? (click again)"; o caminho só aparece no toast depois) | Sim, Undo (backup) | `WipPanel.tsx:51-58` |
| Reset hard | nenhuma | Só no rótulo: "hard: discard changes" | Sim, snapshot + Undo | `CommitMenu.tsx:198,220`; `history.rs:82` |
| Excluir branch / forçar | nenhuma | Rótulo "Undo brings it back" só no forçado | Sim, Undo | `RefMenu.tsx:316-317` |
| Excluir tag | nenhuma | não | Sim (journal `tag_delete`) | `RefMenu.tsx:310`; `branch.rs:12` |
| Stash drop | 2º clique, reseta só no blur | não | Sim (journal `stash_drop`) | `StashPanel.tsx:274-278` |
| Force push | 2º clique no banner de erro | Sim ("replaces the remote branch"), usa `--force-with-lease` | Não (remoto) | `App.tsx:607-611`; `remote.rs:116` |
| Unshare stash compartilhado | diálogo real | Sim (quem perde, como recuperar) | Sim ("Recently unshared") | `SharedStashMenu.tsx` UnshareDialog |
| Descartar stash mantido | 2º clique | não | Não | `SharedStashPanel.tsx:132` |
| Podar journal | 2º clique | contagem no rótulo | Não | `JournalPanel.tsx:148-158` |
| Abortar merge/rebase/cherry-pick | nenhuma | não | NÃO (sem journal) | `WipPanel.tsx:136`; `merge.rs:133-139` |
| Rebase sobre (não interativo) | nenhuma | não | Sim (journal `rebase`) | `RefMenu.tsx:305` |
| Rebase interativo | editor + Start | lista de commits | Sim | `RebaseEditor.tsx` |
| Merge | diálogo com previsão de conflitos | lista de arquivos | Sim | `RefMenu.tsx:323-341` |

## Findings

### [MOD-1] Sem ErrorBoundary: um resource que falha pode deixar o app em branco
- Severidade: ALTA
- Item da matriz: UI-006
- Local: `app/ui/src/App.tsx:304` (details), `features/diff/DiffView.tsx:24`, `features/journal/JournalPanel.tsx:11`, `features/stash/StashPanel.tsx:11`, `features/shared-stash/SharedStashMenu.tsx:71`
- Evidência: `grep -rn "ErrorBoundary\|unhandledrejection" app/ui/src` não retorna nada. Esses fetchers de `createResource` não têm `.catch`, ao contrário de `RefMenu.tsx:25`, `RebaseEditor.tsx:17`, `ConflictEditor.tsx:78`. Exemplo: `createResource(() => ..., ([id, sha]) => api.details(id, sha))`, lido por `<Show when={details()} ...>`. No Solid, ler um resource rejeitado lança durante a renderização; sem boundary, o `render(() => <App />)` raiz (`main.tsx:14`) é desmontado. Não reproduzido em runtime.
- Por que importa: um commit que some após reescrita externa, falha ao ler lista de stash ou journal (IO_ERROR) deixa janela em branco sem mensagem; o usuário perde abas/estado abertos.
- Sugestão de correção: envolver `<App/>` (e cada painel) em `<ErrorBoundary fallback>` mostrando `errorText`; adicionar `.catch` chamando `fail(e)` nos cinco resources; registrar `unhandledrejection` para `fail`.
- Esforço estimado: P

### [MOD-2] Abortar merge/rebase/cherry-pick não tem confirmação e não pode ser desfeito
- Severidade: ALTA
- Item da matriz: F-021
- Local: `app/ui/src/features/wip/WipPanel.tsx:136`, `crates/ak-core/src/ops/merge.rs:133-139`
- Evidência: `onClick={() => void props.run(() => api.mergeAbort(id), { graph: true, toast: t("toast.mergeAborted") })}`; o core executa `git <what> --abort` e retorna `OpOutcome { head, ..Default::default() }` (sem entrada no journal, então o toast não tem Undo).
- Por que importa: abortar descarta todas as resoluções de conflito já salvas na árvore (inclusive as feitas no ConflictEditor); um clique errado no botão de perigo perde tudo, sem aviso nomeando a perda.
- Sugestão de correção: diálogo de confirmação ("Abort merge? Resolutions of N files will be lost") com foco inicial em Cancel; ou snapshot com `git stash create` antes de abortar e registrar no journal para Undo (como o reset hard faz).
- Esforço estimado: P

### [MOD-3] Reset hard roda imediatamente a partir do menu do commit
- Severidade: MÉDIA
- Item da matriz: F-025
- Local: `app/ui/src/features/graph/CommitMenu.tsx:198,220`
- Evidência: `const reset = (mode) => go(() => api.history(..."reset", sha(), { mode }), ...)` e `{item(..., t("commitmenu.resetHard"), () => reset("hard"))}`; sem diálogo. O core mitiga: "a hard reset keeps a snapshot of the discarded changes for Undo" (`history.rs:82`). O Undo do toast dura 10 s (`App.tsx:198`); depois só Ctrl+Z / Journal.
- Por que importa: destrutivo e alcançável por teclado (ArrowDown x5 + Enter) sem prévia de quantos arquivos/commits se perdem; a rede de segurança depende de o usuário notar o toast de 10 s.
- Sugestão de correção: diálogo de confirmação só para `hard`, listando arquivos não commitados e commits que ficam inalcançáveis; soft/mixed continuam diretos.
- Esforço estimado: P

### [MOD-4] Excluir branch, excluir forçado e excluir tag não têm confirmação
- Severidade: MÉDIA
- Item da matriz: UI-004
- Local: `app/ui/src/features/refs/RefMenu.tsx:310,316-317`
- Evidência: `go(() => api.branchDelete(props.repoId, r().name, true), ...)` direto do item de menu; os dois itens de exclusão ficam lado a lado e diferem só pelo rótulo ("Delete" / "Delete anyway (Undo brings it back)", `i18n.ts:138`). Desfazível via journal (`branch.rs:12`).
- Por que importa: a exclusão forçada tira da visão commits não mesclados; escolher o item vizinho errado exclui na hora. A exclusão segura já falha com `BRANCH_NOT_MERGED` (bom), então o caminho forçado poderia perguntar só nesse caso.
- Sugestão de correção: sem confirmação na exclusão segura; em `BRANCH_NOT_MERGED` abrir diálogo com a contagem de commits não mesclados e "Delete anyway"; confirmar exclusão de tag ou ao menos manter o toast com Undo.
- Esforço estimado: P

### [MOD-5] Rebase não interativo começa direto do menu de refs
- Severidade: BAIXA
- Item da matriz: UI-004
- Local: `app/ui/src/features/refs/RefMenu.tsx:305`
- Evidência: `go(() => api.rebase(props.repoId, r().name), t("toast.rebased"))`; o item de merge abre diálogo com prévia (`RefMenu.tsx:323`), o rebase não.
- Por que importa: reescrita de histórico sem confirmação nem previsão de conflitos, enquanto o merge tem ambos; inconsistente. Desfazível (`rebase` em `branch.rs:12`), por isso BAIXA.
- Sugestão de correção: reutilizar o padrão do diálogo de merge (commits a reaplicar, Cancel focado).
- Esforço estimado: P

### [MOD-6] Estado de confirmação do force push sobrevive ao descarte do banner
- Severidade: MÉDIA
- Item da matriz: F-023
- Local: `app/ui/src/App.tsx:68,607-611`
- Evidência: `onClick={() => { if (!forceConfirm()) return setForceConfirm(true); setForceConfirm(false); setError(null); void net("push", true); }}`; o botão Dismiss é só `setError(null)` e nunca reseta `forceConfirm`. Após "Force push" -> Dismiss, o próximo banner `PUSH_REJECTED` já aparece armado ("Click again: replaces the remote branch") e um clique faz o force push.
- Por que importa: o rótulo armado avisa, mas a proteção de dois passos é reduzida silenciosamente. `--force-with-lease` (`remote.rs:116`) limita o dano.
- Sugestão de correção: resetar `forceConfirm` quando o erro mudar/for descartado (`createEffect(on(error, () => setForceConfirm(false)))`) e adicionar reset por blur/timeout.
- Esforço estimado: P

### [MOD-7] Escape fecha todos os overlays abertos de uma vez (sem tratar só o mais interno)
- Severidade: MÉDIA
- Item da matriz: UI-005
- Local: `app/ui/src/features/overlay.ts:37-41` vs a pilha de traps em `:27,44`
- Evidência: o Tab respeita `traps[traps.length - 1] !== self`, mas o Escape não: `if (e.key === "Escape") { e.preventDefault(); onClose(); return; }` roda no listener de document de todo overlay montado. O diálogo de credenciais é montado na raiz do app independentemente de outros overlays (`App.tsx:633`, evento `credentials-request` a qualquer momento, p.ex. durante fetch do Shared Stash com menu/Journal aberto).
- Por que importa: o Escape destinado a cancelar o prompt de credenciais também fecha o Journal/diálogo de ref (e vice-versa, Escape num menu responde `null` a um pedido de credenciais pendente). Idem o listener de Escape do DiffView (`DiffView.tsx:22`).
- Sugestão de correção: em `key`, tratar Escape só se `traps[traps.length-1] === self` (registrar em `traps` mesmo sem `root`).
- Esforço estimado: P

### [MOD-8] Confirmações "click again": sem anúncio, sem atraso mínimo, texto fraco
- Severidade: MÉDIA
- Item da matriz: UI-007
- Local: `WipPanel.tsx:51-58,77`, `StashPanel.tsx:274-278,305-306`, `JournalPanel.tsx:185`, `SharedStashPanel.tsx:132`
- Evidência: a única mudança é o texto do botão (`{confirm() === item.path ? t("wip.discardConfirm") : t("wip.discard")}`) sem região live e sem mudança de foco; timer de 3 s no discard, mas o stash drop só reseta no blur; um duplo clique dispara os dois cliques e confirma na hora; o texto "Discard? (click again)" não diz o que se perde, e a mesma redação cobre apagar arquivo não rastreado (itens `UNTRACKED` passam pelo mesmo `discard()`).
- Por que importa: usuários de leitor de tela não ouvem a mudança de estado; duplo clique rápido derrota a proteção; o usuário não é avisado de que as alterações (ou o arquivo não rastreado) somem.
- Sugestão de correção: anúncio `aria-live` ou `alertdialog` real para discard/drop; ignorar o segundo clique em ~400 ms; nomear a perda ("Discard 12 changed lines in a.ts" / "Delete untracked file").
- Esforço estimado: M

### [MOD-9] Diálogos são `<dialog open>` sem `showModal()`; a página atrás não fica inerte
- Severidade: BAIXA
- Item da matriz: UI-005
- Local: todos os diálogos, p.ex. `NameDialog.tsx:156`, `RebaseEditor.tsx:32`, `ConflictEditor.tsx:98`
- Evidência: `<dialog ref={dialog} open class="modal" aria-modal="true" ...>`; a contenção depende do handler de Tab em `overlay.ts:43-48`.
- Por que importa: navegação por ponteiro/cursor virtual de TA pode alcançar o fundo; `docs/a11y/audit.md:36` declara Pass só para teclado. Cliques do mouse são bloqueados por `.modal-back` (verificado no CSS `styles.css:168`).
- Sugestão de correção: aplicar `inert` na raiz do app enquanto houver trap, ou usar `showModal()`.
- Esforço estimado: M

### [MOD-10] Texto em inglês vaza em pt-BR para erros sem catálogo; GIT_FAILED não tem fix localizado
- Severidade: BAIXA
- Item da matriz: UI-006
- Local: `app/ui/src/i18n.ts:450-458`; strings do core, p.ex. `stash.rs:96`, `history.rs:60`, `remote.rs:29`
- Evidência: `errorFix` retorna `e.suggestion` (inglês, vindo do core) quando não há chave `err.<CODE>.fix`; `GIT_FAILED` tem `err.GIT_FAILED` mas não `.fix`, então "Commit or stash your local changes first." aparece sem tradução. `errorText` também cai em `e.message`. Os 38 códigos emitidos por `OpError::new/UiError::new` têm chave `err.` (checado por script); o stderr cru fica só em `detail` atrás do toggle "Details" (banner em `App.tsx`, `showDetail`).
- Por que importa: lacuna parcial de i18n; o tratamento do stderr em si atende o objetivo "sem toast de stderr cru".
- Sugestão de correção: adicionar chaves `err.GIT_FAILED.fix`/`.fix`, ou o core enviar códigos de sugestão.
- Esforço estimado: P

### [MOD-11] Sem guarda de reentrância em `run()`, undo/redo e botões de stash
- Severidade: BAIXA
- Item da matriz: F-025
- Local: `App.tsx` `run()` (~linha 185), Undo/Redo da toolbar (só `disabled={!tab()}`), `StashPanel.tsx:303-305`
- Evidência: só existem `netBusy` (fetch/pull/push), `busy` do WipPanel (commit) e `busy` do ShareDialog. Ctrl+Z repetido / duplo clique em Apply/Pop geram chamadas concorrentes; o lock do core responde `REPO_BUSY` (`err.REPO_BUSY` existe), que então aparece como banner de erro.
- Por que importa: erros espúrios; dois Pops rápidos poderiam aplicar duas vezes se a janela do lock falhar.
- Sugestão de correção: sinal global `opBusy` definido em `run/undo/redo`, desabilitando esses botões.
- Esforço estimado: P

### [MOD-12] Toast de slot único, some em 10 s/4 s e não pausa em hover/foco
- Severidade: BAIXA
- Item da matriz: UI-006
- Local: `App.tsx:108,197-198,216,235,655`
- Evidência: `toastTimer = window.setTimeout(() => setToast(null), 10000)`; todo `setToast` substitui o anterior, inclusive seu botão Undo. `<output class="toast">` é região live (bom).
- Por que importa: uma segunda operação em 10 s remove o Undo da primeira do toast (Ctrl+Z/Journal continuam); o tempo é difícil para teclado/TA (WCAG 2.2.1).
- Sugestão de correção: pausar o timer em hover/foco, manter uma fila curta.
- Esforço estimado: P

### [MOD-13] Diálogo do Journal mostra "vazio" durante o carregamento e não tem estado de erro
- Severidade: BAIXA
- Item da matriz: F-025
- Local: `features/journal/JournalPanel.tsx:11,33`
- Evidência: `<Show when={(entries() ?? []).length} fallback={<p class="muted">{t("journal.empty")}</p>}>` — durante o load `entries()` é undefined, então "vazio" aparece; um load rejeitado lança (ver MOD-1). Os diálogos de Rebase/Conflict/Merge mostram `t("loading")`.
- Por que importa: mensagem errada piscando; erro não exibido no diálogo.
- Sugestão de correção: ramificar em `entries.loading` / `entries.error`.
- Esforço estimado: P

### [MOD-14] Prompt de credenciais mostra só o prompt cru do git, sem contexto de repo/remoto
- Severidade: BAIXA
- Item da matriz: UI-005
- Local: `App.tsx:633-635`, `app/src-tauri/src/askpass.rs:139`
- Evidência: `<NameDialog label={c.prompt || t("creds.title")} secret={c.secret} .../>`; o payload tem `repo_id` mas a UI não mostra repo, remoto nem operação; pedidos enfileirados aparecem um por vez sem contador. A senha não é aparada (`NameDialog.tsx:151`, bom); input é `type=password` e `autocomplete=off`. Escape responde `null` (cancela).
- Por que importa: com várias abas ou fetch do Shared Stash em segundo plano, o usuário não sabe por que uma senha é pedida.
- Sugestão de correção: adicionar título com nome do repo e operação; mostrar "1 de N".
- Esforço estimado: P

## O que já está bom
- Focus trap com pilha para Tab, Escape, retorno de foco e helpers de tecla de menu (`overlay.ts`, testado em `overlay.test.ts`); roles/`aria-labelledby` em todo diálogo; menus usam `role=menu/menuitem` com setas.
- NameDialog não fecha no clique do backdrop (sem perda de texto); o diálogo de Merge foca Cancel primeiro e prevê conflitos; Unshare é confirmação real nomeando consequências; ConflictEditor bloqueia salvar com marcadores restantes; RebaseEditor valida squash-primeiro e reword vazio.
- Undo via journal cobre discard, reset hard, exclusão de branch/tag, stash drop e rebase; force push usa `--force-with-lease`.
- Erros mapeiam para códigos estáveis com texto localizado (todos os 38 códigos emitidos têm `err.*`), stderr escondido em "Details", `role="alert"` no banner, `<output>` live para toasts.
- Guardas de duplo envio existem para commit (`busy`), rede (`netBusy`) e ShareDialog (`busy`).

## Não verificado
- Renderização real, ordem de foco, saída de leitor de tela e capturas de tela (UI não executada).
- Se o MOD-1 realmente deixa o app em branco no webview do Tauri (comportamento do Solid inferido do código, não reproduzido).
- Comportamento real do GitKraken (REA ux-analysis.md não encontrado).
- Se o Undo de reset hard / discard restaura corretamente (há testes do core em `ops.rs`, não reexecutados aqui).
- Empilhamento CSS do banner de erro atrás de `.modal-back` durante erros em modal (p.ex. falha ao salvar no ConflictEditor mantém o diálogo aberto; o banner fica escurecido pelo backdrop, `styles.css:168`).
