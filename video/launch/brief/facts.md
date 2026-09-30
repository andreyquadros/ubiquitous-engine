# ubiqX AI — folha de fatos para o vídeo de lançamento

> **O que é isto.** A referência de verdade para todo texto que aparece na tela do vídeo de lançamento.
> Se uma frase não cabe numa linha deste documento, ela não entra no vídeo.
> *EN: product truth sheet. Every on-screen claim must trace back to an entry here. PT-BR first, short English notes where useful.*
>
> Verificado em 2026-09-29 contra o código e a documentação do repositório (branch `claude/sharp-bardeen-sk27ae`).
> Fontes lidas: `README.md`, `docs/LICENSING.md`, `docs/ARCHITECTURE.md`, `docs/WINDOWS-LINUX.md` §4,
> `site/public/llms.txt`, `site/src/i18n.ts` + `site/src/config.ts` (copy da landing),
> `apps/desktop/src/i18n/messages/*.ts` (textos PT-BR da interface real), `apps/desktop/src/pages/*.tsx`,
> e o código de apoio em `crates/ubiqx-core`, `crates/ubiqx-engine` e `services/ubi-api` para confirmar números.
> Referências no formato `arquivo:linha` (linhas do estado atual do repositório).

---

## 1. Em uma frase, e o problema

**One-liner (PT-BR):**
> **O ubiqX AI registra sozinho em que apps, janelas e sites você trabalha, classifica cada bloco de tempo com IA nas categorias que você cria e entrega o relatório diário e mensal por categoria pronto.**

Versão curta para a tela: **"Controle de tempo automático com IA."** (`site/src/i18n.ts:115`, título da página)

**O problema que ele resolve:**
- Você passa o dia no computador e, no fim, não sabe dizer para onde foram as horas: trocas de janela, abas abertas, o "só um minuto" que virou quarenta (`site/src/i18n.ts:140-143`).
- Quem toca várias frentes (clientes, projetos, instituições) precisa **prestar contas do tempo**, e isso vira um segundo trabalho: cronômetro que esquece de apertar, planilha preenchida de memória na sexta (`site/src/i18n.ts:188-189`, `site/public/llms.txt:64-70`).
- Painéis de produtividade comuns mostram gráficos; eles não entregam **o relatório por categoria** que você precisa enviar (`site/public/llms.txt:24-25`).

*EN: automatic, AI-classified time tracking into the user's own categories, ending in ready-to-hand-in reports. Local-first desktop app with a mascot (UBI) that watches your focus.*

---

## 2. O loop central (uma linha cada)

| Etapa | O que acontece | Fonte |
|---|---|---|
| **Captura** | A cada 5 s (padrão) anota app, título da janela e, no navegador, o site ativo; agrupa em blocos de contexto. Sem teclas, sem conteúdo. | `README.md:32-33`, `crates/ubiqx-core/src/model.rs:980` |
| **Classifica** | Regras → memória das correções → IA (texto, em lote) → IA (visão, só em blocos ambíguos), nas categorias que você escreveu. | `README.md:26-30`, `crates/ubiqx-engine/src/classify.rs:70`, `:333`, `:356`, `:462` |
| **Revisão** | Só entra na fila o que a cadeia não resolveu; você decide com as teclas 1–9, e cada decisão ensina o classificador. | `apps/desktop/src/pages/Review.tsx:386-413`, `README.md:47-50` |
| **Relatório** | No horário de cada categoria, um relatório diário com atividades, minutos e evidências, que vira o mensal em Markdown. | `docs/ARCHITECTURE.md` §5 "Relatórios", `README.md:51-52` |

Linha narrativa sugerida (voz da marca, não é cópia do trailer): **Captura → Classifica → Revisa → Entrega.**

---

## 3. Recursos (16)

Formato de cada entrada: **id** · manchete (≤ 5 palavras) · subtítulo (≤ 12 palavras) · o que faz de verdade · evidência · onde aparece no app.
Nomes de tela = rotas reais: `dashboard` (Hoje), `timeline`, `review` (Revisão), `reports` (Relatórios), `categories`, `insights`, `settings` (Configurações), `focus` (Foco), `onboarding`.

### 3.1 `registro-automatico`
- **Manchete:** Ele registra. Você trabalha.
- **Subtítulo:** App, janela e site ativos, a cada 5 segundos. Sem play.
- **O que faz:** Um amostrador em segundo plano lê o app em foco, o título da janela e, em navegadores, a URL/domínio da aba ativa (5 s por padrão, configurável); o segmentador junta amostras iguais em **blocos de contexto** e fecha o bloco na troca de contexto, na ociosidade (3 min sem teclado/mouse, padrão) ou em suspensão. Não lê teclas nem conteúdo de página.
- **Evidência:** `README.md:16-20`, `README.md:32-33`; `docs/ARCHITECTURE.md` §5 "Sampler" e "Segmenter"; `crates/ubiqx-core/src/model.rs:980-982` (5 s, 180 s, bloco mínimo 20 s); `apps/desktop/src/i18n/messages/onboarding.ts:46`.
- **Tela:** `timeline`: lista "Blocos do dia" (app, título, "{início} até {fim}", chip de categoria, duração; bloco atual com "em andamento"). `dashboard`: card "Linha do tempo" (faixa de 24 h).

