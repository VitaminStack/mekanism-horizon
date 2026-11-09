//! Mekanism energy cubes (all tiers) and the Energy Tablet for Horizon.
//! Blocks, items and recipes are registered by the separate content mod.
//!
//! Hooks used (native hooks API; see Horizon docs/native-mods.md):
//! - `block_entity` (API 15): a saved block entity for each cube.
//! - `block_entity_energy` (API 142): the cube's energy buffer. Horizon saves the
//!   amount itself in the block entity NBT as `energy` (Tag::Int).
//! - `block_entity_ticks` / `block_entity_tick` (API 15/17): every cube ejects
//!   from its output sides into neighbouring cubes; the creative cube stays full.
//! - `state_for_placement` (API 76): the cube's front faces its placer.
//! - `block_entity_container` (API 17) + content menu type: a mod block
//!   menu opened by Horizon's normal block-use path, including singleplayer.
//! - `screen`, `screen_draw`, `screen_click`, `menu_button` (API 19/20):
//!   GUI for stored energy, throughput and all six configurable sides.
//! - `destroy_block`, `loot_drops`, `place_block` (API 14/75): a broken cube's
//!   item keeps its energy and side config, and placing it restores them.
//! - `item_energy_buffer` (API 307): the Energy Tablet's and the cube items'
//!   buffers, saved by Horizon in the stack's `minecraft:custom_data` as `horizon_energy`.
//! - `tooltip` (API 12): a cube item's stored energy.
//!
//! Horizon's energy API uses i32 amounts. Mekanism's tiers up to Ultimate fit;
//! the creative cube (Long.MAX_VALUE in Mekanism) is capped at i32::MAX.
//!
//! Known API gaps (see docs/ENERGY-CUBE.md): Horizon's energy faces are per block
//! type, not per block entity, so other mods may still fill or drain any side;
//! a trait mod has no energy helper for other mods' blocks, so ejection reaches
//! only Mekanism cubes.

//!
//! Metallurgic Infuser (`infuser.rs`; M05 groundwork, needed for M02's steel,
//! circuits and alloys): the same block-entity, container, menu, screen and
//! energy hooks, plus `recipes_of_type`/`recipe_ingredient`/`recipe_result`
//! (API 36) for Mekanism's own `metallurgic_infusing` and `chemical_conversion`
//! recipe files, shipped by the content mod.
//!
//! Enrichment Chamber and Crusher (`machine.rs`): the infuser's hooks for one
//! item in, one item out, with Mekanism's `enriching` and `crushing` recipes.
//!
//! Machines and generators loop Mekanism's sound while running: `Host::play_sound`
//! (API 61, mod sound events from the content mods' `sounds.json`), replayed when the clip ends.
//!
//! Heat, Solar and Bio Generator (`generator.rs`; M04, ids of the separate
//! `mekanismgenerators` content mod): the same block-entity, container, menu,
//! screen and energy hooks; they read the world through `Host::block`,
//! `biome`, `light`, `weather`, `day_time` and `dimension` (API 20/97), and
//! push their energy into the neighbouring cube or machine on their output side.

mod cube;
mod generator;
mod infuser;
mod machine;

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use cube::{Mode, Sides};
use horizon_client::mod_api::{
    NativeBlockEdit, NativeBlockRun, BlockEntityDraw, BlockEntityRenderBoundingBoxEvent,
    BlockEntitySubmitEvent, BlockEntityUpdateTagEvent, StandaloneModelsEvent,
    BlockEntityContainerEvent, BlockEntityEnergyEvent, BlockEntityEvent, BlockEntityTickEvent,
    BlockEntityTicksEvent, Compound, DestroyBlockEvent, Flow, Hooks, Host, Ingredient,
    ItemEnergyBuffer, ItemEnergyBufferEvent, ItemStack, LevelHost, LootDropsEvent, MenuButtonEvent,
    ModContainer, ModEnergyBuffer, NativeScreen, PlaceBlockEvent, ScreenClickEvent,
    ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, StateForPlacementEvent, Tag, Text, TooltipEvent,
};

/// (block id, capacity, max insert/extract per call) from Mekanism's `EnergyCubeTier`.
const CUBES: [(&str, i32, i32); 5] = [
    ("mekanism:basic_energy_cube", 4_000_000, 4_000),
    ("mekanism:advanced_energy_cube", 16_000_000, 16_000),
    ("mekanism:elite_energy_cube", 64_000_000, 64_000),
    ("mekanism:ultimate_energy_cube", 256_000_000, 256_000),
    ("mekanism:creative_energy_cube", i32::MAX, i32::MAX),
];
const CREATIVE_CUBE: &str = "mekanism:creative_energy_cube";
/// Debug blocks for testing the cubes (TODO N08): the source pushes energy into
/// neighbouring cubes, the sink pulls it out, each by the cube's own per-call limit.
const DEBUG_SOURCE: &str = "mekanism:debug_energy_source";
const DEBUG_SINK: &str = "mekanism:debug_energy_sink";
/// Mekanism `GearConfig`: tablet max energy 1,000,000, charge rate 5,000.
const TABLET: &str = "mekanism:energy_tablet";
const TABLET_CAPACITY: i32 = 1_000_000;
const TABLET_RATE: i32 = 5_000;

/// Block entity NBT: the energy Horizon keeps (`energy`) and the side config by
/// relative side (`mekanism_sides`, 6 bytes of `cube::Mode`). Cube items carry
/// both in `minecraft:custom_data`, the energy as Horizon's `horizon_energy`.
const ENERGY: &str = "energy";
const SIDES: &str = "mekanism_sides";
const ITEM_ENERGY: &str = "horizon_energy";
/// Registered by `data/mekanism/horizon/menu/energy_cube.json`.
const CUBE_MENU: &str = "mekanism:energy_cube";
/// `SoundSource.BLOCKS` and `WEATHER` ordinals.
const SOUND_BLOCKS: usize = 4;
const SOUND_WEATHER: usize = 3;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(MekanismNative::default())
}

fn is_debug(block: &str) -> bool {
    block == DEBUG_SOURCE || block == DEBUG_SINK
}

fn cube(block: &str) -> Option<(i32, i32)> {
    CUBES
        .iter()
        .find(|(id, ..)| *id == block)
        .map(|&(_, capacity, rate)| (capacity, rate))
}

/// A key in the compound's own key type (Horizon's NBT string).
fn key<K: From<&'static str>>(_: &BTreeMap<K, Tag>, name: &'static str) -> K {
    name.into()
}

fn read_energy(nbt: &Compound, name: &'static str) -> i32 {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn read_sides(nbt: &Compound) -> Option<Sides> {
    match nbt.get(&key(nbt, SIDES)) {
        Some(Tag::ByteArray(b)) => cube::sides_from_bytes(b),
        _ => None,
    }
}

/// Write only on change: a write marks the entity changed (and resyncs it).
fn write_int(nbt: &mut Compound, name: &'static str, value: i32) {
    let k = key(nbt, name);
    if nbt.get(&k) != Some(&Tag::Int(value)) {
        nbt.insert(k, Tag::Int(value));
    }
}

fn facing(host: &dyn Host, pos: [i32; 3]) -> usize {
    host.block_property(pos, "facing")
        .and_then(|f| cube::dir_from_name(&f))
        .unwrap_or(cube::NORTH)
}

fn offset(pos: [i32; 3], dir: usize) -> [i32; 3] {
    let d = cube::OFFSETS[dir];
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}

/// Show the energy port on each world side whose relative side has a mode.
fn update_ports(host: &mut dyn Host, pos: [i32; 3], sides: &Sides) {
    let f = facing(host, pos);
    for (rel, mode) in sides.iter().enumerate() {
        let dir = cube::absolute(rel, f);
        let want = if *mode == Mode::None { "false" } else { "true" };
        if host.block_property(pos, &format!("port_{}", cube::DIR_NAMES[dir])).as_deref() != Some(want) {
            host.set_block_property(pos, &format!("port_{}", cube::DIR_NAMES[dir]), want);
        }
    }
}

