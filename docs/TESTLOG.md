# Ingame test log

Horizon Nightly 2026-11-08 `fdc87ae`, native hooks API 411, Mekanism base 26.3 (`330b7b4d`).
Build: `scripts/Build-Content.ps1` (content), `scripts/Build-Native.ps1 -TrustNativeCode` (native).

## 09.11.2026

| Test | Result |
|---|---|
| Native build with trust approval, installation in the launcher | ok |
| `/give` + placing all 5 Energy Cubes | ok |
| Cube drop with netherite pickaxe (loot format `condition`) | ok |
| Osmium drops, smelting, ingots to block | ok |
| Ore and salt generation (flat format 26.3, biome tag `spawn_ores`) | ok |
| Block entity per cube (Agent Control `/data`) | ok |
| Creative Cube fills up to `i32::MAX` | ok |
| `energy: 123456` survives saving + restart | ok |
| Energy Tablet stores energy | ok |
| Tablet does not stack (`max_stack_size` 1) | pending |
| Debug source fills Basic Cube: 4,000/tick, stops at 4,000,000 (Agent Control) | ok |
| Debug sink empties cube: 4,000/tick down to 0 | ok |
| Per-side input/output through Horizon's energy path | pending (needs a real source, M04/M05) |

Build log: native build `native-fdc87ae-8abc9345573f` (`scripts/Build-Native.ps1 -TrustNativeCode`), launcher log without errors.

Screenshots: shared in chat, not stored in the repo.

## M01 (09.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-f7d32c38dabb` (with GUI patch: barrel template + block menu `mekanism:energy_cube`).

| Test | Result |
|---|---|
| Original model in the item (hotbar) | ok (block model not checked on its own in the game) |
| Front faces the player (`facing=north`/`east` depending on view direction) | ok |
| Right-click opens GUI: energy, bar, I/O limit, output, 6 sides | ok (screenshot) |
| Left-click side: Out to Off, `port_up=false`; right-click: back to Out, `port_up=true` | ok |
| Debug source fills A, A gives 4,000 FE/t to B: A 0, B 252,000 | ok |
| Breaking (netherite pickaxe): item `horizon_energy` + `mekanism_sides`; placing restores both (604,000 → 604,000, 2×) | ok |
| Cube placed anew with 100,000 FE, front to B: gives everything to B, sum 504,000 + 100,000 = 604,000 | ok |
| Chunk unload (tp 3000/3000 and back): energy and sides stay | ok |
| Restart: sides | pending (energy: N09) |
| Craft recipe in the game | pending |
| Tooltip with energy | pending (cannot be read via Agent Control) |

Once, B jumped from 252,000 to 504,000 on the first placement. Not reproducible (2 repeats without deviation). Cube A afterwards carried `components.custom_data` with 252,000, so it was broken and placed again by hand in between. The energy probably came from this item, not from duplication.

## M04 Heat and Solar Generator (09.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-c77934992415`; content mods `mekanism` and `mekanismgenerators` (the latter must be set to `mod.mekanismgenerators:true` in `horizon-mods.txt`; new mods start switched off).

| Test | Result |
|---|---|
| Both generators via `setblock`, block entity is created | ok |
| Solar (day, open sky) above Basic Cube: energy flows down into the cube (110, 535 FE) | ok |
| Heat Generator: coal in the slot becomes 80 mB lava per piece and is burned in 100 mB steps | ok |
| Heat Generator: energy from the front into a cube (234 FE) | ok |
| Lava at the sides (also flowing) raises the temperature (5 sides: 285 K to about 337 K); without lava it returns to the biome temperature | ok |
| Heat Generator GUI opens: title, fuel slot, lava gauge, temperature (285 K), FE/t, energy bar | ok (screenshot) |
| Lava bucket in the slot with a non-empty tank stays in place (does not fit completely) | ok |
| Solar Generator GUI, tooltips, night/rain, Nether, restart | ok |
| Bio Generator with Bio Fuel | ok |
| Machine and generator sounds (incl. Bio, Wind) | ok |
| Wind Generator item icon | fixed, ok |
| Advanced Solar Generator: foot | fixed, ok |
| Advanced Solar, Wind Generator (multiblock, rotor), Enrichment Chamber, Crusher | ok |

## P0 refactor: machine framework, recipes, energy (09.11.2026)

Build: `scripts/Build-Content.ps1`, `scripts/Build-Native.ps1 -TrustNativeCode`.

| Test | Result |
|---|---|
| Metallurgic Infuser, Enrichment Chamber, Crusher after the refactor (one tick, one recipe cache) | ok |
| Energy Cube to machines, generators to cube and machine | ok |
| Breaking Wind/Advanced Solar: bounding blocks disappear at once (`set_block` instead of `edit_blocks`) | ok |
| Breaking a bounding block in creative mode drops nothing | ok |
| GUI via every bounding block (mirrored menu `mekanism:bounding_block`) | ok |

## M06 Tanks (09.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-482f2e77aeb3` (branch `feat/m06-chemistry-foundation` after rebase onto P0), content from the same state, creative world `TankTest`.

