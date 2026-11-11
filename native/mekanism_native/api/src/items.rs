//! The hand-held gear (M11): Dictionary, Canteen, Gauge Dropper and Atomic Disassembler.
//! Rules and numbers are in `gear.rs`; this wires them to Horizon's item hooks, which
//! `lib.rs` forwards here (see docs/ITEM_HOOKS.md for which hooks exist and which do not).
//!
//! Item state lives in the stack's `minecraft:custom_data`, in the keys the tank items
//! already use (`mekanism_fluid`, `mekanism_amount`), plus `mekanism_mode`; the energy is
//! Horizon's own `horizon_energy` (`item_energy_buffer`).
//!
//! Hooks: `right_click_block` (a block under the cursor, also machines and tanks that open a
//! menu), `use_item` (in the air), `use_duration`/`use_animation`/`living_entity_use_item_start`/
//! `living_entity_use_item_finish` (drinking), `living_incoming_damage_v2` (weapon bonus),
//! `destroy_block` (vein mining), `inventory_tick` (which slot is the main hand),
//! `item_bar` and `tooltip`.
//!
//! A hook that changes the stack in hand does it through `set_inventory_slot`: the hooks
//! give the stack by reference, and `carried_stack` works only inside `inventory_tick`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_client::mod_api::{
    CanEatEvent, Compound, DestroyBlockEvent, Direction, Flow, Host, InventoryTickEvent, ItemBarEvent, ItemStack,
    LivingEntityUseItemFinishEvent, LivingEntityUseItemStartEvent, OnEquipEvent, LivingIncomingDamageV2Event,
    Tag, Text, UseAnimation, UseAnimationEvent, UseDurationEvent,
};

use super::gear::{self, Mode};
use super::{chemicals, dictionary_tags, key, read_energy, ITEM_ENERGY};

const ITEM_FLUID: &str = "mekanism_fluid";
const ITEM_AMOUNT: &str = "mekanism_amount";
const ITEM_MODE: &str = "mekanism_mode";
const CUSTOM_DATA: &str = "minecraft:custom_data";
/// The tag of blocks the vein mode breaks together (`mekanism:atomic_disassembler_ore`).
const VEIN_TAG: &str = "mekanism:atomic_disassembler_ore";
/// Item bar colours (`rgb`).
const ENERGY_COLOR: u32 = 0x2ED8FF;
/// How long after a block click the `use_item` of the same click is ignored.
const CLICK_SPAN: Duration = Duration::from_millis(250);
const PASTE_COLOR: u32 = 0xEB6CA3;
const FLUID_COLOR: u32 = 0x3F76E4;

#[derive(Default)]
pub struct State {
    /// When a gear item last took a right click on a block: the same click then reaches `use_item`,
    /// which must not act on it again.
    clicked: Mutex<HashMap<u128, Instant>>,
    /// Food points a drink gave, added to the player at their next `inventory_tick`.
    fed: Mutex<HashMap<u128, i32>>,
    /// Changes of the main-hand stack waiting for the next `inventory_tick`.
    pending: Mutex<HashMap<u128, Vec<Edit>>>,
    /// The item each player holds in the off hand, from `on_equip`: the host has no inventory read
    /// for the off hand. Empty until the player changes it, so a login with the item is not seen.
    off_hand: Mutex<HashMap<u128, String>>,
    /// Set while a vein breaks, so its own `destroy_block` calls do not start another vein.
    veining: AtomicBool,
}

/// The disassembler's energy buffer, for `item_energy_buffer`.
pub fn energy_buffer(item: &str) -> Option<(i32, i32)> {
    (item == gear::DISASSEMBLER).then_some((gear::DISASSEMBLER_CAPACITY, gear::DISASSEMBLER_RATE))
}

