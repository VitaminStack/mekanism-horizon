//! Electric machines: the table every processing machine is one entry of
//! (Enrichment Chamber, Crusher, Metallurgic Infuser, Osmium Compressor,
//! Combiner, Precision Sawmill, Purification Chamber, Chemical Injection
//! Chamber), and the operation they share. The parts that
//! need no Horizon types, so their tests run in `logic/energy`; recipes are in
//! `recipe.rs`, the tick and screen in `process.rs`.
//!
//! A new machine of this kind is one more entry in `MACHINES` plus its content
//! files (block, menu, recipes): the hooks in `lib.rs` find it through `find`.
//!
//! Values from Mekanism 26.3: `TileEntityElectricMachine`,
//! `TileEntityAdvancedElectricMachine` (`MAX_GAS` 210), `TileEntityCombiner`,
//! `TileEntityPrecisionSawmill` (`BASE_TICKS_REQUIRED` 10 s each, slot
//! positions), `UsageConfig` (enrichmentChamber 5, crusher 8,
//! metallurgicInfuser 5, osmiumCompressor 15, combiner 5, precisionSawmill 5,
//! purificationChamber 10, chemicalInjectionChamber 15 per tick) and `AttributeEnergy.STORAGE_MULTIPLIER` (storage 4 x usage).
//!
//! Not ported: the energy slot (charging from items), upgrades, side config
//! and redstone control.

use std::sync::atomic::{AtomicU64, Ordering};

use super::infuser;

/// A chemical a buffer takes: Mekanism id, English name, bar colour.
pub type Chemical = (&'static str, &'static str, u32);

/// A chemical buffer filled from an item slot (Mekanism's
/// `ChemicalInventorySlot.fillOrConvert` by `chemical_conversion` recipes):
/// the Metallurgic Infuser's infuse buffer, the Osmium Compressor's tank.
pub struct Buffer {
    /// The slot whose items are converted into the buffer.
    pub slot: usize,
    /// What it takes; the index + 1 is what the NBT and menu data store.
    pub types: &'static [Chemical],
    pub max: i32,
    /// Screen rect of its gauge, and of its dump button if it has one.
    pub gauge: [f32; 4],
    pub dump: Option<[f32; 4]>,
}

impl Buffer {
    pub fn type_index(&self, id: &str) -> Option<usize> {
        self.types.iter().position(|(t, ..)| *t == id)
    }
}

/// A buffer's contents: one type at a time, as Mekanism's chemical tank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tank {
    /// `Buffer::types` index; meaningless while `amount` is 0.
    pub kind: usize,
    pub amount: i32,
}

impl Tank {
    /// Whether `amount` of `kind` fits whole under `max` (Mekanism converts
    /// one item only when all its output fits).
    pub fn fits(&self, kind: usize, amount: i32, max: i32) -> bool {
        (self.amount == 0 || self.kind == kind) && amount > 0 && self.amount + amount <= max
    }

    pub fn add(&mut self, kind: usize, amount: i32, max: i32) {
        debug_assert!(self.fits(kind, amount, max));
        if self.amount == 0 {
            self.kind = kind;
        }
        self.amount += amount;
    }

    pub fn has(&self, kind: usize, amount: i32) -> bool {
        self.amount > 0 && self.kind == kind && self.amount >= amount
    }

    /// Whether `kind` may go in: the buffer is empty or holds it already.
    pub fn takes(&self, kind: usize) -> bool {
        self.amount == 0 || self.kind == kind
    }

    pub fn take(&mut self, amount: i32) {
        self.amount = (self.amount - amount).max(0);
    }
}

