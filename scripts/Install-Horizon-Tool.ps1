# Install official Horizon content mod CLI; requires Rust + Cargo from https://rustup.rs/
$ErrorActionPreference = 'Stop'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo nicht gefunden. Installiere Rust unter https://rustup.rs/ (Windows MSVC), starte PowerShell neu und wiederhole den Befehl.'
}
Write-Host 'Installiere offizielles cargo-horizon-mod (Windows)...'
& cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod
if ($LASTEXITCODE -ne 0) { throw 'Die Installation von cargo-horizon-mod ist fehlgeschlagen.' }
Write-Host 'Bereit. Starte .\scripts\Build-Content.ps1'
