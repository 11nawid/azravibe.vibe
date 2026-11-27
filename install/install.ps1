# Azra one-command installer for Windows.
#   irm https://raw.githubusercontent.com/11nawid/azravibe.vibe/main/install/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "11nawid/azravibe.vibe"
$installDir = Join-Path $env:LOCALAPPDATA "Azra\bin"
$zip = Join-Path $env:TEMP "azra-install.zip"

Write-Host "Finding the latest Azra release..." -ForegroundColor Cyan
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -UseBasicParsing
$assetInfo = $release.assets | Where-Object { $_.name -like "azra-windows-x86_64*.zip" } | Select-Object -First 1
if ($null -eq $assetInfo) {
    throw "No Windows asset found in the latest release."
}
$url = $assetInfo.browser_download_url
Write-Host "Downloading $($assetInfo.name)..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Expand-Archive -Path $zip -DestinationPath $installDir -Force
Remove-Item $zip -ErrorAction SilentlyContinue

$exe = Join-Path $installDir "azra.exe"

# The exe owns PATH setup so the installer and standalone exe behave identically.
& $exe setup-path

# Refresh this terminal session as well.
$machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$env:PATH = "$machinePath;$userPath"
Write-Host "Installed: $exe" -ForegroundColor Green
& $exe --version

# Set up VS Code highlighting automatically when VS Code is available.
Write-Host "Installing the Azra VS Code extension..." -ForegroundColor Cyan
& $exe setup-vscode

Write-Host "Done! Open a new terminal and run: azra run app.vibe" -ForegroundColor Green
