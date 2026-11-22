# NoFap: módulo do ubiqX e produto separado

Pedido: um módulo "nofap" que trava sites pornográficos por DNS (com uma lista extensa), usa a IA
que o app já tem para olhar o que está na tela quando o site é desconhecido, aconselha a pessoa e
corta o acesso; dentro do ubiqX e também como um produto à parte, para quem só quer vencer a
pornografia.

É uma ferramenta de autocontrole que uma pessoa adulta instala no próprio computador: visível
(ícone na bandeja enquanto está ligada), com consentimento explícito para cada camada, sem modo
escondido, sem relatório para terceiros, sem conta e sem telemetria. Vem desligada até a pessoa
terminar o assistente.

## O que o módulo faz: cinco camadas

| Camada | O que faz |
|---|---|
| 1. Guarda do app | A cada 2 s compara o site da aba da frente com uma lista embutida de cerca de 240 mil domínios, regras de palavras e de domínio e a lista da própria pessoa, tudo no computador. Um acerto fecha a aba e abre um cartão de apoio. |
| 2. Arquivo `hosts` | Um bloco marcado com os ~20 mil sites adultos mais visitados (8 mil no Windows), mais os que a pessoa digitou. Removido byte a byte ao desligar. |
| 3. DNS filtrado (opcional) | DNS do sistema apontado para o Cloudflare for Families, com o operador nomeado na tela; "Restaurar minha rede" desfaz. |
| 4. Políticas dos navegadores (opcional) | DNS-over-HTTPS desligado e SafeSearch ligado, para as camadas 2 e 3 valerem dentro do navegador. Fechada nesta versão (ver tabela abaixo). |
| 5. Verificação por IA (opcional) | Quando as regras locais não decidem, o provedor de IA já configurado recebe uma foto reduzida só da janela da frente e responde uma palavra (explícito, sugestivo, nada, não sei). Bancos, saúde, governo, e-mail, mensageiros e gerenciadores de senha nunca são fotografados. |

O aconselhamento vem de um banco de mensagens revisado (PT-BR e EN), nunca de texto gerado por um
modelo: cartão de apoio, respiração, "sair da tela por 10 min", a frase de valores e os planos
"se-então" que a pessoa escreveu, check-in, e uma aba de ajuda com recursos de apoio. Afrouxar a
proteção espera um tempo que a pessoa escolheu (padrão 10 min) e pede nova confirmação. As
alterações no sistema passam por um ajudante elevado (o próprio executável com
`--shield-helper`), com um pedido de senha do sistema por mudança, sem serviço residente.

Detalhes: `docs/nofap/DESIGN.md` (desenho), `docs/nofap/handoff/INTEGRATOR.md` (decisões que o
alteram), `docs/nofap/PRIVACY.md` (o que é visto, enviado e guardado), `docs/nofap/REMOVAL.md`
(como remover tudo, inclusive à mão).

## Dois produtos

- **ubiqX**: página **NoFap** no menu lateral, com o mesmo serviço rodando dentro do motor.
- **NoFap separado** (`apps/nofap/`): um segundo app Tauri com o mesmo código (`ubiqx-nofap`,
  `NofapService`, a mesma interface), sem nada de produtividade (nenhum rastreador, classificador ou
  relatório existe nesse processo), com dados, chaves e atualizações próprios. No sistema ele usa o
  nome de máquina `ubiqx-shield` (identificador `ai.ubiqx.shield`, executável `ubiqx-shield`).

## O que está verificado, e como

- **Testes automáticos** em Linux, macOS e Windows no CI: Rust do workspace inteiro (os dois apps
  incluídos), interface (Vitest), scripts de release (`node --test`) e a lista de bloqueio
  (`build_blocklist.py --check`, reconstrução idêntica byte a byte). A camada do sistema é testada
  com sistemas falsos que executam os comandos de cada SO; a lista é testada contra nomes inocentes
  conhecidos ("disputas", "Pornichet", "aeromexico" e outros).
