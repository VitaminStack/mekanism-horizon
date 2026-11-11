//! Electric Pump and Fluidic Plenisher (M06): the rules without Horizon types,
//! so their tests run in `logic/energy`. The world reads and writes, the saved
//! node lists, the container and the screen are in `pumps.rs`.
//!
//! Values and algorithm from Mekanism 26.3: `TileEntityElectricPump` (tank
//! 10 buckets, an operation every 19 ticks, 256 mB per tick into the block
//! above, `UsageConfig.electricPump` 2, range `GeneralConfig.maxPumpRange` 80,
//! `pumpInfiniteFluidSources` off) and `TileEntityFluidicPlenisher` (tank 10
//! buckets, an operation every 20 ticks, `UsageConfig.fluidicPlenisher` 2,
//! `maxPlenisherNodes` 4,000), storage 4 x usage as every machine.
//!
//! The pump takes the source block below it (and then, from the blocks it
//! already took from, their neighbours within range) one bucket per
//! operation. A source that the world refills (water with
//! `water_source_conversion`, lava with `lava_source_conversion`) stays, as
//! Mekanism's "infinite" rule has it; any other is removed. The plenisher
//! walks the same way over air and flowing fluid below it, one block per
//! operation, and puts one bucket of its fluid in each.
//!
//! Not ported: upgrades (speed, filter's heavy water), the item slots (fluid
//! containers, energy), redstone control, fluid-logged blocks and
//! `LiquidBlockContainer`s (kelp, seagrass); the plenisher's replaceable
//! blocks are a short list (`Cell::Replaceable` in `pumps.rs`) as Horizon names
//! no `canBeReplaced`.

use std::collections::HashSet;

use super::chemicals;

pub const PUMP: &str = "mekanism:electric_pump";
pub const PLENISHER: &str = "mekanism:fluidic_plenisher";
/// Registered by `data/mekanism/horizon/menu/<block>.json`.
pub const PUMP_MENU: &str = PUMP;
pub const PLENISHER_MENU: &str = PLENISHER;

pub const BUCKET: i32 = 1_000;
pub const TANK: i32 = 10 * BUCKET;
pub const USAGE: i32 = 2;
pub const CAPACITY: i32 = 4 * USAGE;
pub const PUMP_TICKS: i32 = 19;
pub const PLENISHER_TICKS: i32 = 20;
pub const PUMP_RATE: i32 = 256;
pub const PUMP_RANGE: f64 = 80.0;
pub const MAX_NODES: usize = 4_000;

pub type Pos = [i32; 3];

pub fn is_pump(block: &str) -> bool {
    block == PUMP
}

pub fn is_plenisher(block: &str) -> bool {
    block == PLENISHER
}

pub fn is_either(block: &str) -> bool {
    is_pump(block) || is_plenisher(block)
}

fn add(p: Pos, d: [i32; 3]) -> Pos {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

/// The six neighbours of a block, the plenisher's without up when `up` is false.
fn neighbours(p: Pos, up: bool) -> impl Iterator<Item = Pos> {
    const D: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];
    D.into_iter().filter(move |d| up || d[1] != 1).map(move |d| add(p, d))
}

/// A vanilla fluid that can be pumped and placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liquid {
    Water,
    Lava,
}

impl Liquid {
    pub fn id(self) -> &'static str {
        match self {
            Liquid::Water => chemicals::WATER,
            Liquid::Lava => chemicals::LAVA,
        }
    }

    pub fn from_id(id: &str) -> Option<Liquid> {
        match id {
            chemicals::WATER => Some(Liquid::Water),
            chemicals::LAVA => Some(Liquid::Lava),
            _ => None,
        }
    }
}

/// The pump's and the plenisher's one fluid tank.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tank {
    pub id: Option<String>,
    pub amount: i32,
}

impl Tank {
    pub fn new(id: Option<String>, amount: i32) -> Tank {
        let amount = amount.clamp(0, TANK);
        if id.is_none() || amount == 0 { Tank::default() } else { Tank { id, amount } }
    }

    pub fn room(&self) -> i32 {
        TANK - self.amount
    }

