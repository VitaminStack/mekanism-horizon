//! Metallurgic Infuser: what it has beyond the other electric machines (its
//! `machine::MACHINES` entry): the infuse buffer, its gauge and dump button.
//! No Horizon types, so the tests run in `logic/energy` without building Horizon.
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

pub const MAX_INFUSE: i32 = 1_000;

use super::machine::inside;

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

/// Slot positions in Mekanism's `GuiMetallurgicInfuser` (176 x 166).
pub const SLOT_POS: [[f32; 2]; SLOTS] = [[17.0, 35.0], [51.0, 43.0], [109.0, 43.0]];

/// Screen rects (x, y, w, h) from the screen's top left.
pub const INFUSE_GAUGE: [f32; 4] = [7.0, 17.0, 6.0, 54.0];
pub const ENERGY_GAUGE: [f32; 4] = [164.0, 17.0, 6.0, 54.0];
pub const PROGRESS_BAR: [f32; 4] = [72.0, 46.0, 32.0, 6.0];
/// Mekanism's `GuiGasDumpButton` under the infusion slot: empties the buffer.
pub const DUMP_BUTTON: [f32; 4] = [16.0, 56.0, 18.0, 12.0];
/// Menu button id of the dump button (the cube's side buttons use 0 to 11).
pub const BUTTON_DUMP: i32 = 100;

/// Tooltip lines for the infuse gauge and the dump button under `at`, from
/// the buffer's menu data (`machine::DATA_INFUSE`, `machine::DATA_TYPE`).
pub fn tooltip(infuse: i32, kind: i32, at: [f32; 2]) -> Vec<String> {
    if inside(INFUSE_GAUGE, at) {
        let name = kind.checked_sub(1).and_then(|k| TYPES.get(k as usize)).map_or("Empty", |t| t.1);
        return vec![format!("{name}: {infuse} / {MAX_INFUSE} mB")];
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
    fn types_resolve() {
        assert_eq!(type_index("mekanism:carbon"), Some(0));
        assert_eq!(type_index("mekanism:oxygen"), None);
    }
}
