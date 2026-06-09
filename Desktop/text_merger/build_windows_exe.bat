@echo off
setlocal
cd /d "%~dp0"
python tools\build_exe.py
exit /b %ERRORLEVEL%
