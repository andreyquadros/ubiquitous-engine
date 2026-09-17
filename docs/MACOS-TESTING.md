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
scripts/install-ubi-model.sh ~/Downloads/Ubi.glb   # opcional: mascote 3D
cd apps/desktop && pnpm install
pnpm tauri build                                   # gera target/release/bundle/macos/ubiqX.app
cd ../.. && scripts/codesign-dev.sh                # assina com "ubiqX Dev"
open target/release/bundle/macos/ubiqX.app
```

Teste **sempre pelo `.app` empacotado**. `pnpm tauri dev` funciona para iterar na interface, mas
o binário de desenvolvimento não carrega o `Info.plist` (sem `LSUIElement`, sem descrição de
uso de Apple Events) e as notificações não aparecem.

## 4. Primeira execução (onboarding)

1. **Chave de API** — crie uma chave em https://console.anthropic.com em um *workspace
   dedicado* com limite de gasto mensal (é a rede de segurança real). Cole no ubiqX; ele valida
   e guarda no Keychain (clique em *Permitir sempre* na primeira vez).
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
| Chave de API | Keychain, serviço `ai.ubiqx` |

Nada além das chamadas à API configurada sai da máquina. *Configurações → Privacidade* mostra
exatamente o texto enviado à IA por bloco e permite apagar tudo.

## 6. Problemas comuns

| Sintoma | Causa | Solução |
|---|---|---|
| Títulos vazios, prints falham | Gravação de Tela não concedida ou app não reiniciado | Conceder e reiniciar o ubiqX |
| Permissão pedida de novo a cada build | Binário sem assinatura estável | `scripts/codesign-dev.sh` |
| URL do navegador não aparece | Automação negada (`-1743`) | Ajustes → Privacidade → Automação → ubiqX → marcar o navegador |
| "IA não configurada" | Chave ausente/inválida | Configurações → IA → validar chave |
| "IA indisponível: cobrança…" | Conta sem créditos ou chave desativada | Adicionar créditos no console da Anthropic e salvar a chave de novo |
| Cmd+Q "não fecha" o app | Esperado: Cmd+Q só esconde a janela | Para encerrar de verdade: tray → **Sair** |
| Log sem linhas do rastreador | Nível de log baixo | `UBIQX_LOG=debug` antes de abrir o app; o arquivo fica em `~/Library/Logs/ai.ubiqx.app/` |
| macOS 15 mostra aviso periódico de captura de tela | Comportamento do sistema para apps que usam captura | Esperado; clique em *Continuar a permitir* |
