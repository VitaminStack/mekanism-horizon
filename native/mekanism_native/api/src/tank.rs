//! Fluid Tank and Chemical Tank (all tiers): the parts that need no Horizon
//! types, so their tests run in `logic/energy`. Hooks are in `tanks.rs`.
//!
//! Values from Mekanism 26.3: `FluidTankTier`, `ChemicalTankTier` (capacity,
//! transfer rate per tick), `TileEntityChemicalTank` / `ChemicalUtils.dump`
//! (dumping `capacity / 400` per tick; dumping excess down to
//! `dumpExcessKeepRatio` 0.85 at the transfer rate), `TileEntityFluidTank`
//! (an active tank pushes its transfer rate into the block below).
//!
//! The contents are Horizon's own tank storage (`fluids` in the block-entity
//! NBT); a chemical is a `chemicals::CHEMICAL_PREFIX` id. Creative tiers keep
//! what they hold full. Horizon tanks count in i32 millibuckets, so the
//! creative tanks (Long.MAX_VALUE in Mekanism) hold i32::MAX.
//!
//! Not ported: the item fill/drain slots, Mekanism's chemical side config and
//! ejector, the fluid tank's edit mode and valve journal, redstone control.

use crate::chemicals;
use crate::cube;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Fluid,
    Chemical,
}

#[derive(Clone, Copy, Debug)]
pub struct Tier {
    pub block: &'static str,
    pub family: Family,
    pub capacity: i32,
    pub rate: i32,
    pub creative: bool,
}

const fn tier(block: &'static str, family: Family, capacity: i32, rate: i32) -> Tier {
    Tier { block, family, capacity, rate, creative: false }
}

const B: i32 = 1_000;
pub const TANKS: [Tier; 10] = [
    tier("mekanism:basic_fluid_tank", Family::Fluid, 32 * B, B),
    tier("mekanism:advanced_fluid_tank", Family::Fluid, 64 * B, 4 * B),
    tier("mekanism:elite_fluid_tank", Family::Fluid, 128 * B, 16 * B),
    tier("mekanism:ultimate_fluid_tank", Family::Fluid, 256 * B, 64 * B),
    Tier { creative: true, ..tier("mekanism:creative_fluid_tank", Family::Fluid, i32::MAX, i32::MAX) },
    tier("mekanism:basic_chemical_tank", Family::Chemical, 64 * B, B),
    tier("mekanism:advanced_chemical_tank", Family::Chemical, 256 * B, 16 * B),
    tier("mekanism:elite_chemical_tank", Family::Chemical, 1_024 * B, 128 * B),
    tier("mekanism:ultimate_chemical_tank", Family::Chemical, 8_192 * B, 512 * B),
    Tier { creative: true, ..tier("mekanism:creative_chemical_tank", Family::Chemical, i32::MAX, i32::MAX) },
];

pub fn tier_of(block: &str) -> Option<&'static Tier> {
    TANKS.iter().find(|t| t.block == block)
}

/// Whether a tank of `family` takes `fluid` (`native_block_entity_accepts_fluid`).
pub fn accepts(family: Family, fluid: &str) -> bool {
    chemicals::is_chemical(fluid) == (family == Family::Chemical)
}

/// Registered by `data/mekanism/horizon/menu/fluid_tank.json` / `chemical_tank.json`.
pub fn menu(family: Family) -> &'static str {
    match family {
        Family::Fluid => "mekanism:fluid_tank",
        Family::Chemical => "mekanism:chemical_tank",
    }
}

/// The tank's mode byte: for fluid tanks 0 idle, 1 active (push down); for
/// chemical tanks Mekanism's `GasMode` order: 0 idle, 1 dumping excess, 2 dumping.
pub fn modes(family: Family) -> u8 {
    match family {
        Family::Fluid => 2,
        Family::Chemical => 3,
    }
}

pub fn next_mode(family: Family, mode: u8, forward: bool) -> u8 {
    let n = modes(family);
    if forward { (mode + 1) % n } else { (mode + n - 1) % n }
}

pub fn mode_name(family: Family, mode: u8) -> &'static str {
    match (family, mode) {
        (Family::Fluid, 1) => "Auto-eject down: On",
        (Family::Fluid, _) => "Auto-eject down: Off",
        (Family::Chemical, 1) => "Dumping Excess",
        (Family::Chemical, 2) => "Dumping",
        (Family::Chemical, _) => "Idle",
    }
}

/// `ChemicalUtils.dump`: how much a chemical tank voids this tick.
pub fn dump(t: &Tier, mode: u8, amount: i32) -> i32 {
    if t.creative || amount <= 0 {
        return 0;
    }
    match mode {
        2 => (t.capacity / 400).min(amount),
        1 => {
            let keep = (t.capacity as i64 * 85 / 100) as i32;
            (amount - keep).clamp(0, t.rate)
        }
        _ => 0,
    }
}

/// Menu data slots (16-bit wire entries).
pub const DATA_AMOUNT: usize = 0; // two entries
pub const DATA_KIND: usize = 2; // `chemicals::index` + 1, 0 empty or unknown
pub const DATA_MODE: usize = 3;
pub const DATA_LEN: usize = 4;

