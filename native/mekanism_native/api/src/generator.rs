//! Heat Generator and Solar Generator (Mekanism: Generators, M04): the parts
//! that need no Horizon types, so their tests run in `logic/energy` without
//! building Horizon. World reads, slots and energy output are in `lib.rs`.
//!
//! Values from Mekanism 26.3: `GeneratorsConfig` (heat 100 per tick from lava,
//! 7 per lava side, 10 in the Nether, a one-bucket tank, 100 mB per tick;
//! solar 2 FE/t peak), `GeneratorsStorageConfig` (heat 12 s, solar 2 s of
//! output), `TileEntityHeatGenerator`, `TileEntitySolarGenerator`,
//! `BasicHeatCapacitor`, `ITileHeatHandler` and `HeatAPI`.
//!
//! Bio Generator (`TileEntityBioGenerator`): bio fuel items fill a 200 mB
//! bioethanol tank (64 mB per Bio Fuel, 9 x that per block), and each tick
//! burns 1 mB for 25 FE when all of it fits (`GeneratorsConfig`; storage 30 s).
//! The tank holds no fluid of Horizon's: there are no pipes yet.
//!
//! Advanced Solar Generator (`TileEntityAdvancedSolarGenerator`, 12 FE/t peak,
//! storage 12 s) and Wind Generator (`TileEntityWindGenerator`, 1 to 5 FE/t by
//! height, storage 5 s) are multiblocks: bounding blocks (`mekanism:bounding_block`)
//! fill the rest of their shape, placed and removed by the native mod.
//!
//! Approximations: the ambient temperature and the solar peak use the biome's
//! base temperature and downfall from the table below (vanilla's values; Horizon
//! gives mods the biome's id only), without vanilla's height adjustment. Sky
//! darkening and brightness follow vanilla's day-cycle formula from the time of
//! day and the rain and thunder levels.
//!
//! Not ported: the energy (charging) slot, redstone control, heat transfer to
//! adjacent heat blocks (there are none yet), filling the Bio Generator from
//! fluid containers, the `c:no_wind` dimension blacklist (no dimension here
//! without a sky has wind anyway), and using a bounding block to open the
//! main block's screen.

pub const HEAT: &str = "mekanismgenerators:heat_generator";
pub const SOLAR: &str = "mekanismgenerators:solar_generator";
pub const BIO: &str = "mekanismgenerators:bio_generator";
pub const ADVANCED_SOLAR: &str = "mekanismgenerators:advanced_solar_generator";
pub const WIND: &str = "mekanismgenerators:wind_generator";
pub const BOUNDING: &str = "mekanism:bounding_block";
/// Registered by `data/mekanismgenerators/horizon/menu/<generator>.json`.
pub const HEAT_MENU: &str = HEAT;
pub const SOLAR_MENU: &str = SOLAR;
pub const BIO_MENU: &str = BIO;
pub const ADVANCED_SOLAR_MENU: &str = ADVANCED_SOLAR;
pub const WIND_MENU: &str = WIND;
/// Mekanism's loops while running (`tile.generator.*` in `sounds.json`) and
/// their clip lengths in ticks.
pub const HEAT_SOUND: &str = "mekanismgenerators:tile.generator.heat";
pub const HEAT_SOUND_TICKS: i32 = 26;
pub const SOLAR_SOUND: &str = "mekanismgenerators:tile.generator.solar";
pub const SOLAR_SOUND_TICKS: i32 = 20;
pub const BIO_SOUND: &str = "mekanismgenerators:tile.generator.bio";
pub const BIO_SOUND_TICKS: i32 = 56;
pub const WIND_SOUND: &str = "mekanismgenerators:tile.generator.wind";
pub const WIND_SOUND_TICKS: i32 = 258;

pub const TICKS_PER_SECOND: i32 = 20;

// --- Heat Generator ---
pub const HEAT_STORAGE: i32 = 12 * TICKS_PER_SECOND;
pub const HEAT_GENERATION: f64 = 100.0;
pub const HEAT_GENERATION_LAVA: f64 = 7.0;
pub const HEAT_GENERATION_NETHER: f64 = 10.0;
pub const LAVA_TANK: i32 = 1_000;
pub const LAVA_RATE: i32 = 100;
/// One bucket, and vanilla's burn time of a lava bucket: a furnace fuel item
/// gives `LAVA_TANK * burn time / LAVA_BURN_TIME` mB (`FluidFuelInventorySlot`).
pub const LAVA_BURN_TIME: i32 = 20_000;
pub const HEAT_CAPACITY: f64 = 10.0;
pub const INVERSE_CONDUCTION: f64 = 5.0;
pub const INVERSE_INSULATION: f64 = 100.0;
pub const THERMAL_EFFICIENCY: f64 = 0.5;
pub const AMBIENT_TEMP: f64 = 300.0;
pub const AIR_INVERSE_COEFFICIENT: f64 = 10_000.0;
const EPSILON: f64 = 1e-6;
/// Six lava sides plus the block's own position, the Nether bonus and burning lava.
pub const MAX_PRODUCTION: i32 =
    (HEAT_GENERATION_LAVA as i32) * 7 + HEAT_GENERATION_NETHER as i32 + HEAT_GENERATION as i32;

