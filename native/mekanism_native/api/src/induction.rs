//! Induction Matrix: the parts that need no Horizon types, so their tests run
//! in `logic/energy`. Hooks are in `inductions.rs`, the structure itself in
//! `multiblock.rs`/`multiblocks.rs`.
//!
//! Values from Mekanism 26.3:
//! - `InductionCellTier` capacity: Basic 8,000,000,000, Advanced 64,000,000,000,
//!   Elite 512,000,000,000, Ultimate 4,000,000,000,000;
//! - `InductionProviderTier` output per tick: Basic 256,000, Advanced 2,048,000,
//!   Elite 16,384,000, Ultimate 131,072,000;
//! - `MatrixEnergyContainer`: capacity = sum of the cells (`MathUtils.addClamped`),
//!   transfer cap = sum of the providers; `MatrixValidator.postcheck`: at least one
//!   cell and one provider (`MATRIX_NO_CELLS`, `MATRIX_NO_PROVIDERS`).
//!
//! Energy is FE (docs/ENERGY-UNITS.md). The matrix counts in i64, the master block
//! keeps the stored energy, but every cell is above `i32::MAX` and Horizon's energy
//! buffers are i32: the ports' own buffers (what cables and neighbours see) hold at
//! most `PORT_BUFFER` and the screen shows the real i64 amounts. In Mekanism the
//! energy sits in the cells (`shouldCache(ENERGY)` is false); here the master keeps
//! it, so it is lost if the master block is destroyed with no neighbouring casing.
//!
//! Not ported: the energy item slots, the per-port share being the real (shared)
//! transfer cap: each port moves at most `transfer cap / ports` per tick.

use crate::multiblock::{Def, Type};

pub const CASING: &str = "mekanism:induction_casing";
pub const PORT: &str = "mekanism:induction_port";

/// `(block, capacity)`.
pub const CELLS: [(&str, i64); 4] = [
    ("mekanism:basic_induction_cell", 8_000_000_000),
    ("mekanism:advanced_induction_cell", 64_000_000_000),
    ("mekanism:elite_induction_cell", 512_000_000_000),
    ("mekanism:ultimate_induction_cell", 4_000_000_000_000),
];
/// `(block, output per tick)`.
pub const PROVIDERS: [(&str, i64); 4] = [
    ("mekanism:basic_induction_provider", 256_000),
    ("mekanism:advanced_induction_provider", 2_048_000),
    ("mekanism:elite_induction_provider", 16_384_000),
    ("mekanism:ultimate_induction_provider", 131_072_000),
];

pub const DEF: Def = Def {
    ty: Type::Matrix,
    rules: crate::multiblock::MATRIX,
    frames: &[CASING],
    plain: &[],
    valves: &[PORT],
    controllers: &[],
    interior: &[
        CELLS[0].0,
        CELLS[1].0,
        CELLS[2].0,
        CELLS[3].0,
        PROVIDERS[0].0,
        PROVIDERS[1].0,
        PROVIDERS[2].0,
        PROVIDERS[3].0,
    ],
};

/// What a port's own Horizon energy buffer holds at most (the largest provider's output).
pub const PORT_BUFFER: i32 = 131_072_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub cells: u32,
    pub providers: u32,
    pub capacity: i64,
    pub transfer: i64,
}

pub fn totals<'a>(blocks: impl IntoIterator<Item = &'a str>) -> Totals {
    let mut t = Totals::default();
    for b in blocks {
        if let Some((_, c)) = CELLS.iter().find(|(id, _)| *id == b) {
            t.cells += 1;
            t.capacity = t.capacity.saturating_add(*c);
        } else if let Some((_, o)) = PROVIDERS.iter().find(|(id, _)| *id == b) {
            t.providers += 1;
            t.transfer = t.transfer.saturating_add(*o);
        }
    }
    t
}

/// `MatrixValidator.postcheck`.
pub fn check(t: &Totals) -> Result<(), &'static str> {
    if t.cells == 0 {
        Err("no induction cells")
    } else if t.providers == 0 {
        Err("no induction providers")
    } else {
        Ok(())
    }
}

