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

// --- Armour, bow and flamethrower (M11, second part) ---------------------------------------
// Values are read from Mekanism 26.3's Java: `ItemElectricBow`, `ItemFreeRunners`,
// `CommonPlayerTickHandler.livingFall`, `ItemScubaTank`, `ItemJetpack`, `ItemFlamethrower`.

pub const JETPACK: &str = "mekanism:jetpack";
pub const JETPACK_ARMORED: &str = "mekanism:jetpack_armored";
pub const FREE_RUNNERS: &str = "mekanism:free_runners";
pub const FREE_RUNNERS_ARMORED: &str = "mekanism:free_runners_armored";
pub const SCUBA_MASK: &str = "mekanism:scuba_mask";
pub const SCUBA_TANK: &str = "mekanism:scuba_tank";
pub const ELECTRIC_BOW: &str = "mekanism:electric_bow";
pub const FLAMETHROWER: &str = "mekanism:flamethrower";

pub const HYDROGEN: &str = "mekanism:chemical/hydrogen";
pub const OXYGEN: &str = "mekanism:chemical/oxygen";

/// Jetpack, scuba tank and flamethrower hold 24 buckets and fill 16 mB a click (`GearConfig`).
pub const CHEMICAL_CAPACITY: i32 = 24_000;

pub const BOW_CAPACITY: i32 = 120_000;
pub const BOW_RATE: i32 = 600;
pub const BOW_COST: i32 = 120;
pub const BOW_FIRE_COST: i32 = 1_200;

pub const RUNNERS_CAPACITY: i32 = 64_000;
pub const RUNNERS_RATE: i32 = 320;
/// Energy per point of fall damage taken off (`GearConfig` free runner fall energy cost).
pub const RUNNERS_FALL_COST: i32 = 50;
/// Share of the fall damage the boots can take off (`freeRunnerFallDamageRatio`).
pub const RUNNERS_FALL_RATIO: f64 = 1.0;
/// Blocks a fall is free of damage (`Attributes.SAFE_FALL_DISTANCE` default).
pub const SAFE_FALL: f64 = 3.0;
/// Added to `minecraft:step_height` in Normal mode while not sneaking (`ItemFreeRunners`).
pub const RUNNERS_STEP_BOOST: f64 = 0.5;
pub const BASE_STEP_HEIGHT: f64 = 0.6;

/// Free Runners' modes (`IFreeRunnerItem.FreeRunnerMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnerMode {
    /// No fall damage and the step boost.
    Normal,
    /// No fall damage only.
    Safety,
    Disabled,
}

impl RunnerMode {
    pub fn from_byte(b: u8) -> RunnerMode {
        match b {
            1 => RunnerMode::Safety,
            2 => RunnerMode::Disabled,
            _ => RunnerMode::Normal,
        }
    }

    pub fn byte(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> RunnerMode {
        match self {
            RunnerMode::Normal => RunnerMode::Safety,
            RunnerMode::Safety => RunnerMode::Disabled,
            RunnerMode::Disabled => RunnerMode::Normal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RunnerMode::Normal => "Normal",
            RunnerMode::Safety => "Safety",
            RunnerMode::Disabled => "Disabled",
        }
    }
}

/// What the boots do to a fall (`CommonPlayerTickHandler.livingFall`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    /// The whole fall damage is absorbed: cancel the event.
    pub cancel: bool,
    /// The fall distance the event goes on with when it is not cancelled.
    pub distance: f64,
    /// Energy the absorption takes.
    pub cost: i32,
}

/// The absorption of a fall of `distance` blocks (`multiplier` = the entity's fall damage
/// multiplier) by boots in `mode` holding `energy`; `None` when they do nothing.
pub fn absorb_fall(mode: RunnerMode, energy: i32, distance: f64, multiplier: f32) -> Option<Fall> {
    if mode == RunnerMode::Disabled || energy <= 0 || multiplier <= 0.0 {
        return None;
    }
    let multiplier = f64::from(multiplier);
    let damage = ((distance - SAFE_FALL).max(0.0) * multiplier).ceil();
    if damage <= 0.0 {
        return None;
    }
    let required = (f64::from(RUNNERS_FALL_COST) * damage * RUNNERS_FALL_RATIO).ceil();
    let extracted = f64::from(energy).min(required);
    let absorbed = RUNNERS_FALL_RATIO * extracted / required;
    let remaining = damage * (1.0 - absorbed);
    let cost = extracted as i32;
    if remaining <= 1e-4 {
        Some(Fall { cancel: true, distance, cost })
    } else {
        Some(Fall { cancel: false, distance: remaining / multiplier + SAFE_FALL, cost })
    }
}

