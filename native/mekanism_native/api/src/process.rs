//! The host side of every processing machine in `machine::MACHINES`: its
//! tick, container, screen and drawing, one implementation for all of them.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Host, ItemStack, ModContainer, NativeScreen, ScreenDrawEvent,
};

use super::machine::{self, Buffer, Machine, Tank};
use super::recipe::{ChemicalInput, Recipes};
use super::{ENERGY, PROGRESS, facing, gauge, infuser, merged, read_energy, shrink, write_int};

/// Block entity NBT of a machine with a chemical buffer: `Buffer::types`
/// index + 1 (0 while empty) and the amount in mB. The names date from the
/// Metallurgic Infuser, the first such machine; saved worlds keep them.
pub const BUFFER_TYPE: &str = "mekanism_infuse_type";
pub const BUFFER: &str = "mekanism_infuse";

pub fn read_tank(nbt: &Compound, b: &Buffer) -> Tank {
    let kind = read_energy(nbt, BUFFER_TYPE);
    let amount = read_energy(nbt, BUFFER).clamp(0, b.max);
    if kind < 1 || kind as usize > b.types.len() || amount == 0 {
        return Tank::default();
    }
    Tank { kind: kind as usize - 1, amount }
}

/// The buffer type a recipe's chemical is, if the machine's buffer takes it.
fn chemical_kind(m: &Machine, c: &ChemicalInput) -> Option<usize> {
    m.buffer?.type_index(&c.id)
}

/// One tick, as Mekanism's `TileEntityElectricMachine` and its kin: the
/// buffer's slot (if any) fills the chemical buffer, then the recipe that the
/// inputs and buffer satisfy runs. Item inputs (and a chemical used once) are
/// used when the operation completes; a per-tick chemical every tick it
/// advances. Progress restarts when no recipe fits, and waits while the outputs
/// have no room or the per-tick chemical runs short. Returns whether it is
/// running, for its loop sound.
pub fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, m: &Machine, recipes: &mut Recipes) -> bool {
    let pos = event.pos;
    let entity = &*event.entity;
    let mut items: Vec<Option<ItemStack>> = entity.items.clone();
    items.resize(m.slots(), None);
    let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, m.capacity());
    let mut progress = read_energy(&entity.nbt, PROGRESS).clamp(0, machine::TICKS);
    let mut tank = m.buffer.map(|b| read_tank(&entity.nbt, b));

    if let (Some(b), Some(t)) = (m.buffer, tank.as_mut()) {
        if let Some(stack) = &items[b.slot] {
            let fill = recipes
                .conversion(host)
                .iter()
                .filter(|c| c.input.test(stack))
                .find_map(|c| b.type_index(&c.chemical).map(|kind| (kind, c.amount)));
            if let Some((kind, amount)) = fill {
                if t.fits(kind, amount, b.max) {
                    t.add(kind, amount, b.max);
                    items[b.slot] = shrink(stack, 1);
                }
            }
        }
    }

    let extra = |n: u32, test: &dyn Fn(&ItemStack) -> bool| {
        m.extra.and_then(|e| items[e].as_ref()).is_some_and(|s| test(s) && s.count >= n)
    };
    let recipe = items[m.input].as_ref().and_then(|input| {
        recipes.processing(host, m.recipe).iter().find(|r| {
            r.input.test(input)
                && input.count >= r.count
                && r.extra.as_ref().is_none_or(|(ingredient, n)| extra(*n, &|s| ingredient.test(s)))
                && r.chemical.as_ref().is_none_or(|c| match (chemical_kind(m, c), tank) {
                    // A per-tick chemical is waited for; one used at the end must be there.
                    (Some(kind), Some(t)) if c.per_tick => t.takes(kind),
                    (Some(kind), Some(t)) => t.has(kind, c.amount),
                    _ => false,
                })
        })
    });
    let mut active = false;
    match recipe {
        None => progress = 0,
        Some(r) => {
            // Room for every output (the secondary in full, whatever its chance).
            let out = r.output.as_ref().map(|o| merged(host, items[m.output].as_ref(), o));
            let second = r.secondary.as_ref().map(|(s, chance)| {
                (m.secondary.and_then(|slot| merged(host, items[slot].as_ref(), s)), *chance)
            });
            let room = out.as_ref().is_none_or(Option::is_some) && second.as_ref().is_none_or(|s| s.0.is_some());
            let per_tick = r.chemical.as_ref().filter(|c| c.per_tick).and_then(|c| Some((chemical_kind(m, c)?, c.amount)));
            let fed = per_tick.is_none_or(|(kind, amount)| tank.is_some_and(|t| t.has(kind, amount)));
            if room && fed {
                active = energy >= m.usage;
                if let (true, Some((_, amount)), Some(t)) = (active, per_tick, tank.as_mut()) {
                    t.take(amount);
                }
                if machine::step(m.usage, &mut energy, &mut progress) {
                    if let (Some(c), Some(t)) = (r.chemical.as_ref().filter(|c| !c.per_tick), tank.as_mut()) {
                        t.take(c.amount);
                    }
                    let input = items[m.input].as_ref().expect("recipe matched an input");
                    items[m.input] = shrink(input, r.count);
                    if let (Some(slot), Some((_, n))) = (m.extra, &r.extra) {
                        let stack = items[slot].as_ref().expect("recipe matched an extra input");
                        items[slot] = shrink(stack, *n);
                    }
                    if let Some(Some(o)) = out {
                        items[m.output] = Some(o);
                    }
                    if let (Some(slot), Some((Some(s), chance))) = (m.secondary, second) {
                        if machine::random() <= chance {
                            items[slot] = Some(s);
                        }
                    }
                }
            }
        }
    }

    // The model lights up while running; set the property only on change.
    let want = if active { "true" } else { "false" };
    if host.block_property(pos, "active").is_some_and(|have| have != want) {
        host.set_block_property(pos, "active", want);
    }
    // Write only what changed: each write marks the entity changed.
    let facing = facing(host, pos);
    let e = &mut *event.entity;
    if e.items != items {
        e.items = items;
    }
    write_int(&mut e.nbt, ENERGY, energy);
    write_int(&mut e.nbt, PROGRESS, progress);
    if let Some(t) = tank {
        write_int(&mut e.nbt, BUFFER, t.amount);
        write_int(&mut e.nbt, BUFFER_TYPE, if t.amount > 0 { t.kind as i32 + 1 } else { 0 });
    }
    let data = machine::menu_data(energy, progress, facing, active, tank);
    if e.data != data {
        e.data = data;
    }
    active
}

