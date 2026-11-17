//! The Teleporter's rules (M08), with no Horizon types so the tests run in `logic/energy`.
//! The host side is `teleporters.rs`.
//!
//! Ported from Mekanism 26.3:
//! - `content/teleporter/TileEntityTeleporter` (`calculateEnergyCost`, `hasFrame`,
//!   `getTeleporterTargetPos`, `getTeleportBounds`, `getStatus`, `teleDelay`, `didTeleport`)
//! - `config/UsageConfig` (`teleporterBase` 100, `teleporterDistance` 2,
//!   `teleporterDimensionPenalty` 1000) and `config/StorageConfig` (`teleporter` 1,000).
//!
//! All numbers are FE as in Mekanism (`docs/ENERGY-UNITS.md`: 1:1).

use std::collections::BTreeSet;

use super::frequency::Pos;

/// `UsageConfig.teleporterBase`.
pub const BASE_COST: i32 = 100;
/// `UsageConfig.teleporterDistance`: per block.
pub const DISTANCE_COST: f64 = 2.0;
/// `UsageConfig.teleporterDimensionPenalty`.
pub const DIMENSION_PENALTY: i32 = 1000;
/// `StorageConfig.teleporter`. Mekanism 26.3 carries a "26.3: Update these config values"
/// TODO here: 1,000 FE can pay for no cross-dimension teleport (at least 1,100) and for
/// no teleport over about 450 blocks. Ported as it is, in this one place, on purpose.
pub const ENERGY_CAPACITY: i32 = 1_000;
/// `TileEntityTeleporter.teleDelay` after a teleport lands (ticks).
pub const DELAY_TICKS: u64 = 5;
/// A tick in milliseconds (the host measures time with the wall clock).
pub const TICK_MS: u64 = 50;
/// `GearConfig.portableTeleporter`: capacity and charge rate (FE).
pub const PORTABLE_CAPACITY: i32 = 1_000_000;
pub const PORTABLE_CHARGE_RATE: i32 = 5_000;

pub const OVERWORLD: &str = "minecraft:overworld";
pub const NETHER: &str = "minecraft:the_nether";
pub const END: &str = "minecraft:the_end";

/// The six directions, `Direction` in Minecraft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

pub const DIRS: [Dir; 6] = [Dir::Down, Dir::Up, Dir::North, Dir::South, Dir::West, Dir::East];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

impl Dir {
    pub fn step(self) -> Pos {
        match self {
            Dir::Down => [0, -1, 0],
            Dir::Up => [0, 1, 0],
            Dir::North => [0, 0, -1],
            Dir::South => [0, 0, 1],
            Dir::West => [-1, 0, 0],
            Dir::East => [1, 0, 0],
        }
    }

    fn axis(self) -> Axis {
        match self {
            Dir::Down | Dir::Up => Axis::Y,
            Dir::North | Dir::South => Axis::Z,
            Dir::West | Dir::East => Axis::X,
        }
    }

    /// `Direction.toYRot`-based yaw of a teleport through a frame facing `self`: Mekanism
    /// turns the player to look out of the frame (`getTeleportYaw`).
    pub fn yaw(self) -> Option<f32> {
        match self {
            Dir::North => Some(180.0),
            Dir::South => Some(0.0),
            Dir::West => Some(90.0),
            Dir::East => Some(270.0),
            Dir::Down | Dir::Up => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dir::Down => "down",
            Dir::Up => "up",
            Dir::North => "north",
            Dir::South => "south",
            Dir::West => "west",
            Dir::East => "east",
        }
    }
}

fn at(pos: Pos, dir: Pos, n: i32) -> Pos {
    [pos[0] + dir[0] * n, pos[1] + dir[1] * n, pos[2] + dir[2] * n]
}

fn axis_step(axis: Axis) -> Pos {
    match axis {
        Axis::X => [1, 0, 0],
        Axis::Y => [0, 1, 0],
        Axis::Z => [0, 0, 1],
    }
}

/// The frame found around a teleporter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub dir: Dir,
    pub rotated: bool,
}

