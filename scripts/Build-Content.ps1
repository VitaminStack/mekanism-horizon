# Run from any current directory: .\scripts\Build-Content.ps1
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Content = Join-Path $Root 'content\mekanism'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo fehlt. Erst Rust (https://rustup.rs/) installieren.'
}
Push-Location $Content
try {
    & cargo horizon-mod build
    if ($LASTEXITCODE -ne 0) { throw 'Horizon Content Build fehlgeschlagen.' }
    $Bundle = Join-Path $Content 'target\horizon-mod\mekanism.hmod'
    if (-not (Test-Path -LiteralPath $Bundle)) { throw "Paket nicht gefunden: $Bundle" }
    & cargo horizon-mod check $Bundle
    if ($LASTEXITCODE -ne 0) { throw 'Offizielle Horizon-Paketpruefung fehlgeschlagen.' }
    Write-Host "Mod erstellt: $Bundle" -ForegroundColor Green
    
    # Automatisch in Horizon installieren
    $ConfigFile = Join-Path $Root 'scripts\horizon-game-dir.txt'
    
    if (-not (Test-Path -LiteralPath $ConfigFile)) {
        throw "Horizon-Spielordner fehlt: $ConfigFile"
    }
    
    $GameDir = (Get-Content -LiteralPath $ConfigFile -Raw).Trim().Trim('"')
    
    if (-not (Test-Path -LiteralPath (Join-Path $GameDir 'options.txt'))) {
        throw "Ungueltiger Spielordner: $GameDir (options.txt fehlt)"
    }
    
    $ModFolder = Join-Path $GameDir 'horizon-mods'
    $Destination = Join-Path $ModFolder 'mekanism.hmod'
    
    New-Item -ItemType Directory -Force -Path $ModFolder | Out-Null
    Copy-Item -LiteralPath $Bundle -Destination $Destination -Force
    
    Write-Host "Mekanism automatisch installiert:" -ForegroundColor Green
    Write-Host $Destination

} finally {
    Pop-Location
}
