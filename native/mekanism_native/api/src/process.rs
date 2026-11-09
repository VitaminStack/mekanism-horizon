//! The host side of every processing machine in `machine::MACHINES`: its
//! tick, container, screen and drawing, one implementation for all of them.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Host, ItemStack, ModContainer, NativeScreen, ScreenDrawEvent,
};

use super::machine::{self, Machine};
use super::recipe::Recipes;
use super::{ENERGY, PROGRESS, facing, gauge, infuser, merged, read_energy, shrink, write_int};

/// Block entity NBT of a machine with an infuse buffer: `infuser::TYPES`
/// index + 1 (0 while empty) and the amount in mB.
pub const INFUSE_TYPE: &str = "mekanism_infuse_type";
pub const INFUSE: &str = "mekanism_infuse";

pub fn read_tank(nbt: &Compound) -> infuser::Tank {
    let kind = read_energy(nbt, INFUSE_TYPE);
    let amount = read_energy(nbt, INFUSE).clamp(0, infuser::MAX_INFUSE);
    if kind < 1 || kind as usize > infuser::TYPES.len() || amount == 0 {
        return infuser::Tank::default();
    }
    infuser::Tank { kind: kind as usize - 1, amount }
}

/// One tick, as Mekanism's `TileEntityElectricMachine` and
/// `TileEntityMetallurgicInfuser`: the infusion slot (if any) fills the infuse
/// buffer, then the recipe that the input slot and buffer satisfy runs; its
/// inputs are used when the operation completes, and progress restarts when no
/// recipe fits. Returns whether it is running, for its loop sound.
pub fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, m: &Machine, recipes: &mut Recipes) -> bool {
    let pos = event.pos;
    let entity = &*event.entity;
    let mut items: Vec<Option<ItemStack>> = entity.items.clone();
    items.resize(m.slots(), None);
    let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, m.capacity());
    let mut progress = read_energy(&entity.nbt, PROGRESS).clamp(0, machine::TICKS);
    let mut tank = m.infusion.map(|_| read_tank(&entity.nbt));

    if let (Some(slot), Some(t)) = (m.infusion, tank.as_mut()) {
        if let Some(stack) = &items[slot] {
            if let Some(c) = recipes.conversion(host).iter().find(|c| c.input.test(stack)) {
                if t.fits(c.kind, c.amount) {
                    t.add(c.kind, c.amount);
                    items[slot] = shrink(stack, 1);
                }
            }
        }
    }

    let has = |chemical: Option<(usize, i32)>| match chemical {
        None => true,
        Some((kind, amount)) => tank.is_some_and(|t| t.has(kind, amount)),
    };
    let ready = items[m.input].as_ref().and_then(|input| {
        recipes
            .processing(host, m.recipe)
            .iter()
            .find(|r| r.input.test(input) && input.count >= r.count && has(r.chemical))
            .and_then(|r| merged(host, items[m.output].as_ref(), &r.output).map(|out| (r.count, r.chemical, out)))
    });
    let mut active = false;
    match ready {
        Some((count, chemical, out)) => {
            active = energy >= m.usage;
            if machine::step(m.usage, &mut energy, &mut progress) {
                if let (Some((_, amount)), Some(t)) = (chemical, tank.as_mut()) {
                    t.take(amount);
                }
                let input = items[m.input].as_ref().expect("recipe matched an input");
                items[m.input] = shrink(input, count);
                items[m.output] = Some(out);
            }
        }
        None => progress = 0,
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
        write_int(&mut e.nbt, INFUSE, t.amount);
        write_int(&mut e.nbt, INFUSE_TYPE, if t.amount > 0 { t.kind as i32 + 1 } else { 0 });
    }
    let data = machine::menu_data(energy, progress, facing, active, tank);
    if e.data != data {
        e.data = data;
    }
    active
}

/// Hoppers as on a furnace: the bottom takes the output; the top fills the
/// input, and the sides fill the infusion slot or, without one, the input
/// (Mekanism's side config is not ported).
pub fn container(m: &Machine) -> ModContainer {
    let side = vec![m.infusion.unwrap_or(m.input)];
    let mut inputs: Vec<usize> = m.infusion.into_iter().collect();
    inputs.push(m.input);
    ModContainer {
        slots: m.slots(),
        faces: [vec![m.output], vec![m.input], side.clone(), side.clone(), side.clone(), side],
        inputs,
        outputs: vec![m.output],
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
/// energy gauge right; with an infuse buffer, its gauge left of the infusion
/// slot and the dump button under it. Amounts are in the tooltips (`machine::tooltip`).
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
    if m.infusion.is_some() {
        let (infuse, kind) = (d(machine::DATA_INFUSE), d(machine::DATA_TYPE));
        let color = kind.checked_sub(1).and_then(|k| infuser::TYPES.get(k as usize)).map_or(0, |t| t.2);
        gauge(host, event.batch, at(infuser::INFUSE_GAUGE), infuse as f32 / infuser::MAX_INFUSE as f32, color);
        let [bx, by, bw, bh] = at(infuser::DUMP_BUTTON);
        host.fill(event.batch, [bx, by, bw, bh], 0xFF262A2F);
        host.fill(event.batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], 0xFFAF4141);
        host.text(event.batch, "x", [bx + 6.0, by + 2.0], 0xFFFFFF, true);
        host.text(event.batch, &format!("{energy} FE"), [x + 128.0, y + 6.0], 0x404040, false);
    }
    host.text(event.batch, "Inventory", [x + 8.0, y + machine::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The dump button empties (voids) the infuse buffer, as Mekanism's.
pub fn click(m: &Machine, at: [f32; 2]) -> Option<i32> {
    (m.infusion.is_some() && machine::inside(infuser::DUMP_BUTTON, at)).then_some(infuser::BUTTON_DUMP)
}

pub fn button(m: &Machine, nbt: &mut Compound, id: i32) {
    if m.infusion.is_some() && id == infuser::BUTTON_DUMP {
        write_int(nbt, INFUSE, 0);
        write_int(nbt, INFUSE_TYPE, 0);
    }
}
