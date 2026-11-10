# Run from any current directory: .\scripts\Build-Content.ps1
# Builds and installs the content mods: mekanism, mekanismgenerators and mekanismadditions.
$ErrorActionPreference = 'Stop'
# Horizon Launcher unter Windows
$LauncherDir = "F:\Games\Horizon Launcher"

if (Test-Path "$LauncherDir\horizon-launcher.exe") {
    $env:PATH = "$LauncherDir;$env:PATH"
}
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo fehlt. Erst Rust (https://rustup.rs/) installieren.'
}

$ConfigFile = Join-Path $Root 'scripts\horizon-game-dir.txt'
if (-not (Test-Path -LiteralPath $ConfigFile)) {
    throw "Horizon-Spielordner fehlt: $ConfigFile"
}
$GameDir = (Get-Content -LiteralPath $ConfigFile -Raw).Trim().Trim('"')
if (-not (Test-Path -LiteralPath (Join-Path $GameDir 'options.txt'))) {
    throw "Ungueltiger Spielordner: $GameDir (options.txt fehlt)"
}
$ModFolder = Join-Path $GameDir 'horizon-mods'
New-Item -ItemType Directory -Force -Path $ModFolder | Out-Null

foreach ($Mod in 'mekanism', 'mekanismgenerators', 'mekanismadditions') {
    $Content = Join-Path $Root "content\$Mod"
    Push-Location $Content
    try {
        & cargo horizon-mod build
        if ($LASTEXITCODE -ne 0) { throw "Horizon Content Build fehlgeschlagen: $Mod" }
        $Bundle = Join-Path $Content "target\horizon-mod\$Mod.hmod"
        if (-not (Test-Path -LiteralPath $Bundle)) { throw "Paket nicht gefunden: $Bundle" }
        & cargo horizon-mod check $Bundle
        if ($LASTEXITCODE -ne 0) { throw "Offizielle Horizon-Paketpruefung fehlgeschlagen: $Mod" }
        Write-Host "Mod erstellt: $Bundle" -ForegroundColor Green

        # Automatisch in Horizon installieren
        $Destination = Join-Path $ModFolder "$Mod.hmod"
        Copy-Item -LiteralPath $Bundle -Destination $Destination -Force
        Write-Host "$Mod automatisch installiert:" -ForegroundColor Green
        Write-Host $Destination
    } finally {
        Pop-Location
    }
}
