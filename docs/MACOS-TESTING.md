# Testando o ubiqX no seu Mac

O ubiqX é um app de barra de menus. Para o primeiro teste local siga esta ordem — ela evita
os dois problemas clássicos de apps de rastreamento em desenvolvimento: **permissões que
somem a cada rebuild** e **títulos de janela vazios**.

## 1. Pré-requisitos

```bash
xcode-select --install                 # ferramentas de linha de comando
curl https://sh.rustup.rs -sSf | sh    # Rust estável (>= 1.85)
brew install pnpm                      # ou: npm i -g pnpm
```

## 2. Identidade de assinatura estável (uma vez)

O macOS liga as permissões (Gravação de Tela, Automação) e os itens do Keychain à assinatura do
binário. Um binário sem assinatura estável muda de identidade a cada compilação e o sistema
"esquece" tudo. Crie um certificado local:

1. Abra **Acesso às Chaves** (Keychain Access) → menu **Assistente de Certificado** →
   **Criar um Certificado…**
2. Nome: `ubiqX Dev` · Tipo de identidade: *Auto-assinado* · Tipo de certificado: **Assinatura de código**.
3. Conclua. (Se preferir, use seu certificado *Apple Development* e exporte
   `UBIQX_SIGN_IDENTITY="Apple Development: Seu Nome (TEAMID)"`.)

## 3. Build e assinatura

```bash
git clone https://github.com/andreyquadros/ubiquitous-engine ubiqx && cd ubiqx
scripts/install-ubi-model.sh                       # opcional: arte do UBI (ubi.png e/ou Ubi.glb em ~/Downloads)
cd apps/desktop && pnpm install
pnpm tauri build                                   # gera target/release/bundle/macos/ubiqX.app
cd ../.. && scripts/codesign-dev.sh                # assina com "ubiqX Dev"
open target/release/bundle/macos/ubiqX.app
```

Teste **sempre pelo `.app` empacotado**. `pnpm tauri dev` funciona para iterar na interface, mas
o binário de desenvolvimento não carrega o `Info.plist` (sem `LSUIElement`, sem descrição de
uso de Apple Events) e as notificações não aparecem.

## 3.1 Sem compilar no Mac: GitHub Actions ou Codemagic

Se não quiser instalar Rust e Xcode, baixe o `.app` pronto. Os dois caminhos geram o mesmo bundle; a
diferença é só onde ele é compilado.

**GitHub Actions (já configurado).** Cada push compila o app num runner macOS. Em *Actions → CI → o run do
seu branch → Artifacts*, baixe `ubiqX-macos-app`: é um zip com `ubiqX.app.zip` dentro (o zip interno é
gerado com `ditto`, que preserva permissões e symlinks). Repositório público: minutos ilimitados.

**Codemagic.** O `codemagic.yaml` na raiz define o workflow *ubiqX macOS app* (Mac mini M2, dentro dos
500 minutos gratuitos por mês; um build leva ~10–15 min sem cache e bem menos com cache).

1. Entre em https://codemagic.io com a conta do GitHub, *Add application* → este repositório → *codemagic.yaml*.
2. *Start new build* → workflow *ubiqX macOS app* → escolha o branch (`main` ou `claude/...`).
3. No fim, em *Artifacts*, baixe `ubiquitous-engine_<n>_artifacts.zip`: dentro vêm o `ubiqX.app` e o
   `ubiqX_<versão>_aarch64.dmg`. Um build leva ~13 min na primeira vez (o cache do `target` é enviado ao final).
   Pushes em `main` e `claude/*` também disparam builds sozinhos (ajuste `triggering` no yaml para mudar isso).

**Abrir um app baixado.** O bundle sai com assinatura ad hoc e o macOS põe downloads em quarentena, então
aparece "está danificado" ou "desenvolvedor não identificado". Remova a quarentena e assine com a sua
identidade local do passo 2 (assim as permissões sobrevivem a novos downloads). Prefira o `.dmg` quando
houver: ele preserva permissões e symlinks do bundle.

```bash
cd ~/Downloads && ditto -x -k ubiquitous-engine_*_artifacts.zip ubiqx-build && cd ubiqx-build
open *.dmg                                   # arraste o ubiqX para Aplicativos e ejete a imagem
xattr -dr com.apple.quarantine /Applications/ubiqX.app
codesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app   # = scripts/codesign-dev.sh /Applications/ubiqX.app
open /Applications/ubiqX.app
```

