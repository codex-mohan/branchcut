[CmdletBinding()]
param(
    [string]$InstallRoot = $(
        if ($env:CARGO_HOME) { $env:CARGO_HOME }
        else { Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cargo' }
    )
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$installRootPath = [System.IO.Path]::GetFullPath($InstallRoot)
$binary = Join-Path $installRootPath 'bin\branchcut.exe'
$wasInstalled = Test-Path -LiteralPath $binary -PathType Leaf

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    & cargo uninstall branchcut --root $installRootPath
    if ($LASTEXITCODE -ne 0 -and $wasInstalled) {
        throw "cargo uninstall failed with exit code $LASTEXITCODE"
    }
}
elseif ($wasInstalled) {
    Remove-Item -LiteralPath $binary -Force
}

if (Test-Path -LiteralPath $binary -PathType Leaf) {
    throw "Branchcut is still present at $binary"
}

if ($wasInstalled) {
    Write-Host "Uninstalled Branchcut from $installRootPath"
}
else {
    Write-Host "Branchcut was not installed in $installRootPath"
}
