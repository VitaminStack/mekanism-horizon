# Mekanism Horizon – Windows-Entwicklungsanleitung

Getesteter Stand: **Windows / Visual Studio 2026**, **Horizon Nightly `fdc87ae`**, Content-Mod **v0.2.0**, nativer Rust-Begleiter (Build `native-fdc87ae-8abc9345573f`). Energiespeicherung im Spiel geprüft, siehe [Testprotokoll](docs/TESTLOG.md).

## Voraussetzungen

- [Horizon Launcher](https://github.com/horizon-rust/horizon) mit einer installierten Spielinstanz.
- Rust und Cargo: `cargo --version` und `rustc --version` prüfen.
- Horizon-Content-CLI `cargo horizon-mod` (über `scripts/Install-Horizon-Tool.ps1` einrichten).
- Windows PowerShell und optional Visual Studio 2026 (lokale `Strg+B`-Konfiguration).

Der native Horizon-Build verwendet eine **vom Launcher verwaltete Rust-Toolchain** und erzeugt eine eigene Spielversion. Die Content-Mod (`.hmod`) enthält keinen nativen Rust-Code.

## Projekt auschecken und Instanz konfigurieren

```powershell
git clone https://github.com/VitaminStack/mekanism-horizon.git
cd .\mekanism-horizon
```

Im Horizon Launcher die gewünschte Instanz auswählen und ihren **Spielordner** öffnen. Dort befindet sich `options.txt`. Diesen Pfad in die lokal anzulegende Textdatei `scripts/horizon-game-dir.txt` eintragen, zum Beispiel:

```text
C:\Pfad\zur\Horizon-Instanz
```

**Nicht** `horizon-mods` angeben, sondern den **übergeordneten Ordner**. Die Datei wird von Git ignoriert und enthält bewusst keine allgemeingültige Pfadangabe.

## Content-Mod testen, bauen und installieren

Die folgenden Befehle im Stammordner des Repositories ausführen:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Test-Content.ps1"
cargo test --manifest-path .\logic\energy\Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Der Build erzeugt `content\mekanism\target\horizon-mod\mekanism.hmod`, prüft die Datei mit `cargo horizon-mod check` und installiert sie unter `<Spielordner>\horizon-mods\mekanism.hmod`.

**Content-Cache:** Das aktuelle Skript kopiert die Datei beim Build erneut, auch wenn der Inhalt gleich bleibt. Horizon kann dadurch seinen Content-Cache neu erzeugen. Im lokalen Test dauerte ein erster Start rund 46 Sekunden; der zweite Start ohne weiteren Build war schnell. Die geplante Optimierung ist ein SHA-256-Vergleich vor dem Kopieren.

## Native Rust-Mod bauen

Der native Begleiter liegt unter `native/mekanism_native/`. Der Code deklariert eine Block-Entity und eine Energie-Capability. Er läuft **mit den Rechten des angemeldeten Benutzers**. Daher erst den Code kontrollieren und anschließend ausdrücklich freigeben:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

Das Skript verlangt zusätzlich die Bestätigung `BUILD`. Die native Mod wird über `horizon-launcher native-build` in eine eigene Spielversion kompiliert. Der erste vollständige Build dauerte auf dem getesteten System **10 Minuten 50 Sekunden**; Folgebuilds können schneller sein.

### Portabler Launcher unter Windows

Falls `Horizon Launcher nicht gefunden` erscheint, liegt `horizon-launcher.exe` vermutlich nicht im `PATH` oder am Standardinstallationsort. Beispiel für eine **portabel installierte Version** (Pfad auf deinem System anpassen):

```powershell
$env:PATH = "F:\Games\Horizon Launcher;$env:PATH"
Get-Command horizon-launcher.exe
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

Der `PATH`-Eintrag gilt nur für dieses PowerShell-Fenster.

## Testen im Spiel

Horizon **über den Launcher** mit der zugehörigen Instanz und der neu gebauten nativen Spielversion starten. Content-Mod aktivieren; nach Registry-Änderungen das Spiel neu starten. Ausschließlich eine Testwelt oder eine zuvor gesicherte Welt verwenden.

```mcfunction
/give @s mekanism:osmium_ingot 64
/give @s mekanism:osmium_block 64
/give @s mekanism:basic_energy_cube 1
```

Energie testen: `mekanism:debug_energy_source` bzw. `mekanism:debug_energy_sink` neben einen Cube setzen (4.000/Tick), Füllstand mit `/data get block <x> <y> <z>` lesen.

**Bestätigt:** Block-Entity je Cube, Einfügen/Entnehmen, Limits, Leer/Voll, Speichern und Neustart. Ergebnisse in [docs/TESTLOG.md](docs/TESTLOG.md).

**Noch offen:** seitenweise Ein-/Ausgabe über Horizons Energie-Pfad (braucht echte Quelle). Der Cube hat **keine GUI**.

## CI

`.github/workflows/ci.yml` läuft bei Push auf `main` und bei Pull Requests: `scripts/validate_content.py` (JSON-Syntax, Manifest) und `cargo test` für `logic/energy`. Der native Build läuft **nicht** in CI, er braucht den Horizon Launcher.

## Häufige Fehler

| Meldung | Lösung |
| --- | --- |
| `options.txt fehlt` | Prüfen, ob `horizon-game-dir.txt` auf den **Spielordner** statt auf `horizon-mods` zeigt. |
| `Horizon Launcher nicht gefunden` | Pfad zur Launcher-EXE wie oben temporär in `PATH` aufnehmen. |
| PowerShell blockiert Skripte | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <Skript>` verwenden. |
| Erster Spielstart langsam | Wahrscheinlich Content-Cache nach dem letzten Build neu aufgebaut; zweiten Start ohne erneuten Build prüfen. |
| Cube platziert, aber keine GUI | In v0.2.0 erwartet; GUI und vollständige Energiefunktion sind nicht implementiert. |
| `git apply` meldet bereits existierende Dateien | Patch möglicherweise schon angewendet: zuerst `git status` und `git apply --reverse --check <patch>` prüfen. |

## Weitere Informationen

- [Projektübersicht](README.md)
- [Energy-Cube-Technik](docs/ENERGY-CUBE.md)
- [Portierungs-Roadmap](TODO.md)
- [Horizon Native-Mod-Dokumentation](https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md)

**Lizenzhinweis:** Inoffizieller Port. Rechte und Copyright-Hinweise des [Mekanism-Originalprojekts](https://github.com/mekanism/Mekanism) bei jeder späteren Übernahme von Code oder Assets bewahren. Siehe [LICENSE](LICENSE).
