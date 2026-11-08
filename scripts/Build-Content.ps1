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
} finally {
    Pop-Location
}
