$ErrorActionPreference = "Stop"
$src = $PSScriptRoot
if (-not $src) { $src = Get-Location }
if (-not (Test-Path (Join-Path $src "still.exe"))) {
    Write-Host "still.exe was not found next to install.ps1"
    Write-Host "extract the whole zip first, then run install.ps1 from the extracted folder"
    exit 1
}
$dest = Join-Path $env:LOCALAPPDATA "Programs\Still"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item (Join-Path $src "still.exe") (Join-Path $dest "still.exe") -Force
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
if (($userPath -split ";") -notcontains $dest) {
    if ($userPath -eq "") {
        [Environment]::SetEnvironmentVariable("Path", $dest, "User")
    } else {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$dest", "User")
    }
    Write-Host "added $dest to your user PATH"
}
$env:Path += ";$dest"
$ver = & (Join-Path $dest "still.exe") version
Write-Host "installed: $ver"
Write-Host ""
Write-Host "open a NEW terminal and try:"
Write-Host "  still version"
Write-Host "  still run examples\fizzbuzz.strc"
Write-Host ""
Write-Host "still needs a C compiler: Visual Studio Build Tools with"
Write-Host "'Desktop development with C++' - see SETUP-WINDOWS.md"
Write-Host "to uninstall later: powershell -ExecutionPolicy Bypass -File .\uninstall.ps1"
