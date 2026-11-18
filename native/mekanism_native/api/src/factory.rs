//! Mekanism's factories (`FactoryTier`, `FactoryType`, `TileEntityFactory` and
//! its subclasses) without Horizon: the tier and type tables, the slot layout,
//! the screen geometry, the shared energy and the move of a machine's (or a
//! lower tier's) contents into a factory. The host side is `factories.rs`.
//!
//! A factory runs `processes` copies of its base machine's operation side by
//! side. All of them draw from one energy buffer (in order, each at the base
//! machine's usage per tick) and, where the base machine has one, share one
//! chemical tank and one extra slot (the combiner's second input, or the slot
//! that fills the tank).

use super::machine::{self, Machine};

/// `FactoryTier`: how many processes, and where `TileEntityFactory`'s slots start and how far
/// apart they sit.
pub struct Tier {
    pub name: &'static str,
    pub processes: usize,
    base_x: f32,
    step: f32,
}

pub const TIERS: [Tier; 4] = [
    Tier { name: "basic", processes: 3, base_x: 55.0, step: 38.0 },
    Tier { name: "advanced", processes: 5, base_x: 35.0, step: 26.0 },
    Tier { name: "elite", processes: 7, base_x: 29.0, step: 19.0 },
    Tier { name: "ultimate", processes: 9, base_x: 27.0, step: 19.0 },
];

pub const ULTIMATE: usize = 3;

/// `FactoryType`: the name in the block id and the machine whose operation it repeats. Mekanism
/// has a ninth, smelting, whose recipes are vanilla's (not reachable from here).
pub const TYPES: [(&str, &str); 8] = [
    ("enriching", "mekanism:enrichment_chamber"),
    ("crushing", "mekanism:crusher"),
    ("compressing", "mekanism:osmium_compressor"),
    ("combining", "mekanism:combiner"),
    ("purifying", "mekanism:purification_chamber"),
    ("injecting", "mekanism:chemical_injection_chamber"),
    ("infusing", super::infuser::BLOCK),
    ("sawing", "mekanism:precision_sawmill"),
];

/// A factory block: a tier and a type, as indices into `TIERS` and `TYPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Factory {
    pub tier: usize,
    pub kind: usize,
}

/// The factory a block id names (`mekanism:elite_combining_factory`).
pub fn find(block: &str) -> Option<Factory> {
    let name = block.strip_prefix("mekanism:")?.strip_suffix("_factory")?;
    let (tier, rest) = name.split_once('_')?;
    Some(Factory {
        tier: TIERS.iter().position(|t| t.name == tier)?,
        kind: TYPES.iter().position(|t| t.0 == rest)?,
    })
}

/// The block id of the factory of `tier` for `base`'s machine, for the tier installer.
pub fn block_of(tier: usize, kind: usize) -> String {
    format!("mekanism:{}_{}_factory", TIERS[tier].name, TYPES[kind].0)
}

/// Mekanism's `AttributeEnergy` of a factory: half the base machine's storage, at least one
/// tick's usage, for every process.
pub fn capacity(base: &Machine, processes: usize) -> i32 {
    let per = (base.storage as f64 * 0.5).max(base.usage as f64);
    (per * processes as f64).min(i32::MAX as f64) as i32
}

/// The factory's slots by role. Inputs, outputs and (for the sawmill) secondary outputs come one
/// per process; the shared slot is the combiner's extra input or the chemical tank's fill slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slots {
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub secondary: Vec<usize>,
    pub shared: Option<usize>,
}

impl Slots {
    /// A single machine's slots, in the same roles.
    pub fn of_machine(m: &Machine) -> Slots {
        Slots {
            inputs: m.input.into_iter().collect(),
            outputs: m.output.into_iter().collect(),
            secondary: m.secondary.into_iter().collect(),
            shared: m.buffer.map(|b| b.slot).or(m.extra),
        }
    }

    pub fn count(&self) -> usize {
        self.inputs.len() + self.outputs.len() + self.secondary.len() + self.shared.iter().len()
    }
}

