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
    Write-Host 'Cargo was not found. Installing a minimal stable Rust toolchain with rustup...'
    $architecture = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
    $rustupInstaller = Join-Path $env:TEMP 'rustup-init.exe'
    Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/$architecture" -OutFile $rustupInstaller
    & $rustupInstaller -y --profile minimal --default-toolchain stable
    if ($LASTEXITCODE -ne 0) {
        throw "rustup failed with exit code $LASTEXITCODE"
    }

    $rustCargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
    $env:Path = "$(Join-Path $rustCargoHome 'bin');$env:Path"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo is still unavailable after rustup completed. Open a new terminal and run this installer again.'
}

$installRootPath = [System.IO.Path]::GetFullPath($InstallRoot)
$repositoryRoot = $null
if ($PSScriptRoot) {
    $candidateRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
    if (Test-Path -LiteralPath (Join-Path $candidateRoot 'Cargo.toml') -PathType Leaf) {
        $repositoryRoot = $candidateRoot
    }
}

$arguments = @('install', '--locked', '--force', '--root', $installRootPath)
if ($repositoryRoot) {
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
