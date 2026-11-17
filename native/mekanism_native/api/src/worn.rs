//! Worn and held powered gear (M11): Jetpack, Free Runners, Scuba Mask and Tank, Electric Bow
//! and Flamethrower. Rules and numbers are in `gear.rs`; this wires them to Horizon's hooks.
//! See docs/sessions/m11-gear.md for what each hook gives and where this differs from Mekanism.
//!
//! State sits in the stack's `custom_data` like the hand-held gear (`items.rs`): hydrogen or
//! oxygen as `mekanism_fluid`/`mekanism_amount`, energy as `horizon_energy`, the mode as the
//! byte `mekanism_mode`.
//!
//! Three hooks (`living_fall`, `living_breathe`) get no `Host`, so `inventory_tick` keeps a
//! short-lived snapshot of what each player wears, per entity id. A snapshot older than
//! `SNAP_LIFE` is ignored, so taking the gear off ends its effect within a second. What those
//! hooks use up is booked in `costs` and paid at the next `inventory_tick` of the slot.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use horizon_client::mod_api::{
    ArrowLooseEvent, ClientTickPostEvent, Compound, EntityShootBowEvent, EquipmentSlot, Flow, Host,
    InventoryTickEvent, ItemStack, KeyCode, LivingBreatheEvent, LivingFallEvent, NetFromClientEvent,
    NetFromServerEvent, OnUseTickEvent, Tag, Text, UseDurationEvent,
};

use super::gear::{self, FlameMode, JetMode, RunnerMode};
use super::items::{edit_data, energy_of, fluid_of, set_energy, set_fluid};
use super::key;

const ITEM_MODE: &str = "mekanism_mode";
/// How long a snapshot of worn gear counts.
const SNAP_LIFE: Duration = Duration::from_millis(1_000);
/// How long a thrust message of the client counts.
const THRUST_LIFE: Duration = Duration::from_millis(300);
/// Client to server: one byte, 1 while the jetpack pushes.
const THRUST_CHANNEL: &str = "mekanism:jet_thrust";
/// Server to client: `[mode, has_fuel]`, mode 255 when no jetpack is worn.
const STATE_CHANNEL: &str = "mekanism:jet_state";
/// The armour slots `inventory_tick` reports: 100 to 103 (feet to head), as the hook's doc says;
/// 36 to 39 as docs/ITEM_HOOKS.md has it.
const FEET: [i32; 2] = [100, 36];
const CHEST: [i32; 2] = [102, 38];
/// Eye height of a standing player.
const EYE: f64 = 1.62;
/// Use ticks the flamethrower can be held: vanilla's bow and crossbow take 72000.
const FLAME_USE_TICKS: i32 = 72_000;

#[derive(Clone, Copy)]
struct Snap {
    at: Instant,
    boots: Option<(RunnerMode, i32)>,
    scuba: bool,
}

/// What the client knows of the worn jetpack.
#[derive(Default)]
struct Client {
    mode: Option<JetMode>,
    fuel: bool,
    at: Option<Instant>,
    last: Option<[f64; 3]>,
    thrusting: bool,
    sent: Option<Instant>,
}

#[derive(Default)]
pub struct State {
    snaps: Mutex<HashMap<u32, Snap>>,
    /// Energy the boots still owe for a fall, by entity id (overwritten, never added: the hook
    /// runs on both sides).
    fall_cost: Mutex<HashMap<u32, i32>>,
    /// When a player's client last said the jetpack pushes.
    thrust: Mutex<HashMap<u128, Instant>>,
    /// When the server last saw a jetpack thrust, by entity id.
    thrust_eid: Mutex<HashMap<u32, Instant>>,
    /// The state last sent to a player's client: `[mode, has_fuel]`, and when.
    told: Mutex<HashMap<u128, ([u8; 2], Instant)>>,
    /// Players whose step height this mod raised.
    stepped: Mutex<HashSet<u128>>,
    /// Energy a bow shot owes, by player.
    bow_cost: Mutex<HashMap<u128, i32>>,
    /// mB of hydrogen a flamethrower owes, by player.
    flame_cost: Mutex<HashMap<u128, i32>>,
    client: Mutex<Client>,
}

fn mode_byte(stack: &ItemStack) -> Option<u8> {
    stack.custom_data().and_then(|d| match d.get(&key(d, ITEM_MODE)) {
        Some(Tag::Byte(b)) => Some(*b as u8),
        _ => None,
    })
}

fn set_mode(stack: &mut ItemStack, byte: u8) {
    edit_data(stack, |d| {
        d.insert(key(d, ITEM_MODE), Tag::Byte(byte as i8));
    });
}