/// `HeatAPI.getAmbientTemp`: 300 K at a biome temperature of 0.8.
pub fn ambient_temp(biome_temperature: f64) -> f64 {
    AMBIENT_TEMP + 25.0 * (biome_temperature.clamp(-5.0, 5.0) - 0.8)
}

/// Passive heat per tick (`getBoost`): 7 per adjacent lava block, 10 in the Nether.
pub fn boost(lava_sides: u32, nether: bool) -> f64 {
    HEAT_GENERATION_LAVA * lava_sides as f64 + if nether { HEAT_GENERATION_NETHER } else { 0.0 }
}

/// The lava a furnace fuel item adds (Mekanism's `FluidFuelInventorySlot.forFuel`).
pub fn fuel_lava(burn_time: i32) -> i32 {
    (LAVA_TANK as i64 * burn_time as i64 / LAVA_BURN_TIME as i64) as i32
}

/// Vanilla furnace fuels by item id (`FuelValues`), for items Horizon names no
/// burn time of; tags are checked in `lib.rs` with `FUEL_TAGS`.
pub const FUELS: [(&str, i32); 10] = [
    ("minecraft:coal_block", 16_000),
    ("minecraft:dried_kelp_block", 4_001),
    ("minecraft:blaze_rod", 2_400),
    ("minecraft:coal", 1_600),
    ("minecraft:charcoal", 1_600),
    ("minecraft:stick", 100),
    ("minecraft:bamboo", 50),
    ("minecraft:scaffolding", 50),
    ("minecraft:bowl", 100),
    ("minecraft:crafting_table", 300),
];
pub const FUEL_TAGS: [(&str, i32); 5] = [
    ("minecraft:logs", 300),
    ("minecraft:planks", 300),
    ("minecraft:wooden_slabs", 150),
    ("minecraft:saplings", 100),
    ("minecraft:wool", 100),
];

pub fn fuel_burn_time(item: &str) -> Option<i32> {
    FUELS.iter().find(|(id, _)| *id == item).map(|&(_, t)| t)
}

/// What the Heat Generator keeps between ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heat {
    pub heat: f64,
    pub energy: i32,
    pub lava: i32,
}

/// One tick's outcome besides the new state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatTick {
    pub active: bool,
    pub produced: i32,
    /// Kelvin per tick lost to the air (Mekanism's "dissipated").
    pub environment_loss: f64,
}

impl Heat {
    /// A new generator is at the ambient temperature (`BasicHeatCapacitor.getHeat`).
    pub fn new(ambient: f64) -> Self {
        Heat { heat: HEAT_CAPACITY * ambient, energy: 0, lava: 0 }
    }

    pub fn temperature(&self) -> f64 {
        self.heat / HEAT_CAPACITY
    }

    fn handle_heat(&mut self, transfer: f64) {
        if transfer.abs() > EPSILON {
            self.heat = (self.heat + transfer).max(0.0);
        }
    }

    /// `TileEntityHeatGenerator.onUpdateServer` after the fuel slot: passive
    /// heat, burning lava while the buffer has room, heat to energy, then the
    /// loss to the air on all six sides (the bottom is not insulated).
    pub fn tick(&mut self, boost: f64, ambient: f64) -> HeatTick {
        let prev = self.energy;
        self.handle_heat(boost);
        let mut out = HeatTick::default();
        if self.lava >= LAVA_RATE && self.energy < HEAT_STORAGE {
            self.lava -= LAVA_RATE;
            out.active = true;
            self.handle_heat(HEAT_GENERATION);
        }

        // `simulate`: Carnot efficiency between the generator and the air.
        let temp = self.temperature();
        let carnot = 1.0 - ambient.min(temp) / ambient.max(temp);
        let heat_lost = THERMAL_EFFICIENCY * (temp - ambient);
        self.handle_heat(-heat_lost);
        let from_heat = (heat_lost.abs() * carnot).min(i32::MAX as f64) as i32;
        self.energy += from_heat.min(MAX_PRODUCTION).min(HEAT_STORAGE - self.energy);

        // `simulateEnvironment`, one capacitor seen from each side.
        for side in 0..6 {
            let insulation = if side == 0 { 0.0 } else { INVERSE_INSULATION };
            let inv = AIR_INVERSE_COEFFICIENT + insulation + INVERSE_CONDUCTION;
            let t = (self.temperature() - ambient) / inv;
            self.handle_heat(-t * HEAT_CAPACITY);
            if t > 0.0 {
                out.environment_loss += t;
            }
        }
        out.produced = (self.energy - prev).max(0);
        out
    }
}

// --- Solar Generator ---
pub const SOLAR_STORAGE: i32 = 2 * TICKS_PER_SECOND;
pub const SOLAR_GENERATION: f64 = 2.0;