    /// A bucket of `liquid` fits (the tank is empty or holds the same).
    fn fits_bucket(&self, liquid: Liquid) -> bool {
        self.room() >= BUCKET && (self.amount == 0 || self.id.as_deref() == Some(liquid.id()))
    }

    fn add_bucket(&mut self, liquid: Liquid) {
        self.id = Some(liquid.id().to_owned());
        self.amount += BUCKET;
    }

    fn take_bucket(&mut self) {
        self.amount -= BUCKET;
        if self.amount == 0 {
            self.id = None;
        }
    }
}

// --- Electric Pump ---

pub trait PumpWorld {
    /// The fluid whose source block is at `pos` (None for anything else, or an unloaded block).
    fn source(&self, pos: Pos) -> Option<Liquid>;
    /// Whether taking a source of `liquid` leaves it (the world refills it).
    fn infinite(&self, liquid: Liquid) -> bool;
    /// Takes the source block away.
    fn remove(&mut self, pos: Pos);
}

/// What the pump keeps between operations (saved with it): the fluid it is
/// pumping, and the blocks that have sources in or next to them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PumpState {
    pub active: Option<Liquid>,
    pub nodes: Vec<Pos>,
}

pub struct Pump {
    pub energy: i32,
    pub operating: i32,
    pub tank: Tank,
    pub state: PumpState,
}

fn distance(a: Pos, b: Pos) -> f64 {
    let d = |i: usize| (a[i] - b[i]) as f64;
    (d(0) * d(0) + d(1) * d(1) + d(2) * d(2)).sqrt()
}

impl Pump {
    /// One bucket from the source block at `pos`, into the tank: only the fluid
    /// being pumped, only whole, and a source that stays is not removed.
    fn suck(&mut self, world: &mut dyn PumpWorld, pos: Pos, add_node: bool) -> bool {
        let Some(liquid) = world.source(pos) else { return false };
        if self.state.active.is_some_and(|a| a != liquid) || !self.tank.fits_bucket(liquid) {
            return false;
        }
        if !world.infinite(liquid) {
            world.remove(pos);
        }
        self.tank.add_bucket(liquid);
        self.state.active = Some(liquid);
        if add_node && !self.state.nodes.contains(&pos) {
            self.state.nodes.push(pos);
        }
        true
    }

    /// `suck(level, transaction)`: below the pump first, then each saved node
    /// in turn (in an order `shuffle` picks), then their neighbours in range.
    fn find(&mut self, world: &mut dyn PumpWorld, origin: Pos, shuffle: &mut dyn FnMut(&mut Vec<Pos>)) -> bool {
        if self.suck(world, add(origin, [0, -1, 0]), true) {
            return true;
        }
        let mut nodes = self.state.nodes.clone();
        shuffle(&mut nodes);
        for node in nodes {
            if self.suck(world, node, false) {
                return true;
            }
            for next in neighbours(node, true) {
                if distance(origin, next) <= PUMP_RANGE && self.suck(world, next, true) {
                    return true;
                }
            }
            self.state.nodes.retain(|n| *n != node);
        }
        false
    }

    /// One tick: with room for a bucket and energy for the tick, advance; every
    /// `PUMP_TICKS` look for a source, and forget the nodes when there is none.
    /// The energy is spent only while a fluid is being pumped or when this
    /// operation took a block, as Mekanism's. Returns whether it was spent.
    pub fn tick(&mut self, world: &mut dyn PumpWorld, origin: Pos, shuffle: &mut dyn FnMut(&mut Vec<Pos>)) -> bool {
        let room = self.tank.amount == 0 || self.tank.room() >= BUCKET;
        if !room || self.energy < USAGE {
            return false;
        }
        let mut used = self.state.active.is_some();
        self.operating += 1;
        if self.operating >= PUMP_TICKS {
            self.operating = 0;
            if self.find(world, origin, shuffle) {
                used = true;
            } else {
                self.state = PumpState::default();
            }
        }
        if used {
            self.energy -= USAGE;
        }
        used
    }
}

// --- Fluidic Plenisher ---

/// What a block is to the plenisher (`canReplace`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Air,
    /// A plant or layer that fluid replaces.
    Replaceable,
    /// Fluid that is not a source.
    Flowing,
    Source,
    Solid,
    /// Not loaded, or outside the world.
    Unloaded,
}