/// `TileEntityTeleporter.hasFrame`: for each direction (non-rotated, then rotated), the
/// frame is the two blocks beside the teleporter, the block three away, and a pair beside
/// each of the 1, 2 and 3 blocks away along the direction. `is_frame` tells whether the
/// block at a position is a `mekanism:teleporter_frame`.
pub fn find_frame(pos: Pos, is_frame: &dyn Fn(Pos) -> bool) -> Option<Frame> {
    for dir in DIRS {
        for rotated in [false, true] {
            let alt = match (rotated, dir.axis()) {
                (true, Axis::Z) => Axis::X,
                (true, _) => Axis::Z,
                (false, Axis::Y) => Axis::X,
                (false, _) => Axis::Y,
            };
            let a = axis_step(alt);
            let pair = |n: i32| {
                let centre = at(pos, dir.step(), n);
                is_frame(at(centre, a, 1)) && is_frame(at(centre, a, -1))
            };
            if pair(0) && is_frame(at(pos, dir.step(), 3)) && pair(1) && pair(2) && pair(3) {
                return Some(Frame { dir, rotated });
            }
        }
    }
    None
}

/// `getTeleporterTargetPos`: where a teleport to this teleporter lands (a block position;
/// the player stands in its middle).
pub fn target_pos(pos: Pos, frame: Option<Frame>) -> Pos {
    match frame {
        None => at(pos, [0, 1, 0], 1),
        Some(Frame { dir: Dir::Down, .. }) => at(pos, [0, -1, 0], 2),
        Some(f) => at(pos, f.dir.step(), 1),
    }
}

/// `getTeleportBounds`: the blocks one and two away from the teleporter along the frame,
/// as `[min, max]` corners (`max` exclusive), or `None` without a frame.
pub fn bounds(pos: Pos, frame: Option<Frame>) -> Option<([f64; 3], [f64; 3])> {
    let f = frame?;
    let (a, b) = (at(pos, f.dir.step(), 1), at(pos, f.dir.step(), 2));
    let min = [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])].map(f64::from);
    let max = [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])].map(|v| f64::from(v) + 1.0);
    Some((min, max))
}

/// Whether `p` is inside `bounds` (`AABB.contains`: lower inclusive, upper exclusive).
pub fn inside(bounds: ([f64; 3], [f64; 3]), p: [f64; 3]) -> bool {
    (0..3).all(|i| p[i] >= bounds.0[i] && p[i] < bounds.1[i])
}

/// `getCoordinateScale`: how many overworld blocks one block of the dimension is.
pub fn coordinate_scale(dimension: &str) -> f64 {
    if dimension == NETHER {
        8.0
    } else {
        1.0
    }
}

/// `calculateEnergyCost` for one entity: `base`, plus the distance, plus the
/// dimension penalty when crossing dimensions, times the riders and the entity.
/// `from` is the entity's position, `to` the destination's block position.
pub fn energy_cost(from: [f64; 3], from_dim: &str, to: Pos, to_dim: &str, passengers: u32) -> i32 {
    let distance_cost = if from_dim == to_dim {
        let d = (0..3).map(|i| (from[i] - f64::from(to[i])).powi(2)).sum::<f64>().sqrt();
        (DISTANCE_COST * d).round()
    } else {
        let (cur, tgt) = (coordinate_scale(from_dim), coordinate_scale(to_dim));
        let (x, z) = if cur <= tgt {
            (from[0] * (cur / tgt) - f64::from(to[0]), from[2] * (cur / tgt) - f64::from(to[2]))
        } else {
            (from[0] - f64::from(to[0]) * (tgt / cur), from[2] - f64::from(to[2]) * (tgt / cur))
        };
        let y = from[1] - f64::from(to[1]);
        let d = (x * x + y * y + z * z).sqrt();
        f64::from(DIMENSION_PENALTY) + (DISTANCE_COST * d).round()
    };
    let cost = BASE_COST.saturating_add(saturated(distance_cost));
    cost.saturating_mul(1 + passengers.min(i32::MAX as u32 - 1) as i32)
}

