# M08: Frequencies, Teleporter, Portable Teleporter, Quantum Entangloporter

Branch `claude/m08-teleport-frequency-b0a9ab`. All values come from Mekanism 26.3 (Java), taken over 1:1 unless named under "Gaps".

## Scope

| Module (`native/mekanism_native/api/src/`) | Content | Horizon-free / tests |
|---|---|---|
| `frequency.rs` | Frequency registry (name + owner + public/private), shared buffers, `exchange()`, panel buttons, save/load | yes, 14 tests |
| `teleporter.rs` | Cost, frame detection, target position, status, gate (cooldown) | yes, tests in `logic/energy` |
| `teleporters.rs` | Host: Teleporter block, frequency panel, Portable Teleporter | no |
| `entangloporter.rs` | Host: Quantum Entangloporter | no |

`lib.rs`: only minimal hooks (kind variants `Teleporter`/`Entangloporter`, match arms, early branches in `item_energy_buffer`, `use_item`, `tooltip`, `inventory_tick`, `item_bar`, new hook `player_tick`). `items.rs` unchanged.

Content: `mekanism:teleporter`, `teleporter_frame`, `quantum_entangloporter`, `portable_teleporter`, `teleportation_core` (generator `gen_resources.py`, block "M08 frequencies"). Models, textures, lang, loot and recipes come via the generator + importer.

## Ported values (source: Java)

- Teleporter (`TileEntityTeleporter`, `UsageConfig`, `StorageConfig`): base cost 100 FE, 2 FE/block of distance, dimension change +1000 FE, delay 5 ticks, energy storage 1000 FE.
- Coordinate scale: Nether 8, otherwise 1. Rider factor (1 + riders). Saturating arithmetic.
- Status order: NoFrequency, NoFrame, NoDestination, NotEnoughEnergy, Ready.
- Frame: for each direction, first not rotated, then rotated. Target position: above it without a frame, `below(2)` with frame DOWN, otherwise `relative(dir)`. Yaw: N 180, S 0, W 90, E 270.
- Entangloporter buffer (Ultimate tier, `InventoryFrequency`): energy 256 000 000, fluid 256 000, chemical 8 192 000, one item slot.
- Portable Teleporter (`GearConfig`): energy buffer 1 000 000, transfer 5 000.

## Design decisions

- State is in module-wide `static Mutex`, no new struct fields.
- Registry file `<world_dir>/mekanism_horizon/frequencies.txt` (tab-separated). Written at once on create/delete, otherwise at most every 5 s when buffers changed. Loaded lazily per `world_dir`.
- Active Teleporters are not saved; each ticking Teleporter reports to the registry (ACTIVE_MS = 3000, wall clock).
- Only players are teleported (`player_tick`, API 31). Horizon has no entity-in-AABB helper.
- Frequency GUI: 16 numbered channels (public/private), buttons Back, Next, public toggle, Set (create if missing + bind), Delete (owner or op).
- Entangloporter "window": on its tick each member moves its local contents into the pool and refills a window (energy 256 000, fluid 64 000, chemical 512 000, item = stack limit). Conservation is tested. Reason: Horizon energy is `i32`, so the pool does not fit.
- Members: tank 0 = fluid, tank 1 = chemical in the `fluids` list. Stacks with components stay in the slot. `broken()` gives the window back to the pool.
- Portable Teleporter: using it in the air teleports to the nearest active Teleporter of the frequency. Sneak+use switches channel/mode. Energy is deducted through a PENDING queue in `inventory_tick` (SNBT rebuild of `custom_data`, duplicated from `items.rs` because it is private there).

## Gaps compared to upstream

- No text input for frequency names (only 16 channels).
- No Security Desk, no trusted list, no colors.
- Only players, no mobs/items.
- Teleporter capacity 1000 FE (upstream TODO "26.3: Update these config values"): dimension changes and distances above about 450 blocks are therefore impossible. Ported as is.
- Entangloporter: energy only windowed, no heat, no side configuration, no energy slot.
- Portable Teleporter without GUI (`open_menu` is LAN only).
- No chunk loading.
- Registry flush has up to 5 s delay; no `world_saved`/`server_stopping` (signatures not verified). A crash loses up to ~5 s of buffer changes.
- Recipe `mekanism:mek_data` was simplified to `crafting_shaped` by the importer; the energy transfer from the tablet for the Portable Teleporter is dropped.
- Teleporter light level 3 not ported.
- Recipe ingredients checked (5 recipes): all items/tags resolve. The tag `c:ender_pearls` was missing in the content and was added as `content/mekanism/data/c/tags/item/ender_pearls.json` (`minecraft:ender_pearl`). Tag `mekanism:alloys/atomic` points to `#c:alloys/ultimate` (present).
- The generator run also added `#mekanism:transmitters` (M06, tag exists) to `blocks_motion_no_leaves.json` and `mineable/pickaxe.json` and re-sorted the entries in `pickaxe.json` (no entries lost). `docs/parity.json` was not regenerated (`parity.py --check`: 781 newly done).

## Not compile-verified

The native crate was not built (lead's rule: no `native-build`). Derived from source/generator rules, never compiled:
the `player_tick` signature and `PlayerTickEvent` import, `host.world_dir()`, `host.teleport_player`, `event.tank` as `usize`, `host.max_stack_size`, the new match arms in `lib.rs`, all code in `teleporters.rs` and `entangloporter.rs`.
Checked instead: 159 unit tests (`cargo test --manifest-path logic/energy/Cargo.toml`), `scripts/validate_content.py`, `scripts/parity.py --check`, `cargo horizon-mod build` + `check` for the content package.

## In-game test list

1. The game starts, content loads, five new items/blocks in the creative inventory.
2. Place a Teleporter + frame, the GUI opens, choose a channel, Set, status changes (NoFrequency → NoFrame/NoDestination).
3. Two Teleporters with the same frequency, each supplied with energy, frames around them: a player in the field is teleported after 5 ticks, energy drops by the cost.
4. Too little energy → status NotEnoughEnergy, no teleport. A dimension change is expected to fail (1000 FE).
5. Private/public: a second player does not see the first player's private channels. Delete only as owner/op.
6. Reload the world: frequencies are there, Entangloporter contents after restart (≤5 s loss).
7. Two Entangloporters with the same frequency: energy/fluid/chemical/item is shared, breaking returns the contents, nothing duplicated.
8. Portable Teleporter: sneak+use switches channel, use teleports to the nearest active Teleporter, energy drops.
