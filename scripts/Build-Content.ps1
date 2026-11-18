# Run from any current directory: .\scripts\Build-Content.ps1
# Builds and installs the content mods: mekanism, mekanismgenerators, mekanismtools and mekanismadditions.
$ErrorActionPreference = 'Stop'
# Horizon Launcher on Windows
$LauncherDir = "F:\Games\Horizon Launcher"

if (Test-Path "$LauncherDir\horizon-launcher.exe") {
    $env:PATH = "$LauncherDir;$env:PATH"
}
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo is missing. Install Rust (https://rustup.rs/) first.'
}

$ConfigFile = Join-Path $Root 'scripts\horizon-game-dir.txt'
if (-not (Test-Path -LiteralPath $ConfigFile)) {
    throw "Horizon game folder missing: $ConfigFile"
}
$GameDir = (Get-Content -LiteralPath $ConfigFile -Raw).Trim().Trim('"')
if (-not (Test-Path -LiteralPath (Join-Path $GameDir 'options.txt'))) {
    throw "Invalid game folder: $GameDir (options.txt missing)"
}
$ModFolder = Join-Path $GameDir 'horizon-mods'
New-Item -ItemType Directory -Force -Path $ModFolder | Out-Null

foreach ($Mod in 'mekanism', 'mekanismgenerators', 'mekanismtools', 'mekanismadditions') {
    $Content = Join-Path $Root "content\$Mod"
    Push-Location $Content
    try {
        & cargo horizon-mod build
        if ($LASTEXITCODE -ne 0) { throw "Horizon content build failed: $Mod" }
        $Bundle = Join-Path $Content "target\horizon-mod\$Mod.hmod"
        if (-not (Test-Path -LiteralPath $Bundle)) { throw "Package not found: $Bundle" }
        & cargo horizon-mod check $Bundle
        if ($LASTEXITCODE -ne 0) { throw "Official Horizon package check failed: $Mod" }
        Write-Host "Mod built: $Bundle" -ForegroundColor Green

        # Install into Horizon automatically
        $Destination = Join-Path $ModFolder "$Mod.hmod"
        Copy-Item -LiteralPath $Bundle -Destination $Destination -Force
        Write-Host "$Mod installed automatically:" -ForegroundColor Green
        Write-Host $Destination
    } finally {
        Pop-Location
    }
}
