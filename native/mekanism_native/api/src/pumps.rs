//! The host side of the Electric Pump and the Fluidic Plenisher (M06): the
//! world reads and writes, the saved walk, the container, the tick and the
//! screen. Their Horizon-free rules and values are in `pump.rs`.
//!
//! The world: `Host::block` / `block_property` read a fluid block (a source has
//! `level` 0), `set_block` (flags 3) takes one away or places one, `Level::
//! is_loaded` keeps both out of unloaded chunks, `Level::game_rule` gives the
//! `water_source_conversion` / `lava_source_conversion` rules Mekanism's
//! "infinite source" rule reads. The pump pushes up into the tank above it with
//! `fluid_insert`, as `tanks.rs` does.

use horizon_client::mod_api::{
    BlockEntityTickEvent, Compound, Direction, GameRuleValue, Host, LevelRef, ModContainer, ModEnergyBuffer,
    ModFluidTanks, NativeScreen, ScreenDrawEvent, Tag,
};

use super::{chemicals, cube, facing, gauge, key, offset, pump, read_energy, tanks, write_int, ENERGY};
use pump::{Cell, Liquid, Pos};

/// Block entity NBT besides Horizon's `fluids` and `energy`: the operation
/// counter, and the walk (flat x, y, z lists) the pump or plenisher keeps.
const OPERATING: &str = "mekanism_operating";
const PUMP_TYPE: &str = "mekanism_pump_type";
const PUMP_NODES: &str = "mekanism_pump_nodes";
const PLEN_ACTIVE: &str = "mekanism_plenisher_active";
const PLEN_USED: &str = "mekanism_plenisher_used";
const PLEN_FINISHED: &str = "mekanism_plenisher_finished";
/// `Block.UPDATE_ALL`.
const UPDATE_ALL: i32 = 3;

pub(super) fn name(plenisher: bool) -> &'static str {
    if plenisher { "Fluidic Plenisher" } else { "Electric Pump" }
}

/// The pump gives its fluid out of the top; the plenisher takes it in through
/// the top (Mekanism's `IFluidHandler` sides). One tank of ten buckets.
pub(super) fn fluids(plenisher: bool) -> ModFluidTanks {
    let mut faces: [Vec<usize>; 6] = std::array::from_fn(|_| Vec::new());
    faces[cube::UP] = vec![0];
    ModFluidTanks {
        tanks: vec![pump::TANK],
        faces,
        inputs: if plenisher { vec![0] } else { Vec::new() },
        outputs: if plenisher { Vec::new() } else { vec![0] },
    }
}

/// One hidden placeholder slot; there are no item slots.
pub(super) fn container(plenisher: bool) -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some((if plenisher { pump::PLENISHER_MENU } else { pump::PUMP_MENU }).to_owned()),
    }
}

/// Takes energy on every side and gives none.
pub(super) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: pump::CAPACITY,
        max_insert: pump::CAPACITY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

pub(super) fn screen() -> NativeScreen {
    NativeScreen {
        width: pump::WIDTH,
        height: pump::HEIGHT,
        slots: vec![None],
        inventory_y: pump::INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: false,
    }
}

fn read_list(nbt: &Compound, name: &'static str) -> Vec<Pos> {
    match nbt.get(&key(nbt, name)) {
        Some(Tag::IntArray(v)) => v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
        _ => Vec::new(),
    }
}

fn write_list<'a>(nbt: &mut Compound, name: &'static str, list: impl Iterator<Item = &'a Pos>) {
    let flat: Vec<i32> = list.flat_map(|p| p.iter().copied()).collect();
    let k = key(nbt, name);
    if flat.is_empty() {
        nbt.remove(&k);
    } else if nbt.get(&k) != Some(&Tag::IntArray(flat.clone())) {
        nbt.insert(k, Tag::IntArray(flat));
    }
}

/// The world as the pump and plenisher see it: the entity's own level.
struct World<'a> {
    host: &'a mut dyn Host,
    level: LevelRef,
    dimension: String,
}

impl World<'_> {
    fn loaded(&self, pos: Pos) -> bool {
        self.level.is_loaded(&*self.host, pos)
    }

    fn set(&mut self, pos: Pos, block: &str) -> bool {
        let Some(state) = self.host.default_state(block) else { return false };
        self.host.set_block(&self.dimension, pos, state, UPDATE_ALL)
    }

    /// A fluid block's liquid and whether it is a source (`level` 0).
    fn liquid(&self, pos: Pos, block: &str) -> Option<(Liquid, bool)> {
        let liquid = Liquid::from_id(block)?;
        let source = self.host.block_property(pos, "level").is_none_or(|l| l == "0");
        Some((liquid, source))
    }
}

