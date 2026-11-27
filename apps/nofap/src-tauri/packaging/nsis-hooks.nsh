; Installer hooks of the standalone setup (bundle.windows.nsis.installerHooks).
;
; Before the files go, the app's own executable takes off what this product applied to
; the system (the hosts block, the family resolver, the browser policies), keeping what
; the other product on this machine still wants. The installer is per-user and not
; elevated, so the executable is started with the "runas" verb: Windows shows one UAC
; prompt. If the user declines, the uninstall goes on and the system keeps what was
; applied; REMOVAL.md of the source tree has the steps to take it off.
;
; Skipped whenever the uninstall is part of installing this app again, because an update
; must not take the protection off:
; - the in-app update starts the uninstaller with /UPDATE ($UpdateMode = 1);
; - a newer (or the same, or an older) -setup.exe run by hand shows the reinstall page,
;   whose default choice runs the installed uninstaller without /UPDATE but in place, from
;   the install folder, with "_?=<install folder>". A real uninstall (Windows Settings,
;   Control Panel, or uninstall.exe opened by hand) never runs in place: NSIS first copies
;   the uninstaller to a temporary folder and starts the copy from there. So an uninstaller
;   whose own folder ($EXEDIR) is the install folder, or that still carries its installed
;   name, was started by a setup that goes on to install the app again, and it keeps the
;   system changes and the state file that records them.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode = 1
  ${OrIf} $EXEDIR == $INSTDIR
  ${OrIf} $EXEFILE == "uninstall.exe"
    DetailPrint "This app is being installed again: its system changes stay in place"
  ${ElseIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    DetailPrint "Removing the system changes of this app (an administrator prompt follows)"
    ExecShellWait "runas" "$INSTDIR\${MAINBINARYNAME}.exe" "--shield-helper remove --owner shield --scope all" SW_HIDE
  ${EndIf}
!macroend
