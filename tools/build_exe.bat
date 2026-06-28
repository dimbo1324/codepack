@echo off
REM Build helper used from tools/; delegates to the Python PyInstaller wrapper.
setlocal
cd /d "%~dp0\.."
python tools\build_exe.py
exit /b %ERRORLEVEL%
