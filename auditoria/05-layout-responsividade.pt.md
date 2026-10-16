# 05 · Layout e responsividade

## Limitações
Análise estática de `App.tsx`, `styles.css`, `features/split` e `tauri.conf.json`. Sem execução, não há medidas em HiDPI nem em gerenciadores de janela com tiling.

## Achados

### [LAY-01] Breakpoints abaixo de 900 px são código morto na janela padrão
- Severidade: BAIXA
- Item da matriz: NF-008
- Local: `app/src-tauri/tauri.conf.json:14` (`minWidth: 900`, `minHeight: 600`) contra `styles.css:305,309,359-360,444` (`max-width: 860px` e `620px`)
- Evidência: a janela nunca fica menor que 900 px, então as regras de 860 e 620 px nunca disparam. Em gerenciadores de janela com tiling (Sway, i3, GNOME com extensões) o `minWidth` é muitas vezes ignorado, e aí o layout "telefone" aparece sem ter sido testado: `.left { display: none }` esconde as branches inteiras (`styles.css:310-312`).
- Sugestão de correção: decidir o mínimo real (por exemplo 800 px), testar o layout nele e remover ou revisar os breakpoints abaixo; em largura estreita trocar o painel esquerdo por gaveta recolhível em vez de escondê-lo.
- Esforço: P

### [LAY-02] No tamanho mínimo, o grafo fica com ~300 px
- Severidade: MÉDIA
- Item da matriz: UI
- Local: `App.tsx:48-50` (esquerda 232 px, direita 360 px por padrão), `:468,511` (limites 140-420 e 240-600)
- Evidência: em 900 px de janela, 232 + 360 deixam o centro com ~300 px; a coluna do grafo (até 12 lanes × 16 px) e a mensagem competem por esse espaço. Os painéis laterais não recolhem; as larguras só variam por arraste.
- Sugestão de correção: botões de recolher/expandir cada lateral (com animação, ver UI-01) e atalho; reduzir o padrão da direita para ~320 px abaixo de 1100 px de janela; largura mínima do centro respeitada pelos splitters.
- Esforço: M

### [LAY-03] Painel de diff e detalhes competem pelo centro
- Severidade: MÉDIA
- Item da matriz: F-012
- Local: `App.tsx:470-509`, `styles.css:101,451`, `DiffView.tsx:16` (modo "full")
- Evidência: o diff abre embaixo do grafo, dividindo a altura (320 px por padrão, `App.tsx:50`); em tela de 768 px de altura sobra pouco para o grafo. Existe modo de tela cheia do diff, sem atalho visível.
- Sugestão de correção: opção de abrir o diff na lateral direita ou em página própria; atalho (F11/Alt+Enter) para tela cheia; lembrar a preferência.
- Esforço: M

### [LAY-04] Escala de fonte e HiDPI
- Severidade: MÉDIA
- Item da matriz: F-025, NF-008
- Local: `styles.css:5-7` (`13px`), `GraphView.tsx:10-12` (`ROW_H = 28`, `LANE_W = 16`, fontes do canvas em px)
- Evidência: o canvas trata `devicePixelRatio` corretamente (`GraphView.tsx:123-128`), mas fontes e linhas do canvas usam pixels fixos; não há zoom (PRD pede 80-200 %), então o texto não acompanha a escala do sistema/usuário. A altura de linha fixa em 28 px corta texto maior.
- Sugestão de correção: um fator de zoom global em `settings.json`, aplicado a `--ak-scale`, ao `ROW_H`/`LANE_W` e às fontes do canvas; atalhos Ctrl+= / Ctrl+- / Ctrl+0.
- Esforço: M

### [LAY-05] Splitters: estado só em `localStorage`
- Severidade: BAIXA
- Item da matriz: F-034
- Local: `features/split/Splitter.tsx:7-15`
- Evidência: larguras salvas por chave no `localStorage` do WebKit; se o armazenamento for limpo, voltam ao padrão, e não acompanham o restante das preferências.
- Sugestão de correção: persistir no mesmo `settings.json` da tela de configurações (FUN-06).
- Esforço: P

## O que já está bom
- Splitters com teclado (setas 16 px, Home/End) e limites.
- Canvas com `ResizeObserver` e atribuição de `width/height` apenas quando muda.
- Barra de ferramentas esconde rótulos antes de quebrar (`styles.css:304`).

## Não verificado
Janela maximizada em 4K, escala fracionária do Wayland, aparência a 125 %/150 % e comportamento em monitores mistos.
