# Session m12-content-parity: Parität der drei Sub-Mods

Nur Content, kein Rust. Branch `claude/m12-content-parity-e82fbe`. Gemessen mit `python scripts/parity.py --upstream <Checkout>` in eine Temp-Kopie (`docs/parity.json`/`docs/PARITY.md` bewusst nicht committet).

## Geändert

- `scripts/gen_additions.py`: Rezepte werden jetzt für alle Typen übernommen, deren Mekanism-Item-IDs registriert sind: auch Umfärben (`neoforge:difference`, Horizons Crafting-Lader kennt den Typ), `mekanism:painting` und `mekanism:pigment_extracting` (Pigmente sind Chemikalien, keine Items, und werden nicht geprüft). Dazu: graue Basismodelle `block/plastic/*` und `block/glow_panel`, graue Basistexturen und CTM-Sheets, alle upstream-Lang-Schlüssel (en_us/de_de).
- `scripts/gen_tools.py`: Schild-Texturen (`entity/shield/*`), Trim-Palette, alle upstream-Lang-Schlüssel.
- NEU `scripts/import_generator_extras.py` (läuft nach `gen_resources.py`, `import_upstream.py`, `import_generator_models.py`; fügt nur hinzu): Turbine Valve/Vent, Fission Reactor Port/Logic Adapter, Fusion Reactor Controller/Port/Logic Adapter als einfache Blöcke (Modelle, Blockstate mit `active`/`mode`, Loot, Rezept), alle fehlenden Texturen, alle Lang-Schlüssel, Tag-Einträge der neuen Blöcke. Die Blöcke stehen NICHT in `scripts/registry.json`/`gen_resources.py` (andere Session); wer sie dort aufnimmt, überschreibt nur identische Dateien.
- Neu erzeugte Dateien: `content/mekanismadditions`, `content/mekanismtools`, `content/mekanismgenerators` (nur Generator-Ausgabe).

## Parität (missing, vorher -> nachher)

| Mod / Kategorie | vorher | nachher |
|---|---:|---:|
| additions items | 26 | 26 |
| additions recipes | 825 | 99 |
| additions tags | 14 | 14 |
| additions models | 49 | 20 |
| additions textures | 41 | 28 |
| additions lang | 228 | 0 |
| additions loot | 7 | 7 |
| tools textures | 7 | 0 |
| tools lang | 1007 | 0 |
| generators items | 11 | 4 |
| generators blocks | 12 | 5 |
| generators recipes | 13 | 6 |
| generators tags | 30 | 30 |
| generators models | 17 | 6 |
| generators textures | 39 | 0 |
| generators lang | 355 | 0 |
| generators loot | 7 | 0 |

## Was bleibt `missing` und warum

**Additions**
- Rezepte (99): 48 brauchen `mekanism:hdpe_sheet` (Plastic-Grundrezept, Transparent, Glow Panel; nicht in `registry.json`, M06). Der Generator nimmt sie automatisch mit, sobald das Item registriert ist: nach dem Merge `gen_additions.py` neu laufen lassen. 50 Balloon-Rezepte (craft, recolor, painting) und das Walkie-Talkie-Rezept: Items nicht registriert.
- Items (26): 16+2 Balloons, Walkie-Talkie, 7 Baby-Spawn-Eggs. Brauchen nativen Code: Balloons (Entität, Leine, Platzen), Walkie-Talkie (Kanäle, Sprachpakete), Baby-Mobs (eigene Entitätsklassen, Biom-Spawns). Ein Platzhalter-Item ohne Funktion wurde nicht gebaut (nichts vortäuschen).
- Loot (7), Entity-/Damage-Type-Tags (`c:entity_type/*`, `minecraft:entity_type/*`, `balloon_invulnerable`, Balloon-Item-Tag), Items-Modelle/-Texturen der Balloons/Walkie/Eggs, Entity-Texturen: gehören zu den obigen Entitäten/Items.
- Tags `minecraft:block/mineable/axe` (nur `remove`-Einträge), `mekanism:data_component_type/config/clearable` (Komponente `walkie_data`): kein Gegenstück.

**Tools**: nichts mehr `missing`. Die Schild-Texturen und die Trim-Palette liegen vor, werden aber nicht genutzt (Vanilla-Schildmodell, keine Mekanism-Paletten, siehe docs/TOOLS.md).

**Generators**
- Items/Blöcke/Modelle (4/5/6): Eimer und Fluide `bioethanol`, `deuterium`, `tritium`, `fusion_fuel` (Horizon kennt nur Vanilla-Fluide); `superheating_element` hat upstream keinen Registrierungseintrag; Wind-Generator-Blades-Modell nutzt den OBJ-Lader (Horizon-Content liest ihn nicht, der native Mod zeichnet den Rotor); `entity/mekasuit_modules` (Modulsystem fehlt).
- Rezepte (6): `geothermal_generator_unit`, `solar_recharging_unit` (Module), `rotary/*` und `separator/heavy_water` (Chemikalien Deuterium/Tritium/Fusion Fuel).
- Tags (30): Fluid-/Eimer-Tags (keine Fluide), `minecraft:damage_type/*` und `fluid/entity_floatable` (Schadensarten/Fluide, keine Content-Typen), `mekanism:block|item/structures*`, `mekanism:data_component_type/config/clearable`, `mekanism:mekanism/chemical/*` (Namespace `mekanism`: ein Content-Mod darf nur eigene, `minecraft`- und `c`-Tags ergänzen; gehören in `content/mekanism`, nicht in meinen Besitz).

## Hinweise an die Lead-Session

- Reihenfolge der Skripte nach dem Merge: `gen_resources.py`, `import_upstream.py`, `import_generator_models.py`, `import_generator_extras.py`, `gen_additions.py`, `gen_tools.py`.
- Falls `gen_resources.py` die 7 neuen Generator-Blöcke selbst registriert: `import_generator_extras.py` kann dann entfallen; dort `registry.json` aufnehmen, damit `import_upstream.py` Tags/Lang dafür filtert.
- Die 7 neuen Generator-Blöcke sind reine Blöcke (kein Multiblock, keine GUI), wie die Casings.
- `validate_content.py` erwartet weiter 271 Additions-Blöcke und 91 Tools-Items (unverändert).

## Ingame-Checkliste

1. Additions: Umfärben per Werkbank: z. B. 4 rote Plastic-Blöcke (Rezept `plastic/block/recolor/*`) mit Farbstoff umfärben; die Zutat darf nicht der Zielblock selbst sein (Difference-Ingredient). Mit Plastic Slabs/Stairs/Fences/Glow wiederholen.
2. Additions: Painting Machine: Plastic-Block + Pigment (64 mB) ergibt gefärbten Block; Aqua und Dark Red mitprüfen.
3. Additions: Pigment Extractor: gefärbten Plastic-Block extrahieren (48 mB Pigment), Slabs/Stairs (kleinere Mengen).
4. Additions: Namen im Inventar deutsch/englisch prüfen (Tags, Tooltips).
5. Tools: Namen/Tooltips im Inventar prüfen (de/en).
6. Generators: Turbine Valve, Turbine Vent, Fission/Fusion Port, Logic Adapter und Fusion Controller craften (Ingredient: `#c:circuits/*` muss existieren), setzen, Modell/Textur prüfen; Fission Port und Fusion Port/Controller haben je Zustand ein Modell (Standard: Input / aus). Ohne Funktion: Rechtsklick tut nichts.
7. Generators: Drop beim Abbauen (Pickaxe nötig).
