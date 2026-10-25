# Auditoria 03 — UI visual, acessibilidade e i18n

## Limitações
- Análise estática (código, CSS, JSON de temas, docs) mais cálculos numéricos de contraste. Nenhuma captura de tela foi feita e a UI não foi executada nesta passagem; ordem de foco, overflow e renderização não foram observados ao vivo.
- Razões de contraste calculadas com a mesma fórmula WCAG de `scripts/contrast.mjs` (script descartável no scratchpad, nada gravado no repositório).
- Leitor de tela (Orca) e comportamento do WebKitGTK não testados (a auditoria do projeto diz o mesmo).
- IDs da matriz vêm de `auditoria/00-matriz-documentacao-vs-codigo.md`.

## Findings

### [UIV-1] A escolha de tema não é persistida
- Severidade: MÉDIA
- Item da matriz: F-028
- Local: app/ui/src/theme/theme.ts:19-20,46-50; app/ui/src/App.tsx:350-352
- Evidência: `const [choice, setChoice] = createSignal<ThemeChoice>("system");` e `initTheme()` sempre chama `setTheme("system")`. Não há `localStorage` em theme.ts, enquanto o idioma persiste (`i18n.ts:391 STORAGE_KEY = "ak.lang"`).
- Por que importa: Quem escolhe Atlas Dark/Light num sistema configurado para o outro precisa reescolher a cada abertura; inconsistente com o seletor de idioma.
- Sugestão de correção: Gravar a escolha em `ak.theme` (try/catch como em i18n.ts) e lê-la em `initTheme()`.
- Esforço estimado: P

### [UIV-2] A paleta entregue não corresponde ao design-tokens.md
- Severidade: MÉDIA
- Item da matriz: F-028
- Local: themes/atlas-dark.theme.json, themes/atlas-light.theme.json vs Obsidian `03-design/design-tokens.md` §3–4
- Evidência: doc: canvas escuro `#0E1116`, surface `#151A21`, accent `#F2A93B` (âmbar), anel de foco `#6CB6FF`. Código: canvas `#17151F`, surface `#1D1A27`, accent `#A78BFA` (violeta), anel `#8DB8F2`; accent claro `#6D3FD1` vs doc `#9A5B00`. Cores de lane, status e diff coincidem. Os conjuntos de chaves dos dois temas são idênticos (paridade OK).
- Por que importa: A identidade especificada ("neutros slate + Atlas Amber") não é a que é entregue; quem consome docs/screenshots é induzido a erro e lane.0 âmbar deixou de ser igual ao accent.
- Sugestão de correção: Atualizar o design-tokens.md (registrando a decisão) ou realinhar os temas; rodar contrast.mjs de novo.
- Esforço estimado: P

### [UIV-3] Sem temas de alto contraste nem suporte a forced-colors
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: themes/ (2 arquivos); app/ui/src/styles.css:448; contrast.mjs:35
- Evidência: design-tokens.md §8 define os tipos `high-contrast-dark|light` (texto 7:1); `ls themes` mostra apenas `atlas-dark` e `atlas-light`. O CSS tem `prefers-contrast: more` que só troca `--lg-edge`; `grep forced-colors styles.css` não acha nada. O ramo 7:1 (`hc`) do script nunca é exercitado.
- Por que importa: Usuários com baixa visão no Linux/GNOME em alto contraste não têm suporte; o caminho 7:1 testado é código morto.
- Sugestão de correção: Adicionar dois temas de alto contraste e um bloco `forced-colors`, ou documentar o descope.
- Esforço estimado: M

