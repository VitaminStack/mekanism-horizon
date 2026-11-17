//! Dynamic Tank and Dynamic Valve (and the Structural Glass block, which
//! `TankValidator` and `MatrixValidator` both treat as INVALID casing, so it
//! is registered but joins no structure here): the parts that need no
//! Horizon types, so their tests run in `logic/energy`. Hooks are in
//! `dynamic_tanks.rs`, the structure itself in `multiblock.rs`/`multiblocks.rs`.
//!
//! Values from Mekanism 26.3:
//! - `TankMultiblockData.setVolume`: capacity = volume x `dynamicTankFluidPerTank`
//!   (`GeneralConfig`: `16_000L * FluidType.BUCKET_VOLUME` = 16,000,000 mB), the
//!   volume being the whole bounding box including the walls (`Structure.getVolume`);
//! - `GeneralConfig` auto eject: 1,024 mB per tick for fluids and chemicals;
//! - `TankMultiblockData.DEFAULT_EDIT_MODE` = `ContainerEditMode.BOTH` (BOTH, FILL, EMPTY).
//!
//! The contents are one fluid or chemical (a `chemicals::CHEMICAL_PREFIX` id as in
//! `tank.rs`), kept as an i64 by the structure's master block. Horizon's own tanks
//! hold at most 1,000,000 mB each, so a valve exposes a small buffer tank that it
//! empties into the master and refills from it every tick.
//!
//! Not ported: the item input/output slots (`HybridInventorySlot`), the redstone
//! output by fill level, the gauge's scale animation, computer methods, the
//! tank's own contents in Horizon's i32 tanks: a valve moves at most its buffer
//! per tick, but the master counts in i64 so the full capacity is real.

use crate::chemicals;
use crate::multiblock::{Def, Type};

pub const TANK: &str = "mekanism:dynamic_tank";
pub const VALVE: &str = "mekanism:dynamic_valve";
pub const GLASS: &str = "mekanism:structural_glass";

pub const DEF: Def = Def {
    ty: Type::Tank,
    rules: crate::multiblock::TANK,
    frames: &[TANK],
    plain: &[],
    valves: &[VALVE],
    controllers: &[],
    interior: &[],
};

/// `dynamicTankFluidPerTank`: mB of capacity per block of the bounding box.
pub const PER_VOLUME: i64 = 16_000 * 1_000;
/// `fluidAutoEjectRate` / `chemicalAutoEjectRate`.
pub const EJECT_RATE: i32 = 1_024;
/// A valve's own Horizon tank: what pipes can put in or take out between ticks.
pub const BUFFER: i32 = 32_000;

pub fn capacity(volume: i64) -> i64 {
    volume.saturating_mul(PER_VOLUME)
}

/// `ContainerEditMode`: which way the valves move fluid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Both,
    Fill,
    Empty,
}

impl Mode {
    pub fn from_byte(b: u8) -> Mode {
        match b {
            1 => Mode::Fill,
            2 => Mode::Empty,
            _ => Mode::Both,
        }
    }

    pub fn byte(self) -> u8 {
        self as u8
    }

    pub fn next(self, forward: bool) -> Mode {
        Mode::from_byte(((self as u8) + if forward { 1 } else { 2 }) % 3)
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Both => "Both",
            Mode::Fill => "Fill",
            Mode::Empty => "Empty",
        }
    }
}

/// A tank's contents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    pub fluid: Option<String>,
    pub amount: i64,
}

impl Stored {
    pub fn new(fluid: Option<String>, amount: i64) -> Stored {
        if amount > 0 && fluid.is_some() { Stored { fluid, amount } } else { Stored::default() }
    }

    fn take(&mut self, n: i64) {
        self.amount -= n;
        if self.amount <= 0 {
            *self = Stored::default();
        }
    }
}

/// One valve tick between its buffer (`buffer_capacity` mB at most) and the
/// master's tank. In `Fill`/`Both` the buffer's contents go into the master
/// (whatever fits, and only the master's own fluid); in `Empty`/`Both` the buffer
/// is refilled from the master. Mekanism's `ContainerEditMode.BOTH` lets a valve
/// do either; one tick moves in first, then out.
pub fn exchange(buffer: &mut Stored, tank: &mut Stored, capacity: i64, buffer_capacity: i32, mode: Mode) {
    if mode != Mode::Empty && buffer.amount > 0 {
        let same = tank.amount == 0 || tank.fluid == buffer.fluid;
        if same {
            let moved = buffer.amount.min(capacity - tank.amount).max(0);
            if moved > 0 {
                if tank.amount == 0 {
                    tank.fluid = buffer.fluid.clone();
                }
                tank.amount += moved;
                buffer.take(moved);
            }
        }
    }
    if mode != Mode::Fill && tank.amount > 0 {
        let same = buffer.amount == 0 || buffer.fluid == tank.fluid;
        if same {
            let moved = tank.amount.min(buffer_capacity as i64 - buffer.amount).max(0);
            if moved > 0 {
                if buffer.amount == 0 {
                    buffer.fluid = tank.fluid.clone();
                }
                buffer.amount += moved;
                tank.take(moved);
            }
        }
    }
}

