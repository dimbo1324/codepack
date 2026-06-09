# Project Exporter Desktop

Windows 11 desktop utility for preparing safe AI-ready project export bundles.

## Run from source

```bat
py -3.11 -m venv .venv
.venv\Scripts\activate
python -m pip install --upgrade pip
pip install -r requirements.txt
python main.py
```

## Tests

```bat
python -m pytest
```

## Build EXE

```bat
python tools\build_exe.py
```

Output: `dist\ProjectExporterDesktop.exe`.

## Build Setup.exe

Install Inno Setup, then run:

```bat
tools\build_setup.bat
```

The installer script is `build\installer\ProjectExporterDesktop.iss`.