/// Whether `item` is one of the jetpacks, boots, scuba tanks or flamethrower whose chemical is
/// stored in the stack, and which chemical it is.
pub fn chemical_of(item: &str) -> Option<&'static str> {
    match item {
        gear::JETPACK | gear::JETPACK_ARMORED | gear::FLAMETHROWER => Some(gear::HYDROGEN),
        gear::SCUBA_TANK => Some(gear::OXYGEN),
        _ => None,
    }
}

impl State {
    /// The armour slots and the held bow and flamethrower, once a tick each.
    pub fn inventory_tick(&self, host: &mut dyn Host, event: &InventoryTickEvent) {
        let Some(player) = host.acting_player() else { return };
        let eid = player.entity_id as u32;
        if FEET.contains(&event.slot) {
            self.boots(host, player.uuid, eid, event.slot);
        } else if CHEST.contains(&event.slot) {
            self.chest(host, player.uuid, eid, event.slot);
        } else if event.selected {
            self.held(host, player.uuid, event.slot);
        }
    }

    /// Mutate the stack in `slot` through a copy (`carried_stack` borrows the host).
    fn rewrite(host: &mut dyn Host, slot: i32, change: impl FnOnce(&mut ItemStack)) {
        let Some(mut stack) = host.carried_stack(slot).cloned() else { return };
        change(&mut stack);
        if let Some(target) = host.carried_stack(slot) {
            *target = stack;
        }
    }

    fn boots(&self, host: &mut dyn Host, uuid: u128, eid: u32, slot: i32) {
        let worn = host
            .carried_stack(slot)
            .cloned()
            .filter(|s| matches!(&*s.item, gear::FREE_RUNNERS | gear::FREE_RUNNERS_ARMORED));
        let Some(stack) = worn else {
            self.snaps.lock().unwrap().entry(eid).and_modify(|s| s.boots = None);
            self.fall_cost.lock().unwrap().remove(&eid);
            if self.stepped.lock().unwrap().remove(&uuid) {
                host.set_attribute_base(uuid, "minecraft:step_height", gear::BASE_STEP_HEIGHT);
            }
            return;
        };
        let owed = self.fall_cost.lock().unwrap().remove(&eid).unwrap_or(0);
        let energy = (energy_of(&stack) - owed).max(0);
        if owed > 0 {
            Self::rewrite(host, slot, |s| set_energy(s, energy));
        }
        let mode = RunnerMode::from_byte(mode_byte(&stack).unwrap_or(0));
        self.snap(eid, |s| s.boots = Some((mode, energy)));
        let sneaking = host.entity(eid).and_then(|e| host.entity_is_shift_key_down(e)).unwrap_or(false);
        let want = gear::step_height(mode, energy, sneaking);
        let base = host.attribute(uuid, "minecraft:step_height").map(|a| a.0);
        if base.is_some_and(|b| (b - want).abs() > 1e-6) {
            host.set_attribute_base(uuid, "minecraft:step_height", want);
        }
        self.stepped.lock().unwrap().insert(uuid);
    }

    fn snap(&self, eid: u32, change: impl FnOnce(&mut Snap)) {
        let mut snaps = self.snaps.lock().unwrap();
        let s = snaps.entry(eid).or_insert(Snap { at: Instant::now(), boots: None, scuba: false });
        change(s);
        s.at = Instant::now();
    }

