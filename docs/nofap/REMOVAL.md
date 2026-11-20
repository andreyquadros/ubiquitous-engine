# Como remover o NoFap do computador

Tudo o que o NoFap pode deixar numa máquina, e como tirar cada coisa: pelo app, com um
comando, ou à mão, mesmo que o app já tenha sido apagado. Vale para os dois produtos: o
módulo dentro do ubiqX e o app separado. No sistema, os dois usam o mesmo nome neutro,
`ubiqx-shield`.

Quem tem a senha de administrador deste computador sempre consegue remover a proteção. Isso é
de propósito: o NoFap dificulta, não tranca.

Se você só apagou o app, o computador continua funcionando: os sites da lista continuam
barrados pelo sistema e, se você tinha escolhido o DNS filtrado, ele continua em uso. Nada
quebra; só não há mais o app para desfazer com um clique. Siga a parte 2 ou a parte 3.

## O que pode ter ficado

| O quê | Onde | Sai com |
|---|---|---|
| Bloco de sites no arquivo `hosts` | `/etc/hosts` (macOS, Linux); `C:\Windows\System32\drivers\etc\hosts` | partes 1, 2 ou 3 |
| DNS do sistema apontando para a Cloudflare for Families (`1.1.1.3`, `1.0.0.3`, `2606:4700:4700::1113`, `2606:4700:4700::1003`) | serviços de rede (macOS); adaptadores (Windows); `/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf` (Linux) | partes 1, 2 ou 3 |
| Políticas dos navegadores | registro (Windows); arquivos `ubiqx-shield.json` e `/etc/firefox/policies/policies.json` (Linux); preferências do Firefox (macOS) | partes 1, 2 ou 3 |
| Pasta de estado | `/var/lib/ubiqx-shield` (Linux); `/Library/Application Support/ubiqx-shield` (macOS); `C:\Program Files\ubiqx-shield` (Windows) | partes 1, 2 ou 3 |
| Dados na sua conta: banco de dados, exportações, registros, chave de IA, início automático, permissões | pastas da sua conta e cofre do sistema | parte 4 |

Antes de começar, feche os navegadores. Depois de terminar, abra de novo: é quando eles
largam o aviso "Gerenciado pela sua organização" e os endereços que tinham em memória.

## 1. Pelo app

1. Abra o NoFap (no ubiqX, a página do módulo) e escolha desligar a proteção. O pedido
   espera o tempo que você mesmo definiu e depois pede uma confirmação.
2. Confirmado o desligamento, o sistema pede a senha de administrador para tirar as camadas
   do sistema. Se você recusar, a página mostra o botão **Remover do sistema**, que faz o
   mesmo pedido de novo.
3. Se quiser tirar também os dados, com a proteção desligada use **Apagar tudo do NoFap**.
4. Desligue "Abrir junto com o computador" antes de desinstalar.

**Desinstaladores.** O que cada formato faz ao desinstalar:

| Formato | Ao desinstalar |
|---|---|
| Windows, instalador `-setup.exe` | o desinstalador pede a permissão de administrador uma vez e remove as camadas do sistema. Se você recusar, a desinstalação termina e sobra o que está na parte 2. Uma atualização do app não remove nada |
| Windows, `.msi` | não remove as camadas do sistema: use a parte 2 antes de desinstalar, ou a parte 3 depois |
| Linux, `.deb` | `sudo apt remove <pacote>` remove as camadas do sistema antes de tirar os arquivos (o pacote do ubiqX se chama `ubiqx`; `dpkg -S /usr/bin/ubiqx-shield` diz o nome do pacote do app separado). Uma atualização do pacote não remove nada |
| Linux, AppImage | o AppImage não consegue aplicar as camadas do sistema, então não há o que remover delas; apague o arquivo e veja a parte 4 |
| macOS, `.app` (arrastado do `.dmg`) | arrastar para o Lixo não remove nada do sistema: use a parte 2 antes de apagar o app, ou a parte 3 depois |

O comportamento dos desinstaladores ainda precisa ser conferido em máquina real (item 15 de
[REAL-MACHINE-CHECKLIST.md](REAL-MACHINE-CHECKLIST.md)). Depois de desinstalar, vale passar
pela parte 5 e conferir.

## 2. Com um comando

O próprio executável do app remove tudo do sistema quando é chamado com `--shield-helper`.
Não abre janela, não precisa do banco de dados e funciona com o app fechado. `--owner all`
remove o que os dois produtos pediram; `--owner ubiqx` ou `--owner shield` remove só o de um
e mantém o que o outro ainda quer.

