//! Energy between Mekanism blocks: which blocks take it, and how much one push
//! moves. No Horizon types, so the tests run in `logic/energy`.
//!
//! Unit: amounts are Horizon's energy (i32, shown as FE) and carry Mekanism's
//! joule values one to one (see docs/ENERGY-UNITS.md). Every Mekanism 26.3
//! value ported so far fits i32 except the creative cube's `Long.MAX_VALUE`,
//! which is capped at `i32::MAX`. Sums are bounded by the taker's capacity, so
//! none overflows: `accept` never returns more than the room left.
//!
//! Horizon gives a trait mod no energy access to other mods' blocks (see
//! docs/upstream/horizon-energy-host.md), so pushes reach only the blocks
//! `acceptor` knows.

use super::{cube, machine};

/// A block that takes energy from a neighbouring Mekanism block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acceptor {
    pub capacity: i32,
    /// The most it takes in one call.
    pub rate: i32,
    /// Takes energy only through the sides its side config sets to input
    /// (cubes); the others take it on every side.
    pub sided: bool,
}

pub fn acceptor(block: &str) -> Option<Acceptor> {
    if let Some((capacity, rate)) = cube::tier(block) {
        return Some(Acceptor { capacity, rate, sided: true });
    }
    // Machines take up to their whole buffer per call (no side config ported).
    machine::find(block).map(|m| Acceptor { capacity: m.capacity(), rate: m.capacity(), sided: false })
}

impl Acceptor {
    /// How much of `budget` (what the giver may still send this tick) and
    /// `stored` (what it holds) moves into this block holding `theirs`.
    /// Never negative, never more than the room left.
    pub fn accept(&self, budget: i32, stored: i32, theirs: i32) -> i32 {
        cube::transfer(budget, stored, theirs, self.capacity, self.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubes_are_sided_and_limited_by_their_tier() {
        let a = acceptor("mekanism:basic_energy_cube").unwrap();
        assert_eq!(a, Acceptor { capacity: 4_000_000, rate: 4_000, sided: true });
        assert_eq!(a.accept(1_000_000, 1_000_000, 0), 4_000);
        assert_eq!(a.accept(1_000_000, 1_000_000, 3_999_000), 1_000);
    }

    #[test]
    fn machines_take_their_whole_buffer_on_any_side() {
        for m in &machine::MACHINES {
            let a = acceptor(m.block).unwrap();
            assert_eq!(a, Acceptor { capacity: m.capacity(), rate: m.capacity(), sided: false });
            assert_eq!(a.accept(i32::MAX, i32::MAX, 0), m.capacity());
            assert_eq!(a.accept(i32::MAX, i32::MAX, m.capacity()), 0);
        }
    }

    #[test]
    fn other_blocks_take_nothing() {
        assert!(acceptor("minecraft:furnace").is_none());
        assert!(acceptor("mekanismgenerators:heat_generator").is_none());
    }

    /// The creative cube at `i32::MAX`, and saved amounts outside the buffer.
    #[test]
    fn extreme_amounts_never_overflow() {
        let creative = acceptor(cube::CREATIVE).unwrap();
        assert_eq!(creative.accept(i32::MAX, i32::MAX, i32::MAX), 0);
        assert_eq!(creative.accept(i32::MAX, i32::MAX, i32::MAX - 5), 5);
        assert_eq!(creative.accept(i32::MAX, i32::MAX, 0), i32::MAX);
        let basic = acceptor("mekanism:basic_energy_cube").unwrap();
        assert_eq!(basic.accept(i32::MAX, i32::MAX, i32::MAX), 0, "more saved than the buffer holds");
        assert_eq!(basic.accept(i32::MAX, i32::MAX, i32::MIN), 4_000, "negative saved amount");
        assert_eq!(basic.accept(-1, 100, 0), 0);
        assert_eq!(basic.accept(100, -1, 0), 0);
        for (block, capacity, _) in cube::TIERS {
            let a = acceptor(block).unwrap();
            for theirs in [0, 1, capacity / 2, capacity - 1, capacity] {
                let moved = a.accept(i32::MAX, i32::MAX, theirs);
                assert!(moved >= 0 && theirs.checked_add(moved).is_some_and(|sum| sum <= capacity), "{block} {theirs}");
            }
        }
    }
}
