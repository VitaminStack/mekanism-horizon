//! The host side of the Quantum Entangloporter (M08). The buffers' rules and the exchange that
//! keeps them conserved are Horizon-free in `frequency.rs` (`exchange`); read its header.
//!
//! Mekanism's entangloporter has no storage of its own: every one on an inventory frequency
//! shares that frequency's buffers (`InventoryFrequency`: energy, fluid, chemical, one item slot).
//! Horizon's energy, fluid and container hooks read a block entity's own buffer, so each member
//! carries a window of the shared buffer: every tick it hands everything it holds back to the
//! registry's pool and takes a window out again (`frequency::exchange`). What a neighbour put in or
//! took out of the window moves the pool by the same amount; nothing is made or lost. The pool is
//! saved with the registry (`teleporters.rs`).
//!
//! Not ported: heat, the per-side item/fluid/chemical/energy config (every side takes and gives
//! all four), the item slot's own energy-in slot, and the 256,000,000 FE buffer reaching a
//! neighbour in one go (a member shows 256,000 FE at a time, Horizon's energy is an `i32`).

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Host, ItemStack, ModContainer, ModEnergyBuffer, ModFluidTanks, NativeScreen,
    ScreenDrawEvent, Tag,
};

use crate::frequency::{self as freq, Kind as FKind, Tank, exchange};
use crate::{ENERGY, chemicals, cube, key, read_energy, tank, write_int};
use super::teleporters as tp;

/// Registered by `data/mekanism/horizon/menu/quantum_entangloporter.json`.
const MENU: &str = "mekanism:quantum_entangloporter";

/// Menu data after the panel's four entries (amounts take two, 16 bits each; kinds one).
const DATA_ENERGY: usize = 4;
const DATA_FLUID: usize = 6;
const DATA_FLUID_KIND: usize = 8;
const DATA_CHEMICAL: usize = 9;
const DATA_CHEMICAL_KIND: usize = 11;
const DATA_LEN: usize = 12;

pub(super) fn container() -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: vec![0],
        menu: Some(MENU.to_owned()),
    }
}

pub(super) fn energy_buffer() -> ModEnergyBuffer {
    let window = freq::ENERGY_WINDOW as i32;
    ModEnergyBuffer {
        capacity: window,
        max_insert: window,
        max_extract: window,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// Tank 0 takes fluids, tank 1 chemicals, from every side.
pub(super) fn fluids() -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![freq::FLUID_WINDOW as i32, freq::CHEMICAL_WINDOW as i32],
        faces: std::array::from_fn(|_| vec![0, 1]),
        inputs: vec![0, 1],
        outputs: vec![0, 1],
    }
}

