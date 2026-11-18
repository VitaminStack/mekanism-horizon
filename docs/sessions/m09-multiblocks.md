# M09 – Multiblocks (session `m09-multiblocks`)

Branch `claude/m09-multiblocks-640bbc`, base `main` @ 6a6da54. One commit per step, prefix `feat(m09):`.

## What was ported

1. General multiblock system (pure logic + host).
2. Dynamic Tank, Dynamic Valve, Structural Glass.
3. Induction Matrix: Casing, Port, four cell tiers, four provider tiers.
4. Thermal Evaporation Plant: **not ported** (only validator rules, see gaps).

Boiler and Turbine stay untouched (as instructed).

### Files

| File | Content |
|---|---|
| `native/mekanism_native/api/src/multiblock.rs` | Pure logic: flood fill, bounding box, rules per position, `merge_stored` (10 tests) |
| `native/mekanism_native/api/src/dynamic_tank.rs` | Pure logic: capacity, valve exchange, edit modes, menu data (10 tests) |
| `native/mekanism_native/api/src/induction.rs` | Pure logic: cell/provider sums, port flow, menu data (10 tests) |
| `native/mekanism_native/api/src/multiblocks.rs` | Host: formation, linking, revalidation, forwarding of the hooks |
| `native/mekanism_native/api/src/dynamic_tanks.rs` | Host: tank contents in NBT, valve ↔ master, screen |
| `native/mekanism_native/api/src/inductions.rs` | Host: matrix energy in NBT, port ↔ master, screen |
| `native/mekanism_native/api/src/lib.rs` | Additive only: `Kind::Multiblock`, match arms, `place_block`/`destroy_block` |
| `logic/energy/src/lib.rs` | Three new `#[path]` modules for the tests |
| `scripts/gen_resources.py` | Additive: block "multiblocks (M09)"; generated content (blockstates, items, models, textures, lang, recipes, loot, menus, tags, `registry.json`) |

## Values and their source (upstream Java)

Upstream checkout: `mekanism-upstream-26.3`.

| Value | Source |
|---|---|
| Dynamic Tank capacity = volume (full bounding box) × 16 000 000 mB (`16_000L * BUCKET_VOLUME`) | `GeneralConfig.dynamicTankFluidPerTank`, `TankMultiblockData` |
| Ejection rate 1 024 mB/tick | `TankMultiblockData` |
| Edit modes Both / Fill / Empty (default Both) | `ContainerEditMode` |
| Valve buffer 32 000 mB (Horizon tank, exchanged with the master each tick) | Own adaptation to Horizon's fluid API |
| Cell capacities 8e9, 64e9, 512e9, 4e12 | `InductionCellTier` |
| Provider output 256 000, 2 048 000, 16 384 000, 131 072 000 | `InductionProviderTier` |
| Capacity = saturating sum of the cells, transfer = sum of the providers | `MatrixMultiblockData`, `MatrixEnergyContainer` |
| At least 1 cell and 1 provider | `MatrixValidator.postcheck` |
| Size 3..=18 per axis | `MultiblockManager` / validator constants |
| Hardness 5/9 (Casing, Valve, Port), 3.5/16 (cells, providers, glass) | `BlockBasicMultiblock`, `BlockTile` |
| Frame/face/interior rules (edge only frame, face frame/valve/controller, inside air or interior) | `FormationProtocol`, `CuboidStructureValidator` |

## Design

- A structure is the connected shell (flood fill) with a bounding box. Results: `Formed`, `Invalid(reason)`, `Unloaded`. `Unloaded` never dissolves anything.
- The master holds the shared state (NBT key `mekanism_mb_*`). When a master is destroyed by hand, the contents go to a neighboring member; with explosions/commands they are lost.
- There are no `block_changed`/`chunk_load` hooks. Members check by polling in `block_entity_tick`, staggered by position hash: unformed ones every 200 ticks (and at once on `place_block`), formed members their link every 10 ticks, the master its structure every 20 ticks.
- Induction Port: Horizon energy buffer (max. `PORT_BUFFER` = 131 072 000), share per tick = transfer / port count. Input empties the buffer into the matrix, Output fills it and gives to Mekanism neighbors via `give()`.

