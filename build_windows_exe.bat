@echo off
REM Root-level convenience entrypoint for building the Windows executable.
setlocal
cd /d "%~dp0"
python tools\build_exe.py
exit /b %ERRORLEVEL%