| Test | Result |
|---|---|
| Fluid/Chemical Tank: block entity, tick (`mekanism_mode`) | ok |
| Debug Chemical Source fills Basic Chemical Tank with hydrogen | ok |
| Fluid Tank does not accept hydrogen | ok |
| Chemical Tank GUI: title, contents, transfer, mode button switches | ok |
| Dumping Excess holds 85% (54,400 of 64,000 mB) | ok |
| Dumping empties 160 mB/tick | ok |
| Breaking (survival) gives an item with contents; placing restores them | ok |
| Chemical Tank turns toward the placer | ok |
| Creative Chemical Tank stays full (`i32::MAX`) | ok |
| Fluid Tank "Auto-eject down": 5,000 mB into the tank below, valve `active=true` | ok |
| Water bucket fills Fluid Tank (2 × 1,000 mB, empty bucket returned); empty bucket takes 1,000 mB; lava bucket rejected by water tank (retest `native-fdc87ae-c2fbcfc22770`, bucket via `right_click_block`) | ok |
| GUI layout: name and amount in their own lines, display ends above "Inventory" (retest) | ok |
| Floor under tanks visible (`occludes: false`, retest) | ok |
| Fluid in Fluid Tank visible: water and lava depending on fill level, fill level follows changes, empty tank shows nothing (`native-fdc87ae-2f1bbd36b98c`) | ok |
| Other fluids: brine as water box in its own color, fill level correct (`native-fdc87ae-1ac2734b93a0`) | ok |
| Fluid with shader pack (Bliss) | Error: displayed wrongly; correct without shader. Caused by Horizon's shader path for `draw_fluid_box`, not by the mod |

## M12 Tools (10.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-352025dc49dc`, content `mekanismtools` 0.1.0 next to `mekanism` and `mekanismgenerators`, creative world `TankTest` (set to survival).

| Test | Result |
|---|---|
| Content loads (91 items, 107 recipes, tags), no errors in the log | ok |
| Attack damage and speed (`attribute`): Bronze Pickaxe 4, Refined Obsidian Sword 12, Bronze Axe 10, Lapis Hoe 1/2.0, Steel Shovel 5.5, Osmium Spear 5, Diamond Paxel 10 | ok (matches Mekanism formula) |
| Refined Obsidian set on: armor 30 (cap), toughness 20, knockback resistance 0.8 | ok |
| Armor texture on the player model (inventory screenshot), item icons | ok |
| Durability: Bronze Pickaxe breaks after 375, Refined Obsidian Pickaxe after 4096 | ok |
| Mining tier: Bronze mines diamond ore, not obsidian; Steel mines obsidian | ok |
| Diamond Paxel strips oak log (axe) | ok |
| Diamond Paxel turns grass into a path (shovel) | Error/Blocker: axe action only |
| Recipe: 3 bronze ingots + 2 sticks give Bronze Pickaxe (tag `#c:ingots/bronze`) | ok |
| `recipe give` for paxel, armor, shield, ingot recipes | ok |
| Shields in the inventory: vanilla shield model instead of own texture | known deviation |
| Shield blocking, spear attack, horse/nautilus armor on the animal, repair at the anvil, enchanting table, restart, multiplayer | pending |

## M05 Osmium Compressor, Combiner, Precision Sawmill (09.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-352025dc49dc`, content from branch `feat/m05-sawmill-combiner-compressor`, creative world `TankTest`, energy from Creative Energy Cubes.

| Test | Result |
|---|---|
| Compressor: osmium ingot in the lower slot becomes 200 mB osmium; Refined Obsidian dust to ingot, 1 mB/tick (progress 157, osmium 43) | ok |
| Compressor: second ingot is loaded when there is room; empty tank stops it (`active=false`) | ok |
| Compressor: osmium block (1,800 mB) is not converted into the 210 mB tank | ok |
| Compressor GUI: title, slots, osmium bar, progress, energy (screenshot) | ok |
| Combiner: 8 raw iron + deepslate cobblestone to deepslate iron ore, + cobblestone to iron ore | ok, after fix (missing `c:` tags) |
| Combiner: other item in the output slot pauses it, after emptying it continues | ok |
| Sawmill: 6 oak logs to 36 planks + 1 sawdust (chance 0.25) | ok, after fix (`mekanism:sawdust` registered) |
| Sawmill: bed pauses while sawdust occupies the second slot; afterwards planks + 3 wool | ok |
| Enrichment Chamber: iron ore to 2 iron dust (new tag `c:ores/iron`); Infuser: coal + iron ingot to Enriched Iron | ok |
| Restart: contents, osmium tank and progress (Combiner 182) stay | ok |
| Sounds | not tested (Agent Control does not hear) |

## M06 Electrolytic Separator, Rotary Condensentrator (10.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-6132d14d5002`, content from branch `feat/m06-separator-condensentrator`, creative world `TankTest`, energy from Creative Energy Cubes (front toward the machine), tank contents via `/data merge`.

