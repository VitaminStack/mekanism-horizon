//! Bins (M08), the parts that need no Horizon types, so their tests run in
//! `logic/energy`. The host side (tick, hoppers, clicks, render) is `bins.rs`.
//!
//! Values from Mekanism 26.3: `BinTier` (capacity in items), `TileEntityBin`
//! (one item type, auto-eject of one stack every half second into the block
//! below while active, a lock that keeps the type when the bin is empty) and
//! `BlockBin` (right click inserts the held stack, left click on the front takes
//! one item, or a stack when sneaking).
//!
//! A bin holds one item type and a count. Horizon's slots stop at 99 items, so
//! the count is a number in the block-entity NBT; two real slots only serve
//! automation: an input slot a hopper fills and the bin absorbs, and an output
//! window the bin keeps filled with up to one stack for a hopper to take.
//! The creative bin never runs dry nor fills; its capacity is `i32::MAX`
//! (Mekanism's `Long.MAX_VALUE`; Horizon's item counts are 32-bit).
//!
//! Not ported: items with components or damage (enchantments, custom names,
//! ...) are refused, as is the redstone/side config of Mekanism's bin.

#[derive(Clone, Copy, Debug)]
pub struct Tier {
    pub block: &'static str,
    pub capacity: i32,
    pub creative: bool,
}

pub const BINS: [Tier; 5] = [
    Tier { block: "mekanism:basic_bin", capacity: 4_096, creative: false },
    Tier { block: "mekanism:advanced_bin", capacity: 8_192, creative: false },
    Tier { block: "mekanism:elite_bin", capacity: 32_768, creative: false },
    Tier { block: "mekanism:ultimate_bin", capacity: 262_144, creative: false },
    Tier { block: "mekanism:creative_bin", capacity: i32::MAX, creative: true },
];

pub fn tier_of(block: &str) -> Option<&'static Tier> {
    BINS.iter().find(|t| t.block == block)
}

/// Slots of the block's container: the window a hopper takes from, the input a hopper fills.
pub const WINDOW: usize = 0;
pub const INPUT: usize = 1;
pub const SLOTS: usize = 2;

/// Ticks between two auto-ejects (`MekanismUtils.TICKS_PER_HALF_SECOND`).
pub const EJECT_TICKS: u8 = 10;

/// What a bin holds: the item (kept while locked even when empty), how many,
/// and whether the type is locked. The count excludes the output window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contents {
    pub item: Option<String>,
    pub count: i32,
    pub locked: bool,
}

impl Contents {
    /// An empty, unlocked bin forgets its type.
    fn normalise(&mut self) {
        if self.count <= 0 && !self.locked {
            self.count = 0;
            self.item = None;
        }
    }

    pub fn accepts(&self, item: &str) -> bool {
        self.item.as_deref().is_none_or(|have| have == item)
    }

    /// Puts up to `count` of `item` in; returns how many went in.
    pub fn insert(&mut self, t: &Tier, item: &str, count: u32) -> u32 {
        if !self.accepts(item) || count == 0 {
            return 0;
        }
        let room = (t.capacity - self.count).max(0) as u32;
        let moved = count.min(room);
        if t.creative {
            // Takes the type and is full from then on; nothing of the offer is used up.
            self.item = Some(item.to_owned());
            self.count = t.capacity;
            return 0;
        }
        if moved > 0 {
            self.item = Some(item.to_owned());
            self.count += moved as i32;
        }
        moved
    }

    /// Takes up to `count` out; returns how many came out. The creative bin
    /// gives all that is asked and keeps its count.
    pub fn extract(&mut self, t: &Tier, count: u32) -> u32 {
        if self.item.is_none() {
            return 0;
        }
        if t.creative {
            return count;
        }
        let moved = count.min(self.count.max(0) as u32);
        self.count -= moved as i32;
        self.normalise();
        moved
    }

    /// Toggles the lock (`TileEntityBin.toggleLock`): not for the creative bin,
    /// and an empty bin has no type to lock. Returns whether it changed.
    pub fn toggle_lock(&mut self, t: &Tier) -> bool {
        if t.creative || self.item.is_none() {
            return false;
        }
        self.locked = !self.locked;
        self.normalise();
        true
    }
}

/// How many to take from the bin to top the output window up to `max_stack`.
pub fn refill(window: u32, max_stack: u32) -> u32 {
    max_stack.saturating_sub(window)
}

/// A stack the bin can store: Horizon's item counts of plain items only.
pub fn storable(damage: u32, has_components: bool) -> bool {
    damage == 0 && !has_components
}

/// What a left click takes: one item, a stack when sneaking.
pub fn click_amount(sneaking: bool, max_stack: u32) -> u32 {
    if sneaking { max_stack.max(1) } else { 1 }
}