Sem `.dmg` (build do GitHub Actions, ou DMG que falhou), use o `ubiqX.app` do zip: `ditto -x -k` no zip,
`xattr -dr com.apple.quarantine ubiqX.app`, assine e abra. Se mesmo assim o macOS disser "danificado", o zip
perdeu o bit de execução: `chmod +x ubiqX.app/Contents/MacOS/ubiqX` e assine de novo.

Depois siga o passo 4 normalmente. Para o app já sair assinado da nuvem, exporte o certificado `ubiqX Dev`
como `.p12` (Acesso às Chaves → Meus Certificados → botão direito → Exportar) e cadastre no Codemagic um grupo
de variáveis `ubiqx_apple` com `APPLE_CERTIFICATE` (`base64 -i ubiqx-dev.p12 | pbcopy`),
`APPLE_CERTIFICATE_PASSWORD` e `APPLE_SIGNING_IDENTITY=ubiqX Dev`, descomentando `groups` no yaml. Com um
certificado *Developer ID* (Apple Developer Program) mais `APPLE_ID`, `APPLE_PASSWORD` e `APPLE_TEAM_ID` o
Tauri também notariza, e o app abre em qualquer Mac sem os comandos acima.

**Arte do UBI.** Os builds na nuvem só incluem o mascote se os arquivos estiverem no repositório: o modelo 3D
`apps/desktop/public/ubi/Ubi.glb` (o UBI principal) e/ou a ilustração `apps/desktop/public/ubi/ubi.png`. Envie-os
pelo GitHub (*Add file → Upload files* dentro da pasta, nomes exatamente `Ubi.glb` e `ubi.png`) ou rode
`scripts/install-ubi-model.sh` e faça commit dos dois arquivos — o `Ubi.glb` deve ser commitado como o PNG.

## 3.2 Atualizações: o ubiqX se atualiza sozinho

Cada commit que a nuvem compila vira uma atualização para quem já usa o ubiqX, e a instalação agora é
automática: aparece uma barra de progresso, o ubiqX baixa a nova versão, instala e se reabre sozinho. Não é
preciso baixar nada à mão. O caminho manual (o `.dmg`) continua ali como reserva, para quando a instalação
automática falhar ou você preferir fazer você mesmo.

**Como funciona.** A CI publica cada build na release rolante **`continuous`** do repositório
(`https://github.com/andreyquadros/ubiquitous-engine/releases/tag/continuous`), sempre com os mesmos nomes de
arquivo, e escreve ali **dois feeds**:

- **`latest.json`** — o aviso. Traz versão, data do commit (`build.epoch`), número do build, sha, as notas do
  commit e uma entrada por plataforma em `platforms` (`darwin-aarch64`, `windows-x86_64`, `linux-x86_64`, cada
  uma com `url`, `kind`, `size` e, no macOS, `app_zip_url`) apontando para os instaladores
  `ubiqX-macos-aarch64.dmg`, `ubiqX-macos-aarch64.app.zip`, `ubiqX-windows-x86_64-setup.exe`,
  `ubiqX-windows-x86_64.msi`, `ubiqX-linux-x86_64.AppImage` e `ubiqX-linux-x86_64.deb`. É o que o app lê 45 s
  depois de abrir e a cada 6 h para dizer "tem build novo" no banner, no menu da barra de menus e na
  notificação. Esse feed não mudou.
- **`updater.json`** — a instalação. É o formato do `tauri-plugin-updater`
  (`{version, notes, pub_date, platforms: {"darwin-aarch64": {signature, url}, …}}`) e aponta para os
  **artefatos de atualização**: `ubiqX-macos-aarch64.app.tar.gz` (macOS), `ubiqX-windows-x86_64-setup.nsis.zip`
  (Windows) e `ubiqX-linux-x86_64.AppImage.tar.gz` (Linux). Cada um vem com a assinatura `.sig` ao lado, e o
  ubiqX só instala um pacote cuja assinatura confere com a chave pública gravada em
  `apps/desktop/src-tauri/tauri.conf.json` (`plugins > updater > pubkey`).

**Versão por build.** O updater compara versões (semver), então a versão precisa crescer a cada build. A CI
grava `0.1.<número do build>` em `tauri.conf.json` antes de compilar (`scripts/app-version.mjs`, chamado por
`scripts/ci-app-version.sh`); o número do build é a contagem de commits, a mesma que aparece em
*Configurações → Atualizações → Número*. Assim o número que você vê no app, o número dos dois feeds e o número
que o updater compara são sempre o mesmo. Data, sha e branch continuam vindo do `scripts/build-info.sh`.
Compilando na sua máquina, sem esse passo, a versão continua a base (`0.1.0`).

