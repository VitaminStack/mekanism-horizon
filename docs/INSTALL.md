# Installationsanleitung (Windows)

Mekanism Horizon besteht aus **Content-Mods** (`.hmod`, reine Daten) und einer **nativen Rust-Mod** (`.hnmod`, Maschinenlogik). Ohne die native Mod laden Items, Blöcke, Rezepte und Weltgenerierung, aber Maschinen, Generatoren, Energie und Tanks tun nichts.

> Experimenteller Prototyp. Nur in Testwelten verwenden und Welten vorher sichern. Bekannte Einschränkungen: [KNOWN_ISSUES.md](KNOWN_ISSUES.md).

## Horizon-Version

Alles ist gegen **Horizon Nightly 2026-11-08, Commit [`fdc87ae`](https://github.com/horizon-rust/horizon/commit/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca)** (Native-Hooks-API 411) gebaut und getestet. Der Pin steht an zwei Stellen und muss übereinstimmen:

- `native/mekanism_native/horizon-mixin.toml` (`horizon_revision`)
- `native/mekanism_native/api/Cargo.toml` (`rev`)

Andere Horizon-Stände können laufen, sind aber nicht getestet. Die native Mod hängt an der Hook-API, ein Versionswechsel braucht ggf. Code-Änderungen.

## Voraussetzungen

| Was | Wofür | Prüfen |
|---|---|---|
| [Horizon Launcher](https://github.com/horizon-rust/horizon) mit einer Instanz | Spielstart, nativer Build | `horizon-launcher` im Launcher-Ordner |
| Rust (MSVC-Toolchain, [rustup](https://rustup.rs/)) | Content-Build, Logik-Tests | `cargo --version` |
| `cargo-horizon-mod` | Content bauen und prüfen | `scripts/Install-Horizon-Tool.ps1` |
| Python 3.11+ | Content-Validierung, Generatoren | `python --version` |
| Git | Repository klonen | `git --version` |

Der native Build nutzt die **vom Launcher verwaltete Toolchain**, nicht deine eigene. Visual Studio 2026 ist optional.

## 1. Repository und Spielordner

```powershell
git clone https://github.com/VitaminStack/mekanism-horizon.git
cd .\mekanism-horizon
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Install-Horizon-Tool.ps1"
```

Im Launcher die Instanz wählen und ihren **Spielordner** öffnen (dort liegt `options.txt`). Den Pfad in die lokale Datei `scripts/horizon-game-dir.txt` schreiben, eine Zeile, zum Beispiel `C:\Pfad\zur\Horizon-Instanz`. Nicht `horizon-mods` angeben, sondern den übergeordneten Ordner. Die Datei steht in `.gitignore`.

## 2. Content bauen und installieren

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Test-Content.ps1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

`Build-Content.ps1` baut nacheinander vier Content-Mods, prüft jedes Paket mit `cargo horizon-mod check` und kopiert es nach `<Spielordner>\horizon-mods\`:

| Mod | Inhalt | Braucht |
|---|---|---|
| `mekanism` | Kern: Erze, Maschinen, Energy Cubes, Chemie | nativ |
| `mekanismgenerators` | Generatoren | `mekanism`, nativ |
| `mekanismtools` | Werkzeuge, Rüstung, Paxel | `mekanism` |
| `mekanismadditions` | Plastic-Blöcke, Glow Panels, Obsidian TNT | nur Content |

Neue Mods starten **aus**. In `<Spielordner>\horizon-mods.txt` bzw. in der Mod-Liste des Launchers aktivieren (Zeile `mod.<id>:true`). Nach Registry-Änderungen Spiel neu starten.

Der erste Start nach einem Build ist langsam (Content-Cache, im Test rund 46 s), der zweite schnell.

## 3. Native Mod bauen (Vertrauensfreigabe)

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

### Was „Native Code vertrauen“ bedeutet

Horizon kompiliert den Rust-Code in `native/mekanism_native/` in eine eigene Spielversion und führt ihn **im Spielprozess mit deinen Windows-Benutzerrechten** aus. Er unterliegt keiner Sandbox: Er könnte Dateien lesen und schreiben oder Netzwerkverbindungen öffnen. Darum:

- Der Schalter `-TrustNativeCode` ist Pflicht. Ohne ihn bricht das Skript ab.
- Danach fragt das Skript nach der Eingabe `BUILD`. `-Confirmed` überspringt diese Frage, nur nutzen, wenn du den Quellcode selbst geprüft hast.
- Prüfe den Code (mindestens `native/mekanism_native/api/src/`) vor dem ersten Build und nach jedem Pull. Baue nur Stände aus Quellen, denen du vertraust, auch bei fremden Pull Requests.

Der erste Vollbuild dauerte auf dem Testsystem rund 11 Minuten, Folgebuilds sind schneller. Nach Erfolg wählt das Skript die neueste `native-*`-Version für die Instanz `horizon` und entfernt ältere (`-CleanupOnly` macht nur das).

Launcher nicht gefunden? Pfad angeben:

```powershell
$env:HORIZON_LAUNCHER = "F:\Games\Horizon Launcher\horizon-launcher.exe"
```

oder `-Launcher <Pfad>`. Suchreihenfolge: Parameter/`HORIZON_LAUNCHER`, `PATH`, `%LOCALAPPDATA%\Programs\Horizon Launcher`, `F:\Games\Horizon Launcher`.

## 4. Starten

Horizon über den Launcher mit der neu gebauten `native-*`-Version starten, Content-Mods aktivieren, Testwelt öffnen.

```mcfunction
/give @s mekanism:basic_energy_cube 1
/give @s mekanism:enrichment_chamber 1
/give @s mekanism:osmium_ingot 64
```

## Häufige Fehler

| Meldung | Lösung |
|---|---|
| `Horizon-Spielordner fehlt` / `options.txt fehlt` | `scripts/horizon-game-dir.txt` anlegen bzw. auf den Spielordner statt `horizon-mods` setzen |
| `Cargo fehlt` | Rust installieren, PowerShell neu starten |
| `Horizon Launcher nicht gefunden` | `HORIZON_LAUNCHER` oder `-Launcher` setzen |
| `Native Code braucht deine ausdrueckliche Freigabe` | `-TrustNativeCode` ergänzen (siehe oben) |
| PowerShell blockiert Skripte | mit `powershell.exe -NoProfile -ExecutionPolicy Bypass -File ...` starten |
| Neue Mod fehlt im Spiel | Mod in `horizon-mods.txt` aktivieren, Spiel neu starten |
| Maschinen platzierbar, aber ohne Funktion | native Version nicht gewählt oder nicht gebaut |
| `Alte Version nicht entfernt` | Horizon läuft noch, schließen und `-CleanupOnly` |

Weitere Windows-Details: [README-WINDOWS.md](../README-WINDOWS.md). Testergebnisse: [TESTLOG.md](TESTLOG.md).
