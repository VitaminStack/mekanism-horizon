# Mitmachen

Mekanism Horizon ist ein inoffizieller Port von [Mekanism](https://github.com/mekanism/Mekanism) (Branch `26.3`) nach [Minecraft Horizon](https://github.com/horizon-rust/horizon). Das Repository ist öffentlich und steht unter MIT ([LICENSE](LICENSE), Details zu Fremdinhalten in [docs/LICENSES.md](docs/LICENSES.md)). Beiträge sind willkommen.

## Ablauf: Fork und Pull Request

1. Repository forken, Fork klonen, Branch von `main` anlegen. Namensschema: `feat/m06-...`, `fix/...`, `docs/...`.
2. Änderung machen, lokal prüfen (siehe unten), in kleinen Commits einchecken. Commit-Stil: Conventional Commits mit Meilenstein, z. B. `feat(m06): Chemical Oxidizer and Chemical Infuser`.
3. Branch pushen, Pull Request gegen `main` öffnen. Beschreibung: was, warum, wie getestet (Tests, Ingame-Nachweis, Log oder Screenshot).
4. `main` ist geschützt: Änderungen nur per Pull Request. Es sind **0 Reviews** und **keine Status-Checks** vorgeschrieben. Prüfe deshalb selbst, ob CI (`.github/workflows/ci.yml`) grün ist, und warte nicht auf eine Freigabe, um ordentlich zu testen.
5. Nach dem Merge aktualisiert ein Workflow den Fortschrittsblock in `README.md` aus `TODO.md`.

Regeln aus [TODO.md](TODO.md): Ein Punkt wird nur abgehakt, wenn er **bestätigt** ist, also ingame mit echten Inputs und Outputs getestet, nicht nur gebaut. Offene Lücken gehören als Notiz an den Punkt und in [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md). Originalverhalten und Registry-IDs aus Mekanism 26.3 prüfen. Content (`.hmod`) und nativen Code trennen.

Lizenz: Wer Code oder Assets aus Mekanism übernimmt, bewahrt Copyright und Lizenztext ([docs/LICENSES.md](docs/LICENSES.md)). Keine Inhalte aus anderen Quellen ohne passende Lizenz.

## Tests

```powershell
python scripts/validate_content.py
cargo test --manifest-path logic/energy/Cargo.toml
```

Beides läuft auch in CI. `scripts/Test-Content.ps1` ruft die Validierung auf. Nach jeder Content-Änderung zusätzlich bauen (prüft mit `cargo horizon-mod check`):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Die Crate `native/mekanism_native/api` enthält Unit-Tests, lässt sich aber nur über den Horizon Launcher bauen (hängt an Horizons Client). Native Änderungen baust du mit:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
```

Das führt nativen Code mit deinen Benutzerrechten aus, siehe [docs/INSTALL.md](docs/INSTALL.md). Das gilt auch für fremde Pull Requests: erst den Code lesen, dann bauen.

## Ingame-Test per Agent Control

Der Launcher legt beim nativen Build das Werkzeug `horizon-ctl.exe` ab:

```
%APPDATA%\horizon-launcher\versions\<native-version>\horizon-ctl.exe   state | command | setblock | quit
```

- Horizon über den Launcher starten, Testwelt öffnen (Kreativ, Energie aus Creative Energy Cubes).
- Vor dem ersten Weltbefehl auf `player.block_pos` im `state` warten. Frühere Befehle stürzen Horizon ab.
- Per `command`/`setblock` Maschinen platzieren, `/give`/`item replace` für Inputs, `/data get block` für Zustand. Neustart und Chunk-Unload (`tp` weit weg und zurück) prüfen.
- Ergebnisse als Tabelle in [docs/TESTLOG.md](docs/TESTLOG.md) eintragen: Datum, Build-Name, Test, Ergebnis (`ok`, `Fehler`, `ausstehend`). Mehrere zusammengehörige Tests bündeln, nicht jeden Handgriff einzeln.

Neue Content-Mods starten in Horizon deaktiviert: in `horizon-mods.txt` als `mod.<id>:true` einschalten.

## Generierter Content: Reihenfolge der Skripte

Ressourcen-Content ist **generiert**. Nicht von Hand ändern, sondern den Generator anpassen und neu laufen lassen. Reihenfolge:

```powershell
python scripts/gen_resources.py
python -I scripts/import_upstream.py <Pfad-zum-Mekanism-26.3-Checkout>
python -I scripts/import_generator_models.py <Pfad-zum-Mekanism-26.3-Checkout>
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

1. `gen_resources.py` schreibt den Horizon-Teil: Registry-Deklarationen, Loot, Ore-Generation, `scripts/registry.json`.
2. `import_upstream.py` kopiert Assets, Rezepte, Tags, Sounds und Namen aus dem Upstream-Checkout (Sparse-Clone reicht, Pfade siehe Docstring des Skripts).
3. `import_generator_models.py` baut die Generator-Modelle (Wind, Solar usw.) aus den Upstream-Modellen.

Die Reihenfolge zählt, weil Schritt 2 die von Schritt 1 geschriebene `registry.json` liest und Schritt 3 auf den importierten Assets aufbaut. Weitere Generatoren: `gen_chemicals.py`, `import_energy_cube.py`, `gen_tools.py` (Tools, [docs/TOOLS.md](docs/TOOLS.md)) und `gen_additions.py` (Additions, [docs/ADDITIONS.md](docs/ADDITIONS.md)), jeweils mit Upstream-Checkout als Argument.

Regeln für Horizon-Content: Tags nur in den Namespaces `mekanism`, `minecraft` und `c`. Loot nutzt Horizons `condition`-Objektform (`apply_bonus`/`match_tool` fehlen). Worldgen im flachen 26.3-Format. Siehe [AGENTS-WINDOWS.md](AGENTS-WINDOWS.md).

## Hinweise

- Warnungen wie `warning: LF will be replaced by CRLF` bei `git add` unter Windows sind harmlos.
- `README.md` zwischen `<!-- progress:start -->` und `<!-- progress:end -->` nicht von Hand ändern. Der Block wird aus `TODO.md` erzeugt (`python scripts/sync_readme_progress.py`).
- Pfade zur Horizon-Instanz gehören nur in `scripts/horizon-game-dir.txt` (ignoriert), nie ins Repository.
- Probleme melden: GitHub-Issue mit Horizon-Stand, Native-Build-Name, Mod-Versionen, Reproduktionsschritten und Log. Bekanntes steht in [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md).
