# Fluids und Chemicals (M06)

Entwurf und Stand für das Chemie-/Fluid-Fundament. Basis: Mekanism 26.3 (`31bb4906`).

## Problem

Mekanism 26.3 hat ein eigenes Chemical-Registry (Datapack `data/mekanism/mekanism/chemical/*.json`,
Serializer `basic` und `enum_color_pigment`) und echte Welt-Fluids (`MekanismFluids`). Horizon bietet
Content-Mods weder ein Chemical-Registry noch eigene Welt-Fluids oder Eimer. Block-Entities können aber
Fluid-Tanks haben (API 54, Hook `block_entity_fluids`), die **jede Fluid-ID außerhalb von `minecraft`**
speichern, mit NBT sichern (`fluids`) und über `fluid_insert` / `fluid_extract` / `move_fluids`
bewegen (API 54/55, `mod_fluids`).

## Entscheidung

Beides sind Tank-IDs, Tabellen in `native/mekanism_native/api/src/chemicals_table.rs`
(erzeugt von `scripts/gen_chemicals.py` aus dem Upstream-Checkout: 18 Fluids, 64 Chemicals).

| Art | Tank-ID | Beispiel |
|---|---|---|
| Fluid | `mekanism:<id>` (+ `minecraft:water`/`lava`) | `mekanism:brine` |
| Chemical | `mekanism:chemical/<id>` | `mekanism:chemical/hydrogen` |

- Flüssiges Hydrogen (Fluid) und Hydrogen (Chemical) kollidieren so nie.
- `block_entity_accepts_fluid` trennt die Familien: Fluid-Tanks nehmen keine Chemicals, Chemical-Tanks
  keine Fluids.
- Kein eigenes Netzwerk/Datapack-Sync nötig: Horizon synchronisiert Block-Entity-NBT und Menüdaten selbst.
  Das Menü überträgt `chemicals::index + 1` (16 Bit); Namen und Farben liegen im Client-Code der Native-Mod.
- Unbekannte IDs (andere Mod, entferntes Chemical) bleiben erhalten und zeigen ihre ID.

## Grenzen (Horizon)

- Keine Welt-Fluid-Blöcke und Eimer für Mekanism-Fluids; nur Wasser/Lava-Eimer an Fluid-Tanks.
- Mengen sind `i32` Millibucket; Creative-Tanks (Long.MAX_VALUE) halten `i32::MAX`.
- Rohr-/Tube-Netze (`native_move_fluids` kennt nur Nachbarblöcke) bleiben offen, siehe Gruppe 2.

## Stand Tanks (Branch `feat/m06-chemistry-foundation`)

Fluid Tank und Chemical Tank, je Basic/Advanced/Elite/Ultimate/Creative: Werte aus `FluidTankTier` /
`ChemicalTankTier`, Auto-Eject nach unten (Fluid), Dump-Modi `Idle`/`Dumping Excess`/`Dumping`
(Chemical, `ChemicalUtils.dump`), Inhalt am Item beim Abbau, GUI mit Füllstand und Modus-Button,
Debug-Block `mekanism:debug_chemical_source` (füllt Hydrogen in Nachbartanks).

Nicht portiert: Item-Slots zum Füllen/Leeren, Seitenkonfiguration, Redstone, Valve-Journal, Edit-Mode.

**Ingame-Tests und Native-Build (Launcher) stehen noch aus.** Unit-Tests: `cargo test --manifest-path logic/energy/Cargo.toml`.

## Electrolytic Separator und Rotary Condensentrator

Beide laufen über dieselbe Tank-Tabelle (`machine::FluidTanks`: je Tank Gauge und Kapazität, dazu die füll- und
entnehmbaren Tanks) und `machine::convert` (ein Eingang, alle Outputs passen ganz oder nichts läuft).
- Separator (`mekanism:separating`): Tank 0 Fluid-Eingang (24.000 mB), Tank 1/2 linke/rechte Chemie (2.400 mB), ein
  Durchlauf je Tick. Der Importer macht aus dem Fluid-Tag die Fluid-ID (`#c:brine` zu `mekanism:brine`,
  `#minecraft:water` zu `minecraft:water`) und lässt das Deuterium-Rezept aus (kein Tank-ID für `mekanismgenerators`-Chemie).
