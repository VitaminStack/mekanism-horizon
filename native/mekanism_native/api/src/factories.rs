//! The host side of the factories (`factory.rs`): their tick, container,
//! screen and drawing, and the Tier Installer that turns a machine or a lower
//! factory into the next one.
//!
//! A factory repeats its base machine's operation (`process::tick`'s item
//! branch) in every one of its processes: each has its own input, output and
//! progress, and all take their energy, chemical and extra input from the same
//! buffers. The processes run in order each tick, so the earlier ones get the
//! shared energy first, as Mekanism's `TileEntityFactory.onUpdateServer` does.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Host, ItemStack, ModBlockEntity, ModContainer, NativeScreen, ScreenDrawEvent, Tag,
};

use super::factory::{self, Factory};
use super::machine::{self, Tank};
use super::process::{BUFFER, BUFFER_TYPE};
use super::recipe::{Recipe, Recipes};
use super::{ENERGY, PROGRESS, cube, facing, gauge, infuser, key, merged, read_energy, shrink, write_int};

/// The processes' progress in ticks, one entry per process (`IntArray`).
pub const FACTORY_PROGRESS: &str = "mekanism_factory_progress";
/// `Block.UPDATE_ALL`.
const UPDATE_ALL: i32 = 3;

fn read_progress(nbt: &Compound, processes: usize, ticks: i32) -> Vec<i32> {
    let mut progress = vec![0; processes];
    if let Some(Tag::IntArray(saved)) = nbt.get(&key(nbt, FACTORY_PROGRESS)) {
        for (slot, value) in progress.iter_mut().zip(saved) {
            *slot = (*value).clamp(0, ticks);
        }
    }
    progress
}

fn write_progress(nbt: &mut Compound, progress: &[i32]) {
    let k = key(nbt, FACTORY_PROGRESS);
    let value = Tag::IntArray(progress.to_vec());
    if nbt.get(&k) != Some(&value) {
        nbt.insert(k, value);
    }
}

/// The shared tank: `Buffer::types` index + 1 and the amount, as a machine's, but `processes` times as big.
fn read_tank(nbt: &Compound, f: &Factory) -> Option<Tank> {
    let (b, max) = (f.base().buffer?, f.tank_max()?);
    let kind = read_energy(nbt, BUFFER_TYPE);
    let amount = read_energy(nbt, BUFFER).clamp(0, max);
    if kind < 1 || kind as usize > b.types.len() || amount == 0 {
        return Some(Tank::default());
    }
    Some(Tank { kind: kind as usize - 1, amount })
}

/// Whether process `i`'s input matches `r`: the stack, the shared extra input and the tank's chemical, as
/// `process::tick` tests them.
fn matches(f: &Factory, r: &Recipe, input: &ItemStack, shared: Option<&ItemStack>, tank: Option<Tank>) -> bool {
    let base = f.base();
    r.input.test(input)
        && input.count >= r.count
        && r.chemical_output.is_none()
        && r.extra.as_ref().is_none_or(|(ingredient, n)| {
            base.extra.is_some() && shared.is_some_and(|s| ingredient.test(s) && s.count >= *n)
        })
        && r.chemical.as_ref().is_none_or(|c| match (base.buffer.and_then(|b| b.type_index(&c.id)), tank) {
            // A per-tick chemical is waited for; one used at the end must be there.
            (Some(kind), Some(t)) if c.per_tick => t.takes(kind),
            (Some(kind), Some(t)) => t.has(kind, c.amount),
            _ => false,
        })
}

