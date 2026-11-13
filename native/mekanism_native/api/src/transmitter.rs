//! Mechanical Pipes, Pressurized Tubes and Thermodynamic Conductors (M06): the
//! parts that need no Horizon types, so their tests run in `logic/energy`.
//! Hooks are in `transmitters.rs`.
//!
//! Values from Mekanism 26.3: `PipeTier`, `TubeTier` (capacity, transfer rate
//! per tick), `ConductorTier` (inverse conduction 5, heat capacity 1, inverse
//! insulation 10 / 400 / 8,000 / 100,000), `ITileHeatHandler` (`simulateAdjacent`,
//! `simulateEnvironment`) and `HeatAPI`.
//!
//! Horizon has no transmitter networks, so each pipe, tube and conductor is a
//! block entity of its own and moves what it holds to its neighbours:
//! - Pipes keep fluids and tubes chemicals (the tank family of `tank.rs`) in
//!   one Horizon block-entity tank of the tier's capacity, and push up to the
//!   tier's rate per tick into neighbours whose tank is emptier, which levels a
//!   line out. Mekanism's network shares one buffer of all its pipes and sends
//!   one rate for the whole network; here the rate is per block.
//! - Conductors keep heat in their own NBT and trade it with neighbouring
//!   conductors and Heat Generators the way Mekanism's heat network does, one
//!   block at a time instead of a network pass.
//!
//! Not ported: pull/push/off side modes and the Configurator, the network and
//! its redstone mode, upgrading a placed transmitter, fluid leaks.

use crate::generator;
use crate::tank::Family;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pipe,
    Tube,
    Conductor,
}

#[derive(Clone, Copy, Debug)]
pub struct Tier {
    pub block: &'static str,
    pub kind: Kind,
    /// mB a pipe or tube holds (0 for a conductor).
    pub capacity: i32,
    /// mB a pipe or tube moves per tick (0 for a conductor).
    pub rate: i32,
    /// A conductor's `getInverseConductionInsulation`.
    pub insulation: f64,
}

const B: i32 = 1_000;
const fn buffer(block: &'static str, kind: Kind, capacity: i32, rate: i32) -> Tier {
    Tier { block, kind, capacity, rate, insulation: 0.0 }
}
const fn conductor(block: &'static str, insulation: f64) -> Tier {
    Tier { block, kind: Kind::Conductor, capacity: 0, rate: 0, insulation }
}

pub const TRANSMITTERS: [Tier; 12] = [
    buffer("mekanism:basic_mechanical_pipe", Kind::Pipe, 2 * B, B / 4),
    buffer("mekanism:advanced_mechanical_pipe", Kind::Pipe, 8 * B, B),
    buffer("mekanism:elite_mechanical_pipe", Kind::Pipe, 32 * B, 8 * B),
    buffer("mekanism:ultimate_mechanical_pipe", Kind::Pipe, 128 * B, 32 * B),
    buffer("mekanism:basic_pressurized_tube", Kind::Tube, 4 * B, 750),
    buffer("mekanism:advanced_pressurized_tube", Kind::Tube, 16 * B, 2 * B),
    buffer("mekanism:elite_pressurized_tube", Kind::Tube, 256 * B, 64 * B),
    buffer("mekanism:ultimate_pressurized_tube", Kind::Tube, 1_024 * B, 256 * B),
    conductor("mekanism:basic_thermodynamic_conductor", 10.0),
    conductor("mekanism:advanced_thermodynamic_conductor", 400.0),
    conductor("mekanism:elite_thermodynamic_conductor", 8_000.0),
    conductor("mekanism:ultimate_thermodynamic_conductor", 100_000.0),
];

pub fn tier_of(block: &str) -> Option<&'static Tier> {
    TRANSMITTERS.iter().find(|t| t.block == block)
}

impl Tier {
    /// What the buffer holds: pipes fluids, tubes chemicals.
    pub fn family(&self) -> Option<Family> {
        match self.kind {
            Kind::Pipe => Some(Family::Fluid),
            Kind::Tube => Some(Family::Chemical),
            Kind::Conductor => None,
        }
    }