- **Execuções reais no CI** (workflow `nofap-smoke.yml`): em `ubuntu-latest`, `macos-latest` e
  `windows-latest`, o ajudante real aplica e remove `hosts` e DNS como administrador e o script
  confere tudo: o bloqueio vale para o sistema, o resolvedor responde e recusa, e depois o arquivo
  `hosts` volta byte a byte com o mesmo dono e permissões, e o DNS volta ao que era. Os três verdes
  desde a execução 6; o registro de cada execução, com o que deu errado e o que mudou, está em
  `docs/nofap/REAL-MACHINE-CHECKLIST.md`, seção 3.
- **Portão por sistema** (`ubiqx_nofap::gate::TIER_GATE`): uma camada de sistema só age num SO
  depois de ter evidência real. Hoje:

  | SO | `hosts` | DNS | Políticas dos navegadores |
  |---|---|---|---|
  | Linux | aberto | aberto | fechado |
  | macOS | aberto | aberto | fechado |
  | Windows | aberto | aberto | fechado |

  Com uma camada fechada, a guarda do app e a verificação por IA continuam funcionando.

- **Portão final deste PR** (rodado no último commit): formatação, clippy com `-D warnings`, os
  testes do workspace duas vezes, a verificação cruzada para macOS e Windows, os testes do
  `ubi-api`, a lista, typecheck, Vitest e os dois builds da interface, os testes dos scripts de
  release, e o `/etc/hosts` da máquina de build sem nenhuma linha `ubiqx-shield`.

## Revisão adversarial final

Seis frentes independentes (privilégios, privacidade, lógica, lista, experiência, entrega) leram o
PR inteiro; cada achado passou por três céticos e só ficou com dois votos. 41 achados, 33
confirmados, todos corrigidos com teste que falhava antes (dois críticos: a segunda foto de uma
verificação podia pegar outra página na mesma janela; "Apagar tudo" escondia camadas ainda
aplicadas no sistema). O que ficou de fora está nas seções "Final review" dos hand-offs U2, U4, U5
e U6.

## O que só uma pessoa numa máquina real pode verificar

`docs/nofap/REAL-MACHINE-CHECKLIST.md`, seções 4 e 5: os pedidos de senha de cada sistema, as
políticas dos navegadores, DoH no Chrome e no Firefox e o Private Relay do Safari, portal cativo,
a verificação por IA em provedores reais, o cartão que não rouba o teclado (macOS e Windows),
desinstalação com os dois produtos lado a lado, nomes instalados, Windows Home com a política de
execução padrão, e outros. Cada item diz o que conta como evidência.

## Decisões em aberto, suas

1. **O nome.** NOFAP é marca registrada nos EUA (NoFap LLC, que litiga), e "NoFap" é o que a pessoa
   vê; o pacote `.deb` se chama `no-fap`. Nenhum nome foi pesquisado no INPI nem no USPTO
   (checklist, item 17). Trocar o nome exibido é uma constante e dois campos de configuração
   (`DESIGN.md` 0.1); o nome de máquina `ubiqx-shield` não muda.
2. **Recursos de ajuda não verificados.** Telefones e endereços de `resources.json` não foram
   conferidos por uma pessoa; o teste de release falha até alguém preencher `verified_on` e
   `verified_by` (checklist, item 16).
3. **Termos não lidos.** Os termos dos provedores de IA sobre capturas possivelmente explícitas e
   os do Cloudflare for Families (atribuição incluída) ainda não foram lidos (itens 4 e 12).
4. **Publicação do produto separado.** O job `publish-shield` só roda na `main` com
   `vars.NOFAP_PUBLISH == 'true'`, que não existe ainda.
5. **Chave de licença de desenvolvimento.** A chave pública embutida em
   `crates/ubiqx-core/src/license.rs` é a do par de desenvolvimento; gere o par de produção e troque
   antes de vender (`docs/LICENSING.md`).

Também em aberto (checklist, seção 6): se sair do app deve esperar o tempo de espera com o módulo
ligado, e assinar os builds com um ajudante separado.

## Trabalho anterior neste PR

Os vídeos de lançamento do ubiqX em PT-BR (o filme e o corte 9:16 para Reels e TikTok), até o
commit `410c3d6`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01VfkL44GfUWLADtSCLmWRfs
