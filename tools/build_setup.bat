@echo off
REM Inno Setup wrapper; expects the PyInstaller executable to exist first.
setlocal
cd /d "%~dp0\.."
if not exist "dist\ProjectExporterDesktop.exe" (
  echo dist\ProjectExporterDesktop.exe was not found.
  echo Run tools\build_exe.bat first.
  exit /b 1
)
set "ISCC_EXE="
if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" set "ISCC_EXE=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if not defined ISCC_EXE if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" set "ISCC_EXE=%ProgramFiles%\Inno Setup 6\ISCC.exe"
if not defined ISCC_EXE if exist "%LocalAppData%\Programs\Inno Setup 6\ISCC.exe" set "ISCC_EXE=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"
if defined ISCC_EXE (
  "%ISCC_EXE%" "build\installer\ProjectExporterDesktop.iss"
  exit /b %ERRORLEVEL%
)
iscc "build\installer\ProjectExporterDesktop.iss"
exit /b %ERRORLEVEL%
