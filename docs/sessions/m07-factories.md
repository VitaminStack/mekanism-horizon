# M07 Factories (Session m07-factories)

Branch `claude/m07-factories-bb48a4`. Stand: Code und Content committet, **ingame ungetestet, nativ nicht kompiliert** (siehe Lücken).

## Was portiert wurde

32 Factories (Basic/Advanced/Elite/Ultimate x Enriching, Crushing, Compressing, Combining, Purifying, Injecting, Infusing, Sawing) und 4 Tier Installer.

| Modul | Inhalt |
| --- | --- |
| `native/mekanism_native/api/src/factory.rs` | Horizon-frei: Tier-/Typ-Tabellen, Slot-Layout, GUI-Maße, Energiekapazität, Schritt auf geteilter Energie, Umzug der Items beim Upgrade. 16 Unit-Tests, eingebunden in `logic/energy`. |
| `native/mekanism_native/api/src/factories.rs` | Host: Tick, Container, Screen, Zeichnen, Tooltip, Dump-Button, Tier-Installer-Rechtsklick. |
| `lib.rs`, `energy.rs` | Nur additive Zeilen: `Kind::Factory`, je eine Zeile pro Hook, `factory_tick`, Energie-Akzeptor. |
| `scripts/gen_resources.py` | Ein markierter Block: 32 Blöcke, 32 Menüs, 4 Installer-Items. Modelle, Blockstates, Rezepte, Tags, Namen kommen über `import_upstream.py` (Rezepte `factory/<tier>/`, `tier_installer/`). |

## Werte und ihre Quelle (Mekanism 26.3)

- Prozesse 3/5/7/9, Slot-Start x 55/35/29/27, Abstand 38/26/19/19: `FactoryTier.java`, `TileEntityItemToItemFactory.java`, `TileEntitySawingFactory.java`.
- Eingabe y 13, Ausgabe y 57, Sägewerk-Nebenausgabe y 77, geteilter Slot (7, 57): dieselben Dateien sowie `TileEntityCombiningFactory.java`, `TileEntityItemStackChemicalToItemStackFactory.java`.
- Energie: `Factory.java` (`max(Speicher*0.5, Verbrauch) * Prozesse`), Verbrauch je Prozess = Basis-Maschine, `AttributeEnergy.java`.
- Dauer 200 Ticks je Vorgang: `TileEntityFactory.BASE_TICKS_REQUIRED`.
- Geteilter Chemie-Tank `MAX_GAS`/`MAX_INFUSE` x Prozesse: `TileEntityItemStackChemicalToItemStackFactory.getInitialChemicalTanks`.
- GUI: Breite +34 bei Ultimate, Höhe +11 mit Tank, +21 beim Sägewerk, Inventar-Versatz 85/95/105 (x 26 Ultimate), Energieleiste `width-12`, 16, Höhe 52/73, Fortschritt DOWN 8x20 bei `4+x`, 33, Tankleiste (7,76), Dump (148/182, 76, 21x10): `GuiFactory.java`, `FactoryContainer.java`, `GuiDumpButton.java`.
- Tier Installer: `ItemTierInstaller.java`, `AttributeUpgradeable` in `Factory.java` (Maschine nur Basic, Factory Tier N nur Installer N+1).

## Verhalten

- Pro Prozess eigener Fortschritt (`mekanism_factory_progress`, IntArray) und eigene Ein-/Ausgabe. Die Prozesse laufen je Tick der Reihe nach und ziehen aus einem Energiepuffer; wenn die Energie fehlt, stehen die hinteren.
- Kein Rezept: Fortschritt 0. Ausgabe voll oder Chemie fehlt: pausiert, Fortschritt bleibt.
- Geteilter Chemie-Tank (Compressing, Purifying, Injecting, Infusing) wird über den geteilten Slot gefüllt, ein Item je Tick; Dump-Button leert ihn. Combining: der geteilte Slot ist der Extra-Input.
- Sawing: Nebenausgabe pro Prozess mit Chance (`machine::random`).
- Brummen und `active`-Modell wie bei den Maschinen. Loot: nur der Block (Standard-Loot des Generators).
- Rezepte, Chemie- und Verbrauchswerte kommen aus den Basis-Maschinen (`machine::find`, `Recipes::processing`), nicht doppelt gepflegt.