// --- Bio Generator ---
pub const BIO_STORAGE: i32 = 30 * TICKS_PER_SECOND;
pub const BIO_GENERATION: i32 = 25;
pub const BIO_TANK: i32 = 200;
pub const BIO_FUEL_PER_ITEM: i32 = 64;
/// `#c:fuels/bio` and `#c:fuels/block/bio` (`biofuelFromItem`).
pub const BIO_FUEL_TAG: &str = "c:fuels/bio";
pub const BIO_FUEL_BLOCK_TAG: &str = "c:fuels/block/bio";

/// One Bio Generator tick: burn 1 mB for `BIO_GENERATION` when the energy
/// all fits. Returns whether it ran (the generator is active).
pub fn bio_tick(fuel: &mut i32, energy: &mut i32) -> bool {
    if *fuel < 1 || *energy + BIO_GENERATION > BIO_STORAGE {
        return false;
    }
    *fuel -= 1;
    *energy += BIO_GENERATION;
    true
}

/// Mekanism's `FluidFuelInventorySlot.fillOrBurn`: an item is used only when
/// all the fuel it gives fits in the tank.
pub fn bio_fits(fuel: i32, from_item: i32) -> bool {
    from_item > 0 && fuel + from_item <= BIO_TANK
}

// --- multiblocks ---

/// Offsets of a multiblock's bounding blocks from its main block (`withBounding`).
pub fn bounding_offsets(block: &str) -> Vec<[i32; 3]> {
    match block {
        WIND => (1..=4).map(|y| [0, y, 0]).collect(),
        ADVANCED_SOLAR => {
            let mut v = vec![[0, 1, 0]];
            for x in -1..=1 {
                for z in -1..=1 {
                    v.push([x, 2, z]);
                }
            }
            v
        }
        _ => Vec::new(),
    }
}

pub fn is_multiblock(block: &str) -> bool {
    block == WIND || block == ADVANCED_SOLAR
}

/// Where the main block of a bounding block at `pos` may be: one candidate per
/// multiblock offset (the caller checks which holds a main block that claims it).
pub fn main_candidates(pos: [i32; 3]) -> Vec<(&'static str, [i32; 3])> {
    [WIND, ADVANCED_SOLAR]
        .into_iter()
        .flat_map(|b| {
            bounding_offsets(b).into_iter().map(move |o| (b, [pos[0] - o[0], pos[1] - o[1], pos[2] - o[2]]))
        })
        .collect()
}

// --- Advanced Solar Generator ---
pub const ADVANCED_SOLAR_STORAGE: i32 = 12 * TICKS_PER_SECOND;
pub const ADVANCED_SOLAR_GENERATION: f64 = 12.0;
/// The panel's nine sky checks: the 3 x 3 above its top, two blocks up.
pub const ADVANCED_SOLAR_CHECKS: usize = 9;

/// `getProduction` with `checkCanSeeSun`: it runs when more than four of the
/// nine positions see the sun, each one that does adds the biome's peak, and
/// the average scales the 12 FE/t by the sky's brightness.
pub fn advanced_solar_production(seeing: usize, darkness: f32, peak: f32) -> i32 {
    if seeing <= 4 {
        return 0;
    }
    let brightness = (1.0 - darkness).max(0.0);
    let multiplier = seeing as f32 * peak / ADVANCED_SOLAR_CHECKS as f32;
    (ADVANCED_SOLAR_GENERATION * (brightness * multiplier) as f64).clamp(0.0, i32::MAX as f64) as i32
}

// --- Wind Generator ---
pub const WIND_STORAGE: i32 = 5 * TICKS_PER_SECOND;
pub const WIND_GENERATION_MIN: f64 = 1.0;
pub const WIND_GENERATION_MAX: f64 = 5.0;
pub const WIND_MIN_Y: i32 = 24;
/// `DimensionType.MAX_Y`, the default of `windGenerationMaxY`.
pub const WIND_MAX_Y: i32 = 2031;

/// Vanilla's build heights by dimension: (min y, height, logical height).
pub fn dimension_heights(dimension: &str) -> (i32, i32, i32) {
    match dimension {
        "minecraft:the_nether" => (0, 256, 128),
        "minecraft:the_end" => (0, 256, 256),
        _ => (-64, 384, 384),
    }
}

/// `getMultiplier`: 0 when the top (four blocks up) can't see the sky,
/// otherwise rising linearly from 1 at y 24 to 5 at the top of the world.
pub fn wind_multiplier(top_y: i32, sees_sky: bool, dimension: &str) -> f64 {
    if !sees_sky {
        return 0.0;
    }
    let (min_build, height, logical) = dimension_heights(dimension);
    let max_level = (min_build + height).min(min_build + logical) - 1;
    let min_y = WIND_MIN_Y.max(min_build);
    let max_y = WIND_MAX_Y.min(max_level);
    let clamped = top_y.clamp(min_y, max_y);
    let slope = (WIND_GENERATION_MAX - WIND_GENERATION_MIN) / (max_y - min_y) as f64;
    (WIND_GENERATION_MIN + slope * (clamped - min_y) as f64) / WIND_GENERATION_MIN
}

