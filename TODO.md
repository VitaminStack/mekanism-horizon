# Mekanism Horizon - Portierungs-TODO

**Stand:** 09.11.2026  
**Ziel:** Schrittweise Portierung des aktuellen Mekanism nach Minecraft Horizon, mit Windows/Visual Studio 2026 als Entwicklungsumgebung.

## Referenz und Arbeitsregeln

- **Original:** [mekanism/Mekanism, Branch `26.3`](https://github.com/mekanism/Mekanism/tree/26.3), abgeglichen mit Commit [`31bb4906`](https://github.com/mekanism/Mekanism/commit/31bb4906bc6c2ea4a2422d93cd1214df3ea7de4e).
- **Originalversion aus `gradle.properties`:** Mekanism **10.8.0**, Minecraft **26.3**, NeoForge **26.3.0.101**, `release_type=alpha`. Dies ist eine Entwicklungsbasis, nicht automatisch eine stabile Release-Version.
- **Horizon-Basis:** [`horizon-rust/horizon` @ `fdc87ae`](https://github.com/horizon-rust/horizon/commit/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca) (Nightly 2026-11-08, installiert im Launcher), Native-Hooks-API noch nicht gegen diesen Commit geprüft. Pin steht in `native/mekanism_native/horizon-mixin.toml` und `api/Cargo.toml`.
- **Geplanter Umfang:** Mekanism Core und danach die Original-Unterprojekte **Generators, Tools, Additions**.
- **Status:** `[x]` bedeutet **bestätigt**. `[ ]` bedeutet offen, auch wenn schon Code/Dateien existieren. Nicht nur wegen eines erfolgreichen Builds abhaken.
- **Jede Umsetzung:** Originalverhalten und Registry-IDs prüfen; Content (`.hmod`) und native Logik (`.hnmod`) sauber trennen; speichern/neuladen testen; Tests + Ingame-Nachweis im PR dokumentieren.

## Die nächsten 10 konkreten Aufgaben - zuerst diese erledigen

- [x] **N01** Lokalen Energy-Cube-Patch-Zustand prüfen: `git status --short` und `git apply --reverse --check .\mekanism-energy-cube-v0.2.0.patch`. Vorhandene Dateien NICHT überschreiben. *(09.11.2026: Working Tree sauber, Patch-Inhalt ist bereits committet, Patch-Datei existiert nicht mehr.)*
- [x] **N02** Nach dem Patch `Test-Content.ps1` und `cargo test --manifest-path .\logic\energy\Cargo.toml` erneut erfolgreich ausführen.
- [x] **N03** `Strg+B`: `.hmod` bauen, automatisch kopieren; `/give @s mekanism:basic_energy_cube` im Spiel testen und Block platzieren. *(09.11.2026: give, platzieren und Drop bestätigt; Loot-Tabellen brauchen Horizon-Format `condition`.)*
- [x] **N04** Native-Horizon-API-Version/Nightly-Commit bestimmen und `native/mekanism_native/` darauf abstimmen. *(09.11.2026: gepinnt auf Horizon `fdc87ae` (installierter Nightly); Hook-Signaturen nur gegen 99bd9655 (API 413) statisch geprüft. Kompilieren erst in N05.)*
- [ ] **N05** Native-Mod mit dem Horizon Launcher bauen und mit bewusst erteilter Vertrauensfreigabe installieren; Logs prüfen.
- [ ] **N06** Beim Platzieren wird eine native Block-Entity mit Energie-Capability angelegt.
- [ ] **N07** Basic Energy Cube: 4.000.000 Kapazität, 4.000 Ein-/Ausgabe-Limit pro Aufruf (Standard aus Original-`EnergyCubeTier`; später konfigurierbar).
- [ ] **N08** Energie mit Test-Hook/-Generator einfuegen und entnehmen; Seiten, Limits, Leer/Voll testen.
- [ ] **N09** Welt speichern, Horizon beenden und wieder starten: gespeicherte Energie muss erhalten bleiben.
- [ ] **N10** Erfolgreichen Ingame-Test, Build-Log und Screenshots dokumentieren, committen, PR erstellen.

## M00 - Funktionierende Projektbasis und Qualitaet [P0]

- [x] Eigenes Windows-Repository mit `content/`, `logic/`, `scripts/` eingerichtet.
- [x] Rust/Cargo unter Windows installiert und vier Energy-Core-Unit-Tests bestanden.
- [x] Content-Mod per `Strg+B` in Visual Studio 2026 bauen.
- [x] `.hmod` nach erfolgreichem Build automatisch in `horizon-mods/` kopieren.
- [x] Osmiumbarren und Osmiumblock im Spiel getestet.
- [ ] Lokale v0.2.0-Änderungen sicher versionieren; Remote und lokal abgleichen.
- [x] Horizon-Nightly, Native-Hook-API und Mekanism-Upstream-Commit pinnen.
- [ ] Test- und Buildanleitung für Windows dokumentieren.
- [ ] CI: Content-Validierung, Rust-Tests, JSON-Checks; Native-Build wenn reproduzierbar.
- [ ] Lizenz und Copyright der Original-Assets/Codeuebernahmen prüfen und beilegen.

## M01 - Basic Energy Cube: erster wirklich funktionierender Block [P0]

- [ ] Block, Itemmodell, Texture, Loot, Übersetzungen und Rezept stimmen.
- [ ] Native Block-Entity beim Platzieren und Entfernen korrekt erzeugen/entfernen.
- [ ] Horizon Energie-Hook (ab API 142) mit `ModEnergyBuffer` anbinden.
- [ ] Kapazität, Eingabe, Ausgabe und Simulations-Aufrufe korrekt begrenzen.
- [ ] Alle Seiten/Facing testen; Eingabe/Ausgabe konfigurieren.
- [ ] NBT-Persistenz bei Save, Chunk-Unload und Neustart.
- [ ] Energieübergabe ohne Duplikation oder Verlust.
- [ ] GUI: Speicherstand, Transferlimit, I/O-Seiten und Status.
- [ ] Verhalten beim Abbau/Neuplatzieren mit Original vergleichen.
- [ ] Integrationstests + Ingame-Demonstration abschliessen.

**Fertig, wenn:** Der Cube nimmt echte Energie auf, gibt sie ab, kann konfiguriert werden und behaelt den Ladezustand nach Neustart.

## M02 - Rohstoffe, Weltgenerierung und Grundrezepte [P1]

- [x] Originale Registry-IDs von Items/Blöcken/Ressourcen inventarisieren. *(Ressourcen-IDs nach Mekanism 26.3 in `scripts/gen_resources.py`, Liste in `scripts/registry.json`.)*
- [ ] Originaltexturen, Modelle, Blockstates, Sounds und Sprachen mit Lizenzhinweisen integrieren. *(Für Ressourcen per `scripts/import_upstream.py`; MIT-Lizenz in `content/mekanism/LICENSE-MEKANISM`. Sounds und Energy-Cube-Modell fehlen noch.)*
- [x] Osmium-, Tin/Zinn-, Uranium-/Uran- und Fluorite-/Fluorit-Erze inklusive Deepslate-Varianten. *(Plus Blei und Salz.)*
- [x] Weltgenerierung, Erzverteilung, Drops und Mining-Level (nur neue Chunks) testen. *(Werte aus Original-`OreType`/`WorldConfig`. Kein Glück/Behutsamkeit: Horizon-Loot kennt `apply_bonus`/`match_tool` nicht.)*
- [x] Ingots, Nuggets, Dusts, Raw Materials, Storage Blocks (z.B. Bronze, Steel, Refined Glowstone, Refined Obsidian).
- [ ] Steel Casing, Control Circuits (Basic/Advanced/Elite/Ultimate), Alloys und Enriched Materials. *(Registriert; die meisten Rezepte brauchen den Metallurgic Infuser (M05).)*
- [ ] Crafting-, Smelting-, Blasting- und Resource-Tags nach Original-Rezepten. *(97 Vanilla-Rezepte + Tags importiert; Maschinenrezepte folgen mit ihren Maschinen.)*
- [ ] Survival-Test: alle Materialien für erste Maschinen ohne Cheat erhältlich. *(Blockiert: Stahl, Circuits und Alloys brauchen Metallurgic Infuser (M05).)*

## M03 - Energie- und Maschinen-Fundament [P0/P1]

- [ ] Joule/FE-Konvertierung, Overflow-Schutz und Einheiten in UI/API festlegen.
- [ ] Energie in Items und Blöcken über gemeinsame Capabilities.
- [ ] Energy Tablet und alle Energy Cubes: Basic, Advanced, Elite, Ultimate, Creative.
- [ ] Original-Tier-Werte, Transferlimits und Konfigurationen übernehmen.
- [ ] Universal Cables (vier Tiers), Verbindungsgraph und Nachbar-Updates.
- [ ] Energieverteilung über Chunks, Server-Ticks und Redstone korrekt.
- [ ] Gemeinsames Block-Entity-, Recipe-, Inventory- und Save/Load-Framework.
- [ ] Lasttest: 100+ Cables und Cubes; keine Energie-Duplikation bei Netz-Reload.

## M04 - Energieerzeugung / Generators-Grundstufe [P1]

- [ ] Heat Generator (Brennstoff, Umgebung und Ausgabeseiten).
- [ ] Solar Generator und Advanced Solar Generator.
- [ ] Wind Generator (Höhe/Wetter nach Original).
- [ ] Bio Generator und Bio Fuel.
- [ ] Gas-Burning Generator, nachdem Chemie/Fluids funktionieren.
- [ ] GUIs, Energieabgabe, Upgrades und Balancing der Generatoren.
- [ ] Survival-Test: Generator -> Universal Cable -> Energy Cube -> Maschine.

## M05 - Maschinen und erste Processing-Ketten [P1]

- [ ] Allgemeines Rezept-System: Inputs, Outputs, Energie je Tick, Fortschritt, Nebenprodukte.
- [ ] Maschinen-Inventare, Menüs, GUI, Upgrades, Auto-Eject, Sided I/O, Redstone-Modi.
- [ ] Metallurgic Infuser und Infusions-Ressourcen.
- [ ] Enrichment Chamber.
- [ ] Energized Smelter.
- [ ] Crusher.
- [ ] Precision Sawmill.
- [ ] Osmium Compressor und Combiner.
- [ ] Formulation/Assembly: Formulaic Assemblicator und Oredictionificator.
- [ ] Leere Energie, voller Output, Rezeptwechsel, Save/Load und Chunk-Unload prüfen.

## M06 - Fluids, Chemicals und Rohstoffverarbeitung [P1]

- [ ] Chemie-IDs/Registry sowie Netzwerk-/Datapack-Synchronisation planen.
- [ ] Fluids: Water, Steam, Brine, Lithium, Heavy Water, Nutritional Paste etc.
- [ ] Chemicals: Hydrogen, Oxygen, Chlorine, Sulfur-Komponenten, Ethene, Slurries, Pigments, Isotope etc.
- [ ] Fluid Tank und Chemical Tank (Basic/Advanced/Elite/Ultimate/Creative).
- [ ] Mechanical Pipes und Pressurized Tubes (vier Tiers).
- [ ] Thermodynamic Conductors und Heat-System.
- [ ] Electric Pump und Fluidic Plenisher.
- [ ] Electrolytic Separator.
- [ ] Chemical Oxidizer und Chemical Infuser.
- [ ] Rotary Condensentrator.
- [ ] Purification Chamber und Chemical Injection Chamber.
- [ ] Chemical Dissolution Chamber, Chemical Washer, Chemical Crystallizer.
- [ ] Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator.
- [ ] Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine.
- [ ] Erzverarbeitung und Mehrfachertraege (bis zu Original-Spitzenwerten) als End-to-End-Kette.
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
- [ ] Bins (inkl. Creative), Personal Chest, Personal Barrel, Chargepad.
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
- [ ] Alle Generatoren, Turbinen, Boiler und Reaktoren des Originalmoduls vollständig abgleichen.
- [ ] Solar Panel, Turbine Blade, Hohlraum, alle Original-Modelle und Rezepte.
- [ ] `mekanismgenerators`-Namespace und Abhängigkeiten prüfen.

### Mekanism: Tools
- [ ] Paxels aus Wood, Stone, Copper, Iron, Gold, Diamond, Netherite.
- [ ] Tool-/Armor-Sets aus Bronze, Lapis, Osmium, Steel, Refined Glowstone, Refined Obsidian.
- [ ] Haltbarkeit, Mining, Verzauberung, Reparatur und alle Rezepte.
- [ ] `mekanismtools`-Namespace und Assets prüfen.

### Mekanism: Additions
- [ ] Plastikvarianten: Slick, Glow, Reinforced, Transparent, Roads, Stairs, Slabs, Fences, Gates.
- [ ] Glow Panels und Obsidian TNT.
- [ ] Balloons, Walkie-Talkies, Baby-Mobs und Spawn Eggs.
- [ ] Models, Animationen, Entity-Netzwerk und `mekanismadditions`-Namespace.

## M13 - Paritaet, Integration, Performance und Releases [fortlaufend]

- [ ] Maschinenlesbare Liste `Original-ID -> Horizon-ID -> Status` pflegen.
- [ ] Originalrezept-/Tag-/Asset-Abgleich automatisieren und Regressionen finden.
- [ ] Testmatrix für jede Maschine: Rezepte, Energie, Fluids, Chemicals, GUI, Seiteneinstellungen.
- [ ] Testmatrix für jede Block-Entity: Abbauen, Chunk-Unload, Neustart, neue Version, Multiplayer.
- [ ] Performance: 1, 100, 1.000 aktive Maschinen; Energie- und Transportnetze.
- [ ] Vollstaendigen Survival-Playthrough vom Osmium bis ins Endgame spielen.
- [ ] AE2-Horizon- und weitere Technikmod-Interoperabilitaet mit klaren APIs testen.
- [ ] Savegame-Migrationen, ID-Stabilitaet und Upgrade-Pfade dokumentieren.
- [ ] GUI, Shader-Kompatibilitaet, deutsche/englische Übersetzung und Barrierefreiheit testen.
- [ ] README, Installationsanleitung (Windows, Content+Native), CHANGELOG, Known Issues erstellen.
- [ ] Lizenz, Copyright und Asset-Attribution der Originaldateien vor Releases prüfen.
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