/// The `minecraft:step_height` the boots want: boosted in Normal mode with energy, while not sneaking.
pub fn step_height(mode: RunnerMode, energy: i32, sneaking: bool) -> f64 {
    if mode == RunnerMode::Normal && energy > 0 && !sneaking {
        BASE_STEP_HEIGHT + RUNNERS_STEP_BOOST
    } else {
        BASE_STEP_HEIGHT
    }
}

/// Energy a bow shot takes, the fire mode (Flame) costing ten times more.
pub fn bow_cost(fire: bool) -> i32 {
    if fire { BOW_FIRE_COST } else { BOW_COST }
}

/// A shot is released only when the bow can pay for it.
pub fn bow_can_shoot(energy: i32, fire: bool) -> bool {
    energy >= bow_cost(fire)
}

/// Max air supply of a living entity in ticks (`LivingEntity.getMaxAirSupply`).
pub const MAX_AIR: i32 = 300;

/// A scuba set works with the mask on the head, a tank with oxygen on the chest and the tank's switch on.
pub fn scuba_active(mask: bool, oxygen: i32, switched_on: bool) -> bool {
    mask && switched_on && oxygen > 0
}

/// Oxygen a tick takes: what refills the air supply to the top, at least 1 (`ItemScubaTank`).
pub fn scuba_use(air: i32, stored: i32) -> i32 {
    (MAX_AIR - air).max(1).min(stored.max(0))
}

/// The jetpack's modes (`IJetpackItem.JetpackMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JetMode {
    Normal,
    Hover,
    Vector,
    Disabled,
}

impl JetMode {
    pub fn from_byte(b: u8) -> JetMode {
        match b {
            1 => JetMode::Hover,
            2 => JetMode::Vector,
            3 => JetMode::Disabled,
            _ => JetMode::Normal,
        }
    }

    pub fn byte(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> JetMode {
        match self {
            JetMode::Normal => JetMode::Hover,
            JetMode::Hover => JetMode::Vector,
            JetMode::Vector => JetMode::Disabled,
            JetMode::Disabled => JetMode::Normal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            JetMode::Normal => "Regular",
            JetMode::Hover => "Hover",
            JetMode::Vector => "Vector",
            JetMode::Disabled => "Disabled",
        }
    }
}

/// Upward speed per tick the jetpack adds (`IJetpackItem.getThrustFactor`: 0.15 per mB, 1 mB a tick).
pub const JET_THRUST: f64 = 0.15;
pub const JET_FUEL_PER_TICK: i32 = 1;

/// Whether the jetpack thrusts this tick: hover also holds the player when not on the ground
/// (`IJetpackItem.getPlayerJetpackMode`), the other modes only while ascending.
pub fn jet_active(mode: JetMode, ascending: bool, descending: bool, on_ground: bool) -> bool {
    match mode {
        JetMode::Disabled => false,
        JetMode::Hover => (ascending && !descending) || !on_ground,
        _ => ascending,
    }
}

/// The new velocity of a thrusting jetpack (`IJetpackItem.handleJetpackMotion`). `up` is the
/// look-based up vector of the vector mode; sneaking turns vector into regular.
pub fn jet_motion(
    mode: JetMode,
    v: [f64; 3],
    up: [f64; 3],
    ascending: bool,
    descending: bool,
    on_ground: bool,
    sneaking: bool,
) -> [f64; 3] {
    let t = JET_THRUST;
    let mode = if mode == JetMode::Vector && sneaking { JetMode::Normal } else { mode };
    let coef = (-v[1]).exp().min(1.0);
    match mode {
        JetMode::Normal => [v[0] + 0.08 * v[0], v[1] + t * coef, v[2] + 0.08 * v[2]],
        JetMode::Vector => [v[0] + up[0] * t, v[1] + up[1] * t * coef, v[2] + up[2] * t],
        JetMode::Hover => {
            let mut y = v[1];
            if ascending == descending {
                if !on_ground || y > 0.0 {
                    y = if y.abs() < 0.03 { 0.0 } else { y * 0.5 };
                }
            } else if ascending {
                y = (y + t).min(2.0 * t);
            } else if !on_ground {
                y = (y - t).max(-2.0 * t);
            }
            [v[0], y, v[2]]
        }
        JetMode::Disabled => v,
    }
}

/// The flamethrower's modes (`ItemFlamethrower.FlamethrowerMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlameMode {
    /// Damages and ignites mobs.
    Combat,
    /// Smelts what it hits.
    Heat,
    /// Sets blocks on fire.
    Inferno,
}