O comando termina com 0 quando tudo saiu, 3 quando uma parte não saiu (rode de novo, ou siga
a parte 3) e 4 quando não conseguiu começar.

### macOS

```sh
# ubiqX
sudo "/Applications/ubiqX.app/Contents/MacOS/ubiqX" --shield-helper remove --owner all --scope all

# app separado
sudo "/Applications/NoFap.app/Contents/MacOS/ubiqx-shield" --shield-helper remove --owner all --scope all
```

Se o app está em outro lugar, o executável é o único arquivo dentro de
`<app>.app/Contents/MacOS/`.

### Linux (`.deb`)

```sh
# ubiqX
sudo /usr/bin/ubiqx --shield-helper remove --owner all --scope all

# app separado
sudo /usr/bin/ubiqx-shield --shield-helper remove --owner all --scope all
```

### Windows

No PowerShell (não precisa abrir como administrador: o Windows pede a permissão):

```powershell
# ubiqX, instalado pelo -setup.exe
$exe = "$env:LOCALAPPDATA\ubiqX\ubiqX.exe"
# ubiqX, instalado pelo .msi:      $exe = "$env:ProgramFiles\ubiqX\ubiqX.exe"
# app separado, pelo -setup.exe:   $exe = "$env:LOCALAPPDATA\NoFap\ubiqx-shield.exe"

$p = Start-Process -FilePath $exe -ArgumentList '--shield-helper','remove','--owner','all','--scope','all' -Verb RunAs -Wait -PassThru
$p.ExitCode
```

Para achar o executável: clique com o botão direito no atalho do app e escolha "Abrir local
do arquivo".

### Sem o app instalado: o auxiliar compilado do código

Quem tem o código-fonte pode compilar o auxiliar sozinho, sem interface gráfica. Ele recebe a
ação direto, sem `--shield-helper`:

```sh
cargo build --release -p ubiqx-platform --bin ubiqx-shield-helper
sudo target/release/ubiqx-shield-helper remove --owner all --scope all
```

No Windows, num PowerShell aberto como administrador:

```powershell
cargo build --release -p ubiqx-platform --bin ubiqx-shield-helper
.\target\release\ubiqx-shield-helper.exe remove --owner all --scope all
```

Para só devolver o DNS ao automático e manter o resto, troque `--scope all` por
`--scope resolver`.

## 3. À mão, sem o app

Cada bloco abaixo é independente: use só os que se aplicam. Os comandos apagam apenas o que
tem a marca ou o nome do `ubiqx-shield`, o DNS que ainda é o que ele pôs, e os ajustes de
navegador com os nomes que ele usa.

### 3.1 Linux

**Arquivo `hosts`.** Faz uma cópia e apaga da linha BEGIN à linha END. Se uma das duas marcas
tiver sido apagada à mão, o comando não mexe em nada; nesse caso abra o arquivo
(`sudo nano /etc/hosts`) e apague as linhas do bloco.

```sh
sudo cp -p /etc/hosts /etc/hosts.antes-de-remover
sudo perl -0pi -e 's/^# >>> ubiqx-shield .*?^# <<< ubiqx-shield END[^\n]*\n?//ms' /etc/hosts
sudo resolvectl flush-caches 2>/dev/null || true
```

**DNS.**

```sh
sudo rm -f /etc/systemd/resolved.conf.d/90-ubiqx-shield.conf
sudo systemctl restart systemd-resolved
```

**Políticas dos navegadores.** Os arquivos com o nome do `ubiqx-shield` são dele:

```sh
sudo rm -f /etc/opt/chrome/policies/managed/ubiqx-shield.json \
           /etc/chromium/policies/managed/ubiqx-shield.json \
           /etc/chromium-browser/policies/managed/ubiqx-shield.json \
           /etc/opt/edge/policies/managed/ubiqx-shield.json \
           /etc/brave/policies/managed/ubiqx-shield.json
```

O do Firefox não tem o nome no arquivo, e só é criado quando não existia nenhum. Olhe antes
de apagar:

```sh
cat /etc/firefox/policies/policies.json
```

Se o conteúdo for exatamente este, o arquivo é do `ubiqx-shield` e pode ser apagado com
`sudo rm /etc/firefox/policies/policies.json`. Se tiver mais coisas, é de outra pessoa ou de
outro programa: deixe como está.