/// The Advanced Solar Generator above its main block (`scripts/import_generator_models.py`):
/// the upper pole and the panels, drawn one block up.
pub const ADVANCED_SOLAR_TOP: &str = "mekanismgenerators:block/advanced_solar_generator/top";

/// The tower and blade models (`scripts/import_generator_models.py`, mirrors
/// `scripts/wind_parts.json`): each with how many blocks it was moved per axis
/// to fit a model's -16 to 32, and whether it is a blade (turns about the hub).
pub const WIND_PARTS: [(&str, [f32; 3], bool); 7] = [
    ("mekanismgenerators:block/wind_generator/rotor_0", [-2.0, 2.0, 0.0], true),
    ("mekanismgenerators:block/wind_generator/rotor_1", [0.0, 4.0, 0.0], true),
    ("mekanismgenerators:block/wind_generator/rotor_2", [0.0, 6.0, 0.0], true),
    ("mekanismgenerators:block/wind_generator/rotor_3", [4.0, 1.0, 0.0], true),
    ("mekanismgenerators:block/wind_generator/tower_4", [0.0, 0.0, 0.0], false),
    ("mekanismgenerators:block/wind_generator/tower_5", [0.0, 2.0, 0.0], false),
    ("mekanismgenerators:block/wind_generator/tower_6", [0.0, 4.0, 0.0], false),
];
/// The blades' hub, in blocks from the main block's corner, facing north; they turn about z.
pub const WIND_HUB: [f32; 3] = [0.5, 4.5, 0.156_248_75];

/// `RenderWindGenerator`'s turn per tick while running: faster the higher it stands.
pub fn wind_rotor_speed(y: i32, dimension: &str) -> f32 {
    let (min_build, height, logical) = dimension_heights(dimension);
    let max_level = (min_build + height).min(min_build + logical) - 1;
    32.0 * (y + 4 - min_build) as f32 / (max_level + 1 - min_build) as f32
}

/// `getCurrentGeneration`, truncated as `MathUtils.clampToInt`.
pub fn wind_generation(multiplier: f64) -> i32 {
    (WIND_GENERATION_MIN * multiplier).clamp(0.0, i32::MAX as f64) as i32
}

/// Vanilla biome climate: (id, temperature, downfall, has precipitation).
pub const BIOMES: [(&str, f32, f32, bool); 64] = [
    ("minecraft:plains", 0.8, 0.4, true),
    ("minecraft:sunflower_plains", 0.8, 0.4, true),
    ("minecraft:snowy_plains", 0.0, 0.5, true),
    ("minecraft:ice_spikes", 0.0, 0.5, true),
    ("minecraft:desert", 2.0, 0.0, false),
    ("minecraft:swamp", 0.8, 0.9, true),
    ("minecraft:mangrove_swamp", 0.8, 0.9, true),
    ("minecraft:forest", 0.7, 0.8, true),
    ("minecraft:flower_forest", 0.7, 0.8, true),
    ("minecraft:birch_forest", 0.6, 0.6, true),
    ("minecraft:old_growth_birch_forest", 0.6, 0.6, true),
    ("minecraft:dark_forest", 0.7, 0.8, true),
    ("minecraft:pale_garden", 0.7, 0.8, true),
    ("minecraft:old_growth_pine_taiga", 0.3, 0.8, true),
    ("minecraft:old_growth_spruce_taiga", 0.25, 0.8, true),
    ("minecraft:taiga", 0.25, 0.8, true),
    ("minecraft:snowy_taiga", -0.5, 0.4, true),
    ("minecraft:savanna", 2.0, 0.0, false),
    ("minecraft:savanna_plateau", 2.0, 0.0, false),
    ("minecraft:windswept_hills", 0.2, 0.3, true),
    ("minecraft:windswept_gravelly_hills", 0.2, 0.3, true),
    ("minecraft:windswept_forest", 0.2, 0.3, true),
    ("minecraft:windswept_savanna", 2.0, 0.0, false),
    ("minecraft:jungle", 0.95, 0.9, true),
    ("minecraft:sparse_jungle", 0.95, 0.8, true),
    ("minecraft:bamboo_jungle", 0.95, 0.9, true),
    ("minecraft:badlands", 2.0, 0.0, false),
    ("minecraft:eroded_badlands", 2.0, 0.0, false),
    ("minecraft:wooded_badlands", 2.0, 0.0, false),
    ("minecraft:meadow", 0.5, 0.8, true),
    ("minecraft:cherry_grove", 0.5, 0.8, true),
    ("minecraft:grove", -0.2, 0.8, true),
    ("minecraft:snowy_slopes", -0.3, 0.9, true),
    ("minecraft:frozen_peaks", -0.7, 0.9, true),
    ("minecraft:jagged_peaks", -0.7, 0.9, true),
    ("minecraft:stony_peaks", 1.0, 0.3, true),
    ("minecraft:river", 0.5, 0.5, true),
    ("minecraft:frozen_river", 0.0, 0.5, true),
    ("minecraft:beach", 0.8, 0.4, true),
    ("minecraft:snowy_beach", 0.05, 0.3, true),
    ("minecraft:stony_shore", 0.2, 0.3, true),
    ("minecraft:warm_ocean", 0.5, 0.5, true),
    ("minecraft:lukewarm_ocean", 0.5, 0.5, true),
    ("minecraft:deep_lukewarm_ocean", 0.5, 0.5, true),
    ("minecraft:ocean", 0.5, 0.5, true),
    ("minecraft:deep_ocean", 0.5, 0.5, true),
    ("minecraft:cold_ocean", 0.5, 0.5, true),
    ("minecraft:deep_cold_ocean", 0.5, 0.5, true),
    ("minecraft:frozen_ocean", 0.0, 0.5, true),
    ("minecraft:deep_frozen_ocean", 0.5, 0.5, true),
    ("minecraft:mushroom_fields", 0.9, 1.0, true),
    ("minecraft:dripstone_caves", 0.8, 0.4, true),
    ("minecraft:lush_caves", 0.5, 0.5, true),
    ("minecraft:deep_dark", 0.8, 0.4, true),
    ("minecraft:nether_wastes", 2.0, 0.0, false),
    ("minecraft:warped_forest", 2.0, 0.0, false),
    ("minecraft:crimson_forest", 2.0, 0.0, false),
    ("minecraft:soul_sand_valley", 2.0, 0.0, false),
    ("minecraft:basalt_deltas", 2.0, 0.0, false),
    ("minecraft:the_end", 0.5, 0.5, false),
    ("minecraft:end_highlands", 0.5, 0.5, false),
    ("minecraft:end_midlands", 0.5, 0.5, false),
    ("minecraft:small_end_islands", 0.5, 0.5, false),
    ("minecraft:end_barrens", 0.5, 0.5, false),
];

