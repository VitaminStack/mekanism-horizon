//! The host side of the Dynamic Tank and Dynamic Valve (M09): the master's
//! contents in NBT, the valves' exchange with it, the screen. Rules and values
//! are in `dynamic_tank.rs` (Mekanism's `TankMultiblockData`), the structure in
//! `multiblocks.rs`.
//!
//! The master holds the contents as an i64 (`mekanism_mb_amount`) of one fluid or
//! chemical (`mekanism_mb_fluid`); a valve exposes a small Horizon tank that it
//! trades with the master each tick, so pipes and tubes connect to valves.
//! Every member mirrors the master's menu data, so any casing opens the screen.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, Host, ModContainer, ModFluidTanks, NativeScreen, ScreenDrawEvent, Tag,
};

use super::dynamic_tank::{self as dt, Mode, Stored};
use super::multiblock::Structure;
use super::multiblocks::read_pos;
use super::{chemicals, cube, gauge, key, offset, tank, tanks};

const FLUID: &str = "mekanism_mb_fluid";
const AMOUNT: &str = "mekanism_mb_amount";
const CAPACITY: &str = "mekanism_mb_capacity";
const MODE: &str = "mekanism_mb_mode";
const MASTER: &str = "mekanism_mb_master";

fn read_long(nbt: &Compound, name: &'static str) -> i64 {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Long(v)) => *v,
        _ => 0,
    }
}

fn write_long(nbt: &mut Compound, name: &'static str, v: i64) {
    let k = key(nbt, name);
    if v == 0 {
        nbt.remove(&k);
    } else if nbt.get(&k) != Some(&Tag::Long(v)) {
        nbt.insert(k, Tag::Long(v));
    }
}

fn read_stored(nbt: &Compound) -> Stored {
    let fluid = match nbt.get(&key(nbt, FLUID)) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    Stored::new(fluid, read_long(nbt, AMOUNT))
}

fn write_stored(nbt: &mut Compound, s: &Stored) {
    write_long(nbt, AMOUNT, s.amount);
    let k = key(nbt, FLUID);
    match &s.fluid {
        Some(f) if s.amount > 0 => {
            let tag = Tag::String(f.as_str().into());
            if nbt.get(&k) != Some(&tag) {
                nbt.insert(k, tag);
            }
        }
        _ => {
            nbt.remove(&k);
        }
    }
}

fn read_mode(nbt: &Compound) -> Mode {
    match nbt.get(&key(nbt, MODE)) {
        Some(Tag::Byte(b)) => Mode::from_byte(*b as u8),
        _ => Mode::Both,
    }
}

fn write_mode(nbt: &mut Compound, m: Mode) {
    let k = key(nbt, MODE);
    if m == Mode::Both {
        nbt.remove(&k);
    } else if nbt.get(&k) != Some(&Tag::Byte(m.byte() as i8)) {
        nbt.insert(k, Tag::Byte(m.byte() as i8));
    }
}

pub(super) fn has_contents(nbt: &Compound) -> bool {
    read_stored(nbt).amount > 0
}

/// What the members of a new structure carried (their caches), added up.
pub(super) fn merge(parts: &[Compound]) -> Compound {
    let (fluid, amount) = super::multiblock::merge_stored(
        parts.iter().map(|p| {
            let s = read_stored(p);
            (s.fluid, s.amount)
        }),
        i64::MAX,
    );
    let mut out = Compound::new();
    write_stored(&mut out, &Stored::new(fluid, amount));
    if let Some(p) = parts.iter().find(|p| read_mode(p) != Mode::Both) {
        write_mode(&mut out, read_mode(p));
    }
    out
}

/// A block about to be broken hands its contents to `heir` (added to what it has).
pub(super) fn inherit(heir: &mut Compound, from: &Compound) {
    let merged = merge(&[heir.clone(), from.clone()]);
    write_stored(heir, &read_stored(&merged));
    write_mode(heir, read_mode(&merged));
}

/// Drop the contents (a member that is not the master holds none).
pub(super) fn clear(nbt: &mut Compound) {
    write_stored(nbt, &Stored::default());
    write_long(nbt, CAPACITY, 0);
}

/// After (re-)validation: capacity from the volume, contents clamped to it.
pub(super) fn validated(nbt: &mut Compound, s: &Structure) {
    let capacity = dt::capacity(s.volume());
    write_long(nbt, CAPACITY, capacity);
    let mut stored = read_stored(nbt);
    if stored.amount > capacity {
        stored.amount = capacity;
        write_stored(nbt, &stored);
    }
}

pub(super) fn menu_data(nbt: &Compound, formed: bool) -> [i32; dt::DATA_LEN] {
    dt::menu_data(&read_stored(nbt), read_long(nbt, CAPACITY), read_mode(nbt), formed)
}