/// Air, replaceable blocks and flowing fluid always; a source only while
/// pathfinding (so the walk goes on over fluid already placed).
pub fn can_replace(cell: Cell, pathfinding: bool) -> bool {
    match cell {
        Cell::Air | Cell::Replaceable | Cell::Flowing => true,
        Cell::Source => pathfinding,
        Cell::Solid | Cell::Unloaded => false,
    }
}

pub trait PlenWorld {
    fn cell(&self, pos: Pos) -> Cell;
    /// Puts a source block of `liquid` at `pos`; false if the world refused.
    fn place(&mut self, pos: Pos, liquid: Liquid) -> bool;
}

/// What the plenisher keeps between operations (saved with it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlenState {
    /// Blocks to fill, in the order they were found.
    pub active: Vec<Pos>,
    pub used: HashSet<Pos>,
    pub finished: bool,
}

pub struct Plenisher {
    pub energy: i32,
    pub operating: i32,
    pub tank: Tank,
    pub state: PlenState,
}

impl Plenisher {
    fn can_replace(&self, world: &dyn PlenWorld, pos: Pos, check_nodes: bool, pathfinding: bool) -> bool {
        !(check_nodes && self.state.used.contains(&pos)) && can_replace(world.cell(pos), pathfinding)
    }

    /// `doPlenish`: fill the first block found that is loaded (or drop it if
    /// not), and queue its neighbours (not the one above) that fluid can reach.
    fn plenish(&mut self, world: &mut dyn PlenWorld, origin: Pos, liquid: Liquid) {
        if self.state.used.len() >= MAX_NODES {
            self.state.finished = true;
            return;
        }
        if self.state.active.is_empty() {
            let below = add(origin, [0, -1, 0]);
            if !self.state.used.is_empty() || !self.can_replace(world, below, true, true) {
                self.state.finished = true;
                return;
            }
            self.state.active.push(below);
        }
        let mut done = Vec::new();
        for node in self.state.active.clone() {
            done.push(node);
            if world.cell(node) == Cell::Unloaded {
                continue;
            }
            if self.can_replace(world, node, true, false) && self.tank.amount >= BUCKET && world.place(node, liquid) {
                self.tank.take_bucket();
            }
            for next in neighbours(node, false) {
                let queued = self.state.active.contains(&next) || done.contains(&next);
                if !queued && world.cell(next) != Cell::Unloaded && self.can_replace(world, next, true, true) {
                    self.state.active.push(next);
                }
            }
            break;
        }
        self.state.used.extend(done.iter().copied());
        self.state.active.retain(|p| !done.contains(p));
    }

    /// One tick, as `TileEntityFluidicPlenisher.onUpdateServer`: with a bucket
    /// and the energy, advance; every `PLENISHER_TICKS` either walk one more
    /// block or, when the walk is over, refill the block below. Energy is spent
    /// while walking and for a refill. Returns whether it was spent.
    pub fn tick(&mut self, world: &mut dyn PlenWorld, origin: Pos) -> bool {
        let Some(liquid) = self.tank.id.as_deref().and_then(Liquid::from_id) else { return false };
        if self.tank.amount < BUCKET || self.energy < USAGE {
            return false;
        }
        let mut used = false;
        self.operating += 1;
        if self.operating >= PLENISHER_TICKS {
            self.operating = 0;
            if self.state.finished {
                let below = add(origin, [0, -1, 0]);
                if self.can_replace(world, below, false, false) && world.place(below, liquid) {
                    self.tank.take_bucket();
                    used = true;
                }
            } else {
                self.plenish(world, origin, liquid);
                used = true;
            }
        } else if !self.state.finished {
            used = true;
        }
        if used {
            self.energy -= USAGE;
        }
        used
    }

    /// Sneak-click: forget the walk.
    pub fn reset(&mut self) {
        self.state = PlenState::default();
    }
}

