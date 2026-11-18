# Mekanism Horizon - Porting TODO

**As of:** 18.11.2026  
**Goal:** Port the current Mekanism to Minecraft Horizon step by step, with Windows/Visual Studio 2026 as the development environment.

## Reference and working rules

- **Original:** [mekanism/Mekanism, branch `26.3`](https://github.com/mekanism/Mekanism/tree/26.3), compared with commit [`31bb4906`](https://github.com/mekanism/Mekanism/commit/31bb4906bc6c2ea4a2422d93cd1214df3ea7de4e).
- **Original version from `gradle.properties`:** Mekanism **10.8.0**, Minecraft **26.3**, NeoForge **26.3.0.101**, `release_type=alpha`. This is a development base, not automatically a stable release.
- **Horizon base:** [`horizon-rust/horizon` @ `fdc87ae`](https://github.com/horizon-rust/horizon/commit/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca) (nightly 2026-11-08, installed in the launcher), native hooks API 411; the native mod is built against this commit and tested in game. The pin is in `native/mekanism_native/horizon-mixin.toml` and `api/Cargo.toml`.
- **Planned scope:** Mekanism Core, then the original subprojects **Generators, Tools, Additions**.
- **Status:** `[x]` means **confirmed**. `[ ]` means open, even if code/files already exist. Do not check an item only because a build succeeded.
- **Every change:** check original behavior and registry IDs; keep content (`.hmod`) and native logic (`.hnmod`) apart; test save/reload; document tests and in-game proof in the PR.

## Next 10 concrete tasks - do these first

- [x] **N01** Check the local Energy Cube patch state: `git status --short` and `git apply --reverse --check .\mekanism-energy-cube-v0.2.0.patch`. Do NOT overwrite existing files. *(09.11.2026: working tree clean, patch content already committed, patch file no longer exists.)*
- [x] **N02** After the patch, run `Test-Content.ps1` and `cargo test --manifest-path .\logic\energy\Cargo.toml` again successfully.
- [x] **N03** `Ctrl+B`: build the `.hmod`, copy it automatically; test `/give @s mekanism:basic_energy_cube` in game and place the block. *(09.11.2026: give, place and drop confirmed; loot tables need the Horizon `condition` format.)*
- [x] **N04** Determine the native Horizon API version/nightly commit and align `native/mekanism_native/` with it. *(09.11.2026: pinned to Horizon `fdc87ae` (installed nightly); hook signatures only checked statically against 99bd9655 (API 413). Compiling only in N05.)*
- [x] **N05** Build the native mod with the Horizon Launcher and install it with a deliberately granted trust approval; check the logs. *(09.11.2026: `scripts/Build-Native.ps1 -TrustNativeCode`, build `native-fdc87ae-8abc9345573f`, confirmed in game.)*
- [x] **N06** Placing creates a native block entity with an energy capability. *(09.11.2026 via Agent Control: all 5 cubes have a block entity.)*
- [x] **N07** Basic Energy Cube: 4,000,000 capacity, 4,000 input/output limit per call (default from the original `EnergyCubeTier`; configurable later).
- [x] **N08** Insert and extract energy with a test hook/generator; test sides, limits, empty/full. *(09.11.2026: `debug_energy_source`/`debug_energy_sink` via Agent Control: 4,000/tick, cap at 4,000,000, drain to 0. They write the cube NBT directly; Horizon's own side path (`insert_faces`) untested.)*
- [x] **N09** Save the world, quit Horizon and start it again: stored energy must stay. *(09.11.2026: `energy: 123456` survives save + restart.)*
- [x] **N10** Document the successful in-game test, build log and screenshots, commit, create a PR. *(09.11.2026: log in `docs/TESTLOG.md`; screenshots only in chat.)*

## M00 - Working project base and quality [P0]

- [x] Own Windows repository with `content/`, `logic/`, `scripts/` set up.
- [x] Rust/Cargo installed on Windows and four energy core unit tests pass.
- [x] Build the content mod with `Ctrl+B` in Visual Studio 2026.
- [x] Copy the `.hmod` to `horizon-mods/` automatically after a successful build.
- [x] Osmium ingot and osmium block tested in game.
- [x] Version the local v0.2.0 changes safely; sync remote and local. *(09.11.2026: working tree clean, all branches pushed.)*
- [x] Pin the Horizon nightly, native hook API and Mekanism upstream commit.
- [x] Document the test and build guide for Windows. *(`README-WINDOWS.md`.)*
- [x] CI: content validation, Rust tests, JSON checks; native build when reproducible. *(`.github/workflows/ci.yml`; native build not in CI, it needs the launcher.)*
- [x] Check and include the license and copyright of the original assets/code taken over. *(MIT, `content/mekanism/LICENSE-MEKANISM`.)*

## M01 - Basic Energy Cube: first fully working block [P0]

- [x] Block, item model, texture, loot, translations and recipe are correct. *(Original model without LEDs/core, loot as in the original without the explosion condition, original recipe. In game: recipe, tooltip ok; missing `c:` tags added.)*
- [x] Create/remove the native block entity correctly on place and break.
- [x] Connect the Horizon energy hook (from API 142) with `ModEnergyBuffer`. *(Connected; sides only per block type, not per block entity.)*
- [x] Limit capacity, input, output and simulation calls correctly. *(Unit tests `cube::transfer`; foreign energy source not available in game.)*
- [x] Test all sides/facing; configure input/output. *(Facing + side config via GUI, tested 09.11.2026.)*
- [x] NBT persistence on save, chunk unload and restart.
- [x] Energy transfer without duplication or loss. *(Eject only into Mekanism cubes; total constant in game; unit tests `cube::transfer`.)*
- [x] GUI: stored energy, transfer limit, I/O sides and status. *(Own screen via block menu `mekanism:energy_cube`.)*
- [x] Compare behavior on break/re-place with the original. *(Item keeps energy + sides as in the original.)*
- [x] Finish integration tests + in-game demonstration. *(A→B transfer, GUI, break, chunk unload, restart in game, 09.11.2026.)*

**Done when:** the cube takes real energy, gives it out, can be configured and keeps its charge after a restart.

## M02 - Resources, world generation and basic recipes [P1]

- [x] Inventory the original registry IDs of items/blocks/resources. *(Resource IDs after Mekanism 26.3 in `scripts/gen_resources.py`, list in `scripts/registry.json`.)*
- [ ] Integrate original textures, models, blockstates, sounds and languages with license notes. *(For resources via `scripts/import_upstream.py`; MIT license in `content/mekanism/LICENSE-MEKANISM`. Block sound types as in the original (stone, deepslate, salt = sand; tested in game 09.11.2026). Energy Cube model via `scripts/import_energy_cube.py`. Sounds: `sounds.json` + `.ogg` of the existing machines and generators (Infuser, Enrichment Chamber, Crusher, Compressor, Combiner, Precision Sawmill, Purification and Injection Chamber, Heat, Solar) via `import_upstream.py` (`SOUNDS`), with subtitles; Horizon plays mod sound events since API 61, the native mod repeats the clip while the machine runs. Sounds incl. Bio and Wind tested in game 09.11.2026. The remaining Mekanism sound events follow with their machines/items.)*
- [x] Osmium, tin, uranium and fluorite ores including deepslate variants. *(Plus lead and salt.)*
- [x] Test world generation, ore distribution, drops and mining level (new chunks only). *(Values from the original `OreType`/`WorldConfig`. No Fortune/Silk Touch: Horizon loot does not know `apply_bonus`/`match_tool`.)*
- [x] Ingots, nuggets, dusts, raw materials, storage blocks (e.g. bronze, steel, refined glowstone, refined obsidian).
- [ ] Steel Casing, Control Circuits (Basic/Advanced/Elite/Ultimate), alloys and enriched materials. *(Registered. Infusing recipes run through the Metallurgic Infuser (base tier from M05; tested in game 09.11.2026: Carbon, Enriched Iron, Steel Dust, Steel Casing, dump, tooltips). Elite/Ultimate circuits and Reinforced/Atomic Alloy: chain Crusher (obsidian/diamond dust) and Enrichment Chamber (Enriched Diamond/Refined Obsidian) implemented, untested in game.)*
- [x] Crafting, smelting, blasting and resource tags after the original recipes. *(Confirmed in game 09.11.2026: steel ingot in the furnace, Steel Casing, infuser recipe. Vanilla recipes + tags imported, plus `metallurgic_infusing` and infuse `chemical_conversion`. Missing NeoForge `c:` tags for vanilla items in `gen_resources.py` (`CONVENTION_TAGS`; since 09.11.2026 all named by imported recipes, e.g. `c:ores/iron`, `c:raw_materials/iron`, `c:dyes/*`). More machine recipes follow with their machines.)*
- [ ] Survival test: all materials for the first machines obtainable without cheats. *(Path: craft the Infuser, coal to Carbon, iron to Enriched Iron, then Steel Dust, Steel Ingot (furnace), Steel Casing; osmium + redstone to Basic Circuit. Energy without cheats from a Heat Generator (coal) directly next to the machine. Untested in game.)*

## M03 - Energy and machine foundation [P0/P1]

- [x] Define Joule/FE conversion, overflow protection and units in UI/API. *(Decided 09.11.2026: 1:1 FE like Mekanism 26.3, which has no joules any more (`docs/ENERGY-UNITS.md`). Overflow protection in `energy::Acceptor::accept` with unit tests; Creative Cube capped at `i32::MAX`.)*
- [x] Energy in items and blocks through shared capabilities. *(Horizon's energy API: `item_energy_buffer` for the tablet and cube items, `block_entity_energy` for cubes and the Infuser; tested in game 09.11.2026.)*
- [x] Energy Tablet and all Energy Cubes: Basic, Advanced, Elite, Ultimate, Creative. *(Tested 09.11.2026. Placeholder look; Horizon shows no item charge bar; Creative capped at i32::MAX; crafted cubes do not take over stored energy.)*
- [x] Take over the original tier values, transfer limits and configurations. *(Tier values from `EnergyCubeTier`, tablet from `GearConfig`; not configurable.)*
- [ ] Universal Cables (four tiers), connection graph and neighbor updates. *(Built in Wave 1 (`docs/sessions/m08-cables-transport.md`): 4 tiers with values from `CableTier`, each cable has its own buffer and evens out with its neighbors every tick, arms as block states. Reaches only Mekanism blocks: Horizon gives mods no energy access to foreign blocks. Open: no shared network, no arm towards generators, full chain generator -> cable -> cube not accepted in game.)*
- [ ] Energy distribution across chunks, server ticks and redstone correct.
- [x] Shared block entity, recipe, inventory and save/load framework. *(P0 refactor, tested in game 09.11.2026: one `Kind` dispatch for all hooks, machine table `machine::MACHINES` with one tick/container/screen (`process.rs`), recipe cache per type (`recipe.rs`), NBT helpers for energy/progress/buffers. Tanks (M06) use the same dispatch.)*
- [ ] Load test: 100+ cables and cubes; no energy duplication on network reload.

## M04 - Energy generation / Generators base tier [P1]

- [x] Heat Generator (fuel, environment and output sides). *(Tested in game 09.11.2026: own content mod `content/mekanismgenerators` with upstream IDs, logic in `native/mekanism_native/api/src/generator.rs`. Lava tank 1000 mB, 100 mB/t, lava neighbors 7/side, Nether +10, heat/Carnot model as in the original, energy from the front into a cube/machine. Not ported: energy slot, redstone; heat transfer only to Thermodynamic Conductors.)*
- [x] Solar Generator and Advanced Solar Generator. *(Solar Generator tested in game 09.11.2026: 2 FE/t peak, biome factor from the vanilla climate table, time of day/rain after the vanilla formula, energy downwards. Advanced Solar tested in game 09.11.2026: 12 FE/t peak, multiblock with bounding blocks (column + 3×3 panel), needs more than 4 of the 9 sky checks, storage 240 FE; breaking one part breaks everything, missing space drops the block as an item. Model: foot as block model, top drawn one block higher by the native mod (Horizon ignores the original's root `transform`), via `scripts/import_generator_models.py`.)*
- [x] Wind Generator (height/weather as in the original). *(Tested in game 09.11.2026: 1–5 FE/t linear with the height of the tip (y 24 to the world top), tip must see the sky, storage 100 FE, multiblock 5 high, sound (weather). Model: base as block model, tower and rotor via `scripts/import_generator_models.py` from the Blockbench original, drawn by the native mod, rotor turns while running. Item icon: scaled-down full model instead of Mekanism's special renderer. Screen through every bounding block (mirrored menu), break without ghost block, no drop in creative: tested in game 09.11.2026 (P0 refactor). Not ported: `c:no_wind` dimensions.)*
- [x] Bio Generator and Bio Fuel. *(Tested in game 09.11.2026: 25 FE/t, 1 mB/t from a 200 mB tank, 64 mB per Bio Fuel, storage 600 FE, energy from the front, sound. Bio Fuel from the Crusher (all `crushing/biofuel` recipes), Block of Bio Fuel. Not ported: filling via fluid container/pipe, energy slot, redstone; Block of Bio Fuel as furnace fuel.)*
- [x] Gas-Burning Generator. *(Tested in game 11.11.2026: 1,000 mB chemical tank, hydrogen 2 and ethene 8 FE per mB, at most 10 mB/t by fill level, storage 2,000 FE, energy from the front into an Energy Cube, sound, GUI. Not ported: chemical/energy slots, tank on all sides instead of all but the front (Horizon faces per block type), redstone. Details in `docs/GENERATORS.md`.)*
- [ ] GUIs, energy output, upgrades and balancing of the generators.
- [ ] Survival test: generator -> Universal Cable -> Energy Cube -> machine.

## M05 - Machines and first processing chains [P1]

- [ ] General recipe system: inputs, outputs, energy per tick, progress, byproducts. *(Partly: `recipe::Recipe` reads item input with count, optional chemical input and output for all machine recipe types; energy per tick and progress via `machine::step`, tested in game 09.11.2026. Since M05 part 2 also a second item input (`extra_input`), secondary output with chance (`secondary_output`) and chemical per tick (`per_tick_usage`); tested in game 09.11.2026. Since M06 also purification/injection (`purifying`, `injecting`; oxygen, hydrogen chloride, steam per tick; tested in game 10.11.2026). Since the Oxidizer/Infuser (M06) also chemical output into an output tank (`Recipe::chemical_output`, `FluidTanks`) and two chemical inputs (`recipe::Infusion`); a full output tank pauses like a full item output. Since Separator/Rotary also two chemical outputs from one fluid (`recipe::Separation`, `machine::convert`) and fluid tanks (`recipe::Rotation`). Missing: fluid outputs of the pump and item outputs with a chemical secondary output.)*
- [ ] Machine inventories, menus, GUI, upgrades, auto-eject, sided I/O, redstone modes. *(Wave 1 (`docs/sessions/m07-machine-framework.md`): energy slot, upgrade slots, redstone modes Disabled/High/Low/Pulse; accepted in game 18.11.2026: Energy Tablet charges the machine, Speed x8, upgrades and energy survive a restart. Blocked: sided I/O and auto-eject per block entity (Horizon sides are static per block type).)*
- [x] Metallurgic Infuser and infusion resources. *(Tested in game 09.11.2026. Values from the original; 8 infuse types, all `metallurgic_infusing` recipes. Missing: side config; infuse/energy do not stay on the item when broken; bar colors approximated.)*
- [x] Enrichment Chamber. *(Tested in game 09.11.2026: `native/mekanism_native/api/src/machine.rs`, 5 FE/t, 200 ticks, storage 20 FE, all `mekanism:enriching` recipes, upstream model with `active`, sound.)*
- [ ] Energized Smelter. *(Blocked: Horizon gives native mods no `minecraft:smelting` recipes; `recipes_of_type` only returns types outside `minecraft`, `native_recipes` is internal.)*
- [x] Crusher. *(Tested in game 09.11.2026: like the Enrichment Chamber, 8 FE/t, storage 32 FE, all `mekanism:crushing` recipes.)*
- [x] Precision Sawmill. *(Tested in game 09.11.2026: 6 oak logs to 36 planks + 1 sawdust, bed to planks + wool; pauses while the secondary output does not fit. 5 FE/t, storage 20 FE, all `mekanism:sawing` recipes, secondary output by `secondary_chance`, sound.)*
- [x] Osmium Compressor and Combiner. *(Tested in game 09.11.2026: refined obsidian dust to ingot, osmium 1 mB/tick, stops with an empty tank, an osmium block does not fit in the tank; raw iron + (deepslate) cobblestone to ore; pauses on an occupied output; content after restart. Compressor: 15 FE/t, storage 60 FE, osmium tank 210 mB from osmium ingots (200 mB), 1 mB per tick, all `mekanism:compressing` recipes. Combiner: 5 FE/t, second input at the bottom, all `mekanism:combining` recipes. A full output now pauses as in the original instead of resetting progress (all machines).)*
- [x] Oredictionificator. *(Wave 1, accepted in game 18.11.2026: GUI, filter `dusts/bronze` to `dust_bronze`, bronze dust is converted, cobblestone stays in the input, filters survive a restart. Not ported: redstone mode, side config, security, upgrades.)*
- [ ] Formulaic Assemblicator. *(Block, item, recipes and models registered, no behavior. Blocker: native mods get no vanilla crafting recipes.)*
- [ ] Check empty energy, full output, recipe change, save/load and chunk unload. *(Partly in game: a full output pauses (Combiner, Sawmill), a restart keeps the content, an empty chemical tank holds the progress (Compressor, Purification, Injection). Open: recipe change, chunk unload.)*

## M06 - Fluids, chemicals and ore processing [P1]

**Groups (each its own branch, in this order):**

1. `feat/m06-chemistry-foundation` - chemical IDs/registry, fluids, chemicals, Fluid Tank, Chemical Tank.
2. `feat/m06-transport-heat` - Mechanical Pipes, Pressurized Tubes, Thermodynamic Conductors, heat system.
3. `feat/m06-basic-chemical-machines` - Electric Pump, Fluidic Plenisher, Electrolytic Separator, Chemical Oxidizer, Chemical Infuser, Rotary Condensentrator.
4. `feat/m06-ore-processing` - Purification Chamber, Chemical Injection Chamber, Dissolution Chamber, Washer, Crystallizer, ore processing end to end.
5. `feat/m06-advanced-chemistry` - Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator.
6. `feat/m06-nutrition-pigments` - Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine; then tank/throughput/restart tests.


- [x] Plan chemical IDs/registry and network/datapack sync. *(Plan in `docs/CHEMISTRY.md`, tables via `scripts/gen_chemicals.py` (18 fluids, 64 chemicals). Chemicals run as `mekanism:chemical/<id>` fluids in Horizon's block entity tanks, content sent to the client via update tag; tested in game 09.11.2026 with hydrogen and brine.)*
- [ ] Fluids: water, steam, brine, lithium, heavy water, nutritional paste etc. *(IDs, names and colors in `chemicals_table.rs`; tanks store and show them. Open: production by machines, buckets for mod fluids (Horizon has none).)*
- [ ] Chemicals: hydrogen, oxygen, chlorine, sulfur components, ethene, slurries, pigments, isotopes etc. *(IDs, names and colors in `chemicals_table.rs`; the Chemical Tank stores them. Oxygen and hydrogen chloride fill the Purification/Injection Chamber from items (flint, salt) for now, osmium the Compressor. Open: production by machines.)*
- [x] Fluid Tank and Chemical Tank (Basic/Advanced/Elite/Ultimate/Creative). *(Tested in game 09.11.2026 via `horizon-ctl` (`docs/TESTLOG.md`, M06 tanks): filling, buckets (water/lava), auto-eject downwards, Dumping/Dumping Excess, Creative stays full, item keeps its content, fluid visible in the glass. Tier values by unit test. Not ported: item fill slots, chemical side config, edit mode, redstone; a lava tank gives no block light; drawn wrong with a shader pack (Horizon).)*
- [x] Mechanical Pipes and Pressurized Tubes (four tiers). *(Tested in game 13.11.2026 (base chain, tube with hydrogen, restart; "M06 acceptance" in `docs/TESTLOG.md`); throughput into large tanks (125 instead of 250 mB/t) and tube arm to fluid tanks fixed and rechecked the same day; higher tiers and pipe chains of 200+ blocks untested. 8 blocks `mekanism:<tier>_mechanical_pipe` / `_pressurized_tube`, recipes, names, textures from upstream; capacity/rate per tier from `PipeTier`/`TubeTier` (unit test). Logic `transmitter.rs`, host `transmitters.rs`: one Horizon tank per block (pipe fluids, tube chemicals), pushes up to the rate into neighbors with a lower fill level each tick (no network, capacity and rate apply per block, not per network); arms as block states `down`..`east`. Blockers/limits: Horizon has no transmitter network, Mekanism's OBJ models (own loader) are replaced by boxes; pull/push/off side modes missing. See `docs/CHEMISTRY.md`.)*
- [x] Thermodynamic Conductors and heat system. *(Tested in game 13.11.2026 (Heat Generator with lava, two conductors, about 323 K after 200 ticks, restart); higher tiers untested. 4 blocks `mekanism:<tier>_thermodynamic_conductor`: heat capacity 1, inverse conduction 5, insulation 10/400/8,000/100,000 (`ConductorTier`), heat between neighbors after `ITileHeatHandler.simulateAdjacent` (calorimetry) and loss to the air (`simulateEnvironment`) in `transmitter.rs`; connects with conductors and the Heat Generator (takes and gives heat through its `mekanism_heat`). Limits: no heat network (each conductor computes on its own in tick order), no other heat sources/sinks, since Boiler, Fission, Fusion, Resistive Heater and others are missing.)*
- [x] Electric Pump and Fluidic Plenisher. *(Tested in game 11.11.2026 (`docs/TESTLOG.md`): pump 2 FE/t, storage 8 FE, one source per 19 ticks, water stays (`water_source_conversion`), lava is used up, follows sources within radius 80, gives 256 mB/t upwards into a Fluid Tank; Plenisher 20 ticks per block, breadth-first search downwards, then refill. Logic `pump.rs`, host `pumps.rs`. Not ported: upgrades, heavy water filter, item slots, redstone, sneak reset; limits in `docs/KNOWN_ISSUES.md`.)*
- [x] Electrolytic Separator. *(Tested in game 10.11.2026: 4 FE/t (2 x hydrogen fuel value 2), storage 16 FE, one pass per tick, all `mekanism:separating` recipes (water to 2 hydrogen + 1 oxygen, brine 10 mB to sodium + chlorine), input 24,000 mB, left/right output 2,400 mB each, ejects into all neighbors (1,024 mB/t), a full output pauses, input only recipe fluids. Fluid input through the tank API (`block_entity_fluids`): neighbors and pipes fill it; water bucket missing (Horizon hook only for tanks). Not ported: item slots (fluid/chemical/energy), left/right dump modes, side config; deuterium recipe (`mekanismgenerators:deuterium`, `energy_multiplier` 2) follows with the Fusion/Generators chemical set.)*
- [x] Chemical Oxidizer and Chemical Infuser. *(Tested in game 10.11.2026: Oxidizer 15 FE/t, 100 ticks, all `mekanism:oxidizing` recipes (sulfur dioxide 100 mB per dust), output tank 10,000 mB; Infuser 15 FE/t, one pass per tick, `chemical_infusing` in both tank orders, three tanks of 10,000 mB each. Chemicals sit in Horizon's block entity tanks (`fluids`): neighbor tanks and machines fill the Infuser inputs (`block_entity_accepts_fluid` by recipe), the output is ejected into all neighbors (1,024 mB/t); a full output pauses; content after restart. Missing: chemical item slots (fill/empty tanks by item), side config/auto-eject switch (always all sides), recipe pair check on filling (Horizon's hook does not know the other tank), arrow animation in the Infuser; crafting recipes (`mek_data` with Dynamic Tank) missing until Dynamic Tank/personal storage exist, test via `/give`. Gauge layout simplified (176x166 instead of Mekanism's larger GUI).)*
- [x] Rotary Condensentrator. *(Tested in game 10.11.2026: 2 FE/t, storage 8 FE, one pass per tick, all `mekanism:rotary` recipes in both directions, one tank each (chemical left, fluid right) with 10,000 mB, mode button in the GUI (default chemical -> fluid, stays after restart), the output tank ejects into neighbors; fluid output into water/fluid tank and chemical output into a Chemical Tank. Blocker/limit: Horizon's `block_entity_accepts_fluid` and the tank sides are static, so neighbors fill and empty both tanks regardless of the mode (Mekanism locks the output tank against filling). Not ported: item slots, side config.)*
- [x] Purification Chamber and Chemical Injection Chamber. *(Tested in game 10.11.2026 (`docs/TESTLOG.md`): iron to clumps with oxygen, gunpowder to sulfur with hydrogen chloride, stops with an empty tank. Purification 10 FE/t, Injection 15 FE/t, tank 210 mB, 1 mB per tick, all `purifying`/`injecting` recipes, clump items. Chemicals only from items for now (flint: oxygen, salt: hydrogen chloride); steam recipes run only once a machine makes steam.)*
- [x] Chemical Dissolution Chamber, Chemical Washer, Chemical Crystallizer. *(Tested in game 11.11.2026 (`docs/TESTLOG.md`): Dissolution 16 FE/t, 100 ticks, acid 1 mB per tick, ore to 1,000 mB dirty slurry; Washer 5 FE/t, one pass per tick, 5 mB water per mB slurry; Crystallizer 15 FE/t, 200 ticks, 200 mB clean slurry per crystal; all `dissolution`, `washing`, `crystallizing` recipes, tanks of 10,000 mB each, output tanks eject into neighbors, a full output pauses, empty acid holds the progress. New items: crystals, shards, dirty dusts for 7 metals; slurries were already in the chemical table. Missing: chemical item slots, side config.)*
- [x] Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator. *(Implemented 13.11.2026, tested in game 13.11.2026 (Reaction Chamber, Centrifuge, Activator, GUI, restart; "M06 acceptance" in `docs/TESTLOG.md`; Reaction without energy and Activator in other biomes untested): Reaction 2 FE/t + `energy_required`, duration per recipe, all or nothing; Centrifuge 8 FE/t, one pass per tick; Activator without energy, rate by biome and sun. New items `substrate`, `hdpe_pellet`, Generators chemicals deuterium/tritium/fusion fuel. Limits: 2-block height of Centrifuge and Activator (bounding not ported), pellet recipes missing (radiation), side config; details in `docs/CHEMISTRY.md`, test plan in `docs/TESTLOG.md`.)*
- [x] Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine. *(Done, tested in game 13.11.2026 (Liquifier, Extractor, Mixer, Painting, restart; "M06 acceptance" in `docs/TESTLOG.md`; a one-time loss of 2 red dyes in the Extractor not reproduced); unit tests in `logic/energy`. Liquifier 8 FE/t, 100 ticks, fluid tank 10,000 mB Nutritional Paste (food x 50 mB), 40 vanilla food recipes `mekanism:liquification` (Horizon has no API that reads an item's food values, hence a list instead of Mekanism's dynamic recipe; mod food and container return (bowl) missing); Pigment Extractor 5 FE/t, 100 ticks, pigment tank 20,000 mB, 275 `pigment_extracting` recipes; Pigment Mixer 5 FE/t, one pass per tick, inputs 1,000 mB, output 2,000 mB, 16 `pigment_mixing` recipes (crafting needs `hdpe_rod`, M06 Advanced Chemistry); Painting Machine 5 FE/t, 200 ticks, pigment tank 15,000 mB, 288 `painting` recipes (13 categories, `neoforge:difference` resolved to lists in the importer), new item `dye_base`. All run through existing operations (`Op::Item`, `Op::Infuse`); not ported: chemical item slots (pigment items), side config, arrow animations.)*
- [x] Ore processing and multiple yields (up to the original peak values) as an end-to-end chain. *(Tested in game 11.11.2026: 1 iron ore to 5 iron ingots via Dissolution, Washer, Crystallizer, Injection, Purification, Crusher, Enrichment, furnace; description in `docs/ORE_PROCESSING.md`. Limits: Energized Smelter blocked (vanilla furnace), sulfuric acid and water via `/data merge` instead of the full Oxidizer/Infuser/Rotary chain.)*
- [ ] Test full tanks, throughput limits, chemical/fluid losses and world restarts.

## M07 - Factories, upgrades, automation [P2]

- [ ] Basic/Advanced/Elite/Ultimate Factories for the relevant processing types. *(Built in Wave 1 (`docs/sessions/m07-factories.md`): 32 factories for Enriching, Crushing, Compressing, Combining, Purifying, Injecting, Infusing, Sawing. Not accepted in game yet; the capacity formula differs from the original.)*
- [ ] Parallel processing, slot assignment, upgrade mechanics. *(Built with the factories, not accepted in game.)*
- [ ] Speed, Energy, Muffling and other original upgrades. *(Speed x8 accepted in game 18.11.2026: 1 operation per 20 ticks, buffer 9,500 FE. Energy and Muffling not tested in game. Chemical, Anchor, Filter, Stone Generator and XP upgrades exist only as items without effect.)*
- [ ] Tier Installer and factory upgrades. *(4 Tier Installers built, not accepted in game.)*
- [ ] Configurator: rotate, side selection, redstone modes. *(Rotate mode accepted in game 18.11.2026 (machine east/south, cube up). Other modes blocked: Horizon sides are static per block type.)*
- [ ] Configuration Card and Network Reader.
- [ ] Logistical Sorter and production-ready automation.
- [ ] Redstone/comparator/hopper compatibility and stress tests.

## M08 - Logistics, storage and special machines [P2]

- [ ] Logistical Transporters (four tiers), Restrictive and Diversion Transporter. *(Built in Wave 1 (`docs/sessions/m08-cables-transport.md`). Accepted in game 18.11.2026: arm click sets "Pull", items flow chest to chest. Not tested in game: tier speeds, Restrictive, Diversion with redstone.)*
- [ ] Route finding, filters, priorities, backpressure and network reload. *(Dijkstra over transporter blocks (512 blocks), restart keeps items in transit (unit tests); no colors, no stack merging.)*
- [x] Bins (Basic to Ultimate and Creative): amount in the block entity NBT (Horizon slots end at 99), hoppers fill/empty through two slots, right-click on the front puts the hand in, left-click on the front takes 1 item (sneak: stack), sneak + empty hand locks the type (front) or toggles auto-eject (sides). The item keeps its content, the front shows item and amount. Capacities after `BinTier`, Creative `i32::MAX`. See `native/.../bin.rs`, `bins.rs`.
- [x] Personal Chest and Personal Barrel: 9x6 menu, owner = placer (UUID in NBT), opening and breaking only for the owner or operators, hoppers locked out. No Security Desk (Robit/Security Desk open). See `storage.rs`, `storages.rs`.
- [ ] Chargepad: not ported. The block needs player inventory access per tick and item energy of foreign items; Horizon only offers `player_tick` and `set_inventory_slot` for that, but no item energy helper. See `docs/KNOWN_ISSUES.md`.
- [x] Digital Miner. *(Wave 1, accepted in game 18.11.2026: 17 bounding blocks + main block (3×3×2); without a filter and not inverted it mines nothing (as in the original); inverted with radius 2, Y 60 to 63 it mines, uses energy, shows the active model and ejects into the chest 2 behind and 1 above; Silk Touch gives stone instead of cobblestone at 12× energy; settings survive a restart. Not ported: redstone, side config, security, upgrades, XP, anchor.)*
- [ ] Seismic Vibrator. *(Not built.)*
- [x] Teleporter, Portable Teleporter, Teleporter Frames. *(Wave 1, accepted in game 18.11.2026: teleport A to B costs energy (-119 FE), too little energy means no teleport; Portable Teleporter teleports (-116 FE); frequencies survive a restart. Channel selection replaces the text entry GUI; GUI labels overlap.)*
- [ ] Quantum Entangloporter with energy, items, fluids and chemicals. *(Wave 1. Accepted in game 18.11.2026 for items and energy: sharing between members, restarts without duplication, a member whose chunk unloads gives its window back. Fluids, chemicals and members in different dimensions not tested.)*
- [ ] Robit and Security Desk.
- [ ] QIO Drive Array, Drives, Dashboard, Importer, Exporter, Redstone Adapter, Portable Dashboard.
- [ ] Test persistence, dimensions and multiplayer permissions.

## M09 - Multiblocks and industry [P2]

- [x] General multiblock structure system with forming, breaking, chunk borders and validation. *(Wave 1 (`docs/sessions/m09-multiblocks.md`), accepted in game 18.11.2026: forming, invalid structure without a provider, dissolving and re-forming keep the content, chunk unload keeps the structure. Polling: forming can take up to about 10 s.)*
- [x] Dynamic Tank and Dynamic Valve. *(Accepted in game 18.11.2026: 432,000 mB capacity, valve in Both mode takes inflow (fixed during acceptance).)*
- [ ] Thermal Evaporation Plant (controller, valves, blocks). *(Only validator rules prepared, no host logic.)*
- [x] Induction Matrix incl. cells/providers/ports for all tiers. *(Accepted in game 18.11.2026: 3×4×3 with cell and provider forms (8 G capacity, 256 k transfer); port input from a Creative Cube charges the matrix, port output charges a Basic Cube.)*
- [ ] Thermoelectric Boiler incl. heat/steam processing.
- [ ] Industrial Turbine: rotor, coil, vent, Saturating Condenser, logic.
- [ ] Test multiblock GUI, status display, save/load and rebuild.

## M10 - Reactors, radiation and endgame [P3]

- [ ] Nuclear fuel and uranium/fissile recipe chains.
- [ ] Fission Reactor: casings, ports, logic, assemblies, rods, cooling.
- [ ] Radiation system, Radioactive Waste Barrel, Dosimeter, Hazmat.
- [ ] Plutonium, polonium, pellets and their processing.
- [ ] Fusion Reactor, Hohlraum, fuel, laser ignition.
- [ ] SPS (Supercritical Phase Shifter), antimatter, Supercharged Coils.
- [ ] Antiprotonic Nucleosynthesizer.
- [ ] Laser, Laser Amplifier, Laser Tractor Beam.
- [ ] Test stability, shutdown, accident/safety logic and save/load.

## M11 - Gear, items and gameplay aids [P2/P3]

Hook check and feasibility per item: [docs/ITEM_HOOKS.md](docs/ITEM_HOOKS.md). Native in `items.rs`/`gear.rs`/`worn.rs`. Items without a function are not registered.

- [x] Hook audit against `fdc87ae` (API 411): item use, tick in the inventory, energy buffer, item NBT, movement, breathing, keys. Correction: `use_item_on` exists (API 13), see `docs/TOOLS.md`.
- [x] Atomic Disassembler: energy buffer, mining, no durability, weapon bonus (2,000 FE), sneak mode Normal/Vein, vein mining (tag `atomic_disassembler_ore`, 128 blocks). *(Modes Slow/Fast/Off and speed change per mode missing: mining speed is static.)*
- [x] Flamethrower. *(Wave 1, accepted in game 18.11.2026: uses hydrogen, kills a pig. Hitscan cone instead of a flame entity; Heat mode acts like Combat.)*
- [ ] Electric Bow. *(Built in Wave 1. Blocker: Horizon draws and shoots only `minecraft:bow` (hard-coded), and the host API has no call to spawn entities or projectiles.)*
- [ ] HDPE-Reinforced Elytra: blocker, the recipe needs `mekanism:hdpe_sheet` (M06).
- [x] Free Runners and Scuba equipment. *(Wave 1, accepted in game 18.11.2026: step height 1.1, back to 0.6 after taking them off; Scuba keeps air at 300 under water while oxygen drops, without the mask air falls; enchantments survive energy use.)*
- [ ] Jetpack, Armored Jetpack, Armored Free Runners. *(Built in Wave 1 (`docs/sessions/m11-gear.md`), not accepted in game. No armor model on the body.)*
- [ ] Hazmat Suit: blocker, no radiation system.
- [ ] MekaSuit (all parts), Meka-Tool, Modification Station, modules: blocker, no module system.
- [x] Canteen (Nutritional Paste, drinking, bar; refill by right-clicking a tank instead of the Nutritional Liquifier), Gauge Dropper (block tanks instead of GUI gauges), Dictionary (tags only from the generated list).
- [ ] Geiger Counter, Dosimeter: blocker, no radiation system.
- [ ] Crafting Formula (Formulaic Assemblicator has no behavior), Network Reader, Portable QIO Dashboard (QIO and polonium pellets missing): blocker.
- [ ] Test player movement, item NBT, client/server sync and security. *(Item NBT, energy, vein mining, drinking and dropper tested, see TESTLOG; movement, LAN and server open.)*

## M12 - Complete the subprojects [P2/P3]

### Mekanism: Generators
Comparison with upstream 26.3 (`31bb4906`) in `docs/GENERATORS.md`.
- [x] Heat, Solar, Advanced Solar, Wind and Bio Generator (native, M04).
- [x] Solar Panel, Turbine Blade, Hohlraum (items), Turbine Casing/Rotor, Fission Reactor Casing, Fission Fuel Assembly, Control Rod Assembly, Fusion Reactor Frame, Reactor Glass, Laser Focus Matrix, Electromagnetic Coil, Rotational Complex, Saturating Condenser (blocks without logic): models, textures, loot, lang and recipes.
- [x] `mekanismgenerators` namespace, mod manifest and dependency on `mekanism` checked (the manifest has no dependency field).
- [ ] Industrial Turbine, Fission Reactor, Fusion Reactor (multiblock forming, controller, valves, vents, ports, logic adapter, GUIs): blocker, needs native multiblock code (M09/M10). Turbine Blade cannot be put into the rotor yet. *(Wave 1 added Turbine Valve/Vent, Fission Reactor Port/Logic Adapter, Fusion Reactor Controller/Port/Logic Adapter as plain blocks without function.)*
- [x] Gas-Burning Generator (native, M04; `mekanism:electrolytic_core` is craftable).
- [ ] Chemicals/fluids deuterium, tritium, fusion fuel, bioethanol with buckets and recipes (Rotary, Separator, Neutron Activator): blocker, the chemical table only knows `mekanism`, Horizon only vanilla fluids.
- [ ] Fusion Reactor Frame is not craftable until `#c:pellets/polonium` exists (M07). Modules (Geothermal, Solar Recharging Unit): module system missing.

### Mekanism: Tools
- [x] Paxels of wood, stone, copper, iron, gold, diamond, netherite (axe action; shovel action missing, see `docs/TOOLS.md`).
- [x] Tool/armor sets of bronze, lapis, osmium, steel, refined glowstone, refined obsidian (incl. spear, shield, horse and nautilus armor).
- [x] Durability, mining, enchanting, repair and all recipes.
- [x] Check the `mekanismtools` namespace and assets. *(Wave 1: shield textures, trim palette and all upstream lang keys added; nothing `missing` any more in parity.)*
- [ ] Paxel as a shovel (needs a native `use_on` hook), shield texture, durability tooltip, mob equipment: blockers in `docs/TOOLS.md`.

### Mekanism: Additions
- [x] Plastic variants: Slick, Glow, Reinforced, Transparent, roads, stairs, slabs, fences, gates in all 18 colors (271 blocks, `content/mekanismadditions`, see `docs/ADDITIONS.md`).
- [x] Glow Panels and Obsidian TNT (Obsidian TNT explodes like vanilla TNT, see below).
- [ ] Plastic base recipe and Glow Panel recipe: need `mekanism:hdpe_sheet` (M06); Painting Machine recipes (`mekanism:painting`, recoloring) are imported since M06 group 6, recoloring at the crafting table since Wave 1 (untested in game); plastic and Glow Panels wait for `hdpe_sheet`.
- [ ] Obsidian TNT with radius 12 and fuse time: blocker, its own entity/explosion needs native code. Plastic Road without speed boost (`stepOn`), no CTM/emissive textures, no lava fluid logging.
- [ ] Balloons, Walkie-Talkies, baby mobs and spawn eggs: only researched, not built. Horizon content knows `entity_type` (vanilla template + attributes), but no own AI, models, biome spawns, entity interaction or network. Needs native code, see `docs/ADDITIONS.md`.
- [x] `mekanismadditions` namespace, models (pre-colored per color) and assets.

## M13 - Parity, integration, performance and releases [ongoing]

- [x] Maintain a machine-readable list `original ID -> Horizon ID -> status`. *(`python scripts/parity.py --upstream <Mekanism checkout>` writes `docs/parity.json` (items, blocks, recipes, tags, models, textures, lang keys, loot; status done/partial/blocked/missing) and `docs/PARITY.md` (counts per mod/category, largest gaps). Regenerate and commit after each port step; `blocked` comes from the TODO blockers in `BLOCKERS` in the script.)*
- [x] Automate the original recipe/tag/asset comparison and find regressions. *(`python scripts/parity.py --check` evaluates the checked-in `docs/parity.json` against `content/` without an upstream checkout and exits with 1 if an entry marked `done` earlier disappears; runs as the CI step "Parity regression check".)*
- [ ] Test matrix for every machine: recipes, energy, fluids, chemicals, GUI, side settings.
- [ ] Test matrix for every block entity: break, chunk unload, restart, new version, multiplayer.
- [ ] Performance: 1, 100, 1,000 active machines; energy and transport networks.
- [ ] Play a full survival playthrough from osmium to the endgame.
- [ ] Test AE2-Horizon and other tech mod interoperability with clear APIs.
- [ ] Document savegame migrations, ID stability and upgrade paths.
- [ ] Test GUI, shader compatibility, German/English translation and accessibility.
- [x] Create README, installation guide (Windows, content+native), CHANGELOG, Known Issues. *(10.11.2026: `docs/INSTALL.md`, `CHANGELOG.md` (PR #1 to #16), `docs/KNOWN_ISSUES.md`, `CONTRIBUTING.md`; the README links them. 18.11.2026: repository documentation translated to English.)*
- [ ] Check license, copyright and asset attribution of the original files before releases. *(Check 10.11.2026 in `docs/LICENSES.md`: all MIT, `LICENSE-MEKANISM` present per mod; open: attribution in README/mod description, owners `SozialHilfe`/`VitaminStack`, third-party contributions, Horizon license, comparison before release.)*
- [ ] Document CI builds, release artifacts, hashes and test beta.
- [ ] Compare upstream 26.3 changes regularly and update this list.

## Definition of Done (for every single feature)

- [ ] Built successfully with a documented Horizon build and a matching API version.
- [ ] Automated tests and original behavior compared traceably.
- [ ] Tested in game with real survival inputs and outputs.
- [ ] Save state correct after save, chunk reload and restart.
- [ ] No duplication, missing drops or obvious performance/multiplayer bugs.
- [ ] Code, log/screenshot, README/CHANGELOG and commit/PR present.

> A phase is only done when its acceptance conditions are met. A block that shows up in the inventory is not a working machine yet. It is also not guaranteed that Horizon already offers all needed native APIs today; API gaps are documented as their own issues.

## Sources

- [Original version `gradle.properties`](https://github.com/mekanism/Mekanism/blob/26.3/gradle.properties)
- [Original Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismBlocks.java)
- [Original Items](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismItems.java)
- [Original Block Types](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/registries/MekanismBlockTypes.java)
- [Original Energy Cube Tier](https://github.com/mekanism/Mekanism/blob/26.3/src/main/java/mekanism/common/tier/EnergyCubeTier.java)
- [Generators Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/generators/java/mekanism/generators/common/registries/GeneratorsBlocks.java)
- [Tools Items](https://github.com/mekanism/Mekanism/blob/26.3/src/tools/java/mekanism/tools/common/registries/ToolsItems.java)
- [Additions Blocks](https://github.com/mekanism/Mekanism/blob/26.3/src/additions/java/mekanism/additions/common/registries/AdditionsBlocks.java)
- [Horizon Native Mod API](https://github.com/horizon-rust/horizon/blob/main/docs/native-mods.md)
- [Horizon content mod documentation](https://github.com/horizon-rust/horizon/blob/main/docs/mods.md)