    /// Whether `other` (a block id) is a transmitter this one joins up with:
    /// the same kind of any tier.
    pub fn joins(&self, other: &str) -> bool {
        tier_of(other).is_some_and(|o| o.kind == self.kind)
    }
}

/// A neighbour's tank, as far as it takes the pipe's fluid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbour {
    pub dir: usize,
    pub amount: i32,
    pub capacity: i32,
}

/// The fluid to offer each neighbour this tick. Only neighbours with room and a
/// lower fill ratio than the buffer get any, so a line levels out and nothing
/// flows back into a fuller tank; the tier's rate is split between them.
pub fn push_plan(amount: i32, capacity: i32, rate: i32, neighbours: &[Neighbour]) -> Vec<(usize, i32)> {
    if amount <= 0 || capacity <= 0 {
        return Vec::new();
    }
    let lower: Vec<&Neighbour> = neighbours
        .iter()
        .filter(|n| {
            n.capacity > n.amount && (n.amount as i64) * (capacity as i64) < (amount as i64) * (n.capacity as i64)
        })
        .collect();
    if lower.is_empty() {
        return Vec::new();
    }
    let budget = rate.min(amount).max(0);
    let k = lower.len() as i32;
    lower
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let share = budget / k + i32::from((i as i32) < budget % k);
            (n.dir, share.min(n.capacity - n.amount))
        })
        .filter(|&(_, share)| share > 0)
        .collect()
}

/// One heat store: a conductor, or a Heat Generator seen from its neighbour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub heat: f64,
    pub capacity: f64,
    pub inverse_conduction: f64,
    pub inverse_insulation: f64,
}

const EPSILON: f64 = 1e-6;

impl Cell {
    /// A conductor (`BASIC_HEAT_CAPACITY` 1, inverse conduction 5); a new one
    /// is at the ambient temperature.
    pub fn conductor(t: &Tier, heat: Option<f64>, ambient: f64) -> Cell {
        Cell {
            heat: heat.unwrap_or(ambient),
            capacity: 1.0,
            inverse_conduction: 5.0,
            inverse_insulation: t.insulation,
        }
    }

    /// A Heat Generator's capacitor (`generator.rs` values).
    pub fn generator(heat: f64) -> Cell {
        Cell {
            heat,
            capacity: generator::HEAT_CAPACITY,
            inverse_conduction: generator::INVERSE_CONDUCTION,
            inverse_insulation: generator::INVERSE_INSULATION,
        }
    }

    pub fn temperature(&self) -> f64 {
        self.heat / self.capacity
    }

    fn handle_heat(&mut self, transfer: f64) {
        if transfer.abs() > EPSILON {
            self.heat = (self.heat + transfer).max(0.0);
        }
    }

    /// `simulateEnvironment`: loss to the air through each of the six sides.
    /// Returns the Kelvin lost (only while hotter than the air).
    pub fn environment(&mut self, ambient: f64) -> f64 {
        let mut lost = 0.0;
        for _ in 0..6 {
            let inverse = generator::AIR_INVERSE_COEFFICIENT + self.inverse_insulation + self.inverse_conduction;
            let t = (self.temperature() - ambient) / inverse;
            self.handle_heat(-t * self.capacity);
            if t > 0.0 {
                lost += t;
            }
        }
        lost
    }

