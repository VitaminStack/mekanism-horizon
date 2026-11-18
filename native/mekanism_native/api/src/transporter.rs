//! Logistical Transporters (M08): the rules that need no Horizon types, so their
//! tests run in `logic/energy`. The host side (tick, container, arms) is in
//! `transporters.rs`.
//!
//! Values from Mekanism 26.3: `TransporterTier` (pull amount 1 / 16 / 32 / 64
//! items, speed 5 / 10 / 20 / 50), `LogisticalTransporterBase` (a stack's
//! `progress` grows by the speed each tick and passes on at 100; `getCost` is
//! the Ultimate's speed divided by the tier's), `RestrictiveTransporter` (a
//! Basic one that costs 1,000) and `DiversionTransporter` (a Basic one with a
//! `DiversionControl` per side: disabled, high, low).
//!
//! Horizon has no transmitter networks, so each transporter is a block entity
//! of its own holding the stacks in transit (see `transporters.rs`). A stack
//! that has dwelt `ceil(100 / speed)` ticks in a block looks for the cheapest
//! way to an inventory (`route`, Dijkstra over the transporter blocks, a
//! transporter's cost per block entered) and is handed on: into the next
//! transporter or the inventory. Nothing leaves a transporter before the next
//! holder has taken it, so no item is lost or made (`tests::items_are_conserved`).
//!
//! Not ported: colours, the stack-by-stack path prediction of the network,
//! idle paths (a stack with no way to go waits where it is), the Configurator.

use super::{bin, cube, machine, storage};

pub type Pos = [i32; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Logistical,
    Restrictive,
    Diversion,
}

#[derive(Clone, Copy, Debug)]
pub struct Tier {
    pub block: &'static str,
    pub variant: Variant,
    /// Items a Pull side takes per attempt (`TransporterTier.getPullAmount`).
    pub pull: u32,
    /// Progress a stack gains per tick (`TransporterTier.getSpeed`); 100 passes it on.
    pub speed: i32,
}

const fn tier(block: &'static str, variant: Variant, pull: u32, speed: i32) -> Tier {
    Tier { block, variant, pull, speed }
}

pub const TRANSPORTERS: [Tier; 6] = [
    tier("mekanism:basic_logistical_transporter", Variant::Logistical, 1, 5),
    tier("mekanism:advanced_logistical_transporter", Variant::Logistical, 16, 10),
    tier("mekanism:elite_logistical_transporter", Variant::Logistical, 32, 20),
    tier("mekanism:ultimate_logistical_transporter", Variant::Logistical, 64, 50),
    // Both take the Basic tier's values.
    tier("mekanism:restrictive_transporter", Variant::Restrictive, 1, 5),
    tier("mekanism:diversion_transporter", Variant::Diversion, 1, 5),
];

pub fn tier_of(block: &str) -> Option<&'static Tier> {
    TRANSPORTERS.iter().find(|t| t.block == block)
}

/// What the Ultimate's speed is to this one's, times two (`getCost`: Basic 10,
/// Advanced 5, Elite 2.5, Ultimate 1; the Restrictive 1,000). Whole numbers.
pub fn cost(t: &Tier) -> u32 {
    match t.variant {
        Variant::Restrictive => 2_000,
        _ => (100 / t.speed) as u32,
    }
}

/// Ticks a stack stays in one block before it is passed on.
pub fn dwell_ticks(t: &Tier) -> i32 {
    (100 + t.speed - 1) / t.speed
}

/// Mekanism's `ConnectionType`, per side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// Joins neighbours and takes what is pushed in; sends items on.
    Normal = 0,
    /// Only sends out of this side (never takes from it).
    Push = 1,
    /// Pulls items out of the inventory on this side.
    Pull = 2,
    /// No connection.
    None = 3,
}

