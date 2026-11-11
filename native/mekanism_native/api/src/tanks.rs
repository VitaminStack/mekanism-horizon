//! The host side of the Fluid and Chemical Tanks (M06): tick, contents in NBT,
//! buckets, screen and the item that keeps the contents. Their Horizon-free
//! rules and values are in `tank.rs`, the fluid and chemical ids in `chemicals.rs`.
//!
//! Horizon's block-entity tanks (`block_entity_fluids`, API 54; mod fluid ids,
//! `mod_fluids`) hold both, a chemical as a `mekanism:chemical/<id>` fluid id;
//! `block_entity_accepts_fluid` keeps fluids and chemicals out of each other's
//! tanks. `fluid_insert`/`fluid_tanks`/`fluid_extract` (API 54) fill the tank
//! below, buckets and the debug chemical source; `right_click_block` (API 21)
//! handles water and lava buckets on fluid tanks, before the tank opens its
//! screen (`use_item_on` only hears clicks the block let through).

use horizon_client::mod_api::{
    BlockEntityDraw, BlockEntityTickEvent, Compound, Direction, Host, ItemStack, ModContainer,
    ModFluidTanks, NativeFluidRenderOptions, NativeFluidStack, NativeScreen, ScreenDrawEvent, Tag, Text,
};

use super::{chemicals, cube, gauge, key, offset, read_energy, tank};

/// Pushes `mekanism:chemical/hydrogen` into every neighbouring tank, for testing
/// chemical tanks until a machine makes chemicals (M06 next branches).
pub(super) const DEBUG_CHEMICAL_SOURCE: &str = "mekanism:debug_chemical_source";
const DEBUG_CHEMICAL: &str = "hydrogen";
/// Tank block-entity NBT besides Horizon's `fluids`: the mode byte (`tank::next_mode`).
/// Tank items carry the contents in `minecraft:custom_data`.
const TANK_MODE: &str = "mekanism_mode";
const ITEM_FLUID: &str = "mekanism_fluid";
const ITEM_AMOUNT: &str = "mekanism_amount";

/// What a tank holds, kept from its break to its drop: fluid, amount, mode.
pub(super) type Contents = (Option<String>, i32, u8);

/// Contents of a tank: Horizon's `fluids` list, first tank (`Fluid`, `Amount`).
pub(super) fn read_fluid(nbt: &Compound) -> (Option<String>, i32) {
    let Some(Tag::List(list)) = nbt.get(&key(nbt, "fluids")) else { return (None, 0) };
    let Some(Tag::Compound(tank)) = list.first() else { return (None, 0) };
    let fluid = match tank.get(&key(tank, "Fluid")) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()).filter(|s| s != "minecraft:empty"),
        _ => None,
    };
    let amount = read_energy(tank, "Amount");
    if fluid.is_none() || amount <= 0 {
        return (None, 0);
    }
    (fluid, amount)
}

/// `read_fluid`'s writer, in Horizon's layout; only on change.
pub(super) fn write_fluid(nbt: &mut Compound, fluid: Option<&str>, amount: i32) {
    let want = match fluid {
        Some(f) if amount > 0 => (Some(f.to_owned()), amount),
        _ => (None, 0),
    };
    if read_fluid(nbt) == want {
        return;
    }
    let mut tank = Compound::new();
    tank.insert(key(&tank, "Fluid"), Tag::String(want.0.as_deref().unwrap_or("minecraft:empty").into()));
    tank.insert(key(&tank, "Amount"), Tag::Int(want.1));
    nbt.insert(key(nbt, "fluids"), Tag::List(vec![Tag::Compound(tank)]));
}

fn read_byte(nbt: &Compound, name: &'static str) -> u8 {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Byte(b)) => *b as u8,
        _ => 0,
    }
}

fn write_byte(nbt: &mut Compound, name: &'static str, value: u8) {
    let k = key(nbt, name);
    if nbt.get(&k) != Some(&Tag::Byte(value as i8)) {
        nbt.insert(k, Tag::Byte(value as i8));
    }
}

fn direction(dir: usize) -> Direction {
    Direction::ALL[dir]
}

