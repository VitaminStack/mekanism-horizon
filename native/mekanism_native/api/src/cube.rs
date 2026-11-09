//! Energy cube rules without Horizon types, so `logic/energy` can test them
//! with plain `cargo test` (it includes this file by path).
//!
//! Directions use vanilla's `Direction` numbers, as Horizon's energy faces do:
//! 0 down, 1 up, 2 north, 3 south, 4 west, 5 east.
//! A cube's side config is kept by *relative* side, as Mekanism's `RelativeSide`,
//! so it turns with the cube: 0 front, 1 back, 2 left, 3 right, 4 top, 5 bottom.

pub const DOWN: usize = 0;
pub const UP: usize = 1;
pub const NORTH: usize = 2;
pub const SOUTH: usize = 3;
pub const WEST: usize = 4;
pub const EAST: usize = 5;

pub const DIR_NAMES: [&str; 6] = ["down", "up", "north", "south", "west", "east"];
pub const OFFSETS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

pub const FRONT: usize = 0;
pub const BACK: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const TOP: usize = 4;
pub const BOTTOM: usize = 5;
pub const REL_NAMES: [&str; 6] = ["front", "back", "left", "right", "top", "bottom"];

/// Mekanism's `DataType` for one side: no connection, energy in, energy out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    None = 0,
    Input = 1,
    Output = 2,
}

impl Mode {
    pub fn from_byte(b: u8) -> Mode {
        match b {
            1 => Mode::Input,
            2 => Mode::Output,
            _ => Mode::None,
        }
    }
    /// Left click goes forward (none, input, output), right click back.
    pub fn cycle(self, forward: bool) -> Mode {
        match (self, forward) {
            (Mode::None, true) | (Mode::Output, false) => Mode::Input,
            (Mode::Input, true) | (Mode::None, false) => Mode::Output,
            (Mode::Output, true) | (Mode::Input, false) => Mode::None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Mode::None => "none",
            Mode::Input => "input",
            Mode::Output => "output",
        }
    }
}

/// Side config by relative side.
pub type Sides = [Mode; 6];

/// Mekanism's default energy config for a cube: output at the front, input elsewhere.
pub const DEFAULT_SIDES: Sides = [Mode::Output, Mode::Input, Mode::Input, Mode::Input, Mode::Input, Mode::Input];

pub fn sides_from_bytes(bytes: &[u8]) -> Option<Sides> {
    (bytes.len() == 6).then(|| std::array::from_fn(|i| Mode::from_byte(bytes[i])))
}

pub fn sides_to_bytes(sides: &Sides) -> Vec<u8> {
    sides.iter().map(|&m| m as u8).collect()
}

pub fn opposite(dir: usize) -> usize {
    dir ^ 1
}

pub fn dir_from_name(name: &str) -> Option<usize> {
    DIR_NAMES.iter().position(|&n| n == name)
}

/// Turn a horizontal direction clockwise, seen from above (north, east, south, west).
fn clockwise(dir: usize) -> usize {
    match dir {
        NORTH => EAST,
        EAST => SOUTH,
        SOUTH => WEST,
        WEST => NORTH,
        other => other,
    }
}

/// The world direction of relative side `rel` on a cube whose front faces `facing`.
/// Horizontal fronts as Mekanism's `RelativeSide`: top up, left the front turned
/// clockwise. A front facing up has its top to the north, one facing down to the south.
pub fn absolute(rel: usize, facing: usize) -> usize {
    let (top, left) = match facing {
        UP => (NORTH, EAST),
        DOWN => (SOUTH, EAST),
        f => (UP, clockwise(f)),
    };
    match rel {
        FRONT => facing,
        BACK => opposite(facing),
        LEFT => left,
        RIGHT => opposite(left),
        TOP => top,
        _ => opposite(top),
    }
}

/// The relative side that faces world direction `dir`.
pub fn relative(dir: usize, facing: usize) -> usize {
    (0..6).find(|&rel| absolute(rel, facing) == dir).unwrap_or(FRONT)
}

/// Mekanism turns a cube's front toward its placer (`getNearestLookingDirection().getOpposite()`).
/// `yaw` 0 looks south, 90 west; `pitch` above 0 looks down.
pub fn facing_from_look(yaw: f32, pitch: f32) -> usize {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    let look = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
    let axis = (0..3).max_by(|&a, &b| look[a].abs().total_cmp(&look[b].abs())).unwrap_or(2);
    let looking = match (axis, look[axis] > 0.0) {
        (0, true) => EAST,
        (0, false) => WEST,
        (1, true) => UP,
        (1, false) => DOWN,
        (_, true) => SOUTH,
        (_, false) => NORTH,
    };
    opposite(looking)
}

