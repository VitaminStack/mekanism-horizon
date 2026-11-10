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
