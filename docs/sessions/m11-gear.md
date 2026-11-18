# M11 Gear: Jetpack, Free Runners, Scuba, Electric Bow, Flamethrower

Branch `claude/m11-gear-5ad777`. Status: logic checked by unit tests, content validated and built as `.hmod`,
**native code not compiled** (the `native-build` check is blocked for this session). Everything in `worn.rs`,
the extensions in `items.rs`/`lib.rs` and `gear.rs` is untested in game.

## What was built

| Item | Values (source: Mekanism 26.3 Java under `src/main/java/mekanism/common/`) |
|---|---|
| Jetpack / Armored Jetpack | 24000 mB hydrogen (`item/gear/ItemJetpack`, `config/GearConfig`), 1 mB/tick, thrust 0.15 (`item/interfaces/IJetpackItem`), modes Regular/Hover/Vector/Disabled, Armored: armor 8, toughness 2 (`MekanismStartupConfig`) |
| Free Runners / Armored | 64000 FE, charge rate 320, 50 FE per point of damage, ratio 1.0, safe fall height 3 (`ItemFreeRunners`, `IFreeRunnerItem`, `CommonPlayerTickHandler.livingFall`), modes Normal/Safety/Disabled, step +0.5 only in Normal and not while sneaking. Armored: armor 3, toughness 2 |
| Scuba Mask / Scuba Tank | Tank 24000 mB oxygen, mask on head, tank on chest (`ItemScubaTank`, `ItemScubaMask`); each tick at least 1 mB, otherwise as much as refills the air |
| Electric Bow | 120000 FE, charge rate 600, 120 FE per shot, 1200 with fire mode (`ItemElectricBow`), enchantability 1 |
| Flamethrower | 24000 mB hydrogen, 1 mB/tick, modes Combat/Heat/Inferno, damage 10, 5 s burning, range (`ItemFlamethrower`, `entity/EntityFlame`) |

Pure rules with unit tests in `native/mekanism_native/api/src/gear.rs` (`cargo test --manifest-path logic/energy/Cargo.toml`:
139 tests green): fall formula (`absorb_fall`), step height, bow cost, scuba usage, jetpack movement/activation,
modes, flamethrower cone. Hook wiring in `worn.rs`; `items.rs` has tank filling, bars, tooltips;
`lib.rs` only additive forwarding.

Content (only via generators): `scripts/gen_resources.py` (block "worn and held powered gear"), then
`import_upstream.py`, `import_generator_models.py`, `gen_dictionary_tags.py`. Recipes, models, textures, names
came from the import. The run also brought already outdated generated files up to date (e.g. transmitter tags,
order in `mineable/pickaxe.json`).

## Usage

- **Filling**: right-click with a Jetpack/Scuba Tank/Flamethrower on a Chemical Tank with hydrogen or oxygen, 1000 mB per click (Mekanism fills via slot/Condensentrator).
- **Mode**: sneak + right-click with the item in hand (Runners, Jetpack, Flamethrower, Scuba Tank on/off, Bow fire mode).
- **Jetpack**: space = rise, Shift = descend (Hover). Runs on the client.

## Differences and gaps (nothing faked)