/// Block entity NBT of the infuser (besides Horizon's `energy`).
const PROGRESS: &str = "mekanism_progress";
/// `infuser::TYPES` index + 1; 0 while the buffer is empty.
const INFUSE_TYPE: &str = "mekanism_infuse_type";
const INFUSE: &str = "mekanism_infuse";
/// Block entity NBT of the Heat Generator: stored heat (Tag::Double) and lava in mB.
/// The Bio Generator keeps its bioethanol in mB as `mekanism_bio_fuel`.
const BIO_FUEL: &str = "mekanism_bio_fuel";
const HEAT: &str = "mekanism_heat";
const LAVA: &str = "mekanism_lava";
/// A Wind Generator turning its blades, sent to clients (1 or 0).
const WIND_ACTIVE: &str = "mekanism_wind_active";

/// A `mekanism:metallurgic_infusing` recipe: `item_input`, `chemical_input`, `output`.
struct Infusing {
    input: Ingredient,
    count: u32,
    kind: usize,
    amount: i32,
    output: ItemStack,
}

/// A `mekanism:chemical_conversion` recipe into an infuse type (one item).
struct Conversion {
    input: Ingredient,
    kind: usize,
    amount: i32,
}

/// Parsed recipes, kept while the server's recipe table is the same one
/// (its slices' addresses and lengths); `/reload` gives new ones.
#[derive(Default)]
struct InfuserRecipes {
    key: [usize; 4],
    infusing: Vec<Infusing>,
    conversion: Vec<Conversion>,
}

impl InfuserRecipes {
    fn refresh(&mut self, host: &dyn Host) {
        let infusing = host.recipes_of_type(infuser::INFUSING);
        let conversion = host.recipes_of_type(infuser::CONVERSION);
        let key = [infusing.as_ptr() as usize, infusing.len(), conversion.as_ptr() as usize, conversion.len()];
        if key == self.key {
            return;
        }
        self.key = key;
        self.infusing = infusing
            .iter()
            .filter_map(|r| {
                let j = &r.json;
                let chem = &j["chemical_input"];
                Some(Infusing {
                    input: host.recipe_ingredient(&j["item_input"]["ingredient"])?,
                    count: j["item_input"]["count"].as_u64().unwrap_or(1) as u32,
                    kind: infuser::type_index(chem["ingredient"].as_str()?)?,
                    amount: chem["amount"].as_i64()? as i32,
                    output: host.recipe_result(&j["output"])?,
                })
            })
            .collect();
        self.conversion = conversion
            .iter()
            .filter_map(|r| {
                let j = &r.json;
                Some(Conversion {
                    input: host.recipe_ingredient(&j["input"]["ingredient"])?,
                    kind: infuser::type_index(j["output"]["id"].as_str()?)?,
                    amount: j["output"]["amount"].as_i64()? as i32,
                })
            })
            .collect();
    }
}

/// A one-item-in, one-item-out machine recipe (`input`, `output`).
struct ItemRecipe {
    input: Ingredient,
    count: u32,
    output: ItemStack,
}

/// Parsed recipes of one machine's type, kept while the server's table for it
/// is the same one (its slice's address and length).
#[derive(Default)]
struct MachineRecipes {
    key: [usize; 2],
    recipes: Vec<ItemRecipe>,
}

impl MachineRecipes {
    fn refresh(&mut self, host: &dyn Host, kind: &str) {
        let all = host.recipes_of_type(kind);
        let key = [all.as_ptr() as usize, all.len()];
        if key == self.key {
            return;
        }
        self.key = key;
        self.recipes = all
            .iter()
            .filter_map(|r| {
                let j = &r.json;
                Some(ItemRecipe {
                    input: host.recipe_ingredient(&j["input"]["ingredient"])?,
                    count: j["input"]["count"].as_u64().unwrap_or(1) as u32,
                    output: host.recipe_result(&j["output"])?,
                })
            })
            .collect();
    }
}

/// Energy capacity of a machine that takes energy on every side.
fn machine_capacity(block: &str) -> Option<i32> {
    if block == infuser::BLOCK {
        return Some(infuser::CAPACITY);
    }
    machine::find(block).map(machine::Machine::capacity)
}

fn is_machine(block: &str) -> bool {
    machine_capacity(block).is_some()
}

fn read_tank(nbt: &Compound) -> infuser::Tank {
    let kind = read_energy(nbt, INFUSE_TYPE);
    let amount = read_energy(nbt, INFUSE).clamp(0, infuser::MAX_INFUSE);
    if kind < 1 || kind as usize > infuser::TYPES.len() || amount == 0 {
        return infuser::Tank::default();
    }
    infuser::Tank { kind: kind as usize - 1, amount }
}

/// `count` fewer of the stack; none when that empties it.
fn shrink(stack: &ItemStack, count: u32) -> Option<ItemStack> {
    (stack.count > count).then(|| ItemStack { count: stack.count - count, ..stack.clone() })
}

/// The output slot with `out` added, if it all fits (same item and components).
fn merged(host: &dyn Host, slot: Option<&ItemStack>, out: &ItemStack) -> Option<ItemStack> {
    let Some(have) = slot else { return Some(out.clone()) };
    let max = host.max_stack_size(&out.item).unwrap_or(64);
    (have.item == out.item && have.extra == out.extra && have.count + out.count <= max)
        .then(|| ItemStack { count: have.count + out.count, ..have.clone() })
}

/// A vertical gauge filling from the bottom.
fn gauge(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, rect: [f32; 4], fill: f32, argb: u32) {
    let [x, y, w, h] = rect;
    host.fill(batch, rect, 0xFF242A2F);
    let inner = (h - 2.0) * fill.clamp(0.0, 1.0);
    if inner > 0.0 {
        host.fill(batch, [x + 1.0, y + h - 1.0 - inner, w - 2.0, inner], argb);
    }
}

/// The infuser's screen, in Mekanism's layout: infuse gauge left of the
/// infusion slot, progress bar between input and output, energy gauge right.
fn draw_infuser(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let (energy, progress, infuse, kind) =
        (d(infuser::DATA_ENERGY), d(infuser::DATA_PROGRESS), d(infuser::DATA_INFUSE), d(infuser::DATA_TYPE));
    let title = host.screen_title().unwrap_or_else(|| "Metallurgic Infuser".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);

    // Type name and amounts are in the gauges' tooltips (`infuser::tooltip`).
    let color = kind.checked_sub(1).and_then(|k| infuser::TYPES.get(k as usize)).map_or(0, |t| t.2);
    let at = |[rx, ry, w, h]: [f32; 4]| [x + rx, y + ry, w, h];
    gauge(host, event.batch, at(infuser::INFUSE_GAUGE), infuse as f32 / infuser::MAX_INFUSE as f32, color);
    gauge(host, event.batch, at(infuser::ENERGY_GAUGE), energy as f32 / infuser::CAPACITY as f32, 0xFF50C878);

    let [px, py, pw, ph] = at(infuser::PROGRESS_BAR);
    host.fill(event.batch, [px, py, pw, ph], 0xFF242A2F);
    let done = progress as f32 / infuser::TICKS as f32;
    if done > 0.0 {
        host.fill(event.batch, [px + 1.0, py + 1.0, (pw - 2.0) * done, ph - 2.0], 0xFF3FA9F5);
    }
    let [bx, by, bw, bh] = at(infuser::DUMP_BUTTON);
    host.fill(event.batch, [bx, by, bw, bh], 0xFF262A2F);
    host.fill(event.batch, [bx + 1.0, by + 1.0, bw - 2.0, bh - 2.0], 0xFFAF4141);
    host.text(event.batch, "x", [bx + 6.0, by + 2.0], 0xFFFFFF, true);
    host.text(event.batch, &format!("{energy} FE"), [x + 128.0, y + 6.0], 0x404040, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + infuser::INVENTORY_Y - 10.0], 0x404040, false);
}

