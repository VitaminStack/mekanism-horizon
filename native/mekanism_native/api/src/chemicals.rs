//! Mekanism's fluids and chemicals as Horizon fluid ids (M06 groundwork).
//!
//! Horizon has no chemical registry and only vanilla world fluids, but its
//! block-entity tanks (API 54, `mod_fluids`) store any namespaced fluid id and
//! save it with the entity. So both kinds live in tanks as ids:
//! - fluids as `mekanism:<id>` (`mekanism:brine`), plus vanilla water/lava;
//! - chemicals as `mekanism:chemical/<id>` (`mekanism:chemical/hydrogen`), so a
//!   chemical never merges with the liquid of the same name, and a fluid tank
//!   can refuse chemicals (and a chemical tank fluids) by the id alone.
//!
//! The tables come from Mekanism 26.3 (`scripts/gen_chemicals.py`). Horizon
//! syncs block-entity NBT and menu data itself, so there is no extra network or
//! datapack registry to keep in step; ids unknown here (another mod's fluid, or
//! a chemical removed upstream) still move and save, they just show their id.

include!("chemicals_table.rs");

pub const CHEMICAL_PREFIX: &str = "mekanism:chemical/";
pub const WATER: &str = "minecraft:water";
pub const LAVA: &str = "minecraft:lava";
const WATER_COLOR: u32 = 0xFF3F76E4;
const LAVA_COLOR: u32 = 0xFFE25A1C;

/// Whether a tank id is a chemical (Mekanism's chemical tanks and gauges) rather than a fluid.
pub fn is_chemical(id: &str) -> bool {
    id.starts_with(CHEMICAL_PREFIX)
}

/// The tank id of chemical `path` (`hydrogen` -> `mekanism:chemical/hydrogen`).
pub fn chemical(path: &str) -> String {
    format!("{CHEMICAL_PREFIX}{path}")
}

/// The tank id of a recipe's chemical (`mekanism:hydrogen` -> `mekanism:chemical/hydrogen`);
/// Mekanism Generators' chemicals (`mekanismgenerators:tritium`) get one the same way. None for an
/// id of another namespace.
pub fn from_recipe(id: &str) -> Option<String> {
    let path = id.strip_prefix("mekanism:").or_else(|| id.strip_prefix("mekanismgenerators:"))?;
    (!path.is_empty() && !path.contains(':')).then(|| chemical(path))
}

/// Index of `id` in `known()` order: water, lava, `FLUIDS`, then `CHEMICALS`.
/// Menu data carries `index + 1` (0 for empty or unknown), which fits 16 bits.
pub fn index(id: &str) -> Option<usize> {
    match id {
        WATER => return Some(0),
        LAVA => return Some(1),
        _ => {}
    }
    if let Some(path) = id.strip_prefix(CHEMICAL_PREFIX) {
        return CHEMICALS.iter().position(|c| c.0 == path).map(|i| 2 + FLUIDS.len() + i);
    }
    let path = id.strip_prefix("mekanism:")?;
    FLUIDS.iter().position(|f| f.0 == path).map(|i| 2 + i)
}

/// The tank id at `index` (`index`'s inverse).
pub fn id_at(index: usize) -> Option<String> {
    match index {
        0 => Some(WATER.to_owned()),
        1 => Some(LAVA.to_owned()),
        i if i < 2 + FLUIDS.len() => FLUIDS.get(i - 2).map(|f| format!("mekanism:{}", f.0)),
        i => CHEMICALS.get(i - 2 - FLUIDS.len()).map(|c| chemical(c.0)),
    }
}

/// Name and colour of the entry at `index` (see `index`).
pub fn entry(index: usize) -> Option<(&'static str, u32)> {
    match index {
        0 => Some(("Water", WATER_COLOR)),
        1 => Some(("Lava", LAVA_COLOR)),
        i if i < 2 + FLUIDS.len() => FLUIDS.get(i - 2).map(|f| (f.1, f.2)),
        i => CHEMICALS.get(i - 2 - FLUIDS.len()).map(|c| (c.1, c.2)),
    }
}

/// The English name shown for a tank id; the id itself when unknown.
pub fn name(id: &str) -> String {
    index(id).and_then(entry).map_or_else(|| id.to_owned(), |e| e.0.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluids_and_chemicals_stay_apart() {
        assert!(is_chemical(&chemical("hydrogen")));
        assert!(!is_chemical("mekanism:hydrogen"));
        assert_ne!(index("mekanism:hydrogen"), index(&chemical("hydrogen")));
        assert_eq!(name("mekanism:hydrogen"), "Liquid Hydrogen");
        assert_eq!(name(&chemical("hydrogen")), "Hydrogen");
    }

    #[test]
    fn recipe_ids_become_tank_ids() {
        assert_eq!(from_recipe("mekanism:brine").as_deref(), Some("mekanism:chemical/brine"));
        assert_eq!(from_recipe("mekanismgenerators:tritium").as_deref(), Some("mekanism:chemical/tritium"));
        assert_eq!(from_recipe("create:brine"), None);
        assert_eq!(from_recipe("mekanism:"), None);
    }

    #[test]
    fn index_round_trips() {
        for id in [WATER, LAVA, "mekanism:brine", "mekanism:heavy_water", "mekanism:chemical/oxygen",
                   "mekanism:chemical/clean_iron", "mekanism:chemical/aqua"] {
            let i = index(id).unwrap_or_else(|| panic!("{id} known"));
            assert!(entry(i).is_some(), "{id}");
            assert_eq!(id_at(i).as_deref(), Some(id), "{id}");
            assert!(i + 1 <= 0xFFFF);
        }
        assert_eq!(index("create:chocolate"), None);
        assert_eq!(index("mekanism:chemical/empty"), None);
        assert_eq!(name("create:chocolate"), "create:chocolate");
    }

    #[test]
    fn tables_hold_mekanisms_ids() {
        for f in ["brine", "steam", "lithium", "heavy_water", "nutritional_paste", "sodium"] {
            assert!(FLUIDS.iter().any(|e| e.0 == f), "fluid {f}");
        }
        for c in ["hydrogen", "oxygen", "chlorine", "sulfur_dioxide", "ethene", "dirty_iron",
                  "clean_gold", "white", "polonium", "antimatter", "water_vapor", "tritium", "deuterium", "fusion_fuel"] {
            assert!(CHEMICALS.iter().any(|e| e.0 == c), "chemical {c}");
        }
        // Ids must be valid Horizon tank ids (lowercase, digits, `_`).
        for (id, ..) in FLUIDS.iter().chain(CHEMICALS.iter()) {
            assert!(id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'), "{id}");
        }
    }
}