/// How much one push moves: no more than the giver may send this tick, holds,
/// and the taker has room for and takes in one call. Never negative.
pub fn transfer(budget: i32, stored: i32, target_stored: i32, target_capacity: i32, target_rate: i32) -> i32 {
    let room = target_capacity.saturating_sub(target_stored).max(0);
    budget.min(stored).min(room).min(target_rate).max(0)
}

/// Energy with an SI prefix, as Mekanism's GUI shows it: `4.00 M`, `950`.
pub fn si(amount: i64) -> String {
    const PREFIXES: [(i64, &str); 3] = [(1_000_000_000, "G"), (1_000_000, "M"), (1_000, "k")];
    for (scale, prefix) in PREFIXES {
        if amount.abs() >= scale {
            return format!("{:.2} {prefix}", amount as f64 / scale as f64);
        }
    }
    amount.to_string()
}

/// Menus are content-defined (`mekanism:energy_cube`), not `Host::open_menu`.
/// Data is sent in signed 16-bit slots by Horizon: store each i32 in two halves.
pub const DATA_ENERGY: usize = 0;
pub const DATA_EJECTED: usize = 2;
pub const DATA_FACING: usize = 4;
pub const DATA_SIDES: usize = 5;
pub const MENU_DATA_SLOTS: usize = 11;

/// The six clickable side buttons, in GUI pixels relative to the panel.
pub const REL_BUTTON_NAMES: [&str; 6] = ["Front", "Back", "Left", "Right", "Top", "Bottom"];
pub const SIDE_BUTTON_RECTS: [[f32; 4]; 6] = [
    [8.0, 69.0, 76.0, 18.0],
    [92.0, 69.0, 76.0, 18.0],
    [8.0, 91.0, 76.0, 18.0],
    [92.0, 91.0, 76.0, 18.0],
    [8.0, 113.0, 76.0, 18.0],
    [92.0, 113.0, 76.0, 18.0],
];

pub fn side_button_at(at: [f32; 2], right: bool) -> Option<i32> {
    SIDE_BUTTON_RECTS.iter().position(|&[x, y, w, h]| {
        at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
    }).map(|rel| rel as i32 + if right { 6 } else { 0 })
}

pub fn side_button_action(button: i32) -> Option<(usize, bool)> {
    match button {
        0..=5 => Some((button as usize, true)),
        6..=11 => Some(((button - 6) as usize, false)),
        _ => None,
    }
}

pub fn encode_amount(value: i32) -> [i32; 2] {
    [value & 0xFFFF, (value >> 16) & 0xFFFF]
}

pub fn read_amount(data: &[i32], offset: usize) -> i32 {
    let lo = data.get(offset).copied().unwrap_or(0) as u32 & 0xFFFF;
    let hi = data.get(offset + 1).copied().unwrap_or(0) as u32 & 0xFFFF;
    ((hi << 16) | lo) as i32
}