/// Horizon's energy is i32.
pub fn horizon(v: i64) -> i32 {
    v.clamp(0, i32::MAX as i64) as i32
}

/// Each port's per-tick share of the transfer cap.
pub fn share(transfer: i64, ports: i64) -> i64 {
    transfer / ports.max(1)
}

/// An input port: its buffer flows into the matrix, at most `share` and what fits.
/// Returns (matrix energy, port buffer).
pub fn take_in(stored: i64, capacity: i64, buffer: i32, share: i64) -> (i64, i32) {
    let moved = (buffer as i64).min(share).min((capacity - stored).max(0)).max(0);
    (stored + moved, buffer - moved as i32)
}

/// An output port: its buffer is refilled from the matrix, at most `share` and what fits.
pub fn fill_out(stored: i64, buffer: i32, share: i64) -> (i64, i32) {
    let moved = stored.min(share).min((PORT_BUFFER - buffer).max(0) as i64).max(0);
    (stored - moved, buffer + moved as i32)
}

/// Port modes (`TileEntityInductionPort` input / output).
pub fn mode_name(output: bool) -> &'static str {
    if output { "Output" } else { "Input" }
}

/// Menu data: energy (3 entries of 16 bits), capacity (3), transfer (3), cells,
/// providers, port mode, formed.
pub const DATA_ENERGY: usize = 0;
pub const DATA_CAPACITY: usize = 3;
pub const DATA_TRANSFER: usize = 6;
pub const DATA_CELLS: usize = 9;
pub const DATA_PROVIDERS: usize = 10;
pub const DATA_MODE: usize = 11;
pub const DATA_FORMED: usize = 12;
pub const DATA_LEN: usize = 13;

pub fn menu_data(energy: i64, t: &Totals, output: bool, formed: bool) -> [i32; DATA_LEN] {
    let [e0, e1, e2] = crate::dynamic_tank::encode_long(energy);
    let [c0, c1, c2] = crate::dynamic_tank::encode_long(t.capacity);
    let [t0, t1, t2] = crate::dynamic_tank::encode_long(t.transfer);
    [e0, e1, e2, c0, c1, c2, t0, t1, t2, t.cells.min(0xFFFF) as i32, t.providers.min(0xFFFF) as i32, output as i32, formed as i32]
}

pub const BUTTON_MODE: i32 = 210;

/// The port's mode button on its screen.
pub const MODE_BUTTON: [f32; 4] = [8.0, 58.0, 160.0, 16.0];

pub fn button_at(at: [f32; 2]) -> Option<i32> {
    crate::machine::inside(MODE_BUTTON, at).then_some(BUTTON_MODE)
}