/// `minecraft:iron_ingot` as "Iron Ingot".
pub fn pretty_name(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    path.split(['_', '/'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The bin's description, for the item tooltip and the front label.
pub fn label(t: &Tier, c: &Contents, extra: i32) -> String {
    let Some(item) = c.item.as_deref() else { return "Empty".to_owned() };
    let total = if t.creative { "Infinite".to_owned() } else { (c.count + extra).to_string() };
    let lock = if c.locked { " (locked)" } else { "" };
    format!("{}: {total}{lock}", pretty_name(item))
}

/// The front's short count text: `1234`, `12.3K`, `1.2M`.
pub fn short_count(count: i64) -> String {
    match count {
        i64::MAX.. => "inf".to_owned(),
        c if c >= 1_000_000 => format!("{:.1}M", c as f64 / 1_000_000.0),
        c if c >= 10_000 => format!("{:.1}K", c as f64 / 1_000.0),
        c => c.to_string(),
    }
}

/// Creative bins count as full; `i32::MAX` is shown as infinite.
pub fn front_count(t: &Tier, total: i32) -> i64 {
    if t.creative { i64::MAX } else { total as i64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tier(block: &str) -> &'static Tier {
        tier_of(block).unwrap()
    }

    #[test]
    fn capacities_match_bin_tier() {
        let cap = |b: &str| tier(b).capacity;
        assert_eq!(cap("mekanism:basic_bin"), 4_096);
        assert_eq!(cap("mekanism:advanced_bin"), 8_192);
        assert_eq!(cap("mekanism:elite_bin"), 32_768);
        assert_eq!(cap("mekanism:ultimate_bin"), 262_144);
        assert!(tier("mekanism:creative_bin").creative);
        assert!(tier_of("mekanism:basic_fluid_tank").is_none());
    }

    #[test]
    fn insert_stops_at_capacity_and_type() {
        let t = tier("mekanism:basic_bin");
        let mut c = Contents::default();
        assert_eq!(c.insert(t, "minecraft:cobblestone", 3_000), 3_000);
        assert_eq!(c.insert(t, "minecraft:dirt", 1), 0, "one type only");
        assert_eq!(c.insert(t, "minecraft:cobblestone", 3_000), 1_096, "only what fits");
        assert_eq!(c.count, 4_096);
        assert_eq!(c.insert(t, "minecraft:cobblestone", 1), 0, "full");
    }

    #[test]
    fn empty_bin_forgets_its_type_unless_locked() {
        let t = tier("mekanism:advanced_bin");
        let mut c = Contents::default();
        c.insert(t, "minecraft:coal", 10);
        assert_eq!(c.extract(t, 100), 10);
        assert_eq!(c.item, None);
        assert!(c.accepts("minecraft:dirt"));

        c.insert(t, "minecraft:coal", 10);
        assert!(c.toggle_lock(t));
        assert_eq!(c.extract(t, 10), 10);
        assert_eq!(c.item.as_deref(), Some("minecraft:coal"), "the lock keeps the type");
        assert!(!c.accepts("minecraft:dirt"));
        assert!(c.toggle_lock(t), "unlock");
        assert_eq!(c.item, None, "empty and unlocked: type gone");
    }

    #[test]
    fn empty_bin_cannot_lock() {
        let t = tier("mekanism:basic_bin");
        assert!(!Contents::default().toggle_lock(t));
        let mut c = Contents::default();
        let creative = tier("mekanism:creative_bin");
        c.insert(creative, "minecraft:stone", 1);
        assert!(!c.toggle_lock(creative), "no lock on the creative bin");
    }

    #[test]
    fn creative_bin_is_infinite() {
        let t = tier("mekanism:creative_bin");
        let mut c = Contents::default();
        assert_eq!(c.extract(t, 64), 0, "nothing without a type");
        assert_eq!(c.insert(t, "minecraft:stone", 1), 0, "sets the type, uses nothing up");
        assert_eq!(c.count, i32::MAX);
        assert_eq!(c.extract(t, 64), 64);
        assert_eq!(c.count, i32::MAX);
        assert_eq!(label(t, &c, 0), "Stone: Infinite");
    }

    #[test]
    fn window_refill_tops_up_to_one_stack() {
        assert_eq!(refill(0, 64), 64);
        assert_eq!(refill(40, 64), 24);
        assert_eq!(refill(64, 64), 0);
        assert_eq!(refill(16, 16), 0, "16-stack items too");
    }

    #[test]
    fn components_are_refused() {
        assert!(storable(0, false));
        assert!(!storable(5, false));
        assert!(!storable(0, true));
    }

    #[test]
    fn clicks_take_one_or_a_stack() {
        assert_eq!(click_amount(false, 64), 1);
        assert_eq!(click_amount(true, 64), 64);
        assert_eq!(click_amount(true, 16), 16);
    }

    #[test]
    fn names_and_counts_read_well() {
        assert_eq!(pretty_name("minecraft:iron_ingot"), "Iron Ingot");
        assert_eq!(pretty_name("mekanism:dust_osmium"), "Dust Osmium");
        assert_eq!(short_count(12), "12");
        assert_eq!(short_count(12_345), "12.3K");
        assert_eq!(short_count(262_144), "262.1K");
        assert_eq!(short_count(2_500_000), "2.5M");
        let t = tier("mekanism:basic_bin");
        let mut c = Contents::default();
        c.insert(t, "minecraft:cobblestone", 100);
        assert_eq!(label(t, &c, 64), "Cobblestone: 164");
        c.locked = true;
        assert_eq!(label(t, &c, 0), "Cobblestone: 100 (locked)");
        assert_eq!(label(t, &Contents::default(), 0), "Empty");
    }
}
