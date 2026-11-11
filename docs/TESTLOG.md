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
| GUI Solar Generator, Tooltips, Nacht/Regen, Nether, Neustart | ok |
| Bio Generator mit Bio Fuel | ok |
| Sounds Maschinen und Generatoren (inkl. Bio, Wind) | ok |
| Wind Generator Item-Icon | behoben, ok |
| Advanced Solar Generator: Fuß | behoben, ok |
| Advanced Solar, Wind Generator (Multiblock, Rotor), Enrichment Chamber, Crusher | ok |

## P0-Refactor: Maschinen-Framework, Rezepte, Energie (09.11.2026)

Build: `scripts/Build-Content.ps1`, `scripts/Build-Native.ps1 -TrustNativeCode`.

| Test | Ergebnis |
|---|---|
| Metallurgic Infuser, Enrichment Chamber, Crusher nach dem Refactor (ein Tick, ein Rezept-Cache) | ok |
| Energie Cube zu Maschinen, Generatoren zu Cube und Maschine | ok |
| Wind/Advanced Solar abbauen: Bounding-Blöcke verschwinden sofort (`set_block` statt `edit_blocks`) | ok |
| Abbau eines Bounding-Blocks im Kreativmodus droppt nichts | ok |
| GUI über jeden Bounding-Block (gespiegeltes Menü `mekanism:bounding_block`) | ok |

## M06 Tanks (09.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-482f2e77aeb3` (Branch `feat/m06-chemistry-foundation` nach Rebase auf P0), Content aus demselben Stand, Kreativwelt `TankTest`.

| Test | Ergebnis |
|---|---|
| Fluid/Chemical Tank: Block-Entity, Tick (`mekanism_mode`) | ok |
| Debug Chemical Source füllt Basic Chemical Tank mit Wasserstoff | ok |
| Fluid Tank nimmt keinen Wasserstoff an | ok |
| Chemical-Tank-GUI: Titel, Inhalt, Transfer, Modus-Knopf schaltet | ok |
| Dumping Excess hält 85 % (54.400 von 64.000 mB) | ok |
| Dumping leert 160 mB/Tick | ok |
| Abbauen (Survival) gibt Item mit Inhalt; Platzieren stellt ihn wieder her | ok |
| Chemical Tank dreht sich zum Platzierenden | ok |
| Creative Chemical Tank bleibt voll (`i32::MAX`) | ok |
| Fluid Tank „Auto-eject down“: 5.000 mB in den Tank darunter, Ventil `active=true` | ok |
| Wasser-Eimer füllt Fluid Tank (2 × 1.000 mB, leerer Eimer zurück); leerer Eimer entnimmt 1.000 mB; Lava-Eimer in Wassertank abgelehnt (Retest `native-fdc87ae-c2fbcfc22770`, Eimer über `right_click_block`) | ok |
| GUI-Layout: Name und Menge in eigenen Zeilen, Anzeige endet über „Inventory“ (Retest) | ok |
| Boden unter Tanks sichtbar (`occludes: false`, Retest) | ok |
| Flüssigkeit im Fluid Tank sichtbar: Wasser und Lava je nach Füllstand, Füllstand folgt Änderungen, leerer Tank zeigt nichts (`native-fdc87ae-2f1bbd36b98c`) | ok |
| Andere Flüssigkeiten: Sole als Wasser-Box in eigener Farbe, Füllstand stimmt (`native-fdc87ae-1ac2734b93a0`) | ok |
| Flüssigkeit mit Shaderpack (Bliss) | Fehler: falsch dargestellt; ohne Shader korrekt. Liegt an Horizons Shader-Pfad für `draw_fluid_box`, nicht am Mod |

## M12 Tools (10.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-352025dc49dc`, Content `mekanismtools` 0.1.0 neben `mekanism` und `mekanismgenerators`, Kreativwelt `TankTest` (auf Survival gestellt).