/// Whether tank `index` takes `fluid`.
pub(super) fn accepts(index: usize, fluid: &str) -> bool {
    let family = if index == 0 { tank::Family::Fluid } else { tank::Family::Chemical };
    tank::accepts(family, fluid)
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: 176.0,
        height: 212.0,
        slots: vec![Some([150.0, 80.0])],
        inventory_y: 130.0,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

// --- the two block-entity tanks in Horizon's `fluids` list (tank 0 fluid, tank 1 chemical) ---

fn read_tanks(nbt: &Compound) -> [Tank; 2] {
    let mut out = [Tank::default(), Tank::default()];
    let Some(Tag::List(list)) = nbt.get(&key(nbt, "fluids")) else { return out };
    for (slot, entry) in out.iter_mut().zip(list) {
        let Tag::Compound(c) = entry else { continue };
        let fluid = match c.get(&key(c, "Fluid")) {
            Some(Tag::String(s)) => Some(s.to_string_lossy()).filter(|s| s != "minecraft:empty"),
            _ => None,
        };
        let amount = i64::from(read_energy(c, "Amount"));
        if fluid.is_some() && amount > 0 {
            *slot = Tank { kind: fluid, amount };
        }
    }
    out
}

fn write_tanks(nbt: &mut Compound, tanks: &[Tank; 2]) {
    let clean = |t: &Tank| match &t.kind {
        Some(k) if t.amount > 0 => Tank { kind: Some(k.clone()), amount: t.amount },
        _ => Tank::default(),
    };
    let want = [clean(&tanks[0]), clean(&tanks[1])];
    if read_tanks(nbt) == want {
        return;
    }
    let list = want
        .iter()
        .map(|t| {
            let mut c = Compound::new();
            c.insert(key(&c, "Fluid"), Tag::String(t.kind.as_deref().unwrap_or("minecraft:empty").into()));
            c.insert(key(&c, "Amount"), Tag::Int(t.amount.clamp(0, i64::from(i32::MAX)) as i32));
            Tag::Compound(c)
        })
        .collect();
    nbt.insert(key(nbt, "fluids"), Tag::List(list));
}

/// The item id of a stack that carries nothing but its id and count.
fn plain_kind(host: &dyn Host, stack: &ItemStack) -> Option<String> {
    let id: &str = &stack.item;
    let plain = host.item_from_snbt(&format!("{{id:{id:?},count:1}}"))?;
    (plain.extra == stack.extra).then(|| id.to_owned())
}

/// `count` of the item `id`, made from `template` (a stack of that item) or from the host.
fn stack_of(host: &dyn Host, id: &str, count: i64, templates: &[&ItemStack]) -> Option<ItemStack> {
    let count = u32::try_from(count).ok()?;
    match templates.iter().find(|t| &*t.item == id) {
        Some(t) => Some(ItemStack { count, ..(*t).clone() }),
        None => host.item_from_snbt(&format!("{{id:{id:?},count:{count}}}")),
    }
}

/// One member's tick: exchange with the pool, write the window back, report to the menu.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
    let id = tp::bound(&event.entity.nbt, FKind::Inventory);
    let mut energy = Tank::new(None, i64::from(read_energy(&event.entity.nbt, ENERGY)));
    let [mut fluid, mut chemical] = read_tanks(&event.entity.nbt);
    if event.entity.items.is_empty() {
        event.entity.items.push(None);
    }
    let slot = event.entity.items[0].clone();
    // A stack with components (enchantments, contents) stays in the slot: the pool keeps ids only.
    let slot_kind = slot.as_ref().and_then(|s| plain_kind(host, s));
    let slot_foreign = slot.is_some() && slot_kind.is_none();
    let mut item = match (&slot, &slot_kind) {
        (Some(s), Some(k)) => Tank::new(Some(k.as_str()), i64::from(s.count)),
        _ => Tank::default(),
    };
    // The pool's item decides the stack limit, so ask the host before taking the lock.
    let pool_item = id.as_ref().and_then(|i| tp::with(host, |s, _| s.registry.get(i).and_then(|f| f.shared.item.kind.clone())));
    let template = pool_item
        .as_deref()
        .filter(|k| slot_kind.as_deref() != Some(*k))
        .and_then(|k| host.item_from_snbt(&format!("{{id:{k:?},count:1}}")));
    let limit = pool_item
        .as_deref()
        .or(slot_kind.as_deref())
        .and_then(|k| host.max_stack_size(k))
        .map_or(64, i64::from);

    let before_item = (item.kind.clone(), item.amount);
    let me = freq::Member::new(host.dimension(), event.pos);
    let turn = super::multiblocks::game_time(host);
    let shown = tp::with(host, |s, now| {
        let i = id.as_ref()?;
        s.registry.touch(i, &me, now);
        // Members take turns at the window, so every member's neighbours reach the pool.
        let holds = freq::holds_window(&s.registry.active(i, now, tp::ACTIVE_MS), &me, turn);
        let window = |w: i64| if holds { w } else { 0 };
        let Some(f) = s.registry.get_mut(i) else { return None };
        let before = f.shared.clone();
        let mut pool = Tank::new(None, f.shared.energy);
        exchange(&mut pool, freq::ENERGY_CAP, &mut energy, window(freq::ENERGY_WINDOW));
        f.shared.energy = pool.amount;
        exchange(&mut f.shared.fluid, freq::FLUID_CAP, &mut fluid, window(freq::FLUID_WINDOW));
        exchange(&mut f.shared.chemical, freq::CHEMICAL_CAP, &mut chemical, window(freq::CHEMICAL_WINDOW));
        if !slot_foreign {
            exchange(&mut f.shared.item, limit, &mut item, window(limit));
        }
        if f.shared != before {
            s.registry.dirty = true;
        }
        let shared = &s.registry.get(id.as_ref()?)?.shared;
        Some((
            shared.energy + energy.amount,
            shared.fluid.amount + fluid.amount,
            shared.chemical.amount + chemical.amount,
            shared.fluid.kind.clone().or_else(|| fluid.kind.clone()),
            shared.chemical.kind.clone().or_else(|| chemical.kind.clone()),
        ))
    });

    if shown.is_some() {
        write_int(&mut event.entity.nbt, ENERGY, energy.amount.clamp(0, i64::from(i32::MAX)) as i32);
        write_tanks(&mut event.entity.nbt, &[fluid.clone(), chemical.clone()]);
        if !slot_foreign && (item.kind.clone(), item.amount) != before_item {
            let next = item
                .kind
                .as_deref()
                .and_then(|k| stack_of(host, k, item.amount, &[slot.as_ref(), template.as_ref()].into_iter().flatten().collect::<Vec<_>>()));
            event.entity.items[0] = next;
        }
    }

    let [a, b, c, d] = tp::panel_data(&event.entity.nbt, FKind::Inventory);
    let mut data = vec![0; DATA_LEN];
    data[..4].copy_from_slice(&[a, b, c, d]);
    let (e, f, ch, fk, ck) = shown.unwrap_or_else(|| (energy.amount, fluid.amount, chemical.amount, fluid.kind.clone(), chemical.kind.clone()));
    let clamp = |v: i64| v.clamp(0, i64::from(i32::MAX)) as i32;
    data[DATA_ENERGY..DATA_ENERGY + 2].copy_from_slice(&cube::encode_amount(clamp(e)));
    data[DATA_FLUID..DATA_FLUID + 2].copy_from_slice(&cube::encode_amount(clamp(f)));
    data[DATA_CHEMICAL..DATA_CHEMICAL + 2].copy_from_slice(&cube::encode_amount(clamp(ch)));
    let kind_of = |k: &Option<String>| k.as_deref().and_then(chemicals::index).map_or(0, |i| i as i32 + 1);
    data[DATA_FLUID_KIND] = kind_of(&fk);
    data[DATA_CHEMICAL_KIND] = kind_of(&ck);
    if event.entity.data != data {
        event.entity.data = data;
    }
}