| Test | Result |
|---|---|
| Separator: 5,000 mB water to hydrogen (left) and oxygen (right) in two Chemical Tanks, ratio 2 : 1, 2 mB water per tick | ok |
| Separator: brine 100 mB to 10 sodium + 10 chlorine (10 runs) | ok |
| Separator: left tank at 2,399 mB hydrogen, water unchanged, machine paused | ok |
| Separator GUI: title, input gauge with water, two output gauges, energy (screenshot) | ok |
| Rotary Chemical -> Fluid: 500 mB steam to water, ejected into adjacent Fluid Tank (1 mB/tick) | ok |
| Rotary mode button in the GUI switches (`mekanism_rotary_mode` 0 to 1), label changes, Fluid -> Chemical: 300 mB water to steam in the adjacent Chemical Tank | ok |
| Mode stays after restart (1) | ok |
| First GUI version: gauges covered the title, button text stuck out of the button | fixed (retest ok except a few pixels of button width, widened afterwards, not tested again) |
| Sounds, tooltips in the game, crafting recipes (Separator needs `electrolytic_core`, Rotary `mek_data` with tanks), chunk unload | not tested |

## M06 Purification Chamber, Chemical Injection Chamber (10.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-52fd95314c56`, content from branch `feat/m06-purification-injection`, creative world `TankTest`, energy from Creative Energy Cubes (front toward the machine).

| Test | Result |
|---|---|
| Purification: flint in the lower slot becomes 10 mB oxygen; 2 raw iron to 4 iron clumps (`clump_iron`), 1 mB/tick | ok |
| Purification: tank empty, progress holds (100), continues after flint | ok |
| Injection: salt becomes 2 mB hydrogen chloride, gunpowder to sulfur dust (`gunpowder_to_sulfur`), standstill with empty tank, then continues | ok |
| Purification GUI: title, slots, bar, energy (screenshot) | ok |
| Block entity data, tank type (`mekanism_infuse_type`) and amount | ok |
| `#mekanism:shards/*` recipes (tag not shipped) load without errors in the log | ok |
| Oxygen oxidation of copper, steam recipes, sounds, restart | not tested (steam needs a machine that produces it) |


## M12 Additions (10.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-52fd95314c56` (only content needed), content `mekanismadditions` (`scripts/Build-Content.ps1`, `cargo horizon-mod check` ok), creative world `TankTest`, test area at Y 150.

| Test | Result |
|---|---|
| Mod loads, 271 blocks via `setblock` (Plastic, Slick, Glow, Reinforced, Road, Transparent, Stairs, Slab on top, Fence, Gate, Glow Panel, Obsidian TNT) | ok, no log warnings (screenshot) |
| Pre-colored textures: colors correct (aqua, red, yellow, white, green, blue, purple, orange, lime, pink, cyan) | ok |
| Transparent blocks and slabs translucent | ok |
| Fences connect to neighboring fence, gate opens as a model | ok (visual) |
| Light: Glow block 10, Glow Panel 15 (`horizon-ctl light`) | ok |
| Glow Panel `facing=north/up`, waterloggable; falls off without a support block and drops item | ok |
| Survival breaking without tool: double slab drops 2, Glow block 1 (loot format `modifier` from upstream) | ok |
| Obsidian TNT ignited with redstone block: explodes like vanilla TNT (crater radius ~2 in smooth stone) | ok, but radius 4 instead of 12 (Blocker, see `docs/ADDITIONS.md`) |
| `/recipe give` stairs recipe (`plastic/stairs/red`) | ok (recipe loaded) |
| Craft recipe in the crafting grid | pending |
| Slick recipe in the Enrichment Chamber (Plastic -> Slick, recipe lies in namespace `mekanismadditions`) | pending |
| Slick friction (0.98) on the block | pending (movement not measurable via Agent Control) |
| Block items in inventory/hotbar (model, coloring) | pending (only `/give` tested) |
| Restart, chunk unload | pending |

## M06 Chemical Oxidizer, Chemical Infuser (10.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-51be04f8dfaa`, content from branch `feat/m06-oxidizer-chem-infuser`, creative world `TankTest`, energy from Creative Energy Cubes. Items via `item replace block ... container.0` (the tick does not read the lowercase NBT `items`).

| Test | Result |
|---|---|
| Oxidizer: 3 sulfur dust to 300 mB sulfur dioxide (100 mB per dust, 100 ticks), progress runs, energy stays 60 | ok |
| Oxidizer ejects the output into an adjacent Chemical Tank (300 mB `sulfur_dioxide`) | ok |
| Oxidizer next to Infuser: Infuser takes sulfur dioxide into the left tank (`accepts` by recipes), 500 to 1,100 mB | ok |
| Infuser: 100 mB oxygen + sulfur dioxide give sulfur trioxide (1 O + 2 SO2 to 2 SO3, 63 runs), tank order does not matter | ok |
| Infuser: output tank at 9,999 mB, inputs unchanged, machine paused | ok |
| GUIs: three gauges (Infuser) or slot, bar, gauge (Oxidizer), energy (screenshots) | ok |
| Restart: tank contents (sulfur dioxide, oxygen, sulfur trioxide) stay | ok |
| First build: deadlock in the server thread, because the eject (`fluid_insert`) called the neighbor's `accepts` hook under the recipe lock; eject now runs after the lock | fixed |
| Sounds, tooltips, crafting recipes (`mek_data` with Dynamic Tank) | not tested |