### 3.2 `prints-janela-ativa`
- **Manchete:** Prints só da janela ativa.
- **Subtítulo:** Esparsos, depois de 20 s no mesmo contexto. Apagados após o uso.
- **O que faz:** Os prints são disparados pelo bloco, não por um timer cego: o primeiro só depois de **20 s no mesmo bloco**, só da **janela em foco**, **nunca** enquanto um app bloqueado estiver visível, repetidos a cada 2 min (padrão) enquanto o bloco continua. Reduzidos e em JPEG; apagados logo após a classificação, a menos que "Manter screenshots para revisão" esteja ligado. Política escolhida pelo usuário: *Nunca* / *Apenas nestes apps* / *Todos, exceto bloqueados*; teto de imagens por hora.
- **Evidência:** `crates/ubiqx-engine/src/screenshots.rs:12-13` (`FIRST_CAPTURE_AFTER_SECS = 20`), `:37-39`, `:60-69` (app bloqueado visível → não captura); `crates/ubiqx-engine/src/classify.rs:462-495` (visão só para ambíguos; apaga após uso); `crates/ubiqx-core/src/model.rs:983-987`; `docs/ARCHITECTURE.md` §5 "Prints"; `apps/desktop/src/i18n/messages/settings.ts:128-152`.
- **Tela:** `settings` → Privacidade, card "Análise visual: screenshots para a IA" (três opções). `onboarding` passo 6 "Análise visual". `review` → detalhes do grupo (miniatura do print ou "Sem captura de tela para este bloco").

### 3.3 `categorias-suas`
- **Manchete:** As suas categorias.
- **Subtítulo:** Você descreve cada frente. A IA segue a sua descrição.
- **O que faz:** O usuário cria as categorias (cliente, projeto, instituição…) com nome, **descrição obrigatória** ("O que conta como trabalho desta categoria"), palavras-chave, cor, ícone, horário do relatório e "Conta como trabalho". A IA lê a descrição antes de classificar cada bloco. Categorias do sistema: Sem categoria, Distração, Pausa, Privado.
- **Evidência:** `apps/desktop/src/i18n/messages/categories.ts:8`, `:41-54`; `apps/desktop/src/i18n/messages/onboarding.ts:75`; `apps/desktop/src/i18n/messages/common.ts:90-97`; `site/public/llms.txt:22-23`.
- **Tela:** `categories`: coluna "Suas categorias" (demo: IFRO, Incubadora, Cidades Inteligentes) e o formulário com o campo "O que conta como trabalho desta categoria". `onboarding` passo 4 "Suas categorias de trabalho".
- **Nota:** a versão longa "As suas categorias, não as dele" (6 palavras) vem da landing (`site/src/i18n.ts:196`) e funciona como cartela em dois tempos.

### 3.4 `classificacao-em-cadeia`
- **Manchete:** Regras, memória, IA.
- **Subtítulo:** O que é grátis resolve primeiro. A IA só vê o resto.
- **O que faz:** Cadeia de classificadores: regras do usuário → memória de correções (similaridade ≥ 0,85 com blocos já corrigidos) → regras aprendidas → modelo de **texto** em lote (dados já mascarados) → **visão** só para blocos ambíguos que têm print, dentro da política e do teto por hora. Um bloco que falha 5 vezes vai para a Revisão. Resultado típico mostrado no app: "X pelas regras, Y pela IA, Z por visão e N para revisar".
- **Evidência:** `crates/ubiqx-engine/src/classify.rs:1-2`, `:39` (`MAX_ATTEMPTS = 5`), `:70` (cadeia local), `:333`, `:356`, `:462`; `crates/ubiqx-core/src/learning.rs:21` (0,85); `README.md:34-37`; `docs/ARCHITECTURE.md` §5 "ClassifyWorker".
- **Tela:** `review`: botão "Classificar agora" → toast "Classificação concluída — {local} pelas regras, {remote} pela IA, {vision} por visão e {needs_review} para revisar" (`review.ts:101-102`). `timeline`: selo de origem de cada bloco ("Classificado por: regra / memória / IA / visão / você", `ui.ts:26`, `common.ts:125-129`).

### 3.5 `revisao-so-duvidas`
- **Manchete:** Revisão só das dúvidas.
- **Subtítulo:** Teclas 1 a 9 decidem. O resto o Ubi já resolveu.
- **O que faz:** A fila de Revisão agrupa por app/domínio e mostra **só o que a cadeia não conseguiu resolver**; o que o Ubi classificou com confiança fica na lista de baixo. Atalhos: ↑/↓ (ou j/k) movem, Enter abre os detalhes, **1–9 atribuem a n-ésima categoria**. A decisão vale para todos os blocos do grupo e ensina o classificador.
- **Evidência:** `apps/desktop/src/pages/Review.tsx:386-413` (regex `^[1-9]$`), `:597-607` (legenda das teclas); `apps/desktop/src/i18n/messages/review.ts:36-37`; `README.md:47-48`; `docs/ARCHITECTURE.md` §6.
- **Tela:** `review`: lista "Grupos para revisão" (linha "Precisa de revisão", "Confiança: …") + card à direita "Atribuir ao grupo selecionado" com as teclas `↑ ↓ mover · Enter detalhes · 1–9 atribuir`. Estado vazio: UBI "Nada em dúvida!".

### 3.6 `confirmar-vira-memoria`
- **Manchete:** Um clique vira memória.
- **Subtítulo:** Confirme o que o Ubi decidiu. Amanhã ele resolve sozinho.
- **O que faz:** Na seção "Classificados neste dia (N)", o botão **"Confirmar os N"** transforma de uma vez as respostas do classificador em respostas suas. Isso alimenta a memória, que resolve o mesmo contexto **de graça** nos dias seguintes (sem chamar a IA). Toast: "N blocos viraram memória: o Ubi resolve sozinho da próxima vez."
- **Evidência:** `apps/desktop/src/pages/Review.tsx:362-384`, `:556-563`; `crates/ubiqx-engine/src/learning.rs:45-59` (`confirm_groups`); `apps/desktop/src/i18n/messages/review.ts:76-80`, `:98-99`; `README.md:47-49`.
- **Tela:** `review` → seção "Classificados neste dia (N)" aberta → botão "Confirmar os N" → toast "N grupos confirmados".

