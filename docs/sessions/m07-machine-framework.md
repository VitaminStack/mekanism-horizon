# Session m07-machine-framework (machine framework: upgrades, energy slot, redstone, configurator)

Branch `claude/m07-machine-framework-b79164`. Not tested in game, native code not compiled (see below).

## What was ported

| Part | Value | Upstream source |
|---|---|---|
| Upgrade items `mekanism:upgrade_speed/energy/muffling` + item-only `chemical`, `anchor`, `filter`, `stone_generator`, `xp_containment` | one item per kind (upstream 26.3: one item `mekanism:upgrade` with data component `upgrade_type`) | `MekanismItems.UPGRADE`, `UpgradeUtils` |
| Maximum counts | Speed 8, Energy 8, Muffling **1** (26.3: `Upgrade.create` without max = 1), Chemical 8 | `MekanismDatapackRegistries` (Upgrades) |
| Ticks | `max(1, trunc(def * 10^(-speed/8)))` | `UpgradeUtils.getTicks` |
| Energy usage | `ceil(def * 10^(2*speed/8 - energy/8))` | `UpgradeUtils.getEnergyPerTick` |
| Capacity | `def * 10^(energy/8)` | `UpgradeUtils.getMaxEnergy` |
| Multiplier 10 | `maxUpgradeMultiplier` default | `GeneralConfig` |
| Muffling | volume `1 - muffling/max` (1 upgrade = silent) | `TileEntityMekanism.getSoundVolume` |
| Redstone modes Disabled/High/Low/Pulse | Pulse = powered and previous tick unpowered | `IRedstoneControl.RedstoneControl`, `TileEntityMekanism.isRedstoneActivated` |
| Energy slot | charges from Energy Tablet, energy cubes, Disassembler at their charge rate | `EnergyInventorySlot` |
| Recipes of the upgrades and the Configurator | imported (result rewritten to single items) | `UpgradeRecipeProvider`, `configurator.json` |
| Configurator (mode Rotate) | right-click turns the front to the clicked side (machines horizontal only, cubes all six sides) | `ItemConfigurator.useOn` (ROTATE) |

Code: `native/mekanism_native/api/src/upgrade.rs` (Horizon-free, tests in `logic/energy`), `machine.rs` (table: `all_slots`, `energy_slot`, `upgrade_slot`, `upgrade_out_slot`, `upgraded`, `data_slots`), `process.rs` (tick, screen, buttons), `lib.rs` (hooks `use_item_on`, `menu_slot_rules`, volume, `item_buffer`).
Generators: `scripts/gen_resources.py` and `scripts/import_upstream.py`, each with one marked M07 block.

## Usage

* Slots come after the machine's own slots, appended (old saves stay valid): energy slot, upgrade input, upgrade output (results slot).
  Positions (bottom edge, `machine::ENERGY_SLOT_POS` etc.): x 98 / 118 / 138, y 54.
* Put an upgrade in the input: it is installed at once (as far as there is room). The small buttons `S`, `E`, `M` (they show the installed count) move one of that kind back to the output on click.
* Button `RS:` cycles through Disabled, High, Low, Pulse.
* Solar Neutron Activator: no slots/upgrades/redstone (upstream: only XP containment).

## Gaps and blockers

* **Smelter**: `minecraft:smelting` stays unreachable. `recipes_of_type` returns only non-`minecraft` types, `minecraft:*` recipes are internal (`crafting.rs`, split `cooking`/`custom`). The Energized Smelter stays unported.
* **Energy capacity** is static per block type in Horizon (`native_block_entity_energy`, asked once at construction). The declared value is the maximum (Energy upgrades x8 = 10 times); the tick limits the stored value to the actual, scaled capacity. Because of this, foreign sources can feed up to 10 times as much between two ticks, and the rest is lost.
* **Side configuration/auto-eject**: Horizon has static sides per block type (hopper faces are part of `ModContainer`, not per block entity). A per-side switchable configuration of the machines and the Configurator modes items/fluids/chemicals/energy/heat is not portable. Auto-eject of the machines stays as in M06 (tanks) or off. Energy cubes have their own side configuration (M03).
* Configurator: modes Wrench and Empty are missing (no item mode: items here carry no typed component), sneak rotation (opposite side) is missing, no chat message. The model variants `configurator_empty/rotate/wrench` are imported but unused.
* Chemical/Anchor/Filter/Stone Generator/XP upgrades: items only, machines do not accept them (effect missing: gas usage, chunk loading, pump, Cobblestone Generator, XP).
* Upgrades do not stay on the broken block (upstream: data component on the item); `loot_drops` does not copy them. They are lost when the block is broken (follow-up work: drop them as items or as an SNBT component like energy cubes).
* Shift-click (`menu_quick_move`) into the new slots is not implemented; player clicks are governed by `menu_slot_rules`, hoppers do not reach the new slots (not in `ModContainer.faces`).
* Pulse: one tick of release per edge (as upstream); for long operations this means only one tick of progress is made.
* The progress display converts to table ticks so the bar is correct with Speed upgrades. The Reaction Chamber keeps its recipe duration (Speed acts only through energy usage).
* The layout of the three slots/buttons is freely chosen (one 176x166 screen instead of upstream's upgrade window); collisions with the tank displays of some machines are not checked.

## Not checked by the compiler

The native build did not run (coordination rule). Not checked against the compiler, only read against the Horizon source:
`UseItemOnEvent` (`stack`, `pos`, `face`), `MenuSlotRulesEvent` (`menu`, `slot`, `stack`, `may_place`), `host.neighbor_signal(&dimension, pos)`, `host.item_from_snbt`, the visibility of `items::edit_data` (now `pub(crate)`). `rustfmt` parses all changed files.

## Checks

* `cargo test --manifest-path logic/energy/Cargo.toml`: 144 tests ok (new: `upgrade::*`, `machine::extra_slots_*`, `upgrades_change_*`, `menu_files_match_the_table`).
* `python scripts/validate_content.py`: ok. `python scripts/parity.py --check`: 0 regressions.
* `cargo horizon-mod build` / `check` for `content/mekanism`: ok.

## In-game checklist (bundled)

1. Build the game with `Build-Content.ps1`. At start: no loading errors; `upgrade_*` and `configurator` in the creative inventory with model and names.
2. Recipes: Speed Upgrade (osmium dust, infused alloy, glass), Muffling (wool + muffling center), Configurator.
3. Open the Enrichment Chamber: energy slot, upgrade input, upgrade output visible, nothing overlaps, buttons `RS:Disabled`, `S0 E0 M0` visible. An old machine from an earlier save loads without loss of input/output.
4. Put a full Energy Tablet in the energy slot: the machine charges at 5000 FE/t up to capacity, the tablet discharges.
5. Insert a Speed upgrade x1: it disappears, display `S1`, usage tooltip rises, an operation takes ~149 instead of 200 ticks. x8: 20 ticks, 800 FE/t (Crusher: 8*100).
6. Energy upgrade: capacity and usage as in the tooltip. Click `S`/`E`: one upgrade in the output, the counter drops; the output accepts no items from the player.
7. Muffling x1: operating noise goes silent; after removal it is back.
8. Redstone: High = only with signal, Low = only without, Pulse = one operation tick per edge; mode persists after restart.
9. Right-click the Configurator on a machine/energy cube: the front turns to the clicked side; the top of a machine is ignored.
10. Open the Solar Neutron Activator: as before, without extras.
11. The energy slot accepts only energy items, the upgrade input only upgrades (also check with shift-click).