## M12 Generators: components (10.11.2026, via Agent Control `horizon-ctl`)

Build: content `mekanismgenerators` 0.2.0 with `scripts/Build-Content.ps1` (`cargo horizon-mod check` and `validate_content.py` ok), native `native-fdc87ae-6132d14d5002`, creative world `TankTest`. The first start ran with an outdated package in `horizon-mods` (shared folder, overwritten by a parallel session): new blocks unknown; after rebuild and restart ok.

| Test | Result |
|---|---|
| Content loads with 4 mods, no content warnings for `mekanismgenerators` (only warning: `mekanism:salt`, known) | ok |
| `setblock` of all 11 new blocks, state is reported back | ok |
| `give` Turbine Blade and Hohlraum | ok |
| `recipe give` `turbine/blade`, `hohlraum`, `reactor/frame` (recipe with missing tag `#c:pellets/polonium`) | ok, recipes exist, load without errors |
| Models, textures, item icons visible | not tested (screenshots showed only terrain, world underwater/night) |
| Crafting at the crafting table, Hohlraum in the Metallurgic Infuser, break drop | pending |

## M11 Equipment: Disassembler, Canteen, Gauge Dropper, Dictionary (11.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-96a82b9ef986` (branch `feat/m11-utility-items`), content `mekanism` from the same state, creative world `TankTest` (set to survival for mining and damage). Tanks with contents via `give ...[minecraft:custom_data={mekanism_fluid:...,mekanism_amount:...}]` and placing, item states with `item replace entity @s weapon.mainhand`.

| Test | Result |
|---|---|
| All four items load (content, names, textures) | ok (first start failed on `minecraft:unbreakable`, then stock template) |
| Dictionary: right-click on stone lists block and item tags | ok |
| Dictionary: right-click in the air reads the offhand (iron ingot: `#c:ingots/iron` etc.) | ok, but only after an offhand change (no host access to the offhand, `on_equip` cache) |
| Canteen: right-click on tank with paste draws 1,000 mB into the item (tank 10,000 → 9,000) | ok |
| Canteen: drinking (32 ticks) at 17 hunger gives 3 points, costs 150 mB; at 16 gives 4 points, costs 200 mB | ok |
| Canteen: hunger drops again afterwards (saturation 0), vanilla bread stays up | open: same cause not proven, Canteen gives no saturation |
| Gauge Dropper: right-click on tank draws 16 mB water; other fluid is rejected (paste tank, dropper with water) | ok |
| Gauge Dropper: sneak plus right-click on tank gives back 16 mB (item 100 → 84, tank +16); sneak plus right-click in the air empties | ok (only after fix: the click also triggered `use_item` and emptied the item) |
| Disassembler: energy in the item (`horizon_energy`), mining costs 10 FE | ok |
| Disassembler: sneak plus right-click switches Normal / Vein, action bar message | ok |
| Disassembler Vein: one iron ore breaks 5 connected ores (incl. diagonal), 50 FE | ok (only after tag `c:ores` for blocks; unknown before) |
| Disassembler without energy (5 FE): ore stays in place | ok |
| Disassembler weapon: hit on pig kills in one hit, costs 2,000 FE | ok |
| `set_inventory_slot` in use hooks, `set_component` on `custom_data` | Error: Horizon refuses both; workaround documented in [ITEM_HOOKS.md](ITEM_HOOKS.md) |
| Not tested | Item bars, tooltips, multiplayer/LAN, vein limit 128, Fortune/Silk Touch, offhand at login |

## M08 Bins, Personal Chest, Personal Barrel (11.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-a5d971619c14` (before that `...55c418ea8e62`, `...e85ab11785b6`), content with `scripts/Build-Content.ps1`, creative world `TankTest`, platform at Y 98.

| Test | Result |
|---|---|
| Hopper above Basic Bin fills it, window slot holds up to one stack, rest in `mekanism_bin_count` | ok |
| Right-click on the front inserts the hand (64 Kobblestone); right-click on side/back does nothing | ok |
| Left-click on the front (survival) gives 1 item, amount drops | ok |
| Breaking (survival, iron pickaxe): item keeps contents (`mekanism_bin_item/count/lock` in `custom_data`, 126), input slot drops as an item; placing restores it (64 in the window + 62) | ok |
| Eject: sneak right-click on the side switches `active=true`; hopper below fills a chest (39 iron in 4 s) | ok |
| Sneak + empty hand on the front locks the type (action bar "Bin locked: Cobblestone"); lock stays with empty bin | ok |
| Creative Bin: takes diamond as type, amount `i32::MAX`, consumes nothing from the hand | ok |
| Bin without lock forgets the type when hopper/eject empties it; second type in the input slot blocked at first (error, fixed: `accepts` also checks the window) | fixed |
| Item on the front (Cobblestone) and number "10" visible; pale at night | ok |
| Restart: contents, lock and type of the bin stay | ok |
| Personal Chest/Barrel: placing sets `mekanism_owner` (UUID) and name; chest turns toward the placer | ok |
| Personal Chest opens 9x6 menu "Personal Chest"; shift-click inserts items; breaking gives items and chest | ok |
| Foreign owner (NBT set to a made-up UUID): menu opens anyway, because the test player is an operator (intended) | Rejection not testable (singleplayer only) |
| Hopper at Personal Chest | not tested |
| Crafting recipes (`#c:chests/wooden` and `#c:barrels/wooden` added), tooltip with contents, sounds | not tested |
| Chargepad | not ported (see KNOWN_ISSUES) |
## M04 Gas-Burning Generator, M06 Electric Pump, Fluidic Plenisher (11.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-f49dcb5728e7`, content from branch `feat/m04-gas-generator-pump` (`scripts/Build-Content.ps1`), creative world `TankTest`, energy from Creative Energy Cubes in front of the machine.

