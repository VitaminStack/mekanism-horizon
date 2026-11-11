//! Rules of the hand-held gear (M11): Canteen, Gauge Dropper and Atomic Disassembler. No
//! Horizon types, so the tests run in `logic/energy`; `items.rs` wires them to the hooks.
//!
//! Values are Mekanism 26.3's defaults (`GearConfig` and the item classes) as far as they
//! are known here; the upstream checkout used for assets has no Java sources, so the
//! numbers below are the ones recorded in docs/ITEM_HOOKS.md, not re-read from the code.

use std::collections::HashSet;

pub const DICTIONARY: &str = "mekanism:dictionary";
pub const CANTEEN: &str = "mekanism:canteen";
pub const GAUGE_DROPPER: &str = "mekanism:gauge_dropper";
pub const DISASSEMBLER: &str = "mekanism:atomic_disassembler";

/// Mekanism's nutritional paste is the one fluid a canteen holds.
pub const NUTRITIONAL_PASTE: &str = "mekanism:nutritional_paste";
pub const CANTEEN_CAPACITY: i32 = 64_000;
/// mB of paste for one food point (`MekanismConfig.general.nutritionalPasteMBPerFood`).
pub const PASTE_PER_FOOD: i32 = 50;
/// Ticks a drink takes (`ItemCanteen.getUseDuration`).
pub const CANTEEN_USE_TICKS: i32 = 32;
pub const MAX_FOOD: i32 = 20;

/// The Gauge Dropper holds 16 buckets and moves 16 mB a click (`ItemGaugeDropper`).
pub const DROPPER_CAPACITY: i32 = 16_000;
pub const DROPPER_STEP: i32 = 16;

pub const DISASSEMBLER_CAPACITY: i32 = 1_000_000;
pub const DISASSEMBLER_RATE: i32 = 5_000;
/// Energy for one hit with the extra weapon damage (`GearConfig` disassembler weapon usage).
pub const WEAPON_COST: i32 = 2_000;
/// Extra damage on top of the item's own attack, which the template gives without energy.
pub const WEAPON_BONUS: f32 = 16.0;
/// Energy per block mined, and per block of a vein.
pub const MINE_COST: i32 = 10;
/// Most blocks one vein break takes (`GearConfig` disassembler vein limit: 128).
pub const VEIN_LIMIT: usize = 128;

/// What a canteen does for a player with `food` food points and `stored` mB: the food gained
/// and the paste used, in whole food points; none when the bar is full or the canteen empty.
pub fn drink(stored: i32, food: i32) -> Option<(i32, i32)> {
    let need = MAX_FOOD - food.clamp(0, MAX_FOOD);
    let gain = need.min(stored.max(0) / PASTE_PER_FOOD);
    (gain > 0).then_some((gain, gain * PASTE_PER_FOOD))
}

/// Whether a drink would start: hungry and some paste left.
pub fn can_drink(stored: i32, food: i32) -> bool {
    food < MAX_FOOD && stored >= PASTE_PER_FOOD
}

/// A fluid held in a gauge dropper: `(fluid, amount)`.
pub type Held = (Option<String>, i32);

/// Draw up to `DROPPER_STEP` from a tank holding `tank` into a dropper holding `held`. The
/// dropper takes only the fluid it already holds (or any when empty) and as much as fits.
/// Returns the fluid and the amount moved.
pub fn draw(held: &Held, tank: &(String, i32)) -> Option<(String, i32)> {
    let (fluid, have) = tank;
    if *have <= 0 || held.0.as_deref().is_some_and(|f| f != fluid) {
        return None;
    }
    let moved = DROPPER_STEP.min(*have).min(DROPPER_CAPACITY - held.1.max(0));
    (moved > 0).then(|| (fluid.clone(), moved))
}

/// Put up to `DROPPER_STEP` of the dropper's content into a tank with `room` mB free: the amount moved.
pub fn put(held: &Held, room: i32) -> i32 {
    if held.0.is_none() { 0 } else { DROPPER_STEP.min(held.1.max(0)).min(room.max(0)) }
}

/// The disassembler's mining modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One block at a time, as any tool.
    Normal,
    /// Breaks the whole vein of an ore the block belongs to.
    Vein,
}

impl Mode {
    pub fn from_byte(b: u8) -> Mode {
        if b == 1 { Mode::Vein } else { Mode::Normal }
    }