pub struct Machine {
    pub block: &'static str,
    /// Registered by `data/mekanism/horizon/menu/<id>.json`.
    pub menu: &'static str,
    /// Mekanism's recipe type, shipped by the content mod (shapes: `recipe.rs`).
    pub recipe: &'static str,
    pub name: &'static str,
    pub usage: i32,
    /// Mekanism's loop while running (`tile.machine.<id>` in `sounds.json`)
    /// and its clip length in ticks, after which it is played again.
    pub sound: &'static str,
    pub sound_ticks: i32,
    /// Slots, by their index in the block entity's saved items.
    pub input: usize,
    pub output: usize,
    /// The Combiner's second item input (`extra_input`).
    pub extra: Option<usize>,
    /// The Precision Sawmill's output for its chance-based `secondary_output`.
    pub secondary: Option<usize>,
    pub buffer: Option<&'static Buffer>,
    /// Slot positions by slot index, and screen rects (x, y, w, h) from the
    /// screen's top left, as in Mekanism's GUI for the machine (176 x 166).
    pub slot_pos: &'static [[f32; 2]],
    pub energy_gauge: [f32; 4],
    pub progress_bar: [f32; 4],
}

impl Machine {
    pub const fn capacity(&self) -> i32 {
        4 * self.usage
    }

    pub const fn slots(&self) -> usize {
        self.slot_pos.len()
    }

    /// Every output slot: the output, then the secondary one.
    pub fn outputs(&self) -> Vec<usize> {
        std::iter::once(self.output).chain(self.secondary).collect()
    }
}

/// Mekanism's `GuiElectricMachine`: input above the bar's start, output right.
const ELECTRIC_SLOT_POS: [[f32; 2]; 2] = [[64.0, 17.0], [116.0, 35.0]];
const ELECTRIC_ENERGY_GAUGE: [f32; 4] = [164.0, 16.0, 6.0, 54.0];
const ELECTRIC_PROGRESS_BAR: [f32; 4] = [86.0, 38.0, 28.0, 8.0];

/// One item in, one item out.
const fn electric(
    block: &'static str,
    recipe: &'static str,
    name: &'static str,
    usage: i32,
    sound: &'static str,
    sound_ticks: i32,
) -> Machine {
    Machine {
        block,
        menu: block,
        recipe,
        name,
        usage,
        sound,
        sound_ticks,
        input: 0,
        output: 1,
        extra: None,
        secondary: None,
        buffer: None,
        slot_pos: &ELECTRIC_SLOT_POS,
        energy_gauge: ELECTRIC_ENERGY_GAUGE,
        progress_bar: ELECTRIC_PROGRESS_BAR,
    }
}

/// `TileEntityAdvancedElectricMachine` and `TileEntityCombiner`: input 0 above,
/// the second input 1 below it (64, 53), output 2.
const TWO_INPUT_SLOT_POS: [[f32; 2]; 3] = [[64.0, 17.0], [64.0, 53.0], [116.0, 35.0]];

/// The Osmium Compressor's tank (`MAX_GAS`), filled with osmium from ingots
/// and blocks in slot 1; its `GuiChemicalBar` sits between the two slots.
pub const OSMIUM_TANK: Buffer = Buffer {
    slot: 1,
    types: &[("mekanism:osmium", "Osmium", 0xFF52BDCA)],
    max: 210,
    gauge: [68.0, 36.0, 6.0, 12.0],
    dump: None,
};

/// The Purification Chamber's tank: `MAX_GAS` of oxygen, the one chemical its
/// recipes use, from flint in slot 1 (`flint_to_oxygen`, 10 mB each). The gauge
/// is `GuiAdvancedElectricMachine`'s, as the Osmium Compressor's.
pub const OXYGEN_TANK: Buffer = Buffer {
    slot: 1,
    types: &[("mekanism:oxygen", "Oxygen", 0xFF6CE2FF)],
    max: 210,
    gauge: [68.0, 36.0, 6.0, 12.0],
    dump: None,
};

/// The Chemical Injection Chamber's tank: what its recipes use (oxygen to
/// oxidize copper, hydrogen chloride, water vapor), one at a time. Only oxygen
/// (flint) and hydrogen chloride (salt) come from items; water vapor needs a
/// machine that makes it (later work), so its recipes wait for one.
pub const INJECTION_TANK: Buffer = Buffer {
    slot: 1,
    types: &[
        ("mekanism:oxygen", "Oxygen", 0xFF6CE2FF),
        ("mekanism:hydrogen_chloride", "Hydrogen Chloride", 0xFFA8F1E9),
        ("mekanism:water_vapor", "Water Vapor", 0xFFC8C8C8),
    ],
    max: 210,
    gauge: [68.0, 36.0, 6.0, 12.0],
    dump: None,
};

