; Azeroth Navigator NSIS hooks (task M9.4).
;
; Included by the Tauri NSIS template (tauri-bundler 2.10.2, pinned through
; @tauri-apps/cli 2.12.2). The template defines PRODUCTNAME, BUNDLEID and
; MAINBINARYNAME, declares $UpdateMode and $NoShortcutMode and includes
; LogicLib before this file. Re-check those names when the CLI is upgraded.
;
; The installer never writes into the game folder on its own. The AddOns
; directory of the Forever client is unverified
; (docs/compatibility/ADDON_INSTALL.md), so the user chooses it in
; addon\Install-AzerothNavigatorAddon.ps1. That script records the AddOns
; folder it wrote to in ANAV_ADDON_MARKER (UTF-16LE, no BOM, no newline).
; These hooks act only on that recorded folder, and only through the same
; script, which touches nothing but its AzerothNavigator subfolder: no other
; addon, SavedVariables file or client file.

!define ANAV_ADDON_SHORTCUT "$SMPROGRAMS\${PRODUCTNAME} - WoW addon.lnk"
!define ANAV_ADDON_MARKER "$LOCALAPPDATA\${BUNDLEID}\addon-location.txt"
!define ANAV_ADDON_SCRIPT "$INSTDIR\addon\Install-AzerothNavigatorAddon.ps1"
; The shortcut is opened by Explorer (64-bit), so System32 is the real one.
!define ANAV_SHORTCUT_POWERSHELL "$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe"

; The installer itself is a 32-bit process without file system redirection
; disabled: System32 would start the 32-bit PowerShell. Sysnative reaches the
; 64-bit one on 64-bit Windows.
!macro ANAV_POWERSHELL_PATH out
  ${If} ${RunningX64}
    StrCpy ${out} "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  ${Else}
    StrCpy ${out} "$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe"
  ${EndIf}
!macroend

; Reads the recorded AddOns folder into $R2 ("" when absent or unreadable,
; or when the folder no longer holds this addon).
!macro ANAV_READ_ADDON_MARKER
  StrCpy $R2 ""
  ${If} ${FileExists} "${ANAV_ADDON_MARKER}"
    ClearErrors
    FileOpen $R1 "${ANAV_ADDON_MARKER}" r
    ${IfNot} ${Errors}
      FileReadUTF16LE $R1 $R2 1024
      FileClose $R1
    ${EndIf}
    ${IfNot} ${FileExists} "$R2\AzerothNavigator\AzerothNavigator.toc"
      StrCpy $R2 ""
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $R1
  Push $R2
  Push $R3
  Push $R4
  !insertmacro ANAV_POWERSHELL_PATH $R4

  ${If} $NoShortcutMode <> 1
    CreateShortcut "${ANAV_ADDON_SHORTCUT}" "${ANAV_SHORTCUT_POWERSHELL}" \
      '-NoProfile -ExecutionPolicy Bypass -File "${ANAV_ADDON_SCRIPT}"' \
      "$INSTDIR\${MAINBINARYNAME}.exe" 0
  ${EndIf}

  !insertmacro ANAV_READ_ADDON_MARKER
  ${If} $R2 != ""
    ; Keep the in-game addon at the same version as the application.
    MessageBox MB_YESNO|MB_ICONQUESTION \
      "Update the Azeroth Navigator addon in$\r$\n$R2\AzerothNavigator$\r$\nto this version?$\r$\n$\r$\nClose the game first. Other addons are not touched." \
      /SD IDNO IDNO anav_skip_addon_update
      DetailPrint "Updating the addon in $R2"
      nsExec::ExecToLog '"$R4" -NoProfile -ExecutionPolicy Bypass -File "${ANAV_ADDON_SCRIPT}" -Action Install -AddOnsPath "$R2" -Quiet'
      Pop $R3
      ${If} $R3 != 0
        MessageBox MB_OK|MB_ICONEXCLAMATION \
          "The addon was not updated (result: $R3). The previous addon files were kept.$\r$\nUse Start menu > ${PRODUCTNAME} - WoW addon to retry." \
          /SD IDOK
      ${EndIf}
    anav_skip_addon_update:
  ${ElseIf} $UpdateMode <> 1
    MessageBox MB_YESNO|MB_ICONQUESTION \
      "Install the World of Warcraft addon now?$\r$\n$\r$\nYou will be asked for the game's Interface\AddOns folder. You can also do this later from Start menu > ${PRODUCTNAME} - WoW addon." \
      /SD IDNO IDNO anav_skip_addon_install
      Exec '"$R4" -NoProfile -ExecutionPolicy Bypass -File "${ANAV_ADDON_SCRIPT}"'
    anav_skip_addon_install:
  ${EndIf}

  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $R1
  Push $R2
  Push $R3
  Push $R4

  ${If} $UpdateMode <> 1
    Delete "${ANAV_ADDON_SHORTCUT}"

    !insertmacro ANAV_READ_ADDON_MARKER
    ${If} $R2 != ""
    ${AndIf} ${FileExists} "${ANAV_ADDON_SCRIPT}"
      MessageBox MB_YESNO|MB_ICONQUESTION \
        "Also remove the Azeroth Navigator addon from$\r$\n$R2\AzerothNavigator ?$\r$\n$\r$\nClose the game first. Other addons, SavedVariables and game settings are not touched." \
        /SD IDNO IDNO anav_keep_addon
        ; The helper applies the same checks as a manual removal (title, running game)
        ; and forgets the recorded folder.
        !insertmacro ANAV_POWERSHELL_PATH $R4
        nsExec::ExecToLog '"$R4" -NoProfile -ExecutionPolicy Bypass -File "${ANAV_ADDON_SCRIPT}" -Action Uninstall -AddOnsPath "$R2" -Quiet'
        Pop $R3
        ${If} $R3 != 0
          MessageBox MB_OK|MB_ICONEXCLAMATION \
            "The addon was not removed (result: $R3).$\r$\nDelete $R2\AzerothNavigator by hand if you no longer need it." \
            /SD IDOK
        ${EndIf}
      anav_keep_addon:
    ${EndIf}
  ${EndIf}

  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
!macroend