impl Connection {
    pub fn from_byte(b: u8) -> Connection {
        match b {
            1 => Connection::Push,
            2 => Connection::Pull,
            3 => Connection::None,
            _ => Connection::Normal,
        }
    }
    /// The Configurator's order.
    pub fn next(self) -> Connection {
        match self {
            Connection::Normal => Connection::Push,
            Connection::Push => Connection::Pull,
            Connection::Pull => Connection::None,
            Connection::None => Connection::Normal,
        }
    }
}

/// Mekanism's `DiversionControl`: when a side connects (redstone-controlled).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diversion {
    Disabled = 0,
    High = 1,
    Low = 2,
}

impl Diversion {
    pub fn from_byte(b: u8) -> Diversion {
        match b {
            1 => Diversion::High,
            2 => Diversion::Low,
            _ => Diversion::Disabled,
        }
    }
    pub fn next(self) -> Diversion {
        match self {
            Diversion::Disabled => Diversion::High,
            Diversion::High => Diversion::Low,
            Diversion::Low => Diversion::Disabled,
        }
    }
    /// `modeReqsMet`: High needs a redstone signal, Low none.
    pub fn met(self, powered: bool) -> bool {
        match self {
            Diversion::Disabled => true,
            Diversion::High => powered,
            Diversion::Low => !powered,
        }
    }
}

/// The saved sides of one transporter: connection per side, and the diversion
/// control per side (only a Diversion Transporter uses the latter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sides {
    pub connection: [Connection; 6],
    pub diversion: [Diversion; 6],
}

impl Default for Sides {
    fn default() -> Self {
        Sides { connection: [Connection::Normal; 6], diversion: [Diversion::Disabled; 6] }
    }
}

impl Sides {
    pub fn to_bytes(&self) -> ([u8; 6], [u8; 6]) {
        (self.connection.map(|c| c as u8), self.diversion.map(|d| d as u8))
    }
    pub fn from_bytes(connection: &[u8], diversion: &[u8]) -> Sides {
        let mut s = Sides::default();
        for d in 0..6 {
            s.connection[d] = Connection::from_byte(connection.get(d).copied().unwrap_or(0));
            s.diversion[d] = Diversion::from_byte(diversion.get(d).copied().unwrap_or(0));
        }
        s
    }
}

/// What a transporter block offers its neighbours, given the redstone state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Per-block cost of entering it (`cost`).
    pub cost: u32,
    /// Sides it joins a neighbour on (not off, and its diversion rule is met).
    pub connect: [bool; 6],
    /// Sides it may deliver an item out of (connected, and not a Pull side).
    pub send: [bool; 6],
}

impl Link {
    pub fn of(t: &Tier, sides: &Sides, powered: bool) -> Link {
        let mut link = Link { cost: cost(t), connect: [false; 6], send: [false; 6] };
        for d in 0..6 {
            let on = sides.connection[d] != Connection::None
                && (t.variant != Variant::Diversion || sides.diversion[d].met(powered));
            link.connect[d] = on;
            link.send[d] = on && sides.connection[d] != Connection::Pull;
        }
        link
    }
}

/// What is at a position, as far as a route cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Air, or a block that takes nothing.
    Empty,
    Transporter(Link),
    /// A block with an inventory an item can go into.
    Inventory,
}

pub trait Graph {
    fn at(&mut self, pos: Pos) -> Cell;
}

/// Where a stack goes next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hop {
    pub dir: usize,
    /// The block in that direction is the inventory itself.
    pub deliver: bool,
}

fn step(pos: Pos, dir: usize) -> Pos {
    let d = cube::OFFSETS[dir];
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}

/// At most this many blocks are looked at for one route.
pub const SEARCH_LIMIT: usize = 512;

/// The cheapest way from the transporter at `start` (offering `link`) to an
/// inventory: the first side to leave by, and whether that side is the
/// inventory. `from` is the side the stack came in by (not walked back into
/// unless nothing else leads anywhere), `origin` the inventory it was pulled
/// from (never a destination), `avoid` an inventory that refused it (skipped
/// anywhere on the way, so a stack does not shuttle to a full chest).
/// Ties go to the lower side number. None when no inventory can be reached.
pub fn route(g: &mut dyn Graph, start: Pos, link: &Link, from: Option<usize>, origin: Option<Pos>, avoid: Option<Pos>) -> Option<Hop> {
    search(g, start, link, from, origin, avoid).or_else(|| {
        from.and_then(|_| search(g, start, link, None, origin, avoid))
    })
}

