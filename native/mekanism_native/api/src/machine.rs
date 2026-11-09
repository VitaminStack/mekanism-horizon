//! Electric machines: the table every processing machine is one entry of
//! (Enrichment Chamber, Crusher, Metallurgic Infuser), and the operation they
//! share. The parts that need no Horizon types, so their tests run in
//! `logic/energy`; recipes are in `recipe.rs`, the tick and screen in `process.rs`.
//!
//! A new machine of this kind is one more entry in `MACHINES` plus its content
//! files (block, menu, recipes): the hooks in `lib.rs` find it through `find`.
//!
//! Values from Mekanism 26.3: `TileEntityElectricMachine`
//! (`BASE_TICKS_REQUIRED` 10 s, slots input 64,17 and output 116,35),
//! `UsageConfig` (enrichmentChamber 5, crusher 8, metallurgicInfuser 5 per
//! tick) and `AttributeEnergy.STORAGE_MULTIPLIER` (storage 4 x usage).
//!
//! Not ported: the energy slot (charging from items), upgrades, side config
//! and redstone control.

use super::infuser;

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
    /// The slot whose items fill the infuse buffer (`infuser::Tank`); a machine
    /// without that buffer has none.
    pub infusion: Option<usize>,
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
        infusion: None,
        slot_pos: &ELECTRIC_SLOT_POS,
        energy_gauge: ELECTRIC_ENERGY_GAUGE,
        progress_bar: ELECTRIC_PROGRESS_BAR,
    }
}

pub const MACHINES: [Machine; 3] = [
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
        infusion: Some(infuser::SLOT_INFUSION),
        slot_pos: &infuser::SLOT_POS,
        energy_gauge: infuser::ENERGY_GAUGE,
        progress_bar: infuser::PROGRESS_BAR,
    },
];

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

/// Menu data slots (each a 16-bit wire entry; every value fits one). The last
/// two are sent only by a machine with an infuse buffer.
pub const DATA_ENERGY: usize = 0;
pub const DATA_PROGRESS: usize = 1;
pub const DATA_FACING: usize = 2;
pub const DATA_ACTIVE: usize = 3;
pub const DATA_INFUSE: usize = 4;
/// `infuser::TYPES` index + 1; 0 while the buffer is empty.
pub const DATA_TYPE: usize = 5;

pub fn menu_data(energy: i32, progress: i32, facing: usize, active: bool, tank: Option<infuser::Tank>) -> Vec<i32> {
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
    if machine.infusion.is_some() {
        return infuser::tooltip(d(DATA_INFUSE), d(DATA_TYPE), at);
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
        let e = machine("mekanism:enrichment_chamber");
        let c = machine("mekanism:crusher");
        let i = machine("mekanism:metallurgic_infuser");
        assert_eq!((e.usage, e.capacity()), (5, 20));
        assert_eq!((c.usage, c.capacity()), (8, 32));
        assert_eq!((i.usage, i.capacity()), (5, 20));
        assert!(find("mekanism:basic_energy_cube").is_none());
    }

    /// Slot indices are saved with the block entity: they must not move.
    #[test]
    fn slots_keep_their_saved_order() {
        let c = machine("mekanism:crusher");
        assert_eq!((c.input, c.output, c.infusion, c.slots()), (0, 1, None, 2));
        let i = machine("mekanism:metallurgic_infuser");
        assert_eq!((i.input, i.output, i.infusion, i.slots()), (1, 2, Some(0), 3));
    }

    #[test]
    fn every_machine_is_well_formed() {
        for m in &MACHINES {
            assert!(m.usage > 0 && m.sound_ticks > 0, "{}", m.block);
            assert!(m.input < m.slots() && m.output < m.slots() && m.input != m.output, "{}", m.block);
            assert!(m.infusion.is_none_or(|s| s < m.slots() && s != m.input && s != m.output), "{}", m.block);
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
    fn menu_data_fits_16_bits() {
        let d = menu_data(32, TICKS - 1, 5, true, None);
        assert_eq!(d.len(), 4);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        let full = infuser::Tank { kind: 7, amount: infuser::MAX_INFUSE };
        let d = menu_data(20, TICKS - 1, 5, true, Some(full));
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!((d[DATA_INFUSE], d[DATA_TYPE]), (1_000, 8));
        let empty = infuser::Tank { kind: 3, amount: 0 };
        assert_eq!(menu_data(0, 0, 2, false, Some(empty))[DATA_TYPE], 0);
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
        let data = menu_data(15, 100, 2, true, Some(infuser::Tank { kind: 0, amount: 260 }));
        assert_eq!(tooltip(m, &data, [8.0, 40.0]), vec!["Carbon: 260 / 1000 mB"]);
        assert_eq!(tooltip(m, &data, [165.0, 40.0])[0], "Energy: 15 / 20 FE");
        assert_eq!(tooltip(m, &data, [80.0, 48.0]), vec!["Progress: 50%"]);
        assert_eq!(tooltip(m, &data, [20.0, 60.0]).len(), 1);
        assert!(tooltip(m, &data, [90.0, 10.0]).is_empty());
        assert_eq!(tooltip(m, &[], [8.0, 40.0]), vec!["Empty: 0 / 1000 mB"]);
    }
}
