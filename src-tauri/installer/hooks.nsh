; Vaal Street additions to Tauri's installer.

; Windows remembers program icons by path: after an update that changed the
; icon, shortcuts would keep the old one. Ask it to forget and redraw.
!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$SYSDIR\ie4uinit.exe" -ClearIconCache'
  ExecWait '"$SYSDIR\ie4uinit.exe" -show'
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
!macroend

; Tauri already removes the startup entry ("Vaal Street" in the Run list).
; When "delete the application data" is ticked, also remove Vaal Street's
; folder (settings, wealth history, runs, cache).
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RMDir /r "$APPDATA\VaalStreet"
  ${EndIf}
!macroend