/// (temperature, downfall, has precipitation); plains for biomes not in the table.
pub fn climate(biome: Option<&str>) -> (f32, f32, bool) {
    biome
        .and_then(|b| BIOMES.iter().find(|(id, ..)| *id == b))
        .map_or((0.8, 0.4, true), |&(_, t, d, p)| (t, d, p))
}

/// `SolarCheck.peakMultiplier`: hot, dry biomes are better.
pub fn peak_multiplier((temperature, downfall, precipitation): (f32, f32, bool)) -> f32 {
    let temp_eff = 0.3 * (0.8 - temperature);
    let humidity_eff = if precipitation { -0.3 * downfall } else { 0.0 };
    1.0 + temp_eff + humidity_eff
}

/// Vanilla's darkening of the sky in 0 to 1 (`Level.updateSkyBrightness`),
/// from the time of day and the rain and thunder levels.
pub fn sky_darkness(day_time: i64, rain: f32, thunder: f32) -> f32 {
    let d = ((day_time.rem_euclid(24_000)) as f64 / 24_000.0 - 0.25).rem_euclid(1.0);
    let e = 0.5 - (d * std::f64::consts::PI).cos() / 2.0;
    let angle = (d * 2.0 + e) / 3.0;
    let mut g = 1.0 - ((angle * std::f64::consts::TAU).cos() * 2.0 + 0.2);
    g = 1.0 - g.clamp(0.0, 1.0);
    g *= 1.0 - rain as f64 * 5.0 / 16.0;
    g *= 1.0 - thunder as f64 * 5.0 / 16.0;
    (1.0 - g) as f32
}

/// `LightLevelUtils.canSeeSun`: a dimension with sky light, the sky darkened by
/// less than 4 of 11 steps, and full sky light (`Level.canSeeSky`).
pub fn sees_sun(has_sky_light: bool, darkness: f32, sky_light: u8) -> bool {
    has_sky_light && ((darkness * 11.0) as i32) < 4 && sky_light >= 15
}

/// `TileEntitySolarGenerator.getProduction`: the peak scaled by the sky's
/// brightness, truncated (`MathUtils.clampToInt`).
pub fn solar_production(sees: bool, darkness: f32, peak: f32) -> i32 {
    if !sees {
        return 0;
    }
    let brightness = (1.0 - darkness).max(0.0);
    (SOLAR_GENERATION * (brightness * peak) as f64).clamp(0.0, i32::MAX as f64) as i32
}

// --- menus and screens (176 x 166, Mekanism's GUI coordinates) ---
pub const WIDTH: f32 = 176.0;
pub const HEIGHT: f32 = 166.0;
pub const INVENTORY_Y: f32 = 84.0;
pub const ENERGY_BAR: [f32; 4] = [164.0, 15.0, 6.0, 54.0];