/// The side a click on `face` (a unit normal) hits, as `cube::OFFSETS` index.
fn dir_of_normal(face: [i32; 3]) -> Option<usize> {
    cube::OFFSETS.iter().position(|o| *o == face)
}

/// One tank per tank block, reached from every side.
pub(super) fn fluids(t: &tank::Tier) -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![t.capacity],
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: vec![0],
    }
}

/// One hidden placeholder slot; the mode is a button.
pub(super) fn container(t: &tank::Tier) -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(tank::menu(t.family).to_owned()),
    }
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: tank::WIDTH,
        height: tank::HEIGHT,
        slots: vec![None],
        inventory_y: tank::INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

/// One tank tick (`TileEntityFluidTank` / `TileEntityChemicalTank`): an
/// active fluid tank pushes its transfer rate into the block below; a
/// chemical tank dumps by its mode; a creative tank stays full.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &tank::Tier) {
    let pos = event.pos;
    let (fluid, mut amount) = read_fluid(&event.entity.nbt);
    let mode = read_byte(&event.entity.nbt, TANK_MODE) % tank::modes(t.family);
    if t.creative && fluid.is_some() {
        amount = t.capacity;
    }
    if let Some(id) = fluid.as_deref() {
        match t.family {
            tank::Family::Fluid if mode == 1 && amount > 0 => {
                // Into the block below through its top; what it refused stays here.
                let offer = t.rate.min(amount);
                let left = host.fluid_insert(offset(pos, cube::DOWN), direction(cube::UP), id, offer);
                amount -= (offer - left).clamp(0, offer);
            }
            tank::Family::Chemical => amount -= tank::dump(t, mode, amount),
            _ => {}
        }
    }
    if t.creative && fluid.is_some() {
        amount = t.capacity;
    }
    let e = &mut *event.entity;
    write_fluid(&mut e.nbt, fluid.as_deref(), amount);
    write_byte(&mut e.nbt, TANK_MODE, mode);
    let data = tank::menu_data(amount, fluid.as_deref(), mode);
    if e.data.as_slice() != data.as_slice() {
        e.data = data.to_vec();
    }
    // Mekanism's fluid tank shows its valve lit while active.
    if t.family == tank::Family::Fluid {
        let want = if mode == 1 { "true" } else { "false" };
        if host.block_property(pos, "active").as_deref() != Some(want) {
            host.set_block_property(pos, "active", want);
        }
    }
}

/// The debug chemical source fills every neighbouring chemical tank.
pub(super) fn debug_tick(host: &mut dyn Host, pos: [i32; 3]) {
    let id = chemicals::chemical(DEBUG_CHEMICAL);
    for dir in 0..6 {
        host.fluid_insert(offset(pos, dir), direction(cube::opposite(dir)), &id, 1_000);
    }
}

/// Water and lava buckets fill and empty fluid tanks (Mekanism's
/// `BlockFluidTank` bucket handling); mod fluids have no buckets in Horizon.
/// `item` is the main hand's. Returns whether the click was used.
pub(super) fn use_bucket(host: &mut dyn Host, pos: [i32; 3], face: [i32; 3], item: &str, t: &tank::Tier) -> bool {
    if t.family != tank::Family::Fluid {
        return false;
    }
    let Some(dir) = dir_of_normal(face) else { return false };
    let face = direction(dir);
    let full = match item {
        "minecraft:water_bucket" => Some(chemicals::WATER),
        "minecraft:lava_bucket" => Some(chemicals::LAVA),
        _ => None,
    };
    if let Some(fluid) = full {
        // Whole bucket or nothing: check the room first.
        let room = host
            .fluid_tanks(host.dimension(), pos, face)
            .and_then(|tanks| tanks.into_iter().next())
            .map_or(0, |tk| match tk.fluid.as_deref() {
                None => tk.capacity,
                Some(f) if f == fluid => tk.capacity - tk.amount,
                _ => 0,
            });
        if room >= 1_000 && host.take_items(item, 1) == 1 {
            let left = host.fluid_insert(pos, face, fluid, 1_000);
            debug_assert_eq!(left, 0);
            host.give_items("minecraft:bucket", 1);
        }
        return true;
    }
    if item == "minecraft:bucket" {
        let held = host
            .fluid_tanks(host.dimension(), pos, face)
            .and_then(|tanks| tanks.into_iter().next())
            .and_then(|tk| Some((tk.fluid?, tk.amount)));
        let bucket = match held {
            Some((f, n)) if n >= 1_000 && f == chemicals::WATER => "minecraft:water_bucket",
            Some((f, n)) if n >= 1_000 && f == chemicals::LAVA => "minecraft:lava_bucket",
            _ => return false,
        };
        if host.take_items(item, 1) == 1 {
            host.fluid_extract(host.dimension(), pos, face, 1_000);
            host.give_items(bucket, 1);
        }
        return true;
    }
    false
}

