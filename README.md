# Project Exporter Desktop

Project Exporter Desktop is a Windows desktop application for preparing clean, safe, AI-ready snapshots of software projects. It helps you package source code, reports, project structure, Git context, dependency summaries, security notes, and ready-to-paste AI context without manually collecting files.

The application is designed for people who want to send a project to ChatGPT, Claude, Codex, or another AI assistant for review, debugging, onboarding, refactoring, or security analysis.

## Who This App Is For

Use Project Exporter Desktop if you want to:

- Copy a project into an AI chat without accidentally including heavy folders like `node_modules`.
- Create a clean ZIP archive of a project for review.
- Generate a text dump of selected project files.
- Estimate token size before pasting code into an AI assistant.
- Export only changed files instead of the whole project.
- Review project analytics such as language breakdown, LOC, dependencies, Git commits, and security risks.
- Keep a project watched and update clipboard exports when files change.
- Use presets for ChatGPT, Claude Code, code review, security audit, and onboarding workflows.

You do not need to know programming to install and run the released Windows application.

## Supported Operating System

The packaged application is built for:

- Windows 11, 64-bit
- Windows 10, 64-bit should also work, but Windows 11 is the primary tested target

The developer build scripts are intended for Windows PowerShell.

## Main Features

- Russian and English interface.
- Safe project export with secret-aware filtering.
- Automatic stack detection for Node, Python, Go, Rust, Java, .NET, Flutter, and other common project types.
- File preview page with included, excluded, and warning states.
- Token estimation for large-language-model workflows.
- Clipboard export for quick paste into AI chats.
- Developer context field that is placed at the top of the AI context.
- AI presets for common workflows.
- Differential export modes:
  - full export
  - since last export
  - since Git reference
  - only uncommitted changes
- Local analytics page without network requests.
- System tray support with quick export.
- Watch mode for project changes.
- Export history and snapshot comparison.
- Light, dark, and system themes.
- Installer and standalone EXE builds.

## Download and Install for Normal Users

The easiest way to install the application is to use the GitHub Release installer.

1. Open the release page:
   - https://github.com/dimbo1324/codepack/releases/tag/v1.0.0

2. Download:
   - `ProjectExporterDesktopSetup-1.0.0.exe`

3. Double-click the downloaded installer.

4. Follow the setup wizard.

5. When the installer asks for a destination folder, choose where you want the application to be installed.

6. Finish the installation.

7. Start the application from the Start menu shortcut:
   - `Project Exporter Desktop`

If Windows SmartScreen warns you about an unknown publisher, this usually means the EXE is not code-signed. Choose the additional information option and continue only if you downloaded the file from the official repository release page.

## Standalone EXE

The release also includes:

- `ProjectExporterDesktop.exe`

This is a standalone executable. You can run it directly without using the installer.

Use the installer if you want Start menu shortcuts and standard Windows uninstall integration. Use the standalone EXE if you want a portable-style run.

## Verify Downloaded Files

The release includes:

- `SHA256SUMS.txt`

