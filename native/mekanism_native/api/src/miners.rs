//! The host side of the Digital Miner (M08): settings in the block entity NBT, the search and
//! mining run, its drops, ejection, the bounding blocks and the screen. Rules are Horizon-free
//! in `miner.rs`.
//!
//! Differences from Mekanism (docs/sessions/m08-special-machines.md): the scan runs a slice per
//! tick instead of a thread, the search and the drops that did not fit are not saved (a loaded
//! miner that was running searches again, as upstream's does), filters are the items in the
//! nine filter slots and one block tag, and replace targets, xp containment, upgrades and the
//! chunk loading anchor are not ported.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, Host, HudBatch, ItemStack, MenuButtonEvent, ModContainer,
    ModEnergyBuffer, NativeScreen, ScreenDrawEvent, Tag,
};

use crate::miner::{self, Action, Settings, State, View};
use crate::{cube, facing, generator, generators, key, offset, read_energy, write_int, ENERGY};

const RADIUS: &str = "mekanism_dm_radius";
const MIN_Y: &str = "mekanism_dm_min_y";
const MAX_Y: &str = "mekanism_dm_max_y";
const FLAGS: &str = "mekanism_dm_flags";
const TAG: &str = "mekanism_dm_tag";

/// A fully charged Atomic Disassembler mines as a diamond-or-better pickaxe; the silk
/// touch variant carries the enchantment (`TileEntityDigitalMiner.getDrops`).
const TOOL: &str = "{id:\"minecraft:netherite_pickaxe\",count:1}";
const SILK_TOOL: &str = "{id:\"minecraft:netherite_pickaxe\",count:1,components:{\"minecraft:enchantments\":{\"minecraft:silk_touch\":1}}}";

/// What a running miner holds that is not saved.
#[derive(Default)]
struct Run {
    search: miner::Search,
    /// Ticks until the next block (`delay`).
    delay: i32,
    eject_delay: i32,
    /// Drops that found no room; nothing is mined until they are stored.
    overflow: Vec<ItemStack>,
    settings: Settings,
    filters: Vec<String>,
}

static RUNS: Mutex<BTreeMap<(String, [i32; 3]), Run>> = Mutex::new(BTreeMap::new());

pub(super) fn container() -> ModContainer {
    let main: Vec<usize> = (0..miner::MAIN_SLOTS).collect();
    ModContainer {
        slots: miner::SLOTS,
        // The main slots reach hoppers from the top and the sides (upstream: top and back).
        faces: [Vec::new(), main.clone(), main.clone(), main.clone(), main.clone(), main],
        inputs: Vec::new(),
        outputs: (0..miner::MAIN_SLOTS).collect(),
        menu: Some(miner::MENU.to_owned()),
    }
}

pub(super) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: miner::CAPACITY,
        max_insert: miner::CAPACITY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: miner::WIDTH,
        height: miner::HEIGHT,
        slots: miner::slot_positions(),
        inventory_y: miner::INVENTORY_Y,
        inventory_x: miner::INVENTORY_X,
        own_labels: true,
        own_background: false,
    }
}

fn read_opt(nbt: &Compound, name: &'static str) -> Option<i32> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Int(v)) => Some(*v),
        _ => None,
    }
}

fn load(nbt: &Compound) -> (Settings, bool) {
    let d = Settings::default();
    let flags = read_opt(nbt, FLAGS).unwrap_or(0);
    let s = Settings {
        radius: read_opt(nbt, RADIUS).unwrap_or(d.radius),
        min_y: read_opt(nbt, MIN_Y).unwrap_or(d.min_y),
        max_y: read_opt(nbt, MAX_Y).unwrap_or(d.max_y),
        silk: flags & miner::FLAG_SILK != 0,
        eject: flags & miner::FLAG_EJECT != 0,
        inverse: flags & miner::FLAG_INVERSE != 0,
        tag: read_opt(nbt, TAG).and_then(|t| usize::try_from(t - 1).ok()).filter(|t| *t < miner::TAG_CHOICES.len()),
    };
    (s, flags & miner::FLAG_RUNNING != 0)
}

fn save(nbt: &mut Compound, s: &Settings, running: bool) {
    write_int(nbt, RADIUS, s.radius);
    write_int(nbt, MIN_Y, s.min_y);
    write_int(nbt, MAX_Y, s.max_y);
    let flag = |on: bool, bit: i32| if on { bit } else { 0 };
    let flags = flag(s.silk, miner::FLAG_SILK) | flag(s.eject, miner::FLAG_EJECT) | flag(s.inverse, miner::FLAG_INVERSE)
        | flag(running, miner::FLAG_RUNNING);
    write_int(nbt, FLAGS, flags);
    write_int(nbt, TAG, s.tag.map_or(0, |t| t as i32 + 1));
}

