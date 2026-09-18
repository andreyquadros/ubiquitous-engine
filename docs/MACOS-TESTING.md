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

**Arte do UBI.** Os builds na nuvem só incluem o mascote se `apps/desktop/public/ubi/ubi.png` estiver no
repositório: envie o PNG pelo GitHub (*Add file → Upload files* dentro da pasta, nome exatamente `ubi.png`)
ou rode `scripts/install-ubi-model.sh` e faça commit do arquivo. O `Ubi.glb` continua fora do git.

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
| Cmd+Q "não fecha" o app | Esperado: Cmd+Q só esconde a janela | Para encerrar de verdade: tray → **Sair** |
| Log sem linhas do rastreador | Nível de log baixo | `UBIQX_LOG=debug` antes de abrir o app; o arquivo fica em `~/Library/Logs/ai.ubiqx.app/` |
| macOS 15 mostra aviso periódico de captura de tela | Comportamento do sistema para apps que usam captura | Esperado; clique em *Continuar a permitir* |