### 3.7 `aprende-com-correcoes`
- **Manchete:** Corrigiu? Ele aprende.
- **Subtítulo:** Cada correção vira memória e sugere a regra "Sempre".
- **O que faz:** Uma correção (1) reclassifica o bloco, (2) grava a correção (memória + exemplos para a IA), (3) penaliza a regra que errou — ela se desativa sozinha após 2 erros, (4) pode ajustar blocos semelhantes do dia/mês, (5) quando **2 correções concordam** sugere uma regra "Sempre: sei.ifro.edu.br → IFRO". Regras de domínio específico podem ser aplicadas sozinhas; em domínios compartilhados (Gmail, WhatsApp, Docs…) ele pede confirmação.
- **Evidência:** `crates/ubiqx-engine/src/learning.rs:98-167`; `crates/ubiqx-core/src/learning.rs:1-11`, `:99-106`, `:178-179`; `crates/ubiqx-core/src/model.rs:319-322` (auto-desativação); `README.md:48-50`; `docs/ARCHITECTURE.md` §5 "Aprendizado".
- **Tela:** `review`/`timeline`: chip "Sempre: `domínio` → Categoria" (`components/ui/BlockBits.tsx:84`) e toast "Regra criada". `categories`: card "Regras" com origem "Aprendida" / "Sua" e coluna "Acertos e erros".

### 3.8 `relatorio-diario`
- **Manchete:** O relatório sai pronto.
- **Subtítulo:** Por categoria, com atividades, minutos e evidências. Na hora marcada.
- **O que faz:** No horário de cada categoria (padrão 18:00, configurável) sai um relatório diário por categoria: itens no passado com tipo (reunião, desenvolvimento, ensino…), minutos, evidências, horário e "continua de…", mais destaques. Editável; editado à mão nunca é sobrescrito; fica "Desatualizado" se os blocos mudarem e pode ser regenerado. Escrito pela IA com o seu perfil e a descrição da categoria; sem IA, um template local agrupa por app/domínio.
- **Evidência:** `docs/ARCHITECTURE.md` §5 "Relatórios"; `crates/ubiqx-core/src/model.rs:1008` (18:00); `apps/desktop/src/i18n/messages/reports.ts:9-38`; `apps/desktop/src/pages/Reports.tsx:263-315`; `apps/desktop/src/i18n/messages/common.ts:131-139` (tipos).
- **Tela:** `reports` → aba "Diário": card do relatório com chips "Destaques", tabela Atividade / Tipo / Minutos / Horário, botões "Copiar Markdown" e "Regenerar", linha "Gera automaticamente às {hora}".

### 3.9 `relatorio-mensal`
- **Manchete:** O mês inteiro, num Markdown.
- **Subtítulo:** Visão mensal por categoria, que junta o que continuou. Baixe o .md.
- **O que faz:** Consolida os relatórios diários de uma categoria num único relatório mensal, agrupando continuações ("continuou X — 3 dias, 7 h 20"), renderizado localmente; "Copiar Markdown" ou "Baixar .md" (`relatorio-{categoria}-{mês}.md`). Feito para virar o relatório de atividades entregue a um cliente ou instituição.
- **Evidência:** `README.md:51-52`; `apps/desktop/src/i18n/messages/reports.ts:50-61`; `apps/desktop/src/pages/Reports.tsx:410-433`; `site/public/llms.txt:33-35`.
- **Tela:** `reports` → aba "Mensal": seletor de categoria e mês, botão "Gerar relatório mensal", "Relatório mensal — {mês}, consolidado a partir dos relatórios diários", botões "Copiar Markdown" / "Baixar .md".

### 3.10 `score-de-foco-ubi`
- **Manchete:** Seu foco em um número.
- **Subtítulo:** Score de 0 a 100. O UBI muda de humor junto.
- **O que faz:** Score de foco 0–100 calculado localmente: fatia do tempo em categorias produtivas (tempo sem categoria conta metade), menos uma penalidade por trocas de contexto acima de 12/h. O humor do UBI segue o score: **Empolgado** (≥ 85), **Focado** (65–84), **Tranquilo** (45–64), **Preocupado** (< 45), **Descansando** (sem atividade). O UBI mostra uma dica por humor e dá avisos (distração, pausa após 90 min, elogio) com teto diário, intervalo mínimo, horário silencioso e silêncio em apps de reunião.
- **Evidência:** `crates/ubiqx-core/src/insights.rs:114-142`, `:157-165`; `apps/desktop/src/i18n/messages/ubi.ts:10-14`; `apps/desktop/src/i18n/messages/common.ts:113-117`; `apps/desktop/src/i18n/messages/settings.ts:170-194`; `README.md:53`.
- **Tela:** `dashboard` (Hoje) → bloco "Resumo do dia": mostrador de score com a legenda "de foco", números Tempo produtivo / Distrações / Maior foco contínuo / Trocas por hora, e o UBI 3D com balão de fala. `insights` → gráfico "Score de foco — De 0 a 100, por dia".

### 3.11 `modo-foco`
- **Manchete:** Foco que se defende.
- **Subtítulo:** Bloqueie apps e sites. O UBI fecha na hora.
- **O que faz:** Página Foco: lista de **Bloqueios** (apps e sites) que o guarda fecha quando aparecem na frente (fecha o app, fecha ou esvazia a aba, ou só avisa); **sessões de foco** de 5 a 240 min com a tarefa escrita ("Focar"), que podem esconder/minimizar as outras janelas, segurar também o que cai em Distração e rodar um atalho (macOS) ou comando (Windows/Linux) ao começar e ao encerrar; registro de "Intervenções recentes"; três avisos antes de desativar um bloqueio usado nos últimos 15 min; convite "Me ajude a focar" quando há 10 trocas de app em 15 min.
- **Evidência:** `crates/ubiqx-engine/src/focus.rs:1-3`, `:298-309`; `crates/ubiqx-core/src/focus.rs:13-29`, `:59-65`; `apps/desktop/src/i18n/messages/focus.ts:7-137`; `README.md:83-84`.
- **Tela:** `focus`: card "Sessão de foco" ("Que tarefa você precisa fazer agora?", Duração, botão "Focar"; em andamento: "Restam", "N distrações seguradas"), card "Bloqueios" (demo: YouTube, Instagram, Discord), "Intervenções recentes". Janela "Aviso do UBI" com "Ok, foco!" e frases como "Eu seguro a distração; você segura o foco."

