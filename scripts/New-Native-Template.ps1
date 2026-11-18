# Create current official typed native mod skeleton when we are ready for machines.
# Windows: use Horizon launcher native-build for compilation and gameplay.
$ErrorActionPreference = 'Stop'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cargo is missing. https://rustup.rs/' }
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$NativeParent = Join-Path $Root 'native'
$NativeDir = Join-Path $NativeParent 'mekanism_native'
if (Test-Path -LiteralPath $NativeDir) { throw "Native template already exists: $NativeDir" }
New-Item -ItemType Directory -Path $NativeParent -Force | Out-Null
& cargo horizon-mod new $NativeDir --template native --name 'Mekanism Horizon Native'
if ($LASTEXITCODE -ne 0) { throw 'Could not create the native template.' }
Write-Host "Native template created: $NativeDir" -ForegroundColor Green
Write-Host 'After reviewing the code yourself, build with horizon-launcher native-build <path> --trust-native-code.'