impl FlameMode {
    pub fn from_byte(b: u8) -> FlameMode {
        match b {
            1 => FlameMode::Heat,
            2 => FlameMode::Inferno,
            _ => FlameMode::Combat,
        }
    }

    pub fn byte(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> FlameMode {
        match self {
            FlameMode::Combat => FlameMode::Heat,
            FlameMode::Heat => FlameMode::Inferno,
            FlameMode::Inferno => FlameMode::Combat,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FlameMode::Combat => "Combat",
            FlameMode::Heat => "Heat",
            FlameMode::Inferno => "Inferno",
        }
    }
}

/// `EntityFlame`: reach (speed 0.5 x lifespan 80 = 40 blocks, cut to what a hitscan covers),
/// damage and seconds of fire.
pub const FLAME_RANGE: f64 = 16.0;
pub const FLAME_DAMAGE: f32 = 10.0;
pub const FLAME_FIRE_SECONDS: i32 = 5;
pub const FLAME_FUEL_PER_TICK: i32 = 1;
/// Ticks between two scans of the flame for targets.
pub const FLAME_INTERVAL: i32 = 2;
/// Half-angle of the flame's cone in radians.
pub const FLAME_ANGLE: f64 = 0.25;

/// Whether `target` lies in the flame's cone from `from` along the unit vector `dir`: within
/// `range`, and no more than `max_angle` radians off the line of sight.
pub fn in_cone(from: [f64; 3], dir: [f64; 3], target: [f64; 3], range: f64, max_angle: f64) -> bool {
    let d = [target[0] - from[0], target[1] - from[1], target[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > range {
        return false;
    }
    if len < 1e-6 {
        return true;
    }
    (d[0] * dir[0] + d[1] * dir[1] + d[2] * dir[2]) / len >= max_angle.cos()
}

/// The unit view vector of a player's `yaw` and `pitch` in degrees.
pub fn look(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

/// The look-based up vector of the jetpack's vector mode.
pub fn jet_up(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (-f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [y.sin() * p.sin(), p.cos(), y.cos() * p.sin()]
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

    #[test]
    fn runners_absorb_a_fall_with_energy() {
        // 10 blocks: 7 damage, 350 energy.
        let f = absorb_fall(RunnerMode::Normal, 64_000, 10.0, 1.0).unwrap();
        assert!(f.cancel);
        assert_eq!(f.cost, 350);
        // Nothing under the safe distance, nothing when off or empty.
        assert_eq!(absorb_fall(RunnerMode::Normal, 64_000, 3.0, 1.0), None);
        assert_eq!(absorb_fall(RunnerMode::Disabled, 64_000, 10.0, 1.0), None);
        assert_eq!(absorb_fall(RunnerMode::Safety, 0, 10.0, 1.0), None);
        // 100 energy pays for 2 of 7 damage: 5 are left, the fall goes on as 8 blocks.
        let f = absorb_fall(RunnerMode::Safety, 100, 10.0, 1.0).unwrap();
        assert!(!f.cancel);
        assert_eq!(f.cost, 100);
        assert!((f.distance - 8.0).abs() < 1e-9);
        // The multiplier counts: 0.5 halves the damage (ceil(3.5) = 4).
        assert_eq!(absorb_fall(RunnerMode::Normal, 64_000, 10.0, 0.5).unwrap().cost, 200);
    }

    #[test]
    fn runners_step_only_in_normal_mode() {
        assert_eq!(step_height(RunnerMode::Normal, 1, false), BASE_STEP_HEIGHT + RUNNERS_STEP_BOOST);
        assert_eq!(step_height(RunnerMode::Normal, 1, true), BASE_STEP_HEIGHT);
        assert_eq!(step_height(RunnerMode::Safety, 1, false), BASE_STEP_HEIGHT);
        assert_eq!(step_height(RunnerMode::Normal, 0, false), BASE_STEP_HEIGHT);
        assert_eq!(RunnerMode::Disabled.next(), RunnerMode::Normal);
        assert_eq!(RunnerMode::from_byte(RunnerMode::Safety.byte()), RunnerMode::Safety);
    }

    #[test]
    fn bow_shots_cost_energy() {
        assert_eq!(bow_cost(false), 120);
        assert_eq!(bow_cost(true), 1_200);
        assert!(bow_can_shoot(120, false));
        assert!(!bow_can_shoot(119, false));
        assert!(!bow_can_shoot(1_199, true));
    }

    #[test]
    fn scuba_takes_oxygen_to_refill() {
        assert!(scuba_active(true, 1, true));
        assert!(!scuba_active(false, 1, true));
        assert!(!scuba_active(true, 0, true));
        assert!(!scuba_active(true, 100, false));
        // Full air: the 1 mB a tick; low air: what refills it, or what is left.
        assert_eq!(scuba_use(MAX_AIR, 500), 1);
        assert_eq!(scuba_use(0, 500), 300);
        assert_eq!(scuba_use(0, 20), 20);
    }

    #[test]
    fn jetpack_modes_move_the_player() {
        let at_rest = [0.0, 0.0, 0.0];
        let up = jet_up(0.0, 0.0);
        let n = jet_motion(JetMode::Normal, at_rest, up, true, false, false, false);
        assert!((n[1] - JET_THRUST).abs() < 1e-9);
        // Falling: the full thrust is added; rising fast: it fades.
        let fall = jet_motion(JetMode::Normal, [0.0, -1.0, 0.0], up, true, false, false, false);
        assert!((fall[1] - (-1.0 + JET_THRUST)).abs() < 1e-9);
        let rise = jet_motion(JetMode::Normal, [0.0, 2.0, 0.0], up, true, false, false, false);
        assert!(rise[1] - 2.0 < JET_THRUST * 0.2);
        // Hover caps the climb at twice the thrust.
        let h = jet_motion(JetMode::Hover, [0.0, 1.0, 0.0], up, true, false, false, false);
        assert!((h[1] - 2.0 * JET_THRUST).abs() < 1e-9);
        let d = jet_motion(JetMode::Hover, [0.0, -1.0, 0.0], up, false, true, false, false);
        assert!((d[1] + 2.0 * JET_THRUST).abs() < 1e-9);
        // Hover holds: no keys, in the air: a slow speed dies away.
        assert_eq!(jet_motion(JetMode::Hover, [0.0, 0.01, 0.0], up, false, false, false, false)[1], 0.0);
        // Vector while sneaking is regular.
        assert_eq!(
            jet_motion(JetMode::Vector, at_rest, [1.0, 0.0, 0.0], true, false, false, true),
            jet_motion(JetMode::Normal, at_rest, up, true, false, false, false)
        );
        // Vector pushes along the look-based up vector.
        let v = jet_motion(JetMode::Vector, at_rest, [1.0, 0.0, 0.0], true, false, false, false);
        assert!((v[0] - JET_THRUST).abs() < 1e-9);
    }

    #[test]
    fn jetpack_active_rules() {
        assert!(!jet_active(JetMode::Disabled, true, false, false));
        assert!(jet_active(JetMode::Normal, true, false, true));
        assert!(!jet_active(JetMode::Normal, false, false, false));
        assert!(jet_active(JetMode::Hover, false, false, false));
        assert!(!jet_active(JetMode::Hover, false, false, true));
        assert_eq!(JetMode::Disabled.next(), JetMode::Normal);
        assert_eq!(JetMode::from_byte(JetMode::Vector.byte()), JetMode::Vector);
    }

    #[test]
    fn flame_cone_and_modes() {
        let dir = look(0.0, 0.0);
        assert!((dir[2] - 1.0).abs() < 1e-9);
        let cone = |t| in_cone([0.0; 3], dir, t, FLAME_RANGE, FLAME_ANGLE);
        assert!(cone([0.0, 0.0, 5.0]));
        assert!(!cone([0.0, 0.0, 17.0]));
        assert!(!cone([5.0, 0.0, 1.0]));
        assert!(!cone([0.0, 0.0, -3.0]));
        assert_eq!(FlameMode::Inferno.next(), FlameMode::Combat);
        assert_eq!(FlameMode::from_byte(FlameMode::Heat.byte()), FlameMode::Heat);
    }
}