/// Hoppers as on a furnace: the bottom takes the outputs; the top fills the
/// input, and the sides fill the buffer's slot, the extra input or, without
/// either, the input (Mekanism's side config is not ported).
pub fn container(m: &Machine) -> ModContainer {
    let second = m.buffer.map(|b| b.slot).or(m.extra);
    let side = vec![second.unwrap_or(m.input)];
    let mut inputs: Vec<usize> = second.into_iter().collect();
    inputs.push(m.input);
    ModContainer {
        slots: m.slots(),
        faces: [m.outputs(), vec![m.input], side.clone(), side.clone(), side.clone(), side],
        inputs,
        outputs: m.outputs(),
        menu: Some(m.menu.to_owned()),
    }
}

pub fn screen(m: &Machine) -> NativeScreen {
    NativeScreen {
        width: machine::WIDTH,
        height: machine::HEIGHT,
        slots: m.slot_pos.iter().map(|&p| Some(p)).collect(),
        inventory_y: machine::INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

/// The screen in Mekanism's layout: progress bar between input and output,
/// energy gauge right; with a chemical buffer, its gauge (and the infuser's
/// dump button). Amounts are in the tooltips (`machine::tooltip`).
pub fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, m: &Machine) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let (energy, progress) = (d(machine::DATA_ENERGY), d(machine::DATA_PROGRESS));
    let title = host.screen_title().unwrap_or_else(|| m.name.to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let at = |[rx, ry, w, h]: [f32; 4]| [x + rx, y + ry, w, h];
    gauge(host, event.batch, at(m.energy_gauge), energy as f32 / m.capacity() as f32, 0xFF50C878);
    let [px, py, pw, ph] = at(m.progress_bar);
    host.fill(event.batch, [px, py, pw, ph], 0xFF242A2F);
    let done = progress as f32 / machine::TICKS as f32;
    if done > 0.0 {
        host.fill(event.batch, [px + 1.0, py + 1.0, (pw - 2.0) * done, ph - 2.0], 0xFF3FA9F5);
    }
    if let Some(b) = m.buffer {
        let (amount, kind) = (d(machine::DATA_BUFFER), d(machine::DATA_BUFFER_TYPE));
        let color = kind.checked_sub(1).and_then(|k| b.types.get(k as usize)).map_or(0, |t| t.2);
        gauge(host, event.batch, at(b.gauge), amount as f32 / b.max as f32, color);
        if let Some(dump) = b.dump {
            let [bx, by, bw, bh] = at(dump);
            host.fill(event.batch, [bx, by, bw, bh], 0xFF262A2F);
            host.fill(event.batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], 0xFFAF4141);
            host.text(event.batch, "x", [bx + 6.0, by + 2.0], 0xFFFFFF, true);
            host.text(event.batch, &format!("{energy} FE"), [x + 128.0, y + 6.0], 0x404040, false);
        }
    }
    host.text(event.batch, "Inventory", [x + 8.0, y + machine::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The dump button empties (voids) the buffer, as Mekanism's.
pub fn click(m: &Machine, at: [f32; 2]) -> Option<i32> {
    let dump = m.buffer.and_then(|b| b.dump)?;
    machine::inside(dump, at).then_some(infuser::BUTTON_DUMP)
}

pub fn button(m: &Machine, nbt: &mut Compound, id: i32) {
    if m.buffer.is_some_and(|b| b.dump.is_some()) && id == infuser::BUTTON_DUMP {
        write_int(nbt, BUFFER, 0);
        write_int(nbt, BUFFER_TYPE, 0);
    }
}