### [UIV-4] Lacunas de contraste não cobertas pelo contrast.mjs
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: scripts/contrast.mjs:6-12; styles.css:93,134,261 (linhas selecionadas), letras de status `.st-*`
- Evidência: razões calculadas fora de PAIRS. Atlas Light, cor de status sobre `bg.selected`: added 3,85, modified 4,25, deleted 4,23, renamed 4,39 (todas < 4,5); as letras `.st-A/M/D/R` são desenhadas em `.files button.active` (bg-selected). Atlas Dark: `text.muted` sobre `bg.selected` 3,86 e sobre `bg.overlay` 4,22 (< 4,5). lane.1 claro sobre `bg.selected` 3,55 (≥3, OK para gráficos). O anel de foco está bem: escuro 8,83/6,48/7,09 (canvas/selected/overlay), claro 4,97/4,34/5,67.
- Por que importa: A auditoria declara 1.4.3 "pass", mas o script só testa fundos surface/canvas; a correção do SHA (F-24) tratou uma única ocorrência.
- Sugestão de correção: Incluir fundos selected e overlay em PAIRS; escurecer as cores de status do tema claro ou usar texto secundário nas linhas selecionadas.
- Esforço estimado: P

### [UIV-5] Bordas de inputs usam border.default (1,3:1)
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: styles.css:42,48 (`input`, `select`), themes `color.border.default`
- Evidência: `border.default` sobre `bg.surface`: escuro `#332E40`/`#1D1A27` = 1,31, claro 1,37. O script só verifica `border.strong` ≥3. Os campos ficam em `bg.raised` sobre a surface.
- Por que importa: WCAG 1.4.11 exige 3:1 para o limite que identifica um controle; a linha 1.4.11 da auditoria diz "Pass" sem considerar isso.
- Sugestão de correção: Usar `border-strong` em inputs/selects (como `.seg` já faz) ou um preenchimento que atinja 3:1.
- Esforço estimado: P

### [UIV-6] Fontes diferem dos tokens e nenhuma é empacotada
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: styles.css:5-6 e 201-202; app/ui/public (só favicon.png, mock)
- Evidência: o primeiro `:root` declara Inter/JetBrains Mono (design-tokens §6 "bundled Inter"); a sobrescrita posterior usa `"Ubuntu Sans", Ubuntu, Cantarell`. `grep ttf|woff` em app/ui não encontra arquivos de fonte.
- Por que importa: A renderização depende de fontes do sistema; larguras de glifo (`clipText` no canvas do grafo) variam entre máquinas; a primeira declaração é morta.
- Sugestão de correção: Escolher uma pilha, apagar a declaração morta, empacotar ou documentar a dependência.
- Esforço estimado: P

### [UIV-7] Erro cru exibido no grafo; inglês fixo no overlay de performance
- Severidade: MÉDIA
- Item da matriz: UI-006, NF-010
- Local: app/ui/src/features/graph/GraphView.tsx:99,517,520-523
- Evidência: `setLoadError(String(e?.message ?? e));` renderizado como `<div class="graph-error" role="alert">{loadError()}</div>` (ignora `errorText()`/`err.<CODE>`). O overlay (Ctrl+Shift+P) tem literais `first paint`, `draw`, `painted frames/s`, `commits · lanes`, e `rowCount.toLocaleString()` sem locale. No resto, datas/números usam `activeLocale()` corretamente.
- Por que importa: Um chunk que falha mostra texto do motor sem tradução; UI-006 promete mensagens humanas com códigos estáveis.
- Sugestão de correção: Passar `e` por `errorText()`; internacionalizar (ou isentar explicitamente) o overlay de desenvolvimento.
- Esforço estimado: P

### [UIV-8] Lacunas de teclado adiadas: redimensionar colunas do grafo, lista WIP
- Severidade: MÉDIA
- Item da matriz: NF-008, UI-007
- Local: docs/a11y/audit.md F-14, F-23; features/wip/WipPanel.tsx
- Evidência: auditoria: "F-14 … Column resize handles are pointer-only … deferred", "F-23 … Header labels … text outside the grid … deferred"; matriz UI-007 (do HANDOFF): sem navegação por teclado no WIP. Os splitters têm teclado (auditoria §4).
- Por que importa: 2.1.1 aparece como "Pass after fixes" na tabela-resumo, mas F-14 é violação conhecida; F-15 (ordem Tab toolbar→sidebar→grafo→diff→detalhes) está como pass embora difira de algumas ordens de leitura.
- Sugestão de correção: Redimensionar com setas nas alças do cabeçalho (reusar a lógica do Splitter); foco itinerante nas linhas do WIP; semântica de cabeçalho após testes com Orca.
- Esforço estimado: M