pub fn menu_data(amount: i32, fluid: Option<&str>, mode: u8) -> [i32; DATA_LEN] {
    let [lo, hi] = cube::encode_amount(amount);
    let kind = fluid.filter(|_| amount > 0).and_then(chemicals::index).map_or(0, |i| i as i32 + 1);
    [lo, hi, kind, mode as i32]
}

/// Screen layout (176 x 166, inventory below).
pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;
pub const GAUGE: [f32; 4] = [8.0, 18.0, 18.0, 58.0];
pub const MODE_BUTTON: [f32; 4] = [32.0, 58.0, 136.0, 16.0];
/// Menu button ids of the mode button (left click forward, right click back).
pub const BUTTON_MODE_NEXT: i32 = 200;
pub const BUTTON_MODE_PREV: i32 = 201;

pub fn button_at(at: [f32; 2], right: bool) -> Option<i32> {
    crate::machine::inside(MODE_BUTTON, at).then_some(if right { BUTTON_MODE_PREV } else { BUTTON_MODE_NEXT })
}

/// Lines describing the contents, from the menu data.
pub fn contents_line(t: &Tier, data: &[i32]) -> String {
    let amount = cube::read_amount(data, DATA_AMOUNT).max(0);
    let kind = data.get(DATA_KIND).copied().unwrap_or(0);
    let name = (kind as usize).checked_sub(1).and_then(chemicals::entry).map_or("Empty", |e| e.0);
    if t.creative {
        return format!("{name}: Infinite");
    }
    format!("{name}: {amount} / {} mB", t.capacity)
}

pub fn tooltip(t: &Tier, data: &[i32], at: [f32; 2]) -> Vec<String> {
    if crate::machine::inside(GAUGE, at) {
        return vec![contents_line(t, data)];
    }
    if crate::machine::inside(MODE_BUTTON, at) {
        return vec!["Left: next  |  Right: previous".to_owned()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_match_mekanism() {
        let cap = |b: &str| tier_of(b).map(|t| (t.capacity, t.rate));
        assert_eq!(cap("mekanism:basic_fluid_tank"), Some((32_000, 1_000)));
        assert_eq!(cap("mekanism:ultimate_fluid_tank"), Some((256_000, 64_000)));
        assert_eq!(cap("mekanism:basic_chemical_tank"), Some((64_000, 1_000)));
        assert_eq!(cap("mekanism:ultimate_chemical_tank"), Some((8_192_000, 512_000)));
        assert!(tier_of("mekanism:creative_chemical_tank").is_some_and(|t| t.creative));
        assert!(tier_of("mekanism:basic_energy_cube").is_none());
    }

    #[test]
    fn families_refuse_each_other() {
        assert!(accepts(Family::Fluid, "minecraft:water"));
        assert!(accepts(Family::Fluid, "mekanism:brine"));
        assert!(!accepts(Family::Fluid, "mekanism:chemical/hydrogen"));
        assert!(accepts(Family::Chemical, "mekanism:chemical/hydrogen"));
        assert!(!accepts(Family::Chemical, "minecraft:lava"));
    }

    #[test]
    fn dumping_follows_chemical_utils() {
        let t = tier_of("mekanism:basic_chemical_tank").unwrap();
        assert_eq!(dump(t, 0, 50_000), 0);
        assert_eq!(dump(t, 2, 50_000), 160, "capacity / 400");
        assert_eq!(dump(t, 2, 100), 100, "never below empty");
        assert_eq!(dump(t, 1, 54_400), 0, "at 85 % keeps all");
        assert_eq!(dump(t, 1, 54_900), 500);
        assert_eq!(dump(t, 1, 64_000), 1_000, "at most the transfer rate");
        let c = tier_of("mekanism:creative_chemical_tank").unwrap();
        assert_eq!(dump(c, 2, 1_000), 0, "creative never dumps");
    }

    #[test]
    fn modes_cycle() {
        assert_eq!(next_mode(Family::Chemical, 2, true), 0);
        assert_eq!(next_mode(Family::Chemical, 0, false), 2);
        assert_eq!(next_mode(Family::Fluid, 1, true), 0);
    }

    #[test]
    fn menu_data_names_contents() {
        let t = tier_of("mekanism:basic_fluid_tank").unwrap();
        let d = menu_data(12_000, Some("mekanism:brine"), 1);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(contents_line(t, &d), "Brine: 12000 / 32000 mB");
        assert_eq!(menu_data(0, Some("mekanism:brine"), 0)[DATA_KIND], 0, "empty shows no kind");
        let u = tier_of("mekanism:ultimate_chemical_tank").unwrap();
        let d = menu_data(8_192_000, Some("mekanism:chemical/oxygen"), 0);
        assert_eq!(cube::read_amount(&d, DATA_AMOUNT), 8_192_000);
        assert!(contents_line(u, &d).starts_with("Oxygen: "));
    }

    #[test]
    fn mode_button_hits() {
        assert_eq!(button_at([40.0, 60.0], false), Some(BUTTON_MODE_NEXT));
        assert_eq!(button_at([40.0, 60.0], true), Some(BUTTON_MODE_PREV));
        assert_eq!(button_at([2.0, 2.0], false), None);
    }
}
