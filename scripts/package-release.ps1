param(
    [Parameter(Mandatory = $true)][string]$ArchiveName,
    [Parameter(Mandatory = $true)][string]$BinaryPath
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $RootDir "dist"
$StageDir = Join-Path $DistDir $ArchiveName

if (Test-Path $StageDir) {
    Remove-Item -Recurse -Force $StageDir
}

New-Item -ItemType Directory -Path $StageDir | Out-Null

Copy-Item (Join-Path $RootDir $BinaryPath) (Join-Path $StageDir "crondrop.exe")
Copy-Item (Join-Path $RootDir "SPEC.md") (Join-Path $StageDir "SPEC.md")
Copy-Item (Join-Path $RootDir "packaging/README.md") (Join-Path $StageDir "README-packaging.md")

$WindowsStartup = Join-Path $RootDir "packaging/windows/crondrop.cmd"
if (Test-Path $WindowsStartup) {
    Copy-Item $WindowsStartup (Join-Path $StageDir "crondrop.cmd")
}

$ZipPath = Join-Path $DistDir "$ArchiveName.zip"
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}

Compress-Archive -Path $StageDir -DestinationPath $ZipPath