/// The master's own screen data, kept current every tick.
pub(super) fn master_tick(nbt: &Compound, data: &mut Vec<i32>) {
    let want = menu_data(nbt, true).to_vec();
    if *data != want {
        *data = want;
    }
}

/// A valve's tick: trade with the master, and in `Empty` mode push into neighbours.
pub(super) fn member_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, master: [i32; 3]) {
    if &*event.entity.id != dt::VALVE || master == event.pos {
        return;
    }
    let Some(m) = host.mod_block_entity(master) else { return };
    let (mut tank, capacity, mode) = (read_stored(&m.nbt), read_long(&m.nbt, CAPACITY), read_mode(&m.nbt));
    let (bf, ba) = tanks::read_fluid(&event.entity.nbt);
    let mut buffer = Stored::new(bf, ba as i64);
    let (tank0, buffer0) = (tank.clone(), buffer.clone());
    dt::exchange(&mut buffer, &mut tank, capacity, dt::BUFFER, mode);
    if mode == Mode::Empty {
        if let Some(id) = buffer.fluid.clone() {
            let offer = dt::eject(&buffer);
            let mut left = offer;
            for dir in 0..6 {
                if left <= 0 {
                    break;
                }
                let refused = host.fluid_insert(offset(event.pos, dir), Direction::ALL[cube::opposite(dir)], &id, left);
                left = refused.clamp(0, left);
            }
            buffer = Stored::new(buffer.fluid.clone(), buffer.amount - (offer - left) as i64);
        }
    }
    if buffer != buffer0 {
        tanks::write_fluid(&mut event.entity.nbt, buffer.fluid.as_deref(), buffer.amount.clamp(0, i32::MAX as i64) as i32);
    }
    if tank != tank0 {
        host.change_mod_block_entity(master, &mut |m| write_stored(&mut m.nbt, &tank));
    }
}

/// The valve's buffer tank, reached from every side.
pub(super) fn fluids() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![dt::BUFFER],
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: vec![0],
    }
}

/// One hidden placeholder slot; the edit mode is a button.
pub(super) fn container(menu: &str) -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(menu.to_owned()),
    }
}

pub(super) fn screen() -> NativeScreen {
    tanks::screen()
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let data = event.data;
    let title = host.screen_title().unwrap_or_else(|| "Dynamic Tank".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let formed = data.get(dt::DATA_FORMED).copied().unwrap_or(0) != 0;
    if !formed {
        host.text(event.batch, "Not part of a formed structure", [x + 8.0, y + 24.0], 0xAA3333, false);
        host.text(event.batch, "Inventory", [x + 8.0, y + tank::INVENTORY_Y - 10.0], 0x404040, false);
        return;
    }
    let amount = dt::read_long(data, dt::DATA_AMOUNT);
    let capacity = dt::read_long(data, dt::DATA_CAPACITY).max(1);
    let kind = data.get(dt::DATA_KIND).copied().unwrap_or(0);
    let color = (kind as usize).checked_sub(1).and_then(chemicals::entry).map_or(0, |e| e.1);
    let [gx, gy, gw, gh] = tank::GAUGE;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], amount as f32 / capacity as f32, color);
    host.text(event.batch, dt::name(data), [x + 32.0, y + 20.0], 0x333333, false);
    host.text(event.batch, &dt::amount_text(amount), [x + 32.0, y + 31.0], 0x333333, false);
    host.text(event.batch, &format!("Capacity: {}", dt::amount_text(capacity)), [x + 32.0, y + 43.0], 0x404040, false);
    let mode = Mode::from_byte(data.get(dt::DATA_MODE).copied().unwrap_or(0) as u8);
    let [bx, by, bw, bh] = tank::MODE_BUTTON;
    host.fill(event.batch, [x + bx, y + by, bw, bh], 0xFF262A2F);
    host.fill(
        event.batch,
        [x + bx + 1.0, y + by + 1.0, bw - 2.0, bh - 2.0],
        if mode == Mode::Both { 0xFF6B7075 } else { 0xFFAF4141 },
    );
    host.text(event.batch, &format!("Valves: {}", mode.name()), [x + bx + 5.0, y + by + 4.0], 0xFFFFFF, true);
    host.text(event.batch, "Inventory", [x + 8.0, y + tank::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The edit mode button changes the master's mode.
pub(super) fn button(host: &mut dyn Host, pos: [i32; 3], nbt: &mut Compound, id: i32) {
    let forward = match id {
        dt::BUTTON_MODE_NEXT => true,
        dt::BUTTON_MODE_PREV => false,
        _ => return,
    };
    let Some(master) = read_pos(nbt, MASTER) else { return };
    if master == pos {
        let next = read_mode(nbt).next(forward);
        write_mode(nbt, next);
    } else {
        host.change_mod_block_entity(master, &mut |m| {
            let next = read_mode(&m.nbt).next(forward);
            write_mode(&mut m.nbt, next);
        });
    }
}