| Test | Result |
|---|---|
| Gas-Burning Generator with 1,000 mB hydrogen: tank empties (to 288 mB after a few ticks), Energy Cube in front of the front charges (1,436 FE) | ok |
| Pump above water source: 7,000 mB in 5 s, `mekanism_pump_type` 1, source stays (default rule `water_source_conversion`) | ok |
| Pump above lava: nodes grow onto the neighboring sources, lava is consumed, 5,000 mB go up into a Fluid Tank | ok |
| Plenisher (water, 10,000 mB) in 3 x 3 x 2 pit: places sources, path in the NBT, water flow fills the rest; with refill `finished` 1, lists cleared | ok |
| Finished Plenisher re-places removed lava below it (1,000 mB deducted per block) | ok |
| Plenisher GUI: title, gauge in lava color, status "Finished", fluid name, amount, energy bar (screenshot) | ok |
| Pump GUI, tooltips, sound, crafting recipes, restart (tank/path stay), chunk unload | not tested |
| First content state was overwritten by a parallel build from another session in the shared `horizon-mods` folder (blocks "unknown"); rebuilt | fixed |
## M06 Dissolution Chamber, Washer, Crystallizer and ore chain (11.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-1c7ad4927bd3`, content from branch `feat/m06-ore-processing`, creative world `TankTest`, energy from Creative Energy Cubes (front toward the machine), tank contents via `/data merge`, tick rate via `/tick rate` (up to 1,000/s) for the long runs. The machines stand next to each other and eject their output tanks into the neighbor (Dissolution, Washer, Crystallizer).

| Test | Result |
|---|---|
| Dissolution: 1 iron ore + 100 mB sulfuric acid (1 mB per tick, 100 ticks) gives 1,000 mB dirty iron slurry, ejected into the Washer (acid 1,000 to 900) | ok |
| Washer: slurry + water (5 mB per 1 mB slurry, one run per tick) gives clean slurry, ejected into the Crystallizer (water 6,000 to 1,000) | ok |
| Crystallizer: 200 mB clean slurry per iron crystal, 200 ticks, 5 crystals from one ore | ok |
| Chain continued: 5 crystals + hydrogen chloride (Injection Chamber) to 5 shards, + oxygen (Purification Chamber) to 5 clumps, Crusher to 5 dirty dusts, Enrichment Chamber to 5 dusts, furnace to **5 iron ingots from one iron ore** (vanilla furnace with the imported `from_dust_smelting`) | ok |
| Crystallizer output full (64 gold crystals): machine pauses, slurry is still filled in (1,000 to 1,063 mB) | ok |
| Dissolution without acid: progress holds (81), continues after refill | ok |
| GUIs: title, gauges (water blue, slurry yellow, acid olive), item slot, progress bar, energy (screenshots) | ok |
| Sounds, tooltips, crafting recipes, world restart, chunk unload of the new machines | not tested |

## M06 Nutritional Liquifier, Pigment Extractor, Pigment Mixer, Painting Machine: test plan (results: "M06 acceptance" below)

State: implemented and checked by unit test (`cargo test --manifest-path logic/energy/Cargo.toml`: values, slots, tank capacities, mixing until output full, menu data 16 bit, tooltips) and `scripts/Test-Content.ps1`. The native build (`scripts/Build-Native.ps1`) and all ingame tests are pending, because `horizon-client` does not build locally without the launcher. TODO items stay open until this plan is worked through.

Setup: creative world, energy from Creative Energy Cubes in front of the front, tank contents via `/data merge`, tick rate via `/tick rate`. Pigment IDs are tank IDs `mekanism:chemical/<pigment>`, paste is `mekanism:nutritional_paste`.

