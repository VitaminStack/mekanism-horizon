# Session m08-cables-transport: Universal Cables and Logistical Transporter

Branch `claude/m08-cables-transport-d803b6`. None of this was tested in game
(Build-Content, Build-Native and starting the game were blocked for this session).

## What is ported

### Task A: Universal Cables (TODO M03)

The four blocks `mekanism:{basic,advanced,elite,ultimate}_universal_cable`.

- Values from `CableTier.java`: capacity 8,000 / 128,000 / 1,024,000 / 8,192,000. The rate per tick equals the capacity.
- Each cable is a block entity with its own Horizon energy buffer (`block_entity_energy`, NBT `energy`).
  It accepts any amount on each side up to its capacity and gives nothing out through the capability.
- In its tick the cable distributes by itself (`transmitter::distribute`): first to endpoints (cubes, machines, pumps),
  then balancing (equalizing the fill level) with neighboring cables. It hands out through `give`, and it subtracts only what
  the neighbor actually took.
- Arms are six block states (`down`..`east`) as with pipes.
- Energy conservation: `transmitter::tests::a_line_of_mixed_cables_conserves_energy_every_tick` and
  `a_full_sink_backs_the_line_up_without_loss` check the sum on every tick.

**Question: can a native mod push energy into foreign block entities?** No, not with Horizon `fdc87ae`.
`native_energy*` are `pub(in crate::game)`, and `Host` has no energy call
(see `docs/upstream/horizon-energy-host.md`). Result: cables reach only Mekanism blocks
that `energy::acceptor` knows. Foreign mods (RF/FE blocks) stay unreachable.

Differences from Mekanism:

- No shared network with one buffer; each cable has its own buffer (balancing per tick, not instant).
- Generators (`generators.rs`, not my file) output only to the front and to the bottom. A cable must touch there.
- No side configuration and no Configurator for cables.

### Task B: Logistical, Restrictive and Diversion Transporter (TODO M08)

Six blocks: four tiers `*_logistical_transporter`, `mekanism:restrictive_transporter`, `mekanism:diversion_transporter`.

- Values from `TransporterTier.java`: pull 1 / 16 / 32 / 64 items, speed 5 / 10 / 20 / 50. Each tick a stack gains
  `speed` progress and is passed on at 100 (dwell time `ceil(100/speed)` ticks per block).
- Path costs as in `getCost`: Ultimate speed divided by tier speed (doubled, integer: 20/10/5/2).
  Restrictive costs 1,000 (here 2,000, same ratio), Diversion is Basic.
- Diversion: per side Disabled / High / Low (`DiversionControl`). High needs a redstone signal, Low needs none.
  Right-click with an empty hand cycles the clicked side.
- Structure: block entity with 8 slots. Slot 0 is the input (hopper, `insert_item`), slots 1..7 hold stacks
  in transit. Progress, entry side and origin are in `mekanism_transit` (IntArray) and are saved with it.
- A stack is passed on only when the next holder has taken it: into the next transporter entity via
  `change_mod_block_entity`, into an inventory via `insert_item` (the rest stays). No loss on save, load,
  chunk unload. On breaking, the container drops its slots itself (`docs/native-mods.md`, section Container).
- Pathfinding: Dijkstra over the transporter blocks (limit 512 blocks), cheapest path to an inventory. Equal
  costs go to the lower side number. The inventory the stack was pulled from is never a target. An inventory that
  rejected the stack is skipped for that stack. A stack goes back only if nothing else leads anywhere.
  With no target it waits and tries again later.
- Tests (`logic/energy`, `transporter.rs`): conservation of the item sum on every tick, dwell time, cheapest path,
  Restrictive is avoided, tie, a full target block falls back to the next one, Diversion/side modes, save format.

Differences from Mekanism:

- **Pull rule:** Mekanism pulls with the Configurator. This repo does not have the Configurator. Replacement: sneak +
  right-click with an empty hand cycles the connection mode of the clicked side (Normal, Push, Pull, None);
  message in the action bar. Pull fetches `pull` items every 10 ticks.
- No `container_slots` in the host (gameplay-internal only). So inventories cannot be listed: whether a target will accept a stack
  cannot be checked beforehand (`insert_item` is a real insert), and Pull blindly searches slots 0..53 with `extract_item`.
- No network with path prediction, no colors, no idle paths, no stack merging.
  Two transporters that are both full and want to swap stacks can block each other (nothing is lost).
- The default connection is Normal (no pull), as in Mekanism.

### Not done

- **Logistical Sorter** (`mekanism:logistical_sorter`): not started. Still missing.
- No native compile check: `horizon-launcher native-build` was blocked. Everything in `transmitters.rs`,
  `transporters.rs` and the `lib.rs` changes is **not compiled**. It follows the existing patterns (`bins.rs`, `pumps.rs`) and the
  signatures from the generated `Host`; errors in the first real build are probably only small type errors.

## Checks

| Check | Result |
|---|---|
| `cargo test --manifest-path logic/energy/Cargo.toml` | 149 tests ok |
| `python scripts/validate_content.py` | OK, 5763 JSON files |
| `python scripts/parity.py --check` | 0 regressions |
| `cargo horizon-mod build` / `check` (Task A, content) | ok |
| Native compilation | not run (blocked) |

## In-game checklist

Cables:
1. Spawn an energy cube, place cables: arms appear toward cubes, machines, pumps and cables.
2. Solar generator at the front or bottom of a cable, cable to a cube: the cube charges, the cables empty.
3. Basic cable filled to 8,000, Advanced with higher throughput.
4. Break a cable and place it again: no error.

Transporters:
1. Place a transporter between chest A and chest B. Sneak + right-click (empty hand) on the side facing A until "Pull".
   Items move to B (Basic: 1 item per 10 ticks).
2. Tier speed: Ultimate is clearly faster than Basic.
3. Full target block: items go into the second chest; no items disappear.
4. A Restrictive Transporter is used only if there is no other path.
5. Diversion: right-click (empty hand) sets a side to High; without redstone it blocks, with a signal it opens.
6. Break a transporter with items: items fall out. Save and load the world: items keep moving.
7. A hopper below/above a transporter pushes items in.

## Open items

- Logistical Sorter and filters.
- Configurator (pull/push by tool).
- Shared cable networks with one buffer; energy into foreign mods (host call missing).
- Stack merging and deadlock avoidance between full transporters.
- Make up the native compilation.