/// An Enrichment Chamber's or Crusher's screen, in Mekanism's layout:
/// progress bar between input and output, energy gauge right.
fn draw_machine(host: &dyn Host, event: &mut ScreenDrawEvent, m: &machine::Machine) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let (energy, progress) = (d(machine::DATA_ENERGY), d(machine::DATA_PROGRESS));
    let title = host.screen_title().unwrap_or_else(|| m.name.to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let at = |[rx, ry, w, h]: [f32; 4]| [x + rx, y + ry, w, h];
    gauge(host, event.batch, at(machine::ENERGY_GAUGE), energy as f32 / m.capacity() as f32, 0xFF50C878);
    let [px, py, pw, ph] = at(machine::PROGRESS_BAR);
    host.fill(event.batch, [px, py, pw, ph], 0xFF242A2F);
    let done = progress as f32 / machine::TICKS as f32;
    if done > 0.0 {
        host.fill(event.batch, [px + 1.0, py + 1.0, (pw - 2.0) * done, ph - 2.0], 0xFF3FA9F5);
    }
    host.text(event.batch, "Inventory", [x + 8.0, y + machine::INVENTORY_Y - 10.0], 0x404040, false);
}

/// Each cube's energy and sides are synchronised by its block-entity menu data.
/// Unlike `open_menu`, the block menu works for local and remote players.
#[derive(Default)]
struct MekanismNative {
    /// Energy and sides of cubes a player is breaking, for their drop.
    broken: Mutex<HashMap<(String, [i32; 3]), (i32, Sides)>>,
    recipes: Mutex<InfuserRecipes>,
    machine_recipes: Mutex<HashMap<&'static str, MachineRecipes>>,
    /// Ticks until each running block's loop sound plays again (not saved).
    hums: Mutex<HashMap<(String, [i32; 3]), i32>>,
}

impl MekanismNative {
    /// Mekanism's machines loop their sound while active (its client
    /// `TileTickableSound`); here the server replays the clip each time it
    /// ends, for the players near it, in the `blocks` category.
    fn hum(&self, host: &mut dyn Host, pos: [i32; 3], sound: &str, ticks: i32, active: bool) {
        self.hum_as(host, pos, sound, ticks, active, SOUND_BLOCKS);
    }

    fn hum_as(&self, host: &mut dyn Host, pos: [i32; 3], sound: &str, ticks: i32, active: bool, source: usize) {
        let place = (host.dimension().to_owned(), pos);
        let mut hums = self.hums.lock().unwrap();
        if !active {
            hums.remove(&place);
            return;
        }
        let left = hums.entry(place).or_insert(0);
        if *left <= 0 {
            let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
            host.play_sound(sound, at, source, 1.0, 1.0);
            *left = ticks;
        }
        *left -= 1;
    }

