# Known Issues

As of 10.11.2026, version 0.2.0 (content mods `mekanismtools` and `mekanismadditions` 0.1.0). Collected from the blocker notes in [TODO.md](../TODO.md), [TOOLS.md](TOOLS.md), [ADDITIONS.md](ADDITIONS.md) and [TESTLOG.md](TESTLOG.md). Details and reasons are listed there. "Horizon limit" means: Horizon's content path or native hook API (`fdc87ae`, API 411) does not offer it.

## General

- **Prototype.** Use it only in test worlds. Save migrations and ID stability between versions are not guaranteed (M13 open).
- **Native mod required.** Machines, generators, energy and tanks only work with the native mod ([INSTALL.md](INSTALL.md)). It runs without a sandbox, with user permissions.
- **Horizon pin.** Tested only with `fdc87ae`. Newer builds can change the hook API.
- **No configuration.** Values (tiers, energy use, material values, TNT radius) are fixed Mekanism defaults. Horizon content has no config.
- **Multiplayer, mod interop (AE2 and others), performance** with many machines, and shader compatibility are not systematically tested.
- **No transport.** Cables, pipes, tubes and logistics are missing (M07/M08). Energy only flows between adjacent blocks.
- **Missing mod fluid buckets.** Horizon has no buckets for mod fluids. Only tanks store them.
- **Native tests do not run with `cargo test`.** The crate `native/mekanism_native/api` depends on Horizon's client and cannot be built without the launcher toolchain. CI only checks content validation and `logic/energy`.
- **Line-ending warnings** (`LF will be replaced by CRLF`) on `git add` under Windows are harmless.
- **Inconsistent upstream reference.** TODO and `docs/CHEMISTRY.md` name Mekanism commit `31bb4906`. `docs/TESTLOG.md` names `330b7b4d`. Unify them at the next upstream comparison.

## Machines and generators

These apply to all of the following: Metallurgic Infuser, Enrichment Chamber, Crusher, Osmium Compressor, Combiner, Precision Sawmill, Purification/Injection Chamber, Chemical Oxidizer and Infuser, Chemical Dissolution Chamber, Washer and Crystallizer.

