# Mekanism Horizon

**Inoffizieller Mekanism-Port für [Minecraft Horizon](https://github.com/horizon-rust/horizon)** – entwickelt mit Rust, PowerShell und Visual Studio 2026 unter Windows.

> **Experimenteller Prototyp v0.2.0 – nicht produktionsreif.** Die Content-Mod ist im Spiel getestet und die native Rust-Erweiterung wurde erfolgreich kompiliert. Dass der Energy Cube tatsächlich Energie speichert und über Neustarts behält, ist **noch nicht getestet**. Vor Tests Spielstände sichern.

## Aktueller Stand

| Bestandteil | Status |
| --- | --- |
| Osmiumbarren (`mekanism:osmium_ingot`) | Ingame bestätigt |
| Osmiumblock (`mekanism:osmium_block`) | Ingame bestätigt |
| Basic Energy Cube (`mekanism:basic_energy_cube`) | Als Item registriert und platzierbar |
| Content-Mod (`.hmod`) | Windows-Build und Ingame-Test erfolgreich |
| Native Rust-Mod | Mit Horizon Nightly `fdc87ae` gebaut und für die Instanz ausgewählt |
| Native Block-Entity und Energie-Capability | Implementiert, Ingame-Funktion noch offen |
| Energieaufnahme, Ausgabe und NBT-Persistenz | Noch nicht nachgewiesen |
| Maschinen, Generatoren, Kabel und GUI | Noch nicht implementiert |

Die Osmium- und Energy-Cube-Modelle sind vorläufig. Das Cube-Rezept ist ein Entwicklungsrezept und noch nicht das Originalrezept.

## Projektstruktur

- [`content/mekanism/`](content/mekanism/) – Horizon Content-Mod: Blöcke, Items, Modelle, Sprachen, Rezepte, Loot.
- [`native/mekanism_native/`](native/mekanism_native/) – Native Rust-Hooks für die Block-Entity und den Energiepuffer.
- [`logic/energy/`](logic/energy/) – Unabhängige Rust-Referenzlogik mit Unit-Tests; **nicht** die Runtime.
- [`scripts/`](scripts/) – Windows-Build, Validierung und automatische Installation.
- [`docs/ENERGY-CUBE.md`](docs/ENERGY-CUBE.md) – Technische Energy-Cube-Dokumentation.
- [`TODO.md`](TODO.md) – Roadmap für Mekanism Core, Generators, Tools und Additions.
- [`README-WINDOWS.md`](README-WINDOWS.md) – Ausführliche Entwicklungs- und Installationsanleitung.

## Schnellstart (Windows)

Benötigt werden Rust/Cargo, die Horizon-Mod-CLI, ein Horizon Launcher und eine eingerichtete Horizon-Instanz. Ausführung im Repository-Stammordner:

```powershell
# Bei Bedarf zuerst die Horizon-Mod-CLI installieren:
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Install-Horizon-Tool.ps1"

# Tests und Content-Build:
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Test-Content.ps1"
cargo test --manifest-path .\logic\energy\Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Für die automatische Installation muss `scripts/horizon-game-dir.txt` lokal den **Horizon-Spielordner mit `options.txt`** enthalten – nicht den Unterordner `horizon-mods`. Die Konfigurationsdatei wird nicht eingecheckt. Das Build-Skript kopiert `mekanism.hmod` in den Spielordner.

Die native Mod wird **separat** gebaut. Nur nach Prüfung des Codes und ausdrücklicher Zustimmung:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

`horizon-launcher.exe` muss im `PATH` oder am Standard-Installationsort erreichbar sein. Native Mods laufen mit den Rechten des angemeldeten Benutzers. Der erste Native-Build kann das Spiel vollständig neu kompilieren.

Nach dem Build Horizon über den Launcher starten und in einer Testwelt ausprobieren:

```mcfunction
/give @s mekanism:osmium_ingot 64
/give @s mekanism:osmium_block 64
/give @s mekanism:basic_energy_cube 1
```

## Technische Grenzen

- Der Basic Energy Cube meldet einen Puffer mit **4.000.000** Energieeinheiten und **4.000** Einheiten maximaler Ein- und Ausgabe **pro API-Aufruf**, nicht zwingend pro Tick.
- Für einen tatsächlichen Ladetest fehlt noch eine Energiequelle oder ein Debug-Hook. Ein platzierter Block beweist keine funktionierende Energie-Speicherung.
- Es gibt noch keine GUI, automatischen Transport, Kabel, Generatoren, Seiteneinstellungen oder Erhaltung der Ladung im abgebauten Item.
- Der Horizon Content-Cache kann nach erneuter Installation der `.hmod` neu aufgebaut werden; dadurch kann der erste Spielstart deutlich länger dauern.
- Die Build-Skripte werden noch portabler gemacht; sie enthalten derzeit Windows-spezifische Annahmen.

Die nächsten Schritte und Abnahmekriterien stehen in [TODO.md](TODO.md). Der aktuelle Prototyp orientiert sich am [Mekanism-Originalcode (Branch 26.3)](https://github.com/mekanism/Mekanism/tree/26.3).

## Lizenz und Credits

Dieses Projekt ist **inoffiziell** und steht nicht in Verbindung mit dem Mekanism- oder Horizon-Team. Der ursprüngliche [Mekanism-Code](https://github.com/mekanism/Mekanism) ist MIT-lizenziert; bei übernommenem Code und Assets müssen die anwendbaren Copyright- und Lizenzhinweise erhalten bleiben. Die Lizenz dieses Repositories steht in [LICENSE](LICENSE).
