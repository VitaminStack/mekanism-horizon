//! The host side of the Bins (M08): tick, hoppers, clicks, the item that keeps
//! the contents and the item and count drawn on the front. The rules and values
//! are Horizon-free in `bin.rs`; read its header for what is and is not ported.
//!
//! The count lives in the block-entity NBT (`mekanism_bin_*`); the container's
//! two slots only serve hoppers. Every operation pulls the output window back
//! into the count, does its work and refills the window (`open` / `close`), so
//! the count and the window never disagree. A broken bin remembers its contents
//! from `destroy_block` to `loot_drops`, which put them on the item's
//! `minecraft:custom_data`; placing the item restores them.

use std::collections::HashMap;
use std::sync::Mutex;

use horizon_client::mod_api::{
    BlockEntityDraw, BlockEntityTickEvent, Compound, Direction, DisplayContext, Host, ItemStack, ModBlockEntity,
    ModContainer, Tag, Text,
};

use crate::bin::{self, Contents, Tier};
use crate::{cube, facing, key, offset, read_energy, write_int};

const ITEM: &str = "mekanism_bin_item";
const COUNT: &str = "mekanism_bin_count";
const LOCK: &str = "mekanism_bin_lock";
const CLICK: &str = "minecraft:ui.button.click";
/// `SoundSource.PLAYERS` and `BLOCKS` ordinals.
const SOUND_PLAYERS: usize = 7;
const SOUND_BLOCKS: usize = 4;

/// What a bin about to break holds, from its break to its drop.
static BROKEN: Mutex<Option<HashMap<(String, [i32; 3]), Contents>>> = Mutex::new(None);
/// Ticks until each bin ejects next (not saved).
static EJECT: Mutex<Option<HashMap<(String, [i32; 3]), u8>>> = Mutex::new(None);

fn with_map<K: Eq + std::hash::Hash, V, R>(
    map: &Mutex<Option<HashMap<K, V>>>,
    f: impl FnOnce(&mut HashMap<K, V>) -> R,
) -> R {
    f(map.lock().unwrap().get_or_insert_with(HashMap::new))
}

fn read_string(nbt: &Compound, name: &'static str) -> Option<String> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()).filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn read_byte(nbt: &Compound, name: &'static str) -> bool {
    matches!(nbt.get(&key(nbt, name)), Some(Tag::Byte(b)) if *b != 0)
}

/// The contents kept in `nbt` (the block entity's, or a bin item's custom data).
fn read(nbt: &Compound, t: &Tier) -> Contents {
    let mut c = Contents {
        item: read_string(nbt, ITEM),
        count: read_energy(nbt, COUNT).clamp(0, t.capacity),
        locked: read_byte(nbt, LOCK),
    };
    if t.creative && c.item.is_some() {
        c.count = t.capacity;
    }
    c
}

/// `read`'s writer; each field only on change, as a write marks the entity changed.
fn write(nbt: &mut Compound, c: &Contents) {
    let k = key(nbt, ITEM);
    match &c.item {
        Some(item) if nbt.get(&k) != Some(&Tag::String(item.as_str().into())) => {
            nbt.insert(k, Tag::String(item.as_str().into()));
        }
        None if nbt.contains_key(&k) => {
            nbt.remove(&k);
        }
        _ => {}
    }
    write_int(nbt, COUNT, c.count);
    let k = key(nbt, LOCK);
    if read_byte(nbt, LOCK) != c.locked {
        nbt.insert(k, Tag::Byte(c.locked as i8));
    }
}

/// Both slots exist, and the window's stack goes back into the count.
fn open(e: &mut ModBlockEntity, t: &Tier) -> Contents {
    e.items.resize(bin::SLOTS, None);
    let mut c = read(&e.nbt, t);
    if let Some(w) = e.items[bin::WINDOW].take() {
        if c.item.is_none() {
            c.item = Some(w.item.to_string());
        }
        if !t.creative && c.item.as_deref() == Some(&*w.item) {
            c.count = (c.count + w.count as i32).min(t.capacity);
        }
    }
    // Emptied by a hopper since the last tick: an unlocked bin forgets its type.
    if c.count <= 0 && !c.locked {
        c.item = None;
    }
    c
}

