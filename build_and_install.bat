@echo off
REM Builds the executable, builds the installer, and launches the interactive installer wizard.
setlocal
cd /d "%~dp0"

echo Building Project Exporter Desktop executable...
call tools\build_exe.bat
if errorlevel 1 (
  echo Executable build failed.
  exit /b 1
)

echo Building Project Exporter Desktop installer...
call tools\build_setup.bat
if errorlevel 1 (
  echo Installer build failed.
  exit /b 1
)

set "SETUP_EXE="
for /f "delims=" %%F in ('dir /b /o-d "dist\installer\ProjectExporterDesktopSetup-*.exe" 2^>nul') do (
  if not defined SETUP_EXE set "SETUP_EXE=dist\installer\%%F"
)

if not defined SETUP_EXE (
  echo Installer was not found in dist\installer.
  exit /b 1
)

echo Launching interactive installer. Choose the destination folder in the setup wizard.
start "" "%SETUP_EXE%"
exit /b 0