/// `TileEntityPrecisionSawmill`: input 0, output 1, secondary output 2.
const SAWMILL_SLOT_POS: [[f32; 2]; 3] = [[56.0, 17.0], [116.0, 35.0], [132.0, 35.0]];

pub const MACHINES: [Machine; 8] = [
    electric(
        "mekanism:enrichment_chamber",
        "mekanism:enriching",
        "Enrichment Chamber",
        5,
        "mekanism:tile.machine.enrichment_chamber",
        82,
    ),
    electric("mekanism:crusher", "mekanism:crushing", "Crusher", 8, "mekanism:tile.machine.crusher", 37),
    Machine {
        block: infuser::BLOCK,
        menu: infuser::MENU,
        recipe: infuser::INFUSING,
        name: "Metallurgic Infuser",
        usage: 5,
        sound: infuser::SOUND,
        sound_ticks: infuser::SOUND_TICKS,
        input: infuser::SLOT_INPUT,
        output: infuser::SLOT_OUTPUT,
        extra: None,
        secondary: None,
        buffer: Some(&infuser::BUFFER),
        slot_pos: &infuser::SLOT_POS,
        energy_gauge: infuser::ENERGY_GAUGE,
        progress_bar: infuser::PROGRESS_BAR,
    },
    Machine {
        input: 0,
        output: 2,
        buffer: Some(&OSMIUM_TANK),
        slot_pos: &TWO_INPUT_SLOT_POS,
        ..electric(
            "mekanism:osmium_compressor",
            "mekanism:compressing",
            "Osmium Compressor",
            15,
            "mekanism:tile.machine.compressor",
            COMPRESSOR_SOUND_TICKS,
        )
    },
    Machine {
        input: 0,
        output: 2,
        extra: Some(1),
        slot_pos: &TWO_INPUT_SLOT_POS,
        ..electric(
            "mekanism:combiner",
            "mekanism:combining",
            "Combiner",
            5,
            "mekanism:tile.machine.combiner",
            COMBINER_SOUND_TICKS,
        )
    },
    Machine {
        secondary: Some(2),
        slot_pos: &SAWMILL_SLOT_POS,
        energy_gauge: [164.0, 15.0, 6.0, 54.0],
        progress_bar: [78.0, 38.0, 28.0, 8.0],
        ..electric(
            "mekanism:precision_sawmill",
            "mekanism:sawing",
            "Precision Sawmill",
            5,
            "mekanism:tile.machine.precision_sawmill",
            SAWMILL_SOUND_TICKS,
        )
    },
    Machine {
        input: 0,
        output: 2,
        buffer: Some(&OXYGEN_TANK),
        slot_pos: &TWO_INPUT_SLOT_POS,
        ..electric(
            "mekanism:purification_chamber",
            "mekanism:purifying",
            "Purification Chamber",
            10,
            "mekanism:tile.machine.purification_chamber",
            PURIFICATION_SOUND_TICKS,
        )
    },
    Machine {
        input: 0,
        output: 2,
        buffer: Some(&INJECTION_TANK),
        slot_pos: &TWO_INPUT_SLOT_POS,
        ..electric(
            "mekanism:chemical_injection_chamber",
            "mekanism:injecting",
            "Chemical Injection Chamber",
            15,
            "mekanism:tile.machine.chemical_injection_chamber",
            INJECTION_SOUND_TICKS,
        )
    },
];

/// Clip lengths of `tile/compressor.ogg`, `tile/combiner.ogg` and
/// `tile/precision_sawmill.ogg`, `tile/purification_chamber.ogg` and
/// `tile/chemical_injection_chamber.ogg` in ticks, rounded up.
const COMPRESSOR_SOUND_TICKS: i32 = 57;
const COMBINER_SOUND_TICKS: i32 = 57;
const SAWMILL_SOUND_TICKS: i32 = 34;
const PURIFICATION_SOUND_TICKS: i32 = 72;
const INJECTION_SOUND_TICKS: i32 = 40;

pub fn find(block: &str) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| m.block == block)
}

pub const TICKS: i32 = 200;

