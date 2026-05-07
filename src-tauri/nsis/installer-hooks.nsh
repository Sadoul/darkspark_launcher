; ============================================================
; DarkSpark Launcher - Custom NSIS Installer Hooks
; ============================================================
!macro NSIS_HOOK_PREINSTALL
  ; Silent uninstall of previous versions
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\com.darkspark.launcher_is1" "UninstallString"
  ${If} $R0 != ""
    ExecWait '"$R0" /S _?=$INSTDIR'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Launch DarkSpark Launcher if already installed, otherwise first-install is done
  IfFileExists "$INSTDIR\darkspark-launcher.exe" 0 done
    ExecShell "open" "$INSTDIR\darkspark-launcher.exe"
  done:
!macroend