impl pump::PumpWorld for World<'_> {
    fn source(&self, pos: Pos) -> Option<Liquid> {
        if !self.loaded(pos) {
            return None;
        }
        let (liquid, source) = self.liquid(pos, &self.host.block(pos))?;
        source.then_some(liquid)
    }

    /// Mekanism's rule with `pumpInfiniteFluidSources` off: water that the world
    /// makes again (`water_source_conversion`, default on) and lava likewise
    /// (`lava_source_conversion`, default off) is not used up.
    fn infinite(&self, liquid: Liquid) -> bool {
        let (rule, default) = match liquid {
            Liquid::Water => ("water_source_conversion", true),
            Liquid::Lava => ("lava_source_conversion", false),
        };
        match self.level.game_rule(&*self.host, rule) {
            Some(GameRuleValue::Bool(b)) => b,
            _ => default,
        }
    }

    fn remove(&mut self, pos: Pos) {
        self.set(pos, "minecraft:air");
    }
}

/// Plants and the like that fluid pushes aside (`BlockState.canBeReplaced`);
/// Horizon names no such property, so this is a list.
const REPLACEABLE: [&str; 9] = [
    "minecraft:short_grass",
    "minecraft:tall_grass",
    "minecraft:fern",
    "minecraft:large_fern",
    "minecraft:dead_bush",
    "minecraft:vine",
    "minecraft:fire",
    "minecraft:soul_fire",
    "minecraft:glow_lichen",
];

impl pump::PlenWorld for World<'_> {
    fn cell(&self, pos: Pos) -> Cell {
        if !self.loaded(pos) {
            return Cell::Unloaded;
        }
        let block = self.host.block(pos);
        if matches!(block.as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") {
            return Cell::Air;
        }
        if let Some((_, source)) = self.liquid(pos, &block) {
            return if source { Cell::Source } else { Cell::Flowing };
        }
        if REPLACEABLE.contains(&block.as_str()) { Cell::Replaceable } else { Cell::Solid }
    }

    fn place(&mut self, pos: Pos, liquid: Liquid) -> bool {
        self.set(pos, liquid.id())
    }
}

/// Shuffles `nodes` in place (Fisher-Yates) from a small generator seeded by
/// the game time and the pump's place: Horizon's own random is not needed for
/// which of the pump's sources comes first.
fn shuffler(seed: u64) -> impl FnMut(&mut Vec<Pos>) {
    let mut state = seed | 1;
    move |nodes| {
        for i in (1..nodes.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            nodes.swap(i, (state % (i as u64 + 1)) as usize);
        }
    }
}