/// One tick of the factory. Returns whether any process ran, for the loop sound.
pub fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, f: &Factory, recipes: &mut Recipes) -> bool {
    let (pos, base, slots) = (event.pos, f.base(), f.slots());
    let entity = &*event.entity;
    let mut items: Vec<Option<ItemStack>> = entity.items.clone();
    items.resize(slots.count(), None);
    let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, f.capacity());
    let mut progress = read_progress(&entity.nbt, f.processes(), base.ticks);
    let mut tank = read_tank(&entity.nbt, f);
    let max = f.tank_max().unwrap_or(0);

    // The shared slot fills the tank, one item a tick, when it all fits.
    if let (Some(b), Some(t), Some(slot)) = (base.buffer, tank.as_mut(), slots.shared) {
        if let Some(stack) = &items[slot] {
            let fill = recipes
                .conversion(host)
                .iter()
                .filter(|c| c.input.test(stack))
                .find_map(|c| b.type_index(&c.chemical).map(|kind| (kind, c.amount)));
            if let Some((kind, amount)) = fill {
                if t.fits(kind, amount, max) {
                    t.add(kind, amount, max);
                    items[slot] = shrink(stack, 1);
                }
            }
        }
    }

    let mut active = false;
    let list = recipes.processing(host, base.recipe);
    for i in 0..f.processes() {
        let (input_slot, output_slot) = (slots.inputs[i], slots.outputs[i]);
        let shared = slots.shared.filter(|_| base.extra.is_some()).and_then(|s| items[s].as_ref());
        let recipe = items[input_slot]
            .as_ref()
            .and_then(|input| list.iter().find(|r| matches(f, r, input, shared, tank)));
        // Room for every output: the secondary in full, whatever its chance.
        let out = recipe.and_then(|r| r.output.as_ref()).map(|o| merged(host, items[output_slot].as_ref(), o));
        let second = recipe.and_then(|r| r.secondary.as_ref()).map(|(s, chance)| {
            (slots.secondary.get(i).and_then(|&slot| merged(host, items[slot].as_ref(), s)), *chance)
        });
        let room = out.as_ref().is_none_or(Option::is_some) && second.as_ref().is_none_or(|s| s.0.is_some());
        let per_tick = recipe
            .and_then(|r| r.chemical.as_ref())
            .filter(|c| c.per_tick)
            .and_then(|c| Some((base.buffer?.type_index(&c.id)?, c.amount)));
        let fed = per_tick.is_none_or(|(kind, amount)| tank.is_some_and(|t| t.has(kind, amount)));
        let phase = factory::phase(recipe.is_some(), room, fed);
        let mut spent = None;
        let run = factory::advance(phase, base.usage, base.ticks, &mut energy, &mut progress[i], || spent = per_tick);
        if let (Some((_, amount)), Some(t)) = (spent, tank.as_mut()) {
            t.take(amount);
        }
        active |= run.active;
        let Some(r) = recipe else { continue };
        if !run.done {
            continue;
        }
        if let (Some(c), Some(t)) = (r.chemical.as_ref().filter(|c| !c.per_tick), tank.as_mut()) {
            t.take(c.amount);
        }
        let input = items[input_slot].as_ref().expect("a recipe matched an input");
        items[input_slot] = shrink(input, r.count);
        if let (Some(shared), Some((_, n))) = (slots.shared.filter(|_| base.extra.is_some()), &r.extra) {
            let stack = items[shared].as_ref().expect("a recipe matched an extra input");
            items[shared] = shrink(stack, *n);
        }
        if let Some(Some(o)) = out {
            items[output_slot] = Some(o);
        }
        if let Some((Some(s), chance)) = second {
            if machine::random() <= chance {
                items[slots.secondary[i]] = Some(s);
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
    write_progress(&mut e.nbt, &progress);
    if let Some(t) = tank {
        write_int(&mut e.nbt, BUFFER, t.amount);
        write_int(&mut e.nbt, BUFFER_TYPE, if t.amount > 0 { t.kind as i32 + 1 } else { 0 });
    }
    let t = tank.unwrap_or_default();
    let mut data = vec![0; factory::DATA_PROGRESS];
    data[factory::DATA_ENERGY] = energy;
    data[factory::DATA_FACING] = facing as i32;
    data[factory::DATA_ACTIVE] = active as i32;
    data[factory::DATA_TANK] = t.amount;
    data[factory::DATA_TANK_TYPE] = if t.amount > 0 { t.kind as i32 + 1 } else { 0 };
    data.extend(&progress);
    if e.data != data {
        e.data = data;
    }
    active
}

/// Hoppers as on a machine: the bottom takes the outputs, the top fills the
/// inputs, the sides the shared slot (the inputs, without one).
pub fn container(f: &Factory) -> ModContainer {
    let s = f.slots();
    let outputs: Vec<usize> = s.outputs.iter().chain(&s.secondary).copied().collect();
    let side = s.shared.map_or_else(|| s.inputs.clone(), |slot| vec![slot]);
    let inputs: Vec<usize> = s.shared.into_iter().chain(s.inputs.iter().copied()).collect();
    ModContainer {
        slots: s.count(),
        faces: [outputs.clone(), s.inputs.clone(), side.clone(), side.clone(), side.clone(), side],
        inputs,
        outputs,
        menu: Some(f.block()),
    }
}

pub fn screen(f: &Factory) -> NativeScreen {
    let [inventory_x, inventory_y] = f.inventory();
    NativeScreen {
        width: f.width(),
        height: f.height(),
        slots: f.slot_pos().into_iter().map(Some).collect(),
        inventory_y,
        inventory_x,
        own_labels: true,
        own_background: false,
    }
}

/// The screen in Mekanism's layout (`GuiFactory`): a progress arrow under each input, the energy
/// gauge right, and with a chemical tank its bar and dump button under the slots.
pub fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, f: &Factory) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| f.name());
    host.text(event.batch, &title, [x + 8.0, y + 4.0], 0x404040, false);
    let at = |[rx, ry, w, h]: [f32; 4]| [x + rx, y + ry, w, h];
    gauge(host, event.batch, at(f.energy_gauge()), d(factory::DATA_ENERGY) as f32 / f.capacity() as f32, 0xFF50C878);
    let ticks = f.base().ticks as f32;
    for i in 0..f.processes() {
        let [px, py, pw, ph] = at(f.progress_bar(i));
        host.fill(event.batch, [px, py, pw, ph], 0xFF242A2F);
        let done = (d(factory::DATA_PROGRESS + i) as f32 / ticks).clamp(0.0, 1.0);
        if done > 0.0 {
            host.fill(event.batch, [px + 1.0, py + 1.0, pw - 2.0, (ph - 2.0) * done], 0xFF3FA9F5);
        }
    }
    if let (Some(bar), Some(max), Some(b)) = (f.tank_bar(), f.tank_max(), f.base().buffer) {
        let [bx, by, bw, bh] = at(bar);
        host.fill(event.batch, [bx, by, bw, bh], 0xFF242A2F);
        let color = d(factory::DATA_TANK_TYPE).checked_sub(1).and_then(|k| b.types.get(k as usize)).map_or(0, |t| t.2);
        let share = (d(factory::DATA_TANK) as f32 / max as f32).clamp(0.0, 1.0);
        if share > 0.0 {
            host.fill(event.batch, [bx + 1.0, by + 1.0, (bw - 2.0) * share, bh - 2.0], color);
        }
    }
    if let Some(dump) = f.dump() {
        let [bx, by, bw, bh] = at(dump);
        host.fill(event.batch, [bx, by, bw, bh], 0xFF262A2F);
        host.fill(event.batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], 0xFFAF4141);
        host.text(event.batch, "x", [bx + 8.0, by + 1.0], 0xFFFFFF, true);
    }
    let [ix, iy] = f.inventory();
    host.text(event.batch, "Inventory", [x + ix, y + iy - 10.0], 0x404040, false);
}

