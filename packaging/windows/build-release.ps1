[CmdletBinding()]
param([string]$PortableZip)
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Run this release build on Windows.' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$base = Join-Path $repo 'releases\0.2.0-beta'
$dest = Join-Path $base 'windows-x64'
if (Test-Path -LiteralPath $dest) {
    $dest = Join-Path $base ('windows-x64-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
}
$work = Join-Path $repo 'native-prototype\target\release-packaging'
$tools = Join-Path $work 'wix-4.0.6'
$wix = Join-Path $tools 'wix.exe'
$target = 'x86_64-pc-windows-msvc'
$previousTarget = $env:CARGO_TARGET_DIR
$previousRoll = $env:DOTNET_ROLL_FORWARD
function Check([string]$step) {
    if ($LASTEXITCODE -ne 0) { throw "$step failed (exit $LASTEXITCODE)." }
}
Push-Location $repo
try {
    $required = if ($PortableZip) { @('dotnet') } else { @('cargo','npm.cmd','dotnet') }
    foreach ($cmd in $required) { Get-Command $cmd -ErrorAction Stop | Out-Null }
    New-Item -ItemType Directory -Force $work | Out-Null
    $env:DOTNET_ROLL_FORWARD = 'Major'
    if (!(Test-Path -LiteralPath $wix)) {
        dotnet tool install wix --tool-path $tools --version 4.0.6
        Check 'Restore WiX'
    }
    $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
    if (!$iscc) {
        $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
                  "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
                  "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") |
            Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    }
    if (!$iscc) { throw 'Install Inno Setup 6 with: winget install --id JRSoftware.InnoSetup -e' }
    if ($PortableZip) {
        $PortableZip = (Resolve-Path -LiteralPath $PortableZip).Path
    } else {
        $env:CARGO_TARGET_DIR = Join-Path $repo 'native-prototype\target\phase2-preview\cargo'
        if (!(Test-Path 'node_modules\.bin\tauri.cmd')) {
            npm.cmd ci
            Check 'Restore frontend dependencies'
        }
        cargo test --locked --manifest-path native-prototype/Cargo.toml --target $target
        Check 'Native tests'
        npm.cmd run tauri build -- --no-bundle --target $target
        Check 'Settings build'
        cargo build --locked --release --manifest-path native-prototype/Cargo.toml --target $target
        Check 'Native release build'
    }
    $stage = New-Item -ItemType Directory -Path (Join-Path $work ([guid]::NewGuid()))
    $payload = New-Item -ItemType Directory -Path (Join-Path $stage.FullName 'payload')
    $artifacts = New-Item -ItemType Directory -Path (Join-Path $stage.FullName 'output')
    if ($PortableZip) {
        Expand-Archive -LiteralPath $PortableZip -DestinationPath $payload.FullName
        foreach ($name in 'StackDock.exe','stack-dock-native.exe','LICENSE') {
            if (!(Test-Path -LiteralPath (Join-Path $payload.FullName $name) -PathType Leaf)) { throw "Missing portable file: $name" }
        }
    } else {
        $release = Join-Path $env:CARGO_TARGET_DIR "$target\release"
        foreach ($name in 'stack-dock-native.exe','StackDock.exe') {
            Copy-Item -LiteralPath (Join-Path $release $name) -Destination $payload.FullName
        }
        Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination $payload.FullName
    }
    $icon = Join-Path $repo 'native-prototype\assets\stackdock-native.ico'
    $msi = Join-Path $artifacts.FullName 'StackDock-Native-0.2.0-x64.msi'
    $setup = Join-Path $artifacts.FullName 'StackDock-Native-0.2.0-beta-x64-Setup.exe'
    $portable = Join-Path $artifacts.FullName 'StackDock-Native-0.2.0-beta-x64-Portable.zip'
    Push-Location $work
    try {
        & $wix build -arch x64 -d "Payload=$($payload.FullName)" -d "NativeIcon=$icon" -pdbtype none -intermediateFolder (Join-Path $stage.FullName 'msi-obj') -o $msi (Join-Path $PSScriptRoot 'product.wxs')
        Check 'MSI build'
        & $iscc "/DNativeIcon=$icon" "/DMsi=$msi" "/DOutputFolder=$($artifacts.FullName)" (Join-Path $PSScriptRoot 'setup.iss')
        Check 'Setup EXE build'
    } finally { Pop-Location }
    if ($PortableZip) { Copy-Item -LiteralPath $PortableZip -Destination $portable }
    else { Compress-Archive -Path (Join-Path $payload.FullName '*') -DestinationPath $portable }
    foreach ($file in $msi,$setup,$portable) {
        if ((Get-Item -LiteralPath $file).Length -le 0) { throw "Empty output: $file" }
    }
    New-Item -ItemType Directory -Force $base | Out-Null
    [IO.Directory]::Move($artifacts.FullName,$dest)
    Remove-Item -LiteralPath $stage.FullName -Recurse
    Write-Host "StackDock Native release files: $dest"
    Get-ChildItem -LiteralPath $dest | Select-Object Name,Length
} finally {
    $env:DOTNET_ROLL_FORWARD = $previousRoll
    $env:CARGO_TARGET_DIR = $previousTarget
    Pop-Location
}
