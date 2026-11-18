//! The Digital Miner's rules (M08), Horizon-free: settings, search order, mining order,
//! energy use, filters, the screen layout and menu data. The host side is `miners.rs`.
//!
//! Upstream: `TileEntityDigitalMiner` (defaults, clamps, `tryMineBlock`, `getOffsetForIndex`),
//! `ThreadMinerSearch` (the scan), `MinerEnergyContainer.updateMinerEnergyPerTick` (energy use),
//! `GeneralConfig` (`maxRadius` 32, `silkMultiplier` 12, `ticksPerMine` 80), `UsageConfig`
//! (`digitalMiner` 40) and `MekanismBlockTypes.DIGITAL_MINER` (the bounding blocks).

use crate::cube;

pub const BLOCK: &str = "mekanism:digital_miner";
pub const MENU: &str = "mekanism:digital_miner";
pub const MAIN_SLOTS: usize = 27;
pub const FILTER_SLOTS: usize = 9;
pub const SLOTS: usize = MAIN_SLOTS + FILTER_SLOTS;

/// `TileEntityDigitalMiner`.
pub const DEFAULT_RADIUS: i32 = 10;
pub const DEFAULT_HEIGHT_RANGE: i32 = 60;
pub const DEFAULT_MIN_Y: i32 = 0;
pub const DEFAULT_MAX_Y: i32 = DEFAULT_MIN_Y + DEFAULT_HEIGHT_RANGE;
/// `GeneralConfig.minerMaxRadius`, `minerSilkMultiplier`, `minerTicksPerMine`.
pub const MAX_RADIUS: i32 = 32;
pub const SILK_MULTIPLIER: i32 = 12;
pub const TICKS_PER_MINE: i32 = 80;
/// `UsageConfig.digitalMiner`.
pub const BASE_USAGE: i32 = 40;
/// Horizon's buffer is one size per block type; upstream's is 4 x the current use, which a
/// block type cannot follow. This holds the use of every setting (at most 1,920 FE/t).
pub const CAPACITY: i32 = 100_000;
/// `MekanismUtils.TICKS_PER_HALF_SECOND`: the time between ejections.
pub const EJECT_TICKS: i32 = 10;
/// Positions the search looks at per tick (upstream scans in a thread).
pub const SEARCH_PER_TICK: usize = 2_048;
/// The most blocks one tick looks at, skipping the ones it cannot or may not mine.
pub const MINE_LOOKS_PER_TICK: usize = 512;

/// The block tags a tag filter can use (the screen cannot list the game's tags).
pub const TAG_CHOICES: [&str; 8] = [
    "minecraft:base_stone_overworld",
    "minecraft:dirt",
    "minecraft:sand",
    "minecraft:logs",
    "minecraft:leaves",
    "minecraft:snow",
    "minecraft:mineable/pickaxe",
    "c:ores",
];

/// `HandleBoundingBlock` of the miner: x and z -1..=1, y 0..=1 around the main block.
pub fn bounding_offsets() -> Vec<[i32; 3]> {
    let mut v = Vec::new();
    for x in -1..=1 {
        for y in 0..=1 {
            for z in -1..=1 {
                if (x, y, z) != (0, 0, 0) {
                    v.push([x, y, z]);
                }
            }
        }
    }
    v
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub radius: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub silk: bool,
    pub eject: bool,
    pub inverse: bool,
    /// Which of `TAG_CHOICES` is the tag filter, if any.
    pub tag: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { radius: DEFAULT_RADIUS, min_y: DEFAULT_MIN_Y, max_y: DEFAULT_MAX_Y, silk: false, eject: false, inverse: false, tag: None }
    }
}

impl Settings {
    pub fn diameter(&self) -> i32 {
        self.radius * 2 + 1
    }

    /// `getTotalSize`.
    pub fn total_size(&self) -> usize {
        let d = self.diameter() as usize;
        d * d * (self.max_y - self.min_y + 1).max(0) as usize
    }

    /// `getStartingPos`: the lowest corner of the box.
    pub fn start(&self, miner: [i32; 3]) -> [i32; 3] {
        [miner[0] - self.radius, self.min_y, miner[2] - self.radius]
    }

    /// `setRadius`: 0 to the largest radius.
    pub fn set_radius(&mut self, radius: i32) {
        self.radius = radius.clamp(0, MAX_RADIUS);
    }