To verify a downloaded file in PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 .\ProjectExporterDesktopSetup-1.0.0.exe
```

Compare the printed hash with the matching line in `SHA256SUMS.txt`.

## First Launch

When you open the app, you will see a wizard-like interface.

The main workflow is:

1. Select a project folder.
2. Choose export settings.
3. Choose security settings.
4. Review the preview page.
5. Run the export.
6. Open the result or copy the dump to clipboard.

The original project folder is not modified during export.

## Basic Usage

### 1. Select a Project

On the Project page, choose the root folder of the project you want to export.

Example:

```text
C:\Users\You\Projects\my-app
```

You can also write context for the AI assistant. For example:

```text
I want you to review this project and find architecture problems, security risks, and possible simplifications.
```

This context is added at the top of the generated AI file.

### 2. Choose Settings

The Settings page lets you choose:

- export profile
- AI preset
- text file size limit
- ZIP part size
- theme
- watch mode
- differential export mode

If you are not sure what to choose, start with a preset such as:

- ChatGPT
- Claude Code
- Code Review
- Security Audit
- Onboarding

### 3. Configure Security

The Security page controls how strict the export should be.

Recommended default:

- Safe mode enabled
- Secret redaction enabled
- Project files included only after previewing the export

The app is designed to avoid common dangerous files such as:

- `.env`
- private keys
- credentials files
- local databases
- archives
- build artifacts
- dependency folders

Always review the preview before sharing a project with an external service.

### 4. Preview Files

The Preview page shows the file tree before export.

Files are marked by state:

- included
- excluded
- warning

Use this page to check that large or sensitive files are not included.

### 5. Copy Dump to Clipboard

Use the clipboard export button when you want to paste the project directly into an AI chat.

This is useful for:

- quick code review
- asking an AI to explain the project
- sharing only text files
- avoiding ZIP uploads

### 6. Run Export

When you run the export, the app creates an export folder and usually a ZIP archive.

The result contains:

- copied project files
- structure report
- Git report
- text dump
- metadata manifest
- analytics reports
- AI context files
- security scan output

## Differential Export

Differential export helps avoid sending the full project every time.

Available modes:

- Full export
- Since last export
- Since Git reference
- Only uncommitted changes

Use differential export when you are working iteratively with an AI assistant and only want to send what changed.

## Analytics Page

The Analytics page works locally and does not send data over the network.

It can show:

- detected stack
- language distribution
- lines of code
- dependencies
- dependency warnings
- recent Git commits
- security risks

## System Tray

When the app is minimized or closed, it can stay available in the Windows system tray.

The tray menu includes:

- quick export
- open
- exit

If the tray icon is inside the Windows overflow panel, click the small arrow near the clock to find it.

## Zoom

You can change the UI zoom from the View menu or with keyboard shortcuts.

Shortcuts:

```text
Ctrl++      Zoom in
Ctrl+=      Zoom in
Ctrl+-      Zoom out
Ctrl+_      Zoom out on some keyboard layouts
Ctrl+0      Reset zoom
```

The selected zoom level is saved in the app settings.

## Change Language

Use:

```text
View -> Switch to Russian
```

or, in Russian:

```text
Вид -> Переключить на английский
```

The interface updates without restarting the app.

## Build From Source

These steps are for developers or advanced users.

### Requirements

Install:

- Python 3.11 or newer
- Windows 10/11 64-bit
- Inno Setup 6, if you want to build the installer

The current release was built with Python 3.14 and PyInstaller 6.20, but the project configuration requires Python 3.11 or newer.

### Install Dependencies

Open PowerShell in the repository root:

```powershell
cd "C:\Users\dim4d\Desktop\From git\home-scripts"
```

Install runtime and development dependencies:

```powershell
python -m pip install -r requirements.txt
python -m pip install -e ".[dev]"
```

### Run the Application From Source

```powershell
python main.py
```

or:

```powershell
python -m project_exporter_desktop
```

## Run Tests

Run the full local test gate:

```powershell
python -m ruff format --check main.py build_windows_exe.py tools\build_exe.py tools\smoke_export.py src tests
python -m ruff check src tests tools main.py build_windows_exe.py
python -m pytest -p no:cacheprovider
python -m compileall -q src tests tools main.py build_windows_exe.py
python tools\smoke_export.py
```

Expected result:

```text
All checks passed
pytest passed
Smoke result: PASS
```

On Windows, pytest may occasionally print a temporary-folder cleanup warning at process exit. If pytest reports that all tests passed and the command exits successfully, the test run is considered successful.

## Build the EXE

To build the standalone executable:

```powershell
python tools\build_exe.py
```

Output:

```text
dist\ProjectExporterDesktop.exe
```

## Build the Installer

Install Inno Setup 6 first.

Then run:

```powershell
tools\build_setup.bat
```

Output:

```text
dist\installer\ProjectExporterDesktopSetup-1.0.0.exe
```

## Build and Start Installer With One Script

Use:

```powershell
.\build_and_install.bat
```

This script:

1. Builds the EXE.
2. Builds the installer.
3. Starts the interactive setup wizard.

The setup wizard lets you choose the installation directory.

## Uninstall

### Normal Windows Uninstall

Use Windows Settings:

1. Open Settings.
2. Go to Apps.
3. Find `Project Exporter Desktop`.
4. Click Uninstall.

You can also uninstall from Control Panel if the app is listed there.

### Full Cleanup Script

The repository includes a full cleanup helper:

```powershell
.\uninstall_project_exporter.bat
```

The script asks:

```text
Continue with full removal? [Y/n]
```

Press Enter or type `Y` to continue. Type `N` to cancel.

The script attempts to:

- close the running app
- run the official uninstaller
- remove the installation folder
- remove Start menu shortcuts
- remove desktop shortcuts
- remove pinned taskbar shortcut files
- remove remaining uninstall registry entries
- remove local app settings
- remove local app logs

Use this script only when you want a complete local cleanup.

## Where Settings Are Stored

User settings are stored in the user profile:

```text
%USERPROFILE%\.project_exporter_desktop.json
%USERPROFILE%\.project_exporter_profiles.json
```

Logs are stored under:

```text
%LOCALAPPDATA%\ProjectExporterDesktop\logs
```

## Generated Output

The app can generate:

- ZIP archives
- copied staging folders
- text reports
- JSON metadata
- SARIF security output
- AI context folders
- clipboard dumps

Generated export output is not intended to be committed to Git.

## Safety Notes

Project Exporter Desktop includes safety filters, but no automated tool can guarantee perfect secret detection.

Before sharing an export:

1. Use Safe mode.
2. Keep secret redaction enabled.
3. Review the Preview page.
4. Open the generated archive if the data is sensitive.
5. Do not upload private credentials, keys, tokens, or production data.

## Troubleshooting

### The app does not start

Try reinstalling with the latest setup file from the release page.

If you are running from source, reinstall dependencies:

```powershell
python -m pip install -r requirements.txt
python -m pip install -e ".[dev]"
```

### The tray icon is still visible

If Windows still shows the tray icon after uninstall, move the mouse over the tray area. Windows sometimes keeps stale tray icons until the area repaints.

If the app is actually still running, use the full cleanup script:

```powershell
.\uninstall_project_exporter.bat
```

### Zoom does not work

Use:

```text
Ctrl++
Ctrl+=
Ctrl+-
Ctrl+_
Ctrl+0
```

You can also use the View menu.

### The installer cannot be built

Make sure Inno Setup 6 is installed.

If `iscc` is not in PATH, the build script also checks common install locations:

```text
C:\Program Files (x86)\Inno Setup 6
C:\Program Files\Inno Setup 6
%LOCALAPPDATA%\Programs\Inno Setup 6
```

### PyInstaller build fails

Reinstall dependencies:

```powershell
python -m pip install -r requirements.txt
python -m pip install -e ".[dev]"
```

Then rebuild:

```powershell
python tools\build_exe.py
```

## Repository Structure

Important folders and files:

```text
src\project_exporter_desktop      Application source code
tests                             Automated tests
tools                             Build, smoke, and uninstall helper scripts
build\pyinstaller                 PyInstaller spec
build\installer                   Inno Setup script
assets                            Application icon
dist                              Local build output
```

## Release Artifacts

The v1.0.0 release publishes:

```text
ProjectExporterDesktop.exe
ProjectExporterDesktopSetup-1.0.0.exe
SHA256SUMS.txt
```

## License

No explicit license file is currently included. Treat the project as all rights reserved unless the repository owner adds a license.
