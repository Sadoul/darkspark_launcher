; ============================================================
; DarkSpark Launcher - Custom NSIS Installer Hooks
; ============================================================
!macro NSIS_HOOK_PREINSTALL
  ; Silent uninstall of previous versions
  ReadRegStr $R0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\DarkSpark Launcher" "UninstallString"
  ${If} $R0 != ""
    ExecWait '"$R0" /S _?=$INSTDIR'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Always launch the launcher after install/upgrade
  IfFileExists "$INSTDIR\darkspark-launcher.exe" 0 +3
    ExecShell "open" "$INSTDIR\darkspark-launcher.exe"
    Goto done
  done:
!macroend
