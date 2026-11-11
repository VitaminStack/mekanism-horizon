//! Mekanism for Horizon: the native half. Blocks, items, menus and recipes are
//! registered by the separate content mods (`mekanism`, `mekanismgenerators`).
//!
//! Every block this mod runs is one `Kind`, found by `kind`; each hook matches
//! on it once. Modules:
//! - `cube`: energy cube tiers, side config and menu data (Horizon-free).
//! - `energy`: which blocks take energy from a neighbour, and how much (Horizon-free).
//! - `machine`, `infuser`: the processing machines' table and operation (Horizon-free);
//!   `process`: their tick, container and screen; `recipe`: Mekanism's recipe files.
//! - `generator` (Horizon-free) and `generators`: Heat, Solar, Bio, Advanced
//!   Solar and Wind Generator.
//! - `chemicals`, `tank` (Horizon-free) and `tanks`: fluids, chemicals and the
//!   Fluid and Chemical Tanks (M06).
//! - `gear` (Horizon-free) and `items`: Dictionary, Canteen, Gauge Dropper and Atomic
//!   Disassembler (M11; hooks in docs/ITEM_HOOKS.md).
//!
//! Hooks used (native hooks API; see Horizon docs/native-mods.md):
//! - `block_entity` (API 15): a saved block entity for each block.
//! - `block_entity_energy` (API 142): the energy buffer. Horizon saves the
//!   amount itself in the block entity NBT as `energy` (Tag::Int).
//! - `block_entity_ticks` / `block_entity_tick` (API 15/17): cubes eject from
//!   their output sides, machines run, generators produce and push.
//! - `state_for_placement` (API 76): the front faces the placer.
//! - `block_entity_container` (API 17) + content menu type: a mod block
//!   menu opened by Horizon's normal block-use path, including singleplayer.
//! - `screen`, `screen_draw`, `screen_click`, `menu_button` (API 19/20): GUIs.
//! - `destroy_block`, `loot_drops`, `place_block` (API 14/75): a broken cube's
//!   item keeps its energy and side config, and placing it restores them.
//! - `item_energy_buffer` (API 307): the Energy Tablet's and the cube items'
//!   buffers, saved by Horizon in the stack's `minecraft:custom_data` as `horizon_energy`.
//! - `tooltip` (API 12): a cube item's stored energy.
//! - `recipes_of_type`/`recipe_ingredient`/`recipe_result` (API 36): recipes.
//! - `block_entity_fluids`, `block_entity_accepts_fluid` (API 54), `right_click_block`
//!   (API 21): the tanks' contents and buckets (`tanks.rs`).
//! - `Host::play_sound` (API 61): machines and generators loop Mekanism's
//!   sound while running, replayed when the clip ends.
//!
//! Energy is Horizon's i32 (see `energy.rs` and docs/ENERGY-UNITS.md).
//!
//! Known API gaps (see docs/ENERGY-CUBE.md): Horizon's energy faces are per block
//! type, not per block entity, so other mods may still fill or drain any side;
//! a trait mod has no energy helper for other mods' blocks, so pushes reach
//! only Mekanism blocks (docs/upstream/horizon-energy-host.md).