    /// One infuser tick: fill the infuse buffer from the infusion slot, then run
    /// the recipe that the input slot and buffer satisfy, as Mekanism's
    /// `TileEntityMetallurgicInfuser`: the chemical and input are used when the
    /// operation completes; progress restarts when no recipe fits.
    fn infuser_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(infuser::SLOTS, None);
        let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, infuser::CAPACITY);
        let mut progress = read_energy(&entity.nbt, PROGRESS).clamp(0, infuser::TICKS);
        let mut tank = read_tank(&entity.nbt);

        let mut recipes = self.recipes.lock().unwrap();
        recipes.refresh(host);

        if let Some(stack) = &items[infuser::SLOT_INFUSION] {
            if let Some(c) = recipes.conversion.iter().find(|c| c.input.test(stack)) {
                if tank.fits(c.kind, c.amount) {
                    tank.add(c.kind, c.amount);
                    items[infuser::SLOT_INFUSION] = shrink(stack, 1);
                }
            }
        }

        let ready = items[infuser::SLOT_INPUT].as_ref().and_then(|input| {
            recipes
                .infusing
                .iter()
                .find(|r| r.input.test(input) && input.count >= r.count && tank.has(r.kind, r.amount))
                .and_then(|r| merged(host, items[infuser::SLOT_OUTPUT].as_ref(), &r.output).map(|out| (r, out)))
        });
        let mut active = false;
        match ready {
            Some((r, out)) => {
                active = energy >= infuser::USAGE;
                if infuser::step(&mut energy, &mut progress) {
                    tank.take(r.amount);
                    let input = items[infuser::SLOT_INPUT].as_ref().expect("recipe matched an input");
                    items[infuser::SLOT_INPUT] = shrink(input, r.count);
                    items[infuser::SLOT_OUTPUT] = Some(out);
                }
            }
            None => progress = 0,
        }
        drop(recipes);
        self.hum(host, pos, infuser::SOUND, infuser::SOUND_TICKS, active);

        // Write only what changed: each write marks the entity changed.
        let facing = facing(host, pos);
        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, energy);
        write_int(&mut e.nbt, PROGRESS, progress);
        write_int(&mut e.nbt, INFUSE, tank.amount);
        write_int(&mut e.nbt, INFUSE_TYPE, if tank.amount > 0 { tank.kind as i32 + 1 } else { 0 });
        let data = infuser::menu_data(energy, progress, tank, facing, active);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One tick of an Enrichment Chamber or Crusher, as Mekanism's
    /// `TileEntityElectricMachine`: the input is used when the operation
    /// completes; progress restarts when no recipe fits.
    fn machine_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent, m: &machine::Machine) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(machine::SLOTS, None);
        let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, m.capacity());
        let mut progress = read_energy(&entity.nbt, PROGRESS).clamp(0, machine::TICKS);

        let mut all = self.machine_recipes.lock().unwrap();
        let recipes = all.entry(m.recipe).or_default();
        recipes.refresh(host, m.recipe);
        let ready = items[machine::SLOT_INPUT].as_ref().and_then(|input| {
            recipes
                .recipes
                .iter()
                .find(|r| r.input.test(input) && input.count >= r.count)
                .and_then(|r| merged(host, items[machine::SLOT_OUTPUT].as_ref(), &r.output).map(|out| (r.count, out)))
        });
        drop(all);
        let mut active = false;
        match ready {
            Some((count, out)) => {
                active = energy >= m.usage;
                if machine::step(m.usage, &mut energy, &mut progress) {
                    let input = items[machine::SLOT_INPUT].as_ref().expect("recipe matched an input");
                    items[machine::SLOT_INPUT] = shrink(input, count);
                    items[machine::SLOT_OUTPUT] = Some(out);
                }
            }
            None => progress = 0,
        }

        self.hum(host, pos, m.sound, m.sound_ticks, active);
        // The model lights up while running; set the property only on change.
        let want = if active { "true" } else { "false" };
        if host.block_property(pos, "active").as_deref() != Some(want) {
            host.set_block_property(pos, "active", want);
        }
        let facing = facing(host, pos);
        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, energy);
        write_int(&mut e.nbt, PROGRESS, progress);
        let data = machine::menu_data(energy, progress, facing, active);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// Push energy out of the cube's output sides into neighbouring cubes whose
    /// side facing it takes input; at most the cube's transfer rate per tick.
    fn eject(&self, host: &mut dyn Host, pos: [i32; 3], rate: i32, stored: &mut i32, sides: &Sides) -> i32 {
        let f = facing(host, pos);
        let mut budget = rate;
        let mut moved_total = 0;
        for (rel, mode) in sides.iter().enumerate() {
            if *mode != Mode::Output || budget <= 0 || *stored <= 0 {
                continue;
            }
            let moved = give(host, pos, cube::absolute(rel, f), budget, *stored);
            *stored -= moved;
            budget -= moved;
            moved_total += moved;
        }
        moved_total
    }

    /// One Heat Generator tick: the fuel slot fills the lava tank (a lava
    /// bucket whole, leaving the bucket; a furnace fuel by its burn time), then
    /// `generator::Heat::tick`, then the energy goes out of the front.
    fn heat_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(1, None);
        let ambient = generator::ambient_temp(generator::climate(host.biome(pos).as_deref()).0 as f64);
        let mut state = generator::Heat {
            heat: read_double(&entity.nbt, HEAT).unwrap_or_else(|| generator::Heat::new(ambient).heat),
            energy: read_energy(&entity.nbt, ENERGY).clamp(0, generator::HEAT_STORAGE),
            lava: read_energy(&entity.nbt, LAVA).clamp(0, generator::LAVA_TANK),
        };

        if let Some(stack) = &items[0] {
            if &*stack.item == "minecraft:lava_bucket" {
                if state.lava + generator::LAVA_TANK <= generator::LAVA_TANK {
                    if let Some(bucket) = host.item_from_snbt("{id:\"minecraft:bucket\",count:1}") {
                        state.lava += generator::LAVA_TANK;
                        items[0] = Some(bucket);
                    }
                }
            } else if let Some(lava) = burn_time(host, stack).map(generator::fuel_lava) {
                if lava > 0 && state.lava + lava <= generator::LAVA_TANK {
                    state.lava += lava;
                    items[0] = shrink(stack, 1);
                }
            }
        }

        let lava_sides = (0..6).filter(|&d| host.block(offset(pos, d)) == "minecraft:lava").count() as u32;
        let nether = host.dimension() == "minecraft:the_nether";
        let tick = state.tick(generator::boost(lava_sides, nether), ambient);
        let f = facing(host, pos);
        state.energy -= give(host, pos, f, state.energy, state.energy);
        self.hum(host, pos, generator::HEAT_SOUND, generator::HEAT_SOUND_TICKS, tick.active);
        // The model glows while burning; set the property only on change.
        let want = if tick.active { "true" } else { "false" };
        if host.block_property(pos, "active").as_deref() != Some(want) {
            host.set_block_property(pos, "active", want);
        }

        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, state.energy);
        write_int(&mut e.nbt, LAVA, state.lava);
        let k = key(&e.nbt, HEAT);
        e.nbt.insert(k, Tag::Double(state.heat.to_bits()));
        let data = generator::heat_menu_data(&state, &tick, f);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Solar Generator tick: produce while it sees the sun, then the energy
    /// goes out of the bottom.
    /// One Bio Generator tick, as Mekanism's `TileEntityBioGenerator`: fill
    /// the tank from the fuel slot, burn 1 mB for 25 FE, push out of the front.
    fn bio_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let entity = &*event.entity;
        let mut items: Vec<Option<ItemStack>> = entity.items.clone();
        items.resize(1, None);
        let mut energy = read_energy(&entity.nbt, ENERGY).clamp(0, generator::BIO_STORAGE);
        let mut fuel = read_energy(&entity.nbt, BIO_FUEL).clamp(0, generator::BIO_TANK);

        if let Some(stack) = &items[0] {
            let per_item = if host.server_item_in_tag(stack, generator::BIO_FUEL_TAG).unwrap_or(false) {
                generator::BIO_FUEL_PER_ITEM
            } else if host.server_item_in_tag(stack, generator::BIO_FUEL_BLOCK_TAG).unwrap_or(false) {
                9 * generator::BIO_FUEL_PER_ITEM
            } else {
                0
            };
            if generator::bio_fits(fuel, per_item) {
                fuel += per_item;
                items[0] = shrink(stack, 1);
            }
        }

        let active = generator::bio_tick(&mut fuel, &mut energy);
        let f = facing(host, pos);
        energy -= give(host, pos, f, energy, energy);
        self.hum(host, pos, generator::BIO_SOUND, generator::BIO_SOUND_TICKS, active);

        let e = &mut *event.entity;
        if e.items != items {
            e.items = items;
        }
        write_int(&mut e.nbt, ENERGY, energy);
        write_int(&mut e.nbt, BIO_FUEL, fuel);
        let data = generator::bio_menu_data(energy, fuel, active, f);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Advanced Solar Generator tick: its nine sky checks are the 3 x 3
    /// above the panel (the sky light over each bounding block of its top).
    fn advanced_solar_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let storage = generator::ADVANCED_SOLAR_STORAGE;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, storage);
        let dimension = host.dimension();
        let sky_light = dimension != "minecraft:the_nether" && dimension != "minecraft:the_end";
        let (rain, thunder) = host.weather();
        let darkness = generator::sky_darkness(host.day_time(), rain, thunder);
        let seeing = if generator::sees_sun(sky_light, darkness, 15) {
            (-1..=1)
                .flat_map(|x| (-1..=1).map(move |z| [pos[0] + x, pos[1] + 3, pos[2] + z]))
                .filter(|&p| host.light(p).1 >= 15)
                .count()
        } else {
            0
        };
        let peak = generator::peak_multiplier(generator::climate(host.biome(pos).as_deref()));
        let produced = generator::advanced_solar_production(seeing, darkness, peak).min(storage - stored);
        stored += produced;
        let f = facing(host, pos);
        stored -= give(host, pos, f, stored, stored);
        stored -= give(host, pos, cube::DOWN, stored, stored);
        self.hum(host, pos, generator::SOLAR_SOUND, generator::SOLAR_SOUND_TICKS, produced > 0);

        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        let data = generator::solar_menu_data(stored, produced, seeing > 4, f, peak);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    /// One Wind Generator tick: the output rises with the height of its top
    /// (four blocks up), which must see the sky.
    fn wind_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let storage = generator::WIND_STORAGE;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, storage);
        let dimension = host.dimension().to_owned();
        let top = [pos[0], pos[1] + 4, pos[2]];
        let sees_sky = host.light([top[0], top[1] + 1, top[2]]).1 >= 15;
        let multiplier = generator::wind_multiplier(top[1], sees_sky, &dimension);
        let produced = generator::wind_generation(multiplier).min(storage - stored);
        stored += produced;
        let f = facing(host, pos);
        stored -= give(host, pos, f, stored, stored);
        stored -= give(host, pos, cube::DOWN, stored, stored);
        let active = multiplier > 0.0;
        self.hum_as(host, top, generator::WIND_SOUND, generator::WIND_SOUND_TICKS, active, SOUND_WEATHER);

        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        if read_energy(&e.nbt, WIND_ACTIVE) != active as i32 {
            write_int(&mut e.nbt, WIND_ACTIVE, active as i32);
            host.block_entity_sync(pos);
        }
        let data = generator::solar_menu_data(stored, produced, active, f, multiplier as f32);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }

    fn solar_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        let pos = event.pos;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, generator::SOLAR_STORAGE);
        let dimension = host.dimension();
        let sky_light = dimension != "minecraft:the_nether" && dimension != "minecraft:the_end";
        let (rain, thunder) = host.weather();
        let darkness = generator::sky_darkness(host.day_time(), rain, thunder);
        // The panel is a full block here: the sky light it gets is the block's above it.
        let sees = generator::sees_sun(sky_light, darkness, host.light(offset(pos, cube::UP)).1);
        let peak = generator::peak_multiplier(generator::climate(host.biome(pos).as_deref()));
        let produced = generator::solar_production(sees, darkness, peak).min(generator::SOLAR_STORAGE - stored);
        stored += produced;
        stored -= give(host, pos, cube::DOWN, stored, stored);
        self.hum(host, pos, generator::SOLAR_SOUND, generator::SOLAR_SOUND_TICKS, produced > 0);

        let f = facing(host, pos);
        let e = &mut *event.entity;
        write_int(&mut e.nbt, ENERGY, stored);
        let data = generator::solar_menu_data(stored, produced, sees, f, peak);
        if e.data.as_slice() != data.as_slice() {
            e.data = data.to_vec();
        }
    }
}