- Rotary (`mekanism:rotary`): Tank 0 Chemical, Tank 1 Fluid (je 10.000 mB), Modus in NBT `mekanism_rotary_mode`
  (0 Chemical zu Fluid, 1 umgekehrt), Knopf im GUI (`BUTTON_MODE`), das Menü hängt den Modus nach den Tanks an.
Blocker: `block_entity_accepts_fluid` kennt nur Block, Tank-Nummer und Fluid (keine Position, kein Modus), und
Tank-Seiten sind statisch; der Rotary-Modus kann Ein-/Ausgabe von außen deshalb nicht sperren. Fluid-Items und
Chemie-Items (Slots der Originale) fehlen: Horizon hat Fluid-Item-API nur für Items, nicht für Slot-Füllung.

## Chemie-Ausgabe der Maschinen (Chemical Oxidizer, Chemical Infuser)

Die Tanks dieser Maschinen sind Horizons Block-Entity-Tanks (`fluids`, Reihenfolge: Eingänge, dann Output), mit
`block_entity_fluids` von außen erreichbar: Eingänge füllbar, Output entnehmbar. Chemie-Rezepte: `oxidizing`
(Item zu Chemical, `Recipe::chemical_output`) und `chemical_infusing` (`recipe::Infusion`, beide Seiten
vertauschbar). Der Output wird jeden Tick (bis 1.024 mB) per `fluid_insert` in alle sechs Nachbarn geschoben
(Mekanisms Ejector samt Side-Config ist nicht portiert). `block_entity_accepts_fluid` bekommt keine Position und
keinen Tank-Inhalt, deshalb nimmt der Infuser jedes Chemical, das in einem Rezept vorkommt; passt der Partner
nicht, wartet die Maschine.

Blocker/Grenzen: kein Item-Slot-Fill der Tanks (Chemie-Items fehlen), Mengen `i32`, kein Pfeil-Overlay.
Der Eject-Aufruf liegt außerhalb des Rezept-Locks, weil `fluid_insert` den `accepts`-Hook des Nachbarn
synchron aufruft (derselbe Mutex: sonst Deadlock).

## Electric Pump und Fluidic Plenisher

Logik in `pump.rs` (Horizon-frei, Tests in `logic/energy`), Host-Seite in `pumps.rs`. Beide haben einen Tank mit 10.000 mB als Horizon-Block-Entity-Tank (`minecraft:water` / `minecraft:lava`), 2 FE/t, Speicher 8 FE.

- **Pump:** alle 19 Ticks eine Quelle (1.000 mB), zuerst unter der Pumpe, dann die Nachbarn bereits gepumpter Blöcke im Radius 80 (rekursiv, wie `TileEntityElectricPump`). Der Tank gibt bis 256 mB/t nach oben ab (`fluid_insert`). Wasser mit Spielregel `water_source_conversion` und Lava mit `lava_source_conversion` verschwinden nicht (Mekanisms `pumpInfiniteFluidSources` aus).
- **Plenisher:** nimmt Wasser/Lava nur von oben. Alle 20 Ticks ein Block: erst die Breitensuche über Luft und fließendes Fluid unterhalb (nie nach oben, höchstens 4.000 Blöcke), dann setzt er je Durchlauf einen Eimer. Der Weg steht im Block-Entity-NBT (`mekanism_plenisher_*`).
- Welt-Zugriff: `Host::block`, `block_property` (`level` 0 = Quelle), `set_block` (Flags 3), `Level::is_loaded` und `Level::game_rule`. Entladene Chunks werden nie gelesen oder beschrieben.
## Erzverarbeitung (Dissolution Chamber, Chemical Washer, Chemical Crystallizer)