/// What a fluid tank sends its clients: its contents (Horizon's `fluids`),
/// for `draw_contents`. Chemical tanks are opaque and send nothing.
pub(super) fn update_tag(nbt: &Compound, t: &tank::Tier) -> Option<Compound> {
    if t.family != tank::Family::Fluid {
        return None;
    }
    let (fluid, amount) = read_fluid(nbt);
    let mut tag = Compound::new();
    write_fluid(&mut tag, fluid.as_deref(), amount);
    if !tag.contains_key(&key(&tag, "fluids")) {
        // An empty tank still sends its (empty) list, so the client's copy empties too.
        tag.insert(key(&tag, "fluids"), Tag::List(Vec::new()));
    }
    Some(tag)
}

/// The glass of `fluid_tank_base.json` spans 2..14 px on every axis.
const INNER_MIN: f32 = 2.02 / 16.0;
const INNER_MAX: f32 = 13.98 / 16.0;

/// The fluid seen through a fluid tank's glass, as high as it is full
/// (Mekanism's `RenderFluidTank`). Horizon's fluid box draws water and lava;
/// other fluids are drawn as water in their own colour.
pub(super) fn draw_contents(host: &dyn Host, nbt: &Compound, draw: &mut BlockEntityDraw, t: &tank::Tier) {
    let (Some(fluid), amount) = read_fluid(nbt) else { return };
    let fill = if t.creative { 1.0 } else { (amount as f32 / t.capacity as f32).clamp(0.0, 1.0) };
    let top = INNER_MIN + (INNER_MAX - INNER_MIN) * fill;
    let bounds = [INNER_MIN, INNER_MIN, INNER_MIN, INNER_MAX, top, INNER_MAX];
    let own = fluid == chemicals::WATER || fluid == chemicals::LAVA;
    let argb = (!own).then(|| chemicals::index(&fluid).and_then(chemicals::entry).map_or(0xFFFF_FFFF, |e| e.1));
    let stack = NativeFluidStack {
        fluid: if own { fluid } else { chemicals::WATER.to_owned() },
        amount,
        components: Compound::new(),
    };
    let options = NativeFluidRenderOptions { argb, render_bottom: true, uv_shrink_ratio: 4.0 / 2048.0 };
    host.draw_fluid_box(draw, &stack, bounds, options);
}

