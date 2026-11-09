# Draft upstream request: energy access for native trait mods

Not posted. Draft for the Horizon repository (pinned `fdc87ae`, API 411).

## Problem

A native trait mod can move energy only between its own block entities, by
editing their NBT through `Host::change_mod_block_entity`. It cannot push
energy into, or pull it from, blocks of other mods: the capacity, rate and
faces of their `ModEnergyBuffer` are unknown to it, and writing their NBT
would bypass their limits.

Horizon already has the logic: `native_energy`, `native_energy_insert`,
`native_energy_extract` and `native_move_energy` in
`crates/horizon-client/src/game/machines/native_energy.rs`, but they are
`pub(in crate::game)` and the generated `Host` trait exposes none of them.

## Request

Expose on `Host` (names as Horizon prefers):

- `energy(pos, face) -> Option<(stored, capacity)>`
- `energy_insert(pos, face, amount, simulate) -> i32`
- `energy_extract(pos, face, amount, simulate) -> i32`

respecting each block's `ModEnergyBuffer` limits and faces. Optional: a
per-block-entity face override, since faces are per block type today.

## What it unblocks

Mekanism's cubes and generators feeding other mods' machines, and Universal
Cables (M03).
