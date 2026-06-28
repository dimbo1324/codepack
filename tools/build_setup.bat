@echo off
REM Inno Setup wrapper; expects the PyInstaller executable to exist first.
setlocal
cd /d "%~dp0\.."
if not exist "dist\ProjectExporterDesktop.exe" (
  echo dist\ProjectExporterDesktop.exe was not found.
  echo Run tools\build_exe.bat first.
  exit /b 1
)
iscc "build\installer\ProjectExporterDesktop.iss"
exit /b %ERRORLEVEL%