/// The world's lowest and highest block y.
fn world(host: &dyn Host) -> (i32, i32) {
    let (min, height) = host.world_height();
    (min, min + height - 1)
}

/// Whether a filter takes the block: an item in a filter slot, or the tag filter's tag.
fn matches(host: &dyn Host, name: &str, filters: &[String], tag: Option<&str>) -> bool {
    filters.iter().any(|f| f == name) || tag.is_some_and(|t| host.block_name_in_tag(name, t).unwrap_or(false))
}

/// Adds `stack` to the main slots (onto matching stacks first, then into empty slots); what
/// did not fit comes back.
fn store(host: &dyn Host, items: &mut [Option<ItemStack>], mut stack: ItemStack) -> Option<ItemStack> {
    let max = host.max_stack_size(&stack.item).unwrap_or(64);
    for slot in items[..miner::MAIN_SLOTS].iter_mut() {
        if let Some(have) = slot {
            if have.item == stack.item && have.extra == stack.extra && have.count < max {
                let n = (max - have.count).min(stack.count);
                have.count += n;
                stack.count -= n;
                if stack.count == 0 {
                    return None;
                }
            }
        }
    }
    for slot in items[..miner::MAIN_SLOTS].iter_mut() {
        if slot.is_none() {
            let n = max.min(stack.count);
            *slot = Some(ItemStack { count: n, ..stack.clone() });
            stack.count -= n;
            if stack.count == 0 {
                return None;
            }
        }
    }
    Some(stack)
}

/// `tryMineBlock`: the next block of the queue that is still there and wanted, broken with the
/// miner's tool; its drops go to the inventory.
fn mine(host: &mut dyn Host, run: &mut Run, s: &Settings, pos: [i32; 3], items: &mut [Option<ItemStack>]) {
    let tool = host.item_from_snbt(if s.silk { SILK_TOOL } else { TOOL });
    let tag = s.tag.map(|t| miner::TAG_CHOICES[t]);
    for _ in 0..miner::MINE_LOOKS_PER_TICK {
        let Some(index) = run.search.queue.next() else { return };
        run.search.queue.pop();
        let at = miner::offset_for_index(s.start(pos), s.diameter(), index as usize);
        let name = host.block(at);
        if !miner::can_break(&name) || !miner::accepted(s.inverse, matches(host, &name, &run.filters, tag)) {
            continue;
        }
        if let Some(drops) = host.destroy_block(at, tool.as_ref(), true) {
            for drop in drops {
                if let Some(rest) = store(host, items, drop) {
                    run.overflow.push(rest);
                }
            }
            return;
        }
    }
}

/// Auto eject: every ten ticks the stacks go into the container two blocks behind and one up.
fn eject(host: &mut dyn Host, pos: [i32; 3], items: &mut [Option<ItemStack>]) {
    let front = facing(host, pos);
    let back = cube::opposite(front);
    let target = offset(offset(offset(pos, cube::UP), back), back);
    for slot in items[..miner::MAIN_SLOTS].iter_mut() {
        let Some(stack) = slot.take() else { continue };
        *slot = host.insert_item(target, Direction::ALL[front], stack);
    }
}

fn state_of(running: bool, run: &Run) -> State {
    match (running, run.search.done, run.search.started) {
        (true, true, _) => State::Ready,
        (true, false, _) => State::Searching,
        (false, true, true) => State::Paused,
        _ => State::Idle,
    }
}

pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
    let pos = event.pos;
    let place = (host.dimension().to_owned(), pos);
    let (wmin, wmax) = world(host);
    let (mut s, running) = load(&event.entity.nbt);
    s.fit(wmin, wmax);
    let mut items = event.entity.items.clone();
    items.resize(miner::SLOTS, None);
    let filters: Vec<String> = items[miner::MAIN_SLOTS..].iter().flatten().map(|f| (*f.item).to_owned()).collect();

    // Taken out of the map while it runs: the host calls below may reach the hooks again.
    let mut run = RUNS.lock().unwrap().remove(&place).unwrap_or_default();
    if run.filters != filters || run.settings != s {
        if run.search.started {
            run = Run::default();
        }
        run.filters = filters.clone();
        run.settings = s;
    }

    if running && !run.search.done {
        if !miner::searches(&s, filters.len()) {
            run.search.started = true;
            run.search.done = true;
        } else {
            let tag = s.tag.map(|t| miner::TAG_CHOICES[t]);
            let mut known: HashMap<String, bool> = HashMap::new();
            let level: &dyn Host = &*host;
            run.search.step(&s, pos, miner::SEARCH_PER_TICK, &mut |p| {
                let name = level.block(p);
                if !miner::can_break(&name) {
                    return false;
                }
                *known
                    .entry(name.clone())
                    .or_insert_with(|| miner::accepted(s.inverse, matches(level, &name, &filters, tag)))
            });
        }
    }

    let usage = miner::energy_per_tick(&s, wmax - wmin + 1);
    let mut energy = read_energy(&event.entity.nbt, ENERGY).clamp(0, miner::CAPACITY);

    // Drops that did not fit are tried again before anything is mined.
    for stack in std::mem::take(&mut run.overflow) {
        if let Some(rest) = store(host, &mut items, stack) {
            run.overflow.push(rest);
        }
    }

    let mut active = false;
    if running && run.search.done && !run.search.queue.is_empty() && run.overflow.is_empty() && energy >= usage {
        energy -= usage;
        active = true;
        run.delay = (run.delay - 1).max(0);
        if run.delay == 0 {
            mine(host, &mut run, &s, pos, &mut items);
            run.delay = miner::TICKS_PER_MINE;
        }
    }

    if s.eject && run.eject_delay <= 0 {
        eject(host, pos, &mut items);
        run.eject_delay = miner::EJECT_TICKS;
    } else {
        run.eject_delay -= 1;
    }

    // The model lights up while it mines; set the property only on change.
    let want = if active { "true" } else { "false" };
    if host.block_property(pos, "active").is_some_and(|have| have != want) {
        host.set_block_property(pos, "active", want);
    }

    let view = View {
        settings: s,
        running,
        active,
        state: state_of(running, &run),
        to_mine: run.search.queue.len().min(i32::MAX as usize) as i32,
        energy,
        usage,
    };
    RUNS.lock().unwrap().insert(place, run);

    let e = &mut *event.entity;
    if e.items != items {
        e.items = items;
    }
    write_int(&mut e.nbt, ENERGY, energy);
    save(&mut e.nbt, &s, running);
    let data = miner::menu_data(&view);
    if e.data != data {
        e.data = data;
    }
}

/// A press: settings change only while the miner is stopped, and start the search anew.
pub(super) fn button(host: &dyn Host, event: &mut MenuButtonEvent) {
    let Some(action) = miner::action_of(event.id) else { return };
    let place = (host.dimension().to_owned(), event.pos);
    let (wmin, wmax) = world(host);
    let e = &mut *event.entity;
    let (mut s, mut running) = load(&e.nbt);
    match action {
        Action::StartStop => {
            let mut runs = RUNS.lock().unwrap();
            // Stopping in the middle of the search throws it away (`stop` while SEARCHING).
            if running && runs.get(&place).is_some_and(|r| !r.search.done) {
                runs.remove(&place);
            }
            running = !running;
        }
        Action::Reset => {
            RUNS.lock().unwrap().remove(&place);
            running = false;
        }
        _ if running => return,
        _ => {
            if !miner::apply(&mut s, action, (wmin, wmax)) {
                return;
            }
            RUNS.lock().unwrap().remove(&place);
        }
    }
    save(&mut e.nbt, &s, running);
}

