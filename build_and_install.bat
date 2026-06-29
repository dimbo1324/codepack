@echo off
REM ============================================================
REM  build_and_install.bat
REM  Full build pipeline:
REM    1. Compile Python app -> single EXE  (PyInstaller)
REM    2. Package EXE -> Windows installer  (Inno Setup)
REM    3. Launch the generated installer wizard
REM
REM  Inno Setup 6 is installed automatically from tools\vendor\
REM  if it is not already present on this machine.
REM ============================================================
setlocal
cd /d "%~dp0"

echo.
echo ============================================================
echo   Project Exporter Desktop  ^|  Build ^& Install
echo ============================================================
echo.

:: ── Step 1: Build PyInstaller executable ─────────────────────────────────────
echo [1/2] Compiling executable (PyInstaller)...
echo ─────────────────────────────────────────────────────────────
call tools\build_exe.bat
if errorlevel 1 (
  echo.
  echo [FAIL] Executable build failed. See output above for details.
  exit /b 1
)
echo.
echo [OK]  Executable ready: dist\ProjectExporterDesktop.exe
echo.

:: ── Step 2: Build Inno Setup installer ───────────────────────────────────────
echo [2/2] Building Windows installer (Inno Setup)...
echo ─────────────────────────────────────────────────────────────
call tools\build_setup.bat
if errorlevel 1 (
  echo.
  echo [FAIL] Installer build failed. See output above for details.
  exit /b 1
)
echo.

:: ── Step 3: Locate and launch the installer ───────────────────────────────────
set "SETUP_EXE="
for /f "delims=" %%F in ('dir /b /o-d "dist\installer\ProjectExporterDesktopSetup-*.exe" 2^>nul') do (
  if not defined SETUP_EXE set "SETUP_EXE=dist\installer\%%F"
)

if not defined SETUP_EXE (
  echo [ERROR] Installer not found in dist\installer\
  exit /b 1
)

echo ============================================================
echo   Build complete!
echo   Installer: %SETUP_EXE%
echo ============================================================
echo.
echo Launching installer wizard...
start "" "%SETUP_EXE%"
exit /b 0