/// `Mth.saturatedCast`-like: a double as an `i32`, clamped.
fn saturated(v: f64) -> i32 {
    v.clamp(0.0, f64::from(i32::MAX)) as i32
}

/// `TeleporterStatus` (the order is Mekanism's; only `Ready` is not an error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    NoFrequency,
    NoFrame,
    NoDestination,
    NotEnoughEnergy,
    Ready,
}

impl Status {
    pub fn code(self) -> i32 {
        self as i32
    }

    pub fn from_code(code: i32) -> Status {
        match code {
            0 => Status::NoFrequency,
            1 => Status::NoFrame,
            2 => Status::NoDestination,
            3 => Status::NotEnoughEnergy,
            _ => Status::Ready,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Status::NoFrequency => "No frequency",
            Status::NoFrame => "No frame",
            Status::NoDestination => "No link found",
            Status::NotEnoughEnergy => "Not enough energy",
            Status::Ready => "Ready",
        }
    }

    pub fn is_error(self) -> bool {
        self != Status::Ready
    }

    /// `getStatus`: the first problem in Mekanism's order. `energy` is the stored
    /// energy, `cheapest` what the cheapest teleport costs.
    pub fn of(has_frequency: bool, has_frame: bool, has_destination: bool, energy: i32, cheapest: i32) -> Status {
        if !has_frequency {
            Status::NoFrequency
        } else if !has_frame {
            Status::NoFrame
        } else if !has_destination {
            Status::NoDestination
        } else if energy < cheapest {
            Status::NotEnoughEnergy
        } else {
            Status::Ready
        }
    }
}

/// A teleporter's arrival state: the delay after a landing and who just arrived (`teleDelay`
/// and `didTeleport`). A player that arrived is not sent on again until they have stepped
/// out of the bounds; nobody is sent in the first ticks after a landing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    ready_at: u64,
    arrived: BTreeSet<u128>,
}

impl Gate {
    /// A teleport lands here at `now` (milliseconds).
    pub fn arrive(&mut self, player: u128, now: u64) {
        self.arrived.insert(player);
        self.ready_at = now + DELAY_TICKS * TICK_MS;
    }

    /// Whether `player`, standing in the bounds at `now`, would be teleported.
    pub fn may_send(&self, player: u128, now: u64) -> bool {
        now >= self.ready_at && !self.arrived.contains(&player)
    }

