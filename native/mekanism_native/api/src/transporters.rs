//! The host side of the Logistical, Restrictive and Diversion Transporters
//! (M08): the container, the tick, the arms and the clicks. The rules, values
//! and routing are Horizon-free in `transporter.rs`; read its header for what is
//! and is not ported.
//!
//! A transporter is a block entity whose container has eight slots: slot 0 is
//! the intake (what a hopper, a Logistical Sorter or `insert_item` puts in; only
//! this slot is reachable from outside), slots 1..8 hold the stacks in transit.
//! Their progress, entry side and origin are saved next to them (`mekanism_transit`),
//! so nothing is lost on save, load or chunk unload, and a broken transporter
//! spills its slots like any container.
//!
//! A stack moves on only when the next holder has taken it: into the next
//! transporter by `change_mod_block_entity` (its free slot, the progress over 100
//! carried), into an inventory by `insert_item` (what comes back stays).
//! Horizon has no call that lists another block's slots (`container_slots` is not
//! on `Host`), so pulling from an inventory scans its slots with `extract_item`.

use std::collections::HashMap;
use std::sync::Mutex;

use horizon_client::mod_api::{BlockEntityTickEvent, Compound, Direction, Host, ItemStack, ModBlockEntity, ModContainer, Tag, Text};

use crate::transporter::{self, Action, Cell, Connection, Graph, Link, Pos, Sides, Tier, Transit, Variant};
use crate::{cube, key, offset};

/// Slot 0 takes what is put in; slots 1.. hold what travels.
pub(super) const SLOTS: usize = 8;
const MODES: &str = "mekanism_modes";
const DIVERSION: &str = "mekanism_diversion";
const TRANSIT: &str = "mekanism_transit";
/// Ticks between two pulls of a Pull side (the scan of a neighbour's slots is not free).
const PULL_TICKS: u8 = 10;
/// The most slots of a neighbour scanned for one pull (a double chest).
const SCAN_SLOTS: usize = 54;

/// Ticks until each transporter may pull next (not saved).
static PULL: Mutex<Option<HashMap<(String, Pos), u8>>> = Mutex::new(None);

fn direction(dir: usize) -> Direction {
    Direction::ALL[dir]
}

/// Slot 0 for the outside, every side.
pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: SLOTS,
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: Vec::new(),
        menu: None,
    }
}

fn read_bytes(nbt: &Compound, name: &'static str) -> Vec<u8> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::ByteArray(b)) => b.clone(),
        _ => Vec::new(),
    }
}

fn read_sides(nbt: &Compound) -> Sides {
    Sides::from_bytes(&read_bytes(nbt, MODES), &read_bytes(nbt, DIVERSION))
}

fn write_tag(nbt: &mut Compound, name: &'static str, tag: Tag) {
    let k = key(nbt, name);
    if nbt.get(&k) != Some(&tag) {
        nbt.insert(k, tag);
    }
}

fn write_sides(nbt: &mut Compound, sides: &Sides) {
    let (connection, diversion) = sides.to_bytes();
    write_tag(nbt, MODES, Tag::ByteArray(connection.to_vec()));
    write_tag(nbt, DIVERSION, Tag::ByteArray(diversion.to_vec()));
}

/// The transit state of every slot (index 0 unused); a stack without a saved
/// state (an old save, an edited one) starts fresh.
fn read_transit(m_nbt: &Compound, items: &[Option<ItemStack>]) -> Vec<Transit> {
    let saved: &[i32] = match m_nbt.get(&key(m_nbt, TRANSIT)) {
        Some(Tag::IntArray(v)) => v,
        _ => &[],
    };
    (0..SLOTS)
        .map(|i| match saved.chunks(Transit::INTS).nth(i) {
            Some(c) if c.len() == Transit::INTS && items.get(i).is_some_and(|s| s.is_some()) => Transit::from_ints(c),
            _ => Transit::fresh(None, None),
        })
        .collect()
}