/// Move up to `budget` of `stored` into the block on side `dir` of `pos`: a
/// cube whose side facing it takes input, or a machine (every side takes
/// energy; no side config ported). Returns what moved: what the neighbour gains
/// the giver loses, so none is lost or made.
fn give(host: &mut dyn Host, pos: [i32; 3], dir: usize, budget: i32, stored: i32) -> i32 {
    if budget <= 0 || stored <= 0 {
        return 0;
    }
    let at = offset(pos, dir);
    let their_facing = facing(host, at);
    let mut moved = 0;
    host.change_mod_block_entity(at, &mut |m| {
        if let Some(capacity) = machine_capacity(&m.id) {
            let theirs = read_energy(&m.nbt, ENERGY);
            moved = cube::transfer(budget, stored, theirs, capacity, capacity);
            if moved > 0 {
                write_int(&mut m.nbt, ENERGY, theirs + moved);
            }
            return;
        }
        let Some((capacity, their_rate)) = cube(&m.id) else { return };
        let their_side = cube::relative(cube::opposite(dir), their_facing);
        let their_sides = read_sides(&m.nbt).unwrap_or(cube::DEFAULT_SIDES);
        if their_sides[their_side] != Mode::Input {
            return;
        }
        let theirs = read_energy(&m.nbt, ENERGY);
        moved = cube::transfer(budget, stored, theirs, capacity, their_rate);
        if moved > 0 {
            write_int(&mut m.nbt, ENERGY, theirs + moved);
        }
    });
    moved
}

/// A furnace fuel's burn time: vanilla's fuels by id, then by item tag.
fn burn_time(host: &dyn Host, stack: &ItemStack) -> Option<i32> {
    generator::fuel_burn_time(&stack.item).or_else(|| {
        generator::FUEL_TAGS
            .iter()
            .find(|(tag, _)| host.server_item_in_tag(stack, tag).unwrap_or(false))
            .map(|&(_, t)| t)
    })
}

fn read_double(nbt: &Compound, name: &'static str) -> Option<f64> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits)).filter(|v| v.is_finite() && *v >= 0.0),
        _ => None,
    }
}

fn is_generator(block: &str) -> bool {
    block == generator::HEAT || block == generator::SOLAR || block == generator::BIO || generator::is_multiblock(block)
}

/// Generators drawn on the solar screen: Solar, Advanced Solar and Wind.
fn uses_solar_screen(block: &str) -> bool {
    block == generator::SOLAR || generator::is_multiblock(block)
}

fn add(pos: [i32; 3], o: [i32; 3]) -> [i32; 3] {
    [pos[0] + o[0], pos[1] + o[1], pos[2] + o[2]]
}

fn is_air(host: &dyn Host, pos: [i32; 3]) -> bool {
    let b = host.block(pos);
    b == "minecraft:air" || b == "minecraft:cave_air" || b == "minecraft:void_air"
}

/// Sets `palette` at `parts` (only where `only` passes), straight into the chunks.
fn edit(host: &mut dyn Host, parts: &[[i32; 3]], palette: &str, only: Option<&str>) {
    let edit = NativeBlockEdit {
        palette: vec![palette.to_owned()],
        filters: only.map(|f| vec![f.to_owned()]).unwrap_or_default(),
        runs: parts
            .iter()
            .map(|p| NativeBlockRun { x: p[0], z: p[2], y: p[1], height: 1, surface: None, block: 0, only: only.map(|_| 0) })
            .collect(),
    };
    let dimension = host.dimension().to_owned();
    let _ = host.edit_blocks(&dimension, &edit);
}

/// Mekanism's `HandleBoundingBlock`: the rest of a multiblock's shape is
/// bounding blocks. A placement without room for them is undone and the main
/// block drops as an item, as close as Horizon gets to vanilla's refused placement.
fn place_multiblock(host: &mut dyn Host, pos: [i32; 3], block: &str) {
    let parts: Vec<[i32; 3]> = generator::bounding_offsets(block).into_iter().map(|o| add(pos, o)).collect();
    if !parts.iter().all(|&p| is_air(host, p)) {
        host.destroy_block(pos, None, false);
        return;
    }
    edit(host, &parts, generator::BOUNDING, None);
}

/// Removes a multiblock's bounding blocks (only bounding blocks, leaving `except`).
fn remove_bounding(host: &mut dyn Host, main: [i32; 3], block: &str, except: Option<[i32; 3]>) {
    let parts: Vec<[i32; 3]> = generator::bounding_offsets(block)
        .into_iter()
        .map(|o| add(main, o))
        .filter(|&p| Some(p) != except)
        .collect();
    edit(host, &parts, "minecraft:air", Some(generator::BOUNDING));
}

/// The main block a bounding block at `pos` belongs to.
fn bounding_main(host: &dyn Host, pos: [i32; 3]) -> Option<(&'static str, [i32; 3])> {
    generator::main_candidates(pos)
        .into_iter()
        .find(|&(block, main)| host.mod_block_entity(main).is_some_and(|e| &*e.id == block))
}

/// Machines and generators face the placer horizontally (Mekanism's `AttributeStateFacing`).
fn faces_horizontally(block: &str) -> bool {
    is_machine(block) || is_generator(block)
}

/// A vertical energy bar with its own fill colour.
fn energy_bar(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, origin: [f32; 2], energy: i32, capacity: i32) {
    let [x, y, w, h] = generator::ENERGY_BAR;
    gauge(host, batch, [origin[0] + x, origin[1] + y, w, h], energy as f32 / capacity as f32, 0xFF50C878);
}

/// The Heat Generator's screen: fuel slot left, lava gauge in the middle,
/// energy bar right; the temperature under the fuel slot.
/// The Bio Generator's screen, in Mekanism's layout: fuel bar at the left
/// edge, fuel slot, energy bar right.
/// Turn of a facing about y, counterclockwise from above (the blockstate's `y` is clockwise).
fn facing_degrees(dir: usize) -> f32 {
    match dir {
        cube::SOUTH => 180.0,
        cube::WEST => 90.0,
        cube::EAST => -90.0,
        _ => 0.0,
    }
}

/// `v` turned `degrees` counterclockwise about y (seen from above) through the block's centre.
fn turn_y(v: [f32; 3], degrees: f32, about: f32) -> [f32; 3] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (x, z) = (v[0] - about, v[2] - about);
    [about + x * cos + z * sin, v[1], about - x * sin + z * cos]
}

/// The Wind Generator's tower and blades, turned to its facing; the blades
/// spin about the hub while it runs. Kept until the block entity changes.
fn draw_wind(host: &dyn Host, pos: [i32; 3], active: bool, draw: &mut BlockEntityDraw) {
    let turn = facing_degrees(facing(host, pos));
    let speed = if active { generator::wind_rotor_speed(pos[1], host.dimension()) } else { 0.0 };
    // `spin` takes the axis and pivot in the block's own coordinates, whatever the pose.
    let axis = turn_y([0.0, 0.0, 1.0], turn, 0.0);
    let hub = turn_y(generator::WIND_HUB, turn, 0.5);
    draw.push();
    draw.rotate_about([0.0, 1.0, 0.0], turn, [0.5, 0.0, 0.5]);
    for blades in [false, true] {
        if blades {
            draw.spin(axis, hub, speed, 0.0);
        }
        for (model, moved, _) in generator::WIND_PARTS.iter().filter(|p| p.2 == blades) {
            draw.push();
            draw.translate([-moved[0], -moved[1], -moved[2]]);
            host.draw_model(draw, model, 0xFFFFFF);
            draw.pop();
        }
    }
    draw.still();
    draw.pop();
    draw.keep();
}