### [UIV-9] Acessibilidade verificada só no Chromium; o alvo é WebKitGTK
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: docs/a11y/audit.md §1, §7
- Evidência: "Not done here: a real screen reader (Orca) on the packaged app, and WebKitGTK-specific behaviour (the dev mock runs in Chromium)". A auditoria mira WCAG 2.1 AA enquanto o PRD NFR-7 / cabeçalho do contrast.mjs citam 2.2.
- Por que importa: Mapeamento de nome/papel e `aria-activedescendant` numa grade sobre canvas são as partes mais arriscadas e não foram confirmadas no motor real.
- Sugestão de correção: Fazer uma passada com Orca no AppImage; incluir critérios 2.2 (2.4.11 foco não obscurecido, 2.5.8 tamanho do alvo — controles têm `--ctl-h: 28px`, botões de ícone 24px em styles.css:30).
- Esforço estimado: M

### [UIV-10] Tamanhos fixos em px: sem suporte a escala de fonte
- Severidade: BAIXA
- Item da matriz: NF-009
- Local: styles.css:7,202 (`font-size: 13px`), vários `font-size: 11/12px`; altura da linha do canvas fixa
- Evidência: `:root { font-size: 13px; }` e unidades px em todo o CSS; a matriz NF-009 registra "font scaling not handled".
- Por que importa: A escala de texto do GNOME / zoom ignora ou quebra linhas de canvas de altura fixa.
- Sugestão de correção: Usar rem para texto e derivar a altura da linha do grafo do tamanho raiz.
- Esforço estimado: M

### [UIV-11] Rótulos pt-BR até 3,5x mais longos que em inglês
- Severidade: BAIXA
- Item da matriz: NF-010
- Local: app/ui/src/locales/pt-BR.ts
- Evidência: paridade de chaves: 374 en / 374 pt-BR, nenhuma faltando ou sobrando. 62 chaves são >1,4x mais longas, p.ex. `rebase.reword` 6→28, `wip.unstage` 7→21, `ss.unshare.submit` 7→21, `ss.tabs` 12→29, `wip.unstageAll` 11→26. A toolbar tem `overflow-x: auto` (styles.css:34); `.tool` é `white-space: nowrap`.
- Por que importa: Botões e abas em sidebar/WIP estreitos podem cortar ou forçar rolagem horizontal; não testado visualmente.
- Sugestão de correção: Capturar pt-BR a 1024 px para toolbar, WIP, abas de shared stash e editor de rebase; usar `title`/reticências onde necessário.
- Esforço estimado: P

## O que já está bom
- Paridade de chaves en/pt-BR completa (374/374), garantida por `satisfies` + `i18n.test.ts`; sem literais em `aria-label=`, `title=`, `placeholder=` nos TSX (grep limpo); datas/números usam `activeLocale()`; `html lang` acompanha o idioma.
- Componentes usam tokens `--ak-*`; as únicas cores literais no CSS são sombras/scrims pretos (`rgb(0 0 0 / .x)`) e nenhuma no TSX; o canvas do grafo lê as cores do tema.
- `node scripts/contrast.mjs`: atlas-dark, atlas-light, 0 falhas; lanes ≥ 3:1 sobre o canvas; anel de foco ≥ 4,3:1 em todas as superfícies.
- Boa base de a11y: anéis `:focus-visible` visíveis, trap de Tab em modais, menus em Menu/Shift+F10, `prefers-reduced-motion` e `prefers-reduced-transparency` respeitados, ícones lucide `aria-hidden`.

## Não verificado
- Renderização ao vivo, overflow em pt-BR, ordem de foco no app em execução; Orca; WebKitGTK; se `text.muted` é de fato usado em linhas selecionadas além do SHA do grafo; conteúdo de docs/screenshots/phase5; docs/specs/accessibility.md além do resumo da auditoria.