/// Where the menu data starts per process: energy, facing, active, the tank's amount and type, then
/// the progress of every process (each in ticks of the operation).
pub const DATA_ENERGY: usize = 0;
pub const DATA_FACING: usize = 1;
pub const DATA_ACTIVE: usize = 2;
pub const DATA_TANK: usize = 3;
pub const DATA_TANK_TYPE: usize = 4;
pub const DATA_PROGRESS: usize = 5;

/// The dump button, as `GuiDumpButton`, 21 x 10.
pub const DUMP_SIZE: [f32; 2] = [21.0, 10.0];
/// The button's id in `menu_button`; the infuser's dump button has the same one.
pub const BUTTON_DUMP: i32 = super::infuser::BUTTON_DUMP;

impl Factory {
    pub fn base(&self) -> &'static Machine {
        machine::find(TYPES[self.kind].1).expect("a factory type's base machine is registered")
    }

    pub fn block(&self) -> String {
        block_of(self.tier, self.kind)
    }

    pub fn name(&self) -> String {
        let (tier, kind) = (TIERS[self.tier].name, TYPES[self.kind].0);
        let cap = |s: &str| s[..1].to_uppercase() + &s[1..];
        format!("{} {} Factory", cap(tier), cap(kind))
    }

    pub fn processes(&self) -> usize {
        TIERS[self.tier].processes
    }

    pub fn capacity(&self) -> i32 {
        capacity(self.base(), self.processes())
    }

    /// The most mB the shared tank holds (`MAX_GAS` or `MAX_INFUSE` per process), if it has one.
    pub fn tank_max(&self) -> Option<i32> {
        self.base().buffer.map(|b| b.max * self.processes() as i32)
    }

    pub fn sawing(&self) -> bool {
        self.base().secondary.is_some()
    }

    pub fn slots(&self) -> Slots {
        let p = self.processes();
        let (b, shared) = (self.base(), self.base().buffer.is_some() || self.base().extra.is_some());
        let secondary: Vec<usize> = if b.secondary.is_some() { (2 * p..3 * p).collect() } else { Vec::new() };
        let next = 2 * p + secondary.len();
        Slots { inputs: (0..p).collect(), outputs: (p..2 * p).collect(), secondary, shared: shared.then_some(next) }
    }

    pub fn slot_count(&self) -> usize {
        self.slots().count()
    }

    /// `TileEntityFactory`'s positions: input row above, output row below, the sawmill's secondary row under that.
    fn column(&self, i: usize) -> f32 {
        let t = &TIERS[self.tier];
        t.base_x + i as f32 * t.step
    }

    /// Slot positions by index, as the GUI places them.
    pub fn slot_pos(&self) -> Vec<[f32; 2]> {
        let s = self.slots();
        let mut pos = vec![[0.0; 2]; s.count()];
        for (i, &slot) in s.inputs.iter().enumerate() {
            pos[slot] = [self.column(i), 13.0];
        }
        for (i, &slot) in s.outputs.iter().enumerate() {
            pos[slot] = [self.column(i), 57.0];
        }
        for (i, &slot) in s.secondary.iter().enumerate() {
            pos[slot] = [self.column(i), 77.0];
        }
        if let Some(slot) = s.shared {
            pos[slot] = [7.0, 57.0];
        }
        pos
    }

    /// `GuiFactory`: the Ultimate is 34 wider; a tank bar adds 11 to the height, the sawmill 21.
    pub fn width(&self) -> f32 {
        machine::WIDTH + if self.tier == ULTIMATE { 34.0 } else { 0.0 }
    }

    pub fn height(&self) -> f32 {
        machine::HEIGHT + if self.base().buffer.is_some() { 11.0 } else if self.sawing() { 21.0 } else { 0.0 }
    }

    /// `FactoryContainer`'s player inventory offsets.
    pub fn inventory(&self) -> [f32; 2] {
        let y = if self.base().buffer.is_some() { 95.0 } else if self.sawing() { 105.0 } else { 85.0 };
        [if self.tier == ULTIMATE { 26.0 } else { 8.0 }, y]
    }

    pub fn energy_gauge(&self) -> [f32; 4] {
        [self.width() - 12.0, 16.0, 6.0, if self.sawing() { 73.0 } else { 52.0 }]
    }

    /// The `GuiProgress` of process `i`: 8 x 20, filling downwards.
    pub fn progress_bar(&self, i: usize) -> [f32; 4] {
        [4.0 + self.column(i), 33.0, 8.0, 20.0]
    }

    /// The tank's horizontal bar, and the dump button right of it.
    pub fn tank_bar(&self) -> Option<[f32; 4]> {
        self.base().buffer?;
        Some([7.0, 76.0, if self.tier == ULTIMATE { 172.0 } else { 138.0 }, 4.0])
    }

    pub fn dump(&self) -> Option<[f32; 4]> {
        self.base().buffer?;
        Some([if self.tier == ULTIMATE { 182.0 } else { 148.0 }, 76.0, DUMP_SIZE[0], DUMP_SIZE[1]])
    }

    pub fn data_len(&self) -> usize {
        DATA_PROGRESS + self.processes()
    }
}

