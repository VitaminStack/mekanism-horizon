# M08 – Special machines (session doc)

Branch `claude/m08-special-machines-fb6601`. Upstream reference: Mekanism 26.3 (`src/main/java/mekanism/common/...`).
Mekanism is MIT-licensed; the notice is kept.

**Important:** The native build was not allowed in this session. Only the Horizon-free modules were
compiled and tested (`cargo test --manifest-path logic/energy/Cargo.toml`, 149 tests ok).
All host modules are **not compile-verified** (see below).

## Commits

| Commit | Content |
|---|---|
| `7c3da58` | Oredictionificator |
| `0bb896d` | Formulaic Assemblicator + Crafting Formula (registration only) |
| `dc3ac70` | Digital Miner |

Seismic Vibrator and Seismic Reader (task 4, optional) were **not** started.

## 1. Oredictionificator

Modules: `oredict.rs` (Horizon-free, 7 tests), `oredicts.rs` (host), `oredict_tags.rs` (generated from
`scripts/gen_oredict_tags.py`), router `special.rs`.

| Value | Source |
|---|---|
| No energy need | `TileEntityOredictionificator` |
| Slots input (56,115), output (164,115); GUI 236x230, inventory at (38,148) | `GuiOredictionificator` |
| Allowed tag prefixes `c:` + `ingots/ ores/ dusts/ nuggets/ storage_blocks/ raw_materials/` | `GeneralConfig.java:229-230` |
| Each tick min(count, free space) items are converted | `TileEntityOredictionificator.tickServer` |

Differences:
- Max. 8 filters, 3 visible; tag and output are chosen with cycle buttons (Horizon has no text input in screens).
- Input matching is hybrid: generated tag table OR `server_item_in_tag` (see blocker 1).
- Tags of other mods under `c:` cannot be filtered if they are not in the table.
- Not ported: redstone mode, side configuration, security, upgrades.

## 2. Formulaic Assemblicator + Crafting Formula

Registered only: block, item, recipes, models, lang. **No behavior** (blocker 2).

## 3. Digital Miner

Modules: `miner.rs` (Horizon-free, 10 tests), `miners.rs` (host), wiring in `special.rs`, `energy.rs`, `lib.rs`.

| Value | Source |
|---|---|
| Default: radius 10, minY 0, maxY 60 | `TileEntityDigitalMiner` |
| maxRadius 32, silkMultiplier 12, ticksPerMine 80 | `GeneralConfig` |
| Base usage 40 FE/tick | `UsageConfig` (digitalMiner) |
| Usage/tick: `ceil(base * (silk?12:1) * (1+max(0,(r-10)/22)) * (1+max(0,(maxY-minY-60)/(worldHeight-1-60))))` | `MinerEnergyContainer.updateMinerEnergyPerTick` |
| Search order: x fastest, then z, then y; start corner `(x-r, minY, z-r)` | `ThreadMinerSearch` / `TileEntityDigitalMiner` |
| Mining: by chunk in discovery order, highest index first; one block per 80 ticks | `TileEntityDigitalMiner.tickServer` |
| Ejection every 10 ticks, through the top-back cell into the container behind it | `TileEntityDigitalMiner` |
| 27 slots at (8+x*18, 92+y*18) | `GuiDigitalMiner` |
| Footprint x/z -1..1, y 0..1 (17 bounding blocks) | `BlockDigitalMiner` / `Attributes` |
| Radius 0..32; minY in [worldMin, maxY]; maxY in [minY, worldMax] | `TileEntityDigitalMiner.setRadius/setMinY/setMaxY` |
| `miner_blacklist` tag is empty upstream | Datapack |

Differences:
- Static buffer 100,000 FE (upstream dynamic, 4 x usage).
- Filter = items in 9 filter slots + one tag choice (8 block tags), no filter GUI with replace target/output.
- Tool for mining: netherite pickaxe (Silk Touch when Silk is active), upstream a fully charged Atomic Disassembler.
- Search state and overflow are not saved; after a restart it searches again.
- Settings are locked while the miner is running.
- Hoppers pull from the top and the sides.
- Not ported: redstone, side configuration, security, upgrades, XP, anchor, filter on the dropped item.
- Unbreakable blocks via a list (no hardness API).
- Right-click on a bounding block of the miner is canceled (no call to open the menu of another block).
- Blockstate: `mekanism:holiday_selector` removed (Horizon does not know the type); `active` is a boolean property and is set in `miners::tick` only on change.

