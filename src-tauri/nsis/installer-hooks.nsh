; ============================================================
; DarkSpark Launcher - Custom NSIS Installer Hooks
; ============================================================
!macro NSIS_HOOK_PREINSTALL
  ; Silent uninstall of previous versions
  ReadRegStr $R0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\com.darkspark.launcher_is1" "UninstallString"
  ${If} $R0 != ""
    ExecWait '"$R0" /S _?=$INSTDIR'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Always launch the launcher after install/upgrade
  IfFileExists "$INSTDIR\darkspark-launcher.exe" 0 done
    ExecShell "open" "$INSTDIR\darkspark-launcher.exe"
  done:
!macroend