## Gaps and blockers

- **Thermal Evaporation Plant is missing.** Validator rules (open top, controller count) and `Type::Evaporation` stubs are prepared, host logic is missing.
- **Mekanism sources do not charge ports via `give()`.** `energy::acceptor` in `energy.rs` is an ID table (not part of this session). Horizon cables use the buffer. Note for the lead session: add ports to the table.
- Structural Glass is (as upstream) not part of a tank or matrix.
- Contents (fluid/energy) live in the master. Cells have no block entity. If the master is destroyed without a neighboring member, the contents are gone.
- No energy/item slots, no redstone output, no `shouldCache` behavior for cell energy.
- The edit mode acts only through the buffer exchange; `accepts_fluid` is not enforced per mode.
- Polling instead of events: formation/dissolution can take up to about 10 s. A large incomplete shell makes every unformed member flood again every 200 ticks.
- The property `formed` has no model difference.
- A valve buffer with a different fluid than the master gets stuck.

## Not compile-verified

The `native-build` check is forbidden (lead's decision). The native code follows existing compiled patterns but was **not built**. Risk points:

- `host.level(&dim)` and `LevelRef::is_loaded` / `game_time`.
- `&*e.id` (deref of the entity ID).
- Borrow pattern in `form()` with `own: Option<(Pos, &mut Compound)>`.
- Types of the `change_mod_block_entity` closures.
- `Tag::Long`.
- Literals `ModEnergyBuffer` and `ModFluidTanks`.
- `ScreenDrawEvent.block`.
- `event.entity.data` as `Vec<i32>`.

## Checked

| Check | Result |
|---|---|
| `cargo test --manifest-path logic/energy/Cargo.toml` | 162 passed, 0 failed |
| `python scripts/validate_content.py` | OK, 5737 JSON files |
| `python scripts/parity.py --check` | 13029 entries, 0 regressions, 873 newly done |
| `cargo horizon-mod build` + `check` (`content/mekanism`) | OK, `mekanism 0.2.0`, 2 045 988 bytes |

## In-game test list

All only after a manual build by the lead session.

Dynamic Tank:
1. A 3×3×3 cube of Dynamic Tank, empty inside: after at most about 10 s the blocks form; the screen shows "Capacity 16 000 B × volume" (27 → 432 000 B).
2. Replace one edge with a Dynamic Valve: the valve is not accepted (an edge needs a frame). Put a valve in a face: valid.
3. Connect a pipe to the valve, feed in water: the fill level in the screen rises.
4. Set the edit mode to Empty: the valve pushes (1 024 mB/tick) into adjacent containers.
5. Remove one block: the structure dissolves, the contents stay in the master cache; place it again: the contents return.
6. Break the master by hand: the contents go to a neighboring member.
7. Fill the interior with a block: the structure becomes invalid.
8. Size 19 and 2: invalid (limits 3..=18).
9. Unload/load the chunk: the structure does not dissolve.

Induction Matrix:
1. 3×3×3: casing shell, 1 port in a face, 1 cell + 1 provider inside, the rest air: it forms.
2. Without a provider or without a cell: invalid (`postcheck`).
3. Capacity in the screen: Basic 8 G, Advanced 64 G, Elite 512 G, Ultimate 4 T; transfer accordingly 256 k … 131.072 M.
4. Port in Input mode: cable/generator fills the matrix. Switch the port to Output with the button: the matrix charges an adjacent consumer.
5. Several ports: the total transfer is split across all ports.
6. A Mekanism energy source directly next to the port: does **not** charge (known gap).
7. Break a cell: capacity drops, energy is clamped to the capacity.

Structural Glass:
1. Place it and check that it forms no tank/matrix structure.
2. Shell with glass instead of casing: stays invalid (matches upstream).

## Open items

- Implement the Evaporation Plant (step 4).
- Extend `energy::acceptor` with the Induction Port (lead, `energy.rs`).
- Check native compilation of the new modules in an allowed environment.
- Optional: model variant for `formed`, event-based revalidation, redstone output.
