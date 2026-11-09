# Ingame-Testprotokoll

Horizon Nightly 2026-11-08 `fdc87ae`, Native-Hooks-API 411, Mekanism-Basis 26.3 (`330b7b4d`).
Build: `scripts/Build-Content.ps1` (Content), `scripts/Build-Native.ps1 -TrustNativeCode` (Native).

## 09.11.2026

| Test | Ergebnis |
|---|---|
| Native-Build mit Vertrauensfreigabe, Installation im Launcher | ok |
| `/give` + Platzieren aller 5 Energy Cubes | ok |
| Cube-Drop mit Netherite-Spitzhacke (Loot-Format `condition`) | ok |
| Osmium-Drops, Schmelzen, Barren zu Block | ok |
| Erz- und Salzgenerierung (Flat-Format 26.3, Biome-Tag `spawn_ores`) | ok |
| Block-Entity je Cube (Agent Control `/data`) | ok |
| Creative Cube füllt sich auf `i32::MAX` | ok |
| `energy: 123456` überlebt Speichern + Neustart | ok |
| Energy Tablet speichert Energie | ok |
| Tablet stapelt nicht (`max_stack_size` 1) | ausstehend |
| Debug-Quelle füllt Basic Cube: 4.000/Tick, stoppt bei 4.000.000 (Agent Control) | ok |
| Debug-Senke leert Cube: 4.000/Tick bis 0 | ok |
| Seitenweise Ein-/Ausgabe über Horizons Energie-Pfad | ausstehend (braucht echte Quelle, M04/M05) |

Build-Log: Native-Build `native-fdc87ae-8abc9345573f` (`scripts/Build-Native.ps1 -TrustNativeCode`), Launcher-Log ohne Fehler.

Screenshots: wurden im Chat geteilt, nicht im Repo abgelegt.

## M01 (09.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-f7d32c38dabb` (mit GUI-Patch: Barrel-Template + Block-Menü `mekanism:energy_cube`).

| Test | Ergebnis |
|---|---|
| Original-Modell im Item (Hotbar) | ok (Block-Modell im Spiel nicht einzeln geprüft) |
| Front zeigt zum Spieler (`facing=north`/`east` je nach Blickrichtung) | ok |
| Rechtsklick öffnet GUI: Energie, Balken, I/O-Limit, Ausgabe, 6 Seiten | ok (Screenshot) |
| Linksklick Seite: Out zu Aus, `port_up=false`; Rechtsklick: zurück zu Out, `port_up=true` | ok |
| Debug-Quelle füllt A, A gibt 4.000 FE/t an B: A 0, B 252.000 | ok |
| Abbau (Netherite-Spitzhacke): Item `horizon_energy` + `mekanism_sides`; Platzieren stellt beides her (604.000 → 604.000, 2×) | ok |
| Würfel mit 100.000 FE neu gesetzt, Front zu B: gibt alles an B ab, Summe 504.000 + 100.000 = 604.000 | ok |
| Chunk-Unload (tp 3000/3000 und zurück): Energie und Seiten bleiben | ok |
| Neustart: Seiten | ausstehend (Energie: N09) |
| Rezept ingame craften | ausstehend |
| Tooltip mit Energie | ausstehend (nicht per Agent Control lesbar) |

Einmal sprang B beim ersten Platzieren von 252.000 auf 504.000. Nicht reproduzierbar (2 Wiederholungen ohne Abweichung); Würfel A trug danach `components.custom_data` mit 252.000, wurde also zwischendurch von Hand abgebaut und neu gesetzt. Vermutlich kam die Energie aus diesem Item, nicht aus einer Duplikation.

## M04 Heat und Solar Generator (09.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-c77934992415`; Content-Mods `mekanism` und `mekanismgenerators` (letzterer muss in `horizon-mods.txt` als `mod.mekanismgenerators:true` an sein, neue Mods starten aus).

| Test | Ergebnis |
|---|---|
| Beide Generatoren per `setblock`, Block-Entity wird angelegt | ok |
| Solar (Tag, unter freiem Himmel) über Basic Cube: Energie fließt nach unten in den Cube (110, 535 FE) | ok |
| Heat Generator: Kohle in den Slot wird zu 80 mB Lava je Stück und in 100-mB-Schritten verbrannt | ok |
| Heat Generator: Energie aus der Front in einen Cube (234 FE) | ok |
| Lava an den Seiten (auch fließend) hebt die Temperatur (5 Seiten: 285 K auf etwa 337 K); ohne Lava Rückkehr zur Biom-Temperatur | ok |
| GUI Heat Generator öffnet: Titel, Brennstoff-Slot, Lava-Gauge, Temperatur (285 K), FE/t, Energiebalken | ok (Screenshot) |
| Lava-Eimer im Slot bei nicht leerem Tank bleibt liegen (passt nicht ganz) | ok |
| GUI Solar Generator, Tooltips, Nacht/Regen, Nether, Neustart | ausstehend |
| Bio Generator mit Bio Fuel | ok |
| Sounds Maschinen und Generatoren (inkl. Bio, Wind) | ok |
| Wind Generator Item-Icon | fehlte (Spezial-Renderer), behoben, erneut testen |
| Advanced Solar Generator: Fuß | fehlte (Root-`transform` ignoriert), behoben, erneut testen |
