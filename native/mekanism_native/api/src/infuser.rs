//! Metallurgic Infuser: the parts that need no Horizon types, so their tests
//! run in `logic/energy` without building Horizon. Item matching and slots are
//! in `lib.rs`.
//!
//! Values from Mekanism 26.3: `TileEntityMetallurgicInfuser` (`MAX_INFUSE` one
//! bucket, `BASE_TICKS_REQUIRED` 10 s), `UsageConfig.metallurgicInfuser` (5 per
//! tick) and `AttributeEnergy.STORAGE_MULTIPLIER` (storage 4 x usage).
//!
//! Not ported: the energy slot (charging from items), upgrades, side config,
//! redstone control, and dumping the infuse buffer. Energy comes in from
//! neighbouring cubes or any block that fills Horizon's energy buffer.

pub const BLOCK: &str = "mekanism:metallurgic_infuser";
/// Registered by `data/mekanism/horizon/menu/metallurgic_infuser.json`.
pub const MENU: &str = "mekanism:metallurgic_infuser";
pub const INFUSING: &str = "mekanism:metallurgic_infusing";
pub const CONVERSION: &str = "mekanism:chemical_conversion";
/// Mekanism's loop while running and its clip length in ticks.
pub const SOUND: &str = "mekanism:tile.machine.metallurgic_infuser";
pub const SOUND_TICKS: i32 = 29;

pub const USAGE: i32 = 5;
pub const CAPACITY: i32 = 4 * USAGE;
pub const TICKS: i32 = 200;
pub const MAX_INFUSE: i32 = 1_000;

pub const SLOT_INFUSION: usize = 0;
pub const SLOT_INPUT: usize = 1;
pub const SLOT_OUTPUT: usize = 2;
pub const SLOTS: usize = 3;

/// Infuse types (chemicals with Mekanism's infuse-type attribute), with a bar
/// colour close to the chemical's (approximate: Mekanism's tints are not in
/// the checked-out sources). Index + 1 is what the menu data and NBT store.
pub const TYPES: [(&str, &str, u32); 8] = [
    ("mekanism:carbon", "Carbon", 0xFF2C2C2C),
    ("mekanism:redstone", "Redstone", 0xFFB30505),
    ("mekanism:diamond", "Diamond", 0xFF6CEDD8),
    ("mekanism:refined_obsidian", "Refined Obsidian", 0xFF7C00ED),
    ("mekanism:tin", "Tin", 0xFFCCCCD9),
    ("mekanism:gold", "Gold", 0xFFF2CD67),
    ("mekanism:bio", "Biomass", 0xFF5A4630),
    ("mekanism:fungi", "Fungi", 0xFF7D6E54),
];

pub fn type_index(id: &str) -> Option<usize> {
    TYPES.iter().position(|(t, ..)| *t == id)
}

/// The infuse buffer: one type at a time, as Mekanism's chemical tank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tank {
    /// `TYPES` index; meaningless while `amount` is 0.
    pub kind: usize,
    pub amount: i32,
}

impl Tank {
    /// Whether `amount` of `kind` fits whole (Mekanism converts one item only
    /// when all its output fits).
    pub fn fits(&self, kind: usize, amount: i32) -> bool {
        (self.amount == 0 || self.kind == kind) && amount > 0 && self.amount + amount <= MAX_INFUSE
    }

    pub fn add(&mut self, kind: usize, amount: i32) {
        debug_assert!(self.fits(kind, amount));
        if self.amount == 0 {
            self.kind = kind;
        }
        self.amount += amount;
    }

    pub fn has(&self, kind: usize, amount: i32) -> bool {
        self.amount > 0 && self.kind == kind && self.amount >= amount
    }

    pub fn take(&mut self, amount: i32) {
        self.amount = (self.amount - amount).max(0);
    }
}

