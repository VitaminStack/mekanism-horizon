# Welle 1 – gebündelte Ingame-Checkliste

Branch `test/wave1-acceptance` (8 Sitzungs-Branches gemergt). Vorher bauen:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Native.ps1" -TrustNativeCode
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\Build-Content.ps1"
```

Der Native-Code der Welle ist **noch nie kompiliert** worden. Compile-Fehler aus dem ersten Build zuerst an die Lead-Session geben.
Kreativwelt, Energie aus Creative Energy Cubes. Details je Punkt in der jeweiligen `docs/sessions/<name>.md`.

## 1. Laden und Content
- [ ] Spiel startet ohne Ladefehler, Native-Mod aktiv.
- [ ] Neue Items/Blöcke mit Modell und Namen im Kreativ-Inventar: Upgrades, Configurator, 32 Factories, 4 Tier Installer, 4 Universal Cables, Transporter (4 Tiers, Restrictive, Diversion), Jetpack, Free Runners, Scuba, Electric Bow, Flamethrower, Dynamic Tank/Valve, Structural Glass, Induction Casing/Port/Cells/Providers, Teleporter, Teleporter Frame, Portable Teleporter, Quantum Entangloporter, Oredictionificator, Formulaic Assemblicator, Crafting Formula, Digital Miner.
- [ ] Additions: Umfärben per Werkbank, Painting Machine und Pigment Extractor mit Plastic; Namen de/en (Additions, Tools).
- [ ] Generators: Turbine Valve/Vent, Fission/Fusion Port, Logic Adapter, Fusion Controller craften, setzen, abbauen (ohne Funktion).

## 2. Maschinen-Framework (`m07-machine-framework.md`)
- [ ] Enrichment Chamber: Energy-Slot, Upgrade-Slots, Knöpfe `RS`, `S/E/M` sichtbar; alte Maschine aus früherem Spielstand lädt ohne Verlust.
- [ ] Energy Tablet im Energy-Slot lädt die Maschine.
- [ ] Speed x8: 20 Ticks je Operation; Energy-Upgrade ändert Kapazität; Muffling macht stumm; Upgrade per Klick zurück.
- [ ] Redstone High/Low/Pulse; Modus bleibt nach Neustart.
- [ ] Configurator dreht Maschine/Cube zur geklickten Seite.

## 3. Factories (`m07-factories.md`)
- [ ] Basic Enriching Factory: 3 Prozesse parallel; voller Output pausiert.
- [ ] Combining/Compressing/Sawing: Extra-Slot, Tank, Nebenausgabe; Ultimate-Layout (9 Spalten).
- [ ] Tier Installer: Chamber → Basic Factory behält Inhalt; nichts doppelt oder verloren.
- [ ] Neustart/Chunk-Unload behält Fortschritt, Energie, Tank; Abbau ohne Duplikat.

## 4. Kabel und Transporter (`m08-cables-transport.md`)
- [ ] Solar Generator → Kabel → Cube lädt; Arme verbinden sich.
- [ ] Transporter Truhe A → B (Sneak+Rechtsklick „Pull“), Tier-Tempo, volles Ziel, Diversion mit Redstone.
- [ ] Abbau mit Items: Items fallen; Neustart: Items laufen weiter.

## 5. Ausrüstung (`m11-gear.md`)
- [ ] Jetpack an Chemical Tank (Wasserstoff) füllen, fliegen, Modi Hover/Vector/Disabled.
- [ ] Free Runners: kein Fallschaden (Energie sinkt), Step-Assist.
- [ ] Scuba Maske + Tank unter Wasser; Electric Bow (120 FE/Schuss); Flamethrower (Kegel, Feuer).
- [ ] Verzauberungen bleiben nach Laden/Verbrauch erhalten.

## 6. Multiblocks (`m09-multiblocks.md`)
- [ ] Dynamic Tank 3×3×3 formt sich, Valve in Fläche, Pipe füllt, Abbau löst auf, Inhalt kehrt zurück.
- [ ] Induction Matrix 3×3×3 mit Zelle + Provider formt sich; Port Input/Output; Kapazitätsanzeige.
- [ ] Chunk-Unload löst Strukturen nicht auf.

## 7. Teleporter (`m08-teleport-frequency.md`)
- [ ] Zwei Teleporter mit Rahmen, gleicher Kanal: Spieler wird teleportiert, Energie sinkt; zu wenig Energie → kein Teleport.
- [ ] Portable Teleporter: Kanal wechseln (Sneak), teleportieren.
- [ ] Zwei Entangloporter gleicher Frequenz teilen Energie/Fluid/Chemie/Item; Neustart behält Frequenzen.

## 8. Spezialmaschinen (`m08-special-machines.md`)
- [ ] Oredictionificator: Filter auf Tag, Eingang wird zum gewählten Item; Filter nach Neustart.
- [ ] Digital Miner: 17 Bounding-Blöcke, Radius/minY/maxY, Abbau mit Filter, Silk Touch, Auswurf hinten, Neustart.
- [ ] Formulaic Assemblicator: nur Block + Item, ohne Funktion (Blocker: keine Vanilla-Crafting-Rezepte für Native-Mods).

## Bekannte Lücken dieser Welle
- Logistical Sorter, Thermal Evaporation Plant, Seismic Vibrator/Reader: nicht gebaut.
- Factories und Framework haben je eigene Upgrade-Logik (`factory.rs`, `upgrade.rs`): nach der Abnahme zusammenführen.