pub const HEAT_FUEL_SLOT: [f32; 2] = [17.0, 35.0];
pub const HEAT_LAVA_GAUGE: [f32; 4] = [55.0, 18.0, 66.0, 50.0];
pub const HEAT_DATA_ENERGY: usize = 0;
pub const HEAT_DATA_LAVA: usize = 1;
pub const HEAT_DATA_TEMP: usize = 2;
pub const HEAT_DATA_PRODUCING: usize = 3;
/// Environment loss in milli-kelvin per tick.
pub const HEAT_DATA_LOSS: usize = 4;
pub const HEAT_DATA_FACING: usize = 5;
pub const HEAT_DATA_ACTIVE: usize = 6;
pub const HEAT_DATA_LEN: usize = 7;

pub fn heat_menu_data(state: &Heat, tick: &HeatTick, facing: usize) -> [i32; HEAT_DATA_LEN] {
    [
        state.energy,
        state.lava,
        state.temperature().round().clamp(0.0, 65_535.0) as i32,
        tick.produced,
        (tick.environment_loss * 1_000.0).round().clamp(0.0, 65_535.0) as i32,
        facing as i32,
        tick.active as i32,
    ]
}

pub const SOLAR_SCREEN: [f32; 4] = [40.0, 23.0, 96.0, 40.0];
pub const SOLAR_SUN: [f32; 4] = [18.0, 35.0, 16.0, 16.0];
pub const SOLAR_DATA_ENERGY: usize = 0;
pub const SOLAR_DATA_PRODUCING: usize = 1;
pub const SOLAR_DATA_SEES_SUN: usize = 2;
pub const SOLAR_DATA_FACING: usize = 3;
/// The biome's peak multiplier in percent.
pub const SOLAR_DATA_PEAK: usize = 4;
pub const SOLAR_DATA_LEN: usize = 5;

pub fn solar_menu_data(energy: i32, producing: i32, sees: bool, facing: usize, peak: f32) -> [i32; SOLAR_DATA_LEN] {
    [energy, producing, sees as i32, facing as i32, (peak * 100.0).round().clamp(0.0, 65_535.0) as i32]
}

pub const BIO_FUEL_SLOT: [f32; 2] = [17.0, 35.0];
pub const BIO_FUEL_BAR: [f32; 4] = [7.0, 15.0, 4.0, 52.0];
pub const BIO_SCREEN: [f32; 4] = [48.0, 23.0, 80.0, 40.0];
pub const BIO_DATA_ENERGY: usize = 0;
pub const BIO_DATA_FUEL: usize = 1;
pub const BIO_DATA_PRODUCING: usize = 2;
pub const BIO_DATA_FACING: usize = 3;
pub const BIO_DATA_LEN: usize = 4;

pub fn bio_menu_data(energy: i32, fuel: i32, active: bool, facing: usize) -> [i32; BIO_DATA_LEN] {
    [energy, fuel, if active { BIO_GENERATION } else { 0 }, facing as i32]
}

pub fn inside(rect: [f32; 4], at: [f32; 2]) -> bool {
    let [x, y, w, h] = rect;
    at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
}

fn energy_lines(energy: i32, capacity: i32, producing: i32) -> Vec<String> {
    vec![format!("Energy: {energy} / {capacity} FE"), format!("Producing: {producing} FE/t")]
}

pub fn heat_tooltip(data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(HEAT_LAVA_GAUGE, at) {
        return vec![format!("Lava: {} / {LAVA_TANK} mB", d(HEAT_DATA_LAVA))];
    }
    if inside(ENERGY_BAR, at) {
        return energy_lines(d(HEAT_DATA_ENERGY), HEAT_STORAGE, d(HEAT_DATA_PRODUCING));
    }
    Vec::new()
}

pub fn bio_tooltip(data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(BIO_FUEL_BAR, at) {
        return vec![format!("Bioethanol: {} / {BIO_TANK} mB", d(BIO_DATA_FUEL))];
    }
    if inside(ENERGY_BAR, at) {
        return energy_lines(d(BIO_DATA_ENERGY), BIO_STORAGE, d(BIO_DATA_PRODUCING));
    }
    Vec::new()
}

/// Energy storage of the generators that use the solar screen.
pub fn panel_storage(block: &str) -> i32 {
    match block {
        ADVANCED_SOLAR => ADVANCED_SOLAR_STORAGE,
        WIND => WIND_STORAGE,
        _ => SOLAR_STORAGE,
    }
}

