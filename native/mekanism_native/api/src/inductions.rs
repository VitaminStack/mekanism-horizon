//! The host side of the Induction Matrix (M09): the master's energy in NBT, the
//! ports' exchange with it, the screen. Rules and values are in `induction.rs`
//! (Mekanism's `MatrixEnergyContainer`), the structure in `multiblocks.rs`.
//!
//! The master holds the energy as an i64 (`mekanism_mb_energy`); a port keeps a
//! Horizon energy buffer (`energy`, up to `induction::PORT_BUFFER`) that it trades
//! with the master each tick: an input port empties its buffer into the matrix, an
//! output port refills its buffer from it and offers the energy to the Mekanism
//! blocks beside it (`give`); cables and other mods' blocks use the buffer itself.

use horizon_client::mod_api::{BlockEntityTickEvent, Compound, Host, ModEnergyBuffer, ScreenDrawEvent, Tag};

use super::dynamic_tanks::{read_long, write_long};
use super::induction::{self as ind, Totals};
use super::multiblock::Structure;
use super::multiblocks::read_pos;
use super::{ENERGY, give, key, read_energy, tank, write_int};

const STORED: &str = "mekanism_mb_energy";
const CAPACITY: &str = "mekanism_mb_capacity";
const TRANSFER: &str = "mekanism_mb_transfer";
const CELLS: &str = "mekanism_mb_cells";
const PROVIDERS: &str = "mekanism_mb_providers";
const PORTS: &str = "mekanism_mb_ports";
/// A port's own mode: 1 output, 0 input.
const PORT_OUTPUT: &str = "mekanism_mb_port_output";
const MASTER: &str = "mekanism_mb_master";

fn read_count(nbt: &Compound, name: &'static str) -> i32 {
    read_energy(nbt, name)
}

pub(super) fn has_contents(nbt: &Compound) -> bool {
    read_long(nbt, STORED) > 0
}

pub(super) fn merge(parts: &[Compound]) -> Compound {
    let total = parts.iter().fold(0i64, |a, p| a.saturating_add(read_long(p, STORED)));
    let mut out = Compound::new();
    write_long(&mut out, STORED, total);
    out
}

pub(super) fn inherit(heir: &mut Compound, from: &Compound) {
    let total = read_long(heir, STORED).saturating_add(read_long(from, STORED));
    write_long(heir, STORED, total);
}

pub(super) fn clear(nbt: &mut Compound) {
    for name in [STORED, CAPACITY, TRANSFER] {
        write_long(nbt, name, 0);
    }
    for name in [CELLS, PROVIDERS, PORTS] {
        let k = key(nbt, name);
        nbt.remove(&k);
    }
}

/// After (re-)validation: capacity, transfer cap and counts from the interior.
pub(super) fn validated(nbt: &mut Compound, s: &Structure, t: &Totals) {
    write_long(nbt, CAPACITY, t.capacity);
    write_long(nbt, TRANSFER, t.transfer);
    write_int(nbt, CELLS, t.cells as i32);
    write_int(nbt, PROVIDERS, t.providers as i32);
    write_int(nbt, PORTS, s.valves.len() as i32);
    let stored = read_long(nbt, STORED);
    if stored > t.capacity {
        write_long(nbt, STORED, t.capacity);
    }
}

fn totals(nbt: &Compound) -> Totals {
    Totals {
        cells: read_count(nbt, CELLS).max(0) as u32,
        providers: read_count(nbt, PROVIDERS).max(0) as u32,
        capacity: read_long(nbt, CAPACITY),
        transfer: read_long(nbt, TRANSFER),
    }
}

fn output(nbt: &Compound) -> bool {
    matches!(nbt.get(&key(nbt, PORT_OUTPUT)), Some(Tag::Byte(1)))
}

pub(super) fn menu_data(nbt: &Compound, formed: bool, port: Option<&Compound>) -> [i32; ind::DATA_LEN] {
    ind::menu_data(read_long(nbt, STORED), &totals(nbt), port.is_some_and(output), formed)
}

/// The master's own screen data, kept current every tick.
pub(super) fn master_tick(nbt: &Compound, data: &mut Vec<i32>, is_port: bool) {
    let want = menu_data(nbt, true, is_port.then_some(nbt)).to_vec();
    if *data != want {
        *data = want;
    }
}