/// One tick of an operation that can run: spend `USAGE` and advance, or stall
/// without energy. Returns true when the operation completes this tick, and
/// then progress starts over.
pub fn step(energy: &mut i32, progress: &mut i32) -> bool {
    if *energy < USAGE {
        return false;
    }
    *energy -= USAGE;
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
pub const DATA_INFUSE: usize = 2;
pub const DATA_TYPE: usize = 3;
pub const DATA_FACING: usize = 4;
pub const DATA_ACTIVE: usize = 5;
pub const DATA_LEN: usize = 6;

pub fn menu_data(energy: i32, progress: i32, tank: Tank, facing: usize, active: bool) -> [i32; DATA_LEN] {
    let kind = if tank.amount > 0 { tank.kind as i32 + 1 } else { 0 };
    [energy, progress, tank.amount, kind, facing as i32, active as i32]
}

/// Slot positions in Mekanism's `GuiMetallurgicInfuser` (176 x 166).
pub const SLOT_POS: [[f32; 2]; SLOTS] = [[17.0, 35.0], [51.0, 43.0], [109.0, 43.0]];
pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;

/// Screen rects (x, y, w, h) from the screen's top left.
pub const INFUSE_GAUGE: [f32; 4] = [7.0, 17.0, 6.0, 54.0];
pub const ENERGY_GAUGE: [f32; 4] = [164.0, 17.0, 6.0, 54.0];
pub const PROGRESS_BAR: [f32; 4] = [72.0, 46.0, 32.0, 6.0];
/// Mekanism's `GuiGasDumpButton` under the infusion slot: empties the buffer.
pub const DUMP_BUTTON: [f32; 4] = [16.0, 56.0, 18.0, 12.0];
/// Menu button id of the dump button (the cube's side buttons use 0 to 11).
pub const BUTTON_DUMP: i32 = 100;

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    let [x, y, w, h] = rect;
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

/// Tooltip lines for the gauges and the dump button under `at`, from the menu data.
pub fn tooltip(data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(INFUSE_GAUGE, at) {
        let name = d(DATA_TYPE).checked_sub(1).and_then(|k| TYPES.get(k as usize)).map_or("Empty", |t| t.1);
        return vec![format!("{name}: {} / {MAX_INFUSE} mB", d(DATA_INFUSE))];
    }
    if inside(ENERGY_GAUGE, at) {
        return vec![format!("Energy: {} / {CAPACITY} FE", d(DATA_ENERGY)), format!("Usage: {USAGE} FE/t")];
    }
    if inside(PROGRESS_BAR, at) {
        return vec![format!("Progress: {}%", d(DATA_PROGRESS) * 100 / TICKS)];
    }
    if inside(DUMP_BUTTON, at) {
        return vec!["Dump infuse buffer".to_owned()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tank_takes_one_type_up_to_a_bucket() {
        let mut t = Tank::default();
        assert!(t.fits(0, 1_000));
        assert!(!t.fits(0, 1_001));
        t.add(0, 990);
        assert!(t.fits(0, 10));
        assert!(!t.fits(0, 20));
        assert!(!t.fits(1, 10), "other type while not empty");
        t.take(990);
        assert!(t.fits(1, 10), "empty tank takes any type");
        assert!(!t.fits(1, 0));
    }

    #[test]
    fn tank_has_needs_type_and_amount() {
        let mut t = Tank::default();
        assert!(!t.has(0, 0), "empty tank has nothing");
        t.add(2, 20);
        assert!(t.has(2, 20));
        assert!(!t.has(2, 21));
        assert!(!t.has(0, 10));
    }

    #[test]
    fn operation_takes_200_ticks_and_5_per_tick() {
        let (mut energy, mut progress) = (i32::MAX, 0);
        let start = energy;
        let done = (1..=TICKS).filter(|_| step(&mut energy, &mut progress)).count();
        assert_eq!(done, 1);
        assert_eq!(progress, 0);
        assert_eq!(start - energy, TICKS * USAGE);
    }

    #[test]
    fn no_energy_stalls_without_losing_progress() {
        let (mut energy, mut progress) = (USAGE - 1, 7);
        assert!(!step(&mut energy, &mut progress));
        assert_eq!((energy, progress), (USAGE - 1, 7));
    }

    #[test]
    fn menu_data_fits_16_bits() {
        let d = menu_data(CAPACITY, TICKS - 1, Tank { kind: 7, amount: MAX_INFUSE }, 5, true);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(d[DATA_TYPE], 8);
        assert_eq!(menu_data(0, 0, Tank { kind: 3, amount: 0 }, 2, false)[DATA_TYPE], 0);
    }

    #[test]
    fn tooltips_name_what_is_hovered() {
        let data = menu_data(15, 100, Tank { kind: 0, amount: 260 }, 2, true);
        assert_eq!(tooltip(&data, [8.0, 40.0]), vec!["Carbon: 260 / 1000 mB"]);
        assert_eq!(tooltip(&data, [165.0, 40.0])[0], "Energy: 15 / 20 FE");
        assert_eq!(tooltip(&data, [80.0, 48.0]), vec!["Progress: 50%"]);
        assert_eq!(tooltip(&data, [20.0, 60.0]).len(), 1);
        assert!(tooltip(&data, [90.0, 10.0]).is_empty());
        assert_eq!(tooltip(&[], [8.0, 40.0]), vec!["Empty: 0 / 1000 mB"]);
    }

    #[test]
    fn types_resolve() {
        assert_eq!(type_index("mekanism:carbon"), Some(0));
        assert_eq!(type_index("mekanism:oxygen"), None);
    }
}