/// Solar, Advanced Solar and Wind Generator share the solar screen and data
/// (the wind's `SOLAR_DATA_PEAK` is its height multiplier in percent).
pub fn solar_tooltip(block: &str, data: &[i32], at: [f32; 2]) -> Vec<String> {
    let d = |i: usize| data.get(i).copied().unwrap_or(0);
    if inside(ENERGY_BAR, at) {
        return energy_lines(d(SOLAR_DATA_ENERGY), panel_storage(block), d(SOLAR_DATA_PRODUCING));
    }
    if inside(SOLAR_SUN, at) {
        if block == WIND {
            let wind = if d(SOLAR_DATA_SEES_SUN) != 0 { "Wind" } else { "No wind" };
            return vec![wind.to_owned(), format!("Height multiplier: {}%", d(SOLAR_DATA_PEAK))];
        }
        let sees = if d(SOLAR_DATA_SEES_SUN) != 0 { "Sees the sun" } else { "No sun" };
        return vec![sees.to_owned(), format!("Biome peak: {}%", d(SOLAR_DATA_PEAK))];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounding_shapes_match_mekanism() {
        assert_eq!(bounding_offsets(WIND), vec![[0, 1, 0], [0, 2, 0], [0, 3, 0], [0, 4, 0]]);
        let a = bounding_offsets(ADVANCED_SOLAR);
        assert_eq!(a.len(), 10);
        assert!(a.contains(&[0, 1, 0]) && a.contains(&[-1, 2, 1]) && a.contains(&[0, 2, 0]));
        assert!(bounding_offsets(SOLAR).is_empty());
        let c = main_candidates([5, 10, 5]);
        assert!(c.contains(&(WIND, [5, 6, 5])));
        assert!(c.contains(&(ADVANCED_SOLAR, [6, 8, 4])));
    }

    #[test]
    fn advanced_solar_needs_five_of_nine() {
        assert_eq!(advanced_solar_production(4, 0.0, 1.0), 0);
        assert_eq!(advanced_solar_production(9, 0.0, 1.0), 12);
        assert_eq!(advanced_solar_production(5, 0.0, 1.0), 6, "12 * 5/9 truncated");
        assert_eq!(advanced_solar_production(9, 0.5, 1.0), 6);
    }

    #[test]
    fn wind_rises_with_height() {
        let ow = "minecraft:overworld";
        assert_eq!(wind_multiplier(100, false, ow), 0.0);
        assert_eq!(wind_generation(wind_multiplier(24, true, ow)), 1);
        assert_eq!(wind_generation(wind_multiplier(-10, true, ow)), 1, "clamped to y 24");
        assert_eq!(wind_generation(wind_multiplier(319, true, ow)), 5);
        assert_eq!(wind_generation(wind_multiplier(400, true, ow)), 5);
        let mid = wind_multiplier(171, true, ow);
        assert!((mid - 2.99).abs() < 0.02, "{mid}");
        assert_eq!(wind_generation(wind_multiplier(127, true, "minecraft:the_nether")), 5);
    }

    #[test]
    fn bio_burns_one_mb_for_25_when_it_fits() {
        let (mut fuel, mut energy) = (2, 0);
        assert!(bio_tick(&mut fuel, &mut energy));
        assert_eq!((fuel, energy), (1, 25));
        let mut full = BIO_STORAGE - 24;
        assert!(!bio_tick(&mut fuel, &mut full), "25 does not fit");
        assert_eq!((fuel, full), (1, BIO_STORAGE - 24));
        fuel = 0;
        assert!(!bio_tick(&mut fuel, &mut energy));
    }

    #[test]
    fn bio_items_fit_whole() {
        assert!(bio_fits(0, BIO_FUEL_PER_ITEM));
        assert!(bio_fits(136, BIO_FUEL_PER_ITEM));
        assert!(!bio_fits(137, BIO_FUEL_PER_ITEM));
        assert!(!bio_fits(0, 9 * BIO_FUEL_PER_ITEM), "a block never fits the 200 mB tank");
        assert!(!bio_fits(0, 0));
    }

    #[test]
    fn bio_tooltips() {
        let d = bio_menu_data(100, 64, true, 2);
        assert_eq!(bio_tooltip(&d, [8.0, 30.0]), vec!["Bioethanol: 64 / 200 mB"]);
        assert_eq!(bio_tooltip(&d, [165.0, 30.0])[1], "Producing: 25 FE/t");
    }

    fn run(state: &mut Heat, boost: f64, ambient: f64, ticks: usize) -> HeatTick {
        let mut last = HeatTick::default();
        for _ in 0..ticks {
            last = state.tick(boost, ambient);
        }
        last
    }

    #[test]
    fn idle_generator_stays_at_ambient() {
        let mut h = Heat::new(300.0);
        let t = run(&mut h, 0.0, 300.0, 100);
        assert!((h.temperature() - 300.0).abs() < 1e-6);
        assert_eq!((h.energy, t.produced, t.active), (0, 0, false));
    }

    #[test]
    fn lava_around_it_produces_energy_without_fuel() {
        let mut h = Heat::new(300.0);
        run(&mut h, boost(6, false), 300.0, 400);
        // Steady state: 42 heat in, 0.5 (T - 300) out after the boost: T about 379.8 K.
        assert!((h.temperature() - 379.8).abs() < 1.0, "{}", h.temperature());
        assert_eq!(h.energy, HEAT_STORAGE, "nothing takes the energy: the buffer fills");
        assert_eq!(h.tick(boost(6, false), 300.0).produced, 0, "and stops");
        // 42 heat at a Carnot efficiency of about 0.22 gives 9 FE/t.
        h.energy = 0;
        assert_eq!(h.tick(boost(6, false), 300.0).produced, 9);
    }

    #[test]
    fn burning_lava_uses_100_mb_per_tick_while_there_is_room() {
        let mut h = Heat::new(300.0);
        h.lava = 250;
        assert!(h.tick(0.0, 300.0).active);
        assert!(h.tick(0.0, 300.0).active);
        assert!(!h.tick(0.0, 300.0).active, "50 mB left is less than one tick's");
        assert_eq!(h.lava, 50);
        let mut full = Heat { heat: 3_000.0, energy: HEAT_STORAGE, lava: 1_000 };
        assert!(!full.tick(0.0, 300.0).active, "a full buffer burns nothing");
        assert_eq!(full.lava, 1_000);
    }

    #[test]
    fn production_never_exceeds_room_or_cap() {
        let mut h = Heat { heat: 1e9, energy: HEAT_STORAGE - 3, lava: 0 };
        let t = h.tick(0.0, 300.0);
        assert_eq!(t.produced, 3);
        assert_eq!(h.energy, HEAT_STORAGE);
        let mut h = Heat { heat: 1e9, energy: 0, lava: 0 };
        assert_eq!(h.tick(0.0, 300.0).produced, MAX_PRODUCTION);
    }

    #[test]
    fn fuel_items_give_lava_by_burn_time() {
        assert_eq!(fuel_lava(LAVA_BURN_TIME), LAVA_TANK);
        assert_eq!(fuel_lava(1_600), 80);
        assert_eq!(fuel_burn_time("minecraft:coal"), Some(1_600));
        assert_eq!(fuel_burn_time("minecraft:dirt"), None);
    }

    #[test]
    fn ambient_follows_biome_temperature() {
        assert_eq!(ambient_temp(0.8), 300.0);
        assert_eq!(ambient_temp(2.0), 330.0);
        assert_eq!(ambient_temp(-50.0), 300.0 + 25.0 * -5.8);
    }

    #[test]
    fn sky_is_clear_at_noon_and_dark_at_midnight() {
        assert!(sky_darkness(6_000, 0.0, 0.0) < 0.01);
        assert!(sky_darkness(18_000, 0.0, 0.0) > 0.99);
        assert!(sees_sun(true, sky_darkness(6_000, 0.0, 0.0), 15));
        assert!(!sees_sun(true, sky_darkness(18_000, 0.0, 0.0), 15));
        assert!(!sees_sun(false, 0.0, 15), "no sky light in the Nether");
        assert!(!sees_sun(true, 0.0, 14), "something above the panel");
        // Rain darkens by 3 of 11 steps at noon (still sun), rain and thunder by 5.
        assert!(sees_sun(true, sky_darkness(30_000, 1.0, 0.0), 15));
        assert!(!sees_sun(true, sky_darkness(6_000, 1.0, 1.0), 15));
    }

    #[test]
    fn solar_production_by_biome() {
        let noon = sky_darkness(6_000, 0.0, 0.0);
        let plains = peak_multiplier(climate(Some("minecraft:plains")));
        let desert = peak_multiplier(climate(Some("minecraft:desert")));
        let snowy = peak_multiplier(climate(Some("minecraft:snowy_plains")));
        assert!((plains - 0.88).abs() < 1e-5);
        assert!((desert - 0.64).abs() < 1e-5);
        assert!(snowy > plains);
        assert_eq!(solar_production(true, noon, plains), 1);
        assert_eq!(solar_production(true, noon, snowy), 2);
        assert_eq!(solar_production(false, noon, snowy), 0);
        assert_eq!(climate(Some("modded:biome")), climate(None));
    }

    #[test]
    fn menu_data_fits_16_bits() {
        let h = Heat { heat: 1e9, energy: HEAT_STORAGE, lava: LAVA_TANK };
        let t = HeatTick { active: true, produced: MAX_PRODUCTION, environment_loss: 1e9 };
        assert!(heat_menu_data(&h, &t, 5).iter().all(|v| (0..=0xFFFF).contains(v)));
        assert!(solar_menu_data(SOLAR_STORAGE, 2, true, 5, 1.44).iter().all(|v| (0..=0xFFFF).contains(v)));
    }

    #[test]
    fn tooltips_name_what_is_hovered() {
        let h = Heat { heat: 4_000.0, energy: 120, lava: 300 };
        let data = heat_menu_data(&h, &HeatTick { produced: 40, ..Default::default() }, 2);
        assert_eq!(heat_tooltip(&data, [60.0, 30.0]), vec!["Lava: 300 / 1000 mB"]);
        assert_eq!(heat_tooltip(&data, [165.0, 30.0])[1], "Producing: 40 FE/t");
        assert!(heat_tooltip(&data, [10.0, 10.0]).is_empty());
        let data = solar_menu_data(20, 1, true, 2, 0.88);
        assert_eq!(solar_tooltip(SOLAR, &data, [20.0, 40.0]), vec!["Sees the sun", "Biome peak: 88%"]);
        assert_eq!(solar_tooltip(SOLAR, &data, [165.0, 30.0])[0], "Energy: 20 / 40 FE");
    }
}
