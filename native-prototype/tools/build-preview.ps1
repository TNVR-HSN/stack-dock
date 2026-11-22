[CmdletBinding()]
param(
    [ValidateSet('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc')]
    [string]$Target = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'Build the StackDock preview on Windows.'
}

$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$previewRoot = Join-Path $repo 'native-prototype\target\phase2-preview'
$previousTargetDir = $env:CARGO_TARGET_DIR

Push-Location -LiteralPath $repo
try {
    $env:CARGO_TARGET_DIR = Join-Path $previewRoot 'cargo'

    & npm.cmd run tauri build -- --no-bundle --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw 'StackDock Settings build failed.'
    }

    & cargo build --locked --release --manifest-path '.\native-prototype\Cargo.toml' --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw 'Native StackDock build failed.'
    }

    $release = Join-Path $env:CARGO_TARGET_DIR "$Target\release"
    $settingsExe = Get-Item -LiteralPath (Join-Path $release 'StackDock.exe')
    $dockExe = Get-Item -LiteralPath (Join-Path $release 'stack-dock-native.exe')

    $previewName = "StackDock-Phase2-$Target-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' +
        [guid]::NewGuid().ToString('N').Substring(0, 8)
    $preview = New-Item -ItemType Directory -Path (Join-Path $previewRoot $previewName)
    $license = Join-Path $repo 'LICENSE'
    Copy-Item -LiteralPath $settingsExe.FullName, $dockExe.FullName, $license -Destination $preview.FullName

    $archive = Join-Path $previewRoot "$previewName.zip"
    $packageFiles = @('StackDock.exe', 'stack-dock-native.exe', 'LICENSE') |
        ForEach-Object { Join-Path $preview.FullName $_ }
    Compress-Archive -LiteralPath $packageFiles -DestinationPath $archive

    Write-Host "Preview folder: $($preview.FullName)"
    Write-Host "Start the dock: $(Join-Path $preview.FullName 'stack-dock-native.exe')"
    Write-Host "Shareable preview ZIP: $archive"
} finally {
    $env:CARGO_TARGET_DIR = $previousTargetDir
    Pop-Location
}
