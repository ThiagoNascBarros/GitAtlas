# 03 · UI visual, acessibilidade e i18n

## Limitações
Análise estática de CSS/TSX; as capturas de `docs/screenshots/phase5` não foram reavaliadas e o app não foi executado. Contraste vem de `scripts/contrast.mjs` (0 falhas).

## Achados

### [UI-01] Nenhuma animação de abrir/fechar componentes
- Severidade: MÉDIA
- Item da matriz: X-005
- Local: `app/ui/src/styles.css` (só `:332` transição de opacidade do splitter e `:485` spinner); `design-tokens.md §6` do REA define `motion.duration.fast/normal = 90/160 ms` e `motion.easing.standard`
- Evidência: modais, menus de contexto, toasts, banners de erro, painéis recolhíveis (`<details>` da lista de refs) e troca de aba aparecem e somem instantaneamente. Os tokens de movimento nunca foram implementados.
- Por que importa: é exatamente a queixa do dono ("não me agradam as animações, falta abertura e fechamento de componentes"): a interface "pisca" em vez de entrar e sair.
- Sugestão de correção: criar tokens `--ak-motion-fast/normal` e `--ak-ease`; entrada de modal (fade + escala 0,98→1, 160 ms) e backdrop (fade); menu (fade + 4 px); toast (slide + fade) e saída animada antes de desmontar (Solid `<Transition>` de `solid-transition-group` ou classe de saída com `animationend`); `<details>` com altura animada (`interpolate-size`/grid 0fr→1fr); todos desligados em `prefers-reduced-motion` (já existe a regra global `:198`) e numa chave "Reduzir animações" nas configurações. Animar só `opacity`/`transform`.
- Esforço: M

### [UI-02] Ícones repetidos ou que contradizem a ação
- Severidade: MÉDIA
- Item da matriz: UI
- Local: `features/graph/CommitMenu.tsx:47-51`; `features/refs/RefMenu.tsx:74-76`; `App.tsx:333-334`
- Evidência:
  - Cherry-pick usa `Redo2` e Revert usa `Undo2`, os **mesmos** ícones dos botões Desfazer/Refazer da barra (`App.tsx:333-334`). Isso mistura "reverter um commit" com "desfazer a última operação".
  - Reset soft, mixed e hard usam o mesmo `RotateCcw`; o hard (destrutivo) não se distingue visualmente.
  - Merge, Rebase e Rebase interativo usam `GitMerge` nas três linhas do menu de refs (`RefMenu.tsx:74-76`).
  - O ícone `Check` serve para "Checkout" (parece "confirmar").
  - `GitBranch` é o ícone da aba "Refs", de cada branch e do rótulo da branch na barra.
  - `Terminal` representa a versão do Git na barra de status (`App.tsx:666`); `Clock` para data de commit.
- Sugestão de correção (Lucide, mesma biblioteca): cherry-pick `Cherry`/`GitCommitHorizontal` + seta (ou `CopyPlus`), revert `Undo` substituído por `GitPullRequestArrow`/`Ban`/`History` rotacionado (evitar o par undo/redo), reset soft/mixed/hard em escada (`RotateCcw` neutro, `RotateCcw` + `TriangleAlert` e `Trash2` em vermelho para hard), merge `GitMerge`, rebase `GitCompareArrows`/`Split`, rebase interativo `ListOrdered`, checkout `LogIn`/`CornerDownRight`, versão do git `GitCommit`/`Info`. Definir uma tabela única de ícones por ação em `features/icons.ts` e testá-la contra duplicatas (teste simples).
- Esforço: P

### [UI-03] Barra superior sobrecarregada e sem tela de configurações
- Severidade: MÉDIA
- Item da matriz: F-034
- Local: `App.tsx:311-368`
- Evidência: numa só barra ficam abas, branch, Desfazer/Refazer, Fetch/Pull/Push, Nova branch, Stash, Journal, seletor de tema e seletor de idioma. Os `<select>` nativos de tema e idioma ficam no canto superior direito, com o código do idioma como texto sobreposto (`.lang-code`).
- Sugestão de correção: mover tema e idioma para uma tela Settings (FUN-06) acessada por uma engrenagem na barra; manter só ações do repositório na barra; agrupar Nova branch/Stash no menu "+".
- Esforço: M

### [UI-04] Glass/blur em excesso prejudica leitura e desempenho
- Severidade: MÉDIA
- Item da matriz: NF-007
- Local: `styles.css:373-393`, `:291`, `:446`
- Evidência: tinta translúcida (`color-mix 66-82 %`) mais blur de 28 px nas laterais e grupos da barra, e três camadas de regras `.modal`/`menu.ctx` sobrepostas (`:170`, `:291`, `:389`, `:436`) que se sobrescrevem. O contraste do script mede os tokens sólidos, não o texto sobre o fundo translúcido.
- Sugestão de correção: consolidar as definições de `.modal`/`.menu`/`.toast` em um único bloco; superfícies sólidas nas laterais; blur apenas em overlays. Ligado a PERF-02.
- Esforço: P

### [UI-05] Textos e estados do Git em inglês cru na interface
- Severidade: BAIXA
- Item da matriz: NF-009
- Local: `App.tsx:42-43` (`RAW_MESSAGE_CODES` mostra stderr bruto em "Detalhes")
- Evidência: erros do Git aparecem como texto bruto em inglês em `Details`; a mensagem principal é traduzida e há 10 códigos cuja dica continua em inglês (HANDOFF).
- Sugestão de correção: código de dica vindo do core e `err.<CODE>.fix` completo em pt-BR.
- Esforço: M

### [UI-06] Acessibilidade: itens diferidos
- Severidade: BAIXA
- Item da matriz: NF-007
- Local: `docs/a11y/audit.md` F-14, F-15, F-23
- Evidência: redimensionar colunas só com mouse, ordem de Tab diff vs. detalhes, cabeçalho do grafo fora do grid; Orca e a tecla Menu no WebKitGTK não foram testados.
- Esforço: M

### [UI-07] Toast fixo de 10 s, sem pausa ao passar o mouse
- Severidade: BAIXA
- Local: `App.tsx:107-108,197-198`
- Evidência: o toast com "Desfazer" some após 10 s mesmo com o ponteiro sobre ele, e erros e confirmações compartilham o mesmo mecanismo.
- Esforço: P

## O que já está bom
- Tokens de cor em dois temas, com verificação de contraste automatizada.
- Biblioteca de ícones única (Lucide) e `aria-hidden` consistente.
- i18n completo com paridade en/pt-BR garantida por tipo e teste.
- Respeito a `prefers-reduced-motion`, `prefers-reduced-transparency` e `prefers-contrast`.

## Não verificado
Aparência real em HiDPI, em tema claro, e legibilidade do texto sobre as áreas translúcidas.