fn write_transit(nbt: &mut Compound, transit: &[Transit]) {
    let flat: Vec<i32> = transit.iter().flat_map(|t| t.to_ints()).collect();
    write_tag(nbt, TRANSIT, Tag::IntArray(flat));
}

fn free_slot(items: &[Option<ItemStack>]) -> Option<usize> {
    (1..SLOTS).find(|&i| items.get(i).is_none_or(|s| s.is_none()))
}

/// A graph read from the world as it is this tick.
struct World<'a> {
    host: &'a mut dyn Host,
    cache: HashMap<Pos, Cell>,
}

impl Graph for World<'_> {
    fn at(&mut self, pos: Pos) -> Cell {
        if let Some(c) = self.cache.get(&pos) {
            return *c;
        }
        let block = self.host.block(pos);
        let cell = if let Some(t) = transporter::tier_of(&block) {
            let sides = self.host.mod_block_entity(pos).map(|m| read_sides(&m.nbt)).unwrap_or_default();
            let dimension = self.host.dimension().to_owned();
            let powered = t.variant == Variant::Diversion && self.host.neighbor_signal(&dimension, pos) > 0;
            Cell::Transporter(Link::of(t, &sides, powered))
        } else if transporter::is_inventory_block(&block) {
            Cell::Inventory
        } else {
            Cell::Empty
        };
        self.cache.insert(pos, cell);
        cell
    }
}

/// Gives the stack to the transporter at `to`, which it enters by `entered`.
/// False when that block has no room (or is not a transporter).
fn receive(host: &mut dyn Host, to: Pos, entered: usize, stack: &ItemStack, transit: &Transit) -> bool {
    let mut taken = false;
    host.change_mod_block_entity(to, &mut |m: &mut ModBlockEntity| {
        if transporter::tier_of(&m.id).is_none() {
            return;
        }
        m.items.resize(SLOTS, None);
        let Some(i) = free_slot(&m.items) else { return };
        let mut all = read_transit(&m.nbt, &m.items);
        all[i] = transporter::arrived(transit, entered);
        m.items[i] = Some(stack.clone());
        write_transit(&mut m.nbt, &all);
        taken = true;
    });
    taken
}

/// Takes `tier.pull` items from the first non-empty slot of each Pull side's inventory.
fn pull(host: &mut dyn Host, e: &mut ModBlockEntity, pos: Pos, t: &Tier, sides: &Sides, transit: &mut [Transit]) {
    if !sides.connection.contains(&Connection::Pull) || free_slot(&e.items).is_none() {
        return;
    }
    let place = (host.dimension().to_owned(), pos);
    let due = {
        let mut guard = PULL.lock().unwrap();
        let left = guard.get_or_insert_with(HashMap::new).entry(place.clone()).or_insert(0);
        *left = left.saturating_sub(1);
        if *left == 0 {
            *left = PULL_TICKS;
            true
        } else {
            false
        }
    };
    if !due {
        return;
    }
    for d in 0..6 {
        if sides.connection[d] != Connection::Pull {
            continue;
        }
        let Some(slot) = free_slot(&e.items) else { return };
        let np = offset(pos, d);
        if !transporter::is_inventory_block(&host.block(np)) {
            continue;
        }
        let dimension = place.0.as_str();
        for from in 0..SCAN_SLOTS {
            if let Some(stack) = host.extract_item(dimension, np, direction(cube::opposite(d)), from, t.pull).filter(|s| s.count > 0) {
                e.items[slot] = Some(stack);
                transit[slot] = Transit::fresh(Some(d), Some(np));
                break;
            }
        }
    }
}

/// Sets each arm's block state: a side that is on and has a transporter or an
/// inventory beyond it.
fn show_arms(host: &mut dyn Host, pos: Pos, sides: &Sides) {
    for dir in 0..6 {
        let block = host.block(offset(pos, dir));
        let on = sides.connection[dir] != Connection::None
            && (transporter::tier_of(&block).is_some() || transporter::is_inventory_block(&block));
        let want = if on { "true" } else { "false" };
        if host.block_property(pos, cube::DIR_NAMES[dir]).as_deref() != Some(want) {
            host.set_block_property(pos, cube::DIR_NAMES[dir], want);
        }
    }
}

pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &'static Tier) {
    let pos = event.pos;
    let sides = read_sides(&event.entity.nbt);
    show_arms(host, pos, &sides);
    let e = &mut *event.entity;
    e.items.resize(SLOTS, None);
    let mut transit = read_transit(&e.nbt, &e.items);
    let dimension = host.dimension().to_owned();
    let powered = t.variant == Variant::Diversion && host.neighbor_signal(&dimension, pos) > 0;
    let link = Link::of(t, &sides, powered);

    // What was put in goes into a free slot of the line.
    if e.items[0].is_some() {
        if let Some(i) = free_slot(&e.items) {
            e.items[i] = e.items[0].take();
            transit[i] = Transit::fresh(None, None);
        }
    }
    pull(host, e, pos, t, &sides, &mut transit);

    let mut world = World { host, cache: HashMap::new() };
    for i in 1..SLOTS {
        let Some(stack) = e.items[i].clone() else { continue };
        let action = transporter::advance(&mut transit[i], t.speed, &mut world, pos, &link);
        match action {
            Action::Stay => {}
            Action::Forward(d) => {
                if receive(world.host, offset(pos, d), cube::opposite(d), &stack, &transit[i]) {
                    e.items[i] = None;
                } else {
                    transporter::refused(&mut transit[i], action, Some(pos));
                }
            }
            Action::Deliver(d) => {
                let rest = world.host.insert_item(offset(pos, d), direction(cube::opposite(d)), stack.clone()).filter(|s| s.count > 0);
                if let Some(rest) = rest {
                    e.items[i] = Some(rest);
                    transporter::refused(&mut transit[i], action, Some(pos));
                } else {
                    e.items[i] = None;
                }
            }
        }
    }
    write_transit(&mut e.nbt, &transit);
}

/// The side a clicked face (a unit normal) is.
fn side_of(face: [i32; 3]) -> Option<usize> {
    (0..6).find(|&d| cube::OFFSETS[d] == face)
}

/// Right click with an empty hand. A Diversion Transporter cycles the
/// diversion control of the side clicked (Mekanism: disabled, high, low); sneaking
/// cycles the connection of the side clicked (normal, push, pull, none), which
/// stands in for the Configurator, not in this repository yet. True when the
/// click was ours.
pub(super) fn right_click(host: &mut dyn Host, pos: Pos, face: [i32; 3], t: &Tier) -> bool {
    if host.held().is_some() {
        return false;
    }
    let sneaking = host.sneaking();
    if !sneaking && t.variant != Variant::Diversion {
        return false;
    }
    // The arm looked at, else the clicked face (a chest hides the face that points at it).
    let picked = host.acting_player().and_then(|p| {
        let eye = [p.position[0], p.position[1] + if sneaking { 1.27 } else { 1.62 }, p.position[2]];
        transporter::picked_side(pos, eye, transporter::view_vector(p.yaw, p.pitch))
    });
    let Some(side) = picked.or_else(|| side_of(face)) else { return false };
    let mut line = None;
    host.change_mod_block_entity(pos, &mut |m: &mut ModBlockEntity| {
        let mut sides = read_sides(&m.nbt);
        line = Some(if sneaking {
            sides.connection[side] = sides.connection[side].next();
            format!("{}: {:?}", cube::DIR_NAMES[side], sides.connection[side])
        } else {
            sides.diversion[side] = sides.diversion[side].next();
            format!("{}: {:?}", cube::DIR_NAMES[side], sides.diversion[side])
        });
        write_sides(&mut m.nbt, &sides);
    });
    let Some(line) = line else { return false };
    if let Some(p) = host.acting_player() {
        host.action_bar(p.uuid, Text::literal(line));
    }
    true
}
