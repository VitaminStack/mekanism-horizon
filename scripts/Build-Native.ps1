# Run from the repository root or anywhere else; uses the currently selected launcher instance.
# Trusting native code is a decision for the user, not automatic.
param([switch]$TrustNativeCode)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$NativeDir = Join-Path $Root 'native\mekanism_native'
$LauncherCommand = Get-Command horizon-launcher -ErrorAction SilentlyContinue
$Launcher = if ($LauncherCommand) { $LauncherCommand.Source } else { $null }
if (-not $Launcher) {
    $Candidate = Join-Path $env:LOCALAPPDATA 'Programs\Horizon Launcher\horizon-launcher.exe'
    if (Test-Path -LiteralPath $Candidate) { $Launcher = $Candidate }
}
if (-not $Launcher) {
    throw 'Horizon Launcher nicht gefunden. Launcher installieren oder horizon-launcher.exe im PATH verfuegbar machen.'
}
if (-not (Test-Path -LiteralPath (Join-Path $NativeDir 'horizon-mixin.toml'))) {
    throw 'Native Mod nicht gefunden.'
}
if (-not $TrustNativeCode) {
    throw 'Native Code braucht deine ausdrueckliche Freigabe: .\scripts\Build-Native.ps1 -TrustNativeCode'
}
Write-Host 'WARNUNG: Horizon kompiliert und startet nativen Rust-Code mit deinen Benutzerrechten.' -ForegroundColor Yellow
$Confirm = Read-Host 'Quellcode geprueft? Zum Fortfahren BUILD eingeben'
if ($Confirm -cne 'BUILD') { throw 'Build abgebrochen.' }
& $Launcher native-build $NativeDir --trust-native-code
if ($LASTEXITCODE -ne 0) { throw 'Horizon Native-Build fehlgeschlagen.' }
Write-Host 'Native Companion gebaut. Horizon ueber den Launcher neu starten.' -ForegroundColor Green