/// One tick of an operation that can run: spend `usage` and advance, or stall
/// without energy. Returns true when the operation completes this tick, and
/// then progress starts over.
pub fn step(usage: i32, energy: &mut i32, progress: &mut i32) -> bool {
    if *energy < usage {
        return false;
    }
    *energy -= usage;
    *progress += 1;
    if *progress >= TICKS {
        *progress = 0;
        return true;
    }
    false
}

/// A number in [0, 1), as Mekanism's `RandomSource.nextDouble` for the
/// sawmill's secondary output (splitmix64 over a shared counter).
pub fn random() -> f64 {
    static STATE: AtomicU64 = AtomicU64::new(0x853C_49E6_748F_EA9B);
    let mut z = STATE.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Menu data slots (each a 16-bit wire entry; every value fits one). The last
/// two are sent only by a machine with a chemical buffer.
pub const DATA_ENERGY: usize = 0;
pub const DATA_PROGRESS: usize = 1;
pub const DATA_FACING: usize = 2;
pub const DATA_ACTIVE: usize = 3;
pub const DATA_BUFFER: usize = 4;
/// `Buffer::types` index + 1; 0 while the buffer is empty.
pub const DATA_BUFFER_TYPE: usize = 5;

pub fn menu_data(energy: i32, progress: i32, facing: usize, active: bool, tank: Option<Tank>) -> Vec<i32> {
    let mut data = vec![energy, progress, facing as i32, active as i32];
    if let Some(tank) = tank {
        data.extend([tank.amount, if tank.amount > 0 { tank.kind as i32 + 1 } else { 0 }]);
    }
    data
}

pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    let [x, y, w, h] = rect;
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

/// Tooltip lines for the gauge, bar or button under `at`, from the menu data.
pub fn tooltip(machine: &Machine, data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(machine.energy_gauge, at) {
        return vec![
            format!("Energy: {} / {} FE", d(DATA_ENERGY), machine.capacity()),
            format!("Usage: {} FE/t", machine.usage),
        ];
    }
    if inside(machine.progress_bar, at) {
        return vec![format!("Progress: {}%", d(DATA_PROGRESS) * 100 / TICKS)];
    }
    let Some(b) = machine.buffer else { return Vec::new() };
    if inside(b.gauge, at) {
        let kind = d(DATA_BUFFER_TYPE);
        let name = kind.checked_sub(1).and_then(|k| b.types.get(k as usize)).map_or("Empty", |t| t.1);
        return vec![format!("{name}: {} / {} mB", d(DATA_BUFFER), b.max)];
    }
    if b.dump.is_some_and(|r| inside(r, at)) {
        return vec![infuser::DUMP_TOOLTIP.to_owned()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(block: &str) -> &'static Machine {
        find(block).unwrap()
    }

    #[test]
    fn values_match_mekanism() {
        let usage = |b: &str| {
            let m = machine(b);
            (m.usage, m.capacity())
        };
        assert_eq!(usage("mekanism:enrichment_chamber"), (5, 20));
        assert_eq!(usage("mekanism:crusher"), (8, 32));
        assert_eq!(usage("mekanism:metallurgic_infuser"), (5, 20));
        assert_eq!(usage("mekanism:osmium_compressor"), (15, 60));
        assert_eq!(usage("mekanism:combiner"), (5, 20));
        assert_eq!(usage("mekanism:precision_sawmill"), (5, 20));
        assert_eq!(usage("mekanism:purification_chamber"), (10, 40));
        assert_eq!(usage("mekanism:chemical_injection_chamber"), (15, 60));
        assert_eq!(machine("mekanism:purification_chamber").buffer.unwrap().max, 210);
        assert_eq!(machine("mekanism:chemical_injection_chamber").buffer.unwrap().max, 210);
        assert_eq!(machine("mekanism:osmium_compressor").buffer.unwrap().max, 210);
        assert!(find("mekanism:basic_energy_cube").is_none());
    }

    /// Slot indices are saved with the block entity: they must not move.
    #[test]
    fn slots_keep_their_saved_order() {
        let slots = |b: &str| {
            let m = machine(b);
            (m.input, m.output, m.extra, m.secondary, m.buffer.map(|b| b.slot), m.slots())
        };
        assert_eq!(slots("mekanism:crusher"), (0, 1, None, None, None, 2));
        assert_eq!(slots("mekanism:metallurgic_infuser"), (1, 2, None, None, Some(0), 3));
        assert_eq!(slots("mekanism:osmium_compressor"), (0, 2, None, None, Some(1), 3));
        assert_eq!(slots("mekanism:combiner"), (0, 2, Some(1), None, None, 3));
        assert_eq!(slots("mekanism:precision_sawmill"), (0, 1, None, Some(2), None, 3));
        assert_eq!(slots("mekanism:purification_chamber"), (0, 2, None, None, Some(1), 3));
        assert_eq!(slots("mekanism:chemical_injection_chamber"), (0, 2, None, None, Some(1), 3));
    }

    #[test]
    fn every_machine_is_well_formed() {
        for m in &MACHINES {
            assert!(m.usage > 0 && m.sound_ticks > 0, "{}", m.block);
            let mut used: Vec<usize> = [m.input, m.output].into_iter().chain(m.extra).chain(m.secondary).collect();
            used.extend(m.buffer.map(|b| b.slot));
            let n = used.len();
            used.sort();
            used.dedup();
            assert_eq!(used.len(), n, "{}: a slot used twice", m.block);
            assert!(used.iter().all(|&s| s < m.slots()), "{}", m.block);
            assert_eq!(MACHINES.iter().filter(|o| o.block == m.block).count(), 1, "{}", m.block);
        }
    }

    #[test]
    fn operation_takes_200_ticks_at_usage() {
        let (mut energy, mut progress) = (i32::MAX, 0);
        let start = energy;
        let done = (1..=TICKS).filter(|_| step(8, &mut energy, &mut progress)).count();
        assert_eq!(done, 1);
        assert_eq!(progress, 0);
        assert_eq!(start - energy, TICKS * 8);
    }

    #[test]
    fn no_energy_stalls_without_losing_progress() {
        let (mut energy, mut progress) = (4, 7);
        assert!(!step(5, &mut energy, &mut progress));
        assert_eq!((energy, progress), (4, 7));
    }

    #[test]
    fn tank_takes_one_type_up_to_its_max() {
        let mut t = Tank::default();
        assert!(t.fits(0, 1_000, 1_000));
        assert!(!t.fits(0, 1_001, 1_000));
        t.add(0, 990, 1_000);
        assert!(t.fits(0, 10, 1_000));
        assert!(!t.fits(0, 20, 1_000));
        assert!(!t.fits(1, 10, 1_000), "other type while not empty");
        assert!(!t.takes(1) && t.takes(0));
        t.take(990);
        assert!(t.fits(1, 10, 1_000), "empty tank takes any type");
        assert!(!t.fits(1, 0, 1_000));
    }

    #[test]
    fn tank_has_needs_type_and_amount() {
        let mut t = Tank::default();
        assert!(!t.has(0, 0), "empty tank has nothing");
        t.add(2, 20, 1_000);
        assert!(t.has(2, 20));
        assert!(!t.has(2, 21));
        assert!(!t.has(0, 10));
    }

    /// One osmium ingot (200 mB) fits the compressor's 210 mB tank, a second
    /// only once it ran low; a block (1800 mB) never.
    #[test]
    fn compressor_tank_takes_an_ingot_at_a_time() {
        let mut t = Tank::default();
        assert!(!t.fits(0, 1_800, OSMIUM_TANK.max));
        t.add(0, 200, OSMIUM_TANK.max);
        assert!(!t.fits(0, 200, OSMIUM_TANK.max));
        t.take(195);
        assert!(t.fits(0, 200, OSMIUM_TANK.max));
        assert_eq!(OSMIUM_TANK.type_index("mekanism:osmium"), Some(0));
        assert_eq!(OSMIUM_TANK.type_index("mekanism:carbon"), None);
    }

    /// Buffer types are saved as index + 1: their order must not move.
    #[test]
    fn buffer_types_keep_their_saved_order() {
        assert_eq!(OXYGEN_TANK.type_index("mekanism:oxygen"), Some(0));
        assert_eq!(INJECTION_TANK.type_index("mekanism:oxygen"), Some(0));
        assert_eq!(INJECTION_TANK.type_index("mekanism:hydrogen_chloride"), Some(1));
        assert_eq!(INJECTION_TANK.type_index("mekanism:water_vapor"), Some(2));
        assert_eq!(OXYGEN_TANK.type_index("mekanism:hydrogen_chloride"), None);
    }

    /// Flint (10 mB) fills the 210 mB tank 21 times; a tank holding another
    /// chemical takes none until it is empty.
    #[test]
    fn injection_tank_holds_one_chemical() {
        let mut t = Tank::default();
        for _ in 0..21 {
            assert!(t.fits(0, 10, INJECTION_TANK.max));
            t.add(0, 10, INJECTION_TANK.max);
        }
        assert!(!t.fits(0, 10, INJECTION_TANK.max));
        t.take(210);
        t.add(1, 2, INJECTION_TANK.max);
        assert!(!t.fits(0, 10, INJECTION_TANK.max));
    }

    #[test]
    fn random_stays_in_unit_range() {
        let rolls: Vec<f64> = (0..1_000).map(|_| random()).collect();
        assert!(rolls.iter().all(|r| (0.0..1.0).contains(r)));
        let low = rolls.iter().filter(|&&r| r < 0.5).count();
        assert!((350..650).contains(&low), "{low} of 1000 below 0.5");
    }

    #[test]
    fn menu_data_fits_16_bits() {
        let d = menu_data(32, TICKS - 1, 5, true, None);
        assert_eq!(d.len(), 4);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        let full = Tank { kind: 7, amount: infuser::BUFFER.max };
        let d = menu_data(20, TICKS - 1, 5, true, Some(full));
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!((d[DATA_BUFFER], d[DATA_BUFFER_TYPE]), (1_000, 8));
        let empty = Tank { kind: 3, amount: 0 };
        assert_eq!(menu_data(0, 0, 2, false, Some(empty))[DATA_BUFFER_TYPE], 0);
    }

    #[test]
    fn tooltips_name_what_is_hovered() {
        let m = machine("mekanism:crusher");
        let data = menu_data(16, 100, 2, true, None);
        assert_eq!(tooltip(m, &data, [165.0, 40.0]), vec!["Energy: 16 / 32 FE", "Usage: 8 FE/t"]);
        assert_eq!(tooltip(m, &data, [90.0, 40.0]), vec!["Progress: 50%"]);
        assert!(tooltip(m, &data, [10.0, 10.0]).is_empty());
    }

    #[test]
    fn infuser_tooltips_name_what_is_hovered() {
        let m = machine("mekanism:metallurgic_infuser");
        let data = menu_data(15, 100, 2, true, Some(Tank { kind: 0, amount: 260 }));
        assert_eq!(tooltip(m, &data, [8.0, 40.0]), vec!["Carbon: 260 / 1000 mB"]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0])[0], "Energy: 15 / 20 FE");
        assert_eq!(tooltip(m, &data, [80.0, 48.0]), vec!["Progress: 50%"]);
        assert_eq!(tooltip(m, &data, [20.0, 60.0]), vec!["Dump infuse buffer"]);
        assert!(tooltip(m, &data, [90.0, 10.0]).is_empty());
        assert_eq!(tooltip(m, &[], [8.0, 40.0]), vec!["Empty: 0 / 1000 mB"]);
    }

    #[test]
    fn injection_tooltip_names_the_chemical() {
        let m = machine("mekanism:chemical_injection_chamber");
        let data = menu_data(30, 0, 2, true, Some(Tank { kind: 1, amount: 8 }));
        assert_eq!(tooltip(m, &data, [70.0, 40.0]), vec!["Hydrogen Chloride: 8 / 210 mB"]);
    }

    #[test]
    fn compressor_tooltip_shows_osmium() {
        let m = machine("mekanism:osmium_compressor");
        let data = menu_data(60, 0, 2, false, Some(Tank { kind: 0, amount: 200 }));
        assert_eq!(tooltip(m, &data, [70.0, 40.0]), vec!["Osmium: 200 / 210 mB"]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0])[0], "Energy: 60 / 60 FE");
    }
}