| Test | Ergebnis |
|---|---|
| Content lädt (91 Items, 107 Rezepte, Tags), keine Fehler im Log | ok |
| Angriffsschaden und -tempo (`attribute`): Bronze-Spitzhacke 4, Refined-Obsidian-Schwert 12, Bronze-Axt 10, Lapis-Hacke 1/2,0, Steel-Schaufel 5,5, Osmium-Speer 5, Diamant-Paxel 10 | ok (stimmt mit Mekanism-Formel) |
| Refined-Obsidian-Set an: Rüstung 30 (Deckel), Zähigkeit 20, Rückstoßresistenz 0,8 | ok |
| Rüstungs-Textur auf dem Spielermodell (Inventar-Screenshot), Item-Icons | ok |
| Haltbarkeit: Bronze-Spitzhacke bricht nach 375, Refined-Obsidian-Spitzhacke nach 4096 | ok |
| Mining-Stufe: Bronze baut Diamanterz ab, Obsidian nicht; Steel baut Obsidian ab | ok |
| Diamant-Paxel schält Eichenstamm (Axt) | ok |
| Diamant-Paxel formt Gras zum Pfad (Schaufel) | Fehler/Blocker: nur Axt-Aktion |
| Rezept: 3 Bronzebarren + 2 Stöcke ergeben Bronze-Spitzhacke (Tag `#c:ingots/bronze`) | ok |
| `recipe give` für Paxel-, Rüstungs-, Schild-, Barren-Rezepte | ok |
| Schilde im Inventar: Vanilla-Schildmodell statt eigener Textur | bekannte Abweichung |
| Schild blocken, Speer-Angriff, Pferde-/Nautilus-Rüstung am Tier, Reparatur am Amboss, Verzauberungstisch, Neustart, Multiplayer | ausstehend |

## M05 Osmium Compressor, Combiner, Precision Sawmill (09.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-352025dc49dc`, Content aus Branch `feat/m05-sawmill-combiner-compressor`, Kreativwelt `TankTest`, Energie aus Creative Energy Cubes.

| Test | Ergebnis |
|---|---|
| Compressor: Osmiumbarren im unteren Slot wird zu 200 mB Osmium; Refined-Obsidian-Staub zu Barren, 1 mB/Tick (Fortschritt 157, Osmium 43) | ok |
| Compressor: zweiter Barren wird nachgeladen, wenn Platz ist; leerer Tank stoppt (`active=false`) | ok |
| Compressor: Osmiumblock (1.800 mB) wird nicht in den 210-mB-Tank umgewandelt | ok |
| Compressor-GUI: Titel, Slots, Osmium-Balken, Fortschritt, Energie (Screenshot) | ok |
| Combiner: 8 Rohes Eisen + Bruchstein aus Tiefenschiefer zu Tiefenschiefer-Eisenerz, + Bruchstein zu Eisenerz | ok, nach Fix (fehlende `c:`-Tags) |
| Combiner: anderes Item im Output-Slot pausiert, nach Leeren läuft es weiter | ok |
| Sawmill: 6 Eichenstämme zu 36 Brettern + 1 Sägespäne (Chance 0,25) | ok, nach Fix (`mekanism:sawdust` registriert) |
| Sawmill: Bett pausiert, solange Sägespäne den Zweitslot belegen; danach Bretter + 3 Wolle | ok |
| Enrichment Chamber: Eisenerz zu 2 Eisenstaub (neuer Tag `c:ores/iron`); Infuser: Kohle + Eisenbarren zu Enriched Iron | ok |
| Neustart: Inhalte, Osmium-Tank und Fortschritt (Combiner 182) bleiben | ok |
| Sounds | nicht geprüft (Agent Control hört nicht) |

## M06 Electrolytic Separator, Rotary Condensentrator (10.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-6132d14d5002`, Content aus Branch `feat/m06-separator-condensentrator`, Kreativwelt `TankTest`, Energie aus Creative Energy Cubes (Front zur Maschine), Tankinhalte per `/data merge`.

| Test | Ergebnis |
|---|---|
| Separator: 5.000 mB Wasser zu Wasserstoff (links) und Sauerstoff (rechts) in zwei Chemical Tanks, Verhältnis 2 : 1, 2 mB Wasser je Tick | ok |
| Separator: Sole 100 mB zu 10 Natrium + 10 Chlor (10 Durchläufe) | ok |
| Separator: linker Tank bei 2.399 mB Wasserstoff, Wasser unverändert, Maschine pausiert | ok |
| Separator-GUI: Titel, Eingangs-Gauge mit Wasser, zwei Output-Gauges, Energie (Screenshot) | ok |
| Rotary Chemical -> Fluid: 500 mB Wasserdampf zu Wasser, ausgeworfen in angrenzenden Fluid Tank (1 mB/Tick) | ok |
| Rotary Modus-Knopf im GUI schaltet (`mekanism_rotary_mode` 0 zu 1), Beschriftung wechselt, Fluid -> Chemical: 300 mB Wasser zu Wasserdampf im angrenzenden Chemical Tank | ok |
| Modus bleibt nach Neustart (1) | ok |
| Erste GUI-Fassung: Gauges überdeckten den Titel, Knopftext ragte aus dem Knopf | behoben (Retest ok bis auf wenige Pixel Knopfbreite, danach verbreitert, nicht erneut geprüft) |
| Sounds, Tooltips im Spiel, Crafting-Rezepte (Separator braucht `electrolytic_core`, Rotary `mek_data` mit Tanks), Chunk-Unload | nicht geprüft |