    fn chest(&self, host: &mut dyn Host, uuid: u128, eid: u32, slot: i32) {
        let stack = host.carried_stack(slot).cloned();
        let item = stack.as_ref().map(|s| s.item.to_string()).unwrap_or_default();
        match item.as_str() {
            gear::SCUBA_TANK => {
                let stack = stack.unwrap();
                let (fluid, oxygen) = fluid_of(&stack);
                let oxygen = if fluid.as_deref() == Some(gear::OXYGEN) { oxygen } else { 0 };
                let on = mode_byte(&stack).unwrap_or(1) != 0;
                let mask = host
                    .player_equipment(uuid, EquipmentSlot::Head)
                    .is_some_and(|m| &*m.item == gear::SCUBA_MASK);
                let active = gear::scuba_active(mask, oxygen, on);
                self.snap(eid, |s| s.scuba = active);
                if active {
                    let air = host.entity(eid).and_then(|e| host.entity_air_supply(e)).unwrap_or(gear::MAX_AIR);
                    let used = gear::scuba_use(air, oxygen);
                    Self::rewrite(host, slot, |s| set_fluid(s, Some(gear::OXYGEN), oxygen - used));
                }
                self.tell(host, uuid, None);
            }
            gear::JETPACK | gear::JETPACK_ARMORED => {
                self.snap(eid, |s| s.scuba = false);
                let stack = stack.unwrap();
                let (fluid, fuel) = fluid_of(&stack);
                let fuel = if fluid.as_deref() == Some(gear::HYDROGEN) { fuel } else { 0 };
                let mode = JetMode::from_byte(mode_byte(&stack).unwrap_or(0));
                let fresh = self.thrust.lock().unwrap().get(&uuid).is_some_and(|t| t.elapsed() < THRUST_LIFE);
                if fresh && mode != JetMode::Disabled && fuel >= gear::JET_FUEL_PER_TICK {
                    self.thrust_eid.lock().unwrap().insert(eid, Instant::now());
                    // Creative flight uses no fuel, as in Mekanism.
                    if host.acting_player().is_some_and(|p| p.game_mode != 1) {
                        Self::rewrite(host, slot, |s| {
                            set_fluid(s, Some(gear::HYDROGEN), fuel - gear::JET_FUEL_PER_TICK)
                        });
                    }
                }
                self.tell(host, uuid, Some([mode.byte(), u8::from(fuel > 0)]));
            }
            _ => {
                self.snap(eid, |s| s.scuba = false);
                self.tell(host, uuid, None);
            }
        }
    }

    /// Tell a player's client what the worn jetpack is, when it changes and every second.
    fn tell(&self, host: &mut dyn Host, uuid: u128, now: Option<[u8; 2]>) {
        let now = now.unwrap_or([255, 0]);
        let mut told = self.told.lock().unwrap();
        let due = told.get(&uuid).is_none_or(|(last, at)| *last != now || at.elapsed() > Duration::from_secs(1));
        if due && !(now[0] == 255 && !told.contains_key(&uuid)) {
            told.insert(uuid, (now, Instant::now()));
            drop(told);
            let _ = host.net_send_to_player(uuid, STATE_CHANNEL, &now);
        }
    }

    /// The main-hand stack: pay for bow shots and flamethrower fuel booked since the last tick.
    fn held(&self, host: &mut dyn Host, uuid: u128, slot: i32) {
        let bow = self.bow_cost.lock().unwrap().remove(&uuid);
        let flame = self.flame_cost.lock().unwrap().remove(&uuid);
        if bow.is_none() && flame.is_none() {
            return;
        }
        Self::rewrite(host, slot, |s| match &*s.item {
            gear::ELECTRIC_BOW => {
                if let Some(cost) = bow {
                    let energy = energy_of(s) - cost;
                    set_energy(s, energy);
                }
            }
            gear::FLAMETHROWER => {
                if let Some(cost) = flame {
                    let (_, have) = fluid_of(s);
                    set_fluid(s, Some(gear::HYDROGEN), (have - cost).max(0));
                }
            }
            _ => {}
        });
    }

    fn fresh(&self, entity: u32) -> Option<Snap> {
        self.snaps.lock().unwrap().get(&entity).copied().filter(|s| s.at.elapsed() < SNAP_LIFE)
    }

    /// Free Runners take fall damage off for energy; a jetpack that pushed a moment ago ends the fall.
    pub fn living_fall(&self, event: &mut LivingFallEvent<'_>) -> Flow {
        if self.thrust_eid.lock().unwrap().get(&event.entity).is_some_and(|t| t.elapsed() < Duration::from_millis(500))
        {
            return Flow::Cancel;
        }
        let Some((mode, energy)) = self.fresh(event.entity).and_then(|s| s.boots) else { return Flow::Continue };
        let Some(fall) = gear::absorb_fall(mode, energy, event.distance, event.multiplier) else {
            return Flow::Continue;
        };
        self.fall_cost.lock().unwrap().insert(event.entity, fall.cost);
        if fall.cancel {
            return Flow::Cancel;
        }
        event.distance = fall.distance;
        Flow::Continue
    }

    /// A scuba set breathes for the player: no air used, the supply refilled.
    pub fn living_breathe(&self, event: &mut LivingBreatheEvent<'_>) {
        if self.fresh(event.entity).is_some_and(|s| s.scuba) {
            event.can_breathe = true;
            event.consume = 0;
            event.refill = gear::MAX_AIR;
        }
    }

    /// A bow that cannot pay for its shot does not release it.
    pub fn arrow_loose(&self, event: &mut ArrowLooseEvent<'_>) -> Flow {
        if &*event.stack.item != gear::ELECTRIC_BOW {
            return Flow::Continue;
        }
        let fire = mode_byte(event.stack).unwrap_or(0) != 0;
        if gear::bow_can_shoot(energy_of(event.stack), fire) { Flow::Continue } else { Flow::Cancel }
    }