    /// `setMinY`: from the world's lowest block up to the maximum.
    pub fn set_min_y(&mut self, y: i32, world_min: i32) {
        self.min_y = y.clamp(world_min, self.max_y.max(world_min));
    }

    /// `setMaxY`: from the minimum up to the world's highest block.
    pub fn set_max_y(&mut self, y: i32, world_max: i32) {
        self.max_y = y.clamp(self.min_y, world_max.max(self.min_y));
    }

    /// Saved values brought back into range (a changed world height, an old save).
    pub fn fit(&mut self, world_min: i32, world_max: i32) {
        self.radius = self.radius.clamp(0, MAX_RADIUS);
        self.min_y = self.min_y.clamp(world_min, world_max);
        self.max_y = self.max_y.clamp(self.min_y, world_max);
    }
}

/// `getOffsetForIndex`: x runs fastest, then z, then y.
pub fn offset_for_index(start: [i32; 3], diameter: i32, index: usize) -> [i32; 3] {
    let (i, d) = (index as i64, diameter as i64);
    [start[0] + (i % d) as i32, start[1] + (i / d / d) as i32, start[2] + ((i / d) % d) as i32]
}

/// `MinerEnergyContainer.updateMinerEnergyPerTick`; `world_height` is `level.getHeight()`.
pub fn energy_per_tick(s: &Settings, world_height: i32) -> i32 {
    let mut base = BASE_USAGE as i64;
    if s.silk {
        base = (base * SILK_MULTIPLIER as i64).min(i32::MAX as i64);
    }
    let radius_range = (MAX_RADIUS - DEFAULT_RADIUS) as f64;
    let height_range = (world_height - 1 - DEFAULT_HEIGHT_RANGE) as f64;
    let radius_cost = if radius_range == 0.0 { 0.0 } else { ((s.radius - DEFAULT_RADIUS) as f64 / radius_range).max(0.0) };
    let height_cost = if height_range == 0.0 {
        0.0
    } else {
        ((s.max_y - s.min_y - DEFAULT_HEIGHT_RANGE) as f64 / height_range).max(0.0)
    };
    ((base as f64) * (1.0 + radius_cost) * (1.0 + height_cost)).ceil().min(i32::MAX as f64) as i32
}

/// Blocks the miner never breaks: the ones with no destroy speed (`getDestroySpeed < 0`),
/// air, liquids. Horizon has no call for a block's hardness, so unbreakable blocks are listed.
pub fn can_break(block: &str) -> bool {
    const NEVER: [&str; 14] = [
        "minecraft:bedrock",
        "minecraft:barrier",
        "minecraft:light",
        "minecraft:command_block",
        "minecraft:chain_command_block",
        "minecraft:repeating_command_block",
        "minecraft:structure_block",
        "minecraft:structure_void",
        "minecraft:jigsaw",
        "minecraft:test_block",
        "minecraft:end_portal",
        "minecraft:end_portal_frame",
        "minecraft:end_gateway",
        "minecraft:nether_portal",
    ];
    const LIQUID: [&str; 2] = ["minecraft:water", "minecraft:lava"];
    !(block.is_empty()
        || matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
        || block == "mekanism:bounding_block"
        || block == "minecraft:reinforced_deepslate"
        || NEVER.contains(&block)
        || LIQUID.contains(&block))
}

/// The search says mine a block when the inverse mode differs from "a filter matches".
pub fn accepted(inverse: bool, matches: bool) -> bool {
    inverse != matches
}

/// `ThreadMinerSearch.run` and `tryMineBlock`: with the normal mode and no filter, nothing is mined.
pub fn searches(s: &Settings, filters: usize) -> bool {
    s.inverse || filters > 0 || s.tag.is_some()
}

/// The blocks to mine, by chunk in the order they were found; inside a chunk, the highest
/// index first (`tryMineBlock` goes back to front so supports go last).
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    chunks: Vec<((i32, i32), Vec<u32>)>,
}

pub fn chunk_of(pos: [i32; 3]) -> (i32, i32) {
    (pos[0] >> 4, pos[2] >> 4)
}