## M06 Purification Chamber, Chemical Injection Chamber (10.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-52fd95314c56`, Content aus Branch `feat/m06-purification-injection`, Kreativwelt `TankTest`, Energie aus Creative Energy Cubes (Front zur Maschine).

| Test | Ergebnis |
|---|---|
| Purification: Flint im unteren Slot wird zu 10 mB Sauerstoff; 2 Rohes Eisen zu 4 Eisen-Klumpen (`clump_iron`), 1 mB/Tick | ok |
| Purification: Tank leer, Fortschritt hält (100), läuft nach Flint weiter | ok |
| Injection: Salz wird zu 2 mB Chlorwasserstoff, Schießpulver zu Schwefelstaub (`gunpowder_to_sulfur`), Stillstand bei leerem Tank, danach weiter | ok |
| Purification-GUI: Titel, Slots, Balken, Energie (Screenshot) | ok |
| Block-Entity-Daten, Tank-Typ (`mekanism_infuse_type`) und Menge | ok |
| `#mekanism:shards/*`-Rezepte (Tag nicht ausgeliefert) laden ohne Fehler im Log | ok |
| Sauerstoff-Oxidation von Kupfer, Wasserdampf-Rezepte, Sounds, Neustart | nicht geprüft (Wasserdampf braucht eine Maschine, die ihn herstellt) |


## M12 Additions (10.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-52fd95314c56` (nur Content nötig), Content `mekanismadditions` (`scripts/Build-Content.ps1`, `cargo horizon-mod check` ok), Kreativwelt `TankTest`, Testfläche auf Y 150.

| Test | Ergebnis |
|---|---|
| Mod lädt, 271 Blöcke per `setblock` (Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slab oben, Fence, Gate, Glow Panel, Obsidian TNT) | ok, keine Log-Warnungen (Screenshot) |
| Vorgefärbte Texturen: Farben stimmen (Aqua, Rot, Gelb, Weiß, Grün, Blau, Lila, Orange, Lime, Rosa, Cyan) | ok |
| Transparent-Blöcke und -Slabs durchscheinend | ok |
| Fences verbinden mit Nachbar-Fence, Gate öffnet sich als Modell | ok (visuell) |
| Licht: Glow-Block 10, Glow Panel 15 (`horizon-ctl light`) | ok |
| Glow Panel `facing=north/up`, wasserfähig; fällt ohne Stützblock ab und droppt Item | ok |
| Survival-Abbau ohne Werkzeug: Double Slab droppt 2, Glow-Block 1 (Loot-Format `modifier` von Upstream) | ok |
| Obsidian TNT mit Redstoneblock gezündet: explodiert wie Vanilla-TNT (Krater Radius ~2 in Glattstein) | ok, aber Radius 4 statt 12 (Blocker, siehe `docs/ADDITIONS.md`) |
| `/recipe give` Stairs-Rezept (`plastic/stairs/red`) | ok (Rezept geladen) |
| Rezept im Crafting-Grid craften | ausstehend |
| Slick-Rezept in der Enrichment Chamber (Plastic -> Slick, Rezept liegt im Namespace `mekanismadditions`) | ausstehend |
| Slick-Reibung (0,98) auf dem Block | ausstehend (Bewegung nicht per Agent Control messbar) |
| Block-Items im Inventar/Hotbar (Modell, Färbung) | ausstehend (nur `/give` geprüft) |
| Neustart, Chunk-Unload | ausstehend |

## M06 Chemical Oxidizer, Chemical Infuser (10.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-51be04f8dfaa`, Content aus Branch `feat/m06-oxidizer-chem-infuser`, Kreativwelt `TankTest`, Energie aus Creative Energy Cubes. Items per `item replace block ... container.0` (das kleingeschriebene NBT `items` wird vom Tick nicht gelesen).

