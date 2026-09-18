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

## 3.2 Atualizações: como o app avisa e como instalar

Cada commit que a nuvem compila vira uma atualização para quem já usa o ubiqX. Não há auto-instalação:
o app avisa, você baixa o `.dmg` com um clique e instala como no § 3.1.

**Como funciona.** A CI publica cada build na release rolante **`continuous`** do repositório
(`https://github.com/andreyquadros/ubiquitous-engine/releases/tag/continuous`). A release recebe sempre os
mesmos arquivos, com nomes estáveis, para as três plataformas: `ubiqX-macos-aarch64.dmg` e
`ubiqX-macos-aarch64.app.zip` (macOS), `ubiqX-windows-x86_64-setup.exe` e `ubiqX-windows-x86_64.msi`
(Windows), `ubiqX-linux-x86_64.AppImage` e `ubiqX-linux-x86_64.deb` (Linux), mais o `latest.json`, o "feed"
com versão, data do commit (`build.epoch`), número do build, sha, as notas do commit e uma entrada por
plataforma em `platforms` (`darwin-aarch64`, `windows-x86_64`, `linux-x86_64`, cada uma com `url`, `kind`,
`size` e, no macOS, `app_zip_url`). O app escolhe a entrada do sistema e da arquitetura em que roda (com
`…-universal` e depois qualquer chave do mesmo sistema como reserva). Cada binário sai da compilação com
esses mesmos números gravados (`scripts/build-info.sh` os calcula e o `build.rs` os grava). O app baixa o
`latest.json` 45 s depois de abrir e depois a cada 6 h; se a versão for maior, ou igual com um commit mais
novo, há atualização. Builds de desenvolvimento (`pnpm tauri dev`, sem git) não verificam sozinhos; a
verificação manual em Configurações continua funcionando para testar. Instalação no Windows e no Linux:
[`WINDOWS-LINUX.md`](WINDOWS-LINUX.md) § 1.

**Onde aparece.** Um banner no topo do app com a data e o sha do build e os botões **Como instalar**,
**Depois** e **Baixar (.dmg)**; a seção **Atualizações** em Configurações (build atual, última verificação,
**Verificar agora**, interruptor da verificação automática e, quando há build novo, as notas do commit com
**Baixar** e **Página do release**); uma notificação do macOS uma vez por build ("Nova versão do ubiqX. Build
18/09 15:04 (a1b2c3d) já está disponível"); e um item no menu da barra de menus, **Verificar atualizações…**
enquanto nenhum build novo é conhecido e **Baixar a nova versão (18/09 15:04 a1b2c3d)…** depois que um
aparece. **Depois** esconde o banner só daquele build; o próximo avisa de novo.

**GitHub Actions (automático).** Já está configurado: todo push compila os três sistemas (jobs `macos-app`,
`windows-app` e `linux-app`) e o job `publish-continuous` publica os seis instaladores e o `latest.json` de
uma vez, com o `GITHUB_TOKEN` que o próprio Actions fornece (o job tem `permissions: contents: write`), então o
feed nunca mostra um build pela metade. Pull requests só compilam. Um push novo cancela o build anterior do
mesmo branch, e a publicação recusa sobrescrever um build mais novo do que o dela, então a release nunca
"volta no tempo". Quando o `latest.json` já publicado é do **mesmo** commit (Codemagic e Actions a publicar o
mesmo push), as plataformas são mescladas: cada publicador troca só as suas.

**Codemagic (opcional, precisa de um token).** O Codemagic não recebe token do GitHub sozinho. Para ele
também publicar:

1. No GitHub: *Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new
   token*. Repositório: só `ubiquitous-engine`. Permissão: **Contents: Read and write**. Copie o token.
2. No Codemagic: *Teams → Global variables and secrets* (ou na aplicação, *Environment variables*): variável
   `GITHUB_TOKEN`, valor = o token, grupo **`ubiqx_github`**, marque *Secure*.
3. No `codemagic.yaml`, descomente as linhas
   ```yaml
   groups:
     - ubiqx_github
   ```
   (e `- ubiqx_apple` se também tiver o grupo de assinatura). Faça commit.

Sem o grupo a etapa *Publicar atualização (GitHub Releases)* escreve "Publicação pulada: GITHUB_TOKEN não
definido" e o build continua disponível em Artifacts. Os dois caminhos publicam na mesma release: com o
mesmo commit as plataformas se somam (o Codemagic só traz o macOS); com commits diferentes, quem terminar por
último com o commit mais novo vence e o outro pula.

**Instalar a atualização.** O botão **Baixar** abre o `.dmg` no navegador. O bundle continua com assinatura
ad hoc e em quarentena, então repita a sequência do § 3.1:

```bash
open ~/Downloads/ubiqX-macos-aarch64.dmg          # arraste o ubiqX para Aplicativos (substitua) e ejete
xattr -dr com.apple.quarantine /Applications/ubiqX.app
codesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app   # = scripts/codesign-dev.sh /Applications/ubiqX.app
open /Applications/ubiqX.app
```

Assinar com a **mesma** identidade `ubiqX Dev` a cada atualização é o que mantém Gravação de Tela, Automação e
o Keychain: o macOS liga essas permissões à assinatura, e um binário reassinado com a mesma identidade é, para
ele, o mesmo app. Se pular o `codesign`, o novo build volta a pedir tudo.

**Outro feed.** Para apontar o app a um fork ou a um servidor próprio, compile com
`UBIQX_UPDATE_FEED_URL=https://.../latest.json pnpm tauri build` (o valor fica gravado no binário; o padrão é o
`latest.json` da release `continuous` deste repositório). O `scripts/publish-release.mjs` aceita `--repo`,
`--tag` e vários `--asset` (plataforma e tipo inferidos pelo nome do arquivo; `--dmg` e `--app-zip` continuam
valendo) para publicar em outro lugar; `node scripts/publish-release.mjs --help` lista tudo.

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
| Atualização instalada pede Gravação de Tela de novo | Novo bundle sem a assinatura `ubiqX Dev` | `xattr -dr com.apple.quarantine` e `codesign` com a mesma identidade (§ 3.2) |
| Cmd+Q "não fecha" o app | Esperado: Cmd+Q só esconde a janela | Para encerrar de verdade: tray → **Sair** |
| Log sem linhas do rastreador | Nível de log baixo | `UBIQX_LOG=debug` antes de abrir o app; o arquivo fica em `~/Library/Logs/ai.ubiqx.app/` |
| macOS 15 mostra aviso periódico de captura de tela | Comportamento do sistema para apps que usam captura | Esperado; clique em *Continuar a permitir* |