impl Queue {
    pub fn push(&mut self, chunk: (i32, i32), index: u32) {
        match self.chunks.iter_mut().find(|c| c.0 == chunk) {
            Some(c) => c.1.push(index),
            None => self.chunks.push((chunk, vec![index])),
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.iter().map(|c| c.1.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// The index mined next.
    pub fn next(&self) -> Option<u32> {
        self.chunks.first().and_then(|c| c.1.last().copied())
    }

    pub fn pop(&mut self) {
        if let Some(c) = self.chunks.first_mut() {
            c.1.pop();
            if c.1.is_empty() {
                self.chunks.remove(0);
            }
        }
    }
}

/// The scan: positions by index, a slice per tick.
#[derive(Default, Debug, Clone)]
pub struct Search {
    pub next: usize,
    pub queue: Queue,
    pub started: bool,
    pub done: bool,
}

impl Search {
    /// Looks at up to `budget` more positions; `accept` says whether a position is to be mined.
    /// The miner's own position is skipped. Returns when the whole box has been seen.
    pub fn step(&mut self, s: &Settings, miner: [i32; 3], budget: usize, accept: &mut dyn FnMut([i32; 3]) -> bool) {
        self.started = true;
        let (start, d, size) = (s.start(miner), s.diameter(), s.total_size());
        let end = (self.next + budget).min(size);
        for i in self.next..end {
            let pos = offset_for_index(start, d, i);
            if pos != miner && accept(pos) {
                self.queue.push(chunk_of(pos), i as u32);
            }
        }
        self.next = end;
        self.done = end >= size;
    }
}

// --- screen ---

pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 288.0;
pub const INVENTORY_X: f32 = 8.0;
pub const INVENTORY_Y: f32 = 206.0;
pub const SLOT_X: f32 = 8.0;
pub const MAIN_Y: f32 = 116.0;
pub const FILTER_Y: f32 = 176.0;

pub fn slot_positions() -> Vec<Option<[f32; 2]>> {
    (0..MAIN_SLOTS)
        .map(|i| Some([SLOT_X + (i % 9) as f32 * 18.0, MAIN_Y + (i / 9) as f32 * 18.0]))
        .chain((0..FILTER_SLOTS).map(|i| Some([SLOT_X + i as f32 * 18.0, FILTER_Y])))
        .collect()
}

pub const DATA_RADIUS: usize = 0;
pub const DATA_MIN_Y: usize = 1;
pub const DATA_MAX_Y: usize = 2;
pub const DATA_FLAGS: usize = 3;
pub const DATA_STATE: usize = 4;
/// Two entries each (`cube::encode_amount`).
pub const DATA_TO_MINE: usize = 5;
pub const DATA_ENERGY: usize = 7;
pub const DATA_USAGE: usize = 9;
pub const DATA_TAG: usize = 11;
pub const DATA_LEN: usize = 12;

pub const FLAG_SILK: i32 = 1;
pub const FLAG_EJECT: i32 = 2;
pub const FLAG_INVERSE: i32 = 4;
pub const FLAG_RUNNING: i32 = 8;
pub const FLAG_ACTIVE: i32 = 16;

/// `ThreadMinerSearch.State`: idle, searching, paused (stopped after a search), ready.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle = 0,
    Searching = 1,
    Paused = 2,
    Ready = 3,
}

pub fn state_name(state: i32) -> &'static str {
    match state {
        1 => "Searching",
        2 => "Paused",
        3 => "Ready",
        _ => "Idle",
    }
}

pub struct View {
    pub settings: Settings,
    pub running: bool,
    pub active: bool,
    pub state: State,
    pub to_mine: i32,
    pub energy: i32,
    pub usage: i32,
}

pub fn menu_data(v: &View) -> Vec<i32> {
    let s = &v.settings;
    let flag = |on: bool, bit: i32| if on { bit } else { 0 };
    let mut data = vec![0; DATA_LEN];
    data[DATA_RADIUS] = s.radius;
    data[DATA_MIN_Y] = s.min_y;
    data[DATA_MAX_Y] = s.max_y;
    data[DATA_FLAGS] = flag(s.silk, FLAG_SILK) | flag(s.eject, FLAG_EJECT) | flag(s.inverse, FLAG_INVERSE)
        | flag(v.running, FLAG_RUNNING) | flag(v.active, FLAG_ACTIVE);
    data[DATA_STATE] = v.state as i32;
    data[DATA_TO_MINE..DATA_TO_MINE + 2].copy_from_slice(&cube::encode_amount(v.to_mine));
    data[DATA_ENERGY..DATA_ENERGY + 2].copy_from_slice(&cube::encode_amount(v.energy));
    data[DATA_USAGE..DATA_USAGE + 2].copy_from_slice(&cube::encode_amount(v.usage));
    data[DATA_TAG] = s.tag.map_or(0, |t| t as i32 + 1);
    data
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Radius(i32),
    MinY(i32),
    MaxY(i32),
    Silk,
    Eject,
    Inverse,
    Tag,
    StartStop,
    Reset,
}

pub const STEPS: [i32; 4] = [-10, -1, 1, 10];
const K_RADIUS: i32 = 0;
const K_MIN: i32 = 1;
const K_MAX: i32 = 2;
const K_SILK: i32 = 3;
const K_EJECT: i32 = 4;
const K_INVERSE: i32 = 5;
const K_TAG: i32 = 6;
const K_START: i32 = 7;
const K_RESET: i32 = 8;

pub fn id_of(a: Action) -> i32 {
    let step = |d: i32| STEPS.iter().position(|s| *s == d).unwrap_or(0) as i32;
    match a {
        Action::Radius(d) => K_RADIUS * 8 + step(d),
        Action::MinY(d) => K_MIN * 8 + step(d),
        Action::MaxY(d) => K_MAX * 8 + step(d),
        Action::Silk => K_SILK * 8,
        Action::Eject => K_EJECT * 8,
        Action::Inverse => K_INVERSE * 8,
        Action::Tag => K_TAG * 8,
        Action::StartStop => K_START * 8,
        Action::Reset => K_RESET * 8,
    }
}

pub fn action_of(id: i32) -> Option<Action> {
    let (kind, arg) = (id.div_euclid(8), id.rem_euclid(8) as usize);
    let delta = STEPS.get(arg).copied();
    Some(match kind {
        K_RADIUS => Action::Radius(delta?),
        K_MIN => Action::MinY(delta?),
        K_MAX => Action::MaxY(delta?),
        K_SILK => Action::Silk,
        K_EJECT => Action::Eject,
        K_INVERSE => Action::Inverse,
        K_TAG => Action::Tag,
        K_START => Action::StartStop,
        K_RESET => Action::Reset,
        _ => return None,
    })
}

pub const SETTING_Y: f32 = 16.0;
pub const SETTING_STEP: f32 = 14.0;
pub const STEP_X: [f32; 4] = [70.0, 88.0, 106.0, 124.0];
pub const STEP_W: f32 = 16.0;
pub const BUTTON_H: f32 = 12.0;
/// Toggle buttons: (rect, action), two per row.
pub const TOGGLES: [([f32; 4], Action); 6] = [
    ([8.0, 58.0, 76.0, 12.0], Action::Silk),
    ([92.0, 58.0, 76.0, 12.0], Action::Eject),
    ([8.0, 72.0, 76.0, 12.0], Action::Inverse),
    ([92.0, 72.0, 76.0, 12.0], Action::Tag),
    ([8.0, 86.0, 76.0, 12.0], Action::StartStop),
    ([92.0, 86.0, 76.0, 12.0], Action::Reset),
];

pub fn step_rect(row: usize, step: usize) -> [f32; 4] {
    [STEP_X[step], SETTING_Y + SETTING_STEP * row as f32, STEP_W, BUTTON_H]
}

fn inside([x, y, w, h]: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

pub fn button_at(at: [f32; 2]) -> Option<i32> {
    for (rect, action) in TOGGLES {
        if inside(rect, at) {
            return Some(id_of(action));
        }
    }
    for row in 0..3 {
        for (step, delta) in STEPS.iter().enumerate() {
            if inside(step_rect(row, step), at) {
                let action = match row {
                    0 => Action::Radius(*delta),
                    1 => Action::MinY(*delta),
                    _ => Action::MaxY(*delta),
                };
                return Some(id_of(action));
            }
        }
    }
    None
}

/// What a press changes in the settings; the host handles start, stop and reset. Returns
/// whether the settings changed. `world` is (lowest block y, highest block y).
pub fn apply(s: &mut Settings, a: Action, (world_min, world_max): (i32, i32)) -> bool {
    let before = *s;
    match a {
        Action::Radius(d) => s.set_radius(s.radius + d),
        Action::MinY(d) => s.set_min_y(s.min_y + d, world_min),
        Action::MaxY(d) => s.set_max_y(s.max_y + d, world_max),
        Action::Silk => s.silk = !s.silk,
        Action::Eject => s.eject = !s.eject,
        Action::Inverse => s.inverse = !s.inverse,
        Action::Tag => {
            s.tag = match s.tag {
                None => Some(0),
                Some(t) if t + 1 < TAG_CHOICES.len() => Some(t + 1),
                Some(_) => None,
            }
        }
        Action::StartStop | Action::Reset => {}
    }
    *s != before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_use_follows_the_upstream_formula() {
        let h = 384; // overworld: height range 384 - 1 - 60 = 323
        let mut s = Settings::default();
        assert_eq!(energy_per_tick(&s, h), 40);
        s.silk = true;
        assert_eq!(energy_per_tick(&s, h), 480);
        // Maximum radius doubles it; so does the full height.
        s.radius = MAX_RADIUS;
        assert_eq!(energy_per_tick(&s, h), 960);
        s.min_y = -64;
        s.max_y = -64 + 60 + 323;
        assert_eq!(energy_per_tick(&s, h), 1_920);
        // Smaller than the defaults costs nothing less than the base.
        let small = Settings { radius: 1, min_y: 0, max_y: 5, ..Settings::default() };
        assert_eq!(energy_per_tick(&small, h), 40);
        // A part of the range rounds up: radius 11 of 22 steps.
        let r11 = Settings { radius: 11, ..Settings::default() };
        assert_eq!(energy_per_tick(&r11, h), (40.0f64 * (1.0 + 1.0 / 22.0)).ceil() as i32);
        assert!(energy_per_tick(&s, h) <= CAPACITY);
    }

    #[test]
    fn settings_clamp_like_upstream() {
        let mut s = Settings::default();
        s.set_radius(99);
        assert_eq!(s.radius, 32);
        s.set_radius(-3);
        assert_eq!(s.radius, 0);
        s.set_min_y(-500, -64);
        assert_eq!(s.min_y, -64);
        s.set_min_y(500, -64);
        assert_eq!(s.min_y, s.max_y, "the minimum stops at the maximum");
        s.set_max_y(9999, 319);
        assert_eq!(s.max_y, 319);
        s.set_max_y(-9999, 319);
        assert_eq!(s.max_y, s.min_y);
        let mut old = Settings { radius: 50, min_y: -100, max_y: 900, ..Settings::default() };
        old.fit(-64, 319);
        assert_eq!((old.radius, old.min_y, old.max_y), (32, -64, 319));
    }

    #[test]
    fn index_order_runs_x_then_z_then_y() {
        let start = [10, 5, 20];
        assert_eq!(offset_for_index(start, 3, 0), [10, 5, 20]);
        assert_eq!(offset_for_index(start, 3, 1), [11, 5, 20]);
        assert_eq!(offset_for_index(start, 3, 3), [10, 5, 21]);
        assert_eq!(offset_for_index(start, 3, 9), [10, 6, 20]);
        assert_eq!(offset_for_index(start, 3, 26), [12, 7, 22]);
        let s = Settings { radius: 1, min_y: 5, max_y: 7, ..Settings::default() };
        assert_eq!(s.total_size(), 27);
        assert_eq!(s.start([11, 6, 21]), [10, 5, 20]);
    }

    #[test]
    fn search_visits_every_position_once_and_skips_the_miner() {
        let s = Settings { radius: 1, min_y: 0, max_y: 2, ..Settings::default() };
        let miner = [1, 0, 1];
        let mut seen = Vec::new();
        let mut search = Search::default();
        let mut slices = 0;
        while !search.done {
            search.step(&s, miner, 10, &mut |p| {
                seen.push(p);
                true
            });
            slices += 1;
        }
        assert_eq!(slices, 3, "27 positions in slices of 10");
        assert_eq!(seen.len(), 26, "all but the miner");
        assert_eq!(seen[0], [0, 0, 0]);
        assert_eq!(*seen.last().unwrap(), [2, 2, 2]);
        assert_eq!(search.queue.len(), 26);
    }

    #[test]
    fn mining_goes_by_chunk_in_found_order_and_high_index_first() {
        // A box over two chunks: x 14..=17 (chunk 0 and 1), one layer.
        let s = Settings { radius: 2, min_y: 0, max_y: 1, ..Settings::default() };
        let miner = [16, 0, 0];
        let mut search = Search::default();
        // Only the second layer (y = 1) and every position of it.
        search.step(&s, miner, usize::MAX, &mut |p| p[1] == 1 && p[2] == 0);
        let q = &mut search.queue;
        assert_eq!(q.len(), 5);
        // Chunk 0 (x 14, 15) was found first, then chunk 1 (x 16, 17, 18).
        let order: Vec<[i32; 3]> = std::iter::from_fn(|| {
            let i = q.next()?;
            q.pop();
            Some(offset_for_index(s.start(miner), s.diameter(), i as usize))
        })
        .collect();
        let xs: Vec<i32> = order.iter().map(|p| p[0]).collect();
        assert_eq!(xs, vec![15, 14, 18, 17, 16]);
        assert!(q.is_empty());
    }

    #[test]
    fn upper_layers_are_mined_before_lower_ones() {
        let s = Settings { radius: 0, min_y: 0, max_y: 3, ..Settings::default() };
        let mut search = Search::default();
        search.step(&s, [0, 99, 0], usize::MAX, &mut |_| true);
        let mut ys = Vec::new();
        while let Some(i) = search.queue.next() {
            ys.push(offset_for_index(s.start([0, 99, 0]), 1, i as usize)[1]);
            search.queue.pop();
        }
        assert_eq!(ys, vec![3, 2, 1, 0]);
    }

    #[test]
    fn filters_and_inverse() {
        assert!(accepted(false, true));
        assert!(!accepted(false, false));
        assert!(!accepted(true, true));
        assert!(accepted(true, false));
        let plain = Settings::default();
        assert!(!searches(&plain, 0), "no filter, normal mode: nothing to mine");
        assert!(searches(&plain, 1));
        assert!(searches(&Settings { inverse: true, ..plain }, 0));
        assert!(searches(&Settings { tag: Some(0), ..plain }, 0));
    }

    #[test]
    fn unbreakable_and_special_blocks_are_skipped() {
        for b in ["minecraft:air", "minecraft:bedrock", "minecraft:water", "minecraft:lava", "mekanism:bounding_block", ""] {
            assert!(!can_break(b), "{b}");
        }
        for b in ["minecraft:stone", "minecraft:iron_ore", "minecraft:obsidian"] {
            assert!(can_break(b), "{b}");
        }
    }

    #[test]
    fn footprint_is_seventeen_bounding_blocks() {
        let b = bounding_offsets();
        assert_eq!(b.len(), 17);
        assert!(!b.contains(&[0, 0, 0]));
        assert!(b.contains(&[0, 1, 0]) && b.contains(&[-1, 1, 1]));
        assert!(b.iter().all(|o| (-1..=1).contains(&o[0]) && (0..=1).contains(&o[1]) && (-1..=1).contains(&o[2])));
    }

    #[test]
    fn buttons_and_menu_data_round_trip() {
        for a in [Action::Silk, Action::Eject, Action::Inverse, Action::Tag, Action::StartStop, Action::Reset] {
            assert_eq!(action_of(id_of(a)), Some(a));
        }
        for d in STEPS {
            for a in [Action::Radius(d), Action::MinY(d), Action::MaxY(d)] {
                assert_eq!(action_of(id_of(a)), Some(a));
            }
        }
        assert_eq!(action_of(K_RADIUS * 8 + 5), None);
        let mid = |r: [f32; 4]| [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0];
        assert_eq!(button_at(mid(step_rect(1, 3))).and_then(action_of), Some(Action::MinY(10)));
        assert_eq!(button_at(mid(step_rect(2, 0))).and_then(action_of), Some(Action::MaxY(-10)));
        assert_eq!(button_at(mid(TOGGLES[4].0)).and_then(action_of), Some(Action::StartStop));
        assert_eq!(button_at([100.0, 250.0]), None);
        // The three step rows end above the toggles; the toggles end above the main slots.
        assert!(step_rect(2, 0)[1] + BUTTON_H <= TOGGLES[0].0[1]);
        assert!(TOGGLES[5].0[1] + BUTTON_H < MAIN_Y);

        let mut s = Settings::default();
        assert!(apply(&mut s, Action::Radius(10), (-64, 319)));
        assert_eq!(s.radius, 20);
        assert!(!apply(&mut s, Action::StartStop, (-64, 319)));
        for _ in 0..TAG_CHOICES.len() + 2 {
            apply(&mut s, Action::Tag, (-64, 319));
        }
        assert_eq!(s.tag, Some(0), "the tag choice cycles through none");
        let v = View { settings: s, running: true, active: false, state: State::Ready, to_mine: 70_000, energy: 99_999, usage: 480 };
        let data = menu_data(&v);
        assert_eq!(data.len(), DATA_LEN);
        assert_eq!(cube::read_amount(&data, DATA_TO_MINE), 70_000);
        assert_eq!(cube::read_amount(&data, DATA_ENERGY), 99_999);
        assert_eq!(data[DATA_FLAGS], FLAG_RUNNING);
        assert_eq!(data[DATA_TAG], 1);
    }
}
