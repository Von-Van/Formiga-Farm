# Builds Formiga Farm for Windows, as a portable zip and a per-user installer that writes the
# registry values Formiga Desktop looks for (formiga-farm-contract's discovery module names them).
param(
    [switch]$SkipInstaller
)

$ErrorActionPreference = "Stop"
$RepoDir = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $RepoDir "dist"
$Exe = Join-Path $RepoDir "target\x86_64-pc-windows-msvc\release\formiga-farm.exe"
$Metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $RepoDir "Cargo.toml") | ConvertFrom-Json
$Version = if ($env:FORMIGA_FARM_VERSION) { $env:FORMIGA_FARM_VERSION } else { ($Metadata.packages | Where-Object name -eq "formiga-farm").version }
$Version = $Version.TrimStart("v")
$Portable = Join-Path $DistDir "Formiga-Farm-$Version-windows-x64.zip"
$Installer = Join-Path $DistDir "Formiga-Farm-$Version-windows-x64.msi"
$PortableDir = Join-Path $DistDir "Formiga-Farm-portable"
$License = Join-Path $RepoDir "LICENSE"

function Write-Checksum([string]$Path) {
    $Hash = (Get-FileHash -Algorithm SHA256 $Path).Hash.ToLowerInvariant()
    $Name = Split-Path -Leaf $Path
    Set-Content -NoNewline -Path "$Path.sha256" -Value "$Hash  $Name`n"
}

Push-Location $RepoDir
try {
    rustup target add x86_64-pc-windows-msvc
    cargo build --release --locked -p formiga-farm --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    # The Farm version comes from the binary itself, so the installer can never claim another.
    # Farm is a windowed program with no console of its own, so its output is caught in a file
    # and waited for, rather than read from a console it never opens.
    New-Item -ItemType Directory -Force $DistDir | Out-Null
    $Caught = Join-Path $DistDir "farm-version.txt"
    $Run = Start-Process -FilePath $Exe -ArgumentList "--farm-version" -NoNewWindow -Wait -PassThru -RedirectStandardOutput $Caught
    if ($Run.ExitCode -ne 0) { throw "formiga-farm --farm-version failed" }
    $FarmFormatVersion = (Get-Content -Raw $Caught).Trim()
    Remove-Item $Caught
    if ($FarmFormatVersion -notmatch '^\d+$') { throw "formiga-farm gave no Farm version: '$FarmFormatVersion'" }
    # And so does the icon, from the same picture as the window's.
    $IconDir = Join-Path $DistDir "icon"
    $Run = Start-Process -FilePath $Exe -ArgumentList "--icon", "`"$IconDir`"" -NoNewWindow -Wait -PassThru
    if ($Run.ExitCode -ne 0) { throw "formiga-farm --icon failed" }
    $Icon = Join-Path $IconDir "FormigaFarm.ico"
    if ($env:FORMIGA_SIGNTOOL_CERT_SHA1) {
        signtool sign /sha1 $env:FORMIGA_SIGNTOOL_CERT_SHA1 /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $Exe
        if ($LASTEXITCODE -ne 0) { throw "signtool sign failed for the program" }
    }
    New-Item -ItemType Directory -Force $DistDir | Out-Null
    if (Test-Path $PortableDir) { Remove-Item -Recurse -Force $PortableDir }
    New-Item -ItemType Directory -Force $PortableDir | Out-Null
    Copy-Item $Exe (Join-Path $PortableDir "Formiga Farm.exe")
    Copy-Item (Join-Path $RepoDir "packaging\windows\README.txt") (Join-Path $PortableDir "Read Me.txt")
    # The MIT licence travels with every copy, in the zip and installed beside the program.
    Copy-Item $License (Join-Path $PortableDir "License.txt")
    Compress-Archive -Force -Path (Join-Path $PortableDir "*") -DestinationPath $Portable
    Write-Checksum $Portable

    if (-not $SkipInstaller) {
        if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
            throw "WiX 4 CLI is required for the MSI. Install it with: dotnet tool install --global wix --version 4.0.5"
        }
        wix build `
            -d "FarmExe=$Exe" `
            -d "FarmVersion=$Version" `
            -d "FarmFormatVersion=$FarmFormatVersion" `
            -d "FarmIcon=$Icon" `
            -d "FarmLicense=$License" `
            -arch x64 `
            -o $Installer `
            (Join-Path $RepoDir "packaging\windows\FormigaFarm.wxs")
        if ($LASTEXITCODE -ne 0) { throw "wix build failed" }
        if ($env:FORMIGA_SIGNTOOL_CERT_SHA1) {
            signtool sign /sha1 $env:FORMIGA_SIGNTOOL_CERT_SHA1 /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $Installer
            if ($LASTEXITCODE -ne 0) { throw "signtool sign failed for the installer" }
        }
        Write-Checksum $Installer
    }
} finally {
    Pop-Location
}
