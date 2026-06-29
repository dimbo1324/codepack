@echo off
REM ============================================================
REM  build_setup.bat
REM  Builds the Inno Setup installer for Project Exporter Desktop.
REM  Auto-installs Inno Setup 6 from tools\vendor\ if not found.
REM ============================================================
setlocal
cd /d "%~dp0\.."

:: ── 1. Ensure PyInstaller executable exists ───────────────────────────────────
if not exist "dist\ProjectExporterDesktop.exe" (
  echo [ERROR] dist\ProjectExporterDesktop.exe not found.
  echo         Run tools\build_exe.bat first.
  exit /b 1
)

:: ── 2. Locate ISCC.exe ────────────────────────────────────────────────────────
set "ISCC_EXE="
call :find_iscc

:: ── 3. If not found — auto-install from bundled vendor copy ──────────────────
if not defined ISCC_EXE (
  echo [INFO] Inno Setup not found on this machine.
  call :install_inno
  if errorlevel 1 exit /b 1
  call :find_iscc
)

if not defined ISCC_EXE (
  echo [ERROR] ISCC.exe could not be located after installation.
  echo         Please install Inno Setup 6 manually and re-run.
  exit /b 1
)

:: ── 4. Compile the installer ──────────────────────────────────────────────────
echo [INFO] Inno Setup: %ISCC_EXE%
echo [INFO] Compiling installer script...
"%ISCC_EXE%" "build\installer\ProjectExporterDesktop.iss"
exit /b %ERRORLEVEL%


:: ─────────────────────────────────────────────────────────────────────────────
:find_iscc
  if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
    goto :eof
  )
  if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%ProgramFiles%\Inno Setup 6\ISCC.exe"
    goto :eof
  )
  if exist "%LocalAppData%\Programs\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"
    goto :eof
  )
  for /f "delims=" %%I in ('where iscc 2^>nul') do (
    set "ISCC_EXE=%%I"
    goto :eof
  )
goto :eof


:: ─────────────────────────────────────────────────────────────────────────────
:install_inno
  set "VENDOR_INNO=%~dp0vendor\innosetup-6.7.3.exe"
  if not exist "%VENDOR_INNO%" (
    echo [ERROR] Bundled Inno Setup installer not found: %VENDOR_INNO%
    echo         Add innosetup-6.7.3.exe to tools\vendor\ and retry.
    exit /b 1
  )
  echo [INFO] Installing Inno Setup silently from bundled copy...
  "%VENDOR_INNO%" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
  if errorlevel 1 (
    echo [ERROR] Inno Setup installation failed (exit code %ERRORLEVEL%).
    exit /b 1
  )
  echo [OK]   Inno Setup installed successfully.
goto :eof