/// The count refills the window to one stack and everything is saved.
fn close(e: &mut ModBlockEntity, t: &Tier, mut c: Contents, max_stack: u32) {
    if let Some(item) = c.item.clone() {
        let n = bin::refill(0, max_stack);
        let taken = c.extract(t, n);
        if taken > 0 {
            e.items[bin::WINDOW] = Some(ItemStack::new(&item, taken));
            // The window keeps the type, the saved NBT too (the client draws it from there).
            c.item = Some(item);
        }
    }
    write(&mut e.nbt, &c);
}

fn max_stack(host: &dyn Host, item: &str) -> u32 {
    host.max_stack_size(item).unwrap_or(64).max(1)
}

/// The stack size of the bin's item, for refilling the window.
fn stack_size_of(host: &dyn Host, e: &ModBlockEntity, t: &Tier) -> u32 {
    read(&e.nbt, t)
        .item
        .or_else(|| e.items.get(bin::WINDOW).and_then(|s| s.as_ref()).map(|s| s.item.to_string()))
        .map_or(64, |item| max_stack(host, &item))
}

/// The stack size of the item the bin at `pos` keeps (64 when it is empty).
fn size_at(host: &dyn Host, pos: [i32; 3], t: &Tier) -> u32 {
    host.mod_block_entity(pos).map_or(64, |e| stack_size_of(host, &e, t))
}

/// Hoppers reach both slots from every side: they fill the input and take the window.
pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: bin::SLOTS,
        faces: std::array::from_fn(|_| vec![bin::WINDOW, bin::INPUT]),
        inputs: vec![bin::INPUT],
        outputs: vec![bin::WINDOW],
        menu: None,
    }
}

/// A hopper puts only an item the bin keeps, plain, into the input slot.
pub(super) fn accepts(t: &Tier, e: &ModBlockEntity, slot: usize, stack: &ItemStack) -> bool {
    slot == bin::INPUT
        && bin::storable(stack.damage, stack.extra.is_some())
        && open_view(e, t).accepts(&stack.item)
}

/// What the bin holds including the window, which keeps the type while the count is empty.
fn open_view(e: &ModBlockEntity, t: &Tier) -> Contents {
    let mut c = read(&e.nbt, t);
    if c.item.is_none() {
        c.item = e.items.get(bin::WINDOW).and_then(|s| s.as_ref()).map(|s| s.item.to_string());
    }
    c
}

/// One bin tick: the input goes into the count, the window is refilled and
/// an active bin pushes the window into the block below every half second.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &Tier) {
    let pos = event.pos;
    let size = stack_size_of(host, event.entity, t);
    let e = &mut *event.entity;
    let mut c = open(e, t);
    if let Some(stack) = e.items[bin::INPUT].take() {
        if bin::storable(stack.damage, stack.extra.is_some()) && c.accepts(&stack.item) {
            let moved = c.insert(t, &stack.item, stack.count);
            let used = if t.creative { stack.count } else { moved };
            e.items[bin::INPUT] = crate::shrink(&stack, used);
        } else {
            e.items[bin::INPUT] = Some(stack);
        }
    }
    close(e, t, c, size);

    let place = (host.dimension().to_owned(), pos);
    let active = host.block_property(pos, "active").as_deref() == Some("true");
    let due = with_map(&EJECT, |m| {
        if !active {
            m.remove(&place);
            return false;
        }
        let left = m.entry(place).or_insert(0);
        *left = left.saturating_sub(1);
        if *left == 0 {
            *left = bin::EJECT_TICKS;
            true
        } else {
            false
        }
    });
    if due && let Some(stack) = e.items[bin::WINDOW].take() {
        let rest = host.insert_item(offset(pos, cube::DOWN), Direction::Up, stack);
        e.items[bin::WINDOW] = rest.filter(|s| s.count > 0);
    }
}

/// Whether `face` (a unit normal) is the bin's front.
fn is_front(host: &dyn Host, pos: [i32; 3], face: [i32; 3]) -> bool {
    cube::OFFSETS[facing(host, pos)] == face
}