| Test | Expectation |
|---|---|
| Liquifier: cooked beef (8) in the slot | 100 ticks, 8 FE/t, +400 mB paste in the tank, item consumed; bread (5) gives 250 mB |
| Liquifier: tank with 9,700 mB, next steak (400 mB) | Machine pauses, nothing consumed, continues after removal |
| Liquifier: tank next to it (Fluid Tank below/above) | Paste is ejected (up to 1,024 mB/t), Chemical Tank does not accept it |
| Liquifier: item without a food recipe (stone) | stays in the slot, no progress |
| Extractor: red dye | 100 ticks, +256 mB red pigment; rose/poppy 768 mB (`pigment_extracting`); other pigment with full/occupied tank waits |
| Extractor: 78 red dyes | Tank 19,968 / 20,000 mB, 79th dye waits (no partial amount) |
| Mixer: 1,000 mB red + 1,000 mB yellow into both inputs (both orders) | per tick 1 + 1 gives 2 mB orange, after 1,000 ticks 2,000 mB orange, inputs empty, output is ejected into neighbors |
| Mixer: output full (2,000 mB) | pauses, nothing consumed; without energy it stops |
| Mixer: input with mismatched pigment (red + red) | does not run |
| Painting: 256 mB red in the tank, white wool | 200 ticks, 5 FE/t, red wool, 256 mB deducted at the end; output slot occupied: pauses |
| Painting: `dye_base` + 256 mB red | Red dye; recipes for carpet, glass, concrete etc. like wool (without the already red variant) |
| Painting: fill in 15,000 mB pigment (Creative Tank next to it) | Tank 15,000 / 15,000 mB, excess stays outside |
| All four: world restart with half progress | Progress, tank contents, energy stay (NBT `fluids`, `mekanism_progress`); GUI shows fill level immediately |
| All four: chunk unload and reload, breaking with contents | no loss of the item in the slot; when breaking with a filled tank, observe and note whether the contents stay on the item (Mekanism: yes, not ported here, probably lost) |
| GUI: title, gauges (paste pink, pigments in their color), progress bar, tooltips with amount/capacity, sounds | Check screenshots, gauge positions are estimated from Mekanism's GUI |
| Crafting recipes (Liquifier, Extractor, Painting Machine); Mixer recipe missing until `hdpe_rod` | Recipe book shows three, `dye_base` from wood dust + clay |
| Eject throughput: output tank with 20,000 mB pigment next to Chemical Tank | at most 1,024 mB/t |

Open, not testable without the game: whether `#c:dyes/*` tags cover all colors for the extraction recipes in Horizon (the recipes name the items directly, only recipes with unknown items are missing in the game).

## M06 Mechanical Pipes, Pressurized Tubes, Thermodynamic Conductors (test plan; results: "M06 acceptance" below)

State: unit tests only (`cargo test --manifest-path logic/energy/Cargo.toml`: tier values, `push_plan`, heat calculation), `cargo check` of the native crate and `scripts/Test-Content.ps1`. None of this ran ingame; after the build (`Build-Native.ps1`, `Build-Content.ps1`) check in a creative world:

| Test | Expectation / Result |
|---|---|
| All 12 blocks and items appear, names, recipes (Basic Pipe: steel, empty bucket, steel; tier upgrade `PPP/PAP/PPP`), selection box, breaking gives the block | open |
| Basic Pipe chain (3 blocks): Creative Fluid Tank with water (auto-eject down, pipe below) to empty Fluid Tank; water flows at about 250 mB/t until both are equally full | open |
| Arms appear to neighboring pipe and tank, disappear when the neighbor is broken; pipe does not connect to tube or conductor | open |
| Pipe accepts no chemicals, tube no fluids | open |
| Tube chain with hydrogen from `debug_chemical_source` via Chemical Tank | open |
| Tier rates (Advanced 1,000, Elite 8,000, Ultimate 32,000 mB/t) against tank fill level over time (`/tick rate`) | open |
| Separator, Oxidizer or Pump output into an adjacent pipe/tube that forwards into a tank | open |
| Flow balances out, does not run back into the full source tank, no deadlock when filling into neighbors | open |
| Restart: pipe contents stay; chunk unload; performance with 200+ pipes (connection check per tick) | open |
| Heat Generator with lava next to conductor: `mekanism_heat` of the conductor rises, generator temperature falls; Basic vs. Ultimate: Ultimate loses less to the air | open |
| Conductor chain balances temperatures (`/data get block ...`), cold conductor gives nothing back | open |
| Rendering: center cube and arms with upstream textures, item model (GUI, hand), multipart blockstate loads | open |
| Blocks without container/menu: right-click opens nothing, and `block_entity_fluids` works even without `block_entity_container` | open |

## M06 Pressurized Reaction Chamber, Isotopic Centrifuge, Solar Neutron Activator (13.11.2026, branch `feat/m06-advanced-chem`)

**Results: "M06 acceptance" below.** Before that, checked only by `cargo test` (machine values, `react`/`transmute`, Activator rate and cycle, tooltips), `scripts/Test-Content.ps1` and a type check of the native crate. Test plan (creative world, Creative Energy Cube at the machine, tank contents via `/data merge`, `/tick rate` for long runs):

