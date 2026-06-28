@echo off
REM Launches the full Project Exporter Desktop removal script with an explicit warning prompt.
setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\uninstall_project_exporter.ps1"
exit /b %ERRORLEVEL%