/// A tank's screen: a gauge in the contents' colour, the amount and the mode button.
pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, t: &tank::Tier) {
    let [x, y] = event.origin;
    let data = event.data;
    let title = host.screen_title().unwrap_or_else(|| "Tank".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let amount = cube::read_amount(data, tank::DATA_AMOUNT).max(0);
    let kind = data.get(tank::DATA_KIND).copied().unwrap_or(0);
    let color = (kind as usize).checked_sub(1).and_then(chemicals::entry).map_or(0, |e| e.1);
    let fill = if t.creative && amount > 0 { 1.0 } else { amount as f32 / t.capacity as f32 };
    let [gx, gy, gw, gh] = tank::GAUGE;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], fill, color);
    // Name and amount on lines of their own: together they overflow the panel.
    let (name, amount_text) = tank::contents(t, data);
    host.text(event.batch, name, [x + 32.0, y + 20.0], 0x333333, false);
    host.text(event.batch, &amount_text, [x + 32.0, y + 31.0], 0x333333, false);
    let rate = if t.creative { "Infinite".to_owned() } else { format!("{} mB/t", t.rate) };
    host.text(event.batch, &format!("Transfer: {rate}"), [x + 32.0, y + 43.0], 0x404040, false);
    let mode = data.get(tank::DATA_MODE).copied().unwrap_or(0) as u8;
    let [bx, by, bw, bh] = tank::MODE_BUTTON;
    let on = mode != 0;
    host.fill(event.batch, [x + bx, y + by, bw, bh], 0xFF262A2F);
    host.fill(event.batch, [x + bx + 1.0, y + by + 1.0, bw - 2.0, bh - 2.0], if on { 0xFFAF4141 } else { 0xFF6B7075 });
    host.text(event.batch, tank::mode_name(t.family, mode), [x + bx + 5.0, y + by + 4.0], 0xFFFFFF, true);
    host.text(event.batch, "Inventory", [x + 8.0, y + tank::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The mode button cycles the mode; returns the menu data to show at once.
pub(super) fn button(t: &tank::Tier, nbt: &mut Compound, id: i32) -> Option<Vec<i32>> {
    let forward = match id {
        tank::BUTTON_MODE_NEXT => true,
        tank::BUTTON_MODE_PREV => false,
        _ => return None,
    };
    let mode = tank::next_mode(t.family, read_byte(nbt, TANK_MODE), forward);
    write_byte(nbt, TANK_MODE, mode);
    let (fluid, amount) = read_fluid(nbt);
    Some(tank::menu_data(amount, fluid.as_deref(), mode).to_vec())
}

/// What a tank about to break holds, for its drop.
pub(super) fn contents(nbt: &Compound) -> Contents {
    let (fluid, amount) = read_fluid(nbt);
    (fluid, amount, read_byte(nbt, TANK_MODE))
}

/// Mekanism's loot table copies `mekanism:fluids`; here the contents and mode go
/// into the item's custom data (creative tanks refill themselves).
pub(super) fn fill_drops(host: &dyn Host, drops: &mut [ItemStack], (fluid, amount, mode): Contents) {
    let Some(id) = fluid.as_deref().filter(|_| amount > 0) else { return };
    for drop in drops.iter_mut().filter(|d| tank::tier_of(&d.item).is_some()) {
        let snbt = format!(
            "{{id:\"{}\",count:{},components:{{\"minecraft:custom_data\":{{{ITEM_FLUID}:\"{id}\",{ITEM_AMOUNT}:{amount},{TANK_MODE}:{mode}b}}}}}}",
            drop.item, drop.count
        );
        if let Some(stack) = host.item_from_snbt(&snbt) {
            *drop = stack;
        }
    }
}

/// Placing a tank item restores the contents and mode it carries.
pub(super) fn place(host: &mut dyn Host, pos: [i32; 3], stack: &ItemStack, t: &tank::Tier) {
    let Some(data) = stack.custom_data() else { return };
    let amount = read_energy(data, ITEM_AMOUNT).clamp(0, t.capacity);
    let fluid = match data.get(&key(data, ITEM_FLUID)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    let mode = read_byte(data, TANK_MODE);
    host.change_mod_block_entity(pos, &mut |m| {
        if tank::tier_of(&m.id).is_none() {
            return;
        }
        write_fluid(&mut m.nbt, fluid.as_deref().filter(|f| tank::accepts(t.family, f)), amount);
        write_byte(&mut m.nbt, TANK_MODE, mode);
    });
}

/// A tank item's contents.
pub(super) fn tooltip(stack: &ItemStack, t: &tank::Tier) -> (Text, u32) {
    let data = stack.custom_data();
    let amount = data.map_or(0, |d| read_energy(d, ITEM_AMOUNT));
    let name = data
        .and_then(|d| match d.get(&key(d, ITEM_FLUID)) {
            Some(Tag::String(s)) => Some(chemicals::name(&s.to_string_lossy())),
            _ => None,
        })
        .filter(|_| amount > 0)
        .unwrap_or_else(|| "Empty".to_owned());
    let cap = if t.creative { "Infinite".to_owned() } else { format!("{amount} / {} mB", t.capacity) };
    (Text::literal(format!("{name}: {cap}")), 0xFF_AAAAAA)
}