fn click_sound(host: &mut dyn Host, pos: [i32; 3]) {
    let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
    host.play_sound(CLICK, at, SOUND_BLOCKS, 0.3, 1.0);
}

fn say(host: &mut dyn Host, line: String) {
    if let Some(p) = host.acting_player() {
        host.action_bar(p.uuid, Text::literal(line));
    }
}

/// Right click on a bin (`BlockBin.useItemOn`): the held stack goes in at the
/// front; sneaking with an empty hand locks the type there, and switches the
/// auto-eject on or off on the other sides. Always takes the click: bins have no screen.
pub(super) fn right_click(host: &mut dyn Host, pos: [i32; 3], face: [i32; 3], t: &'static Tier) {
    let front = is_front(host, pos, face);
    let held = host.held();
    let bin_size = size_at(host, pos, t);
    let sneaking = host.sneaking();
    if sneaking && held.is_none() {
        if front {
            let mut locked = None;
            host.change_mod_block_entity(pos, &mut |m| {
                let mut c = open(m, t);
                if c.toggle_lock(t) {
                    locked = Some((c.locked, c.item.clone()));
                }
                close(m, t, c, bin_size);
            });
            if let Some((now, item)) = locked {
                click_sound(host, pos);
                let name = item.map_or_else(String::new, |i| bin::pretty_name(&i));
                say(host, format!("Bin {}: {name}", if now { "locked" } else { "unlocked" }));
            }
        } else {
            let want = if host.block_property(pos, "active").as_deref() == Some("true") { "false" } else { "true" };
            host.set_block_property(pos, "active", want);
            click_sound(host, pos);
        }
        return;
    }
    let Some((item, count, plain)) = held else { return };
    if !front || !plain {
        return;
    }
    let size = max_stack(host, &item);
    // How much the bin takes, before the stack leaves the hand.
    let mut room = 0;
    host.change_mod_block_entity(pos, &mut |m| {
        let mut c = open(m, t);
        let mut probe = c.clone();
        room = if t.creative { count } else { probe.insert(t, &item, count) };
        if t.creative && c.accepts(&item) {
            c.insert(t, &item, 1);
        }
        close(m, t, c, size);
    });
    if room == 0 {
        return;
    }
    let taken = if t.creative { 0 } else { host.take_items(&item, room) };
    if taken == 0 {
        return;
    }
    host.change_mod_block_entity(pos, &mut |m| {
        let mut c = open(m, t);
        c.insert(t, &item, taken);
        close(m, t, c, size);
    });
    let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
    host.play_sound("minecraft:entity.item.pickup", at, SOUND_PLAYERS, 0.2, 1.4);
}

/// Left click on the front takes one item, a stack when sneaking
/// (`BlockBin.attack`); the block is still attacked as usual.
pub(super) fn left_click(host: &mut dyn Host, pos: [i32; 3], face: [i32; 3], t: &'static Tier) {
    if !is_front(host, pos, face) {
        return;
    }
    let size = size_at(host, pos, t);
    let mut item = None;
    host.change_mod_block_entity(pos, &mut |m| {
        let c = open(m, t);
        item = c.item.clone();
        close(m, t, c, size);
    });
    let Some(item) = item else { return };
    let want = bin::click_amount(host.sneaking(), size);
    let given = host.give_items(&item, want);
    if given == 0 {
        return;
    }
    host.change_mod_block_entity(pos, &mut |m| {
        let mut c = open(m, t);
        c.extract(t, given);
        close(m, t, c, size);
    });
}

/// What a bin about to break holds, remembered for its drop; the window is
/// emptied so the container's own drop does not repeat it.
pub(super) fn broken(host: &mut dyn Host, pos: [i32; 3], t: &'static Tier) {
    let mut held = Contents::default();
    host.change_mod_block_entity(pos, &mut |m| {
        held = open(m, t);
        write(&mut m.nbt, &held);
    });
    let place = (host.dimension().to_owned(), pos);
    with_map(&BROKEN, |m| m.insert(place, held));
    with_map(&EJECT, |m| m.remove(&(host.dimension().to_owned(), pos)));
}