| Test | Expectation |
|---|---|
| Place all three blocks, open GUI | Title, gauges, slots; Centrifuge/Activator show the 2-block height (see limits in `CHEMISTRY.md`) |
| Reaction: coal + 100 mB water + 100 mB oxygen | after 100 ticks 1 sulfur dust and 100 mB hydrogen, 2 FE/t |
| Reaction: substrate recipe `water_ethene` | 400 ticks, 2 + 2 FE/t, 8 substrate; progress in % in the bar |
| Reaction: full chemical output or full item slot | Machine pauses, nothing is consumed |
| Reaction: too little fluid/chemical | Progress is reset, energy not consumed |
| Centrifuge: Nuclear Waste (10 mB) | 1 mB plutonium per tick, 8 FE/t; tank eject into neighbors |
| Centrifuge without energy / full output | stops |
| Activator at noon, open sky, plains | 64 mB/t lithium to tritium; tooltip "Production 64.00" |
| Activator at night, in rain, under a roof | Rate falls to 0, bar empty; tooltip |
| Activator in desert/jungle | Rate by biome factor; below 1 mB/t one run every n ticks |
| Sounds Reaction and Centrifuge, restart, chunk unload | Loop, tanks and progress stay |

## M06 acceptance: Nutrition/pigments, pipes/tubes/conductors, advanced chemistry (13.11.2026, via Agent Control `horizon-ctl`)

Build: native `native-fdc87ae-e93aa89e0e57` (first run for PR 25 and 26 with `native-fdc87ae-b0b22102ae30`), content and native from the local test branch `test/m06-acceptance` (`main` + PR 25, 26 and 27, state `214186a`), creative world `M06Accept`, energy from Creative Energy Cubes next to the machine, tank contents via `/data merge`.

| Test | Result |
|---|---|
| Liquifier: 2 cooked beef, 100 ticks each, 400 mB paste (800 mB); stone stays in the slot without progress | ok |
| Liquifier: tank 9,700 mB, next steak waits; Fluid Tank below gets the paste, Chemical Tank does not | ok |
| Extractor: red dye gives 256 mB red; tank 19,968 mB, further dye waits and stays in the slot | ok |
| Extractor: 2 red dyes disappeared once without output (after `/data merge` during operation) | not reproduced (3 repeats); watch |
| Mixer: 1,000 + 1,000 mB red and yellow to 2,000 mB orange, 2 mB per tick; full output pauses; red + red does not run | ok |
| Painting: white wool + 256 mB red to red wool, 256 mB deducted; GUI with gauge, progress, energy (screenshot) | ok |
| Basic Pipe: chain Creative Tank, 2 pipes, Fluid Tank; arms to pipe and tank | ok |
| Basic Pipe: throughput into a large tank about 125 instead of 250 mB/t | was a deviation, **fixed**: endpoints (tanks, machines, not above) take the full rate regardless of fill fraction; rechecked with build `native-fdc87ae-cd0f0bc7d6a2`: Creative Tank, 3 pipes, Fluid Tank gives 5,000 mB in 20 ticks = 250 mB/t |
| Tube with hydrogen into a Chemical Tank; Fluid Tank next to it stays empty | ok; arm to the Fluid Tank was a deviation, **fixed** (arm only to tanks of its own family; rechecked, tube next to Fluid Tanks without arm) |
| Heat Generator with lava to Conductor 1 to Conductor 2: about 323 K after 200 ticks | ok |
| Restart: items, tanks, progress, energy, heat of Liquifier, Extractor, Mixer, Painting, Pipe, Tank, Conductor stay | ok |
| Reaction Chamber: coal + water + oxygen to sulfur dust + hydrogen (100 mB each), 2 FE/t, 100 ticks | ok |
| Reaction Chamber: recipe `water_ethene` to 8 substrate + 10 mB oxygen (400 ticks, 200 mB water, 100 mB ethene) | ok |
| Reaction Chamber: full chemical output (9,950 mB) pauses, nothing consumed | ok |
| Reaction Chamber: GUI with gauges, slots, progress, energy (screenshot) | ok |
| Centrifuge: 10 mB Nuclear Waste to 1 mB plutonium; uranium hexafluoride to Fissile Fuel (1 to 1); tank contents stay | ok |
| Activator: noon 66 mB/t lithium to tritium (1,320 mB in 20 ticks); night and roof 0 | ok |
| Activator: GUI (screenshot) | ok |
| Restart: tank contents of Reaction Chamber, Centrifuge, Activator (lithium + tritium stays 7,300 mB) | ok |
| Pipes/tubes/conductors tiers Advanced to Ultimate, pipe from machine output, 200+ pipes, chunk unload, sounds, tooltips, crafting recipes; Reaction Chamber without energy, Activator in other biomes, 2-block height | not tested |


## Wave 1 acceptance: upgrades, transport, equipment, multiblocks, teleport, special machines (18.11.2026, via Agent Control `horizon-ctl`)

Build: content and native from the local test branch `test/wave1-acceptance` (`main` + the 8 wave 1 branches). First run with native `native-fdc87ae-29d7630f6d54`. Rechecks after the fixes with `native-fdc87ae-c2013bd9866e` and last `native-fdc87ae-5cf7278cbc49` (state after `dc21d6b` plus chunk unload fix). Creative world `W1Accept`. Checklist: `docs/sessions/WAVE1-CHECKLIST.md`.