fn search(g: &mut dyn Graph, start: Pos, start_link: &Link, skip: Option<usize>, origin: Option<Pos>, avoid: Option<Pos>) -> Option<Hop> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};
    // (cost, first side) of the best way to each transporter block, and of the best inventory.
    let mut best: HashMap<Pos, (u32, usize)> = HashMap::new();
    let mut links: HashMap<Pos, Link> = HashMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(start, (0, 0));
    links.insert(start, *start_link);
    heap.push(Reverse((0u32, 0usize, start)));
    let mut goal: Option<(u32, usize, bool)> = None;
    let mut looked = 0;
    while let Some(Reverse((c, first, pos))) = heap.pop() {
        if best.get(&pos).is_some_and(|&b| b < (c, first)) {
            continue;
        }
        if goal.is_some_and(|(gc, ..)| c > gc) {
            break;
        }
        looked += 1;
        if looked > SEARCH_LIMIT {
            break;
        }
        let link = links[&pos];
        for d in 0..6 {
            if !link.connect[d] {
                continue;
            }
            if pos == start && (skip == Some(d)) {
                continue;
            }
            let at = step(pos, d);
            let lead = if pos == start { d } else { first };
            match g.at(at) {
                Cell::Inventory => {
                    if link.send[d] && origin != Some(at) && avoid != Some(at) {
                        let cand = (c, lead, pos == start);
                        if goal.is_none_or(|(gc, gf, _)| (cand.0, cand.1) < (gc, gf)) {
                            goal = Some(cand);
                        }
                    }
                }
                Cell::Transporter(next) => {
                    if !next.connect[cube::opposite(d)] || !link.send[d] {
                        continue;
                    }
                    let nc = c + next.cost;
                    if best.get(&at).is_none_or(|&b| (nc, lead) < b) {
                        best.insert(at, (nc, lead));
                        links.insert(at, next);
                        heap.push(Reverse((nc, lead, at)));
                    }
                }
                Cell::Empty => {}
            }
        }
    }
    goal.map(|(_, dir, deliver)| Hop { dir, deliver })
}

/// Whether a block is one an item can be put into: the vanilla containers,
/// Mekanism's machines, bins and personal storage, and any other mod's block
/// (the transporter tries, the block refuses what it cannot take).
pub fn is_inventory_block(id: &str) -> bool {
    if id.is_empty() || id == "minecraft:air" || id == "minecraft:cave_air" || id == "minecraft:void_air" {
        return false;
    }
    let (ns, name) = id.split_once(':').unwrap_or(("minecraft", id));
    match ns {
        "minecraft" => {
            matches!(
                name,
                "chest" | "trapped_chest" | "barrel" | "hopper" | "dropper" | "dispenser" | "furnace" | "blast_furnace"
                    | "smoker" | "brewing_stand" | "crafter" | "chiseled_bookshelf"
            ) || name.ends_with("shulker_box")
        }
        "mekanism" => machine::find(id).is_some() || bin::tier_of(id).is_some() || storage::find(id).is_some(),
        "mekanismgenerators" | "mekanismtools" | "mekanismadditions" => false,
        _ => true,
    }
}

/// A stack in transit: how far along its block it is, the side it came in by
/// and the inventory it was pulled from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Transit {
    pub progress: i32,
    /// Side of this block the stack entered by (`NO_SIDE` for an insert from a hopper).
    pub from: u8,
    pub origin: Option<Pos>,
    /// The inventory that refused the stack last.
    pub refused: Option<Pos>,
}

pub const NO_SIDE: u8 = 6;

impl Transit {
    pub const INTS: usize = 10;