## Blockers (proven from the source)

1. **No tag listing in the host.** Only membership tests (`server_item_in_tag`, `block_name_in_tag`). Hence the generated table.
2. **Vanilla crafting recipes are not reachable.** `native_recipes_of_type` returns only `recipes().custom.get(kind)`
   (`crates/horizon-client/src/game/native_hooks.rs:11288-11293`). `custom_type` (`crafting.rs:1333-1337`) keeps only types
   with `:` and without the prefix `minecraft:`. The loader (`crafting.rs:1546-1559`) routes `minecraft:crafting_*` into `crafting`,
   which is not exposed. `native_recipes` (`native_hooks.rs:11359`) is not in the host trait; `docs/native-mods.md:4999-5001`
   lists `recipes` under "not yet in the trait". So the Formulaic Assemblicator cannot resolve recipes.
3. Screens know only `host.text` and `host.fill`, no string passing; names are decoded from compile-time tables.
4. Screen hooks get no `pos`; GUI state is encoded as `i32` data (16-bit wire), 32-bit values via `cube::encode_amount`.

## Not compile-verified

Only Horizon-free logic was compiled/tested. Not compiled:
- `native/mekanism_native/api/src/oredicts.rs`
- `native/mekanism_native/api/src/miners.rs`
- `native/mekanism_native/api/src/special.rs`
- New arms in `lib.rs` (`block_entity_container`, `container_accepts`, `block_entity_energy`, `block_entity_tick`, `state_for_placement`, `screen`, `screen_draw`, `screen_click`, `menu_button`, `right_click_block`, `destroy_block`, `place_block`)
- Extension of `acceptor()` in `energy.rs`
- The generated `oredict_tags.rs` (compiled only as a Horizon-free module in `logic/energy`)

## Checks run

- `python scripts/validate_content.py`: OK (5653 JSON files)
- `cargo test --manifest-path logic/energy/Cargo.toml`: 149 passed
- `python scripts/parity.py --check`: 0 regressions
- `cargo horizon-mod build` + `check` in `content/mekanism`: ok (`mekanism 0.2.0`, API 0.2, content only)

## In-game test list

Oredictionificator:
1. Place it, the menu opens (236x230). The inventory sits below the device.
2. Create a filter, cycle tag and output, scroll with more than 3 filters.
3. Iron ingot tag: input `minecraft:iron_ingot` becomes the chosen output item; stack size stays, limited per tick by space.
4. A hopper pushes in only matching items. A disabled filter does not convert.
5. Filters survive restart/chunk reload.

Formulaic Assemblicator: block and Crafting Formula appear in the creative inventory with model and names. No behavior expected.

Digital Miner:
1. Place it: 17 bounding blocks appear, placing it against an obstacle is rejected. Breaking the main block or a bounding block removes all.
2. Open the menu (main block). Change radius/minY/maxY with buttons, limits 0..32 and world limits.
3. Feed energy by cable/cube; check usage per tick (radius 10, height 60: 40 FE/tick; Silk: 480 FE/tick; radius 32: 40*(1+22/22)=80).
4. Start: the search runs, then one block per 80 ticks, highest index first; the model switches to `active`.
5. Filter slot with an item or tag choice: only matching blocks are mined (or the inverse).
6. Silk Touch: an ore block drops itself.
7. Ejection: container behind the miner, top-back cell, every 10 ticks.
8. Full inventory: overflow blocks mining, then it continues.
9. Settings are locked while it is running.
10. Reload the world: the search starts again, settings and filters stay.

## Open items

- Native build and in-game test are pending; host code is verified only after them.
- The Formulaic Assemblicator needs a host API for crafting recipes.
- Seismic Vibrator and Seismic Reader are missing.
- Dynamic energy buffer, filter GUI with replace target, upgrades, redstone, side configuration are missing.