mod chemicals;
mod cube;
mod dictionary_tags;
mod energy;
mod gear;
mod generator;
mod generators;
mod infuser;
mod items;
mod machine;
mod process;
mod recipe;
mod tank;
mod tanks;

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use cube::{Mode, Sides};
use horizon_client::mod_api::{
    BlockEntityAcceptsFluidEvent, BlockEntityContainerEvent, BlockEntityEnergyEvent, BlockEntityEvent,
    BlockEntityFluidsEvent, BlockEntityRenderBoundingBoxEvent, BlockEntitySubmitEvent, BlockEntityTickEvent,
    BlockEntityTicksEvent, BlockEntityUpdateTagEvent, Compound, DestroyBlockEvent, Flow, Hooks, Host,
    ItemEnergyBuffer, ItemEnergyBufferEvent, ItemStack, LevelHost, LootDropsEvent, MenuButtonEvent,
    ModContainer, ModEnergyBuffer, ModFluidTanks, NativeScreen, PlaceBlockEvent, ScreenClickEvent,
    ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, StandaloneModelsEvent, StateForPlacementEvent, Tag,
    Text, TooltipEvent, RightClickBlockEvent, CanEatEvent, InventoryTickEvent, ItemBarEvent, OnEquipEvent,
    LivingEntityUseItemFinishEvent, LivingEntityUseItemStartEvent, LivingIncomingDamageV2Event,
    UseAnimationEvent, UseDurationEvent, UseItemEvent,
};
use machine::Machine;

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
/// A processing machine's progress in ticks.
const PROGRESS: &str = "mekanism_progress";
/// Registered by `data/mekanism/horizon/menu/energy_cube.json`.
const CUBE_MENU: &str = "mekanism:energy_cube";
/// `SoundSource.BLOCKS` and `WEATHER` ordinals.
const SOUND_BLOCKS: usize = 4;
const SOUND_WEATHER: usize = 3;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(MekanismNative::default())
}

