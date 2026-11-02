; CalcDex installer — per-user, no admin needed.
; The release workflow passes /DAppVersion=<version>; the
; default below is the current version for hand builds.
!ifndef AppVersion
  !define AppVersion "0.9.0"
!endif
Unicode true
Name "CalcDex"
OutFile "..\dist\CalcDex-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\CalcDex"
RequestExecutionLevel user
Icon "..\assets\icon.ico"
UninstallIcon "..\assets\icon.ico"
SetCompressor /SOLID lzma

Page directory
Page instfiles

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\dist\CalcDex.exe"
  File "..\LICENSE"
  File "..\README.md"
  CreateDirectory "$SMPROGRAMS\CalcDex"
  CreateShortcut "$SMPROGRAMS\CalcDex\CalcDex.lnk" "$INSTDIR\CalcDex.exe"
  CreateShortcut "$DESKTOP\CalcDex.lnk" "$INSTDIR\CalcDex.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CalcDex" "DisplayName" "CalcDex"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CalcDex" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CalcDex" "DisplayVersion" "${AppVersion}"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\CalcDex.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\CalcDex\CalcDex.lnk"
  RMDir "$SMPROGRAMS\CalcDex"
  Delete "$DESKTOP\CalcDex.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\CalcDex"
SectionEnd