### 3.12 `privacidade`
- **Manchete:** Privado por padrão.
- **Subtítulo:** Dados mascarados antes da IA. Você vê tudo o que saiu.
- **O que faz:** Antes de qualquer chamada à IA: URL sem query string (só o domínio vai), e-mails, telefones, CPF/CNPJ e sequências longas de dígitos mascarados, títulos de apps de mensagens reduzidos ao nome do app. A tela **"Dados enviados à IA"** mostra exatamente o que saiu, por dia e por bloco. Apps bloqueados (gerenciadores de senha por padrão), domínios bloqueados, janelas anônimas e **Modo privado** (30 min / 1 hora / Até amanhã / Até eu desligar) viram blocos "[privado]": o tempo conta, o conteúdo não, e nada disso vai à IA. Banco SQLite local, chave no cofre do sistema, sem conta, sem telemetria. Opções "Somente local", "Exportar meus dados" e "Apagar todos os dados".
- **Evidência:** `crates/ubiqx-core/src/redact.rs:1-6`; `README.md:44-46`; `docs/ARCHITECTURE.md` §7; `apps/desktop/src/i18n/messages/settings.ts:105-125`, `:154-160`, `:243-247`; `apps/desktop/src/i18n/messages/common.ts:96-97`; `crates/ubiqx-core/src/model.rs:989-996` (apps bloqueados padrão).
- **Tela:** `settings` → Privacidade: card "O que sai da sua máquina", botão "Ver o que foi enviado à IA hoje" → diálogo "Dados enviados à IA — Exatamente o que saiu da sua máquina em {data}"; chips do "Modo privado". `timeline`: selo "enviado à IA". `review` → detalhes: "Dados enviados à IA".

### 3.13 `escolha-sua-ia`
- **Manchete:** Você escolhe a IA.
- **Subtítulo:** Claude, OpenAI ou Grok, com a sua chave. Troque quando quiser.
- **O que faz:** Três provedores com chave própria: **Anthropic Claude, OpenAI e xAI Grok**. A chave fica no cofre do sistema (Keychain no macOS, Gerenciador de Credenciais no Windows, Secret Service no Linux), é validada ao salvar, e a troca de provedor acontece em Configurações → IA sem reiniciar; as chaves dos outros ficam guardadas. Modelos editáveis ("Listar modelos da conta"). Só o provedor selecionado recebe dados.
- **Evidência:** `README.md:38-40`, `:141-149`; `docs/ARCHITECTURE.md` §1 (O6b), §7; `apps/desktop/src/lib/providers.ts:9-16`; `apps/desktop/src/i18n/messages/settings.ts:32-80`.
- **Tela:** `settings` → IA: "Provedor de IA" (Claude / OpenAI / Grok / IA do Ubi), "Chave configurada", "Modelos da {provedor}", "Orçamento mensal". `onboarding` passo 2 "Escolha sua IA".

### 3.14 `ia-do-ubi`
- **Manchete:** Ou deixe com o Ubi.
- **Subtítulo:** IA do Ubi no plano mensal: sem chave de API.
- **O que faz:** No plano mensal (R$ 49/mês) não há chave de provedor: a chave de licença (chega por e-mail, começa com `UBIQX-`) libera o provedor **"IA do Ubi"**, cujas chamadas passam pelo servidor do Ubi, que escolhe os modelos (um rápido para classificar e ver imagens, um forte para relatórios), mede o gasto do mês com um limite incluso e mostra o uso em Configurações.
- **Evidência:** `docs/LICENSING.md:9-12`, §3; `README.md:41-43`; `apps/desktop/src/i18n/messages/settings.ts:292-340`; `apps/desktop/src/i18n/messages/onboarding.ts:137-154`; `services/ubi-api/src/config.rs:17`.
- **Tela:** `onboarding` passo 2: dois cartões "Deixar o Ubi cuidar da IA — R$ 49/mês" e "Usar minha própria chave — R$ 197/ano ou 10x de R$ 25". `settings` → Licença: "Uso da IA do Ubi neste mês — {gasto} de {limite}".

### 3.15 `teto-de-gasto`
- **Manchete:** Custo de IA sob controle.
- **Subtítulo:** Orçamento mensal: aviso aos 80%, pausa aos 100%.
- **O que faz:** Você define o **orçamento mensal de IA** (US$ 5 por padrão). Aos 80 % o UBI avisa; aos 100 % a IA pausa até o mês seguinte e a classificação local (regras + memória) continua. Regras e memória resolvem a maioria dos blocos de graça; a IA recebe lotes; imagens têm teto por hora. Estimativas com 8 h/dia: US$ 3–6/mês (Claude), US$ 1–3 (OpenAI), US$ 0,50–2 (Grok).
- **Evidência:** `crates/ubiqx-core/src/model.rs:942-943`, `:1003`; `crates/ubiqx-engine/src/classify.rs:98-104`, `:138`, `:161`; `apps/desktop/src/i18n/messages/settings.ts:82-88`; `apps/desktop/src/i18n/messages/ubi.ts:23-25`; `README.md:34-37`, `:141-145`; `docs/ARCHITECTURE.md` §5 "Saúde da IA / custo".
- **Tela:** `settings` → IA: campo "Orçamento mensal — US$ por mês" com a dica "Ao atingir, a IA pausa até o próximo mês; a classificação local continua." `dashboard` → card do UBI: "Uso da IA no mês — de {orçamento}".