/// How much a valve in `Empty` mode pushes out of its buffer per tick.
pub fn eject(buffer: &Stored) -> i32 {
    buffer.amount.clamp(0, EJECT_RATE as i64) as i32
}

/// Menu data: amount (3 entries of 16 bits), capacity (3), kind, mode, formed.
pub const DATA_AMOUNT: usize = 0;
pub const DATA_CAPACITY: usize = 3;
pub const DATA_KIND: usize = 6;
pub const DATA_MODE: usize = 7;
pub const DATA_FORMED: usize = 8;
pub const DATA_LEN: usize = 9;

pub fn encode_long(v: i64) -> [i32; 3] {
    let v = v.clamp(0, (1 << 48) - 1);
    [(v & 0xFFFF) as i32, ((v >> 16) & 0xFFFF) as i32, ((v >> 32) & 0xFFFF) as i32]
}

pub fn read_long(data: &[i32], at: usize) -> i64 {
    (0..3).map(|i| ((data.get(at + i).copied().unwrap_or(0) & 0xFFFF) as i64) << (16 * i)).sum()
}

pub fn menu_data(tank: &Stored, capacity: i64, mode: Mode, formed: bool) -> [i32; DATA_LEN] {
    let [a0, a1, a2] = encode_long(tank.amount);
    let [c0, c1, c2] = encode_long(capacity);
    let kind = tank.fluid.as_deref().filter(|_| tank.amount > 0).and_then(chemicals::index).map_or(0, |i| i as i32 + 1);
    [a0, a1, a2, c0, c1, c2, kind, mode.byte() as i32, formed as i32]
}

/// Menu button ids: the edit mode button, as the tanks' (left forward, right back).
pub const BUTTON_MODE_NEXT: i32 = crate::tank::BUTTON_MODE_NEXT;
pub const BUTTON_MODE_PREV: i32 = crate::tank::BUTTON_MODE_PREV;

pub fn button_at(at: [f32; 2], right: bool) -> Option<i32> {
    crate::tank::button_at(at, right)
}

/// "12.5 B" style amounts: mB below 10 buckets, then buckets with SI suffixes.
pub fn amount_text(mb: i64) -> String {
    if mb < 10_000 {
        format!("{mb} mB")
    } else {
        format!("{} B", crate::cube::si(mb / 1_000))
    }
}

/// The contents' name from the menu data (fluids and chemicals of `chemicals`' table).
pub fn name(data: &[i32]) -> &'static str {
    let kind = data.get(DATA_KIND).copied().unwrap_or(0);
    (kind as usize).checked_sub(1).and_then(chemicals::entry).map_or("Empty", |e| e.0)
}

pub fn contents_line(data: &[i32]) -> String {
    let amount = read_long(data, DATA_AMOUNT);
    let capacity = read_long(data, DATA_CAPACITY);
    format!("{}: {} / {}", name(data), amount_text(amount), amount_text(capacity))
}