/// A broken (or about to be broken) entangloporter gives its window back to the pool; its own
/// buffers are then emptied so a cancelled break does not count them twice. Its item slot
/// drops with the block.
pub(super) fn broken(host: &mut dyn Host, pos: [i32; 3]) {
    let Some(entity) = host.mod_block_entity(pos) else { return };
    let Some(id) = tp::bound(&entity.nbt, FKind::Inventory) else { return };
    let mut energy = Tank::new(None, i64::from(read_energy(&entity.nbt, ENERGY)));
    let [mut fluid, mut chemical] = read_tanks(&entity.nbt);
    let found = tp::with(host, |s, _| {
        let Some(f) = s.registry.get_mut(&id) else { return false };
        let mut pool = Tank::new(None, f.shared.energy);
        exchange(&mut pool, freq::ENERGY_CAP, &mut energy, 0);
        f.shared.energy = pool.amount;
        exchange(&mut f.shared.fluid, freq::FLUID_CAP, &mut fluid, 0);
        exchange(&mut f.shared.chemical, freq::CHEMICAL_CAP, &mut chemical, 0);
        s.registry.dirty = true;
        true
    });
    if found {
        host.change_mod_block_entity(pos, &mut |m| {
            write_int(&mut m.nbt, ENERGY, 0);
            write_tanks(&mut m.nbt, &[Tank::default(), Tank::default()]);
        });
    }
}

pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let title = host.screen_title().unwrap_or_else(|| "Quantum Entangloporter".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let (energy, fluid, chemical) = (
        cube::read_amount(event.data, DATA_ENERGY).max(0),
        cube::read_amount(event.data, DATA_FLUID).max(0),
        cube::read_amount(event.data, DATA_CHEMICAL).max(0),
    );
    let name = |i: usize| {
        (event.data.get(i).copied().unwrap_or(0) as usize)
            .checked_sub(1)
            .and_then(chemicals::entry)
            .map_or("Empty", |e| e.0)
    };
    let lines = [
        format!("Energy: {} / {} FE", cube::si(i64::from(energy)), cube::si(freq::ENERGY_CAP)),
        format!("Fluid: {} {} mB", name(DATA_FLUID_KIND), cube::si(i64::from(fluid))),
        format!("Chemical: {} {} mB", name(DATA_CHEMICAL_KIND), cube::si(i64::from(chemical))),
    ];
    for (n, line) in lines.iter().enumerate() {
        host.text(event.batch, line, [x + 8.0, y + 88.0 + 11.0 * n as f32], 0x333333, false);
    }
    tp::draw_panel(host, event, "Frequency");
    host.text(event.batch, "Item", [x + 150.0, y + 70.0], 0x404040, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + 120.0], 0x404040, false);
}
