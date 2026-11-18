# Session m12-content-parity: parity of the three sub-mods

Content only, no Rust. Branch `claude/m12-content-parity-e82fbe`. Measured with `python scripts/parity.py --upstream <checkout>` into a temp copy (`docs/parity.json`/`docs/PARITY.md` deliberately not committed).

## Changed

- `scripts/gen_additions.py`: recipes are now taken over for all types whose Mekanism item IDs are registered: also recoloring (`neoforge:difference`, Horizon's crafting loader knows the type), `mekanism:painting` and `mekanism:pigment_extracting` (pigments are chemicals, not items, and are not checked). In addition: gray base models `block/plastic/*` and `block/glow_panel`, gray base textures and CTM sheets, all upstream lang keys (en_us/de_de).
- `scripts/gen_tools.py`: shield textures (`entity/shield/*`), trim palette, all upstream lang keys.
- NEW `scripts/import_generator_extras.py` (runs after `gen_resources.py`, `import_upstream.py`, `import_generator_models.py`; only adds): Turbine Valve/Vent, Fission Reactor Port/Logic Adapter, Fusion Reactor Controller/Port/Logic Adapter as simple blocks (models, blockstate with `active`/`mode`, loot, recipe), all missing textures, all lang keys, tag entries of the new blocks. The blocks are NOT in `scripts/registry.json`/`gen_resources.py` (another session); whoever adds them there overwrites only identical files.
- Newly created files: `content/mekanismadditions`, `content/mekanismtools`, `content/mekanismgenerators` (generator output only).

## Parity (missing, before -> after)

| Mod / category | before | after |
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

## What stays `missing` and why

**Additions**
- Recipes (99): 48 need `mekanism:hdpe_sheet` (basic Plastic recipe, Transparent, Glow Panel; not in `registry.json`, M06). The generator picks them up automatically once the item is registered: after the merge, run `gen_additions.py` again. 50 Balloon recipes (craft, recolor, painting) and the Walkie-Talkie recipe: items not registered.
- Items (26): 16+2 Balloons, Walkie-Talkie, 7 baby spawn eggs. They need native code: Balloons (entity, leash, popping), Walkie-Talkie (channels, voice packets), baby mobs (own entity classes, biome spawns). A placeholder item without function was not built (nothing faked).
- Loot (7), entity/damage-type tags (`c:entity_type/*`, `minecraft:entity_type/*`, `balloon_invulnerable`, Balloon item tag), item models/textures of the Balloons/Walkie/eggs, entity textures: they belong to the entities/items above.
- Tags `minecraft:block/mineable/axe` (only `remove` entries), `mekanism:data_component_type/config/clearable` (component `walkie_data`): no counterpart.

**Tools**: nothing is `missing` anymore. The shield textures and the trim palette exist but are not used (vanilla shield model, no Mekanism palettes, see docs/TOOLS.md).

**Generators**
- Items/blocks/models (4/5/6): buckets and fluids `bioethanol`, `deuterium`, `tritium`, `fusion_fuel` (Horizon knows only vanilla fluids); `superheating_element` has no registration entry upstream; the Wind Generator blades model uses the OBJ loader (Horizon content does not read it, the native mod draws the rotor); `entity/mekasuit_modules` (module system missing).
- Recipes (6): `geothermal_generator_unit`, `solar_recharging_unit` (modules), `rotary/*` and `separator/heavy_water` (chemicals Deuterium/Tritium/Fusion Fuel).
- Tags (30): fluid/bucket tags (no fluids), `minecraft:damage_type/*` and `fluid/entity_floatable` (damage types/fluids, not content types), `mekanism:block|item/structures*`, `mekanism:data_component_type/config/clearable`, `mekanism:mekanism/chemical/*` (namespace `mekanism`: a content mod may only add its own, `minecraft` and `c` tags; they belong in `content/mekanism`, not in my ownership).

## Notes for the lead session

- Order of the scripts after the merge: `gen_resources.py`, `import_upstream.py`, `import_generator_models.py`, `import_generator_extras.py`, `gen_additions.py`, `gen_tools.py`.
- If `gen_resources.py` registers the 7 new generator blocks itself: `import_generator_extras.py` can then be dropped; add `registry.json` there so `import_upstream.py` filters tags/lang for them.
- The 7 new generator blocks are plain blocks (no multiblock, no GUI), like the casings.
- `validate_content.py` still expects 271 Additions blocks and 91 Tools items (unchanged).

## In-game checklist

1. Additions: recolor at the crafting table: e.g. recolor 4 red Plastic blocks (recipe `plastic/block/recolor/*`) with dye; the ingredient must not be the target block itself (difference ingredient). Repeat with Plastic slabs/stairs/fences/glow.
2. Additions: Painting Machine: Plastic block + pigment (64 mB) gives a colored block; also check Aqua and Dark Red.
3. Additions: Pigment Extractor: extract a colored Plastic block (48 mB pigment), slabs/stairs (smaller amounts).
4. Additions: check names in the inventory in German/English (tags, tooltips).
5. Tools: check names/tooltips in the inventory (de/en).
6. Generators: craft Turbine Valve, Turbine Vent, Fission/Fusion Port, Logic Adapter and Fusion Controller (ingredient: `#c:circuits/*` must exist), place them, check model/texture; Fission Port and Fusion Port/Controller have one model per state (default: input / off). No function: right-click does nothing.
7. Generators: drop when breaking (pickaxe needed).