/// The dump button empties (voids) the tank, as Mekanism's.
pub fn click(f: &Factory, at: [f32; 2]) -> Option<i32> {
    machine::inside(f.dump()?, at).then_some(factory::BUTTON_DUMP)
}

pub fn button(f: &Factory, nbt: &mut Compound, id: i32) {
    if f.dump().is_some() && id == factory::BUTTON_DUMP {
        write_int(nbt, BUFFER, 0);
        write_int(nbt, BUFFER_TYPE, 0);
    }
}

/// Tooltip lines for the gauge, bar or button under `at`, from the menu data.
pub fn tooltip(f: &Factory, data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if machine::inside(f.energy_gauge(), at) {
        return vec![
            format!("Energy: {} / {} FE", d(factory::DATA_ENERGY), f.capacity()),
            format!("Usage: {} FE/t per process", f.base().usage),
        ];
    }
    if let Some(i) = (0..f.processes()).find(|&i| machine::inside(f.progress_bar(i), at)) {
        return vec![format!("Progress: {}%", d(factory::DATA_PROGRESS + i) * 100 / f.base().ticks)];
    }
    if let (Some(bar), Some(max), Some(b)) = (f.tank_bar(), f.tank_max(), f.base().buffer) {
        if machine::inside(bar, at) {
            let kind = d(factory::DATA_TANK_TYPE);
            let name = kind.checked_sub(1).and_then(|k| b.types.get(k as usize)).map_or("Empty", |t| t.1);
            return vec![format!("{name}: {} / {max} mB", d(factory::DATA_TANK))];
        }
    }
    if f.dump().is_some_and(|r| machine::inside(r, at)) {
        return vec![infuser::DUMP_TOOLTIP.to_owned()];
    }
    Vec::new()
}

