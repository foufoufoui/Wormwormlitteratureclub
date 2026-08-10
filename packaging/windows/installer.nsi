; Worm Worm Literature Club - Windows installer.
; Build from the repo root with:
;   makensis -DAPP_VERSION=1.0.0 packaging/windows/installer.nsi
;
; Expects the packaged files (wwlc.exe + assets/) under staging/WWLC/.
; CI passes -DSTAGING_DIR=<absolute path> because `File` paths are resolved
; relative to this script's directory otherwise.

Unicode true
SetCompressor /SOLID lzma

!ifndef APP_VERSION
  !define APP_VERSION "1.0.0"
!endif

!ifndef STAGING_DIR
  !define STAGING_DIR "..\..\staging\WWLC"
!endif

!ifndef OUT_FILE
  !define OUT_FILE "wwlc-installer.exe"
!endif

!define APP_NAME "Worm Worm Literature Club"
!define APP_EXE "wwlc.exe"
!define APP_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\WWLC"

Name "${APP_NAME}"
OutFile "${OUT_FILE}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "${APP_UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${STAGING_DIR}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  WriteRegStr HKLM "${APP_UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${APP_UNINST_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${APP_UNINST_KEY}" "Publisher" "probablyfoui"
  WriteRegStr HKLM "${APP_UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${APP_UNINST_KEY}" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  RMDir /r "$SMPROGRAMS\${APP_NAME}"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKLM "${APP_UNINST_KEY}"
SectionEnd