/// One tick of a pump or plenisher: run `pump::Pump::tick` / `Plenisher::tick`
/// on the saved state, push the pump's fluid up, write everything back.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, plenisher: bool) {
    let pos = event.pos;
    let dimension = host.dimension().to_owned();
    let Some(level) = host.level(&dimension) else { return };
    let nbt = &event.entity.nbt;
    let (fluid, amount) = tanks::read_fluid(nbt);
    let tank = pump::Tank::new(fluid, amount);
    let energy = read_energy(nbt, ENERGY).clamp(0, pump::CAPACITY);
    let operating = read_energy(nbt, OPERATING).max(0);
    let seed = level.game_time(&*host).unwrap_or(0) as u64 ^ ((pos[0] as u64) << 40 | (pos[2] as u64 & 0xFF_FFFF) << 16);
    let mut world = World { host: &mut *host, level, dimension };
    let (tank, energy, operating, used, kind, finished, saved) = if plenisher {
        let state = pump::PlenState {
            active: read_list(nbt, PLEN_ACTIVE),
            used: read_list(nbt, PLEN_USED).into_iter().collect(),
            finished: read_energy(nbt, PLEN_FINISHED) != 0,
        };
        let mut p = pump::Plenisher { energy, operating, tank, state };
        let used = pump::Plenisher::tick(&mut p, &mut world, pos);
        let finished = p.state.finished;
        (p.tank, p.energy, p.operating, used, None, finished, Saved::Plenisher(p.state))
    } else {
        let state = pump::PumpState {
            active: match read_energy(nbt, PUMP_TYPE) {
                1 => Some(Liquid::Water),
                2 => Some(Liquid::Lava),
                _ => None,
            },
            nodes: read_list(nbt, PUMP_NODES),
        };
        let mut p = pump::Pump { energy, operating, tank, state };
        let used = p.tick(&mut world, pos, &mut shuffler(seed));
        let kind = p.state.active.map(|l| l.id());
        (p.tank, p.energy, p.operating, used, kind, false, Saved::Pump(p.state))
    };
    let mut tank = tank;
    if !plenisher && tank.amount > 0 {
        // Up into the block above, through its bottom face, as much as it takes.
        if let Some(id) = tank.id.clone() {
            let offer = tank.amount.min(pump::PUMP_RATE);
            let left = host.fluid_insert(offset(pos, cube::UP), Direction::ALL[cube::DOWN], &id, offer);
            tank.amount -= (offer - left).clamp(0, offer);
            if tank.amount == 0 {
                tank.id = None;
            }
        }
    }

    let e = &mut *event.entity;
    write_int(&mut e.nbt, ENERGY, energy);
    write_int(&mut e.nbt, OPERATING, operating);
    tanks::write_fluid(&mut e.nbt, tank.id.as_deref(), tank.amount);
    match saved {
        Saved::Pump(state) => {
            write_int(&mut e.nbt, PUMP_TYPE, state.active.map_or(0, |l| if l == Liquid::Water { 1 } else { 2 }));
            write_list(&mut e.nbt, PUMP_NODES, state.nodes.iter());
        }
        Saved::Plenisher(state) => {
            write_int(&mut e.nbt, PLEN_FINISHED, state.finished as i32);
            // A finished plenisher only refills below: it needs no walk any more.
            let keep = !state.finished;
            write_list(&mut e.nbt, PLEN_ACTIVE, state.active.iter().filter(|_| keep));
            let mut used: Vec<Pos> = state.used.iter().copied().filter(|_| keep).collect();
            used.sort_unstable();
            write_list(&mut e.nbt, PLEN_USED, used.iter());
        }
    }
    let data = pump::menu_data(energy, &tank, kind, used, facing(host, pos), finished);
    if e.data.as_slice() != data.as_slice() {
        e.data = data.to_vec();
    }
}

enum Saved {
    Pump(pump::PumpState),
    Plenisher(pump::PlenState),
}

/// The screen: Mekanism's layout (gauge left, status screen, energy bar right).
pub(super) fn draw(host: &dyn Host, event: &mut ScreenDrawEvent, plenisher: bool) {
    let [x, y] = event.origin;
    let d = |i: usize| event.data.get(i).copied().unwrap_or(0);
    let title = host.screen_title().unwrap_or_else(|| name(plenisher).to_owned());
    host.text(event.batch, &title, [x + 8.0, y + 6.0], 0x404040, false);
    let entry = usize::try_from(d(pump::DATA_KIND)).ok().and_then(|i| i.checked_sub(1)).and_then(chemicals::entry);
    let [gx, gy, gw, gh] = pump::GAUGE;
    gauge(host, event.batch, [x + gx, y + gy, gw, gh], d(pump::DATA_AMOUNT) as f32 / pump::TANK as f32, entry.map_or(0, |e| e.1));
    let [bx, by, bw, bh] = pump::ENERGY_BAR;
    gauge(host, event.batch, [x + bx, y + by, bw, bh], d(pump::DATA_ENERGY) as f32 / pump::CAPACITY as f32, 0xFF50C878);
    let [sx, sy, sw, sh] = pump::INNER_SCREEN;
    host.fill(event.batch, [x + sx, y + sy, sw, sh], 0xFF242A2F);
    let status = match (plenisher, d(pump::DATA_USED) != 0, d(pump::DATA_FINISHED) != 0) {
        (true, _, true) => "Finished",
        (true, true, false) => "Walking",
        (false, true, _) => "Pumping",
        _ => "Idle",
    };
    host.text(event.batch, status, [x + sx + 4.0, y + sy + 4.0], 0x3FA9F5, false);
    if let Some(e) = entry {
        host.text(event.batch, e.0, [x + sx + 4.0, y + sy + 16.0], 0x3FA9F5, false);
    }
    host.text(event.batch, &format!("{} mB", d(pump::DATA_AMOUNT)), [x + sx + 4.0, y + sy + 28.0], 0x3FA9F5, false);
    host.text(event.batch, "Inventory", [x + 8.0, y + pump::INVENTORY_Y - 10.0], 0x404040, false);
}