**Onde aparece.** Um banner no topo do app com a data e o sha do build e os botões **Como instalar**,
**Depois**, **Baixar (.dmg)** e **Atualizar agora**; a seção **Atualizações** em Configurações (build atual,
última verificação, **Verificar agora**, interruptor da verificação automática e, quando há build novo, as
notas do commit com **Atualizar agora**, **Baixar (.dmg)** e **Página do release**); uma notificação do macOS
uma vez por build; e um item no menu da barra de menus. **Depois** esconde o banner só daquele build.

**Atualizar com um clique.** Clique em **Atualizar agora** (no banner ou em Configurações). No macOS aparece
antes um aviso curto — leia o parágrafo seguinte — com **Atualizar mesmo assim** e **Cancelar**. Depois disso
a barra mostra quantos MB de quantos já vieram, o pacote é instalado e o ubiqX fecha e abre de novo na versão
nova. Se algo der errado, a mensagem do erro aparece ali mesmo, com **Tentar de novo** e o **Baixar (.dmg)**
do lado para o caminho manual.

**O preço no macOS: você vai reconceder as permissões.** Isto não está resolvido, e o app avisa antes de
instalar. O macOS amarra Gravação de Tela, Automação e os itens do Keychain à **assinatura de código** do
bundle. A CI assina o `ubiqX.app` *ad hoc* (`APPLE_SIGNING_IDENTITY="-"`), ou seja, sem certificado: a
identidade do app é o hash do próprio binário e muda a cada build. Quando o updater troca o bundle, o sistema
vê um app diferente do que tinha autorizado, então **Gravação de Tela e Automação voltam a ser pedidas** e o
Keychain pode perguntar de novo pela chave de IA. Não há como contornar isso pelo lado do app: é o
funcionamento do TCC (o banco de permissões do macOS).

**O que resolveria.** Uma **identidade de assinatura estável** usada pela própria CI, de modo que todo build
saia com o mesmo certificado e, para o macOS, continue sendo o mesmo app:

1. **Certificado Developer ID Application** (Apple Developer Program, US$ 99/ano). É o caminho completo:
   exporte o certificado como `.p12`, cadastre no GitHub Actions os secrets `APPLE_CERTIFICATE` (o `.p12` em
   base64), `APPLE_CERTIFICATE_PASSWORD` e `APPLE_SIGNING_IDENTITY`, e o Tauri assina no lugar do `-` ad hoc.
   Com `APPLE_ID`, `APPLE_PASSWORD` e `APPLE_TEAM_ID` ele ainda notariza, e aí o `.dmg` também abre em
   qualquer Mac sem o `xattr`. Este é o único caminho que a Apple documenta e suporta.
2. **Certificado autoassinado `ubiqX Dev` exportado para a CI** (grátis). Em tese resolve o mesmo problema —
   o requisito de designação passa a apontar para o certificado, não para o hash do binário — mas é um
   comportamento que a Apple não documenta, não ajuda com o Gatekeeper (o `.dmg` baixado continua pedindo
   `xattr -dr com.apple.quarantine`) e **não foi testado aqui**. Se quiser tentar, é o mesmo grupo de
   variáveis do item 1, com o `.p12` do certificado que você criou no § 2.

Enquanto nenhum dos dois estiver configurado, a atualização automática funciona e é rápida, mas custa
reconceder as permissões em *Ajustes do Sistema → Privacidade e Segurança*. Se você não quiser pagar esse
preço a cada build, continue no caminho manual do § 3.1, que reassina com a sua identidade local `ubiqX Dev`.
No Windows e no Linux nada disso se aplica: a atualização automática não custa nada.

**Segredos da CI (quem mantém o repositório precisa cadastrar).** Em *Settings → Secrets and variables →
Actions → New repository secret*:

| Secret | O que é |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | conteúdo do arquivo de chave privada gerado por `pnpm tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | a senha dessa chave |

A metade pública dessa chave já está em `apps/desktop/src-tauri/tauri.conf.json`. Os três jobs de build
(`macos-app`, `windows-app`, `linux-app`) começam com a etapa **Updater signing key**: sem os dois secrets ela
para o job com a mensagem dizendo exatamente o que falta, em vez de gerar um pacote sem assinatura que o app
recusaria depois. A chave privada nunca entra no repositório. Para trocar a chave, gere outro par
(`pnpm tauri signer generate -w ubiqx-updater.key`), troque o `pubkey` no `tauri.conf.json` e atualize os dois
secrets — quem estiver numa versão antiga terá de atualizar à mão uma última vez.

**GitHub Actions (automático).** Todo push compila os três sistemas (jobs `macos-app`, `windows-app` e
`linux-app`) e o job `publish-continuous` publica os instaladores, os artefatos de atualização assinados, o
`latest.json` e o `updater.json` de uma vez, com o `GITHUB_TOKEN` que o próprio Actions fornece. Pull requests
só compilam. Um push novo cancela o build anterior do mesmo branch, e a publicação recusa sobrescrever um
build mais novo do que o dela. Quando o `latest.json` já publicado é do **mesmo** commit (Codemagic e Actions
a publicar o mesmo push), as plataformas são mescladas nos dois feeds: cada publicador troca só as suas.

**Codemagic (opcional, precisa de tokens).** O Codemagic não recebe token do GitHub sozinho, e agora também
precisa da chave de assinatura para compilar. Para ele publicar:

1. No GitHub: *Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new
   token*. Repositório: só `ubiquitous-engine`. Permissão: **Contents: Read and write**. Copie o token.
2. No Codemagic: *Teams → Global variables and secrets*: variável `GITHUB_TOKEN` no grupo **`ubiqx_github`**,
   e as variáveis `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` no grupo
   **`ubiqx_updater`**; marque todas como *Secure*.
3. No `codemagic.yaml`, descomente as linhas dos grupos (`ubiqx_github`, `ubiqx_updater` e, se tiver,
   `ubiqx_apple`). Faça commit.

Sem o grupo `ubiqx_github` a etapa *Publicar atualização (GitHub Releases)* escreve "Publicação pulada:
GITHUB_TOKEN não definido" e o build continua em Artifacts. Sem o `ubiqx_updater` o build **falha** na etapa
*Versão deste build e chave de assinatura*, de propósito.

**Instalação manual (reserva).** O botão **Baixar (.dmg)** abre o instalador no navegador. O bundle continua
com assinatura ad hoc e em quarentena, então repita a sequência do § 3.1:

```bash
open ~/Downloads/ubiqX-macos-aarch64.dmg          # arraste o ubiqX para Aplicativos (substitua) e ejete
xattr -dr com.apple.quarantine /Applications/ubiqX.app
codesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app   # = scripts/codesign-dev.sh /Applications/ubiqX.app
open /Applications/ubiqX.app
```

Assinar com a **mesma** identidade `ubiqX Dev` a cada atualização é o que mantém Gravação de Tela, Automação e
o Keychain: o macOS liga essas permissões à assinatura, e um binário reassinado com a mesma identidade é, para
ele, o mesmo app. É exatamente o que a atualização automática não consegue fazer hoje. Instalação no Windows e
no Linux: [`WINDOWS-LINUX.md`](WINDOWS-LINUX.md) § 1.

**Conferir uma assinatura à mão.** Cada artefato de atualização é publicado com o seu `.sig` ao lado:

```bash
minisign -Vm ubiqX-macos-aarch64.app.tar.gz -P "$(cat apps/desktop/src-tauri/tauri.conf.json | python3 -c 'import json,sys,base64; print(base64.b64decode(json.load(sys.stdin)["plugins"]["updater"]["pubkey"]).decode().splitlines()[1])')"
```

**Outro feed.** Para apontar o app a um fork ou a um servidor próprio, compile com
`UBIQX_UPDATE_FEED_URL=https://.../latest.json pnpm tauri build` (o aviso) e troque
`plugins > updater > endpoints` no `tauri.conf.json` (a instalação). O `scripts/publish-release.mjs` aceita
`--repo`, `--tag` e vários `--asset` (plataforma e tipo inferidos pelo nome do arquivo; `--dmg` e `--app-zip`
continuam valendo) para publicar em outro lugar; `node scripts/publish-release.mjs --help` lista tudo.

## 4. Primeira execução (onboarding)