### 3.16 `multiplataforma-bilingue`
- **Manchete:** macOS, Windows e Linux.
- **Subtítulo:** O mesmo app, em português e inglês. Atualiza sozinho.
- **O que faz:** macOS 13+ (Apple Silicon, `.dmg`), Windows 10/11 64 bits (`.exe`/`.msi`), Linux 64 bits em sessão X11 (`.AppImage`/`.deb`). Fica na barra de menus (macOS) ou na bandeja do sistema. Interface em português (padrão) e inglês, trocada na hora em Configurações → Geral. Atualização no app: verifica ao abrir e a cada 6 h; "Atualizar agora" baixa, instala e reabre.
- **Evidência:** `README.md:73-87`, `:125-133`; `site/src/i18n.ts:290-292`; `site/public/llms.txt:11`, `:39-41`; `crates/ubiqx-core/src/lang.rs:12-18`; `apps/desktop/src/i18n/messages/settings.ts:27-28`, `:267-268`; `apps/desktop/src/i18n/messages/updates.ts:22`.
- **Tela:** `settings` → Geral (Idioma) e Atualizações; ícone na barra de menus/bandeja.

### Telas de apoio (B-roll, sem entrada própria)
- **`timeline`** — reclassificar um bloco em um clique ("Reclassificar este bloco"), "Dividir bloco", "Adicionar atividade manual" (para reuniões presenciais, leitura no papel), filtro "Somente não classificados". Fonte: `apps/desktop/src/i18n/messages/timeline.ts:19-79`; `docs/ARCHITECTURE.md` §6.
- **`insights`** — "Últimos 7 dias": Tempo produtivo, Score médio, Maior foco contínuo, Trocas por hora; gráficos "Horas por categoria" e "Score de foco"; "Recomendações do UBI" (geradas sob demanda, custam uma chamada ao modelo de relatórios); "O que o UBI já disse". Fonte: `apps/desktop/src/i18n/messages/insights.ts:7-45`.
- **`onboarding`** — 7 passos: Como funciona · Escolha sua IA · Permissões · Categorias · Horários · Análise visual · Concluir; abre com "Oi, eu sou o UBI." Fonte: `apps/desktop/src/i18n/messages/onboarding.ts:17-23`, `:43`.

> **Dados de demonstração.** As capturas da interface vêm do modo navegador com dados simulados (`apps/desktop/src/lib/mock.ts`: categorias IFRO, Incubadora, Cidades Inteligentes; bloqueios YouTube, Instagram, Discord). Qualquer número que aparecer nessas telas (horas, score, contagens) é **demo**: não o apresente como resultado médio de usuários.

---

## 4. Números e fatos seguros (com fonte)

Todos conferidos na fonte indicada. Use exatamente estas grafias.

| Fato | Valor para a tela | Fonte | Observação |
|---|---|---|---|
| Plano mensal | **R$ 49/mês** — "Mensal, com a IA do Ubi" | `docs/LICENSING.md:12`; `site/src/i18n.ts:267-268`; `settings.ts:303`; `onboarding.ts:140` | Inclui a IA, sem chave de API. |
| Plano anual | **R$ 197/ano ou 10x de R$ 25** — "Anual, com a sua IA" | `docs/LICENSING.md:11`; `site/src/i18n.ts:251-253`; `settings.ts:302`; `onboarding.ts:143` | Você paga a IA direto ao provedor. Escreva sempre a frase inteira (ver §5.10). |
| Linha de planos | "Dois jeitos de pagar. O mesmo app." | `site/src/i18n.ts:246` | Nos dois planos o app é o mesmo e os dados ficam na máquina (`site/src/i18n.ts:282`). |
| Provedores com chave própria | **3** — Anthropic Claude, OpenAI, xAI Grok | `README.md:38-40`; `apps/desktop/src/lib/providers.ts:12` | + a opção gerenciada "IA do Ubi" = 4 escolhas no seletor. |
| Sistemas | **3** — macOS · Windows · Linux | `README.md:75-85` | macOS 13+ Apple Silicon; Windows 10/11 64 bits; Linux 64 bits X11. |
| Idiomas | **2** — português (padrão) e inglês | `site/public/llms.txt:11`; `crates/ubiqx-core/src/lang.rs:12-18` | |
| Amostragem | a cada **5 s** (padrão) | `crates/ubiqx-core/src/model.rs:980`; `README.md:27` | Configurável. "A cada poucos segundos" é a versão sem número. |
| Primeiro print | só depois de **20 s** no mesmo contexto | `crates/ubiqx-engine/src/screenshots.rs:13`, `:37` | Só da janela ativa; nunca com app bloqueado visível (`:60-69`). |
| Atalhos da Revisão | teclas **1–9** | `apps/desktop/src/pages/Review.tsx:406-407`, `:606` | + ↑/↓ e Enter. |
| Confirmar em lote | **1 clique** ("Confirmar os N") | `apps/desktop/src/pages/Review.tsx:556-563` | |
| Score de foco | **0 a 100** | `crates/ubiqx-core/src/insights.rs:114`; `insights.ts:26-27` | |
| Humores do UBI | **5** — Descansando, Tranquilo, Focado, Empolgado, Preocupado | `apps/desktop/src/i18n/messages/common.ts:113-117`; `insights.rs:132-142` | |
| Sessão de foco | **5 a 240 min** | `crates/ubiqx-core/src/focus.rs:16-17`; `focus.ts:29` | |
| Orçamento de IA (sua chave) | padrão **US$ 5/mês**; aviso a **80 %**, pausa a **100 %** | `crates/ubiqx-core/src/model.rs:1003`; `crates/ubiqx-engine/src/classify.rs:138`, `:161`; `docs/ARCHITECTURE.md` §5 | Valor editável pelo usuário. |
| Custo estimado da IA (8 h/dia) | Claude **US$ 3–6/mês**, OpenAI **US$ 1–3**, Grok **US$ 0,50–2** | `README.md:141-145`; `settings.ts:44-51` | São **estimativas** — escreva "estimativa". A landing resume como "centavos por dia" (`site/src/i18n.ts:259`). |
| Limite da IA do Ubi | limite mensal incluso (padrão do servidor US$ 6 por assinante) | `docs/LICENSING.md:12`; `services/ubi-api/src/config.rs:17` | Configurável pelo operador: na tela, prefira "limite mensal incluso", sem o número. |
| Relatório diário | no horário de cada categoria; padrão **18:00** | `crates/ubiqx-core/src/model.rs:1008`; `categories.ts:39-40` | |
| Exportação de relatórios | **Markdown** (Copiar Markdown / Baixar .md) | `reports.ts:29`, `:55`; `README.md:51-52` | "Exportar meus dados" gera JSON (`docs/ARCHITECTURE.md` §7). |
| Regra sugerida | após **2** correções concordantes | `crates/ubiqx-core/src/learning.rs:99-100` | Frase: "Sempre: sei.ifro.edu.br → IFRO" (`README.md:50`). |
| Regra que erra | desativa sozinha após **2** erros | `crates/ubiqx-core/src/model.rs:319-322` | |
| Tentativas antes da Revisão | **5** | `crates/ubiqx-engine/src/classify.rs:39` | Técnico; não precisa ir à tela. |
| Modo privado | 30 min · 1 hora · Até amanhã · Até eu desligar | `apps/desktop/src/i18n/messages/settings.ts:117-120`; `docs/ARCHITECTURE.md` §7 | |
| Pausa sugerida | após **90 min** sem pausa | `crates/ubiqx-core/src/insights.rs:162`; `settings.ts:180` | |
| Convite ao foco | **10** trocas de app em **15 min** | `crates/ubiqx-core/src/focus.rs:27-28` | |
| Avisos do UBI | até **4** por dia, intervalo de **60 min** (padrão) | `crates/ubiqx-core/src/model.rs:856-857` | Editáveis. |
| Atualizações | verifica ao abrir e a cada **6 h** | `README.md:129`; `settings.ts:268` | |
| Onboarding | **7** passos | `onboarding.ts:17-23` | |
| Código aberto | licença **MIT** | `LICENSE:1`; `README.md:189`; `site/src/i18n.ts:386` | |
| Trailer existente | 58 s, pt-BR | `site/public/llms.txt:87` | |