    /// An arrow left the bow: book its energy, and light it in fire mode.
    pub fn entity_shoot_bow(&self, host: &mut dyn Host, event: &EntityShootBowEvent<'_>) {
        let Some(held) = host.held_stack(event.player).filter(|h| &*h.item == gear::ELECTRIC_BOW) else { return };
        let fire = mode_byte(&held).unwrap_or(0) != 0;
        self.bow_cost.lock().unwrap().insert(event.player, gear::bow_cost(fire));
        if fire {
            let mut nbt = Compound::default();
            nbt.insert(key(&nbt, "Fire"), Tag::Short(i16::MAX));
            let dimension = host.dimension().to_owned();
            host.merge_entity_nbt(&dimension, event.projectile, nbt);
        }
    }

    /// Sneaking with the item in hand changes its mode (Free Runners, Jetpack, Scuba Tank, Bow,
    /// Flamethrower). The armour pieces would otherwise equip themselves.
    pub fn use_item(&self, host: &mut dyn Host, items: &super::items::State, stack: &ItemStack) -> Flow {
        if !host.sneaking() {
            return Flow::Continue;
        }
        let (next, name): (u8, String) = match &*stack.item {
            gear::FREE_RUNNERS | gear::FREE_RUNNERS_ARMORED => {
                let m = RunnerMode::from_byte(mode_byte(stack).unwrap_or(0)).next();
                (m.byte(), format!("Free Runners: {}", m.name()))
            }
            gear::JETPACK | gear::JETPACK_ARMORED => {
                let m = JetMode::from_byte(mode_byte(stack).unwrap_or(0)).next();
                (m.byte(), format!("Jetpack: {}", m.name()))
            }
            gear::FLAMETHROWER => {
                let m = FlameMode::from_byte(mode_byte(stack).unwrap_or(0)).next();
                (m.byte(), format!("Flamethrower: {}", m.name()))
            }
            gear::SCUBA_TANK => {
                let on = mode_byte(stack).unwrap_or(1) == 0;
                (u8::from(on), format!("Scuba Tank: {}", if on { "on" } else { "off" }))
            }
            gear::ELECTRIC_BOW => {
                let on = mode_byte(stack).unwrap_or(0) == 0;
                (u8::from(on), format!("Electric Bow: fire mode {}", if on { "on" } else { "off" }))
            }
            _ => return Flow::Continue,
        };
        let Some(player) = host.acting_player() else { return Flow::Continue };
        host.action_bar(player.uuid, Text::literal(name));
        items.edit_held(host, move |_, s| set_mode(s, next));
        Flow::Cancel
    }

// --- flamethrower ---------------------------------------------------------------------------

impl State {
    /// A flamethrower is held down like a bow, and starts only with hydrogen.
    pub fn use_duration(&self, event: &mut UseDurationEvent<'_>) {
        if &*event.stack.item == gear::FLAMETHROWER {
            event.duration = FLAME_USE_TICKS;
        }
    }

    pub fn can_start(&self, stack: &ItemStack) -> bool {
        &*stack.item != gear::FLAMETHROWER || fluid_of(stack).1 >= gear::FLAME_FUEL_PER_TICK
    }

    /// Every tick of use: 1 mB of hydrogen, and every `FLAME_INTERVAL` ticks the flame's effect.
    ///
    /// Mekanism shoots an `EntityFlame` that flies 40 blocks; Horizon has no entity type to add,
    /// so the flame is a cone scanned from the player's eye (`gear::in_cone`).
    pub fn on_use_tick(&self, host: &mut dyn Host, event: &OnUseTickEvent<'_>) {
        if &*event.stack.item != gear::FLAMETHROWER || event.entity.is_some() {
            return;
        }
        let Some(player) = host.acting_player() else { return };
        let owed = self.flame_cost.lock().unwrap().get(&player.uuid).copied().unwrap_or(0);
        if fluid_of(event.stack).1 - owed < gear::FLAME_FUEL_PER_TICK {
            return;
        }
        if player.game_mode != 1 {
            *self.flame_cost.lock().unwrap().entry(player.uuid).or_insert(0) += gear::FLAME_FUEL_PER_TICK;
        }
        if event.remaining % gear::FLAME_INTERVAL != 0 {
            return;
        }
        let mode = FlameMode::from_byte(mode_byte(event.stack).unwrap_or(0));
        let from = [player.position[0], player.position[1] + EYE, player.position[2]];
        let dir = gear::look(player.yaw, player.pitch);
        host.flame_particle([from[0] + dir[0], from[1] + dir[1] - 0.2, from[2] + dir[2]], false);
        let dimension = host.dimension().to_owned();
        let own = player.entity_id as u32;
        for (id, kind, at) in host.level_entities(&dimension) {
            if id == own || kind == "minecraft:item" || kind == "minecraft:experience_orb" {
                continue;
            }
            // Entity positions are at the feet; aim at the middle of a person.
            let centre = [at[0], at[1] + 0.9, at[2]];
            if !gear::in_cone(from, dir, centre, gear::FLAME_RANGE, gear::FLAME_ANGLE) {
                continue;
            }
            host.hurt_entity(&dimension, id, gear::FLAME_DAMAGE, "minecraft:on_fire", Some(own));
            let mut nbt = Compound::default();
            nbt.insert(key(&nbt, "Fire"), Tag::Short((gear::FLAME_FIRE_SECONDS * 20) as i16));
            host.merge_entity_nbt(&dimension, id, nbt);
        }
        if mode == FlameMode::Inferno {
            Self::ignite(host, &dimension, from, dir);
        }
    }