    pub fn fresh(from: Option<usize>, origin: Option<Pos>) -> Transit {
        Transit { progress: 0, from: from.map_or(NO_SIDE, |d| d as u8), origin, refused: None }
    }

    pub fn to_ints(&self) -> [i32; Transit::INTS] {
        let [x, y, z] = self.origin.unwrap_or([0; 3]);
        let [rx, ry, rz] = self.refused.unwrap_or([0; 3]);
        [self.progress, self.from as i32, self.origin.is_some() as i32, x, y, z, self.refused.is_some() as i32, rx, ry, rz]
    }

    pub fn from_ints(i: &[i32]) -> Transit {
        let get = |n: usize| i.get(n).copied().unwrap_or(0);
        Transit {
            progress: get(0).clamp(0, 1_000),
            from: get(1).clamp(0, NO_SIDE as i32) as u8,
            origin: (get(2) != 0).then(|| [get(3), get(4), get(5)]),
            refused: (get(6) != 0).then(|| [get(7), get(8), get(9)]),
        }
    }

    fn side(v: u8) -> Option<usize> {
        (v < NO_SIDE).then_some(v as usize)
    }
}

/// What a stack does this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Stays in this block.
    Stay,
    /// Offer it to the transporter on this side.
    Forward(usize),
    /// Offer it to the inventory on this side.
    Deliver(usize),
}

/// One tick of one stack: it gains `speed` progress, and once it reaches 100
/// it asks for a route. No route, or none that accepts it, and it waits where
/// it is for another block's time (`progress` back to 0). The caller carries
/// out the action and, when the receiver refuses, calls `refused`.
pub fn advance(t: &mut Transit, speed: i32, g: &mut dyn Graph, pos: Pos, link: &Link) -> Action {
    if t.progress < 100 {
        t.progress = (t.progress + speed).min(1_000);
        if t.progress < 100 {
            return Action::Stay;
        }
    }
    match route(g, pos, link, Transit::side(t.from), t.origin, t.refused) {
        Some(Hop { dir, deliver: true }) => Action::Deliver(dir),
        Some(Hop { dir, deliver: false }) => Action::Forward(dir),
        None => {
            // Nothing else takes it: try the refusing inventory again later.
            t.progress = 0;
            t.refused = None;
            Action::Stay
        }
    }
}

/// The receiver turned the offered stack (all or part of it) down.
pub fn refused(t: &mut Transit, action: Action, at: Option<Pos>) {
    t.progress = 0;
    if let (Action::Deliver(d), Some(at)) = (action, at) {
        t.refused = Some(step(at, d));
    }
}

/// What the next block holds a stack by: entered through `entered` (the side
/// of the next block it came through), `carry` progress over 100 kept.
pub fn arrived(t: &Transit, entered: usize) -> Transit {
    Transit { progress: t.progress % 100, from: entered as u8, origin: t.origin, refused: t.refused }
}

