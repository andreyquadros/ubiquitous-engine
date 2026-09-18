# ubiqX no Windows e no Linux

O mesmo app, o mesmo feed de atualização, os mesmos dados. O que muda é o adaptador de plataforma
(`crates/ubiqx-platform/src/{windows,linux}/`) e alguns rótulos da interface. Este guia cobre como
compilar, como instalar, o que funciona em cada sistema e o que ainda não funciona.

## 1. Baixar e instalar

Cada push na `main` publica a release rolante [`continuous`](https://github.com/andreyquadros/ubiquitous-engine/releases/tag/continuous)
com os instaladores das três plataformas, sempre com os mesmos nomes:

| Sistema | Arquivo | O que faz |
|---|---|---|
| Windows 10/11 (x64) | `ubiqX-windows-x86_64-setup.exe` | instalador NSIS, por usuário, sem privilégios de administrador |
| Windows 10/11 (x64) | `ubiqX-windows-x86_64.msi` | pacote MSI (WiX), para quem instala por política ou script |
| Linux (x64) | `ubiqX-linux-x86_64.AppImage` | roda de qualquer pasta, sem instalar |
| Linux (x64, Debian/Ubuntu) | `ubiqX-linux-x86_64.deb` | instala em `/usr/bin/ubiqx` com entrada de menu |

**Windows.** Abra o instalador e siga os passos. O binário ainda não tem assinatura de editor, então o
SmartScreen pode avisar: clique em *Mais informações* e depois em *Executar assim mesmo*. O ubiqX fica na
bandeja do sistema (ao lado do relógio); o instalador substitui a versão anterior e mantém dados e
configurações. Não há permissão a conceder.

**Linux.**

```bash
chmod +x ~/Downloads/ubiqX-linux-x86_64.AppImage && ~/Downloads/ubiqX-linux-x86_64.AppImage
# ou
sudo apt install ./ubiqX-linux-x86_64.deb
```

O AppImage precisa de FUSE 2 (`libfuse2` no Ubuntu 22.04+; a maioria das distros já traz) e o `.deb`
depende de `libwebkit2gtk-4.1-0`, `libgtk-3-0` e `libayatana-appindicator3-1` (ou `libappindicator3-1`),
os mesmos pacotes de qualquer app Tauri 2. O ícone aparece na bandeja do sistema; no GNOME é preciso a
extensão *AppIndicator and KStatusNotifierItem Support*.

**Atualizações.** O app lê o `latest.json` da release ao abrir e a cada 6 h, como no macOS
([`MACOS-TESTING.md`](MACOS-TESTING.md) § 3.2). A entrada do feed é escolhida pela chave
`windows-x86_64` / `linux-x86_64`; **Baixar** abre o `.exe` ou o `.AppImage` no navegador (o `.msi` e o
`.deb` ficam em `alternates`). A seção *Atualizações* em Configurações mostra as instruções do sistema em uso.

## 2. Compilar

Pré-requisitos comuns: Rust estável (`rustup`), Node 22, pnpm 10 e `pnpm install` em `apps/desktop`.
A identidade do build (epoch, número, sha, branch) vem de `scripts/build-info.sh` e precisa de bash: no
Windows use o Git Bash.

```bash
# Windows (Git Bash; precisa do Visual Studio Build Tools com "Desktop development with C++" e do WebView2, já presente no Windows 11)
eval "$(bash scripts/build-info.sh)"
cd apps/desktop && pnpm tauri build --bundles msi,nsis
# saída: target/release/bundle/msi/*.msi e target/release/bundle/nsis/*-setup.exe

# Linux (Ubuntu 22.04+)
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev libgtk-3-dev libssl-dev libfuse2 build-essential
eval "$(bash scripts/build-info.sh)"
cd apps/desktop && pnpm tauri build --bundles appimage,deb
# saída: target/release/bundle/appimage/*.AppImage e target/release/bundle/deb/*.deb
```

Sem toolchain: o GitHub Actions compila os três sistemas a cada push (artifacts `ubiqX-windows-app` e
`ubiqX-linux-app`, além do `ubiqX-macos-app`) e o job `publish-continuous` publica tudo de uma vez na release
`continuous`. O `scripts/publish-release.mjs` aceita vários `--asset` e infere plataforma e tipo pelo nome
do arquivo; quando o Codemagic (só macOS) publica o mesmo commit, as plataformas do `latest.json` são
mescladas em vez de sobrescritas.

Só para checar os adaptadores do Windows a partir de outro sistema (sem linker):

```bash
rustup target add x86_64-pc-windows-msvc
cargo check -p ubiqx-platform --target x86_64-pc-windows-msvc
```

O crate `ubiqx-desktop` não entra nessa checagem: `ring` e `libsqlite3-sys` compilam C para o alvo MSVC e
precisam de `cl.exe` e do Windows SDK. Ele é compilado de verdade no job `windows-app` da CI (e no seu
Windows, pelo `pnpm tauri build`).

Interface no navegador com dados simulados, fingindo outro sistema: `cd apps/desktop && pnpm dev` e abra
`http://localhost:1420/?platform=windows` ou `?platform=linux` (combina com `?update=available`,
`?onboarding=1` e `?lang=en`).

## 3. O que funciona

| | macOS | Windows | Linux (X11) |
|---|---|---|---|
| App, janela e título em foco | Accessibility/CG | `GetForegroundWindow` + `QueryFullProcessImageNameW` | `_NET_ACTIVE_WINDOW`, `_NET_WM_PID`, `_NET_WM_NAME` |
| Identidade do app (chave de regras e bloqueios) | bundle id (`com.google.Chrome`) | nome do executável em minúsculas, sem `.exe` (`chrome`, `msedge`, `code`); apps da Store (Configurações, Calculadora, WhatsApp…) pelo processo hospedado, não pelo `ApplicationFrameHost` | nome do executável em `/proc/<pid>/exe` (`google-chrome`, `firefox`, `code`) |
| Nome do app | nome do bundle | `FileDescription` do executável, ou o nome do arquivo | `Name` do `.desktop`, ou a classe `WM_CLASS` |
| URL da aba ativa | AppleScript (Automação) | UI Automation: Chrome, Edge, Brave, Opera, Vivaldi, Firefox | só pelo título da janela (ver § 4) |
| Ociosidade | CGEventSource | `GetLastInputInfo` | extensão XScreenSaver |
| Screenshots da janela ativa | ScreenCaptureKit/CG | `xcap`, recortado ao retângulo da janela (DWM) | X11 `GetImage` (`x11rb`), recortado à janela ativa |
| Permissões | Gravação de tela, Automação, Acessibilidade | nenhuma (só notificações) | nenhuma (só notificações) |
| Chave de API | Keychain | Gerenciador de Credenciais; se falhar, arquivo só do usuário na pasta de dados | chaveiro Secret Service; sem ele, arquivo `0600` na pasta de dados |
| Catálogo de apps (busca do Foco) | `/Applications` | atalhos do Menu Iniciar (`.lnk`) + `App Paths` do registro | `.desktop` em `/usr/share`, `~/.local/share` e exports do Flatpak |
| Notificações | Centro de notificações | toast do Windows | libnotify |
| Iniciar com o sistema | Login Items | chave `Run` do registro | `~/.config/autostart` |
| Pasta de dados | `~/Library/Application Support/ai.ubiqx.app` | `%AppData%\ai.ubiqx.app` | `~/.local/share/ai.ubiqx.app` |

O que é registrado, o que sai para a IA e o modo privado são idênticos nos três sistemas: a engine é a mesma
e só os adaptadores mudam.

## 4. Limitações conhecidas

**Wayland (Linux).** O adaptador fala X11, e um cliente X11 só enxerga janelas X11. Numa sessão Wayland
(o padrão do GNOME e do KDE hoje) só os apps que rodam pelo XWayland são rastreados; os nativos Wayland
(Firefox no GNOME desde a 121, os apps GTK4/GNOME, Chrome ou VS Code com o backend Wayland ligado) ficam
invisíveis: enquanto um deles tem foco, o `_NET_ACTIVE_WINDOW` do XWayland não aponta para nada, o
`foreground()` devolve "nada em foco", o rastreador não cria blocos e o guarda do Foco não age. Sem
`DISPLAY` nenhum (Wayland sem XWayland, headless) acontece o mesmo, e o app avisa uma vez no log. Para o
rastreamento completo, entre com **GNOME on Xorg** ou **Plasma (X11)** na tela de login. A captura de
tela segue a mesma regra: sem X11 ela devolve um erro de plataforma e a engine continua sem screenshots.
Um caminho pelos portais do Wayland (`xdg-desktop-portal`) fica para uma versão futura.

**URL do navegador no Linux.** Não há rota de acessibilidade ainda. O adaptador só devolve uma URL quando o
título da janela termina com o nome de um navegador conhecido (`… - Google Chrome`, `… — Mozilla Firefox`,
etc.) **e** contém um domínio reconhecível; caso contrário o bloco fica com o nome do app e o título. Regras
por domínio e bloqueios de sites dependem disso: funcionam para páginas que mostram o domínio no título e
não para as demais.

**URL do navegador no Windows.** Lida pela UI Automation: a primeira caixa de edição com `ValuePattern` nos
120 px superiores da janela (Chromium) ou a `urlbar-input` (Firefox). Enquanto você digita na barra o valor
não é uma URL e o adaptador devolve "sem URL"; qualquer falha da UIA também vira "sem URL", nunca um erro.
Navegadores fora da lista (Arc, Zen, Waterfox…) ficam só com o título.

**Foco (bloqueios e sessões).**

| Ação | macOS | Windows | Linux |
|---|---|---|---|
| Fechar app bloqueado | `quit` via AppleScript | `WM_CLOSE` em todas as janelas do executável; após ~1,5 s, `TerminateProcess` | `SIGTERM` nos processos do executável (só do seu usuário) |
| Fechar a aba bloqueada | AppleScript no navegador | `Ctrl+W` por `SendInput` na janela em foco | `Ctrl+W` por XTest na janela em foco |
| Deixar a aba em branco | AppleScript | `Ctrl+L`, `about:blank`, `Enter` | `Ctrl+L`, `about:blank`, `Return` |
| Esconder as outras janelas | System Events (pede permissão) | minimiza cada janela visível de outros apps | iconifica cada janela mapeada de outros apps |
| Atalho ao começar/encerrar a sessão | app Atalhos (`shortcuts run`) | uma linha de comando (`cmd /C …`) | uma linha de comando (`sh -c …`) |

No Windows e no Linux os dois campos de "atalho" viram **Comando ao começar/encerrar a sessão**: qualquer linha de
comando serve (ligar o Não perturbe, mudar o status no Slack, tocar um som). As exceções do guarda incluem
os apps do próprio sistema (`explorer`, `systemsettings`, `taskmgr`; `gnome-shell`, `plasmashell`, `nautilus`,
`dolphin`, `gnome-control-center`) e todos os terminais conhecidos (`windowsterminal`, `cmd`, `powershell`,
`gnome-terminal`, `konsole`, `alacritty`, `kitty`, `wezterm`) recebem a mesma tolerância do Terminal no macOS.

**Screenshots no Windows.** O recorte usa o retângulo estendido da janela (DWM), então a sombra não entra;
janelas em outro monitor com escala diferente podem sair com bordas de alguns pixels.

## 5. Problemas comuns

- **"Nada em foco" o dia inteiro no Linux**: `echo $DISPLAY` vazio significa sessão Wayland sem XWayland.
  Entre com "GNOME on Xorg" / "Plasma (X11)" na tela de login.
- **Alguns apps nunca aparecem na Timeline (Linux)**: `echo $XDG_SESSION_TYPE` mostra `wayland`; esses apps
  são nativos Wayland e o rastreador só vê os que rodam pelo XWayland. Ou entre numa sessão X11, ou force o
  X11 no app (`MOZ_ENABLE_WAYLAND=0` para o Firefox, `--ozone-platform=x11` para Chrome/Electron,
  `GDK_BACKEND=x11` para apps GTK).
- **Sem ícone na bandeja (GNOME)**: instale a extensão *AppIndicator and KStatusNotifierItem Support*.
- **Chave de API "não configurada" depois de reiniciar (Linux)**: sem um Secret Service ativo (gnome-keyring,
  KWallet com a ponte, KeePassXC), o app grava a chave num arquivo `secrets.json` (modo `0600`) na pasta de
  dados. Se a pasta foi apagada, cole a chave de novo em Configurações.
- **SmartScreen bloqueia o instalador**: *Mais informações* → *Executar assim mesmo*. O `.msi` passa pelo
  mesmo aviso.
- **Sites bloqueados não fecham no Linux**: a URL vem do título; abra o site com a aba em foco e verifique
  na Timeline se o domínio aparece. Se não aparecer, bloqueie o app.
- **Compilação no Windows reclama de `link.exe`**: instale o Visual Studio Build Tools com a carga de
  trabalho *Desktop development with C++* e abra um terminal novo.