    pub fn byte(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> Mode {
        match self {
            Mode::Normal => Mode::Vein,
            Mode::Vein => Mode::Normal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Normal => "Normal",
            Mode::Vein => "Vein Mining",
        }
    }
}

/// How many blocks of a vein the energy pays for: `MINE_COST` each, the first block included.
pub fn affordable(energy: i32) -> usize {
    (energy.max(0) / MINE_COST) as usize
}

/// The blocks joined to `start` (not `start` itself) for which `same` holds, found breadth-first
/// through all 26 neighbours, at most `limit`. The order is nearest first.
pub fn vein(start: [i32; 3], limit: usize, mut same: impl FnMut([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    let mut seen = HashSet::from([start]);
    let mut queue = std::collections::VecDeque::from([start]);
    let mut found = Vec::new();
    while let Some(at) = queue.pop_front() {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let next = [at[0] + dx, at[1] + dy, at[2] + dz];
                    if found.len() >= limit || !seen.insert(next) || !same(next) {
                        continue;
                    }
                    found.push(next);
                    queue.push_back(next);
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canteen_feeds_in_whole_food_points() {
        assert_eq!(drink(1_000, 10), Some((10, 500)));
        // Not enough paste for all the hunger: what it pays for.
        assert_eq!(drink(120, 0), Some((2, 100)));
        assert_eq!(drink(49, 0), None);
        assert_eq!(drink(5_000, MAX_FOOD), None);
        assert!(can_drink(50, 19));
        assert!(!can_drink(49, 0));
        assert!(!can_drink(64_000, 20));
    }

    #[test]
    fn dropper_moves_a_step_of_one_fluid() {
        let tank = ("mekanism:chemical/hydrogen".to_owned(), 1_000);
        assert_eq!(draw(&(None, 0), &tank), Some((tank.0.clone(), DROPPER_STEP)));
        // Only the fluid it holds, and what fits.
        assert_eq!(draw(&(Some("minecraft:water".into()), 5), &tank), None);
        assert_eq!(draw(&(Some(tank.0.clone()), DROPPER_CAPACITY - 4), &tank), Some((tank.0.clone(), 4)));
        assert_eq!(draw(&(Some(tank.0.clone()), DROPPER_CAPACITY), &tank), None);
        assert_eq!(draw(&(None, 0), &(tank.0.clone(), 0)), None);
        assert_eq!(draw(&(None, 0), &(tank.0, 9)), Some(("mekanism:chemical/hydrogen".into(), 9)));
        assert_eq!(put(&(Some("a".into()), 100), 1_000), DROPPER_STEP);
        assert_eq!(put(&(Some("a".into()), 7), 1_000), 7);
        assert_eq!(put(&(Some("a".into()), 100), 3), 3);
        assert_eq!(put(&(None, 0), 100), 0);
    }

    #[test]
    fn modes_cycle() {
        assert_eq!(Mode::Normal.next(), Mode::Vein);
        assert_eq!(Mode::Vein.next(), Mode::Normal);
        assert_eq!(Mode::from_byte(Mode::Vein.byte()), Mode::Vein);
        assert_eq!(Mode::from_byte(9), Mode::Normal);
    }

    #[test]
    fn vein_follows_touching_blocks() {
        // A diagonal chain of ore, and one block apart from it.
        let ore: HashSet<[i32; 3]> = [[1, 0, 0], [2, 1, 0], [3, 1, 1], [9, 9, 9]].into();
        let found = vein([0, 0, 0], VEIN_LIMIT, |p| ore.contains(&p));
        assert_eq!(found, vec![[1, 0, 0], [2, 1, 0], [3, 1, 1]]);
        assert!(vein([0, 0, 0], VEIN_LIMIT, |_| false).is_empty());
    }

    #[test]
    fn vein_stops_at_the_limit() {
        let line = |p: [i32; 3]| p[1] == 0 && p[2] == 0 && p[0] >= 0;
        assert_eq!(vein([0, 0, 0], 5, line).len(), 5);
        // Never the start itself, and no block twice.
        let all = vein([0, 0, 0], 1_000, |p| p[0].abs() <= 2 && p[1].abs() <= 2 && p[2].abs() <= 2);
        assert_eq!(all.len(), 124);
        assert!(!all.contains(&[0, 0, 0]));
    }

    #[test]
    fn energy_pays_for_blocks() {
        assert_eq!(affordable(0), 0);
        assert_eq!(affordable(25), 2);
        assert_eq!(affordable(-5), 0);
    }
}
