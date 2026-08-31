[CmdletBinding()]
param(
    [string]$InstallRoot = $(
        if ($env:CARGO_HOME) { $env:CARGO_HOME }
        else { Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cargo' }
    ),
    [string]$Repository = 'https://github.com/codex-mohan/branchcut.git',
    [string]$Branch = 'master'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo was not found. Install Rust from https://rustup.rs/ and run this installer again.'
}

$installRootPath = [System.IO.Path]::GetFullPath($InstallRoot)
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$manifest = Join-Path $repositoryRoot 'Cargo.toml'
$arguments = @('install', '--locked', '--force', '--root', $installRootPath)

if (Test-Path -LiteralPath $manifest -PathType Leaf) {
    $arguments += @('--path', $repositoryRoot)
    Write-Host "Installing Branchcut from $repositoryRoot"
}
else {
    $arguments += @('--git', $Repository, '--branch', $Branch)
    Write-Host "Installing Branchcut from $Repository ($Branch)"
}

& cargo @arguments
if ($LASTEXITCODE -ne 0) {
    throw "cargo install failed with exit code $LASTEXITCODE"
}

$binary = Join-Path $installRootPath 'bin\branchcut.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "Cargo completed but the Branchcut executable was not found at $binary"
}

& $binary --version

$binDirectory = Split-Path -Parent $binary
$pathEntries = $env:Path -split [System.IO.Path]::PathSeparator
if ($pathEntries -notcontains $binDirectory) {
    Write-Warning "$binDirectory is not currently on PATH. Add it to run 'branchcut' from any directory."
}

Write-Host "Installed: $binary"
Write-Host "Uninstall: cargo uninstall branchcut --root `"$installRootPath`""