pub fn energy_text(fe: i64) -> String {
    format!("{} FE", crate::cube::si(fe))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiblock::Cell;

    #[test]
    fn totals_sum_cells_and_providers() {
        let t = totals([
            "mekanism:basic_induction_cell",
            "mekanism:ultimate_induction_cell",
            "mekanism:elite_induction_provider",
            "mekanism:basic_induction_provider",
            "minecraft:stone",
        ]);
        assert_eq!(t, Totals { cells: 2, providers: 2, capacity: 4_008_000_000_000, transfer: 16_640_000 });
        assert_eq!(check(&t), Ok(()));
    }

    #[test]
    fn needs_a_cell_and_a_provider() {
        assert_eq!(check(&totals(["mekanism:basic_induction_provider"])), Err("no induction cells"));
        assert_eq!(check(&totals(["mekanism:basic_induction_cell"])), Err("no induction providers"));
    }

    #[test]
    fn capacity_saturates_instead_of_wrapping() {
        let many = std::iter::repeat("mekanism:ultimate_induction_cell").take(3_000_000);
        let t = totals(many);
        assert_eq!(t.capacity, 12_000_000_000_000_000_000_i128.min(i64::MAX as i128) as i64);
    }

    #[test]
    fn every_cell_exceeds_the_i32_limit() {
        assert!(CELLS.iter().all(|(_, c)| *c > i32::MAX as i64));
        assert_eq!(horizon(CELLS[0].1), i32::MAX);
        assert_eq!(horizon(-5), 0);
    }

    #[test]
    fn input_port_fills_the_matrix_within_its_share() {
        assert_eq!(take_in(0, 1_000, 400, 1_000), (400, 0));
        assert_eq!(take_in(0, 1_000, 400, 100), (100, 300), "share limits");
        assert_eq!(take_in(900, 1_000, 400, 1_000), (1_000, 300), "capacity limits");
        assert_eq!(take_in(1_000, 1_000, 400, 1_000), (1_000, 400), "full matrix refuses");
    }

    #[test]
    fn output_port_refills_its_buffer() {
        assert_eq!(fill_out(5_000, 0, 1_000), (4_000, 1_000));
        assert_eq!(fill_out(500, 0, 1_000), (0, 500));
        assert_eq!(fill_out(5_000, PORT_BUFFER - 10, 1_000), (4_990, PORT_BUFFER));
        assert_eq!(fill_out(5_000, PORT_BUFFER, 1_000), (5_000, PORT_BUFFER));
    }

    #[test]
    fn shares_split_the_transfer_cap() {
        assert_eq!(share(16_640_000, 4), 4_160_000);
        assert_eq!(share(100, 0), 100);
    }

    #[test]
    fn menu_data_round_trips_trillions() {
        let t = Totals { cells: 3, providers: 2, capacity: 12_000_000_000_000, transfer: 262_144_000 };
        let d = menu_data(9_876_543_210_000, &t, true, true);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(crate::dynamic_tank::read_long(&d, DATA_ENERGY), 9_876_543_210_000);
        assert_eq!(crate::dynamic_tank::read_long(&d, DATA_CAPACITY), 12_000_000_000_000);
        assert_eq!(crate::dynamic_tank::read_long(&d, DATA_TRANSFER), 262_144_000);
        assert_eq!((d[DATA_CELLS], d[DATA_PROVIDERS], d[DATA_MODE], d[DATA_FORMED]), (3, 2, 1, 1));
    }

    #[test]
    fn def_classifies_the_matrix_blocks() {
        assert_eq!(DEF.classify(CASING), Cell::Frame);
        assert_eq!(DEF.classify(PORT), Cell::Valve);
        assert_eq!(DEF.classify("mekanism:elite_induction_cell"), Cell::Interior);
        assert_eq!(DEF.classify("mekanism:ultimate_induction_provider"), Cell::Interior);
        assert!(DEF.is_member(PORT) && DEF.is_member(CASING) && !DEF.is_member("mekanism:basic_induction_cell"));
    }

    #[test]
    fn a_whole_matrix_validates_through_the_shared_validator() {
        use crate::multiblock::{validate, Outcome};
        use std::collections::HashMap;
        let mut w: HashMap<[i32; 3], &str> = HashMap::new();
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..5 {
                    let e = [x, y, z].iter().filter(|&&v| v == 0 || v == 4).count();
                    w.insert([x, y, z], if e >= 1 { CASING } else { "minecraft:air" });
                }
            }
        }
        w.insert([0, 2, 2], PORT);
        w.insert([1, 1, 1], "mekanism:basic_induction_cell");
        w.insert([2, 1, 1], "mekanism:basic_induction_provider");
        let o = validate([0, 0, 0], &DEF.rules, &mut |p| DEF.classify(w.get(&p).copied().unwrap_or("minecraft:air")));
        let Outcome::Formed(s) = o else { panic!("{o:?}") };
        assert_eq!(s.valves, vec![[0, 2, 2]]);
        let t = totals(s.interior.iter().map(|p| w[p]));
        assert_eq!((t.cells, t.providers), (1, 1));
        assert_eq!(check(&t), Ok(()));
    }
}