/// The result of one process's tick on the shared energy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    /// It had the energy and advanced.
    pub active: bool,
    /// The operation completed.
    pub done: bool,
}

/// One tick of a process that has everything but maybe the energy: it draws `usage` from the shared
/// `energy` and advances, or stalls. The processes call this in order, so a later one finds what the
/// earlier ones left.
pub fn run(usage: i32, ticks: i32, energy: &mut i32, progress: &mut i32) -> Run {
    let active = *energy >= usage;
    Run { active, done: machine::step(usage, ticks, energy, progress) }
}

/// What a process does this tick, from what the host found: no recipe for its input, a recipe it
/// cannot run (output full, chemical short), or one that can run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No recipe: the operation starts over.
    Idle,
    /// Output full or chemical short: it waits where it is and draws nothing.
    Hold,
    Run,
}

pub fn phase(has_recipe: bool, room: bool, fed: bool) -> Phase {
    match (has_recipe, room && fed) {
        (false, _) => Phase::Idle,
        (true, false) => Phase::Hold,
        (true, true) => Phase::Run,
    }
}

/// One tick of a process in `phase`. `powered` runs first, only when `Run` finds the energy for the
/// tick (the per-tick chemical is taken there).
pub fn advance(phase: Phase, usage: i32, ticks: i32, energy: &mut i32, progress: &mut i32, powered: impl FnOnce()) -> Run {
    match phase {
        Phase::Idle => {
            *progress = 0;
            Run { active: false, done: false }
        }
        Phase::Hold => Run { active: false, done: false },
        Phase::Run => {
            if *energy >= usage {
                powered();
            }
            run(usage, ticks, energy, progress)
        }
    }
}

/// Move the contents of one layout into another: input k to input k, and so on, what does not fit
/// (a machine into a factory fits all; the shared slot goes to the shared slot) is dropped. For a
/// machine upgraded to a factory and a factory upgraded to the next tier, whose slots keep their
/// order.
pub fn move_items<T: Clone>(from: &Slots, items: &[Option<T>], to: &Slots) -> Vec<Option<T>> {
    let mut moved = vec![None; to.count()];
    let at = |slots: &[usize], k: usize| slots.get(k).and_then(|&s| items.get(s)).cloned().flatten();
    let mut copy = |src: &[usize], dst: &[usize]| {
        for (k, &slot) in dst.iter().enumerate() {
            moved[slot] = at(src, k);
        }
    };
    copy(&from.inputs, &to.inputs);
    copy(&from.outputs, &to.outputs);
    copy(&from.secondary, &to.secondary);
    if let (Some(src), Some(dst)) = (from.shared, to.shared) {
        moved[dst] = items.get(src).cloned().flatten();
    }
    moved
}

/// The tier installer's target: a machine's first factory, or the next tier of a factory of the
/// same type. `None` for the last tier, or when the installer's tier is not the next one up.
pub fn upgrade(current: Option<Factory>, base: Option<usize>, installer: usize) -> Option<Factory> {
    match (current, base) {
        (Some(f), _) => (f.tier + 1 == installer).then_some(Factory { tier: installer, kind: f.kind }),
        (None, Some(kind)) => (installer == 0).then_some(Factory { tier: 0, kind }),
        _ => None,
    }
}