---

## 5. Afirmações proibidas (e o que dizer no lugar)

1. **Números de usuários, avaliações, depoimentos, logos de clientes, "usado por equipes da…", prêmios, imprensa.** Não existe nada disso na fonte. Não invente contadores ("+10 mil pessoas") nem estrelas.
2. **"100 % offline" / "funciona sem internet" sem ressalva.** Sem rede, ele continua registrando e classifica por regras e memória; a classificação por IA, a visão, os relatórios escritos por IA e as recomendações precisam de rede (`site/public/llms.txt:76-78`, `docs/ARCHITECTURE.md` §5). O "100 % offline" de `docs/LICENSING.md:47` é só a verificação da chave de licença: não reaproveite. Diga: *"Sem internet, ele continua registrando."*
3. **"Sem IA na nuvem", "IA local", "IA no dispositivo", "roda um modelo no seu computador".** Falso: a IA é a API da Anthropic, OpenAI ou xAI, ou o servidor do Ubi (`README.md:38-43`, `:102`). Local são as regras, a memória, o score e os relatórios por template.
4. **"Nada sai do seu computador" / "seus dados nunca saem".** Saem, mascarados: nome do app, título da janela, domínio e, se a política permitir, um print reduzido da janela ativa (`settings.ts:109-110`). Diga: *"Seus dados ficam na sua máquina. Para a IA vai só o mínimo, mascarado, e você vê tudo o que saiu."*
5. **"Nunca tira prints" ou, no outro extremo, "grava a sua tela" / "lê o que você digita".** Ele tira prints esparsos (só janela ativa, só após 20 s) e não lê teclas nem conteúdo de página (`README.md:32-33`). Não mostre keylogger, gravação de vídeo nem leitura de texto digitado.
6. **"Nunca captura a tela inteira".** Quando a plataforma não informa o id da janela, a captura mira a tela onde a janela está (`crates/ubiqx-engine/src/screenshots.rs:71-77`). Diga só *"prints da janela ativa"*.
7. **Linux com Wayland.** Só sessão X11; em Wayland apenas apps sob XWayland são vistos, os nativos ficam invisíveis (`docs/WINDOWS-LINUX.md:110-120`). No Linux a URL vem só do título da janela. Não mostre GNOME/KDE em Wayland como suportado; escreva "Linux (X11)" onde houver detalhe.
8. **Mac Intel / versões antigas.** macOS só 13+ e **Apple Silicon**; Windows 10/11 **64 bits** (`site/src/i18n.ts:290-291`).
9. **"Criptografado", "criptografia de ponta a ponta", "cofre criptografado para os seus dados".** O banco é SQLite local sem criptografia; só as chaves (API e licença) ficam no cofre do sistema. Diga *"banco local"* e *"chave no cofre do sistema"*.
10. **Parcelamento.** Nunca "10x sem juros", nunca "R$ 197 em 10x", nunca "a partir de R$ 16/mês". 10 × R$ 25 = R$ 250, diferente de R$ 197. Escreva sempre, literalmente: **"R$ 197/ano ou 10x de R$ 25"**.
11. **"Grátis", "teste grátis", "trial", "desconto de lançamento", "preço promocional".** Não há trial nem desconto (`docs/LICENSING.md:14-15`). A tolerância atual sem licença (modo *soft*) é detalhe interno, não é oferta.
12. **"Assine agora" como CTA principal / checkout ativo.** Os links de pagamento ainda estão vazios e a landing mostra "Em breve — Pagamento em configuração" (`site/src/config.ts:5-6`, `site/src/i18n.ts:280-281`). O CTA do vídeo é **baixar**: *"Baixe em ubiqx.com.br"*. Preço pode aparecer como informação.
13. **"Nunca erra", "100 % de precisão", "classifica tudo sozinho", "me corrige uma vez e eu nunca mais erro".** Blocos incertos vão para a Revisão, a memória exige semelhança alta, regras podem ser contrariadas. A frase do balão da landing (`site/src/i18n.ts:153`) promete demais: não use. Diga *"Corrigiu? Ele aprende."*
14. **Ganhos quantificados**: "economize 10 h por semana", "2x mais produtivo", "reduz distrações em 40 %". Não há dado que sustente.
15. **Integrações e recursos inexistentes**: Slack/Jira/Notion/Google Agenda/Toggl, app de celular, versão web, sincronização entre dispositivos, conta na nuvem, painel de equipe, gestor vendo o time, faturamento/horas cobráveis, cronômetro manual. É um app de desktop de um usuário só, local-first. (Não posicione como monitoramento de funcionários.)
16. **Exportar em PDF, DOCX, Excel ou CSV.** Relatórios saem em Markdown; dados em JSON. DOCX está no roadmap, não no produto (`README.md:183-185`).
17. **"Assinado e notarizado pela Apple", "verificado pela Microsoft".** O macOS vem com assinatura ad hoc (Gatekeeper pode reclamar) e o instalador do Windows não tem assinatura de editor (SmartScreen) (`site/src/i18n.ts:303-313`, `README.md:132-133`).
18. **Nomes de versão de modelo na tela** ("GPT-5", "Claude Sonnet 5", "Grok 4.1"). São padrões editáveis que mudam; na IA do Ubi quem escolhe é o servidor. Use os nomes dos provedores: **Claude, OpenAI, Grok**. Não sugira parceria ou endosso da Anthropic, OpenAI ou xAI (nada de "em parceria com", "oficial", "powered by" com logos).
19. **Concorrentes pelo nome** (Rize, RescueTime, Toggl). A comparação existe no FAQ, mas no vídeo vira alegação comparativa sem prova. Não cite.
20. **Números dos prints na tela** (1024 px, 48 h, "6 imagens por hora"). O texto do onboarding (`onboarding.ts:48`, `:106`) e os dados de demo (`mock.ts:531-542`) divergem dos padrões do motor (1280 px, 24 h, 20 por hora, `model.rs:984-985`, `:1002`). Fale só "esparsos" e "reduzidos".
21. **"IA do Ubi ilimitada".** Tem limite mensal por assinante; ao atingir, a IA pausa até o mês seguinte (`settings.ts:322`).
22. **"Relatórios escritos por IA mesmo sem chave".** Sem IA há relatórios por template (`README.md:151`); no modo "Somente local" relatórios e recomendações por IA ficam indisponíveis (`settings.ts:90`).
23. **"Um assistente que conversa com você" / "chat com o UBI".** O UBI mostra dicas e avisos e, sob demanda, recomendações semanais. Não há chat.
24. **"Bloqueia sites em qualquer navegador".** Leitura/fechamento de aba depende do navegador e do sistema (macOS: Safari, Chrome, Arc, Brave, Edge, Vivaldi, Opera; Windows: Chrome, Edge, Brave, Opera, Vivaldi, Firefox; Linux: só pelo título) (`README.md:79`, `:83`).
25. **"Conta o tempo longe do computador".** Só por entrada manual ("Adicionar atividade manual").
26. **"Nunca se conecta à internet" / "zero rede".** Além da IA, o app consulta o feed de atualizações (ao abrir e a cada 6 h, desligável) e, no plano mensal, o servidor do Ubi. "Sem telemetria" continua verdadeiro (nenhum SDK de telemetria no código).

