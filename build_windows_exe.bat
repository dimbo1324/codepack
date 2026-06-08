@echo off
setlocal
cd /d "%~dp0"
python -m pip install --upgrade pip
python -m pip install pyinstaller
python build_windows_exe.py
if errorlevel 1 (
  echo Build failed.
  exit /b 1
)
echo.
echo Done. EXE should be in the dist folder.
endlocal