    /// Inferno: the first free block along the flame next to a solid one catches fire.
    fn ignite(host: &mut dyn Host, dimension: &str, from: [f64; 3], dir: [f64; 3]) {
        let cell = |t: f64| [0, 1, 2].map(|i| (from[i] + dir[i] * t).floor() as i32);
        let mut before = cell(0.0);
        let mut t = 0.5;
        while t <= 8.0 {
            let at = cell(t);
            if at != before && host.block(at) != "minecraft:air" {
                if host.block(before) == "minecraft:air" {
                    if let Some(state) = host.default_state("minecraft:fire") {
                        host.set_block(dimension, before, state, 3);
                    }
                }
                return;
            }
            before = at;
            t += 0.5;
        }
    }
}

// --- jetpack: the client pushes, the server pays ---------------------------------------------

impl State {
    /// The client's thrust message: one byte, 1 while the jetpack pushes.
    pub fn net_from_client(&self, event: &NetFromClientEvent<'_>) {
        if event.channel == THRUST_CHANNEL && event.data.first() == Some(&1) {
            self.thrust.lock().unwrap().insert(event.player, Instant::now());
        }
    }

    /// The server's word on the worn jetpack: `[mode, has_fuel]`.
    pub fn net_from_server(&self, event: &NetFromServerEvent<'_>) {
        if event.channel != STATE_CHANNEL || event.data.len() < 2 {
            return;
        }
        let mut c = self.client.lock().unwrap();
        c.mode = (event.data[0] != 255).then(|| JetMode::from_byte(event.data[0]));
        c.fuel = event.data[1] != 0;
        c.at = Some(Instant::now());
    }

    /// Mekanism moves the player on the server; Horizon's server cannot set a player's velocity
    /// (`entity_set_velocity` is refused for players), so the client does it: the velocity is
    /// estimated from the last two positions and `push_player` adds the difference.
    pub fn client_tick_post(&self, host: &mut dyn Host, _event: &ClientTickPostEvent) {
        let Some(me) = host.player() else { return };
        let here = [me.x, me.y, me.z];
        let mut c = self.client.lock().unwrap();
        let last = c.last.replace(here);
        let known = c.at.is_some_and(|t| t.elapsed() < Duration::from_secs(3));
        let (Some(mode), true, true, Some(last)) = (c.mode, c.fuel, known, last) else {
            c.thrusting = false;
            return;
        };
        let v = [here[0] - last[0], here[1] - last[1], here[2] - last[2]];
        let ascending = host.is_key_down(KeyCode::Space);
        let descending = host.is_key_down(KeyCode::ShiftLeft);
        // Standing still on the ground shows as no vertical change: the nearest thing to `onGround`.
        let on_ground = v[1].abs() < 1e-4;
        let active = gear::jet_active(mode, ascending, descending, on_ground);
        if active {
            let up = gear::jet_up(me.yaw, me.pitch);
            let next = gear::jet_motion(mode, v, up, ascending, descending, on_ground, descending);
            host.push_player([
                (next[0] - v[0]) as f32,
                (next[1] - v[1]) as f32,
                (next[2] - v[2]) as f32,
            ]);
        }
        let due = c.sent.is_none_or(|t| t.elapsed() > Duration::from_millis(100));
        if active && due {
            c.sent = Some(Instant::now());
            drop(c);
            let _ = host.net_send_to_server(THRUST_CHANNEL, &[1]);
        } else {
            c.thrusting = active;
        }
    }
}