    /// One side of `simulateAdjacent`: heat flows from `self` into `sink` when
    /// `self` is warmer, towards the temperature the calorimetry gives.
    /// Returns the Kelvin moved.
    pub fn conduct_into(&mut self, sink: &mut Cell) -> f64 {
        let (temp, sink_temp) = (self.temperature(), sink.temperature());
        if temp <= sink_temp {
            return 0.0;
        }
        let final_temp = (temp * self.capacity + sink_temp * sink.capacity) / (self.capacity + sink.capacity);
        let moved = (temp - final_temp) / (sink.inverse_conduction + self.inverse_conduction);
        self.handle_heat(-moved * self.capacity);
        sink.handle_heat(moved * self.capacity);
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(dir: usize, amount: i32, capacity: i32) -> Neighbour {
        Neighbour { dir, amount, capacity }
    }

    #[test]
    fn tier_values_match_mekanism() {
        let pipe = |b: &str| tier_of(b).map(|t| (t.capacity, t.rate)).unwrap();
        assert_eq!(pipe("mekanism:basic_mechanical_pipe"), (2_000, 250));
        assert_eq!(pipe("mekanism:advanced_mechanical_pipe"), (8_000, 1_000));
        assert_eq!(pipe("mekanism:elite_mechanical_pipe"), (32_000, 8_000));
        assert_eq!(pipe("mekanism:ultimate_mechanical_pipe"), (128_000, 32_000));
        assert_eq!(pipe("mekanism:basic_pressurized_tube"), (4_000, 750));
        assert_eq!(pipe("mekanism:advanced_pressurized_tube"), (16_000, 2_000));
        assert_eq!(pipe("mekanism:elite_pressurized_tube"), (256_000, 64_000));
        assert_eq!(pipe("mekanism:ultimate_pressurized_tube"), (1_024_000, 256_000));
        let insulation: Vec<f64> = TRANSMITTERS.iter().filter(|t| t.kind == Kind::Conductor).map(|t| t.insulation).collect();
        assert_eq!(insulation, [10.0, 400.0, 8_000.0, 100_000.0]);
    }

    #[test]
    fn families_and_joining() {
        let pipe = tier_of("mekanism:basic_mechanical_pipe").unwrap();
        let tube = tier_of("mekanism:elite_pressurized_tube").unwrap();
        assert_eq!(pipe.family(), Some(Family::Fluid));
        assert_eq!(tube.family(), Some(Family::Chemical));
        assert_eq!(tier_of("mekanism:ultimate_thermodynamic_conductor").unwrap().family(), None);
        assert!(pipe.joins("mekanism:ultimate_mechanical_pipe"));
        assert!(!pipe.joins("mekanism:basic_pressurized_tube"));
        assert!(!pipe.joins("mekanism:basic_fluid_tank"));
    }

    #[test]
    fn push_goes_only_to_emptier_neighbours_and_respects_rate() {
        // Pipe 1,000 / 2,000; the emptier tank and the empty one get the 250 mB/t, the fuller one nothing.
        let plan = push_plan(1_000, 2_000, 250, &[n(0, 100, 10_000), n(1, 9_000, 10_000), n(2, 0, 10_000)]);
        assert_eq!(plan, [(0, 125), (2, 125)]);
        assert_eq!(plan.iter().map(|p| p.1).sum::<i32>(), 250);
        // The same ratio is level: nothing moves either way.
        assert!(push_plan(1_000, 2_000, 250, &[n(3, 5_000, 10_000)]).is_empty());
        // A full neighbour or an empty pipe moves nothing.
        assert!(push_plan(1_000, 2_000, 250, &[n(3, 10_000, 10_000)]).is_empty());
        assert!(push_plan(0, 2_000, 250, &[n(3, 0, 10_000)]).is_empty());
    }

    #[test]
    fn push_shares_the_remainder_and_is_capped_by_room() {
        let plan = push_plan(2_000, 2_000, 250, &[n(0, 0, 100), n(1, 0, 100), n(4, 0, 100)]);
        // 250 split three ways, 84 + 83 + 83, each at most the room (100).
        assert_eq!(plan, [(0, 84), (1, 83), (4, 83)]);
        let capped = push_plan(2_000, 2_000, 250, &[n(0, 90, 100)]);
        assert_eq!(capped, [(0, 10)]);
        // Never more than the pipe holds.
        assert_eq!(push_plan(100, 2_000, 250, &[n(0, 0, 10_000)]), [(0, 100)]);
    }

    #[test]
    fn a_line_levels_out() {
        // Two pipes in a row: all fluid in the first, the plan moves it on until both are level.
        let (mut a, mut b) = (2_000, 0);
        for _ in 0..100 {
            if let Some(&(_, x)) = push_plan(a, 2_000, 250, &[n(0, b, 2_000)]).first() {
                a -= x;
                b += x;
            }
        }
        assert_eq!(a + b, 2_000);
        assert!((a - b).abs() <= 1);
    }

    #[test]
    fn conductor_starts_at_ambient_and_stays_there() {
        let t = tier_of("mekanism:basic_thermodynamic_conductor").unwrap();
        let mut c = Cell::conductor(t, None, 300.0);
        assert_eq!(c.temperature(), 300.0);
        assert_eq!(c.environment(300.0), 0.0);
        assert_eq!(c.temperature(), 300.0);
    }

    #[test]
    fn environment_loss_follows_insulation() {
        let hot = |name: &str| {
            let t = tier_of(name).unwrap();
            let mut c = Cell::conductor(t, Some(400.0), 300.0);
            let lost = c.environment(300.0);
            (c.temperature(), lost)
        };
        let (basic_t, basic_lost) = hot("mekanism:basic_thermodynamic_conductor");
        let (ultimate_t, ultimate_lost) = hot("mekanism:ultimate_thermodynamic_conductor");
        // Basic: six sides of (100 K) / (10000 + 10 + 5) each, from the shrinking temperature.
        assert!(basic_t < 400.0 && basic_t > 399.0);
        assert!(basic_lost > 0.059 && basic_lost < 0.06);
        // Ultimate (100,000 insulation) loses a tenth as much.
        assert!(ultimate_t > basic_t);
        assert!(ultimate_lost < basic_lost / 9.0);
    }

    #[test]
    fn heat_flows_hot_to_cold_by_calorimetry_and_conserves_heat() {
        let t = tier_of("mekanism:basic_thermodynamic_conductor").unwrap();
        let mut hot = Cell::conductor(t, Some(600.0), 300.0);
        let mut cold = Cell::conductor(t, Some(300.0), 300.0);
        let total = hot.heat + cold.heat;
        // Equal capacities: final 450 K, moved (600 - 450) / (5 + 5) = 15 K.
        assert_eq!(hot.conduct_into(&mut cold), 15.0);
        assert_eq!((hot.heat, cold.heat), (585.0, 315.0));
        assert_eq!(hot.heat + cold.heat, total);
        // The colder one never pushes back.
        assert_eq!(cold.conduct_into(&mut hot), 0.0);
    }

    #[test]
    fn conductors_settle_on_one_temperature() {
        let t = tier_of("mekanism:basic_thermodynamic_conductor").unwrap();
        let mut line = [Cell::conductor(t, Some(900.0), 300.0), Cell::conductor(t, Some(300.0), 300.0),
                        Cell::conductor(t, Some(300.0), 300.0)];
        for _ in 0..200 {
            for i in 0..2 {
                let (a, b) = line.split_at_mut(i + 1);
                a[i].conduct_into(&mut b[0]);
                let (a, b) = line.split_at_mut(i + 1);
                b[0].conduct_into(&mut a[i]);
            }
        }
        let temps: Vec<f64> = line.iter().map(Cell::temperature).collect();
        assert!(temps.iter().all(|t| (t - 500.0).abs() < 1.0), "{temps:?}");
    }

    #[test]
    fn a_generator_warms_a_conductor() {
        let t = tier_of("mekanism:basic_thermodynamic_conductor").unwrap();
        let mut generator = Cell::generator(10.0 * 500.0);
        let mut conductor = Cell::conductor(t, None, 300.0);
        let moved = generator.conduct_into(&mut conductor);
        assert!(moved > 0.0);
        assert!(conductor.temperature() > 300.0);
        assert!(generator.temperature() < 500.0);
    }
}