1. **Escolha a IA e cole a chave** — Anthropic Claude (https://console.anthropic.com), OpenAI
   (https://platform.openai.com/api-keys) ou xAI Grok (https://console.x.ai). Crie a chave em um
   *workspace/projeto dedicado* com limite de gasto mensal (é a rede de segurança real). O ubiqX
   valida a chave com uma chamada mínima e a guarda no Keychain (clique em *Permitir sempre* na
   primeira vez). Dá para trocar de provedor depois em Configurações → IA sem perder as chaves.
2. **Gravação de Tela** — o ubiqX pede a permissão. Depois de conceder, **reinicie o app**:
   o macOS só aplica a permissão em um processo novo. Sem ela os títulos de janela chegam
   vazios e a captura falha (o app detecta e avisa).
3. **Automação** — para ler a URL do navegador ativo. O sistema pergunta uma vez por navegador
   (Safari, Chrome, Arc, Brave, Edge…) e **só para os navegadores abertos naquele momento**: abra o
   navegador que você usa antes de clicar em "Pedir permissão". A primeira consulta a cada navegador
   espera até 2 min pela sua resposta ao alerta. Firefox não expõe a URL: fica só o título.
4. **Categorias** — crie IFRO, Incubadora, Cidades Inteligentes… com uma descrição de
   *o que conta como trabalho desta categoria*. A IA usa exatamente esse texto.
5. **Horário do relatório** por categoria (padrão 18:00) e **iniciar com o sistema**.

## 5. Onde ficam os dados

| O quê | Onde |
|---|---|
| Banco de dados (SQLite) | `~/Library/Application Support/ai.ubiqx.app/ubiqx.db` |
| Prints (JPEG, apagados após uso) | `~/Library/Application Support/ai.ubiqx.app/screenshots.noindex/` |
| Logs | `~/Library/Logs/ai.ubiqx.app/` |
| Chaves de API (uma por provedor) | Keychain, serviço `ai.ubiqx` |

Nada além das chamadas à API configurada sai da máquina. *Configurações → Privacidade* mostra
exatamente o texto enviado à IA por bloco e permite apagar tudo.

## 6. Problemas comuns

| Sintoma | Causa | Solução |
|---|---|---|
| Títulos vazios, prints falham | Gravação de Tela não concedida ou app não reiniciado | Conceder e reiniciar o ubiqX |
| Permissão pedida de novo a cada build | Binário sem assinatura estável | `scripts/codesign-dev.sh` |
| "Solicitar" da Automação avisa que nenhum navegador está aberto | O alerta do macOS só existe para navegadores em execução | Abra o Safari/Chrome/Arc… e clique de novo; o status vira "Concedida" ao permitir |
| URL do navegador não aparece | Automação negada (`-1743`) | Ajustes → Privacidade → Automação → ubiqX → marcar o navegador |
| "IA não configurada" | Chave ausente/inválida | Configurações → IA → validar chave |
| "IA indisponível: cobrança…" | Conta sem créditos ou chave desativada | Adicionar créditos no console da Anthropic e salvar a chave de novo |
| "ubiqX está danificado" / "desenvolvedor não identificado" ao abrir um app baixado | Quarentena do Gatekeeper num bundle com assinatura ad hoc | `xattr -dr com.apple.quarantine ubiqX.app` e assinar (§ 3.1) |
| Banner de atualização não aparece mesmo com commit novo | Build de desenvolvimento (epoch 0) não verifica sozinho, verificação automática desligada, ou a CI ainda não publicou | Configurações → Atualizações → **Verificar agora**; confira a release `continuous` no GitHub e o log do job *Publish continuous release* (§ 3.2) |
| "Publicação pulada: GITHUB_TOKEN não definido" no log do Codemagic | Grupo `ubiqx_github` não criado ou `groups` ainda comentado no yaml | Criar o token fine-grained e o grupo, descomentar `groups` (§ 3.2); enquanto isso o GitHub Actions publica sozinho |
| Atualização automática pede Gravação de Tela e Automação de novo | Esperado: a CI assina o bundle ad hoc, sem identidade fixa, e o macOS vê um app diferente | Reconceder em Ajustes → Privacidade e Segurança, ou usar o caminho manual e reassinar com `ubiqX Dev`; solução definitiva no § 3.2 |
| Job da CI falha em **Updater signing key** | Os secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` não foram cadastrados | Cadastrar os dois em Settings → Secrets and variables → Actions (§ 3.2) |
| **Atualizar agora** diz "Este build não consegue se atualizar sozinho" | O `updater.json` não tem entrada para este sistema/arquitetura, ou o build saiu sem assinatura | Usar **Baixar (.dmg)** e instalar como no § 3.1; conferir o `updater.json` da release `continuous` |
| Cmd+Q "não fecha" o app | Esperado: Cmd+Q só esconde a janela | Para encerrar de verdade: tray → **Sair** |
| Log sem linhas do rastreador | Nível de log baixo | `UBIQX_LOG=debug` antes de abrir o app; o arquivo fica em `~/Library/Logs/ai.ubiqx.app/` |
| macOS 15 mostra aviso periódico de captura de tela | Comportamento do sistema para apps que usam captura | Esperado; clique em *Continuar a permitir* |
