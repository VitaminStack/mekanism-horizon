# M07 Factories (session m07-factories)

Branch `claude/m07-factories-bb48a4`. Status: code and content committed, **untested in game, native code not compiled** (see gaps).

## What was ported

32 factories (Basic/Advanced/Elite/Ultimate x Enriching, Crushing, Compressing, Combining, Purifying, Injecting, Infusing, Sawing) and 4 Tier Installers.

| Module | Content |
| --- | --- |
| `native/mekanism_native/api/src/factory.rs` | Horizon-free: tier/type tables, slot layout, GUI dimensions, energy capacity, step on shared energy, item move on upgrade. 16 unit tests, hooked into `logic/energy`. |
| `native/mekanism_native/api/src/factories.rs` | Host: tick, container, screen, drawing, tooltip, dump button, Tier Installer right-click. |
| `lib.rs`, `energy.rs` | Only additive lines: `Kind::Factory`, one line per hook, `factory_tick`, energy acceptor. |
| `scripts/gen_resources.py` | One marked block: 32 blocks, 32 menus, 4 installer items. Models, blockstates, recipes, tags, names come via `import_upstream.py` (recipes `factory/<tier>/`, `tier_installer/`). |

## Values and their source (Mekanism 26.3)

- Processes 3/5/7/9, slot start x 55/35/29/27, spacing 38/26/19/19: `FactoryTier.java`, `TileEntityItemToItemFactory.java`, `TileEntitySawingFactory.java`.
- Input y 13, output y 57, sawmill secondary output y 77, shared slot (7, 57): the same files plus `TileEntityCombiningFactory.java`, `TileEntityItemStackChemicalToItemStackFactory.java`.
- Energy: `Factory.java` (`max(storage*0.5, usage) * processes`), usage per process = base machine, `AttributeEnergy.java`.
- Duration 200 ticks per operation: `TileEntityFactory.BASE_TICKS_REQUIRED`.
- Shared chemical tank `MAX_GAS`/`MAX_INFUSE` x processes: `TileEntityItemStackChemicalToItemStackFactory.getInitialChemicalTanks`.
- GUI: width +34 for Ultimate, height +11 with tank, +21 for the sawmill, inventory offset 85/95/105 (x 26 Ultimate), energy bar `width-12`, 16, height 52/73, progress DOWN 8x20 at `4+x`, 33, tank bar (7,76), dump (148/182, 76, 21x10): `GuiFactory.java`, `FactoryContainer.java`, `GuiDumpButton.java`.
- Tier Installer: `ItemTierInstaller.java`, `AttributeUpgradeable` in `Factory.java` (a machine takes only Basic, a Factory of tier N takes only Installer N+1).

## Behavior

- Each process has its own progress (`mekanism_factory_progress`, IntArray) and its own input/output. The processes run in order each tick and draw from one energy buffer; when energy runs out, the later ones stand still.
- No recipe: progress 0. Output full or chemical missing: paused, progress stays.
- The shared chemical tank (Compressing, Purifying, Injecting, Infusing) is filled through the shared slot, one item per tick; the dump button empties it. Combining: the shared slot is the extra input.
- Sawing: secondary output per process with a chance (`machine::random`).
- Hum and `active` model as with the machines. Loot: only the block (the generator's default loot).
- Recipes, chemical and usage values come from the base machines (`machine::find`, `Recipes::processing`), not maintained twice.

## Gaps and blockers

- **Smelting factory missing**: needs `minecraft:smelting` recipes and the Energized Smelter, neither of which is reachable.
- **Native code not compiled.** The launcher build was not run on the lead's instruction. Checked only against the signatures in the Horizon source (rev `fdc87ae`) and against the existing patterns in `process.rs`/`bins.rs`. First step when testing: fix build errors.
- **Tier Installer**: according to the Horizon source (`native_set_block`), `set_block` to a different block creates the new block entity in the same call; `change_mod_block_entity` fills it. The old entity is emptied first so nothing can fall to the ground. Not proven: whether Horizon drops container contents when replacing (nothing found in the source). Check in game.
- Not ported: energy slot (7, 13), auto-sort and sort tab, side configuration, upgrades, recipe book, progress warnings.
- Hoppers and shift-click fill every input slot without a recipe check (Mekanism filters). Recipe check at tick: an item without a recipe stays where it is.
- Menu data is 16-bit; the largest value (Ultimate Infusing tank 9000) fits.
- Required changes to foreign files: none. `process::BUFFER`, `BUFFER_TYPE` and `machine` fields are only read. If `m07-machine-framework` renames `Machine` fields, `factory.rs`/`factories.rs` must follow.
- **Merge risk**: `m07-machine-framework` (branch `claude/m07-machine-framework-b79164`) changes `machine.rs`, `process.rs`, `lib.rs` and adds `upgrade.rs` (energy and upgrade slots, `data_len`, `Machine::upgraded`). `factory.rs`/`factories.rs` only read `Machine` fields, `Tank`, `BUFFER`, `Recipe`, `processing`, `conversion`; the additional machine slots (energy, upgrades) would not be carried over by `Slots::of_machine` in the installer (role move), so the upgrade items of a machine would be lost on conversion. After the merge, check the installer for this and adjust `of_machine`.
- The pause/reset rule (`factory::phase`/`advance`) is tested; `factories::tick` only calls it.
- The generator run pulled in old state from elsewhere (commit `26f5d10`): transmitter tags (`tags/{block,item}/transmitters*`), `c/tags/block/relocation_not_supported.json`, `mineable/pickaxe.json`, `blocks_motion_no_leaves.json`; the lang files contain only factory and installer names. Produced by the importer, harmless.

## In-game checklist

After the native build, in Creative, with energy from a Creative Energy Cube:

1. All 32 blocks are in the Creative tab, model and name are correct, `active` switches the model.
2. Basic Enriching Factory: ore in all 3 inputs, apply energy: all 3 progress arrows run, after 10 s one dust each. Output full: the arrow stops, and continues after emptying.
3. Too little energy (e.g. 10 FE): only the first processes run. Check the energy bar and tooltip (capacity 30 for Basic Enriching).
4. Combining: extra item in slot (7,57), usage per operation.
5. Compressing/Purifying/Injecting/Infusing: an item in the tank slot fills the bar, the tooltip shows the amount, dump empties it. Usage per operation.
6. Sawing: the secondary output appears at the bottom (y 77), height 187.
7. Ultimate: width 210, slots 9 columns, inventory at x 26.
8. Restart and chunk unload: progress, energy, tank stay.
9. Hopper: top fills, bottom empties, side fills the shared slot.
10. Break the block: the drop is only the block (contents fall as items, no duplicate).
11. Tier Installer: Basic on a filled Enrichment Chamber (items, energy, progress stay, facing stays), Advanced on a Basic Factory, an Ultimate Factory stays unchanged, a wrong tier is ignored. Non-Creative: the installer is consumed. Check that nothing is duplicated or lost.
12. The loop sound plays only while working.
