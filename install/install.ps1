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

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host "Added $installDir to your user PATH (new terminals will see it)." -ForegroundColor Green
}

$exe = Join-Path $installDir "azra.exe"
Write-Host "Installed: $exe" -ForegroundColor Green
& $exe --version

# Set up VS Code highlighting automatically when VS Code is available.
$code = Get-Command code -ErrorAction SilentlyContinue
if ($code) {
    Write-Host "Installing the Azra VS Code extension..." -ForegroundColor Cyan
    & $exe setup-vscode
} else {
    Write-Host "VS Code CLI not found; run `azra setup-vscode` later to enable .vibe highlighting." -ForegroundColor Yellow
}

Write-Host "Done! Open a new terminal and run: azra run app.vibe" -ForegroundColor Green
