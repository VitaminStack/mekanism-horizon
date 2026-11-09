//! Electric machines (one item in, one item out): Enrichment Chamber and
//! Crusher. The parts that need no Horizon types, so their tests run in
//! `logic/energy`; item matching and slots are in `lib.rs`.
//!
//! Values from Mekanism 26.3: `TileEntityElectricMachine`
//! (`BASE_TICKS_REQUIRED` 10 s, slots input 64,17 and output 116,35),
//! `UsageConfig` (enrichmentChamber 5, crusher 8 per tick) and
//! `AttributeEnergy.STORAGE_MULTIPLIER` (storage 4 x usage).
//!
//! Not ported, as on the infuser: the energy slot (charging from items),
//! upgrades, side config and redstone control.

pub struct Machine {
    pub block: &'static str,
    /// Registered by `data/mekanism/horizon/menu/<id>.json`.
    pub menu: &'static str,
    /// Mekanism's recipe type (`input`, `output`), shipped by the content mod.
    pub recipe: &'static str,
    pub name: &'static str,
    pub usage: i32,
    /// Mekanism's loop while running (`tile.machine.<id>` in `sounds.json`)
    /// and its clip length in ticks, after which it is played again.
    pub sound: &'static str,
    pub sound_ticks: i32,
}

impl Machine {
    pub const fn capacity(&self) -> i32 {
        4 * self.usage
    }
}

pub const MACHINES: [Machine; 2] = [
    Machine {
        block: "mekanism:enrichment_chamber",
        menu: "mekanism:enrichment_chamber",
        recipe: "mekanism:enriching",
        name: "Enrichment Chamber",
        usage: 5,
        sound: "mekanism:tile.machine.enrichment_chamber",
        sound_ticks: 82,
    },
    Machine {
        block: "mekanism:crusher",
        menu: "mekanism:crusher",
        recipe: "mekanism:crushing",
        name: "Crusher",
        usage: 8,
        sound: "mekanism:tile.machine.crusher",
        sound_ticks: 37,
    },
];

pub fn find(block: &str) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| m.block == block)
}

pub const TICKS: i32 = 200;

pub const SLOT_INPUT: usize = 0;
pub const SLOT_OUTPUT: usize = 1;
pub const SLOTS: usize = 2;

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

/// Menu data slots (each a 16-bit wire entry; every value fits one).
pub const DATA_ENERGY: usize = 0;
pub const DATA_PROGRESS: usize = 1;
pub const DATA_FACING: usize = 2;
pub const DATA_ACTIVE: usize = 3;
pub const DATA_LEN: usize = 4;

pub fn menu_data(energy: i32, progress: i32, facing: usize, active: bool) -> [i32; DATA_LEN] {
    [energy, progress, facing as i32, active as i32]
}

/// Slot positions in Mekanism's `GuiElectricMachine` (176 x 166).
pub const SLOT_POS: [[f32; 2]; SLOTS] = [[64.0, 17.0], [116.0, 35.0]];
pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;

/// Screen rects (x, y, w, h) from the screen's top left.
pub const ENERGY_GAUGE: [f32; 4] = [164.0, 16.0, 6.0, 54.0];
pub const PROGRESS_BAR: [f32; 4] = [86.0, 38.0, 28.0, 8.0];

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    let [x, y, w, h] = rect;
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

/// Tooltip lines for the gauge and bar under `at`, from the menu data.
pub fn tooltip(machine: &Machine, data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(ENERGY_GAUGE, at) {
        return vec![
            format!("Energy: {} / {} FE", d(DATA_ENERGY), machine.capacity()),
            format!("Usage: {} FE/t", machine.usage),
        ];
    }
    if inside(PROGRESS_BAR, at) {
        return vec![format!("Progress: {}%", d(DATA_PROGRESS) * 100 / TICKS)];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_mekanism() {
        let e = find("mekanism:enrichment_chamber").unwrap();
        let c = find("mekanism:crusher").unwrap();
        assert_eq!((e.usage, e.capacity()), (5, 20));
        assert_eq!((c.usage, c.capacity()), (8, 32));
        assert!(find("mekanism:metallurgic_infuser").is_none());
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
        let d = menu_data(32, TICKS - 1, 5, true);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
    }

    #[test]
    fn tooltips_name_what_is_hovered() {
        let m = find("mekanism:crusher").unwrap();
        let data = menu_data(16, 100, 2, true);
        assert_eq!(tooltip(m, &data, [165.0, 40.0]), vec!["Energy: 16 / 32 FE", "Usage: 8 FE/t"]);
        assert_eq!(tooltip(m, &data, [90.0, 40.0]), vec!["Progress: 50%"]);
        assert!(tooltip(m, &data, [10.0, 10.0]).is_empty());
    }
}