| Test | Ergebnis |
|---|---|
| Oxidizer: 3 Schwefelstaub zu 300 mB Schwefeldioxid (100 mB je Staub, 100 Ticks), Fortschritt läuft, Energie bleibt 60 | ok |
| Oxidizer wirft den Output in einen angrenzenden Chemical Tank aus (300 mB `sulfur_dioxide`) | ok |
| Oxidizer neben Infuser: Infuser nimmt Schwefeldioxid in den linken Tank (`accepts` nach Rezepten), 500 bis 1.100 mB | ok |
| Infuser: 100 mB Sauerstoff + Schwefeldioxid ergeben Schwefeltrioxid (1 O + 2 SO2 zu 2 SO3, 63 Durchläufe), Tank-Reihenfolge egal | ok |
| Infuser: Output-Tank bei 9.999 mB, Eingänge unverändert, Maschine pausiert | ok |
| GUIs: drei Gauges (Infuser) bzw. Slot, Balken, Gauge (Oxidizer), Energie (Screenshots) | ok |
| Neustart: Tankinhalte (Schwefeldioxid, Sauerstoff, Schwefeltrioxid) bleiben | ok |
| Erster Build: Deadlock im Server-Thread, weil der Eject (`fluid_insert`) den `accepts`-Hook des Nachbarn unter dem Rezept-Lock aufrief; Eject jetzt nach dem Lock | behoben |
| Sounds, Tooltips, Crafting-Rezepte (`mek_data` mit Dynamic Tank) | nicht geprüft |

## M12 Generators: Bauteile (10.11.2026, per Agent Control `horizon-ctl`)

Build: Content `mekanismgenerators` 0.2.0 mit `scripts/Build-Content.ps1` (`cargo horizon-mod check` und `validate_content.py` ok), Native `native-fdc87ae-6132d14d5002`, Kreativwelt `TankTest`. Erster Start lief mit einem veralteten Paket in `horizon-mods` (geteilter Ordner, von einer parallelen Session überschrieben): neue Blöcke unbekannt; nach Neubau und Neustart ok.

| Test | Ergebnis |
|---|---|
| Content lädt mit 4 Mods, keine Content-Warnungen zu `mekanismgenerators` (einzige Warnung: `mekanism:salt`, bekannt) | ok |
| `setblock` aller 11 neuen Blöcke, Zustand wird zurückgemeldet | ok |
| `give` Turbine Blade und Hohlraum | ok |
| `recipe give` `turbine/blade`, `hohlraum`, `reactor/frame` (Rezept mit fehlendem Tag `#c:pellets/polonium`) | ok, Rezepte existieren, Laden ohne Fehler |
| Modelle, Texturen, Item-Icons sichtbar | nicht geprüft (Screenshots zeigten nur Gelände, Welt unter Wasser/Nacht) |
| Craften im Crafting-Tisch, Hohlraum im Metallurgic Infuser, Abbau-Drop | ausstehend |

## M08 Bins, Personal Chest, Personal Barrel (11.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-a5d971619c14` (zuvor `...55c418ea8e62`, `...e85ab11785b6`), Content mit `scripts/Build-Content.ps1`, Kreativwelt `TankTest`, Plattform bei Y 98.

| Test | Ergebnis |
|---|---|
| Hopper über Basic Bin füllt ihn, Fenster-Slot hält bis zu einem Stack, Rest in `mekanism_bin_count` | ok |
| Rechtsklick vorn legt die Hand ein (64 Kobblestone); Rechtsklick auf Seite/hinten tut nichts | ok |
| Linksklick vorn (Survival) gibt 1 Item, Menge sinkt | ok |
| Abbau (Survival, Eisenspitzhacke): Item behält Inhalt (`mekanism_bin_item/count/lock` in `custom_data`, 126), Eingabe-Slot fällt als Item; Platzieren stellt ihn wieder her (64 im Fenster + 62) | ok |
| Eject: Sneak-Rechtsklick auf die Seite schaltet `active=true`; Hopper darunter füllt eine Kiste (39 Eisen in 4 s) | ok |
| Sneak + leere Hand vorn sperrt den Typ (Actionbar „Bin locked: Cobblestone“); Sperre bleibt bei leerem Bin | ok |
| Creative Bin: nimmt Diamant als Typ, Menge `i32::MAX`, verbraucht nichts aus der Hand | ok |
| Bin ohne Lock vergisst den Typ, wenn Hopper/Eject ihn leeren; zweiter Typ im Eingangsslot blockierte erst (Fehler, behoben: `accepts` prüft auch das Fenster) | behoben |
| Item auf der Front (Cobblestone) und Zahl „10“ sichtbar; nachts blass | ok |
| Neustart: Inhalt, Sperre und Typ des Bins bleiben | ok |
| Personal Chest/Barrel: Platzieren setzt `mekanism_owner` (UUID) und Namen; Chest dreht sich nach Platzierer | ok |
| Personal Chest öffnet 9x6-Menü „Personal Chest“; Shift-Klick legt Items ein; Abbau gibt Items und Chest | ok |
| Fremder Besitzer (NBT auf Fantasie-UUID): Menü öffnet trotzdem, weil der Testspieler Operator ist (gewollt) | Ablehnung nicht prüfbar (nur Singleplayer) |
| Hopper am Personal Chest | nicht geprüft |
| Crafting-Rezepte (`#c:chests/wooden` und `#c:barrels/wooden` ergänzt), Tooltip mit Inhalt, Sounds | nicht geprüft |
| Chargepad | nicht portiert (siehe KNOWN_ISSUES) |
## M04 Gas-Burning Generator, M06 Electric Pump, Fluidic Plenisher (11.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-f49dcb5728e7`, Content aus Branch `feat/m04-gas-generator-pump` (`scripts/Build-Content.ps1`), Kreativwelt `TankTest`, Energie aus Creative Energy Cubes vor der Maschine.