/// A port's tick: trade with the master.
pub(super) fn member_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, master: [i32; 3]) {
    if &*event.entity.id != ind::PORT || master == event.pos {
        return;
    }
    let Some(m) = host.mod_block_entity(master) else { return };
    let (stored, t) = (read_long(&m.nbt, STORED), totals(&m.nbt));
    let share = ind::share(t.transfer, read_count(&m.nbt, PORTS) as i64);
    let buffer = read_energy(&event.entity.nbt, ENERGY).max(0);
    let out = output(&event.entity.nbt);
    let (new_stored, mut new_buffer) = if out {
        ind::fill_out(stored, buffer, share)
    } else {
        ind::take_in(stored, t.capacity, buffer, share)
    };
    if out {
        // Mekanism's own blocks beside the port take what they accept.
        for dir in 0..6 {
            if new_buffer <= 0 {
                break;
            }
            new_buffer -= give(host, event.pos, dir, new_buffer, new_buffer).clamp(0, new_buffer);
        }
    }
    write_int(&mut event.entity.nbt, ENERGY, new_buffer);
    if new_stored != stored {
        host.change_mod_block_entity(master, &mut |m| write_long(&mut m.nbt, STORED, new_stored));
    }
}

/// A port's buffer, reached from every side.
pub(super) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ind::PORT_BUFFER,
        max_insert: ind::PORT_BUFFER,
        max_extract: ind::PORT_BUFFER,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let data = event.data;
    let title = host.screen_title().unwrap_or_else(|| "Induction Matrix".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let inventory = [x + 8.0, y + tank::INVENTORY_Y - 10.0];
    if data.get(ind::DATA_FORMED).copied().unwrap_or(0) == 0 {
        host.text(event.batch, "Not part of a formed structure", [x + 8.0, y + 24.0], 0xAA3333, false);
        host.text(event.batch, "Inventory", inventory, 0x404040, false);
        return;
    }
    let energy = super::dynamic_tank::read_long(data, ind::DATA_ENERGY);
    let capacity = super::dynamic_tank::read_long(data, ind::DATA_CAPACITY);
    let transfer = super::dynamic_tank::read_long(data, ind::DATA_TRANSFER);
    host.text(event.batch, &format!("Energy: {} / {}", ind::energy_text(energy), ind::energy_text(capacity)), [x + 8.0, y + 20.0], 0x333333, false);
    let fill = if capacity > 0 { energy as f32 / capacity as f32 } else { 0.0 };
    host.fill(event.batch, [x + 8.0, y + 33.0, 160.0, 10.0], 0xFF242A2F);
    host.fill(event.batch, [x + 9.0, y + 34.0, 158.0 * fill.clamp(0.0, 1.0), 8.0], 0xFF50C878);
    host.text(event.batch, &format!("Transfer: {} per tick", ind::energy_text(transfer)), [x + 8.0, y + 46.0], 0x404040, false);
    let (cells, providers) = (data.get(ind::DATA_CELLS).copied().unwrap_or(0), data.get(ind::DATA_PROVIDERS).copied().unwrap_or(0));
    host.text(event.batch, &format!("Cells: {cells}   Providers: {providers}"), [x + 8.0, y + 58.0 - 2.0], 0x404040, false);
    if event.block == ind::PORT {
        let [bx, by, bw, bh] = ind::MODE_BUTTON;
        let out = data.get(ind::DATA_MODE).copied().unwrap_or(0) != 0;
        host.fill(event.batch, [x + bx, y + by + 12.0, bw, bh], 0xFF262A2F);
        host.fill(event.batch, [x + bx + 1.0, y + by + 13.0, bw - 2.0, bh - 2.0], if out { 0xFFAF4141 } else { 0xFF3E7BB6 });
        host.text(event.batch, &format!("Port: {}", ind::mode_name(out)), [x + bx + 5.0, y + by + 16.0], 0xFFFFFF, true);
    }
    host.text(event.batch, "Inventory", inventory, 0x404040, false);
}

/// The mode button of a port screen (the casing has none).
pub(super) fn button_at(block: &str, at: [f32; 2]) -> Option<i32> {
    if block != ind::PORT {
        return None;
    }
    let [bx, by, bw, bh] = ind::MODE_BUTTON;
    super::machine::inside([bx, by + 12.0, bw, bh], at).then_some(ind::BUTTON_MODE)
}

/// The port toggles between input and output; its menu data updates at once.
pub(super) fn button(nbt: &mut Compound, data: &mut Vec<i32>, id: i32) {
    if id != ind::BUTTON_MODE || read_pos(nbt, MASTER).is_none() {
        return;
    }
    let k = key(nbt, PORT_OUTPUT);
    let next = !output(nbt);
    nbt.insert(k, Tag::Byte(next as i8));
    if let Some(slot) = data.get_mut(ind::DATA_MODE) {
        *slot = next as i32;
    }
}