- Missing: energy slot, upgrades, side configuration, redstone control.
- Infuse contents and energy do not stay on the item when the block is broken (Metallurgic Infuser).
- Recipe changes and chunk unload are not tested for every machine.
- Bar colors are approximated. The GUI layout is simplified (176x166 instead of Mekanism's larger interface for the Chemical Infuser).
- Purification/Injection take chemicals from items for now (flint: oxygen, salt: hydrogen chloride). Water vapor recipes only work once a machine exists that produces it.
- Chemical Oxidizer/Infuser: no chemical item slots, no auto-eject switch (always all sides), no recipe-pair check when filling, no arrow animation. Crafting recipes (`mek_data` with Dynamic Tank) are missing until Dynamic Tank/Personal Storage exist. Test with `/give`.
- Item outputs with a chemical second output are missing in the recipe system.
- Ore chain ([ORE_PROCESSING.md](ORE_PROCESSING.md)): Energized Smelter is blocked (vanilla furnace for dust to ingot), there is no water bucket for the Washer, and wrong slurry in the Washer input blocks it (Horizon's `accepts` hook does not know the position or the other tank).
- Fluid outputs (Electrolytic Separator, Pump) and item outputs with a chemical second output are missing in the recipe system.
- Machine sounds, tooltips and crafting recipes are not checked in-game everywhere ([TESTLOG.md](TESTLOG.md)).

## Energy Cube

- The per-side path through Horizon's `insert_faces` is not tested with a real source (debug blocks write NBT directly).
- Restart test of the side setting, crafting recipe in-game and tooltip with energy are open.
- Tablet does not stack (`max_stack_size` 1): test open.
- One-time, non-reproducible jump from 252,000 to 504,000 FE in test M01. Probably item energy, not duplication (see TESTLOG).

## Tanks and chemistry

- Fluid display is wrong with a **shader pack** (Bliss). The cause is in Horizon's shader path for `draw_fluid_box`. It is correct without shaders.
- Production of many fluids/chemicals is still missing (Dissolution Chamber, Washer, Crystallizer, Neutron Activator and others).
- **Electric Pump:** pumps only water and lava (no heavy water through the filter upgrade, no upgrades at all), no fluid container slots, no redstone control, no reset by sneak-right-click (it resets itself when no source is found any more). Water in waterlogged blocks (kelp, seagrass) is not pumped. Tank output goes only upward, like Mekanism.
- **Fluidic Plenisher:** accepts only water and lava (Horizon has no world blocks for Mekanism fluids). Replaceable blocks are a fixed list (grass, fern, bush, vines, fire, glow lichen), because Horizon does not expose `canBeReplaced`. No upgrades, no redstone, water in the Nether does not evaporate.
- **Gas-Burning Generator:** tank reachable from all sides, no item slots, see [GENERATORS.md](GENERATORS.md).
- Full tanks, throughput limits and world-restart tests are not complete.

## Bins, Personal Chest and Personal Barrel

- **Bins hold only plain items.** The Bin rejects items with components (enchantments, names, damage). Mekanism stores the whole stack.
- **Creative Bin:** capacity `i32::MAX` instead of `Long.MAX_VALUE` (Horizon's item counters are 32 bit).
- **Missing compared to Mekanism:** side configuration and redstone control of the Bin. The lock only works with sneak-right-click on the front.
- **Hopper on the Bin:** a hopper sees two slots (input and a window of up to one stack). Whether a hopper below empties it fast enough is not measured.
- **Personal Chest/Barrel without Security Desk.** Horizon has no player security. The owner is the placer's UUID in the block entity NBT. It is checked on right-click and on break. Explosions, pistons and mods that read block entities directly bypass it. Operators are always allowed.
- **No custom look in the menu:** it uses `minecraft:generic_9x6`, without a Mekanism texture.
- **Chargepad not ported** (Horizon limit): the block must charge the items of all players on it every tick. `player_tick` and `set_inventory_slot` exist, but there is no helper that writes energy into other players' items. Item energy is stored as `horizon_energy` in the data. Doable as follow-up work.
- For the in-game test, see [TESTLOG.md](TESTLOG.md), section M08.

## Tools (`mekanismtools`)

See the table in [TOOLS.md](TOOLS.md). In short:

- **Paxel** is only an axe, not a shovel (no grass path). It needs a native `item_use_on` hook.
- **Paxel recipe** starts undamaged. It does not average the durability of the ingredients.
- **Shield** shows the vanilla model instead of the Mekanism texture.
- **Armor trims** have no Mekanism palettes, **no** durability tooltip (`DISPLAY_HP`), no mob spawn equipment, no advancements.
- Pending tests: shield blocking, spear, horse/nautilus armor, anvil repair, enchanting, restart, multiplayer.

## Additions (`mekanismadditions`)

See [ADDITIONS.md](ADDITIONS.md). In short:

- **Obsidian TNT** explodes like vanilla TNT (radius 4 instead of 12, fixed fuse time). A custom entity/explosion is missing.
- **Plastic Road** has no speed boost. **Glow/Transparent** have no emissive/CTM and no face culling.
- **No lava fluid logging** for fences, slabs and Obsidian TNT.
- **Missing recipes:** base Plastic recipe and Glow Panels (need `hdpe_sheet`, M06 chemistry), Pigment Mixer crafting (`hdpe_rod`).
- **Not built:** baby mobs, balloons, walkie-talkies, spawn eggs (entities and networking are missing in Horizon content).
- Open in-game: crafting in the grid, Slick recipe in the Enrichment Chamber, Slick friction, inventory items, restart/chunk unload.

## Advanced chemistry machines (M06)

Pressurized Reaction Chamber, Isotopic Centrifuge and Solar Neutron Activator are **untested in-game** (test plan in [TESTLOG.md](TESTLOG.md)). In Mekanism, the Centrifuge and the Activator are two blocks high. The second bounding part is missing here (see [CHEMISTRY.md](CHEMISTRY.md)). Reaction recipes with plutonium/polonium pellets are missing until the radiation system exists.

## Not yet ported

Multiblocks (Thermal Evaporation, Turbine, Fission/Fusion Reactor, Boiler, SPS and others), cables and transporters, Digital Miner, Teleporter, QIO storage, Jetpack/armor with modules (Mekanism Core), Meka-Tool. Current status: [TODO.md](../TODO.md).

## Reporting bugs

Open a GitHub issue with the Horizon build (commit/nightly), native build name, mod versions, steps to reproduce and a log excerpt. See [CONTRIBUTING.md](../CONTRIBUTING.md).

## Gear and utilities (M11)

Details and feasibility per item: [ITEM_HOOKS.md](ITEM_HOOKS.md).

- **Not registered**: Electric Bow, Flamethrower, Jetpack, Free Runners, Scuba, Hazmat, MekaSuit, Meka-Tool, Geiger Counter, Dosimeter, Crafting Formula, Network Reader, Portable QIO Dashboard, HDPE Elytra. A craftable item without a function would be fake. Reasons: missing systems (radiation, modules, QIO, transmitters, Formulaic Assemblicator), missing `hdpe_sheet`, or hooks exist but are not built yet.
- **Atomic Disassembler**: only the modes Normal and Vein Mining (sneak plus right-click). Slow, Fast and Off are missing. The mining speed is a fixed component. Energy only charges through Horizon's cubes and tablet logic, with no charging slot in the item. Without energy it mines nothing.
- **Canteen**: refill by right-click on a tank with Nutritional Paste (1,000 mB), because the Nutritional Liquifier (M06 group 6, untested in-game) only knows vanilla food and does not fill chemical items.
- **Gauge Dropper**: works on block tanks and machines with a fluid tank, not on GUI gauges.
- **Dictionary**: checks a generated tag list (Horizon cannot list tags). So it knows no tags outside the content mods and fewer vanilla tags.
- **Canteen**: gives no saturation. A drunk hunger point drops again at saturation 0 like any other (whether the original gives saturation is not checked).
- **Dictionary (offhand)**: Horizon offers no read access to the offhand. The item there is only recognized after the next swap (`on_equip`), not right after loading the world.
- **Item changes** (energy, fill level, mode) take effect with up to one tick of delay: use hooks must not write the inventory, so the change is applied in the next `inventory_tick`. When the stack is rebuilt, other components are lost, but these items have none.
- **Numeric values** (capacities, costs, step sizes) are Mekanism's default values to the best of our knowledge, not read from the Java source (sparse checkout without `src/main/java`).

## Pipes, Tubes, Conductors (M06, untested in-game)

- **No network**: capacity and rate of a tier apply per block, not per network. Long lines buffer more than Mekanism would. Throughput is the tier rate per block and tick.
- **No side modes**: Pull/Push/Off and the Configurator are missing. Pipes only take what neighbors push into them (tank auto-eject downward, machine outputs, pump upward). They do not pull from tanks.
- **Model**: Mekanism's OBJ models (custom loader) are missing in Horizon. The parts are cuboids. The selection and collision box is only the center cube (Horizon shapes depend on one property, not six).
- **Connections** are set by the tick (`down`..`east`) with up to one tick of delay and one check per neighbor per tick. Many pipes can make this noticeable.
- **Heat**: only conductor to conductor and to the Heat Generator. Each conductor calculates on its own in tick order instead of in one network round.
