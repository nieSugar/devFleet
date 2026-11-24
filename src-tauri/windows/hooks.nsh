!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --install-terminal-path' $0
  ${If} ${Errors}
  ${OrIf} $0 = 1
    DetailPrint "Could not add DevFleet to PATH."
  ${ElseIf} $0 = 0
    WriteRegStr HKCU "${MANUPRODUCTKEY}" "TerminalPath" "$INSTDIR"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $0 HKCU "${MANUPRODUCTKEY}" "TerminalPath"
  ${If} $UpdateMode <> 1
  ${AndIf} $0 == $INSTDIR
    ; Check before changing PATH because the stock check can cancel uninstall.
    !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    ClearErrors
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall-terminal-path' $0
    ${If} ${Errors}
    ${OrIf} $0 <> 0
      DetailPrint "Could not remove DevFleet from PATH."
    ${Else}
      DeleteRegValue HKCU "${MANUPRODUCTKEY}" "TerminalPath"
    ${EndIf}
  ${EndIf}
!macroend