/// What a block of this mod is.
#[derive(Clone, Copy)]
enum Kind {
    Cube { capacity: i32, rate: i32 },
    Debug { source: bool },
    Machine(&'static Machine),
    /// One of `generators::ALL`.
    Generator(&'static str),
    /// A multiblock's bounding block (`generator::BOUNDING`).
    Bounding,
    Tank(&'static tank::Tier),
    /// `tanks::DEBUG_CHEMICAL_SOURCE`.
    DebugChemical,
}

fn kind(block: &str) -> Option<Kind> {
    if let Some((capacity, rate)) = cube::tier(block) {
        return Some(Kind::Cube { capacity, rate });
    }
    if block == DEBUG_SOURCE || block == DEBUG_SINK {
        return Some(Kind::Debug { source: block == DEBUG_SOURCE });
    }
    if let Some(m) = machine::find(block) {
        return Some(Kind::Machine(m));
    }
    if block == generator::BOUNDING {
        return Some(Kind::Bounding);
    }
    if let Some(t) = tank::tier_of(block) {
        return Some(Kind::Tank(t));
    }
    if block == tanks::DEBUG_CHEMICAL_SOURCE {
        return Some(Kind::DebugChemical);
    }
    generators::find(block).map(Kind::Generator)
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

/// Each block's state is synchronised by its block-entity menu data.
/// Unlike `open_menu`, the block menu works for local and remote players.
#[derive(Default)]
struct MekanismNative {
    /// Energy and sides of cubes a player is breaking, for their drop.
    broken: Mutex<HashMap<(String, [i32; 3]), (i32, Sides)>>,
    /// Contents and mode of tanks a player is breaking, for their drop.
    broken_tanks: Mutex<HashMap<(String, [i32; 3]), tanks::Contents>>,
    recipes: Mutex<recipe::Recipes>,
    /// Ticks until each running block's loop sound plays again (not saved).
    hums: Mutex<HashMap<(String, [i32; 3]), i32>>,
    /// The hand-held gear's own state (`items.rs`).
    gear: items::State,
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

    fn machine_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent, m: &Machine) {
        let active = process::tick(host, event, m, &mut self.recipes.lock().unwrap());
        // After the lock is released: a neighbour's `accepts` hook takes it too.
        process::eject_output(host, event, m);
        self.hum(host, event.pos, m.sound, m.sound_ticks, active);
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

    /// A cube ejects from its output sides; the creative cube stays full.
    fn cube_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent, capacity: i32, rate: i32) {
        let pos = event.pos;

        // Side config: Mekanism's default until the cube has its own.
        let saved = read_sides(&event.entity.nbt);
        let sides = saved.unwrap_or(cube::DEFAULT_SIDES);
        if saved.is_none() {
            let k = key(&event.entity.nbt, SIDES);
            event.entity.nbt.insert(k, Tag::ByteArray(cube::sides_to_bytes(&sides)));
            update_ports(host, pos, &sides);
        }

        let creative = event.entity.id == cube::CREATIVE;
        let mut stored = read_energy(&event.entity.nbt, ENERGY).clamp(0, capacity);
        if creative {
            // The creative cube never runs dry.
            stored = capacity;
        }
        let ejected = self.eject(host, pos, rate, &mut stored, &sides);
        if creative {
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
}

/// The debug source fills neighbouring cubes, the sink drains them, each by
/// the cube's own rate per tick.
fn debug_tick(host: &mut dyn Host, pos: [i32; 3], source: bool) {
    for dir in 0..6 {
        host.change_mod_block_entity(offset(pos, dir), &mut |m| {
            let Some((capacity, rate)) = cube::tier(&m.id) else { return };
            let stored = read_energy(&m.nbt, ENERGY).clamp(0, capacity);
            let next = if source { stored + rate.min(capacity - stored) } else { stored - rate.min(stored) };
            write_int(&mut m.nbt, ENERGY, next);
        });
    }
}

/// Move up to `budget` of `stored` into the block on side `dir` of `pos`, if
/// it takes energy there (`energy::acceptor`: a cube through a side set to
/// input, a machine on every side). Returns what moved: what the neighbour
/// gains the giver loses, so none is lost or made.
fn give(host: &mut dyn Host, pos: [i32; 3], dir: usize, budget: i32, stored: i32) -> i32 {
    if budget <= 0 || stored <= 0 {
        return 0;
    }
    let at = offset(pos, dir);
    let their_facing = facing(host, at);
    let mut moved = 0;
    host.change_mod_block_entity(at, &mut |m| {
        let Some(acceptor) = energy::acceptor(&m.id) else { return };
        if acceptor.sided {
            let their_side = cube::relative(cube::opposite(dir), their_facing);
            if read_sides(&m.nbt).unwrap_or(cube::DEFAULT_SIDES)[their_side] != Mode::Input {
                return;
            }
        }
        let theirs = read_energy(&m.nbt, ENERGY);
        moved = acceptor.accept(budget, stored, theirs);
        if moved > 0 {
            write_int(&mut m.nbt, ENERGY, theirs + moved);
        }
    });
    moved
}

/// The energy cube's screen: stored energy, throughput and its six side buttons.
fn draw_cube(host: &dyn Host, event: &mut ScreenDrawEvent, capacity: i32, rate: i32) {
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

impl Hooks for MekanismNative {
    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent) -> Option<(String, Compound)> {
        kind(event.block).map(|_| (event.block.to_owned(), Compound::new()))
    }

    /// The block's own menu, not the LAN-only `Host::open_menu` helper.
    fn block_entity_container(&self, _host: &dyn Host, event: &mut BlockEntityContainerEvent) -> Option<ModContainer> {
        match kind(event.block)? {
            Kind::Machine(m) => Some(process::container(m)),
            Kind::Generator(g) => Some(generators::container(g)),
            Kind::Bounding => Some(generators::bounding_container()),
            Kind::Tank(t) => Some(tanks::container(t)),
            // One hidden placeholder slot; all interactions are buttons.
            Kind::Cube { .. } => Some(ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| Vec::new()),
                inputs: Vec::new(),
                outputs: Vec::new(),
                menu: Some(CUBE_MENU.to_owned()),
            }),
            Kind::Debug { .. } | Kind::DebugChemical => None,
        }
    }

    fn block_entity_energy(&self, _host: &dyn Host, event: &mut BlockEntityEnergyEvent) -> Option<ModEnergyBuffer> {
        match kind(event.block)? {
            // Machines take energy on every side and give none.
            Kind::Machine(m) => Some(ModEnergyBuffer {
                capacity: m.capacity(),
                max_insert: m.capacity(),
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            }),
            Kind::Generator(g) => Some(generators::energy_buffer(g)),
            Kind::Cube { capacity, rate } => Some(ModEnergyBuffer {
                capacity,
                max_insert: rate,
                max_extract: rate,
                insert_faces: [true; 6],
                extract_faces: [true; 6],
            }),
            Kind::Debug { .. } | Kind::Bounding | Kind::Tank(_) | Kind::DebugChemical => None,
        }
    }

    /// One tank per tank block, reached from every side.
    fn block_entity_fluids(&self, _host: &dyn Host, event: &mut BlockEntityFluidsEvent) -> Option<ModFluidTanks> {
        match kind(event.block)? {
            Kind::Tank(t) => Some(tanks::fluids(t)),
            Kind::Machine(m) => m.tanks.map(process::fluids),
            _ => None,
        }
    }

    /// Fluid tanks take no chemicals and chemical tanks no fluids.
    fn block_entity_accepts_fluid(&self, host: &dyn Host, event: &mut BlockEntityAcceptsFluidEvent) -> Flow {
        match kind(event.block) {
            Some(Kind::Tank(t)) if !tank::accepts(t.family, event.fluid) => Flow::Cancel,
            Some(Kind::Machine(m)) if m.tanks.is_some()
                && !process::accepts(host, m, &mut self.recipes.lock().unwrap(), event.tank, event.fluid) => Flow::Cancel,
            _ => Flow::Continue,
        }
    }

    /// Buckets on a fluid tank fill or empty it instead of opening its screen.
    fn right_click_block(&self, host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        if matches!(self.gear.right_click_block(host, event.pos, event.face), Flow::Cancel) {
            return Flow::Cancel;
        }
        let Some(Kind::Tank(t)) = kind(&host.block(event.pos)) else { return Flow::Continue };
        let Some((item, _, _)) = host.held() else { return Flow::Continue };
        if tanks::use_bucket(host, event.pos, event.face, &item, t) { Flow::Cancel } else { Flow::Continue }
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent) {
        if kind(event.block).is_some() {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent) {
        match kind(&event.entity.id) {
            Some(Kind::Machine(m)) => self.machine_tick(host, event, m),
            Some(Kind::Generator(g)) => self.generator_tick(host, event, g),
            Some(Kind::Cube { capacity, rate }) => self.cube_tick(host, event, capacity, rate),
            Some(Kind::Debug { source }) => debug_tick(host, event.pos, source),
            Some(Kind::Bounding) => generators::bounding_tick(host, event),
            Some(Kind::Tank(t)) => tanks::tick(host, event, t),
            Some(Kind::DebugChemical) => tanks::debug_tick(host, event.pos),
            None => {}
        }
    }

    /// Mekanism turns the cube's front toward its placer; machines, generators
    /// and chemical tanks face the placer horizontally (Mekanism's
    /// `AttributeStateFacing`); fluid tanks keep the template's.
    fn state_for_placement(&self, level: &dyn LevelHost, event: &mut StateForPlacementEvent) -> Option<Option<horizon_client::mod_api::BlockStateId>> {
        let pitch = match kind(&event.context.item)? {
            Kind::Cube { .. } => event.context.pitch,
            Kind::Machine(_) | Kind::Generator(_) => 0.0,
            Kind::Tank(t) if t.family == tank::Family::Chemical => 0.0,
            Kind::Debug { .. } | Kind::Bounding | Kind::Tank(_) | Kind::DebugChemical => return None,
        };
        let facing = cube::facing_from_look(event.context.yaw, pitch);
        level
            .state_with_property(event.state, "facing", cube::DIR_NAMES[facing])
            .map(Some)
    }

    /// A Horizon mod screen replaces the default inventory artwork when the
    /// content menu for this block opens.
    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent) -> Option<NativeScreen> {
        match kind(event.block)? {
            Kind::Machine(m) => Some(process::screen(m)),
            Kind::Generator(g) => Some(generators::screen(g)),
            // Only the Advanced Solar and the Wind Generator are multiblocks: one layout.
            Kind::Bounding => Some(generators::screen(generator::WIND)),
            Kind::Tank(_) => Some(tanks::screen()),
            // The cube's dummy container slot is hidden.
            Kind::Cube { .. } => Some(NativeScreen {
                width: 176.0,
                height: 238.0,
                slots: vec![None],
                inventory_y: 156.0,
                inventory_x: 8.0,
                own_labels: true,
                own_background: false,
            }),
            Kind::Debug { .. } | Kind::DebugChemical => None,
        }
    }

    /// Drawn on top of Horizon's plain panel; no item slots are used for labels.
    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent) {
        match kind(event.block) {
            Some(Kind::Machine(m)) => process::draw(host, event, m),
            Some(Kind::Generator(g)) => generators::draw(host, event, g),
            Some(Kind::Bounding) => {
                if let Some(g) = generators::bounding_generator(event.data) {
                    generators::draw(host, event, g);
                }
            }
            Some(Kind::Cube { capacity, rate }) => draw_cube(host, event, capacity, rate),
            Some(Kind::Tank(t)) => tanks::draw(host, event, t),
            _ => {}
        }
    }

    /// Mouse hits a button, not an inventory slot. The click is sent via a
    /// ContainerButtonClick packet and handled in `menu_button`.
    fn screen_click(&self, _host: &dyn Host, event: &mut ScreenClickEvent) -> Option<i32> {
        match kind(event.block)? {
            Kind::Machine(m) => process::click(m, event.at),
            Kind::Cube { .. } => cube::side_button_at(event.at, event.right),
            Kind::Tank(_) => tank::button_at(event.at, event.right),
            _ => None,
        }
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut ScreenTooltipEvent) {
        match kind(event.block) {
            Some(Kind::Machine(m)) => event.lines.extend(machine::tooltip(m, event.data, event.at)),
            Some(Kind::Generator(g)) => event.lines.extend(generators::tooltip(g, event.data, event.at)),
            Some(Kind::Bounding) => {
                if let Some(g) = generators::bounding_generator(event.data) {
                    event.lines.extend(generators::tooltip(g, event.data, event.at));
                }
            }
            Some(Kind::Tank(t)) => event.lines.extend(tank::tooltip(t, event.data, event.at)),
            _ => {}
        }
    }

    /// Button presses update the actual block entity on the server, then
    /// normal ContainerData sync refreshes the screen for every player.
    fn menu_button(&self, host: &mut dyn Host, event: &mut MenuButtonEvent) {
        match kind(&event.entity.id) {
            Some(Kind::Machine(m)) => process::button(m, &mut event.entity.nbt, event.id),
            Some(Kind::Tank(t)) => {
                if let Some(data) = tanks::button(t, &mut event.entity.nbt, event.id) {
                    event.entity.data = data;
                }
            }
            Some(Kind::Cube { .. }) => {
                let Some((rel, forward)) = cube::side_button_action(event.id) else { return };
                let mut sides = read_sides(&event.entity.nbt).unwrap_or(cube::DEFAULT_SIDES);
                sides[rel] = sides[rel].cycle(forward);
                let k = key(&event.entity.nbt, SIDES);
                event.entity.nbt.insert(k, Tag::ByteArray(cube::sides_to_bytes(&sides)));
                update_ports(host, event.pos, &sides);
                let ejected = cube::read_amount(&event.entity.data, cube::DATA_EJECTED);
                let stored = read_energy(&event.entity.nbt, ENERGY);
                event.entity.data = cube::menu_data(stored, ejected, facing(host, event.pos), &sides).to_vec();
            }
            _ => {}
        }
    }

    /// The Wind Generator's tower and blades (Mekanism's `RenderWindGenerator`) and the
    /// Advanced Solar Generator's top, drawn by their block entities.
    fn standalone_models(&self, event: &mut StandaloneModelsEvent) {
        event.models.extend(generator::WIND_PARTS.iter().map(|(model, _, _)| (*model).to_owned()));
        event.models.push(generator::ADVANCED_SOLAR_TOP.to_owned());
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent) -> Option<Compound> {
        if let Some(Kind::Tank(t)) = kind(&event.entity.id) {
            return tanks::update_tag(&event.entity.nbt, t);
        }
        if &*event.entity.id != generator::WIND {
            return None;
        }
        let mut tag = Compound::default();
        write_int(&mut tag, generators::WIND_ACTIVE, read_energy(&event.entity.nbt, generators::WIND_ACTIVE));
        Some(tag)
    }

    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent) {
        if event.block == generator::WIND {
            let active = read_energy(&event.entity.nbt, generators::WIND_ACTIVE) != 0;
            generators::draw_wind(host, event.pos, active, event.draw);
        } else if event.block == generator::ADVANCED_SOLAR {
            generators::draw_advanced_solar_top(host, event.pos, event.draw);
        } else if let Some(Kind::Tank(t)) = kind(event.block) {
            tanks::draw_contents(host, &event.entity.nbt, event.draw, t);
        }
    }

    fn block_entity_render_bounding_box(&self, _host: &dyn Host, event: &mut BlockEntityRenderBoundingBoxEvent) {
        if event.block == generator::WIND {
            // The blades reach about four blocks round the hub, four and a half up.
            *event.bounds = [-4.5, 0.0, -4.5, 5.5, 9.0, 5.5];
        } else if event.block == generator::ADVANCED_SOLAR {
            // The panels reach a block past each side, three blocks up.
            *event.bounds = [-1.0, 0.0, -1.0, 2.0, 3.0, 2.0];
        }
    }

    /// Remember what a cube or tank holds before a player breaks it, for its drop.
    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        if matches!(self.gear.destroy_block(host, event), Flow::Cancel) {
            return Flow::Cancel;
        }
        self.hums.lock().unwrap().remove(&(host.dimension().to_owned(), event.pos));
        // Breaking a multiblock takes its bounding blocks with it; breaking a
        // bounding block breaks the main block, which drops (Mekanism's
        // BlockBounding), except for a player in creative mode.
        if host.block(event.pos) == generator::BOUNDING {
            if let Some((block, main)) = generators::bounding_main(host, event.pos) {
                generators::remove_bounding(host, main, block, Some(event.pos));
                let creative = host.acting_player().is_some_and(|p| p.game_mode == 1);
                // Collected drops are not spawned: a creative break drops nothing.
                let _ = host.destroy_block(main, None, creative);
            }
        } else if let Some(entity) = host.mod_block_entity(event.pos) {
            if let Some(block) = generators::find(&entity.id).filter(|g| generator::is_multiblock(g)) {
                generators::remove_bounding(host, event.pos, block, None);
            }
        }
        if let Some(entity) = host.mod_block_entity(event.pos) {
            let place = (host.dimension().to_owned(), event.pos);
            match kind(&entity.id) {
                Some(Kind::Cube { .. }) => {
                    let sides = read_sides(&entity.nbt).unwrap_or(cube::DEFAULT_SIDES);
                    self.broken.lock().unwrap().insert(place, (read_energy(&entity.nbt, ENERGY), sides));
                }
                Some(Kind::Tank(_)) => {
                    self.broken_tanks.lock().unwrap().insert(place, tanks::contents(&entity.nbt));
                }
                _ => {}
            }
        }
        Flow::Continue
    }