/// The side a player's look picks on a transporter at `pos`, as Mekanism's per-side hit boxes do:
/// the `joined` arm (`5/16..11/16` across, from the core to the block's face) the ray from `eye`
/// along `dir` meets first. Only joined arms are in the block's shape, so only they can be
/// clicked. None when it meets the core first or misses: then the clicked face counts.
/// Lets a player pick the side that faces a chest, whose own face the chest hides.
pub fn picked_side(pos: Pos, eye: [f64; 3], dir: [f64; 3], joined: [bool; 6]) -> Option<usize> {
    const LO: f64 = 5.0 / 16.0;
    const HI: f64 = 11.0 / 16.0;
    let local = [eye[0] - pos[0] as f64, eye[1] - pos[1] as f64, eye[2] - pos[2] as f64];
    let hit = |min: [f64; 3], max: [f64; 3]| -> Option<f64> {
        let (mut near, mut far) = (0.0f64, f64::INFINITY);
        for a in 0..3 {
            if dir[a].abs() < 1e-12 {
                if local[a] < min[a] || local[a] > max[a] {
                    return None;
                }
                continue;
            }
            let (t1, t2) = ((min[a] - local[a]) / dir[a], (max[a] - local[a]) / dir[a]);
            near = near.max(t1.min(t2));
            far = far.min(t1.max(t2));
        }
        (near <= far).then_some(near)
    };
    let core = hit([LO; 3], [HI; 3]);
    let mut best: Option<(f64, usize)> = None;
    for (side, offset) in cube::OFFSETS.iter().enumerate().filter(|&(side, _)| joined[side]) {
        let (mut min, mut max) = ([LO; 3], [HI; 3]);
        for a in 0..3 {
            match offset[a] {
                -1 => (min[a], max[a]) = (0.0, LO),
                1 => (min[a], max[a]) = (HI, 1.0),
                _ => {}
            }
        }
        if let Some(t) = hit(min, max) {
            if best.is_none_or(|(b, _)| t < b) {
                best = Some((t, side));
            }
        }
    }
    let (t, side) = best?;
    core.is_none_or(|c| t < c).then_some(side)
}

