# Create current official typed native mod skeleton when we are ready for machines.
# Windows: use Horizon launcher native-build for compilation and gameplay.
$ErrorActionPreference = 'Stop'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cargo fehlt. https://rustup.rs/' }
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$NativeParent = Join-Path $Root 'native'
$NativeDir = Join-Path $NativeParent 'mekanism_native'
if (Test-Path -LiteralPath $NativeDir) { throw "Native-Template bereits vorhanden: $NativeDir" }
New-Item -ItemType Directory -Path $NativeParent -Force | Out-Null
& cargo horizon-mod new $NativeDir --template native --name 'Mekanism Horizon Native'
if ($LASTEXITCODE -ne 0) { throw 'Native-Template konnte nicht erstellt werden.' }
Write-Host "Native-Template angelegt: $NativeDir" -ForegroundColor Green
Write-Host 'Nach eigener Codepruefung mit horizon-launcher native-build <Pfad> --trust-native-code bauen.'