pub fn menu_data(stored: i32, ejected: i32, facing: usize, sides: &Sides) -> [i32; MENU_DATA_SLOTS] {
    let mut data = [0; MENU_DATA_SLOTS];
    data[DATA_ENERGY..DATA_ENERGY + 2].copy_from_slice(&encode_amount(stored));
    data[DATA_EJECTED..DATA_EJECTED + 2].copy_from_slice(&encode_amount(ejected));
    data[DATA_FACING] = facing as i32;
    for (rel, mode) in sides.iter().enumerate() {
        data[DATA_SIDES + rel] = *mode as i32;
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_sides_cover_all_directions_for_every_facing() {
        for facing in 0..6 {
            let mut seen = [false; 6];
            for rel in 0..6 {
                seen[absolute(rel, facing)] = true;
                assert_eq!(relative(absolute(rel, facing), facing), rel);
            }
            assert_eq!(seen, [true; 6], "facing {facing}");
        }
    }

    #[test]
    fn front_and_back_follow_facing() {
        assert_eq!(absolute(FRONT, NORTH), NORTH);
        assert_eq!(absolute(BACK, NORTH), SOUTH);
        assert_eq!(absolute(TOP, EAST), UP);
        assert_eq!(absolute(BOTTOM, WEST), DOWN);
        assert_eq!(absolute(LEFT, NORTH), EAST);
        assert_eq!(absolute(RIGHT, NORTH), WEST);
    }

    #[test]
    fn faces_the_placer() {
        // Looking north (yaw 180) at the block: its front faces south, back at the player.
        assert_eq!(facing_from_look(180.0, 0.0), SOUTH);
        assert_eq!(facing_from_look(0.0, 0.0), NORTH);
        assert_eq!(facing_from_look(90.0, 0.0), EAST);
        assert_eq!(facing_from_look(270.0, 0.0), WEST);
        assert_eq!(facing_from_look(0.0, 80.0), UP);
        assert_eq!(facing_from_look(0.0, -80.0), DOWN);
    }

    #[test]
    fn mode_cycle_both_ways() {
        let mut m = Mode::None;
        for want in [Mode::Input, Mode::Output, Mode::None] {
            m = m.cycle(true);
            assert_eq!(m, want);
        }
        for want in [Mode::Output, Mode::Input, Mode::None] {
            m = m.cycle(false);
            assert_eq!(m, want);
        }
    }

    #[test]
    fn sides_round_trip_and_reject_bad_length() {
        let sides = [Mode::None, Mode::Output, Mode::Input, Mode::Input, Mode::None, Mode::Output];
        assert_eq!(sides_from_bytes(&sides_to_bytes(&sides)), Some(sides));
        assert_eq!(sides_from_bytes(&[1, 2]), None);
        assert_eq!(sides_from_bytes(&[9; 6]), Some([Mode::None; 6]));
    }

    #[test]
    fn transfer_never_loses_or_creates_energy() {
        assert_eq!(transfer(4_000, 10_000, 0, 4_000_000, 4_000), 4_000);
        assert_eq!(transfer(4_000, 1_000, 0, 4_000_000, 4_000), 1_000);
        assert_eq!(transfer(4_000, 10_000, 3_999_000, 4_000_000, 4_000), 1_000);
        assert_eq!(transfer(16_000, 20_000, 0, 4_000_000, 4_000), 4_000);
        assert_eq!(transfer(4_000, 10_000, 4_000_000, 4_000_000, 4_000), 0);
        // Corrupt saves (over capacity, negative) move nothing.
        assert_eq!(transfer(4_000, 10_000, 5_000_000, 4_000_000, 4_000), 0);
        assert_eq!(transfer(4_000, -5, 0, 4_000_000, 4_000), 0);
        assert_eq!(transfer(i32::MAX, i32::MAX, i32::MIN, i32::MAX, i32::MAX), i32::MAX);
    }

    #[test]
    fn si_prefixes() {
        assert_eq!(si(950), "950");
        assert_eq!(si(4_000), "4.00 k");
        assert_eq!(si(4_000_000), "4.00 M");
        assert_eq!(si(i32::MAX as i64), "2.15 G");
    }

    #[test]
    fn menu_buttons_round_trip_and_do_not_touch_inventory() {
        for rel in 0..6 {
            let [x, y, w, h] = SIDE_BUTTON_RECTS[rel];
            for right in [false, true] {
                let id = side_button_at([x + w / 2.0, y + h / 2.0], right).unwrap();
                assert_eq!(side_button_action(id), Some((rel, !right)));
            }
        }
        assert_eq!(side_button_at([8.0, 157.0], false), None);
        assert_eq!(side_button_action(-1), None);
        assert_eq!(side_button_action(12), None);
    }

    #[test]
    fn menu_data_survives_signed_16_bit_wire_values() {
        let data = menu_data(i32::MAX, 123_456_789, EAST, &DEFAULT_SIDES);
        // The server sends 16-bit signed shorts, which may arrive negative.
        let wire: Vec<i32> = data.iter().map(|&v| (v as i16) as i32).collect();
        assert_eq!(read_amount(&wire, DATA_ENERGY), i32::MAX);
        assert_eq!(read_amount(&wire, DATA_EJECTED), 123_456_789);
        assert_eq!(wire[DATA_FACING], EAST as i32);
        assert_eq!(wire[DATA_SIDES], Mode::Output as i32);
        assert_eq!(read_amount(&[], DATA_ENERGY), 0);
    }
}