// --- menus and screens (176 x 166, Mekanism's GUI coordinates) ---
pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;
pub const GAUGE: [f32; 4] = [6.0, 13.0, 18.0, 60.0];
pub const INNER_SCREEN: [f32; 4] = [54.0, 23.0, 80.0, 42.0];
pub const ENERGY_BAR: [f32; 4] = [164.0, 15.0, 6.0, 54.0];
pub const DATA_ENERGY: usize = 0;
pub const DATA_AMOUNT: usize = 1;
/// `chemicals::index` + 1 of the fluid in the tank, else the one being pumped (0 none).
pub const DATA_KIND: usize = 2;
pub const DATA_USED: usize = 3;
pub const DATA_FACING: usize = 4;
/// The plenisher's walk is over.
pub const DATA_FINISHED: usize = 5;
pub const DATA_LEN: usize = 6;

pub fn menu_data(energy: i32, tank: &Tank, kind: Option<&str>, used: bool, facing: usize, finished: bool) -> [i32; DATA_LEN] {
    let shown = tank.id.as_deref().or(kind);
    [
        energy,
        tank.amount,
        shown.and_then(chemicals::index).map_or(0, |i| i as i32 + 1),
        used as i32,
        facing as i32,
        finished as i32,
    ]
}

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    let [x, y, w, h] = rect;
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

/// The fluid shown by menu data kind `kind`, by name.
fn kind_name(kind: i32) -> String {
    usize::try_from(kind)
        .ok()
        .and_then(|k| k.checked_sub(1))
        .and_then(chemicals::entry)
        .map_or_else(|| "No fluid".to_owned(), |e| e.0.to_owned())
}