fn draw_bio(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| "Bio-Generator".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [bx, by, bw, bh] = generator::BIO_FUEL_BAR;
    let fuel = d(generator::BIO_DATA_FUEL) as f32 / generator::BIO_TANK as f32;
    gauge(host, event.batch, [x + bx, y + by, bw, bh], fuel, 0xFFC8B43C);
    energy_bar(host, event.batch, event.origin, d(generator::BIO_DATA_ENERGY), generator::BIO_STORAGE);
    let [sx, sy, sw, sh] = generator::BIO_SCREEN;
    host.fill(event.batch, [x + sx, y + sy, sw, sh], 0xFF242A2F);
    host.text(event.batch, &format!("{} FE/t", d(generator::BIO_DATA_PRODUCING)), [x + sx + 4.0, y + sy + 4.0], 0x3FA9F5, false);
    host.text(event.batch, &format!("{} mB", d(generator::BIO_DATA_FUEL)), [x + sx + 4.0, y + sy + 16.0], 0x3FA9F5, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

fn draw_heat(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| "Heat Generator".to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [gx, gy, gw, gh] = generator::HEAT_LAVA_GAUGE;
    let lava = d(generator::HEAT_DATA_LAVA) as f32 / generator::LAVA_TANK as f32;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], lava, 0xFFE8641E);
    energy_bar(host, event.batch, event.origin, d(generator::HEAT_DATA_ENERGY), generator::HEAT_STORAGE);
    host.text(event.batch, &format!("{} K", d(generator::HEAT_DATA_TEMP)), [x + 8.0, y + 58.0], 0x404040, false);
    host.text(event.batch, &format!("{} FE/t", d(generator::HEAT_DATA_PRODUCING)), [x + 124.0, y + 6.0], 0x404040, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

/// The Solar Generator's screen: Mekanism's inner screen with energy and
/// production, the sun indicator left of it, the energy bar right.
fn draw_solar(host: &dyn Host, event: &mut ScreenDrawEvent) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| "Solar Generator".to_owned());
    let storage = generator::panel_storage(event.block);
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let [sx, sy, sw, sh] = generator::SOLAR_SCREEN;
    host.fill(event.batch, [x + sx, y + sy, sw, sh], 0xFF1C2620);
    let energy = d(generator::SOLAR_DATA_ENERGY);
    host.text(
        event.batch,
        &format!("{} / {} FE", cube::si(energy as i64), cube::si(storage as i64)),
        [x + sx + 4.0, y + sy + 6.0],
        0x3CFE9A,
        false,
    );
    host.text(
        event.batch,
        &format!("{} FE/t", d(generator::SOLAR_DATA_PRODUCING)),
        [x + sx + 4.0, y + sy + 22.0],
        0x3CFE9A,
        false,
    );
    let [ix, iy, iw, ih] = generator::SOLAR_SUN;
    let sun = if d(generator::SOLAR_DATA_SEES_SUN) != 0 { 0xFFFFD34D } else { 0xFF3A3F44 };
    host.fill(event.batch, [x + ix, y + iy, iw, ih], 0xFF262A2F);
    host.fill(event.batch, [x + ix + 2.0, y + iy + 2.0, iw - 4.0, ih - 4.0], sun);
    energy_bar(host, event.batch, event.origin, energy, storage);
    host.text(event.batch, "Inventory", [x + 8.0, y + generator::INVENTORY_Y - 10.0], 0x404040, false);
}

