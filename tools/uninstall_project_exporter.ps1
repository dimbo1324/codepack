# Full local uninstall helper for Project Exporter Desktop.

$ErrorActionPreference = "Stop"

$appName = "Project Exporter Desktop"
$appId = "{B783D1BA-B1D1-4E91-8E71-BC6AC05B5C4D}_is1"
$processName = "ProjectExporterDesktop"
$scriptRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$uninstallRoots = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"
)

Write-Host ""
Write-Host "WARNING: this script will fully remove $appName from this computer." -ForegroundColor Yellow
Write-Host "It will close the running app, run the application uninstaller, remove the installation folder,"
Write-Host "remove shortcuts, remove Control Panel uninstall entries if they remain, and delete local user settings."
Write-Host ""
$answer = (Read-Host "Continue with full removal? [Y/n]").Trim()
if ($answer -and $answer -notmatch "^(y|yes)$") {
    Write-Host "Removal cancelled."
    exit 0
}

function Add-UniquePath([System.Collections.ArrayList]$paths, [string]$path) {
    if (-not $path) {
        return
    }
    try {
        $resolved = (Resolve-Path $path -ErrorAction Stop).Path
    } catch {
        $resolved = $path
    }
    if (-not $paths.Contains($resolved)) {
        [void]$paths.Add($resolved)
    }
}

function Get-InstallEntries {
    $entries = @()
    foreach ($root in $uninstallRoots) {
        if (-not (Test-Path $root)) {
            continue
        }
        foreach ($item in Get-ChildItem $root -ErrorAction SilentlyContinue) {
            try {
                $props = Get-ItemProperty $item.PSPath -ErrorAction Stop
            } catch {
                continue
            }
            if ($item.PSChildName -eq $appId -or $props.DisplayName -eq $appName) {
                $entries += [pscustomobject]@{
                    KeyPath = $item.PSPath
                    InstallLocation = [string]$props.InstallLocation
                    UninstallString = [string]$props.UninstallString
                }
            }
        }
    }
    return $entries
}

function Get-ExecutableFromCommand([string]$commandText) {
    if (-not $commandText) {
        return ""
    }
    $trimmed = $commandText.Trim()
    if ($trimmed.StartsWith('"')) {
        $end = $trimmed.IndexOf('"', 1)
        if ($end -gt 1) {
            return $trimmed.Substring(1, $end - 1)
        }
    }
    return ($trimmed -split "\s+", 2)[0]
}

function Resolve-UninstallerPath([string]$uninstallString, [string]$installLocation) {
    if ($uninstallString) {
        $candidate = Get-ExecutableFromCommand $uninstallString
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }
    if ($installLocation) {
        $candidate = Join-Path $installLocation "unins000.exe"
        if (Test-Path $candidate) {
            return $candidate
        }
    }
    return ""
}

function Get-ShortcutInstallLocations {
    $locations = [System.Collections.ArrayList]::new()
    $shortcutRoots = @(
        [Environment]::GetFolderPath("Desktop"),
        (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"),
        (Join-Path $env:APPDATA "Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar")
    )
    try {
        $shell = New-Object -ComObject WScript.Shell
    } catch {
        return $locations
    }
    foreach ($root in $shortcutRoots) {
        if (-not (Test-Path $root)) {
            continue
        }
        foreach ($shortcutFile in Get-ChildItem -Path $root -Filter "*.lnk" -Recurse -ErrorAction SilentlyContinue) {
            try {
                $shortcut = $shell.CreateShortcut($shortcutFile.FullName)
                $target = [string]$shortcut.TargetPath
            } catch {
                continue
            }
            if ($target -and (Split-Path $target -Leaf) -eq "ProjectExporterDesktop.exe") {
                Add-UniquePath $locations (Split-Path $target -Parent)
            }
        }
    }
    return $locations
}

function Stop-RunningApp {
    $locations = [System.Collections.ArrayList]::new()
    $processes = Get-Process -Name $processName -ErrorAction SilentlyContinue
    foreach ($process in $processes) {
        try {
            if ($process.Path) {
                Add-UniquePath $locations (Split-Path $process.Path -Parent)
            }
        } catch {
        }
    }
    if ($processes) {
        Write-Host "Stopping running $appName processes..."
        foreach ($process in $processes) {
            try {
                $process.CloseMainWindow() | Out-Null
            } catch {
            }
        }
        Start-Sleep -Seconds 2
        Get-Process -Name $processName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
    return $locations
}

function Remove-InstallDirectory([string]$installLocation) {
    if (-not $installLocation -or -not (Test-Path $installLocation)) {
        return
    }
    $resolved = (Resolve-Path $installLocation).Path
    if ($resolved -eq $scriptRoot -or $resolved.StartsWith("$scriptRoot\", [System.StringComparison]::OrdinalIgnoreCase)) {
        Write-Warning "Skipped repository path: $resolved"
        return
    }
    $leaf = Split-Path $resolved -Leaf
    $hasAppFiles = (Test-Path (Join-Path $resolved "ProjectExporterDesktop.exe")) -or
        (Test-Path (Join-Path $resolved "unins000.exe"))
    if ($leaf -ne $appName -and -not $hasAppFiles) {
        Write-Warning "Skipped suspicious install directory: $resolved"
        return
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction SilentlyContinue
}

$entries = Get-InstallEntries
$installLocations = [System.Collections.ArrayList]::new()

foreach ($path in (Stop-RunningApp)) {
    Add-UniquePath $installLocations $path
}

foreach ($path in (Get-ShortcutInstallLocations)) {
    Add-UniquePath $installLocations $path
}

foreach ($entry in $entries) {
    if ($entry.InstallLocation) {
        Add-UniquePath $installLocations $entry.InstallLocation
    }
    $uninstaller = Resolve-UninstallerPath $entry.UninstallString $entry.InstallLocation
    if ($uninstaller) {
        Add-UniquePath $installLocations (Split-Path $uninstaller -Parent)
        Write-Host "Running uninstaller: $uninstaller"
        $process = Start-Process -FilePath $uninstaller -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Write-Warning "Uninstaller exited with code $($process.ExitCode). Continuing cleanup."
        }
    }
}

$defaultInstall = Join-Path $env:LOCALAPPDATA "Programs\Project Exporter Desktop"
Add-UniquePath $installLocations $defaultInstall

foreach ($path in (Stop-RunningApp)) {
    Add-UniquePath $installLocations $path
}

$installLocations | Sort-Object -Unique | ForEach-Object { Remove-InstallDirectory $_ }

$shortcutPaths = @(
    (Join-Path ([Environment]::GetFolderPath("Desktop")) "$appName.lnk"),
    (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$appName"),
    (Join-Path $env:APPDATA "Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\$appName.lnk")
)
foreach ($path in $shortcutPaths) {
    if (Test-Path $path) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$settingsFiles = @(
    (Join-Path $HOME ".project_exporter_desktop.json"),
    (Join-Path $HOME ".project_exporter_profiles.json"),
    (Join-Path $env:LOCALAPPDATA "ProjectExporterDesktop")
)
foreach ($path in $settingsFiles) {
    if (Test-Path $path) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

foreach ($entry in Get-InstallEntries) {
    Remove-Item -LiteralPath $entry.KeyPath -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "$appName removal completed." -ForegroundColor Green