| Test | Ergebnis |
|---|---|
| Gas-Burning Generator mit 1.000 mB Hydrogen: Tank leert sich (zu 288 mB nach wenigen Ticks), Energy Cube vor der Front lädt (1.436 FE) | ok |
| Pump über Wasserquelle: 7.000 mB in 5 s, `mekanism_pump_type` 1, Quelle bleibt (Standardregel `water_source_conversion`) | ok |
| Pump über Lava: Knoten wachsen auf die Nachbarquellen, Lava wird verbraucht, 5.000 mB gehen nach oben in einen Fluid Tank | ok |
| Plenisher (Wasser, 10.000 mB) in 3 x 3 x 2 Grube: setzt Quellen, Weg im NBT, Wasserfluss füllt Rest; mit Nachfüllung `finished` 1, Listen gelöscht | ok |
| Fertiger Plenisher setzt entferntes Lava unter sich neu (1.000 mB Abzug je Block) | ok |
| GUI Plenisher: Titel, Gauge in Lavafarbe, Status „Finished“, Fluidname, Menge, Energiebalken (Screenshot) | ok |
| Pump-GUI, Tooltips, Sound, Crafting-Rezepte, Neustart (Tank/Weg bleiben), Chunk-Unload | nicht geprüft |
| Erster Content-Stand wurde von einem parallelen Build einer anderen Session im geteilten `horizon-mods`-Ordner überschrieben (Blöcke „unknown“); neu gebaut | behoben |
## M06 Dissolution Chamber, Washer, Crystallizer und Erzkette (11.11.2026, per Agent Control `horizon-ctl`)

Build: Native `native-fdc87ae-1c7ad4927bd3`, Content aus Branch `feat/m06-ore-processing`, Kreativwelt `TankTest`, Energie aus Creative Energy Cubes (Front zur Maschine), Tankinhalte per `/data merge`, Tick-Rate per `/tick rate` (bis 1.000/s) für die langen Läufe. Die Maschinen stehen nebeneinander und werfen ihre Output-Tanks in den Nachbarn aus (Dissolution, Washer, Crystallizer).

| Test | Ergebnis |
|---|---|
| Dissolution: 1 Eisenerz + 100 mB Schwefelsäure (1 mB je Tick, 100 Ticks) ergibt 1.000 mB schmutzigen Eisen-Schlamm, ausgeworfen in den Washer (Säure 1.000 zu 900) | ok |
| Washer: Schlamm + Wasser (5 mB je 1 mB Schlamm, ein Durchlauf je Tick) ergibt sauberen Schlamm, ausgeworfen in den Crystallizer (Wasser 6.000 zu 1.000) | ok |
| Crystallizer: 200 mB sauberer Schlamm je Eisenkristall, 200 Ticks, 5 Kristalle aus einem Erz | ok |
| Kette weiter: 5 Kristalle + Chlorwasserstoff (Injection Chamber) zu 5 Splittern, + Sauerstoff (Purification Chamber) zu 5 Klumpen, Crusher zu 5 schmutzigen Stäuben, Enrichment Chamber zu 5 Stäuben, Ofen zu **5 Eisenbarren aus einem Eisenerz** (Vanilla-Ofen mit dem importierten `from_dust_smelting`) | ok |
| Crystallizer-Output voll (64 Goldkristalle): Maschine pausiert, Schlamm wird weiter eingefüllt (1.000 zu 1.063 mB) | ok |
| Dissolution ohne Säure: Fortschritt hält (81), läuft nach Nachfüllen weiter | ok |
| GUIs: Titel, Gauges (Wasser blau, Schlamm gelb, Säure oliv), Item-Slot, Fortschrittsbalken, Energie (Screenshots) | ok |
| Sounds, Tooltips, Crafting-Rezepte, Weltneustart, Chunk-Unload der neuen Maschinen | nicht geprüft |
