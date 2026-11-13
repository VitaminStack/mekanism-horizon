//! The host side of Mechanical Pipes, Pressurized Tubes and Thermodynamic
//! Conductors (M06): the tick, the tank and the connection arms. Their
//! Horizon-free rules and values are in `transmitter.rs`.
//!
//! A pipe or tube is a block entity with one Horizon block-entity tank
//! (`block_entity_fluids`, API 54) that takes only its own family
//! (`block_entity_accepts_fluid`) and offers nothing to neighbours that ask;
//! it pushes into emptier neighbours itself with `fluid_insert`. A conductor
//! keeps its heat in `mekanism_heat` (the Heat Generator's key) and trades it
//! with neighbours through `change_mod_block_entity`, as the energy cubes do.
//! The six boolean block states `down`..`east` show an arm to every
//! neighbour the block joins; they are set from the tick.

use horizon_client::mod_api::{BlockEntityTickEvent, Direction, Host, ModFluidTanks, Tag};

use super::{cube, generator, generators, key, offset, tank, tanks, transmitter};
use transmitter::{Cell, Kind, Neighbour, Tier};

/// The conductor's heat in Kelvin times its capacity (1), a `Tag::Double`.
const HEAT: &str = generators::HEAT;

fn direction(dir: usize) -> Direction {
    Direction::ALL[dir]
}

/// One tank of the tier's capacity, reached from every side; neighbours may
/// fill it but not empty it (a pipe moves its own contents).
pub(super) fn fluids(t: &Tier) -> ModFluidTanks {
    ModFluidTanks {
        tanks: vec![t.capacity],
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: Vec::new(),
    }
}

/// Whether the neighbour at `dir` gets an arm: another transmitter of the
/// kind, a block with a tank (pipes and tubes) or a Heat Generator (conductors).
fn connects(host: &dyn Host, pos: [i32; 3], dir: usize, t: &Tier) -> bool {
    let np = offset(pos, dir);
    let block = host.block(np);
    if t.joins(&block) {
        return true;
    }
    match t.kind {
        Kind::Conductor => block == generator::HEAT,
        Kind::Pipe | Kind::Tube => {
            // A tank of the other family (a fluid tank next to a tube) gets no arm.
            tank::tier_of(&block).is_none_or(|b| Some(b.family) == t.family())
                && host
                    .fluid_tanks(host.dimension(), np, direction(cube::opposite(dir)))
                    .is_some_and(|tanks| !tanks.is_empty())
        }
    }
}

/// Sets each arm's block state, only where it changed.
fn show_arms(host: &mut dyn Host, pos: [i32; 3], t: &Tier) {
    for dir in 0..6 {
        let want = if connects(host, pos, dir, t) { "true" } else { "false" };
        if host.block_property(pos, cube::DIR_NAMES[dir]).as_deref() != Some(want) {
            host.set_block_property(pos, cube::DIR_NAMES[dir], want);
        }
    }
}

/// One tick of any of the three kinds.
pub(super) fn tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &Tier) {
    show_arms(host, event.pos, t);
    match t.kind {
        Kind::Conductor => heat_tick(host, event, t),
        Kind::Pipe | Kind::Tube => push_tick(host, event, t),
    }
}

/// Offers the buffer to the emptier neighbours (`transmitter::push_plan`).
fn push_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &Tier) {
    let pos = event.pos;
    let (fluid, amount) = tanks::read_fluid(&event.entity.nbt);
    let Some(id) = fluid.filter(|_| amount > 0) else { return };
    let dimension = host.dimension().to_owned();
    let mut neighbours = Vec::new();
    for dir in 0..6 {
        let Some(tanks) = host.fluid_tanks(&dimension, offset(pos, dir), direction(cube::opposite(dir))) else {
            continue;
        };
        // Only the tanks that could take this fluid count.
        let (mut held, mut capacity) = (0, 0);
        for tank in tanks.iter().filter(|tk| tk.fluid.as_deref().is_none_or(|f| f == id)) {
            held += tank.amount;
            capacity += tank.capacity;
        }
        if capacity > 0 {
            neighbours.push(Neighbour { dir, amount: held, capacity, sink: !t.joins(&host.block(offset(pos, dir))) && dir != cube::UP });
        }
    }
    let mut left = amount;
    for (dir, offer) in transmitter::push_plan(amount, t.capacity, t.rate, &neighbours) {
        let refused = host.fluid_insert(offset(pos, dir), direction(cube::opposite(dir)), &id, offer);
        left -= (offer - refused).clamp(0, offer);
    }
    tanks::write_fluid(&mut event.entity.nbt, Some(&id), left);
}

fn write_heat(nbt: &mut horizon_client::mod_api::Compound, heat: f64) {
    let k = key(nbt, HEAT);
    let tag = Tag::Double(heat.to_bits());
    if nbt.get(&k) != Some(&tag) {
        nbt.insert(k, tag);
    }
}

/// `ITileHeatHandler.simulate` for one conductor: heat to each warmer-than-the-
/// neighbour side (conductors and Heat Generators; a generator that is warmer
/// than the conductor warms it too), then the loss to the air.
fn heat_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent, t: &Tier) {
    let pos = event.pos;
    let ambient = generator::ambient_temp(generator::climate(host.biome(pos).as_deref()).0 as f64);
    let mut cell = Cell::conductor(t, generators::read_double(&event.entity.nbt, HEAT), ambient);
    for dir in 0..6 {
        host.change_mod_block_entity(offset(pos, dir), &mut |m| {
            if let Some(other) = transmitter::tier_of(&m.id).filter(|o| o.kind == Kind::Conductor) {
                let mut sink = Cell::conductor(other, generators::read_double(&m.nbt, HEAT), ambient);
                if cell.conduct_into(&mut sink) > 0.0 {
                    write_heat(&mut m.nbt, sink.heat);
                }
            } else if m.id == generator::HEAT {
                let heat = generators::read_double(&m.nbt, HEAT).unwrap_or(generator::HEAT_CAPACITY * ambient);
                let mut source = Cell::generator(heat);
                let moved = cell.conduct_into(&mut source) + source.conduct_into(&mut cell);
                if moved > 0.0 {
                    write_heat(&mut m.nbt, source.heat);
                }
            }
        });
    }
    cell.environment(ambient);
    write_heat(&mut event.entity.nbt, cell.heat);
}