/// The look direction of a player's `yaw` and `pitch` in degrees (`Entity.calculateViewVector`).
pub fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = ((yaw as f64).to_radians(), (pitch as f64).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const SLOTS: usize = 8;
    const BASIC: &str = "mekanism:basic_logistical_transporter";
    const ULTIMATE: &str = "mekanism:ultimate_logistical_transporter";

    fn link(block: &str) -> Link {
        Link::of(tier_of(block).unwrap(), &Sides::default(), false)
    }

    /// A little world: transporters and inventories by position, each with the item
    /// count it holds; transporters hold a Vec of stacks (count, Transit).
    #[derive(Default)]
    struct World {
        lines: HashMap<Pos, (Link, Vec<(u32, Transit)>)>,
        inventories: HashMap<Pos, (u32, u32)>, // held, capacity
        speed: HashMap<Pos, i32>,
    }

    impl Graph for World {
        fn at(&mut self, pos: Pos) -> Cell {
            if let Some((l, _)) = self.lines.get(&pos) {
                Cell::Transporter(*l)
            } else if self.inventories.contains_key(&pos) {
                Cell::Inventory
            } else {
                Cell::Empty
            }
        }
    }

    impl World {
        fn line(&mut self, from: Pos, dir: usize, len: usize, block: &str) -> Pos {
            let mut p = from;
            for _ in 0..len {
                self.lines.insert(p, (link(block), Vec::new()));
                self.speed.insert(p, tier_of(block).unwrap().speed);
                p = step(p, dir);
            }
            p
        }
        fn total(&self) -> u32 {
            self.lines.values().map(|(_, s)| s.iter().map(|(c, _)| *c).sum::<u32>()).sum::<u32>()
                + self.inventories.values().map(|(h, _)| *h).sum::<u32>()
        }
        /// One tick for every stack: forward moves hand the whole stack on if the next
        /// block has room (4 stacks), deliveries fill an inventory up to its capacity.
        fn tick(&mut self) {
            let positions: Vec<Pos> = self.lines.keys().copied().collect();
            for pos in positions {
                let mut stacks = std::mem::take(&mut self.lines.get_mut(&pos).unwrap().1);
                let l = self.lines[&pos].0;
                let speed = self.speed[&pos];
                let mut keep = Vec::new();
                for (count, mut t) in stacks.drain(..) {
                    let action = advance(&mut t, speed, self, pos, &l);
                    match action {
                        Action::Stay => keep.push((count, t)),
                        Action::Forward(d) => {
                            let np = step(pos, d);
                            let next = self.lines.get_mut(&np).unwrap();
                            if next.1.len() < SLOTS {
                                next.1.push((count, arrived(&t, cube::opposite(d))));
                            } else {
                                refused(&mut t, action, Some(pos));
                                keep.push((count, t));
                            }
                        }
                        Action::Deliver(d) => {
                            let inv = self.inventories.get_mut(&step(pos, d)).unwrap();
                            let took = count.min(inv.1 - inv.0);
                            inv.0 += took;
                            if took < count {
                                refused(&mut t, action, Some(pos));
                                keep.push((count - took, t));
                            }
                        }
                    }
                }
                self.lines.get_mut(&pos).unwrap().1.extend(keep);
            }
        }
    }

    #[test]
    fn tier_values_match_mekanism() {
        let v: Vec<(u32, i32)> = TRANSPORTERS.iter().map(|t| (t.pull, t.speed)).collect();
        assert_eq!(v, [(1, 5), (16, 10), (32, 20), (64, 50), (1, 5), (1, 5)]);
        assert_eq!(dwell_ticks(tier_of(BASIC).unwrap()), 20);
        assert_eq!(dwell_ticks(tier_of(ULTIMATE).unwrap()), 2);
        // getCost: 50 / speed, the Restrictive 1,000 (both doubled).
        let c: Vec<u32> = TRANSPORTERS.iter().map(cost).collect();
        assert_eq!(c, [20, 10, 5, 2, 2_000, 20]);
    }

    #[test]
    fn a_stack_takes_dwell_ticks_per_block() {
        let mut w = World::default();
        let end = w.line([0, 0, 0], cube::EAST, 3, BASIC);
        w.inventories.insert(end, (0, 64));
        w.lines.get_mut(&[0, 0, 0]).unwrap().1.push((10, Transit::fresh(None, None)));
        let mut arrived_at = None;
        for tick in 1..=200 {
            w.tick();
            if w.inventories[&end].0 == 10 {
                arrived_at = Some(tick);
                break;
            }
        }
        // 20 ticks in each of three blocks, the last tick hands it into the chest.
        assert!(matches!(arrived_at, Some(58..=60)), "{arrived_at:?}");
    }

    #[test]
    fn items_are_conserved() {
        // A junction: a line east with branches to two chests; one chest is tiny.
        let mut w = World::default();
        let mut p = w.line([0, 0, 0], cube::EAST, 4, BASIC);
        w.inventories.insert(p, (0, 1_000));
        p = [2, 0, 0];
        w.lines.insert(step(p, cube::UP), (link(ULTIMATE), Vec::new()));
        w.speed.insert(step(p, cube::UP), 50);
        w.inventories.insert(step(step(p, cube::UP), cube::UP), (0, 7));
        for i in 0..6 {
            w.lines.get_mut(&[0, 0, 0]).unwrap().1.push((16 + i, Transit::fresh(None, None)));
        }
        let total = w.total();
        assert_eq!(total, 6 * 16 + 15);
        for tick in 0..4_000 {
            w.tick();
            assert_eq!(w.total(), total, "tick {tick}");
        }
        // Everything ended in a chest: the near small one is full, the rest in the big one.
        for (p, (_, s)) in &w.lines { assert!(s.is_empty(), "{p:?} {s:?}"); }
        assert_eq!(w.inventories[&[2, 2, 0]].0, 7);
        assert_eq!(w.inventories[&[4, 0, 0]].0, total - 7);
    }

    #[test]
    fn the_cheapest_route_wins_and_restrictive_is_avoided() {
        // Two ways round a block to the same chest: an Ultimate path and a Restrictive one.
        let mut w = World::default();
        let start = [0, 0, 0];
        w.lines.insert(start, (link(BASIC), Vec::new()));
        // North: Restrictive transporter then the chest; south: Ultimate then a chest, one block longer.
        let restrictive = link("mekanism:restrictive_transporter");
        w.lines.insert(step(start, cube::NORTH), (restrictive, Vec::new()));
        w.inventories.insert(step(step(start, cube::NORTH), cube::NORTH), (0, 64));
        w.lines.insert(step(start, cube::SOUTH), (link(ULTIMATE), Vec::new()));
        w.lines.insert(step(step(start, cube::SOUTH), cube::SOUTH), (link(ULTIMATE), Vec::new()));
        w.inventories.insert([0, 0, 3], (0, 64));
        let hop = route(&mut w, start, &link(BASIC), None, None, None);
        assert_eq!(hop, Some(Hop { dir: cube::SOUTH, deliver: false }));
        // Without the alternative it still goes through the Restrictive one.
        w.lines.remove(&step(start, cube::SOUTH));
        let hop = route(&mut w, start, &link(BASIC), None, None, None);
        assert_eq!(hop, Some(Hop { dir: cube::NORTH, deliver: false }));
    }

    #[test]
    fn ties_go_to_the_lower_side_and_an_adjacent_chest_is_delivered_to() {
        let mut w = World::default();
        let start = [0, 0, 0];
        w.lines.insert(start, (link(BASIC), Vec::new()));
        w.inventories.insert(step(start, cube::EAST), (0, 64));
        w.inventories.insert(step(start, cube::WEST), (0, 64));
        assert_eq!(route(&mut w, start, &link(BASIC), None, None, None), Some(Hop { dir: cube::WEST, deliver: true }));
        // The chest the stack was pulled from is never a destination; a refusing one is skipped.
        let origin = Some(step(start, cube::WEST));
        assert_eq!(route(&mut w, start, &link(BASIC), None, origin, None), Some(Hop { dir: cube::EAST, deliver: true }));
        assert_eq!(route(&mut w, start, &link(BASIC), None, None, Some(step(start, cube::WEST))), Some(Hop { dir: cube::EAST, deliver: true }));
        assert_eq!(route(&mut w, start, &link(BASIC), None, origin, Some(step(start, cube::EAST))), None);
    }

    #[test]
    fn a_stack_goes_back_only_when_nothing_else_leads_anywhere() {
        let mut w = World::default();
        let start = [1, 0, 0];
        w.lines.insert(start, (link(BASIC), Vec::new()));
        w.lines.insert([0, 0, 0], (link(BASIC), Vec::new()));
        w.inventories.insert([-1, 0, 0], (0, 64));
        // Came from the west, the only way on is back west.
        assert_eq!(route(&mut w, start, &link(BASIC), Some(cube::WEST), None, None), Some(Hop { dir: cube::WEST, deliver: false }));
        // With a chest on the east side it does not turn round.
        w.inventories.insert([2, 0, 0], (0, 64));
        assert_eq!(route(&mut w, start, &link(BASIC), Some(cube::WEST), None, None), Some(Hop { dir: cube::EAST, deliver: true }));
    }

    #[test]
    fn connections_and_diversion_shape_the_graph() {
        let t = tier_of("mekanism:diversion_transporter").unwrap();
        let mut sides = Sides::default();
        sides.diversion[cube::EAST] = Diversion::High;
        sides.diversion[cube::WEST] = Diversion::Low;
        sides.connection[cube::UP] = Connection::Pull;
        sides.connection[cube::DOWN] = Connection::None;
        let off = Link::of(t, &sides, false);
        let on = Link::of(t, &sides, true);
        // High needs power, Low needs none; Disabled always connects.
        assert!(!off.connect[cube::EAST] && off.connect[cube::WEST]);
        assert!(on.connect[cube::EAST] && !on.connect[cube::WEST]);
        assert!(off.connect[cube::NORTH]);
        // A Pull side connects but never delivers; an off side does neither.
        assert!(off.connect[cube::UP] && !off.send[cube::UP]);
        assert!(!off.connect[cube::DOWN] && !off.send[cube::DOWN]);
        // Only a Diversion Transporter has the redstone rule.
        let plain = Link::of(tier_of(BASIC).unwrap(), &sides, false);
        assert!(plain.connect[cube::EAST]);
        // Saved as bytes and read back.
        let (c, d) = sides.to_bytes();
        assert_eq!(Sides::from_bytes(&c, &d), sides);
        assert_eq!(Sides::from_bytes(&[], &[]), Sides::default());
    }

    #[test]
    fn a_stack_with_no_way_on_waits_and_loses_nothing() {
        let mut w = World::default();
        w.lines.insert([0, 0, 0], (link(ULTIMATE), Vec::new()));
        w.speed.insert([0, 0, 0], 50);
        w.lines.get_mut(&[0, 0, 0]).unwrap().1.push((64, Transit::fresh(None, None)));
        for _ in 0..500 {
            w.tick();
            assert_eq!(w.total(), 64);
        }
        assert_eq!(w.lines[&[0, 0, 0]].1.len(), 1);
    }

    #[test]
    fn transit_is_saved_as_ints() {
        let t = Transit { progress: 55, from: 3, origin: Some([-4, 70, 9]), refused: Some([1, 2, 3]) };
        assert_eq!(Transit::from_ints(&t.to_ints()), t);
        let none = Transit::fresh(None, None);
        assert_eq!(Transit::from_ints(&none.to_ints()), none);
        // Nonsense from an edited save is clamped.
        let bad = Transit::from_ints(&[-5, 99, 0]);
        assert_eq!((bad.progress, bad.from, bad.refused), (0, NO_SIDE, None));
        assert_eq!(Transit::from_ints(&[]), Transit { progress: 0, from: 0, origin: None, refused: None });
        assert_eq!(arrived(&Transit { progress: 130, from: 2, origin: None, refused: None }, 5).progress, 30);
    }

    #[test]
    fn which_blocks_count_as_inventories() {
        for b in ["minecraft:chest", "minecraft:hopper", "minecraft:white_shulker_box", "minecraft:furnace", "create:depot"] {
            assert!(is_inventory_block(b), "{b}");
        }
        for b in ["minecraft:air", "minecraft:stone", "minecraft:iron_block", "mekanism:basic_logistical_transporter", "mekanism:steel_casing", ""] {
            assert!(!is_inventory_block(b), "{b}");
        }
        assert!(is_inventory_block("mekanism:basic_bin"));
        assert!(is_inventory_block("mekanism:enrichment_chamber"));
    }

    #[test]
    fn a_look_at_the_arm_picks_that_side() {
        // Standing west of a transporter at the origin, above a chest on its west, looking down
        // at the west arm: west (4) is picked although the chest hides the west face.
        let eye = [-1.2, 1.6, 0.5];
        let target = [0.15, 0.5, 0.5];
        let dir = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
        let all = [true; 6];
        assert_eq!(picked_side([0, 0, 0], eye, dir, all), Some(4));
        // Straight down the north arm: north.
        assert_eq!(picked_side([0, 0, 0], [0.5, 0.5, -2.0], [0.0, 0.0, 1.0], all), Some(2));
        // Diagonally into the core, past every arm: the clicked face decides.
        assert_eq!(picked_side([0, 0, 0], [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0], all), None);
        // Missing the block entirely.
        assert_eq!(picked_side([0, 0, 0], [5.0, 5.0, 5.0], [1.0, 0.0, 0.0], all), None);
        // Seen from the north, grazing the top of an unjoined north arm on the way to the joined
        // west arm (as in game): west, since the north arm is not there.
        let (eye, target) = ([1.0, 1.27, -1.5], [0.24, 0.57, 0.31]);
        let dir = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
        let west_east = [false, false, false, false, true, true];
        assert_eq!(picked_side([0, 0, 0], eye, dir, west_east), Some(4));
        // Down the unjoined north arm into the core: the clicked face decides.
        assert_eq!(picked_side([0, 0, 0], [0.5, 0.5, -2.0], [0.0, 0.0, 1.0], west_east), None);
    }

    #[test]
    fn view_vector_matches_minecraft() {
        let south = view_vector(0.0, 0.0);
        assert!((south[2] - 1.0).abs() < 1e-9 && south[0].abs() < 1e-9);
        let down = view_vector(0.0, 90.0);
        assert!((down[1] + 1.0).abs() < 1e-9);
    }
}
