//! Metallurgic Infuser: what it has beyond the other electric machines (its
//! `machine::MACHINES` entry): the infuse buffer's types, layout and dump button.
//! No Horizon types, so the tests run in `logic/energy` without building Horizon.
//!
//! Values from Mekanism 26.3: `TileEntityMetallurgicInfuser` (`MAX_INFUSE` one
//! bucket, `BASE_TICKS_REQUIRED` 10 s), `UsageConfig.metallurgicInfuser` (5 per
//! tick) and `AttributeEnergy.STORAGE_MULTIPLIER` (storage 4 x usage).
//!
//! Not ported: the energy slot (charging from items), upgrades, side config and
//! redstone control. Energy comes in from neighbouring cubes or any block that
//! fills Horizon's energy buffer.

use super::machine::{Buffer, Chemical};

pub const BLOCK: &str = "mekanism:metallurgic_infuser";
/// Registered by `data/mekanism/horizon/menu/metallurgic_infuser.json`.
pub const MENU: &str = "mekanism:metallurgic_infuser";
pub const INFUSING: &str = "mekanism:metallurgic_infusing";
pub const CONVERSION: &str = "mekanism:chemical_conversion";
/// Mekanism's loop while running and its clip length in ticks.
pub const SOUND: &str = "mekanism:tile.machine.metallurgic_infuser";
pub const SOUND_TICKS: i32 = 29;

pub const SLOT_INFUSION: usize = 0;
pub const SLOT_INPUT: usize = 1;
pub const SLOT_OUTPUT: usize = 2;
pub const SLOTS: usize = 3;

/// Infuse types (chemicals with Mekanism's infuse-type attribute), with a bar
/// colour close to the chemical's (approximate: Mekanism's tints are not in
/// the checked-out sources). Index + 1 is what the menu data and NBT store.
pub const TYPES: [Chemical; 8] = [
    ("mekanism:carbon", "Carbon", 0xFF2C2C2C),
    ("mekanism:redstone", "Redstone", 0xFFB30505),
    ("mekanism:diamond", "Diamond", 0xFF6CEDD8),
    ("mekanism:refined_obsidian", "Refined Obsidian", 0xFF7C00ED),
    ("mekanism:tin", "Tin", 0xFFCCCCD9),
    ("mekanism:gold", "Gold", 0xFFF2CD67),
    ("mekanism:bio", "Biomass", 0xFF5A4630),
    ("mekanism:fungi", "Fungi", 0xFF7D6E54),
];

/// The infuse buffer: one type at a time, up to a bucket (`MAX_INFUSE`), its
/// gauge left of the infusion slot and Mekanism's `GuiGasDumpButton` under it.
pub const BUFFER: Buffer = Buffer {
    slot: SLOT_INFUSION,
    types: &TYPES,
    max: 1_000,
    gauge: [7.0, 17.0, 6.0, 54.0],
    dump: Some([16.0, 56.0, 18.0, 12.0]),
};

/// Slot positions in Mekanism's `GuiMetallurgicInfuser` (176 x 166).
pub const SLOT_POS: [[f32; 2]; SLOTS] = [[17.0, 35.0], [51.0, 43.0], [109.0, 43.0]];

/// Screen rects (x, y, w, h) from the screen's top left.
pub const ENERGY_GAUGE: [f32; 4] = [164.0, 17.0, 6.0, 54.0];
pub const PROGRESS_BAR: [f32; 4] = [72.0, 46.0, 32.0, 6.0];
/// Menu button id of the dump button (the cube's side buttons use 0 to 11).
pub const BUTTON_DUMP: i32 = 100;
pub const DUMP_TOOLTIP: &str = "Dump infuse buffer";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_resolve() {
        assert_eq!(BUFFER.type_index("mekanism:carbon"), Some(0));
        assert_eq!(BUFFER.type_index("mekanism:oxygen"), None);
    }
}