---

## 6. Copy de marca reutilizável

**Nome e grafia**
- Produto: **ubiqX AI**. No logotipo: "ubiqX" com o **X em azul volt** (#4d8dff). No texto corrido o app se chama "o ubiqX".
- Mascote: **UBI** (maiúsculas quando é o personagem: "o UBI avisa"). O serviço gerenciado é **"IA do Ubi"** e a interface diz "o Ubi decidiu" (grafia exata da UI, `review.ts:77`, `settings.ts:333`).

**Tagline:** **Retome o controle do seu dia.** (end card do trailer; variante da landing: "Descubra para onde vão as suas horas. E retome o controle do dia.", `site/src/i18n.ts:141`)

**CTA:** **Baixe em ubiqx.com.br** · variantes por sistema: "Baixar para macOS" / "Baixar para Windows" / "Baixar para Linux" (`site/src/i18n.ts:138`)

**Domínio:** **ubiqx.com.br** (`site/public/llms.txt:8`)

**Linha de plataformas:** **macOS · Windows · Linux**

**Frases aprovadas da landing (podem ir para a tela como estão)**
- "Os dias não estão menores. O seu tempo é que está vazando." (`site/src/i18n.ts:140`)
- "Trabalhe o dia inteiro. O relatório se escreve sozinho." (`site/src/i18n.ts:118`)
- "Três passos. Nenhum deles é seu." (`site/src/i18n.ts:169`)
- "Menos tempo prestando contas do tempo." (`site/src/i18n.ts:188`)
- "Dois jeitos de pagar. O mesmo app." (`site/src/i18n.ts:246`)
- "Controle de tempo automático com IA. O seu dia, contado sem você anotar nada." (`site/src/i18n.ts:381`)
- "Sem contas, sem telemetria." (`site/src/i18n.ts:146`)

**Trailer cinematográfico (referência de voz, não copiar cena a cena):** "Você senta às 8. Levanta às 18." · "Os dias não estão menores. O seu tempo está vazando." · "Registra sozinho — Cada app, cada janela, cada site. Sem cronômetro. Sem planilha." · "Classifica com IA — O seu dia, nas suas categorias. Quanto foi foco. Quanto foi distração." · "Aprende e entrega — Corrija com uma tecla. O relatório já sai pronto." · "Chega de procrastinar no escuro. Retome o controle do seu dia."

**Banco de frases novas na mesma voz (todas cobertas por §3)**
- "Você trabalha. Ele anota." (3.1)
- "O seu dia já está sendo contado." (3.1)
- "Só pergunta o que não sabe." (3.5)
- "Uma tecla. E ele aprende." (3.5 / 3.7)
- "Confirmou? Amanhã ele resolve sozinho." (3.6)
- "Às 18h, o relatório já está pronto." (3.8 — 18h é o padrão; alternativa neutra: "Na hora marcada, o relatório está pronto.")
- "Do dia para o mês, em um Markdown." (3.9)
- "Eu seguro a distração; você segura o foco." (fala real do UBI, `crates/ubiqx-core/src/focus.rs:64`)
- "Seus dados ficam com você. A IA vê só o mínimo." (3.12)
- "Claude, OpenAI ou Grok. Ou deixe com o Ubi." (3.13 / 3.14)

**Falas reais do UBI (seguras, existem no app)**
- "Oi, eu sou o UBI." (`onboarding.ts:43`)
- "Você está no ritmo. Eu cuido do registro — segue o jogo." (`ubi.ts:12`)
- "Muitas trocas de contexto hoje. Vamos fechar uma coisa de cada vez?" (`ubi.ts:14`)
- "Nada em dúvida!" (`review.ts:83`) · "Fila vazia. Bom trabalho!" (`review.ts:18`)
- "Não! Foque na sua produtividade." (`focus.ts:128`)

---

## 7. Glossário da interface (PT-BR, grafia exata)

Use estas palavras para o vídeo bater com o app real. Onde houver diferença entre a landing e a UI, vale a UI.

**Navegação (menu lateral)** — `nav.ts:7-14`
Hoje · Timeline · Revisão · Relatórios · Categorias · Insights · Foco · Configurações
(A página chama-se **"Timeline"**, em inglês; "Linha do tempo" é o título do card no Hoje.)

**Hoje (dashboard)** — `dashboard.ts:7-40`, `charts.ts:12`
Resumo do dia · Score de foco (legenda do mostrador: "de foco") · Tempo produtivo · Distrações · Maior foco contínuo · Trocas por hora · Linha do tempo · Tempo por categoria · Foco por hora · Apps mais usados · "Precisa de revisão: N blocos" · "Tudo classificado" · "{duração} de foco hoje"

**Timeline** — `timeline.ts:8-79`, `ui.ts:26-32`
Blocos do dia · em andamento · Reclassificar este bloco · Dividir bloco · Adicionar atividade manual · Somente não classificados · Modo privado · Ocioso · manual · precisa de revisão · enviado à IA · Classificado por: {origem} · Criar regra · **Sempre:** `domínio` → Categoria

**Revisão** — `review.ts:6-106`
**Classificar agora** · Grupos para revisão · Precisa de revisão · Confiança · Atribuir ao grupo selecionado · mover / detalhes / atribuir · Classificados neste dia (N) · **Confirmar os N** · "N blocos viraram memória: o Ubi resolve sozinho da próxima vez." · Nada em dúvida! · Dados enviados à IA · Classificação concluída

**Relatórios** — `reports.ts:8-62`
Diário · Mensal · Destaques · Atividade / Tipo / Minutos / Horário · **Copiar Markdown** · Regenerar · Gerar agora · Gerar relatório mensal · Relatório mensal · **Baixar .md** · Desatualizado · Editado · "Gera automaticamente às {hora}"
Tipos de atividade (`common.ts:131-139`): Desenvolvimento · Reunião · Comunicação · Documentação · Ensino · Pesquisa · Extensão · Gestão · Outro

**Categorias** — `categories.ts:7-107`, `common.ts:90-97`
Suas categorias · Do sistema · Arquivadas · Nova categoria · "O que conta como trabalho desta categoria" · Palavras-chave · Horário do relatório · Conta como trabalho · Regras · Aprendida / Sua · Acertos e erros · Domínio · App · Título contém · Regex
Do sistema: Sem categoria · Distração · Pausa · Privado
Origem da classificação (`common.ts:125-129`): regra · memória · IA · visão · você

**Insights** — `insights.ts:7-45`
Últimos 7 dias · Tempo produtivo · Score médio · Maior foco contínuo · Trocas por hora · Horas por categoria · Score de foco ("De 0 a 100, por dia") · Recomendações do UBI · O que o UBI já disse

**Foco** — `focus.ts:8-137`
Sessão de foco · "Que tarefa você precisa fazer agora?" · Duração · **Focar** · Encerrar · Restam · "N distrações seguradas" · Bloqueios · Intervenções recentes · Opções · app fechado / aba fechada · Aviso do UBI · **Ok, foco!** · Me ajude a focar · Espera um pouco

**Configurações** — `settings.ts:8-342`
Seções: Geral · IA · Rastreamento · Privacidade · Relatórios · UBI e notificações · Atualizações · Permissões do macOS · Sobre · Licença
Itens: Provedor de IA · Chave configurada · Validar e salvar · Listar modelos da conta · Orçamento mensal · Somente local · Rastreamento ativo · Iniciar com o sistema · O que sai da sua máquina · **Dados enviados à IA** · **Modo privado** · Análise visual · Apps bloqueados · Domínios bloqueados · Manter screenshots para revisão · Quem é você · Avisos do UBI · Horário silencioso · Apps silenciosos · Exportar meus dados · Apagar todos os dados · **IA do Ubi** · Uso da IA do Ubi neste mês · Atualizar agora

**UBI e estados** — `common.ts:113-123`, `ubi.ts:16-23`
Humores: Descansando · Tranquilo · Focado · Empolgado · Preocupado
IA: IA ativa · IA não configurada · IA pausada · IA instável · Uso da IA no mês
Rastreador: Rastreando · Pausado · Privado · Ocioso

**Onboarding** — `onboarding.ts:17-23`
Como funciona · Escolha sua IA · Permissões · Categorias · Horários · Análise visual · Concluir

**Onde o app mora:** "barra de menus" (macOS) / "bandeja do sistema" (Windows e Linux) (`onboarding.ts:117`, `:124`)

**Evite no vídeo termos que o app não usa:** "pontuação" (use **Score de foco**), "tarefas/projetos" como entidade (o app usa **categorias** e **blocos**), "cronômetro" como recurso (o app não tem), "dashboard" em PT (use **Hoje**), "painel de controle".