| Test | Result |
|---|---|
| Speed Upgrade x8 in the Enrichment Chamber: 1 operation per 20 ticks, buffer 9,500 FE | ok, after a bug was **fixed**: cubes filled only the base buffer, the machine never held the energy of one tick (`badaff7`) |
| Configurator turns machine (east, south) and Energy Cube (up) | ok, after a bug was **fixed** (`badaff7`) |
| Redstone label in the GUI no longer overlaps | ok, **fixed** (`badaff7`) |
| Energy Tablet charges the machine | ok |
| Restart: upgrades and energy stay | ok |
| Logistical Transporter: click on an arm gives "west: Pull", items flow | ok, after a bug was **fixed**: the hitbox was empty (shapes in pixels instead of block units), and the side selection also hit sides without an arm (`badaff7`, `0cd5d54`) |
| Glow Panels: hitbox | **fixed** (shapes in block units, `0cd5d54`); not rechecked ingame |
| Free Runners: step height 1.1, after taking off 0.6 | ok, after a bug was **fixed** (step height stayed after taking off, `badaff7`) |
| Scuba Mask + Tank: air stays 300 underwater, oxygen drops; without mask the air drops | ok |
| Flamethrower: consumes hydrogen, kills a pig | ok |
| Enchantment stays after energy use | ok |
| Electric Bow | **Blocker**: Horizon only draws and shoots `minecraft:bow` (hard-wired), and the host API has no call to spawn entities or projectiles |
| Dynamic Tank: forms, capacity 432,000 mB; dissolving and rebuilding keeps the contents; chunk unload keeps the structure | ok |
| Dynamic Valve in Both mode takes inflow | ok, after a bug was **fixed**: the buffer was filled completely, inflow never arrived (`34e1eeb`) |
| Induction Matrix: invalid without provider; 3×4×3 with cell and provider forms (8 G capacity, 256 k transfer) | ok |
| Induction Port input from a Creative Cube charges the matrix; output charges a Basic Cube | ok, after a bug was **fixed**: ports did not accept Mekanism energy, cables got no arm (`34e1eeb`) |
| Teleporter A to B: teleport, energy −119 FE; too little energy: no teleport | ok |
| Portable Teleporter teleports (−116 FE); channel selection counts up to `freq::CHANNELS`, then switches public/private | ok (channel selection is a replacement for the GUI, designed that way) |
| Quantum Entangloporter: items from E1 arrive at E2, a hopper at E2 empties the pool | ok, after a bug was **fixed**: E1 kept the whole window, and a member without a window showed an empty stack (count 0). Now the members take turns at the window every 10 ticks (`dc21d6b`) |
| Entangloporter restart: 10 diamonds became 20 (before that, also a loss in one run) | **fixed**: the registry is now written only when the world is saved (`world_saved`), together with the chunks. Rechecked: 10 diamonds over 3 restarts, plus 1 restart in the middle of emptying by hopper, exactly 10 in the chest. 5,000 FE stay 5,000 FE over 1 restart (two separate readings, not counted while emptied) (`dc21d6b`) |
| Entangloporter, one member unloaded: E3 at 600 64 600 on frequency 1, 10 diamonds in, back to E1/E2: the hopper at E2 fetches exactly 10; E3 is empty after reloading | ok, after a bug was **fixed**: the unloaded member kept the window in its slot. Now it gives the window back to the pool at `chunk_unload` |
| Teleporter after the registry changes (build `5cf7278cbc49`): teleport to the nearest active teleporter of the frequency, −117 FE | ok |
| Restart: frequencies (Teleporter, Entangloporter) stay bound | ok |
| Oredictionificator: GUI, filter `dusts/bronze` to `dust_bronze`; bronze dust is converted, cobblestone stays in the input | ok |
| Oredictionificator restart: filters stay | ok |
| Digital Miner: placing gives 17 bounding blocks + main block (3×3×2) | ok |
| Digital Miner without filter, not inverted: mines nothing | ok (as in the original) |
| Digital Miner inverted, radius 2, Y 60 to 63: mines, consumes energy, model active, ejects into the chest 2 behind and 1 above the miner | ok |
| Digital Miner Silk Touch: stone instead of cobblestone, consumption ×12 | ok |
| Digital Miner restart: radius stays 2 (only this value checked) | ok |
| Formulaic Assemblicator: block and item exist (without function, as planned) | ok |
| Entangloporter: fluid and chemical sharing, autosave during play (only quitting checked), members in different dimensions | not tested |

Open (not fixed):

- Electric Bow: Blocker in Horizon (see above).
- Teleporter and Entangloporter GUI: the channel label overlaps the ">" button, the text "Not bound…" runs over, the label "Item" overlaps.
- Oredictionificator GUI: "Left: next Right: previous" overlaps "Inventory".
- Basic Solar Generator outputs only downward.
- Universal Cable shows no arm to generators.
- Warning on load: placed feature `mekanism:salt`.
- The Factory capacity formula differs from the original.
- Teleporter capacity is 1,000 FE. The code comment says this matches the original; not rechecked.
- Frequencies are now written only when the world is saved. After a crash, the changes since the last autosave are missing. Chunks that were unloaded and written between two autosaves are newer than the registry: a crash can then also duplicate Entangloporter contents, not only lose them.
- Entangloporter members take turns by the `game_time` of their dimension. If the times of two dimensions drift apart, a member may get its turn less often (not tested).
