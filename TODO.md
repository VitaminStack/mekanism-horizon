# Mekanism Horizon - Portierungs-TODO

**Stand:** 09.11.2026  
**Ziel:** Schrittweise Portierung des aktuellen Mekanism nach Minecraft Horizon, mit Windows/Visual Studio 2026 als Entwicklungsumgebung.

## Referenz und Arbeitsregeln

- **Original:** [mekanism/Mekanism, Branch `26.3`](https://github.com/mekanism/Mekanism/tree/26.3), abgeglichen mit Commit [`31bb4906`](https://github.com/mekanism/Mekanism/commit/31bb4906bc6c2ea4a2422d93cd1214df3ea7de4e).
- **Originalversion aus `gradle.properties`:** Mekanism **10.8.0**, Minecraft **26.3**, NeoForge **26.3.0.101**, `release_type=alpha`. Dies ist eine Entwicklungsbasis, nicht automatisch eine stabile Release-Version.
- **Horizon-Basis:** [`horizon-rust/horizon` @ `fdc87ae`](https://github.com/horizon-rust/horizon/commit/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca) (Nightly 2026-11-08, installiert im Launcher), Native-Hooks-API 411; der Native-Mod wird gegen diesen Commit gebaut und ingame getestet. Pin steht in `native/mekanism_native/horizon-mixin.toml` und `api/Cargo.toml`.
- **Geplanter Umfang:** Mekanism Core und danach die Original-Unterprojekte **Generators, Tools, Additions**.
- **Status:** `[x]` bedeutet **bestätigt**. `[ ]` bedeutet offen, auch wenn schon Code/Dateien existieren. Nicht nur wegen eines erfolgreichen Builds abhaken.
- **Jede Umsetzung:** Originalverhalten und Registry-IDs prüfen; Content (`.hmod`) und native Logik (`.hnmod`) sauber trennen; speichern/neuladen testen; Tests + Ingame-Nachweis im PR dokumentieren.

## Die nächsten 10 konkreten Aufgaben - zuerst diese erledigen

- [x] **N01** Lokalen Energy-Cube-Patch-Zustand prüfen: `git status --short` und `git apply --reverse --check .\mekanism-energy-cube-v0.2.0.patch`. Vorhandene Dateien NICHT überschreiben. *(09.11.2026: Working Tree sauber, Patch-Inhalt ist bereits committet, Patch-Datei existiert nicht mehr.)*
- [x] **N02** Nach dem Patch `Test-Content.ps1` und `cargo test --manifest-path .\logic\energy\Cargo.toml` erneut erfolgreich ausführen.
- [x] **N03** `Strg+B`: `.hmod` bauen, automatisch kopieren; `/give @s mekanism:basic_energy_cube` im Spiel testen und Block platzieren. *(09.11.2026: give, platzieren und Drop bestätigt; Loot-Tabellen brauchen Horizon-Format `condition`.)*
- [x] **N04** Native-Horizon-API-Version/Nightly-Commit bestimmen und `native/mekanism_native/` darauf abstimmen. *(09.11.2026: gepinnt auf Horizon `fdc87ae` (installierter Nightly); Hook-Signaturen nur gegen 99bd9655 (API 413) statisch geprüft. Kompilieren erst in N05.)*
- [x] **N05** Native-Mod mit dem Horizon Launcher bauen und mit bewusst erteilter Vertrauensfreigabe installieren; Logs prüfen. *(09.11.2026: `scripts/Build-Native.ps1 -TrustNativeCode`, Build `native-fdc87ae-8abc9345573f`, Ingame bestätigt.)*
- [x] **N06** Beim Platzieren wird eine native Block-Entity mit Energie-Capability angelegt. *(09.11.2026 per Agent Control: alle 5 Cubes haben Block-Entity.)*
- [x] **N07** Basic Energy Cube: 4.000.000 Kapazität, 4.000 Ein-/Ausgabe-Limit pro Aufruf (Standard aus Original-`EnergyCubeTier`; später konfigurierbar).
- [x] **N08** Energie mit Test-Hook/-Generator einfuegen und entnehmen; Seiten, Limits, Leer/Voll testen. *(09.11.2026: `debug_energy_source`/`debug_energy_sink` per Agent Control: 4.000/Tick, Deckel bei 4.000.000, Entleeren bis 0. Schreiben die Cube-NBT direkt; Horizons eigener Seiten-Pfad (`insert_faces`) ungetestet.)*
- [x] **N09** Welt speichern, Horizon beenden und wieder starten: gespeicherte Energie muss erhalten bleiben. *(09.11.2026: `energy: 123456` überlebt Speichern + Neustart.)*
- [x] **N10** Erfolgreichen Ingame-Test, Build-Log und Screenshots dokumentieren, committen, PR erstellen. *(09.11.2026: Protokoll in `docs/TESTLOG.md`; Screenshots nur im Chat.)*

## M00 - Funktionierende Projektbasis und Qualitaet [P0]

- [x] Eigenes Windows-Repository mit `content/`, `logic/`, `scripts/` eingerichtet.
- [x] Rust/Cargo unter Windows installiert und vier Energy-Core-Unit-Tests bestanden.
- [x] Content-Mod per `Strg+B` in Visual Studio 2026 bauen.
- [x] `.hmod` nach erfolgreichem Build automatisch in `horizon-mods/` kopieren.
- [x] Osmiumbarren und Osmiumblock im Spiel getestet.
- [x] Lokale v0.2.0-Änderungen sicher versionieren; Remote und lokal abgleichen. *(09.11.2026: Arbeitsbaum sauber, alle Branches gepusht.)*
- [x] Horizon-Nightly, Native-Hook-API und Mekanism-Upstream-Commit pinnen.
- [x] Test- und Buildanleitung für Windows dokumentieren. *(`README-WINDOWS.md`.)*
- [x] CI: Content-Validierung, Rust-Tests, JSON-Checks; Native-Build wenn reproduzierbar. *(`.github/workflows/ci.yml`; Native-Build nicht in CI, braucht Launcher.)*
- [x] Lizenz und Copyright der Original-Assets/Codeuebernahmen prüfen und beilegen. *(MIT, `content/mekanism/LICENSE-MEKANISM`.)*

## M01 - Basic Energy Cube: erster wirklich funktionierender Block [P0]

- [x] Block, Itemmodell, Texture, Loot, Übersetzungen und Rezept stimmen. *(Original-Modell ohne LEDs/Kern, Loot wie Original ohne Explosionsbedingung, Original-Rezept. Ingame: Rezept, Tooltip ok; fehlende `c:`-Tags ergänzt.)*
- [x] Native Block-Entity beim Platzieren und Entfernen korrekt erzeugen/entfernen.
- [x] Horizon Energie-Hook (ab API 142) mit `ModEnergyBuffer` anbinden. *(Angebunden; Seiten nur pro Blocktyp, nicht pro Block-Entity.)*
- [x] Kapazität, Eingabe, Ausgabe und Simulations-Aufrufe korrekt begrenzen. *(Unit-Tests `cube::transfer`; Fremd-Energiequelle ingame nicht verfügbar.)*
- [x] Alle Seiten/Facing testen; Eingabe/Ausgabe konfigurieren. *(Facing + Seitenkonfig per GUI, getestet 09.11.2026.)*
- [x] NBT-Persistenz bei Save, Chunk-Unload und Neustart.
- [x] Energieübergabe ohne Duplikation oder Verlust. *(Eject nur an Mekanism-Cubes; Summe ingame konstant; Unit-Tests `cube::transfer`.)*
- [x] GUI: Speicherstand, Transferlimit, I/O-Seiten und Status. *(Eigener Screen über Block-Menü `mekanism:energy_cube`.)*
- [x] Verhalten beim Abbau/Neuplatzieren mit Original vergleichen. *(Item behält Energie + Seiten wie Original.)*
- [x] Integrationstests + Ingame-Demonstration abschliessen. *(A→B-Übergabe, GUI, Abbau, Chunk-Unload, Neustart ingame, 09.11.2026.)*

**Fertig, wenn:** Der Cube nimmt echte Energie auf, gibt sie ab, kann konfiguriert werden und behaelt den Ladezustand nach Neustart.

## M02 - Rohstoffe, Weltgenerierung und Grundrezepte [P1]

- [x] Originale Registry-IDs von Items/Blöcken/Ressourcen inventarisieren. *(Ressourcen-IDs nach Mekanism 26.3 in `scripts/gen_resources.py`, Liste in `scripts/registry.json`.)*
- [ ] Originaltexturen, Modelle, Blockstates, Sounds und Sprachen mit Lizenzhinweisen integrieren. *(Für Ressourcen per `scripts/import_upstream.py`; MIT-Lizenz in `content/mekanism/LICENSE-MEKANISM`. Block-Sound-Typen wie Original (Stone, Deepslate, Salz = Sand; ingame getestet 09.11.2026). Energy-Cube-Modell per `scripts/import_energy_cube.py`. Sounds: `sounds.json` + `.ogg` der vorhandenen Maschinen und Generatoren (Infuser, Enrichment Chamber, Crusher, Compressor, Combiner, Precision Sawmill, Purification und Injection Chamber, Heat, Solar) per `import_upstream.py` (`SOUNDS`), mit Untertiteln; Horizon spielt Mod-Sound-Events seit API 61, der Native-Mod wiederholt den Clip, solange die Maschine läuft. Sounds inkl. Bio und Wind ingame getestet 09.11.2026. Die übrigen Mekanism-Sound-Events folgen mit ihren Maschinen/Items.)*
- [x] Osmium-, Tin/Zinn-, Uranium-/Uran- und Fluorite-/Fluorit-Erze inklusive Deepslate-Varianten. *(Plus Blei und Salz.)*
- [x] Weltgenerierung, Erzverteilung, Drops und Mining-Level (nur neue Chunks) testen. *(Werte aus Original-`OreType`/`WorldConfig`. Kein Glück/Behutsamkeit: Horizon-Loot kennt `apply_bonus`/`match_tool` nicht.)*
- [x] Ingots, Nuggets, Dusts, Raw Materials, Storage Blocks (z.B. Bronze, Steel, Refined Glowstone, Refined Obsidian).
- [ ] Steel Casing, Control Circuits (Basic/Advanced/Elite/Ultimate), Alloys und Enriched Materials. *(Registriert. Infusing-Rezepte laufen über den Metallurgic Infuser (Grundstufe aus M05; ingame getestet 09.11.2026: Carbon, Enriched Iron, Steel Dust, Steel Casing, Dump, Tooltips). Elite/Ultimate Circuits und Reinforced/Atomic Alloy: Kette Crusher (Obsidian-/Diamant-Staub) und Enrichment Chamber (Enriched Diamond/Refined Obsidian) implementiert, ingame ungetestet.)*
- [x] Crafting-, Smelting-, Blasting- und Resource-Tags nach Original-Rezepten. *(Ingame bestätigt 09.11.2026: Steel Ingot im Ofen, Steel Casing, Infuser-Rezept. Vanilla-Rezepte + Tags importiert, dazu `metallurgic_infusing` und Infuse-`chemical_conversion`. Fehlende NeoForge-`c:`-Tags für Vanilla-Items in `gen_resources.py` (`CONVENTION_TAGS`; seit 09.11.2026 alle von importierten Rezepten genannten, z.B. `c:ores/iron`, `c:raw_materials/iron`, `c:dyes/*`). Weitere Maschinenrezepte folgen mit ihren Maschinen.)*
- [ ] Survival-Test: alle Materialien für erste Maschinen ohne Cheat erhältlich. *(Weg: Infuser craften, Kohle zu Carbon, Eisen zu Enriched Iron, dann Steel Dust, Steel Ingot (Ofen), Steel Casing; Osmium + Redstone zu Basic Circuit. Energie ohne Cheat aus Heat Generator (Kohle) direkt neben der Maschine; ohne Universal Cables nur als Nachbarblock. Ingame ungetestet.)*

## M03 - Energie- und Maschinen-Fundament [P0/P1]

- [x] Joule/FE-Konvertierung, Overflow-Schutz und Einheiten in UI/API festlegen. *(Entschieden 09.11.2026: 1:1 FE wie Mekanism 26.3, das keine Joule mehr hat (`docs/ENERGY-UNITS.md`). Overflow-Schutz in `energy::Acceptor::accept` mit Unit-Tests; Creative Cube auf `i32::MAX` begrenzt.)*
- [x] Energie in Items und Blöcken über gemeinsame Capabilities. *(Horizons Energie-API: `item_energy_buffer` für Tablet und Cube-Items, `block_entity_energy` für Cubes und Infuser; ingame getestet 09.11.2026.)*
- [x] Energy Tablet und alle Energy Cubes: Basic, Advanced, Elite, Ultimate, Creative. *(Getestet 09.11.2026. Platzhalter-Optik; Horizon zeigt keinen Item-Ladebalken; Creative auf i32::MAX begrenzt; gecraftete Cubes übernehmen keine gespeicherte Energie.)*
- [x] Original-Tier-Werte, Transferlimits und Konfigurationen übernehmen. *(Tier-Werte aus `EnergyCubeTier`, Tablet aus `GearConfig`; nicht konfigurierbar.)*
- [ ] Universal Cables (vier Tiers), Verbindungsgraph und Nachbar-Updates. *(Blockiert: Horizon-Native-API bietet Mods keine Energie-Helfer (`native_move_energy` ist spielintern).)*
- [ ] Energieverteilung über Chunks, Server-Ticks und Redstone korrekt.
- [x] Gemeinsames Block-Entity-, Recipe-, Inventory- und Save/Load-Framework. *(P0-Refactor, ingame getestet 09.11.2026: ein `Kind`-Dispatch für alle Hooks, Maschinen-Tabelle `machine::MACHINES` mit einem Tick/Container/Screen (`process.rs`), Rezept-Cache je Typ (`recipe.rs`), NBT-Helfer für Energie/Fortschritt/Puffer. Tanks (M06) nutzen denselben Dispatch.)*
- [ ] Lasttest: 100+ Cables und Cubes; keine Energie-Duplikation bei Netz-Reload.

## M04 - Energieerzeugung / Generators-Grundstufe [P1]

- [x] Heat Generator (Brennstoff, Umgebung und Ausgabeseiten). *(Ingame getestet 09.11.2026: eigener Content-Mod `content/mekanismgenerators` mit Upstream-IDs, Logik in `native/mekanism_native/api/src/generator.rs`. Lava-Tank 1000 mB, 100 mB/t, Lava-Nachbarn 7/Seite, Nether +10, Wärme-/Carnot-Modell wie Original, Energie aus der Front in Cube/Maschine. Nicht portiert: Energie-Slot, Redstone, Wärmeübertragung an Nachbarn.)*
- [x] Solar Generator und Advanced Solar Generator. *(Solar Generator ingame getestet 09.11.2026: 2 FE/t Spitze, Biom-Faktor aus Vanilla-Klimatabelle, Tageszeit/Regen nach Vanilla-Formel, Energie nach unten. Advanced Solar ingame getestet 09.11.2026: 12 FE/t Spitze, Multiblock mit Bounding-Blöcken (Säule + 3×3-Paneel), braucht mehr als 4 der 9 Himmelsprüfungen, Speicher 240 FE; Abbau eines Teils baut alles ab, fehlender Platz lässt den Block als Item fallen. Modell: Fuß als Blockmodell, Oberteil vom Native-Mod einen Block höher gezeichnet (Horizon ignoriert das Root-`transform` des Originals), per `scripts/import_generator_models.py`.)*
- [x] Wind Generator (Höhe/Wetter nach Original). *(Ingame getestet 09.11.2026: 1–5 FE/t linear nach Höhe der Spitze (y 24 bis Weltobergrenze), Spitze muss Himmel sehen, Speicher 100 FE, Multiblock 5 hoch, Sound (Wetter). Modell: Basis als Blockmodell, Turm und Rotor per `scripts/import_generator_models.py` aus dem Blockbench-Original, vom Native-Mod gezeichnet, Rotor dreht sich im Betrieb. Item-Icon: verkleinertes Vollmodell statt Mekanisms Spezial-Renderer. Bildschirm über jeden Bounding-Block (gespiegeltes Menü), Abbau ohne Geisterblock, kein Drop im Kreativmodus: ingame getestet 09.11.2026 (P0-Refactor). Nicht portiert: `c:no_wind`-Dimensionen.)*
- [x] Bio Generator und Bio Fuel. *(Ingame getestet 09.11.2026: 25 FE/t, 1 mB/t aus 200-mB-Tank, 64 mB je Bio Fuel, Speicher 600 FE, Energie aus der Front, Sound. Bio Fuel aus dem Crusher (alle `crushing/biofuel`-Rezepte), Block of Bio Fuel. Nicht portiert: Befüllen per Fluid-Container/Pipe, Energie-Slot, Redstone; Block of Bio Fuel als Ofen-Brennstoff.)*
- [x] Gas-Burning Generator. *(Ingame getestet 11.11.2026: 1.000-mB-Chemical-Tank, Hydrogen 2 und Ethene 8 FE je mB, höchstens 10 mB/t nach Füllstand, Speicher 2.000 FE, Energie aus der Front in einen Energy Cube, Sound, GUI. Nicht portiert: Chemie-/Energie-Slots, Tank nur an allen Seiten statt ohne Front (Horizon-Flächen je Blocktyp), Redstone. Details `docs/GENERATORS.md`.)*
- [ ] GUIs, Energieabgabe, Upgrades und Balancing der Generatoren.
- [ ] Survival-Test: Generator -> Universal Cable -> Energy Cube -> Maschine.

## M05 - Maschinen und erste Processing-Ketten [P1]

- [ ] Allgemeines Rezept-System: Inputs, Outputs, Energie je Tick, Fortschritt, Nebenprodukte. *(Teilweise: `recipe::Recipe` liest Item-Input mit Menge, optionalen Chemie-Input und Output für alle Maschinen-Rezepttypen; Energie je Tick und Fortschritt über `machine::step`, ingame getestet 09.11.2026. Seit M05-Teil 2 auch zweiter Item-Input (`extra_input`), Zweitausgabe mit Chance (`secondary_output`) und Chemie pro Tick (`per_tick_usage`); ingame getestet 09.11.2026. Seit M06 auch Purification/Injection (`purifying`, `injecting`; Sauerstoff, Chlorwasserstoff, Wasserdampf je Tick; ingame getestet 10.11.2026). Seit dem Oxidizer/Infuser (M06) auch Chemie-Ausgabe in einen Output-Tank (`Recipe::chemical_output`, `FluidTanks`) und zwei Chemie-Inputs (`recipe::Infusion`); voller Output-Tank pausiert wie ein voller Item-Output. Seit Separator/Rotary auch zwei Chemie-Outputs aus einem Fluid (`recipe::Separation`, `machine::convert`) und Fluid-Tanks (`recipe::Rotation`). Fehlt: Fluid-Outputs der Pumpe und Item-Outputs mit Chemie-Zweitausgabe.)*
- [ ] Maschinen-Inventare, Menüs, GUI, Upgrades, Auto-Eject, Sided I/O, Redstone-Modi.
- [x] Metallurgic Infuser und Infusions-Ressourcen. *(Ingame getestet 09.11.2026. Werte aus Original; 8 Infuse-Typen, alle `metallurgic_infusing`-Rezepte. Fehlt: Energie-Slot, Upgrades, Side-Config, Redstone; Infuse/Energie bleiben beim Abbau nicht am Item; Balkenfarben genähert.)*
- [x] Enrichment Chamber. *(Ingame getestet 09.11.2026: `native/mekanism_native/api/src/machine.rs`, 5 FE/t, 200 Ticks, Speicher 20 FE, alle `mekanism:enriching`-Rezepte, Upstream-Modell mit `active`, Sound. Fehlt wie beim Infuser: Energie-Slot, Upgrades, Side-Config, Redstone.)*
- [ ] Energized Smelter. *(Blockiert: Horizon gibt Native-Mods keine `minecraft:smelting`-Rezepte; `recipes_of_type` liefert nur Typen außerhalb von `minecraft`, `native_recipes` ist intern.)*
- [x] Crusher. *(Ingame getestet 09.11.2026: wie Enrichment Chamber, 8 FE/t, Speicher 32 FE, alle `mekanism:crushing`-Rezepte.)*
- [x] Precision Sawmill. *(Ingame getestet 09.11.2026: 6 Eichenstämme zu 36 Brettern + 1 Sägespäne, Bett zu Brettern + Wolle; Pause, solange die Zweitausgabe nicht passt. 5 FE/t, Speicher 20 FE, alle `mekanism:sawing`-Rezepte, Zweitausgabe nach `secondary_chance`, Sound. Fehlt wie beim Infuser: Energie-Slot, Upgrades, Side-Config, Redstone.)*
- [x] Osmium Compressor und Combiner. *(Ingame getestet 09.11.2026: Refined-Obsidian-Staub zu Barren, Osmium 1 mB/Tick, Stillstand bei leerem Tank, Osmiumblock passt nicht in den Tank; Rohes Eisen + (Tiefenschiefer-)Bruchstein zu Erz; Pause bei belegtem Output; Inhalt nach Neustart. Compressor: 15 FE/t, Speicher 60 FE, Osmium-Tank 210 mB aus Osmiumbarren (200 mB), 1 mB je Tick, alle `mekanism:compressing`-Rezepte. Combiner: 5 FE/t, zweiter Input unten, alle `mekanism:combining`-Rezepte. Voller Output pausiert jetzt wie im Original, statt den Fortschritt zurückzusetzen (alle Maschinen).)*
- [ ] Formulation/Assembly: Formulaic Assemblicator und Oredictionificator.
- [ ] Leere Energie, voller Output, Rezeptwechsel, Save/Load und Chunk-Unload prüfen. *(Teilweise ingame: voller Output pausiert (Combiner, Sawmill), Neustart behält Inhalt, leerer Chemie-Tank hält den Fortschritt (Compressor, Purification, Injection). Offen: Rezeptwechsel, Chunk-Unload.)*

## M06 - Fluids, Chemicals und Rohstoffverarbeitung [P1]

**Gruppen (je eigener Branch, in dieser Reihenfolge):**

1. `feat/m06-chemistry-foundation` - Chemie-IDs/Registry, Fluids, Chemicals, Fluid Tank, Chemical Tank.
2. `feat/m06-transport-heat` - Mechanical Pipes, Pressurized Tubes, Thermodynamic Conductors, Heat-System.
3. `feat/m06-basic-chemical-machines` - Electric Pump, Fluidic Plenisher, Electrolytic Separator, Chemical Oxidizer, Chemical Infuser, Rotary Condensentrator.
4. `feat/m06-ore-processing` - Purification Chamber, Chemical Injection Chamber, Dissolution Chamber, Washer, Crystallizer, Erzverarbeitung End-to-End.
5. `feat/m06-advanced-chemistry` - Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator.
6. `feat/m06-nutrition-pigments` - Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine; danach Tank-/Durchsatz-/Neustart-Tests.


- [x] Chemie-IDs/Registry sowie Netzwerk-/Datapack-Synchronisation planen. *(Plan in `docs/CHEMISTRY.md`, Tabellen per `scripts/gen_chemicals.py` (18 Fluids, 64 Chemicals). Chemicals laufen als `mekanism:chemical/<id>`-Fluids in Horizons Block-Entity-Tanks, Inhalt per Update-Tag zum Client; ingame getestet 09.11.2026 mit Wasserstoff und Sole.)*
- [ ] Fluids: Water, Steam, Brine, Lithium, Heavy Water, Nutritional Paste etc. *(IDs, Namen und Farben in `chemicals_table.rs`; Tanks speichern und zeigen sie. Offen: Herstellung durch Maschinen, Eimer für Mod-Fluids (Horizon hat keine).)*
- [ ] Chemicals: Hydrogen, Oxygen, Chlorine, Sulfur-Komponenten, Ethene, Slurries, Pigments, Isotope etc. *(IDs, Namen und Farben in `chemicals_table.rs`; Chemical Tank speichert sie. Oxygen und Hydrogen Chloride füllen Purification/Injection Chamber vorerst aus Items (Flint, Salz), Osmium den Compressor. Offen: Herstellung durch Maschinen.)*
- [x] Fluid Tank und Chemical Tank (Basic/Advanced/Elite/Ultimate/Creative). *(Ingame getestet 09.11.2026 per `horizon-ctl` (`docs/TESTLOG.md`, M06 Tanks): Füllen, Eimer (Wasser/Lava), Auto-eject nach unten, Dumping/Dumping Excess, Creative bleibt voll, Item behält Inhalt, Flüssigkeit im Glas sichtbar. Tier-Werte per Unit-Test. Nicht portiert: Item-Füll-Slots, Chemie-Side-Config, Edit-Mode, Redstone; Lava-Tank gibt kein Blocklicht; mit Shaderpack falsch dargestellt (Horizon).)*
- [ ] Mechanical Pipes und Pressurized Tubes (vier Tiers).
- [ ] Thermodynamic Conductors und Heat-System.
- [x] Electric Pump und Fluidic Plenisher. *(Ingame getestet 11.11.2026 (`docs/TESTLOG.md`): Pump 2 FE/t, Speicher 8 FE, eine Quelle je 19 Ticks, Wasser bleibt (`water_source_conversion`), Lava wird verbraucht, folgt Quellen im Radius 80, gibt 256 mB/t nach oben in einen Fluid Tank; Plenisher 20 Ticks je Block, Breitensuche nach unten, danach Nachfüllen. Logik `pump.rs`, Host `pumps.rs`. Nicht portiert: Upgrades, Schwerwasser-Filter, Item-Slots, Redstone, Sneak-Reset; Grenzen in `docs/KNOWN_ISSUES.md`.)*
- [x] Electrolytic Separator. *(Ingame getestet 10.11.2026: 4 FE/t (2 x Wasserstoff-Brennwert 2), Speicher 16 FE, ein Durchlauf je Tick, alle `mekanism:separating`-Rezepte (Wasser zu 2 Wasserstoff + 1 Sauerstoff, Sole 10 mB zu Natrium + Chlor), Eingang 24.000 mB, linker/rechter Output je 2.400 mB, Auswurf in alle Nachbarn (1.024 mB/t), voller Output pausiert, Eingang nur Rezept-Fluids. Fluid-Eingang per Tank-API (`block_entity_fluids`): Nachbarn und Pipes füllen ihn; Wasser-Eimer fehlt (Horizon-Hook nur für Tanks). Nicht portiert: Item-Slots (Fluid/Chemie/Energie), Dump-Modi links/rechts, Upgrades, Side-Config, Redstone; Deuterium-Rezept (`mekanismgenerators:deuterium`, `energy_multiplier` 2) folgt mit dem Fusion-/Generators-Chemie-Satz.)*
- [x] Chemical Oxidizer und Chemical Infuser. *(Ingame getestet 10.11.2026: Oxidizer 15 FE/t, 100 Ticks, alle `mekanism:oxidizing`-Rezepte (Schwefeldioxid 100 mB je Staub), Output-Tank 10.000 mB; Infuser 15 FE/t, ein Durchlauf je Tick, `chemical_infusing` in beide Tank-Reihenfolgen, drei Tanks je 10.000 mB. Chemie liegt in Horizons Block-Entity-Tanks (`fluids`): Nachbar-Tanks und Maschinen füllen die Infuser-Eingänge (`block_entity_accepts_fluid` nach Rezepten), der Output wird in alle Nachbarn ausgeworfen (1.024 mB/t); voller Output pausiert; Inhalt nach Neustart. Fehlt: Chemie-Item-Slots (Tanks per Item füllen/leeren), Energie-Slot, Upgrades, Side-Config/Auto-Eject-Schalter (immer alle Seiten), Redstone, Rezeptpaar-Prüfung beim Einfüllen (Horizons Hook kennt den anderen Tank nicht), Pfeil-Animation im Infuser; Crafting-Rezepte (`mek_data` mit Dynamic Tank) fehlen bis Dynamic Tank/Personal Storage existieren, Test per `/give`. Gaugen-Layout vereinfacht (176x166 statt Mekanisms größerem GUI).)*
- [x] Rotary Condensentrator. *(Ingame getestet 10.11.2026: 2 FE/t, Speicher 8 FE, ein Durchlauf je Tick, alle `mekanism:rotary`-Rezepte in beide Richtungen, je ein Tank (Chemical links, Fluid rechts) mit 10.000 mB, Modus-Knopf im GUI (Standard Chemical -> Fluid, bleibt nach Neustart), der Output-Tank wirft in Nachbarn aus; Fluid-Ausgabe in Wasser-/Fluid-Tank und Chemie-Ausgabe in Chemical Tank. Blocker/Grenze: Horizons `block_entity_accepts_fluid` und die Tank-Seiten sind statisch, deshalb füllen und entleeren Nachbarn beide Tanks unabhängig vom Modus (Mekanism sperrt den Output-Tank für Einfüllen). Nicht portiert: Item-Slots, Upgrades, Side-Config, Redstone.)*
- [x] Purification Chamber und Chemical Injection Chamber. *(Ingame getestet 10.11.2026 (`docs/TESTLOG.md`): Eisen zu Klumpen mit Sauerstoff, Schießpulver zu Schwefel mit Chlorwasserstoff, Stillstand bei leerem Tank. Purification 10 FE/t, Injection 15 FE/t, Tank 210 mB, 1 mB je Tick, alle `purifying`-/`injecting`-Rezepte, Klumpen-Items. Chemie vorerst nur aus Items (Flint: Sauerstoff, Salz: Chlorwasserstoff); Wasserdampf-Rezepte laufen erst mit einer Maschine, die ihn herstellt. Fehlt: Energie-Slot, Upgrades, Side-Config, Redstone.)*
- [x] Chemical Dissolution Chamber, Chemical Washer, Chemical Crystallizer. *(Ingame getestet 11.11.2026 (`docs/TESTLOG.md`): Dissolution 16 FE/t, 100 Ticks, Säure 1 mB je Tick, Erz zu 1.000 mB schmutzigem Schlamm; Washer 5 FE/t, ein Durchlauf je Tick, 5 mB Wasser je mB Schlamm; Crystallizer 15 FE/t, 200 Ticks, 200 mB sauberer Schlamm je Kristall; alle `dissolution`-, `washing`-, `crystallizing`-Rezepte, Tanks je 10.000 mB, Output-Tanks werfen in Nachbarn aus, voller Output pausiert, leere Säure hält den Fortschritt. Neue Items: Kristalle, Splitter, schmutzige Stäube für 7 Metalle; Schlämme standen schon in der Chemie-Tabelle. Fehlt: Chemie-Item-Slots, Energie-Slot, Upgrades, Side-Config, Redstone.)*
- [ ] Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator.
- [ ] Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine.
- [x] Erzverarbeitung und Mehrfachertraege (bis zu Original-Spitzenwerten) als End-to-End-Kette. *(Ingame getestet 11.11.2026: 1 Eisenerz zu 5 Eisenbarren über Dissolution, Washer, Crystallizer, Injection, Purification, Crusher, Enrichment, Ofen; Beschreibung in `docs/ORE_PROCESSING.md`. Grenzen: Energized Smelter blockiert (Vanilla-Ofen), keine Pipes (Nachbarn/Hand), Schwefelsäure und Wasser per `/data merge` statt aus der vollen Oxidizer/Infuser/Rotary-Kette.)*
- [ ] Volle Tanks, Durchsatzlimits, Chemie-/Fluid-Verluste und Weltneustarts testen.

## M07 - Factories, Upgrades, Automatisierung [P2]

- [ ] Basic/Advanced/Elite/Ultimate Factories für relevante Processing-Typen.
- [ ] Parallele Verarbeitung, Slot-Belegung, Upgrade-Mechanik.
- [ ] Speed, Energy, Muffling und weitere Original-Upgrades.
- [ ] Tier Installer und Factory-Upgrades.
- [ ] Configurator: Rotieren, Seitenwahl, Redstone-Modi.
- [ ] Configuration Card und Network Reader.
- [ ] Logistical Sorter und produktionsfähige Automation.
- [ ] Redstone-/Comparator-/Hopper-Kompatibilitaet und Stresstests.

## M08 - Logistik, Lager und Spezialmaschinen [P2]

- [ ] Logistical Transporters (vier Tiers), Restrictive und Diversion Transporter.
- [ ] Routenfindung, Filter, Prioritaeten, Backpressure und Netzwerk-Reload.
- [x] Bins (Basic bis Ultimate und Creative): Menge im Block-Entity-NBT (Horizon-Slots enden bei 99), Hopper fuellen/leeren ueber zwei Slots, Rechtsklick vorn legt die Hand ein, Linksklick vorn nimmt 1 Item (Sneak: Stack), Sneak + leere Hand sperrt den Typ (vorn) oder schaltet Auto-Eject (Seiten). Item behaelt den Inhalt, Front zeigt Item und Menge. Kapazitaeten nach `BinTier`, Creative `i32::MAX`. Siehe `native/.../bin.rs`, `bins.rs`.
- [x] Personal Chest und Personal Barrel: 9x6-Menue, Besitzer = Platzierer (UUID im NBT), Oeffnen und Abbauen nur fuer Besitzer oder Operatoren, Hopper ausgesperrt. Kein Security Desk (Robit/Security Desk offen). Siehe `storage.rs`, `storages.rs`.
- [ ] Chargepad: nicht portiert. Der Block braucht Spieler-Inventar-Zugriff pro Tick und Item-Energie fremder Items; Horizon bietet dafuer nur `player_tick` und `set_inventory_slot`, aber keinen Item-Energie-Helfer. Siehe `docs/KNOWN_ISSUES.md`.
- [ ] Digital Miner und Seismic Vibrator.
- [ ] Teleporter, Portable Teleporter, Teleporter Frames.
- [ ] Quantum Entangloporter mit Energie, Items, Fluids und Chemicals.
- [ ] Robit und Security Desk.
- [ ] QIO Drive Array, Drives, Dashboard, Importer, Exporter, Redstone Adapter, Portable Dashboard.
- [ ] Persistenz, Dimensionen und Multiplayer-Berechtigungen testen.

## M09 - Multiblocks und Industrie [P2]

- [ ] Allgemeines Multiblock-Struktursystem mit Formung, Abbruch, Chunkgrenzen und Validierung.
- [ ] Dynamic Tank und Dynamic Valve.
- [ ] Thermal Evaporation Plant (Controller, Valves, Blocks).
- [ ] Induction Matrix inkl. Cells/Providers/Ports für alle Tiers.
- [ ] Thermoelectric Boiler inkl. Heat/Steam-Verarbeitung.
- [ ] Industrial Turbine: Rotor, Coil, Vent, Saturating Condenser, Logic.
- [ ] Multiblock-GUI, Statusanzeige, Save/Load und Rebuild testen.

## M10 - Reaktoren, Strahlung und Endgame [P3]

- [ ] Nuclear Fuel- und Uranium-/Fissile-Recipe-Ketten.
- [ ] Fission Reactor: Casings, Ports, Logic, Assemblies, Rods, Kühlung.
- [ ] Radiation-System, Radioactive Waste Barrel, Dosimeter, Hazmat.
- [ ] Plutonium, Polonium, Pellets und deren Verarbeitung.
- [ ] Fusion Reactor, Hohlraum, Brennstoff, Laser-Zuendung.
- [ ] SPS (Supercritical Phase Shifter), Antimatter, Supercharged Coils.
- [ ] Antiprotonic Nucleosynthesizer.
- [ ] Laser, Laser Amplifier, Laser Tractor Beam.
- [ ] Stabilitaet, Abschaltung, Unfall-/Sicherheitslogik und Save/Load testen.

## M11 - Ausruestung, Items und Spielhilfen [P2/P3]

- [ ] Atomic Disassembler, Electric Bow, Flamethrower, HDPE-Reinforced Elytra.
- [ ] Jetpack, Armored Jetpack, Free Runners, Armored Free Runners.
- [ ] Scuba Equipment und Hazmat Suit.
- [ ] MekaSuit (alle Teile), Meka-Tool und Modification Station.
- [ ] MekaSuit-/Tool-Module, Energiekosten und Effekte.
- [ ] Canteen, Gauge Dropper, Geiger Counter, Dosimeter, Dictionary.
- [ ] Crafting Formula, Network Reader und Portable QIO Dashboard.
- [ ] Player-Movement, Item-NBT, Client/Server-Sync und Sicherheit testen.

## M12 - Unterprojekte komplettieren [P2/P3]

### Mekanism: Generators
Abgleich mit Upstream 26.3 (`31bb4906`) in `docs/GENERATORS.md`.
- [x] Heat, Solar, Advanced Solar, Wind und Bio Generator (nativ, M04).
- [x] Solar Panel, Turbine Blade, Hohlraum (Items), Turbine Casing/Rotor, Fission Reactor Casing, Fission Fuel Assembly, Control Rod Assembly, Fusion Reactor Frame, Reactor Glass, Laser Focus Matrix, Electromagnetic Coil, Rotational Complex, Saturating Condenser (Blöcke ohne Logik): Modelle, Texturen, Loot, Lang und Rezepte.
- [x] `mekanismgenerators`-Namespace, Mod-Manifest und Abhängigkeit zu `mekanism` geprüft (Manifest hat kein Abhängigkeitsfeld).
- [ ] Industrial Turbine, Fission Reactor, Fusion Reactor (Multiblock-Formung, Controller, Valves, Vents, Ports, Logic Adapter, GUIs): Blocker, braucht nativen Multiblock-Code (M09/M10). Turbine Blade lässt sich noch nicht in den Rotor stecken.
- [x] Gas-Burning Generator (nativ, M04; `mekanism:electrolytic_core` ist craftbar).
- [ ] Chemikalien/Fluide Deuterium, Tritium, Fusion Fuel, Bioethanol samt Eimern und Rezepten (Rotary, Separator, Neutron Activator): Blocker, Chemie-Tisch kennt nur `mekanism`, Horizon nur Vanilla-Fluide.
- [ ] Fusion Reactor Frame ist nicht craftbar, bis `#c:pellets/polonium` existiert (M07). Module (Geothermal, Solar Recharging Unit): Modulsystem fehlt.

### Mekanism: Tools
- [x] Paxels aus Wood, Stone, Copper, Iron, Gold, Diamond, Netherite (Axt-Aktion; Schaufel-Aktion fehlt, siehe `docs/TOOLS.md`).
- [x] Tool-/Armor-Sets aus Bronze, Lapis, Osmium, Steel, Refined Glowstone, Refined Obsidian (inkl. Spear, Shield, Horse und Nautilus Armor).
- [x] Haltbarkeit, Mining, Verzauberung, Reparatur und alle Rezepte.
- [x] `mekanismtools`-Namespace und Assets prüfen.
- [ ] Paxel als Schaufel (braucht nativen `use_on`-Hook), Schild-Textur, Haltbarkeits-Tooltip, Mob-Ausrüstung: Blocker in `docs/TOOLS.md`.

### Mekanism: Additions
- [x] Plastikvarianten: Slick, Glow, Reinforced, Transparent, Roads, Stairs, Slabs, Fences, Gates in allen 18 Farben (271 Blöcke, `content/mekanismadditions`, siehe `docs/ADDITIONS.md`).
- [x] Glow Panels und Obsidian TNT (Obsidian TNT explodiert wie Vanilla-TNT, siehe unten).
- [ ] Plastic-Grundrezept und Glow-Panel-Rezept: brauchen `mekanism:hdpe_sheet` (M06); Umfärben und Painting-Machine-Rezepte (`mekanism:painting`) fehlen.
- [ ] Obsidian TNT mit Radius 12 und Zündzeit: Blocker, eigene Entität/Explosion braucht nativen Code. Plastic Road ohne Geschwindigkeitsschub (`stepOn`), keine CTM/Emissive-Texturen, kein Lava-Fluid-Logging.
- [ ] Balloons, Walkie-Talkies, Baby-Mobs und Spawn Eggs: nur untersucht, nicht gebaut. Horizon-Content kennt `entity_type` (Vanilla-Vorlage + Attribute), aber keine eigene KI, Modelle, Biom-Spawns, Entity-Interaktion oder Netzwerk. Braucht nativen Code, siehe `docs/ADDITIONS.md`.
- [x] `mekanismadditions`-Namespace, Modelle (je Farbe vorgefärbt) und Assets.

## M13 - Paritaet, Integration, Performance und Releases [fortlaufend]

- [x] Maschinenlesbare Liste `Original-ID -> Horizon-ID -> Status` pflegen. *(`python scripts/parity.py --upstream <Mekanism-Checkout>` schreibt `docs/parity.json` (Items, Blöcke, Rezepte, Tags, Modelle, Texturen, Lang-Keys, Loot; Status done/partial/blocked/missing) und `docs/PARITY.md` (Zähler je Mod/Kategorie, größte Lücken). Nach jedem Port-Schritt neu erzeugen und committen; `blocked` kommt aus den TODO-Blockern in `BLOCKERS` im Skript.)*
- [x] Originalrezept-/Tag-/Asset-Abgleich automatisieren und Regressionen finden. *(`python scripts/parity.py --check` wertet die eingecheckte `docs/parity.json` gegen `content/` aus, ohne Upstream-Checkout, und endet mit Exit 1, wenn ein früher `done` markierter Eintrag verschwindet; läuft als CI-Schritt „Parity regression check“.)*
- [ ] Testmatrix für jede Maschine: Rezepte, Energie, Fluids, Chemicals, GUI, Seiteneinstellungen.
- [ ] Testmatrix für jede Block-Entity: Abbauen, Chunk-Unload, Neustart, neue Version, Multiplayer.
- [ ] Performance: 1, 100, 1.000 aktive Maschinen; Energie- und Transportnetze.
- [ ] Vollstaendigen Survival-Playthrough vom Osmium bis ins Endgame spielen.
- [ ] AE2-Horizon- und weitere Technikmod-Interoperabilitaet mit klaren APIs testen.
- [ ] Savegame-Migrationen, ID-Stabilitaet und Upgrade-Pfade dokumentieren.
- [ ] GUI, Shader-Kompatibilitaet, deutsche/englische Übersetzung und Barrierefreiheit testen.
- [x] README, Installationsanleitung (Windows, Content+Native), CHANGELOG, Known Issues erstellen. *(10.11.2026: `docs/INSTALL.md`, `CHANGELOG.md` (PR #1 bis #16), `docs/KNOWN_ISSUES.md`, `CONTRIBUTING.md`; README verlinkt sie.)*
- [ ] Lizenz, Copyright und Asset-Attribution der Originaldateien vor Releases prüfen. *(Prüfung 10.11.2026 in `docs/LICENSES.md`: alles MIT, `LICENSE-MEKANISM` je Mod vorhanden; offen: Attribution in README/Mod-Beschreibung, Inhaber `SozialHilfe`/`VitaminStack`, Drittbeiträge, Horizon-Lizenz, Abgleich vor Release.)*
- [ ] CI-Builds, Release-Artefakte, Hashes und Test-Beta dokumentieren.
- [ ] Upstream-26.3-Änderungen regelmäßig vergleichen und diese Liste aktualisieren.

## Definition of Done (für jede einzelne Funktion)

- [ ] Mit dokumentiertem Horizon-Build und passender API-Version erfolgreich gebaut.
- [ ] Automatisierte Tests und Originalverhalten nachvollziehbar verglichen.
- [ ] Ingame mit echten Survival-Inputs und Outputs getestet.
- [ ] Speicherstand nach Save, Chunk-Reload und Neustart korrekt.
- [ ] Keine Duplizierung, fehlenden Drops oder offensichtliche Performance-/Multiplayer-Fehler.
- [ ] Code, Log/Screenshot, README/CHANGELOG und Commit/PR vorhanden.

> Eine Phase ist nur fertig, wenn ihre Abnahmebedingungen erfüllt sind. Ein Block, der im Inventar erscheint, ist noch keine funktionierende Maschine. Es ist auch nicht garantiert, dass Horizon heute schon alle benoetigten nativen APIs anbietet; API-Luecken werden als eigene Issues dokumentiert.

## Quellverweise

- [Originalversion `gradle.properties`](https://github.com/mekanism/Mekanism/blob/26.3/gradle.properties)
- [Original Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismBlocks.java)
- [Original Items](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismItems.java)
- [Original Block Types](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismBlockTypes.java)
- [Original Energy Cube Tier](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/tier/EnergyCubeTier.java)
- [Generators Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/generators/java/mekanism/generators/common/registries/GeneratorsBlocks.java)
- [Tools Items](https://github.com/mekanism/Mekanism/blob/26.3/src/tools/java/mekanism/tools/common/registries/ToolsItems.java)
- [Additions Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/additions/java/mekanism/additions/common/registries/AdditionsBlocks.java)
- [Horizon Native Mod API](https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md)
- [Horizon Content-Mod-Dokumentation](https://github.com/horizon-rust/horizon/blob/main/docs/mods.md)
