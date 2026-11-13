# Changelog

Alle nennenswerten Änderungen, gruppiert nach Meilenstein aus [TODO.md](TODO.md). Quelle: Git-Verlauf und gemergte Pull Requests (#1 bis #16). Bekannte Lücken: [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md). Das Format folgt locker [Keep a Changelog](https://keepachangelog.com/). Es gibt noch keine getaggte Version; Content-Mods `mekanism` und `mekanismgenerators` tragen 0.2.0, `mekanismtools` und `mekanismadditions` 0.1.0.

Basis: Mekanism 26.3 (Mekanism 10.8.0 alpha), Horizon Nightly 2026-11-08 `fdc87ae`.

## [Unreleased] (Stand 11.11.2026)

### M06 Nutrition und Pigmente
- Nutritional Liquifier, Pigment Extractor, Pigment Mixer und Painting Machine (nativ, Tabelle `machine::MACHINES`) mit 40 Nahrungs-, 275 Extraktions-, 16 Misch- und 288 Painting-Rezepten, Item `dye_base`; ingame ungetestet (Testplan in `docs/TESTLOG.md`).

### M06 Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator
- Drei native Chemie-Maschinen mit Rezepten, Modellen, Sounds und Namen aus Upstream (`mekanism:reaction`, `centrifuging`, `activating`), neue Items `substrate` und `hdpe_pellet`, Generators-Chemicals (Deuterium, Tritium, Fusion Fuel) in der Chemie-Tabelle. Ingame ungetestet, siehe [docs/CHEMISTRY.md](docs/CHEMISTRY.md) und [docs/TESTLOG.md](docs/TESTLOG.md).
- Fix: fehlende Klammer in `kind()` (`lib.rs`) aus einem Merge von `main`, das native Crate kompilierte nicht.

### M06 Ingame-Abnahme (13.11.2026)
- Nutrition/Pigmente, Pipes/Tubes/Conductors und Advanced Chemistry ingame abgenommen (Ergebnisse in `docs/TESTLOG.md`, „M06 Abnahme“).
- Fix: Pipes füllen Tanks und Maschinen am Ende mit der vollen Rate (vorher etwa die Hälfte durch die Füllanteil-Regel); Tubes bekommen keinen Arm mehr zu Fluid Tanks.

### M11 Ausrüstung und Spielhilfen
- Hook-Audit und Machbarkeitsmatrix aller M11-Items in `docs/ITEM_HOOKS.md` (Horizon `fdc87ae`, API 411).
- Atomic Disassembler (Energiepuffer, Waffenplus, Vein-Modus), Canteen, Gauge Dropper und Dictionary als native Items (`items.rs`, `gear.rs`) mit Assets, Rezepten und Namen aus Upstream. Nicht machbare Items (Radiation, Module, QIO) bleiben unregistriert, siehe `docs/KNOWN_ISSUES.md`.

### M08 Lager
- Bins (Basic bis Creative), Personal Chest und Personal Barrel (Besitzer-Schutz ohne Security Desk). Chargepad offen, siehe [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md).

### M13 Dokumentation
- Installationsanleitung, Known Issues, Contributing, Lizenzprüfung und dieser Changelog.

### M04 Generatoren
- Gas-Burning Generator (nativ): Chemical-Tank, Brennwerte Hydrogen/Ethene, GUI, Sound; `electrolytic_core` und `electrolytic_separator` craftbar.

### M06 Chemie und Maschinen
- Electric Pump und Fluidic Plenisher (nativ, `pump.rs`/`pumps.rs`): Wasser und Lava aus/in die Welt, Tank 10.000 mB, Breitensuche wie Upstream.
- Chemical Dissolution Chamber, Chemical Washer und Chemical Crystallizer; Kristall-, Splitter- und schmutzige-Staub-Items für 7 Metalle; Erzkette (1 Erz zu 5 Barren) ingame getestet, siehe [docs/ORE_PROCESSING.md](docs/ORE_PROCESSING.md).
- Chemical Oxidizer und Chemical Infuser ([#16](https://github.com/VitaminStack/mekanism-horizon/pull/16)). Chemie-Ausgabe in Output-Tanks, zwei Chemie-Inputs; ein Deadlock beim Auswerfen unter dem Rezept-Lock wurde behoben.
- Purification Chamber und Chemical Injection Chamber mit Klumpen-Items ([#14](https://github.com/VitaminStack/mekanism-horizon/pull/14)).
- Fluid- und Chemical-Tabellen aus Upstream, Fluid und Chemical Tanks (Auto-Eject, Dumping-Modi, Eimer, Fluid-Rendering, GUI) ([#10](https://github.com/VitaminStack/mekanism-horizon/pull/10), [#11](https://github.com/VitaminStack/mekanism-horizon/pull/11)).

### M12 Tools und Additions
- Content-Mod `mekanismadditions` 0.1.0: 271 Blöcke (Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slabs, Fences, Gates, Glow Panels, Obsidian TNT), Texturen je Farbe vorgefärbt ([#15](https://github.com/VitaminStack/mekanism-horizon/pull/15)).
- Content-Mod `mekanismtools` 0.1.0: 91 Items (Paxel, Werkzeug- und Rüstungssets in 6 Materialien, Schilde, Speere, Pferde- und Nautilus-Rüstung), 107 Rezepte ([#13](https://github.com/VitaminStack/mekanism-horizon/pull/13)).

### M05 Maschinen
- Osmium Compressor, Combiner und Precision Sawmill ([#12](https://github.com/VitaminStack/mekanism-horizon/pull/12)). Fixes: Sägespäne-Item, fehlende `c:`-Tags.

### P0 Framework
- Gemeinsames Maschinen-Framework: ein Tick, ein Rezept-Cache, Energie-Akzeptoren; Multiblock-Fixes (Bounding-Blöcke) ([#9](https://github.com/VitaminStack/mekanism-horizon/pull/9)).

### M02 Maschinen und Ressourcen
- Enrichment Chamber und Crusher mit Maschinen-Sounds ([#8](https://github.com/VitaminStack/mekanism-horizon/pull/8)).
- Metallurgic Infuser, Infuse-Typen und Infusions-Rezepte, Block-Sounds nach Mekanism ([#5](https://github.com/VitaminStack/mekanism-horizon/pull/5)).
- Rohstoffe: Erze, Barren, Staub, Blöcke, Legierungen und Worldgen nach 26.3-Format, Assets, Rezepte, Tags und Namen aus Upstream (en_us, de_de) (Commit `cd48129`, [#1](https://github.com/VitaminStack/mekanism-horizon/pull/1) bis [#3](https://github.com/VitaminStack/mekanism-horizon/pull/3) Vorarbeit).

### M04 Generatoren
- Heat Generator und Solar Generator ([#7](https://github.com/VitaminStack/mekanism-horizon/pull/7)).
- Bio Generator mit Bio Fuel, Advanced Solar Generator und Wind Generator (Multiblock mit Rotor) ([#8](https://github.com/VitaminStack/mekanism-horizon/pull/8)).

### M03 und M01 Energie
- Alle fünf Energy-Cube-Tiers und Energy Tablet ([#2](https://github.com/VitaminStack/mekanism-horizon/pull/2)).
- Basic Energy Cube vollständig: GUI, Seiten-I/O, Item-Energie, Speichern/Chunk-Reload ([#4](https://github.com/VitaminStack/mekanism-horizon/pull/4), [#5](https://github.com/VitaminStack/mekanism-horizon/pull/5)). Debug-Energiequelle und -senke zum Testen.

### M00 Projektbasis
- Basic Energy Cube Prototyp v0.2.0 (Content + native Block-Entity und Energie) und Projektdokumentation ([#1](https://github.com/VitaminStack/mekanism-horizon/pull/1)).
- CI: Content-Validierung und Energie-Tests (`.github/workflows/ci.yml`); Workflow, der den Fortschritt in `README.md` aus `TODO.md` erzeugt ([#3](https://github.com/VitaminStack/mekanism-horizon/pull/3), [#6](https://github.com/VitaminStack/mekanism-horizon/pull/6)).
- Build-Skripte für Content und Native, Testprotokoll `docs/TESTLOG.md`.

## Pull Requests

| PR | Datum | Titel |
|---|---|---|
| #1 | 08.11.2026 | Basic Energy Cube prototype v0.2.0 and project documentation |
| #2 | 09.11.2026 | Native-Test, Testprotokoll und Debug-Energieblöcke (N05, N08, N10) |
| #3 | 09.11.2026 | N-Abschluss: Energy-Cube-Ingame-Test (N05-N10) |
| #4 | 09.11.2026 | M01: Basic Energy Cube |
| #5 | 09.11.2026 | Metallurgic Infuser, infusing recipes and block sounds (M02) |
| #6 | 09.11.2026 | docs: sync confirmed progress |
| #7 | 09.11.2026 | Heat and Solar Generator (M04) |
| #8 | 09.11.2026 | M02/M04: Enrichment Chamber, Crusher, Bio, Advanced Solar and Wind Generator |
| #9 | 09.11.2026 | P0: machine framework, recipe cache, energy acceptors; multiblock fixes |
| #10 | 09.11.2026 | M06: fluid/chemical tables, fluid and chemical tanks |
| #11 | 09.11.2026 | docs: tick confirmed TODO items, update README progress |
| #12 | 10.11.2026 | M05: Osmium Compressor, Combiner, Precision Sawmill |
| #13 | 10.11.2026 | feat(m12): Mekanism Tools content mod |
| #14 | 10.11.2026 | feat(m06): Purification Chamber and Chemical Injection Chamber |
| #15 | 10.11.2026 | feat: Mekanism Additions content mod (M12) |
| #16 | 10.11.2026 | feat(m06): Chemical Oxidizer and Chemical Infuser |

## 0.2.0 (08.11.2026)

Erster Prototyp: Osmium (Erz, Barren, Block), Basic Energy Cube mit nativem Begleiter, Windows-Build-Skripte, Dokumentation.