impl Hooks for MekanismNative {
    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent) -> Option<(String, Compound)> {
        (cube(event.block).is_some() || is_debug(event.block) || is_machine(event.block) || is_generator(event.block))
            .then(|| (event.block.to_owned(), Compound::new()))
    }

    /// The block's own menu, not the LAN-only `Host::open_menu` helper.
    /// The one placeholder slot is hidden; all interactions are buttons.
    fn block_entity_container(&self, _host: &dyn Host, event: &mut BlockEntityContainerEvent) -> Option<ModContainer> {
        if event.block == infuser::BLOCK {
            // Hoppers as on a furnace: top into the input, sides into the infusion
            // slot, bottom takes the output (Mekanism's side config is not ported).
            let side = vec![infuser::SLOT_INFUSION];
            return Some(ModContainer {
                slots: infuser::SLOTS,
                faces: [vec![infuser::SLOT_OUTPUT], vec![infuser::SLOT_INPUT], side.clone(), side.clone(), side.clone(), side],
                inputs: vec![infuser::SLOT_INFUSION, infuser::SLOT_INPUT],
                outputs: vec![infuser::SLOT_OUTPUT],
                menu: Some(infuser::MENU.to_owned()),
            });
        }
        if let Some(m) = machine::find(event.block) {
            // Hoppers as on a furnace: top and sides into the input, bottom takes the output.
            let input = vec![machine::SLOT_INPUT];
            return Some(ModContainer {
                slots: machine::SLOTS,
                faces: [vec![machine::SLOT_OUTPUT], input.clone(), input.clone(), input.clone(), input.clone(), input],
                inputs: vec![machine::SLOT_INPUT],
                outputs: vec![machine::SLOT_OUTPUT],
                menu: Some(m.menu.to_owned()),
            });
        }
        if event.block == generator::HEAT {
            // Hoppers fill the fuel slot from any side.
            return Some(ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| vec![0]),
                inputs: vec![0],
                outputs: Vec::new(),
                menu: Some(generator::HEAT_MENU.to_owned()),
            });
        }
        if event.block == generator::BIO {
            // Hoppers fill the fuel slot from any side.
            return Some(ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| vec![0]),
                inputs: vec![0],
                outputs: Vec::new(),
                menu: Some(generator::BIO_MENU.to_owned()),
            });
        }
        if uses_solar_screen(event.block) {
            // One hidden placeholder slot, as the cube's.
            let menu = match event.block {
                generator::ADVANCED_SOLAR => generator::ADVANCED_SOLAR_MENU,
                generator::WIND => generator::WIND_MENU,
                _ => generator::SOLAR_MENU,
            };
            return Some(ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| Vec::new()),
                inputs: Vec::new(),
                outputs: Vec::new(),
                menu: Some(menu.to_owned()),
            });
        }
        cube(event.block)?;
        Some(ModContainer {
            slots: 1,
            faces: std::array::from_fn(|_| Vec::new()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            menu: Some(CUBE_MENU.to_owned()),
        })
    }

    fn block_entity_energy(&self, _host: &dyn Host, event: &mut BlockEntityEnergyEvent) -> Option<ModEnergyBuffer> {
        if let Some(capacity) = machine_capacity(event.block) {
            return Some(ModEnergyBuffer {
                capacity,
                max_insert: capacity,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            });
        }
        // Generators only give. Horizon's faces are per block type, not per facing:
        // other mods may take the Heat Generator's energy on any side.
        if event.block == generator::HEAT {
            return Some(ModEnergyBuffer {
                capacity: generator::HEAT_STORAGE,
                max_insert: 0,
                max_extract: generator::HEAT_STORAGE,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            });
        }
        if event.block == generator::BIO {
            return Some(ModEnergyBuffer {
                capacity: generator::BIO_STORAGE,
                max_insert: 0,
                max_extract: generator::BIO_STORAGE,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            });
        }
        if generator::is_multiblock(event.block) {
            // Front and bottom in Mekanism; Horizon's faces are per block type, not per facing.
            let storage = generator::panel_storage(event.block);
            return Some(ModEnergyBuffer {
                capacity: storage,
                max_insert: 0,
                max_extract: storage,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            });
        }
        if event.block == generator::SOLAR {
            let mut bottom = [false; 6];
            bottom[cube::DOWN] = true;
            return Some(ModEnergyBuffer {
                capacity: generator::SOLAR_STORAGE,
                max_insert: 0,
                max_extract: generator::SOLAR_STORAGE,
                insert_faces: [false; 6],
                extract_faces: bottom,
            });
        }
        cube(event.block).map(|(capacity, rate)| ModEnergyBuffer {
            capacity,
            max_insert: rate,
            max_extract: rate,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        })
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent) {
        if cube(event.block).is_some() || is_debug(event.block) || is_machine(event.block) || is_generator(event.block) {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        if event.entity.id == infuser::BLOCK {
            self.infuser_tick(host, event);
            return;
        }
        if let Some(m) = machine::find(&event.entity.id) {
            self.machine_tick(host, event, m);
            return;
        }
        if event.entity.id == generator::HEAT {
            self.heat_tick(host, event);
            return;
        }
        if event.entity.id == generator::SOLAR {
            self.solar_tick(host, event);
            return;
        }
        if event.entity.id == generator::BIO {
            self.bio_tick(host, event);
            return;
        }
        if event.entity.id == generator::ADVANCED_SOLAR {
            self.advanced_solar_tick(host, event);
            return;
        }
        if event.entity.id == generator::WIND {
            self.wind_tick(host, event);
            return;
        }
        if is_debug(&event.entity.id) {
            let source = event.entity.id == DEBUG_SOURCE;
            let pos = event.pos;
            for dir in 0..6 {
                host.change_mod_block_entity(offset(pos, dir), &mut |m| {
                    let Some((capacity, rate)) = cube(&m.id) else { return };
                    let stored = read_energy(&m.nbt, ENERGY);
                    let next = if source {
                        stored.saturating_add(rate.min(capacity - stored))
                    } else {
                        stored - rate.min(stored)
                    };
                    write_int(&mut m.nbt, ENERGY, next);
                });
            }
            return;
        }
        let Some((capacity, rate)) = cube(&event.entity.id) else { return };
        let pos = event.pos;

        // Side config: Mekanism's default until the cube has its own.
        let saved = read_sides(&event.entity.nbt);
        let sides = saved.unwrap_or(cube::DEFAULT_SIDES);
        let changed = saved.is_none();
        if changed {
            let k = key(&event.entity.nbt, SIDES);
            event.entity.nbt.insert(k, Tag::ByteArray(cube::sides_to_bytes(&sides)));
            update_ports(host, pos, &sides);
        }

        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, capacity);
        if event.entity.id == CREATIVE_CUBE {
            // The creative cube never runs dry.
            stored = capacity;
        }
        let ejected = self.eject(host, pos, rate, &mut stored, &sides);
        if event.entity.id == CREATIVE_CUBE {
            stored = capacity;
        }
        write_int(&mut event.entity.nbt, ENERGY, stored);

        // ContainerData is synchronised by Horizon for both singleplayer and LAN.
        // Each wire entry is 16-bit, so every 32-bit FE amount takes two entries.
        let data = cube::menu_data(stored, ejected, facing(host, pos), &sides);
        if event.entity.data.as_slice() != data.as_slice() {
            event.entity.data = data.to_vec();
        }
    }

    /// Mekanism turns the cube's front toward its placer.
    /// Machines and generators face the placer horizontally (Mekanism's `AttributeStateFacing`).
    fn state_for_placement(&self, level: &dyn LevelHost, event: &mut StateForPlacementEvent) -> Option<Option<horizon_client::mod_api::BlockStateId>> {
        let horizontal = faces_horizontally(&event.context.item);
        let pitch = if horizontal { 0.0 } else { event.context.pitch };
        if !horizontal {
            cube(&event.context.item)?;
        }
        let facing = cube::facing_from_look(event.context.yaw, pitch);
        level
            .state_with_property(event.state, "facing", cube::DIR_NAMES[facing])
            .map(Some)
    }

    /// A Horizon mod screen replaces the default inventory artwork when the
    /// content menu for this block opens. Hide the dummy container slot.
    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent) -> Option<NativeScreen> {
        if event.block == infuser::BLOCK {
            return Some(NativeScreen {
                width: infuser::WIDTH,
                height: infuser::HEIGHT,
                slots: infuser::SLOT_POS.iter().map(|&p| Some(p)).collect(),
                inventory_y: infuser::INVENTORY_Y,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            });
        }
        if machine::find(event.block).is_some() {
            return Some(NativeScreen {
                width: machine::WIDTH,
                height: machine::HEIGHT,
                slots: machine::SLOT_POS.iter().map(|&p| Some(p)).collect(),
                inventory_y: machine::INVENTORY_Y,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            });
        }
        if is_generator(event.block) {
            let fuel = match event.block {
                generator::HEAT => Some(generator::HEAT_FUEL_SLOT),
                generator::BIO => Some(generator::BIO_FUEL_SLOT),
                _ => None,
            };
            return Some(NativeScreen {
                width: generator::WIDTH,
                height: generator::HEIGHT,
                slots: vec![fuel],
                inventory_y: generator::INVENTORY_Y,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            });
        }
        cube(event.block)?;
        Some(NativeScreen {
            width: 176.0,
            height: 238.0,
            slots: vec![None],
            inventory_y: 156.0,
            inventory_x: 8.0,
            own_labels: true,
            own_background: false,
        })
    }

    /// Drawn on top of Horizon's plain panel; no item slots are used for labels.
    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent) {
        if event.block == infuser::BLOCK {
            draw_infuser(host, event);
            return;
        }
        if let Some(m) = machine::find(event.block) {
            draw_machine(host, event, m);
            return;
        }
        if event.block == generator::HEAT {
            draw_heat(host, event);
            return;
        }
        if uses_solar_screen(event.block) {
            draw_solar(host, event);
            return;
        }
        if event.block == generator::BIO {
            draw_bio(host, event);
            return;
        }
        let Some((capacity, rate)) = cube(event.block) else { return };
        let [x, y] = event.origin;
        let stored = cube::read_amount(event.data, cube::DATA_ENERGY).clamp(0, capacity);
        let ejected = cube::read_amount(event.data, cube::DATA_EJECTED).max(0);
        let progress = stored as f32 / capacity as f32;

        let title = host.screen_title().unwrap_or_else(|| "Energy Cube".to_owned());
        host.text(event.batch, &title, [x + 8.0, y + 7.0], 0x404040, false);
        host.text(
            event.batch,
            &format!("Energy: {} / {} FE", cube::si(stored as i64), cube::si(capacity as i64)),
            [x + 8.0, y + 24.0],
            0x333333,
            false,
        );
        host.fill(event.batch, [x + 8.0, y + 37.0, 160.0, 10.0], 0xFF242A2F);
        if progress > 0.0 {
            host.fill(event.batch, [x + 9.0, y + 38.0, 158.0 * progress, 8.0], 0xFF50C878);
        }
        host.text(
            event.batch,
            &format!("I/O: {} FE/t  Out: {} FE/t", cube::si(rate as i64), cube::si(ejected as i64)),
            [x + 8.0, y + 53.0],
            0x404040,
            false,
        );

        for rel in 0..6 {
            let mode = Mode::from_byte(event.data.get(cube::DATA_SIDES + rel).copied().unwrap_or(0) as u8);
            let [bx, by, w, h] = cube::SIDE_BUTTON_RECTS[rel];
            let color = match mode {
                Mode::None => 0xFF6B7075,
                Mode::Input => 0xFF278447,
                Mode::Output => 0xFFAF4141,
            };
            host.fill(event.batch, [x + bx, y + by, w, h], 0xFF262A2F);
            host.fill(event.batch, [x + bx + 1.0, y + by + 1.0, w - 2.0, h - 2.0], color);
            let status = match mode {
                Mode::None => "Off",
                Mode::Input => "In",
                Mode::Output => "Out",
            };
            let label = format!("{}: {}", cube::REL_BUTTON_NAMES[rel], status);
            host.text(event.batch, &label, [x + bx + 5.0, y + by + 5.0], 0xFFFFFF, true);
        }
        host.text(event.batch, "Left: next  |  Right: previous", [x + 8.0, y + 132.0], 0x555555, false);
        host.text(event.batch, "Inventory", [x + 8.0, y + 145.0], 0x404040, false);
    }

    /// Mouse hits a side button, not an inventory slot. The click is sent
    /// via a ContainerButtonClick packet and handled in `menu_button`.
    fn screen_click(&self, _host: &dyn Host, event: &mut ScreenClickEvent) -> Option<i32> {
        if event.block == infuser::BLOCK {
            return infuser::inside(infuser::DUMP_BUTTON, event.at).then_some(infuser::BUTTON_DUMP);
        }
        cube(event.block)?;
        cube::side_button_at(event.at, event.right)
    }

    /// Button presses update the actual block entity on the server, then
    /// normal ContainerData sync refreshes the screen for every player.
    /// The Wind Generator's tower and blades, drawn by its block entity (Mekanism's `RenderWindGenerator`).
    fn standalone_models(&self, event: &mut StandaloneModelsEvent) {
        event.models.extend(generator::WIND_PARTS.iter().map(|(model, _, _)| (*model).to_owned()));
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent) -> Option<Compound> {
        if &*event.entity.id != generator::WIND {
            return None;
        }
        let mut tag = Compound::default();
        write_int(&mut tag, WIND_ACTIVE, read_energy(&event.entity.nbt, WIND_ACTIVE));
        Some(tag)
    }

    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent) {
        if event.block == generator::WIND {
            let active = read_energy(&event.entity.nbt, WIND_ACTIVE) != 0;
            draw_wind(host, event.pos, active, event.draw);
        }
    }

    fn block_entity_render_bounding_box(&self, _host: &dyn Host, event: &mut BlockEntityRenderBoundingBoxEvent) {
        if event.block == generator::WIND {
            // The blades reach about four blocks round the hub, four and a half up.
            *event.bounds = [-4.5, 0.0, -4.5, 5.5, 9.0, 5.5];
        }
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut ScreenTooltipEvent) {
        if event.block == infuser::BLOCK {
            event.lines.extend(infuser::tooltip(event.data, event.at));
        } else if let Some(m) = machine::find(event.block) {
            event.lines.extend(machine::tooltip(m, event.data, event.at));
        } else if event.block == generator::HEAT {
            event.lines.extend(generator::heat_tooltip(event.data, event.at));
        } else if uses_solar_screen(event.block) {
            event.lines.extend(generator::solar_tooltip(event.block, event.data, event.at));
        } else if event.block == generator::BIO {
            event.lines.extend(generator::bio_tooltip(event.data, event.at));
        }
    }

    fn menu_button(&self, host: &mut dyn Host, event: &mut MenuButtonEvent) {
        if event.entity.id == infuser::BLOCK {
            // Dump: the infuse buffer is emptied (voided), as Mekanism's dump button.
            if event.id == infuser::BUTTON_DUMP {
                write_int(&mut event.entity.nbt, INFUSE, 0);
                write_int(&mut event.entity.nbt, INFUSE_TYPE, 0);
            }
            return;
        }
        let Some((rel, forward)) = cube::side_button_action(event.id) else { return };
        if cube(&event.entity.id).is_none() { return; }
        let mut sides = read_sides(&event.entity.nbt).unwrap_or(cube::DEFAULT_SIDES);
        sides[rel] = sides[rel].cycle(forward);
        let k = key(&event.entity.nbt, SIDES);
        event.entity.nbt.insert(k, Tag::ByteArray(cube::sides_to_bytes(&sides)));
        update_ports(host, event.pos, &sides);
        let ejected = cube::read_amount(&event.entity.data, cube::DATA_EJECTED);
        let stored = read_energy(&event.entity.nbt, ENERGY);
        event.entity.data = cube::menu_data(stored, ejected, facing(host, event.pos), &sides).to_vec();
    }

    /// Remember what a cube holds before a player breaks it, for its drop.
    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        self.hums.lock().unwrap().remove(&(host.dimension().to_owned(), event.pos));
        // Breaking a multiblock takes its bounding blocks with it; breaking a
        // bounding block breaks the main block, which drops (Mekanism's BlockBounding).
        if let Some(entity) = host.mod_block_entity(event.pos) {
            if generator::is_multiblock(&entity.id) {
                let block: &'static str = if &*entity.id == generator::WIND { generator::WIND } else { generator::ADVANCED_SOLAR };
                remove_bounding(host, event.pos, block, None);
            }
        } else if host.block(event.pos) == generator::BOUNDING {
            if let Some((block, main)) = bounding_main(host, event.pos) {
                remove_bounding(host, main, block, Some(event.pos));
                host.destroy_block(main, None, false);
            }
        }
        if let Some(entity) = host.mod_block_entity(event.pos) {
            if cube(&entity.id).is_some() {
                let sides = read_sides(&entity.nbt).unwrap_or(cube::DEFAULT_SIDES);
                self.broken.lock().unwrap().insert(
                    (host.dimension().to_owned(), event.pos),
                    (read_energy(&entity.nbt, ENERGY), sides),
                );
            }
        }
        Flow::Continue
    }

    /// A broken cube drops an item that keeps its energy and side config
    /// (Mekanism's loot table copies `mekanism:energy` and `mekanism:side_config`).
    fn loot_drops(&self, host: &mut dyn Host, event: &mut LootDropsEvent) {
        let Some((pos, _)) = event.context.block else { return };
        let place = (event.context.dimension.to_owned(), pos);
        let remembered = self.broken.lock().unwrap().remove(&place);
        let held = host
            .mod_block_entity(pos)
            .filter(|e| cube(&e.id).is_some())
            .map(|e| (read_energy(&e.nbt, ENERGY), read_sides(&e.nbt).unwrap_or(cube::DEFAULT_SIDES)))
            .or(remembered);
        let Some((energy, sides)) = held else { return };
        for drop in event.drops.iter_mut() {
            let Some((capacity, _)) = cube(&drop.item) else { continue };
            let bytes = cube::sides_to_bytes(&sides)
                .iter()
                .map(|b| format!("{b}b"))
                .collect::<Vec<_>>()
                .join(",");
            let snbt = format!(
                "{{id:\"{}\",count:{},components:{{\"minecraft:custom_data\":{{{ITEM_ENERGY}:{},{SIDES}:[B;{bytes}]}}}}}}",
                drop.item,
                drop.count,
                energy.clamp(0, capacity)
            );
            if let Some(stack) = host.item_from_snbt(&snbt) {
                *drop = stack;
            }
        }
    }

    /// Placing a cube item restores the energy and side config it carries.
    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent) {
        if generator::is_multiblock(&event.stack.item) {
            place_multiblock(host, event.pos, &event.stack.item);
            return;
        }
        let Some((capacity, _)) = cube(&event.stack.item) else { return };
        let dimension = host.dimension().to_owned();
        self.broken.lock().unwrap().remove(&(dimension, event.pos));
        let Some(data) = event.stack.custom_data() else { return };
        let energy = read_energy(data, ITEM_ENERGY).clamp(0, capacity);
        let sides = read_sides(data);
        host.change_mod_block_entity(event.pos, &mut |m| {
            if cube(&m.id).is_none() {
                return;
            }
            if energy > 0 {
                write_int(&mut m.nbt, ENERGY, energy);
            }
            if let Some(sides) = sides {
                let k = key(&m.nbt, SIDES);
                m.nbt.insert(k, Tag::ByteArray(cube::sides_to_bytes(&sides)));
            }
        });
        if let Some(sides) = sides {
            update_ports(host, event.pos, &sides);
        }
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent) {
        let Some((capacity, _)) = cube(&event.stack.item) else { return };
        let stored = event.stack.custom_data().map_or(0, |d| read_energy(d, ITEM_ENERGY));
        let line = Text::translate_with_fallback(
            "gui.mekanism.energy_cube.stored",
            "Stored: %s / %s FE",
            &[Text::literal(cube::si(stored as i64)), Text::literal(cube::si(capacity as i64))],
        );
        event.lines.push((line, 0xFF_AAAAAA));
    }

    fn item_energy_buffer(&self, _host: &dyn Host, event: &mut ItemEnergyBufferEvent) -> Option<ItemEnergyBuffer> {
        if event.item == TABLET {
            return Some(ItemEnergyBuffer { capacity: TABLET_CAPACITY, max_insert: TABLET_RATE, max_extract: TABLET_RATE });
        }
        cube(event.item).map(|(capacity, rate)| ItemEnergyBuffer { capacity, max_insert: rate, max_extract: rate })
    }
}
