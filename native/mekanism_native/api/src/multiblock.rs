//! General multiblock structure validation (M09), free of Horizon types so its
//! tests run in `logic/energy`. The host side is `multiblocks.rs`.
//!
//! Ported from Mekanism 26.3 `lib/multiblock/CuboidStructureValidator`,
//! `FormationProtocol` and `StructureHelper`: the structure is a cuboid whose
//! 12 edges are frame blocks, whose faces are frame/valve blocks (or the
//! controller) and whose interior is empty (air), or, for the Induction
//! Matrix, may also hold cells and providers. Sizes are 3..=18 per axis.
//!
//! Not ported: `Structure`/`VoxelPlane` (Mekanism's incremental plane
//! scanner). Here the connected shell blocks are flood-filled from the seed,
//! their bounding box is the cuboid, and every position of that box is then
//! checked against the rule of its place (edge, face, inside) like
//! `validateNode` does. The accepted shapes are the same.

use std::collections::{HashMap, HashSet, VecDeque};

pub type Pos = [i32; 3];

/// What one world position is, as far as a structure is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Chunk not loaded: the answer is unknown, never "broken".
    Unloaded,
    Air,
    /// Casing block (Dynamic Tank, Induction Casing, Thermal Evaporation Block).
    Frame,
    /// Valve or port: allowed on faces only.
    Valve,
    /// The structure's controller block: allowed on faces, exactly one.
    Controller,
    /// A block that may sit inside (Induction Cell/Provider).
    Interior,
    /// Anything else.
    Foreign,
}

impl Cell {
    fn is_shell(self) -> bool {
        matches!(self, Cell::Frame | Cell::Valve | Cell::Controller)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Tank,
    Matrix,
    Evaporation,
}

/// The blocks one structure type is built from, and its rules.
#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub ty: Type,
    pub rules: Rules,
    pub frames: &'static [&'static str],
    /// Frames without a block entity (Structural Glass): they join the shell but hold no state.
    pub plain: &'static [&'static str],
    pub valves: &'static [&'static str],
    pub controllers: &'static [&'static str],
    pub interior: &'static [&'static str],
}

impl Def {
    pub fn classify(&self, block: &str) -> Cell {
        let any = |list: &[&str]| list.contains(&block);
        if any(self.frames) || any(self.plain) {
            Cell::Frame
        } else if any(self.valves) {
            Cell::Valve
        } else if any(self.controllers) {
            Cell::Controller
        } else if any(self.interior) {
            Cell::Interior
        } else if block == "minecraft:air" || block == "minecraft:cave_air" || block == "minecraft:void_air" {
            Cell::Air
        } else {
            Cell::Foreign
        }
    }

    /// Whether `block` is a shell block with a block entity (carries the structure's link).
    pub fn is_member(&self, block: &str) -> bool {
        self.frames.contains(&block) || self.valves.contains(&block) || self.controllers.contains(&block)
    }
}

/// What a structure type demands (`CuboidStructureValidator` + the tile's constants).
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub min: i32,
    pub max: i32,
    /// Width and depth must be exactly this (evaporation plant: 4).
    pub footprint: Option<i32>,
    /// Top layer is open: its corners are ignored, its ring is a face, its inside is air.
    pub open_top: bool,
    /// Exactly one controller (a second one is a failure, `noIgnore`).
    pub one_controller: bool,
    /// Cells and providers may fill the inside.
    pub interior_blocks: bool,
}

/// `CuboidStructureValidator.MAX_SIZE`; every type's minimum is 3.
pub const MAX_SIZE: i32 = 18;

pub const TANK: Rules =
    Rules { min: 3, max: MAX_SIZE, footprint: None, open_top: false, one_controller: false, interior_blocks: false };
pub const MATRIX: Rules =
    Rules { min: 3, max: MAX_SIZE, footprint: None, open_top: false, one_controller: false, interior_blocks: true };
