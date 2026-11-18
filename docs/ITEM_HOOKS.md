# Item Hooks and Gear (M11)

Check of Horizon's item hooks against the pin `fdc87ae` (native hooks API 411) and the feasibility of each M11 item. Sources: `native-hooks.json` and `docs/native-mods.md` of the Horizon checkout, and the generated `Hooks` trait (`mod_api_generated.rs`). Implementation: `native/mekanism_native/api/src/items.rs` (hooks) and `gear.rs` (rules and numbers, without Horizon types, tests in `logic/energy`).

> Correction to [TOOLS.md](TOOLS.md): it says Horizon has no `use_on` hook. That is true for the content path, not for the native mod: `use_item_on` (API 13) listens for a right-click with an item on a block that the block let through. The Paxel shovel action is therefore feasible natively (not built yet).

## Available hooks

"Server" = runs on the server thread (singleplayer, LAN, dedicated). All hooks only listen and change the event they receive, unless noted otherwise. Blocking gaps are listed below.

| Need | Hook / host call | API | Note |
|---|---|---|---|
| Right-click on a block with an item | `right_click_block` (before block and item, cancelable), `use_item_on` (only if the block lets it through) | 21, 13 | Machines and tanks open a menu, so use `right_click_block` for items on tanks |
| Right-click in the air | `use_item` (cancelable = claimed) | 13 | Sneak state through `Host::sneaking` |
| Left-click on a block, hit on a mob | `left_click_block`, `attack_entity`, `hurt_enemy` | 21, 39 | |
| Change damage | `living_incoming_damage_v2` (weapon as snapshot, `amount` changeable, host) | 384 | only the player's melee/sweep/spear on mobs; v1/v3 without weapon |
| Item per tick in the inventory | `inventory_tick(slot, selected)`, `carried_stack(slot)` | 39 | Armor slots 36-39, `carried_stack` only inside the hook; main hand = the tick with `selected` |
| Change a stack outside of that | `Host::held_stack(uuid)` reads; `set_inventory_slot` is **not usable** in the use hooks (player state swapped, tested) | 39+ | Changes are queued in `items.rs` and written in the next `inventory_tick` through `carried_stack` |
| Equipping | `on_equip` (player), `living_equipment_change` (only mobs and armor stands) | 39, 148 | There is no host query for the offhand (`player_stacks` only knows the 36 main slots). The Dictionary remembers it from `on_equip`, so it only sees an item after the next swap |
| Item state | read `minecraft:custom_data` through `ItemStack::custom_data`; write only by rebuilding the stack with `Host::item_from_snbt` | 65 | `Host::set_component` only works for own IDs registered through `component_types`: for `minecraft:custom_data` it returns `UnknownType` (tested) |
| Default components | `modify_default_components` | 79 | |
| Item energy | `item_energy_buffer` (capacity, input/output) | 307 | Horizon stores `horizon_energy` in `custom_data` and charges items in cubes/tablet slots itself |
| Item fluid tank | `item_fluid_tank` | - | not used here: chemicals are fluids with the `mekanism:chemical/` prefix and are stored in `custom_data` (`mekanism_fluid`, `mekanism_amount`, as with the tank items) |
| Use over time | `use_duration`, `use_animation`, `can_eat`, `living_entity_use_item_start/finish/tick`, `on_use_tick`, `release_using`, `living_entity_use_item_stop` | 90, 131 | An item without a vanilla use and with `use_duration > 0` is held (drinking, charging) |
| Eating, hunger | `Host::player_food`, `set_player_status`, `food_level_changed` | 169 | |
| Bow | `arrow_loose` (cancelable), `entity_shoot_bow`, `living_get_projectile` | - | |
| Player movement | `Host::entity_velocity/entity_set_velocity`, `push_player`, `set_flight`, `set_flying_speed`, `player_tick`, `player_toggle_flight`, `player_velocity`, `entity_toggle_glide` | 31, 153 | The server sets the velocity; there is no client prediction hook for it |
| Breathing | `living_breathe(can_breathe, consume, refill)` | 144 | Players and mobs with generic air logic |
| Keys | `input_key`, `key`, `mod_key` (client), `net_send_to_server`/`net_from_client` | 40 | A custom key needs a client hook plus a message to the server |
| Item display | `item_bar` (bar), `item_decorations` (drawing), `tooltip`, `tooltip_component(_draw)` | 60, 89, 12, 68 | |
| Own item model per state | `item_model_property` | 385 | E.g. Geiger Counter levels, Jetpack mode |
| Register items/blocks | Content mod (`data/<ns>/horizon/item/*.json`: vanilla `template` plus `components`) or `items` hook | - | Only vanilla behavior through components: tool, armor (`equippable`), elytra (`glider`), food |
| Break a block | `destroy_block` (cancelable), `Host::destroy_block(pos, tool, collect)` | 14 | Vein mining calls it for neighbors |
| Chat/notice | `Host::tell`, `action_bar`, `feedback` | - | |

Not available or not usable:

- No hook that **lists** the tags of a block or item: only a membership test per tag (`block_name_in_tag`, `server_item_in_tag`). So the Dictionary checks a generated list (`scripts/gen_dictionary_tags.py`, all tags of the content mods plus a few vanilla tags).
- No dynamic attributes per stack state: attack value, mining speed and armor values come from the static component. So the Disassembler's weapon bonus runs through `living_incoming_damage_v2`.
- No radiation or module system in the port. Item NBT alone does not carry that.
- No hook for player fall damage except `living_incoming_damage` (v1: damage type, changeable amount, without host). That is enough for Free Runners, but it has no armor stack.
- No client prediction hook for movement: Jetpack thrust sets the velocity on the server. In LAN and on dedicated servers this can stutter (untested).
- GUI clicks with an item on the mouse (Gauge Dropper on a GUI gauge) are only reachable through `menu_clicked`/`screen_click` of the respective menu. The Mekanism gauges exist only in the own tank screens.

## Feasibility per item

Status: **implemented** (in this branch, tested in-game, see [TESTLOG.md](TESTLOG.md)), **feasible** (hooks exist, not built), **blocked** (ingredient or system missing in the port).

| Item | Status | Hooks / reason |
|---|---|---|
| Atomic Disassembler | implemented (partly) | Energy buffer 1,000,000 FE, mining speed 20 for pickaxe/axe/shovel/hoe (component), without durability (stock template; `minecraft:unbreakable` is not a known component type in Horizon content), weapon bonus 16 for 2,000 FE (`living_incoming_damage_v2`), mode by sneak+right-click (`use_item`), vein mode (`destroy_block` plus `Host::destroy_block`, tag `mekanism:atomic_disassembler_ore` = `#c:ores` + logs, max. 128 blocks, 10 FE per block), no block is mined without energy. Missing: modes Slow/Fast/Off (mining speed is static; whether `set_component` may change the `minecraft:tool` component of a stack is unchecked), energy from the tablet slot, config values |
| Electric Bow | feasible | `item_energy_buffer`, `arrow_loose` (consume arrow/deduct energy, fire mode), vanilla `bow` template; the recipe only needs ingredients that exist |
| Flamethrower | feasible | Gas in the item (hydrogen, `custom_data`), `use_duration` plus `on_use_tick` for continuous fire, fire through `Host::set_block`/particles; ingredients (Chemical Tank, `#c:tools/igniter`) exist. Modes and mob burning unchecked |
| HDPE-Reinforced Elytra | blocked (ingredient) | Content: `minecraft:glider`/`equippable` components; the recipe needs `mekanism:hdpe_sheet` (M06) |
| Jetpack, Armored Jetpack | feasible | Chest armor (template plus `equippable`), hydrogen in the item, thrust through `entity_set_velocity` from `inventory_tick`/`player_tick`, mode and key through `input_key` plus `net_send_to_server`; armor model/texture exist as upstream assets. Armor values not checked against Java |
| Free Runners, Armored Free Runners | feasible | Energy buffer, fall damage through `living_incoming_damage` (reduce amount), boots template; armor values unverified |
| Scuba Mask and Tank | feasible | `living_breathe` (`can_breathe`, `consume`) with oxygen from the tank item in the armor slot; ingredients exist. Water breathing and night vision effect (`add_effect`) possible |
| Hazmat Suit | blocked (system) | The armor itself would be content, but radiation protection needs a radiation system, which the port does not have; armor values not verified |
| MekaSuit (all parts), Meka-Tool, Modification Station | blocked (system) | The module system (module items, slots, config screens, energy use per module) does not exist; ingredients (pellets, Teleportation Core and others) are mostly missing |
| Canteen | implemented | Capacity 64,000 mB Nutritional Paste in `custom_data`, 50 mB per hunger point, 32 ticks of drinking (`use_duration`, `use_animation` Drink, `living_entity_use_item_start/finish`), item bar and tooltip. In the original it is refilled by the Nutritional Liquifier (missing): here by right-click on a tank with paste (1,000 mB per click) |
| Gauge Dropper | implemented (partly) | 16,000 mB capacity, right-click on a tank/machine with a fluid tank takes 16 mB of a fluid or chemical, sneak plus right-click gives 16 mB back, sneak plus right-click in the air empties it. The original works on GUI gauges; that stays open |
| Geiger Counter, Dosimeter | blocked (system) | no radiation system; item model states (`item_model_property`) would be feasible |
| Dictionary | implemented | Right-click on a block: block and item tags; right-click in the air: tags of the item in the offhand. Only tags from the generated list (no list hook) |
| Crafting Formula | blocked (system) | belongs to the Formulaic Assemblicator (missing); item NBT for recipe lists would be feasible |
| Network Reader | blocked (system) | reads transmitter networks (cables, pipes; M07/M08 open) |
| Portable QIO Dashboard | blocked (system, ingredient) | needs the QIO Dashboard and frequencies (M10 open), the recipe needs polonium pellets |

## Items not built

Items without a function are deliberately not registered (Geiger Counter, Dosimeter, Crafting Formula, Network Reader, Portable QIO Dashboard, Hazmat, MekaSuit, Meka-Tool, HDPE Elytra, Jetpack, Free Runners, Scuba, Electric Bow, Flamethrower). A craftable item that does nothing would be fake. Order of the next candidates: Electric Bow, Free Runners, Scuba, Jetpack, Flamethrower.

## Values

The Mekanism numbers (Canteen capacity and rate, Dropper step size, Disassembler energy and costs, vein limit) are Mekanism's default values to the best of our knowledge. They are **not** read from the Java sources: the upstream checkout for the assets (`31bb4906`) is a sparse checkout without `src/main/java`. They are collected in `gear.rs` and must be compared against a full upstream checkout.
