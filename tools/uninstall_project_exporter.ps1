# Full local uninstall helper for Project Exporter Desktop.

$ErrorActionPreference = "Stop"

$appName = "Project Exporter Desktop"
$appId = "{B783D1BA-B1D1-4E91-8E71-BC6AC05B5C4D}_is1"
$uninstallRoots = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"
)

Write-Host ""
Write-Host "WARNING: this script will fully remove $appName from this computer." -ForegroundColor Yellow
Write-Host "It will run the application uninstaller, remove the installation folder, remove shortcuts,"
Write-Host "remove Control Panel uninstall entries if they remain, and delete local user settings."
Write-Host ""
$answer = Read-Host "Type DELETE to continue"
if ($answer -ne "DELETE") {
    Write-Host "Removal cancelled."
    exit 0
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

function Resolve-UninstallerPath([string]$uninstallString, [string]$installLocation) {
    if ($uninstallString) {
        $trimmed = $uninstallString.Trim()
        if ($trimmed.StartsWith('"')) {
            $end = $trimmed.IndexOf('"', 1)
            if ($end -gt 1) {
                $candidate = $trimmed.Substring(1, $end - 1)
                if (Test-Path $candidate) {
                    return $candidate
                }
            }
        } else {
            $candidate = ($trimmed -split "\s+", 2)[0]
            if (Test-Path $candidate) {
                return $candidate
            }
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

function Remove-InstallDirectory([string]$installLocation) {
    if (-not $installLocation -or -not (Test-Path $installLocation)) {
        return
    }
    $resolved = (Resolve-Path $installLocation).Path
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
$installLocations = @()

foreach ($entry in $entries) {
    if ($entry.InstallLocation) {
        $installLocations += $entry.InstallLocation
    }
    $uninstaller = Resolve-UninstallerPath $entry.UninstallString $entry.InstallLocation
    if ($uninstaller) {
        Write-Host "Running uninstaller: $uninstaller"
        $process = Start-Process -FilePath $uninstaller -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Write-Warning "Uninstaller exited with code $($process.ExitCode). Continuing cleanup."
        }
    }
}

$defaultInstall = Join-Path $env:LOCALAPPDATA "Programs\Project Exporter Desktop"
$installLocations += $defaultInstall
$installLocations | Sort-Object -Unique | ForEach-Object { Remove-InstallDirectory $_ }

$shortcutPaths = @(
    (Join-Path ([Environment]::GetFolderPath("Desktop")) "$appName.lnk"),
    (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$appName")
)
foreach ($path in $shortcutPaths) {
    if (Test-Path $path) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$settingsFiles = @(
    (Join-Path $HOME ".project_exporter_desktop.json"),
    (Join-Path $HOME ".project_exporter_profiles.json")
)
foreach ($path in $settingsFiles) {
    if (Test-Path $path) {
        Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    }
}

foreach ($entry in Get-InstallEntries) {
    Remove-Item -LiteralPath $entry.KeyPath -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "$appName removal completed." -ForegroundColor Green
