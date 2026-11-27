# Azra one-command installer for Windows.
#   irm https://raw.githubusercontent.com/11nawid/azravibe.vibe/main/install/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "11nawid/azravibe.vibe"
$asset = "azra-windows-x86_64.zip"
$url = "https://github.com/$repo/releases/latest/download/$asset"
$installDir = Join-Path $env:LOCALAPPDATA "Azra\bin"
$zip = Join-Path $env:TEMP "azra-install.zip"

Write-Host "Downloading Azra..." -ForegroundColor Cyan
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
