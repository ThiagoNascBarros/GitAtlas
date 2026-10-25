# 05 - Layout e responsividade

## Limitações
- Auditoria só por leitura de código: App.tsx, styles.css, features/split/Splitter.tsx, features/graph/columns.ts, GraphView.tsx, tauri.conf.json, docs/a11y/audit.md, PRD REA e `03-design/layout-direction.md`.
- Não executei a UI vite/devMock nem redimensionei o painel Browser: não há screenshots; efeitos em execução são inferidos de CSS/JS.
- Comportamento do WebKitGTK (escala de fonte do sistema, DPR por monitor, escala fracionária no Wayland) não foi testado.

### [LAY-1] Limites dos painéis são estáticos e tamanhos salvos não são limitados
- Severidade: ALTA
- Item da matriz: UI-003
- Local: app/ui/src/App.tsx:468,511; features/split/Splitter.tsx:11; styles.css:329,359
- Evidência: `min={140} max={420}` (esquerdo), `min={240} max={600}` (direito); `storedSize` aceita qualquer valor salvo `saved > 0 ? saved : initial`. tauri.conf.json `minWidth: 900`. Máx. esquerdo + máx. direito = 1020 px > 900 px. `.body` é `var(--left-w) 0 minmax(0, 1fr) 0 var(--right-w)`; o limite de largura só existe em `@media (max-width: 860px)`, inalcançável com minWidth 900.
- Por que importa: na janela mínima, ou com tamanhos salvos grandes (ou valor editado como 5000), o painel central com grafo e diff pode colapsar a 0 px.
- Sugestão de correção: limitar ao carregar e ao redimensionar a janela pela largura real; reservar mínimo central (`minmax(320px, 1fr)`).
- Esforço estimado: P

### [LAY-2] Breakpoints abaixo da janela mínima são código morto
- Severidade: MÉDIA
- Item da matriz: UI-003
- Local: app/ui/src/styles.css:305-316,359-363; src-tauri/tauri.conf.json (minWidth 900, minHeight 600)
- Evidência: `@media (max-width: 860px)` e `(max-width: 620px)` (empilhar, ocultar refs, ocultar splitters) estão abaixo de `minWidth: 900`; só as regras de 1100 e 1400 px disparam. Em 900 px os painéis padrão de 232 + 360 px deixam cerca de 308 px ao centro.
- Por que importa: o "abaixo de 620 px o painel de refs some" documentado (layout-direction.md:86) nunca ocorre no app, enquanto o mínimo real (900 px) não é ajustado.
- Sugestão de correção: adicionar breakpoint em 900-1000 px (painéis mais estreitos) ou reduzir minWidth de forma consistente; documentar ou remover regras inalcançáveis.
- Esforço estimado: P

### [LAY-3] Sem zoom de UI (80-200 %) nem escala de fonte do sistema
- Severidade: ALTA
- Item da matriz: NF-009, NF-008 (PRD item 11 zoom 80-200 %; NFR-8)
- Local: app/ui/src (sem ocorrências); GraphView.tsx:150; styles.css:25,85
- Evidência: grep por zoom/font-scale em app/ui/src não acha nada. px fixos: `ctx.font = "13px Inter, Cantarell, system-ui, sans-serif"`, `.graph-rows [role="row"] { height: 28px }`, `grid-template-rows: 34px 44px`. A matriz NF-009 já diz "font scaling not handled". Se o WebKitGTK aplica a escala de texto do sistema a tamanhos em px: Não verificado.
- Por que importa: os requisitos de zoom e escala de fonte do PRD não são atendidos; usuários com baixa visão não ampliam a UI, e a altura fixa das linhas do canvas não acompanharia mesmo se o CSS acompanhasse.
- Sugestão de correção: adicionar zoom de UI persistido (zoom do webview ou `--ui-scale` na raiz guiando tamanhos, ROW_H e fonte do canvas) com Ctrl+= / Ctrl+- / Ctrl+0.
- Esforço estimado: G

### [LAY-4] Canvas do grafo não redesenha quando devicePixelRatio muda
- Severidade: MÉDIA
- Item da matriz: NF-002, NF-009
- Local: app/ui/src/features/graph/GraphView.tsx:122,399
- Evidência: `const dpr = window.devicePixelRatio || 1;` é lido só em `draw()`; o único gatilho de tamanho é `new ResizeObserver(() => setSize({ w: scroller.clientWidth, h: scroller.clientHeight }))`. Sem listener `matchMedia("(resolution: ...)")` (grep: matchMedia só em theme/theme.ts para esquema de cor). DPR fracionário em si é tratado: `Math.round(w * dpr)` mais `setTransform(dpr, ...)`.
- Por que importa: mover a janela entre monitores de escalas diferentes, ou mudar a escala, sem mudar o tamanho CSS pode deixar o canvas borrado até o próximo scroll ou resize. Não reproduzido (Não verificado).
- Sugestão de correção: ouvir uma query `matchMedia` de resolução rearmada e disparar o sinal de redesenho.
- Esforço estimado: P

