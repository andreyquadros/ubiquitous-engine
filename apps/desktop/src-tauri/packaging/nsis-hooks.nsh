; Installer hooks of the ubiqX setup (bundle.windows.nsis.installerHooks).
;
; Before the files go, the app's own executable takes off what this product applied to
; the system (the hosts block, the family resolver, the browser policies), keeping what
; the other product on this machine still wants. The installer is per-user and not
; elevated, so the executable is started with the "runas" verb: Windows shows one UAC
; prompt. If the user declines, the uninstall goes on and the system keeps what was
; applied; REMOVAL.md of the source tree has the steps to take it off.
;
; Skipped when the uninstaller runs as part of an update ($UpdateMode): an update must
; not take the protection off.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    DetailPrint "Removing the system changes of this app (an administrator prompt follows)"
    ExecShellWait "runas" "$INSTDIR\${MAINBINARYNAME}.exe" "--shield-helper remove --owner ubiqx --scope all" SW_HIDE
  ${EndIf}
!macroend
