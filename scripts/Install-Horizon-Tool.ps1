# Install official Horizon content mod CLI; requires Rust + Cargo from https://rustup.rs/
$ErrorActionPreference = 'Stop'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo not found. Install Rust from https://rustup.rs/ (Windows MSVC), restart PowerShell and run the command again.'
}
Write-Host 'Installing official cargo-horizon-mod (Windows)...'
& cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod
if ($LASTEXITCODE -ne 0) { throw 'Installing cargo-horizon-mod failed.' }
Write-Host 'Ready. Run .\scripts\Build-Content.ps1'