    /// A broken cube drops an item that keeps its energy and side config
    /// (Mekanism's loot table copies `mekanism:energy` and `mekanism:side_config`).
    fn loot_drops(&self, host: &mut dyn Host, event: &mut LootDropsEvent) {
        let Some((pos, _)) = event.context.block else { return };
        let place = (event.context.dimension.to_owned(), pos);
        if let Some(contents) = self.broken_tanks.lock().unwrap().remove(&place) {
            tanks::fill_drops(host, &mut event.drops, contents);
        }
        let remembered = self.broken.lock().unwrap().remove(&place);
        let held = host
            .mod_block_entity(pos)
            .filter(|e| cube::tier(&e.id).is_some())
            .map(|e| (read_energy(&e.nbt, ENERGY), read_sides(&e.nbt).unwrap_or(cube::DEFAULT_SIDES)))
            .or(remembered);
        let Some((energy, sides)) = held else { return };
        for drop in event.drops.iter_mut() {
            let Some((capacity, _)) = cube::tier(&drop.item) else { continue };
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

    /// Placing a cube or tank item restores what it carries.
    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent) {
        if generator::is_multiblock(&event.stack.item) {
            generators::place_multiblock(host, event.pos, &event.stack.item);
            return;
        }
        if let Some(t) = tank::tier_of(&event.stack.item) {
            self.broken_tanks.lock().unwrap().remove(&(host.dimension().to_owned(), event.pos));
            tanks::place(host, event.pos, &event.stack, t);
            return;
        }
        let Some((capacity, _)) = cube::tier(&event.stack.item) else { return };
        let dimension = host.dimension().to_owned();
        self.broken.lock().unwrap().remove(&(dimension, event.pos));
        let Some(data) = event.stack.custom_data() else { return };
        let energy = read_energy(data, ITEM_ENERGY).clamp(0, capacity);
        let sides = read_sides(data);
        host.change_mod_block_entity(event.pos, &mut |m| {
            if cube::tier(&m.id).is_none() {
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
        let gear = self.gear.tooltip(&event.stack);
        if !gear.is_empty() {
            event.lines.extend(gear);
            return;
        }
        if let Some(t) = tank::tier_of(&event.stack.item) {
            event.lines.push(tanks::tooltip(&event.stack, t));
            return;
        }
        let Some((capacity, _)) = cube::tier(&event.stack.item) else { return };
        let stored = event.stack.custom_data().map_or(0, |d| read_energy(d, ITEM_ENERGY));
        let line = Text::translate_with_fallback(
            "gui.mekanism.energy_cube.stored",
            "Stored: %s / %s FE",
            &[Text::literal(cube::si(stored as i64)), Text::literal(cube::si(capacity as i64))],
        );
        event.lines.push((line, 0xFF_AAAAAA));
    }

    fn item_energy_buffer(&self, _host: &dyn Host, event: &mut ItemEnergyBufferEvent) -> Option<ItemEnergyBuffer> {
        if let Some((capacity, rate)) = items::energy_buffer(event.item) {
            return Some(ItemEnergyBuffer { capacity, max_insert: rate, max_extract: rate });
        }
        if event.item == TABLET {
            return Some(ItemEnergyBuffer { capacity: TABLET_CAPACITY, max_insert: TABLET_RATE, max_extract: TABLET_RATE });
        }
        cube::tier(event.item).map(|(capacity, rate)| ItemEnergyBuffer { capacity, max_insert: rate, max_extract: rate })
    }
    // Hand-held gear (M11), `items.rs`.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        self.gear.use_item(host, event.stack)
    }

    fn on_equip(&self, host: &mut dyn Host, event: &mut OnEquipEvent<'_>) {
        self.gear.on_equip(host, event);
    }

    fn inventory_tick(&self, host: &mut dyn Host, event: &mut InventoryTickEvent) {
        self.gear.inventory_tick(host, event);
    }

    fn living_incoming_damage_v2(&self, host: &mut dyn Host, event: &mut LivingIncomingDamageV2Event<'_>) -> Flow {
        self.gear.damage(host, event);
        Flow::Continue
    }

    fn living_entity_use_item_start(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemStartEvent<'_>) -> Flow {
        self.gear.use_start(host, event)
    }

    fn living_entity_use_item_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        self.gear.use_finish(host, event);
    }

    fn use_duration(&self, _host: &dyn Host, event: &mut UseDurationEvent<'_>) {
        self.gear.use_duration(event);
    }

    fn use_animation(&self, _host: &dyn Host, event: &mut UseAnimationEvent<'_>) {
        self.gear.use_animation(event);
    }

    fn can_eat(&self, host: &dyn Host, event: &mut CanEatEvent<'_>) {
        self.gear.can_eat(host, event);
    }

    fn item_bar(&self, _host: &dyn Host, event: &mut ItemBarEvent<'_>) {
        self.gear.bar(event);
    }
}