/// A broken bin's item keeps its contents (Mekanism's loot table copies the slot).
pub(super) fn fill_drops(host: &dyn Host, dimension: &str, pos: [i32; 3], drops: &mut [ItemStack]) {
    let Some(c) = with_map(&BROKEN, |m| m.remove(&(dimension.to_owned(), pos))) else { return };
    let Some(item) = c.item.as_deref() else { return };
    for drop in drops.iter_mut().filter(|d| bin::tier_of(&d.item).is_some()) {
        let snbt = format!(
            "{{id:\"{}\",count:{},components:{{\"minecraft:custom_data\":{{{ITEM}:\"{item}\",{COUNT}:{},{LOCK}:{}b}}}}}}",
            drop.item, drop.count, c.count, c.locked as i32
        );
        if let Some(stack) = host.item_from_snbt(&snbt) {
            *drop = stack;
        }
    }
}

/// Placing a bin item restores what it carries.
pub(super) fn place(host: &mut dyn Host, dimension: &str, pos: [i32; 3], stack: &ItemStack, t: &'static Tier) {
    with_map(&BROKEN, |m| m.remove(&(dimension.to_owned(), pos)));
    let Some(data) = stack.custom_data() else { return };
    let c = read(data, t);
    if c.item.is_none() {
        return;
    }
    let size = c.item.as_deref().map_or(64, |i| max_stack(host, i));
    host.change_mod_block_entity(pos, &mut |m| {
        if bin::tier_of(&m.id).is_none() {
            return;
        }
        m.items.resize(bin::SLOTS, None);
        close(m, t, c.clone(), size);
    });
}

/// A bin item's contents, for its tooltip.
pub(super) fn tooltip(stack: &ItemStack, t: &Tier) -> (Text, u32) {
    let c = stack.custom_data().map_or_else(Contents::default, |d| read(d, t));
    let cap = if t.creative { "Infinite".to_owned() } else { format!("{} items", t.capacity) };
    (Text::literal(format!("{} (capacity {cap})", bin::label(t, &c, 0))), 0xFF_AAAAAA)
}

/// What a bin sends its clients: the item, the total and the lock, for `draw_front`.
pub(super) fn update_tag(e: &ModBlockEntity, t: &Tier) -> Compound {
    let mut c = read(&e.nbt, t);
    if let (Some(w), false) = (e.items.get(bin::WINDOW).and_then(|s| s.as_ref()), t.creative) {
        c.count = (c.count + w.count as i32).min(t.capacity);
    }
    let mut tag = Compound::new();
    write(&mut tag, &c);
    tag
}

/// Item and count on the front, as Mekanism's `RenderBin` shows them: the item
/// above, its count below. Kept until the block entity changes.
pub(super) fn draw_front(host: &dyn Host, nbt: &Compound, pos: [i32; 3], draw: &mut BlockEntityDraw, t: &Tier) {
    let c = read(nbt, t);
    let Some(item) = c.item.as_deref() else { return };
    draw.push();
    // A frame whose +z points out of the front, the way an item frame's and a sign's do.
    draw.rotate_about([0.0, 1.0, 0.0], crate::generators::facing_degrees(facing(host, pos)) + 180.0, [0.5, 0.0, 0.5]);
    draw.push();
    draw.translate([0.5, 0.62, 1.02]);
    draw.scale([0.6, 0.6, 0.02]);
    host.draw_item(draw, &ItemStack::new(item, 1), DisplayContext::Fixed, None, 0);
    draw.pop();
    draw.push();
    let shown = bin::short_count(bin::front_count(t, c.count));
    let text = Text::literal(shown.clone());
    draw.translate([0.5, 0.31, 1.03]);
    draw.scale([1.0 / 48.0, -1.0 / 48.0, 1.0 / 48.0]);
    // Centred by an estimate of six font pixels a glyph.
    let width = shown.chars().count() as f32 * 6.0;
    host.draw_text(draw, &text, [-width / 2.0, 0.0], 0xFFFFFF, false, 0x000000);
    draw.pop();
    draw.pop();
    draw.keep();
}