pub fn tooltip(data: &[i32], at: [f32; 2]) -> Vec<String> {
    if crate::machine::inside(crate::tank::GAUGE, at) {
        return vec![contents_line(data)];
    }
    if crate::machine::inside(crate::tank::MODE_BUTTON, at) {
        return vec!["Left: next  |  Right: previous".to_owned()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(f: &str, n: i64) -> Stored {
        Stored::new(Some(f.to_owned()), n)
    }

    #[test]
    fn capacity_is_volume_times_16000_buckets() {
        assert_eq!(capacity(27), 432_000_000);
        assert_eq!(capacity(1), 16_000_000);
        // The biggest tank, 18^3, still fits an i64 (and Mekanism's own config bound).
        assert_eq!(capacity(18 * 18 * 18), 93_312_000_000);
        assert!(capacity(18 * 18 * 18) > i32::MAX as i64);
    }

    #[test]
    fn mode_cycles() {
        assert_eq!(Mode::Both.next(true), Mode::Fill);
        assert_eq!(Mode::Empty.next(true), Mode::Both);
        assert_eq!(Mode::Both.next(false), Mode::Empty);
        assert_eq!(Mode::from_byte(9), Mode::Both);
    }

    #[test]
    fn valve_fills_the_master() {
        let (mut buffer, mut tank) = (s("minecraft:water", 5_000), Stored::default());
        exchange(&mut buffer, &mut tank, 100_000, BUFFER, Mode::Fill);
        assert_eq!(tank, s("minecraft:water", 5_000));
        assert_eq!(buffer, Stored::default());
    }

    #[test]
    fn valve_refuses_another_fluid_and_a_full_tank() {
        let (mut buffer, mut tank) = (s("minecraft:lava", 1_000), s("minecraft:water", 10));
        exchange(&mut buffer, &mut tank, 100_000, BUFFER, Mode::Fill);
        assert_eq!(buffer.amount, 1_000, "other fluid stays in the buffer");
        assert_eq!(tank.amount, 10);
        let (mut buffer, mut tank) = (s("minecraft:water", 1_000), s("minecraft:water", 99_500));
        exchange(&mut buffer, &mut tank, 100_000, BUFFER, Mode::Fill);
        assert_eq!((buffer.amount, tank.amount), (500, 100_000));
    }

    #[test]
    fn valve_refills_its_buffer() {
        let (mut buffer, mut tank) = (Stored::default(), s("mekanism:brine", 1_000_000));
        exchange(&mut buffer, &mut tank, 2_000_000, BUFFER, Mode::Empty);
        assert_eq!((buffer.amount, tank.amount), (BUFFER as i64, 1_000_000 - BUFFER as i64));
        exchange(&mut buffer, &mut tank, 2_000_000, BUFFER, Mode::Fill);
        assert_eq!(buffer.amount, 0, "fill mode takes the buffer into the tank");
        assert_eq!(tank.amount, 1_000_000);
    }

    #[test]
    fn both_mode_does_either() {
        // Pipe put 3,000 in; the same tick refills the buffer.
        let (mut buffer, mut tank) = (s("minecraft:water", 3_000), s("minecraft:water", 50_000));
        exchange(&mut buffer, &mut tank, 100_000, BUFFER, Mode::Both);
        assert_eq!((buffer.amount, tank.amount), (BUFFER as i64, 53_000 - BUFFER as i64));
    }

    #[test]
    fn empty_mode_does_not_fill() {
        let (mut buffer, mut tank) = (s("minecraft:water", 3_000), Stored::default());
        exchange(&mut buffer, &mut tank, 100_000, BUFFER, Mode::Empty);
        assert_eq!(tank.amount, 0);
        assert_eq!(buffer.amount, 3_000);
    }

    #[test]
    fn ejecting_is_capped_at_the_rate() {
        assert_eq!(eject(&s("minecraft:water", 5_000)), 1_024);
        assert_eq!(eject(&s("minecraft:water", 100)), 100);
        assert_eq!(eject(&Stored::default()), 0);
    }

    #[test]
    fn menu_data_round_trips_big_amounts() {
        let t = s("mekanism:chemical/oxygen", 93_312_000_000);
        let d = menu_data(&t, capacity(18 * 18 * 18), Mode::Fill, true);
        assert!(d.iter().all(|v| (0..=0xFFFF).contains(v)));
        assert_eq!(read_long(&d, DATA_AMOUNT), 93_312_000_000);
        assert_eq!(read_long(&d, DATA_CAPACITY), 93_312_000_000);
        assert_eq!(d[DATA_MODE], 1);
        assert!(contents_line(&d).starts_with("Oxygen: "));
        assert_eq!(menu_data(&Stored::default(), 0, Mode::Both, false)[DATA_KIND], 0);
    }

    #[test]
    fn def_classifies_blocks() {
        use crate::multiblock::Cell;
        assert_eq!(DEF.classify(TANK), Cell::Frame);
        assert_eq!(DEF.classify(GLASS), Cell::Foreign, "TankValidator: glass is an INVALID casing");
        assert_eq!(DEF.classify(VALVE), Cell::Valve);
        assert_eq!(DEF.classify("minecraft:air"), Cell::Air);
        assert_eq!(DEF.classify("minecraft:stone"), Cell::Foreign);
        assert!(DEF.is_member(VALVE) && DEF.is_member(TANK) && !DEF.is_member(GLASS));
    }
}