pub fn tooltip(data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(GAUGE, at) {
        let name = if d(DATA_AMOUNT) > 0 { kind_name(d(DATA_KIND)) } else { "Empty".to_owned() };
        return vec![format!("{name}: {} / {TANK} mB", d(DATA_AMOUNT))];
    }
    if inside(ENERGY_BAR, at) {
        return vec![format!("Energy: {} / {CAPACITY} FE", d(DATA_ENERGY)), format!("Usage: {USAGE} FE/t")];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Sources {
        blocks: HashMap<Pos, Liquid>,
        infinite: bool,
        removed: Vec<Pos>,
    }

    impl PumpWorld for Sources {
        fn source(&self, pos: Pos) -> Option<Liquid> {
            self.blocks.get(&pos).copied()
        }
        fn infinite(&self, _: Liquid) -> bool {
            self.infinite
        }
        fn remove(&mut self, pos: Pos) {
            self.blocks.remove(&pos);
            self.removed.push(pos);
        }
    }

    fn pump() -> Pump {
        Pump { energy: CAPACITY, operating: 0, tank: Tank::default(), state: PumpState::default() }
    }

    fn keep(_: &mut Vec<Pos>) {}

    /// Ticks until the pump's energy or the sources run out; returns the operations that took a block.
    fn run(p: &mut Pump, w: &mut Sources, ticks: usize) -> usize {
        let mut taken = 0;
        for _ in 0..ticks {
            p.energy = CAPACITY;
            let before = p.tank.amount;
            p.tick(w, [0, 0, 0], &mut keep);
            if p.tank.amount > before {
                taken += 1;
            }
        }
        taken
    }

    #[test]
    fn pump_takes_the_source_below_every_19_ticks() {
        let mut w = Sources::default();
        w.blocks.insert([0, -1, 0], Liquid::Lava);
        let mut p = pump();
        for _ in 0..18 {
            p.tick(&mut w, [0, 0, 0], &mut keep);
        }
        assert_eq!(p.tank.amount, 0);
        p.tick(&mut w, [0, 0, 0], &mut keep);
        assert_eq!(p.tank, Tank { id: Some(chemicals::LAVA.to_owned()), amount: BUCKET });
        assert!(w.blocks.is_empty(), "lava is not infinite");
        assert_eq!(p.state.active, Some(Liquid::Lava));
    }

    #[test]
    fn infinite_water_stays_and_costs_no_blocks() {
        let mut w = Sources { infinite: true, ..Default::default() };
        w.blocks.insert([0, -1, 0], Liquid::Water);
        let mut p = pump();
        assert_eq!(run(&mut p, &mut w, 19 * 3), 3);
        assert_eq!(p.tank.amount, 3 * BUCKET);
        assert!(w.removed.is_empty() && w.blocks.len() == 1);
    }

    #[test]
    fn pump_follows_sources_outwards_in_range() {
        let mut w = Sources::default();
        for x in 0..4 {
            w.blocks.insert([x, -1, 0], Liquid::Lava);
        }
        let mut p = pump();
        assert_eq!(run(&mut p, &mut w, 19 * 6), 4);
        assert_eq!(p.tank.amount, 4 * BUCKET);
        assert!(w.blocks.is_empty());
        // After the last one nothing is found: the nodes are forgotten and the type is free again.
        run(&mut p, &mut w, 19);
        assert_eq!(p.state, PumpState::default());
        // A source out of range (81 blocks) is not found.
        w.blocks.insert([81, -1, 0], Liquid::Lava);
        w.blocks.insert([0, -1, 0], Liquid::Lava);
        run(&mut p, &mut w, 19 * 3);
        assert_eq!(w.blocks.len(), 1, "only the one in range was taken");
    }

    #[test]
    fn pump_keeps_to_one_fluid_and_waits_for_room() {
        let mut w = Sources::default();
        w.blocks.insert([0, -1, 0], Liquid::Water);
        let mut p = pump();
        p.tank = Tank::new(Some(chemicals::LAVA.to_owned()), BUCKET);
        run(&mut p, &mut w, 40);
        assert_eq!(p.tank.id.as_deref(), Some(chemicals::LAVA), "no water into the lava");
        assert_eq!(w.blocks.len(), 1);
        p.tank = Tank::new(Some(chemicals::WATER.to_owned()), TANK - BUCKET + 1);
        let operating = p.operating;
        assert!(!p.tick(&mut w, [0, 0, 0], &mut keep), "less than a bucket of room: idle");
        assert_eq!(p.operating, operating);
    }

    #[test]
    fn pump_spends_energy_only_while_pumping() {
        let mut w = Sources::default();
        let mut p = pump();
        assert!(!p.tick(&mut w, [0, 0, 0], &mut keep), "nothing active yet: free");
        assert_eq!(p.energy, CAPACITY);
        w.blocks.insert([0, -1, 0], Liquid::Lava);
        p.operating = PUMP_TICKS - 1;
        assert!(p.tick(&mut w, [0, 0, 0], &mut keep), "the operation that took a block");
        assert_eq!(p.energy, CAPACITY - USAGE);
        p.energy = USAGE - 1;
        assert!(!p.tick(&mut w, [0, 0, 0], &mut keep), "too little energy");
    }

    struct Grid {
        cells: HashMap<Pos, Cell>,
        default: Cell,
        placed: Vec<Pos>,
    }

    impl PlenWorld for Grid {
        fn cell(&self, pos: Pos) -> Cell {
            self.cells.get(&pos).copied().unwrap_or(self.default)
        }
        fn place(&mut self, pos: Pos, _: Liquid) -> bool {
            self.cells.insert(pos, Cell::Source);
            self.placed.push(pos);
            true
        }
    }

    fn plenisher(amount: i32) -> Plenisher {
        Plenisher {
            energy: CAPACITY,
            operating: 0,
            tank: Tank::new(Some(chemicals::WATER.to_owned()), amount),
            state: PlenState::default(),
        }
    }

    /// A pit of air under the plenisher (y -1 and -2, x/z -1..=1) in solid stone.
    fn pit() -> Grid {
        let mut cells = HashMap::new();
        for y in [-1, -2] {
            for x in -1..=1 {
                for z in -1..=1 {
                    cells.insert([x, y, z], Cell::Air);
                }
            }
        }
        Grid { cells, default: Cell::Solid, placed: Vec::new() }
    }

    fn operate(p: &mut Plenisher, g: &mut Grid, operations: usize) {
        for _ in 0..operations * PLENISHER_TICKS as usize {
            p.energy = CAPACITY;
            p.tick(g, [0, 0, 0]);
        }
    }

    #[test]
    fn plenisher_fills_a_pit_one_block_per_operation() {
        let mut g = pit();
        let mut p = plenisher(TANK);
        operate(&mut p, &mut g, 1);
        assert_eq!(g.placed, vec![[0, -1, 0]]);
        assert_eq!(p.tank.amount, TANK - BUCKET);
        operate(&mut p, &mut g, 17);
        assert_eq!(g.placed.len(), 10, "ten buckets: the tank is empty");
        assert_eq!(p.tank.amount, 0);
        assert!(g.placed.iter().all(|p| p[1] <= -1), "never above the plenisher's floor");
    }

    #[test]
    fn plenisher_walks_on_without_fluid_it_just_waits() {
        let mut g = pit();
        let mut p = plenisher(BUCKET - 1);
        operate(&mut p, &mut g, 3);
        assert!(g.placed.is_empty());
        assert_eq!(p.operating, 0, "less than a bucket: idle");
    }

    #[test]
    fn plenisher_finishes_and_then_only_refills_below() {
        let mut g = pit();
        let mut p = plenisher(TANK);
        for _ in 0..30 {
            p.tank = Tank::new(Some(chemicals::WATER.to_owned()), TANK);
            operate(&mut p, &mut g, 1);
        }
        assert!(p.state.finished && p.state.active.is_empty());
        assert_eq!(g.placed.len(), 18, "the whole pit: 2 layers of 9");
        // Refill: take the fluid below away; the next operation places it again.
        g.cells.insert([0, -1, 0], Cell::Air);
        p.tank = Tank::new(Some(chemicals::WATER.to_owned()), BUCKET);
        operate(&mut p, &mut g, 1);
        assert_eq!(g.placed.len(), 19);
        // Finished and below is a source: nothing happens and no energy goes.
        p.tank = Tank::new(Some(chemicals::WATER.to_owned()), BUCKET);
        p.energy = CAPACITY;
        for _ in 0..PLENISHER_TICKS * 2 {
            assert!(!p.tick(&mut g, [0, 0, 0]));
        }
        assert_eq!(p.energy, CAPACITY);
    }

    #[test]
    fn plenisher_stops_at_a_solid_floor_and_at_the_node_cap() {
        let mut g = Grid { cells: HashMap::new(), default: Cell::Solid, placed: Vec::new() };
        let mut p = plenisher(TANK);
        operate(&mut p, &mut g, 1);
        assert!(p.state.finished && g.placed.is_empty(), "nothing replaceable below");

        let mut g = Grid { cells: HashMap::new(), default: Cell::Air, placed: Vec::new() };
        let mut p = plenisher(TANK);
        p.state.used = (0..MAX_NODES as i32).map(|i| [i, 100, 0]).collect();
        p.state.active.push([0, -1, 0]);
        operate(&mut p, &mut g, 1);
        assert!(p.state.finished, "4,000 blocks used: the walk is over");
        assert!(g.placed.is_empty());
    }

    #[test]
    fn plenisher_skips_unloaded_blocks_and_walks_over_sources() {
        let mut g = pit();
        g.cells.insert([0, -1, 0], Cell::Source);
        g.cells.insert([1, -1, 0], Cell::Unloaded);
        let mut p = plenisher(TANK);
        operate(&mut p, &mut g, 20);
        assert!(!g.placed.contains(&[0, -1, 0]), "already a source");
        assert!(!g.placed.contains(&[1, -1, 0]), "not loaded");
        assert!(g.placed.contains(&[0, -2, 0]), "but the walk went past the source");
        assert!(can_replace(Cell::Flowing, false) && !can_replace(Cell::Source, false));
    }

    #[test]
    fn menu_data_and_tooltips() {
        let tank = Tank::new(Some(chemicals::LAVA.to_owned()), 2_500);
        let d = menu_data(6, &tank, None, true, 3, false);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(tooltip(&d, [10.0, 20.0]), vec!["Lava: 2500 / 10000 mB"]);
        assert_eq!(tooltip(&d, [165.0, 20.0])[0], "Energy: 6 / 8 FE");
        let idle = menu_data(0, &Tank::default(), Some(chemicals::WATER), false, 0, false);
        assert_eq!(tooltip(&idle, [10.0, 20.0]), vec!["Empty: 0 / 10000 mB"]);
        assert_ne!(idle[DATA_KIND], 0, "the pumped fluid is shown while the tank is empty");
        assert_eq!(CAPACITY, 8);
    }
}
