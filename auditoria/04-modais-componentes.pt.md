# 04 · Modais e componentes

## Limitações
Análise estática de `features/*` e `overlay.ts`. Medidas de tamanho vêm do CSS, não de captura de tela.

## Inventário de modais e menus

| Componente | Arquivo | Tamanho (CSS) | Conteúdo |
|---|---|---|---|
| Nome (branch, stash, credencial) | `refs/NameDialog.tsx` | `.modal` 640 px (`styles.css:170`) | 1 campo + 2 botões, sem título |
| Mesclar | `refs/RefMenu.tsx:103-122` | `.modal` 640 px | texto + lista de conflitos |
| Conflito | `conflict/ConflictEditor.tsx` | até 1100 px, `max-height: 80vh` (rola) | 2 `<pre>` + textarea |
| Rebase interativo | `rebase/RebaseEditor.tsx` | 820 px | lista reordenável |
| Journal | `journal/JournalPanel.tsx` | 640 px | lista |
| Compartilhar stash | `shared-stash/ShareDialog.tsx` | 640 px | formulário |
| Menus de contexto | `RefMenu`, `CommitMenu`, `SharedStashMenu`, `LocalStashMenu` | `menu.ctx` | listas de ações |

## Achados

### [MOD-01] "Nova branch" é um modal de um campo, grande demais e pobre
- Severidade: ALTA
- Item da matriz: F-017, X-001, X-002
- Local: `NameDialog.tsx:12-36`, `App.tsx:640-650`
- Evidência: o diálogo ocupa 640 px de largura para um único campo, sem título (só um rótulo), sem validação ao vivo (o erro só vem do core depois do OK), sem base, sem checkout e sem Git Flow.
- Sugestão de correção: componente `BranchDialog` de ~480 px com título "Nova branch", campos: **tipo** (Livre, Feature, Fix, Release, Hotfix; só aparece com Git Flow ativo), **identificador/ticket** (ex.: `ACT-1010`), **título**, **base** (HEAD, branch, commit), **prévia do nome final** em fonte mono com validação ao vivo, "Mudar para a nova branch", e "Criar e publicar" opcional. Estrutura detalhada em FUN-03/FUN-04.
- Esforço: M

### [MOD-02] Todos os diálogos usam a mesma largura e nenhum título/rodapé consistente
- Severidade: MÉDIA
- Item da matriz: UI
- Local: `styles.css:170,454,469`
- Evidência: `.modal` fixa 640 px (inclusive para um campo), conflito 1100 px dentro de um modal de rolagem (`max-height: 80vh`), rebase 820 px. Não há tamanhos `sm/md/lg/full`, botão fechar (X) no cabeçalho, nem rodapé fixo; o rodapé de botões rola junto com o conteúdo quando o conteúdo é longo.
- Sugestão de correção: componente base `Modal` com `size="sm|md|lg|full"` (400/560/820/tela cheia), cabeçalho com título e X, corpo rolável, rodapé fixo. Conflito e rebase em `lg`/`full`; confirmação em `sm`.
- Esforço: M

### [MOD-03] Editor de conflito pequeno para a tarefa
- Severidade: ALTA
- Item da matriz: F-019
- Local: `ConflictEditor.tsx:38-66`, `styles.css:454-461`
- Evidência: dois painéis de 28 vh e um `textarea` de 30 vh dentro de um modal de no máximo 80 vh: cada painel mostra poucas linhas de código. Comparar e escolher exige rolar várias áreas.
- Sugestão de correção: tela cheia como "visão" do app (como o GitKraken troca o centro por ferramenta de merge), com 3 painéis lado a lado ou 2 em cima e saída embaixo, ocupando toda a altura. Detalhes em FUN-01.
- Esforço: G (junto com FUN-01)

### [MOD-04] Confirmações destrutivas inconsistentes
- Severidade: MÉDIA
- Item da matriz: NF-005
- Local: `WipPanel.tsx:85-93` (duplo clique em 3 s), `StashPanel.tsx:15-17` (duplo clique), `JournalPanel.tsx:19`, `App.tsx:605-609` (push forçado, duplo clique), `CommitMenu.tsx` e `RefMenu.tsx:75-76` (nenhuma)
- Evidência: o padrão "clique de novo para confirmar" aparece em quatro lugares com temporizadores diferentes e some se o usuário demorar; reset hard e force delete executam direto.
- Sugestão de correção: um único `ConfirmDialog` (tamanho `sm`, botão destrutivo vermelho, descreve o que se perde e menciona "Desfazer" quando existir).
- Esforço: P

### [MOD-05] Menus de contexto sem agrupamento nem separadores
- Severidade: BAIXA
- Item da matriz: UI
- Local: `CommitMenu.tsx:46-55` (10 itens contínuos), `RefMenu.tsx:66-99`
- Evidência: ações seguras e destrutivas (reset hard) lado a lado, sem separador, sem atalhos exibidos e sem ordem por frequência.
- Sugestão de correção: grupos (Navegar, Criar, Reescrever histórico, Copiar), separador antes das ações destrutivas, item destrutivo em vermelho, atalho à direita.
- Esforço: P

### [MOD-06] Estados vazio/carregando/erro irregulares
- Severidade: BAIXA
- Item da matriz: UI
- Local: `App.tsx:372-389` (vazio), `:523` (detalhes), `DiffView.tsx` ("loading" em texto), `GraphView.tsx:517`
- Evidência: o carregamento é só o texto "Carregando…"; a tela inicial não oferece recentes (FUN-05) nem clone/init; o erro de chunk do grafo aparece como faixa sem retry (`graph-error`, repete a cada redesenho).
- Sugestão de correção: skeletons nas listas e no grafo, botão "Tentar de novo" no erro do grafo.
- Esforço: M

## O que já está bom
- `useOverlay` entrega Escape, foco ao abrir e devolver foco e armadilha de Tab (`overlay.ts`), usado nos 8 diálogos.
- Backdrop não fecha o diálogo, evitando perder texto digitado.
- Preview de conflitos no merge antes de confirmar; menu acessível por teclado (Menu/Shift+F10).

## Não verificado
Posicionamento dos menus em janelas pequenas e em Wayland; comportamento do foco com leitores de tela.
