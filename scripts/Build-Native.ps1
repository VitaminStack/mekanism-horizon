# Run from the repository root or anywhere else; uses the currently selected launcher instance.
# Trusting native code is a decision for the user, not automatic.
# After a successful build, older native-* versions are removed, so the launcher's
# version list only offers the newest build (no manual re-selection each time).
#   -Launcher PATH   horizon-launcher.exe (or set HORIZON_LAUNCHER)
#   -Confirmed       skip the interactive BUILD prompt (you already reviewed the source)
#   -CleanupOnly     only remove older native-* versions
param(
    [switch]$TrustNativeCode,
    [switch]$Confirmed,
    [switch]$CleanupOnly,
    [string]$Launcher = $env:HORIZON_LAUNCHER
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$NativeDir = Join-Path $Root 'native\mekanism_native'
$LauncherCommand = Get-Command horizon-launcher -ErrorAction SilentlyContinue
if (-not $Launcher -and $LauncherCommand) { $Launcher = $LauncherCommand.Source }
foreach ($Candidate in @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Horizon Launcher\horizon-launcher.exe'),
        'F:\Games\Horizon Launcher\horizon-launcher.exe')) {
    if (-not $Launcher -and (Test-Path -LiteralPath $Candidate)) { $Launcher = $Candidate }
}
if (-not $Launcher) {
    throw 'Horizon Launcher nicht gefunden. -Launcher PFAD angeben oder HORIZON_LAUNCHER setzen.'
}

function Remove-OldNativeVersions {
    $Versions = Join-Path $env:APPDATA 'horizon-launcher\versions'
    $Native = @(Get-ChildItem -LiteralPath $Versions -Directory -Filter 'native-*' |
            Sort-Object LastWriteTime -Descending)
    if ($Native.Count -eq 0) { return }
    # Make Play start the newest build (the launcher may keep an older one selected).
    & $Launcher instance version horizon $Native[0].Name | Out-Null
    Write-Host "Neueste Native-Version: $($Native[0].Name)" -ForegroundColor Green
    foreach ($Old in $Native | Select-Object -Skip 1) {
        & $Launcher remove $Old.Name | Out-Null
        # The launcher's exit code is not reliable here; the folder tells.
        if (Test-Path -LiteralPath $Old.FullName) {
            Write-Warning "$($Old.Name) nicht entfernt (laeuft Horizon noch?)."
        } else {
            Write-Host "Alte Version entfernt: $($Old.Name)"
        }
    }
}

if ($CleanupOnly) { Remove-OldNativeVersions; return }
if (-not (Test-Path -LiteralPath (Join-Path $NativeDir 'horizon-mixin.toml'))) {
    throw 'Native Mod nicht gefunden.'
}
if (-not $TrustNativeCode) {
    throw 'Native Code braucht deine ausdrueckliche Freigabe: .\scripts\Build-Native.ps1 -TrustNativeCode'
}
Write-Host 'WARNUNG: Horizon kompiliert und startet nativen Rust-Code mit deinen Benutzerrechten.' -ForegroundColor Yellow
if (-not $Confirmed) {
    $Confirm = Read-Host 'Quellcode geprueft? Zum Fortfahren BUILD eingeben'
    if ($Confirm -cne 'BUILD') { throw 'Build abgebrochen.' }
}
& $Launcher native-build $NativeDir --trust-native-code
if ($LASTEXITCODE -ne 0) { throw 'Horizon Native-Build fehlgeschlagen.' }
Remove-OldNativeVersions
Write-Host 'Native Companion gebaut. Horizon ueber den Launcher neu starten.' -ForegroundColor Green
