$ErrorActionPreference = "Stop"
$dest = Join-Path $env:LOCALAPPDATA "Programs\Still"
if (Test-Path $dest) {
    Remove-Item $dest -Recurse -Force
    Write-Host "removed $dest"
} else {
    Write-Host "nothing to remove at $dest"
}
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath) {
    $parts = $userPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $dest }
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
    Write-Host "removed Still from your user PATH"
}
