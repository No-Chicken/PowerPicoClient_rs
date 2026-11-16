[CmdletBinding()]
param(
    [switch]$Offline,
    [switch]$SkipChecks,
    [switch]$Updater
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($env:OS -ne 'Windows_NT') {
    throw 'Run this script on Windows with the x64 MSVC Rust toolchain.'
}

function Invoke-Checked {
    param([string]$File, [string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$File failed with exit code $LASTEXITCODE"
    }
}

foreach ($tool in @('node.exe', 'pnpm.cmd', 'cargo.exe', 'rustc.exe')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Missing $tool. Install the Windows prerequisites listed in README.md."
    }
}
if ($Updater -and -not $env:TAURI_SIGNING_PRIVATE_KEY) {
    throw 'Updater builds require TAURI_SIGNING_PRIVATE_KEY; use a release key matching tauri.conf.json.'
}

# Also find Build Tools installed outside the default Visual Studio directory.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) {
    throw 'Install Visual Studio Build Tools with the C++ desktop workload and Windows SDK.'
}
$installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $installation) {
    throw 'No Visual Studio installation with the x64 C++ tools was found.'
}
$vcvars = Join-Path $installation 'VC\Auxiliary\Build\vcvars64.bat'
$devEnvironment = & $env:ComSpec /d /s /c ('""{0}" >nul && set"' -f $vcvars)
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to initialize the MSVC build environment.'
}
foreach ($line in $devEnvironment) {
    if ($line -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
    }
}

$project = Split-Path $PSScriptRoot -Parent
$target = 'x86_64-pc-windows-msvc'
$previousTargetDir = $env:CARGO_TARGET_DIR
$updaterConfig = $null
Push-Location $project
try {
    $env:CARGO_TARGET_DIR = Join-Path $project 'src-tauri\target'
    $package = Get-Content package.json -Raw | ConvertFrom-Json
    $config = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    if ($package.version -ne $config.version) {
        throw 'package.json and tauri.conf.json versions must match.'
    }

    $installArgs = @('install', '--frozen-lockfile')
    $cargoArgs = @('--manifest-path', 'src-tauri/Cargo.toml', '--target', $target, '--locked')
    if ($Offline) {
        $installArgs += '--offline'
        $cargoArgs += '--offline'
    }
    Invoke-Checked 'pnpm.cmd' $installArgs
    if (-not $SkipChecks) {
        Invoke-Checked 'pnpm.cmd' @('check')
        Invoke-Checked 'pnpm.cmd' @('test')
        Invoke-Checked 'cargo.exe' @('fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--', '--check')
        Invoke-Checked 'cargo.exe' (@('clippy') + $cargoArgs + @('--', '-D', 'warnings'))
        Invoke-Checked 'cargo.exe' (@('test') + $cargoArgs)
    }

    $buildArgs = @('tauri', 'build', '--ci', '--target', $target)
    if ($Updater) {
        $updaterConfig = Join-Path ([IO.Path]::GetTempPath()) ('powerpico-updater-' + [guid]::NewGuid() + '.json')
        [IO.File]::WriteAllText($updaterConfig, '{"bundle":{"createUpdaterArtifacts":true}}', [Text.UTF8Encoding]::new($false))
        $buildArgs += @('--config', $updaterConfig)
    }
    $buildArgs += @('--', '--locked')
    if ($Offline) { $buildArgs += '--offline' }
    Invoke-Checked 'pnpm.cmd' $buildArgs

    $bundle = Join-Path $env:CARGO_TARGET_DIR "$target\release\bundle\nsis"
    $installer = Join-Path $bundle "PowerPico Client_$($package.version)_x64-setup.exe"
    if (-not (Test-Path $installer)) { throw "Missing installer: $installer" }
    $output = Join-Path $project 'artifacts\windows'
    New-Item -ItemType Directory $output -Force | Out-Null
    $name = "PowerPico-Client_$($package.version)_windows_x86_64-setup.exe"
    Copy-Item $installer (Join-Path $output $name) -Force
    $assets = @($name)
    $signature = Join-Path $output "$name.sig"
    if ($Updater) {
        if (-not (Test-Path "$installer.sig") -or (Get-Item "$installer.sig").Length -eq 0) {
            throw 'The updater signature was not generated.'
        }
        Copy-Item "$installer.sig" $signature -Force
        $assets += "$name.sig"
    } else {
        foreach ($staleSignature in @($signature, "$installer.sig")) {
            if (Test-Path $staleSignature) { Remove-Item $staleSignature }
        }
    }
    $checksums = foreach ($asset in $assets) {
        $hash = (Get-FileHash (Join-Path $output $asset) -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $asset"
    }
    [IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS'), [string[]]$checksums, [Text.UTF8Encoding]::new($false))
    Write-Host "Windows packages written to $output"
} finally {
    if ($updaterConfig -and (Test-Path $updaterConfig)) { Remove-Item $updaterConfig }
    $env:CARGO_TARGET_DIR = $previousTargetDir
    Pop-Location
}