/// The factory type whose base machine is `block` (the Enrichment Chamber's is enriching).
pub fn kind_of_machine(block: &str) -> Option<usize> {
    TYPES.iter().position(|t| t.1 == block)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn factory(block: &str) -> Factory {
        find(block).unwrap()
    }

    #[test]
    fn block_ids_round_trip_for_every_tier_and_type() {
        for tier in 0..TIERS.len() {
            for kind in 0..TYPES.len() {
                let f = Factory { tier, kind };
                assert_eq!(find(&f.block()), Some(f));
            }
        }
        assert_eq!(find("mekanism:elite_combining_factory"), Some(Factory { tier: 2, kind: 3 }));
        assert_eq!(find("mekanism:basic_smelting_factory"), None);
        assert_eq!(find("mekanism:enrichment_chamber"), None);
        assert_eq!(find("minecraft:basic_enriching_factory"), None);
    }

    #[test]
    fn every_type_has_its_base_machine() {
        for kind in 0..TYPES.len() {
            let f = Factory { tier: 0, kind };
            assert_eq!(f.base().block, TYPES[kind].1);
            assert_eq!(f.base().op, machine::Op::Item);
            assert_eq!(f.base().ticks, machine::TICKS);
        }
    }

    #[test]
    fn processes_by_tier() {
        let p: Vec<usize> = TIERS.iter().map(|t| t.processes).collect();
        assert_eq!(p, [3, 5, 7, 9]);
    }

    #[test]
    fn energy_capacity_is_half_the_storage_per_process() {
        // Enrichment Chamber: usage 5, storage 4 x 5 = 20: 10 per process.
        assert_eq!(factory("mekanism:basic_enriching_factory").capacity(), 30);
        assert_eq!(factory("mekanism:ultimate_enriching_factory").capacity(), 90);
        // Osmium Compressor: usage 15, storage 60: 30 per process.
        assert_eq!(factory("mekanism:advanced_compressing_factory").capacity(), 150);
        // A tier never holds less than one tick of usage per process.
        let m = machine::find("mekanism:crusher").unwrap();
        assert_eq!(capacity(m, 1), m.usage.max(m.storage / 2));
    }

    #[test]
    fn slot_layout_counts() {
        let count = |b: &str| factory(b).slot_count();
        assert_eq!(count("mekanism:basic_enriching_factory"), 6);
        assert_eq!(count("mekanism:ultimate_crushing_factory"), 18);
        // The combiner's extra input and each chemical tank's fill slot are shared.
        assert_eq!(count("mekanism:basic_combining_factory"), 7);
        assert_eq!(count("mekanism:elite_compressing_factory"), 15);
        assert_eq!(count("mekanism:advanced_purifying_factory"), 11);
        assert_eq!(count("mekanism:basic_injecting_factory"), 7);
        assert_eq!(count("mekanism:ultimate_infusing_factory"), 19);
        // The sawmill has a secondary output per process.
        assert_eq!(count("mekanism:basic_sawing_factory"), 9);
        assert_eq!(count("mekanism:ultimate_sawing_factory"), 27);
    }

    #[test]
    fn slots_are_distinct_and_dense() {
        for tier in 0..TIERS.len() {
            for kind in 0..TYPES.len() {
                let f = Factory { tier, kind };
                let s = f.slots();
                let mut all: Vec<usize> = s.inputs.iter().chain(&s.outputs).chain(&s.secondary).copied().chain(s.shared).collect();
                all.sort_unstable();
                assert_eq!(all, (0..f.slot_count()).collect::<Vec<_>>(), "{}", f.block());
                assert_eq!(f.slot_pos().len(), f.slot_count());
            }
        }
    }

    #[test]
    fn slot_positions_follow_the_gui() {
        let pos = factory("mekanism:basic_enriching_factory").slot_pos();
        assert_eq!(pos[0], [55.0, 13.0]);
        assert_eq!(pos[2], [131.0, 13.0]);
        assert_eq!(pos[3], [55.0, 57.0]);
        let pos = factory("mekanism:ultimate_sawing_factory").slot_pos();
        assert_eq!(pos[8], [27.0 + 8.0 * 19.0, 13.0]);
        assert_eq!(pos[9], [27.0, 57.0]);
        assert_eq!(pos[18], [27.0, 77.0]);
        assert_eq!(pos[26], [27.0 + 8.0 * 19.0, 77.0]);
        // The shared slot is under the energy tab's side, at the left edge.
        let f = factory("mekanism:advanced_combining_factory");
        assert_eq!(f.slot_pos()[f.slots().shared.unwrap()], [7.0, 57.0]);
    }

    #[test]
    fn every_slot_fits_the_screen() {
        for tier in 0..TIERS.len() {
            for kind in 0..TYPES.len() {
                let f = Factory { tier, kind };
                for [x, y] in f.slot_pos() {
                    assert!(x + 18.0 <= f.width() && y + 18.0 <= f.height() - 20.0, "{}", f.block());
                }
                let [gx, _, gw, _] = f.energy_gauge();
                assert!(gx + gw <= f.width());
            }
        }
    }

    #[test]
    fn screen_sizes_follow_the_gui() {
        let f = factory("mekanism:basic_enriching_factory");
        assert_eq!((f.width(), f.height(), f.inventory()), (176.0, 166.0, [8.0, 85.0]));
        let f = factory("mekanism:ultimate_compressing_factory");
        assert_eq!((f.width(), f.height(), f.inventory()), (210.0, 177.0, [26.0, 95.0]));
        let f = factory("mekanism:elite_sawing_factory");
        assert_eq!((f.width(), f.height(), f.inventory()), (176.0, 187.0, [8.0, 105.0]));
        assert_eq!(f.energy_gauge(), [164.0, 16.0, 6.0, 73.0]);
        assert_eq!(factory("mekanism:ultimate_infusing_factory").dump(), Some([182.0, 76.0, 21.0, 10.0]));
        assert_eq!(factory("mekanism:basic_combining_factory").dump(), None);
        assert_eq!(f.progress_bar(1), [4.0 + 29.0 + 19.0, 33.0, 8.0, 20.0]);
    }

    #[test]
    fn tank_scales_with_processes() {
        assert_eq!(factory("mekanism:basic_compressing_factory").tank_max(), Some(630));
        assert_eq!(factory("mekanism:ultimate_infusing_factory").tank_max(), Some(9_000));
        assert_eq!(factory("mekanism:basic_crushing_factory").tank_max(), None);
    }

    #[test]
    fn data_covers_every_process() {
        assert_eq!(factory("mekanism:ultimate_crushing_factory").data_len(), DATA_PROGRESS + 9);
    }

    #[test]
    fn processes_share_one_energy_buffer_in_order() {
        // Usage 5, energy for two ticks: the first two processes run, the third stalls.
        let (mut energy, mut progress) = (10, [0, 0, 0]);
        let runs: Vec<Run> = progress.iter_mut().map(|p| run(5, 200, &mut energy, p)).collect();
        assert_eq!(runs.iter().map(|r| r.active).collect::<Vec<_>>(), [true, true, false]);
        assert_eq!((energy, progress), (0, [1, 1, 0]));
        // With one tick of energy left, the first process in order takes it.
        let mut energy = 5;
        let mut p = [0];
        assert!(run(5, 200, &mut energy, &mut p[0]).active);
        assert!(!run(5, 200, &mut energy, &mut p[0]).active);
        assert_eq!(p[0], 1);
    }

    #[test]
    fn idle_resets_hold_waits_and_run_needs_energy() {
        let mut energy = 10;
        let mut progress = 50;
        let mut taken = 0;
        let r = advance(phase(false, true, true), 5, 200, &mut energy, &mut progress, || taken += 1);
        assert_eq!((r, progress, energy, taken), (Run { active: false, done: false }, 0, 10, 0));
        progress = 50;
        for (room, fed) in [(false, true), (true, false)] {
            let r = advance(phase(true, room, fed), 5, 200, &mut energy, &mut progress, || taken += 1);
            assert_eq!((r.active, progress, energy, taken), (false, 50, 10, 0));
        }
        let r = advance(phase(true, true, true), 5, 200, &mut energy, &mut progress, || taken += 1);
        assert_eq!((r.active, progress, energy, taken), (true, 51, 5, 1));
        // Without the energy for a tick it neither advances nor takes its chemical.
        let mut energy = 4;
        let r = advance(Phase::Run, 5, 200, &mut energy, &mut progress, || taken += 1);
        assert_eq!((r.active, progress, taken), (false, 51, 1));
    }

    #[test]
    fn an_operation_completes_after_its_ticks_and_restarts() {
        let (mut energy, mut progress) = (1_000, 0);
        for tick in 1..=200 {
            let r = run(5, 200, &mut energy, &mut progress);
            assert_eq!(r.done, tick == 200, "tick {tick}");
        }
        assert_eq!((energy, progress), (0, 0));
    }

    #[test]
    fn move_items_keeps_the_roles() {
        let machine = Slots::of_machine(machine::find("mekanism:precision_sawmill").unwrap());
        let f = factory("mekanism:basic_sawing_factory").slots();
        let items = [Some("log"), Some("planks"), Some("sawdust")];
        let moved = move_items(&machine, &items, &f);
        assert_eq!(moved[f.inputs[0]], Some("log"));
        assert_eq!(moved[f.outputs[0]], Some("planks"));
        assert_eq!(moved[f.secondary[0]], Some("sawdust"));
        assert_eq!(moved.iter().flatten().count(), 3);
        // Up a tier: the first three processes keep theirs.
        let adv = factory("mekanism:advanced_sawing_factory").slots();
        let up = move_items(&f, &moved, &adv);
        assert_eq!(up.len(), 15);
        assert_eq!(up[adv.inputs[0]], Some("log"));
        assert_eq!(up[adv.outputs[0]], Some("planks"));
        assert_eq!(up[adv.secondary[0]], Some("sawdust"));
    }

    #[test]
    fn move_items_carries_the_shared_slot() {
        let machine = Slots::of_machine(machine::find("mekanism:osmium_compressor").unwrap());
        let f = factory("mekanism:basic_compressing_factory").slots();
        let moved = move_items(&machine, &[Some("ore"), Some("osmium"), Some("ingot")], &f);
        assert_eq!(moved[f.shared.unwrap()], Some("osmium"));
        assert_eq!(moved[f.inputs[0]], Some("ore"));
        assert_eq!(moved[f.outputs[0]], Some("ingot"));
        let machine = Slots::of_machine(machine::find("mekanism:combiner").unwrap());
        let f = factory("mekanism:basic_combining_factory").slots();
        let moved = move_items(&machine, &[Some("dust"), Some("cobble"), Some("ore")], &f);
        assert_eq!(moved[f.shared.unwrap()], Some("cobble"));
    }

    #[test]
    fn upgrades_go_one_tier_up() {
        let enriching = kind_of_machine("mekanism:enrichment_chamber");
        assert_eq!(enriching, Some(0));
        assert_eq!(upgrade(None, enriching, 0), Some(Factory { tier: 0, kind: 0 }));
        // A machine takes only the basic installer.
        assert_eq!(upgrade(None, enriching, 1), None);
        let basic = find("mekanism:basic_crushing_factory");
        assert_eq!(upgrade(basic, None, 1), Some(Factory { tier: 1, kind: 1 }));
        assert_eq!(upgrade(basic, None, 2), None);
        assert_eq!(upgrade(basic, None, 0), None);
        assert_eq!(upgrade(find("mekanism:ultimate_crushing_factory"), None, 3), None);
        assert_eq!(kind_of_machine("mekanism:crusher"), Some(1));
        assert_eq!(kind_of_machine("mekanism:chemical_oxidizer"), None);
    }
}