/// The Tier Installer's tier, from its item id (`mekanism:basic_tier_installer`).
pub fn installer_tier(item: &str) -> Option<usize> {
    let tier = item.strip_prefix("mekanism:")?.strip_suffix("_tier_installer")?;
    factory::TIERS.iter().position(|t| t.name == tier)
}

/// What the entity `old` becomes as `to`: its items by role, its energy and tank (the shared tank is
/// bigger), and the progress of each process it had (a machine's is the first process's).
fn upgraded(old: &ModBlockEntity, from_machine: Option<&'static machine::Machine>, from: Option<Factory>, to: &Factory) -> (Vec<Option<ItemStack>>, Compound) {
    let source = match (from, from_machine) {
        (Some(f), _) => f.slots(),
        (None, Some(m)) => factory::Slots::of_machine(m),
        _ => unreachable!("an upgrade starts from a factory or a machine"),
    };
    let items = factory::move_items(&source, &old.items, &to.slots());
    let mut nbt = Compound::new();
    write_int(&mut nbt, ENERGY, read_energy(&old.nbt, ENERGY).clamp(0, to.capacity()));
    let progress = match from {
        Some(f) => read_progress(&old.nbt, f.processes(), to.base().ticks),
        None => vec![read_energy(&old.nbt, PROGRESS).clamp(0, to.base().ticks)],
    };
    let mut all = vec![0; to.processes()];
    all[..progress.len()].copy_from_slice(&progress);
    write_progress(&mut nbt, &all);
    if let Some(max) = to.tank_max() {
        write_int(&mut nbt, BUFFER, read_energy(&old.nbt, BUFFER).clamp(0, max));
        write_int(&mut nbt, BUFFER_TYPE, read_energy(&old.nbt, BUFFER_TYPE));
    }
    (items, nbt)
}

/// Right click with a Tier Installer on a machine (basic installer) or a factory (the next tier's):
/// the block becomes the next factory, in the same facing, with its contents, and the installer is
/// used up outside creative mode (`ItemTierInstaller.useOn`). Returns whether it did.
pub fn install(host: &mut dyn Host, pos: [i32; 3], installer: &str) -> bool {
    let Some(tier) = installer_tier(installer) else { return false };
    let Some(old) = host.mod_block_entity(pos) else { return false };
    let block = host.block(pos);
    let current = factory::find(&block);
    let base = machine::find(&block);
    let kind = factory::kind_of_machine(&block);
    let Some(target) = factory::upgrade(current, kind, tier) else { return false };
    let Some(state) = host.default_state(&target.block()) else { return false };
    let (items, nbt) = upgraded(&old, base, current, &target);
    let facing = cube::DIR_NAMES[facing(host, pos)];
    let creative = host.acting_player().is_some_and(|p| p.game_mode == 1);
    if !creative && host.take_items(installer, 1) != 1 {
        return false;
    }
    // The old entity goes empty first: what it holds moves to the new one, not to the ground.
    host.change_mod_block_entity(pos, &mut |m| m.items.fill(None));
    let dimension = host.dimension().to_owned();
    if !host.set_block(&dimension, pos, state, UPDATE_ALL) {
        host.change_mod_block_entity(pos, &mut |m| m.items = old.items.clone());
        if !creative {
            host.give_items(installer, 1);
        }
        return false;
    }
    host.set_block_property(pos, "facing", facing);
    host.change_mod_block_entity(pos, &mut |m| {
        m.items = items.clone();
        m.nbt = nbt.clone();
    });
    let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
    host.play_sound("minecraft:block.anvil.use", at, 4, 0.5, 1.2);
    true
}

/// The machine a hum plays for: the factory's base machine's loop.
pub fn sound(f: &Factory) -> (&'static str, i32) {
    (f.base().sound, f.base().sound_ticks)
}