Schlämme (`dirty_*`, `clean_*`) sind Chemicals der Tabelle (`mekanism:chemical/dirty_iron`). Dissolution: Item + Säure
aus Tank 0 (`Machine::chemical_tank`, per-Tick-Verbrauch) zu Schlamm in Tank 1. Washer: `machine::wash` (Wasser + Schlamm
zu Schlamm, alles oder nichts). Crystallizer: `Op::Crystallize`, Chemical aus Tank 0 zu Item im Output-Slot. Rezept-Formen
`dissolution`, `washing`, `crystallizing` in `recipe.rs`; Überblick und Grenzen: `docs/ORE_PROCESSING.md`.

## Mechanical Pipes, Pressurized Tubes, Thermodynamic Conductors (Gruppe 2)

Zwölf Blöcke (je vier Tiers), Werte aus `PipeTier`, `TubeTier`, `ConductorTier` (Mekanism 26.3), Logik Horizon-frei in `transmitter.rs` (Tests in `logic/energy`), Host in `transmitters.rs`.

| Tier | Pipe (Fluid) Kapazität / Rate | Tube (Chemical) Kapazität / Rate | Conductor Isolation |
|---|---|---|---|
| Basic | 2.000 / 250 mB/t | 4.000 / 750 mB/t | 10 |
| Advanced | 8.000 / 1.000 | 16.000 / 2.000 | 400 |
| Elite | 32.000 / 8.000 | 256.000 / 64.000 | 8.000 |
| Ultimate | 128.000 / 32.000 | 1.024.000 / 256.000 | 100.000 |

Horizon hat keine Transmitter-Netze, also ist jeder Block eine eigene Block-Entity:
- **Pipe/Tube**: ein Block-Entity-Tank (`block_entity_fluids`, nur Eingang; `block_entity_accepts_fluid` trennt Fluids und Chemicals). Jeden Tick bietet der Block bis zur Rate des Tiers den Nachbarn Flüssigkeit an (`fluid_insert`), und zwar nur denen mit Platz und niedrigerem Füllstand als er selbst (`push_plan`). Eine Leitung gleicht sich so aus, nichts fließt in einen volleren Tank zurück. Quellen schieben wie bei den Tanks, Maschinen-Outputs und Pumpen selbst in die Pipe. Unterschied zu Mekanism: Kapazität und Rate gelten je Block, nicht für das ganze Netz.
- **Conductor**: Wärme in `mekanism_heat` (wie der Heat Generator). Pro Tick `ITileHeatHandler.simulateAdjacent` zu kälteren Nachbarseiten (Conductors, Heat Generator) und `simulateEnvironment` (sechs Seiten, Luft 10.000 + Isolation + Leitung 5). Neu platzierte Conductors starten bei der Umgebungstemperatur des Bioms. Der Heat Generator gibt Wärme an einen kälteren Conductor ab und nimmt sie von einem wärmeren.
- **Verbindungen**: Blockzustände `down`..`east` (boolean), vom Tick gesetzt: gleichartiger Transmitter beliebigen Tiers, Block mit Fluid-Tank (Pipe/Tube) oder Heat Generator (Conductor). Das Modell ist eine Multipart-Kombination aus Mittelwürfel und sechs Armen (Quader mit den Upstream-Texturen), weil Horizon Mekanisms OBJ-Loader nicht kennt.

Nicht portiert: Seitenmodi Pull/Push/Off (Configurator), Netzwerk-Redstone, Upgrade per Tier-Installer, Auswahlbox je Arm (Horizon-Shapes hängen an einer Property), Wärmequellen außer dem Heat Generator (Boiler, Fission, Fusion, Resistive Heater existieren noch nicht). Grenze: `block_entity_accepts_fluid` kennt keine Position, eine Pipe kann also nicht nach Nachbar sperren.