    /// `player` is not in the bounds.
    pub fn left(&mut self, player: u128) {
        self.arrived.remove(&player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn same_dimension_cost_is_base_plus_two_per_block() {
        // 30 blocks along x from the entity to the destination block: 100 + 2*30.
        assert_eq!(energy_cost([0.0, 64.0, 0.0], OVERWORLD, [30, 64, 0], OVERWORLD, 0), 160);
        // Raw 3-4-5: 2*5 = 10.
        assert_eq!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [3, 4, 0], OVERWORLD, 0), 110);
        // Rounded: 2*sqrt(2) = 2.83 -> 3.
        assert_eq!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [1, 1, 0], OVERWORLD, 0), 103);
        // Standing at the target costs just the base.
        assert_eq!(energy_cost([5.0, 5.0, 5.0], OVERWORLD, [5, 5, 5], OVERWORLD, 0), BASE_COST);
    }

    #[test]
    fn passengers_multiply_the_whole_cost() {
        assert_eq!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [3, 4, 0], OVERWORLD, 1), 220);
        assert_eq!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [3, 4, 0], OVERWORLD, 3), 440);
    }

    #[test]
    fn cost_saturates_instead_of_wrapping() {
        let far = energy_cost([0.0, 0.0, 0.0], OVERWORLD, [i32::MAX, 0, 0], OVERWORLD, 0);
        assert!(far > 0 && far <= i32::MAX, "distance 2^31 saturates the sum");
        assert_eq!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [i32::MAX, 0, 0], OVERWORLD, 2), i32::MAX);
        // A rider count at the integer limit does not overflow the factor.
        assert!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [0, 0, 0], OVERWORLD, u32::MAX) > 0);
    }

    #[test]
    fn crossing_dimensions_adds_the_penalty_and_scales_coordinates() {
        // Overworld (0,64,0) -> End: scales equal (1 <= 1), plain distance 10: 1000 + 20 + 100.
        assert_eq!(energy_cost([0.0, 64.0, 0.0], OVERWORLD, [10, 64, 0], END, 0), 1_120);
        // Overworld -> Nether: cur 1 <= tgt 8, the entity's x/z become x/8:
        // entity (80, 64, 0) is nether x = 10; destination (10, 64, 0): distance 0.
        assert_eq!(energy_cost([80.0, 64.0, 0.0], OVERWORLD, [10, 64, 0], NETHER, 0), 1_100);
        // Nether -> Overworld: cur 8 > tgt 1, the destination's x/z become x/8:
        // entity (10, 64, 0), destination (80, 64, 0) -> x 10: distance 0.
        assert_eq!(energy_cost([10.0, 64.0, 0.0], NETHER, [80, 64, 0], OVERWORLD, 0), 1_100);
        // y is 1:1: 5 blocks up the same x/z.
        assert_eq!(energy_cost([10.0, 69.0, 0.0], NETHER, [80, 64, 0], OVERWORLD, 0), 1_110);
        // Riders multiply the penalty too.
        assert_eq!(energy_cost([80.0, 64.0, 0.0], OVERWORLD, [10, 64, 0], NETHER, 1), 2_200);
    }

    #[test]
    fn the_stock_capacity_pays_for_short_hops_only() {
        // Mekanism's capacity is 1,000 (see ENERGY_CAPACITY): reach is (1000-100)/2 = 450 blocks.
        assert!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [450, 0, 0], OVERWORLD, 0) <= ENERGY_CAPACITY);
        assert!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [451, 0, 0], OVERWORLD, 0) > ENERGY_CAPACITY);
        assert!(energy_cost([0.0, 0.0, 0.0], OVERWORLD, [0, 0, 0], END, 0) > ENERGY_CAPACITY);
    }

    fn frame_set(blocks: &[Pos]) -> impl Fn(Pos) -> bool {
        let set: HashSet<Pos> = blocks.iter().copied().collect();
        move |p| set.contains(&p)
    }

    /// The blocks `find_frame` wants for `dir` and the alternating axis `alt`.
    fn build(pos: Pos, dir: Dir, alt: Pos) -> Vec<Pos> {
        let mut v = vec![at(pos, dir.step(), 3)];
        for n in 0..=3 {
            let c = at(pos, dir.step(), n);
            v.push(at(c, alt, 1));
            v.push(at(c, alt, -1));
        }
        v
    }

    #[test]
    fn frame_is_found_in_each_direction() {
        let pos = [100, 64, 100];
        for (dir, plain, rotated) in [
            (Dir::Up, [1, 0, 0], [0, 0, 1]),
            (Dir::Down, [1, 0, 0], [0, 0, 1]),
            (Dir::North, [0, 1, 0], [1, 0, 0]),
            (Dir::South, [0, 1, 0], [1, 0, 0]),
            (Dir::West, [0, 1, 0], [0, 0, 1]),
            (Dir::East, [0, 1, 0], [0, 0, 1]),
        ] {
            let f = find_frame(pos, &frame_set(&build(pos, dir, plain)));
            assert_eq!(f, Some(Frame { dir, rotated: false }), "{dir:?} plain");
            let f = find_frame(pos, &frame_set(&build(pos, dir, rotated)));
            assert_eq!(f, Some(Frame { dir, rotated: true }), "{dir:?} rotated");
        }
    }

    #[test]
    fn a_frame_missing_a_block_is_no_frame() {
        let pos = [0, 64, 0];
        let all = build(pos, Dir::North, [0, 1, 0]);
        assert!(find_frame(pos, &frame_set(&all)).is_some());
        for skip in 0..all.len() {
            let mut some = all.clone();
            some.remove(skip);
            assert_eq!(find_frame(pos, &frame_set(&some)), None, "block {skip} missing");
        }
        assert_eq!(find_frame(pos, &|_| false), None);
    }

    #[test]
    fn target_and_bounds_follow_the_frame() {
        let pos = [10, 64, 10];
        // No frame: stand on top.
        assert_eq!(target_pos(pos, None), [10, 65, 10]);
        assert_eq!(bounds(pos, None), None);
        // A downward frame: two blocks below (the player stands in the lower of the two).
        let down = Some(Frame { dir: Dir::Down, rotated: false });
        assert_eq!(target_pos(pos, down), [10, 62, 10]);
        assert_eq!(bounds(pos, down), Some(([10.0, 62.0, 10.0], [11.0, 64.0, 11.0])));
        // Sideways: the first block along the frame.
        let east = Some(Frame { dir: Dir::East, rotated: true });
        assert_eq!(target_pos(pos, east), [11, 64, 10]);
        assert_eq!(bounds(pos, east), Some(([11.0, 64.0, 10.0], [13.0, 65.0, 11.0])));
        let b = bounds(pos, east).unwrap();
        assert!(inside(b, [11.5, 64.0, 10.5]));
        assert!(inside(b, [12.9, 64.9, 10.9]));
        assert!(!inside(b, [13.0, 64.0, 10.5]), "upper bound is exclusive");
        assert!(!inside(b, [10.9, 64.0, 10.5]));
    }

    #[test]
    fn yaw_looks_out_of_the_frame() {
        assert_eq!(Dir::North.yaw(), Some(180.0));
        assert_eq!(Dir::South.yaw(), Some(0.0));
        assert_eq!(Dir::West.yaw(), Some(90.0));
        assert_eq!(Dir::East.yaw(), Some(270.0));
        assert_eq!(Dir::Up.yaw(), None);
        assert_eq!(Dir::Down.yaw(), None);
    }

    #[test]
    fn status_reports_the_first_problem_in_mekanisms_order() {
        use Status::*;
        assert_eq!(Status::of(false, false, false, 0, 100), NoFrequency);
        assert_eq!(Status::of(true, false, false, 0, 100), NoFrame);
        assert_eq!(Status::of(true, true, false, 0, 100), NoDestination);
        assert_eq!(Status::of(true, true, true, 99, 100), NotEnoughEnergy);
        assert_eq!(Status::of(true, true, true, 100, 100), Ready);
        assert!(NoFrequency.is_error() && !Ready.is_error());
        for s in [NoFrequency, NoFrame, NoDestination, NotEnoughEnergy, Ready] {
            assert_eq!(Status::from_code(s.code()), s);
        }
    }

    #[test]
    fn gate_blocks_ping_pong_until_the_player_steps_out() {
        let mut g = Gate::default();
        assert!(g.may_send(1, 0));
        g.arrive(1, 1_000);
        assert!(!g.may_send(1, 1_000), "delay");
        assert!(!g.may_send(2, 1_000 + DELAY_TICKS * TICK_MS - 1), "still in the delay, for anyone");
        assert!(g.may_send(2, 1_000 + DELAY_TICKS * TICK_MS), "another player after the delay");
        assert!(!g.may_send(1, 5_000), "the one who arrived stays until they leave");
        g.left(1);
        assert!(g.may_send(1, 5_000));
    }

    #[test]
    fn portable_and_status_constants_match_mekanism() {
        assert_eq!((BASE_COST, DIMENSION_PENALTY, ENERGY_CAPACITY, DELAY_TICKS), (100, 1_000, 1_000, 5));
        assert_eq!((PORTABLE_CAPACITY, PORTABLE_CHARGE_RATE), (1_000_000, 5_000));
        assert_eq!(coordinate_scale(NETHER), 8.0);
        assert_eq!(coordinate_scale(OVERWORLD), 1.0);
        assert_eq!(coordinate_scale("somemod:dim"), 1.0);
    }
}