```json
{
  "policies": {
    "DNSOverHTTPS": {
      "Enabled": false,
      "Locked": true
    }
  }
}
```

**Pasta de estado.**

```sh
sudo rm -rf /var/lib/ubiqx-shield
```

### 3.2 macOS

**Arquivo `hosts`.** Como no Linux: cópia, depois apaga da linha BEGIN à linha END, e não
mexe em nada se faltar uma das marcas.

```sh
sudo cp -p /etc/hosts /etc/hosts.antes-de-remover
sudo perl -0pi -e 's/^# >>> ubiqx-shield .*?^# <<< ubiqx-shield END[^\n]*\n?//ms' /etc/hosts
sudo dscacheutil -flushcache
sudo killall -HUP mDNSResponder
```

**DNS.** Devolve ao automático só os serviços de rede que estão com o endereço da Cloudflare
for Families; um DNS que você configurou depois não é tocado.

```sh
networksetup -listallnetworkservices | tail -n +2 | grep -v '^\*' | while IFS= read -r servico; do
  if networksetup -getdnsservers "$servico" | grep -qx '1.1.1.3'; then
    sudo networksetup -setdnsservers "$servico" empty
    echo "DNS automático de novo: $servico"
  fi
done
```

**Política do Firefox.** No macOS só o Firefox recebe ajuste. Veja o que há e apague as duas
chaves do `ubiqx-shield`:

```sh
defaults read /Library/Preferences/org.mozilla.firefox
sudo defaults delete /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Enabled
sudo defaults delete /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Locked
```

Se depois disso só sobrar `EnterprisePoliciesEnabled`, apague-a também. Se houver outras
políticas, elas não são do `ubiqx-shield`: deixe `EnterprisePoliciesEnabled` como está.

```sh
sudo defaults delete /Library/Preferences/org.mozilla.firefox EnterprisePoliciesEnabled
```

**Pasta de estado.**

```sh
sudo rm -rf "/Library/Application Support/ubiqx-shield"
```

### 3.3 Windows