### [LAY-5] Splitters: sem recolher, sem reset, sem tratamento de pointer cancel
- Severidade: MÉDIA
- Item da matriz: UI-003, NF-008
- Local: app/ui/src/features/split/Splitter.tsx:33-60; styles.css:361
- Evidência: setas de 16 px e Home/End para min/max existem, com `role="separator"` e aria-value*. Home vai para `min` (140/240 px), nunca 0; sem reset por duplo clique; o arrasto adiciona listeners em `window` só para `pointermove`/`pointerup` (sem `pointercancel`, sem pointer capture); `.splitter-x { display: none }` em <=620 px.
- Por que importa: painéis não podem ser ocultados para dar largura total ao grafo; um `pointerup` perdido (perda de foco) deixa a classe `resizing-x` no body e listeners presos. Recolher em si vai além de layout-direction.md:69.
- Sugestão de correção: adicionar pointer capture e `pointercancel`; duplo clique para restaurar o padrão; recolher opcional e persistido.
- Esforço estimado: M

### [LAY-6] Redimensionar colunas do grafo só por ponteiro; colunas ocultas sem controle
- Severidade: MÉDIA
- Item da matriz: F-14, F-23
- Local: app/ui/src/features/graph/GraphView.tsx:64-76,430-436; features/graph/columns.ts:17-27; docs/a11y/audit.md:35,62,71
- Evidência: `<span class="col-resize" ... onPointerDown={(e) => startResize(e, "refs")} />` sem tabindex/role/tecla. A auditoria marca F-14 e F-23 como adiados. Larguras persistem (`ak.graph.columns.v1`, limites 60-480/320/200); autor e depois data se ocultam quando a mensagem ficaria abaixo de `MIN_MESSAGE_W = 160`.
- Por que importa: usuários só de teclado não redimensionam (WCAG 2.1.1, adiamento conhecido); o texto do cabeçalho fica fora do grid (1.3.1); autor/data ocultos não podem ser restaurados em larguras estreitas.
- Sugestão de correção: tornar as alças `role="separator" tabindex="0"` reutilizando a lógica de teclado do Splitter; adicionar alternância de visibilidade de colunas.
- Esforço estimado: M

### [LAY-7] Máximo do splitter do diff é lido sem reatividade e difere do limite CSS
- Severidade: BAIXA
- Item da matriz: UI-003
- Local: app/ui/src/App.tsx:485; styles.css:343
- Evidência: `max={Math.max(160, innerHeight - 260)}` versus CSS `.center > .diff { max-height: calc(100% - 120px) }`.
- Por que importa: aria-valuemax fica desatualizado após redimensionar a janela e End pode definir um valor que o CSS corta.
- Sugestão de correção: derivar o máximo da altura de `.center` via ResizeObserver e compartilhar o limite com o CSS.
- Esforço estimado: P

### [LAY-8] Fonte do canvas e altura da barra divergem da direção de layout
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: app/ui/src/features/graph/GraphView.tsx:150; styles.css:392; REA layout-direction.md:55-57
- Evidência: o doc diz fonte Ubuntu Sans e barra superior de 48 px; o código desenha o texto do canvas com `Inter, Cantarell, system-ui` fixo e a linha final de `.app` é `56px`.
- Por que importa: o texto do grafo e o resto da UI podem divergir em fonte se o token CSS mudar; doc e código discordam.
- Sugestão de correção: ler a fonte do canvas de um token CSS; atualizar o doc ou o CSS.
- Esforço estimado: P

## O que já está bom
- O Splitter tem `role="separator"`, aria-value*, setas/Home/End e tamanhos persistidos com fallback em try/catch (Splitter.tsx).
- Nomes longos usam reticências (styles.css:36,66,136,347,505) e a toolbar rola (`overflow-x: auto`) com rótulos removidos em 1100/860 px.
- Colunas do grafo se adaptam à largura com mínimo de 160 px para a mensagem e larguras persistidas limitadas (columns.ts); o canvas trata DPR fracionário com arredondamento e `setTransform`.
- `prefers-reduced-motion`, `prefers-contrast`, `prefers-reduced-transparency` são respeitados.

## Não verificado
- Layouts em execução em 900x600, 1280x800, 1920x1080; sem screenshots.
- Escala de fonte do sistema e mudança de DPR no WebKitGTK; escala fracionária no Wayland.
- Se o painel central chega mesmo a 0 px (LAY-1) e o visual em 861-900 px (LAY-2): inferido só do CSS.
- Barra de status (UI-008) não localizada nesta área.