fn fluid_of(stack: &ItemStack) -> (Option<String>, i32) {
    let Some(data) = stack.custom_data() else { return (None, 0) };
    let amount = read_energy(data, ITEM_AMOUNT);
    let fluid = match data.get(&key(data, ITEM_FLUID)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    if amount <= 0 { (None, 0) } else { (fluid, amount) }
}

fn energy_of(stack: &ItemStack) -> i32 {
    stack.custom_data().map_or(0, |d| read_energy(d, ITEM_ENERGY)).max(0)
}

fn mode_of(stack: &ItemStack) -> Mode {
    let byte = stack.custom_data().and_then(|d| match d.get(&key(d, ITEM_MODE)) {
        Some(Tag::Byte(b)) => Some(*b as u8),
        _ => None,
    });
    Mode::from_byte(byte.unwrap_or(0))
}

/// SNBT of the tags the gear keeps in `custom_data`; any other kind is left out.
fn snbt(tag: &Tag) -> Option<String> {
    Some(match tag {
        Tag::Byte(n) => format!("{n}b"),
        Tag::Int(n) => n.to_string(),
        Tag::Long(n) => format!("{n}L"),
        Tag::String(s) => format!("{:?}", s.to_string_lossy()),
        Tag::ByteArray(bytes) => {
            format!("[B;{}]", bytes.iter().map(|b| format!("{}b", *b as i8)).collect::<Vec<_>>().join(","))
        }
        Tag::Compound(c) => {
            let entries: Vec<_> = c
                .iter()
                .filter_map(|(k, v)| Some(format!("{:?}:{}", k.to_string_lossy(), snbt(v)?)))
                .collect();
            format!("{{{}}}", entries.join(","))
        }
        _ => return None,
    })
}

/// `stack` with its custom data after `change`. `set_component` only takes a mod's own component
/// types, so the stack is rebuilt from SNBT; its other components are dropped (the gear has none).
fn edit_data(host: &dyn Host, stack: &mut ItemStack, change: impl FnOnce(&mut Compound)) {
    let mut data = stack.custom_data().cloned().unwrap_or_default();
    change(&mut data);
    let Some(data) = snbt(&Tag::Compound(data)) else { return };
    let text = format!("{{id:{:?},count:{},components:{{\"minecraft:custom_data\":{data}}}}}", &*stack.item, stack.count);
    if let Some(next) = host.item_from_snbt(&text) {
        *stack = next;
    }
}

fn set_fluid(host: &dyn Host, stack: &mut ItemStack, fluid: Option<&str>, amount: i32) {
    edit_data(host, stack, |d| match fluid.filter(|_| amount > 0) {
        Some(f) => {
            d.insert(key(d, ITEM_FLUID), Tag::String(f.into()));
            d.insert(key(d, ITEM_AMOUNT), Tag::Int(amount));
        }
        None => {
            d.remove(&key(d, ITEM_FLUID));
            d.remove(&key(d, ITEM_AMOUNT));
        }
    });
}

fn set_energy(host: &dyn Host, stack: &mut ItemStack, energy: i32) {
    edit_data(host, stack, |d| {
        d.insert(key(d, ITEM_ENERGY), Tag::Int(energy.clamp(0, gear::DISASSEMBLER_CAPACITY)));
    });
}

fn fluid_line(fluid: Option<&str>, amount: i32, capacity: i32) -> String {
    match fluid {
        Some(f) if amount > 0 => format!("{}: {amount} / {capacity} mB", chemicals::name(f)),
        _ => format!("Empty: 0 / {capacity} mB"),
    }
}

/// The `Direction` of a face normal; up for a normal that is none of the six.
fn dir_of(face: [i32; 3]) -> Direction {
    let index = super::cube::OFFSETS.iter().position(|o| *o == face).unwrap_or(1);
    Direction::ALL[index]
}

type Edit = Box<dyn FnOnce(&dyn Host, &mut ItemStack) + Send>;

impl State {
    /// Queue a change of the main-hand stack. The use hooks run with the player's state swapped in,
    /// where the inventory cannot be written, so `inventory_tick` applies it a moment later.
    fn edit_held(&self, host: &mut dyn Host, change: impl FnOnce(&dyn Host, &mut ItemStack) + Send + 'static) -> bool {
        let Some(player) = host.acting_player() else { return false };
        self.pending.lock().unwrap().entry(player.uuid).or_default().push(Box::new(change));
        true
    }

    /// Apply the queued changes to the main-hand stack, which `carried_stack` reaches only here.
    pub fn inventory_tick(&self, host: &mut dyn Host, event: &InventoryTickEvent) {
        if !event.selected {
            return;
        }
        let Some(player) = host.acting_player() else { return };
        if let Some(gain) = self.fed.lock().unwrap().remove(&player.uuid) {
            if let Some((food, saturation, exhaustion)) = host.player_food(player.uuid) {
                host.set_player_status(player.uuid, None, Some(((food + gain).min(gear::MAX_FOOD), saturation, exhaustion)), None);
            }
        }
        let edits = self.pending.lock().unwrap().remove(&player.uuid).unwrap_or_default();
        if edits.is_empty() {
            return;
        }
        let Some(mut stack) = host.carried_stack(event.slot).cloned() else { return };
        for edit in edits {
            edit(&*host, &mut stack);
        }
        if let Some(slot) = host.carried_stack(event.slot) {
            *slot = stack;
        }
    }

    /// Remember what the off hand holds.
    pub fn on_equip(&self, host: &mut dyn Host, event: &OnEquipEvent<'_>) {
        if event.slot != "offhand" {
            return;
        }
        if let Some(player) = host.acting_player() {
            let mut held = self.off_hand.lock().unwrap();
            match event.new {
                Some(stack) => held.insert(player.uuid, stack.item.to_string()),
                None => held.remove(&player.uuid),
            };
        }
    }

    /// A right click on a block, before the block's own use. Returns whether it was used.
    pub fn right_click_block(&self, host: &mut dyn Host, pos: [i32; 3], face: [i32; 3]) -> Flow {
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let Some(held) = host.held_stack(player.uuid) else { return Flow::Continue };
        if matches!(&*held.item, gear::DICTIONARY | gear::GAUGE_DROPPER | gear::CANTEEN) {
            self.clicked.lock().unwrap().insert(player.uuid, Instant::now());
        }
        match &*held.item {
            gear::DICTIONARY => {
                let block = host.block(pos);
                self.dictionary(host, player.uuid, &block, true);
                Flow::Cancel
            }
            gear::GAUGE_DROPPER => {
                let sneaking = host.sneaking();
                self.dropper(host, &held, pos, face, sneaking)
            }
            gear::CANTEEN => self.canteen_fill(host, &held, pos, face),
            _ => Flow::Continue,
        }
    }

    /// An item used in the air: the Dictionary reads the off hand, a sneaking Gauge Dropper
    /// empties itself and a sneaking Disassembler changes its mode.
    pub fn use_item(&self, host: &mut dyn Host, stack: &ItemStack) -> Flow {
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let on_block = self.clicked.lock().unwrap().remove(&player.uuid).is_some_and(|t| t.elapsed() < CLICK_SPAN);
        if on_block {
            return Flow::Cancel;
        }
        match &*stack.item {
            gear::DICTIONARY => {
                let off = self.off_hand.lock().unwrap().get(&player.uuid).cloned();
                match off {
                    Some(item) => self.dictionary(host, player.uuid, &item, false),
                    None => {
                        host.tell(player.uuid, Text::literal("Dictionary: use on a block, or hold an item in the off hand."));
                    }
                }
                Flow::Cancel
            }
            gear::GAUGE_DROPPER if host.sneaking() => {
                self.edit_held(host, |h, s| set_fluid(h, s, None, 0));
                host.action_bar(player.uuid, Text::literal("Gauge Dropper emptied"));
                Flow::Cancel
            }
            gear::DISASSEMBLER if host.sneaking() => {
                let mode = mode_of(stack).next();
                self.edit_held(host, move |h, s| {
                    edit_data(h, s, |d| {
                        d.insert(key(d, ITEM_MODE), Tag::Byte(mode.byte() as i8));
                    })
                });
                host.action_bar(player.uuid, Text::literal(format!("Disassembler mode: {}", mode.name())));
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }

    /// Tags of the block at `pos` (`at_block`) or of the item `id`, as chat lines.
    fn dictionary(&self, host: &mut dyn Host, uuid: u128, id: &str, at_block: bool) {
        let mut lines = vec![format!("Tags of {id}:")];
        if at_block {
            let tags: Vec<_> = dictionary_tags::BLOCK_TAGS
                .iter()
                .filter(|t| host.block_name_in_tag(id, t).unwrap_or(false))
                .collect();
            lines.push(format!("Block tags ({}):", tags.len()));
            lines.extend(tags.iter().map(|t| format!("  #{t}")));
        }
        let stack = host.item_from_snbt(&format!("{{id:\"{id}\",count:1}}"));
        let item_tags: Vec<_> = stack.as_ref().map_or_else(Vec::new, |s| {
            dictionary_tags::ITEM_TAGS.iter().filter(|t| host.server_item_in_tag(s, t).unwrap_or(false)).collect()
        });
        lines.push(format!("Item tags ({}):", item_tags.len()));
        lines.extend(item_tags.iter().map(|t| format!("  #{t}")));
        for line in lines {
            host.tell(uuid, Text::literal(line));
        }
    }

    /// Gauge Dropper on a block with fluid tanks: draw from the first tank that has any, or,
    /// sneaking, put the dropper's content into the first tank that takes it.
    fn dropper(&self, host: &mut dyn Host, held: &ItemStack, pos: [i32; 3], face: [i32; 3], sneaking: bool) -> Flow {
        let face = dir_of(face);
        let Some(tanks) = host.fluid_tanks(host.dimension(), pos, face) else { return Flow::Continue };
        let have = fluid_of(held);
        let moved = if sneaking {
            let Some(fluid) = have.0.clone() else { return Flow::Cancel };
            let room = tanks
                .iter()
                .find(|t| t.fluid.as_deref().map_or(true, |f| f == fluid))
                .map_or(0, |t| t.capacity - t.amount);
            let step = gear::put(&have, room);
            if step <= 0 {
                return Flow::Cancel;
            }
            // `fluid_insert` returns what did not fit; the tank's accept hook may refuse a kind.
            let left = host.fluid_insert(pos, face, &fluid, step);
            Some((fluid, -(step - left)))
        } else {
            let Some(from) = tanks.iter().find_map(|t| Some((t.fluid.clone()?, t.amount))) else {
                return Flow::Cancel;
            };
            gear::draw(&have, &from).and_then(|(fluid, step)| {
                let (got, n) = host.fluid_extract(host.dimension(), pos, face, step)?;
                (got == fluid && n > 0).then_some((fluid, n))
            })
        };
        if let Some((fluid, delta)) = moved.filter(|(_, d)| *d != 0) {
            let now = (have.1 + delta).clamp(0, gear::DROPPER_CAPACITY);
            self.edit_held(host, move |h, s| set_fluid(h, s, Some(&fluid), now));
        }
        Flow::Cancel
    }

    /// A canteen on a tank holding nutritional paste draws a bucket of it: Mekanism fills it in the
    /// Nutritional Liquifier, which does not exist here yet.
    fn canteen_fill(&self, host: &mut dyn Host, held: &ItemStack, pos: [i32; 3], face: [i32; 3]) -> Flow {
        let face = dir_of(face);
        let Some(tanks) = host.fluid_tanks(host.dimension(), pos, face) else { return Flow::Continue };
        if !tanks.iter().any(|t| t.fluid.as_deref() == Some(gear::NUTRITIONAL_PASTE) && t.amount > 0) {
            return Flow::Continue;
        }
        let have = fluid_of(held).1;
        let step = 1_000.min(gear::CANTEEN_CAPACITY - have);
        if step > 0 {
            if let Some((fluid, n)) = host.fluid_extract(host.dimension(), pos, face, step) {
                if fluid == gear::NUTRITIONAL_PASTE && n > 0 {
                    self.edit_held(host, move |h, s| set_fluid(h, s, Some(gear::NUTRITIONAL_PASTE), have + n));
                }
            }
        }
        Flow::Cancel
    }

    /// A canteen cannot be drunk with a full food bar or no paste.
    pub fn use_start(&self, host: &mut dyn Host, event: &LivingEntityUseItemStartEvent<'_>) -> Flow {
        if &*event.stack.item != gear::CANTEEN {
            return Flow::Continue;
        }
        let food = host.acting_player().and_then(|p| host.player_food(p.uuid)).map_or(0, |f| f.0);
        if gear::can_drink(fluid_of(event.stack).1, food) { Flow::Continue } else { Flow::Cancel }
    }

    pub fn use_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        if &*event.stack.item != gear::CANTEEN {
            return;
        }
        let Some(player) = host.acting_player() else { return };
        let Some((food, _, _)) = host.player_food(player.uuid) else { return };
        let stored = fluid_of(event.stack).1;
        let Some((gain, used)) = gear::drink(stored, food) else { return };
        *self.fed.lock().unwrap().entry(player.uuid).or_insert(0) += gain;
        let mut next = event.stack.clone();
        set_fluid(&*host, &mut next, Some(gear::NUTRITIONAL_PASTE), stored - used);
        *event.result = Some(next);
    }

    /// The disassembler's weapon bonus, paid in energy; the template's attack is the unpowered one.
    pub fn damage(&self, host: &mut dyn Host, event: &mut LivingIncomingDamageV2Event<'_>) {
        let Some(weapon) = event.weapon.filter(|w| &*w.item == gear::DISASSEMBLER) else { return };
        let energy = energy_of(weapon);
        if energy < gear::WEAPON_COST {
            return;
        }
        event.amount += gear::WEAPON_BONUS;
        self.edit_held(host, |h, s| set_energy(h, s, energy_of(s) - gear::WEAPON_COST));
    }

    /// Mining costs energy; without it the block stays. In vein mode an ore takes its vein with it.
    pub fn destroy_block(&self, host: &mut dyn Host, event: &DestroyBlockEvent) -> Flow {
        if self.veining.load(Ordering::Relaxed) {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let Some(held) = host.held_stack(player.uuid).filter(|h| &*h.item == gear::DISASSEMBLER) else {
            return Flow::Continue;
        };
        if player.game_mode == 1 {
            return Flow::Continue;
        }
        let energy = energy_of(&held);
        if energy < gear::MINE_COST {
            return Flow::Cancel;
        }
        let mut spent = gear::MINE_COST;
        let block = host.block(event.pos);
        if mode_of(&held) == Mode::Vein && host.block_name_in_tag(&block, VEIN_TAG).unwrap_or(false) {
            let limit = gear::affordable(energy - spent).min(gear::VEIN_LIMIT);
            let found = gear::vein(event.pos, limit, |p| host.block(p) == block);
            self.veining.store(true, Ordering::Relaxed);
            for p in &found {
                // `false`: the drops spawn as for any break.
                if host.destroy_block(*p, Some(&held), false).is_some() {
                    spent += gear::MINE_COST;
                }
            }
            self.veining.store(false, Ordering::Relaxed);
        }
        self.edit_held(host, move |h, s| set_energy(h, s, energy_of(s) - spent));
        Flow::Continue
    }

    /// Fill levels of the gear's item bars.
    pub fn bar(&self, event: &mut ItemBarEvent<'_>) {
        let (fill, color) = match &*event.stack.item {
            gear::DISASSEMBLER => (energy_of(event.stack) as f32 / gear::DISASSEMBLER_CAPACITY as f32, ENERGY_COLOR),
            gear::CANTEEN => (fluid_of(event.stack).1 as f32 / gear::CANTEEN_CAPACITY as f32, PASTE_COLOR),
            gear::GAUGE_DROPPER => (fluid_of(event.stack).1 as f32 / gear::DROPPER_CAPACITY as f32, FLUID_COLOR),
            _ => return,
        };
        *event.bar = Some((((fill.clamp(0.0, 1.0) * 13.0).round() as i32).max(i32::from(fill > 0.0)), color));
    }

    pub fn tooltip(&self, stack: &ItemStack) -> Vec<(Text, u32)> {
        let grey = 0xFF_AAAAAA;
        match &*stack.item {
            gear::DISASSEMBLER => vec![
                (Text::literal(format!("Mode: {}", mode_of(stack).name())), grey),
                (
                    Text::literal(format!(
                        "Stored: {} / {} FE",
                        super::cube::si(energy_of(stack) as i64),
                        super::cube::si(gear::DISASSEMBLER_CAPACITY as i64)
                    )),
                    grey,
                ),
            ],
            gear::CANTEEN => {
                let (fluid, n) = fluid_of(stack);
                vec![(Text::literal(fluid_line(fluid.as_deref(), n, gear::CANTEEN_CAPACITY)), grey)]
            }
            gear::GAUGE_DROPPER => {
                let (fluid, n) = fluid_of(stack);
                vec![(Text::literal(fluid_line(fluid.as_deref(), n, gear::DROPPER_CAPACITY)), grey)]
            }
            _ => Vec::new(),
        }
    }

    pub fn use_duration(&self, event: &mut UseDurationEvent<'_>) {
        if &*event.stack.item == gear::CANTEEN {
            event.duration = gear::CANTEEN_USE_TICKS;
        }
    }

    pub fn use_animation(&self, event: &mut UseAnimationEvent<'_>) {
        if &*event.stack.item == gear::CANTEEN {
            *event.animation = UseAnimation::Drink;
        }
    }

    pub fn can_eat(&self, host: &dyn Host, event: &mut CanEatEvent<'_>) {
        if &*event.stack.item == gear::CANTEEN {
            let food = host.acting_player().and_then(|p| host.player_food(p.uuid)).map_or(0, |f| f.0);
            event.can_eat = gear::can_drink(fluid_of(event.stack).1, food);
        }
    }
}
