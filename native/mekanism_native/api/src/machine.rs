//! Electric machines: the table every processing machine is one entry of
//! (Enrichment Chamber, Crusher, Metallurgic Infuser, Osmium Compressor,
//! Combiner, Precision Sawmill, Purification Chamber, Chemical Injection
//! Chamber, Chemical Oxidizer, Chemical Infuser), and the operation they share. The parts that
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
//! purificationChamber 10, chemicalInjectionChamber 15, chemicalOxidizer 15,
//! chemicalInfuser 15 per tick), `TileEntityChemicalOxidizer` (`BASE_TICKS_REQUIRED`
//! 5 s, `MAX_GAS` 10 buckets), `TileEntityChemicalInfuser` (three tanks of
//! `MAX_GAS`, one operation per tick) and `AttributeEnergy.STORAGE_MULTIPLIER`
//! (storage 4 x usage).
//!
//! Not ported: the energy slot (charging from items), upgrades, side config
//! and redstone control.

use std::sync::atomic::{AtomicU64, Ordering};

use super::chemicals;
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

/// A Horizon fluid tank's contents (`fluids` in the block-entity NBT): a fluid
/// or chemical id (`chemicals::chemical`) and millibuckets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fluid {
    pub id: Option<String>,
    pub amount: i32,
}

impl Fluid {
    /// Whether `amount` of `id` fits whole under `max`.
    pub fn fits(&self, id: &str, amount: i32, max: i32) -> bool {
        amount > 0 && self.id.as_deref().is_none_or(|have| have == id) && self.amount + amount <= max
    }

    pub fn add(&mut self, id: &str, amount: i32) {
        self.id.get_or_insert_with(|| id.to_owned());
        self.amount += amount;
    }

    pub fn has(&self, id: &str, amount: i32) -> bool {
        self.id.as_deref() == Some(id) && self.amount >= amount
    }

    pub fn take(&mut self, amount: i32) {
        self.amount = (self.amount - amount).max(0);
        if self.amount == 0 {
            self.id = None;
        }
    }
}

/// A machine's chemical tanks as Horizon fluid tanks (`block_entity_fluids`):
/// the input tanks, then one output tank. Other blocks fill the inputs and
/// drain the output; the machine also ejects the output into its neighbours.
pub struct FluidTanks {
    /// Screen rects of the input tanks' gauges, in tank order.
    pub inputs: &'static [[f32; 4]],
    pub output: [f32; 4],
    /// Capacity of each tank in mB (`MAX_GAS`).
    pub max: i32,
}

impl FluidTanks {
    pub const fn count(&self) -> usize {
        self.inputs.len() + 1
    }

    /// The output tank's number: the input tanks come first.
    pub const fn output_index(&self) -> usize {
        self.inputs.len()
    }

    /// Every gauge: the inputs, then the output.
    pub fn gauges(&self) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.inputs.iter().copied().chain([self.output])
    }
}