pub const EVAPORATION: Rules =
    Rules { min: 3, max: MAX_SIZE, footprint: Some(4), open_top: true, one_controller: true, interior_blocks: false };

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Structure {
    pub min: Pos,
    pub max: Pos,
    /// Every frame, valve and controller block, sorted (y, x, z).
    pub shell: Vec<Pos>,
    pub valves: Vec<Pos>,
    pub controllers: Vec<Pos>,
    /// Interior blocks that are not air (cells, providers), sorted.
    pub interior: Vec<Pos>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Formed(Structure),
    /// Why the shape is not valid (shown to players, kept terse).
    Invalid(&'static str),
    /// A needed chunk is not loaded: neither form nor dissolve anything.
    Unloaded,
}

impl Structure {
    pub fn size(&self) -> [i32; 3] {
        [self.max[0] - self.min[0] + 1, self.max[1] - self.min[1] + 1, self.max[2] - self.min[2] + 1]
    }

    /// Volume of the whole bounding box including the walls (`Structure.getVolume`).
    pub fn volume(&self) -> i64 {
        self.size().iter().map(|&s| s as i64).product()
    }

    /// Blocks strictly inside the walls.
    pub fn inner_volume(&self) -> i64 {
        self.size().iter().map(|&s| (s - 2) as i64).product()
    }

    pub fn contains(&self, p: Pos) -> bool {
        (0..3).all(|a| p[a] >= self.min[a] && p[a] <= self.max[a])
    }

    /// The block that holds the shared state: the controller, else the lowest shell
    /// block `has_entity` accepts (sorted order is deterministic, so every member
    /// agrees on it after a re-validation).
    pub fn master(&self, has_entity: impl Fn(Pos) -> bool) -> Option<Pos> {
        self.controllers.first().copied().or_else(|| self.shell.iter().copied().find(|&p| has_entity(p)))
    }
}

const NEIGHBORS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn add(p: Pos, d: [i32; 3]) -> Pos {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

/// Validate the structure `seed` belongs to. `classify` reads the world.
pub fn validate(seed: Pos, rules: &Rules, classify: &mut dyn FnMut(Pos) -> Cell) -> Outcome {
    let mut memo: HashMap<Pos, Cell> = HashMap::new();
    let mut at = |p: Pos| *memo.entry(p).or_insert_with(|| classify(p));

    match at(seed) {
        Cell::Unloaded => return Outcome::Unloaded,
        c if c.is_shell() => {}
        _ => return Outcome::Invalid("not part of a structure"),
    }

    // Flood-fill the connected shell, giving up as soon as it is too large.
    let mut seen: HashSet<Pos> = HashSet::from([seed]);
    let mut queue = VecDeque::from([seed]);
    let (mut lo, mut hi) = (seed, seed);
    while let Some(p) = queue.pop_front() {
        for d in NEIGHBORS {
            let n = add(p, d);
            if seen.contains(&n) {
                continue;
            }
            match at(n) {
                Cell::Unloaded => return Outcome::Unloaded,
                c if c.is_shell() => {
                    seen.insert(n);
                    for a in 0..3 {
                        lo[a] = lo[a].min(n[a]);
                        hi[a] = hi[a].max(n[a]);
                        if hi[a] - lo[a] + 1 > rules.max {
                            return Outcome::Invalid("too large");
                        }
                    }
                    queue.push_back(n);
                }
                _ => {}
            }
        }
    }

    let size = [hi[0] - lo[0] + 1, hi[1] - lo[1] + 1, hi[2] - lo[2] + 1];
    if size.iter().any(|&s| s < rules.min) {
        return Outcome::Invalid("too small");
    }
    if let Some(f) = rules.footprint {
        if size[0] != f || size[2] != f {
            return Outcome::Invalid("wrong footprint");
        }
    }

    let mut s = Structure { min: lo, max: hi, shell: Vec::new(), valves: Vec::new(), controllers: Vec::new(), interior: Vec::new() };
    for y in lo[1]..=hi[1] {
        for x in lo[0]..=hi[0] {
            for z in lo[2]..=hi[2] {
                let p = [x, y, z];
                let cell = at(p);
                if cell == Cell::Unloaded {
                    return Outcome::Unloaded;
                }
                let on = |v: i32, a: i32, b: i32| (v == a || v == b) as u8;
                let extremes = on(x, lo[0], hi[0]) + on(y, lo[1], hi[1]) + on(z, lo[2], hi[2]);
                let ring = on(x, lo[0], hi[0]) + on(z, lo[2], hi[2]);
                // Where the open top layer departs from the plain cuboid rules.
                let place = if rules.open_top && y == hi[1] {
                    match ring {
                        2 => Place::Ignored,
                        1 => Place::Face,
                        _ => Place::Inside,
                    }
                } else {
                    match extremes {
                        2.. => Place::Edge,
                        1 => Place::Face,
                        _ => Place::Inside,
                    }
                };
                let fits = match place {
                    Place::Edge => cell == Cell::Frame,
                    Place::Face => cell.is_shell(),
                    Place::Inside => cell == Cell::Air || (cell == Cell::Interior && rules.interior_blocks),
                    // An ignored position takes anything except a second controller.
                    Place::Ignored => cell != Cell::Controller,
                };
                if !fits {
                    return Outcome::Invalid(match place {
                        Place::Edge => "edge must be frame",
                        Place::Face => "wall must be frame or valve",
                        Place::Inside => "inside must be empty",
                        Place::Ignored => "controller in a corner",
                    });
                }
                match cell {
                    Cell::Frame => s.shell.push(p),
                    Cell::Valve => {
                        s.shell.push(p);
                        s.valves.push(p);
                    }
                    Cell::Controller => {
                        s.shell.push(p);
                        s.controllers.push(p);
                    }
                    Cell::Interior if place == Place::Inside => s.interior.push(p),
                    _ => {}
                }
            }
        }
    }
    if rules.one_controller && s.controllers.len() != 1 {
        return Outcome::Invalid(if s.controllers.is_empty() { "no controller" } else { "more than one controller" });
    }
    Outcome::Formed(s)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Edge,
    Face,
    Inside,
    Ignored,
}

/// Sum the contents several members carried before they merged into one structure
/// (`MultiblockCache.merge`): the first non-empty fluid decides, only that fluid's
/// amounts add up, and the result never exceeds `capacity`.
pub fn merge_stored(parts: impl IntoIterator<Item = (Option<String>, i64)>, capacity: i64) -> (Option<String>, i64) {
    let mut fluid: Option<String> = None;
    let mut total = 0i64;
    for (f, amount) in parts {
        if amount <= 0 {
            continue;
        }
        match (&fluid, f) {
            (None, Some(f)) => {
                fluid = Some(f);
                total = total.saturating_add(amount);
            }
            (Some(have), Some(f)) if *have == f => total = total.saturating_add(amount),
            _ => {}
        }
    }
    let total = total.min(capacity.max(0));
    if total == 0 { (None, 0) } else { (fluid, total) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world of one hollow box built from `edge`, `face` and the inside filled with `inside`.
    fn boxed(min: Pos, max: Pos) -> HashMap<Pos, Cell> {
        let mut w = HashMap::new();
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    let e = (x == min[0] || x == max[0]) as u8 + (y == min[1] || y == max[1]) as u8 + (z == min[2] || z == max[2]) as u8;
                    w.insert([x, y, z], if e >= 1 { Cell::Frame } else { Cell::Air });
                }
            }
        }
        w
    }

    fn run(w: &HashMap<Pos, Cell>, seed: Pos, rules: &Rules) -> Outcome {
        validate(seed, rules, &mut |p| w.get(&p).copied().unwrap_or(Cell::Air))
    }

    fn formed(o: Outcome) -> Structure {
        match o {
            Outcome::Formed(s) => s,
            other => panic!("expected formed, got {other:?}"),
        }
    }

    #[test]
    fn smallest_tank_forms() {
        let w = boxed([0, 0, 0], [2, 2, 2]);
        let s = formed(run(&w, [0, 0, 0], &TANK));
        assert_eq!(s.size(), [3, 3, 3]);
        assert_eq!(s.volume(), 27);
        assert_eq!(s.inner_volume(), 1);
        assert_eq!(s.shell.len(), 26);
        assert!(s.interior.is_empty());
    }

    #[test]
    fn any_member_gives_the_same_structure() {
        let w = boxed([5, 64, -3], [9, 70, 2]);
        let a = formed(run(&w, [5, 64, -3], &TANK));
        let b = formed(run(&w, [7, 70, 0], &TANK));
        assert_eq!(a, b);
        assert_eq!(a.size(), [5, 7, 6]);
        assert_eq!(a.master(|_| true), Some([5, 64, -3]), "lowest layer first, then x, then z");
    }

    #[test]
    fn size_limits() {
        let w = boxed([0, 0, 0], [1, 2, 2]);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("too small"));
        let w = boxed([0, 0, 0], [17, 17, 17]);
        assert!(matches!(run(&w, [0, 0, 0], &TANK), Outcome::Formed(_)), "18 is the maximum");
        let w = boxed([0, 0, 0], [18, 5, 5]);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("too large"));
    }

    #[test]
    fn broken_shapes_are_refused() {
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([1, 0, 0], Cell::Air); // hole in an edge
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("edge must be frame"));
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([0, 1, 1], Cell::Air); // hole in a face
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("wall must be frame or valve"));
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([1, 1, 1], Cell::Foreign);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("inside must be empty"));
        assert_eq!(run(&w, [1, 1, 1], &TANK), Outcome::Invalid("not part of a structure"));
    }

    #[test]
    fn valves_only_on_faces() {
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([0, 1, 2], Cell::Valve);
        w.insert([3, 2, 1], Cell::Valve);
        let s = formed(run(&w, [0, 0, 0], &TANK));
        assert_eq!(s.valves, vec![[0, 1, 2], [3, 2, 1]]);
        w.insert([2, 0, 0], Cell::Valve);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("edge must be frame"));
    }

    #[test]
    fn matrix_allows_interior_blocks_tank_does_not() {
        let mut w = boxed([0, 0, 0], [4, 4, 4]);
        w.insert([1, 1, 1], Cell::Interior);
        w.insert([2, 1, 1], Cell::Interior);
        let s = formed(run(&w, [0, 0, 0], &MATRIX));
        assert_eq!(s.interior, vec![[1, 1, 1], [2, 1, 1]]);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Invalid("inside must be empty"));
    }

    #[test]
    fn unloaded_chunks_never_dissolve() {
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([3, 3, 3], Cell::Unloaded);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Unloaded);
        assert_eq!(run(&w, [3, 3, 3], &TANK), Outcome::Unloaded);
        let mut w = boxed([0, 0, 0], [3, 3, 3]);
        w.insert([1, 1, 1], Cell::Unloaded);
        assert_eq!(run(&w, [0, 0, 0], &TANK), Outcome::Unloaded);
    }

    /// 4x4 footprint, 5 high: open top, corners free, one controller on a wall.
    fn plant() -> HashMap<Pos, Cell> {
        let mut w = boxed([0, 0, 0], [3, 4, 3]);
        for x in 0..=3 {
            for z in 0..=3 {
                let corner = (x == 0 || x == 3) && (z == 0 || z == 3);
                let ring = !corner && (x == 0 || x == 3 || z == 0 || z == 3);
                w.insert([x, 4, z], if corner { Cell::Air } else if ring { Cell::Frame } else { Cell::Air });
            }
        }
        w.insert([0, 2, 1], Cell::Controller);
        w
    }

    #[test]
    fn evaporation_plant_forms_with_open_top() {
        let w = plant();
        let s = formed(run(&w, [0, 0, 0], &EVAPORATION));
        assert_eq!(s.size(), [4, 5, 4]);
        assert_eq!(s.controllers, vec![[0, 2, 1]]);
        assert_eq!(s.master(|_| false), Some([0, 2, 1]));
        let mut w = plant();
        w.insert([0, 4, 0], Cell::Foreign); // a solar panel on a top corner
        assert!(matches!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Formed(_)));
    }

    #[test]
    fn evaporation_plant_rules() {
        let mut w = plant();
        w.insert([3, 1, 2], Cell::Controller);
        assert_eq!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Invalid("more than one controller"));
        let mut w = plant();
        w.insert([0, 2, 1], Cell::Frame);
        assert_eq!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Invalid("no controller"));
        let mut w = plant();
        w.insert([0, 4, 3], Cell::Controller);
        assert_eq!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Invalid("controller in a corner"));
        let w = boxed([0, 0, 0], [4, 4, 3]);
        assert_eq!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Invalid("wrong footprint"));
        let mut w = plant();
        w.insert([1, 4, 1], Cell::Foreign); // the top layer's inside stays open
        assert_eq!(run(&w, [0, 0, 0], &EVAPORATION), Outcome::Invalid("inside must be empty"));
    }

    #[test]
    fn merging_caches() {
        let w = |s: &str| Some(s.to_owned());
        assert_eq!(merge_stored([], 100), (None, 0));
        assert_eq!(merge_stored([(w("a"), 30), (w("a"), 40)], 100), (w("a"), 70));
        assert_eq!(merge_stored([(w("a"), 30), (w("b"), 40)], 100), (w("a"), 30), "other fluid is lost");
        assert_eq!(merge_stored([(None, 0), (w("b"), 40)], 100), (w("b"), 40));
        assert_eq!(merge_stored([(w("a"), 90), (w("a"), 40)], 100), (w("a"), 100), "clamped to capacity");
        assert_eq!(merge_stored([(w("a"), i64::MAX), (w("a"), 5)], i64::MAX), (w("a"), i64::MAX));
    }
}
