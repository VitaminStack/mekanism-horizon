# Local checks with Python 3.11+; no Rust required.
$ErrorActionPreference = 'Stop'
$Validator = Join-Path $PSScriptRoot 'validate_content.py'
if (-not (Get-Command python -ErrorAction SilentlyContinue)) { throw 'Python 3.11+ fehlt.' }
& python $Validator
if ($LASTEXITCODE -ne 0) { throw 'Content-Validierung fehlgeschlagen.' }