Abra o PowerShell como administrador (menu Iniciar, digite "PowerShell", "Executar como
administrador").

**Arquivo `hosts`.** Faz uma cópia e apaga da linha BEGIN à linha END; não mexe em nada se
faltar uma das marcas. (Outro caminho: abrir o Bloco de Notas como administrador, abrir
`C:\Windows\System32\drivers\etc\hosts` e apagar essas linhas.)

```powershell
$arquivo = "$env:SystemRoot\System32\drivers\etc\hosts"
Copy-Item $arquivo "$arquivo.antes-de-remover"
$texto = [System.IO.File]::ReadAllText($arquivo)
$limpo = [regex]::Replace($texto, '(?ms)^# >>> ubiqx-shield .*?^# <<< ubiqx-shield END[^\n]*\n?', '')
[System.IO.File]::WriteAllText($arquivo, $limpo)
ipconfig /flushdns
```

**DNS.** Devolve ao automático só os adaptadores que estão com o endereço da Cloudflare for
Families.

```powershell
Get-NetAdapter | ForEach-Object {
  $dns = (Get-DnsClientServerAddress -InterfaceIndex $_.ifIndex).ServerAddresses
  if ($dns -contains '1.1.1.3') {
    Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ResetServerAddresses
    "DNS automático de novo: $($_.Name)"
  }
}
ipconfig /flushdns
```

**Políticas dos navegadores.** Num computador de trabalho, estes mesmos valores podem ter
sido postos pela empresa. O `ubiqx-shield` não escreve onde já havia um valor diferente; se
esta máquina é gerenciada por uma organização, não apague à mão: use a parte 2, que só apaga
o que ele mesmo criou. Num computador pessoal:

```powershell
$chromium = 'HKLM\SOFTWARE\Policies\Google\Chrome', 'HKLM\SOFTWARE\Policies\Chromium',
            'HKLM\SOFTWARE\Policies\Microsoft\Edge', 'HKLM\SOFTWARE\Policies\BraveSoftware\Brave'
foreach ($chave in $chromium) {
  foreach ($valor in 'DnsOverHttpsMode', 'ForceGoogleSafeSearch', 'ForceBingSafeSearch', 'TorDisabled') {
    reg.exe delete $chave /v $valor /f 2>$null
  }
}
reg.exe delete 'HKLM\SOFTWARE\Policies\Mozilla\Firefox\DNSOverHTTPS' /v Enabled /f 2>$null
reg.exe delete 'HKLM\SOFTWARE\Policies\Mozilla\Firefox\DNSOverHTTPS' /v Locked /f 2>$null
```

Os valores que o `ubiqx-shield` escreve são: `DnsOverHttpsMode = off` e
`ForceGoogleSafeSearch = 1` (Chrome, Chromium, Edge, Brave), `ForceBingSafeSearch = 2` (só no
Edge), `TorDisabled = 1` (só no Brave), e `Enabled = 0`, `Locked = 1` na chave do Firefox.
Mensagens de "não foi possível localizar" são normais: o valor não existia.

**Pasta de estado.**

```powershell
Remove-Item -Recurse -Force "$env:ProgramFiles\ubiqx-shield"
```

## 4. O que fica na sua conta

Isto não bloqueia nada e não precisa de senha de administrador. São os dados do app.

**Atenção, ubiqX:** a pasta de dados do ubiqX guarda todo o ubiqX (atividades, relatórios,
configurações), não só o NoFap. Para apagar só o que é do NoFap, use "Apagar tudo do NoFap"
dentro do app. Apague a pasta inteira só se estiver removendo o ubiqX.

| | macOS | Windows | Linux |
|---|---|---|---|
| Dados do ubiqX | `~/Library/Application Support/ai.ubiqx.app` | `%AppData%\ai.ubiqx.app` | `~/.local/share/ai.ubiqx.app` |
| Dados do app separado | `~/Library/Application Support/ai.ubiqx.shield` | `%AppData%\ai.ubiqx.shield` | `~/.local/share/ai.ubiqx.shield` |
| Registros (log) | `~/Library/Logs/ai.ubiqx.app`, `~/Library/Logs/ai.ubiqx.shield` | `%LocalAppData%\ai.ubiqx.app\logs`, `%LocalAppData%\ai.ubiqx.shield\logs` | dentro da pasta de dados, em `logs` |
| Cache da interface | `~/Library/Caches/ai.ubiqx.*`, `~/Library/WebKit/ai.ubiqx.*` | `%LocalAppData%\ai.ubiqx.app`, `%LocalAppData%\ai.ubiqx.shield` | `~/.cache/ai.ubiqx.app`, `~/.cache/ai.ubiqx.shield` |
| Arquivo que os dois produtos compartilham | `~/Library/Application Support/ubiqx-shield` | `%LocalAppData%\ubiqx-shield` | `~/.local/share/ubiqx-shield` |
| Início automático | `~/Library/LaunchAgents/ubiqX.plist`, `NoFap.plist` | valores `ubiqX` e `NoFap` em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | `~/.config/autostart/ubiqX.desktop`, `NoFap.desktop` |

O banco de dados (`ubiqx.db` ou `shield.db`) e as exportações (`exports/`) ficam dentro da
pasta de dados.

### macOS, app separado

```sh
rm -rf ~/Library/Application\ Support/ai.ubiqx.shield ~/Library/Logs/ai.ubiqx.shield \
       ~/Library/Caches/ai.ubiqx.shield ~/Library/WebKit/ai.ubiqx.shield \
       ~/Library/Application\ Support/ubiqx-shield
rm -f ~/Library/LaunchAgents/NoFap.plist
# a chave de IA guardada no Keychain: repita até dizer que o item não foi encontrado
security delete-generic-password -s ai.ubiqx.shield
# as permissões de Gravação de Tela e Automação dadas ao app
tccutil reset ScreenCapture ai.ubiqx.shield
tccutil reset AppleEvents ai.ubiqx.shield
```

Para o ubiqX inteiro, os mesmos comandos com `ai.ubiqx.app` no lugar de `ai.ubiqx.shield`,
`ubiqX.plist` no lugar de `NoFap.plist` e `-s ai.ubiqx` no comando `security`.

### Linux, app separado

```sh
rm -rf ~/.local/share/ai.ubiqx.shield ~/.cache/ai.ubiqx.shield ~/.local/share/ubiqx-shield
rm -f ~/.config/autostart/NoFap.desktop
# a chave de IA guardada no chaveiro (sem chaveiro, ela está em secrets.json na pasta de dados)
secret-tool clear service ai.ubiqx.shield
```

Para o ubiqX inteiro, os mesmos comandos com `ai.ubiqx.app`, `ubiqX.desktop` e
`service ai.ubiqx`.

### Windows, app separado

```powershell
Remove-Item -Recurse -Force "$env:APPDATA\ai.ubiqx.shield", "$env:LOCALAPPDATA\ai.ubiqx.shield", "$env:LOCALAPPDATA\ubiqx-shield" -ErrorAction SilentlyContinue
reg.exe delete 'HKCU\Software\Microsoft\Windows\CurrentVersion\Run' /v NoFap /f 2>$null
# a chave de IA guardada no Gerenciador de Credenciais: veja as entradas e apague cada uma
cmdkey /list | Select-String 'ai.ubiqx.shield'
# cmdkey /delete:<o destino mostrado na linha acima>
```

Para o ubiqX inteiro, os mesmos comandos com `ai.ubiqx.app`, o valor `ubiqX` e as entradas
`ai.ubiqx` do Gerenciador de Credenciais.

Se o nome do item de início automático for outro na sua máquina, procure por ele:
`ls ~/Library/LaunchAgents`, `ls ~/.config/autostart`, ou
`reg.exe query HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

O histórico de notificações é do sistema operacional, não do app; como limpar está em
[PRIVACY.md](PRIVACY.md).

## 5. Como conferir que saiu tudo

Com o app ou com o auxiliar, a ação `status` mostra o que há no sistema, sem precisar de
senha (por exemplo `/usr/bin/ubiqx-shield --shield-helper status`). Sem eles:

### Linux

```sh
grep -c 'ubiqx-shield' /etc/hosts                                   # 0
ls /var/lib/ubiqx-shield /etc/systemd/resolved.conf.d/90-ubiqx-shield.conf   # "não encontrado" nos dois
ls /etc/opt/*/policies/managed/ubiqx-shield.json /etc/*/policies/managed/ubiqx-shield.json   # nenhum
resolvectl dns                                                      # sem 1.1.1.3 na linha "Global"
getent hosts canary.ubiqx-shield.invalid                            # nada
```

### macOS

```sh
grep -c 'ubiqx-shield' /etc/hosts                                   # 0
ls "/Library/Application Support/ubiqx-shield"                      # "No such file or directory"
networksetup -listallnetworkservices | tail -n +2 | grep -v '^\*' | while IFS= read -r s; do echo "$s: $(networksetup -getdnsservers "$s" | tr '\n' ' ')"; done   # sem 1.1.1.3
defaults read /Library/Preferences/org.mozilla.firefox              # sem DNSOverHTTPS__*
dscacheutil -q host -a name canary.ubiqx-shield.invalid             # nada
```

### Windows

```powershell
(Select-String -Path "$env:SystemRoot\System32\drivers\etc\hosts" -Pattern 'ubiqx-shield' | Measure-Object).Count   # 0
Test-Path "$env:ProgramFiles\ubiqx-shield"                          # False
Get-DnsClientServerAddress | Where-Object { $_.ServerAddresses -contains '1.1.1.3' }   # nada
reg.exe query 'HKLM\SOFTWARE\Policies\Google\Chrome' /v DnsOverHttpsMode   # "não foi possível localizar"
Resolve-DnsName canary.ubiqx-shield.invalid                         # erro: o nome não existe
```

Nos navegadores, depois de reabrir: `chrome://policy`, `edge://policy`, `brave://policy` e
`about:policies` não devem mais listar os ajustes acima.

## Notas

- Os comandos da parte 3 foram escritos a partir dos valores que o código do `ubiqx-shield`
  grava (marcas do bloco, caminhos, nomes das políticas). O comando do arquivo `hosts` foi
  testado em cópias de arquivos gerados pelo próprio código; os demais ainda não foram
  executados em máquina real (itens 1, 3, 9 e 15 da lista de verificação).
- O comando do arquivo `hosts` pode deixar uma quebra de linha a mais no fim do arquivo,
  quando o arquivo original não terminava com uma. Não tem efeito nenhum.
- Os caminhos de instalação da parte 2 e os nomes do início automático da parte 4 seguem o
  nome de exibição de cada produto. Os do ubiqX são os que os instaladores publicados usam
  hoje; os do app separado ainda não foram conferidos num instalador de verdade, e mudam se o
  nome de exibição dele mudar (`NoFap.app`, `%LocalAppData%\NoFap`, `NoFap.plist`). O nome do
  executável do app separado (`ubiqx-shield`) e tudo o que fica no sistema (parte 3) não
  mudam.