pub(super) fn click(at: [f32; 2]) -> Option<i32> {
    miner::button_at(at)
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let flags = d(miner::DATA_FLAGS);
    let on = |bit: i32| flags & bit != 0;
    let batch: &mut HudBatch = &mut *event.batch;
    let title = host.screen_title().unwrap_or_else(|| "Digital Miner".to_owned());
    host.text(batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let button = |batch: &mut HudBatch, rect: [f32; 4], color: u32, label: &str| {
        let [bx, by, bw, bh] = [x + rect[0], y + rect[1], rect[2], rect[3]];
        host.fill(batch, [bx, by, bw, bh], 0xFF262A2F);
        host.fill(batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], color);
        host.text(batch, label, [bx + 3.0, by + 2.0], 0xFFFFFF, true);
    };
    let rows = [("Radius", d(miner::DATA_RADIUS)), ("Min Y", d(miner::DATA_MIN_Y)), ("Max Y", d(miner::DATA_MAX_Y))];
    for (row, (name, value)) in rows.iter().enumerate() {
        let ry = miner::SETTING_Y + miner::SETTING_STEP * row as f32;
        host.text(batch, &format!("{name}: {value}"), [x + 8.0, y + ry + 2.0], 0x333333, false);
        for (step, delta) in miner::STEPS.iter().enumerate() {
            let label = if *delta > 0 { format!("+{delta}") } else { delta.to_string() };
            button(batch, miner::step_rect(row, step), 0xFF4A6FA5, &label);
        }
    }
    let onoff = |v: bool| if v { "On" } else { "Off" };
    let tag = usize::try_from(d(miner::DATA_TAG) - 1).ok().and_then(|t| miner::TAG_CHOICES.get(t));
    let tag_label = tag.map_or("Tag: none".to_owned(), |t| {
        format!("Tag: {}", t.split_once(':').map_or(*t, |p| p.1).chars().take(10).collect::<String>())
    });
    let running = on(miner::FLAG_RUNNING);
    let toggles = [
        (format!("Silk: {}", onoff(on(miner::FLAG_SILK))), on(miner::FLAG_SILK)),
        (format!("Eject: {}", onoff(on(miner::FLAG_EJECT))), on(miner::FLAG_EJECT)),
        (format!("Invert: {}", onoff(on(miner::FLAG_INVERSE))), on(miner::FLAG_INVERSE)),
        (tag_label, tag.is_some()),
        ((if running { "Stop" } else { "Start" }).to_owned(), running),
        ("Reset".to_owned(), false),
    ];
    for ((rect, _), (label, lit)) in miner::TOGGLES.iter().zip(toggles) {
        button(batch, *rect, if lit { 0xFF278447 } else { 0xFF6B7075 }, &label);
    }
    let to_mine = cube::read_amount(event.data, miner::DATA_TO_MINE);
    let energy = cube::read_amount(event.data, miner::DATA_ENERGY);
    let usage = cube::read_amount(event.data, miner::DATA_USAGE);
    host.text(
        batch,
        &format!("{}  To mine: {}", miner::state_name(d(miner::DATA_STATE)), to_mine),
        [x + 8.0, y + 100.0],
        0x333333,
        false,
    );
    host.text(
        batch,
        &format!("{} / {} FE  Uses {} FE/t", cube::si(energy as i64), cube::si(miner::CAPACITY as i64), usage),
        [x + 8.0, y + 108.0],
        0x333333,
        false,
    );
    host.text(batch, "Filters (items) and tag", [x + 8.0, y + miner::FILTER_Y - 10.0], 0x404040, false);
    host.text(batch, "Inventory", [x + miner::INVENTORY_X, y + miner::INVENTORY_Y - 10.0], 0x404040, false);
}

// --- the bounding blocks (Mekanism's `HandleBoundingBlock`) ---

fn parts(pos: [i32; 3]) -> Vec<[i32; 3]> {
    miner::bounding_offsets().into_iter().map(|o| generators::add(pos, o)).collect()
}

/// The miner a bounding block at `pos` belongs to.
pub(super) fn main_of(host: &dyn Host, pos: [i32; 3]) -> Option<[i32; 3]> {
    miner::bounding_offsets()
        .into_iter()
        .map(|o| [pos[0] - o[0], pos[1] - o[1], pos[2] - o[2]])
        .find(|&m| host.mod_block_entity(m).is_some_and(|e| &*e.id == miner::BLOCK))
}

/// A placement without room for the 17 bounding blocks is undone: the miner drops as an item.
pub(super) fn place(host: &mut dyn Host, pos: [i32; 3]) {
    let parts = parts(pos);
    if !parts.iter().all(|&p| generators::is_air(host, p)) {
        host.destroy_block(pos, None, false);
        return;
    }
    generators::set_parts(host, &parts, generator::BOUNDING, None);
}

/// Breaking the miner takes its bounding blocks; breaking a bounding block breaks the miner
/// (which drops, except for a player in creative mode). Whether `pos` was part of a miner.
pub(super) fn destroy(host: &mut dyn Host, pos: [i32; 3]) -> bool {
    let place = (host.dimension().to_owned(), pos);
    if host.block(pos) == generator::BOUNDING {
        let Some(main) = main_of(host, pos) else { return false };
        let rest: Vec<[i32; 3]> = parts(main).into_iter().filter(|&p| p != pos).collect();
        generators::set_parts(host, &rest, "minecraft:air", Some(generator::BOUNDING));
        RUNS.lock().unwrap().remove(&(host.dimension().to_owned(), main));
        let creative = host.acting_player().is_some_and(|p| p.game_mode == 1);
        let _ = host.destroy_block(main, None, creative);
        return true;
    }
    if host.mod_block_entity(pos).is_some_and(|e| &*e.id == miner::BLOCK) {
        generators::set_parts(host, &parts(pos), "minecraft:air", Some(generator::BOUNDING));
        RUNS.lock().unwrap().remove(&place);
        return true;
    }
    false
}