## Lücken und Blocker

- **Smelting-Factory fehlt**: braucht `minecraft:smelting`-Rezepte und den Energized Smelter, beides nicht erreichbar.
- **Nativ nicht kompiliert.** Der Launcher-Build wurde auf Anweisung der Leitung nicht ausgeführt. Geprüft nur gegen die Signaturen im Horizon-Quelltext (Rev `fdc87ae`) und gegen die vorhandenen Muster in `process.rs`/`bins.rs`. Erster Schritt beim Test: Build-Fehler beheben.
- **Tier Installer**: `set_block` auf einen anderen Block legt laut Horizon-Quelltext (`native_set_block`) den neuen Block-Entity im selben Aufruf an; `change_mod_block_entity` füllt ihn. Der alte Entity wird vorher geleert, damit nichts auf den Boden fallen kann. Nicht bewiesen: ob Horizon beim Ersetzen Container-Inhalt droppt (im Quelltext nichts gefunden). Ingame prüfen.
- Nicht portiert: Energie-Slot (7, 13), Auto-Sortierung und Sortier-Tab, Seiten-Konfiguration, Upgrades, Rezeptbuch, Fortschrittswarnungen.
- Hopper und Shift-Klick füllen jeden Eingabe-Slot ohne Rezeptprüfung (Mekanism filtert). Rezeptprüfung beim Tick: ein Item ohne Rezept bleibt liegen.
- Menü-Daten sind 16-Bit; größter Wert (Tank Ultimate Infusing 9000) passt.
- Benötigte Änderungen an fremden Dateien: keine. `process::BUFFER`, `BUFFER_TYPE` und `machine`-Felder werden nur gelesen. Falls `m07-machine-framework` `Machine`-Felder umbenennt, müssen `factory.rs`/`factories.rs` nachziehen.
- Generator-Lauf zog fremden Altstand mit (Transmitter-Einträge in `mineable/pickaxe.json`, `blocks_motion_no_leaves.json`); harmlos, vom Importer erzeugt.

## Ingame-Checkliste

Nach nativem Build, Creative, Energie aus Creative Energy Cube:

1. Alle 32 Blöcke im Creative-Tab, Modell und Name stimmen, `active` schaltet das Modell.
2. Basic Enriching Factory: Erz in alle 3 Eingaben, Energie anlegen: alle 3 Fortschrittspfeile laufen, nach 10 s je ein Dust. Ausgabe voll: Pfeil bleibt stehen, nach Leeren weiter.
3. Zu wenig Energie (z. B. 10 FE): nur die ersten Prozesse laufen. Energieleiste und Tooltip prüfen (Kapazität 30 bei Basic Enriching).
4. Combining: Extra-Item im Slot (7,57), Verbrauch je Operation.
5. Compressing/Purifying/Injecting/Infusing: Item im Tank-Slot füllt die Leiste, Tooltip zeigt Menge, Dump leert. Verbrauch pro Operation.
6. Sawing: Nebenausgabe erscheint unten (y 77), Höhe 187.
7. Ultimate: Breite 210, Slots 9 Spalten, Inventar bei x 26.
8. Neustart und Chunk-Unload: Fortschritt, Energie, Tank bleiben.
9. Hopper: oben füllt, unten leert, Seite füllt den geteilten Slot.
10. Block abbauen: Drop nur der Block (Inhalt fällt als Items, kein Duplikat).
11. Tier Installer: Basic auf Enrichment Chamber gefüllt (Items, Energie, Fortschritt bleiben, Blickrichtung bleibt), Advanced auf Basic Factory, Ultimate-Factory bleibt unverändert, falscher Tier wird ignoriert. Nicht-Creative: Installer wird verbraucht. Prüfen, dass nichts doppelt oder verloren geht.
12. Loop-Sound läuft nur während der Arbeit.
