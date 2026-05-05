; ============================================================
; RPWorld Launcher - Custom NSIS Installer Hooks
; ============================================================
!macro NSIS_HOOK_PREINSTALL
  ; Still silent uninstall
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\com.rpworld.launcher_is1" "UninstallString"
  ${If} $R0 != ""
    ExecWait '"$R0" /S _?=$INSTDIR'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; --- Launch RPWorld Launcher exactly once after successful install ---
  ; Duplicate windows are prevented by the app-level single-instance mutex.
  IfFileExists "$INSTDIR\rpw-launcher.exe" 0 +3
    ExecShell "open" "$INSTDIR\rpw-launcher.exe"
    Goto done
  IfFileExists "$INSTDIR\RPWorld Launcher.exe" 0 done
    ExecShell "open" "$INSTDIR\RPWorld Launcher.exe"
  done:
!macroend