1. **HDPE-Reinforced Elytra**: not built, `mekanism:hdpe_sheet` is not registered (0 hits in `scripts/registry.json`).
2. **MekaSuit, Meka-Tool, Hazmat, Geiger, Dosimeter**: not built, as instructed.
3. **Jetpack thrust on the client**: `entity_set_velocity`/`entity_push` have no effect for players (the client moves itself). So `client_tick_post` reads space/Shift, estimates the speed from the positions of the last two ticks (`on_ground` ≈ "no y change") and calls `push_player`; the server books the fuel after a message (`mekanism:jet_thrust`, 100 ms interval, valid for 300 ms) and reports mode/fuel back (`mekanism:jet_state`). Hover/Vector are an approximation of `IJetpackItem.handleJetpackMotion`. On LAN/servers it may stutter. The elytra gliding branch is missing. Resetting the fall distance is approximated: fall damage is skipped if the server saw thrust in the last 0.5 s.
4. **No key binding for switching the mode of worn armor**: only via sneak + right-click with the piece in hand (Mekanism's own mode key needs `mod_key`/`key`, untested and not built). The Scuba Tank is **on by default** here (Mekanism: off), otherwise it would be unusable without a key.
5. **Armor model on the body is missing**: Mekanism's Jetpack/Scuba/Runners models are Java renderers, there is no `equipment` JSON. Items are `minecraft:stick` + `minecraft:equippable` without `asset_id`, without durability.
6. **`living_fall`, `living_breathe` have no host**. `inventory_tick` stores a snapshot (max. 1 s old) of boots/scuba per entity ID; the energy for a fall is deducted at the next tick (not in the hook). The snapshot is overwritten, never added, because the hook runs on both sides. Two falls in the same tick pay only once.
7. **Armor slots**: The hook doc says 100 to 103, `docs/ITEM_HOOKS.md` says 36 to 39. `worn.rs` accepts both.
8. **Step assist**: via `set_attribute_base` on `minecraft:step_height` (0.6 to 1.1), reset when taken off. Untested; the base value 0.6 is assumed, other mods that change it are overwritten. Mekanism's additional movement efficiency attribute is missing.
9. **Scuba**: Mekanism's protection from "magic" damage and the speed boost with a full air supply are missing. The oxygen usage of 1 mB/tick on land matches the Java logic. The mask's water-specific vision/Aqua Affinity effect is not checked.
10. **Bow**: a shot is canceled in `arrow_loose` without energy; the energy is deducted only at the tick after `entity_shoot_bow`. Fire mode: NBT `Fire` on the arrow via `merge_entity_nbt`; whether this really makes the arrow burn is untested (Mekanism's way is the Flame enchantment).
11. **Flamethrower without a flame entity**: Horizon cannot add its own entity type with a trajectory. Instead a hitscan cone (16 blocks, ±0.25 rad) every 2 ticks around the player's eyes: damage 10 (`minecraft:on_fire`, the type `mekanism:flamethrower` does not exist) plus 5 s burning via NBT `Fire`. Inferno sets fire on the first free block in front of a surface (up to 8 blocks). **Heat (melting blocks/items) is missing**, Heat acts like Combat. No flame particles along the path, only one at the barrel. Items on the ground are not destroyed.
12. **Additional (not my area)**: Gauge Dropper rate (upstream 250 instead of 16) and vein mining default (upstream off) differ from `gear.rs`. Follow-up for the Dropper's `items.rs`/`gear.rs`.
13. **Not compiled**: `worn.rs`, `items.rs`, `lib.rs`. Signatures were read against the pinned Horizon source (`fdc87ae`) and the generated `mod_api_generated*.rs`, but never seen by the compiler. Names I only assumed: `ClientTickPostEvent` and `KeyCode::{Space,ShiftLeft}` in the `mod_api` re-export, `NativeServerPlayer.{yaw,pitch,entity_id,position}`.

## In-game test list

1. A Chemical Tank with hydrogen: right-click with a Jetpack → the bar fills by 1000 mB per click. Same for the Scuba Tank (oxygen) and the Flamethrower.
2. Put on the Jetpack (right-click), hold space: it rises, fuel drops 1 mB/tick. No fall damage after thrust. Without fuel: no thrust.
3. Modes (sneak + right-click): Hover holds the height, Shift descends slowly; Vector pushes in the look direction; Disabled does not push.
4. Multiplayer/LAN: does the thrust stutter? Does the server use up the fuel?
5. Charge the Free Runners (Energy Tablet/cube), fall from 10 blocks: no damage, 350 FE gone. Without energy: normal damage. Safety: no step boost. Normal: steps of 1 block without jumping, not while sneaking. Take the boots off: step is 0.6 again.
6. Scuba Mask + Tank (oxygen) in water: the air display stays full, the tank drops. Tank off (sneak + right-click): the air drops again. Take the mask off: the air drops again within ~1 s.
7. Charge the Electric Bow, shoot: 120 FE per shot; no shot without energy. Fire mode: 1200 FE, does the arrow burn?
8. Hold the Flamethrower pressed: hydrogen drops 1 mB/tick, mobs in the cone take damage and burn. Inferno: fire on a block. It does not start without hydrogen.
9. Enchantments on the bow and mask are kept after loading/use (`edit_data` now changes in place instead of rebuilding the stack).
10. Recipes of all eight items at the crafting table; names and textures.