/// One operation of a machine that mixes two chemicals (`chemical_infusing`):
/// takes `left` and `right` from the input tanks, in either order (Mekanism's
/// orderless recipe), and puts `out` into the output tank. Does nothing and
/// returns false unless both are there and the output fits whole. Each
/// chemical is a tank id and an amount.
pub fn infuse(tanks: &mut [Fluid], max: i32, left: (&str, i32), right: (&str, i32), out: (&str, i32)) -> bool {
    let [a, b, o] = tanks else { return false };
    if !o.fits(out.0, out.1, max) {
        return false;
    }
    let straight = a.has(left.0, left.1) && b.has(right.0, right.1);
    if !straight && !(a.has(right.0, right.1) && b.has(left.0, left.1)) {
        return false;
    }
    let (first, second) = if straight { (left.1, right.1) } else { (right.1, left.1) };
    a.take(first);
    b.take(second);
    o.add(out.0, out.1);
    true
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
    /// Ticks one operation takes (`BASE_TICKS_REQUIRED`); the Chemical
    /// Infuser, which mixes every tick, has 1.
    pub ticks: i32,
    /// Slots, by their index in the block entity's saved items. A machine
    /// without an item input or output (the infuser, the oxidizer's output)
    /// has none.
    pub input: Option<usize>,
    pub output: Option<usize>,
    /// The Combiner's second item input (`extra_input`).
    pub extra: Option<usize>,
    /// The Precision Sawmill's output for its chance-based `secondary_output`.
    pub secondary: Option<usize>,
    pub buffer: Option<&'static Buffer>,
    /// Chemical tanks that are Horizon fluid tanks (the Oxidizer's output, the
    /// Infuser's three).
    pub tanks: Option<&'static FluidTanks>,
    /// Slots after `slot_pos` that no screen shows (Horizon needs at least one).
    pub hidden: usize,
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
        self.slot_pos.len() + self.hidden
    }

    /// Every output slot: the output, then the secondary one.
    pub fn outputs(&self) -> Vec<usize> {
        self.output.into_iter().chain(self.secondary).collect()
    }

    /// Whether it mixes the two chemicals in its input tanks (`chemical_infusing`).
    pub fn infuses(&self) -> bool {
        self.tanks.is_some_and(|t| t.inputs.len() == 2)
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
        ticks: TICKS,
        input: Some(0),
        output: Some(1),
        extra: None,
        secondary: None,
        buffer: None,
        tanks: None,
        hidden: 0,
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

/// The Chemical Oxidizer (`GuiChemicalOxidizer`): the item at (26, 36), the
/// progress bar right of it and the output gauge (`GaugeType.STANDARD`) at
/// (131, 13); it has an item input and no item output.
const OXIDIZER_SLOT_POS: [[f32; 2]; 1] = [[26.0, 36.0]];
pub const OXIDIZER_TANKS: FluidTanks = FluidTanks { inputs: &[], output: [131.0, 13.0, 18.0, 58.0], max: 10_000 };

/// The Chemical Infuser (`GuiChemicalInfuser`): left and right input gauges at
/// (25, 17) and (133, 17), the output gauge between them (the screen is
/// 176 x 166 here; Mekanism's is taller, with the slots for chemical items).
pub const INFUSER_TANKS: FluidTanks = FluidTanks {
    inputs: &[[25.0, 17.0, 18.0, 54.0], [133.0, 17.0, 18.0, 54.0]],
    output: [79.0, 16.0, 18.0, 54.0],
    max: 10_000,
};

pub const MACHINES: [Machine; 10] = [
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
        ticks: TICKS,
        input: Some(infuser::SLOT_INPUT),
        output: Some(infuser::SLOT_OUTPUT),
        extra: None,
        secondary: None,
        buffer: Some(&infuser::BUFFER),
        tanks: None,
        hidden: 0,
        slot_pos: &infuser::SLOT_POS,
        energy_gauge: infuser::ENERGY_GAUGE,
        progress_bar: infuser::PROGRESS_BAR,
    },
    Machine {
        output: Some(2),
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
        output: Some(2),
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
        output: Some(2),
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
        output: Some(2),
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
    Machine {
        ticks: 100,
        output: None,
        tanks: Some(&OXIDIZER_TANKS),
        slot_pos: &OXIDIZER_SLOT_POS,
        progress_bar: [64.0, 40.0, 48.0, 8.0],
        ..electric(
            "mekanism:chemical_oxidizer",
            "mekanism:oxidizing",
            "Chemical Oxidizer",
            15,
            "mekanism:tile.machine.chemical_oxidizer",
            OXIDIZER_SOUND_TICKS,
        )
    },
    Machine {
        ticks: 1,
        input: None,
        output: None,
        tanks: Some(&INFUSER_TANKS),
        hidden: 1,
        slot_pos: &[],
        // No progress bar: it mixes every tick.
        progress_bar: [0.0; 4],
        ..electric(
            "mekanism:chemical_infuser",
            "mekanism:chemical_infusing",
            "Chemical Infuser",
            15,
            "mekanism:tile.machine.chemical_infuser",
            CHEMICAL_INFUSER_SOUND_TICKS,
        )
    },
];

const OXIDIZER_SOUND_TICKS: i32 = 44;
const CHEMICAL_INFUSER_SOUND_TICKS: i32 = 41;
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
pub fn step(usage: i32, ticks: i32, energy: &mut i32, progress: &mut i32) -> bool {
    if *energy < usage {
        return false;
    }
    *energy -= usage;
    *progress += 1;
    if *progress >= ticks {
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

/// Menu data slots (each a 16-bit wire entry; every value fits one). A machine
/// with a chemical buffer sends two more (amount, type); a machine with fluid
/// tanks sends two per tank (amount, `chemicals::index` + 1, 0 while empty),
/// from `DATA_BUFFER` on, as it has no buffer.
pub const DATA_ENERGY: usize = 0;
pub const DATA_PROGRESS: usize = 1;
pub const DATA_FACING: usize = 2;
pub const DATA_ACTIVE: usize = 3;
pub const DATA_BUFFER: usize = 4;
/// `Buffer::types` index + 1; 0 while the buffer is empty.
pub const DATA_BUFFER_TYPE: usize = 5;

pub fn menu_data(energy: i32, progress: i32, facing: usize, active: bool, tank: Option<Tank>, fluids: &[Fluid]) -> Vec<i32> {
    let mut data = vec![energy, progress, facing as i32, active as i32];
    if let Some(tank) = tank {
        data.extend([tank.amount, if tank.amount > 0 { tank.kind as i32 + 1 } else { 0 }]);
    }
    for f in fluids {
        let kind = f.id.as_deref().and_then(chemicals::index).map_or(0, |i| i as i32 + 1);
        data.extend([f.amount, kind]);
    }
    data
}

/// The name and colour shown for a fluid tank's menu data (type 0 is empty).
pub fn fluid_look(kind: i32) -> (&'static str, u32) {
    (kind as usize).checked_sub(1).and_then(chemicals::entry).unwrap_or(("Empty", 0))
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
        return vec![format!("Progress: {}%", d(DATA_PROGRESS) * 100 / machine.ticks)];
    }
    if let Some(t) = machine.tanks {
        let hit = t.gauges().position(|g| inside(g, at));
        return hit.map_or_else(Vec::new, |i| {
            let (name, _) = fluid_look(d(DATA_BUFFER + 2 * i + 1));
            vec![format!("{name}: {} / {} mB", d(DATA_BUFFER + 2 * i), t.max)]
        });
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
        assert_eq!(slots("mekanism:crusher"), (Some(0), Some(1), None, None, None, 2));
        assert_eq!(slots("mekanism:metallurgic_infuser"), (Some(1), Some(2), None, None, Some(0), 3));
        assert_eq!(slots("mekanism:osmium_compressor"), (Some(0), Some(2), None, None, Some(1), 3));
        assert_eq!(slots("mekanism:combiner"), (Some(0), Some(2), Some(1), None, None, 3));
        assert_eq!(slots("mekanism:precision_sawmill"), (Some(0), Some(1), None, Some(2), None, 3));
        assert_eq!(slots("mekanism:purification_chamber"), (Some(0), Some(2), None, None, Some(1), 3));
        assert_eq!(slots("mekanism:chemical_injection_chamber"), (Some(0), Some(2), None, None, Some(1), 3));
        assert_eq!(slots("mekanism:chemical_oxidizer"), (Some(0), None, None, None, None, 1));
        assert_eq!(slots("mekanism:chemical_infuser"), (None, None, None, None, None, 1));
    }

    #[test]
    fn every_machine_is_well_formed() {
        for m in &MACHINES {
            assert!(m.usage > 0 && m.sound_ticks > 0, "{}", m.block);
            let mut used: Vec<usize> = m.input.into_iter().chain(m.output).chain(m.extra).chain(m.secondary).collect();
            used.extend(m.buffer.map(|b| b.slot));
            let n = used.len();
            used.sort();
            used.dedup();
            assert_eq!(used.len(), n, "{}: a slot used twice", m.block);
            assert!(used.iter().all(|&s| s < m.slots()), "{}", m.block);
            assert!(m.slots() >= 1 && m.slot_pos.len() <= m.slots(), "{}: Horizon needs a slot", m.block);
            assert_eq!(MACHINES.iter().filter(|o| o.block == m.block).count(), 1, "{}", m.block);
        }
    }

    #[test]
    fn operation_takes_200_ticks_at_usage() {
        let (mut energy, mut progress) = (i32::MAX, 0);
        let start = energy;
        let done = (1..=TICKS).filter(|_| step(8, TICKS, &mut energy, &mut progress)).count();
        assert_eq!(done, 1);
        assert_eq!(progress, 0);
        assert_eq!(start - energy, TICKS * 8);
    }

    #[test]
    fn no_energy_stalls_without_losing_progress() {
        let (mut energy, mut progress) = (4, 7);
        assert!(!step(5, TICKS, &mut energy, &mut progress));
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
        let d = menu_data(32, TICKS - 1, 5, true, None, &[]);
        assert_eq!(d.len(), 4);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        let full = Tank { kind: 7, amount: infuser::BUFFER.max };
        let d = menu_data(20, TICKS - 1, 5, true, Some(full), &[]);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!((d[DATA_BUFFER], d[DATA_BUFFER_TYPE]), (1_000, 8));
        let empty = Tank { kind: 3, amount: 0 };
        assert_eq!(menu_data(0, 0, 2, false, Some(empty), &[])[DATA_BUFFER_TYPE], 0);
    }

    #[test]
    fn tooltips_name_what_is_hovered() {
        let m = machine("mekanism:crusher");
        let data = menu_data(16, 100, 2, true, None, &[]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0]), vec!["Energy: 16 / 32 FE", "Usage: 8 FE/t"]);
        assert_eq!(tooltip(m, &data, [90.0, 40.0]), vec!["Progress: 50%"]);
        assert!(tooltip(m, &data, [10.0, 10.0]).is_empty());
    }

    #[test]
    fn infuser_tooltips_name_what_is_hovered() {
        let m = machine("mekanism:metallurgic_infuser");
        let data = menu_data(15, 100, 2, true, Some(Tank { kind: 0, amount: 260 }), &[]);
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
        let data = menu_data(30, 0, 2, true, Some(Tank { kind: 1, amount: 8 }), &[]);
        assert_eq!(tooltip(m, &data, [70.0, 40.0]), vec!["Hydrogen Chloride: 8 / 210 mB"]);
    }

    #[test]
    fn compressor_tooltip_shows_osmium() {
        let m = machine("mekanism:osmium_compressor");
        let data = menu_data(60, 0, 2, false, Some(Tank { kind: 0, amount: 200 }), &[]);
        assert_eq!(tooltip(m, &data, [70.0, 40.0]), vec!["Osmium: 200 / 210 mB"]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0])[0], "Energy: 60 / 60 FE");
    }

    fn fluid(id: &str, amount: i32) -> Fluid {
        Fluid { id: Some(chemicals::chemical(id)), amount }
    }

    #[test]
    fn oxidizer_and_infuser_values_match_mekanism() {
        let ox = machine("mekanism:chemical_oxidizer");
        assert_eq!((ox.usage, ox.capacity(), ox.ticks), (15, 60, 100));
        assert_eq!(ox.tanks.unwrap().max, 10_000);
        assert!(!ox.infuses());
        let inf = machine("mekanism:chemical_infuser");
        assert_eq!((inf.usage, inf.capacity(), inf.ticks), (15, 60, 1));
        assert_eq!(inf.tanks.unwrap().count(), 3);
        assert_eq!(inf.tanks.unwrap().output_index(), 2);
        assert!(inf.infuses());
        assert!(inf.outputs().is_empty(), "no item output");
    }

    /// Hydrogen and chlorine make hydrogen chloride one to one, whichever tank holds which.
    #[test]
    fn infusing_works_either_way_round() {
        let (h, cl, hcl) = (
            chemicals::chemical("hydrogen"),
            chemicals::chemical("chlorine"),
            chemicals::chemical("hydrogen_chloride"),
        );
        for (a, b) in [(fluid("hydrogen", 5), fluid("chlorine", 5)), (fluid("chlorine", 5), fluid("hydrogen", 5))] {
            let mut tanks = vec![a, b, Fluid::default()];
            assert!(infuse(&mut tanks, 10_000, (&h, 1), (&cl, 1), (&hcl, 1)));
            assert_eq!((tanks[0].amount, tanks[1].amount, tanks[2].amount), (4, 4, 1));
            assert_eq!(tanks[2].id.as_deref(), Some(hcl.as_str()));
        }
    }

    /// Sulfur trioxide takes 1 oxygen and 2 sulfur dioxide for 2 of its own.
    #[test]
    fn infusing_uses_the_recipe_amounts_and_empties_a_tank() {
        let (o, so2, so3) = (
            chemicals::chemical("oxygen"),
            chemicals::chemical("sulfur_dioxide"),
            chemicals::chemical("sulfur_trioxide"),
        );
        let mut tanks = vec![fluid("oxygen", 1), fluid("sulfur_dioxide", 3), Fluid::default()];
        assert!(infuse(&mut tanks, 10_000, (&o, 1), (&so2, 2), (&so3, 2)));
        assert_eq!(tanks[0], Fluid::default(), "an emptied tank forgets its chemical");
        assert_eq!(tanks[1].amount, 1);
        assert_eq!(tanks[2].amount, 2);
        assert!(!infuse(&mut tanks, 10_000, (&o, 1), (&so2, 2), (&so3, 2)), "oxygen is out");
    }

    #[test]
    fn infusing_waits_for_inputs_and_output_room() {
        let (h, cl, hcl) = (
            chemicals::chemical("hydrogen"),
            chemicals::chemical("chlorine"),
            chemicals::chemical("hydrogen_chloride"),
        );
        let run = |tanks: &mut Vec<Fluid>| infuse(tanks, 10_000, (&h, 1), (&cl, 1), (&hcl, 1));
        let mut missing = vec![fluid("hydrogen", 5), Fluid::default(), Fluid::default()];
        assert!(!run(&mut missing));
        assert_eq!(missing[0].amount, 5, "nothing taken");
        let mut full = vec![fluid("hydrogen", 5), fluid("chlorine", 5), fluid("hydrogen_chloride", 10_000)];
        assert!(!run(&mut full));
        assert_eq!((full[0].amount, full[1].amount), (5, 5));
        let mut other = vec![fluid("hydrogen", 5), fluid("chlorine", 5), fluid("oxygen", 1)];
        assert!(!run(&mut other), "output holds another chemical");
        let mut two_same = vec![fluid("hydrogen", 5), fluid("hydrogen", 5), Fluid::default()];
        assert!(!run(&mut two_same));
    }

    #[test]
    fn fluid_tracks_one_chemical() {
        let mut f = Fluid::default();
        assert!(f.fits("a", 10, 10) && !f.fits("a", 11, 10) && !f.fits("a", 0, 10));
        f.add("a", 6);
        assert!(f.fits("a", 4, 10) && !f.fits("b", 1, 10));
        assert!(f.has("a", 6) && !f.has("a", 7) && !f.has("b", 1));
        f.take(100);
        assert_eq!(f, Fluid::default());
    }

    #[test]
    fn fluid_tank_menu_data_and_tooltips() {
        let m = machine("mekanism:chemical_infuser");
        let tanks = [fluid("hydrogen", 9_000), Fluid::default(), fluid("hydrogen_chloride", 1_234)];
        let data = menu_data(45, 0, 2, true, None, &tanks);
        assert_eq!(data.len(), 4 + 6);
        assert!(data.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(tooltip(m, &data, [30.0, 40.0]), vec!["Hydrogen: 9000 / 10000 mB"]);
        assert_eq!(tooltip(m, &data, [140.0, 40.0]), vec!["Empty: 0 / 10000 mB"]);
        assert_eq!(tooltip(m, &data, [85.0, 40.0]), vec!["Hydrogen Chloride: 1234 / 10000 mB"]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0])[0], "Energy: 45 / 60 FE");
        assert!(tooltip(m, &data, [60.0, 40.0]).is_empty(), "no progress bar");
        let ox = machine("mekanism:chemical_oxidizer");
        let data = menu_data(60, 50, 2, true, None, &[fluid("sulfur_dioxide", 300)]);
        assert_eq!(tooltip(ox, &data, [140.0, 40.0]), vec!["Sulfur Dioxide: 300 / 10000 mB"]);
        assert_eq!(tooltip(ox, &data, [80.0, 44.0]), vec!["Progress: 50%"]);
    }
}
